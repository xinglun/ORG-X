use std::sync::{Arc, Mutex};

use org_x::features::weekly_radar::application::WeeklyRadarPublisher;
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, FactId, FactValue, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarPublication, WeeklyRadarSnapshot,
};
use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
    TelegramPublisherAdapter, TelegramTransport, TelegramTransportError,
};

#[derive(Clone, Default)]
struct RecordingTransport(Arc<Mutex<Vec<String>>>);

impl TelegramTransport for RecordingTransport {
    fn send_message(
        &self,
        _destination: &str,
        markdown: &str,
    ) -> Result<(), TelegramTransportError> {
        self.0
            .lock()
            .expect("recording lock should not fail")
            .push(markdown.to_owned());
        Ok(())
    }
}

#[test]
fn application_port_forwards_precomputed_fact_without_recalculation() {
    let snapshot = WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-1").unwrap(),
        AsOf::new("2026-08-16").unwrap(),
        UniverseSnapshotId::new("universe-1").unwrap(),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").unwrap(),
        ModelVersion::new("model-v1").unwrap(),
        ScoringVersion::new("score-v1").unwrap(),
    )
    .unwrap();
    let mut publication = WeeklyRadarPublication::new(snapshot);
    publication
        .add_fact(
            FactId::new("summary").unwrap(),
            FactValue::new("already-rendered Markdown").unwrap(),
        )
        .unwrap();

    let transport = RecordingTransport::default();
    let adapter = TelegramPublisherAdapter::new("chat-123", transport.clone()).unwrap();
    adapter
        .publish(&publication)
        .expect("precomputed publication should be delivered");

    assert_eq!(
        transport.0.lock().unwrap().as_slice(),
        ["already-rendered Markdown"]
    );
}
