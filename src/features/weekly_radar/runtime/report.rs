//! Deterministic, provider-neutral Weekly Radar report assembly.
//!
//! The report is the human-first contract for the runtime. It renders only
//! facts already supplied by the runtime model; it does not calculate stage,
//! ranking, scores, or investment conclusions.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use chrono::NaiveDate;
use serde::Serialize;

use super::model::{Confidence, FactStatus, NormalizedFact, RuntimeReportInput};

const MAX_CHANGE_CARDS: usize = 3;
const MAX_COMPANY_CARDS: usize = 5;
const MAX_DISPLAY_CHARS: usize = 240;

/// Stable metadata retained alongside one rendered report snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SnapshotMetadata {
    as_of: NaiveDate,
    fact_count: usize,
    company_count: usize,
}

impl SnapshotMetadata {
    /// Returns the report as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.as_of
    }

    /// Returns the number of normalized facts represented in the snapshot.
    pub const fn fact_count(&self) -> usize {
        self.fact_count
    }

    /// Returns the number of companies represented in the snapshot.
    pub const fn company_count(&self) -> usize {
        self.company_count
    }
}

/// Source and fact counters shown in the System Health section.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceHealthFacts {
    confirmed: usize,
    unknown: usize,
    unavailable: usize,
    unconfirmed: usize,
    discovery_only: usize,
    top5_selection_missing: bool,
    review_items: Vec<String>,
}

impl SourceHealthFacts {
    /// Returns the number of confirmed facts.
    pub const fn confirmed(&self) -> usize {
        self.confirmed
    }

    /// Returns the number of unknown facts.
    pub const fn unknown(&self) -> usize {
        self.unknown
    }

    /// Returns the number of unavailable facts.
    pub const fn unavailable(&self) -> usize {
        self.unavailable
    }

    /// Returns the number of unconfirmed facts.
    pub const fn unconfirmed(&self) -> usize {
        self.unconfirmed
    }

    /// Returns the number of configured discovery-only source families.
    pub const fn discovery_only(&self) -> usize {
        self.discovery_only
    }

    /// Returns whether more than five companies lacked an explicit Top5 selection.
    pub const fn top5_selection_missing(&self) -> bool {
        self.top5_selection_missing
    }

    /// Returns deterministic review item labels.
    pub fn review_items(&self) -> &[String] {
        &self.review_items
    }
}

/// Complete deterministic output of [`render_report`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenderedReport {
    markdown: String,
    snapshot_json: String,
    report_id: String,
    metadata: SnapshotMetadata,
    health: SourceHealthFacts,
}

impl RenderedReport {
    /// Returns the exact Markdown bytes represented as UTF-8 text.
    pub fn markdown(&self) -> &str {
        &self.markdown
    }

    /// Returns the deterministic JSON snapshot text.
    pub fn snapshot_json(&self) -> &str {
        &self.snapshot_json
    }

    /// Returns the deterministic identity derived from sanitized Markdown and
    /// snapshot bytes.
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// Returns stable snapshot metadata.
    pub const fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }

    /// Returns the source-health counters used by the rendered report.
    pub const fn health(&self) -> &SourceHealthFacts {
        &self.health
    }

    /// Returns the report as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.metadata.as_of
    }
}

#[derive(Clone, Debug, Serialize)]
struct SnapshotFact {
    company_id: String,
    kind: String,
    value: Option<String>,
    status: FactStatus,
    confidence: Confidence,
    source_uri: String,
    source_field_or_passage: String,
    effective_date: Option<NaiveDate>,
}

