//! Pure, stage-isolated ranking read model for research priority.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, RankingDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(RankingDomainError::EmptyValue { field });
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
            pub fn new(value: impl Into<String>) -> Result<Self, RankingDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the original value supplied at the boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

macro_rules! bounded_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
        pub struct $name(u8);

        impl $name {
            /// Creates a bounded percentage-style value from 0 through 100.
            pub fn new(value: u8) -> Result<Self, RankingDomainError> {
                if value > 100 {
                    return Err(RankingDomainError::OutOfRange {
                        field: $field,
                        value,
                    });
                }
                Ok(Self(value))
            }

            /// Returns the retained numeric value without recalculation.
            pub fn as_u8(self) -> u8 {
                self.0
            }
        }
    };
}

text_value!(
    RankingCandidateId,
    "ranking candidate id",
    "Stable identity for a candidate in the Ranking Read Model."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity retained by a ranking candidate."
);

bounded_value!(
    EvidenceConfidence,
    "evidence confidence",
    "Independent evidence-confidence value used within one Stage."
);
bounded_value!(
    TransformationScore,
    "transformation score",
    "Independent transformation score used within one Stage."
);
bounded_value!(
    CounterEvidenceRisk,
    "counter evidence risk",
    "Independent counter-evidence risk value; lower values rank first."
);
bounded_value!(
    EvidenceFreshness,
    "evidence freshness",
    "Independent freshness value used within one Stage."
);

/// Validation and collection failures for the Ranking Read Model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RankingDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A bounded value was outside 0 through 100.
    OutOfRange { field: &'static str, value: u8 },
    /// A candidate identity already exists in the read model.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for RankingDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::OutOfRange { field, value } => {
                write!(formatter, "{field} value {value} must be between 0 and 100")
            }
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for RankingDomainError {}

/// Stage grouping owned by the Ranking boundary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// AI is used as a tool.
    Tool,
    /// AI substitutes for a local human task.
    Substitution,
    /// A workflow is reorganized around AI execution and supervision.
    Workflow,
    /// The core production system is redesigned around AI.
    ProductionSystem,
    /// The changed production system has a persistent productivity advantage.
    ProductivityBreakout,
    /// A production system becomes a reference model.
    ReferenceModel,
}

impl Stage {
    /// Returns the stable documented order for the six stage labels.
    pub fn rank(self) -> u8 {
        match self {
            Self::Tool => 0,
            Self::Substitution => 1,
            Self::Workflow => 2,
            Self::ProductionSystem => 3,
            Self::ProductivityBreakout => 4,
            Self::ReferenceModel => 5,
        }
    }

    /// Returns the stable uppercase stage label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Tool => "TOOL",
            Self::Substitution => "SUBSTITUTION",
            Self::Workflow => "WORKFLOW",
            Self::ProductionSystem => "PRODUCTION_SYSTEM",
            Self::ProductivityBreakout => "PRODUCTIVITY_BREAKOUT",
            Self::ReferenceModel => "REFERENCE_MODEL",
        }
    }
}

/// One candidate with independent ranking dimensions retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankingCandidate {
    id: RankingCandidateId,
    company: CompanyReference,
    stage: Stage,
    evidence_confidence: EvidenceConfidence,
    transformation_score: TransformationScore,
    counter_evidence_risk: CounterEvidenceRisk,
    evidence_freshness: EvidenceFreshness,
}

impl RankingCandidate {
    /// Creates a candidate without calculating any ranking dimension.
    pub fn new(
        id: RankingCandidateId,
        company: CompanyReference,
        stage: Stage,
        evidence_confidence: EvidenceConfidence,
        transformation_score: TransformationScore,
        counter_evidence_risk: CounterEvidenceRisk,
        evidence_freshness: EvidenceFreshness,
    ) -> Result<Self, RankingDomainError> {
        Ok(Self {
            id,
            company,
            stage,
            evidence_confidence,
            transformation_score,
            counter_evidence_risk,
            evidence_freshness,
        })
    }

    /// Returns the stable candidate identity.
    pub fn id(&self) -> &RankingCandidateId {
        &self.id
    }

    /// Returns the candidate company.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the explicitly assigned grouping Stage.
    pub fn stage(&self) -> Stage {
        self.stage
    }

    /// Returns the independent evidence confidence.
    pub fn evidence_confidence(&self) -> EvidenceConfidence {
        self.evidence_confidence
    }

    /// Returns the independent transformation score.
    pub fn transformation_score(&self) -> TransformationScore {
        self.transformation_score
    }

    /// Returns the independent counter-evidence risk.
    pub fn counter_evidence_risk(&self) -> CounterEvidenceRisk {
        self.counter_evidence_risk
    }

    /// Returns the independent evidence freshness.
    pub fn evidence_freshness(&self) -> EvidenceFreshness {
        self.evidence_freshness
    }
}

/// Ordered candidate storage with explicit same-Stage ranking.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RankingReadModel {
    candidates: Vec<RankingCandidate>,
}

impl RankingReadModel {
    /// Creates an empty Read Model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a candidate while rejecting duplicate identity.
    pub fn add(&mut self, candidate: RankingCandidate) -> Result<(), RankingDomainError> {
        if self
            .candidates
            .iter()
            .any(|item| item.id() == candidate.id())
        {
            return Err(RankingDomainError::DuplicateIdentity {
                entity: "ranking candidate",
                id: candidate.id().as_str().to_owned(),
            });
        }
        self.candidates.push(candidate);
        Ok(())
    }

    /// Returns candidates in insertion order.
    pub fn candidates(&self) -> &[RankingCandidate] {
        &self.candidates
    }

    /// Returns only the selected Stage, ordered by the fixed Ranking keys.
    pub fn ranked_within_stage(&self, stage: Stage) -> Vec<&RankingCandidate> {
        let mut ranked: Vec<_> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.stage == stage)
            .collect();
        ranked.sort_by(|left, right| {
            right
                .evidence_confidence
                .cmp(&left.evidence_confidence)
                .then_with(|| right.transformation_score.cmp(&left.transformation_score))
                .then_with(|| left.counter_evidence_risk.cmp(&right.counter_evidence_risk))
                .then_with(|| right.evidence_freshness.cmp(&left.evidence_freshness))
                .then_with(|| left.id.cmp(&right.id))
        });
        ranked
    }
}
