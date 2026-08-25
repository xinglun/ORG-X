//! Evidence-first automatic machine reference and independent human reference.
//!
//! This module is the runtime ACL between provider-neutral facts and the
//! Transformation/Ranking contexts. The machine view is deterministic and
//! conservative; the human view is retained separately and never participates
//! in machine Stage or Ranking calculation.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::features::evidence::domain::{
    Claim, CompanyReference as EvidenceCompanyReference, Confidence as EvidenceConfidence,
    ContentHash, EffectiveDate, EvidenceId, EvidencePolarity, EvidenceRecord, EvidenceSet,
    EvidenceType, ExtractorVersion, Freshness, MissingEvidence, MissingEvidenceId, MissingReason,
    NormalizedValue, ObservationTime, SourceTitle, SourceType, SourceUri,
};
use crate::features::ranking::domain::{
    CompanyReference as RankingCompanyReference, CounterEvidenceRisk,
    EvidenceConfidence as RankingEvidenceConfidence, EvidenceFreshness, RankingCandidate,
    RankingCandidateId, RankingReadModel, Stage as RankingStage, TransformationScore,
};
use crate::features::transformation::domain::Stage as TransformationStage;
use crate::features::transformation::domain::{
    ReferenceModelAssessment, ReferenceModelEligibility, ReferenceModelEvidence,
    ReferenceModelEvidenceBundle, ReferenceModelEvidenceFamily,
};

use super::error::RuntimeError;
use super::model::{Confidence, FactStatus, NormalizedFact};

/// Version of the deterministic machine-reference rules.
pub const MACHINE_RULE_VERSION: &str = "weekly-radar-machine-reference/v1";

const SUPPORTING_PREFIX: &str = "judgment.supporting.";
const COUNTER_PREFIX: &str = "judgment.counter.";
const COUNTER_REVIEW_KIND: &str = "judgment.review.REFERENCE_MODEL.counter_evidence_review";
const MISSING_PREFIX: &str = "judgment.missing.";

/// Machine Stage result. `Undetermined` is a valid result and is not treated
/// as a Stage or a Ranking input.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum MachineStage {
    /// The machine rule selected one of the six documented Stage labels.
    Assigned(String),
    /// Evidence gates were not sufficient for a machine Stage reference.
    Undetermined { reason: String },
}

impl MachineStage {
    /// Creates an assigned machine Stage label.
    pub fn assigned(value: impl Into<String>) -> Self {
        Self::Assigned(value.into())
    }

    /// Returns the assigned label, or `UNDETERMINED` for the gated result.
    pub fn label(&self) -> &str {
        match self {
            Self::Assigned(value) => value,
            Self::Undetermined { .. } => "UNDETERMINED",
        }
    }

    /// Returns the reason when the machine result is undetermined.
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Assigned(_) => None,
            Self::Undetermined { reason } => Some(reason),
        }
    }
}

/// A human-authored reference view retained independently from the machine.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HumanReference {
    company_id: String,
    stage: String,
    note: String,
    recorded_at: String,
}

impl HumanReference {
    /// Creates a human reference without making it an input to machine logic.
    pub fn new(
        company_id: impl Into<String>,
        stage: impl Into<String>,
        note: impl Into<String>,
        recorded_at: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let reference = Self {
            company_id: company_id.into(),
            stage: stage.into(),
            note: note.into(),
            recorded_at: recorded_at.into(),
        };
        reference.validate()?;
        Ok(reference)
    }

    fn validate(&self) -> Result<(), RuntimeError> {
        for (field, value) in [
            ("human reference company ID", &self.company_id),
            ("human reference Stage", &self.stage),
            ("human reference note", &self.note),
            ("human reference recorded_at", &self.recorded_at),
        ] {
            if value.trim().is_empty() {
                return Err(RuntimeError::invalid_model(format!(
                    "{field} cannot be blank"
                )));
            }
        }
        DateTime::parse_from_rfc3339(&self.recorded_at).map_err(|_| {
            RuntimeError::invalid_model("human reference recorded_at must be RFC3339")
        })?;
        Ok(())
    }

