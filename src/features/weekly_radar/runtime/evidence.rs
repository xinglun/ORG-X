//! Deterministic claim extraction and validation for Weekly Radar evidence.

use chrono::{NaiveDate, Utc};
use regex::Regex;
use std::fmt;

use super::discovery::DocumentKind;
use super::model::{Confidence, FactStatus, NormalizedFact, Provenance, StructuralDimension};
use super::sources::{SourceKind, SourceMaterialKind, SourceObservation, SourceStatus, SourceTier};
use super::RuntimeError;
use crate::features::transformation::domain::{
    ReferenceModelEvidenceFamily, ReferenceModelSourceRole,
};

const EXTRACTOR_VERSION: &str = "weekly-radar-evidence-v4";
const MAX_FIELD_BYTES: usize = 512;
const MIN_CLAIM_WORDS: usize = 8;

const CHANGE_SIGNALS: &[&str] = &[
    "announc",
    "reorganiz",
    "restructur",
    "consolidat",
    "appoint",
    "moved",
    "shift",
    "launch",
    "adopt",
    "roll",
    "integrat",
    "automat",
    "deploy",
    "replaced",
    "moderniz",
    "doubled",
    "reduced",
    "increased",
];

const CAREERS_EXPLICIT_HIRING_PATTERNS: &[&str] = &[
    "we are hiring",
    "we're hiring",
    "we will hire",
    "we plan to hire",
    "we are recruiting",
    "we're recruiting",
    "we will recruit",
    "we plan to recruit",
    "increase headcount",
    "increased headcount",
    "grow our workforce",
    "growing our workforce",
    "expand our workforce",
    "expanding our workforce",
    "add positions",
    "adding positions",
    "new positions",
    "additional positions",
];

const PRODUCTION_SIGNALS: &[&str] = &[
    "engineering",
    "platform",
    "product",
    "operations",
    "workflow",
    "automation",
    "ai",
    "copilot",
    "agent",
    "data",
    "cloud",
    "research",
    "development",
    "scheduler",
    "model",
    "storage",
    "infrastructure",
];

const ORGANIZATION_DIMENSION_SIGNALS: &[&str] = &[
    "organization",
    "organiz",
    "responsibil",
    "reporting",
    "team",
    "division",
    "headcount",
];

const WORKFLOW_DIMENSION_SIGNALS: &[&str] = &[
    "workflow",
    "process",
    "operating model",
    "operation",
    "approval",
    "scheduling",
    "handoff",
];

const PRODUCTION_SYSTEM_DIMENSION_SIGNALS: &[&str] = &[
    "production system",
    "production platform",
    "deploy",
    "rollout",
    "automation",
    "agent",
    "platform",
    "infrastructure",
    "storage",
    "cloud",
    "scheduler",
    "pipeline",
];

const OPERATING_METRIC_DIMENSION_SIGNALS: &[&str] = &[
    "gpu",
    "utilization",
    "latency",
    "throughput",
    "capacity",
    "cost",
    "margin",
    "cash flow",
    "revenue",
    "free cash flow",
    "productivity",
];

const REFERENCE_MODEL_ORGANIZATION_SIGNALS: &[&str] = &[
    "reorganiz",
    "restructur",
    "reporting line",
    "responsibil",
    "decision right",
    "operating model",
    "organization",
];

const REFERENCE_MODEL_PRODUCTION_SIGNALS: &[&str] = &[
    "production system",
    "ai platform",
    "agentic workflow",
    "agent workflow",
    "automation",
    "engineering workflow",
    "operating model",
    "deployment platform",
];

const REFERENCE_MODEL_OUTCOME_SIGNALS: &[&str] = &[
    "revenue",
    "operating income",
    "operating margin",
    "free cash flow",
    "cash flow",
    "productivity",
    "cost per",
];

