use chrono::{NaiveDate, Utc};
use org_x::features::transformation::domain::{
    ReferenceModelEvidenceFamily, ReferenceModelSourceRole,
};
use org_x::features::weekly_radar::runtime::config::CompanyConfig;
use org_x::features::weekly_radar::runtime::discovery::{document_metadata, DocumentKind};
use org_x::features::weekly_radar::runtime::evidence::{
    extract_evidence_candidate, validate_evidence_candidate, EvidenceCandidate, EvidenceClass,
    EvidencePolarity, EvidenceSourceKind, EvidenceValidationError,
};
use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpResponse};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, ResearchMetrics, RuntimeReportInput,
    StructuralDimension, StructuralEvidenceContract,
};
use org_x::features::weekly_radar::runtime::normalize_source_observation;
use org_x::features::weekly_radar::runtime::report::{render_report_in_language, ReportLanguage};
use org_x::features::weekly_radar::runtime::sec::SecClient;
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, document_observation, DocumentObservationInput, SourceKind,
    SourceMaterialKind, SourceStatus, SourceTier,
};
use std::collections::BTreeMap;
use url::Url;

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

fn document_observation_from_html(
    html: &str,
    document_url: &str,
) -> org_x::features::weekly_radar::runtime::SourceObservation {
    let mut company = company();
    let mut entry_url = Url::parse(document_url).expect("document fixture URL should parse");
    entry_url.set_path("/");
    entry_url.set_query(None);
    entry_url.set_fragment(None);
    let entry_url = entry_url.to_string();
    if document_url.contains("/customers/") {
        company.official_research_sources = vec![entry_url.clone()];
    } else {
        company.official_ir = Some(entry_url.clone());
    }
    let client = FixtureHttpClient::new();
    let label = if document_url.contains("/careers/") {
        "Careers areas"
    } else {
        "Engineering update"
    };
    client.insert(
        &entry_url,
        HttpResponse::ok(format!("<a href=\"{document_url}\">{label}</a>")),
    );
    client.insert(document_url, HttpResponse::ok(html));

    collect_configured_sources(&company, &client, Utc::now())
        .into_iter()
        .find(|observation| {
            observation.material_kind() == SourceMaterialKind::Document
                && observation.url() == Some(document_url)
        })
        .expect("document fixture should be discovered")
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
    assert_eq!(input.research_metrics().structural_evidence(), 0);
    assert_eq!(input.research_metrics().sec_stage_expected(), 0);
    assert_eq!(input.research_metrics().sec_stage_available(), 0);
    assert_eq!(input.research_metrics().sec_fact_expected(), 0);
    assert_eq!(input.research_metrics().sec_fact_available(), 0);
    assert!(input.research_metrics().document_kind_counts().is_empty());
}

#[test]
fn structural_dimension_is_retained_and_legacy_fact_json_defaults_to_none() {
    let provenance = Provenance::from_rfc3339(
        "https://ir.example.test/metrics/update",
        "GPU utilization increased to 80%",
        "2026-08-25T00:00:00Z",
        Some("2026-08-20"),
    )
    .unwrap();
    let fact = NormalizedFact::new_with_structural_dimension(
        "acme",
        "evidence_structural_change_001",
        "Acme reported higher GPU utilization.",
        Some(StructuralDimension::OperatingMetric),
        FactStatus::Known,
        Confidence::High,
        provenance,
    )
    .unwrap();

    assert_eq!(
        fact.structural_dimension(),
        Some(StructuralDimension::OperatingMetric)
    );
    let serialized = serde_json::to_value(&fact).unwrap();
    assert_eq!(
        serialized["structural_dimension"],
        serde_json::json!("operating_metric")
    );

    let legacy = serde_json::json!({
        "company_id": "acme",
        "kind": "evidence_structural_change_002",
        "value": "Legacy structural evidence",
        "status": "KNOWN",
        "confidence": "HIGH",
        "provenance": {
            "source_uri": "https://ir.example.test/legacy",
            "source_field_or_passage": "Legacy passage",
            "retrieved_at": "2026-08-25T00:00:00Z",
            "effective_date": "2026-08-20"
        }
    });
    let legacy_fact: NormalizedFact = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy_fact.structural_dimension(), None);
    assert_eq!(legacy_fact.reference_model_family(), None);
}

#[test]
fn reference_model_family_metadata_is_typed_and_legacy_json_defaults_to_none() {
    let provenance = Provenance::from_rfc3339(
        "https://ir.example.test/ai-platform",
        "The company reorganized engineering responsibilities around an AI platform.",
        "2026-08-25T00:00:00Z",
        Some("2026-08-20"),
    )
    .unwrap();
    let fact = NormalizedFact::new_with_structural_dimension_and_reference_model_metadata(
        "acme",
        "evidence_structural_change_001",
        "The company reorganized engineering responsibilities around an AI platform.",
        Some(StructuralDimension::Organization),
        Some(ReferenceModelEvidenceFamily::OrganizationRewrite),
        None,
        FactStatus::Known,
        Confidence::High,
        provenance,
    )
    .unwrap();

    assert_eq!(
        fact.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::OrganizationRewrite)
    );
    assert_eq!(fact.reference_model_named_peer(), None);
    let serialized = serde_json::to_value(&fact).unwrap();
    assert_eq!(
        serialized["reference_model_family"],
        serde_json::json!("organization_rewrite")
    );

    let legacy = serde_json::json!({
        "company_id": "acme",
        "kind": "evidence_structural_change_003",
        "value": "Legacy structural evidence",
        "status": "KNOWN",
        "confidence": "HIGH",
        "provenance": {
            "source_uri": "https://ir.example.test/legacy",
            "source_field_or_passage": "Legacy passage",
            "retrieved_at": "2026-08-25T00:00:00Z",
            "effective_date": "2026-08-20"
        }
    });
    let legacy_fact: NormalizedFact = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy_fact.reference_model_family(), None);
    assert_eq!(legacy_fact.reference_model_named_peer(), None);
    assert_eq!(legacy_fact.reference_model_source_role(), None);
}

#[test]
fn document_claim_extraction_assigns_reference_model_family_only_after_validation() {
    let observation = document_observation_from_html(
        r#"<html><head><title>AI Organization Update</title></head>
        <body><time datetime="2026-08-20"></time>
        <p>The company reorganized engineering responsibilities and reporting lines around an AI platform.</p>
        </body></html>"#,
        "https://ir.example.test/organization/ai-update",
    );
    let candidate = extract_evidence_candidate(&observation).expect("document claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("official document claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::OrganizationRewrite)
    );
    let fact = validated.to_normalized_fact(1).unwrap();
    assert_eq!(
        fact.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::OrganizationRewrite)
    );
}

