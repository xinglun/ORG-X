//! Opaque validation facts, evidence references, and horizon invariants.

use std::collections::BTreeSet;
use std::fmt;

#[cfg(test)]
mod mod_test;

fn required(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, ValidationDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(ValidationDomainError::EmptyValue { field });
    }
    Ok(value)
}

fn validate_texts(
    field: &'static str,
    values: Vec<String>,
) -> Result<Vec<String>, ValidationDomainError> {
    values
        .into_iter()
        .map(|value| required(field, value))
        .collect()
}

fn validate_evidence(
    values: Vec<EvidenceReference>,
) -> Result<Vec<EvidenceReference>, ValidationDomainError> {
    let mut ids = BTreeSet::new();
    for evidence in &values {
        if !ids.insert(evidence.id.clone()) {
            return Err(ValidationDomainError::DuplicateEvidenceId {
                id: evidence.id.clone(),
            });
        }
    }
    Ok(values)
}

fn validate_metrics(
    values: Vec<MetricObservation>,
) -> Result<Vec<MetricObservation>, ValidationDomainError> {
    let mut names = BTreeSet::new();
    for metric in &values {
        if !names.insert(metric.name.clone()) {
            return Err(ValidationDomainError::DuplicateMetricName {
                name: metric.name.clone(),
            });
        }
    }
    Ok(values)
}

fn evidence_ids(values: impl IntoIterator<Item = EvidenceReference>) -> BTreeSet<String> {
    values.into_iter().map(|evidence| evidence.id).collect()
}

/// Validation-domain construction and collection failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationDomainError {
    /// A required boundary field contained only whitespace.
    EmptyValue { field: &'static str },
    /// An evidence identity appeared more than once in one validation boundary.
    DuplicateEvidenceId { id: String },
    /// A metric name appeared more than once in one observation or baseline.
    DuplicateMetricName { name: String },
    /// A record already contains an observation for the supplied horizon.
    DuplicateHorizon { horizon: ValidationHorizon },
}

impl fmt::Display for ValidationDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateEvidenceId { id } => {
                write!(formatter, "duplicate evidence identity {id}")
            }
            Self::DuplicateMetricName { name } => write!(formatter, "duplicate metric name {name}"),
            Self::DuplicateHorizon { horizon } => {
                write!(
                    formatter,
                    "duplicate validation horizon {}",
                    horizon.as_str()
                )
            }
        }
    }
}

impl std::error::Error for ValidationDomainError {}

/// The three follow-up horizons defined by the validation strategy.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ValidationHorizon {
    /// Observation taken six months after the T0 baseline.
    SixMonths,
    /// Observation taken twelve months after the T0 baseline.
    TwelveMonths,
    /// Observation taken twenty-four months after the T0 baseline.
    TwentyFourMonths,
}

impl ValidationHorizon {
    /// All follow-up horizons in deterministic comparison order.
    pub const FOLLOW_UPS: [Self; 3] = [Self::SixMonths, Self::TwelveMonths, Self::TwentyFourMonths];

    /// Returns a stable display/storage label without deriving a duration.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SixMonths => "six_months",
            Self::TwelveMonths => "twelve_months",
            Self::TwentyFourMonths => "twenty_four_months",
        }
    }
}

/// Source-quality classification supplied with an observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceQuality {
    /// Primary source quality was supplied by the caller.
    Primary,
    /// Secondary source quality was supplied by the caller.
    Secondary,
    /// Source quality was not available.
    Unknown,
}

/// Status supplied for one validation signal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationStatus {
    /// The supplied observation says the signal is confirmed.
    Confirmed,
    /// The supplied observation says the signal remains unknown.
    Unknown,
    /// The supplied observation says the signal is unavailable.
    Unavailable,
}

/// A stable reference to evidence retained by a validation record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceReference {
    id: String,
    description: String,
}

impl EvidenceReference {
    /// Creates an evidence reference while preserving nonblank input.
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, ValidationDomainError> {
        Ok(Self {
            id: required("evidence id", id)?,
            description: required("evidence description", description)?,
        })
    }

    /// Returns the supplied evidence identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the supplied evidence description.
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// An opaque metric value and its source-quality/evidence references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MetricObservation {
    name: String,
    value: String,
    unit: String,
    source_quality: SourceQuality,
    evidence: Vec<EvidenceReference>,
}

impl MetricObservation {
    /// Creates a metric without parsing, converting, or comparing its value.
    pub fn new(
        name: impl Into<String>,
        value: impl Into<String>,
        unit: impl Into<String>,
        source_quality: SourceQuality,
        evidence: Vec<EvidenceReference>,
    ) -> Result<Self, ValidationDomainError> {
        Ok(Self {
            name: required("metric name", name)?,
            value: required("metric value", value)?,
            unit: required("metric unit", unit)?,
            source_quality,
            evidence: validate_evidence(evidence)?,
        })
    }

    /// Returns the supplied metric name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the supplied opaque metric value.
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the supplied metric unit.
    pub fn unit(&self) -> &str {
        &self.unit
    }

    /// Returns the supplied source-quality classification.
    pub const fn source_quality(&self) -> SourceQuality {
        self.source_quality
    }

