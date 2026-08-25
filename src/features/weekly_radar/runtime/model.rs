//! Provider-neutral normalized facts and report input models.

use chrono::{DateTime, NaiveDate, Utc};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use super::error::RuntimeError;

/// Availability state for one normalized runtime fact.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FactStatus {
    /// A deterministic value was found and retained as a confirmed fact.
    Known,
    /// The source was available but the requested fact was ambiguous or absent.
    Unknown,
    /// The configured source was not available for this run.
    Unavailable,
    /// The observation is retained for review but is not authoritative. The
    /// status is provider-neutral; source/provider details remain in provenance.
    Unconfirmed,
}

impl FactStatus {
    /// Returns the stable report/status label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Known => "CONFIRMED",
            Self::Unknown => "UNKNOWN",
            Self::Unavailable => "UNAVAILABLE",
            Self::Unconfirmed => "UNCONFIRMED",
        }
    }
}

/// Coarse confidence retained independently from fact availability.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Confidence {
    /// Confidence is not available from the source or extraction rule.
    Unknown,
    /// The retained fact has weak supporting evidence.
    Low,
    /// The retained fact has moderate supporting evidence.
    Medium,
    /// The retained fact has strong supporting evidence.
    High,
    /// The retained fact uses explicitly approximate source language.
    Approximate,
}

impl Confidence {
    /// Returns the stable report/confidence label.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "UNKNOWN",
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Approximate => "APPROXIMATE",
        }
    }
}

/// Source details retained with every normalized fact.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    source_uri: String,
    source_field_or_passage: String,
    retrieved_at: DateTime<Utc>,
    effective_date: Option<NaiveDate>,
}

impl Provenance {
    /// Creates provenance from already parsed timestamps and dates.
    pub fn new(
        source_uri: impl Into<String>,
        source_field_or_passage: impl Into<String>,
        retrieved_at: DateTime<Utc>,
        effective_date: Option<NaiveDate>,
    ) -> Result<Self, RuntimeError> {
        let provenance = Self {
            source_uri: source_uri.into(),
            source_field_or_passage: source_field_or_passage.into(),
            retrieved_at,
            effective_date,
        };
        provenance.validate()?;
        Ok(provenance)
    }

    /// Creates provenance from the ISO-8601 strings used by source fixtures.
    pub fn from_rfc3339(
        source_uri: impl Into<String>,
        source_field_or_passage: impl Into<String>,
        retrieved_at: &str,
        effective_date: Option<&str>,
    ) -> Result<Self, RuntimeError> {
        let retrieved_at = DateTime::parse_from_rfc3339(retrieved_at)
            .map_err(|_| RuntimeError::invalid_model("retrieved_at must be RFC3339"))?
            .with_timezone(&Utc);
        let effective_date = effective_date
            .map(|date| {
                NaiveDate::parse_from_str(date, "%Y-%m-%d")
                    .map_err(|_| RuntimeError::invalid_model("effective_date must be YYYY-MM-DD"))
            })
            .transpose()?;
        Self::new(
            source_uri,
            source_field_or_passage,
            retrieved_at,
            effective_date,
        )
    }

    /// Validates required source identity fields.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.source_uri.trim().is_empty() {
            return Err(RuntimeError::invalid_model("source URI cannot be blank"));
        }
        if self.source_field_or_passage.trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "source field or passage cannot be blank",
            ));
        }
        Ok(())
    }

    /// Returns the source URI.
    pub fn source_uri(&self) -> &str {
        &self.source_uri
    }

    /// Returns the source field name or evidence passage.
    pub fn source_field_or_passage(&self) -> &str {
        &self.source_field_or_passage
    }

    /// Returns the UTC retrieval timestamp.
    pub const fn retrieved_at(&self) -> &DateTime<Utc> {
        &self.retrieved_at
    }

    /// Returns the optional source-effective date.
    pub const fn effective_date(&self) -> Option<&NaiveDate> {
        self.effective_date.as_ref()
    }
}