const REFERENCE_MODEL_DIFFUSION_SIGNALS: &[&str] = &[
    "adopted by",
    "adopted ",
    "deploys ",
    "rolled out",
    "rolling out",
    "rolls out",
    "implemented by",
    "implemented ",
    "used by",
    " used ",
    "industry standard",
    "peer company",
    "customer adoption",
];

/// Source classification used by the evidence gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceSourceKind {
    Filing,
    OfficialMaterial,
    IndependentMaterial,
    StructuredHiring,
    DiscoveryArticle,
    Other(String),
}

/// Semantic class assigned after a candidate has passed the evidence gate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    /// A dated authoritative claim that is verified but not structurally relevant.
    ValidatedFact,
    /// A verified claim with an explicit structural-domain signal.
    StructuralEvidence,
}

/// Polarity retained for later supporting/counter routing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidencePolarity {
    Supporting,
    Counter,
}

/// A concrete claim candidate that has not yet passed validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCandidate {
    company_id: String,
    company_name: String,
    concrete_change: String,
    effective_date: Option<NaiveDate>,
    production_area: String,
    source_kind: EvidenceSourceKind,
    source_tier: SourceTier,
    polarity: EvidencePolarity,
    source_uri: String,
    source_title: String,
    passage: String,
    document_kind: Option<DocumentKind>,
    reference_model_family: Option<ReferenceModelEvidenceFamily>,
    reference_model_named_peer: Option<String>,
    reference_model_source_role: Option<ReferenceModelSourceRole>,
    provenance: Provenance,
}

impl EvidenceCandidate {
    /// Creates a candidate with source identity and the minimum claim fields.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        company_id: impl Into<String>,
        company_name: impl Into<String>,
        concrete_change: impl Into<String>,
        effective_date: Option<NaiveDate>,
        production_area: impl Into<String>,
        source_kind: EvidenceSourceKind,
        source_tier: SourceTier,
        polarity: EvidencePolarity,
        source_uri: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let company_id = company_id.into();
        let company_name = company_name.into();
        let concrete_change = bounded(concrete_change.into());
        let production_area = bounded(production_area.into());
        let source_uri = source_uri.into();
        if company_id.trim().is_empty()
            || company_name.trim().is_empty()
            || concrete_change.trim().is_empty()
            || source_uri.trim().is_empty()
        {
            return Err(RuntimeError::invalid_model(
                "evidence candidate identity and claim cannot be blank",
            ));
        }
        let provenance = Provenance::new(
            source_uri.clone(),
            concrete_change.clone(),
            Utc::now(),
            effective_date,
        )?;
        Ok(Self {
            company_id,
            company_name,
            concrete_change: concrete_change.clone(),
            effective_date,
            production_area,
            source_kind,
            source_tier,
            polarity,
            source_uri,
            source_title: String::new(),
            passage: concrete_change,
            document_kind: None,
            reference_model_family: None,
            reference_model_named_peer: None,
            reference_model_source_role: None,
            provenance,
        })
    }

    /// Adds bounded source title and passage details used by the validation gate.
    pub fn with_source_details(
        mut self,
        source_title: impl Into<String>,
        passage: impl Into<String>,
    ) -> Self {
        self.source_title = bounded(source_title.into());
        self.passage = bounded(passage.into());
        self.provenance = Provenance::new(
            self.source_uri.clone(),
            self.passage.clone(),
            *self.provenance.retrieved_at(),
            self.effective_date,
        )
        .expect("candidate source details preserve valid provenance");
        self
    }

    /// Adds explicit reference-model metadata after a claim has been bounded.
    pub fn with_reference_model_metadata(
        mut self,
        family: ReferenceModelEvidenceFamily,
        named_peer: Option<String>,
    ) -> Result<Self, RuntimeError> {
        if named_peer
            .as_deref()
            .is_some_and(|peer| peer.trim().is_empty())
        {
            return Err(RuntimeError::invalid_model(
                "reference-model named peer cannot be blank",
            ));
        }
        self.reference_model_family = Some(family);
        self.reference_model_named_peer = named_peer;
        Ok(self)
    }

    /// Adds an explicit source provenance role after the claim is bounded.
    pub fn with_reference_model_source_role(
        mut self,
        source_role: Option<ReferenceModelSourceRole>,
    ) -> Self {
        self.reference_model_source_role = source_role;
        self
    }

    /// Returns the company identity.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the display company name.
    pub fn company_name(&self) -> &str {
        &self.company_name
    }

    /// Returns the concrete change or fact claim.
    pub fn concrete_change(&self) -> &str {
        &self.concrete_change
    }

    /// Returns the optional effective date.
    pub const fn effective_date(&self) -> Option<&NaiveDate> {
        self.effective_date.as_ref()
    }

    /// Returns the production-system area.
    pub fn production_area(&self) -> &str {
        &self.production_area
    }

    /// Returns the source URI.
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Returns the discovered document context retained for deterministic
    /// extraction, when the candidate came from a classified document.
    pub const fn document_kind(&self) -> Option<DocumentKind> {
        self.document_kind
    }
}

