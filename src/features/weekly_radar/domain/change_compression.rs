//! Standalone, fact-preserving Weekly Radar change compression boundary.
//!
//! This module accepts explicit upstream facts and exposes them in fixed
//! sections. It does not infer, rank, score, render, persist, or publish.

use std::{collections::BTreeSet, fmt};

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, ChangeCompressionError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(ChangeCompressionError::EmptyValue { field });
    }
    Ok(value)
}

/// Validation failures for the standalone compression boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeCompressionError {
    /// A required value contained only whitespace.
    EmptyValue { field: &'static str },
    /// An event belongs to a period different from the compression period.
    PeriodMismatch { expected: String, actual: String },
    /// An event identity was repeated across the input sections.
    DuplicateIdentity { id: String },
}

impl fmt::Display for ChangeCompressionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::PeriodMismatch { expected, actual } => {
                write!(
                    formatter,
                    "event period {actual} does not match expected period {expected}"
                )
            }
            Self::DuplicateIdentity { id } => {
                write!(formatter, "duplicate weekly change event identity {id}")
            }
        }
    }
}

impl std::error::Error for ChangeCompressionError {}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank-only input.
            pub fn new(value: impl Into<String>) -> Result<Self, ChangeCompressionError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the supplied value without normalization.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    PeriodId,
    "period",
    "Weekly period supplied to the change compression boundary."
);
text_value!(
    EventId,
    "event id",
    "Stable identity supplied for one weekly change event."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity supplied for one weekly change event."
);
text_value!(
    FactValue,
    "fact",
    "Opaque fact value supplied for one weekly change event."
);

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChangeRecord {
    event_id: EventId,
    period: PeriodId,
    company: CompanyReference,
    fact: FactValue,
}

macro_rules! change_type {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(ChangeRecord);

        impl $name {
            /// Creates the event from explicit upstream facts.
            #[allow(clippy::too_many_arguments)]
            pub fn new(
                event_id: EventId,
                period: PeriodId,
                company: CompanyReference,
                fact: FactValue,
            ) -> Result<Self, ChangeCompressionError> {
                Ok(Self(ChangeRecord {
                    event_id,
                    period,
                    company,
                    fact,
                }))
            }

            /// Returns the supplied event identity.
            pub fn event_id(&self) -> &EventId {
                &self.0.event_id
            }

            /// Returns the supplied period.
            pub fn period(&self) -> &PeriodId {
                &self.0.period
            }

            /// Returns the supplied company identity.
            pub fn company(&self) -> &CompanyReference {
                &self.0.company
            }

            /// Returns the supplied opaque fact value.
            pub fn fact(&self) -> &FactValue {
                &self.0.fact
            }
        }
    };
}

change_type!(
    ImportantStructuralChange,
    "Explicit Important Structural Change fact retained by the compression boundary."
);
change_type!(
    Top5Change,
    "Explicit Top5 Change fact retained by the compression boundary."
);
change_type!(
    StageTransitionChange,
    "Explicit Stage Transition fact retained by the compression boundary."
);
change_type!(
    RisingChange,
    "Explicit Rising fact retained by the compression boundary."
);
change_type!(
    DroppedChange,
    "Explicit Dropped fact retained by the compression boundary."
);

/// Counts used by the stable No Change output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChangeCounts {
    important_structural: usize,
    top5: usize,
    stage_transitions: usize,
    rising: usize,
    dropped: usize,
}

impl ChangeCounts {
    /// Creates the stable all-zero count set.
    pub const fn zero() -> Self {
        Self {
            important_structural: 0,
            top5: 0,
            stage_transitions: 0,
            rising: 0,
            dropped: 0,
        }
    }

    /// Returns the Important Structural Change count.
    pub const fn important_structural(self) -> usize {
        self.important_structural
    }

    /// Returns the Top5 Change count.
    pub const fn top5(self) -> usize {
        self.top5
    }

    /// Returns the Stage Transition count.
    pub const fn stage_transitions(self) -> usize {
        self.stage_transitions
    }

    /// Returns the Rising count.
    pub const fn rising(self) -> usize {
        self.rising
    }

    /// Returns the Dropped count.
    pub const fn dropped(self) -> usize {
        self.dropped
    }
}

/// Stable output emitted when a compression input contains no events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoChange {
    period: PeriodId,
    counts: ChangeCounts,
}

impl NoChange {
    /// Stable label for the no-change boundary.
    pub const LABEL: &'static str = "NO_CHANGE";

    fn new(period: PeriodId) -> Self {
        Self {
            period,
            counts: ChangeCounts::zero(),
        }
    }

    /// Returns the stable no-change label.
    pub const fn label(&self) -> &'static str {
        Self::LABEL
    }

    /// Returns the supplied compression period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns the stable zero counts.
    pub const fn counts(&self) -> ChangeCounts {
        self.counts
    }
}