#[test]
fn microsoft_frontier_claim_prose_is_promotable_after_document_discovery() {
    let observation = document_observation_from_html(
        r#"<html><head>
        <title>Becoming a Frontier Firm</title>
        <meta property="article:published_time" content="2025-12-04T17:00:00+00:00" />
        </head><body>
        <p>Microsoft Digital, the company’s IT team, is rapidly transforming into a Frontier IT Firm—an organization fundamentally restructured for the AI era, where AI-agents are digital colleagues rather than peripheral tools.</p>
        </body></html>"#,
        "https://ir.example.test/frontier/becoming-a-frontier-firm",
    );
    let candidate = extract_evidence_candidate(&observation).expect("frontier claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("frontier claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::OrganizationRewrite)
    );
}

#[test]
fn diffusion_claim_extraction_retains_an_explicit_named_peer() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Platform Adoption</title></head>
        <body><time datetime="2026-08-20"></time>
        <p>The agent workflow was adopted by Peer Alpha and Peer Beta across production operations.</p>
        </body></html>"#,
        "https://ir.example.test/engineering/platform-adoption",
    );
    let candidate = extract_evidence_candidate(&observation).expect("diffusion claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("official diffusion claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("Peer Alpha"));
    let fact = validated.to_normalized_fact(1).unwrap();
    assert_eq!(fact.reference_model_named_peer(), Some("Peer Alpha"));
}

#[test]
fn diffusion_claim_extraction_handles_named_peer_before_adoption_verb() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Frontier adoption</title></head>
        <body><time datetime="2026-08-20"></time>
        <p>PwC adopted the agent workflow across production operations and reported faster delivery.</p>
        </body></html>"#,
        "https://ir.example.test/engineering/frontier-adoption",
    );
    let candidate = extract_evidence_candidate(&observation).expect("diffusion claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("official diffusion claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("PwC"));
}

#[test]
fn official_customer_story_rollout_is_diffusion_evidence() {
    let observation = document_observation_from_html(
        r#"<html><head><title>PwC modernizes with Copilot</title></head>
        <body><time datetime="2026-02-25"></time>
        <p>PwC rolled out Microsoft 365 with Copilot across the entire firm, providing secure tools powered by AI to every employee.</p>
        </body></html>"#,
        "https://www.microsoft.com/en/customers/story/26160-pwc-microsoft-365-enterprise",
    );
    let candidate = extract_evidence_candidate(&observation).expect("rollout claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("official rollout claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("PwC"));
    assert_eq!(
        validated.reference_model_source_role(),
        Some(ReferenceModelSourceRole::SupplierAttribution)
    );
    assert_eq!(
        validated
            .to_normalized_fact(1)
            .unwrap()
            .reference_model_source_role(),
        Some(ReferenceModelSourceRole::SupplierAttribution)
    );
}

#[test]
fn independent_customer_document_promotes_with_independent_source_role() {
    let observation = document_observation(DocumentObservationInput {
        company_id: "acme".to_owned(),
        kind: SourceKind::IndependentResearch,
        tier: SourceTier::IndependentPrimary,
        url: "https://customer.example/library/case-studies/acme-ai".to_owned(),
        title: "Acme AI operations disclosure".to_owned(),
        text: "Acme rolled out an agent workflow across production operations.".to_owned(),
        status: SourceStatus::Known,
        status_reason: "fixture document".to_owned(),
        document_kind: DocumentKind::ProductPlatform,
        source_field_or_passage: "customer disclosure".to_owned(),
        observed_at: Utc::now(),
        effective_date: Some(NaiveDate::from_ymd_opt(2026, 2, 25).unwrap()),
    });
    let candidate = extract_evidence_candidate(&observation).expect("independent claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("independent claim should validate");

    assert_eq!(
        validated.reference_model_source_role(),
        Some(ReferenceModelSourceRole::IndependentCustomerDisclosure)
    );
}

#[test]
fn independent_customer_title_and_body_promote_named_adopter() {
    let observation = document_observation(DocumentObservationInput {
        company_id: "msft".to_owned(),
        kind: SourceKind::IndependentResearch,
        tier: SourceTier::IndependentPrimary,
        url: "https://customer.example/library/case-studies/pwc-microsoft-copilot".to_owned(),
        title: "PwC deploys Microsoft Copilot at enterprise scale".to_owned(),
        text: "PwC is reimagining how work gets done by integrating Microsoft Copilot across its global network. The deployment reached 285000 users across more than 100 countries.".to_owned(),
        status: SourceStatus::Known,
        status_reason: "fixture document".to_owned(),
        document_kind: DocumentKind::ProductPlatform,
        source_field_or_passage: "customer disclosure".to_owned(),
        observed_at: Utc::now(),
        effective_date: Some(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap()),
    });

    let candidate = extract_evidence_candidate(&observation)
        .expect("customer title and body should produce an independent diffusion claim");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("independent diffusion claim should validate");
    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("PwC"));
    assert_eq!(
        validated.reference_model_source_role(),
        Some(ReferenceModelSourceRole::IndependentCustomerDisclosure)
    );
}

