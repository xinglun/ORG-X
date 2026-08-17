use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use org_x::features::weekly_radar::runtime::config::CompanySourceRegistry;
use org_x::features::weekly_radar::runtime::error::RuntimeError;
use org_x::features::weekly_radar::runtime::http::{
    FixtureHttpClient, HttpClient, HttpResponse, UreqHttpClient,
};
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
fn non_confirmed_fact_statuses_cannot_retain_concrete_values() {
    let provenance = Provenance::from_rfc3339(
        "https://example.test/filing/2026",
        "facts.revenue",
        "2026-08-17T00:00:00Z",
        Some("2026-06-30"),
    )
    .expect("fixture provenance should be valid");

    for status in [
        FactStatus::Unknown,
        FactStatus::Unavailable,
        FactStatus::Unconfirmed,
    ] {
        let fact = NormalizedFact::new(
            "acme",
            "revenue",
            "must-not-leak",
            status,
            Confidence::Unknown,
            provenance.clone(),
        )
        .expect("fact should normalize its non-confirmed value away");

        assert_eq!(fact.status(), &status);
        assert_eq!(fact.value(), None);
    }
}

#[test]
fn confirmed_fact_without_value_is_rejected_by_constructor() {
    let provenance = Provenance::from_rfc3339(
        "https://example.test/filing/2026",
        "facts.revenue",
        "2026-08-17T00:00:00Z",
        Some("2026-06-30"),
    )
    .expect("fixture provenance should be valid");

    let result = NormalizedFact::without_value(
        "acme",
        "revenue",
        FactStatus::Known,
        Confidence::High,
        provenance,
    );

    assert!(matches!(result, Err(RuntimeError::InvalidModel { .. })));
}

#[test]
fn deserializing_non_confirmed_fact_clears_concrete_value() {
    let fact: NormalizedFact = serde_json::from_str(
        r#"
        {
          "company_id": "acme",
          "kind": "revenue",
          "value": "must-not-survive-deserialization",
          "status": "UNKNOWN",
          "confidence": "UNKNOWN",
          "provenance": {
            "source_uri": "https://example.test/filing/2026",
            "source_field_or_passage": "facts.revenue",
            "retrieved_at": "2026-08-17T00:00:00Z",
            "effective_date": "2026-06-30"
          }
        }
        "#,
    )
    .expect("valid normalized fact JSON should deserialize");

    assert_eq!(fact.status(), &FactStatus::Unknown);
    assert_eq!(fact.value(), None);
}

#[test]
fn deserializing_malformed_normalized_fact_fails_cleanly() {
    let result = serde_json::from_str::<NormalizedFact>(
        r#"
        {
          "company_id": "",
          "kind": "revenue",
          "value": "123",
          "status": "KNOWN",
          "confidence": "HIGH",
          "provenance": {
            "source_uri": "https://example.test/filing/2026",
            "source_field_or_passage": "facts.revenue",
            "retrieved_at": "2026-08-17T00:00:00Z",
            "effective_date": null
          }
        }
        "#,
    );

    assert!(
        matches!(result, Err(ref error) if error.to_string().contains("company ID")),
        "blank fact identity must be rejected by normalized-fact validation: {result:?}"
    );
}

#[test]
fn deserializing_confirmed_fact_with_null_or_missing_value_is_rejected() {
    let value_variants = [r#""value": null,"#, r#""#];

    for value_field in value_variants {
        let json = format!(
            r#"
            {{
              "company_id": "acme",
              "kind": "revenue",
              {value_field}
              "status": "KNOWN",
              "confidence": "HIGH",
              "provenance": {{
                "source_uri": "https://example.test/filing/2026",
                "source_field_or_passage": "facts.revenue",
                "retrieved_at": "2026-08-17T00:00:00Z",
                "effective_date": "2026-06-30"
              }}
            }}
            "#
        );

        let result = serde_json::from_str::<NormalizedFact>(&json);
        assert!(
            matches!(result, Err(ref error) if error.to_string().contains("value")),
            "confirmed fact JSON without a value must be rejected: {result:?}"
        );
    }
}

#[test]
fn fact_status_labels_are_provider_neutral_and_confirmed_is_human_facing() {
    assert_eq!(FactStatus::Known.as_str(), "CONFIRMED");
    assert_eq!(FactStatus::Unknown.as_str(), "UNKNOWN");
    assert_eq!(FactStatus::Unavailable.as_str(), "UNAVAILABLE");
    assert_eq!(FactStatus::Unconfirmed.as_str(), "UNCONFIRMED");
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

#[test]
fn http_errors_do_not_retain_token_or_chat_id_values() {
    let token = "super-secret-token";
    let chat_id = "super-secret-chat";
    let url = format!(
        "https://example.test/bot{token}/sendMessage?chat_id={chat_id}&token={token}#fragment"
    );
    let error = FixtureHttpClient::new()
        .get(&url, &[])
        .expect_err("missing fixture should return a typed error");

    assert_eq!(error, RuntimeError::FixtureMissing);
    assert!(!error.to_string().contains(token));
    assert!(!error.to_string().contains(chat_id));
    assert!(!format!("{error:?}").contains(token));
    assert!(!format!("{error:?}").contains(chat_id));
}

#[test]
fn public_http_errors_are_unit_variants_without_secret_bearing_diagnostics() {
    let token = "super-secret-token";
    let chat_id = "super-secret-chat";
    let errors = [
        RuntimeError::HttpRequest,
        RuntimeError::HttpResponse,
        RuntimeError::FixtureMissing,
        RuntimeError::FixtureState,
    ];

    for error in errors {
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(!display.contains(token));
        assert!(!display.contains(chat_id));
        assert!(!debug.contains(token));
        assert!(!debug.contains(chat_id));
    }
}

#[test]
fn ureq_client_configures_finite_connect_read_write_and_overall_timeouts() {
    let timeouts = UreqHttpClient::new().timeouts();

    assert!(timeouts.connect() > Duration::ZERO);
    assert!(timeouts.read() > Duration::ZERO);
    assert!(timeouts.write() > Duration::ZERO);
    assert!(timeouts.overall() > Duration::ZERO);
}

#[test]
fn fixture_and_ureq_return_non_success_statuses_as_http_responses() {
    let body = "temporarily unavailable";
    let listener = TcpListener::bind("127.0.0.1:0").expect("local listener should bind");
    let address = listener
        .local_addr()
        .expect("local listener should expose an address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("local client should connect");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        write!(
            stream,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("local response should be written");
    });
    let url = format!("http://{address}/fixture");

    let fixture_response = FixtureHttpClient::with_response(&url, HttpResponse::new(503, body))
        .get(&url, &[])
        .expect("fixture status should be returned as a response");
    assert_eq!(fixture_response.status(), 503);
    assert_eq!(fixture_response.body(), body);

    let ureq_response = UreqHttpClient::new()
        .get(&url, &[])
        .expect("ureq status should be returned as a response");
    assert_eq!(ureq_response.status(), 503);
    assert_eq!(ureq_response.body(), body);

    server.join().expect("local server should finish");
}
