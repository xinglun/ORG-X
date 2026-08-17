//! Pure read-only research reporting structures.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, ReportingDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(ReportingDomainError::EmptyValue { field });
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
            pub fn new(value: impl Into<String>) -> Result<Self, ReportingDomainError> {
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
    ResearchCardId,
    "research card id",
    "Stable identity for one read-only research card."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity retained in a research card."
);
text_value!(
    StageLabel,
    "stage label",
    "Stage label supplied by an upstream read model."
);
text_value!(
    Headline,
    "headline",
    "Headline supplied for a research card."
);
text_value!(
    EvidenceSummary,
    "evidence summary",
    "Supporting evidence summary supplied for a research card."
);
text_value!(
    CounterEvidenceSummary,
    "counter evidence summary",
    "Counter evidence summary supplied for a research card."
);
text_value!(
    MissingProofSummary,
    "missing proof summary",
    "Missing proof summary supplied for a research card."
);
text_value!(
    NextStep,
    "next step",
    "Next research step supplied for a research card."
);
text_value!(
    ExecutiveSummary,
    "executive summary",
    "Executive summary supplied for a research packet."
);

/// Validation and collection failures for Reporting read models.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportingDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A card identity already exists in a section.
    DuplicateIdentity { entity: &'static str, id: String },
    /// Top5 cannot contain more than five cards.
    Top5LimitExceeded { limit: usize },
}

impl fmt::Display for ReportingDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
            Self::Top5LimitExceeded { limit } => {
                write!(formatter, "Top5 cannot contain more than {limit} cards")
            }
        }
    }
}

impl std::error::Error for ReportingDomainError {}

/// A research card containing only facts supplied by upstream read models.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchCard {
    id: ResearchCardId,
    company: CompanyReference,
    stage: StageLabel,
    headline: Headline,
    evidence: EvidenceSummary,
    counter_evidence: CounterEvidenceSummary,
    missing_proof: MissingProofSummary,
    next_step: NextStep,
}

impl ResearchCard {
    /// Creates a card without recalculating any supplied field.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ResearchCardId,
        company: CompanyReference,
        stage: impl Into<String>,
        headline: impl Into<String>,
        evidence: impl Into<String>,
        counter_evidence: impl Into<String>,
        missing_proof: impl Into<String>,
        next_step: impl Into<String>,
    ) -> Result<Self, ReportingDomainError> {
        Ok(Self {
            id,
            company,
            stage: StageLabel::new(stage)?,
            headline: Headline::new(headline)?,
            evidence: EvidenceSummary::new(evidence)?,
            counter_evidence: CounterEvidenceSummary::new(counter_evidence)?,
            missing_proof: MissingProofSummary::new(missing_proof)?,
            next_step: NextStep::new(next_step)?,
        })
    }

    /// Returns the card identity.
    pub fn id(&self) -> &ResearchCardId {
        &self.id
    }

    /// Returns the card company.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the supplied Stage label.
    pub fn stage(&self) -> &StageLabel {
        &self.stage
    }

    /// Returns the supplied headline.
    pub fn headline(&self) -> &Headline {
        &self.headline
    }

    /// Returns the supplied evidence summary.
    pub fn evidence(&self) -> &EvidenceSummary {
        &self.evidence
    }

    /// Returns the supplied counter evidence summary.
    pub fn counter_evidence(&self) -> &CounterEvidenceSummary {
        &self.counter_evidence
    }

    /// Returns the supplied missing proof summary.
    pub fn missing_proof(&self) -> &MissingProofSummary {
        &self.missing_proof
    }

    /// Returns the supplied next research step.
    pub fn next_step(&self) -> &NextStep {
        &self.next_step
    }
}

/// An ordered report section with duplicate card protection.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReportSection {
    cards: Vec<ResearchCard>,
}

impl ReportSection {
    /// Creates an empty report section.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a card while preserving insertion order.
    pub fn add(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        if self.cards.iter().any(|item| item.id() == card.id()) {
            return Err(ReportingDomainError::DuplicateIdentity {
                entity: "research card",
                id: card.id().as_str().to_owned(),
            });
        }
        self.cards.push(card);
        Ok(())
    }

    /// Returns cards in supplied order.
    pub fn cards(&self) -> &[ResearchCard] {
        &self.cards
    }
}

/// The Top5 section with an explicit capacity of five cards.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Top5 {
    cards: Vec<ResearchCard>,
}

impl Top5 {
    /// Maximum number of cards in the Top5 section.
    pub const LIMIT: usize = 5;

    /// Creates an empty Top5 section.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a card while enforcing uniqueness and the capacity limit.
    pub fn add(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        if self.cards.iter().any(|item| item.id() == card.id()) {
            return Err(ReportingDomainError::DuplicateIdentity {
                entity: "research card",
                id: card.id().as_str().to_owned(),
            });
        }
        if self.cards.len() == Self::LIMIT {
            return Err(ReportingDomainError::Top5LimitExceeded { limit: Self::LIMIT });
        }
        self.cards.push(card);
        Ok(())
    }

    /// Returns Top5 cards in supplied order.
    pub fn cards(&self) -> &[ResearchCard] {
        &self.cards
    }
}

/// The four-section read-only research packet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResearchPacket {
    executive_summary: ExecutiveSummary,
    top5: Top5,
    rising: ReportSection,
    watch: ReportSection,
    dropped: ReportSection,
}

impl ResearchPacket {
    /// Groups supplied sections without recalculating membership.
    pub fn new(
        executive_summary: impl Into<String>,
        top5: Top5,
        rising: ReportSection,
        watch: ReportSection,
        dropped: ReportSection,
    ) -> Result<Self, ReportingDomainError> {
        Ok(Self {
            executive_summary: ExecutiveSummary::new(executive_summary)?,
            top5,
            rising,
            watch,
            dropped,
        })
    }

    /// Returns the supplied executive summary.
    pub fn executive_summary(&self) -> &ExecutiveSummary {
        &self.executive_summary
    }

    /// Returns the Top5 section.
    pub fn top5(&self) -> &Top5 {
        &self.top5
    }

    /// Returns the Rising section.
    pub fn rising(&self) -> &ReportSection {
        &self.rising
    }

    /// Returns the Watch section.
    pub fn watch(&self) -> &ReportSection {
        &self.watch
    }

    /// Returns the Dropped section.
    pub fn dropped(&self) -> &ReportSection {
        &self.dropped
    }
}

/// Mutable assembly boundary for a packet; it only accepts supplied sections.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ReportingReadModel {
    top5: Top5,
    rising: ReportSection,
    watch: ReportSection,
    dropped: ReportSection,
}

impl ReportingReadModel {
    /// Creates an empty read model.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a supplied Top5 card.
    pub fn add_top5(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        self.top5.add(card)
    }

    /// Adds a supplied Rising card.
    pub fn add_rising(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        self.rising.add(card)
    }

    /// Adds a supplied Watch card.
    pub fn add_watch(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        self.watch.add(card)
    }

    /// Adds a supplied Dropped card.
    pub fn add_dropped(&mut self, card: ResearchCard) -> Result<(), ReportingDomainError> {
        self.dropped.add(card)
    }

    /// Builds a packet from the supplied sections without recomputation.
    pub fn into_packet(
        self,
        executive_summary: impl Into<String>,
    ) -> Result<ResearchPacket, ReportingDomainError> {
        ResearchPacket::new(
            executive_summary,
            self.top5,
            self.rising,
            self.watch,
            self.dropped,
        )
    }
}