#[test]
fn independent_customer_deployed_past_tense_promotes_named_adopter() {
    let observation = document_observation(DocumentObservationInput {
        company_id: "msft".to_owned(),
        kind: SourceKind::IndependentResearch,
        tier: SourceTier::IndependentPrimary,
        url: "https://www.pwc.com/us/en/library/case-studies/pwc-microsoft-copilot-enterprise-ai.html"
            .to_owned(),
        title: "How PwC scaled Microsoft Copilot securely for 285,000+ users".to_owned(),
        text: "PwC deployed Microsoft Copilot to 285000 users worldwide, scaling secure Responsible AI to help boost productivity, collaboration, and trust.".to_owned(),
        status: SourceStatus::Known,
        status_reason: "fixture document".to_owned(),
        document_kind: DocumentKind::ProductPlatform,
        source_field_or_passage: "customer disclosure".to_owned(),
        observed_at: Utc::now(),
        effective_date: Some(NaiveDate::from_ymd_opt(2026, 1, 16).unwrap()),
    });

    let candidate = extract_evidence_candidate(&observation)
        .expect("past-tense customer deployment should produce a claim");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("past-tense customer deployment should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("PwC"));
    assert_eq!(
        validated.reference_model_source_role(),
        Some(ReferenceModelSourceRole::IndependentCustomerDisclosure)
    );
}

#[test]
fn independent_customer_infinitive_deployment_promotes_named_adopter() {
    let observation = document_observation(DocumentObservationInput {
        company_id: "msft".to_owned(),
        kind: SourceKind::IndependentResearch,
        tier: SourceTier::IndependentPrimary,
        url: "https://www.atosgroup.com/en/press/atos-group-and-microsoft-expand-strategic-collaboration-scale-secure-agentic-ai-across-atos".to_owned(),
        title: "Atos Group and Microsoft Expand Strategic Collaboration to Scale Secure Agentic AI Across Atos Group Workforce and Clients".to_owned(),
        text: "Atos Group becomes the first French Global System Integrator to deploy Microsoft 365 Copilot and one of the largest to roll out secure agentic AI across its workforce.".to_owned(),
        status: SourceStatus::Known,
        status_reason: "fixture document".to_owned(),
        document_kind: DocumentKind::ProductPlatform,
        source_field_or_passage: "customer disclosure".to_owned(),
        observed_at: Utc::now(),
        effective_date: Some(NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()),
    });
    let candidate = extract_evidence_candidate(&observation)
        .expect("infinitive deployment should produce an evidence candidate");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("infinitive deployment claim should validate");
    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("Atos Group"));
    assert_eq!(
        validated.reference_model_source_role(),
        Some(ReferenceModelSourceRole::IndependentCustomerDisclosure)
    );
}

#[test]
fn official_customer_story_usage_is_diffusion_evidence() {
    let observation = document_observation_from_html(
        r#"<html><head><title>NIQ scales product coding with Foundry</title></head>
        <body><time datetime="2025-12-16"></time>
        <p>NIQ used Microsoft Foundry to build Capture as a Service, automating item coding for faster and more scalable product data.</p>
        </body></html>"#,
        "https://www.microsoft.com/en/customers/story/25893-niq-microsoft-foundry",
    );
    let candidate = extract_evidence_candidate(&observation).expect("usage claim expected");
    let validated =
        validate_evidence_candidate(&candidate, NaiveDate::from_ymd_opt(2026, 8, 25).unwrap())
            .expect("official usage claim should validate");

    assert_eq!(
        validated.reference_model_family(),
        Some(ReferenceModelEvidenceFamily::IndustryDiffusion)
    );
    assert_eq!(validated.reference_model_named_peer(), Some("NIQ"));
}

#[test]
fn research_metrics_retain_structural_and_sec_health_counts() {
    let metrics = ResearchMetrics::new(9, 10, 5, 71, 32)
        .with_structural_evidence(2)
        .with_sec_health(20, 18, 80, 74);

    assert_eq!(metrics.structural_evidence(), 2);
    assert_eq!(metrics.sec_stage_expected(), 20);
    assert_eq!(metrics.sec_stage_available(), 18);
    assert_eq!(metrics.sec_fact_expected(), 80);
    assert_eq!(metrics.sec_fact_available(), 74);
}

#[test]
fn research_metrics_document_kind_counts_default_for_legacy_json() {
    let legacy = serde_json::json!({
        "source_available": 9,
        "document_candidates": 10,
        "validated_evidence": 5,
        "pending_leads": 71,
        "unavailable_sources": 32
    });
    let metrics: ResearchMetrics = serde_json::from_value(legacy).unwrap();

    assert!(metrics.document_kind_counts().is_empty());
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
    assert_eq!(
        evidence.fact("revenue").unwrap().reference_model_family(),
        Some(ReferenceModelEvidenceFamily::SustainedOutcome)
    );
    assert!(evidence
        .failures()
        .iter()
        .any(|failure| failure.stage() == "submissions"));
}

