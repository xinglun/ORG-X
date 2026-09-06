//! Pure facts and validation for the Ingestion bounded context.

use std::collections::BTreeSet;
use std::fmt;

#[cfg(test)]
mod mod_test;

use crate::shared::domain::text_value as shared_text_value;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        shared_text_value!($name, $field, $description, IngestionDomainError);
    };
}

text_value!(
    ObservationId,
    "observation id",
    "Stable identity for an external observation."
);
text_value!(
    IngestionReceiptId,
    "ingestion receipt id",
    "Stable identity for an ingestion receipt."
);
text_value!(
    SourceUri,
    "source uri",
    "URI identifying the source of an observation."
);
text_value!(
    SourceTitle,
    "source title",
    "Title supplied by the observation source."
);
text_value!(
    ObservationTime,
    "observation time",
    "Opaque observation timestamp retained at ingestion."
);
text_value!(
    EffectiveDate,
    "effective date",
    "Opaque effective date supplied by a source."
);
text_value!(
    ContentHash,
    "content hash",
    "Content hash supplied with an observation."
);

/// Validation failures for Ingestion facts and receipt assembly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IngestionDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A receipt already contains the supplied observation identity.
    DuplicateObservationId { id: ObservationId },
}

impl fmt::Display for IngestionDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateObservationId { id } => {
                write!(formatter, "duplicate observation identity {}", id.as_str())
            }
        }
    }
}

impl std::error::Error for IngestionDomainError {}

/// Source authority tier supplied by the data-source policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceTier {
    /// Regulatory filings and official company materials.
    A,
    /// Official engineering, careers, executive, or strategy materials.
    B,
    /// Independent financial or industry context.
    C,
    /// Structured datasets and workforce or fundamental estimates.
    D,
    /// Social, forum, or otherwise unverified material.
    E,
}

/// Classification supplied by ingestion without interpreting the observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationKind {
    /// Regulatory filing or equivalent official filing.
    Filing,
    /// Official investor or earnings material.
    InvestorMaterial,
    /// Official careers material.
    Careers,
    /// Official engineering or technical material.
    EngineeringMaterial,
    /// Independent context from a quality source.
    IndependentContext,
    /// Structured dataset observation.
    StructuredDataset,
    /// Unverified claim retained as source material.
    UnverifiedClaim,
    /// A source classification not yet mapped by the boundary.
    Other(String),
}

/// Opaque source observation with provenance metadata preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    id: ObservationId,
    source_uri: SourceUri,
    source_title: SourceTitle,
    observed_at: ObservationTime,
    effective_date: Option<EffectiveDate>,
    content_hash: ContentHash,
    source_tier: SourceTier,
    kind: ObservationKind,
    payload: Vec<u8>,
}

impl Observation {
    /// Creates an observation without interpreting its opaque payload.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ObservationId,
        source_uri: SourceUri,
        source_title: SourceTitle,
        observed_at: ObservationTime,
        effective_date: Option<EffectiveDate>,
        content_hash: ContentHash,
        source_tier: SourceTier,
        kind: ObservationKind,
        payload: impl Into<Vec<u8>>,
    ) -> Result<Self, IngestionDomainError> {
        Ok(Self {
            id,
            source_uri,
            source_title,
            observed_at,
            effective_date,
            content_hash,
            source_tier,
            kind,
            payload: payload.into(),
        })
    }

    /// Returns the observation identity.
    pub fn id(&self) -> &ObservationId {
        &self.id
    }

    /// Returns the source URI.
    pub fn source_uri(&self) -> &SourceUri {
        &self.source_uri
    }

    /// Returns the source title.
    pub fn source_title(&self) -> &SourceTitle {
        &self.source_title
    }

    /// Returns the opaque observation timestamp.
    pub fn observed_at(&self) -> &ObservationTime {
        &self.observed_at
    }

    /// Returns the optional source-supplied effective date.
    pub fn effective_date(&self) -> Option<&EffectiveDate> {
        self.effective_date.as_ref()
    }

    /// Returns the supplied content hash.
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }

    /// Returns the source authority tier.
    pub fn source_tier(&self) -> &SourceTier {
        &self.source_tier
    }

    /// Returns the source classification.
    pub fn kind(&self) -> &ObservationKind {
        &self.kind
    }

    /// Returns the payload exactly as supplied at the boundary.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

/// Receipt that preserves accepted observation order and identity uniqueness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestionReceipt {
    id: IngestionReceiptId,
    accepted_at: ObservationTime,
    observations: Vec<Observation>,
    observation_ids: BTreeSet<ObservationId>,
}

impl IngestionReceipt {
    /// Starts an empty receipt at the supplied acceptance time.
    pub fn new(id: IngestionReceiptId, accepted_at: ObservationTime) -> Self {
        Self {
            id,
            accepted_at,
            observations: Vec::new(),
            observation_ids: BTreeSet::new(),
        }
    }

    /// Returns the receipt identity.
    pub fn id(&self) -> &IngestionReceiptId {
        &self.id
    }

    /// Returns the opaque receipt acceptance time.
    pub fn accepted_at(&self) -> &ObservationTime {
        &self.accepted_at
    }

    /// Adds an observation while rejecting repeated identities.
    pub fn accept(&mut self, observation: Observation) -> Result<(), IngestionDomainError> {
        if !self.observation_ids.insert(observation.id.clone()) {
            return Err(IngestionDomainError::DuplicateObservationId { id: observation.id });
        }
        self.observations.push(observation);
        Ok(())
    }

    /// Returns observations in acceptance order.
    pub fn observations(&self) -> &[Observation] {
        &self.observations
    }

    /// Returns a stable identity view in the same order as accepted observations.
    pub fn observation_ids(&self) -> Vec<&ObservationId> {
        self.observations.iter().map(Observation::id).collect()
    }
}
