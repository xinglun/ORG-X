//! Deterministic archival Markdown rendering for explicit Weekly Radar facts.

use std::fmt;

use crate::features::reporting::domain::{ResearchCard, ResearchPacket};
use crate::features::weekly_radar::domain::change_compression::WeeklyChangeCompression;
use crate::features::weekly_radar::domain::system_health::{Freshness, HealthStatus, SystemHealth};
use crate::features::weekly_radar::domain::top5_weekly_read_model::Top5WeeklyReadModel;
use crate::features::weekly_radar::domain::WeeklyRadarSnapshot;

const EMPTY: &str = "EMPTY";
const NOT_SUPPLIED: &str = "NOT_SUPPLIED";
const NOT_EMITTED: &str = "NOT_EMITTED";

/// Validation errors for explicit records owned by the renderer boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownRenderError {
    /// A required record value contained only whitespace.
    EmptyValue {
        /// The record type containing the invalid value.
        entity: &'static str,
        /// The field containing the invalid value.
        field: &'static str,
    },
}

impl fmt::Display for MarkdownRenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { entity, field } => {
                write!(formatter, "{entity} {field} cannot be empty")
            }
        }
    }
}

impl std::error::Error for MarkdownRenderError {}

fn non_empty(
    entity: &'static str,
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, MarkdownRenderError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(MarkdownRenderError::EmptyValue { entity, field });
    }
    Ok(value)
}

/// An explicit historical Stage record supplied to the report boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StageHistoryEntry {
    id: String,
    period: String,
    company: String,
    previous_stage: String,
    current_stage: String,
    fact: String,
}

impl StageHistoryEntry {
    /// Creates a Stage History record without comparing or deriving stages.
    pub fn new(
        id: impl Into<String>,
        period: impl Into<String>,
        company: impl Into<String>,
        previous_stage: impl Into<String>,
        current_stage: impl Into<String>,
        fact: impl Into<String>,
    ) -> Result<Self, MarkdownRenderError> {
        Ok(Self {
            id: non_empty("stage history", "id", id)?,
            period: non_empty("stage history", "period", period)?,
            company: non_empty("stage history", "company", company)?,
            previous_stage: non_empty("stage history", "previous stage", previous_stage)?,
            current_stage: non_empty("stage history", "current stage", current_stage)?,
            fact: non_empty("stage history", "fact", fact)?,
        })
    }

    /// Returns the supplied identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the supplied period.
    pub fn period(&self) -> &str {
        &self.period
    }

    /// Returns the supplied company.
    pub fn company(&self) -> &str {
        &self.company
    }

    /// Returns the supplied previous Stage label.
    pub fn previous_stage(&self) -> &str {
        &self.previous_stage
    }

    /// Returns the supplied current Stage label.
    pub fn current_stage(&self) -> &str {
        &self.current_stage
    }

    /// Returns the supplied opaque history fact.
    pub fn fact(&self) -> &str {
        &self.fact
    }
}

/// An explicit rank-change record supplied to the report boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankChange {
    id: String,
    period: String,
    company: String,
    previous_rank: Option<u32>,
    current_rank: Option<u32>,
    fact: String,
}

impl RankChange {
    /// Creates a rank-change record without computing a rank or delta.
    pub fn new(
        id: impl Into<String>,
        period: impl Into<String>,
        company: impl Into<String>,
        previous_rank: Option<u32>,
        current_rank: Option<u32>,
        fact: impl Into<String>,
    ) -> Result<Self, MarkdownRenderError> {
        Ok(Self {
            id: non_empty("rank change", "id", id)?,
            period: non_empty("rank change", "period", period)?,
            company: non_empty("rank change", "company", company)?,
            previous_rank,
            current_rank,
            fact: non_empty("rank change", "fact", fact)?,
        })
    }

    /// Returns the supplied identity.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the supplied period.
    pub fn period(&self) -> &str {
        &self.period
    }

    /// Returns the supplied company.
    pub fn company(&self) -> &str {
        &self.company
    }

    /// Returns the supplied previous rank, if one was supplied.
    pub const fn previous_rank(&self) -> Option<u32> {
        self.previous_rank
    }

    /// Returns the supplied current rank, if one was supplied.
    pub const fn current_rank(&self) -> Option<u32> {
        self.current_rank
    }

