//! Pure transformation stages, transitions, proof polarity, and persistence facts.

use std::fmt;

use serde::{Deserialize, Serialize};

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

/// The four independent evidence families required for a reference model.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceModelEvidenceFamily {
    /// Organization, responsibility, reporting, or decision-rights rewrite.
    OrganizationRewrite,
    /// Core production workflow, AI execution, supervision, or control rewrite.
    ProductionSystemRewrite,
    /// Persistent operating or economic outcome over distinct periods.
    SustainedOutcome,
    /// Independent peer adoption, imitation, or industry diffusion.
    IndustryDiffusion,
}

impl ReferenceModelEvidenceFamily {
    /// Returns the stable machine and report label for this family.
    pub const fn label(self) -> &'static str {
        match self {
            Self::OrganizationRewrite => "ORGANIZATION_REWRITE",
            Self::ProductionSystemRewrite => "PRODUCTION_SYSTEM_REWRITE",
            Self::SustainedOutcome => "SUSTAINED_OUTCOME",
            Self::IndustryDiffusion => "INDUSTRY_DIFFUSION",
        }
    }
}

/// Provenance role used to keep supplier attribution separate from
/// independent diffusion corroboration.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceModelSourceRole {
    /// Supplier-controlled material that attributes a technical implementation.
    SupplierAttribution,
    /// Adopter-owned disclosure that corroborates adoption or imitation.
    IndependentCustomerDisclosure,
    /// Regulatory filing or investor-relations result material.
    RegulatoryOrFiling,
    /// Secondary or discovery material that cannot satisfy a hard gate.
    DiscoveryOnly,
}

/// The fail-closed outcome of the reference-model evidence gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceModelEligibility {
    /// The core rewrite exists but one or more hard conditions remain open.
    Candidate,
    /// All four evidence families and the counter-review gate passed.
    Confirmed,
    /// The core rewrite needed to make a reference-model claim is absent.
    NotEligible,
}

impl ReferenceModelEligibility {
    /// Returns the stable machine and report label for this outcome.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Candidate => "CANDIDATE",
            Self::Confirmed => "CONFIRMED",
            Self::NotEligible => "NOT_ELIGIBLE",
        }
    }
}

/// One source-bound claim in a reference-model evidence packet.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReferenceModelEvidence {
    id: String,
    family: ReferenceModelEvidenceFamily,
    description: String,
    source_uri: String,
    period: Option<String>,
    named_peer: Option<String>,
    authoritative: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_role: Option<ReferenceModelSourceRole>,
}

impl ReferenceModelEvidence {
    /// Creates a source-bound claim without inferring its family or authority.
    pub fn new(
        id: impl Into<String>,
        family: ReferenceModelEvidenceFamily,
        description: impl Into<String>,
        source_uri: impl Into<String>,
        period: Option<String>,
        named_peer: Option<String>,
        authoritative: bool,
    ) -> Result<Self, TransformationDomainError> {
        Self::new_with_source_role(
            id,
            family,
            description,
            source_uri,
            period,
            named_peer,
            authoritative,
            None,
        )
    }

    /// Creates a source-bound claim with an explicit provenance role.
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_source_role(
        id: impl Into<String>,
        family: ReferenceModelEvidenceFamily,
        description: impl Into<String>,
        source_uri: impl Into<String>,
        period: Option<String>,
        named_peer: Option<String>,
        authoritative: bool,
        source_role: Option<ReferenceModelSourceRole>,
    ) -> Result<Self, TransformationDomainError> {
        let evidence = Self {
            id: id.into(),
            family,
            description: description.into(),
            source_uri: source_uri.into(),
            period,
            named_peer,
            authoritative,
            source_role,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    fn validate(&self) -> Result<(), TransformationDomainError> {
        for (field, value) in [
            ("reference-model evidence id", &self.id),
            ("reference-model evidence description", &self.description),
            ("reference-model evidence source URI", &self.source_uri),
        ] {
            if value.trim().is_empty() {
                return Err(TransformationDomainError::EmptyValue { field });
            }
        }
        for (field, value) in [
            ("reference-model evidence period", self.period.as_ref()),
            (
                "reference-model evidence named peer",
                self.named_peer.as_ref(),
            ),
        ] {
            if value.is_some_and(|value| value.trim().is_empty()) {
                return Err(TransformationDomainError::EmptyValue { field });
            }
        }
        Ok(())
    }

    /// Returns the stable evidence identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the required evidence family.
    pub const fn family(&self) -> ReferenceModelEvidenceFamily {
        self.family
    }

    /// Returns the bounded claim description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the source URI used for independence checks.
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Returns the effective period when the claim carries one.
    pub fn period(&self) -> Option<&str> {
        self.period.as_deref()
    }

    /// Returns the named peer or adopter when the claim carries one.
    pub fn named_peer(&self) -> Option<&str> {
        self.named_peer.as_deref()
    }

    /// Returns whether the source is allowed to satisfy a hard evidence gate.
    pub const fn authoritative(&self) -> bool {
        self.authoritative
    }

    /// Returns the optional provenance role used by the diffusion gate.
    pub const fn source_role(&self) -> Option<ReferenceModelSourceRole> {
        self.source_role
    }
}

/// Explicit supporting, counter, and missing proof for one reference-model claim.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReferenceModelEvidenceBundle {
    supporting: Vec<ReferenceModelEvidence>,
    counter: Vec<ReferenceModelEvidence>,
    missing: Vec<String>,
    counter_reviewed: bool,
}

impl ReferenceModelEvidenceBundle {
    /// Creates an empty evidence packet.
    pub const fn new() -> Self {
        Self {
            supporting: Vec::new(),
            counter: Vec::new(),
            missing: Vec::new(),
            counter_reviewed: false,
        }
    }

