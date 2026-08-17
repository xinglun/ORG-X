//! Append-only storage boundary for already-computed Weekly Radar snapshots.

use super::super::domain::{SnapshotId, WeeklyRadarDomainError, WeeklyRadarSnapshot};

#[cfg(test)]
#[path = "snapshot_store_test.rs"]
mod snapshot_store_test;

/// Failures returned when a snapshot cannot be appended to the store.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SnapshotStoreError {
    /// A snapshot with the same immutable identity already exists.
    DuplicateSnapshot { id: String },
    /// The supplied snapshot failed domain validation.
    Domain(WeeklyRadarDomainError),
}

impl From<WeeklyRadarDomainError> for SnapshotStoreError {
    fn from(error: WeeklyRadarDomainError) -> Self {
        Self::Domain(error)
    }
}

/// Provider-agnostic boundary for retaining computed snapshot envelopes.
pub trait WeeklyRadarSnapshotStore {
    /// Appends one snapshot and rejects identity reuse without overwriting history.
    fn save(&mut self, snapshot: WeeklyRadarSnapshot) -> Result<(), SnapshotStoreError>;

    /// Looks up a snapshot by its immutable identity.
    fn get(&self, id: &SnapshotId) -> Option<&WeeklyRadarSnapshot>;

    /// Returns all snapshots in their original append order.
    fn snapshots(&self) -> &[WeeklyRadarSnapshot];
}

/// In-memory append-only store used by the application boundary and tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryWeeklyRadarSnapshotStore {
    snapshots: Vec<WeeklyRadarSnapshot>,
}

impl InMemoryWeeklyRadarSnapshotStore {
    /// Creates an empty snapshot history.
    pub fn new() -> Self {
        Self::default()
    }
}

impl WeeklyRadarSnapshotStore for InMemoryWeeklyRadarSnapshotStore {
    fn save(&mut self, snapshot: WeeklyRadarSnapshot) -> Result<(), SnapshotStoreError> {
        if self
            .snapshots
            .iter()
            .any(|stored| stored.id() == snapshot.id())
        {
            return Err(SnapshotStoreError::DuplicateSnapshot {
                id: snapshot.id().as_str().to_owned(),
            });
        }

        self.snapshots.push(snapshot);
        Ok(())
    }

    fn get(&self, id: &SnapshotId) -> Option<&WeeklyRadarSnapshot> {
        self.snapshots.iter().find(|snapshot| snapshot.id() == id)
    }

    fn snapshots(&self) -> &[WeeklyRadarSnapshot] {
        &self.snapshots
    }
}
