use org_x::features::weekly_radar::application::snapshot_store::{
    InMemoryWeeklyRadarSnapshotStore, SnapshotStoreError, WeeklyRadarSnapshotStore,
};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, SnapshotId, UniverseSnapshotId,
    WeeklyRadarSnapshot,
};

fn snapshot(id: &str, as_of: &str) -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new(id).unwrap(),
        AsOf::new(as_of).unwrap(),
        UniverseSnapshotId::new(format!("universe-{id}")).unwrap(),
        EvidenceCutoff::new(format!("cutoff-{id}")).unwrap(),
        ModelVersion::new(format!("model-{id}")).unwrap(),
        ScoringVersion::new(format!("score-{id}")).unwrap(),
    )
    .unwrap()
}

#[test]
fn empty_store_has_no_history() {
    let store = InMemoryWeeklyRadarSnapshotStore::new();

    assert!(store.snapshots().is_empty());
    assert!(store.get(&SnapshotId::new("missing").unwrap()).is_none());
}

#[test]
fn stores_exact_metadata_and_retrieves_in_append_order() {
    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    let first = snapshot("first", "2026-08-15");
    let second = snapshot("second", "2026-08-16");

    store.save(first.clone()).unwrap();
    store.save(second.clone()).unwrap();

    assert_eq!(store.get(first.id()), Some(&first));
    assert_eq!(store.get(second.id()), Some(&second));
    assert_eq!(store.snapshots(), &[first, second]);
}

#[test]
fn duplicate_identity_is_rejected_without_overwriting_first_snapshot() {
    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    let first = snapshot("same", "2026-08-15");
    let replacement = snapshot("same", "2026-08-16");

    store.save(first.clone()).unwrap();
    assert_eq!(
        store.save(replacement),
        Err(SnapshotStoreError::DuplicateSnapshot {
            id: "same".to_owned(),
        })
    );
    assert_eq!(store.snapshots(), &[first]);
}