    /// Returns the evidence references attached to this metric.
    pub fn evidence(&self) -> &[EvidenceReference] {
        &self.evidence
    }
}

/// A validation signal and the evidence supplied for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationSignal {
    status: ValidationStatus,
    note: String,
    evidence: Vec<EvidenceReference>,
}

impl ValidationSignal {
    /// Creates a signal without interpreting its status or note.
    pub fn new(
        status: ValidationStatus,
        note: impl Into<String>,
        evidence: Vec<EvidenceReference>,
    ) -> Result<Self, ValidationDomainError> {
        Ok(Self {
            status,
            note: required("validation signal note", note)?,
            evidence: validate_evidence(evidence)?,
        })
    }

    /// Returns the supplied signal status.
    pub const fn status(&self) -> ValidationStatus {
        self.status
    }

    /// Returns the supplied signal note.
    pub fn note(&self) -> &str {
        &self.note
    }

    /// Returns the evidence references attached to this signal.
    pub fn evidence(&self) -> &[EvidenceReference] {
        &self.evidence
    }
}

/// A peer-group baseline metric retained for later comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PeerBaseline {
    peer_group: String,
    metric: MetricObservation,
}

impl PeerBaseline {
    /// Creates a peer baseline without comparing it to the company metric.
    pub fn new(
        peer_group: impl Into<String>,
        metric: MetricObservation,
    ) -> Result<Self, ValidationDomainError> {
        Ok(Self {
            peer_group: required("peer group", peer_group)?,
            metric,
        })
    }

    /// Returns the supplied peer-group label.
    pub fn peer_group(&self) -> &str {
        &self.peer_group
    }

    /// Returns the retained peer metric.
    pub fn metric(&self) -> &MetricObservation {
        &self.metric
    }
}

/// T0 facts retained before later validation observations exist.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationBaseline {
    company_id: String,
    stage: String,
    evidence: Vec<EvidenceReference>,
    hypotheses: Vec<String>,
    counter_evidence: Vec<EvidenceReference>,
    missing_proof: Vec<String>,
    peer_baseline: Vec<PeerBaseline>,
}

impl ValidationBaseline {
    /// Creates a baseline and rejects duplicate evidence identities across its evidence sets.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        company_id: impl Into<String>,
        stage: impl Into<String>,
        evidence: Vec<EvidenceReference>,
        hypotheses: Vec<impl Into<String>>,
        counter_evidence: Vec<EvidenceReference>,
        missing_proof: Vec<impl Into<String>>,
        peer_baseline: Vec<PeerBaseline>,
    ) -> Result<Self, ValidationDomainError> {
        let evidence = validate_evidence(evidence)?;
        let counter_evidence = validate_evidence(counter_evidence)?;
        let mut all_evidence = evidence_ids(evidence.clone());
        for id in evidence_ids(counter_evidence.clone()) {
            if !all_evidence.insert(id.clone()) {
                return Err(ValidationDomainError::DuplicateEvidenceId { id });
            }
        }
        let hypotheses = validate_texts(
            "hypothesis",
            hypotheses.into_iter().map(Into::into).collect(),
        )?;
        let missing_proof = validate_texts(
            "missing proof",
            missing_proof.into_iter().map(Into::into).collect(),
        )?;
        let peer_metrics = validate_metrics(
            peer_baseline
                .iter()
                .map(|peer| peer.metric.clone())
                .collect(),
        )?;
        let peer_baseline = peer_baseline
            .into_iter()
            .zip(peer_metrics)
            .map(|(peer, _)| peer)
            .collect();

        Ok(Self {
            company_id: required("company id", company_id)?,
            stage: required("baseline stage", stage)?,
            evidence,
            hypotheses,
            counter_evidence,
            missing_proof,
            peer_baseline,
        })
    }

    /// Returns the company identity retained at T0.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the supplied T0 stage text without assigning a stage.
    pub fn stage(&self) -> &str {
        &self.stage
    }

    /// Returns baseline evidence references.
    pub fn evidence(&self) -> &[EvidenceReference] {
        &self.evidence
    }

    /// Returns supplied T0 hypotheses.
    pub fn hypotheses(&self) -> &[String] {
        &self.hypotheses
    }

    /// Returns baseline counter-evidence references.
    pub fn counter_evidence(&self) -> &[EvidenceReference] {
        &self.counter_evidence
    }

    /// Returns supplied missing-proof descriptions.
    pub fn missing_proof(&self) -> &[String] {
        &self.missing_proof
    }

    /// Returns peer baseline metrics.
    pub fn peer_baseline(&self) -> &[PeerBaseline] {
        &self.peer_baseline
    }

    fn evidence_references(&self) -> Vec<EvidenceReference> {
        self.evidence
            .iter()
            .chain(&self.counter_evidence)
            .cloned()
            .chain(
                self.peer_baseline
                    .iter()
                    .flat_map(|peer| peer.metric.evidence.iter().cloned()),
            )
            .collect()
    }
}