    /// Returns the supplied opaque rank-change fact.
    pub fn fact(&self) -> &str {
        &self.fact
    }
}

/// Borrowed inputs for one complete archival Markdown report.
#[derive(Clone, Copy, Debug)]
pub struct MarkdownReportInput<'a> {
    snapshot: &'a WeeklyRadarSnapshot,
    top5: &'a Top5WeeklyReadModel,
    research: &'a ResearchPacket,
    compression: &'a WeeklyChangeCompression,
    stage_history: &'a [StageHistoryEntry],
    rank_changes: &'a [RankChange],
    system_health: Option<&'a SystemHealth>,
}

impl<'a> MarkdownReportInput<'a> {
    /// Binds explicit read models and ordered report facts without copying them.
    pub fn new(
        snapshot: &'a WeeklyRadarSnapshot,
        top5: &'a Top5WeeklyReadModel,
        research: &'a ResearchPacket,
        compression: &'a WeeklyChangeCompression,
        stage_history: &'a [StageHistoryEntry],
        rank_changes: &'a [RankChange],
        system_health: Option<&'a SystemHealth>,
    ) -> Self {
        Self {
            snapshot,
            top5,
            research,
            compression,
            stage_history,
            rank_changes,
            system_health,
        }
    }
}

/// The deterministic in-memory Markdown report produced by the renderer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarkdownDocument(String);

