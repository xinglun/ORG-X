//! Pure diffusion facts retained for auditable research history.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, DiffusionDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(DiffusionDomainError::EmptyValue { field });
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
            pub fn new(value: impl Into<String>) -> Result<Self, DiffusionDomainError> {
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
    "Opaque company identity retained by a diffusion fact."
);
text_value!(
    DiffusionFactId,
    "diffusion fact id",
    "Stable identity for one diffusion fact."
);
text_value!(
    ImitationScope,
    "imitation scope",
    "Scope of the competitor imitation observation."
);
text_value!(
    ObservationDate,
    "observation date",
    "Opaque date retained with an observation."
);
text_value!(
    JobTaxonomyLabel,
    "job taxonomy label",
    "Job label retained with a taxonomy change."
);
text_value!(
    TaxonomyChange,
    "taxonomy change",
    "Description of a supplied job taxonomy change."
);
text_value!(
    BenchmarkName,
    "benchmark name",
    "Name of a benchmark observation."
);
text_value!(
    BenchmarkComparison,
    "benchmark comparison",
    "Opaque comparison retained with a benchmark observation."
);
text_value!(
    BenchmarkPeriod,
    "benchmark period",
    "Period retained with a benchmark observation."
);
text_value!(
    IndustryName,
    "industry name",
    "Industry label retained with an industry diffusion fact."
);
text_value!(
    DiffusionDescription,
    "diffusion description",
    "Description retained with a diffusion fact."
);

/// Validation and collection failures for diffusion facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiffusionDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A fact identity already exists in the profile.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for DiffusionDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for DiffusionDomainError {}

/// Categories of diffusion observations retained without scoring or inference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffusionSignalKind {
    /// A workflow redesign was observed.
    WorkflowRedesign,
    /// A new or changed job taxonomy was observed.
    JobTaxonomy,
    /// A productivity benchmark change was observed.
    ProductivityBenchmark,
    /// Advisory or consulting adoption was observed.
    AdvisoryAdoption,
    /// Capital reallocation related to the production pattern was observed.
    CapitalReallocation,
}

impl DiffusionSignalKind {
    /// Returns the stable label for the observation category.
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkflowRedesign => "WORKFLOW_REDESIGN",
            Self::JobTaxonomy => "JOB_TAXONOMY",
            Self::ProductivityBenchmark => "PRODUCTIVITY_BENCHMARK",
            Self::AdvisoryAdoption => "ADVISORY_ADOPTION",
            Self::CapitalReallocation => "CAPITAL_REALLOCATION",
        }
    }
}

/// A competitor imitation observation with both company identities retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompetitorImitation {
    id: DiffusionFactId,
    subject_company: CompanyReference,
    imitator_company: CompanyReference,
    scope: ImitationScope,
    observed_at: ObservationDate,
}

impl CompetitorImitation {
    /// Creates an imitation fact without deciding whether it proves adoption.
    pub fn new(
        id: DiffusionFactId,
        subject_company: CompanyReference,
        imitator_company: CompanyReference,
        scope: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            id,
            subject_company,
            imitator_company,
            scope: ImitationScope::new(scope)?,
            observed_at: ObservationDate::new(observed_at)?,
        })
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &DiffusionFactId {
        &self.id
    }

    /// Returns the company whose pattern is being observed.
    pub fn subject_company(&self) -> &CompanyReference {
        &self.subject_company
    }

    /// Returns the company recorded as the imitator.
    pub fn imitator_company(&self) -> &CompanyReference {
        &self.imitator_company
    }

    /// Returns the supplied imitation scope.
    pub fn scope(&self) -> &ImitationScope {
        &self.scope
    }

    /// Returns the supplied observation date.
    pub fn observed_at(&self) -> &ObservationDate {
        &self.observed_at
    }
}

/// A changed job taxonomy fact retained with its company and role label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobTaxonomyChange {
    id: DiffusionFactId,
    company: CompanyReference,
    role_label: JobTaxonomyLabel,
    change: TaxonomyChange,
    observed_at: ObservationDate,
}