    /// Returns the referenced company.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the human-supplied Stage reference exactly as recorded.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// Returns the human note exactly as recorded.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Returns the human reference timestamp.
    pub fn recorded_at(&self) -> &str {
        &self.recorded_at
    }
}

/// A provenance-preserving proof view exposed in the runtime snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProofView {
    id: String,
    description: String,
    source_uri: Option<String>,
    fact_kind: Option<String>,
}

impl ProofView {
    /// Returns the stable proof identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the proof description.
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Returns the linked source URI when the proof comes from a fact.
    pub fn source_uri(&self) -> Option<&str> {
        self.source_uri.as_deref()
    }

    /// Returns the linked normalized fact kind when available.
    pub fn fact_kind(&self) -> Option<&str> {
        self.fact_kind.as_deref()
    }

    fn validate(&self, requires_source: bool) -> Result<(), RuntimeError> {
        if self.id.trim().is_empty() || self.description.trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "judgment proof ID and description cannot be blank",
            ));
        }
        if requires_source {
            if self.source_uri.as_deref().is_none_or(str::is_empty)
                || self.fact_kind.as_deref().is_none_or(str::is_empty)
            {
                return Err(RuntimeError::invalid_model(
                    "supporting and counter proof require source and fact kind",
                ));
            }
        } else if self.source_uri.is_some() || self.fact_kind.is_some() {
            return Err(RuntimeError::invalid_model(
                "missing proof cannot carry source or fact kind",
            ));
        }
        Ok(())
    }
}

/// A serializable ranking candidate view produced by the Ranking boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedCandidateView {
    id: String,
    company: String,
    stage: String,
    evidence_confidence: u8,
    transformation_score: u8,
    counter_evidence_risk: u8,
    evidence_freshness: u8,
}

impl RankedCandidateView {
    /// Returns the candidate identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the candidate company.
    pub fn company(&self) -> &str {
        &self.company
    }

    /// Returns the explicit Stage used to isolate ranking.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// Returns the retained Evidence Confidence value.
    pub const fn evidence_confidence(&self) -> u8 {
        self.evidence_confidence
    }

    /// Returns the retained Transformation Score value.
    pub const fn transformation_score(&self) -> u8 {
        self.transformation_score
    }

    /// Returns the retained Counter Evidence Risk value.
    pub const fn counter_evidence_risk(&self) -> u8 {
        self.counter_evidence_risk
    }

    /// Returns the retained Evidence Freshness value.
    pub const fn evidence_freshness(&self) -> u8 {
        self.evidence_freshness
    }
}

/// One company's machine reference and its independent proof inventory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct MachineJudgment {
    company_id: String,
    machine_stage: MachineStage,
    stage_reason: String,
    evidence_cutoff: NaiveDate,
    #[serde(default)]
    reference_model_assessment: ReferenceModelAssessment,
    supporting_proof: Vec<ProofView>,
    counter_proof: Vec<ProofView>,
    missing_proof: Vec<ProofView>,
    ranked_candidates: Vec<RankedCandidateView>,
}

impl MachineJudgment {
    /// Returns the company identity.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the machine Stage or `UNDETERMINED` result.
    pub fn machine_stage(&self) -> &MachineStage {
        &self.machine_stage
    }

    /// Returns the stable Stage decision reason.
    pub fn stage_reason(&self) -> &str {
        &self.stage_reason
    }

    /// Returns the evidence cutoff used by the machine reference.
    pub const fn evidence_cutoff(&self) -> NaiveDate {
        self.evidence_cutoff
    }

    /// Returns the precomputed reference-model evidence assessment.
    pub const fn reference_model_assessment(&self) -> &ReferenceModelAssessment {
        &self.reference_model_assessment
    }

    /// Returns supporting proof with source links retained.
    pub fn supporting_proof(&self) -> &[ProofView] {
        &self.supporting_proof
    }

    /// Returns counter proof with source links retained.
    pub fn counter_proof(&self) -> &[ProofView] {
        &self.counter_proof
    }

    /// Returns explicit missing-proof requirements.
    pub fn missing_proof(&self) -> &[ProofView] {
        &self.missing_proof
    }