impl MarkdownDocument {
    /// Returns the rendered Markdown bytes as text without rewriting them.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stateless renderer for one explicit Weekly Radar report input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MarkdownRenderer;

impl MarkdownRenderer {
    /// Renders all supplied sections in one fixed deterministic order.
    pub fn render(input: &MarkdownReportInput<'_>) -> MarkdownDocument {
        let mut output = String::new();
        render_header(&mut output, input.snapshot);
        render_compression(&mut output, input.compression);
        render_top5(&mut output, input.top5);
        render_research_cards(&mut output, input.research);
        render_card_facts(&mut output, input.research, "Evidence", card_evidence);
        render_card_facts(
            &mut output,
            input.research,
            "Counter Evidence",
            card_counter_evidence,
        );
        render_card_facts(
            &mut output,
            input.research,
            "Missing Proof",
            card_missing_proof,
        );
        render_stage_history(&mut output, input.stage_history);
        render_rank_changes(&mut output, input.rank_changes);
        render_system_health(&mut output, input.system_health);
        MarkdownDocument(output)
    }
}

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn push_heading(output: &mut String, heading: &str) {
    push_line(output, heading);
}

fn push_field(output: &mut String, label: &str, value: &str) {
    output.push_str("- ");
    output.push_str(label);
    output.push_str(": ");
    output.push_str(value);
    output.push('\n');
}

fn push_number(output: &mut String, label: &str, value: u32) {
    push_field(output, label, &value.to_string());
}

fn render_header(output: &mut String, snapshot: &WeeklyRadarSnapshot) {
    push_heading(output, "# Weekly Radar Markdown Report");
    push_heading(output, "## Snapshot");
    push_field(output, "Snapshot ID", snapshot.id().as_str());
    push_field(output, "As Of", snapshot.as_of().as_str());
    push_field(
        output,
        "Universe Snapshot ID",
        snapshot.universe_snapshot_id().as_str(),
    );
    push_field(
        output,
        "Evidence Cutoff",
        snapshot.evidence_cutoff().as_str(),
    );
    push_field(output, "Model Version", snapshot.model_version().as_str());
    push_field(
        output,
        "Scoring Version",
        snapshot.scoring_version().as_str(),
    );
}

fn render_compression(output: &mut String, compression: &WeeklyChangeCompression) {
    push_heading(output, "## Change Compression");
    render_change_section(
        output,
        "Important Structural Change",
        compression.important_structural(),
    );
    render_change_section(output, "Top5 Change", compression.top5());
    render_change_section(output, "Stage Transition", compression.stage_transitions());
    render_change_section(output, "Rising", compression.rising());
    render_change_section(output, "Dropped", compression.dropped());

    push_heading(output, "### No Change");
    match compression.no_change() {
        Some(no_change) => {
            push_field(output, "Label", no_change.label());
            push_field(output, "Period", no_change.period().as_str());
            let counts = no_change.counts();
            push_field(
                output,
                "Important Structural Change Count",
                &counts.important_structural().to_string(),
            );
            push_field(output, "Top5 Change Count", &counts.top5().to_string());
            push_field(
                output,
                "Stage Transition Count",
                &counts.stage_transitions().to_string(),
            );
            push_field(output, "Rising Count", &counts.rising().to_string());
            push_field(output, "Dropped Count", &counts.dropped().to_string());
        }
        None => push_field(output, "No Change", NOT_EMITTED),
    }
}

fn render_change_section<T>(output: &mut String, title: &str, events: &[T])
where
    T: ChangeEventView,
{
    push_heading(output, &format!("### {title}"));
    if events.is_empty() {
        push_field(output, "Events", EMPTY);
        return;
    }
    for event in events {
        push_heading(output, "#### Event");
        push_field(output, "Event ID", event.event_id());
        push_field(output, "Period", event.period());
        push_field(output, "Company", event.company());
        push_field(output, "Fact", event.fact());
    }
}

trait ChangeEventView {
    fn event_id(&self) -> &str;
    fn period(&self) -> &str;
    fn company(&self) -> &str;
    fn fact(&self) -> &str;
}

macro_rules! impl_change_event_view {
    ($type:ty) => {
        impl ChangeEventView for $type {
            fn event_id(&self) -> &str {
                self.event_id().as_str()
            }

            fn period(&self) -> &str {
                self.period().as_str()
            }

            fn company(&self) -> &str {
                self.company().as_str()
            }

            fn fact(&self) -> &str {
                self.fact().as_str()
            }
        }
    };
}

impl_change_event_view!(
    crate::features::weekly_radar::domain::change_compression::ImportantStructuralChange
);
impl_change_event_view!(crate::features::weekly_radar::domain::change_compression::Top5Change);
impl_change_event_view!(
    crate::features::weekly_radar::domain::change_compression::StageTransitionChange
);
impl_change_event_view!(crate::features::weekly_radar::domain::change_compression::RisingChange);
impl_change_event_view!(crate::features::weekly_radar::domain::change_compression::DroppedChange);

fn render_top5(output: &mut String, top5: &Top5WeeklyReadModel) {
    push_heading(output, "## Top5");
    if top5.entries().is_empty() {
        push_field(output, "Entries", EMPTY);
        return;
    }
    for entry in top5.entries() {
        push_heading(output, "### Entry");
        push_field(output, "Candidate", entry.candidate().as_str());
        push_field(output, "Company", entry.company().as_str());
        push_field(output, "Stage", entry.stage().as_str());
        push_field(output, "Direction", entry.direction().as_str());
        push_field(output, "Confidence", entry.confidence().as_str());
        push_field(output, "Key Change", entry.key_change().as_str());
        push_field(output, "Next", entry.next().as_str());
    }
}

fn render_research_cards(output: &mut String, research: &ResearchPacket) {
    push_heading(output, "## Research Cards");
    push_field(
        output,
        "Executive Summary",
        research.executive_summary().as_str(),
    );
    render_card_group(output, "Top5", research.top5().cards());
    render_card_group(output, "Rising", research.rising().cards());
    render_card_group(output, "Watch", research.watch().cards());
    render_card_group(output, "Dropped", research.dropped().cards());
}

fn render_card_group(output: &mut String, title: &str, cards: &[ResearchCard]) {
    push_heading(output, &format!("### {title}"));
    if cards.is_empty() {
        push_field(output, "Cards", EMPTY);
        return;
    }
    for card in cards {
        push_heading(output, "#### Card");
        push_field(output, "Card ID", card.id().as_str());
        push_field(output, "Company", card.company().as_str());
        push_field(output, "Stage", card.stage().as_str());
        push_field(output, "Headline", card.headline().as_str());
        push_field(output, "Next Step", card.next_step().as_str());
    }
}

fn card_evidence(card: &ResearchCard) -> &str {
    card.evidence().as_str()
}

fn card_counter_evidence(card: &ResearchCard) -> &str {
    card.counter_evidence().as_str()
}

fn card_missing_proof(card: &ResearchCard) -> &str {
    card.missing_proof().as_str()
}

fn render_card_facts(
    output: &mut String,
    research: &ResearchPacket,
    title: &str,
    value: fn(&ResearchCard) -> &str,
) {
    push_heading(output, &format!("## {title}"));
    let groups = [
        ("Top5", research.top5().cards()),
        ("Rising", research.rising().cards()),
        ("Watch", research.watch().cards()),
        ("Dropped", research.dropped().cards()),
    ];
    for (group_name, cards) in groups {
        push_heading(output, &format!("### {group_name}"));
        if cards.is_empty() {
            push_field(output, "Cards", EMPTY);
            continue;
        }
        for card in cards {
            push_heading(output, "#### Card");
            push_field(output, "Card ID", card.id().as_str());
            push_field(output, "Company", card.company().as_str());
            push_field(output, "Value", value(card));
        }
    }
}

fn render_stage_history(output: &mut String, entries: &[StageHistoryEntry]) {
    push_heading(output, "## Stage History");
    if entries.is_empty() {
        push_field(output, "Stage History", EMPTY);
        return;
    }
    for entry in entries {
        push_heading(output, "### Entry");
        push_field(output, "History ID", entry.id());
        push_field(output, "Period", entry.period());
        push_field(output, "Company", entry.company());
        push_field(output, "Previous Stage", entry.previous_stage());
        push_field(output, "Current Stage", entry.current_stage());
        push_field(output, "Fact", entry.fact());
    }
}

fn render_rank_changes(output: &mut String, entries: &[RankChange]) {
    push_heading(output, "## Rank Changes");
    if entries.is_empty() {
        push_field(output, "Rank Changes", EMPTY);
        return;
    }
    for entry in entries {
        push_heading(output, "### Entry");
        push_field(output, "Rank Change ID", entry.id());
        push_field(output, "Period", entry.period());
        push_field(output, "Company", entry.company());
        push_field(output, "Previous Rank", &rank_label(entry.previous_rank()));
        push_field(output, "Current Rank", &rank_label(entry.current_rank()));
        push_field(output, "Fact", entry.fact());
    }
}

fn rank_label(value: Option<u32>) -> String {
    value
        .map(|rank| rank.to_string())
        .unwrap_or_else(|| NOT_SUPPLIED.to_owned())
}

fn render_system_health(output: &mut String, health: Option<&SystemHealth>) {
    push_heading(output, "## System Health");
    let Some(health) = health else {
        push_field(output, "System Health", NOT_SUPPLIED);
        return;
    };

    push_field(output, "Status", health_status_label(health.status()));
    push_field(output, "Freshness", freshness_label(health.freshness()));
    let coverage = health.evidence_coverage();
    push_number(output, "Evidence Available", coverage.available());
    push_number(output, "Evidence Expected", coverage.expected());
    push_number(
        output,
        "Evidence Percentage",
        coverage.percentage().value() as u32,
    );

    push_heading(output, "### Degraded Companies");
    if health.degraded_companies().is_empty() {
        push_field(output, "Degraded Companies", EMPTY);
    } else {
        for company in health.degraded_companies() {
            push_heading(output, "#### Company");
            push_field(output, "Company", company.company().as_str());
            push_field(output, "Reason", company.reason().as_str());
        }
    }

    push_heading(output, "### Source Coverage");
    if health.source_coverage().is_empty() {
        push_field(output, "Source Coverage", EMPTY);
    } else {
        for coverage in health.source_coverage() {
            push_heading(output, "#### Source");
            push_field(output, "Source", coverage.source().as_str());
            push_number(output, "Available", coverage.available());
            push_number(output, "Expected", coverage.expected());
            push_number(output, "Percentage", coverage.percentage().value() as u32);
        }
    }

    push_heading(output, "### Extraction Failures");
    if health.extraction_failures().is_empty() {
        push_field(output, "Extraction Failures", EMPTY);
    } else {
        for failure in health.extraction_failures() {
            push_heading(output, "#### Failure");
            push_field(output, "Failure ID", failure.id().as_str());
            push_field(output, "Source", failure.source().as_str());
            push_field(output, "Reason", failure.reason().as_str());
        }
    }
}

fn health_status_label(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Healthy => "HEALTHY",
        HealthStatus::Degraded => "DEGRADED",
        HealthStatus::Unavailable => "UNAVAILABLE",
        HealthStatus::Unknown => "UNKNOWN",
    }
}

fn freshness_label(freshness: Freshness) -> &'static str {
    match freshness {
        Freshness::Current => "CURRENT",
        Freshness::Aging => "AGING",
        Freshness::Stale => "STALE",
        Freshness::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
#[path = "markdown_renderer_test.rs"]
mod module_tests;
