use std::fmt;

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, Top5DomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(Top5DomainError::EmptyValue { field });
    }
    Ok(value)
}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank-only input without trimming accepted input.
            pub fn new(value: impl Into<String>) -> Result<Self, Top5DomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the supplied value exactly as accepted.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    CandidateId,
    "candidate",
    "Stable identity of one supplied Top5 candidate entry."
);
text_value!(
    Company,
    "company",
    "Supplied company value retained by a Top5 entry."
);
text_value!(
    Stage,
    "stage",
    "Supplied Stage value retained by a Top5 entry."
);
text_value!(
    Direction,
    "direction",
    "Supplied Direction value retained by a Top5 entry."
);
text_value!(
    Confidence,
    "confidence",
    "Supplied Confidence value retained by a Top5 entry."
);
text_value!(
    KeyChange,
    "key_change",
    "Supplied Key Change value retained by a Top5 entry."
);
text_value!(
    NextStep,
    "next",
    "Supplied next-step value retained by a Top5 entry."
);

/// Validation failures for the Top5 read-only boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Top5DomainError {
    /// A required fact contained only whitespace.
    EmptyValue { field: &'static str },
    /// The candidate identity already exists in the read model.
    DuplicateIdentity { entity: &'static str, id: String },
    /// An attempt was made to add more than five entries.
    Top5LimitExceeded { limit: usize },
}

impl fmt::Display for Top5DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
            Self::Top5LimitExceeded { limit } => {
                write!(formatter, "Top5 cannot contain more than {limit} entries")
            }
        }
    }
}

impl std::error::Error for Top5DomainError {}

/// One Top5 entry made entirely from supplied upstream facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Top5Entry {
    candidate: CandidateId,
    company: Company,
    stage: Stage,
    direction: Direction,
    confidence: Confidence,
    key_change: KeyChange,
    next: NextStep,
}

impl Top5Entry {
    /// Creates an entry without deriving or changing any supplied fact.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        candidate: CandidateId,
        company: Company,
        stage: Stage,
        direction: Direction,
        confidence: Confidence,
        key_change: KeyChange,
        next: NextStep,
    ) -> Result<Self, Top5DomainError> {
        Ok(Self {
            candidate,
            company,
            stage,
            direction,
            confidence,
            key_change,
            next,
        })
    }

    /// Returns the stable candidate identity.
    pub fn candidate(&self) -> &CandidateId {
        &self.candidate
    }

    /// Returns the supplied company value.
    pub fn company(&self) -> &Company {
        &self.company
    }

    /// Returns the supplied Stage value.
    pub fn stage(&self) -> &Stage {
        &self.stage
    }

    /// Returns the supplied Direction value.
    pub fn direction(&self) -> &Direction {
        &self.direction
    }

    /// Returns the supplied Confidence value.
    pub fn confidence(&self) -> &Confidence {
        &self.confidence
    }

    /// Returns the supplied Key Change value.
    pub fn key_change(&self) -> &KeyChange {
        &self.key_change
    }

    /// Returns the supplied next-step value.
    pub fn next(&self) -> &NextStep {
        &self.next
    }
}

/// Ordered Top5 entries supplied by an upstream decision boundary.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Top5WeeklyReadModel {
    entries: Vec<Top5Entry>,
}

impl Top5WeeklyReadModel {
    /// Maximum number of entries in one Top5 read model.
    pub const LIMIT: usize = 5;

    /// Creates an empty read model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a read model from supplied entries in their original order.
    pub fn from_entries(
        entries: impl IntoIterator<Item = Top5Entry>,
    ) -> Result<Self, Top5DomainError> {
        let mut model = Self::new();
        for entry in entries {
            model.add(entry)?;
        }
        Ok(model)
    }

    /// Appends one supplied entry after identity and capacity validation.
    pub fn add(&mut self, entry: Top5Entry) -> Result<(), Top5DomainError> {
        if self
            .entries
            .iter()
            .any(|existing| existing.candidate() == entry.candidate())
        {
            return Err(Top5DomainError::DuplicateIdentity {
                entity: "top5 candidate",
                id: entry.candidate().as_str().to_owned(),
            });
        }
        if self.entries.len() >= Self::LIMIT {
            return Err(Top5DomainError::Top5LimitExceeded { limit: Self::LIMIT });
        }
        self.entries.push(entry);
        Ok(())
    }

    /// Returns entries in the order supplied by the caller.
    pub fn entries(&self) -> &[Top5Entry] {
        &self.entries
    }

    /// Returns the number of retained entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no entries were supplied.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
#[path = "top5_weekly_read_model_test.rs"]
mod module_tests;
