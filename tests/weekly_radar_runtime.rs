use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramTransport, TelegramTransportError,
};
use org_x::features::weekly_radar::runtime::archive::{retain_recent, write_run, ArchiveError};
use org_x::features::weekly_radar::runtime::config::{CompanyConfig, CompanySourceRegistry};
use org_x::features::weekly_radar::runtime::error::RuntimeError;
use org_x::features::weekly_radar::runtime::http::{
    FixtureHttpClient, HttpClient, HttpResponse, UreqHttpClient,
};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};
use org_x::features::weekly_radar::runtime::normalize_source_observation;
use org_x::features::weekly_radar::runtime::report::{render_report, RenderedReport};
use org_x::features::weekly_radar::runtime::rules::extract_employee_count;
use org_x::features::weekly_radar::runtime::sec::SecClient;
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, SourceKind, SourceStatus, SourceTier,
};
use org_x::features::weekly_radar::runtime::telegram::{
    send_rendered_report_with_transport, TelegramError, TelegramRetryPolicy,
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
fn calibration_registry_contains_only_configured_prd_companies() {
    let registry = CompanySourceRegistry::from_path("config/weekly_radar/companies.json")
        .expect("calibration registry should satisfy runtime validation");
    let expected = [
        ("meta", "META", "0001326801"),
        ("pltr", "PLTR", "0001321655"),
        ("msft", "MSFT", "0000789019"),
        ("goog", "GOOG", "0001652044"),
        ("amzn", "AMZN", "0001018724"),
        ("nvda", "NVDA", "0001045810"),
        ("crm", "CRM", "0001108524"),
        ("adbe", "ADBE", "0000796343"),
        ("ibm", "IBM", "0000051143"),
        ("wmt", "WMT", "0000104169"),
    ];

    assert_eq!(registry.companies().len(), expected.len());
    for (id, ticker, cik) in expected {
        let company = registry
            .company(id)
            .expect("expected company should be configured");
        assert_eq!(company.ticker(), ticker);
        assert_eq!(company.sec_cik(), Some(cik));
        assert!(company.official_ir_url().is_some());
        assert_eq!(company.greenhouse_board(), None);
        assert_eq!(company.lever_site(), None);
    }
}

#[test]
fn configured_hiring_identifiers_reject_path_query_fragment_and_whitespace() {
    for invalid in [
        "acme/jobs",
        "acme?token=1",
        "acme#fragment",
        "acme board",
        "acme.board",
    ] {
        let greenhouse = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            None,
            None,
            None,
            Some(invalid.to_owned()),
            None,
        );
        assert!(
            greenhouse.is_err(),
            "unsafe Greenhouse identifier must be rejected: {invalid}"
        );

        let lever = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            None,
            None,
            None,
            None,
            Some(invalid.to_owned()),
        );
        assert!(
            lever.is_err(),
            "unsafe Lever identifier must be rejected: {invalid}"
        );
    }

    assert!(CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        None,
        None,
        None,
        Some("acme-board_1".to_owned()),
        Some("acme_site-1".to_owned()),
    )
    .is_ok());
}

fn source_test_company() -> CompanyConfig {
    CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        Some("https://example.test/investors".to_owned()),
        Some("https://example.test/careers".to_owned()),
        Some("https://example.test/engineering".to_owned()),
        Some("acme".to_owned()),
        Some("acme".to_owned()),
    )
    .expect("source fixture company should be valid")
}

fn gdelt_fixture_url() -> &'static str {
    "https://api.gdeltproject.org/api/v2/doc/doc?query=%22Acme%20Corporation%22&mode=artlist&format=json&maxrecords=10&sort=HybridRel"
}

#[test]
fn source_collection_skips_gdelt_without_configured_source_endpoints() {
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
    .expect("source-free company fixture should be valid");
    let client = FixtureHttpClient::new();
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);

    assert!(
        observations
            .iter()
            .all(|observation| observation.kind() != SourceKind::Gdelt),
        "GDELT must be skipped when no configured source endpoint exists"
    );
    assert!(
        client.requests().is_empty(),
        "source-free collection must not issue a discovery request"
    );
}

