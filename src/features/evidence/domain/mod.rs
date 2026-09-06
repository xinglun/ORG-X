//! Pure Evidence facts, provenance, and explicit evidence-set membership.

use std::collections::BTreeSet;
use std::fmt;

#[cfg(test)]
mod mod_test;

use crate::shared::domain::text_value as shared_text_value;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        shared_text_value!($name, $field, $description, EvidenceDomainError);
    };
}

text_value!(
    EvidenceId,
    "evidence id",
    "Stable identity for an evidence record."
);
text_value!(
    MissingEvidenceId,
    "missing evidence id",
    "Stable identity for a missing-evidence requirement."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company reference carried by evidence."
);
text_value!(
    ObservationTime,
    "observation time",
    "Opaque observation timestamp retained with evidence."
);
text_value!(
    EffectiveDate,
    "effective date",
    "Opaque effective date supplied with evidence."
);
text_value!(
    SourceUri,
    "source uri",
    "URI identifying the source of evidence."
);
text_value!(
    SourceTitle,
    "source title",
    "Title supplied by the evidence source."
);
text_value!(
    Claim,
    "claim",
    "Source claim retained without interpretation."
);
text_value!(
    NormalizedValue,
    "normalized value",
    "Opaque normalized value attached to a claim."
);
text_value!(
    ExtractorVersion,
    "extractor version",
    "Version of the extraction process recorded with evidence."
);
text_value!(
    ContentHash,
    "content hash",
    "Content hash supplied with evidence."
);

/// Validation and collection failures for Evidence facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A record identity already exists in an evidence set.
    DuplicateEvidenceId { id: EvidenceId },
    /// A missing-evidence identity already exists in an evidence set.
    DuplicateMissingEvidenceId { id: MissingEvidenceId },
    /// A record belongs to a company other than the evidence-set owner.
    CompanyMismatch {
        expected: CompanyReference,
        actual: CompanyReference,
    },
}

impl fmt::Display for EvidenceDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateEvidenceId { id } => {
                write!(formatter, "duplicate evidence identity {}", id.as_str())
            }
            Self::DuplicateMissingEvidenceId { id } => {
                write!(
                    formatter,
                    "duplicate missing-evidence identity {}",
                    id.as_str()
                )
            }
            Self::CompanyMismatch { expected, actual } => write!(
                formatter,
                "evidence company {} does not match set company {}",
                actual.as_str(),
                expected.as_str()
            ),
        }
    }
}

impl std::error::Error for EvidenceDomainError {}

/// Broad evidence category retained for downstream domain use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceType {
    /// Financial or filing-related evidence.
    Financial,
    /// Operating model or workflow evidence.
    Operational,
    /// Organization and responsibility evidence.
    Organization,
    /// Productivity measure evidence.
    Productivity,
    /// Transformation progress evidence.
    Transformation,
    /// A category not yet mapped by the boundary.
    Other(String),
}

/// Source classification retained without coupling to a source implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceType {
    /// Filing or regulatory material.
    Filing,
    /// Official company or technical material.
    OfficialMaterial,
    /// Independent context material.
    IndependentMaterial,
    /// Structured dataset material.
    StructuredDataset,
    /// Unverified material retained for review.
    UnverifiedMaterial,
    /// A classification not yet mapped by the boundary.
    Other(String),
}

/// Polarity used to route a record into supporting or counter evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidencePolarity {
    /// Record supports the associated research claim.
    Supporting,
    /// Record counters or weakens the associated research claim.
    Counter,
}

/// Coarse confidence classification retained as an independent quality dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Confidence {
    /// Confidence is not available from the source.
    Unknown,
    /// Low confidence classification.
    Low,
    /// Medium confidence classification.
    Medium,
    /// High confidence classification.
    High,
}

/// Freshness classification retained as an independent quality dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// Freshness is not available from the source.
    Unknown,
    /// Evidence is current for the consuming boundary.
    Current,
    /// Evidence is aging but remains available for context.
    Aging,
    /// Evidence is stale and requires review before use.
    Stale,
}

/// An evidence record with provenance, claim, and quality facts preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceRecord {
    id: EvidenceId,
    company_id: CompanyReference,
    observed_at: ObservationTime,
    effective_date: Option<EffectiveDate>,
    evidence_type: EvidenceType,
    source_type: SourceType,
    source_uri: SourceUri,
    source_title: SourceTitle,
    claim: Claim,
    normalized_value: Option<NormalizedValue>,
    polarity: EvidencePolarity,
    confidence: Confidence,
    freshness: Freshness,
    extractor_version: ExtractorVersion,
    content_hash: ContentHash,
}