/// Stable validation failure for an incomplete or unsupported candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceValidationError {
    MissingRequiredField {
        field: &'static str,
    },
    DateAfterCutoff {
        effective_date: NaiveDate,
        cutoff: NaiveDate,
    },
    UnsupportedAuthority,
}

impl fmt::Display for EvidenceValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredField { field } => {
                write!(formatter, "missing required field {field}")
            }
            Self::DateAfterCutoff {
                effective_date,
                cutoff,
            } => write!(
                formatter,
                "effective date {effective_date} is after cutoff {cutoff}"
            ),
            Self::UnsupportedAuthority => formatter.write_str("source is not authoritative"),
        }
    }
}

impl std::error::Error for EvidenceValidationError {}

/// A candidate that satisfied every evidence promotion rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedEvidence {
    candidate: EvidenceCandidate,
    content_hash: String,
}

impl ValidatedEvidence {
    /// Returns the company identity.
    pub fn company_id(&self) -> &str {
        self.candidate.company_id()
    }

    /// Returns the production-system area.
    pub fn production_area(&self) -> &str {
        self.candidate.production_area()
    }

    /// Returns the deterministic structural domain, when the claim is structural.
    pub fn structural_dimension(&self) -> Option<StructuralDimension> {
        if self.candidate.document_kind == Some(DocumentKind::Careers) {
            return None;
        }
        structural_dimension_for_text(&self.candidate.passage)
    }

    /// Returns the deterministic reference-model family assigned to the claim.
    pub const fn reference_model_family(&self) -> Option<ReferenceModelEvidenceFamily> {
        self.candidate.reference_model_family
    }

    /// Returns the named peer retained for an industry-diffusion claim.
    pub fn reference_model_named_peer(&self) -> Option<&str> {
        self.candidate.reference_model_named_peer.as_deref()
    }

    /// Returns the source provenance role retained for the validated claim.
    pub const fn reference_model_source_role(&self) -> Option<ReferenceModelSourceRole> {
        self.candidate.reference_model_source_role
    }

    /// Returns the validated effective date.
    pub const fn effective_date(&self) -> Option<&NaiveDate> {
        self.candidate.effective_date()
    }

    /// Returns the deterministic content hash retained with the promotion.
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// Returns the deterministic semantic class of the validated claim.
    pub fn evidence_class(&self) -> EvidenceClass {
        if self.structural_dimension().is_some() {
            EvidenceClass::StructuralEvidence
        } else {
            EvidenceClass::ValidatedFact
        }
    }

