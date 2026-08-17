use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use chrono::NaiveDate;
use org_x::features::weekly_radar::runtime::config::{CompanyConfig, CompanySourceRegistry};
use org_x::features::weekly_radar::runtime::error::RuntimeError;
use org_x::features::weekly_radar::runtime::http::{
    FixtureHttpClient, HttpClient, HttpResponse, UreqHttpClient,
};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};
use org_x::features::weekly_radar::runtime::rules::extract_employee_count;
use org_x::features::weekly_radar::runtime::sec::SecClient;

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

fn sec_test_company() -> CompanyConfig {
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

#[test]
fn sec_collects_annual_aliases_and_sends_user_agent() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let user_agent = "ORG-X weekly-radar test contact@example.test";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(r#"{"filings":{"recent":{}}}"#),
    );
    client.insert(
        facts_url,
        HttpResponse::ok(
            r#"
            {
              "facts": {
                "us-gaap": {
                  "RevenueFromContractWithCustomerExcludingAssessedTax": {
                    "units": {"USD": [
                      {"start":"2023-01-01","end":"2023-12-31","val":90,"accn":"000123456724000001","fy":2023,"fp":"FY","form":"10-K","filed":"2024-02-15"},
                      {"start":"2024-01-01","end":"2024-09-30","val":999,"accn":"000123456724000002","fy":2024,"fp":"Q3","form":"10-Q","filed":"2024-11-01"},
                      {"start":"2024-01-01","end":"2024-12-31","val":100,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                    ]}
                  },
                  "OperatingIncomeLoss": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":20,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "NetIncomeLoss": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":15,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "NetCashProvidedByUsedInOperatingActivities": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":30,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "PaymentsToAcquirePropertyPlantAndEquipment": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":5,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "ResearchAndDevelopmentExpense": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":7,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "ShareBasedCompensation": {"units": {"USD": [
                    {"start":"2024-01-01","end":"2024-12-31","val":3,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}},
                  "EntityNumberOfEmployees": {"units": {"employees": [
                    {"end":"2023-12-31","val":900,"accn":"000123456724000001","fy":2023,"fp":"FY","form":"10-K","filed":"2024-02-15"},
                    {"end":"2024-12-31","val":1000,"accn":"000123456725000001","fy":2024,"fp":"FY","form":"10-K","filed":"2025-02-15"}
                  ]}
                }
              }
            }
            }
            "#,
        ),
    );

    let evidence = SecClient::collect(&company, &client, user_agent)
        .expect("Company Facts fixture should collect");

    let requests = client.requests();
    for fact in evidence.facts() {
        let request = requests
            .iter()
            .find(|request| request.url() == fact.provenance().source_uri())
            .expect("each fact source should have a captured request");
        assert_eq!(
            request.headers(),
            &[("User-Agent".to_owned(), user_agent.to_owned())]
        );
    }
    assert_eq!(evidence.facts().len(), 8);
    assert_eq!(evidence.fact("revenue").unwrap().value(), Some("100"));
    assert_eq!(
        evidence.fact("operating_income").unwrap().value(),
        Some("20")
    );
    assert_eq!(evidence.fact("net_income").unwrap().value(), Some("15"));
    assert_eq!(
        evidence.fact("operating_cash_flow").unwrap().value(),
        Some("30")
    );
    assert_eq!(evidence.fact("capex").unwrap().value(), Some("5"));
    assert_eq!(evidence.fact("r_and_d").unwrap().value(), Some("7"));
    assert_eq!(evidence.fact("sbc").unwrap().value(), Some("3"));
    assert_eq!(
        evidence.fact("employee_count").unwrap().value(),
        Some("1000")
    );
    assert_eq!(
        evidence
            .fact("revenue")
            .unwrap()
            .provenance()
            .effective_date(),
        NaiveDate::from_ymd_opt(2024, 12, 31).as_ref()
    );
}

fn collect_revenue_alias_fixture(
    second_value: i64,
) -> org_x::features::weekly_radar::runtime::sec::CompanyEvidence {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(r#"{"filings":{"recent":{}}}"#),
    );
    client.insert(
        facts_url,
        HttpResponse::ok(&format!(
            r#"
            {{
              "facts": {{
                "us-gaap": {{
                  "RevenueFromContractWithCustomerExcludingAssessedTax": {{"units": {{"USD": [
                    {{"start":"2023-01-01","end":"2023-12-31","val":90,"accn":"000123456724000001","fp":"FY","form":"10-K","filed":"2024-02-15"}},
                    {{"start":"2024-01-01","end":"2024-12-31","val":100,"accn":"000123456725000001","fp":"FY","form":"10-K","filed":"2025-02-15"}}
                  ]}}}},
                  "Revenues": {{"units": {{"USD": [
                    {{"start":"2024-01-01","end":"2024-12-31","val":{second_value},"accn":"000123456725000002","fp":"FY","form":"10-K","filed":"2025-02-15"}}
                  ]}}}}
                }}
              }}
            }}
            "#
        )),
    );

    SecClient::collect(&company, &client, "ORG-X test contact@example.test")
        .expect("ambiguous Company Facts fixture should collect as UNKNOWN")
}