/// One follow-up validation observation at a fixed horizon.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationObservation {
    horizon: ValidationHorizon,
    observed_at: String,
    productivity_divergence: ValidationSignal,
    economic_capture: ValidationSignal,
    production_model_continuity: ValidationSignal,
    competitor_imitation: ValidationSignal,
    industry_diffusion: ValidationSignal,
    metrics: Vec<MetricObservation>,
}

impl ValidationObservation {
    /// Creates a follow-up observation and validates metric/evidence identity uniqueness.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        horizon: ValidationHorizon,
        observed_at: impl Into<String>,
        productivity_divergence: ValidationSignal,
        economic_capture: ValidationSignal,
        production_model_continuity: ValidationSignal,
        competitor_imitation: ValidationSignal,
        industry_diffusion: ValidationSignal,
        metrics: Vec<MetricObservation>,
    ) -> Result<Self, ValidationDomainError> {
        let metrics = validate_metrics(metrics)?;
        let mut references = Vec::new();
        for signal in [
            &productivity_divergence,
            &economic_capture,
            &production_model_continuity,
            &competitor_imitation,
            &industry_diffusion,
        ] {
            references.extend(signal.evidence.iter().cloned());
        }
        references.extend(
            metrics
                .iter()
                .flat_map(|metric| metric.evidence.iter().cloned()),
        );
        validate_evidence(references)?;

        Ok(Self {
            horizon,
            observed_at: required("observation time", observed_at)?,
            productivity_divergence,
            economic_capture,
            production_model_continuity,
            competitor_imitation,
            industry_diffusion,
            metrics,
        })
    }

    /// Returns the fixed follow-up horizon.
    pub const fn horizon(&self) -> ValidationHorizon {
        self.horizon
    }

    /// Returns the supplied opaque observation time.
    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }

    /// Returns the productivity-divergence signal.
    pub fn productivity_divergence(&self) -> &ValidationSignal {
        &self.productivity_divergence
    }

    /// Returns the economic-capture signal.
    pub fn economic_capture(&self) -> &ValidationSignal {
        &self.economic_capture
    }

    /// Returns the production-model-continuity signal.
    pub fn production_model_continuity(&self) -> &ValidationSignal {
        &self.production_model_continuity
    }

    /// Returns the competitor-imitation signal.
    pub fn competitor_imitation(&self) -> &ValidationSignal {
        &self.competitor_imitation
    }

    /// Returns the industry-diffusion signal.
    pub fn industry_diffusion(&self) -> &ValidationSignal {
        &self.industry_diffusion
    }

    /// Returns opaque metrics supplied for the observation.
    pub fn metrics(&self) -> &[MetricObservation] {
        &self.metrics
    }

    fn evidence_references(&self) -> Vec<EvidenceReference> {
        self.productivity_divergence
            .evidence
            .iter()
            .chain(&self.economic_capture.evidence)
            .chain(&self.production_model_continuity.evidence)
            .chain(&self.competitor_imitation.evidence)
            .chain(&self.industry_diffusion.evidence)
            .cloned()
            .chain(
                self.metrics
                    .iter()
                    .flat_map(|metric| metric.evidence.iter().cloned()),
            )
            .collect()
    }
}

/// A T0 baseline and zero or more later validation observations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationRecord {
    baseline: ValidationBaseline,
    observations: Vec<ValidationObservation>,
}

impl ValidationRecord {
    /// Creates an empty validation history for one company.
    pub fn new(baseline: ValidationBaseline) -> Result<Self, ValidationDomainError> {
        validate_evidence(baseline.evidence_references())?;
        Ok(Self {
            baseline,
            observations: Vec::new(),
        })
    }

    /// Adds one observation while preserving the previous record on rejection.
    pub fn add_observation(
        &mut self,
        observation: ValidationObservation,
    ) -> Result<(), ValidationDomainError> {
        if self
            .observations
            .iter()
            .any(|item| item.horizon == observation.horizon)
        {
            return Err(ValidationDomainError::DuplicateHorizon {
                horizon: observation.horizon,
            });
        }

        let mut references = self.baseline.evidence_references();
        references.extend(
            self.observations
                .iter()
                .flat_map(ValidationObservation::evidence_references),
        );
        references.extend(observation.evidence_references());
        validate_evidence(references)?;
        self.observations.push(observation);
        Ok(())
    }

    /// Returns the T0 baseline.
    pub fn baseline(&self) -> &ValidationBaseline {
        &self.baseline
    }

    /// Returns the company identity carried by the baseline.
    pub fn company_id(&self) -> &str {
        self.baseline.company_id()
    }

    /// Returns observations in insertion order.
    pub fn observations(&self) -> &[ValidationObservation] {
        &self.observations
    }

    /// Returns one observation when the horizon is present.
    pub fn observation(&self, horizon: ValidationHorizon) -> Option<&ValidationObservation> {
        self.observations
            .iter()
            .find(|observation| observation.horizon == horizon)
    }

    /// Returns follow-up horizons that have not yet been supplied.
    pub fn missing_horizons(&self) -> Vec<ValidationHorizon> {
        ValidationHorizon::FOLLOW_UPS
            .into_iter()
            .filter(|horizon| self.observation(*horizon).is_none())
            .collect()
    }
}