    /// Converts validated evidence into the existing normalized fact boundary.
    pub fn to_normalized_fact(&self, index: usize) -> Result<NormalizedFact, RuntimeError> {
        if index == 0 {
            return Err(RuntimeError::invalid_model(
                "validated evidence index must be one-based",
            ));
        }
        let provenance = Provenance::new(
            self.candidate.source_uri.clone(),
            format!(
                "title={}; document_kind={}; production_area={}; effective_date={}; passage={}; extractor={}; content_hash={}",
                self.candidate.source_title,
                self.candidate
                    .document_kind
                    .map(DocumentKind::as_str)
                    .unwrap_or("unknown"),
                self.candidate.production_area,
                self.candidate
                    .effective_date
                    .map(|date| date.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                self.candidate.passage,
                EXTRACTOR_VERSION,
                self.content_hash,
            ),
            *self.candidate.provenance.retrieved_at(),
            self.candidate.effective_date,
        )?;
        let kind_prefix = match self.evidence_class() {
            EvidenceClass::ValidatedFact => "evidence_official_material",
            EvidenceClass::StructuralEvidence => "evidence_structural_change",
        };
        NormalizedFact::new_with_structural_dimension_and_reference_model_metadata(
            self.candidate.company_id.clone(),
            format!("{kind_prefix}_{index:03}"),
            self.candidate.concrete_change.clone(),
            self.structural_dimension(),
            self.reference_model_family(),
            self.candidate.reference_model_named_peer.clone(),
            FactStatus::Known,
            Confidence::High,
            provenance,
        )?
        .with_reference_model_source_role(self.reference_model_source_role())
    }
}

/// Extracts a claim candidate only from an authoritative discovered document.
pub fn extract_evidence_candidate(observation: &SourceObservation) -> Option<EvidenceCandidate> {
    if observation.material_kind() != SourceMaterialKind::Document
        || observation.status() != SourceStatus::Known
        || !observation.tier().is_authoritative()
        || observation.provenance().effective_date().is_none()
    {
        return None;
    }
    let title = observation
        .title()
        .filter(|title| !title.trim().is_empty())?;
    let document_kind = observation.document_kind()?;
    let extraction_text = if observation.kind() == SourceKind::IndependentResearch
        && !observation.text().trim().is_empty()
    {
        format!("{title}: {}", observation.text())
    } else {
        observation.text().to_owned()
    };
    let (passage, production_signal) = match document_kind {
        DocumentKind::Careers => extract_careers_claim_sentence(observation.text()),
        _ => extract_claim_sentence(&extraction_text),
    }?;
    let source_kind = match observation.kind() {
        SourceKind::Gdelt => EvidenceSourceKind::DiscoveryArticle,
        SourceKind::Greenhouse | SourceKind::Lever => EvidenceSourceKind::StructuredHiring,
        SourceKind::Sec
        | SourceKind::OfficialIr
        | SourceKind::Careers
        | SourceKind::EngineeringAiBlog
        | SourceKind::OfficialResearch => EvidenceSourceKind::OfficialMaterial,
        SourceKind::IndependentResearch => EvidenceSourceKind::IndependentMaterial,
    };
    let mut candidate = EvidenceCandidate::new(
        observation.company_id(),
        observation.company_id(),
        passage.clone(),
        observation.provenance().effective_date().copied(),
        production_signal,
        source_kind,
        observation.tier(),
        EvidencePolarity::Supporting,
        observation.provenance().source_uri(),
    )
    .ok()?;
    candidate.source_title = bounded(title.to_owned());
    candidate.passage = passage.clone();
    candidate.document_kind = Some(document_kind);
    if let Some(family) = reference_model_family_for_text(document_kind, &passage) {
        let named_peer = (family == ReferenceModelEvidenceFamily::IndustryDiffusion)
            .then(|| reference_model_named_peer_for_text(&passage))
            .flatten();
        candidate = candidate
            .with_reference_model_metadata(family, named_peer)
            .ok()?;
        candidate = candidate.with_reference_model_source_role(
            reference_model_source_role_for_observation(observation.kind()),
        );
    }
    candidate.provenance = Provenance::new(
        observation.provenance().source_uri(),
        passage,
        *observation.provenance().retrieved_at(),
        observation.provenance().effective_date().copied(),
    )
    .ok()?;
    Some(candidate)
}

fn reference_model_source_role_for_observation(
    source_kind: SourceKind,
) -> Option<ReferenceModelSourceRole> {
    match source_kind {
        SourceKind::OfficialResearch => Some(ReferenceModelSourceRole::SupplierAttribution),
        SourceKind::IndependentResearch => {
            Some(ReferenceModelSourceRole::IndependentCustomerDisclosure)
        }
        SourceKind::Sec | SourceKind::OfficialIr => {
            Some(ReferenceModelSourceRole::RegulatoryOrFiling)
        }
        SourceKind::Gdelt => Some(ReferenceModelSourceRole::DiscoveryOnly),
        SourceKind::Careers
        | SourceKind::EngineeringAiBlog
        | SourceKind::Greenhouse
        | SourceKind::Lever => None,
    }
}

fn extract_claim_sentence(text: &str) -> Option<(String, String)> {
    extract_claim_sentence_with_predicate(text, |lower| {
        CHANGE_SIGNALS.iter().any(|signal| lower.contains(signal))
    })
}

fn extract_careers_claim_sentence(text: &str) -> Option<(String, String)> {
    extract_claim_sentence_with_predicate(text, careers_has_explicit_hiring_change)
}

fn careers_has_explicit_hiring_change(lower: &str) -> bool {
    CAREERS_EXPLICIT_HIRING_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

fn extract_claim_sentence_with_predicate(
    text: &str,
    qualifies_change: impl Fn(&str) -> bool,
) -> Option<(String, String)> {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut sentence_start = 0;
    for (index, character) in normalized.char_indices() {
        if !matches!(character, '.' | '!' | '?' | '。' | '！' | '？') {
            continue;
        }
        let end = index + character.len_utf8();
        let sentence = normalized[sentence_start..end].trim();
        sentence_start = end;
        if sentence.split_whitespace().count() < MIN_CLAIM_WORDS {
            continue;
        }
        let lower = sentence.to_ascii_lowercase();
        if !qualifies_change(&lower) {
            continue;
        }
        let Some(production_signal) = PRODUCTION_SIGNALS
            .iter()
            .find(|signal| lower.contains(**signal))
        else {
            continue;
        };
        return Some((
            bounded(sentence.to_owned()),
            (*production_signal).to_owned(),
        ));
    }
    None
}

/// Validates the complete promotion boundary using a fixed evidence cutoff.
pub fn validate_evidence_candidate(
    candidate: &EvidenceCandidate,
    cutoff: NaiveDate,
) -> Result<ValidatedEvidence, EvidenceValidationError> {
    let Some(effective_date) = candidate.effective_date else {
        return Err(EvidenceValidationError::MissingRequiredField {
            field: "effective_date",
        });
    };
    if effective_date > cutoff {
        return Err(EvidenceValidationError::DateAfterCutoff {
            effective_date,
            cutoff,
        });
    }
    for (field, value) in [
        ("company_id", candidate.company_id()),
        ("company_name", candidate.company_name()),
        ("concrete_change", candidate.concrete_change()),
        ("source_uri", candidate.source_uri()),
    ] {
        if value.trim().is_empty() {
            return Err(EvidenceValidationError::MissingRequiredField { field });
        }
    }
    if candidate.production_area.trim().is_empty() {
        return Err(EvidenceValidationError::MissingRequiredField {
            field: "production_area",
        });
    }
    if candidate.source_title.trim().is_empty() {
        return Err(EvidenceValidationError::MissingRequiredField {
            field: "source_title",
        });
    }
    if candidate.passage.trim().is_empty() {
        return Err(EvidenceValidationError::MissingRequiredField { field: "passage" });
    }
    if !candidate.source_tier.is_authoritative() {
        return Err(EvidenceValidationError::UnsupportedAuthority);
    }
    let content_hash = content_hash(&format!(
        "{}|{}|{}|{}",
        candidate.concrete_change,
        candidate.production_area,
        candidate.source_uri,
        candidate.passage
    ));
    Ok(ValidatedEvidence {
        candidate: candidate.clone(),
        content_hash,
    })
}

fn bounded(mut value: String) -> String {
    if value.len() > MAX_FIELD_BYTES {
        value.truncate(MAX_FIELD_BYTES);
    }
    value
}

fn structural_dimension_for_text(text: &str) -> Option<StructuralDimension> {
    let lower = text.to_ascii_lowercase();
    if OPERATING_METRIC_DIMENSION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(StructuralDimension::OperatingMetric);
    }
    if PRODUCTION_SYSTEM_DIMENSION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(StructuralDimension::ProductionSystem);
    }
    if WORKFLOW_DIMENSION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(StructuralDimension::Workflow);
    }
    if ORGANIZATION_DIMENSION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(StructuralDimension::Organization);
    }
    None
}