#[derive(Clone, Debug, Serialize)]
struct SnapshotCoverage {
    source: String,
    expected: usize,
    available: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SnapshotDocument {
    metadata: SnapshotMetadata,
    facts: Vec<SnapshotFact>,
    source_coverage: Vec<SnapshotCoverage>,
    health: SourceHealthFacts,
}

fn compact_text(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut compact = normalized
        .chars()
        .take(MAX_DISPLAY_CHARS)
        .collect::<String>();
    if normalized.chars().count() > MAX_DISPLAY_CHARS {
        compact.push('…');
    }
    compact
}

fn redact_markers(value: &str) -> String {
    let mut result = String::new();
    for (index, word) in value.split_whitespace().enumerate() {
        if index > 0 {
            result.push(' ');
        }
        let lower = word.to_ascii_lowercase();
        if [
            "token=",
            "chat_id=",
            "bot_token=",
            "api_key=",
            "authorization=",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            result.push_str("[REDACTED]");
        } else {
            result.push_str(word);
        }
    }
    result
}

fn safe_text(value: &str) -> String {
    compact_text(&redact_markers(value).replace('`', "'"))
}

fn safe_uri(value: &str) -> String {
    let without_fragment = value.split('#').next().unwrap_or(value);
    let without_query = without_fragment
        .split('?')
        .next()
        .unwrap_or(without_fragment);
    if let Some(scheme_end) = without_query.find("://") {
        let authority_start = scheme_end + 3;
        let path_start = without_query[authority_start..]
            .find('/')
            .map(|offset| authority_start + offset)
            .unwrap_or(without_query.len());
        let authority = &without_query[authority_start..path_start];
        let safe_authority = authority.rsplit('@').next().unwrap_or(authority);
        return format!(
            "{}://{}{}",
            &without_query[..scheme_end],
            safe_authority,
            &without_query[path_start..]
        );
    }
    redact_markers(without_query)
}

fn status_label(status: &FactStatus) -> &'static str {
    status.as_str()
}

fn confidence_label(confidence: &Confidence) -> &'static str {
    confidence.as_str()
}

fn is_structural_change(kind: &str) -> bool {
    matches!(
        kind.to_ascii_lowercase().as_str(),
        "structural_change" | "important_structural_change"
    )
}

fn fact_value(fact: &NormalizedFact) -> String {
    fact.value()
        .map(safe_text)
        .unwrap_or_else(|| "no concrete value supplied".to_owned())
}

fn evidence_line(fact: &NormalizedFact) -> String {
    format!(
        "{} — {}",
        safe_uri(fact.provenance().source_uri()),
        safe_text(fact.provenance().source_field_or_passage())
    )
}

fn fact_line(fact: &NormalizedFact) -> String {
    format!(
        "- {}: {} — {} ({})\n  Evidence: {}",
        safe_text(fact.kind()),
        fact_value(fact),
        status_label(fact.status()),
        confidence_label(fact.confidence()),
        evidence_line(fact)
    )
}

fn source_is_discovery_only(source: &str) -> bool {
    let source = source.to_ascii_lowercase();
    source.contains("gdelt") || source.contains("discovery")
}

fn build_health(input: &RuntimeReportInput, facts: &[&NormalizedFact]) -> SourceHealthFacts {
    let mut health = SourceHealthFacts {
        confirmed: 0,
        unknown: 0,
        unavailable: 0,
        unconfirmed: 0,
        discovery_only: 0,
        top5_selection_missing: false,
        review_items: Vec::new(),
    };
    for fact in facts {
        match fact.status() {
            FactStatus::Known => health.confirmed += 1,
            FactStatus::Unknown => health.unknown += 1,
            FactStatus::Unavailable => health.unavailable += 1,
            FactStatus::Unconfirmed => health.unconfirmed += 1,
        }
        if fact.status() != &FactStatus::Known {
            health.review_items.push(format!(
                "{} / {} — {}",
                safe_text(fact.company_id()),
                safe_text(fact.kind()),
                status_label(fact.status())
            ));
        }
    }
    health.discovery_only = input
        .source_coverage()
        .iter()
        .filter(|coverage| source_is_discovery_only(coverage.source()))
        .count();
    let company_count = facts
        .iter()
        .map(|fact| fact.company_id())
        .collect::<BTreeSet<_>>()
        .len();
    health.top5_selection_missing = company_count > MAX_COMPANY_CARDS;
    health.review_items.sort();
    health
}

