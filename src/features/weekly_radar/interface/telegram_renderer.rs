//! Deterministic, provider-agnostic Markdown view for a Telegram summary.
//!
//! This boundary formats explicit upstream facts only. It does not calculate
//! Weekly Radar domain values, publish messages, access credentials, or split
//! output into multiple messages.

use std::{collections::BTreeSet, fmt};

fn non_empty(field: &'static str, value: impl Into<String>) -> Result<String, TelegramRenderError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(TelegramRenderError::EmptyValue { field });
    }
    Ok(value)
}

/// Validation and atomic rendering failures for the Telegram view boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TelegramRenderError {
    /// A required identity, label, or Markdown fragment was blank.
    EmptyValue { field: &'static str },
    /// A section item identity was repeated across the explicit input.
    DuplicateIdentity { id: String },
    /// An explicit No Change period differs from the input period.
    PeriodMismatch { expected: String, actual: String },
    /// No explicit change collection or No Change fact was supplied.
    MissingChangeState,
    /// Explicit No Change was supplied with at least one change item.
    ConflictingNoChange,
    /// A caller-supplied limit was zero.
    InvalidLimit { field: &'static str },
    /// A section contains more items than its caller-supplied limit.
    ItemLimitExceeded {
        section: &'static str,
        limit: usize,
        actual: usize,
    },
    /// The combined company-card count exceeds its caller-supplied limit.
    CompanyCardLimitExceeded { limit: usize, actual: usize },
    /// The complete Markdown message exceeds its caller-supplied character limit.
    MessageTooLong { limit: usize, actual: usize },
    /// The complete Markdown message exceeds its caller-supplied line limit.
    LineLimitExceeded { limit: usize, actual: usize },
}

impl fmt::Display for TelegramRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { id } => {
                write!(formatter, "duplicate Telegram summary identity {id}")
            }
            Self::PeriodMismatch { expected, actual } => {
                write!(formatter, "expected period {expected}, received {actual}")
            }
            Self::MissingChangeState => {
                write!(
                    formatter,
                    "summary requires explicit change sections or No Change"
                )
            }
            Self::ConflictingNoChange => {
                write!(formatter, "No Change cannot coexist with change sections")
            }
            Self::InvalidLimit { field } => write!(formatter, "{field} must be greater than zero"),
            Self::ItemLimitExceeded {
                section,
                limit,
                actual,
            } => write!(
                formatter,
                "{section} contains {actual} items; limit is {limit}"
            ),
            Self::CompanyCardLimitExceeded { limit, actual } => write!(
                formatter,
                "summary contains {actual} company cards; limit is {limit}"
            ),
            Self::MessageTooLong { limit, actual } => write!(
                formatter,
                "summary contains {actual} characters; limit is {limit}"
            ),
            Self::LineLimitExceeded { limit, actual } => write!(
                formatter,
                "summary contains {actual} lines; limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for TelegramRenderError {}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects whitespace-only input.
            pub fn new(value: impl Into<String>) -> Result<Self, TelegramRenderError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the exact value supplied at this boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    PeriodId,
    "period",
    "Weekly Radar period supplied to the Telegram summary boundary."
);
text_value!(
    ItemId,
    "item id",
    "Stable identity supplied for one Telegram summary item or company card."
);
text_value!(
    CompanyReference,
    "company reference",
    "Opaque company identity supplied for a Telegram company card."
);
text_value!(
    MarkdownFragment,
    "markdown fragment",
    "Complete Markdown supplied for one Telegram summary item or card."
);
text_value!(
    HealthStatusLabel,
    "health status",
    "System Health status supplied by an upstream producer."
);

/// One explicit Important Structural Change or Stage Transition item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SummaryItem {
    id: ItemId,
    markdown: MarkdownFragment,
}

impl SummaryItem {
    /// Creates an item without interpreting or shortening its Markdown.
    pub fn new(id: ItemId, markdown: impl Into<String>) -> Result<Self, TelegramRenderError> {
        Ok(Self {
            id,
            markdown: MarkdownFragment::new(markdown)?,
        })
    }

