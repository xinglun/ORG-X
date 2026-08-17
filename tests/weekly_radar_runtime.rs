use org_x::features::weekly_radar::runtime::config::CompanySourceRegistry;
use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpClient, HttpResponse};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};

#[test]
fn registry_validation_preserves_optional_source_semantics() {
    let registry = CompanySourceRegistry::from_json(
        r#"
        {
          "version": 1,
          "companies": [
            {
              "id": "acme",
              "name": "Acme Corporation",
              "ticker": "ACME",
              "sec_cik": "0001234567",
              "official_ir": "https://example.test/investors",
              "careers": "https://example.test/careers",
              "engineering_ai_blog": "https://example.test/engineering",
              "greenhouse_board": "acme",
              "lever_site": "acme"
            },
            {
              "id": "beta",
              "name": "Beta Systems",
              "ticker": "BETA"
            }
          ]
        }
        "#,
    )
    .expect("valid registry fixture should load");

    assert_eq!(registry.version(), 1);
    assert_eq!(registry.companies().len(), 2);
    assert_eq!(registry.company("acme").unwrap().ticker(), "ACME");
    assert_eq!(
        registry.company("acme").unwrap().official_ir_url(),
        Some("https://example.test/investors")
    );
    assert_eq!(registry.company("beta").unwrap().sec_cik(), None);
    assert_eq!(registry.company("beta").unwrap().greenhouse_board(), None);

    let invalid = CompanySourceRegistry::from_json(
        r#"{
          "version": 1,
          "companies": [
            {"id": "duplicate", "name": "One", "ticker": "ONE"},
            {"id": "duplicate", "name": "Two", "ticker": "TWO"}
          ]
        }"#,
    );
    assert!(invalid.is_err(), "duplicate identities must be rejected");
}

#[test]
fn normalized_fact_retains_status_confidence_and_full_provenance() {
    let provenance = Provenance::from_rfc3339(
        "https://example.test/filing/2026",
        "facts.revenue",
        "2026-08-17T00:00:00Z",
        Some("2026-06-30"),
    )
    .expect("fixture provenance should be valid");
    let fact = NormalizedFact::new(
        "acme",
        "revenue",
        "123456",
        FactStatus::Known,
        Confidence::High,
        provenance.clone(),
    )
    .expect("fixture fact should be valid");

    assert_eq!(fact.company_id(), "acme");
    assert_eq!(fact.kind(), "revenue");
    assert_eq!(fact.value(), Some("123456"));
    assert_eq!(fact.status(), &FactStatus::Known);
    assert_eq!(fact.confidence(), &Confidence::High);
    assert_eq!(fact.provenance(), &provenance);
    assert_eq!(
        fact.provenance().source_uri(),
        "https://example.test/filing/2026"
    );
    assert_eq!(fact.provenance().source_field_or_passage(), "facts.revenue");
    assert_eq!(
        fact.provenance().effective_date().unwrap().to_string(),
        "2026-06-30"
    );

    let mut input = RuntimeReportInput::new("2026-08-17").expect("as-of should be valid");
    input.add_fact(fact).expect("fact should be retained");
    input
        .add_source_coverage(SourceCoverage::new("sec", 1, 1).expect("coverage should be valid"))
        .expect("coverage should be retained");
    assert_eq!(input.facts().len(), 1);
    assert_eq!(input.source_coverage().len(), 1);
}

#[test]
fn injected_fixture_http_client_returns_response_and_records_headers() {
    let client = FixtureHttpClient::new();
    client.insert(
        "https://example.test/facts",
        HttpResponse::ok(r#"{"status":"ok"}"#),
    );

    let response = client
        .get(
            "https://example.test/facts",
            &[("Accept".to_owned(), "application/json".to_owned())],
        )
        .expect("fixture response should be returned");

    assert_eq!(response.status(), 200);
    assert_eq!(response.body(), r#"{"status":"ok"}"#);
    assert_eq!(client.requests().len(), 1);
    assert_eq!(client.requests()[0].url(), "https://example.test/facts");
    assert_eq!(
        client.requests()[0].headers(),
        &[("Accept".to_owned(), "application/json".to_owned())]
    );
}
