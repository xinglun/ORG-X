use super::*;

fn snapshot() -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-2026-w33").unwrap(),
        AsOf::new("2026-08-16").unwrap(),
        UniverseSnapshotId::new("universe-2026-w33").unwrap(),
        EvidenceCutoff::new("2026-08-15T23:59:59Z").unwrap(),
        ModelVersion::new("model-v1").unwrap(),
        ScoringVersion::new("scoring-v1").unwrap(),
    )
    .unwrap()
}

#[test]
fn snapshot_retains_all_supplied_metadata() {
    let snapshot = snapshot();

    assert_eq!(snapshot.id().as_str(), "snapshot-2026-w33");
    assert_eq!(snapshot.as_of().as_str(), "2026-08-16");
    assert_eq!(
        snapshot.universe_snapshot_id().as_str(),
        "universe-2026-w33"
    );
    assert_eq!(snapshot.evidence_cutoff().as_str(), "2026-08-15T23:59:59Z");
    assert_eq!(snapshot.model_version().as_str(), "model-v1");
    assert_eq!(snapshot.scoring_version().as_str(), "scoring-v1");
}

#[test]
fn blank_snapshot_metadata_is_rejected() {
    assert_eq!(
        SnapshotId::new("   "),
        Err(WeeklyRadarDomainError::EmptyValue {
            field: "snapshot id"
        })
    );
}

#[test]
fn publication_binds_snapshot_and_preserves_fact_order() {
    let snapshot = snapshot();
    let mut publication = WeeklyRadarPublication::new(snapshot.clone());

    publication
        .add_fact(
            FactId::new("executive-summary").unwrap(),
            FactValue::new("No meaningful structural change this week.").unwrap(),
        )
        .unwrap();
    publication
        .add_fact(
            FactId::new("system-health").unwrap(),
            FactValue::new("HEALTHY").unwrap(),
        )
        .unwrap();

    assert_eq!(publication.snapshot(), &snapshot);
    assert_eq!(publication.snapshot_id(), snapshot.id());
    assert_eq!(
        publication
            .facts()
            .iter()
            .map(|fact| fact.id().as_str())
            .collect::<Vec<_>>(),
        ["executive-summary", "system-health"]
    );
}

#[test]
fn publication_rejects_duplicate_fact_identity() {
    let mut publication = WeeklyRadarPublication::new(snapshot());
    let fact_id = FactId::new("top5").unwrap();

    publication
        .add_fact(fact_id.clone(), FactValue::new("first").unwrap())
        .unwrap();

    assert_eq!(
        publication.add_fact(fact_id, FactValue::new("second").unwrap()),
        Err(WeeklyRadarDomainError::DuplicateIdentity {
            entity: "publication fact",
            id: "top5".to_owned()
        })
    );
}

#[test]
fn registered_weekly_radar_read_models_are_visible_at_the_domain_boundary() {
    fn assert_registered<T>() {}

    assert_registered::<super::change_compression::WeeklyChangeCompression>();
    assert_registered::<super::system_health::SystemHealth>();
    assert_registered::<super::top5_weekly_read_model::Top5WeeklyReadModel>();
}
