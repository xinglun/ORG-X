//! Deterministic, fact-preserving Rising and Dropped output boundary.
//!
//! This standalone module routes explicit upstream structural evidence changes.
//! It does not infer meaning from raw facts or market movement.

use std::fmt;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank input.
            pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(DomainError::EmptyValue { field: $field });
                }
                Ok(Self(value))
            }

            /// Returns the supplied boundary value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    CompanyReference,
    "company reference",
    "Stable company identity supplied by the upstream Weekly Radar boundary."
);
text_value!(
    StageLabel,
    "stage label",
    "Opaque previous or current Stage value supplied by the upstream boundary."
);
text_value!(
    PeriodId,
    "period id",
    "Identity of the Weekly Radar period containing the change."
);
text_value!(
    EventId,
    "event id",
    "Stable identity supplied for one Rising or Dropped event."
);
text_value!(
    EvidenceId,
    "evidence id",
    "Stable identity for one supporting, counter, or missing proof reference."
);
text_value!(
    Reason,
    "reason",
    "Upstream explanation for the structural evidence change."
);
text_value!(
    NextStep,
    "next step",
    "Upstream next research step retained by the output boundary."
);

/// Deterministic validation and identity failures for this output boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainError {
    /// A required text boundary contained only whitespace.
    EmptyValue { field: &'static str },
    /// A collection already contains the supplied identity.
    DuplicateIdentity { entity: &'static str, id: String },
    /// A proof identity appears in more than one proof category.
    OverlappingEvidence { id: String },
    /// Previous and current states refer to different companies.
    CompanyMismatch { previous: String, current: String },
    /// An event belongs to a different period than its collection.
    PeriodMismatch { expected: String, actual: String },
    /// A company already has an event in the same period.
    CompanyPeriodConflict { period: String, company: String },
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
            Self::OverlappingEvidence { id } => {
                write!(
                    formatter,
                    "evidence identity {id} appears in multiple categories"
                )
            }
            Self::CompanyMismatch { previous, current } => {
                write!(
                    formatter,
                    "previous company {previous} differs from current company {current}"
                )
            }
            Self::PeriodMismatch { expected, actual } => {
                write!(formatter, "expected period {expected}, received {actual}")
            }
            Self::CompanyPeriodConflict { period, company } => {
                write!(
                    formatter,
                    "company {company} already has a change in period {period}"
                )
            }
        }
    }
}

impl std::error::Error for DomainError {}

/// Ordered proof identities retained in one evidence category.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceSet {
    ids: Vec<EvidenceId>,
}

impl EvidenceSet {
    /// Creates an ordered set and rejects duplicate identities.
    pub fn new(ids: Vec<EvidenceId>) -> Result<Self, DomainError> {
        for (index, id) in ids.iter().enumerate() {
            if ids[..index].iter().any(|prior| prior == id) {
                return Err(DomainError::DuplicateIdentity {
                    entity: "evidence",
                    id: id.as_str().to_owned(),
                });
            }
        }
        Ok(Self { ids })
    }

    /// Creates an empty proof collection.
    pub fn empty() -> Self {
        Self { ids: Vec::new() }
    }

    /// Returns proof identities in supplied order.
    pub fn ids(&self) -> &[EvidenceId] {
        &self.ids
    }
}

/// One validated structural evidence change payload.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceDeltaDetails {
    reason: Reason,
    supporting: EvidenceSet,
    counter: EvidenceSet,
    missing: EvidenceSet,
    next: NextStep,
}