    /// Returns the supplied item identity.
    pub fn id(&self) -> &ItemId {
        &self.id
    }

    /// Returns the complete supplied Markdown fragment.
    pub fn markdown(&self) -> &MarkdownFragment {
        &self.markdown
    }
}

/// One explicit company card for Top5, Threshold Distance, Rising, or Dropped.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompanyCard {
    id: ItemId,
    company: CompanyReference,
    markdown: MarkdownFragment,
}

impl CompanyCard {
    /// Creates a card without interpreting or shortening its Markdown body.
    pub fn new(
        id: ItemId,
        company: CompanyReference,
        markdown: impl Into<String>,
    ) -> Result<Self, TelegramRenderError> {
        Ok(Self {
            id,
            company,
            markdown: MarkdownFragment::new(markdown)?,
        })
    }

    /// Returns the supplied card identity.
    pub fn id(&self) -> &ItemId {
        &self.id
    }

    /// Returns the supplied company identity.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns the complete supplied Markdown card body.
    pub fn markdown(&self) -> &MarkdownFragment {
        &self.markdown
    }
}

/// Explicit System Health status and Markdown details for the short view.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemHealthSummary {
    status: HealthStatusLabel,
    markdown: MarkdownFragment,
}

impl SystemHealthSummary {
    /// Creates a health summary without deriving status from its details.
    pub fn new(
        status: impl Into<String>,
        markdown: impl Into<String>,
    ) -> Result<Self, TelegramRenderError> {
        Ok(Self {
            status: HealthStatusLabel::new(status)?,
            markdown: MarkdownFragment::new(markdown)?,
        })
    }

    /// Returns the explicitly supplied status label.
    pub fn status(&self) -> &HealthStatusLabel {
        &self.status
    }

    /// Returns the complete supplied Markdown details.
    pub fn markdown(&self) -> &MarkdownFragment {
        &self.markdown
    }
}

/// Explicit No Change fact supplied by an upstream change compressor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NoChangeSummary {
    period: PeriodId,
    markdown: MarkdownFragment,
}

impl NoChangeSummary {
    /// Stable label for an explicit No Change section.
    pub const LABEL: &'static str = "NO_CHANGE";

    /// Creates No Change without inferring it from empty collections.
    pub fn new(period: PeriodId, markdown: impl Into<String>) -> Result<Self, TelegramRenderError> {
        Ok(Self {
            period,
            markdown: MarkdownFragment::new(markdown)?,
        })
    }

    /// Returns the stable No Change label.
    pub const fn label(&self) -> &'static str {
        Self::LABEL
    }

    /// Returns the supplied No Change period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns the complete supplied No Change statement.
    pub fn markdown(&self) -> &MarkdownFragment {
        &self.markdown
    }
}

fn register_item(seen: &mut BTreeSet<ItemId>, item: &ItemId) -> Result<(), TelegramRenderError> {
    if seen.insert(item.clone()) {
        Ok(())
    } else {
        Err(TelegramRenderError::DuplicateIdentity {
            id: item.as_str().to_owned(),
        })
    }
}

/// Explicit sections supplied for one Telegram summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelegramSummaryInput {
    period: PeriodId,
    important_structural: Vec<SummaryItem>,
    stage_transitions: Vec<SummaryItem>,
    top5: Vec<CompanyCard>,
    threshold_distances: Vec<CompanyCard>,
    rising: Vec<CompanyCard>,
    dropped: Vec<CompanyCard>,
    system_health: Option<SystemHealthSummary>,
    no_change: Option<NoChangeSummary>,
}