#[test]
fn sec_marks_conflicting_latest_aliases_unknown() {
    let evidence = collect_revenue_alias_fixture(110);
    let revenue = evidence
        .fact("revenue")
        .expect("revenue fact should be present");

    assert_eq!(revenue.status(), &FactStatus::Unknown);
    assert_eq!(revenue.value(), None);
}

#[test]
fn sec_marks_same_value_latest_alias_duplicates_unknown() {
    let evidence = collect_revenue_alias_fixture(100);
    let revenue = evidence
        .fact("revenue")
        .expect("revenue fact should be present");

    assert_eq!(revenue.status(), &FactStatus::Unknown);
    assert_eq!(revenue.value(), None);
}

#[test]
fn sec_selects_latest_10k_metadata_and_preserves_employee_passage() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let filing_url =
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000002/acme-2024.htm";
    let user_agent = "ORG-X weekly-radar test contact@example.test";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(
            r#"
            {
              "filings": {"recent": {
                "accessionNumber": ["0001234567-25-000002", "0001234567-25-000001", "0001234567-24-000001"],
                "filingDate": ["2025-02-15", "2025-01-31", "2024-02-15"],
                "reportDate": ["2024-12-31", "2024-09-30", "2023-12-31"],
                "form": ["10-K", "10-Q", "10-K"],
                "primaryDocument": ["acme-2024.htm", "acme-q3.htm", "acme-2023.htm"]
              }}
            }
            "#,
        ),
    );
    client.insert(facts_url, HttpResponse::ok(r#"{"facts":{"us-gaap":{}}}"#));
    client.insert(
        filing_url,
        HttpResponse::ok(
            "<html><body>As of December 31, 2024, we had approximately 1,234 employees.</body></html>",
        ),
    );

    let evidence = SecClient::collect(&company, &client, user_agent)
        .expect("submissions and filing fixtures should collect");
    let employee = evidence
        .fact("employee_count")
        .expect("employee fact should be present");

    assert_eq!(employee.status(), &FactStatus::Known);
    assert_eq!(employee.value(), Some("1234"));
    assert_eq!(employee.confidence(), &Confidence::Approximate);
    assert_eq!(employee.provenance().source_uri(), filing_url);
    assert!(employee
        .provenance()
        .source_field_or_passage()
        .contains("1,234 employees"));
    assert_eq!(
        employee.provenance().effective_date(),
        NaiveDate::from_ymd_opt(2024, 12, 31).as_ref()
    );
    assert_eq!(client.requests().len(), 3);
    for request in client.requests() {
        assert_eq!(
            request.headers(),
            &[("User-Agent".to_owned(), user_agent.to_owned())]
        );
    }
    assert!(client
        .requests()
        .iter()
        .any(|request| request.url() == filing_url));
}

#[test]
fn employee_rule_accepts_one_explicit_dated_workforce_candidate() {
    assert_eq!(
        extract_employee_count(
            "As of December 31, 2024, we had 1,234 employees.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Known
    );
}

#[test]
fn employee_rule_accepts_approximate_workforce_wording() {
    assert_eq!(
        extract_employee_count(
            "At year end, our workforce was approximately 1,200 employees.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Known
    );
}

#[test]
fn employee_rule_marks_conflicting_candidates_unknown() {
    assert_eq!(
        extract_employee_count(
            "As of December 31, 2024, we had 1,000 employees. At year end, our workforce was 1,100 employees.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Unknown
    );
}

#[test]
fn employee_rule_marks_missing_date_unknown() {
    assert_eq!(
        extract_employee_count(
            "We had 1,000 employees.",
            None,
            "https://example.test/filing/2024"
        ),
        FactStatus::Unknown
    );
}

#[test]
fn employee_rule_ignores_unrelated_customer_counts() {
    assert_eq!(
        extract_employee_count(
            "We served more than 1,000 customers during 2024.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Unknown
    );
}

#[test]
fn employee_rule_rejects_customer_population_that_mentions_employees() {
    assert_eq!(
        extract_employee_count(
            "Our customers collectively employ 1,000 employees.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Unknown
    );
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
