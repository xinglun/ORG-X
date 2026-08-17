use std::sync::{Arc, Mutex};

use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, FactId, FactValue, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarPublication, WeeklyRadarSnapshot,
};
use org_x::features::weekly_radar::infrastructure::publication_receipt::{
    PublicationClock, PublicationReceiptService,
};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramMessageId, TelegramPublisherAdapter, TelegramTransport, TelegramTransportError,
};

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<Vec<String>>>);

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<TelegramMessageId, TelegramTransportError> {
        self.0.lock().unwrap().push(markdown.to_owned());
        TelegramMessageId::new(format!("message-{}", self.0.lock().unwrap().len())).map_err(
            |error| TelegramTransportError::Failed {
                reason: error.to_string(),
            },
        )
    }
}

#[derive(Clone)]
struct FixedClock;

impl PublicationClock for FixedClock {
    fn now(&self) -> &str {
        "2026-08-17T15:00:00Z"
    }
}

fn publication() -> WeeklyRadarPublication {
    let snapshot = WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-public").unwrap(),
        AsOf::new("2026-08-16").unwrap(),
        UniverseSnapshotId::new("universe-public").unwrap(),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").unwrap(),
        ModelVersion::new("model-v1").unwrap(),
        ScoringVersion::new("score-v1").unwrap(),
    )
    .unwrap();
    let mut publication = WeeklyRadarPublication::new(snapshot);
    publication
        .add_fact(
            FactId::new("summary").unwrap(),
            FactValue::new("precomputed Telegram message").unwrap(),
        )
        .unwrap();
    publication
}

#[test]
fn public_receipt_service_keeps_application_publisher_port_separate() {
    let transport = RecordingTransport::default();
    let publisher = TelegramPublisherAdapter::new("chat-public", transport.clone()).unwrap();
    let service = PublicationReceiptService::new(publication(), publisher, FixedClock);

    let receipt = service.publish().unwrap();
    assert_eq!(receipt.snapshot_id().as_str(), "snapshot-public");
    assert_eq!(receipt.message_ids().len(), 1);
    assert_eq!(
        transport.0.lock().unwrap().as_slice(),
        ["precomputed Telegram message"]
    );
}
