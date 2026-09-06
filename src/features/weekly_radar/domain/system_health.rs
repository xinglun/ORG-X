//! Explicit, provider-agnostic System Health facts for Weekly Radar output.

use std::fmt;

#[cfg(test)]
#[path = "system_health_test.rs"]
mod module_tests;

use crate::shared::domain::text_value as shared_text_value;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        shared_text_value!($name, $field, $description, SystemHealthDomainError);
    };
}

text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity supplied for a degraded-company health fact."
);
text_value!(
    SourceReference,
    "source reference",
    "Opaque source identity supplied for source coverage or extraction failure."
);
text_value!(
    FailureId,
    "failure id",
    "Stable identity supplied for one extraction failure fact."
);
text_value!(
    Reason,
    "reason",
    "Opaque explanation supplied for a degraded company or extraction failure."
);

/// Explicit overall health label supplied by an upstream producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HealthStatus {
    /// The producer supplied a healthy status.
    Healthy,
    /// The producer supplied a degraded status.
    Degraded,
    /// The producer supplied an unavailable status.
    Unavailable,
    /// The producer supplied no more specific status.
    Unknown,
}

/// Explicit freshness label supplied by an upstream producer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    /// The supplied health facts are current.
    Current,
    /// The supplied health facts are aging.
    Aging,
    /// The supplied health facts are stale.
    Stale,
    /// Freshness was not supplied.
    Unknown,
}

/// A validated percentage retained as supplied coverage data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CoveragePercentage(u8);

impl CoveragePercentage {
    /// Creates a percentage in the inclusive 0..=100 range.
    pub fn new(value: u8) -> Result<Self, SystemHealthDomainError> {
        if value > 100 {
            return Err(SystemHealthDomainError::InvalidPercentage { value });
        }
        Ok(Self(value))
    }

    /// Returns the supplied percentage without recalculation.
    pub const fn value(self) -> u8 {
        self.0
    }
}

/// Evidence coverage counts and percentage supplied by an upstream producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceCoverage {
    available: u32,
    expected: u32,
    percentage: CoveragePercentage,
}

impl EvidenceCoverage {
    /// Retains supplied coverage counts and percentage without deriving one from another.
    pub fn new(
        available: u32,
        expected: u32,
        percentage: u8,
    ) -> Result<Self, SystemHealthDomainError> {
        Ok(Self {
            available,
            expected,
            percentage: CoveragePercentage::new(percentage)?,
        })
    }

    /// Returns the supplied available count.
    pub const fn available(&self) -> u32 {
        self.available
    }

    /// Returns the supplied expected count.
    pub const fn expected(&self) -> u32 {
        self.expected
    }

    /// Returns the supplied percentage.
    pub const fn percentage(&self) -> CoveragePercentage {
        self.percentage
    }
}

/// A company and explicit reason supplied for a degraded-company section.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DegradedCompany {
    company: CompanyReference,
    reason: Reason,
}

impl DegradedCompany {
    /// Retains the supplied company and reason without interpreting either value.
    pub fn new(company: CompanyReference, reason: Reason) -> Self {
        Self { company, reason }
    }

    /// Returns the supplied company identity.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied degradation reason.
    pub fn reason(&self) -> &Reason {
        &self.reason
    }
}

/// Source-specific coverage facts supplied by an upstream producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceCoverage {
    source: SourceReference,
    available: u32,
    expected: u32,
    percentage: CoveragePercentage,
}

impl SourceCoverage {
    /// Retains source coverage counts and percentage without deriving a ratio.
    pub fn new(
        source: SourceReference,
        available: u32,
        expected: u32,
        percentage: u8,
    ) -> Result<Self, SystemHealthDomainError> {
        Ok(Self {
            source,
            available,
            expected,
            percentage: CoveragePercentage::new(percentage)?,
        })
    }

    /// Returns the supplied source identity.
    pub fn source(&self) -> &SourceReference {
        &self.source
    }

    /// Returns the supplied available count.
    pub const fn available(&self) -> u32 {
        self.available
    }

    /// Returns the supplied expected count.
    pub const fn expected(&self) -> u32 {
        self.expected
    }

    /// Returns the supplied percentage.
    pub const fn percentage(&self) -> CoveragePercentage {
        self.percentage
    }
}