#[test]
fn sec_retains_bounded_distinct_outcome_periods_without_duplicate_fact_identity() {
    let client = FixtureHttpClient::new();
    client.insert(
        "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
        HttpResponse::ok(
            r#"{
              "facts": {
                "us-gaap": {
                  "RevenueFromContractWithCustomerExcludingAssessedTax": {
                    "units": {"USD": [
                      {"start":"2023-01-01","end":"2023-12-31","val":90,"accn":"000123456724000001","fp":"FY","form":"10-K","filed":"2024-02-15"},
                      {"start":"2024-01-01","end":"2024-12-31","val":100,"accn":"000123456725000001","fp":"FY","form":"10-K","filed":"2025-02-15"}
                    ]}
                  }
                }
              }
            }"#,
        ),
    );

    let evidence = SecClient::collect(&sec_company(), &client, "ORG-X test contact@example.test")
        .expect("bounded SEC period fixture should collect");
    let revenue = evidence.fact("revenue").unwrap();

    assert_eq!(revenue.value(), Some("100"));
    assert_eq!(
        revenue.reference_model_periods(),
        &["2024-12-31".to_owned(), "2023-12-31".to_owned()]
    );
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
fn ir_deep_discovery_reaches_one_same_origin_nested_document() {
    let mut company = company();
    company.official_ir = Some("https://ir.example.test/investors".to_owned());
    let client = FixtureHttpClient::new();
    client.insert(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok(r#"<a href="/earnings/archive">Earnings archive</a>"#),
    );
    client.insert(
        "https://ir.example.test/earnings/archive",
        HttpResponse::ok(
            r#"<title>Earnings archive</title><a href="/earnings/q3-2026">Q3 earnings release</a><a href="https://evil.example.test/earnings/q3-2026">Cross-origin release</a>"#,
        ),
    );
    client.insert(
        "https://ir.example.test/earnings/q3-2026",
        HttpResponse::ok(
            r#"<title>Q3 earnings release</title><time datetime="2026-08-20"><p>Acme consolidated production scheduling under one platform.</p>"#,
        ),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());

    let nested = observations
        .iter()
        .find(|observation| observation.url() == Some("https://ir.example.test/earnings/q3-2026"))
        .expect("one-hop nested release should be discovered");
    assert_eq!(nested.material_kind(), SourceMaterialKind::Document);
    assert_eq!(
        nested.text(),
        "Acme consolidated production scheduling under one platform."
    );
    let index = observations
        .iter()
        .find(|observation| observation.url() == Some("https://ir.example.test/earnings/archive"))
        .expect("the archive index should remain a document lead");
    assert!(extract_evidence_candidate(index).is_none());
    assert!(
        observations
            .iter()
            .all(|observation| observation.url()
                != Some("https://evil.example.test/earnings/q3-2026"))
    );
}

#[test]
fn ir_deep_discovery_deduplicates_nested_links_and_enforces_global_cap() {
    let mut company = company();
    company.official_ir = Some("https://ir.example.test/investors".to_owned());
    let client = FixtureHttpClient::new();
    let direct_links = (0..8)
        .map(|index| format!("<a href=\"/earnings/archive-{index}\">Earnings archive {index}</a>"))
        .collect::<String>();
    client.insert(
        company.official_ir_url().expect("IR URL exists"),
        HttpResponse::ok(direct_links),
    );
    for index in 0..8 {
        client.insert(
            format!("https://ir.example.test/earnings/archive-{index}"),
            HttpResponse::ok(format!(
                "<a href=\"/earnings/q3-{index}\">Q3 release {index}</a><a href=\"/earnings/shared\">Shared release</a><a href=\"https://evil.example.test/q3-{index}\">Cross origin</a>"
            )),
        );
        client.insert(
            format!("https://ir.example.test/earnings/q3-{index}"),
            HttpResponse::ok(format!(
                "<title>Q3 release {index}</title><time datetime=\"2026-08-20\"><p>Acme reorganized production scheduling in release {index}.</p>"
            )),
        );
    }
    client.insert(
        "https://ir.example.test/earnings/shared",
        HttpResponse::ok(
            "<title>Shared release</title><time datetime=\"2026-08-20\"><p>Acme expanded production automation.</p>",
        ),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let documents = observations
        .iter()
        .filter(|observation| observation.material_kind() == SourceMaterialKind::Document)
        .collect::<Vec<_>>();
    let urls = documents
        .iter()
        .filter_map(|observation| observation.url())
        .collect::<std::collections::BTreeSet<_>>();

    assert_eq!(documents.len(), 12);
    assert_eq!(urls.len(), 12);
    assert_eq!(
        urls.iter()
            .filter(|url| url.ends_with("/earnings/shared"))
            .count(),
        1
    );
    assert!(urls
        .iter()
        .all(|url| url.starts_with("https://ir.example.test/")));
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

#[test]
fn document_metadata_prefers_published_metadata_over_modified_date() {
    let (_, date, _) = document_metadata(
        r#"<html><head>
            <meta property="article:published_time" content="2026-08-19T09:30:00Z">
            <meta property="article:modified_time" content="2026-08-25T09:30:00Z">
        </head><body><p>Acme changed an engineering workflow.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(date, NaiveDate::from_ymd_opt(2026, 8, 19));
}

#[test]
fn document_metadata_reads_bounded_site_specific_release_date_before_modified_date() {
    let (_, date, _) = document_metadata(
        r#"<html><head>
            <meta name="pwcReleaseDate" content="2026-01-16T10:35:00.000-03:00">
            <meta name="pwcLastModifiedDate" content="2026-06-23T13:20:19.535Z">
        </head><body><p>PwC deploys Microsoft Copilot at enterprise scale.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 16));
}

#[test]
fn document_metadata_reads_json_ld_and_iso_datetime() {
    let (_, date, _) = document_metadata(
        r#"<html><head><script type="application/ld+json">
            {"@context":"https://schema.org","datePublished":"2026-08-20T14:00:00Z"}
        </script></head><body><p>Acme launched a production platform.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(date, NaiveDate::from_ymd_opt(2026, 8, 20));
}

#[test]
fn document_metadata_rejects_malformed_dates_without_guessing() {
    let (_, date, _) = document_metadata(
        r#"<html><head>
            <meta property="article:published_time" content="2026-99-99">
            <script type="application/ld+json">
                {"dateModified":"2026-08-23T12:00:00Z"}
            </script>
        </head><body><p>Acme changed a production workflow.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(date, NaiveDate::from_ymd_opt(2026, 8, 23));
}

#[test]
fn discovered_document_retains_document_kind_without_promoting_entry_point() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert_eq!(observation.material_kind(), SourceMaterialKind::Document);
    assert_eq!(observation.document_kind(), Some(DocumentKind::Engineering));
}

#[test]
fn document_body_excludes_title_script_and_metadata_before_claim_extraction() {
    let (title, date, body) = document_metadata(
        r#"<html><head><title>Acme engineering update</title>
        <meta name="description" content="Acme adopted an agent workflow.">
        <script>window.claim = "Acme adopted an agent workflow.";</script></head>
        <body><p>Acme moved its engineering workflow to an agent-assisted scheduler.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(title, "Acme engineering update");
    assert_eq!(date, None);
    assert_eq!(
        body,
        "Acme moved its engineering workflow to an agent-assisted scheduler."
    );
}

#[test]
fn document_body_ignores_navigation_and_social_boilerplate_before_claim_extraction() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><nav><a>Skip to content</a><a>Share on Facebook</a></nav>
        <header>Company navigation</header>
        <div class="social-share"><p>Share this article on LinkedIn.</p></div>
        <div id="main-menu"><a>More navigation</a></div>
        <main><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></main>
        <aside>Share this update with your team.</aside>
        <footer>Privacy policy and cookie settings.</footer></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate = extract_evidence_candidate(&observation)
        .expect("substantive paragraph should remain a candidate");

    assert_eq!(
        candidate.concrete_change(),
        "Acme adopted an agent-assisted engineering workflow for production scheduling."
    );
}

#[test]
fn generic_architecture_description_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Our storage service exposes object storage, file systems, and block-device APIs, and these APIs are built on a horizontally scalable foundational block layer called Tectonic.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn title_only_document_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Acme adopted an agent-assisted engineering workflow.</title>
        <time datetime="2026-08-19"></time></head></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn body_sentence_with_change_and_production_signals_creates_a_bounded_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p>
        <p>The page also contains implementation details.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate = extract_evidence_candidate(&observation).expect("body claim should qualify");

    assert_eq!(
        candidate.concrete_change(),
        "Acme adopted an agent-assisted engineering workflow for production scheduling."
    );
    assert!(!candidate
        .concrete_change()
        .contains("implementation details"));
}

#[test]
fn dated_engineering_document_promotes_a_complete_claim() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate =
        extract_evidence_candidate(&observation).expect("dated engineering claim should qualify");
    let validated = validate_evidence_candidate(&candidate, cutoff())
        .expect("complete authoritative claim should validate");
    let fact = validated
        .to_normalized_fact(1)
        .expect("validated claim should normalize");

    assert_eq!(fact.status(), &FactStatus::Known);
    assert_eq!(
        fact.value(),
        Some("Acme adopted an agent-assisted engineering workflow for production scheduling.")
    );
}

#[test]
fn document_kind_context_is_retained_in_claim_provenance() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate = extract_evidence_candidate(&observation).expect("claim should qualify");
    let fact = validate_evidence_candidate(&candidate, cutoff())
        .expect("claim should validate")
        .to_normalized_fact(1)
        .expect("claim should normalize");

    assert!(fact
        .provenance()
        .source_field_or_passage()
        .contains("document_kind=engineering"));
}

#[test]
fn generic_careers_copy_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Careers areas</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Help transform our clients' data into tangible business value by analyzing information, communicating outcomes, and building trusted data foundations that enable responsible AI adoption across hybrid cloud environments.</p></body></html>"#,
        "https://ir.example.test/careers/areas",
    );

    assert_eq!(observation.document_kind(), Some(DocumentKind::Careers));
    assert!(extract_evidence_candidate(&observation).is_none());
    assert_eq!(
        normalize_source_observation(&observation, 1)
            .expect("generic Careers observation should remain a lead")
            .status(),
        &FactStatus::Unconfirmed
    );
}