/// Structural domain represented by a validated enterprise-change claim.
///
/// This dimension refines `StructuralEvidence`; it does not alter judgment,
/// stage, or ranking semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralDimension {
    /// Reporting lines, responsibilities, teams, divisions, or organization.
    Organization,
    /// A changed business or engineering workflow, process, or operating model.
    Workflow,
    /// A changed platform, deployment, infrastructure, storage, or production system.
    ProductionSystem,
    /// A changed utilization, latency, throughput, capacity, cost, margin, or cash-flow metric.
    OperatingMetric,
}

/// One provider-neutral fact ready for deterministic report assembly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NormalizedFact {
    company_id: String,
    kind: String,
    value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    structural_dimension: Option<StructuralDimension>,
    status: FactStatus,
    confidence: Confidence,
    provenance: Provenance,
}

impl<'de> Deserialize<'de> for NormalizedFact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct NormalizedFactWire {
            company_id: String,
            kind: String,
            value: Option<String>,
            #[serde(default)]
            structural_dimension: Option<StructuralDimension>,
            status: FactStatus,
            confidence: Confidence,
            provenance: Provenance,
        }

        let wire = NormalizedFactWire::deserialize(deserializer)?;
        Self::build_with_dimension(
            wire.company_id,
            wire.kind,
            wire.value,
            wire.structural_dimension,
            wire.status,
            wire.confidence,
            wire.provenance,
        )
        .map_err(|error| D::Error::custom(error.to_string()))
    }
}

impl NormalizedFact {
    /// Creates a fact with a retained normalized value.
    pub fn new(
        company_id: impl Into<String>,
        kind: impl Into<String>,
        value: impl Into<String>,
        status: FactStatus,
        confidence: Confidence,
        provenance: Provenance,
    ) -> Result<Self, RuntimeError> {
        Self::build_with_dimension(
            company_id,
            kind,
            Some(value.into()),
            None,
            status,
            confidence,
            provenance,
        )
    }

    /// Creates a fact with a retained value and an optional structural domain.
    pub fn new_with_structural_dimension(
        company_id: impl Into<String>,
        kind: impl Into<String>,
        value: impl Into<String>,
        structural_dimension: Option<StructuralDimension>,
        status: FactStatus,
        confidence: Confidence,
        provenance: Provenance,
    ) -> Result<Self, RuntimeError> {
        Self::build_with_dimension(
            company_id,
            kind,
            Some(value.into()),
            structural_dimension,
            status,
            confidence,
            provenance,
        )
    }

    /// Creates an explicitly unavailable, unknown, or unconfirmed fact without
    /// inventing a value.
    pub fn without_value(
        company_id: impl Into<String>,
        kind: impl Into<String>,
        status: FactStatus,
        confidence: Confidence,
        provenance: Provenance,
    ) -> Result<Self, RuntimeError> {
        Self::build_with_dimension(company_id, kind, None, None, status, confidence, provenance)
    }

    fn build_with_dimension(
        company_id: impl Into<String>,
        kind: impl Into<String>,
        value: Option<String>,
        structural_dimension: Option<StructuralDimension>,
        status: FactStatus,
        confidence: Confidence,
        provenance: Provenance,
    ) -> Result<Self, RuntimeError> {
        let value = match status {
            FactStatus::Known => Some(value.ok_or_else(|| {
                RuntimeError::invalid_model("confirmed fact value cannot be absent")
            })?),
            FactStatus::Unknown | FactStatus::Unavailable | FactStatus::Unconfirmed => None,
        };
        let fact = Self {
            company_id: company_id.into(),
            kind: kind.into(),
            value,
            structural_dimension,
            status,
            confidence,
            provenance,
        };
        fact.validate()?;
        Ok(fact)
    }

