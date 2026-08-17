use super::snapshot_store::{InMemoryWeeklyRadarSnapshotStore, WeeklyRadarSnapshotStore};

#[test]
fn application_registers_the_snapshot_store_boundary() {
    let store = InMemoryWeeklyRadarSnapshotStore::new();

    assert!(store.snapshots().is_empty());
}