    /// Returns same-Stage ranked candidates only.
    pub fn ranked_candidates(&self) -> &[RankedCandidateView] {
        &self.ranked_candidates
    }
}

/// Complete judgment output retained in the immutable runtime input snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JudgmentSnapshot {
    evidence_cutoff: NaiveDate,
    machine_rule_version: String,
    companies: Vec<MachineJudgment>,
    human_references: Vec<HumanReference>,
}

impl JudgmentSnapshot {
    /// Returns the evidence cutoff.
    pub const fn evidence_cutoff(&self) -> NaiveDate {
        self.evidence_cutoff
    }

    /// Returns the machine rule version.
    pub fn machine_rule_version(&self) -> &str {
        &self.machine_rule_version
    }

    /// Returns all company machine judgments in stable company order.
    pub fn companies(&self) -> &[MachineJudgment] {
        &self.companies
    }

    /// Looks up one company machine judgment.
    pub fn company(&self, company_id: &str) -> Option<&MachineJudgment> {
        self.companies
            .iter()
            .find(|judgment| judgment.company_id() == company_id)
    }

    /// Returns a human reference for one company, if one was supplied.
    pub fn human_reference(&self, company_id: &str) -> Option<&HumanReference> {
        self.human_references
            .iter()
            .find(|reference| reference.company_id() == company_id)
    }

    /// Returns all human references without mixing them into machine output.
    pub fn human_references(&self) -> &[HumanReference] {
        &self.human_references
    }

    /// Returns machine candidates ranked only inside the requested Stage.
    pub fn ranked_within_stage(&self, stage: &str) -> Vec<&RankedCandidateView> {
        let mut seen = BTreeSet::new();
        self.companies
            .iter()
            .flat_map(|judgment| judgment.ranked_candidates())
            .filter(|candidate| {
                candidate.stage() == stage && seen.insert(candidate.id().to_owned())
            })
            .collect()
    }

    /// Validates persisted judgment data before it is rendered or published.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.machine_rule_version.trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "machine rule version cannot be blank",
            ));
        }
        let mut company_ids = BTreeSet::new();
        for judgment in &self.companies {
            if !company_ids.insert(judgment.company_id.as_str()) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate machine judgment company {}",
                    judgment.company_id
                )));
            }
            if judgment.company_id.trim().is_empty() {
                return Err(RuntimeError::invalid_model(
                    "machine judgment company ID cannot be blank",
                ));
            }
            if judgment.evidence_cutoff != self.evidence_cutoff {
                return Err(RuntimeError::invalid_model(format!(
                    "machine judgment cutoff does not match snapshot cutoff for {}",
                    judgment.company_id
                )));
            }
            if judgment.stage_reason.trim().is_empty() {
                return Err(RuntimeError::invalid_model(format!(
                    "machine judgment reason cannot be blank for {}",
                    judgment.company_id
                )));
            }
            let expected_stage = match &judgment.machine_stage {
                MachineStage::Assigned(stage) => {
                    if transformation_stage(stage).is_none() {
                        return Err(RuntimeError::invalid_model(format!(
                            "unknown machine Stage {stage} for {}",
                            judgment.company_id
                        )));
                    }
                    if judgment.ranked_candidates.is_empty() {
                        return Err(RuntimeError::invalid_model(format!(
                            "assigned machine Stage has no Ranking candidates for {}",
                            judgment.company_id
                        )));
                    }
                    Some(stage.as_str())
                }
                MachineStage::Undetermined { .. } => {
                    if !judgment.ranked_candidates.is_empty() {
                        return Err(RuntimeError::invalid_model(format!(
                            "undetermined machine Stage cannot have Ranking candidates for {}",
                            judgment.company_id
                        )));
                    }
                    None
                }
            };

            let mut proof_ids = BTreeSet::new();
            for proof in &judgment.supporting_proof {
                proof.validate(true)?;
                if !proof_ids.insert(proof.id.as_str()) {
                    return Err(RuntimeError::invalid_model(format!(
                        "duplicate judgment proof {}",
                        proof.id
                    )));
                }
            }
            for proof in &judgment.counter_proof {
                proof.validate(true)?;
                if !proof_ids.insert(proof.id.as_str()) {
                    return Err(RuntimeError::invalid_model(format!(
                        "duplicate judgment proof {}",
                        proof.id
                    )));
                }
            }
            for proof in &judgment.missing_proof {
                proof.validate(false)?;
                if !proof_ids.insert(proof.id.as_str()) {
                    return Err(RuntimeError::invalid_model(format!(
                        "duplicate judgment proof {}",
                        proof.id
                    )));
                }
            }

            let mut candidate_ids = BTreeSet::new();
            for candidate in &judgment.ranked_candidates {
                if candidate.id.trim().is_empty()
                    || candidate.company.trim().is_empty()
                    || candidate.stage.trim().is_empty()
                    || transformation_stage(&candidate.stage).is_none()
                    || candidate.evidence_confidence > 100
                    || candidate.transformation_score > 100
                    || candidate.counter_evidence_risk > 100
                    || candidate.evidence_freshness > 100
                {
                    return Err(RuntimeError::invalid_model(format!(
                        "invalid Ranking candidate {}",
                        candidate.id
                    )));
                }
                if expected_stage.is_none_or(|stage| candidate.stage != stage) {
                    return Err(RuntimeError::invalid_model(format!(
                        "Ranking candidate Stage does not match machine Stage for {}",
                        judgment.company_id
                    )));
                }
                if !candidate_ids.insert(candidate.id.as_str()) {
                    return Err(RuntimeError::invalid_model(format!(
                        "duplicate Ranking candidate {}",
                        candidate.id
                    )));
                }
            }
        }
        let mut reference_ids = BTreeSet::new();
        for reference in &self.human_references {
            reference.validate()?;
            if !company_ids.contains(reference.company_id.as_str()) {
                return Err(RuntimeError::invalid_model(format!(
                    "human reference company {} has no machine judgment",
                    reference.company_id
                )));
            }
            if !reference_ids.insert(reference.company_id.as_str()) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate human reference company {}",
                    reference.company_id
                )));
            }
        }
        Ok(())
    }
}