    /// Validates identity fields and retained provenance.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.company_id.trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "fact company ID cannot be blank",
            ));
        }
        if self.kind.trim().is_empty() {
            return Err(RuntimeError::invalid_model("fact kind cannot be blank"));
        }
        self.provenance.validate()
    }

    /// Returns the stable company identifier.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the provider-neutral fact kind.
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the optional normalized value.
    pub fn value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Returns the optional structural domain of this fact.
    pub const fn structural_dimension(&self) -> Option<StructuralDimension> {
        self.structural_dimension
    }

    /// Returns the retained availability status.
    pub const fn status(&self) -> &FactStatus {
        &self.status
    }

    /// Returns the retained confidence classification.
    pub const fn confidence(&self) -> &Confidence {
        &self.confidence
    }

    /// Returns the complete retained provenance.
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// Human-readable company identity retained alongside runtime facts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompanyIdentity {
    id: String,
    name: String,
    ticker: String,
}

impl CompanyIdentity {
    /// Creates a company identity for report presentation.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        ticker: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let identity = Self {
            id: id.into(),
            name: name.into(),
            ticker: ticker.into(),
        };
        identity.validate()?;
        Ok(identity)
    }

    /// Validates the fields required to render a company label.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        for (field, value) in [("company ID", &self.id), ("company name", &self.name)] {
            if value.trim().is_empty() {
                return Err(RuntimeError::invalid_model(format!(
                    "{field} cannot be blank"
                )));
            }
        }
        Ok(())
    }

    /// Returns the stable company ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the ticker symbol.
    pub fn ticker(&self) -> &str {
        &self.ticker
    }
}

/// A safe, company-scoped failure from one source family.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceFailure {
    source: String,
    company_id: String,
    reason: String,
}

impl SourceFailure {
    /// Creates a failure using already sanitized operation context.
    pub fn new(
        source: impl Into<String>,
        company_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        let failure = Self {
            source: source.into(),
            company_id: company_id.into(),
            reason: reason.into(),
        };
        failure.validate()?;
        Ok(failure)
    }

    /// Validates the stable identifiers and non-empty failure category.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        for (field, value) in [
            ("failure source", &self.source),
            ("failure company ID", &self.company_id),
            ("failure reason", &self.reason),
        ] {
            if value.trim().is_empty() {
                return Err(RuntimeError::invalid_model(format!(
                    "{field} cannot be blank"
                )));
            }
        }
        Ok(())
    }

    /// Returns the source family.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the stable company ID.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns the sanitized failure category.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

fn is_zero(value: &usize) -> bool {
    *value == 0
}

/// Acquisition counters kept separate from confirmed fact counts.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResearchMetrics {
    #[serde(default, skip_serializing_if = "is_zero")]
    source_available: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    document_candidates: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    validated_evidence: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    structural_evidence: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    pending_leads: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    unavailable_sources: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    sec_stage_expected: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    sec_stage_available: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    sec_fact_expected: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    sec_fact_available: usize,
}

impl ResearchMetrics {
    /// Creates one complete acquisition-metrics envelope.
    pub const fn new(
        source_available: usize,
        document_candidates: usize,
        validated_evidence: usize,
        pending_leads: usize,
        unavailable_sources: usize,
    ) -> Self {
        Self {
            source_available,
            document_candidates,
            validated_evidence,
            structural_evidence: 0,
            pending_leads,
            unavailable_sources,
            sec_stage_expected: 0,
            sec_stage_available: 0,
            sec_fact_expected: 0,
            sec_fact_available: 0,
        }
    }

    /// Returns a copy with the number of promoted structural evidence records.
    pub const fn with_structural_evidence(mut self, structural_evidence: usize) -> Self {
        self.structural_evidence = structural_evidence;
        self
    }

    /// Returns a copy with distinct SEC stage and normalized-fact health.
    pub const fn with_sec_health(
        mut self,
        sec_stage_expected: usize,
        sec_stage_available: usize,
        sec_fact_expected: usize,
        sec_fact_available: usize,
    ) -> Self {
        self.sec_stage_expected = sec_stage_expected;
        self.sec_stage_available = sec_stage_available;
        self.sec_fact_expected = sec_fact_expected;
        self.sec_fact_available = sec_fact_available;
        self
    }

    /// Returns the number of reachable configured source entry points.
    pub const fn source_available(&self) -> usize {
        self.source_available
    }