/// One source extraction failure supplied by an upstream producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionFailure {
    id: FailureId,
    source: SourceReference,
    reason: Reason,
}

impl ExtractionFailure {
    /// Retains the supplied failure identity, source, and reason.
    pub fn new(id: FailureId, source: SourceReference, reason: Reason) -> Self {
        Self { id, source, reason }
    }

    /// Returns the supplied failure identity.
    pub fn id(&self) -> &FailureId {
        &self.id
    }

    /// Returns the supplied source identity.
    pub fn source(&self) -> &SourceReference {
        &self.source
    }

    /// Returns the supplied failure reason.
    pub fn reason(&self) -> &Reason {
        &self.reason
    }
}

/// Validation and collection failures for System Health facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SystemHealthDomainError {
    /// A required text value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A supplied percentage was outside the inclusive 0..=100 range.
    InvalidPercentage { value: u8 },
    /// A collection already contains the supplied identity.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for SystemHealthDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::InvalidPercentage { value } => {
                write!(formatter, "coverage percentage {value} exceeds 100")
            }
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for SystemHealthDomainError {}

/// Ordered System Health facts supplied by an upstream Weekly Radar producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemHealth {
    status: HealthStatus,
    evidence_coverage: EvidenceCoverage,
    degraded_companies: Vec<DegradedCompany>,
    source_coverage: Vec<SourceCoverage>,
    extraction_failures: Vec<ExtractionFailure>,
    freshness: Freshness,
}

impl SystemHealth {
    /// Creates health from explicit status, evidence coverage, and freshness facts.
    pub fn new(
        status: HealthStatus,
        evidence_coverage: EvidenceCoverage,
        freshness: Freshness,
    ) -> Self {
        Self {
            status,
            evidence_coverage,
            degraded_companies: Vec::new(),
            source_coverage: Vec::new(),
            extraction_failures: Vec::new(),
            freshness,
        }
    }

    /// Returns the explicitly supplied health status without recomputation.
    pub const fn status(&self) -> HealthStatus {
        self.status
    }

    /// Returns the supplied aggregate evidence coverage.
    pub fn evidence_coverage(&self) -> &EvidenceCoverage {
        &self.evidence_coverage
    }

    /// Returns degraded companies in supplied order.
    pub fn degraded_companies(&self) -> &[DegradedCompany] {
        &self.degraded_companies
    }

    /// Returns source coverage facts in supplied order.
    pub fn source_coverage(&self) -> &[SourceCoverage] {
        &self.source_coverage
    }

    /// Returns extraction failures in supplied order.
    pub fn extraction_failures(&self) -> &[ExtractionFailure] {
        &self.extraction_failures
    }

    /// Returns the explicitly supplied freshness label without recomputation.
    pub const fn freshness(&self) -> Freshness {
        self.freshness
    }

    /// Appends a degraded company unless its company identity already exists.
    pub fn add_degraded_company(
        &mut self,
        company: DegradedCompany,
    ) -> Result<(), SystemHealthDomainError> {
        if self
            .degraded_companies
            .iter()
            .any(|existing| existing.company() == company.company())
        {
            return Err(SystemHealthDomainError::DuplicateIdentity {
                entity: "degraded company",
                id: company.company().as_str().to_owned(),
            });
        }
        self.degraded_companies.push(company);
        Ok(())
    }

    /// Appends source coverage unless its source identity already exists.
    pub fn add_source_coverage(
        &mut self,
        coverage: SourceCoverage,
    ) -> Result<(), SystemHealthDomainError> {
        if self
            .source_coverage
            .iter()
            .any(|existing| existing.source() == coverage.source())
        {
            return Err(SystemHealthDomainError::DuplicateIdentity {
                entity: "source coverage",
                id: coverage.source().as_str().to_owned(),
            });
        }
        self.source_coverage.push(coverage);
        Ok(())
    }

    /// Appends an extraction failure unless its failure identity already exists.
    pub fn add_extraction_failure(
        &mut self,
        failure: ExtractionFailure,
    ) -> Result<(), SystemHealthDomainError> {
        if self
            .extraction_failures
            .iter()
            .any(|existing| existing.id() == failure.id())
        {
            return Err(SystemHealthDomainError::DuplicateIdentity {
                entity: "extraction failure",
                id: failure.id().as_str().to_owned(),
            });
        }
        self.extraction_failures.push(failure);
        Ok(())
    }
}