/// Derives a deterministic machine reference from the companies represented by
/// the supplied facts and keeps human references in a separate,
/// non-authoritative lane.
pub fn derive_judgment_snapshot(
    evidence_cutoff: NaiveDate,
    facts: &[NormalizedFact],
    human_references: Vec<HumanReference>,
) -> Result<JudgmentSnapshot, RuntimeError> {
    let company_ids = facts
        .iter()
        .map(|fact| fact.company_id())
        .collect::<BTreeSet<_>>();
    derive_judgment_snapshot_for_companies(evidence_cutoff, company_ids, facts, human_references)
}

/// Derives a deterministic machine reference for an explicit company set.
///
/// Explicit company identities are required so a company with no collected
/// facts remains visible as `UNDETERMINED`. Raw provider observations such as
/// `source_*` are deliberately not treated as Stage signals; only explicit
/// `judgment.*` facts can satisfy the Evidence-first gate.
pub fn derive_judgment_snapshot_for_companies<I, S>(
    evidence_cutoff: NaiveDate,
    company_ids: I,
    facts: &[NormalizedFact],
    human_references: Vec<HumanReference>,
) -> Result<JudgmentSnapshot, RuntimeError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut company_ids = company_ids
        .into_iter()
        .map(|company_id| company_id.as_ref().to_owned())
        .collect::<BTreeSet<_>>();
    let mut grouped = BTreeMap::<String, Vec<&NormalizedFact>>::new();
    for fact in facts {
        fact.validate()?;
        if fact.company_id().trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "judgment company ID cannot be blank",
            ));
        }
        company_ids.insert(fact.company_id().to_owned());
        grouped
            .entry(fact.company_id().to_owned())
            .or_default()
            .push(fact);
    }

    let mut judgments = Vec::new();
    let mut ranking_model = RankingReadModel::new();
    let mut assigned_stages = BTreeMap::<String, TransformationStage>::new();

    for company_id in company_ids {
        let company_facts = grouped.remove(&company_id).unwrap_or_default();
        let eligible_facts = company_facts
            .iter()
            .copied()
            .filter(|fact| fact_is_on_or_before_cutoff(fact, evidence_cutoff))
            .collect::<Vec<_>>();
        let evidence = build_evidence_set(&company_id, &eligible_facts, evidence_cutoff)?;
        let reference_model_assessment = derive_reference_model_assessment(&eligible_facts);
        let (machine_stage, stage_reason, selected_stage) = evaluate_stage(
            &eligible_facts,
            evidence_cutoff,
            reference_model_assessment.eligibility() == ReferenceModelEligibility::Confirmed,
        );
        let supporting_proof = proof_views_for_evidence(evidence.supporting());
        let counter_proof = proof_views_for_evidence(evidence.counter());
        let missing_proof = missing_proof_views(evidence.missing());

        if let Some(stage) = selected_stage.clone() {
            let candidate = build_ranking_candidate(
                &company_id,
                &stage,
                &eligible_facts,
                supporting_proof.len(),
                counter_proof.len(),
                evidence_cutoff,
            )?;
            ranking_model
                .add(candidate)
                .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
            assigned_stages.insert(company_id.clone(), stage);
        }

        judgments.push(MachineJudgment {
            company_id,
            machine_stage,
            stage_reason,
            evidence_cutoff,
            reference_model_assessment,
            supporting_proof,
            counter_proof,
            missing_proof,
            ranked_candidates: Vec::new(),
        });
    }

    judgments.sort_by(|left, right| left.company_id.cmp(&right.company_id));
    for judgment in &mut judgments {
        let Some(stage) = assigned_stages.get(&judgment.company_id) else {
            continue;
        };
        let ranking_stage = ranking_stage(stage);
        judgment.ranked_candidates = ranking_model
            .ranked_within_stage(ranking_stage)
            .into_iter()
            .map(ranked_candidate_view)
            .collect();
    }

    let snapshot = JudgmentSnapshot {
        evidence_cutoff,
        machine_rule_version: MACHINE_RULE_VERSION.to_owned(),
        companies: judgments,
        human_references,
    };
    snapshot.validate()?;
    Ok(snapshot)
}

