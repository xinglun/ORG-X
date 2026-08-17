use std::sync::{Arc, Mutex};

use crate::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, FactId, FactValue, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarPublication, WeeklyRadarSnapshot,
};

use super::{
    PublicationChannel, PublicationClock, PublicationDeliveryError, PublicationReceiptService,
    PublicationStatus, ReceiptAwarePublisher,
};
use crate::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramTransport, TelegramTransportError,
};

#[derive(Clone, Default)]
struct RecordingTransport {
    calls: Arc<Mutex<Vec<String>>>,
    fail_once_at: Arc<Mutex<Option<usize>>>,
}

impl RecordingTransport {
    fn failing_once_at(index: usize) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            fail_once_at: Arc::new(Mutex::new(Some(index))),
        }
    }

    fn calls(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("recording lock should not fail")
            .clone()
    }
}

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        let mut calls = self.calls.lock().expect("recording lock should not fail");
        let index = calls.len();
        calls.push(markdown.to_owned());
        let should_fail = {
            let mut fail_once_at = self
                .fail_once_at
                .lock()
                .expect("failure lock should not fail");
            if *fail_once_at == Some(index) {
                *fail_once_at = None;
                true
            } else {
                false
            }
        };
        if should_fail {
            return Err(TelegramTransportError::Failed {
                reason: "synthetic delivery failure".to_owned(),
            });
        }
        TelegramMessageId::new(format!("message-{index}")).map_err(|error| {
            TelegramTransportError::Failed {
                reason: error.to_string(),
            }
        })
    }
}

#[derive(Clone)]
struct FixedClock(&'static str);

impl PublicationClock for FixedClock {
    fn now(&self) -> &str {
        self.0
    }
}

fn publication(snapshot_id: &str) -> WeeklyRadarPublication {
    let snapshot = WeeklyRadarSnapshot::new(
        SnapshotId::new(snapshot_id).expect("snapshot ID should be valid"),
        AsOf::new("2026-08-16").expect("as-of should be valid"),
        UniverseSnapshotId::new("universe-1").expect("universe should be valid"),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").expect("cutoff should be valid"),
        ModelVersion::new("model-v1").expect("model should be valid"),
        ScoringVersion::new("score-v1").expect("scoring should be valid"),
    )
    .expect("snapshot should be valid");
    let mut publication = WeeklyRadarPublication::new(snapshot);
    publication
        .add_fact(
            FactId::new("summary").expect("fact ID should be valid"),
            FactValue::new("Executive Summary\nStable.").expect("fact should be valid"),
        )
        .expect("fact should be added");
    publication
        .add_fact(
            FactId::new("health").expect("fact ID should be valid"),
            FactValue::new("System Health: HEALTHY").expect("fact should be valid"),
        )
        .expect("fact should be added");
    publication
}

fn service(
    publication: WeeklyRadarPublication,
    transport: RecordingTransport,
) -> PublicationReceiptService<impl ReceiptAwarePublisher, FixedClock> {
    let publisher =
        TelegramPublisherAdapter::new("chat-123", transport).expect("destination should be valid");
    PublicationReceiptService::new(publication, publisher, FixedClock("2026-08-17T15:00:00Z"))
}

#[test]
fn successful_publication_returns_ordered_receipt() {
    let transport = RecordingTransport::default();
    let receipt = service(publication("snapshot-1"), transport.clone())
        .publish()
        .expect("publication should succeed");

    assert_eq!(receipt.channel(), PublicationChannel::Telegram);
    assert_eq!(receipt.snapshot_id().as_str(), "snapshot-1");
    assert_eq!(receipt.published_at(), "2026-08-17T15:00:00Z");
    assert_eq!(receipt.attempt(), 1);
    assert_eq!(receipt.message_ids().len(), 2);
    assert_eq!(receipt.status(), &PublicationStatus::Published);
    assert_eq!(
        transport.calls(),
        ["Executive Summary\nStable.", "System Health: HEALTHY"]
    );
}

#[test]
fn partial_failure_returns_receipt_without_invalidating_snapshot() {
    let transport = RecordingTransport::failing_once_at(1);
    let original = publication("snapshot-1");
    let service = service(original.clone(), transport);

    let failure = service.publish().expect_err("second message should fail");

    assert_eq!(failure.receipt().snapshot_id(), original.snapshot_id());
    assert_eq!(failure.receipt().message_ids().len(), 1);
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
    assert_eq!(
        original.facts()[0].value().as_str(),
        "Executive Summary\nStable."
    );
}

#[test]
fn retry_reuses_exact_payloads_and_same_snapshot_with_incremented_attempt() {
    let transport = RecordingTransport::failing_once_at(1);
    let service = service(publication("snapshot-1"), transport.clone());
    let failure = service.publish().expect_err("first attempt should fail");

    let retry = service
        .retry(failure.receipt())
        .expect("retry should deliver the retained publication");

    assert_eq!(retry.snapshot_id().as_str(), "snapshot-1");
    assert_eq!(retry.attempt(), 2);
    assert_eq!(retry.status(), &PublicationStatus::Published);
    assert_eq!(
        transport.calls(),
        [
            "Executive Summary\nStable.",
            "System Health: HEALTHY",
            "Executive Summary\nStable.",
            "System Health: HEALTHY"
        ]
    );
}

#[test]
fn retry_rejects_receipt_for_a_different_snapshot_before_transport() {
    let source_transport = RecordingTransport::default();
    let source = service(publication("snapshot-1"), source_transport);
    let foreign_receipt = service(
        publication("snapshot-foreign"),
        RecordingTransport::default(),
    )
    .publish()
    .expect("foreign publication should succeed");

    let failure = source
        .retry(&foreign_receipt)
        .expect_err("different snapshot must be rejected");

    assert_eq!(
        failure.error(),
        &PublicationDeliveryError::SnapshotMismatch {
            expected: "snapshot-1".to_owned(),
            actual: "snapshot-foreign".to_owned(),
        }
    );
}
