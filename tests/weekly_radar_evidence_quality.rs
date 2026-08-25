use chrono::Utc;
use org_x::features::weekly_radar::runtime::config::CompanyConfig;
use org_x::features::weekly_radar::runtime::evidence::{
    extract_evidence_candidate, validate_evidence_candidate, EvidenceCandidate, EvidencePolarity,
    EvidenceSourceKind, EvidenceValidationError,
};
use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpResponse};
use org_x::features::weekly_radar::runtime::model::{FactStatus, RuntimeReportInput};
use org_x::features::weekly_radar::runtime::normalize_source_observation;
use org_x::features::weekly_radar::runtime::sec::SecClient;
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, SourceKind, SourceMaterialKind,
};

fn company() -> CompanyConfig {
    CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        Some("https://example.test/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .expect("evidence-quality fixture company should be valid")
}

fn sec_company() -> CompanyConfig {
    CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        Some("0001234567".to_owned()),
        None,
        None,
        None,
        None,
        None,
    )
    .expect("SEC fixture company should be valid")
}

fn company_facts_with_revenue() -> &'static str {
    r#"{
      "facts": {
        "us-gaap": {
          "RevenueFromContractWithCustomerExcludingAssessedTax": {
            "units": {"USD": [{
              "start":"2024-01-01","end":"2024-12-31","val":100,
              "accn":"000123456725000001","fp":"FY","form":"10-K","filed":"2025-02-15"
            }]}
          }
        }
      }
    }"#
}

fn submissions_with_recent_filings() -> &'static str {
    r#"{
      "filings": {"recent": {
        "accessionNumber": ["0001234567-25-000003", "0001234567-25-000002", "0001234567-25-000001", "0001234567-24-000001"],
        "filingDate": ["2025-03-01", "2025-02-15", "2025-01-31", "2024-02-15"],
        "reportDate": ["2024-12-31", "2024-12-31", "2024-09-30", "2023-12-31"],
        "form": ["8-K", "10-K", "10-Q", "10-K"],
        "primaryDocument": ["acme-8k.htm", "acme-2024.htm", "acme-q3.htm", "acme-2023.htm"]
      }}
    }"#
}

#[test]
fn official_entry_point_is_not_a_confirmed_fact() {
    let company = company();
    let client = FixtureHttpClient::with_response(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok("<title>Investor Relations</title><p>Investor Relations</p>"),
    );
    let observation = collect_configured_sources(&company, &client, Utc::now())
        .into_iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("IR entry point should be observed");

    assert_eq!(observation.material_kind(), SourceMaterialKind::EntryPoint);
    let fact = normalize_source_observation(&observation, 1).unwrap();
    assert_eq!(fact.status(), &FactStatus::Unconfirmed);
    assert_eq!(fact.value(), None);
}

#[test]
fn legacy_runtime_input_defaults_research_metrics_to_zero() {
    let legacy = serde_json::json!({
        "as_of": "2026-08-25",
        "companies": [],
        "facts": [],
        "source_coverage": [],
        "source_failures": []
    });
    let input: RuntimeReportInput = serde_json::from_value(legacy).unwrap();

    assert_eq!(input.research_metrics().validated_evidence(), 0);
    assert_eq!(input.research_metrics().source_available(), 0);
}

#[test]
fn sec_keeps_company_facts_when_submissions_request_fails() {
    let client = FixtureHttpClient::new();
    client.insert(
        "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
        HttpResponse::ok(company_facts_with_revenue()),
    );

    let evidence = SecClient::collect(&sec_company(), &client, "ORG-X test contact@example.test")
        .expect("partial SEC result should be retained");

    assert_eq!(
        evidence.fact("revenue").unwrap().status(),
        &FactStatus::Known
    );
    assert!(evidence
        .failures()
        .iter()
        .any(|failure| failure.stage() == "submissions"));
}