fn derive_reference_model_assessment(facts: &[&NormalizedFact]) -> ReferenceModelAssessment {
    let mut bundle = ReferenceModelEvidenceBundle::new();
    let mut counter_reviewed = false;
    for (index, fact) in facts.iter().enumerate() {
        if fact.status() != &FactStatus::Known {
            continue;
        }
        let is_counter = fact.kind().starts_with(COUNTER_PREFIX);
        if is_counter || fact.kind() == COUNTER_REVIEW_KIND {
            counter_reviewed = true;
        }
        let Some(family) = fact.reference_model_family().or_else(|| {
            is_counter.then_some(ReferenceModelEvidenceFamily::ProductionSystemRewrite)
        }) else {
            continue;
        };
        let description = fact
            .value()
            .unwrap_or_else(|| fact.provenance().source_field_or_passage())
            .to_owned();
        let periods = if fact.reference_model_periods().is_empty() {
            vec![fact.provenance().effective_date().map(ToString::to_string)]
        } else {
            fact.reference_model_periods()
                .iter()
                .cloned()
                .map(Some)
                .collect()
        };
        for (period_index, period) in periods.into_iter().enumerate() {
            let id = format!("reference-model:{index}:{period_index}:{}", fact.kind());
            let evidence = match ReferenceModelEvidence::new_with_source_role(
                id,
                family,
                description.clone(),
                fact.provenance().source_uri(),
                period,
                fact.reference_model_named_peer().map(str::to_owned),
                true,
                fact.reference_model_source_role(),
            ) {
                Ok(evidence) => evidence,
                Err(_) => continue,
            };
            if is_counter {
                let _ = bundle.add_counter(evidence);
            } else {
                let _ = bundle.add_supporting(evidence);
            }
        }
    }
    bundle.set_counter_reviewed(counter_reviewed);
    bundle.assess()
}

