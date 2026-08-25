//! Deterministic claim extraction and validation for Weekly Radar evidence.

use chrono::{NaiveDate, Utc};
use std::fmt;

use super::model::{Confidence, FactStatus, NormalizedFact, Provenance};
use super::sources::{SourceKind, SourceMaterialKind, SourceObservation, SourceStatus, SourceTier};
use super::RuntimeError;

const EXTRACTOR_VERSION: &str = "weekly-radar-evidence-v1";
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
    "automat",
    "replaced",
    "moderniz",
    "built",
    "doubled",
    "reduced",
    "increased",
];

const PRODUCTION_SIGNALS: &[&str] = &[
    "engineering",
    "platform",
    "product",
    "operations",
    "workflow",
    "automation",
    "ai",
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

/// Source classification used by the evidence gate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceSourceKind {
    Filing,
    OfficialMaterial,
    StructuredHiring,
    DiscoveryArticle,
    Other(String),
}

impl EvidenceSourceKind {
    fn as_str(&self) -> String {
        match self {
            Self::Filing => "filing".to_owned(),
            Self::OfficialMaterial => "official_material".to_owned(),
            Self::StructuredHiring => "structured_hiring".to_owned(),
            Self::DiscoveryArticle => "discovery_article".to_owned(),
            Self::Other(value) => value.clone(),
        }
    }
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

    /// Returns the validated effective date.
    pub const fn effective_date(&self) -> Option<&NaiveDate> {
        self.candidate.effective_date()
    }

    /// Returns the deterministic content hash retained with the promotion.
    pub fn content_hash(&self) -> &str {
        &self.content_hash
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
                "title={}; production_area={}; effective_date={}; passage={}; extractor={}; content_hash={}",
                self.candidate.source_title,
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
        NormalizedFact::new(
            self.candidate.company_id.clone(),
            format!(
                "evidence_{}_{index:03}",
                self.candidate.source_kind.as_str()
            ),
            self.candidate.concrete_change.clone(),
            FactStatus::Known,
            Confidence::High,
            provenance,
        )
    }
}

/// Extracts a claim candidate only from an authoritative discovered document.
pub fn extract_evidence_candidate(observation: &SourceObservation) -> Option<EvidenceCandidate> {
    if observation.material_kind() != SourceMaterialKind::Document
        || observation.status() != SourceStatus::Known
        || observation.tier() != SourceTier::OfficialPrimary
        || observation.provenance().effective_date().is_none()
    {
        return None;
    }
    let title = observation
        .title()
        .filter(|title| !title.trim().is_empty())?;
    let (passage, production_signal) = extract_claim_sentence(observation.text())?;
    let source_kind = match observation.kind() {
        SourceKind::Gdelt => EvidenceSourceKind::DiscoveryArticle,
        SourceKind::Greenhouse | SourceKind::Lever => EvidenceSourceKind::StructuredHiring,
        SourceKind::OfficialIr | SourceKind::Careers | SourceKind::EngineeringAiBlog => {
            EvidenceSourceKind::OfficialMaterial
        }
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
    candidate.provenance = Provenance::new(
        observation.provenance().source_uri(),
        passage,
        *observation.provenance().retrieved_at(),
        observation.provenance().effective_date().copied(),
    )
    .ok()?;
    Some(candidate)
}

fn extract_claim_sentence(text: &str) -> Option<(String, String)> {
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
        if !CHANGE_SIGNALS.iter().any(|signal| lower.contains(signal)) {
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

fn content_hash(value: &str) -> String {
    let mut hash = 14_695_981_039_346_656_037u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{hash:016x}")
}