    /// Adds authoritative or non-authoritative supporting evidence once.
    pub fn add_supporting(
        &mut self,
        evidence: ReferenceModelEvidence,
    ) -> Result<(), TransformationDomainError> {
        self.add_unique(&evidence)?;
        self.supporting.push(evidence);
        Ok(())
    }

    /// Adds counter evidence once without treating it as missing proof.
    pub fn add_counter(
        &mut self,
        evidence: ReferenceModelEvidence,
    ) -> Result<(), TransformationDomainError> {
        self.add_unique(&evidence)?;
        self.counter.push(evidence);
        Ok(())
    }

    /// Adds a stable missing-proof requirement once.
    pub fn add_missing(
        &mut self,
        requirement: impl Into<String>,
    ) -> Result<(), TransformationDomainError> {
        let requirement = requirement.into();
        if requirement.trim().is_empty() {
            return Err(TransformationDomainError::EmptyValue {
                field: "reference-model missing proof",
            });
        }
        if self.missing.iter().any(|existing| existing == &requirement) {
            return Err(duplicate_identity(
                "reference-model missing proof",
                requirement,
            ));
        }
        self.missing.push(requirement);
        Ok(())
    }

    /// Records that a counter-evidence search was actually performed.
    pub const fn set_counter_reviewed(&mut self, reviewed: bool) {
        self.counter_reviewed = reviewed;
    }

    /// Returns supporting claims in insertion order.
    pub fn supporting(&self) -> &[ReferenceModelEvidence] {
        &self.supporting
    }

    /// Returns counter claims in insertion order.
    pub fn counter(&self) -> &[ReferenceModelEvidence] {
        &self.counter
    }

    /// Returns explicit missing-proof requirements in insertion order.
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// Returns whether the counter-evidence review was represented.
    pub const fn counter_reviewed(&self) -> bool {
        self.counter_reviewed
    }

