//! Standalone Weekly Radar output for explicitly supplied Stage Transitions.
//!
//! This module organizes upstream facts only. It does not infer a transition from
//! evidence, mutate a Stage, compare historical snapshots, or perform reporting
//! and delivery work.

use std::fmt;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, StageTransitionOutputError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(StageTransitionOutputError::EmptyValue { field });
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
            pub fn new(value: impl Into<String>) -> Result<Self, StageTransitionOutputError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the exact boundary value supplied by the caller.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    TransitionEventId,
    "transition event id",
    "Stable identity for one Weekly Radar Stage Transition output."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company reference retained with a supplied transition."
);
text_value!(
    StageLabel,
    "stage label",
    "Opaque from or to Stage label supplied by the upstream boundary."
);
text_value!(
    TransitionDate,
    "transition date",
    "Opaque transition date retained without temporal interpretation."
);
text_value!(
    EvidenceId,
    "evidence id",
    "Stable identity for a supporting, counter, or missing evidence reference."
);
text_value!(
    EvidenceDescription,
    "evidence description",
    "Description retained with a supporting or counter evidence reference."
);
text_value!(
    MissingRequirement,
    "missing evidence requirement",
    "Requirement retained for a missing evidence reference."
);
text_value!(
    Confidence,
    "confidence",
    "Opaque confidence value supplied with a transition output."
);

/// Validation failures for a Stage Transition output boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageTransitionOutputError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// An event or evidence identity already belongs to this output.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for StageTransitionOutputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for StageTransitionOutputError {}

/// Explicit status supplied by the upstream Stage evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionStatus {
    /// The upstream evaluator has confirmed the transition.
    Confirmed,
    /// The upstream evaluator has explicitly marked the transition as a candidate.
    Candidate,
}

impl TransitionStatus {
    /// Returns the stable output label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Confirmed => "CONFIRMED",
            Self::Candidate => "CANDIDATE",
        }
    }
}

/// Priority fact exposed to later Weekly Radar compression.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionPriority {
    /// A transition that is not the explicit Productivity Breakout pair.
    Normal,
    /// The explicit `PRODUCTION_SYSTEM` to `PRODUCTIVITY_BREAKOUT` pair.
    ProductivityBreakoutHigh,
}

impl TransitionPriority {
    /// Returns the stable output label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::ProductivityBreakoutHigh => "PRODUCTIVITY_BREAKOUT_HIGH",
        }
    }
}

/// A supporting or counter evidence reference retained in insertion order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceReference {
    id: EvidenceId,
    description: EvidenceDescription,
}

impl EvidenceReference {
    /// Creates a reference without interpreting its description.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, StageTransitionOutputError> {
        Ok(Self {
            id: EvidenceId::new(id)?,
            description: EvidenceDescription::new(description)?,
        })
    }

    /// Returns the evidence identity.
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    /// Returns the supplied evidence description.
    pub fn description(&self) -> &EvidenceDescription {
        &self.description
    }
}

/// A missing evidence requirement retained in insertion order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingEvidence {
    id: EvidenceId,
    requirement: MissingRequirement,
}

impl MissingEvidence {
    /// Creates a missing requirement without treating it as supporting evidence.
    pub fn new(
        id: impl Into<String>,
        requirement: impl Into<String>,
    ) -> Result<Self, StageTransitionOutputError> {
        Ok(Self {
            id: EvidenceId::new(id)?,
            requirement: MissingRequirement::new(requirement)?,
        })
    }

    /// Returns the missing evidence identity.
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    /// Returns the supplied missing requirement.
    pub fn requirement(&self) -> &MissingRequirement {
        &self.requirement
    }
}

/// Read-only Weekly Radar output organizing one explicitly supplied transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageTransitionOutput {
    event_id: TransitionEventId,
    company: CompanyReference,
    from_stage: StageLabel,
    to_stage: StageLabel,
    transition_date: TransitionDate,
    status: TransitionStatus,
    supporting: Vec<EvidenceReference>,
    counter: Vec<EvidenceReference>,
    missing: Vec<MissingEvidence>,
    confidence: Confidence,
}