    /// Returns the number of bounded document candidates discovered.
    pub const fn document_candidates(&self) -> usize {
        self.document_candidates
    }

    /// Returns the number of candidates promoted to validated evidence.
    pub const fn validated_evidence(&self) -> usize {
        self.validated_evidence
    }

    /// Returns the number of validated claims classified as structural evidence.
    pub const fn structural_evidence(&self) -> usize {
        self.structural_evidence
    }

    /// Returns the number of extracted but not yet validated leads.
    pub const fn pending_leads(&self) -> usize {
        self.pending_leads
    }

    /// Returns the number of configured source acquisitions that were unavailable.
    pub const fn unavailable_sources(&self) -> usize {
        self.unavailable_sources
    }

    /// Returns the number of SEC collection stages expected for configured CIKs.
    pub const fn sec_stage_expected(&self) -> usize {
        self.sec_stage_expected
    }

    /// Returns the number of SEC collection stages that completed without a stage failure.
    pub const fn sec_stage_available(&self) -> usize {
        self.sec_stage_available
    }

    /// Returns the number of normalized SEC facts produced by the adapter.
    pub const fn sec_fact_expected(&self) -> usize {
        self.sec_fact_expected
    }

    /// Returns the number of normalized SEC facts with a known value.
    pub const fn sec_fact_available(&self) -> usize {
        self.sec_fact_available
    }
}

/// Coverage counters for one configured source family.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceCoverage {
    source: String,
    expected: usize,
    available: usize,
    #[serde(default)]
    not_configured: usize,
    #[serde(default, skip_serializing_if = "is_zero")]
    not_applicable: usize,
}

impl SourceCoverage {
    /// Creates coverage counters and rejects impossible values.
    pub fn new(
        source: impl Into<String>,
        expected: usize,
        available: usize,
    ) -> Result<Self, RuntimeError> {
        Self::new_with_states(source, expected, available, 0, 0)
    }

    /// Creates coverage counters including optional-source configuration gaps.
    pub fn new_with_not_configured(
        source: impl Into<String>,
        expected: usize,
        available: usize,
        not_configured: usize,
    ) -> Result<Self, RuntimeError> {
        Self::new_with_states(source, expected, available, not_configured, 0)
    }

    /// Creates coverage counters with explicit configuration and applicability states.
    pub fn new_with_states(
        source: impl Into<String>,
        expected: usize,
        available: usize,
        not_configured: usize,
        not_applicable: usize,
    ) -> Result<Self, RuntimeError> {
        let coverage = Self {
            source: source.into(),
            expected,
            available,
            not_configured,
            not_applicable,
        };
        coverage.validate()?;
        Ok(coverage)
    }

    /// Validates the source identity and counter relationship.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.source.trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "coverage source cannot be blank",
            ));
        }
        if self.available > self.expected {
            return Err(RuntimeError::invalid_model(
                "available coverage cannot exceed expected coverage",
            ));
        }
        if self.not_configured.saturating_add(self.not_applicable)
            > self.expected.saturating_sub(self.available)
        {
            return Err(RuntimeError::invalid_model(
                "configuration and applicability coverage cannot exceed unavailable coverage",
            ));
        }
        Ok(())
    }

    /// Returns the source family label.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the expected observation count.
    pub const fn expected(&self) -> usize {
        self.expected
    }

    /// Returns the available observation count.
    pub const fn available(&self) -> usize {
        self.available
    }

    /// Returns the number of companies without an optional source configured.
    pub const fn not_configured(&self) -> usize {
        self.not_configured
    }

    /// Returns the number of companies for which this source does not apply.
    pub const fn not_applicable(&self) -> usize {
        self.not_applicable
    }

    /// Returns the number of configured companies whose source was unavailable.
    pub const fn unavailable(&self) -> usize {
        self.expected
            .saturating_sub(self.available)
            .saturating_sub(self.not_configured)
            .saturating_sub(self.not_applicable)
    }

    /// Returns an integer percentage, using zero for an empty expectation.
    pub const fn percentage(&self) -> u8 {
        match self.available.checked_mul(100) {
            Some(value) => match value.checked_div(self.expected) {
                Some(value) => value as u8,
                None => 0,
            },
            None => 0,
        }
    }
}

