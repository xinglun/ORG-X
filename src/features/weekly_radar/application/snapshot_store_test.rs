use super::{InMemoryWeeklyRadarSnapshotStore, SnapshotStoreError, WeeklyRadarSnapshotStore};
use crate::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, SnapshotId, UniverseSnapshotId,
    WeeklyRadarSnapshot,
};

fn snapshot(id: &str) -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new(id).unwrap(),
        AsOf::new(format!("as-of-{id}")).unwrap(),
        UniverseSnapshotId::new(format!("universe-{id}")).unwrap(),
        EvidenceCutoff::new(format!("cutoff-{id}")).unwrap(),
        ModelVersion::new(format!("model-{id}")).unwrap(),
        ScoringVersion::new(format!("score-{id}")).unwrap(),
    )
    .unwrap()
}

#[test]
fn append_order_and_empty_history_are_stable() {
    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    assert!(store.snapshots().is_empty());

    let first = snapshot("first");
    let second = snapshot("second");
    store.save(first.clone()).unwrap();
    store.save(second.clone()).unwrap();

    assert_eq!(store.snapshots(), &[first, second]);
}

#[test]
fn duplicate_identity_does_not_replace_history() {
    let mut store = InMemoryWeeklyRadarSnapshotStore::new();
    let first = snapshot("same");
    let replacement = WeeklyRadarSnapshot::new(
        SnapshotId::new("same").unwrap(),
        AsOf::new("different-as-of").unwrap(),
        UniverseSnapshotId::new("different-universe").unwrap(),
        EvidenceCutoff::new("different-cutoff").unwrap(),
        ModelVersion::new("different-model").unwrap(),
        ScoringVersion::new("different-score").unwrap(),
    )
    .unwrap();

    store.save(first.clone()).unwrap();
    assert_eq!(
        store.save(replacement),
        Err(SnapshotStoreError::DuplicateSnapshot {
            id: "same".to_owned(),
        })
    );
    assert_eq!(store.snapshots(), &[first]);
}
