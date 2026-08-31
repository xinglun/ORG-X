use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramTransport, TelegramTransportError,
};
use org_x::features::weekly_radar::runtime::archive::{
    build_input_snapshot, ensure_run_available, ensure_run_replace_available, load_input_snapshot,
    persist_input_snapshot, recover_pending_run, replace_run_with_input_snapshot, retain_recent,
    write_run, write_run_with_input_snapshot, ArchiveError,
};
use org_x::features::weekly_radar::runtime::config::{CompanyConfig, CompanySourceRegistry};
use org_x::features::weekly_radar::runtime::discovery::{
    discover_documents, document_metadata, DocumentKind,
};
use org_x::features::weekly_radar::runtime::error::RuntimeError;
use org_x::features::weekly_radar::runtime::evidence::extract_evidence_candidate;
use org_x::features::weekly_radar::runtime::http::{
    FixtureHttpClient, HttpClient, HttpResponse, UreqHttpClient,
};
use org_x::features::weekly_radar::runtime::judgment::{
    derive_judgment_snapshot, HumanReference, MachineStage,
};
use org_x::features::weekly_radar::runtime::model::{
    CompanyIdentity, Confidence, FactStatus, NormalizedFact, Provenance, ResearchMetrics,
    RuntimeReportInput, SourceCoverage, SourceFailure, StructuralDimension,
};
use org_x::features::weekly_radar::runtime::normalize_source_observation;
use org_x::features::weekly_radar::runtime::report::{
    render_report, render_report_in_language, RenderedReport, ReportLanguage,
};
use org_x::features::weekly_radar::runtime::rules::extract_employee_count;
use org_x::features::weekly_radar::runtime::sec::{
    SecClient, SecDocumentStatus, SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES,
    SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES,
};
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, SourceKind, SourceStatus, SourceTier,
};
use org_x::features::weekly_radar::runtime::telegram::{
    send_rendered_report_with_transport, TelegramError, TelegramRetryPolicy,
};
use serde_json::Value;

#[test]
fn runtime_association_covers_claim_body_metadata_boundary() {
    let (title, date, body) = document_metadata(
        r#"<title>Engineering update</title><meta name="description" content="ignored"><script>const claim = 'ignored';</script><time datetime="2026-08-19"></time><p>Acme reorganized its engineering workflow and consolidated production scheduling under one platform.</p>"#,
        "fallback",
    );

    assert_eq!(title, "Engineering update");
    assert_eq!(date, Some(NaiveDate::from_ymd_opt(2026, 8, 19).unwrap()));
    assert_eq!(
        body,
        "Acme reorganized its engineering workflow and consolidated production scheduling under one platform."
    );
}

#[test]
fn document_metadata_extracts_visible_us_publication_date() {
    let (_, date, _) = document_metadata(
        r#"<title>Customer story</title><p>2/25/2026</p><p>Acme rolled out an agent workflow.</p>"#,
        "fallback",
    );

    assert_eq!(date, Some(NaiveDate::from_ymd_opt(2026, 2, 25).unwrap()));
}

#[test]
fn document_discovery_classifies_standalone_ai_title_tokens() {
    let documents = discover_documents(
        "msft",
        SourceKind::OfficialResearch,
        "https://www.microsoft.com/insidetrack/blog/category/frontier-firm/",
        r#"<a href="/insidetrack/blog/becoming-a-frontier-firm/">Becoming a Frontier Firm: Our playbook for the AI era</a>"#,
        Utc::now(),
    );

    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].document_kind(), DocumentKind::AiAutomation);
}