impl JobTaxonomyChange {
    /// Creates a job taxonomy fact without deriving labor-market meaning.
    pub fn new(
        id: DiffusionFactId,
        company: CompanyReference,
        role_label: impl Into<String>,
        change: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            id,
            company,
            role_label: JobTaxonomyLabel::new(role_label)?,
            change: TaxonomyChange::new(change)?,
            observed_at: ObservationDate::new(observed_at)?,
        })
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &DiffusionFactId {
        &self.id
    }

    /// Returns the company attached to the taxonomy fact.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied role label.
    pub fn role_label(&self) -> &JobTaxonomyLabel {
        &self.role_label
    }

    /// Returns the supplied taxonomy change description.
    pub fn change(&self) -> &TaxonomyChange {
        &self.change
    }

    /// Returns the supplied observation date.
    pub fn observed_at(&self) -> &ObservationDate {
        &self.observed_at
    }
}

/// A benchmark comparison retained without calculating a ranking value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkObservation {
    id: DiffusionFactId,
    company: CompanyReference,
    benchmark: BenchmarkName,
    comparison: BenchmarkComparison,
    period: BenchmarkPeriod,
    observed_at: ObservationDate,
}

impl BenchmarkObservation {
    /// Creates a benchmark fact while preserving the supplied comparison.
    pub fn new(
        id: DiffusionFactId,
        company: CompanyReference,
        benchmark: impl Into<String>,
        comparison: impl Into<String>,
        period: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            id,
            company,
            benchmark: BenchmarkName::new(benchmark)?,
            comparison: BenchmarkComparison::new(comparison)?,
            period: BenchmarkPeriod::new(period)?,
            observed_at: ObservationDate::new(observed_at)?,
        })
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &DiffusionFactId {
        &self.id
    }

    /// Returns the benchmark subject company.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the benchmark name.
    pub fn benchmark(&self) -> &BenchmarkName {
        &self.benchmark
    }

    /// Returns the supplied comparison.
    pub fn comparison(&self) -> &BenchmarkComparison {
        &self.comparison
    }

    /// Returns the supplied benchmark period.
    pub fn period(&self) -> &BenchmarkPeriod {
        &self.period
    }

    /// Returns the supplied observation date.
    pub fn observed_at(&self) -> &ObservationDate {
        &self.observed_at
    }
}

/// An industry-level diffusion fact retained without claiming industry-wide adoption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndustryDiffusion {
    id: DiffusionFactId,
    industry: IndustryName,
    description: DiffusionDescription,
    observed_at: ObservationDate,
}

impl IndustryDiffusion {
    /// Creates an industry diffusion observation without aggregation.
    pub fn new(
        id: DiffusionFactId,
        industry: impl Into<String>,
        description: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            id,
            industry: IndustryName::new(industry)?,
            description: DiffusionDescription::new(description)?,
            observed_at: ObservationDate::new(observed_at)?,
        })
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &DiffusionFactId {
        &self.id
    }

    /// Returns the industry label.
    pub fn industry(&self) -> &IndustryName {
        &self.industry
    }

    /// Returns the supplied industry observation.
    pub fn description(&self) -> &DiffusionDescription {
        &self.description
    }

    /// Returns the supplied observation date.
    pub fn observed_at(&self) -> &ObservationDate {
        &self.observed_at
    }
}

/// A categorized diffusion signal retained without converting it to a score.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffusionSignal {
    id: DiffusionFactId,
    kind: DiffusionSignalKind,
    description: DiffusionDescription,
    observed_at: ObservationDate,
}

impl DiffusionSignal {
    /// Creates a categorized diffusion signal.
    pub fn new(
        id: DiffusionFactId,
        kind: DiffusionSignalKind,
        description: impl Into<String>,
        observed_at: impl Into<String>,
    ) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            id,
            kind,
            description: DiffusionDescription::new(description)?,
            observed_at: ObservationDate::new(observed_at)?,
        })
    }

    /// Returns the fact identity.
    pub fn id(&self) -> &DiffusionFactId {
        &self.id
    }

    /// Returns the supplied category.
    pub fn kind(&self) -> DiffusionSignalKind {
        self.kind
    }

    /// Returns the supplied signal description.
    pub fn description(&self) -> &DiffusionDescription {
        &self.description
    }

    /// Returns the supplied observation date.
    pub fn observed_at(&self) -> &ObservationDate {
        &self.observed_at
    }
}

