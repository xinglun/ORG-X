//! Pure transformation stages, transitions, proof polarity, and persistence facts.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, TransformationDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(TransformationDomainError::EmptyValue { field });
    }
    Ok(value)
}

fn duplicate_identity(entity: &'static str, id: impl Into<String>) -> TransformationDomainError {
    TransformationDomainError::DuplicateIdentity {
        entity,
        id: id.into(),
    }
}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank input.
            pub fn new(value: impl Into<String>) -> Result<Self, TransformationDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the original value supplied at the boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    CompanyReference,
    "company reference",
    "Opaque company reference for a transformation assessment."
);
text_value!(
    StageTransitionId,
    "stage transition id",
    "Stable identity for a stage transition."
);
text_value!(
    TransitionDate,
    "transition date",
    "Opaque date retained with a stage transition."
);
text_value!(
    ProofId,
    "proof id",
    "Stable identity for supporting or counter proof."
);
text_value!(
    ProofDescription,
    "proof description",
    "Description retained with a proof reference."
);
text_value!(
    MissingProofId,
    "missing proof id",
    "Stable identity for a missing proof requirement."
);
text_value!(
    MissingRequirement,
    "missing requirement",
    "Requirement retained for missing proof."
);
text_value!(
    PersistenceWindow,
    "persistence window",
    "Opaque persistence window retained with an assessment."
);
text_value!(
    ObservationCount,
    "observation count",
    "Opaque count of observations retained with persistence."
);

/// Validation and collection failures for transformation facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformationDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// An identity already exists in its owning collection.
    DuplicateIdentity { entity: &'static str, id: String },
    /// A transition attempted to move from a stage to itself.
    SameStageTransition { stage: Stage },
}

impl fmt::Display for TransformationDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
            Self::SameStageTransition { stage } => {
                write!(formatter, "stage transition cannot remain at {stage:?}")
            }
        }
    }
}

impl std::error::Error for TransformationDomainError {}

/// The six transformation stages documented by ORG-X.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    /// AI is used as a tool.
    Tool,
    /// AI substitutes for a local human task.
    Substitution,
    /// The workflow is reorganized around AI execution and human supervision.
    Workflow,
    /// The core production system is redesigned around AI.
    ProductionSystem,
    /// The changed production system produces persistent productivity advantage.
    ProductivityBreakout,
    /// The production system becomes a reference model through diffusion.
    ReferenceModel,
}

impl Stage {
    /// Returns the documented order of this stage.
    pub fn rank(&self) -> u8 {
        match self {
            Self::Tool => 0,
            Self::Substitution => 1,
            Self::Workflow => 2,
            Self::ProductionSystem => 3,
            Self::ProductivityBreakout => 4,
            Self::ReferenceModel => 5,
        }
    }

    /// Returns the stable uppercase label used by the research model.
    pub fn label(&self) -> &'static str {
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

/// An explicit stage transition, including corrective downgrades.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageTransition {
    id: StageTransitionId,
    from: Stage,
    to: Stage,
    transition_date: TransitionDate,
}

impl StageTransition {
    /// Creates a transition and rejects a same-stage no-op.
    pub fn new(
        id: StageTransitionId,
        from: Stage,
        to: Stage,
        transition_date: impl Into<String>,
    ) -> Result<Self, TransformationDomainError> {
        if from == to {
            return Err(TransformationDomainError::SameStageTransition { stage: from });
        }
        Ok(Self {
            id,
            from,
            to,
            transition_date: TransitionDate::new(transition_date)?,
        })
    }

    /// Returns the transition identity.
    pub fn id(&self) -> &StageTransitionId {
        &self.id
    }

    /// Returns the prior stage.
    pub fn from(&self) -> &Stage {
        &self.from
    }

    /// Returns the resulting stage.
    pub fn to(&self) -> &Stage {
        &self.to
    }

    /// Returns the supplied transition date.
    pub fn transition_date(&self) -> &TransitionDate {
        &self.transition_date
    }
}

/// A proof reference whose polarity is assigned by collection membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofReference {
    id: ProofId,
    description: ProofDescription,
}

impl ProofReference {
    /// Creates a proof reference without judging its quality.
    pub fn new(
        id: ProofId,
        description: impl Into<String>,
    ) -> Result<Self, TransformationDomainError> {
        Ok(Self {
            id,
            description: ProofDescription::new(description)?,
        })
    }

    /// Returns the proof identity.
    pub fn id(&self) -> &ProofId {
        &self.id
    }

    /// Returns the retained proof description.
    pub fn description(&self) -> &ProofDescription {
        &self.description
    }
}

/// A missing proof requirement retained explicitly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingProof {
    id: MissingProofId,
    requirement: MissingRequirement,
}

impl MissingProof {
    /// Creates a missing proof requirement without converting absence into support.
    pub fn new(
        id: MissingProofId,
        requirement: impl Into<String>,
    ) -> Result<Self, TransformationDomainError> {
        Ok(Self {
            id,
            requirement: MissingRequirement::new(requirement)?,
        })
    }