fn render_executive_summary(
    input: &RuntimeReportInput,
    facts: &[&NormalizedFact],
    health: &SourceHealthFacts,
    structural_count: usize,
    company_count: usize,
) -> String {
    let review_count = health.unknown + health.unavailable + health.unconfirmed;
    let mut section = format!(
        "## Executive Summary\nAs of {}, the input contains {} CONFIRMED facts across {} companies; unresolved evidence is listed below.",
        input.as_of(), health.confirmed, company_count
    );
    section.push_str(&format!(
        "\n- What changed: {} explicit structural-change fact(s) were supplied.",
        structural_count
    ));
    section.push_str(
        "\n- Why it matters: each displayed card keeps the supplied fact, status, and evidence together.",
    );
    section.push_str(&format!(
        "\n- Data status: {} fact(s) need review; source coverage and discovery-only material are shown in System Health.",
        review_count
    ));
    let evidence_basis = facts
        .first()
        .map(|fact| evidence_line(fact))
        .unwrap_or_else(|| "UNKNOWN — no evidence supplied".to_owned());
    section.push_str(&format!("\nEvidence basis: {evidence_basis}"));
    section
}

fn render_structural_changes(facts: &[&NormalizedFact]) -> Option<String> {
    let mut changes = facts
        .iter()
        .copied()
        .filter(|fact| is_structural_change(fact.kind()))
        .collect::<Vec<_>>();
    changes.sort_by(|left, right| {
        left.company_id()
            .cmp(right.company_id())
            .then_with(|| left.kind().cmp(right.kind()))
    });
    changes.truncate(MAX_CHANGE_CARDS);
    if changes.is_empty() {
        return None;
    }

    let mut section = String::from("## Important Structural Change");
    for (index, fact) in changes.iter().enumerate() {
        section.push_str(&format!(
            "\n### Change {} — {} — {}\n- What: {}\n- Why it matters: this explicitly supplied structural-change signal should be reviewed with its evidence.\n- Evidence: {}",
            index + 1,
            safe_text(fact.company_id()),
            status_label(fact.status()),
            fact_value(fact),
            evidence_line(fact)
        ));
    }
    Some(section)
}

fn render_top5(facts: &[&NormalizedFact]) -> Option<String> {
    let mut companies = BTreeMap::<&str, Vec<&NormalizedFact>>::new();
    for fact in facts {
        companies.entry(fact.company_id()).or_default().push(fact);
    }
    if companies.is_empty() || companies.len() > MAX_COMPANY_CARDS {
        return None;
    }

    let mut section = String::from("## Top5");
    for (company_index, (company, mut company_facts)) in
        companies.into_iter().take(MAX_COMPANY_CARDS).enumerate()
    {
        company_facts.sort_by(|left, right| left.kind().cmp(right.kind()));
        section.push_str(&format!("\n### {}", safe_text(company)));
        for fact in company_facts {
            section.push('\n');
            section.push_str(&fact_line(fact));
        }
        if company_index + 1 == MAX_COMPANY_CARDS {
            break;
        }
    }
    Some(section)
}