/// Ordered diffusion facts for one company.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffusionProfile {
    company: CompanyReference,
    competitor_imitations: Vec<CompetitorImitation>,
    job_taxonomy_changes: Vec<JobTaxonomyChange>,
    benchmarks: Vec<BenchmarkObservation>,
    industry_diffusions: Vec<IndustryDiffusion>,
    signals: Vec<DiffusionSignal>,
}

impl DiffusionProfile {
    /// Creates an empty profile for a company.
    pub fn new(company: CompanyReference) -> Result<Self, DiffusionDomainError> {
        Ok(Self {
            company,
            competitor_imitations: Vec::new(),
            job_taxonomy_changes: Vec::new(),
            benchmarks: Vec::new(),
            industry_diffusions: Vec::new(),
            signals: Vec::new(),
        })
    }

    /// Returns the profile company.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Adds an imitation fact while rejecting duplicate identity.
    pub fn add_competitor_imitation(
        &mut self,
        fact: CompetitorImitation,
    ) -> Result<(), DiffusionDomainError> {
        self.ensure_unique(fact.id())?;
        self.competitor_imitations.push(fact);
        Ok(())
    }

    /// Adds a job taxonomy fact while rejecting duplicate identity.
    pub fn add_job_taxonomy_change(
        &mut self,
        fact: JobTaxonomyChange,
    ) -> Result<(), DiffusionDomainError> {
        self.ensure_unique(fact.id())?;
        self.job_taxonomy_changes.push(fact);
        Ok(())
    }

    /// Adds a benchmark fact while rejecting duplicate identity.
    pub fn add_benchmark(
        &mut self,
        fact: BenchmarkObservation,
    ) -> Result<(), DiffusionDomainError> {
        self.ensure_unique(fact.id())?;
        self.benchmarks.push(fact);
        Ok(())
    }

    /// Adds an industry diffusion fact while rejecting duplicate identity.
    pub fn add_industry_diffusion(
        &mut self,
        fact: IndustryDiffusion,
    ) -> Result<(), DiffusionDomainError> {
        self.ensure_unique(fact.id())?;
        self.industry_diffusions.push(fact);
        Ok(())
    }

    /// Adds a categorized signal while rejecting duplicate identity.
    pub fn add_signal(&mut self, fact: DiffusionSignal) -> Result<(), DiffusionDomainError> {
        self.ensure_unique(fact.id())?;
        self.signals.push(fact);
        Ok(())
    }

    /// Returns imitation facts in insertion order.
    pub fn competitor_imitations(&self) -> &[CompetitorImitation] {
        &self.competitor_imitations
    }

    /// Returns job taxonomy facts in insertion order.
    pub fn job_taxonomy_changes(&self) -> &[JobTaxonomyChange] {
        &self.job_taxonomy_changes
    }

    /// Returns benchmark facts in insertion order.
    pub fn benchmarks(&self) -> &[BenchmarkObservation] {
        &self.benchmarks
    }

    /// Returns industry diffusion facts in insertion order.
    pub fn industry_diffusions(&self) -> &[IndustryDiffusion] {
        &self.industry_diffusions
    }

    /// Returns categorized signals in insertion order.
    pub fn signals(&self) -> &[DiffusionSignal] {
        &self.signals
    }

    fn ensure_unique(&self, id: &DiffusionFactId) -> Result<(), DiffusionDomainError> {
        let duplicate = self
            .competitor_imitations
            .iter()
            .any(|fact| fact.id() == id)
            || self.job_taxonomy_changes.iter().any(|fact| fact.id() == id)
            || self.benchmarks.iter().any(|fact| fact.id() == id)
            || self.industry_diffusions.iter().any(|fact| fact.id() == id)
            || self.signals.iter().any(|fact| fact.id() == id);
        if duplicate {
            return Err(DiffusionDomainError::DuplicateIdentity {
                entity: "diffusion fact",
                id: id.as_str().to_owned(),
            });
        }
        Ok(())
    }
}
