//! Application ports for publishing a precomputed Weekly Radar publication.

pub mod snapshot_store;
pub mod weekly_scheduler;

#[cfg(test)]
mod mod_test;

use super::domain::WeeklyRadarPublication;

/// Boundary failures returned by a publication adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeeklyRadarPublishError {
    /// The adapter is not available for this attempt.
    Unavailable,
    /// The adapter rejected the already-built publication.
    Rejected { reason: String },
    /// The adapter reported a delivery failure without changing the publication.
    Failed { reason: String },
}

/// Provider-agnostic port for delivering one precomputed publication.
///
/// Implementations belong outside the Weekly Radar Domain. A publisher receives
/// an immutable publication and must not recalculate any research conclusion.
pub trait WeeklyRadarPublisher {
    /// Delivers the supplied publication or reports a boundary failure.
    fn publish(&self, publication: &WeeklyRadarPublication) -> Result<(), WeeklyRadarPublishError>;
}
