//! Standalone Threshold Distance read-only boundary.
//!
//! `Distance` is deliberately supplied by an upstream producer. This module
//! retains that value and the evidence lists; it does not derive a distance,
//! compare stages, or make an evidence judgment.

use std::fmt;

#[cfg(test)]
mod threshold_distance_test;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, ThresholdDistanceDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(ThresholdDistanceDomainError::EmptyValue { field });
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
            pub fn new(value: impl Into<String>) -> Result<Self, ThresholdDistanceDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the original boundary value without normalization.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity retained by a Threshold Distance read model."
);
text_value!(
    StageLabel,
    "stage label",
    "Opaque current or next stage label supplied to the Threshold Distance boundary."
);
text_value!(
    EvidenceId,
    "evidence id",
    "Stable identity for a confirmed or missing evidence reference."
);

/// The four stable labels accepted as an upstream-supplied Distance value.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Distance {
    /// The supplied evidence remains far from the next threshold.
    Far,
    /// The supplied evidence is developing toward the next threshold.
    Developing,
    /// The supplied evidence is near the next threshold.
    Near,
    /// The supplied evidence is a supplied candidate for the next threshold.
    Candidate,
}

impl Distance {
    /// Returns the stable uppercase label without deriving it from any facts.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Far => "FAR",
            Self::Developing => "DEVELOPING",
            Self::Near => "NEAR",
            Self::Candidate => "CANDIDATE",
        }
    }
}

impl fmt::Display for Distance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Validation failures for the standalone Threshold Distance boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThresholdDistanceDomainError {
    /// A required scalar value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A required evidence collection had no entries.
    EmptyCollection { field: &'static str },
    /// An evidence identity appeared more than once in one collection.
    DuplicateEvidence {
        collection: &'static str,
        id: String,
    },
    /// An evidence identity appeared in both Confirmed and Missing.
    EvidenceOverlap { id: String },
}

impl fmt::Display for ThresholdDistanceDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::EmptyCollection { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateEvidence { collection, id } => {
                write!(formatter, "duplicate {collection} identity {id}")
            }
            Self::EvidenceOverlap { id } => {
                write!(
                    formatter,
                    "evidence identity {id} cannot be confirmed and missing"
                )
            }
        }
    }
}

impl std::error::Error for ThresholdDistanceDomainError {}

fn validate_unique(
    collection: &'static str,
    evidence: &[EvidenceId],
) -> Result<(), ThresholdDistanceDomainError> {
    for (index, current) in evidence.iter().enumerate() {
        if evidence[..index].iter().any(|previous| previous == current) {
            return Err(ThresholdDistanceDomainError::DuplicateEvidence {
                collection,
                id: current.as_str().to_owned(),
            });
        }
    }
    Ok(())
}

/// Read-only per-company Threshold Distance facts supplied by an upstream producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ThresholdDistance {
    company: CompanyReference,
    current_stage: StageLabel,
    next_stage: StageLabel,
    confirmed_evidence: Vec<EvidenceId>,
    missing_evidence: Vec<EvidenceId>,
    distance: Distance,
}

impl ThresholdDistance {
    /// Retains supplied facts and rejects invalid evidence collection invariants.
    ///
    /// The constructor does not inspect stage names or evidence content to
    /// derive `distance`; callers must provide that upstream value explicitly.
    pub fn new(
        company: CompanyReference,
        current_stage: StageLabel,
        next_stage: StageLabel,
        confirmed_evidence: Vec<EvidenceId>,
        missing_evidence: Vec<EvidenceId>,
        distance: Distance,
    ) -> Result<Self, ThresholdDistanceDomainError> {
        if confirmed_evidence.is_empty() {
            return Err(ThresholdDistanceDomainError::EmptyCollection {
                field: "confirmed evidence",
            });
        }
        if missing_evidence.is_empty() {
            return Err(ThresholdDistanceDomainError::EmptyCollection {
                field: "missing evidence",
            });
        }

        validate_unique("confirmed evidence", &confirmed_evidence)?;
        validate_unique("missing evidence", &missing_evidence)?;

        if let Some(overlap) = confirmed_evidence
            .iter()
            .find(|confirmed| missing_evidence.iter().any(|missing| missing == *confirmed))
        {
            return Err(ThresholdDistanceDomainError::EvidenceOverlap {
                id: overlap.as_str().to_owned(),
            });
        }

        Ok(Self {
            company,
            current_stage,
            next_stage,
            confirmed_evidence,
            missing_evidence,
            distance,
        })
    }

    /// Returns the supplied company identity.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied current stage label without inference.
    pub fn current_stage(&self) -> &StageLabel {
        &self.current_stage
    }

    /// Returns the supplied next stage label without inference.
    pub fn next_stage(&self) -> &StageLabel {
        &self.next_stage
    }

    /// Returns confirmed evidence identities in their supplied order.
    pub fn confirmed_evidence(&self) -> &[EvidenceId] {
        &self.confirmed_evidence
    }

    /// Returns missing evidence identities in their supplied order.
    pub fn missing_evidence(&self) -> &[EvidenceId] {
        &self.missing_evidence
    }

    /// Returns the explicitly supplied Distance without recalculation.
    pub const fn distance(&self) -> Distance {
        self.distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_retains_the_supplied_distance() {
        let threshold = ThresholdDistance::new(
            CompanyReference::new("company").expect("company should be valid"),
            StageLabel::new("current").expect("current stage should be valid"),
            StageLabel::new("next").expect("next stage should be valid"),
            vec![EvidenceId::new("confirmed").expect("evidence should be valid")],
            vec![EvidenceId::new("missing").expect("evidence should be valid")],
            Distance::Far,
        )
        .expect("supplied threshold distance should be accepted");

        assert_eq!(threshold.distance(), Distance::Far);
    }

    #[test]
    fn constructor_rejects_evidence_overlap() {
        let error = ThresholdDistance::new(
            CompanyReference::new("company").expect("company should be valid"),
            StageLabel::new("current").expect("current stage should be valid"),
            StageLabel::new("next").expect("next stage should be valid"),
            vec![EvidenceId::new("shared").expect("evidence should be valid")],
            vec![EvidenceId::new("shared").expect("evidence should be valid")],
            Distance::Candidate,
        )
        .expect_err("overlapping evidence should be rejected");

        assert_eq!(
            error,
            ThresholdDistanceDomainError::EvidenceOverlap {
                id: "shared".to_owned()
            }
        );
    }
}