    /// Returns the missing-proof identity.
    pub fn id(&self) -> &MissingProofId {
        &self.id
    }

    /// Returns the missing requirement.
    pub fn requirement(&self) -> &MissingRequirement {
        &self.requirement
    }
}

/// Supporting, counter, and missing proof collections for one assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationProofSet {
    supporting: Vec<ProofReference>,
    counter: Vec<ProofReference>,
    missing: Vec<MissingProof>,
}

impl TransformationProofSet {
    /// Creates an empty proof set.
    pub fn new() -> Self {
        Self {
            supporting: Vec::new(),
            counter: Vec::new(),
            missing: Vec::new(),
        }
    }

    /// Returns supporting proof in insertion order.
    pub fn supporting(&self) -> &[ProofReference] {
        &self.supporting
    }

    /// Returns counter proof in insertion order.
    pub fn counter(&self) -> &[ProofReference] {
        &self.counter
    }

    /// Returns missing proof requirements in insertion order.
    pub fn missing(&self) -> &[MissingProof] {
        &self.missing
    }

    /// Adds supporting proof unless its identity is already present.
    pub fn add_supporting(
        &mut self,
        proof: ProofReference,
    ) -> Result<(), TransformationDomainError> {
        if self
            .supporting
            .iter()
            .any(|existing| existing.id == proof.id)
            || self.counter.iter().any(|existing| existing.id == proof.id)
        {
            return Err(duplicate_identity(
                "transformation proof",
                proof.id.as_str(),
            ));
        }
        self.supporting.push(proof);
        Ok(())
    }

    /// Adds counter proof unless its identity is already present.
    pub fn add_counter(&mut self, proof: ProofReference) -> Result<(), TransformationDomainError> {
        if self
            .supporting
            .iter()
            .any(|existing| existing.id == proof.id)
            || self.counter.iter().any(|existing| existing.id == proof.id)
        {
            return Err(duplicate_identity(
                "transformation proof",
                proof.id.as_str(),
            ));
        }
        self.counter.push(proof);
        Ok(())
    }

    /// Adds a missing proof requirement unless its identity is already present.
    pub fn add_missing(&mut self, proof: MissingProof) -> Result<(), TransformationDomainError> {
        if self.missing.iter().any(|existing| existing.id == proof.id) {
            return Err(duplicate_identity("missing proof", proof.id.as_str()));
        }
        self.missing.push(proof);
        Ok(())
    }
}

impl Default for TransformationProofSet {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistence facts retained without calculating duration or sufficiency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistenceFact {
    window: PersistenceWindow,
    observation_count: ObservationCount,
}

impl PersistenceFact {
    /// Creates a persistence fact from supplied window and count values.
    pub fn new(
        window: impl Into<String>,
        observation_count: impl Into<String>,
    ) -> Result<Self, TransformationDomainError> {
        Ok(Self {
            window: PersistenceWindow::new(window)?,
            observation_count: ObservationCount::new(observation_count)?,
        })
    }

    /// Returns the supplied persistence window.
    pub fn window(&self) -> &PersistenceWindow {
        &self.window
    }

    /// Returns the supplied observation count.
    pub fn observation_count(&self) -> &ObservationCount {
        &self.observation_count
    }
}

/// A transformation assessment that groups explicit stage facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationAssessment {
    company: CompanyReference,
    current_stage: Stage,
    transitions: Vec<StageTransition>,
    proofs: TransformationProofSet,
    persistence: Option<PersistenceFact>,
}

impl TransformationAssessment {
    /// Creates an assessment with an explicit current stage.
    pub fn new(company: CompanyReference, current_stage: Stage) -> Self {
        Self {
            company,
            current_stage,
            transitions: Vec::new(),
            proofs: TransformationProofSet::new(),
            persistence: None,
        }
    }

    /// Returns the company reference.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the explicitly supplied current stage.
    pub fn current_stage(&self) -> &Stage {
        &self.current_stage
    }

    /// Returns transitions in insertion order.
    pub fn transitions(&self) -> &[StageTransition] {
        &self.transitions
    }

    /// Returns the assessment proof set.
    pub fn proofs(&self) -> &TransformationProofSet {
        &self.proofs
    }

    /// Returns persistence when supplied.
    pub fn persistence(&self) -> Option<&PersistenceFact> {
        self.persistence.as_ref()
    }

    /// Adds a transition unless its identity already exists.
    pub fn add_transition(
        &mut self,
        transition: StageTransition,
    ) -> Result<(), TransformationDomainError> {
        if self
            .transitions
            .iter()
            .any(|existing| existing.id == transition.id)
        {
            return Err(duplicate_identity(
                "stage transition",
                transition.id.as_str(),
            ));
        }
        self.transitions.push(transition);
        Ok(())
    }

    /// Replaces the explicit proof set.
    pub fn set_proofs(&mut self, proofs: TransformationProofSet) {
        self.proofs = proofs;
    }

    /// Replaces the explicit persistence fact.
    pub fn set_persistence(&mut self, persistence: PersistenceFact) {
        self.persistence = Some(persistence);
    }
}