    /// Evaluates the packet without scoring, ranking, or inferring causality.
    pub fn assess(&self) -> ReferenceModelAssessment {
        let authoritative = self
            .supporting
            .iter()
            .filter(|evidence| evidence.authoritative());
        let mut families = std::collections::BTreeSet::new();
        let mut outcome_periods = std::collections::BTreeSet::new();
        let mut diffusion_sources = std::collections::BTreeSet::new();
        let mut diffusion_peers = std::collections::BTreeSet::new();
        let mut supplier_attribution_sources = std::collections::BTreeSet::new();
        for evidence in authoritative {
            families.insert(evidence.family());
            if evidence.family() == ReferenceModelEvidenceFamily::SustainedOutcome {
                if let Some(period) = evidence.period() {
                    outcome_periods.insert(period.to_owned());
                }
            }
            if evidence.family() == ReferenceModelEvidenceFamily::IndustryDiffusion {
                match evidence.source_role() {
                    Some(ReferenceModelSourceRole::IndependentCustomerDisclosure) => {
                        diffusion_sources.insert(evidence.source_uri().to_owned());
                        if let Some(peer) = evidence.named_peer() {
                            diffusion_peers.insert(peer.to_owned());
                        }
                    }
                    Some(ReferenceModelSourceRole::SupplierAttribution) => {
                        supplier_attribution_sources.insert(evidence.source_uri().to_owned());
                    }
                    Some(ReferenceModelSourceRole::RegulatoryOrFiling)
                    | Some(ReferenceModelSourceRole::DiscoveryOnly)
                    | None => {}
                }
            }
        }

        let mut missing = self.missing.clone();
        for (family, requirement) in [
            (
                ReferenceModelEvidenceFamily::OrganizationRewrite,
                "organization_rewrite",
            ),
            (
                ReferenceModelEvidenceFamily::ProductionSystemRewrite,
                "production_system_rewrite",
            ),
            (
                ReferenceModelEvidenceFamily::SustainedOutcome,
                "sustained_outcome",
            ),
            (
                ReferenceModelEvidenceFamily::IndustryDiffusion,
                "industry_diffusion",
            ),
        ] {
            if !families.contains(&family) && !missing.iter().any(|item| item == requirement) {
                missing.push(requirement.to_owned());
            }
        }
        if families.contains(&ReferenceModelEvidenceFamily::SustainedOutcome)
            && outcome_periods.len() < 2
        {
            missing.push("distinct_outcome_periods".to_owned());
        }
        if families.contains(&ReferenceModelEvidenceFamily::IndustryDiffusion)
            && (diffusion_sources.len() < 2 || diffusion_peers.len() < 2)
        {
            missing.push("independent_diffusion_sources".to_owned());
        }
        if !self.counter_reviewed {
            missing.push("counter_evidence_review".to_owned());
        }
        missing.sort();
        missing.dedup();

        let core_rewrite = families.contains(&ReferenceModelEvidenceFamily::OrganizationRewrite)
            && families.contains(&ReferenceModelEvidenceFamily::ProductionSystemRewrite);
        let eligibility = if !core_rewrite {
            ReferenceModelEligibility::NotEligible
        } else if missing.is_empty() {
            ReferenceModelEligibility::Confirmed
        } else {
            ReferenceModelEligibility::Candidate
        };
        ReferenceModelAssessment {
            eligibility,
            supporting_families: families.into_iter().collect(),
            missing,
            counter_evidence_count: self.counter.len(),
            counter_reviewed: self.counter_reviewed,
            distinct_outcome_periods: outcome_periods.len(),
            independent_diffusion_sources: diffusion_sources.len(),
            supplier_attribution_sources: supplier_attribution_sources.len(),
        }
    }

    fn add_unique(
        &self,
        evidence: &ReferenceModelEvidence,
    ) -> Result<(), TransformationDomainError> {
        if self
            .supporting
            .iter()
            .chain(self.counter.iter())
            .any(|existing| existing.id() == evidence.id())
        {
            return Err(duplicate_identity(
                "reference-model evidence",
                evidence.id(),
            ));
        }
        Ok(())
    }
}

/// The immutable read model produced by the reference-model gate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReferenceModelAssessment {
    eligibility: ReferenceModelEligibility,
    supporting_families: Vec<ReferenceModelEvidenceFamily>,
    missing: Vec<String>,
    counter_evidence_count: usize,
    #[serde(default)]
    counter_reviewed: bool,
    distinct_outcome_periods: usize,
    independent_diffusion_sources: usize,
    #[serde(default)]
    supplier_attribution_sources: usize,
}

impl Default for ReferenceModelAssessment {
    fn default() -> Self {
        ReferenceModelEvidenceBundle::new().assess()
    }
}

impl ReferenceModelAssessment {
    /// Returns the fail-closed eligibility result.
    pub const fn eligibility(&self) -> ReferenceModelEligibility {
        self.eligibility
    }

    /// Returns the authoritative families present in the packet.
    pub fn supporting_families(&self) -> &[ReferenceModelEvidenceFamily] {
        &self.supporting_families
    }

    /// Returns missing hard conditions in stable order.
    pub fn missing(&self) -> &[String] {
        &self.missing
    }

    /// Returns the number of counter claims retained.
    pub const fn counter_evidence_count(&self) -> usize {
        self.counter_evidence_count
    }

    /// Returns whether the bounded counter-evidence review was represented.
    pub const fn counter_reviewed(&self) -> bool {
        self.counter_reviewed
    }

    /// Returns the number of distinct outcome periods.
    pub const fn distinct_outcome_periods(&self) -> usize {
        self.distinct_outcome_periods
    }

    /// Returns the number of independent diffusion source URIs.
    pub const fn independent_diffusion_sources(&self) -> usize {
        self.independent_diffusion_sources
    }

    /// Returns the number of authoritative supplier-attribution source URIs.
    pub const fn supplier_attribution_sources(&self) -> usize {
        self.supplier_attribution_sources
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
