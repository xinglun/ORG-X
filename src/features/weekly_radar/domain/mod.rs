//! Pure Weekly Radar snapshot and publication boundary.

use std::fmt;

use self::system_health::SystemHealth;

#[cfg(test)]
mod mod_test;

pub mod change_compression;
pub mod system_health;
pub mod top5_weekly_read_model;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, WeeklyRadarDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(WeeklyRadarDomainError::EmptyValue { field });
    }
    Ok(value)
}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank input.
            pub fn new(value: impl Into<String>) -> Result<Self, WeeklyRadarDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the original boundary value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    SnapshotId,
    "snapshot id",
    "Stable identity for one Weekly Radar snapshot."
);
text_value!(
    AsOf,
    "as_of",
    "As-of marker supplied for a Weekly Radar snapshot."
);
text_value!(
    UniverseSnapshotId,
    "universe snapshot id",
    "Identity of the universe snapshot used by a Weekly Radar run."
);
text_value!(
    EvidenceCutoff,
    "evidence cutoff",
    "Evidence cutoff supplied for a Weekly Radar snapshot."
);
text_value!(
    ModelVersion,
    "model version",
    "Model version supplied for a Weekly Radar snapshot."
);
text_value!(
    ScoringVersion,
    "scoring version",
    "Scoring version supplied for a Weekly Radar snapshot."
);
text_value!(
    FactId,
    "publication fact id",
    "Stable identity for one publication fact."
);
text_value!(
    FactValue,
    "publication fact value",
    "Opaque precomputed value retained by the publication boundary."
);

/// Validation and collection failures for Weekly Radar boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WeeklyRadarDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A publication fact identity already exists in the publication.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for WeeklyRadarDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for WeeklyRadarDomainError {}

/// Immutable metadata envelope identifying one historical Weekly Radar run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeeklyRadarSnapshot {
    id: SnapshotId,
    as_of: AsOf,
    universe_snapshot_id: UniverseSnapshotId,
    evidence_cutoff: EvidenceCutoff,
    model_version: ModelVersion,
    scoring_version: ScoringVersion,
}

impl WeeklyRadarSnapshot {
    /// Creates a snapshot from supplied metadata without recalculating facts.
    pub fn new(
        id: SnapshotId,
        as_of: AsOf,
        universe_snapshot_id: UniverseSnapshotId,
        evidence_cutoff: EvidenceCutoff,
        model_version: ModelVersion,
        scoring_version: ScoringVersion,
    ) -> Result<Self, WeeklyRadarDomainError> {
        Ok(Self {
            id,
            as_of,
            universe_snapshot_id,
            evidence_cutoff,
            model_version,
            scoring_version,
        })
    }

    /// Returns the snapshot identity.
    pub fn id(&self) -> &SnapshotId {
        &self.id
    }

    /// Returns the supplied as-of marker.
    pub fn as_of(&self) -> &AsOf {
        &self.as_of
    }

    /// Returns the supplied universe snapshot identity.
    pub fn universe_snapshot_id(&self) -> &UniverseSnapshotId {
        &self.universe_snapshot_id
    }

    /// Returns the supplied evidence cutoff.
    pub fn evidence_cutoff(&self) -> &EvidenceCutoff {
        &self.evidence_cutoff
    }

    /// Returns the supplied model version.
    pub fn model_version(&self) -> &ModelVersion {
        &self.model_version
    }

    /// Returns the supplied scoring version.
    pub fn scoring_version(&self) -> &ScoringVersion {
        &self.scoring_version
    }
}

/// One opaque, precomputed fact attached to a Weekly Radar publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationFact {
    id: FactId,
    value: FactValue,
}

impl PublicationFact {
    /// Creates a fact without interpreting or recomputing its value.
    pub fn new(id: FactId, value: FactValue) -> Self {
        Self { id, value }
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &FactId {
        &self.id
    }

    /// Returns the opaque fact value.
    pub fn value(&self) -> &FactValue {
        &self.value
    }
}

/// Immutable publication envelope bound to one Weekly Radar snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeeklyRadarPublication {
    snapshot: WeeklyRadarSnapshot,
    facts: Vec<PublicationFact>,
    system_health: Option<SystemHealth>,
}

impl WeeklyRadarPublication {
    /// Creates an empty publication for one snapshot.
    pub fn new(snapshot: WeeklyRadarSnapshot) -> Self {
        Self {
            snapshot,
            facts: Vec::new(),
            system_health: None,
        }
    }

    /// Adds one precomputed fact while preserving input order and identity.
    pub fn add_fact(&mut self, id: FactId, value: FactValue) -> Result<(), WeeklyRadarDomainError> {
        if self.facts.iter().any(|fact| fact.id() == &id) {
            return Err(WeeklyRadarDomainError::DuplicateIdentity {
                entity: "publication fact",
                id: id.as_str().to_owned(),
            });
        }
        self.facts.push(PublicationFact::new(id, value));
        Ok(())
    }

    /// Returns the snapshot bound to this publication.
    pub fn snapshot(&self) -> &WeeklyRadarSnapshot {
        &self.snapshot
    }

    /// Returns the bound snapshot identity.
    pub fn snapshot_id(&self) -> &SnapshotId {
        self.snapshot.id()
    }

    /// Returns facts in supplied order.
    pub fn facts(&self) -> &[PublicationFact] {
        &self.facts
    }

    /// Attaches one supplied System Health section without replacing an existing one.
    pub fn set_system_health(
        &mut self,
        system_health: SystemHealth,
    ) -> Result<(), WeeklyRadarDomainError> {
        if self.system_health.is_some() {
            return Err(WeeklyRadarDomainError::DuplicateIdentity {
                entity: "system health",
                id: self.snapshot.id().as_str().to_owned(),
            });
        }
        self.system_health = Some(system_health);
        Ok(())
    }

    /// Returns the optional System Health section supplied for this publication.
    pub fn system_health(&self) -> Option<&SystemHealth> {
        self.system_health.as_ref()
    }
}