impl EvidenceDeltaDetails {
    /// Creates proof details and rejects identities shared across categories.
    pub fn new(
        reason: Reason,
        supporting: EvidenceSet,
        counter: EvidenceSet,
        missing: EvidenceSet,
        next: NextStep,
    ) -> Result<Self, DomainError> {
        for (left, right) in [
            (supporting.ids(), counter.ids()),
            (supporting.ids(), missing.ids()),
            (counter.ids(), missing.ids()),
        ] {
            if let Some(id) = left.iter().find(|candidate| right.contains(candidate)) {
                return Err(DomainError::OverlappingEvidence {
                    id: id.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            reason,
            supporting,
            counter,
            missing,
            next,
        })
    }

    /// Returns the supplied explanation.
    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    /// Returns supporting proof in supplied order.
    pub fn supporting(&self) -> &EvidenceSet {
        &self.supporting
    }

    /// Returns counter proof in supplied order.
    pub fn counter(&self) -> &EvidenceSet {
        &self.counter
    }

    /// Returns missing proof in supplied order.
    pub fn missing(&self) -> &EvidenceSet {
        &self.missing
    }

    /// Returns the supplied next research step.
    pub fn next(&self) -> &NextStep {
        &self.next
    }
}

/// Explicit non-structural changes that must not create a research event.
#[allow(clippy::enum_variant_names)]
// The `Only` suffix is intentional: it makes the no-event contract explicit
// in each stable boundary label rather than hiding it in a generic variant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralChangeKind {
    /// Only a price observation changed.
    PriceOnly,
    /// Only an upstream rank observation changed.
    RankOnly,
    /// Only an upstream score observation changed.
    ScoreOnly,
}

/// Structured upstream evidence delta routed by this boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuralEvidenceDelta {
    /// Structural evidence became stronger and may emit Rising.
    Strengthened(EvidenceDeltaDetails),
    /// Structural evidence became weaker and may emit Dropped.
    Weakened(EvidenceDeltaDetails),
    /// Counter evidence invalidated the prior structural judgment.
    Invalidated(EvidenceDeltaDetails),
    /// No structural evidence change occurred.
    Unchanged,
    /// Only a non-structural observation changed.
    NonStructural(StructuralChangeKind),
}

impl StructuralEvidenceDelta {
    /// Creates an explicit strengthening delta.
    pub fn strengthened(details: EvidenceDeltaDetails) -> Self {
        Self::Strengthened(details)
    }

    /// Creates an explicit weakening delta.
    pub fn weakened(details: EvidenceDeltaDetails) -> Self {
        Self::Weakened(details)
    }

    /// Creates an explicit invalidation delta.
    pub fn invalidated(details: EvidenceDeltaDetails) -> Self {
        Self::Invalidated(details)
    }

    /// Creates an explicit unchanged delta.
    pub fn unchanged() -> Self {
        Self::Unchanged
    }

    /// Creates a non-structural delta that never emits a research event.
    pub fn non_structural(change: StructuralChangeKind) -> Self {
        Self::NonStructural(change)
    }
}

/// The two output sections supported by this Work Item.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    /// Structural evidence strengthened.
    Rising,
    /// Structural evidence weakened or was invalidated.
    Dropped,
}

/// Company state supplied before or after one Weekly Radar period.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchState {
    company: CompanyReference,
    stage: StageLabel,
}

impl ResearchState {
    /// Creates a state without deriving meaning from its Stage label.
    pub fn new(company: CompanyReference, stage: StageLabel) -> Self {
        Self { company, stage }
    }

    /// Returns the supplied company identity.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied Stage label.
    pub fn stage(&self) -> &StageLabel {
        &self.stage
    }
}

/// One fact-preserving Rising or Dropped event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RisingDroppedEvent {
    event_id: EventId,
    period: PeriodId,
    kind: EventKind,
    company: CompanyReference,
    previous_stage: StageLabel,
    current_stage: StageLabel,
    reason: Reason,
    supporting: EvidenceSet,
    counter: EvidenceSet,
    missing: EvidenceSet,
    next: NextStep,
}

impl RisingDroppedEvent {
    /// Returns the supplied event identity.
    pub fn event_id(&self) -> &EventId {
        &self.event_id
    }

    /// Returns the event period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns whether the event is Rising or Dropped.
    pub fn kind(&self) -> EventKind {
        self.kind
    }