fn reference_model_family_for_text(
    document_kind: DocumentKind,
    text: &str,
) -> Option<ReferenceModelEvidenceFamily> {
    if document_kind == DocumentKind::Careers {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    if REFERENCE_MODEL_DIFFUSION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(ReferenceModelEvidenceFamily::IndustryDiffusion);
    }
    if REFERENCE_MODEL_ORGANIZATION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(ReferenceModelEvidenceFamily::OrganizationRewrite);
    }
    if matches!(document_kind, DocumentKind::Filing | DocumentKind::Earnings)
        && REFERENCE_MODEL_OUTCOME_SIGNALS
            .iter()
            .any(|signal| lower.contains(signal))
    {
        return Some(ReferenceModelEvidenceFamily::SustainedOutcome);
    }
    if REFERENCE_MODEL_PRODUCTION_SIGNALS
        .iter()
        .any(|signal| lower.contains(signal))
    {
        return Some(ReferenceModelEvidenceFamily::ProductionSystemRewrite);
    }
    None
}

fn reference_model_named_peer_for_text(text: &str) -> Option<String> {
    const PEER_MARKERS: &[&str] = &[
        "adopted by ",
        "implemented by ",
        "used by ",
        "customers include ",
    ];
    let lower = text.to_ascii_lowercase();
    for marker in PEER_MARKERS {
        let Some(start) = lower.find(marker) else {
            continue;
        };
        let remainder = text.get(start + marker.len()..)?;
        let peer = remainder
            .split(['.', ',', ';', ':', '。', '，', '；'])
            .next()
            .map(str::trim)
            .and_then(|value| {
                value
                    .split_once(" and ")
                    .map_or(Some(value), |(first, _)| Some(first.trim()))
            })
            .map(|value| value.trim_start_matches("the ").trim())
            .filter(|value| {
                let words = value.split_whitespace().count();
                (1..=8).contains(&words)
                    && !matches!(
                        value.to_ascii_lowercase().as_str(),
                        "customers" | "the industry"
                    )
            })?;
        if !peer.is_empty() {
            return Some(peer.to_owned());
        }
    }
    let adoption_regex = Regex::new(
        r"(?i)^\s*([A-Z][A-Za-z0-9&.'-]*(?:\s+[A-Z][A-Za-z0-9&.'-]*){0,4})\s+(?:has\s+adopted|has\s+implemented|is\s+using|is\s+rolling\s+out|adopted|implemented|used|uses|deployed|deploys|rolled\s+out|rolls\s+out|built|launched)\b",
    )
    .ok()?;
    let peer = adoption_regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().trim())
        .filter(|value| {
            !matches!(
                value.to_ascii_lowercase().as_str(),
                "the company" | "the firm" | "our team" | "our company"
            )
        })?;
    if !peer.is_empty() {
        return Some(peer.to_owned());
    }
    None
}

fn content_hash(value: &str) -> String {
    let mut hash = 14_695_981_039_346_656_037u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{hash:016x}")
}
