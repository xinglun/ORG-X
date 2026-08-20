use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use org_x::features::reporting::domain::{ReportSection, ResearchPacket, Top5 as ResearchTop5};
use org_x::features::weekly_radar::application::snapshot_store::{
    InMemoryWeeklyRadarSnapshotStore, WeeklyRadarSnapshotStore,
};
use org_x::features::weekly_radar::application::weekly_scheduler::{Weekday, WeeklyScheduler};
use org_x::features::weekly_radar::domain::change_compression::{
    PeriodId, WeeklyChangeCompression, WeeklyChangeInput,
};
use org_x::features::weekly_radar::domain::system_health::{
    EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, FactId, FactValue, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarPublication, WeeklyRadarSnapshot,
};
use org_x::features::weekly_radar::infrastructure::archive_store::{
    InMemoryWeeklyRadarArchive, WeeklyRadarArchive,
};
use org_x::features::weekly_radar::infrastructure::publication_receipt::{
    PublicationChannel, PublicationClock, PublicationDeliveryError, PublicationReceiptService,
    PublicationStatus,
};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramTransport, TelegramTransportError,
};
use org_x::features::weekly_radar::interface::markdown_renderer::{
    MarkdownRenderer, MarkdownReportInput,
};
use org_x::features::weekly_radar::interface::semantic_message_splitter::{
    SemanticMessageSplitter, SemanticSplitLimits,
};
use org_x::features::weekly_radar::interface::telegram_renderer::{
    ItemId, NoChangeSummary, PeriodId as TelegramPeriodId, SummaryItem, SystemHealthSummary,
    TelegramRenderLimits, TelegramRenderer, TelegramSummaryInput,
};
use org_x::features::weekly_radar::runtime::archive::{
    load_input_snapshot, persist_input_snapshot, recover_pending_run, write_run_with_input_snapshot,
};
use org_x::features::weekly_radar::runtime::model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};
use org_x::features::weekly_radar::runtime::report::{render_report_in_language, ReportLanguage};
use org_x::features::weekly_radar::runtime::telegram::{
    send_rendered_report_with_transport, TelegramRetryPolicy,
};

#[derive(Default)]
struct TransportState {
    calls: Vec<String>,
    fail_once_at: Option<usize>,
}

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<TransportState>>);

impl RecordingTransport {
    fn failing_once_at(index: usize) -> Self {
        Self(Arc::new(Mutex::new(TransportState {
            calls: Vec::new(),
            fail_once_at: Some(index),
        })))
    }

    fn calls(&self) -> Vec<String> {
        self.0
            .lock()
            .expect("transport lock should not fail")
            .calls
            .clone()
    }
}

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        let mut state = self.0.lock().expect("transport lock should not fail");
        let index = state.calls.len();
        state.calls.push(markdown.to_owned());
        let should_fail = state.fail_once_at == Some(index);
        if should_fail {
            state.fail_once_at = None;
        }
        drop(state);

        if should_fail {
            return Err(TelegramTransportError::Failed {
                reason: "synthetic E2E delivery failure".to_owned(),
            });
        }

        TelegramMessageId::new(format!("e2e-message-{index}")).map_err(|error| {
            TelegramTransportError::Failed {
                reason: error.to_string(),
            }
        })
    }
}

struct FixedClock;

impl PublicationClock for FixedClock {
    fn now(&self) -> &str {
        "2026-08-17T16:00:00Z"
    }
}

fn snapshot() -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-e2e-2026-w33").expect("snapshot ID should be valid"),
        AsOf::new("2026-08-16").expect("as-of should be valid"),
        UniverseSnapshotId::new("universe-e2e-2026-w33").expect("universe should be valid"),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").expect("cutoff should be valid"),
        ModelVersion::new("model-e2e-v1").expect("model should be valid"),
        ScoringVersion::new("scoring-e2e-v1").expect("scoring should be valid"),
    )
    .expect("snapshot should be valid")
}