fn validate_record(
    expected_period: &PeriodId,
    seen: &mut BTreeSet<EventId>,
    record: &ChangeRecord,
) -> Result<(), ChangeCompressionError> {
    if record.period != *expected_period {
        return Err(ChangeCompressionError::PeriodMismatch {
            expected: expected_period.as_str().to_owned(),
            actual: record.period.as_str().to_owned(),
        });
    }
    if !seen.insert(record.event_id.clone()) {
        return Err(ChangeCompressionError::DuplicateIdentity {
            id: record.event_id.as_str().to_owned(),
        });
    }
    Ok(())
}

/// Explicit input collections for one weekly compression period.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeeklyChangeInput {
    period: PeriodId,
    important_structural: Vec<ImportantStructuralChange>,
    top5: Vec<Top5Change>,
    stage_transitions: Vec<StageTransitionChange>,
    rising: Vec<RisingChange>,
    dropped: Vec<DroppedChange>,
}

impl WeeklyChangeInput {
    /// Validates explicit input without sorting, merging, or rewriting it.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        period: PeriodId,
        important_structural: Vec<ImportantStructuralChange>,
        top5: Vec<Top5Change>,
        stage_transitions: Vec<StageTransitionChange>,
        rising: Vec<RisingChange>,
        dropped: Vec<DroppedChange>,
    ) -> Result<Self, ChangeCompressionError> {
        let mut seen = BTreeSet::new();
        for event in &important_structural {
            validate_record(&period, &mut seen, &event.0)?;
        }
        for event in &top5 {
            validate_record(&period, &mut seen, &event.0)?;
        }
        for event in &stage_transitions {
            validate_record(&period, &mut seen, &event.0)?;
        }
        for event in &rising {
            validate_record(&period, &mut seen, &event.0)?;
        }
        for event in &dropped {
            validate_record(&period, &mut seen, &event.0)?;
        }

        Ok(Self {
            period,
            important_structural,
            top5,
            stage_transitions,
            rising,
            dropped,
        })
    }
}

/// Fixed section ordering exposed by a compressed weekly change result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionSection<'a> {
    /// Important Structural Change events in supplied order.
    ImportantStructural(&'a [ImportantStructuralChange]),
    /// Top5 Change events in supplied order.
    Top5(&'a [Top5Change]),
    /// Stage Transition events in supplied order.
    StageTransition(&'a [StageTransitionChange]),
    /// Rising events in supplied order.
    Rising(&'a [RisingChange]),
    /// Dropped events in supplied order.
    Dropped(&'a [DroppedChange]),
    /// The stable No Change output, or its explicit absence when events exist.
    NoChange(Option<&'a NoChange>),
}

/// Ordered Weekly Radar change sections retaining supplied upstream facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeeklyChangeCompression {
    period: PeriodId,
    important_structural: Vec<ImportantStructuralChange>,
    top5: Vec<Top5Change>,
    stage_transitions: Vec<StageTransitionChange>,
    rising: Vec<RisingChange>,
    dropped: Vec<DroppedChange>,
    no_change: Option<NoChange>,
}

impl WeeklyChangeCompression {
    /// Compresses explicit input into fixed sections without domain inference.
    pub fn from_input(input: WeeklyChangeInput) -> Result<Self, ChangeCompressionError> {
        let no_change = if input.important_structural.is_empty()
            && input.top5.is_empty()
            && input.stage_transitions.is_empty()
            && input.rising.is_empty()
            && input.dropped.is_empty()
        {
            Some(NoChange::new(input.period.clone()))
        } else {
            None
        };

        Ok(Self {
            period: input.period,
            important_structural: input.important_structural,
            top5: input.top5,
            stage_transitions: input.stage_transitions,
            rising: input.rising,
            dropped: input.dropped,
            no_change,
        })
    }

    /// Returns the supplied compression period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns Important Structural Change events in input order.
    pub fn important_structural(&self) -> &[ImportantStructuralChange] {
        &self.important_structural
    }

    /// Returns Top5 Change events in input order.
    pub fn top5(&self) -> &[Top5Change] {
        &self.top5
    }

    /// Returns Stage Transition events in input order.
    pub fn stage_transitions(&self) -> &[StageTransitionChange] {
        &self.stage_transitions
    }

    /// Returns Rising events in input order.
    pub fn rising(&self) -> &[RisingChange] {
        &self.rising
    }

    /// Returns Dropped events in input order.
    pub fn dropped(&self) -> &[DroppedChange] {
        &self.dropped
    }

    /// Returns the stable No Change result when all event sections are empty.
    pub fn no_change(&self) -> Option<&NoChange> {
        self.no_change.as_ref()
    }

    /// Returns all six sections in deterministic output order.
    pub fn sections(&self) -> [CompressionSection<'_>; 6] {
        [
            CompressionSection::ImportantStructural(&self.important_structural),
            CompressionSection::Top5(&self.top5),
            CompressionSection::StageTransition(&self.stage_transitions),
            CompressionSection::Rising(&self.rising),
            CompressionSection::Dropped(&self.dropped),
            CompressionSection::NoChange(self.no_change.as_ref()),
        ]
    }
}

#[cfg(test)]
#[path = "change_compression_test.rs"]
mod module_tests;