fn build_evidence_set(
    company_id: &str,
    facts: &[&NormalizedFact],
    cutoff: NaiveDate,
) -> Result<EvidenceSet, RuntimeError> {
    let company = EvidenceCompanyReference::new(company_id)
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let mut set = EvidenceSet::new(company);
    for fact in facts {
        let Some(signal) = parse_signal(fact.kind()) else {
            continue;
        };
        match signal.polarity {
            SignalPolarity::Supporting | SignalPolarity::Counter => {
                if fact.status() != &FactStatus::Known {
                    continue;
                }
                let record = evidence_record_for_fact(fact, signal.polarity, cutoff)?;
                set.add(record)
                    .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
            }
            SignalPolarity::Missing => {
                let id = MissingEvidenceId::new(format!("missing:{}:{}", company_id, fact.kind()))
                    .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
                let requirement = Claim::new(signal.category)
                    .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
                let reason = match fact.status() {
                    FactStatus::Unavailable => MissingReason::Unavailable,
                    FactStatus::Unknown | FactStatus::Unconfirmed => MissingReason::Unknown,
                    FactStatus::Known => MissingReason::NotCollected,
                };
                let missing = MissingEvidence::new(id, requirement, reason)
                    .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
                set.add_missing(missing)
                    .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
            }
        }
    }
    Ok(set)
}

fn evaluate_stage(
    facts: &[&NormalizedFact],
    cutoff: NaiveDate,
    reference_model_confirmed: bool,
) -> (MachineStage, String, Option<TransformationStage>) {
    if reference_model_confirmed {
        return (
            MachineStage::assigned(TransformationStage::ReferenceModel.label()),
            "reference-model evidence bundle confirmed; highest Stage gate passed".to_owned(),
            Some(TransformationStage::ReferenceModel),
        );
    }
    let mut by_stage =
        BTreeMap::<TransformationStage, (usize, usize, usize, BTreeSet<String>)>::new();
    for fact in facts {
        if !fact_is_on_or_before_cutoff(fact, cutoff) {
            continue;
        }
        let Some(signal) = parse_signal(fact.kind()) else {
            continue;
        };
        let Some(stage) = transformation_stage(signal.stage) else {
            continue;
        };
        let entry = by_stage
            .entry(stage)
            .or_insert_with(|| (0, 0, 0, BTreeSet::new()));
        match signal.polarity {
            SignalPolarity::Supporting if fact.status() == &FactStatus::Known => {
                entry.0 += 1;
                entry.3.insert(fact.provenance().source_uri().to_owned());
            }
            SignalPolarity::Counter if fact.status() == &FactStatus::Known => entry.1 += 1,
            SignalPolarity::Missing => entry.2 += 1,
            _ => {}
        }
    }

    let selected = by_stage
        .into_iter()
        .filter(|(stage, (support, counter, missing, sources))| {
            *support >= 2
                && *counter >= 1
                && *missing >= 1
                && sources.len() >= 2
                && stage_is_not_reference_model_or_confirmed(stage, reference_model_confirmed)
        })
        .max_by_key(|(stage, _)| stage.rank());

    match selected {
        Some((stage, (support, counter, missing, sources))) => (
            MachineStage::assigned(stage.label()),
            format!(
                "automatic gate passed for {}: supporting={}, counter={}, missing={}, independent_sources={}",
                stage.label(),
                support,
                counter,
                missing,
                sources.len()
            ),
            Some(stage),
        ),
        None => (
            MachineStage::Undetermined {
                reason: "automatic gate requires two independent supporting sources, one counter signal, and one missing-proof inventory".to_owned(),
            },
            "automatic gate not satisfied; no machine Ranking emitted".to_owned(),
            None,
        ),
    }
}

fn stage_is_not_reference_model_or_confirmed(
    stage: &TransformationStage,
    reference_model_confirmed: bool,
) -> bool {
    *stage != TransformationStage::ReferenceModel || reference_model_confirmed
}