fn markdown_report(snapshot: &WeeklyRadarSnapshot) -> String {
    let period = PeriodId::new("2026-W33").expect("period should be valid");
    let compression = WeeklyChangeCompression::from_input(
        WeeklyChangeInput::new(
            period,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("empty weekly input should be valid"),
    )
    .expect("compression should be valid");
    let top5 = org_x::features::weekly_radar::domain::top5_weekly_read_model::Top5WeeklyReadModel::from_entries(
        Vec::new(),
    )
    .expect("empty Top5 should be valid");
    let research = ResearchPacket::new(
        "No meaningful structural change this week.",
        ResearchTop5::new(),
        ReportSection::new(),
        ReportSection::new(),
        ReportSection::new(),
    )
    .expect("research packet should be valid");
    let health = SystemHealth::new(
        HealthStatus::Healthy,
        EvidenceCoverage::new(1, 1, 100).expect("coverage should be valid"),
        Freshness::Current,
    );

    MarkdownRenderer::render(&MarkdownReportInput::new(
        snapshot,
        &top5,
        &research,
        &compression,
        &[],
        &[],
        Some(&health),
    ))
    .as_str()
    .to_owned()
}

fn no_change_telegram() -> String {
    let period = TelegramPeriodId::new("2026-W33").expect("Telegram period should be valid");
    let no_change =
        NoChangeSummary::new(period.clone(), "No meaningful structural change this week.")
            .expect("no-change fact should be valid");
    let input = TelegramSummaryInput::new(
        period,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        None,
        Some(no_change),
    )
    .expect("no-change Telegram input should be valid");

    TelegramRenderer::render(
        &input,
        TelegramRenderLimits::new(4_096, 40, 20, 20).expect("limits should be valid"),
    )
    .expect("no-change Telegram view should render")
    .as_str()
    .to_owned()
}

fn changed_telegram_chunks() -> Vec<String> {
    let period = TelegramPeriodId::new("2026-W33").expect("Telegram period should be valid");
    let input = TelegramSummaryInput::new(
        period,
        vec![SummaryItem::new(
            ItemId::new("important-e2e").expect("item ID should be valid"),
            "Structural evidence strengthened",
        )
        .expect("item should be valid")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Some(
            SystemHealthSummary::new("HEALTHY", "coverage 1/1; freshness current")
                .expect("health summary should be valid"),
        ),
        None,
    )
    .expect("changed Telegram input should be valid");
    let message = TelegramRenderer::render(
        &input,
        TelegramRenderLimits::new(4_096, 40, 20, 20).expect("limits should be valid"),
    )
    .expect("changed Telegram view should render");
    let split = SemanticMessageSplitter::split(
        message.as_str(),
        SemanticSplitLimits::new(200, 20).expect("split limits should be valid"),
    )
    .expect("semantic split should be valid");
    assert_eq!(
        split.chunks().len(),
        3,
        "E2E retry needs three semantic chunks"
    );
    split
        .chunks()
        .iter()
        .map(|chunk| chunk.markdown().to_owned())
        .collect()
}

fn publication(chunks: &[String]) -> WeeklyRadarPublication {
    let mut publication = WeeklyRadarPublication::new(snapshot());
    for (index, chunk) in chunks.iter().enumerate() {
        publication
            .add_fact(
                FactId::new(format!("telegram-chunk-{index}")).expect("fact ID should be valid"),
                FactValue::new(chunk.clone()).expect("fact value should be valid"),
            )
            .expect("publication fact should be added");
    }
    publication
}

#[test]
fn e2e_success_proves_compute_persist_render_publish_archive_order() {
    let scheduler = WeeklyScheduler::default();
    assert!(scheduler.should_run(Weekday::Monday));

    let mut events = Vec::new();
    let snapshot = snapshot();
    let markdown = markdown_report(&snapshot);
    events.push("Compute");
    assert!(markdown.contains("2026-08-15T23:59:59Z"));
    assert_eq!(markdown, markdown_report(&snapshot));

    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    store
        .save(snapshot.clone())
        .expect("snapshot should persist once");
    events.push("Persist Snapshot");
    assert_eq!(store.get(snapshot.id()), Some(&snapshot));

    let no_change = no_change_telegram();
    events.push("Render");
    assert!(no_change.contains("NO_CHANGE"));
    assert!(no_change.contains("No meaningful structural change this week."));
    assert!(!no_change.contains("## Top5"));

    let chunks = vec![no_change];
    let publisher = TelegramPublisherAdapter::new("chat-e2e", RecordingTransport::default())
        .expect("destination should be valid");
    let service = PublicationReceiptService::new(publication(&chunks), publisher, FixedClock);
    let receipt = service.publish().expect("publication should succeed");
    events.push("Publish");
    assert_eq!(receipt.channel(), PublicationChannel::Telegram);
    assert_eq!(receipt.snapshot_id(), snapshot.id());
    assert_eq!(receipt.status(), &PublicationStatus::Published);

    let mut archive = InMemoryWeeklyRadarArchive::new();
    archive
        .archive(store.get(snapshot.id()).unwrap().clone(), receipt.clone())
        .expect("published receipt should archive");
    events.push("Archive");
    assert_eq!(
        events,
        [
            "Compute",
            "Persist Snapshot",
            "Render",
            "Publish",
            "Archive"
        ]
    );
    assert_eq!(archive.entries().len(), 1);
    assert_eq!(
        archive.get(snapshot.id()).unwrap().receipt().snapshot_id(),
        snapshot.id()
    );
    assert_eq!(archive.get(snapshot.id()).unwrap().snapshot(), &snapshot);
}

#[test]
fn e2e_failure_retries_same_snapshot_and_exact_ordered_payloads() {
    let chunks = changed_telegram_chunks();
    let snapshot = snapshot();
    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    store
        .save(snapshot.clone())
        .expect("snapshot should persist");
    let transport = RecordingTransport::failing_once_at(1);
    let publisher = TelegramPublisherAdapter::new("chat-e2e", transport.clone())
        .expect("destination should be valid");
    let service = PublicationReceiptService::new(publication(&chunks), publisher, FixedClock);

    let failure = service.publish().expect_err("second chunk should fail");
    assert_eq!(failure.receipt().snapshot_id(), snapshot.id());
    assert_eq!(
        failure.receipt().status(),
        &PublicationStatus::Partial {
            failed_message_index: 1,
        }
    );
    assert!(matches!(
        failure.error(),
        PublicationDeliveryError::Transport {
            message_index: 1,
            ..
        }
    ));

    let retry = service
        .retry(failure.receipt())
        .expect("retry should succeed");
    assert_eq!(retry.snapshot_id(), snapshot.id());
    assert_eq!(retry.attempt(), 2);
    assert_eq!(retry.status(), &PublicationStatus::Published);
    assert_eq!(
        transport.calls(),
        [
            chunks[0].clone(),
            chunks[1].clone(),
            chunks[0].clone(),
            chunks[1].clone(),
            chunks[2].clone(),
        ]
    );
    assert_eq!(store.get(snapshot.id()), Some(&snapshot));
}

fn durable_input() -> RuntimeReportInput {
    let mut input = RuntimeReportInput::new("2026-08-17").expect("durable input date is valid");
    input
        .add_fact(
            NormalizedFact::new(
                "e2e-company",
                "revenue",
                "123000000",
                FactStatus::Known,
                Confidence::High,
                Provenance::from_rfc3339(
                    "https://example.test/e2e-source",
                    "facts.revenue",
                    "2026-08-17T00:00:00Z",
                    Some("2026-08-15"),
                )
                .expect("durable provenance is valid"),
            )
            .expect("durable fact is valid"),
        )
        .expect("durable fact is unique");
    input
        .add_source_coverage(SourceCoverage::new("official", 1, 1).expect("coverage is valid"))
        .expect("coverage is unique");
    input
}

#[test]
fn durable_input_survives_failed_delivery_and_supports_exact_retry() {
    let root = std::env::temp_dir().join(format!(
        "org-x-e2e-durable-input-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    let input = durable_input();
    let saved = persist_input_snapshot(&root, "data", &input, ReportLanguage::English, true)
        .expect("input should persist before delivery");
    let report = render_report_in_language(saved.input(), saved.language());
    let failed_transport = RecordingTransport::failing_once_at(0);
    let failure = send_rendered_report_with_transport(
        &report,
        "chat-e2e",
        &failed_transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect_err("first delivery should fail in the fixture transport");
    assert!(failure.to_string().contains("delivery failed"));

    let reloaded = load_input_snapshot(&root, "data", input.as_of())
        .expect("failed delivery must leave the input snapshot");
    assert_eq!(reloaded.input(), &input);
    assert_eq!(reloaded.snapshot_id(), saved.snapshot_id());
    let retry_report = render_report_in_language(reloaded.input(), reloaded.language());
    assert_eq!(retry_report.report_id(), report.report_id());
    let receipt = send_rendered_report_with_transport(
        &retry_report,
        "chat-e2e",
        &RecordingTransport::default(),
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("retry delivery should succeed");
    let manifest =
        write_run_with_input_snapshot(&root, "data", &retry_report, &receipt, Some(&reloaded))
            .expect("retry archive should succeed");

    assert_eq!(
        manifest.input_snapshot(),
        Some("weekly-radar/snapshots/2026-08-17.input.json")
    );
    assert_eq!(manifest.snapshot_id(), Some(saved.snapshot_id()));
    let manifest_json = fs::read_to_string(root.join("weekly-radar/manifest.json"))
        .expect("manifest should be written");
    assert!(manifest_json.contains("2026-08-17.input.json"));
    assert!(manifest_json.contains(saved.snapshot_id()));
    fs::remove_dir_all(root).expect("durable input fixture should be removable");
}

fn archive_transaction_digest(text: &str) -> String {
    let mut digest = 14_695_981_039_346_656_037_u64;
    for byte in text.as_bytes() {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    format!("wr-input-{digest:016x}")
}

#[test]
fn prepared_archive_recovery_reuses_receipt_without_a_second_transport_call() {
    let root = std::env::temp_dir().join(format!(
        "org-x-e2e-archive-recovery-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos()
    ));
    let input = durable_input();
    let saved = persist_input_snapshot(&root, "data", &input, ReportLanguage::English, true)
        .expect("input should persist before delivery");
    let report = render_report_in_language(saved.input(), saved.language());
    let transport = RecordingTransport::default();
    let receipt = send_rendered_report_with_transport(
        &report,
        "chat-e2e",
        &transport,
        TelegramRetryPolicy::new(1, Duration::ZERO),
    )
    .expect("the first delivery should succeed");
    let calls_before_recovery = transport.calls();

    let archive = root.join("weekly-radar");
    let transaction_id = "2026-08-17-e2e-prepared";
    let staging_directory = format!(".transactions/{transaction_id}");
    let staging = archive.join(&staging_directory);
    fs::create_dir_all(&staging).expect("transaction staging should be writable");
    let receipt_json = serde_json::to_string_pretty(&serde_json::json!({
        "as_of": "2026-08-17",
        "report_id": report.report_id(),
        "status": "PUBLISHED",
        "message_ids": receipt
            .message_ids()
            .iter()
            .map(|message_id| message_id.as_str())
            .collect::<Vec<_>>(),
        "attempts": receipt.attempts(),
    }))
    .expect("receipt fixture should serialize")
        + "\n";
    let manifest_value = serde_json::json!({
        "as_of": "2026-08-17",
        "report": "weekly-radar/reports/2026-08-17.md",
        "snapshot": "weekly-radar/snapshots/2026-08-17.json",
        "receipt": "weekly-radar/receipts/2026-08-17.json",
        "input_snapshot": "weekly-radar/snapshots/2026-08-17.input.json",
        "snapshot_id": saved.snapshot_id(),
    });
    let manifest_json = serde_json::to_string_pretty(&manifest_value)
        .expect("manifest fixture should serialize")
        + "\n";
    let staged = [
        ("report.md", report.markdown()),
        ("snapshot.json", report.snapshot_json()),
        ("receipt.json", receipt_json.as_str()),
        ("manifest.json", manifest_json.as_str()),
    ];
    for (name, content) in staged {
        fs::write(staging.join(name), content).expect("staged artifact should be writable");
    }
    let artifacts = staged
        .iter()
        .zip([
            "reports/2026-08-17.md",
            "snapshots/2026-08-17.json",
            "receipts/2026-08-17.json",
            "manifest.json",
        ])
        .map(|((name, content), final_path)| {
            serde_json::json!({
                "final_path": final_path,
                "staged_path": format!("{staging_directory}/{name}"),
                "digest": archive_transaction_digest(content),
            })
        })
        .collect::<Vec<_>>();
    let transaction = serde_json::json!({
        "schema_version": 1,
        "as_of": "2026-08-17",
        "transaction_id": transaction_id,
        "state": "prepared",
        "staging_directory": staging_directory,
        "artifacts": artifacts,
        "manifest": manifest_value,
    });
    fs::create_dir_all(archive.join(".transactions"))
        .expect("transaction record directory should be writable");
    fs::write(
        archive.join(".transactions/2026-08-17.json"),
        serde_json::to_string_pretty(&transaction).expect("transaction should serialize"),
    )
    .expect("prepared transaction should be writable");

    let recovered = recover_pending_run(
        &root,
        "data",
        chrono::NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date is valid"),
    )
    .expect("prepared transaction should recover")
    .expect("recovery should complete the prepared run");
    assert_eq!(recovered.report(), "weekly-radar/reports/2026-08-17.md");
    assert_eq!(transport.calls(), calls_before_recovery);
    assert_eq!(
        fs::read_to_string(archive.join("receipts/2026-08-17.json"))
            .expect("recovered receipt should exist"),
        receipt_json
    );
    fs::remove_dir_all(root).expect("archive recovery fixture should be removable");
}

#[test]
fn workflow_pins_both_checkouts_to_v5() {
    let workflow = include_str!("../.github/workflows/ai-cockpit.yml");
    let versions: Vec<_> = workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- uses: actions/checkout@"))
        .collect();

    assert_eq!(versions, ["v5", "v5"]);
    assert!(!workflow.contains("actions/checkout@v4"));
    assert_eq!(workflow.matches("fetch-depth: 0").count(), 2);
}