fn render_health(input: &RuntimeReportInput, health: &SourceHealthFacts) -> String {
    let mut coverage = input
        .source_coverage()
        .iter()
        .map(|item| {
            format!(
                "{}: {}/{}",
                safe_text(item.source()),
                item.available(),
                item.expected()
            )
        })
        .collect::<Vec<_>>();
    coverage.sort();

    let mut section = format!(
        "## System Health\n- Source coverage: {}\n- Fact status counts: CONFIRMED {}, UNKNOWN {}, UNAVAILABLE {}, UNCONFIRMED {}",
        if coverage.is_empty() {
            "none recorded".to_owned()
        } else {
            coverage.join("; ")
        },
        health.confirmed,
        health.unknown,
        health.unavailable,
        health.unconfirmed
    );
    if health.discovery_only == 0 {
        section.push_str("\n- DISCOVERY ONLY sources: none recorded.");
    } else {
        section.push_str(&format!(
            "\n- DISCOVERY ONLY sources: {} source family/families; discovery material is not authoritative.",
            health.discovery_only
        ));
    }
    if health.top5_selection_missing {
        section.push_str(
            "\n- Top5: UNKNOWN — no explicit Top5 selection was supplied for more than five companies.",
        );
    }
    section.push_str(&format!("\n- Data age: as of {}.", input.as_of()));
    if health.review_items.is_empty() {
        section.push_str("\n- Items needing review: none recorded.");
    } else {
        section.push_str("\n- Items needing review:");
        for item in &health.review_items {
            section.push_str("\n  - ");
            section.push_str(item);
        }
    }
    section
}

fn snapshot_fact(fact: &NormalizedFact) -> SnapshotFact {
    SnapshotFact {
        company_id: safe_text(fact.company_id()),
        kind: safe_text(fact.kind()),
        value: fact.value().map(safe_text),
        status: *fact.status(),
        confidence: *fact.confidence(),
        source_uri: safe_uri(fact.provenance().source_uri()),
        source_field_or_passage: safe_text(fact.provenance().source_field_or_passage()),
        effective_date: fact.provenance().effective_date().copied(),
    }
}

fn report_digest(markdown: &str, snapshot_json: &str) -> String {
    let mut digest = 14_695_981_039_346_656_037_u64;
    for byte in markdown
        .bytes()
        .chain(std::iter::once(0xff))
        .chain(snapshot_json.bytes())
    {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    format!("wr-{digest:016x}")
}

/// Renders one deterministic, human-first report from already normalized input.
pub fn render_report(input: &RuntimeReportInput) -> RenderedReport {
    let mut facts = input.facts().iter().collect::<Vec<_>>();
    facts.sort_by(|left, right| {
        left.company_id()
            .cmp(right.company_id())
            .then_with(|| left.kind().cmp(right.kind()))
    });
    let mut company_ids = facts
        .iter()
        .map(|fact| fact.company_id())
        .collect::<Vec<_>>();
    company_ids.sort_unstable();
    company_ids.dedup();

    let health = build_health(input, &facts);
    let structural_count = facts
        .iter()
        .filter(|fact| is_structural_change(fact.kind()))
        .count();
    let mut sections = vec![render_executive_summary(
        input,
        &facts,
        &health,
        structural_count,
        company_ids.len(),
    )];
    if let Some(section) = render_structural_changes(&facts) {
        sections.push(section);
    }
    if let Some(section) = render_top5(&facts) {
        sections.push(section);
    }
    sections.push(render_health(input, &health));
    let markdown = sections.join("\n\n") + "\n";

    let metadata = SnapshotMetadata {
        as_of: input.as_of(),
        fact_count: facts.len(),
        company_count: company_ids.len(),
    };
    let mut snapshot_facts = facts.into_iter().map(snapshot_fact).collect::<Vec<_>>();
    snapshot_facts.sort_by(|left, right| {
        left.company_id
            .cmp(&right.company_id)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    let mut source_coverage = input
        .source_coverage()
        .iter()
        .map(|item| SnapshotCoverage {
            source: safe_text(item.source()),
            expected: item.expected(),
            available: item.available(),
        })
        .collect::<Vec<_>>();
    source_coverage.sort_by(|left, right| left.source.cmp(&right.source));
    let snapshot_json = serde_json::to_string_pretty(&SnapshotDocument {
        metadata: metadata.clone(),
        facts: snapshot_facts,
        source_coverage,
        health: health.clone(),
    })
    .expect("report snapshot contains only serializable values")
        + "\n";
    let report_id = report_digest(&markdown, &snapshot_json);

    RenderedReport {
        markdown,
        snapshot_json,
        report_id,
        metadata,
        health,
    }
}