#[test]
fn source_adapter_extracts_official_html_without_provider_markup() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(
        company.official_ir_url().unwrap(),
        HttpResponse::ok(
            r#"<html><head><style>hidden</style></head><body>Investor <strong>relations</strong> &amp; updates<script>ignore()</script></body></html>"#,
        ),
    );
    client.insert(gdelt_fixture_url(), HttpResponse::ok(r#"{"articles":[]}"#));
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    let official = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("configured official IR should produce an observation");

    assert_eq!(official.status(), SourceStatus::Known);
    assert_eq!(official.tier(), SourceTier::OfficialPrimary);
    assert_eq!(official.text(), "Investor relations & updates");
    assert_eq!(official.url(), company.official_ir_url());
    assert_eq!(
        official.provenance().source_field_or_passage(),
        "official page text"
    );
    assert_eq!(official.provenance().retrieved_at(), &observed_at);
}

#[test]
fn source_adapter_parses_greenhouse_jobs_with_provenance() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(
        "https://boards-api.greenhouse.io/v1/boards/acme/jobs?content=true",
        HttpResponse::ok(
            r#"
            {
              "jobs": [
                {
                  "id": 101,
                  "title": "Senior Systems Engineer",
                  "updated_at": "2026-08-15T12:00:00Z",
                  "absolute_url": "https://boards.greenhouse.io/acme/jobs/101",
                  "location": {"name": "Remote"},
                  "content": "<p>Build <strong>reliable</strong> systems.</p>"
                }
              ]
            }
            "#,
        ),
    );
    client.insert(gdelt_fixture_url(), HttpResponse::ok(r#"{"articles":[]}"#));
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    let job = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Greenhouse)
        .expect("configured Greenhouse job should produce an observation");

    assert_eq!(job.status(), SourceStatus::Known);
    assert_eq!(job.tier(), SourceTier::StructuredHiring);
    assert_eq!(job.title(), Some("Senior Systems Engineer"));
    assert_eq!(job.text(), "Build reliable systems.");
    assert_eq!(
        job.url(),
        Some("https://boards.greenhouse.io/acme/jobs/101")
    );
    assert_eq!(
        job.provenance().source_field_or_passage(),
        "greenhouse.jobs[101].content"
    );
    assert_eq!(
        job.provenance().effective_date(),
        Some(&NaiveDate::from_ymd_opt(2026, 8, 15).unwrap())
    );
}

#[test]
fn source_adapter_parses_lever_postings_with_provenance() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(
        "https://api.lever.co/v0/postings/acme?mode=json",
        HttpResponse::ok(
            r#"
            [
              {
                "id": "posting-1",
                "text": "Data Platform Engineer",
                "hostedUrl": "https://jobs.lever.co/acme/posting-1",
                "applyUrl": "https://jobs.lever.co/acme/posting-1/apply",
                "descriptionPlain": "Own the data platform.",
                "description": "<p>Own the <em>data</em> platform.</p>",
                "categories": {"location": "Remote", "team": "Engineering"}
              }
            ]
            "#,
        ),
    );
    client.insert(gdelt_fixture_url(), HttpResponse::ok(r#"{"articles":[]}"#));
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    let posting = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Lever)
        .expect("configured Lever posting should produce an observation");

    assert_eq!(posting.status(), SourceStatus::Known);
    assert_eq!(posting.tier(), SourceTier::StructuredHiring);
    assert_eq!(posting.title(), Some("Data Platform Engineer"));
    assert_eq!(posting.text(), "Own the data platform.");
    assert_eq!(posting.url(), Some("https://jobs.lever.co/acme/posting-1"));
    assert_eq!(
        posting.provenance().source_field_or_passage(),
        "lever.postings[posting-1].descriptionPlain"
    );
}

#[test]
fn source_adapter_marks_gdelt_records_as_discovery_only() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(
        gdelt_fixture_url(),
        HttpResponse::ok(
            r#"
        {
          "articles": [
            {
              "url": "https://news.example.test/acme",
              "title": "Acme expands its platform",
              "seendate": "20260817T120000Z",
              "domain": "news.example.test"
            }
          ]
        }
        "#,
        ),
    );
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    let article = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Gdelt)
        .expect("GDELT article should produce an observation");

    assert_eq!(article.status(), SourceStatus::DiscoveryOnly);
    assert_eq!(article.tier(), SourceTier::DiscoveryOnly);
    assert!(!article.is_authoritative());
    assert_eq!(article.title(), Some("Acme expands its platform"));
    assert_eq!(article.url(), Some("https://news.example.test/acme"));
    assert_eq!(article.text(), "Acme expands its platform");
    assert_eq!(
        article.provenance().effective_date(),
        Some(&NaiveDate::from_ymd_opt(2026, 8, 17).unwrap())
    );
    assert!(article
        .provenance()
        .source_field_or_passage()
        .contains("GDELT query"));
}