#[test]
fn careers_role_marketing_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering roles</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Explore engineering roles that help clients adopt AI and cloud solutions across our global teams.</p></body></html>"#,
        "https://ir.example.test/careers/roles",
    );

    assert_eq!(observation.document_kind(), Some(DocumentKind::Careers));
    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn non_careers_hiring_language_does_not_change_candidate_boundary() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>We are hiring 200 engineers for our AI infrastructure team to expand production capacity.</p></body></html>"#,
        "https://ir.example.test/engineering/hiring",
    );

    assert_eq!(observation.document_kind(), Some(DocumentKind::Engineering));
    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn explicit_careers_hiring_change_becomes_a_regular_validated_fact() {
    let observation = document_observation_from_html(
        r#"<html><head><title>AI infrastructure hiring plan</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>We are hiring 200 engineers for our AI infrastructure team to expand production capacity.</p></body></html>"#,
        "https://ir.example.test/careers/areas",
    );

    let candidate = extract_evidence_candidate(&observation)
        .expect("explicit dated hiring change should qualify");
    let validated = validate_evidence_candidate(&candidate, cutoff())
        .expect("explicit hiring claim should validate");
    let fact = validated
        .to_normalized_fact(1)
        .expect("validated hiring claim should normalize");

    assert_eq!(validated.evidence_class(), EvidenceClass::ValidatedFact);
    assert_eq!(validated.structural_dimension(), None);
    assert_eq!(fact.status(), &FactStatus::Known);
    assert!(fact
        .provenance()
        .source_field_or_passage()
        .contains("document_kind=careers"));
}

#[test]
fn generic_or_non_actionable_document_remains_a_pending_lead() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Our storage service exposes object storage, file systems, and block-device APIs, and these APIs are built on a horizontally scalable foundational block layer called Tectonic.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
    assert_eq!(
        normalize_source_observation(&observation, 1)
            .expect("source observation should remain a lead")
            .status(),
        &FactStatus::Unconfirmed
    );
}

#[test]
fn heading_only_document_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><h1>Acme adopted an agent-assisted engineering workflow for production scheduling.</h1></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn production_sentence_without_a_change_action_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>The engineering platform serves customer requests.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn claim_extraction_skips_nonproduction_change_sentences_until_a_valid_claim() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title>
        <time datetime="2026-08-19"></time></head>
        <body><p>Acme adopted a new legal review policy.</p>
        <p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate =
        extract_evidence_candidate(&observation).expect("second body claim should qualify");

    assert_eq!(
        candidate.concrete_change(),
        "Acme adopted an agent-assisted engineering workflow for production scheduling."
    );
}

#[test]
fn document_without_effective_date_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title></head>
        <body><p>Acme adopted an agent-assisted engineering workflow for production scheduling.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
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

fn validated_claim(
    passage: &str,
) -> org_x::features::weekly_radar::runtime::evidence::ValidatedEvidence {
    let candidate = complete_candidate("2026-08-19", "engineering systems")
        .with_source_details("Enterprise change update", passage);
    validate_evidence_candidate(&candidate, cutoff()).unwrap()
}

#[test]
fn validated_structural_claims_receive_specific_dimensions() {
    let cases = [
        (
            "Acme reorganized its engineering teams and moved responsibility to one division.",
            StructuralDimension::Organization,
        ),
        (
            "Acme changed its engineering workflow and approval process for production scheduling.",
            StructuralDimension::Workflow,
        ),
        (
            "Acme deployed a production platform and consolidated storage infrastructure.",
            StructuralDimension::ProductionSystem,
        ),
        (
            "Acme increased GPU utilization and reduced serving latency for production workloads.",
            StructuralDimension::OperatingMetric,
        ),
    ];

    for (passage, expected_dimension) in cases {
        let validated = validated_claim(passage);
        assert_eq!(
            validated.evidence_class(),
            EvidenceClass::StructuralEvidence
        );
        assert_eq!(validated.structural_dimension(), Some(expected_dimension));
        assert_eq!(
            validated
                .to_normalized_fact(1)
                .unwrap()
                .structural_dimension(),
            Some(expected_dimension)
        );
    }
}

#[test]
fn metric_and_production_system_claims_are_not_organization_evidence() {
    for passage in [
        "Acme increased GPU utilization and reduced serving latency for production workloads.",
        "Acme deployed a production platform and consolidated storage infrastructure.",
    ] {
        assert_ne!(
            validated_claim(passage).structural_dimension(),
            Some(StructuralDimension::Organization)
        );
    }
}

#[test]
fn incomplete_structural_claim_cannot_pass_the_promotion_gate() {
    let candidate = complete_candidate("2026-08-19", "engineering workflow")
        .with_source_details("", "Acme reorganized its engineering teams and division.");

    assert_eq!(
        validate_evidence_candidate(&candidate, cutoff()).unwrap_err(),
        EvidenceValidationError::MissingRequiredField {
            field: "source_title"
        }
    );
}