fn build_ranking_candidate(
    company_id: &str,
    stage: &TransformationStage,
    facts: &[&NormalizedFact],
    supporting_count: usize,
    counter_count: usize,
    cutoff: NaiveDate,
) -> Result<RankingCandidate, RuntimeError> {
    let evidence_confidence = (supporting_count.saturating_mul(25) + 25).min(100) as u8;
    let transformation_score = (stage.rank().saturating_mul(20) as usize
        + supporting_count.saturating_mul(10))
    .min(100) as u8;
    let counter_evidence_risk = counter_count.saturating_mul(25).min(100) as u8;
    let evidence_freshness = freshness_score(facts, cutoff);

    RankingCandidate::new(
        RankingCandidateId::new(format!("machine:{company_id}"))
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
        RankingCompanyReference::new(company_id)
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
        ranking_stage(stage),
        RankingEvidenceConfidence::new(evidence_confidence)
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
        TransformationScore::new(transformation_score)
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
        CounterEvidenceRisk::new(counter_evidence_risk)
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
        EvidenceFreshness::new(evidence_freshness)
            .map_err(|error| RuntimeError::invalid_model(error.to_string()))?,
    )
    .map_err(|error| RuntimeError::invalid_model(error.to_string()))
}

fn freshness_score(facts: &[&NormalizedFact], cutoff: NaiveDate) -> u8 {
    let latest = facts
        .iter()
        .filter_map(|fact| fact.provenance().effective_date().copied())
        .max();
    let Some(latest) = latest else {
        return 50;
    };
    let age = cutoff.signed_duration_since(latest).num_days();
    if age <= 30 {
        100
    } else if age <= 90 {
        80
    } else if age <= 365 {
        60
    } else {
        30
    }
}

fn fact_is_on_or_before_cutoff(fact: &NormalizedFact, cutoff: NaiveDate) -> bool {
    fact.provenance()
        .effective_date()
        .is_none_or(|effective_date| *effective_date <= cutoff)
}

fn evidence_record_for_fact(
    fact: &NormalizedFact,
    polarity: SignalPolarity,
    _cutoff: NaiveDate,
) -> Result<EvidenceRecord, RuntimeError> {
    let evidence_id = EvidenceId::new(format!("fact:{}:{}", fact.company_id(), fact.kind()))
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let company = EvidenceCompanyReference::new(fact.company_id())
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let observed_at = ObservationTime::new(fact.provenance().retrieved_at().to_rfc3339())
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let effective_date = fact
        .provenance()
        .effective_date()
        .map(|date| EffectiveDate::new(date.to_string()))
        .transpose()
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let source_uri = SourceUri::new(fact.provenance().source_uri())
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let source_title = SourceTitle::new(fact.provenance().source_uri())
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let claim = Claim::new(fact.value().unwrap_or(fact.kind()))
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let normalized_value = fact
        .value()
        .map(NormalizedValue::new)
        .transpose()
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let extractor_version = ExtractorVersion::new(MACHINE_RULE_VERSION)
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let content_hash = ContentHash::new(content_hash(fact))
        .map_err(|error| RuntimeError::invalid_model(error.to_string()))?;
    let source_type = if fact.provenance().source_uri().starts_with("https://") {
        SourceType::OfficialMaterial
    } else {
        SourceType::Other("runtime_fixture".to_owned())
    };
    let evidence_confidence = match fact.confidence() {
        Confidence::High => EvidenceConfidence::High,
        Confidence::Medium => EvidenceConfidence::Medium,
        Confidence::Low => EvidenceConfidence::Low,
        Confidence::Approximate => EvidenceConfidence::Low,
        Confidence::Unknown => EvidenceConfidence::Unknown,
    };
    let freshness = if fact.provenance().effective_date().is_some() {
        Freshness::Current
    } else {
        Freshness::Unknown
    };
    let evidence_polarity = match polarity {
        SignalPolarity::Supporting => EvidencePolarity::Supporting,
        SignalPolarity::Counter => EvidencePolarity::Counter,
        SignalPolarity::Missing => {
            return Err(RuntimeError::invalid_model(
                "missing signal cannot become an EvidenceRecord",
            ))
        }
    };
    EvidenceRecord::new(
        evidence_id,
        company,
        observed_at,
        effective_date,
        EvidenceType::Transformation,
        source_type,
        source_uri,
        source_title,
        claim,
        normalized_value,
        evidence_polarity,
        evidence_confidence,
        freshness,
        extractor_version,
        content_hash,
    )
    .map_err(|error| RuntimeError::invalid_model(error.to_string()))
}