#[test]
fn source_observations_normalize_to_facts_with_status_and_passage_provenance() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(
        company.official_ir_url().unwrap(),
        HttpResponse::ok("<html><body>Official strategy update</body></html>"),
    );
    client.insert(
        "https://boards-api.greenhouse.io/v1/boards/acme/jobs?content=true",
        HttpResponse::ok(
            r#"{"jobs":[{"id":101,"title":"Systems Engineer","updated_at":"2026-08-15T12:00:00Z","absolute_url":"https://boards.greenhouse.io/acme/jobs/101","content":"<p>Build reliable systems.</p>"}]}"#,
        ),
    );
    client.insert(
        "https://api.lever.co/v0/postings/acme?mode=json",
        HttpResponse::ok(
            r#"[{"id":"posting-1","text":"Data Platform Engineer","hostedUrl":"https://jobs.lever.co/acme/posting-1","descriptionPlain":"Own the data platform."}]"#,
        ),
    );
    client.insert(
        gdelt_fixture_url(),
        HttpResponse::ok(
            r#"{"articles":[{"url":"https://news.example.test/acme","title":"Acme expands its platform","seendate":"20260817T120000Z"}]}"#,
        ),
    );
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();
    let observations = collect_configured_sources(&company, &client, observed_at);

    let official = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::OfficialIr)
        .expect("official observation should exist");
    let official_fact =
        normalize_source_observation(official, 1).expect("official observation should normalize");
    assert_eq!(official_fact.kind(), "source_official_ir_001");
    assert_eq!(official_fact.status(), &FactStatus::Known);
    assert_eq!(official_fact.value(), Some("Official strategy update"));
    assert_eq!(
        official_fact.provenance().source_field_or_passage(),
        "official page text"
    );

    let greenhouse = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Greenhouse)
        .expect("Greenhouse observation should exist");
    let greenhouse_fact = normalize_source_observation(greenhouse, 1)
        .expect("Greenhouse observation should normalize");
    assert_eq!(greenhouse_fact.kind(), "source_greenhouse_001");
    assert_eq!(greenhouse_fact.status(), &FactStatus::Unconfirmed);
    assert_eq!(greenhouse_fact.value(), None);
    assert!(greenhouse_fact
        .provenance()
        .source_field_or_passage()
        .contains("Build reliable systems."));

    let gdelt = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Gdelt)
        .expect("GDELT observation should exist");
    let gdelt_fact =
        normalize_source_observation(gdelt, 1).expect("GDELT observation should normalize");
    assert_eq!(gdelt_fact.kind(), "source_gdelt_001");
    assert_eq!(gdelt_fact.status(), &FactStatus::Unconfirmed);
    assert_eq!(gdelt_fact.value(), None);
    assert!(gdelt_fact
        .provenance()
        .source_field_or_passage()
        .contains("Acme expands its platform"));

    let unavailable = observations
        .iter()
        .find(|observation| observation.kind() == SourceKind::Careers)
        .expect("missing careers observation should exist");
    let unavailable_fact = normalize_source_observation(unavailable, 1)
        .expect("unavailable observation should normalize");
    assert_eq!(unavailable_fact.status(), &FactStatus::Unavailable);
    assert_eq!(unavailable_fact.value(), None);
}

