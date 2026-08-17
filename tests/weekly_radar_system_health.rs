use org_x::features::weekly_radar::domain::system_health::{
    EvidenceCoverage, Freshness, HealthStatus, SystemHealth,
};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, SnapshotId, UniverseSnapshotId,
    WeeklyRadarDomainError, WeeklyRadarPublication, WeeklyRadarSnapshot,
};

fn publication() -> WeeklyRadarPublication {
    WeeklyRadarPublication::new(
        WeeklyRadarSnapshot::new(
            SnapshotId::new("snapshot-health").unwrap(),
            AsOf::new("2026-08-16").unwrap(),
            UniverseSnapshotId::new("universe-health").unwrap(),
            EvidenceCutoff::new("cutoff-health").unwrap(),
            ModelVersion::new("model-health").unwrap(),
            ScoringVersion::new("score-health").unwrap(),
        )
        .unwrap(),
    )
}

#[test]
fn publication_retains_one_supplied_health_section() {
    let mut publication = publication();
    let health = SystemHealth::new(
        HealthStatus::Degraded,
        EvidenceCoverage::new(2, 3, 88).unwrap(),
        Freshness::Aging,
    );
    publication.set_system_health(health.clone()).unwrap();

    assert_eq!(publication.system_health(), Some(&health));
    assert_eq!(publication.snapshot_id().as_str(), "snapshot-health");
}

#[test]
fn publication_rejects_replacing_the_health_section() {
    let mut publication = publication();
    let first = SystemHealth::new(
        HealthStatus::Healthy,
        EvidenceCoverage::new(3, 3, 100).unwrap(),
        Freshness::Current,
    );
    let second = SystemHealth::new(
        HealthStatus::Unavailable,
        EvidenceCoverage::new(0, 3, 0).unwrap(),
        Freshness::Unknown,
    );
    publication.set_system_health(first.clone()).unwrap();

    assert_eq!(
        publication.set_system_health(second),
        Err(WeeklyRadarDomainError::DuplicateIdentity {
            entity: "system health",
            id: "snapshot-health".to_owned(),
        })
    );
    assert_eq!(publication.system_health(), Some(&first));
}