#[test]
fn explicit_production_system_change_becomes_structural_evidence() {
    let candidate = complete_candidate("2026-08-19", "production scheduling").with_source_details(
        "Production scheduling update",
        "Acme consolidated production scheduling under one platform.",
    );

    let validated = validate_evidence_candidate(&candidate, cutoff()).unwrap();

    assert_eq!(
        validated.evidence_class(),
        EvidenceClass::StructuralEvidence
    );
    assert!(validated
        .to_normalized_fact(1)
        .unwrap()
        .kind()
        .starts_with("evidence_structural_change_"));
}

#[test]
fn generic_research_description_remains_a_regular_validated_fact() {
    let candidate = complete_candidate("2026-08-19", "research").with_source_details(
        "Model research update",
        "The research model shifted representation modeling for long-range graph topologies.",
    );

    let validated = validate_evidence_candidate(&candidate, cutoff()).unwrap();

    assert_eq!(validated.evidence_class(), EvidenceClass::ValidatedFact);
    assert!(validated
        .to_normalized_fact(1)
        .unwrap()
        .kind()
        .starts_with("evidence_official_material_"));
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

fn production_acceptance_document(
    company_id: &str,
    source_kind: SourceKind,
    document_kind: DocumentKind,
    source_uri: &str,
    title: &str,
    passage: &str,
    effective_date: &str,
) -> org_x::features::weekly_radar::runtime::SourceObservation {
    document_observation(DocumentObservationInput {
        company_id: company_id.to_owned(),
        kind: source_kind,
        tier: SourceTier::OfficialPrimary,
        url: source_uri.to_owned(),
        title: title.to_owned(),
        text: passage.to_owned(),
        status: SourceStatus::Known,
        status_reason: "2026-08-31 production acceptance fixture".to_owned(),
        document_kind,
        source_field_or_passage: "2026-08-31 production report fact".to_owned(),
        observed_at: Utc::now(),
        effective_date: Some(
            NaiveDate::parse_from_str(effective_date, "%Y-%m-%d")
                .expect("production acceptance date should parse"),
        ),
    })
}

#[test]
fn production_acceptance_structural_false_positives_are_not_promoted() {
    let cases = [
        (
            "amzn",
            SourceKind::Sec,
            DocumentKind::Filing,
            "https://www.sec.gov/Archives/edgar/data/1018724/000101872426000026/amzn-20260630.htm",
            "Amazon.com, Inc. Form 10-Q",
            "Financial Statements 3 Consolidated Statements of Cash Flows 3 Consolidated Statements of Operations 4 Consolidated Statements of Comprehensive Income 5 Consolidated Balance Sheets 6 Notes to Consolidated Financial Statements 7 Item&#160;2.",
            "2026-07-31",
        ),
        (
            "goog",
            SourceKind::Sec,
            DocumentKind::Filing,
            "https://www.sec.gov/Archives/edgar/data/1652044/000165204426000071/goog-20260630.htm",
            "Alphabet Inc. Form 10-Q",
            "FINANCIAL INFORMATION Item&#160;1 Financial Statements 4 Consolidated Balance Sheets - December&#160;31, 2025 and June&#160;30, 2026 4 Consolidated Statements of Income - Three and Six Months Ended June 30 , 2025 and 2026 5 Consolidated Statements of Comprehensive Income.",
            "2026-07-23",
        ),
        (
            "wmt",
            SourceKind::Sec,
            DocumentKind::Filing,
            "https://www.sec.gov/Archives/edgar/data/104169/000010416926000111/wmt-20260604.htm",
            "Walmart Inc. Proxy Statement",
            "The votes on this proposal were as follows: For Against Abstain Broker Non-Votes 278,449,353 6,174,725,696 77,400,387 633,971,647 Finally, the Company's shareholders then voted upon and rejected a shareholder proposal requesting a report on the matter.",
            "2026-06-05",
        ),
        (
            "msft",
            SourceKind::Sec,
            DocumentKind::Earnings,
            "https://www.sec.gov/Archives/edgar/data/789019/000119312526323632/msft-20260729.htm",
            "Microsoft Corporation Form 8-K",
            "Results of Operations and Financial Condition On July 29, 2026, Microsoft Corporation issued a press release announcing its financial results for the fiscal quarter and year ended June 30, 2026.",
            "2026-07-29",
        ),
    ];

    for (company_id, source_kind, document_kind, source_uri, title, passage, effective_date) in
        cases
    {
        let observation = production_acceptance_document(
            company_id,
            source_kind,
            document_kind,
            source_uri,
            title,
            passage,
            effective_date,
        );
        let Some(candidate) = extract_evidence_candidate(&observation) else {
            continue;
        };
        let validated = validate_evidence_candidate(
            &candidate,
            NaiveDate::from_ymd_opt(2026, 8, 31).expect("cutoff should be valid"),
        )
        .expect("production passage should remain a validated fact");
        assert_eq!(validated.evidence_class(), EvidenceClass::ValidatedFact);
        assert_eq!(validated.structural_evidence_contract(), None);
        assert!(validated
            .to_normalized_fact(1)
            .expect("validated fact should normalize")
            .kind()
            .starts_with("evidence_official_material_"));
    }
}

#[test]
fn production_customer_and_partner_subjects_are_not_promoted_as_microsoft_structure() {
    let cases = [
        (
            "Hertz",
            "https://www.microsoft.com/en/customers/story/25989-hertz-microsoft-power-platform",
            "Hertz customer story",
            "As part of its technology modernization strategy, Hertz has begun developing low-code applications and agents with Power Platform.",
        ),
        (
            "PwC",
            "https://www.pwc.com/us/en/services/consulting/engineering-ai.html",
            "PwC Engineering and AI",
            "PwC’s Engineering and AI builders help organizations modernize core platforms and translate AI, cloud, and data innovation into scalable business outcomes.",
        ),
        (
            "Atos Group",
            "https://www.atosgroup.com/en/press/atos-group-and-microsoft-expand-strategic-collaboration-scale-secure-agentic-ai-across-atos",
            "Atos Group and Microsoft strategic collaboration",
            "Atos Group becomes the first French Global System Integrator to deploy Microsoft 365 Copilot and one of the largest to roll out secure agentic AI across its workforce.",
        ),
    ];

    for (expected_subject, source_uri, title, passage) in cases {
        let observation = production_acceptance_document(
            "msft",
            SourceKind::OfficialResearch,
            DocumentKind::ProductPlatform,
            source_uri,
            title,
            passage,
            "2026-08-31",
        );
        let candidate = extract_evidence_candidate(&observation)
            .expect("customer or partner production sentence should be extracted");
        let validated = validate_evidence_candidate(
            &candidate,
            NaiveDate::from_ymd_opt(2026, 8, 31).expect("cutoff should be valid"),
        )
        .expect("customer or partner production sentence should validate");

        assert_eq!(validated.attribution().assessed_company(), "msft");
        assert_eq!(validated.attribution().subject_company(), expected_subject);
        assert_eq!(validated.evidence_class(), EvidenceClass::ValidatedFact);
        assert_eq!(validated.structural_dimension(), None);
        assert_eq!(validated.structural_evidence_contract(), None);
    }
}

#[test]
fn structural_contract_rejects_external_subject_attribution() {
    let result = StructuralEvidenceContract::new(
        "msft",
        "Microsoft",
        "Hertz has begun developing low-code applications and agents with Power Platform.",
        StructuralDimension::ProductionSystem,
        "low-code applications and agents",
        "manual application development",
        "low-code applications and agents with Power Platform",
        NaiveDate::from_ymd_opt(2026, 8, 31).expect("date should be valid"),
        "https://www.microsoft.com/en/customers/story/25989-hertz-microsoft-power-platform",
        "official_company_material",
        "production_system:low-code applications and agents",
        true,
    );

    assert!(result.is_err());
}

#[test]
fn structural_report_requires_a_valid_semantic_contract() {
    let passage = "Financial Statements 3 Consolidated Statements of Cash Flows 3 Consolidated Statements of Operations 4 Consolidated Statements of Comprehensive Income 5 Consolidated Balance Sheets 6 Notes to Consolidated Financial Statements 7 Item&#160;2.";
    let fact = NormalizedFact::new_with_structural_dimension(
        "amzn",
        "evidence_structural_change_001",
        passage,
        Some(StructuralDimension::OperatingMetric),
        FactStatus::Known,
        Confidence::High,
        Provenance::new(
            "https://www.sec.gov/Archives/edgar/data/1018724/000101872426000026/amzn-20260630.htm",
            passage,
            Utc::now(),
            Some(NaiveDate::from_ymd_opt(2026, 7, 31).expect("date should be valid")),
        )
        .expect("production passage provenance should be valid"),
    )
    .expect("dimension-only fixture should be constructible for the boundary test");
    let mut input = input_with_metrics(ResearchMetrics::new(1, 1, 1, 0, 0));
    input
        .add_fact(fact)
        .expect("dimension-only fixture should be retained as a fact");

    let report = render_report_in_language(&input, ReportLanguage::Chinese);

    assert!(!report.markdown().contains("## 结构性证据"));
    assert!(report.markdown().contains("## 已验证事实"));
    assert!(report.markdown().contains(passage));
}

fn input_with_metrics(metrics: ResearchMetrics) -> RuntimeReportInput {
    let mut input = RuntimeReportInput::new("2026-08-25").expect("report date should be valid");
    input.set_research_metrics(metrics);
    input
}

fn input_with_raw_and_validated_evidence() -> RuntimeReportInput {
    let mut input = RuntimeReportInput::new("2026-08-25").expect("report date should be valid");
    input
        .add_fact(
            NormalizedFact::new(
                "acme",
                "revenue",
                "123000000",
                FactStatus::Known,
                Confidence::High,
                Provenance::new(
                    "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
                    "facts.revenue",
                    Utc::now(),
                    Some(NaiveDate::from_ymd_opt(2026, 8, 19).expect("date should be valid")),
                )
                .expect("SEC provenance should be valid"),
            )
            .expect("known SEC fact should be valid"),
        )
        .expect("SEC fact should be added");
    input
        .add_fact(
            NormalizedFact::new(
                "acme",
                "evidence_official_material_001",
                "Acme consolidated production scheduling under one platform.",
                FactStatus::Known,
                Confidence::High,
                Provenance::new(
                    "https://ir.example.test/organization/update",
                    "Acme consolidated production scheduling under one platform.",
                    Utc::now(),
                    Some(NaiveDate::from_ymd_opt(2026, 8, 19).expect("date should be valid")),
                )
                .expect("validated evidence provenance should be valid"),
            )
            .expect("validated evidence should be valid"),
        )
        .expect("validated evidence should be added");
    input
}

#[test]
fn report_separates_validated_facts_from_structural_evidence() {
    let mut input = input_with_raw_and_validated_evidence();
    input
        .add_fact(
            NormalizedFact::new_with_structural_dimension(
                "acme",
                "evidence_structural_change_002",
                "Acme deployed an agent-assisted scheduler to production.",
                Some(StructuralDimension::ProductionSystem),
                FactStatus::Known,
                Confidence::High,
                Provenance::new(
                    "https://ir.example.test/organization/update",
                    "Acme deployed an agent-assisted scheduler to production.",
                    Utc::now(),
                    Some(NaiveDate::from_ymd_opt(2026, 8, 20).expect("date should be valid")),
                )
                .expect("structural evidence provenance should be valid"),
            )
            .expect("structural evidence should be valid")
            .with_structural_evidence_contract(
                StructuralEvidenceContract::new(
                    "acme",
                    "acme",
                    "Acme deployed an agent-assisted scheduler to production.",
                    StructuralDimension::ProductionSystem,
                    "agent-assisted scheduler",
                    "manual scheduling",
                    "agent-assisted scheduler in production",
                    NaiveDate::from_ymd_opt(2026, 8, 20).expect("date should be valid"),
                    "https://ir.example.test/organization/update",
                    "official_company_material",
                    "production_system:agent-assisted scheduler",
                    true,
                )
                .expect("structural evidence contract should be valid"),
            )
            .expect("structural evidence should retain its contract"),
        )
        .expect("structural evidence should be added");
    input.set_research_metrics(
        ResearchMetrics::new(9, 10, 2, 10, 1)
            .with_structural_evidence(1)
            .with_sec_health(20, 20, 16, 14),
    );

    let report = render_report_in_language(&input, ReportLanguage::Chinese);

    assert!(report.markdown().contains("## 已验证事实"));
    assert!(report.markdown().contains("## 结构性证据"));
    assert!(report.markdown().contains("本周新增已验证事实：2"));
    assert!(report.markdown().contains("本周新增结构性证据：1"));
    assert!(report.markdown().contains("SEC 可用事实"));
    let validated_section = report
        .markdown()
        .split("## 已验证事实")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("validated fact section should be rendered");
    assert!(!validated_section.contains("deployed an agent-assisted scheduler"));
    let structural_section = report
        .markdown()
        .split("## 结构性证据")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("structural evidence section should be rendered");
    assert!(structural_section.contains("deployed an agent-assisted scheduler"));
}

#[test]
fn degraded_report_separates_evidence_and_source_availability_counts() {
    let input = input_with_metrics(ResearchMetrics::new(9, 10, 0, 10, 50));
    let report = render_report_in_language(&input, ReportLanguage::Chinese);

    assert!(report.markdown().contains("本周新增已验证事实：0"));
    assert!(report.markdown().contains("来源可用性确认：9"));
    assert!(report.markdown().contains("待验证线索：10"));
    assert!(report.markdown().contains("关键数据源不可用：50"));
    assert!(report.markdown().contains("数据不足"));
    assert!(!report.markdown().contains("Investor Relations"));
    assert!(report.snapshot_json().contains("research_metrics"));
}

#[test]
fn confirmed_information_contains_only_validated_evidence() {
    let input = input_with_raw_and_validated_evidence();

    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let confirmed_section = report
        .markdown()
        .split("## 已验证事实")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("validated evidence section should be rendered");

    assert!(confirmed_section.contains("Acme consolidated production scheduling"));
    assert!(!confirmed_section.contains("123000000"));
    assert!(report
        .markdown()
        .contains("共 1 条已验证事实，其中 0 条结构性证据"));
    assert!(report.markdown().contains("已知事实：2 条"));
}

#[test]
fn localized_reports_keep_validated_evidence_separate_from_known_facts() {
    let input = input_with_raw_and_validated_evidence();

    let japanese = render_report_in_language(&input, ReportLanguage::Japanese);
    assert!(japanese.markdown().contains("## 検証済み事実"));
    assert!(japanese
        .markdown()
        .contains("Acme consolidated production scheduling"));
    let japanese_confirmed_section = japanese
        .markdown()
        .split("## 検証済み事実")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("Japanese validated evidence section should be rendered");
    assert!(!japanese_confirmed_section.contains("123000000"));
    assert!(japanese.markdown().contains("1 件の検証済み事実"));
    assert!(japanese.markdown().contains("既知の事実：2 件"));

    let english = render_report_in_language(&input, ReportLanguage::English);
    assert!(english.markdown().contains("## Validated Facts"));
    assert!(english
        .markdown()
        .contains("Acme consolidated production scheduling"));
    let english_confirmed_section = english
        .markdown()
        .split("## Validated Facts")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("English validated evidence section should be rendered");
    assert!(!english_confirmed_section.contains("123000000"));
    assert!(english.markdown().contains("1 validated facts"));
    assert!(english.markdown().contains("Known facts: 2"));
}

#[test]
fn localized_reports_render_document_kind_counts_without_ranking() {
    let metrics = ResearchMetrics::new(9, 10, 1, 9, 2).with_document_kind_counts(BTreeMap::from([
        ("engineering".to_owned(), 2),
        ("earnings".to_owned(), 1),
    ]));
    let input = input_with_metrics(metrics);

    let english = render_report_in_language(&input, ReportLanguage::English);
    assert!(english
        .markdown()
        .contains("Document kinds: earnings=1, engineering=2"));
    assert!(!english.markdown().contains("Top 5"));

    let chinese = render_report_in_language(&input, ReportLanguage::Chinese);
    assert!(chinese.markdown().contains("文档类型"));
    assert!(chinese.markdown().contains("工程"));
}

#[test]
fn localized_reports_render_structural_dimensions_and_legacy_fallback() {
    let dimensions = [
        (StructuralDimension::Organization, "Organization claim"),
        (StructuralDimension::Workflow, "Workflow claim"),
        (
            StructuralDimension::ProductionSystem,
            "Production system claim",
        ),
        (
            StructuralDimension::OperatingMetric,
            "Operating metric claim",
        ),
    ];
    let mut input = RuntimeReportInput::new("2026-08-25").unwrap();
    for (index, (dimension, value)) in dimensions.into_iter().enumerate() {
        input
            .add_fact(
                NormalizedFact::new_with_structural_dimension(
                    "acme",
                    format!("evidence_structural_change_{:03}", index + 1),
                    value,
                    Some(dimension),
                    FactStatus::Known,
                    Confidence::High,
                    Provenance::new(
                        format!("https://ir.example.test/dimension/{index}"),
                        value,
                        Utc::now(),
                        Some(NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()),
                    )
                    .unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
    }
    input
        .add_fact(
            NormalizedFact::new(
                "acme",
                "evidence_structural_change_005",
                "Legacy structural claim",
                FactStatus::Known,
                Confidence::High,
                Provenance::new(
                    "https://ir.example.test/legacy",
                    "Legacy structural claim",
                    Utc::now(),
                    Some(NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

    let chinese = render_report_in_language(&input, ReportLanguage::Chinese);
    assert!(chinese.markdown().contains("组织变化"));
    assert!(chinese.markdown().contains("工作流变化"));
    assert!(chinese.markdown().contains("生产系统变化"));
    assert!(chinese.markdown().contains("运营指标变化"));
    assert!(chinese.markdown().contains("结构性证据"));
    assert!(chinese
        .snapshot_json()
        .contains("\"structural_dimension\": \"operating_metric\""));

    let japanese = render_report_in_language(&input, ReportLanguage::Japanese);
    assert!(japanese.markdown().contains("組織変化"));
    assert!(japanese.markdown().contains("ワークフロー変化"));
    assert!(japanese.markdown().contains("生産システム変化"));
    assert!(japanese.markdown().contains("運用指標変化"));
    assert!(japanese.markdown().contains("構造的証拠"));

    let english = render_report_in_language(&input, ReportLanguage::English);
    assert!(english.markdown().contains("Organizational change"));
    assert!(english.markdown().contains("Workflow change"));
    assert!(english.markdown().contains("Production-system change"));
    assert!(english.markdown().contains("Operating-metric change"));
    assert!(english.markdown().contains("Structural evidence"));
}

#[test]
fn localized_reports_keep_the_same_metric_values() {
    let input = input_with_metrics(ResearchMetrics::new(9, 10, 1, 9, 50));

    for language in [
        ReportLanguage::Chinese,
        ReportLanguage::Japanese,
        ReportLanguage::English,
    ] {
        let report = render_report_in_language(&input, language);
        assert!(report.markdown().contains("9"));
        assert!(report.markdown().contains("10"));
        assert!(report.markdown().contains("50"));
    }
}
