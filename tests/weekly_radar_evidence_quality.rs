use chrono::Utc;
use org_x::features::weekly_radar::runtime::config::CompanyConfig;
use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpResponse};
use org_x::features::weekly_radar::runtime::model::{FactStatus, RuntimeReportInput};
use org_x::features::weekly_radar::runtime::normalize_source_observation;
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
