//! Append-only archive boundary for verified Weekly Radar publications.
//!
//! The in-memory implementation is deliberately provider-agnostic. It keeps
//! the archive contract executable without introducing an external database,
//! runtime, or persistence policy into the end-to-end Work Item.

use std::fmt;

use crate::features::weekly_radar::domain::{SnapshotId, WeeklyRadarSnapshot};

use super::publication_receipt::{PublicationReceipt, PublicationStatus};

/// Validation failures for an archive append.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeeklyRadarArchiveError {
    /// A snapshot identity has already been archived.
    DuplicateSnapshot { id: String },
    /// Only a fully published receipt can become an archive record.
    ReceiptNotPublished { snapshot_id: String },
    /// The snapshot and receipt must refer to the same immutable identity.
    SnapshotMismatch {
        snapshot_id: String,
        receipt_snapshot_id: String,
    },
}

impl fmt::Display for WeeklyRadarArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSnapshot { id } => {
                write!(formatter, "snapshot {id} is already archived")
            }
            Self::ReceiptNotPublished { snapshot_id } => write!(
                formatter,
                "snapshot {snapshot_id} cannot be archived before publication succeeds"
            ),
            Self::SnapshotMismatch {
                snapshot_id,
                receipt_snapshot_id,
            } => write!(
                formatter,
                "archive snapshot mismatch: snapshot {snapshot_id}, receipt {receipt_snapshot_id}"
            ),
        }
    }
}

impl std::error::Error for WeeklyRadarArchiveError {}

/// One immutable snapshot/receipt pair retained by the archive.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivedWeeklyRadarPublication {
    snapshot: WeeklyRadarSnapshot,
    receipt: PublicationReceipt,
}

impl ArchivedWeeklyRadarPublication {
    /// Returns the archived snapshot value.
    pub fn snapshot(&self) -> &WeeklyRadarSnapshot {
        &self.snapshot
    }

    /// Returns the published receipt paired with the snapshot.
    pub fn receipt(&self) -> &PublicationReceipt {
        &self.receipt
    }
}

/// Provider-agnostic append/read port for Weekly Radar archive records.
pub trait WeeklyRadarArchive {
    /// Appends one published snapshot/receipt pair without overwriting history.
    fn archive(
        &mut self,
        snapshot: WeeklyRadarSnapshot,
        receipt: PublicationReceipt,
    ) -> Result<(), WeeklyRadarArchiveError>;

    /// Looks up an archived record by immutable snapshot identity.
    fn get(&self, id: &SnapshotId) -> Option<&ArchivedWeeklyRadarPublication>;

    /// Returns archive records in append order.
    fn entries(&self) -> &[ArchivedWeeklyRadarPublication];
}

/// Deterministic append-only archive used by the current infrastructure boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryWeeklyRadarArchive {
    entries: Vec<ArchivedWeeklyRadarPublication>,
}

impl InMemoryWeeklyRadarArchive {
    /// Creates an empty archive.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl WeeklyRadarArchive for InMemoryWeeklyRadarArchive {
    fn archive(
        &mut self,
        snapshot: WeeklyRadarSnapshot,
        receipt: PublicationReceipt,
    ) -> Result<(), WeeklyRadarArchiveError> {
        let snapshot_id = snapshot.id().as_str();
        let receipt_snapshot_id = receipt.snapshot_id().as_str();
        if snapshot.id() != receipt.snapshot_id() {
            return Err(WeeklyRadarArchiveError::SnapshotMismatch {
                snapshot_id: snapshot_id.to_owned(),
                receipt_snapshot_id: receipt_snapshot_id.to_owned(),
            });
        }
        if receipt.status() != &PublicationStatus::Published {
            return Err(WeeklyRadarArchiveError::ReceiptNotPublished {
                snapshot_id: snapshot_id.to_owned(),
            });
        }
        if self
            .entries
            .iter()
            .any(|entry| entry.snapshot.id() == snapshot.id())
        {
            return Err(WeeklyRadarArchiveError::DuplicateSnapshot {
                id: snapshot_id.to_owned(),
            });
        }
        self.entries
            .push(ArchivedWeeklyRadarPublication { snapshot, receipt });
        Ok(())
    }

    fn get(&self, id: &SnapshotId) -> Option<&ArchivedWeeklyRadarPublication> {
        self.entries.iter().find(|entry| entry.snapshot.id() == id)
    }

    fn entries(&self) -> &[ArchivedWeeklyRadarPublication] {
        &self.entries
    }
}

#[cfg(test)]
#[path = "archive_store_test.rs"]
mod module_tests;