impl StageTransitionOutput {
    /// Creates an output from explicit upstream facts without calculating them.
    pub fn new(
        event_id: TransitionEventId,
        company: CompanyReference,
        from_stage: StageLabel,
        to_stage: StageLabel,
        transition_date: TransitionDate,
        status: TransitionStatus,
        confidence: Confidence,
    ) -> Self {
        Self {
            event_id,
            company,
            from_stage,
            to_stage,
            transition_date,
            status,
            supporting: Vec::new(),
            counter: Vec::new(),
            missing: Vec::new(),
            confidence,
        }
    }

    /// Returns the transition event identity.
    pub fn event_id(&self) -> &TransitionEventId {
        &self.event_id
    }

    /// Returns the supplied company reference.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied prior Stage label.
    pub fn prior_stage(&self) -> &StageLabel {
        &self.from_stage
    }

    /// Returns the supplied resulting Stage label.
    pub fn to_stage(&self) -> &StageLabel {
        &self.to_stage
    }

    /// Returns the supplied transition date.
    pub fn transition_date(&self) -> &TransitionDate {
        &self.transition_date
    }

    /// Returns the explicit upstream status without promotion.
    pub fn status(&self) -> &TransitionStatus {
        &self.status
    }

    /// Returns the supporting evidence in supplied order.
    pub fn supporting(&self) -> &[EvidenceReference] {
        &self.supporting
    }

    /// Returns the counter evidence in supplied order.
    pub fn counter(&self) -> &[EvidenceReference] {
        &self.counter
    }

    /// Returns the missing evidence requirements in supplied order.
    pub fn missing(&self) -> &[MissingEvidence] {
        &self.missing
    }

    /// Returns the supplied confidence without recalculation.
    pub fn confidence(&self) -> &Confidence {
        &self.confidence
    }

    /// Returns the priority fact mapped only from the two supplied Stage labels.
    pub fn priority(&self) -> TransitionPriority {
        if self.from_stage.as_str() == "PRODUCTION_SYSTEM"
            && self.to_stage.as_str() == "PRODUCTIVITY_BREAKOUT"
        {
            TransitionPriority::ProductivityBreakoutHigh
        } else {
            TransitionPriority::Normal
        }
    }

    /// Adds supporting evidence while preserving order and identity isolation.
    pub fn add_supporting(
        &mut self,
        evidence: EvidenceReference,
    ) -> Result<(), StageTransitionOutputError> {
        self.ensure_unique_evidence(evidence.id())?;
        self.supporting.push(evidence);
        Ok(())
    }

    /// Adds counter evidence while preserving order and identity isolation.
    pub fn add_counter(
        &mut self,
        evidence: EvidenceReference,
    ) -> Result<(), StageTransitionOutputError> {
        self.ensure_unique_evidence(evidence.id())?;
        self.counter.push(evidence);
        Ok(())
    }

    /// Adds a missing evidence requirement while preserving order and identity isolation.
    pub fn add_missing(
        &mut self,
        evidence: MissingEvidence,
    ) -> Result<(), StageTransitionOutputError> {
        self.ensure_unique_evidence(evidence.id())?;
        self.missing.push(evidence);
        Ok(())
    }

    fn ensure_unique_evidence(&self, id: &EvidenceId) -> Result<(), StageTransitionOutputError> {
        let already_present = self.supporting.iter().any(|item| item.id() == id)
            || self.counter.iter().any(|item| item.id() == id)
            || self.missing.iter().any(|item| item.id() == id);
        if already_present {
            return Err(StageTransitionOutputError::DuplicateIdentity {
                entity: "transition evidence",
                id: id.as_str().to_owned(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "stage_transition_output_test.rs"]
mod stage_transition_output_test;