impl TelegramSummaryInput {
    /// Groups explicit facts without sorting, merging, or calculating them.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        period: PeriodId,
        important_structural: Vec<SummaryItem>,
        stage_transitions: Vec<SummaryItem>,
        top5: Vec<CompanyCard>,
        threshold_distances: Vec<CompanyCard>,
        rising: Vec<CompanyCard>,
        dropped: Vec<CompanyCard>,
        system_health: Option<SystemHealthSummary>,
        no_change: Option<NoChangeSummary>,
    ) -> Result<Self, TelegramRenderError> {
        let has_change_items = !important_structural.is_empty()
            || !stage_transitions.is_empty()
            || !top5.is_empty()
            || !threshold_distances.is_empty()
            || !rising.is_empty()
            || !dropped.is_empty();

        match (has_change_items, no_change.is_some()) {
            (false, false) => return Err(TelegramRenderError::MissingChangeState),
            (true, true) => return Err(TelegramRenderError::ConflictingNoChange),
            _ => {}
        }

        if let Some(no_change) = &no_change {
            if no_change.period != period {
                return Err(TelegramRenderError::PeriodMismatch {
                    expected: period.as_str().to_owned(),
                    actual: no_change.period.as_str().to_owned(),
                });
            }
        }

        let mut seen = BTreeSet::new();
        for item in &important_structural {
            register_item(&mut seen, item.id())?;
        }
        for item in &stage_transitions {
            register_item(&mut seen, item.id())?;
        }
        for item in &top5 {
            register_item(&mut seen, item.id())?;
        }
        for item in &threshold_distances {
            register_item(&mut seen, item.id())?;
        }
        for item in &rising {
            register_item(&mut seen, item.id())?;
        }
        for item in &dropped {
            register_item(&mut seen, item.id())?;
        }

        Ok(Self {
            period,
            important_structural,
            stage_transitions,
            top5,
            threshold_distances,
            rising,
            dropped,
            system_health,
            no_change,
        })
    }

    /// Returns the supplied period.
    pub fn period(&self) -> &PeriodId {
        &self.period
    }

    /// Returns Important Structural Change items in supplied order.
    pub fn important_structural(&self) -> &[SummaryItem] {
        &self.important_structural
    }

    /// Returns Stage Transition items in supplied order.
    pub fn stage_transitions(&self) -> &[SummaryItem] {
        &self.stage_transitions
    }

    /// Returns Top5 company cards in supplied order.
    pub fn top5(&self) -> &[CompanyCard] {
        &self.top5
    }

    /// Returns Threshold Distance company cards in supplied order.
    pub fn threshold_distances(&self) -> &[CompanyCard] {
        &self.threshold_distances
    }

    /// Returns Rising company cards in supplied order.
    pub fn rising(&self) -> &[CompanyCard] {
        &self.rising
    }

    /// Returns Dropped company cards in supplied order.
    pub fn dropped(&self) -> &[CompanyCard] {
        &self.dropped
    }

    /// Returns the optional explicit System Health summary.
    pub fn system_health(&self) -> Option<&SystemHealthSummary> {
        self.system_health.as_ref()
    }

    /// Returns the optional explicit No Change summary.
    pub fn no_change(&self) -> Option<&NoChangeSummary> {
        self.no_change.as_ref()
    }
}

/// Caller-supplied atomic constraints for one Telegram Markdown message.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TelegramRenderLimits {
    max_characters: usize,
    max_lines: usize,
    max_items_per_section: usize,
    max_company_cards: usize,
}

impl TelegramRenderLimits {
    /// Creates limits; zero would make every non-empty summary impossible.
    pub fn new(
        max_characters: usize,
        max_lines: usize,
        max_items_per_section: usize,
        max_company_cards: usize,
    ) -> Result<Self, TelegramRenderError> {
        for (field, value) in [
            ("max characters", max_characters),
            ("max lines", max_lines),
            ("max items per section", max_items_per_section),
            ("max company cards", max_company_cards),
        ] {
            if value == 0 {
                return Err(TelegramRenderError::InvalidLimit { field });
            }
        }
        Ok(Self {
            max_characters,
            max_lines,
            max_items_per_section,
            max_company_cards,
        })
    }
}

/// Complete Markdown output with measurements for later delivery adapters.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelegramMessage {
    markdown: String,
    character_count: usize,
    line_count: usize,
    company_card_count: usize,
}

