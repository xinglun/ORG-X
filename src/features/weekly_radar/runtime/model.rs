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

/// One provider-neutral fact ready for deterministic report assembly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct NormalizedFact {
    company_id: String,
    kind: String,
    value: Option<String>,
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
            status: FactStatus,
            confidence: Confidence,
            provenance: Provenance,
        }

        let wire = NormalizedFactWire::deserialize(deserializer)?;
        Self::build(
            wire.company_id,
            wire.kind,
            wire.value,
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
        Self::build(
            company_id,
            kind,
            Some(value.into()),
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
        Self::build(company_id, kind, None, status, confidence, provenance)
    }

    fn build(
        company_id: impl Into<String>,
        kind: impl Into<String>,
        value: Option<String>,
        status: FactStatus,
        confidence: Confidence,
        provenance: Provenance,
    ) -> Result<Self, RuntimeError> {
        let value = match status {
            FactStatus::Known => value,
            FactStatus::Unknown | FactStatus::Unavailable | FactStatus::Unconfirmed => None,
        };
        let fact = Self {
            company_id: company_id.into(),
            kind: kind.into(),
            value,
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

/// Coverage counters for one configured source family.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceCoverage {
    source: String,
    expected: usize,
    available: usize,
}

impl SourceCoverage {
    /// Creates coverage counters and rejects impossible values.
    pub fn new(
        source: impl Into<String>,
        expected: usize,
        available: usize,
    ) -> Result<Self, RuntimeError> {
        let coverage = Self {
            source: source.into(),
            expected,
            available,
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

    /// Returns an integer percentage, using zero for an empty expectation.
    pub const fn percentage(&self) -> u8 {
        if self.expected == 0 {
            0
        } else {
            ((self.available * 100) / self.expected) as u8
        }
    }
}

/// Provider-neutral input envelope consumed by later report assembly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RuntimeReportInput {
    as_of: NaiveDate,
    facts: Vec<NormalizedFact>,
    source_coverage: Vec<SourceCoverage>,
}

impl RuntimeReportInput {
    /// Creates an empty report input for an ISO calendar date.
    pub fn new(as_of: &str) -> Result<Self, RuntimeError> {
        let as_of = NaiveDate::parse_from_str(as_of, "%Y-%m-%d")
            .map_err(|_| RuntimeError::invalid_model("as_of must be YYYY-MM-DD"))?;
        Ok(Self {
            as_of,
            facts: Vec::new(),
            source_coverage: Vec::new(),
        })
    }

    /// Creates an empty report input from a parsed calendar date.
    pub const fn from_date(as_of: NaiveDate) -> Self {
        Self {
            as_of,
            facts: Vec::new(),
            source_coverage: Vec::new(),
        }
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

    /// Returns the report as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.as_of
    }

    /// Returns facts in insertion order.
    pub fn facts(&self) -> &[NormalizedFact] {
        &self.facts
    }

    /// Returns source coverage in insertion order.
    pub fn source_coverage(&self) -> &[SourceCoverage] {
        &self.source_coverage
    }
}