impl EvidenceRecord {
    /// Creates an evidence record from already validated boundary values.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EvidenceId,
        company_id: CompanyReference,
        observed_at: ObservationTime,
        effective_date: Option<EffectiveDate>,
        evidence_type: EvidenceType,
        source_type: SourceType,
        source_uri: SourceUri,
        source_title: SourceTitle,
        claim: Claim,
        normalized_value: Option<NormalizedValue>,
        polarity: EvidencePolarity,
        confidence: Confidence,
        freshness: Freshness,
        extractor_version: ExtractorVersion,
        content_hash: ContentHash,
    ) -> Result<Self, EvidenceDomainError> {
        Ok(Self {
            id,
            company_id,
            observed_at,
            effective_date,
            evidence_type,
            source_type,
            source_uri,
            source_title,
            claim,
            normalized_value,
            polarity,
            confidence,
            freshness,
            extractor_version,
            content_hash,
        })
    }

    /// Returns the evidence identity.
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    /// Returns the company reference.
    pub fn company_id(&self) -> &CompanyReference {
        &self.company_id
    }

    /// Returns the opaque observation time.
    pub fn observed_at(&self) -> &ObservationTime {
        &self.observed_at
    }

    /// Returns the optional effective date.
    pub fn effective_date(&self) -> Option<&EffectiveDate> {
        self.effective_date.as_ref()
    }

    /// Returns the evidence category.
    pub fn evidence_type(&self) -> &EvidenceType {
        &self.evidence_type
    }

    /// Returns the source classification.
    pub fn source_type(&self) -> &SourceType {
        &self.source_type
    }

    /// Returns the source URI.
    pub fn source_uri(&self) -> &SourceUri {
        &self.source_uri
    }

    /// Returns the source title.
    pub fn source_title(&self) -> &SourceTitle {
        &self.source_title
    }

    /// Returns the source claim.
    pub fn claim(&self) -> &Claim {
        &self.claim
    }

    /// Returns the optional opaque normalized value.
    pub fn normalized_value(&self) -> Option<&NormalizedValue> {
        self.normalized_value.as_ref()
    }

    /// Returns the evidence polarity.
    pub fn polarity(&self) -> &EvidencePolarity {
        &self.polarity
    }

    /// Returns the confidence classification.
    pub fn confidence(&self) -> &Confidence {
        &self.confidence
    }

    /// Returns the freshness classification.
    pub fn freshness(&self) -> &Freshness {
        &self.freshness
    }

    /// Returns the extractor version.
    pub fn extractor_version(&self) -> &ExtractorVersion {
        &self.extractor_version
    }

    /// Returns the supplied content hash.
    pub fn content_hash(&self) -> &ContentHash {
        &self.content_hash
    }
}

/// Reason that a required evidence item is not available as a record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissingReason {
    /// The fact is unknown at the current boundary.
    Unknown,
    /// The expected source or fact is unavailable.
    Unavailable,
    /// Collection has not yet been attempted or completed.
    NotCollected,
}

/// Explicit missing-evidence requirement without a fabricated fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingEvidence {
    id: MissingEvidenceId,
    requirement: Claim,
    reason: MissingReason,
}

impl MissingEvidence {
    /// Creates a missing-evidence requirement.
    pub fn new(
        id: MissingEvidenceId,
        requirement: Claim,
        reason: MissingReason,
    ) -> Result<Self, EvidenceDomainError> {
        Ok(Self {
            id,
            requirement,
            reason,
        })
    }

    /// Returns the missing-evidence identity.
    pub fn id(&self) -> &MissingEvidenceId {
        &self.id
    }

    /// Returns the requirement stated by the consuming context.
    pub fn requirement(&self) -> &Claim {
        &self.requirement
    }

    /// Returns the declared missingness reason.
    pub fn reason(&self) -> &MissingReason {
        &self.reason
    }
}

/// Supporting, counter, and missing evidence for one company reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSet {
    company_id: CompanyReference,
    supporting: Vec<EvidenceRecord>,
    counter: Vec<EvidenceRecord>,
    missing: Vec<MissingEvidence>,
    evidence_ids: BTreeSet<EvidenceId>,
    missing_ids: BTreeSet<MissingEvidenceId>,
}

impl EvidenceSet {
    /// Creates an empty set for one company reference.
    pub fn new(company_id: CompanyReference) -> Self {
        Self {
            company_id,
            supporting: Vec::new(),
            counter: Vec::new(),
            missing: Vec::new(),
            evidence_ids: BTreeSet::new(),
            missing_ids: BTreeSet::new(),
        }
    }

    /// Returns the company reference owned by this set.
    pub fn company_id(&self) -> &CompanyReference {
        &self.company_id
    }

    /// Adds a record to its polarity collection.
    pub fn add(&mut self, record: EvidenceRecord) -> Result<(), EvidenceDomainError> {
        if record.company_id != self.company_id {
            return Err(EvidenceDomainError::CompanyMismatch {
                expected: self.company_id.clone(),
                actual: record.company_id,
            });
        }
        if !self.evidence_ids.insert(record.id.clone()) {
            return Err(EvidenceDomainError::DuplicateEvidenceId { id: record.id });
        }
        match record.polarity {
            EvidencePolarity::Supporting => self.supporting.push(record),
            EvidencePolarity::Counter => self.counter.push(record),
        }
        Ok(())
    }

    /// Adds an explicit missing-evidence requirement.
    pub fn add_missing(&mut self, missing: MissingEvidence) -> Result<(), EvidenceDomainError> {
        if !self.missing_ids.insert(missing.id.clone()) {
            return Err(EvidenceDomainError::DuplicateMissingEvidenceId { id: missing.id });
        }
        self.missing.push(missing);
        Ok(())
    }

    /// Returns supporting evidence in insertion order.
    pub fn supporting(&self) -> &[EvidenceRecord] {
        &self.supporting
    }

    /// Returns counter evidence in insertion order.
    pub fn counter(&self) -> &[EvidenceRecord] {
        &self.counter
    }

    /// Returns missing requirements in insertion order.
    pub fn missing(&self) -> &[MissingEvidence] {
        &self.missing
    }
}