#[test]
fn document_discovery_prioritizes_content_paths_over_generic_navigation() {
    let html = r#"
        <a href="/en-us/ai/nav-1">AI navigation one</a>
        <a href="/en-us/ai/nav-2">AI navigation two</a>
        <a href="/en-us/ai/nav-3">AI navigation three</a>
        <a href="/en-us/ai/nav-4">AI navigation four</a>
        <a href="/en-us/ai/nav-5">AI navigation five</a>
        <a href="/en-us/ai/nav-6">AI navigation six</a>
        <a href="/en-us/ai/nav-7">AI navigation seven</a>
        <a href="/en-us/ai/nav-8">AI navigation eight</a>
        <a href="/insidetrack/blog/becoming-a-frontier-firm/">Becoming a Frontier Firm: Our playbook for the AI era</a>
    "#;
    let documents = discover_documents(
        "msft",
        SourceKind::OfficialResearch,
        "https://www.microsoft.com/insidetrack/blog/category/frontier-firm/",
        html,
        Utc::now(),
    );

    assert!(documents
        .iter()
        .any(|document| document.url().contains("becoming-a-frontier-firm")));
}

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
              "official_research_sources": [
                "https://example.test/frontier",
                "https://example.test/customer-stories"
              ],
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
    assert_eq!(
        registry
            .company("acme")
            .unwrap()
            .official_research_source_urls(),
        [
            "https://example.test/frontier",
            "https://example.test/customer-stories"
        ]
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
fn bounded_official_research_sources_are_collected_without_guessing_urls() {
    let company = CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("base company should be valid")
    .with_official_research_sources(vec![
        "https://example.test/frontier".to_owned(),
        "https://example.test/customer-stories".to_owned(),
    ])
    .expect("research sources should validate");
    let client = FixtureHttpClient::new();
    client.insert(
        "https://example.test/frontier",
        HttpResponse::ok("<title>Frontier research</title><p>Official research entrypoint.</p>"),
    );
    client.insert(
        "https://example.test/customer-stories",
        HttpResponse::ok("<title>Customer stories</title><p>Official customer stories.</p>"),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let entry_urls = observations
        .iter()
        .filter(|observation| {
            observation.kind() == SourceKind::OfficialResearch
                && observation.material_kind()
                    == org_x::features::weekly_radar::runtime::sources::SourceMaterialKind::EntryPoint
        })
        .filter_map(|observation| observation.url())
        .collect::<Vec<_>>();
    assert_eq!(
        entry_urls,
        vec![
            "https://example.test/frontier",
            "https://example.test/customer-stories"
        ]
    );
}

#[test]
fn explicit_official_research_document_is_collected_for_claim_extraction() {
    let company = CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("base company should be valid")
    .with_official_research_sources(vec![
        "https://example.test/customers/story/acme-ai-rollout".to_owned()
    ])
    .expect("research source should validate");
    let client = FixtureHttpClient::new();
    client.insert(
        "https://example.test/customers/story/acme-ai-rollout",
        HttpResponse::ok(
            r#"<title>Acme AI rollout</title><time datetime="2026-02-25"></time><p>Acme rolled out an agent workflow across production operations.</p>"#,
        ),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let document = observations
        .iter()
        .find(|observation| {
            observation.material_kind()
                == org_x::features::weekly_radar::runtime::sources::SourceMaterialKind::Document
                && observation.url() == Some("https://example.test/customers/story/acme-ai-rollout")
        })
        .expect("explicit content URL should become a document");
    assert!(extract_evidence_candidate(document).is_some());
}

#[test]
fn explicit_independent_research_document_keeps_cross_origin_role_and_bounds_urls() {
    let independent_url = "https://customer.example/library/case-studies/ai-operations";
    let company = CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        Some("https://supplier.example/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .expect("base company should be valid")
    .with_independent_research_sources(vec![independent_url.to_owned()])
    .expect("independent source should validate");
    let client = FixtureHttpClient::new();
    client.insert(
        independent_url,
        HttpResponse::ok(
            r#"<title>AI operations disclosure</title><time datetime="2026-02-25"></time><p>Acme rolled out an agent workflow across production operations.</p>"#,
        ),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let document = observations
        .iter()
        .find(|observation| {
            observation.kind() == SourceKind::IndependentResearch
                && observation.material_kind()
                    == org_x::features::weekly_radar::runtime::sources::SourceMaterialKind::Document
                && observation.url() == Some(independent_url)
        })
        .expect("explicit independent document should be collected");

    assert_eq!(document.tier(), SourceTier::IndependentPrimary);
    assert!(client
        .requests()
        .iter()
        .any(|request| request.url() == independent_url));
    assert!(!client
        .requests()
        .iter()
        .any(|request| request.url().contains("guessed")));
}

#[test]
fn explicit_atos_press_document_is_collected_without_guessing_urls() {
    let independent_url = "https://www.atosgroup.com/en/press/atos-group-and-microsoft-expand-strategic-collaboration-scale-secure-agentic-ai-across-atos";
    let company = CompanyConfig::new(
        "msft",
        "Microsoft Corporation",
        "MSFT",
        None,
        Some("https://supplier.example/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .expect("base company should be valid")
    .with_independent_research_sources(vec![independent_url.to_owned()])
    .expect("Atos source should validate");
    let client = FixtureHttpClient::new();
    client.insert(
        independent_url,
        HttpResponse::ok(
            r#"<html><head><title>Atos Group and Microsoft expand strategic collaboration</title><meta property="article:published_time" content="2026-06-09T00:00:00Z"></head><body><p>Atos is rolling out Microsoft 365 Copilot to all employees across 54 countries.</p></body></html>"#,
        ),
    );

    let observations = collect_configured_sources(&company, &client, Utc::now());
    let document = observations
        .iter()
        .find(|observation| {
            observation.kind() == SourceKind::IndependentResearch
                && observation.material_kind()
                    == org_x::features::weekly_radar::runtime::sources::SourceMaterialKind::Document
                && observation.url() == Some(independent_url)
        })
        .expect("explicit Atos press source should become a document");

    assert_eq!(document.tier(), SourceTier::IndependentPrimary);
    assert!(extract_evidence_candidate(document).is_some());
    assert!(!client
        .requests()
        .iter()
        .any(|request| request.url().contains("/press/guess")));
}

#[test]
fn configured_source_urls_reject_unsafe_destinations_without_leaking_values() {
    let invalid_urls = [
        "https://",
        "https://user:super-secret@example.com/source",
        "https://example.com/source#fragment",
        "https://localhost/source",
        "https://service.local/source",
        "https://service.internal/source",
        "https://service.lan/source",
        "https://service.home.arpa/source",
        "https://127.0.0.1/source",
        "https://10.0.0.1/source",
        "https://192.168.1.1/source",
        "https://169.254.1.1/source",
        "https://[::1]/source",
        "https://[fc00::1]/source",
        "https://[fe80::1]/source",
        "https://[::ffff:127.0.0.1]/source",
    ];

    for url in invalid_urls {
        let result = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            Some(url.to_owned()),
            None,
            None,
            None,
            None,
        );
        let error = result.expect_err("unsafe configured source URL must be rejected");
        let display = error.to_string();
        let debug = format!("{error:?}");
        assert!(!display.contains(url), "display leaked URL: {display}");
        assert!(!debug.contains(url), "debug leaked URL: {debug}");
        assert!(!display.contains("super-secret"));
        assert!(!debug.contains("super-secret"));
    }

    assert!(CompanyConfig::new(
        "acme",
        "Acme Corporation",
        "ACME",
        None,
        Some("https://example.com/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .is_ok());
    assert!(CompanyConfig::new(
        "fixture",
        "Fixture Corporation",
        "FIX",
        None,
        Some("https://example.test/investors".to_owned()),
        None,
        None,
        None,
        None,
    )
    .is_ok());
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
    assert_eq!(
        registry.company("msft").unwrap().independent_research_source_urls(),
        [
            "https://www.pwc.com/us/en/library/case-studies/pwc-microsoft-copilot-enterprise-ai.html",
        "https://www.atosgroup.com/en/press/atos-group-and-microsoft-expand-strategic-collaboration-scale-secure-agentic-ai-across-atos"
        ]
    );
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
fn source_collection_marks_gdelt_not_applicable_without_configured_source_endpoints() {
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

    assert_eq!(
        observations
            .iter()
            .find(|observation| observation.kind() == SourceKind::Gdelt)
            .map(|observation| observation.status()),
        Some(SourceStatus::NotApplicable)
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
    assert_eq!(official_fact.status(), &FactStatus::Unconfirmed);
    assert_eq!(official_fact.value(), None);
    assert!(official_fact
        .provenance()
        .source_field_or_passage()
        .contains("official page text"));

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
fn absent_optional_sources_are_not_configured_without_guessed_urls() {
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
        assert_eq!(observation.status(), SourceStatus::NotConfigured);
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

#[test]
fn sec_company_facts_accepts_a_payload_above_the_default_http_limit() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let user_agent = "ORG-X weekly-radar test contact@example.test";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(r#"{"filings":{"recent":{}}}"#),
    );
    let facts = serde_json::json!({
        "padding": "x".repeat(1_048_577),
        "facts": {
            "us-gaap": {
                "RevenueFromContractWithCustomerExcludingAssessedTax": {
                    "units": {
                        "USD": [{
                            "start": "2024-01-01",
                            "end": "2024-12-31",
                            "val": 100,
                            "accn": "000123456725000001",
                            "fy": 2024,
                            "fp": "FY",
                            "form": "10-K",
                            "filed": "2025-02-15"
                        }]
                    }
                }
            }
        }
    });
    let facts_body = serde_json::to_string(&facts).expect("large facts fixture should encode");
    assert!(facts_body.len() > 1_048_576);
    assert!(facts_body.len() < SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES);
    client.insert(facts_url, HttpResponse::ok(facts_body));

    let evidence = SecClient::collect(&company, &client, user_agent)
        .expect("SEC Company Facts above the generic limit should be parsed");

    assert_eq!(evidence.fact("revenue").unwrap().value(), Some("100"));
    assert_eq!(client.requests().len(), 2);
    assert!(client
        .requests()
        .iter()
        .all(|request| request.headers() == [("User-Agent".to_owned(), user_agent.to_owned())]));
}

#[test]
fn sec_submissions_accepts_a_payload_above_the_default_http_limit() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let user_agent = "ORG-X weekly-radar test contact@example.test";
    let client = FixtureHttpClient::new();
    let submissions = serde_json::json!({
        "padding": "x".repeat(1_048_577),
        "filings": {"recent": {}}
    });
    let submissions_body =
        serde_json::to_string(&submissions).expect("large submissions fixture should encode");
    assert!(submissions_body.len() > 1_048_576);
    assert!(submissions_body.len() < SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES);
    client.insert(submissions_url, HttpResponse::ok(submissions_body));
    client.insert(
        facts_url,
        HttpResponse::ok(
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
            }"#,
        ),
    );

    let evidence = SecClient::collect(&company, &client, user_agent)
        .expect("SEC submissions above the generic limit should be parsed");

    assert_eq!(evidence.fact("revenue").unwrap().value(), Some("100"));
    assert_eq!(client.requests().len(), 2);
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
    assert_eq!(client.requests().len(), 5);
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
fn sec_fetches_recent_filing_bodies_with_provenance_and_status() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let user_agent = "ORG-X weekly-radar test contact@example.test";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(
            r#"
            {"filings":{"recent":{
              "accessionNumber":["0001234567-25-000003","0001234567-25-000002","0001234567-25-000001"],
              "filingDate":["2025-03-01","2025-02-15","2025-01-31"],
              "reportDate":["2024-12-31","2024-12-31","2024-09-30"],
              "form":["8-K","10-K","10-Q"],
              "primaryDocument":["acme-8k.htm","acme-2024.htm","acme-q3.htm"]
            }}}
            "#,
        ),
    );
    client.insert(facts_url, HttpResponse::ok(r#"{"facts":{}}"#));
    client.insert(
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000003/acme-8k.htm",
        HttpResponse::ok(
            "<title>Acme organization update</title><time datetime=\"2025-03-01\"><p>Acme consolidated production scheduling under one platform.</p>",
        ),
    );
    client.insert(
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000002/acme-2024.htm",
        HttpResponse::ok(
            "<title>Acme annual report</title><time datetime=\"2025-02-15\"><p>Acme expanded its production automation program.</p>",
        ),
    );
    client.insert(
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000001/acme-q3.htm",
        HttpResponse::ok(
            "<title>Acme quarterly report</title><time datetime=\"2025-01-31\"><p>Acme increased engineering investment.</p>",
        ),
    );

    let evidence =
        SecClient::collect(&company, &client, user_agent).expect("SEC fixture should collect");

    assert_eq!(evidence.documents().len(), 3);
    assert!(evidence.documents().iter().all(|document| {
        document.status() == SecDocumentStatus::Known && !document.text().is_empty()
    }));
    let annual = evidence
        .documents()
        .iter()
        .find(|document| document.form() == "10-K")
        .expect("annual filing document should exist");
    assert_eq!(annual.title(), "Acme annual report");
    assert!(annual.text().contains("production automation"));
    assert_eq!(
        annual.filing_date(),
        NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()
    );
    assert_eq!(annual.report_date(), NaiveDate::from_ymd_opt(2024, 12, 31));
    assert!(client.requests().iter().any(|request| {
        request.url() == annual.source_uri()
            && request.headers() == [("User-Agent".to_owned(), user_agent.to_owned())]
    }));
}

#[test]
fn sec_filing_failure_is_independent_and_body_limit_is_finite() {
    let company = sec_test_company();
    let submissions_url = "https://data.sec.gov/submissions/CIK0001234567.json";
    let facts_url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let client = FixtureHttpClient::new();
    client.insert(
        submissions_url,
        HttpResponse::ok(
            r#"
            {"filings":{"recent":{
              "accessionNumber":["0001234567-25-000003","0001234567-25-000002","0001234567-25-000001"],
              "filingDate":["2025-03-01","2025-02-15","2025-01-31"],
              "reportDate":["2024-12-31","2024-12-31","2024-09-30"],
              "form":["8-K","10-K","10-Q"],
              "primaryDocument":["acme-8k.htm","acme-2024.htm","acme-q3.htm"]
            }}}
            "#,
        ),
    );
    client.insert(
        facts_url,
        HttpResponse::ok(
            r#"{"facts":{"us-gaap":{"RevenueFromContractWithCustomerExcludingAssessedTax":{"units":{"USD":[{"start":"2024-01-01","end":"2024-12-31","val":100,"accn":"000123456725000001","fp":"FY","form":"10-K","filed":"2025-02-15"}]}}}}}"#,
        ),
    );
    client.insert(
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000002/acme-2024.htm",
        HttpResponse::ok(format!(
            "<p>{}</p>",
            "x".repeat(SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES + 1)
        )),
    );
    client.insert(
        "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000001/acme-q3.htm",
        HttpResponse::ok("<p>Acme increased engineering investment.</p>"),
    );

    let evidence = SecClient::collect(&company, &client, "ORG-X test contact@example.test")
        .expect("partial SEC fixture should be retained");

    assert_eq!(
        evidence.fact("revenue").unwrap().status(),
        &FactStatus::Known
    );
    assert_eq!(evidence.documents().len(), 3);
    assert!(evidence.documents().iter().any(|document| {
        document.form() == "10-K" && document.status() == SecDocumentStatus::Unavailable
    }));
    assert!(evidence
        .failures()
        .iter()
        .any(|failure| failure.stage() == "filing_document"));
    assert!(evidence
        .failures()
        .iter()
        .all(|failure| !failure.reason().contains("xxxxxxxx")));
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
fn runtime_public_api_retains_structural_dimension_without_changing_fact_identity() {
    let provenance = Provenance::from_rfc3339(
        "https://example.test/evidence/workflow",
        "Acme changed its engineering workflow.",
        "2026-08-17T00:00:00Z",
        Some("2026-08-15"),
    )
    .expect("fixture provenance should be valid");
    let fact = NormalizedFact::new_with_structural_dimension(
        "acme",
        "evidence_structural_change_001",
        "Acme changed its engineering workflow.",
        Some(StructuralDimension::Workflow),
        FactStatus::Known,
        Confidence::High,
        provenance,
    )
    .expect("dimensioned fact should be valid");

    assert_eq!(
        fact.structural_dimension(),
        Some(StructuralDimension::Workflow)
    );
    assert_eq!(fact.kind(), "evidence_structural_change_001");
}

#[test]
fn runtime_metrics_round_trip_document_kind_counts_without_inference() {
    let metrics = ResearchMetrics::new(1, 2, 0, 2, 0).with_document_kind_counts(
        std::collections::BTreeMap::from([
            ("engineering".to_owned(), 1),
            ("earnings".to_owned(), 1),
        ]),
    );

    let serialized = serde_json::to_value(&metrics).expect("metrics should serialize");
    let restored: ResearchMetrics =
        serde_json::from_value(serialized).expect("metrics should deserialize");

    assert_eq!(restored.document_kind_counts().get("engineering"), Some(&1));
    assert_eq!(restored.document_candidates(), 2);
    assert_eq!(restored.validated_evidence(), 0);
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
fn ureq_does_not_follow_redirects() {
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
            "HTTP/1.1 302 Found\r\nLocation: /final\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )
        .expect("redirect response should be written");
    });

    let url = format!("http://{address}/start");
    let response = UreqHttpClient::new()
        .get(&url, &[])
        .expect("redirect should be returned as a response");

    assert_eq!(response.status(), 302);
    server.join().expect("redirect server should finish");
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

#[test]
fn fixture_rejects_sec_company_facts_above_its_finite_limit() {
    let url = "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json";
    let oversized = "x".repeat(SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES + 1);
    let client = FixtureHttpClient::with_response(url, HttpResponse::ok(oversized));

    let error = client
        .get_with_max_body_bytes(url, &[], SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES)
        .expect_err("SEC Company Facts above its source limit must fail closed");

    assert_eq!(error, RuntimeError::HttpResponseTooLarge);
}

#[test]
fn ureq_accepts_a_payload_above_the_default_limit_when_given_a_finite_override() {
    let body = "x".repeat(1_048_577);
    let listener = TcpListener::bind("127.0.0.1:0").expect("local listener should bind");
    let address = listener
        .local_addr()
        .expect("local listener should expose an address");
    let server_body = body.clone();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("local client should connect");
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            server_body.len()
        )
        .expect("local response headers should be written");
        stream
            .write_all(server_body.as_bytes())
            .expect("local response body should be written");
    });
    let url = format!("http://{address}/sec-company-facts");
    let response = UreqHttpClient::new()
        .get_with_max_body_bytes(&url, &[], SEC_COMPANY_FACTS_MAX_RESPONSE_BODY_BYTES)
        .expect("finite SEC override should accept the larger local body");

    assert_eq!(response.body().len(), body.len());
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
            "acme",
            "evidence_official_material_001",
            Some("Validated production workflow claim"),
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

fn legacy_input_snapshot_id(input: &RuntimeReportInput) -> String {
    let bytes = serde_json::to_vec(input).expect("legacy input should be serializable");
    let mut legacy_bytes = Vec::with_capacity(bytes.len());
    let mut cursor = 0;
    let field = b",\"not_applicable\":0";
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(field) {
            cursor += field.len();
        } else {
            legacy_bytes.push(bytes[cursor]);
            cursor += 1;
        }
    }
    let mut digest = 14_695_981_039_346_656_037_u64;
    for byte in legacy_bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    format!("wr-input-{digest:016x}")
}

fn historical_input_snapshot_id_with_explicit_zero_not_applicable(
    input: &RuntimeReportInput,
) -> String {
    let bytes = serde_json::to_vec(input).expect("historical input should be serializable");
    let marker = b",\"not_configured\":";
    let field = b",\"not_applicable\":0";
    let mut historical_bytes = Vec::with_capacity(bytes.len() + field.len() * 3);
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor..].starts_with(marker) {
            let value_start = cursor + marker.len();
            let mut value_end = value_start;
            while bytes.get(value_end).is_some_and(u8::is_ascii_digit) {
                value_end += 1;
            }
            if bytes.get(value_end) == Some(&b'}') {
                historical_bytes.extend_from_slice(&bytes[cursor..value_end]);
                historical_bytes.extend_from_slice(field);
                cursor = value_end;
                continue;
            }
        }
        historical_bytes.push(bytes[cursor]);
        cursor += 1;
    }

    let mut digest = 14_695_981_039_346_656_037_u64;
    for byte in historical_bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    format!("wr-input-{digest:016x}")
}

fn rewrite_snapshot_with_historical_identity(path: &Path) -> String {
    let content = fs::read_to_string(path).expect("snapshot should be readable");
    let mut snapshot: Value = serde_json::from_str(&content).expect("snapshot should be JSON");
    let input: RuntimeReportInput = serde_json::from_value(
        snapshot
            .get("input")
            .cloned()
            .expect("snapshot should contain input"),
    )
    .expect("historical input should decode");
    let snapshot_id = historical_input_snapshot_id_with_explicit_zero_not_applicable(&input);
    snapshot["snapshot_id"] = Value::String(snapshot_id.clone());
    let rewritten = serde_json::to_string_pretty(&snapshot).expect("snapshot should encode") + "\n";
    fs::write(path, rewritten).expect("historical snapshot should be written");
    snapshot_id
}

fn rewrite_snapshot_as_legacy(path: &Path) -> String {
    let content = fs::read_to_string(path).expect("snapshot should be readable");
    let mut snapshot: Value = serde_json::from_str(&content).expect("snapshot should be JSON");
    let coverage = snapshot
        .get_mut("input")
        .and_then(|input| input.get_mut("source_coverage"))
        .and_then(Value::as_array_mut)
        .expect("snapshot should contain source coverage");
    for entry in coverage {
        entry
            .as_object_mut()
            .expect("source coverage entry should be an object")
            .remove("not_applicable");
    }
    let input: RuntimeReportInput = serde_json::from_value(
        snapshot
            .get("input")
            .cloned()
            .expect("snapshot should contain input"),
    )
    .expect("legacy input should decode");
    let snapshot_id = legacy_input_snapshot_id(&input);
    snapshot["snapshot_id"] = Value::String(snapshot_id.clone());
    let rewritten =
        serde_json::to_string_pretty(&snapshot).expect("legacy snapshot should encode") + "\n";
    fs::write(path, rewritten).expect("legacy snapshot should be written");
    snapshot_id
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
    assert_eq!(headings.first(), Some(&"本周摘要"));
    assert_eq!(headings.last(), Some(&"系统状态"));
    assert!(headings.iter().all(|heading| {
        matches!(
            *heading,
            "本周摘要"
                | "已验证事实"
                | "结构性证据"
                | "结构性变化证据"
                | "重点公司"
                | "Rising"
                | "Dropped"
                | "系统状态"
        )
    }));
    assert!(first.markdown().matches("### ").count() <= 8);
    assert!(first.markdown().contains("## 结构性变化证据"));
    assert!(first.markdown().contains("## 重点公司"));
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
    assert!(report.markdown().contains("不生成排名"));
}

#[test]
fn task4_empty_report_states_that_no_evidence_was_supplied() {
    let input = RuntimeReportInput::new("2026-08-17").expect("empty report date should be valid");
    let report = render_report(&input);

    assert!(report
        .markdown()
        .contains("主证据：本周没有可作为主证据的已验证事实"));
}

#[test]
fn task4_report_exposes_explicit_statuses_and_discovery_health_review_items() {
    let report = task4_report();
    let markdown = report.markdown();

    assert!(report.snapshot_json().contains("CONFIRMED"));
    assert!(report.snapshot_json().contains("UNKNOWN"));
    assert!(report.snapshot_json().contains("UNAVAILABLE"));
    assert!(report.snapshot_json().contains("UNCONFIRMED"));
    assert!(markdown.contains("待验证线索"));
    assert!(markdown.contains("来源情况"));
    assert!(markdown.contains("新闻和其他发现材料"));
    assert!(!markdown.contains("source_"));
}

#[test]
fn task8_report_expands_each_validated_evidence_fact_with_date_and_direct_evidence() {
    let report = task4_report();
    let markdown = report.markdown();
    let confirmed_section = markdown
        .split("## 已验证事实")
        .nth(1)
        .and_then(|section| section.split("\n## ").next())
        .expect("confirmed information section should be rendered");

    assert!(confirmed_section.contains("### acme"));
    assert!(confirmed_section.contains("- 信息类型：其他资料"));
    assert!(confirmed_section.contains("- 事实：Validated production workflow claim"));
    assert!(!confirmed_section.contains("- 信息类型：营收"));
    assert!(!confirmed_section.contains("- 事实：123000000"));
    assert!(confirmed_section.contains("- 日期：2026-08-15"));
    assert_eq!(
        confirmed_section
            .matches("- 证据：https://example.test/evidence/2026")
            .count(),
        1,
        "every validated evidence fact needs its own direct evidence link"
    );
}

#[test]
fn task8_report_does_not_turn_unavailable_structural_evidence_into_no_change() {
    let mut input = RuntimeReportInput::new("2026-08-17").expect("date should be valid");
    input
        .add_company(CompanyIdentity::new("acme", "Acme Corporation", "ACME").unwrap())
        .unwrap();
    input
        .add_fact(
            NormalizedFact::without_value(
                "acme",
                "structural_change",
                FactStatus::Unavailable,
                Confidence::Unknown,
                task4_provenance("official source unavailable"),
            )
            .unwrap(),
        )
        .unwrap();

    let markdown = render_report(&input).markdown().to_owned();
    assert!(markdown.contains("无法据此确认本周没有组织变化"));
    assert!(!markdown.contains("本周没有发现已确认的组织结构变化。"));
}

#[test]
fn task7_default_report_is_chinese_and_hides_runtime_diagnostics_from_readers() {
    let report = render_report(&task4_report_input());
    let markdown = report.markdown();

    assert!(markdown.contains("## 本周摘要"));
    assert!(markdown.contains("## 系统状态"));
    assert!(!markdown.contains("source_"));
    assert!(!markdown.contains("UNAVAILABLE"));
    assert!(!markdown.contains("UNCONFIRMED"));
    assert!(!markdown.contains("official: 5/5"));
    assert!(report.snapshot_json().contains("UNAVAILABLE"));
    assert!(report.snapshot_json().contains("source_"));
}

#[test]
fn task7_report_supports_japanese_and_english_without_changing_snapshot_facts() {
    let input = task4_report_input();
    let japanese = render_report_in_language(&input, ReportLanguage::Japanese);
    let english = render_report_in_language(&input, ReportLanguage::English);

    assert!(japanese.markdown().contains("## 週次サマリー"));
    assert!(japanese.markdown().contains("## システム状態"));
    assert!(english.markdown().contains("## Executive Summary"));
    assert!(english.markdown().contains("## System Health"));
    assert_eq!(japanese.metadata(), english.metadata());
    assert!(japanese.snapshot_json().contains("UNAVAILABLE"));
    assert!(english.snapshot_json().contains("UNCONFIRMED"));
}

#[test]
fn task7_unavailable_first_fact_never_becomes_evidence_basis() {
    let mut input = RuntimeReportInput::new("2026-08-17").expect("date should be valid");
    input
        .add_fact(
            NormalizedFact::without_value(
                "acme",
                "source_careers_001",
                FactStatus::Unavailable,
                Confidence::Unknown,
                task4_provenance("official page request unavailable"),
            )
            .expect("unavailable fact should be valid"),
        )
        .expect("fact should be unique");

    let report = render_report(&input);
    assert!(report
        .markdown()
        .contains("本周没有可作为主证据的已验证事实"));
    assert!(!report
        .markdown()
        .contains("official page request unavailable"));
}

#[test]
fn task7_report_groups_source_failures_and_configuration_gaps_for_readers() {
    let mut input = RuntimeReportInput::new("2026-08-17").expect("date should be valid");
    input
        .add_company(CompanyIdentity::new("acme", "Acme Corporation", "ACME").unwrap())
        .unwrap();
    input
        .add_source_coverage(
            SourceCoverage::new_with_not_configured("greenhouse", 1, 0, 1)
                .expect("coverage should be valid"),
        )
        .unwrap();
    input
        .add_source_failure(
            SourceFailure::new("sec", "acme", "HTTP response was unavailable").unwrap(),
        )
        .unwrap();

    let report = render_report(&input);
    let markdown = report.markdown();
    assert!(markdown.contains("SEC 财务与申报资料"));
    assert!(markdown.contains("返回资料不可用"));
    assert!(markdown.contains("Greenhouse 招聘接口：尚未配置"));
    assert!(!markdown.contains("source_sec"));
    assert!(!markdown.contains("HTTP response was unavailable"));
    assert!(report
        .snapshot_json()
        .contains("HTTP response was unavailable"));
    assert!(report.snapshot_json().contains("not_configured"));
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
    assert!(sent[0].contains("主证据："));
    assert!(sent
        .join("\n")
        .contains("https://example.test/evidence/2026"));
    let total_pages = sent.len();
    for (index, page) in sent.iter().enumerate() {
        assert!(
            page.starts_with(&format!("{}/{}\n", index + 1, total_pages)),
            "page {} should start with its one-based number and total",
            index + 1
        );
    }
    let unnumbered = sent
        .iter()
        .map(|page| {
            page.split_once('\n')
                .expect("numbered page should contain a header newline")
                .1
        })
        .collect::<String>();
    assert_eq!(unnumbered, report.markdown());
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

#[test]
fn task5_input_snapshot_round_trips_and_is_idempotent() {
    let root = task4_temp_root("input-snapshot");
    let input = task4_report_input();
    let first = persist_input_snapshot(&root, "data", &input, ReportLanguage::Japanese, true)
        .expect("input snapshot should persist");
    let second = persist_input_snapshot(&root, "data", &input, ReportLanguage::Japanese, true)
        .expect("identical input snapshot should be idempotent");

    assert_eq!(first, second);
    let loaded =
        load_input_snapshot(&root, "data", input.as_of()).expect("input snapshot should load");
    assert_eq!(loaded.input(), &input);
    assert_eq!(loaded.language(), ReportLanguage::Japanese);
    assert!(loaded.has_primary_evidence());
    assert!(loaded.snapshot_id().starts_with("wr-input-"));

    fs::remove_dir_all(root).expect("input snapshot fixture should be removable");
}

#[test]
fn task5_legacy_input_snapshot_without_not_applicable_remains_verifiable() {
    let root = task4_temp_root("legacy-input-snapshot");
    let input = task4_report_input();
    persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("input snapshot should persist");
    let path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let legacy_id = rewrite_snapshot_as_legacy(&path);

    let loaded = load_input_snapshot(&root, "data", input.as_of())
        .expect("legacy input snapshot should remain verifiable");
    assert_eq!(loaded.snapshot_id(), legacy_id);
    assert!(loaded
        .input()
        .source_coverage()
        .iter()
        .all(|coverage| coverage.not_applicable() == 0));

    fs::remove_dir_all(root).expect("legacy snapshot fixture should be removable");
}

#[test]
fn task5_historical_input_snapshot_identity_allows_same_day_replacement() {
    let root = task4_temp_root("historical-input-snapshot-replacement");
    let input = task4_report_input();
    let current_snapshot =
        persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
            .expect("input snapshot should persist");
    let input_path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let historical_id = rewrite_snapshot_with_historical_identity(&input_path);
    let historical_bytes = fs::read(&input_path).expect("historical snapshot bytes should exist");
    assert_ne!(current_snapshot.snapshot_id(), historical_id);

    let historical_snapshot = load_input_snapshot(&root, "data", input.as_of())
        .expect("historical input snapshot should remain verifiable");
    assert_eq!(historical_snapshot.snapshot_id(), historical_id);
    let first_report =
        render_report_in_language(historical_snapshot.input(), historical_snapshot.language());
    let first_receipt = send_rendered_report_with_transport(
        &first_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("historical report should have a receipt");
    write_run_with_input_snapshot(
        &root,
        "data",
        &first_report,
        &first_receipt,
        Some(&historical_snapshot),
    )
    .expect("historical snapshot run should archive");
    ensure_run_replace_available(&root, "data", input.as_of())
        .expect("verified historical run should allow same-day replacement");
    assert_eq!(
        fs::read(&input_path).expect("historical snapshot should remain readable"),
        historical_bytes
    );

    let mut replacement_input = input.clone();
    replacement_input
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("replacement fact should be valid"),
        )
        .expect("replacement fact should be unique");
    let replacement_snapshot =
        build_input_snapshot(&replacement_input, ReportLanguage::Chinese, true)
            .expect("replacement snapshot should be buildable");
    let replacement_report = render_report_in_language(&replacement_input, ReportLanguage::Chinese);
    let replacement_receipt = send_rendered_report_with_transport(
        &replacement_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("replacement report should have a receipt");
    let manifest = replace_run_with_input_snapshot(
        &root,
        "data",
        &replacement_report,
        &replacement_receipt,
        &replacement_snapshot,
    )
    .expect("same-day replacement should complete after historical verification");
    assert_eq!(
        manifest.snapshot_id(),
        Some(replacement_snapshot.snapshot_id())
    );

    fs::remove_dir_all(root).expect("historical replacement fixture should be removable");
}

#[test]
fn task5_new_not_applicable_input_snapshot_round_trips_with_bound_identity() {
    let root = task4_temp_root("new-input-snapshot");
    let mut input = RuntimeReportInput::new("2026-08-17").expect("date should be valid");
    input
        .add_source_coverage(
            SourceCoverage::new_with_states("gdelt-discovery", 5, 1, 0, 1)
                .expect("coverage should be valid"),
        )
        .expect("coverage should be retained");

    let snapshot = persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("new input snapshot should persist");
    let path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let content = fs::read_to_string(path).expect("new snapshot should be readable");
    assert!(content.contains("\"not_applicable\": 1"));

    let loaded = load_input_snapshot(&root, "data", input.as_of())
        .expect("new input snapshot should round-trip");
    assert_eq!(loaded, snapshot);
    assert_eq!(loaded.input().source_coverage()[0].not_applicable(), 1);

    fs::remove_dir_all(root).expect("new snapshot fixture should be removable");
}

#[test]
fn task5_tampered_input_snapshot_is_rejected_before_archive_side_effects() {
    let root = task4_temp_root("tampered-input-snapshot");
    let input = task4_report_input();
    persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("input snapshot should persist");
    let path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let content = fs::read_to_string(&path).expect("snapshot should be readable");
    let mut snapshot: Value = serde_json::from_str(&content).expect("snapshot should be JSON");
    snapshot["input"]["facts"][0]["value"] = Value::String("tampered".to_owned());
    fs::write(
        &path,
        serde_json::to_string_pretty(&snapshot).expect("tampered snapshot should encode") + "\n",
    )
    .expect("tampered snapshot should be written");

    let error = load_input_snapshot(&root, "data", input.as_of())
        .expect_err("tampered snapshot must be rejected");
    assert!(matches!(
        error,
        ArchiveError::InvalidInputSnapshot {
            reason: "input identity does not match content"
        }
    ));
    assert!(!root.join("weekly-radar/reports").exists());
    assert!(!root.join("weekly-radar/receipts").exists());

    fs::remove_dir_all(root).expect("tampered snapshot fixture should be removable");
}

#[test]
fn task5_read_only_verification_accepts_a_legacy_input_snapshot() {
    let root = task4_temp_root("legacy-committed-run");
    let input = task4_report_input();
    persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("input snapshot should persist");
    let input_path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let legacy_id = rewrite_snapshot_as_legacy(&input_path);
    let legacy_snapshot = load_input_snapshot(&root, "data", input.as_of())
        .expect("legacy snapshot should load before archive verification");
    let report = render_report_in_language(legacy_snapshot.input(), legacy_snapshot.language());
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed");
    write_run_with_input_snapshot(&root, "data", &report, &receipt, Some(&legacy_snapshot))
        .expect("legacy snapshot run should archive");

    let manifest = org_x::features::weekly_radar::runtime::archive::verify_committed_run_read_only(
        &root,
        "data",
        input.as_of(),
    )
    .expect("read-only verification should accept the legacy snapshot");
    assert_eq!(manifest.snapshot_id(), Some(legacy_id.as_str()));

    fs::remove_dir_all(root).expect("legacy committed fixture should be removable");
}

#[test]
fn task5_input_snapshot_rejects_a_same_date_conflict_without_mutation() {
    let root = task4_temp_root("input-snapshot-conflict");
    let original = task4_report_input();
    persist_input_snapshot(&root, "data", &original, ReportLanguage::Chinese, true)
        .expect("original input snapshot should persist");
    let path = root.join("weekly-radar/snapshots/2026-08-17.input.json");
    let before = fs::read(&path).expect("original bytes should exist");

    let mut different = original.clone();
    different
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("distinct fixture fact should be valid"),
        )
        .expect("distinct fixture fact should be unique");
    let error = persist_input_snapshot(&root, "data", &different, ReportLanguage::Chinese, true)
        .expect_err("conflicting input must be rejected");

    assert!(matches!(error, ArchiveError::InputSnapshotConflict { .. }));
    assert_eq!(
        fs::read(&path).expect("snapshot should remain").as_slice(),
        before
    );
    fs::remove_dir_all(root).expect("input conflict fixture should be removable");
}

#[test]
fn task5_archive_rejects_same_date_overwrite_without_mutation() {
    let root = task4_temp_root("same-date-overwrite");
    let first_input = task4_report_input();
    let first_snapshot =
        persist_input_snapshot(&root, "data", &first_input, ReportLanguage::Chinese, true)
            .expect("first input snapshot should persist");
    let first_report = render_report_in_language(&first_input, ReportLanguage::Chinese);
    let first_receipt = send_rendered_report_with_transport(
        &first_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("first report should have a receipt");
    write_run_with_input_snapshot(
        &root,
        "data",
        &first_report,
        &first_receipt,
        Some(&first_snapshot),
    )
    .expect("first report should archive");

    let paths = [
        root.join("weekly-radar/reports/2026-08-17.md"),
        root.join("weekly-radar/snapshots/2026-08-17.json"),
        root.join("weekly-radar/receipts/2026-08-17.json"),
        root.join("weekly-radar/manifest.json"),
    ];
    let before = paths
        .iter()
        .map(|path| fs::read(path).expect("first archive bytes should exist"))
        .collect::<Vec<_>>();

    let mut different_input = first_input.clone();
    different_input
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("second fixture fact should be valid"),
        )
        .expect("second fixture fact should be unique");
    let different_report = render_report(&different_input);
    let different_receipt = send_rendered_report_with_transport(
        &different_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("second report should have a receipt");
    let error = write_run(&root, "data", &different_report, &different_receipt)
        .expect_err("same-date final run must be rejected");

    assert!(matches!(error, ArchiveError::ExistingRun { .. }));
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(
            fs::read(path).expect("existing archive should remain"),
            expected
        );
    }
    ensure_run_available(&root, "data", first_report.as_of())
        .expect_err("existing date must not be available");
    fs::remove_dir_all(root).expect("same-date fixture should be removable");
}

#[test]
fn task5_archive_replacement_makes_the_last_successful_update_canonical() {
    let root = task4_temp_root("same-date-replacement");
    let first_input = task4_report_input();
    let first_snapshot =
        persist_input_snapshot(&root, "data", &first_input, ReportLanguage::Chinese, true)
            .expect("first input snapshot should persist");
    let first_report = render_report_in_language(&first_input, ReportLanguage::Chinese);
    let first_receipt = send_rendered_report_with_transport(
        &first_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("first report should have a receipt");
    write_run_with_input_snapshot(
        &root,
        "data",
        &first_report,
        &first_receipt,
        Some(&first_snapshot),
    )
    .expect("first report should archive");

    let mut second_input = first_input.clone();
    second_input
        .add_fact(
            NormalizedFact::new(
                "omega",
                "revenue",
                "99000000",
                FactStatus::Known,
                Confidence::Medium,
                task4_provenance("facts.revenue"),
            )
            .expect("second fixture fact should be valid"),
        )
        .expect("second fixture fact should be unique");
    let second_snapshot = build_input_snapshot(&second_input, ReportLanguage::Chinese, true)
        .expect("second input snapshot should be buildable without persistence");
    let second_report = render_report_in_language(&second_input, ReportLanguage::Chinese);
    let second_receipt = send_rendered_report_with_transport(
        &second_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("second report should have a receipt");

    let manifest = replace_run_with_input_snapshot(
        &root,
        "data",
        &second_report,
        &second_receipt,
        &second_snapshot,
    )
    .expect("the last successful same-day update should replace the prior canonical set");

    assert_eq!(manifest.snapshot_id(), Some(second_snapshot.snapshot_id()));
    assert_eq!(
        load_input_snapshot(&root, "data", second_input.as_of())
            .expect("replacement input snapshot should be persisted")
            .snapshot_id(),
        second_snapshot.snapshot_id()
    );
    assert_eq!(
        fs::read_to_string(root.join("weekly-radar/reports/2026-08-17.md"))
            .expect("canonical report should be readable"),
        second_report.markdown()
    );
    assert_eq!(
        org_x::features::weekly_radar::runtime::archive::verify_committed_run_read_only(
            &root,
            "data",
            second_input.as_of(),
        )
        .expect("replacement archive should verify"),
        manifest
    );

    fs::remove_dir_all(root).expect("same-date replacement fixture should be removable");
}

#[test]
fn task5_archive_replacement_rejects_corrupt_old_canonical_before_overwrite() {
    let root = task4_temp_root("same-date-replacement-corrupt-old");
    let first_input = task4_report_input();
    let first_snapshot =
        persist_input_snapshot(&root, "data", &first_input, ReportLanguage::Chinese, true)
            .expect("first input snapshot should persist");
    let first_report = render_report_in_language(&first_input, ReportLanguage::Chinese);
    let first_receipt = send_rendered_report_with_transport(
        &first_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("first report should have a receipt");
    write_run_with_input_snapshot(
        &root,
        "data",
        &first_report,
        &first_receipt,
        Some(&first_snapshot),
    )
    .expect("first report should archive");
    let report_path = root.join("weekly-radar/reports/2026-08-17.md");
    let receipt_path = root.join("weekly-radar/receipts/2026-08-17.json");
    let transaction_path = root.join("weekly-radar/.transactions/2026-08-17.json");
    let old_report = fs::read(&report_path).expect("old report should be readable");
    let old_receipt = fs::read(&receipt_path).expect("old receipt should be readable");
    let old_transaction = fs::read(&transaction_path).expect("old transaction should be readable");
    fs::write(
        root.join("weekly-radar/snapshots/2026-08-17.json"),
        "corrupt old snapshot\n",
    )
    .expect("fixture should corrupt the old snapshot");

    let second_snapshot = build_input_snapshot(&first_input, ReportLanguage::Chinese, true)
        .expect("replacement input snapshot should be buildable");
    let second_report =
        render_report_in_language(second_snapshot.input(), second_snapshot.language());
    let second_receipt = send_rendered_report_with_transport(
        &second_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("replacement fixture should have a receipt");
    let error = replace_run_with_input_snapshot(
        &root,
        "data",
        &second_report,
        &second_receipt,
        &second_snapshot,
    )
    .expect_err("a corrupt old canonical must block replacement before overwrite");

    assert!(matches!(error, ArchiveError::IncompleteRun { .. }));
    assert_eq!(
        fs::read(report_path).expect("old report should remain"),
        old_report
    );
    assert_eq!(
        fs::read(receipt_path).expect("old receipt should remain"),
        old_receipt
    );
    assert_eq!(
        fs::read(transaction_path).expect("old transaction should remain"),
        old_transaction
    );
    fs::remove_dir_all(root).expect("corrupt replacement fixture should be removable");
}

#[test]
fn archive_transaction_fails_closed_for_partial_residue_and_keeps_legacy_runs_immutable() {
    let root = task4_temp_root("archive-transaction-residue");
    let archive = root.join("weekly-radar");
    let reports = archive.join("reports");
    let snapshots = archive.join("snapshots");
    let receipts = archive.join("receipts");
    fs::create_dir_all(&reports).expect("report directory should exist");
    fs::create_dir_all(&snapshots).expect("snapshot directory should exist");
    fs::create_dir_all(&receipts).expect("receipt directory should exist");
    let as_of = NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date is valid");
    let report_path = reports.join("2026-08-17.md");
    fs::write(&report_path, "partial report").expect("partial report should be written");

    let error = ensure_run_available(&root, "data", as_of)
        .expect_err("one final artifact must be incomplete, not a committed run");
    assert!(matches!(error, ArchiveError::IncompleteRun { .. }));
    assert_eq!(fs::read(&report_path).unwrap(), b"partial report");
    assert!(matches!(
        recover_pending_run(&root, "data", as_of),
        Err(ArchiveError::IncompleteRun { .. })
    ));

    fs::write(snapshots.join("2026-08-17.json"), "legacy snapshot")
        .expect("legacy snapshot should be written");
    fs::write(receipts.join("2026-08-17.json"), "legacy receipt")
        .expect("legacy receipt should be written");
    let error = ensure_run_available(&root, "data", as_of)
        .expect_err("complete legacy final files must remain protected");
    assert!(matches!(error, ArchiveError::ExistingRun { .. }));
    assert_eq!(fs::read(&report_path).unwrap(), b"partial report");
    fs::remove_dir_all(root).expect("residue fixture should be removable");
}

#[test]
fn task5_archive_retention_waits_until_after_a_successful_commit() {
    let root = task4_temp_root("retention-order");
    let old_path = root.join("weekly-radar/reports/2025-01-01.md");
    fs::create_dir_all(old_path.parent().unwrap()).expect("archive directory should exist");
    fs::write(&old_path, "old report").expect("old report should be seeded");

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
            .expect("mismatched receipt fact should be valid"),
        )
        .expect("mismatched receipt fact should be unique");
    let other_report = render_report(&other_input);
    let invalid_receipt = send_rendered_report_with_transport(
        &other_report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("mismatched receipt fixture should be available");
    let error = write_run(&root, "data", &report, &invalid_receipt)
        .expect_err("mismatched receipt should fail before retention");
    assert!(matches!(error, ArchiveError::ReportIdMismatch { .. }));
    assert!(old_path.exists(), "failed commit must not run retention");

    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("valid receipt should be available");
    write_run(&root, "data", &report, &receipt).expect("successful run should commit");
    assert!(
        !old_path.exists(),
        "retention should run after a successful commit"
    );
    fs::remove_dir_all(root).expect("retention fixture should be removable");
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
    assert!(String::from_utf8_lossy(&output.stdout).contains("## 系统状态"));
    assert!(String::from_utf8_lossy(&output.stdout).contains("投资者关系资料"));
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
fn task5_cli_rejects_retry_with_incompatible_options() {
    for recovery_option in [
        "--retry-as-of",
        "--recover-published-as-of",
        "--verify-published-as-of",
        "--republish-published-as-of",
    ] {
        for extra in [
            vec!["--as-of", "2026-08-17"],
            vec!["--language", "ja"],
            vec!["--dry-run"],
        ] {
            let mut args = vec!["weekly-radar", recovery_option, "2026-08-17"];
            args.extend(extra);
            let output = task5_cli(&args);
            assert!(
                !output.status.success(),
                "incompatible recovery options must fail"
            );
            assert!(String::from_utf8_lossy(&output.stderr).contains("recovery options"));
        }
    }
}

#[test]
fn task5_cli_retry_uses_persisted_input_without_source_acquisition() {
    let root = task5_cli_fixture_root("retry-without-acquisition");
    let archive = root.join("archive");
    persist_input_snapshot(
        &archive,
        "data",
        &task4_report_input(),
        ReportLanguage::English,
        true,
    )
    .expect("retry input fixture should persist");
    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            archive.to_str().unwrap(),
            "--retry-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("org-x binary should be executable");

    assert!(
        !output.status.success(),
        "retry without Telegram should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ORGX_TELEGRAM_BOT_TOKEN") || stderr.contains("Telegram"),
        "retry should reach delivery configuration: {stderr}"
    );
    assert!(!stderr.contains("ORGX_SEC_USER_AGENT"));
    fs::remove_dir_all(root).expect("retry fixture should be removable");
}

#[test]
fn task5_cli_reports_a_verified_final_run_as_an_idempotent_success() {
    let root = task5_cli_fixture_root("already-published");
    let input = task4_report_input();
    let snapshot = persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("published input should persist");
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed");
    write_run_with_input_snapshot(&root, "data", &report, &receipt, Some(&snapshot))
        .expect("fixture publication should commit");

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--verify-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("already-published CLI should be executable");

    assert!(
        output.status.success(),
        "a verified final run should be a successful no-op: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ALREADY-PUBLISHED:"));
    assert!(fs::read_to_string(root.join("weekly-radar/manifest.json"))
        .expect("manifest should remain readable")
        .contains("2026-08-17"));
    fs::remove_dir_all(root).expect("already-published fixture should be removable");
}

#[test]
fn task5_cli_republish_requires_telegram_but_preserves_archive() {
    let root = task5_cli_fixture_root("republish-missing-telegram");
    let input = task4_report_input();
    let snapshot = persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("published input should persist");
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed");
    write_run_with_input_snapshot(&root, "data", &report, &receipt, Some(&snapshot))
        .expect("fixture publication should commit");

    let paths = [
        root.join("weekly-radar/snapshots/2026-08-17.input.json"),
        root.join("weekly-radar/reports/2026-08-17.md"),
        root.join("weekly-radar/snapshots/2026-08-17.json"),
        root.join("weekly-radar/receipts/2026-08-17.json"),
        root.join("weekly-radar/manifest.json"),
    ];
    let before = paths
        .iter()
        .map(|path| fs::read(path).expect("republish fixture artifact should be readable"))
        .collect::<Vec<_>>();
    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--republish-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("republish CLI should be executable");

    assert!(
        !output.status.success(),
        "republish needs Telegram credentials"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Telegram bot credentials"));
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(
            fs::read(path).expect("republish artifact should remain readable"),
            expected,
            "republish must not mutate {}",
            path.display()
        );
    }
    fs::remove_dir_all(root).expect("republish fixture should be removable");
}

#[test]
fn task5_cli_rejects_same_date_transaction_manifest_mismatch() {
    let root = task5_cli_fixture_root("already-published-manifest-mismatch");
    let input = task4_report_input();
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed");
    write_run(&root, "data", &report, &receipt).expect("fixture publication should commit");

    let manifest_path = root.join("weekly-radar/manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path).expect("manifest should be readable"),
    )
    .expect("manifest should be valid JSON");
    manifest["report"] = serde_json::Value::String("weekly-radar/reports/2026-08-16.md".to_owned());
    fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).expect("tampered manifest should serialize"),
    )
    .expect("tampered manifest should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--verify-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("manifest verification CLI should be executable");
    assert!(
        !output.status.success(),
        "same-date transaction manifest mismatch must fail closed"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ALREADY-PUBLISHED:"));
    fs::remove_dir_all(root).expect("manifest verification fixture should be removable");
}

#[test]
fn task5_cli_rejects_tampered_legacy_archive_without_creating_lock_metadata() {
    let root = task5_cli_fixture_root("already-published-legacy");
    let input = task4_report_input();
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &Task4RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed");
    write_run(&root, "data", &report, &receipt).expect("fixture publication should commit");
    let transaction_dir = root.join("weekly-radar/.transactions");
    fs::remove_dir_all(&transaction_dir)
        .expect("legacy fixture should remove transaction metadata");

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--verify-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("legacy verification CLI should be executable");
    assert!(
        output.status.success(),
        "a valid legacy archive should verify: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("ALREADY-PUBLISHED:"));
    assert!(
        !transaction_dir.exists(),
        "read-only verification must not create lock or transaction metadata"
    );

    let receipt_path = root.join("weekly-radar/receipts/2026-08-17.json");
    let mut receipt_json: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&receipt_path).expect("legacy receipt should be readable"),
    )
    .expect("legacy receipt should be valid JSON");
    receipt_json["report_id"] = serde_json::Value::String("wr-tampered".to_owned());
    fs::write(
        &receipt_path,
        serde_json::to_string_pretty(&receipt_json).expect("tampered receipt should serialize"),
    )
    .expect("tampered receipt should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--verify-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("tampered verification CLI should be executable");
    assert!(
        !output.status.success(),
        "tampered identity must fail closed"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ALREADY-PUBLISHED:"));

    receipt_json["report_id"] = serde_json::Value::String(receipt.report_id().to_owned());
    receipt_json["attempts"]
        .as_array_mut()
        .expect("legacy receipt attempts should be an array")
        .first_mut()
        .expect("legacy receipt should contain one attempt")
        .clone_from(&serde_json::Value::String("not-a-number".to_owned()));
    fs::write(
        &receipt_path,
        serde_json::to_string_pretty(&receipt_json).expect("malformed receipt should serialize"),
    )
    .expect("malformed receipt should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--verify-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("malformed receipt verification CLI should be executable");
    assert!(
        !output.status.success(),
        "malformed receipt attempts must fail closed"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("ALREADY-PUBLISHED:"));
    fs::remove_dir_all(root).expect("legacy verification fixture should be removable");
}

#[test]
fn task5_cli_verifies_published_run_without_provider_or_telegram_configuration() {
    let root = task5_cli_fixture_root("recover-published");
    let input = task4_report_input();
    let snapshot = persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
        .expect("published recovery input should persist");
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let transport = Task4RecordingTransport::default();
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-123",
        &transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("fixture delivery should succeed once");
    write_run_with_input_snapshot(&root, "data", &report, &receipt, Some(&snapshot))
        .expect("fixture publication should commit");
    let sent_before_recovery = transport.0.lock().unwrap().len();

    let output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            root.to_str().unwrap(),
            "--recover-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("published recovery CLI should be executable");

    assert!(
        output.status.success(),
        "published recovery should not need provider or Telegram configuration: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("READY-TO-PUSH:"));
    assert_eq!(transport.0.lock().unwrap().len(), sent_before_recovery);
    fs::remove_dir_all(root).expect("published recovery fixture should be removable");
}

fn task5_git(root: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .expect("isolated git command should be executable")
}

fn task5_git_success(root: &Path, args: &[&str]) -> String {
    let output = task5_git(root, args);
    assert!(
        output.status.success(),
        "isolated git command should succeed: git -C {} {}\nstderr: {}",
        root.display(),
        args.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

#[cfg(unix)]
#[test]
fn task5_pending_ref_recovers_after_data_push_failure_without_second_telegram() {
    use std::os::unix::fs::PermissionsExt;

    let root = task5_cli_fixture_root("pending-push-recovery");
    let producer = root.join("producer");
    let remote = root.join("remote.git");
    let damaged = root.join("damaged-runner");
    let runner = root.join("recovery-runner");
    fs::create_dir_all(&producer).expect("producer repository should be writable");
    let input = task4_report_input();
    let snapshot = persist_input_snapshot(&producer, "data", &input, ReportLanguage::Chinese, true)
        .expect("producer input should persist");
    let report = render_report_in_language(&input, ReportLanguage::Chinese);
    let transport = Task4RecordingTransport::default();
    let receipt = send_rendered_report_with_transport(
        &report,
        "isolated-chat",
        &transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("recording Telegram should accept the report once");
    write_run_with_input_snapshot(&producer, "data", &report, &receipt, Some(&snapshot))
        .expect("producer archive should commit");
    let sent_before_push = transport.0.lock().unwrap().len();

    let remote_init = Command::new("git")
        .args(["init", "--bare", remote.to_str().unwrap()])
        .output()
        .expect("bare remote should be creatable");
    assert!(remote_init.status.success());
    task5_git_success(&producer, &["init"]);
    task5_git_success(&producer, &["config", "user.name", "isolated-test"]);
    task5_git_success(
        &producer,
        &["config", "user.email", "isolated-test@example.test"],
    );
    task5_git_success(&producer, &["add", "weekly-radar"]);
    task5_git_success(
        &producer,
        &["commit", "-m", "isolated weekly radar publication"],
    );
    task5_git_success(
        &producer,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    let pending_sha = task5_git_success(&producer, &["rev-parse", "HEAD"]);
    let pending_push = task5_git(
        &producer,
        &["push", "origin", "HEAD:refs/heads/weekly-radar-pending"],
    );
    assert!(
        pending_push.status.success(),
        "pending publication ref must be durable: {}",
        String::from_utf8_lossy(&pending_push.stderr)
    );

    let hook = remote.join("hooks/update");
    fs::write(
        &hook,
        "#!/bin/sh\nif [ \"$1\" = \"refs/heads/data\" ]; then echo isolated data push rejection >&2; exit 1; fi\nexit 0\n",
    )
    .expect("failure-injection hook should be writable");
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755))
        .expect("failure-injection hook should be executable");
    let failed_data_push = task5_git(&producer, &["push", "origin", "HEAD:refs/heads/data"]);
    assert!(
        !failed_data_push.status.success(),
        "data push must be rejected once"
    );
    assert!(
        String::from_utf8_lossy(&failed_data_push.stderr).contains("isolated data push rejection")
    );

    let prepare_runner = |path: &Path| {
        fs::create_dir_all(path).expect("runner repository should be writable");
        task5_git_success(path, &["init"]);
        task5_git_success(path, &["remote", "add", "origin", remote.to_str().unwrap()]);
        task5_git_success(path, &["fetch", "origin", "weekly-radar-pending"]);
        task5_git_success(path, &["checkout", "--detach", "FETCH_HEAD"]);
    };
    prepare_runner(&damaged);
    let receipt_path = damaged.join("weekly-radar/receipts/2026-08-17.json");
    let original_receipt = fs::read(&receipt_path).expect("pending receipt should exist");
    fs::write(&receipt_path, b"damaged pending receipt\n")
        .expect("damaged receipt fixture should be writable");
    let damaged_output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            damaged.to_str().unwrap(),
            "--recover-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("damaged recovery CLI should be executable");
    assert!(
        !damaged_output.status.success(),
        "damaged pending state must fail closed"
    );
    assert!(String::from_utf8_lossy(&damaged_output.stderr).contains("incomplete"));
    assert_eq!(
        fs::read(&receipt_path).unwrap(),
        b"damaged pending receipt\n"
    );
    assert_ne!(fs::read(&receipt_path).unwrap(), original_receipt);

    fs::remove_file(&hook).expect("failure-injection hook should be removable");
    prepare_runner(&runner);
    let recovered_output = Command::new(env!("CARGO_BIN_EXE_org-x"))
        .args([
            "weekly-radar",
            "--archive-dir",
            runner.to_str().unwrap(),
            "--recover-published-as-of",
            "2026-08-17",
        ])
        .env_remove("ORGX_SEC_USER_AGENT")
        .env_remove("ORGX_TELEGRAM_BOT_TOKEN")
        .env_remove("ORGX_TELEGRAM_CHAT_ID")
        .output()
        .expect("recovery CLI should be executable");
    assert!(
        recovered_output.status.success(),
        "recovery should verify the original receipt without Telegram: {}",
        String::from_utf8_lossy(&recovered_output.stderr)
    );
    assert!(String::from_utf8_lossy(&recovered_output.stdout).contains("READY-TO-PUSH:"));
    assert_eq!(transport.0.lock().unwrap().len(), sent_before_push);

    let recovered_push = task5_git(&runner, &["push", "origin", "HEAD:refs/heads/data"]);
    assert!(
        recovered_push.status.success(),
        "recovered data tree should publish: {}",
        String::from_utf8_lossy(&recovered_push.stderr)
    );
    let final_data_sha = task5_git_success(&runner, &["ls-remote", "origin", "refs/heads/data"])
        .split_whitespace()
        .next()
        .expect("final data ref should have a commit")
        .to_owned();
    assert_eq!(final_data_sha, pending_sha);
    for relative in [
        "weekly-radar/reports/2026-08-17.md",
        "weekly-radar/snapshots/2026-08-17.json",
        "weekly-radar/receipts/2026-08-17.json",
        "weekly-radar/manifest.json",
    ] {
        assert_eq!(
            fs::read(producer.join(relative)).unwrap(),
            fs::read(runner.join(relative)).unwrap(),
            "recovery must preserve the original {relative} bytes"
        );
    }
    fs::remove_dir_all(root).expect("pending recovery fixture should be removable");
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
    assert!(workflow.contains("dry_run:"));
    assert!(workflow.contains("as_of:"));
    assert!(workflow.contains("language:"));
    assert!(workflow.contains("default: zh-CN"));
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
    assert!(workflow.contains("weekly-radar-pending"));
    assert!(workflow.contains("force-with-lease=refs/heads/weekly-radar-pending:"));
    assert!(workflow.contains("--recover-published-as-of"));
    assert!(workflow.contains("refusing to downgrade the archive"));
    assert!(workflow.contains("Pending publication contains a newer same-date canonical update"));
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
    assert!(workflow.contains("the next run will verify and reuse the original receipt"));
    assert!(workflow.contains("refusing to send again"));
}

#[test]
fn task6_workflow_runs_the_cli_and_rejects_empty_or_unpublished_output() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("cargo run --release -- weekly-radar"));
    assert!(workflow.contains("data_final_run=false"));
    assert!(workflow.contains("\"$data_final_run\" == \"true\""));
    assert!(workflow.contains("normal publication will acquire a new update and make its last successful result canonical"));
    assert!(
        !workflow.contains("already published $as_of; no Telegram or data-branch write was needed")
    );
    assert!(workflow.contains("RECOVERED:"));
    assert!(workflow.contains("--archive-dir \"$GITHUB_WORKSPACE\""));
    assert!(
        workflow.contains("--registry \"$GITHUB_WORKSPACE/config/weekly_radar/companies.json\"")
    );
    assert!(workflow.contains("PUBLISHED:"));
    assert!(workflow.contains("weekly-radar/reports/"));
    assert!(workflow.contains("set -euo pipefail"));
}

#[test]
fn task6_workflow_runs_normal_cli_when_same_date_final_run_exists() {
    let workflow = task6_workflow_text();
    let existing_final_start = workflow
        .find("if [[ \"$REPORT_DRY_RUN\" != \"true\" && \"$data_final_run\" == \"true\" ]]")
        .expect("existing-final branch should exist");
    let existing_final_end = workflow[existing_final_start..]
        .find("elif [[ \"$REPORT_DRY_RUN\" != \"true\" && \"$REPUBLISH_PUBLISHED\" == \"true\" ]]")
        .map(|offset| existing_final_start + offset)
        .expect("existing-final branch should be followed by republish guard");
    let existing_final_branch = &workflow[existing_final_start..existing_final_end];

    assert!(
        existing_final_branch.contains("cargo run --release -- weekly-radar \"${cli_args[@]}\""),
        "an existing final run must still invoke normal publication"
    );
    assert!(
        existing_final_branch.contains("| tee \"$run_output\""),
        "the existing-final publication result must be captured for validation"
    );
}

#[test]
fn task6_workflow_declares_explicit_republish_only_for_manual_validation() {
    let workflow = task6_workflow_text();

    assert!(workflow.contains("republish_published:"));
    assert!(workflow.contains("REPUBLISH_PUBLISHED"));
    assert!(workflow.contains("only available to explicit workflow_dispatch"));
    assert!(workflow.contains("--republish-published-as-of"));
    assert!(workflow.contains("REPUBLISHED:"));
    assert!(workflow.contains("archive and data branch were not changed"));
}

#[test]
fn task8_runtime_persists_machine_reference_separately_from_human_reference() {
    let as_of = NaiveDate::from_ymd_opt(2026, 8, 20).unwrap();
    let facts = vec![
        NormalizedFact::new(
            "acme",
            "judgment.supporting.WORKFLOW.workflow_rewrite",
            "workflow responsibility changed",
            FactStatus::Known,
            Confidence::High,
            Provenance::new(
                "https://source-a.example/workflow",
                "fixture field",
                Utc::now(),
                Some(as_of),
            )
            .unwrap(),
        )
        .unwrap(),
        NormalizedFact::new(
            "acme",
            "judgment.supporting.WORKFLOW.human_supervision",
            "human supervision retained",
            FactStatus::Known,
            Confidence::High,
            Provenance::new(
                "https://source-b.example/operations",
                "fixture field",
                Utc::now(),
                Some(as_of),
            )
            .unwrap(),
        )
        .unwrap(),
        NormalizedFact::new(
            "acme",
            "judgment.counter.WORKFLOW.counter_signal",
            "legacy workflow remains",
            FactStatus::Known,
            Confidence::Medium,
            Provenance::new(
                "https://source-c.example/risk",
                "fixture field",
                Utc::now(),
                Some(as_of),
            )
            .unwrap(),
        )
        .unwrap(),
        NormalizedFact::without_value(
            "acme",
            "judgment.missing.WORKFLOW.persistence",
            FactStatus::Unknown,
            Confidence::Unknown,
            Provenance::new("fixture://missing", "missing proof", Utc::now(), None).unwrap(),
        )
        .unwrap(),
    ];
    let human = HumanReference::new(
        "acme",
        "PRODUCTION_SYSTEM",
        "人的判断独立保留。",
        "2026-08-20T10:00:00Z",
    )
    .unwrap();
    let judgment = derive_judgment_snapshot(as_of, &facts, vec![human]).unwrap();

    assert_eq!(
        judgment.company("acme").unwrap().machine_stage(),
        &MachineStage::assigned("WORKFLOW")
    );
    assert_eq!(
        judgment.human_reference("acme").unwrap().stage(),
        "PRODUCTION_SYSTEM"
    );
    assert_ne!(
        judgment.company("acme").unwrap().machine_stage().label(),
        judgment.human_reference("acme").unwrap().stage()
    );
}
