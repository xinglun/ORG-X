use chrono::{DateTime, Utc};
use org_x::features::weekly_radar::runtime::config::CompanyConfig;
use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpResponse};
use org_x::features::weekly_radar::runtime::model::{
    CompanyIdentity, RuntimeReportInput, SourceCoverage, SourceFailure,
};
use org_x::features::weekly_radar::runtime::report::render_report;
use org_x::features::weekly_radar::runtime::sec::SecClient;
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, SourceKind, SourceStatus,
};

fn observed_at() -> DateTime<Utc> {
    "2026-08-24T00:00:00Z"
        .parse()
        .expect("fixture timestamp is valid")
}

fn company_with_official_source() -> CompanyConfig {
    CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        Some("0001234567".to_owned()),
        Some("https://example.test/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .expect("source fixture company should be valid")
}

#[test]
fn source_status_taxonomy_keeps_not_applicable_distinct() {
    assert_eq!(SourceStatus::Known.as_str(), "KNOWN");
    assert_eq!(SourceStatus::Unavailable.as_str(), "UNAVAILABLE");
    assert_eq!(SourceStatus::NotConfigured.as_str(), "NOT_CONFIGURED");
    assert_eq!(SourceStatus::NotApplicable.as_str(), "NOT_APPLICABLE");
    assert_eq!(SourceStatus::DiscoveryOnly.as_str(), "DISCOVERY_ONLY");
}

#[test]
fn source_collection_distinguishes_not_configured_and_not_applicable() {
    let company = CompanyConfig::new(
        "beta",
        "Beta Systems",
        "BETA",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("source-free company should be valid");
    let client = FixtureHttpClient::new();

    let observations = collect_configured_sources(&company, &client, observed_at());
    let official = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("official source state should be retained");
    let discovery = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Gdelt)
        .expect("GDELT applicability should be explicit");

    assert_eq!(official.status(), SourceStatus::NotConfigured);
    assert_eq!(discovery.status(), SourceStatus::NotApplicable);
    assert_eq!(discovery.status_reason(), "no configured primary source");
    assert!(client.requests().is_empty());
}

#[test]
fn configured_source_failure_has_redacted_reason_and_no_response_body() {
    let company = company_with_official_source();
    let client = FixtureHttpClient::new();
    client.insert(
        company.official_ir_url().expect("official URL exists"),
        HttpResponse::new(503, "secret-body token=do-not-persist"),
    );

    let observation = collect_configured_sources(&company, &client, observed_at())
        .into_iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("configured official source should produce a state");
    let serialized = serde_json::to_string(&observation).expect("observation serializes");

    assert_eq!(observation.status(), SourceStatus::Unavailable);
    assert_eq!(
        observation.status_reason(),
        "official page request unavailable"
    );
    assert!(!observation.status_reason().contains("secret"));
    assert!(!serialized.contains("secret-body"));
    assert!(!serialized.contains("do-not-persist"));
}

#[test]
fn sec_failure_display_is_safe_for_source_status_binding() {
    let company = company_with_official_source();
    let client = FixtureHttpClient::new();
    client.insert(
        "https://data.sec.gov/submissions/CIK0001234567.json",
        HttpResponse::new(503, "secret response body"),
    );

    let evidence = SecClient::collect(&company, &client, "ORG-X test contact@example.test")
        .expect("independent SEC stage failures should retain a partial result");
    let failure = evidence
        .failures()
        .iter()
        .find(|failure| failure.stage() == "submissions")
        .expect("submissions failure should be retained");
    assert_eq!(failure.reason(), "HTTP response unavailable");
    assert!(!failure.reason().contains("secret"));
}

#[test]
fn report_exposes_not_applicable_without_counting_it_as_unavailable() {
    let mut input = RuntimeReportInput::new("2026-08-24").expect("date is valid");
    input
        .add_company(CompanyIdentity::new("beta", "Beta Systems", "BETA").unwrap())
        .unwrap();
    input
        .add_source_coverage(SourceCoverage::new_with_states("gdelt", 1, 0, 0, 1).unwrap())
        .unwrap();
    input
        .add_source_failure(
            SourceFailure::new("sec", "beta", "HTTP response body could not be read").unwrap(),
        )
        .unwrap();

    let report = render_report(&input);

    assert!(report.markdown().contains("不适用"));
    assert!(report.snapshot_json().contains("\"not_applicable\": 1"));
    assert!(report
        .snapshot_json()
        .contains("HTTP response body could not be read"));
}