impl TelegramMessage {
    /// Returns the complete Markdown message without truncation.
    pub fn as_str(&self) -> &str {
        &self.markdown
    }

    /// Returns the Unicode scalar character count of the message.
    pub const fn character_count(&self) -> usize {
        self.character_count
    }

    /// Returns the line count of the complete message.
    pub const fn line_count(&self) -> usize {
        self.line_count
    }

    /// Returns the number of complete company cards included in the message.
    pub const fn company_card_count(&self) -> usize {
        self.company_card_count
    }
}

fn item_block(title: &'static str, items: &[SummaryItem]) -> Option<String> {
    if items.is_empty() {
        return None;
    }
    let mut block = format!("## {title}");
    for item in items {
        block.push_str("\n- ");
        block.push_str(item.markdown.as_str());
    }
    Some(block)
}

fn card_block(title: &'static str, cards: &[CompanyCard]) -> Option<String> {
    if cards.is_empty() {
        return None;
    }
    let mut block = format!("## {title}");
    for card in cards {
        block.push_str("\n- **");
        block.push_str(card.company.as_str());
        block.push_str("** — ");
        block.push_str(card.markdown.as_str());
    }
    Some(block)
}

/// Stateless renderer for one explicit Weekly Radar Telegram summary.
pub struct TelegramRenderer;

impl TelegramRenderer {
    /// Renders complete Markdown blocks in fixed priority order.
    pub fn render(
        input: &TelegramSummaryInput,
        limits: TelegramRenderLimits,
    ) -> Result<TelegramMessage, TelegramRenderError> {
        for (section, count) in [
            (
                "Important Structural Change",
                input.important_structural.len(),
            ),
            ("Stage Transition", input.stage_transitions.len()),
            ("Top5", input.top5.len()),
            ("Threshold Distance", input.threshold_distances.len()),
            ("Rising", input.rising.len()),
            ("Dropped", input.dropped.len()),
        ] {
            if count > limits.max_items_per_section {
                return Err(TelegramRenderError::ItemLimitExceeded {
                    section,
                    limit: limits.max_items_per_section,
                    actual: count,
                });
            }
        }

        let company_card_count = input.top5.len()
            + input.threshold_distances.len()
            + input.rising.len()
            + input.dropped.len();
        if company_card_count > limits.max_company_cards {
            return Err(TelegramRenderError::CompanyCardLimitExceeded {
                limit: limits.max_company_cards,
                actual: company_card_count,
            });
        }

        let mut blocks = vec![format!("*Weekly Radar — {}*", input.period.as_str())];
        for block in [
            item_block("Important Structural Change", &input.important_structural),
            item_block("Stage Transition", &input.stage_transitions),
            card_block("Top5", &input.top5),
            card_block("Threshold Distance", &input.threshold_distances),
            card_block("Rising", &input.rising),
            card_block("Dropped", &input.dropped),
        ]
        .into_iter()
        .flatten()
        {
            blocks.push(block);
        }

        if let Some(health) = input.system_health() {
            blocks.push(format!(
                "## System Health\n- **{}** — {}",
                health.status().as_str(),
                health.markdown().as_str()
            ));
        }
        if let Some(no_change) = input.no_change() {
            blocks.push(format!(
                "## No Change ({})\n- {}",
                no_change.label(),
                no_change.markdown().as_str()
            ));
        }

        let markdown = blocks.join("\n\n");
        let character_count = markdown.chars().count();
        if character_count > limits.max_characters {
            return Err(TelegramRenderError::MessageTooLong {
                limit: limits.max_characters,
                actual: character_count,
            });
        }
        let line_count = markdown.lines().count();
        if line_count > limits.max_lines {
            return Err(TelegramRenderError::LineLimitExceeded {
                limit: limits.max_lines,
                actual: line_count,
            });
        }

        Ok(TelegramMessage {
            markdown,
            character_count,
            line_count,
            company_card_count,
        })
    }
}

#[cfg(test)]
#[path = "telegram_renderer_test.rs"]
mod module_tests;
