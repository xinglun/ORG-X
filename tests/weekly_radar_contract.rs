use org_x::features::weekly_radar::application::{WeeklyRadarPublishError, WeeklyRadarPublisher};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, FactId, FactValue, ModelVersion, ScoringVersion, SnapshotId,
    UniverseSnapshotId, WeeklyRadarPublication, WeeklyRadarSnapshot,
};

struct RecordingPublisher;

impl WeeklyRadarPublisher for RecordingPublisher {
    fn publish(&self, publication: &WeeklyRadarPublication) -> Result<(), WeeklyRadarPublishError> {
        if publication.facts().is_empty() {
            return Err(WeeklyRadarPublishError::Rejected {
                reason: "publication has no facts".to_owned(),
            });
        }
        Ok(())
    }
}

fn publication() -> WeeklyRadarPublication {
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
            FactValue::new("No meaningful structural change this week.").unwrap(),
        )
        .unwrap();
    publication
}

#[test]
fn publisher_port_consumes_one_precomputed_publication() {
    RecordingPublisher.publish(&publication()).unwrap();
}

#[test]
fn publisher_port_rejects_empty_publication_without_recomputing() {
    let snapshot = WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-empty").unwrap(),
        AsOf::new("2026-08-16").unwrap(),
        UniverseSnapshotId::new("universe-empty").unwrap(),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").unwrap(),
        ModelVersion::new("model-v1").unwrap(),
        ScoringVersion::new("score-v1").unwrap(),
    )
    .unwrap();

    assert_eq!(
        RecordingPublisher.publish(&WeeklyRadarPublication::new(snapshot)),
        Err(WeeklyRadarPublishError::Rejected {
            reason: "publication has no facts".to_owned()
        })
    );
}