/// Provider-neutral input envelope consumed by later report assembly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RuntimeReportInput {
    as_of: NaiveDate,
    #[serde(default)]
    companies: Vec<CompanyIdentity>,
    facts: Vec<NormalizedFact>,
    source_coverage: Vec<SourceCoverage>,
    #[serde(default)]
    source_failures: Vec<SourceFailure>,
    #[serde(default)]
    research_metrics: ResearchMetrics,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    judgment: Option<super::judgment::JudgmentSnapshot>,
}

impl<'de> Deserialize<'de> for RuntimeReportInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RuntimeReportInputWire {
            as_of: NaiveDate,
            #[serde(default)]
            companies: Vec<CompanyIdentity>,
            facts: Vec<NormalizedFact>,
            source_coverage: Vec<SourceCoverage>,
            #[serde(default)]
            source_failures: Vec<SourceFailure>,
            #[serde(default)]
            research_metrics: ResearchMetrics,
            #[serde(default)]
            judgment: Option<super::judgment::JudgmentSnapshot>,
        }

        let wire = RuntimeReportInputWire::deserialize(deserializer)?;
        let input = Self {
            as_of: wire.as_of,
            companies: wire.companies,
            facts: wire.facts,
            source_coverage: wire.source_coverage,
            source_failures: wire.source_failures,
            research_metrics: wire.research_metrics,
            judgment: wire.judgment,
        };
        input.validate().map_err(D::Error::custom)?;
        Ok(input)
    }
}

impl RuntimeReportInput {
    /// Creates an empty report input for an ISO calendar date.
    pub fn new(as_of: &str) -> Result<Self, RuntimeError> {
        let as_of = NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
            .map_err(|_| RuntimeError::invalid_model("as_of must be YYYY-MM-DD"))?;
        Ok(Self {
            as_of,
            companies: Vec::new(),
            facts: Vec::new(),
            source_coverage: Vec::new(),
            source_failures: Vec::new(),
            research_metrics: ResearchMetrics::new(0, 0, 0, 0, 0),
            judgment: None,
        })
    }

    /// Creates an empty report input from a parsed calendar date.
    pub const fn from_date(as_of: NaiveDate) -> Self {
        Self {
            as_of,
            companies: Vec::new(),
            facts: Vec::new(),
            source_coverage: Vec::new(),
            source_failures: Vec::new(),
            research_metrics: ResearchMetrics::new(0, 0, 0, 0, 0),
            judgment: None,
        }
    }

    /// Adds one display identity without replacing an existing company.
    pub fn add_company(&mut self, company: CompanyIdentity) -> Result<(), RuntimeError> {
        if self
            .companies
            .iter()
            .any(|existing| existing.id() == company.id())
        {
            return Err(RuntimeError::invalid_model(format!(
                "duplicate company identity {}",
                company.id()
            )));
        }
        self.companies.push(company);
        Ok(())
    }

    /// Adds a fact while rejecting a duplicate company/kind pair.
    pub fn add_fact(&mut self, fact: NormalizedFact) -> Result<(), RuntimeError> {
        if self.facts.iter().any(|existing| {
            existing.company_id() == fact.company_id() && existing.kind() == fact.kind()
        }) {
            return Err(RuntimeError::invalid_model(format!(
                "duplicate fact {} for {}",
                fact.kind(),
                fact.company_id()
            )));
        }
        self.facts.push(fact);
        Ok(())
    }

    /// Adds coverage while rejecting a duplicate source family.
    pub fn add_source_coverage(&mut self, coverage: SourceCoverage) -> Result<(), RuntimeError> {
        if self
            .source_coverage
            .iter()
            .any(|existing| existing.source() == coverage.source())
        {
            return Err(RuntimeError::invalid_model(format!(
                "duplicate source coverage {}",
                coverage.source()
            )));
        }
        self.source_coverage.push(coverage);
        Ok(())
    }