fn proof_views_for_evidence(records: &[EvidenceRecord]) -> Vec<ProofView> {
    records
        .iter()
        .map(|record| ProofView {
            id: record.id().as_str().to_owned(),
            description: record.claim().as_str().to_owned(),
            source_uri: Some(record.source_uri().as_str().to_owned()),
            fact_kind: Some(record.claim().as_str().to_owned()),
        })
        .collect()
}

fn missing_proof_views(records: &[MissingEvidence]) -> Vec<ProofView> {
    records
        .iter()
        .map(|record| ProofView {
            id: record.id().as_str().to_owned(),
            description: record.requirement().as_str().to_owned(),
            source_uri: None,
            fact_kind: None,
        })
        .collect()
}

fn ranked_candidate_view(candidate: &RankingCandidate) -> RankedCandidateView {
    RankedCandidateView {
        id: candidate.id().as_str().to_owned(),
        company: candidate.company().as_str().to_owned(),
        stage: candidate.stage().label().to_owned(),
        evidence_confidence: candidate.evidence_confidence().as_u8(),
        transformation_score: candidate.transformation_score().as_u8(),
        counter_evidence_risk: candidate.counter_evidence_risk().as_u8(),
        evidence_freshness: candidate.evidence_freshness().as_u8(),
    }
}

#[derive(Clone, Copy)]
enum SignalPolarity {
    Supporting,
    Counter,
    Missing,
}

struct ParsedSignal<'a> {
    polarity: SignalPolarity,
    stage: &'a str,
    category: &'a str,
}

fn parse_signal(kind: &str) -> Option<ParsedSignal<'_>> {
    let (prefix, polarity) = if let Some(rest) = kind.strip_prefix(SUPPORTING_PREFIX) {
        (rest, SignalPolarity::Supporting)
    } else if let Some(rest) = kind.strip_prefix(COUNTER_PREFIX) {
        (rest, SignalPolarity::Counter)
    } else {
        let rest = kind.strip_prefix(MISSING_PREFIX)?;
        (rest, SignalPolarity::Missing)
    };
    let mut parts = prefix.splitn(2, '.');
    let stage = parts.next()?.trim();
    let category = parts.next()?.trim();
    if stage.is_empty() || category.is_empty() {
        return None;
    }
    Some(ParsedSignal {
        polarity,
        stage,
        category,
    })
}

fn transformation_stage(label: &str) -> Option<TransformationStage> {
    match label {
        "TOOL" => Some(TransformationStage::Tool),
        "SUBSTITUTION" => Some(TransformationStage::Substitution),
        "WORKFLOW" => Some(TransformationStage::Workflow),
        "PRODUCTION_SYSTEM" => Some(TransformationStage::ProductionSystem),
        "PRODUCTIVITY_BREAKOUT" => Some(TransformationStage::ProductivityBreakout),
        "REFERENCE_MODEL" => Some(TransformationStage::ReferenceModel),
        _ => None,
    }
}

fn ranking_stage(stage: &TransformationStage) -> RankingStage {
    match stage {
        TransformationStage::Tool => RankingStage::Tool,
        TransformationStage::Substitution => RankingStage::Substitution,
        TransformationStage::Workflow => RankingStage::Workflow,
        TransformationStage::ProductionSystem => RankingStage::ProductionSystem,
        TransformationStage::ProductivityBreakout => RankingStage::ProductivityBreakout,
        TransformationStage::ReferenceModel => RankingStage::ReferenceModel,
    }
}

fn content_hash(fact: &NormalizedFact) -> String {
    let mut hash = 2_166_136_261_u64;
    for byte in format!(
        "{}:{}:{}:{}",
        fact.company_id(),
        fact.kind(),
        fact.value().unwrap_or(""),
        fact.provenance().source_uri()
    )
    .bytes()
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    format!("fnv1a64:{hash:016x}")
}