#[test]
fn sec_discovers_only_bounded_recent_filings_with_provenance() {
    let client = FixtureHttpClient::new();
    client.insert(
        "https://data.sec.gov/submissions/CIK0001234567.json",
        HttpResponse::ok(submissions_with_recent_filings()),
    );
    client.insert(
        "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
        HttpResponse::ok(r#"{"facts":{}}"#),
    );

    let evidence = SecClient::collect(&sec_company(), &client, "ORG-X test contact@example.test")
        .expect("SEC document candidates should be retained");

    assert_eq!(evidence.documents().len(), 3);
    assert!(evidence.documents().iter().all(|document| document
        .source_uri()
        .starts_with("https://www.sec.gov/Archives/")));
}

#[test]
fn official_entry_point_discovers_relevant_same_origin_documents_only() {
    let mut company = company();
    company.official_ir = Some("https://ir.example.test/investors".to_owned());
    let client = FixtureHttpClient::new();
    client.insert(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok(
            r#"<a href="/earnings/q2">Q2 Earnings Release</a>
               <a href="https://ir.example.test/organization/update">Organization update</a>
               <a href="https://evil.example.test/leak">Organization update</a>"#,
        ),
    );
    client.insert(
        "https://ir.example.test/earnings/q2",
        HttpResponse::ok("<title>Q2 Earnings Release</title><time datetime=\"2026-08-20\">"),
    );
    client.insert(
        "https://ir.example.test/organization/update",
        HttpResponse::ok("<title>Organization update</title><time datetime=\"2026-08-19\">"),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let documents = observations
        .iter()
        .filter(|observation| observation.material_kind() == SourceMaterialKind::Document)
        .collect::<Vec<_>>();

    assert_eq!(documents.len(), 2);
    assert!(documents.iter().all(|observation| observation
        .url()
        .expect("document URL exists")
        .starts_with("https://ir.example.test/")));
}

#[test]
fn homepage_only_is_available_but_never_a_document_or_confirmed_fact() {
    let company = company();
    let client = FixtureHttpClient::with_response(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok("<title>Investor Relations</title>"),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let entry = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("IR entry point should exist");

    assert_eq!(entry.material_kind(), SourceMaterialKind::EntryPoint);
    assert_eq!(
        normalize_source_observation(entry, 1).unwrap().status(),
        &FactStatus::Unconfirmed
    );
    assert_eq!(
        observations
            .iter()
            .filter(|observation| observation.material_kind() == SourceMaterialKind::Document)
            .count(),
        0
    );
}

#[test]
fn document_discovery_deduplicates_and_caps_followed_links() {
    let mut company = company();
    company.official_ir = Some("https://ir.example.test/investors".to_owned());
    let links = (0..12)
        .map(|index| {
            format!(
                "<a href=\"/engineering/update-{index}#fragment\">Engineering update {index}</a>"
            )
        })
        .collect::<String>();
    let html = format!(
        "{links}<a href=\"/engineering/update-0\">Duplicate update</a><a href=\"https://other.example.test/engineering\">Cross origin</a>"
    );
    let client = FixtureHttpClient::with_response(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok(html),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let documents = observations
        .iter()
        .filter(|observation| observation.material_kind() == SourceMaterialKind::Document)
        .collect::<Vec<_>>();
    let urls = documents
        .iter()
        .map(|observation| observation.url().expect("document URL exists"))
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(documents.len(), 8);
    assert_eq!(urls.len(), 8);
    assert!(urls
        .iter()
        .all(|url| url.starts_with("https://ir.example.test/")));
    assert!(urls.iter().all(|url| !url.contains('#')));
}

fn cutoff() -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(2026, 8, 25).expect("cutoff fixture should be valid")
}

fn complete_candidate(effective_date: &str, production_area: &str) -> EvidenceCandidate {
    EvidenceCandidate::new(
        "acme",
        "Acme Corporation",
        "Acme changed its engineering workflow",
        Some(chrono::NaiveDate::parse_from_str(effective_date, "%Y-%m-%d").unwrap()),
        production_area,
        EvidenceSourceKind::OfficialMaterial,
        org_x::features::weekly_radar::runtime::SourceTier::OfficialPrimary,
        EvidencePolarity::Supporting,
        "https://ir.example.test/organization/update",
    )
    .unwrap()
    .with_source_details(
        "Organization update",
        "Acme changed its engineering workflow.",
    )
}

#[test]
fn evidence_gate_rejects_missing_production_area_and_cutoff_date() {
    let candidate = EvidenceCandidate::new(
        "acme",
        "Acme Corporation",
        "Acme changed responsibility",
        None,
        "",
        EvidenceSourceKind::OfficialMaterial,
        org_x::features::weekly_radar::runtime::SourceTier::OfficialPrimary,
        EvidencePolarity::Supporting,
        "https://ir.example.test/update",
    )
    .unwrap();

    let error = validate_evidence_candidate(&candidate, cutoff()).unwrap_err();

    assert_eq!(
        error,
        EvidenceValidationError::MissingRequiredField {
            field: "effective_date"
        }
    );
}

#[test]
fn complete_authoritative_candidate_becomes_validated_evidence() {
    let candidate = complete_candidate("2026-08-19", "engineering workflow");

    let validated = validate_evidence_candidate(&candidate, cutoff()).unwrap();

    assert_eq!(validated.company_id(), "acme");
    assert_eq!(validated.production_area(), "engineering workflow");
    assert_eq!(
        validated.effective_date(),
        Some(&chrono::NaiveDate::from_ymd_opt(2026, 8, 19).unwrap())
    );
}

#[test]
fn page_level_observation_cannot_create_an_evidence_candidate() {
    let company = company();
    let client = FixtureHttpClient::with_response(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok("<title>Investor Relations</title>"),
    );
    let entry = collect_configured_sources(&company, &client, Utc::now())
        .into_iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("IR entry point should exist");

    assert!(extract_evidence_candidate(&entry).is_none());
}