    /// Returns the company identity.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied previous Stage.
    pub fn previous_stage(&self) -> &StageLabel {
        &self.previous_stage
    }

    /// Returns the supplied current Stage.
    pub fn current_stage(&self) -> &StageLabel {
        &self.current_stage
    }

    /// Returns the supplied reason.
    pub fn reason(&self) -> &Reason {
        &self.reason
    }

    /// Returns supporting proof in supplied order.
    pub fn supporting(&self) -> &EvidenceSet {
        &self.supporting
    }

    /// Returns counter proof in supplied order.
    pub fn counter(&self) -> &EvidenceSet {
        &self.counter
    }

    /// Returns missing proof in supplied order.
    pub fn missing(&self) -> &EvidenceSet {
        &self.missing
    }

    /// Returns the supplied next research step.
    pub fn next(&self) -> &NextStep {
        &self.next
    }
}

/// Routes an explicit evidence delta into one optional research event.
pub fn derive_event(
    event_id: EventId,
    period: PeriodId,
    previous: ResearchState,
    current: ResearchState,
    delta: StructuralEvidenceDelta,
) -> Result<Option<RisingDroppedEvent>, DomainError> {
    if previous.company != current.company {
        return Err(DomainError::CompanyMismatch {
            previous: previous.company.as_str().to_owned(),
            current: current.company.as_str().to_owned(),
        });
    }

    let kind_and_details = match delta {
        StructuralEvidenceDelta::Strengthened(details) => Some((EventKind::Rising, details)),
        StructuralEvidenceDelta::Weakened(details)
        | StructuralEvidenceDelta::Invalidated(details) => Some((EventKind::Dropped, details)),
        StructuralEvidenceDelta::Unchanged | StructuralEvidenceDelta::NonStructural(_) => None,
    };

    let Some((kind, details)) = kind_and_details else {
        return Ok(None);
    };

    Ok(Some(RisingDroppedEvent {
        event_id,
        period,
        kind,
        company: previous.company,
        previous_stage: previous.stage,
        current_stage: current.stage,
        reason: details.reason,
        supporting: details.supporting,
        counter: details.counter,
        missing: details.missing,
        next: details.next,
    }))
}

/// Ordered Rising and Dropped sections for one Weekly Radar period.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeeklyChangeSet {
    period: PeriodId,
    rising: Vec<RisingDroppedEvent>,
    dropped: Vec<RisingDroppedEvent>,
}

impl WeeklyChangeSet {
    /// Creates an empty change set for one period.
    pub fn new(period: PeriodId) -> Self {
        Self {
            period,
            rising: Vec::new(),
            dropped: Vec::new(),
        }
    }

    /// Adds an event while preserving section order and rejecting conflicts.
    pub fn add(&mut self, event: RisingDroppedEvent) -> Result<(), DomainError> {
        if event.period != self.period {
            return Err(DomainError::PeriodMismatch {
                expected: self.period.as_str().to_owned(),
                actual: event.period.as_str().to_owned(),
            });
        }

        let all_events = self.rising.iter().chain(self.dropped.iter());
        if all_events
            .clone()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(DomainError::DuplicateIdentity {
                entity: "weekly change event",
                id: event.event_id.as_str().to_owned(),
            });
        }
        if self
            .rising
            .iter()
            .chain(self.dropped.iter())
            .any(|existing| existing.company == event.company)
        {
            return Err(DomainError::CompanyPeriodConflict {
                period: self.period.as_str().to_owned(),
                company: event.company.as_str().to_owned(),
            });
        }

        match event.kind {
            EventKind::Rising => self.rising.push(event),
            EventKind::Dropped => self.dropped.push(event),
        }
        Ok(())
    }

    /// Returns the change-set period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns Rising events in insertion order.
    pub fn rising(&self) -> &[RisingDroppedEvent] {
        &self.rising
    }

    /// Returns Dropped events in insertion order.
    pub fn dropped(&self) -> &[RisingDroppedEvent] {
        &self.dropped
    }
}