#[test]
fn absent_optional_sources_are_unavailable_without_guessed_urls() {
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
    .expect("minimal source fixture company should be valid");
    let client = FixtureHttpClient::with_response(
        "https://api.gdeltproject.org/api/v2/doc/doc?query=%22Beta%20Systems%22&mode=artlist&format=json&maxrecords=10&sort=HybridRel",
        HttpResponse::ok(r#"{"articles":[]}"#),
    );
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    for kind in [
        SourceKind::OfficialIr,
        SourceKind::Careers,
        SourceKind::EngineeringAiBlog,
        SourceKind::Greenhouse,
        SourceKind::Lever,
    ] {
        let observation = observations
            .iter()
            .find(|observation| observation.kind() == kind)
            .expect("each optional source slot should be represented");
        assert_eq!(observation.status(), SourceStatus::Unavailable);
        assert_eq!(observation.url(), None);
        assert!(observation
            .provenance()
            .source_uri()
            .starts_with("source://weekly-radar/"));
    }
}

#[test]
fn source_observations_serialize_without_a_public_deserialization_path() {
    let company = source_test_company();
    let client = FixtureHttpClient::with_response(
        gdelt_fixture_url(),
        HttpResponse::ok(
            r#"{"articles":[{"url":"https://news.example.test/acme","title":"Acme context","seendate":"20260817T120000Z"}]}"#,
        ),
    );
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();
    let article = collect_configured_sources(&company, &client, observed_at)
        .into_iter()
        .find(|observation| observation.kind() == SourceKind::Gdelt)
        .expect("GDELT fixture should produce an observation");

    let payload = serde_json::to_value(&article).expect("observation should serialize");
    assert_eq!(payload["kind"], "gdelt");
    assert_eq!(payload["status"], "DISCOVERY_ONLY");
    assert_eq!(payload["tier"], "DISCOVERY_ONLY");
    assert!(!article.is_authoritative());
}

#[test]
fn oversized_official_and_hiring_bodies_become_unavailable() {
    const MAX_SOURCE_BODY_BYTES: usize = 1_048_576;

    let company = source_test_company();
    let client = FixtureHttpClient::new();
    let oversized_text = "x".repeat(MAX_SOURCE_BODY_BYTES + 1);
    client.insert(
        company.official_ir_url().unwrap(),
        HttpResponse::ok(format!("<p>{oversized_text}</p>")),
    );
    client.insert(
        "https://boards-api.greenhouse.io/v1/boards/acme/jobs?content=true",
        HttpResponse::ok(format!(r#"{{"jobs":"{oversized_text}"}}"#)),
    );
    client.insert(
        "https://api.lever.co/v0/postings/acme?mode=json",
        HttpResponse::ok(oversized_text),
    );
    client.insert(gdelt_fixture_url(), HttpResponse::ok(r#"{"articles":[]}"#));
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    for kind in [
        SourceKind::OfficialIr,
        SourceKind::Greenhouse,
        SourceKind::Lever,
    ] {
        let observation = observations
            .iter()
            .find(|observation| observation.kind() == kind)
            .expect("oversized source should still have an observation");
        assert_eq!(observation.status(), SourceStatus::Unavailable);
        assert_eq!(observation.text(), "");
    }
}

#[test]
fn over_limit_greenhouse_and_lever_payloads_fail_closed_as_unknown() {
    const MAX_HIRING_RECORDS: usize = 100;

    let company = source_test_company();
    let client = FixtureHttpClient::new();
    let greenhouse_jobs = (0..MAX_HIRING_RECORDS + 5)
        .map(|id| {
            format!(
                r#"{{"id":{id},"title":"Greenhouse Role {id}","absolute_url":"https://jobs.example.test/greenhouse/{id}","content":"Build systems."}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    client.insert(
        "https://boards-api.greenhouse.io/v1/boards/acme/jobs?content=true",
        HttpResponse::ok(format!(r#"{{"jobs":[{greenhouse_jobs}]}}"#)),
    );

    let lever_postings = (0..MAX_HIRING_RECORDS + 5)
        .map(|id| {
            format!(
                r#"{{"id":"posting-{id}","text":"Lever Role {id}","hostedUrl":"https://jobs.example.test/lever/{id}","descriptionPlain":"Build systems."}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    client.insert(
        "https://api.lever.co/v0/postings/acme?mode=json",
        HttpResponse::ok(format!(r#"[{lever_postings}]"#)),
    );
    client.insert(gdelt_fixture_url(), HttpResponse::ok(r#"{"articles":[]}"#));
    let observed_at: DateTime<Utc> = "2026-08-17T00:00:00Z".parse().unwrap();

    let observations = collect_configured_sources(&company, &client, observed_at);
    assert_eq!(
        observations
            .iter()
            .find(|observation| observation.kind() == SourceKind::Greenhouse)
            .map(|observation| observation.status()),
        Some(SourceStatus::Unknown)
    );
    assert_eq!(
        observations
            .iter()
            .find(|observation| observation.kind() == SourceKind::Lever)
            .map(|observation| observation.status()),
        Some(SourceStatus::Unknown)
    );
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
        HttpResponse::ok(format!(
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
fn employee_rule_rejects_customer_suffix_context() {
    assert_eq!(
        extract_employee_count(
            "The filing lists 1,000 employees of our customers.",
            Some(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()),
            "https://example.test/filing/2024"
        ),
        FactStatus::Unknown
    );
}

#[test]
fn employee_rule_rejects_competitor_context() {
    assert_eq!(
        extract_employee_count(
            "The report lists 1,000 employees of a competitor.",
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

#[test]
fn fixture_and_ureq_reject_response_bodies_over_one_mib() {
    const MAX_HTTP_RESPONSE_BODY_BYTES: usize = 1_048_576;
    let oversized = "x".repeat(MAX_HTTP_RESPONSE_BODY_BYTES + 1);
    let fixture_url = "https://example.test/oversized";
    let fixture_error =
        FixtureHttpClient::with_response(fixture_url, HttpResponse::ok(oversized.clone()))
            .get(fixture_url, &[])
            .expect_err("fixture transport should reject oversized bodies");
    assert_eq!(
        fixture_error.to_string(),
        "HTTP response body exceeded configured limit"
    );

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
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            oversized.len()
        )
        .expect("local response headers should be written");
        stream
            .write_all(oversized.as_bytes())
            .expect("local response body should be written");
    });
    let url = format!("http://{address}/oversized");
    let ureq_error = UreqHttpClient::new()
        .get(&url, &[])
        .expect_err("ureq transport should reject oversized bodies");
    assert_eq!(
        ureq_error.to_string(),
        "HTTP response body exceeded configured limit"
    );
    server.join().expect("local server should finish");
}

fn task4_provenance(field: &str) -> Provenance {
    Provenance::from_rfc3339(
        "https://example.test/evidence/2026?token=must-not-appear#fragment",
        field,
        "2026-08-17T00:00:00Z",
        Some("2026-08-15"),
    )
    .expect("task 4 provenance should be valid")
}

fn task4_report_input() -> RuntimeReportInput {
    let mut input = RuntimeReportInput::new("2026-08-17").expect("task 4 date should be valid");
    let facts = [
        (
            "acme",
            "structural_change",
            Some("Operating model changed"),
            FactStatus::Known,
            Confidence::High,
        ),
        (
            "acme",
            "revenue",
            Some("123000000"),
            FactStatus::Known,
            Confidence::High,
        ),
        (
            "beta",
            "employees",
            None,
            FactStatus::Unknown,
            Confidence::Unknown,
        ),
        (
            "gamma",
            "operating_income",
            None,
            FactStatus::Unavailable,
            Confidence::Unknown,
        ),
        (
            "delta",
            "research_and_development",
            None,
            FactStatus::Unconfirmed,
            Confidence::Low,
        ),
        (
            "epsilon",
            "cash_flow",
            Some("45000000"),
            FactStatus::Known,
            Confidence::Medium,
        ),
    ];

    for (company, kind, value, status, confidence) in facts {
        let fact = match value {
            Some(value) => NormalizedFact::new(
                company,
                kind,
                value,
                status,
                confidence,
                task4_provenance(kind),
            ),
            None => NormalizedFact::without_value(
                company,
                kind,
                status,
                confidence,
                task4_provenance(kind),
            ),
        }
        .expect("task 4 fact should be valid");
        input.add_fact(fact).expect("task 4 fact should be unique");
    }

    input
        .add_source_coverage(SourceCoverage::new("official", 5, 5).unwrap())
        .unwrap();
    input
        .add_source_coverage(SourceCoverage::new("sec", 5, 4).unwrap())
        .unwrap();
    input
        .add_source_coverage(SourceCoverage::new("gdelt-discovery", 5, 1).unwrap())
        .unwrap();
    input
}

fn task4_report() -> RenderedReport {
    render_report(&task4_report_input())
}

#[test]
fn task4_report_is_deterministic_and_uses_the_mobile_first_contract() {
    let first = task4_report();
    let second = task4_report();

    assert_eq!(first.markdown(), second.markdown());
    assert_eq!(first.snapshot_json(), second.snapshot_json());
    assert_eq!(first.report_id(), second.report_id());
    assert!(first.report_id().starts_with("wr-"));

    let headings: Vec<&str> = first
        .markdown()
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect();
    assert_eq!(headings.first(), Some(&"Executive Summary"));
    assert_eq!(headings.last(), Some(&"System Health"));
    assert!(headings.iter().all(|heading| {
        matches!(
            *heading,
            "Executive Summary"
                | "Important Structural Change"
                | "Top5"
                | "Rising"
                | "Dropped"
                | "System Health"
        )
    }));
    assert!(first.markdown().matches("### Change ").count() <= 3);
    assert!(first.markdown().matches("### ").count() <= 8);
    assert!(first.markdown().contains("## Important Structural Change"));
    assert!(first.markdown().contains("## Top5"));
    assert!(!first.markdown().contains("Stage"));
    assert!(!first.markdown().contains("rank"));
    assert!(!first.markdown().contains("score"));
    assert!(!first.markdown().contains("invest"));
    assert!(!first.markdown().contains("token=must-not-appear"));
    assert!(!first.snapshot_json().contains("token=must-not-appear"));
}

#[test]
fn task4_report_omits_top5_without_an_explicit_selection_for_more_than_five_companies() {
    let mut input = task4_report_input();
    input
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("extra company fact should be valid"),
        )
        .expect("extra company fact should be unique");

    let report = render_report(&input);
    assert!(!report.markdown().contains("## Top5"));
    assert!(report
        .markdown()
        .contains("Top5: UNKNOWN — no explicit Top5 selection was supplied"));
}

#[test]
fn task4_empty_report_states_that_no_evidence_was_supplied() {
    let input = RuntimeReportInput::new("2026-08-17").expect("empty report date should be valid");
    let report = render_report(&input);

    assert!(report
        .markdown()
        .contains("Evidence basis: UNKNOWN — no evidence supplied"));
}

#[test]
fn task4_report_exposes_explicit_statuses_and_discovery_health_review_items() {
    let report = task4_report();
    let markdown = report.markdown();

    assert!(markdown.contains("CONFIRMED"));
    assert!(markdown.contains("UNKNOWN"));
    assert!(markdown.contains("UNAVAILABLE"));
    assert!(markdown.contains("UNCONFIRMED"));
    assert!(markdown.contains("DISCOVERY ONLY"));
    assert!(markdown.contains("needing review"));
    assert!(markdown.contains("official: 5/5"));
    assert!(markdown.contains("sec: 4/5"));
}

#[derive(Clone, Default)]
struct Task4RecordingTransport(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl TelegramTransport for Task4RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        let mut sent = self.0.lock().expect("task 4 recording lock should work");
        sent.push(markdown.to_owned());
        TelegramMessageId::new(format!("task4-message-{}", sent.len())).map_err(|error| {
            TelegramTransportError::Failed {
                reason: error.to_string(),
            }
        })
    }
}

struct Task4SecretFailingTransport;

impl TelegramTransport for Task4SecretFailingTransport {
    fn send_message(
        &self,
        _destination: &str,
        _markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        Err(TelegramTransportError::Failed {
            reason: "token=task4-secret chat_id=task4-chat".to_owned(),
        })
    }
}

#[derive(Clone, Default)]
struct Task4PartialFailingTransport(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl TelegramTransport for Task4PartialFailingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        let mut sent = self.0.lock().expect("partial transport lock should work");
        sent.push(markdown.to_owned());
        if sent.len() == 2 {
            return Err(TelegramTransportError::Failed {
                reason: "token=task4-secret chat_id=task4-chat".to_owned(),
            });
        }
        TelegramMessageId::new(format!("partial-message-{}", sent.len())).map_err(|error| {
            TelegramTransportError::Failed {
                reason: error.to_string(),
            }
        })
    }
}

#[test]
fn task4_telegram_delivery_preserves_chunk_order_and_redacts_errors() {
    let report = task4_report();
    let transport = Task4RecordingTransport::default();
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("recording transport should receive every report chunk");
    let sent = transport.0.lock().unwrap().clone();

    assert_eq!(receipt.message_ids().len(), sent.len());
    assert_eq!(receipt.message_ids()[0].as_str(), "task4-message-1");
    assert_eq!(receipt.report_id(), report.report_id());
    assert!(sent[0].contains("Evidence basis:"));
    assert!(sent[0].contains("https://example.test/evidence/2026"));
    assert_eq!(sent[0], report.markdown()[..sent[0].len()]);
    for pair in sent.windows(2) {
        assert_ne!(pair[0], pair[1]);
    }

    let error = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4SecretFailingTransport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect_err("failing transport should return a typed error");
    let display = error.to_string();
    assert!(!display.contains("task4-secret"));
    assert!(!display.contains("task4-chat"));
    assert!(!display.contains("token="));
    assert!(!display.contains("chat_id="));
}

#[test]
fn task4_telegram_partial_failure_preserves_accepted_ids_and_attempts() {
    let report = task4_report();
    let error = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4PartialFailingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect_err("second chunk failure should produce a partial delivery error");

    match error {
        TelegramError::DeliveryFailed {
            successful_message_ids,
            successful_attempts,
            ..
        } => {
            assert_eq!(
                successful_message_ids
                    .iter()
                    .map(TelegramMessageId::as_str)
                    .collect::<Vec<_>>(),
                ["partial-message-1"]
            );
            assert_eq!(successful_attempts, [1]);
        }
        other => panic!("expected partial delivery error, got {other:?}"),
    }
}

fn task4_temp_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "org-x-task4-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ))
}

#[test]
fn task4_archive_retains_365_days_and_guards_the_data_branch() {
    let root = task4_temp_root("archive");
    let weekly_radar = root.join("weekly-radar");
    for directory in ["reports", "snapshots", "receipts"] {
        std::fs::create_dir_all(weekly_radar.join(directory)).unwrap();
    }
    for directory in ["reports", "snapshots", "receipts"] {
        std::fs::write(
            weekly_radar.join(directory).join("2025-08-16.expired"),
            "expired",
        )
        .unwrap();
        std::fs::write(
            weekly_radar.join(directory).join("2025-08-17.retained"),
            "retained",
        )
        .unwrap();
    }

    let as_of = NaiveDate::from_ymd_opt(2026, 8, 17).unwrap();
    let retention_error = retain_recent(&root, "main", as_of, 365)
        .expect_err("retention must reject non-data branches before deletion");
    assert!(matches!(
        retention_error,
        ArchiveError::NonDataBranch { .. }
    ));
    let removed = retain_recent(&root, "data", as_of, 365).expect("retention should complete");
    assert_eq!(removed, 3);
    assert!(!weekly_radar.join("reports/2025-08-16.expired").exists());
    assert!(weekly_radar.join("reports/2025-08-17.retained").exists());

    let report = task4_report();
    let transport = Task4RecordingTransport::default();
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("a successful delivery receipt is required for archiving");
    let error = write_run(&root, "main", &report, &receipt)
        .expect_err("non-data branch writes must be rejected at write_run");
    assert!(matches!(error, ArchiveError::NonDataBranch { .. }));

    write_run(&root, "data", &report, &receipt).expect("published data archive should write");
    let markdown = std::fs::read_to_string(weekly_radar.join("reports/2026-08-17.md"))
        .expect("report should be archived");
    assert_eq!(markdown, report.markdown());
    assert!(weekly_radar.join("snapshots/2026-08-17.json").exists());
    let archived_receipt = std::fs::read_to_string(weekly_radar.join("receipts/2026-08-17.json"))
        .expect("receipt should be archived");
    let archived_receipt: serde_json::Value =
        serde_json::from_str(&archived_receipt).expect("archived receipt should be JSON");
    assert_eq!(archived_receipt["status"], "PUBLISHED");
    assert_eq!(archived_receipt["report_id"], report.report_id());
    let expected_message_ids = receipt
        .message_ids()
        .iter()
        .map(TelegramMessageId::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        archived_receipt["message_ids"],
        serde_json::json!(expected_message_ids)
    );
    assert_eq!(
        archived_receipt["attempts"],
        serde_json::json!(receipt.attempts())
    );
    assert_ne!(archived_receipt["status"], "NOT_PUBLISHED");
    assert!(weekly_radar.join("manifest.json").exists());

    std::fs::remove_dir_all(root).expect("task 4 temporary archive should be removable");
}

#[test]
fn task4_archive_rejects_a_receipt_for_a_different_report() {
    let root = task4_temp_root("report-id-mismatch");
    let report = task4_report();
    let mut other_input = task4_report_input();
    other_input
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("different report fact should be valid"),
        )
        .expect("different report fact should be unique");
    let other_report = render_report(&other_input);
    assert_ne!(report.report_id(), other_report.report_id());
    let receipt = send_rendered_report_with_transport(
        &other_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("different report should have a successful receipt");

    let error = write_run(&root, "data", &report, &receipt)
        .expect_err("archive must reject a receipt for another report");
    assert!(matches!(error, ArchiveError::ReportIdMismatch { .. }));
    assert!(!root.exists(), "mismatch must fail before archive mutation");
}

fn task5_cli_fixture_root(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "org-x-task5-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ))
}

fn task5_write_registry(root: &std::path::Path) -> std::path::PathBuf {
    fs::create_dir_all(root).expect("task 5 fixture root should be writable");
    let path = root.join("registry.json");
    fs::write(
        &path,
        r#"{
          "version": 1,
          "companies": [
            {"id": "fixture", "name": "Fixture Systems", "ticker": "FIX"}
          ]
        }"#,
    )
    .expect("task 5 registry fixture should be writable");
    path
}

fn task5_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args(args)
        .env("ORGX_SEC_USER_AGENT", "ORG-X test contact@example.test")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("org-x binary should be executable")
}

#[test]
fn task5_cli_accepts_weekly_radar_dry_run_without_archive_mutation() {
    let root = task5_cli_fixture_root("dry-run");
    let registry = task5_write_registry(&root);
    let archive = root.join("archive");
    let output = task5_cli(&[
        "weekly-radar",
        "--as-of",
        "2026-08-17",
        "--archive-dir",
        archive.to_str().unwrap(),
        "--registry",
        registry.to_str().unwrap(),
        "--dry-run",
    ]);

    assert!(
        output.status.success(),
        "dry-run should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("DRY-RUN"),
        "dry-run should identify its non-mutating mode"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("## System Health"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("official_ir: 0/1"));
    assert!(
        !archive.exists(),
        "dry-run must not create or mutate archive output"
    );
    fs::remove_dir_all(root).expect("task 5 dry-run fixture should be removable");
}

#[test]
fn task5_cli_rejects_invalid_usage() {
    let output = task5_cli(&["not-weekly-radar"]);

    assert!(!output.status.success(), "unknown command must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("weekly-radar"));
}

#[test]
fn task5_cli_requires_sec_user_agent_before_acquisition() {
    let root = task5_cli_fixture_root("missing-user-agent");
    let registry = task5_write_registry(&root);
    let archive = root.join("archive");
    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--as-of",
            "2026-08-17",
            "--archive-dir",
            archive.to_str().unwrap(),
            "--registry",
            registry.to_str().unwrap(),
            "--dry-run",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("org-x binary should be executable");

    assert!(!output.status.success(), "missing SEC User-Agent must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("ORGX_SEC_USER_AGENT"));
    assert!(
        !archive.exists(),
        "configuration failure must not mutate archive"
    );
    fs::remove_dir_all(root).expect("task 5 user-agent fixture should be removable");
}

#[test]
fn task5_cli_blocks_publication_without_primary_evidence() {
    let root = task5_cli_fixture_root("no-primary");
    let registry = task5_write_registry(&root);
    let archive = root.join("archive");
    let output = task5_cli(&[
        "weekly-radar",
        "--as-of",
        "2026-08-17",
        "--archive-dir",
        archive.to_str().unwrap(),
        "--registry",
        registry.to_str().unwrap(),
    ]);

    assert!(!output.status.success(), "no-primary publication must fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("primary evidence"));
    assert!(
        !archive.exists(),
        "publication gate must fail before archive mutation"
    );
    fs::remove_dir_all(root).expect("task 5 primary-evidence fixture should be removable");
}

fn task6_workflow_text() -> String {
    fs::read_to_string(".github/workflows/weekly-radar.yml").expect("Task 6 workflow should exist")
}

#[test]
fn task6_workflow_declares_schedule_dispatch_checkout_permissions_and_secrets() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("cron: '0 0 * * 1'"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("uses: actions/checkout@v5"));
    assert!(!workflow.contains("actions/checkout@v4"));
    assert!(workflow.contains("permissions:\n  contents: write"));
    assert!(workflow.contains("group: weekly-radar-data"));
    for secret in [
        "ORGX_SEC_USER_AGENT",
        "ORGX_TELEGRAM_BOT_TOKEN",
        "ORGX_TELEGRAM_CHAT_ID",
    ] {
        assert!(
            workflow.contains(&format!("secrets.{secret}")),
            "missing {secret}"
        );
    }
}

#[test]
fn task6_workflow_reconstructs_data_and_creates_a_rolling_orphan_update() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("git fetch --no-tags origin main"));
    assert!(workflow.contains("git ls-remote --exit-code --heads origin data"));
    assert!(workflow.contains("git archive origin/data -- weekly-radar"));
    assert!(workflow.contains("365"));
    assert!(workflow.contains("git checkout --orphan"));
    assert!(workflow.contains("git add -- weekly-radar"));
    assert!(workflow.contains("force-with-lease=refs/heads/data:"));
    assert!(workflow.contains("HEAD:refs/heads/data"));
    assert!(!workflow.contains("HEAD:refs/heads/main"));
}

#[test]
fn task6_workflow_fails_safe_for_absent_concurrent_protected_and_empty_cases() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("if git ls-remote --exit-code --heads origin data"));
    assert!(workflow.contains("refs/heads/main"));
    assert!(workflow.contains("GITHUB_REF"));
    assert!(workflow.contains("target_ref=\"refs/heads/data\""));
    assert!(workflow.contains("if [[ \"$target_ref\" == \"refs/heads/main\" ]]"));
    assert!(workflow.contains("if [[ ! -s \"$run_output\" ]]"));
    assert!(workflow.contains("if ! git push --force-with-lease"));
    assert!(workflow.contains("concurrent"));
}

#[test]
fn task6_workflow_runs_the_cli_and_rejects_empty_or_unpublished_output() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("cargo run --release -- weekly-radar"));
    assert!(workflow.contains("--archive-dir \"$GITHUB_WORKSPACE\""));
    assert!(
        workflow.contains("--registry \"$GITHUB_WORKSPACE/config/weekly_radar/companies.json\"")
    );
    assert!(workflow.contains("PUBLISHED:"));
    assert!(workflow.contains("weekly-radar/reports/"));
    assert!(workflow.contains("set -euo pipefail"));
}