    /// Adds one safe source acquisition failure without replacing another.
    pub fn add_source_failure(&mut self, failure: SourceFailure) -> Result<(), RuntimeError> {
        if self.source_failures.iter().any(|existing| {
            existing.source() == failure.source() && existing.company_id() == failure.company_id()
        }) {
            return Err(RuntimeError::invalid_model(format!(
                "duplicate source failure {} for {}",
                failure.source(),
                failure.company_id()
            )));
        }
        self.source_failures.push(failure);
        Ok(())
    }

    /// Validates every nested value and the judgment cutoff before rendering,
    /// persistence, or retry.
    pub fn validate(&self) -> Result<(), RuntimeError> {
        let mut company_ids = std::collections::BTreeSet::new();
        for company in &self.companies {
            company.validate()?;
            if !company_ids.insert(company.id()) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate company identity {}",
                    company.id()
                )));
            }
        }

        let mut fact_ids = std::collections::BTreeSet::new();
        for fact in &self.facts {
            fact.validate()?;
            if !fact_ids.insert((fact.company_id(), fact.kind())) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate fact {} for {}",
                    fact.kind(),
                    fact.company_id()
                )));
            }
        }

        let mut coverage_sources = std::collections::BTreeSet::new();
        for coverage in &self.source_coverage {
            coverage.validate()?;
            if !coverage_sources.insert(coverage.source()) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate source coverage {}",
                    coverage.source()
                )));
            }
        }

        let mut failure_ids = std::collections::BTreeSet::new();
        for failure in &self.source_failures {
            failure.validate()?;
            if !failure_ids.insert((failure.source(), failure.company_id())) {
                return Err(RuntimeError::invalid_model(format!(
                    "duplicate source failure {} for {}",
                    failure.source(),
                    failure.company_id()
                )));
            }
        }

        if let Some(judgment) = &self.judgment {
            judgment.validate()?;
            if judgment.evidence_cutoff() != self.as_of {
                return Err(RuntimeError::invalid_model(
                    "judgment evidence cutoff must match report input as_of",
                ));
            }
        }
        Ok(())
    }

    /// Stores a validated machine/human judgment snapshot without recomputing it.
    pub fn set_judgment(
        &mut self,
        judgment: super::judgment::JudgmentSnapshot,
    ) -> Result<(), RuntimeError> {
        judgment.validate()?;
        if judgment.evidence_cutoff() != self.as_of {
            return Err(RuntimeError::invalid_model(
                "judgment evidence cutoff must match report input as_of",
            ));
        }
        self.judgment = Some(judgment);
        Ok(())
    }

    /// Returns the report as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.as_of
    }

    /// Returns facts in insertion order.
    pub fn facts(&self) -> &[NormalizedFact] {
        &self.facts
    }

    /// Returns company identities in configured order.
    pub fn companies(&self) -> &[CompanyIdentity] {
        &self.companies
    }

    /// Looks up a human-readable company identity.
    pub fn company(&self, id: &str) -> Option<&CompanyIdentity> {
        self.companies.iter().find(|company| company.id() == id)
    }

    /// Returns source coverage in insertion order.
    pub fn source_coverage(&self) -> &[SourceCoverage] {
        &self.source_coverage
    }

    /// Returns safe acquisition failures in insertion order.
    pub fn source_failures(&self) -> &[SourceFailure] {
        &self.source_failures
    }

    /// Replaces acquisition counters before report rendering.
    pub fn set_research_metrics(&mut self, research_metrics: ResearchMetrics) {
        self.research_metrics = research_metrics;
    }

    /// Returns acquisition counters without conflating them with fact status.
    pub const fn research_metrics(&self) -> &ResearchMetrics {
        &self.research_metrics
    }

    /// Returns the validated judgment snapshot when the runtime supplied one.
    pub fn judgment(&self) -> Option<&super::judgment::JudgmentSnapshot> {
        self.judgment.as_ref()
    }
}
