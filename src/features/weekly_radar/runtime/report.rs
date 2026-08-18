//! Deterministic, provider-neutral Weekly Radar report assembly.
//!
//! The Markdown output is a reader-facing localized summary. Provider status,
//! source identifiers, and complete review detail remain in the serialized
//! snapshot so Telegram is useful to a person without losing auditability.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;
use serde::Serialize;

use super::model::{
    CompanyIdentity, Confidence, FactStatus, NormalizedFact, RuntimeReportInput, SourceCoverage,
    SourceFailure,
};

const MAX_CHANGE_CARDS: usize = 3;
const MAX_COMPANY_CARDS: usize = 5;
const MAX_DISPLAY_CHARS: usize = 240;
const MAX_HEALTH_GROUPS: usize = 6;

/// Supported Weekly Radar presentation languages.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ReportLanguage {
    /// Simplified Chinese, the default for scheduled reports.
    #[default]
    Chinese,
    /// Japanese.
    Japanese,
    /// English.
    English,
}

impl ReportLanguage {
    /// Returns the stable CLI and snapshot language identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chinese => "zh-CN",
            Self::Japanese => "ja",
            Self::English => "en",
        }
    }
}

impl FromStr for ReportLanguage {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "zh-CN" | "zh" => Ok(Self::Chinese),
            "ja" | "ja-JP" => Ok(Self::Japanese),
            "en" | "en-US" => Ok(Self::English),
            other => Err(format!(
                "--language must be one of zh-CN, ja, or en; received {other}"
            )),
        }
    }
}

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

/// Source and fact counters retained for the detailed snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SourceHealthFacts {
    confirmed: usize,
    unknown: usize,
    unavailable: usize,
    unconfirmed: usize,
    discovery_only: usize,
    top5_selection_missing: bool,
    review_items: Vec<String>,
    source_failures: Vec<SourceFailure>,
}

impl SourceHealthFacts {
    /// Returns the number of confirmed facts.
    pub const fn confirmed(&self) -> usize {
        self.confirmed
    }

    /// Returns the number of confirmed-unknown facts.
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

    /// Returns the number of discovery-only source families.
    pub const fn discovery_only(&self) -> usize {
        self.discovery_only
    }

    /// Returns whether more than five companies lacked an explicit Top5 selection.
    pub const fn top5_selection_missing(&self) -> bool {
        self.top5_selection_missing
    }

    /// Returns complete deterministic review item labels for the snapshot.
    pub fn review_items(&self) -> &[String] {
        &self.review_items
    }

    /// Returns safe source acquisition failures for the snapshot.
    pub fn source_failures(&self) -> &[SourceFailure] {
        &self.source_failures
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

    /// Returns the deterministic identity derived from Markdown and snapshot bytes.
    pub fn report_id(&self) -> &str {
        &self.report_id
    }

    /// Returns stable snapshot metadata.
    pub const fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }

    /// Returns the detailed source-health counters used by the snapshot.
    pub const fn health(&self) -> &SourceHealthFacts {
        &self.health
    }

    /// Returns the report as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.metadata.as_of
    }
}

#[derive(Clone, Copy)]
struct Labels {
    summary: &'static str,
    changes: &'static str,
    company_observation: &'static str,
    health: &'static str,
    change_status: &'static str,
    no_change: &'static str,
    evidence_basis: &'static str,
    no_primary: &'static str,
    data_status: &'static str,
    source_status: &'static str,
    attention: &'static str,
    no_attention: &'static str,
    confirmed_facts: &'static str,
    pending_leads: &'static str,
    unknown_facts: &'static str,
    unavailable_facts: &'static str,
    source_available: &'static str,
    source_unavailable: &'static str,
    source_not_configured: &'static str,
    discovery_note: &'static str,
    ranking_missing: &'static str,
    as_of: &'static str,
    source_label_official_ir: &'static str,
    source_label_careers: &'static str,
    source_label_engineering: &'static str,
    source_label_greenhouse: &'static str,
    source_label_lever: &'static str,
    source_label_gdelt: &'static str,
    source_label_sec: &'static str,
    field_revenue: &'static str,
    field_operating_income: &'static str,
    field_net_income: &'static str,
    field_cash_flow: &'static str,
    field_capex: &'static str,
    field_r_and_d: &'static str,
    field_sbc: &'static str,
    field_employee_count: &'static str,
    field_structural_change: &'static str,
}

fn labels(language: ReportLanguage) -> Labels {
    match language {
        ReportLanguage::Chinese => Labels {
            summary: "本周摘要",
            changes: "重要组织变化",
            company_observation: "重点公司",
            health: "系统状态",
            change_status: "组织变化",
            no_change: "本周没有发现已确认的组织结构变化。",
            evidence_basis: "主证据",
            no_primary: "本周没有可作为主证据的确认信息。",
            data_status: "数据状态",
            source_status: "来源情况",
            attention: "需要关注",
            no_attention: "目前没有需要人工处理的异常。",
            confirmed_facts: "已确认信息",
            pending_leads: "待核实线索",
            unknown_facts: "无法确定",
            unavailable_facts: "暂不可用",
            source_available: "可用",
            source_unavailable: "暂不可用",
            source_not_configured: "尚未配置",
            discovery_note: "新闻和其他发现材料仅用于找线索，未作为结论依据。",
            ranking_missing: "本周未提供明确的重点公司选择，因此不生成排名。",
            as_of: "数据截至",
            source_label_official_ir: "投资者关系资料",
            source_label_careers: "职业与招聘页面",
            source_label_engineering: "工程与 AI 资料",
            source_label_greenhouse: "Greenhouse 招聘接口",
            source_label_lever: "Lever 招聘接口",
            source_label_gdelt: "新闻发现",
            source_label_sec: "SEC 财务与申报资料",
            field_revenue: "营收",
            field_operating_income: "营业利润",
            field_net_income: "净利润",
            field_cash_flow: "经营现金流",
            field_capex: "资本支出",
            field_r_and_d: "研发投入",
            field_sbc: "股权激励费用",
            field_employee_count: "员工人数",
            field_structural_change: "组织变化",
        },
        ReportLanguage::Japanese => Labels {
            summary: "週次サマリー",
            changes: "重要な組織変化",
            company_observation: "注目企業",
            health: "システム状態",
            change_status: "組織変化",
            no_change: "今週、確認済みの組織構造の変化はありません。",
            evidence_basis: "主な根拠",
            no_primary: "今週、主な根拠として使える確認済み情報はありません。",
            data_status: "データ状態",
            source_status: "情報源の状況",
            attention: "要確認",
            no_attention: "現在、手動対応が必要な異常はありません。",
            confirmed_facts: "確認済み情報",
            pending_leads: "未確認の手がかり",
            unknown_facts: "判定不能",
            unavailable_facts: "取得できず",
            source_available: "利用可能",
            source_unavailable: "取得できず",
            source_not_configured: "未設定",
            discovery_note: "ニュース等の探索情報は手がかりのみで、結論の根拠にはしていません。",
            ranking_missing: "明示的な注目企業の選定がないため、ランキングは作成していません。",
            as_of: "基準日",
            source_label_official_ir: "IR 資料",
            source_label_careers: "採用ページ",
            source_label_engineering: "Engineering / AI 資料",
            source_label_greenhouse: "Greenhouse 採用 API",
            source_label_lever: "Lever 採用 API",
            source_label_gdelt: "ニュース探索",
            source_label_sec: "SEC 財務・提出資料",
            field_revenue: "売上高",
            field_operating_income: "営業利益",
            field_net_income: "純利益",
            field_cash_flow: "営業キャッシュフロー",
            field_capex: "設備投資",
            field_r_and_d: "研究開発費",
            field_sbc: "株式報酬費用",
            field_employee_count: "従業員数",
            field_structural_change: "組織変化",
        },
        ReportLanguage::English => Labels {
            summary: "Executive Summary",
            changes: "Important Organizational Changes",
            company_observation: "Companies to Watch",
            health: "System Health",
            change_status: "Change signal",
            no_change: "No confirmed organizational-structure change was found this week.",
            evidence_basis: "Primary evidence",
            no_primary: "No confirmed information was available as primary evidence this week.",
            data_status: "Data status",
            source_status: "Source status",
            attention: "Needs attention",
            no_attention: "No manual action is currently required.",
            confirmed_facts: "Confirmed information",
            pending_leads: "Leads to verify",
            unknown_facts: "Could not determine",
            unavailable_facts: "Unavailable",
            source_available: "available",
            source_unavailable: "unavailable",
            source_not_configured: "not configured",
            discovery_note:
                "News and other discovery material is used for leads only, not as a conclusion.",
            ranking_missing:
                "No explicit company selection was supplied, so no ranking was generated.",
            as_of: "As of",
            source_label_official_ir: "Investor-relations material",
            source_label_careers: "Careers and hiring pages",
            source_label_engineering: "Engineering and AI material",
            source_label_greenhouse: "Greenhouse hiring API",
            source_label_lever: "Lever hiring API",
            source_label_gdelt: "News discovery",
            source_label_sec: "SEC financial and filing data",
            field_revenue: "Revenue",
            field_operating_income: "Operating income",
            field_net_income: "Net income",
            field_cash_flow: "Operating cash flow",
            field_capex: "CapEx",
            field_r_and_d: "R&D",
            field_sbc: "Stock-based compensation",
            field_employee_count: "Employees",
            field_structural_change: "Organizational change",
        },
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
struct SnapshotCompany {
    id: String,
    name: String,
    ticker: String,
}

#[derive(Clone, Debug, Serialize)]
struct SnapshotCoverage {
    source: String,
    expected: usize,
    available: usize,
    not_configured: usize,
    unavailable: usize,
}

#[derive(Clone, Debug, Serialize)]
struct SnapshotDocument {
    language: String,
    metadata: SnapshotMetadata,
    companies: Vec<SnapshotCompany>,
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

fn source_kind_for_fact(kind: &str) -> Option<&'static str> {
    let lower = kind.to_ascii_lowercase();
    if lower.starts_with("source_official_ir") {
        Some("official_ir")
    } else if lower.starts_with("source_careers") {
        Some("careers")
    } else if lower.starts_with("source_engineering_ai_blog") {
        Some("engineering_ai_blog")
    } else if lower.starts_with("source_greenhouse") {
        Some("greenhouse")
    } else if lower.starts_with("source_lever") {
        Some("lever")
    } else if lower.starts_with("source_gdelt") {
        Some("gdelt")
    } else {
        None
    }
}

fn source_label(source: &str, labels: Labels) -> String {
    match source.to_ascii_lowercase().as_str() {
        "official_ir" => labels.source_label_official_ir.to_owned(),
        "careers" => labels.source_label_careers.to_owned(),
        "engineering_ai_blog" => labels.source_label_engineering.to_owned(),
        "greenhouse" => labels.source_label_greenhouse.to_owned(),
        "lever" => labels.source_label_lever.to_owned(),
        "gdelt" | "gdelt-discovery" => labels.source_label_gdelt.to_owned(),
        "sec" => labels.source_label_sec.to_owned(),
        _ => labels.source_status.to_owned(),
    }
}

fn fact_label(kind: &str, labels: Labels) -> String {
    let lower = kind.to_ascii_lowercase();
    if let Some(source) = source_kind_for_fact(kind) {
        return source_label(source, labels);
    }
    match lower.as_str() {
        "revenue" => labels.field_revenue.to_owned(),
        "operating_income" => labels.field_operating_income.to_owned(),
        "net_income" => labels.field_net_income.to_owned(),
        "operating_cash_flow" => labels.field_cash_flow.to_owned(),
        "capex" => labels.field_capex.to_owned(),
        "r_and_d" => labels.field_r_and_d.to_owned(),
        "sbc" => labels.field_sbc.to_owned(),
        "employee_count" => labels.field_employee_count.to_owned(),
        "structural_change" | "important_structural_change" => {
            labels.field_structural_change.to_owned()
        }
        _ => match labels.field_structural_change {
            "组织变化" => "其他资料".to_owned(),
            "組織変化" => "その他の資料".to_owned(),
            _ => "Other material".to_owned(),
        },
    }
}

fn company_label(input: &RuntimeReportInput, id: &str) -> String {
    input
        .company(id)
        .map(|company| {
            if company.ticker().trim().is_empty() {
                safe_text(company.name())
            } else {
                format!(
                    "{} ({})",
                    safe_text(company.name()),
                    safe_text(company.ticker())
                )
            }
        })
        .unwrap_or_else(|| safe_text(id))
}

fn status_for_snapshot(status: &FactStatus) -> &'static str {
    status.as_str()
}

fn is_structural_change(kind: &str) -> bool {
    matches!(
        kind.to_ascii_lowercase().as_str(),
        "structural_change" | "important_structural_change"
    )
}

fn is_confirmed_primary(fact: &NormalizedFact) -> bool {
    if fact.status() != &FactStatus::Known {
        return false;
    }
    let uri = fact.provenance().source_uri().to_ascii_lowercase();
    uri.contains("sec.gov")
        || source_kind_for_fact(fact.kind()).is_some_and(|source| {
            matches!(source, "official_ir" | "careers" | "engineering_ai_blog")
        })
}

fn fact_value(fact: &NormalizedFact) -> String {
    fact.value()
        .map(safe_text)
        .unwrap_or_else(|| "—".to_owned())
}

fn evidence_line(fact: &NormalizedFact, labels: Labels) -> String {
    let source = safe_uri(fact.provenance().source_uri());
    match labels.evidence_basis {
        "主证据" => format!("来源：{source}"),
        "主な根拠" => format!("出典：{source}"),
        _ => format!("Source: {source}"),
    }
}

fn primary_evidence_uri(fact: &NormalizedFact) -> String {
    safe_uri(fact.provenance().source_uri())
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
        source_failures: input.source_failures().to_vec(),
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
                status_for_snapshot(fact.status())
            ));
        }
    }
    health.discovery_only = input
        .source_coverage()
        .iter()
        .filter(|coverage| coverage.source().to_ascii_lowercase().contains("gdelt"))
        .count();
    let company_count = if input.companies().is_empty() {
        facts
            .iter()
            .map(|fact| fact.company_id())
            .collect::<BTreeSet<_>>()
            .len()
    } else {
        input.companies().len()
    };
    health.top5_selection_missing = company_count > MAX_COMPANY_CARDS;
    health.review_items.sort();
    health.source_failures.sort_by(|left, right| {
        left.source()
            .cmp(right.source())
            .then_with(|| left.company_id().cmp(right.company_id()))
            .then_with(|| left.reason().cmp(right.reason()))
    });
    health
}

fn render_executive_summary(
    input: &RuntimeReportInput,
    facts: &[&NormalizedFact],
    health: &SourceHealthFacts,
    structural_count: usize,
    company_count: usize,
    language: ReportLanguage,
) -> String {
    let labels = labels(language);
    let intro = match language {
        ReportLanguage::Chinese => format!(
            "{} {}，覆盖 {} 家公司，共 {} 条已确认信息。",
            labels.as_of,
            input.as_of(),
            company_count,
            health.confirmed
        ),
        ReportLanguage::Japanese => format!(
            "{} {}、{} 社を対象に {} 件の確認済み情報があります。",
            labels.as_of,
            input.as_of(),
            company_count,
            health.confirmed
        ),
        ReportLanguage::English => format!(
            "As of {}, the report covers {} companies and {} confirmed items.",
            input.as_of(),
            company_count,
            health.confirmed
        ),
    };
    let mut section = format!("## {}\n{}", labels.summary, intro);
    let change_sentence = match language {
        ReportLanguage::Chinese => {
            if structural_count == 0 {
                labels.no_change.to_owned()
            } else {
                format!("发现 {} 条已确认的组织变化。", structural_count)
            }
        }
        ReportLanguage::Japanese => {
            if structural_count == 0 {
                labels.no_change.to_owned()
            } else {
                format!("確認済みの組織変化が {} 件あります。", structural_count)
            }
        }
        ReportLanguage::English => {
            if structural_count == 0 {
                labels.no_change.to_owned()
            } else {
                format!(
                    "{} confirmed organizational changes were found.",
                    structural_count
                )
            }
        }
    };
    let data_sentence = match language {
        ReportLanguage::Chinese => format!(
            "- {}：{} 条待核实线索，{} 条无法确定，{} 条暂不可用。",
            labels.data_status, health.unconfirmed, health.unknown, health.unavailable
        ),
        ReportLanguage::Japanese => format!(
            "- {}：未確認 {} 件、判定不能 {} 件、取得できず {} 件。",
            labels.data_status, health.unconfirmed, health.unknown, health.unavailable
        ),
        ReportLanguage::English => format!(
            "- {}: {} leads to verify, {} could not be determined, {} unavailable.",
            labels.data_status, health.unconfirmed, health.unknown, health.unavailable
        ),
    };
    let separator = if language == ReportLanguage::English {
        ":"
    } else {
        "："
    };
    section.push_str(&format!(
        "\n- {}{separator}{}",
        labels.change_status, change_sentence
    ));
    section.push('\n');
    section.push_str(&data_sentence);
    let evidence_basis = facts
        .iter()
        .copied()
        .find(|fact| is_confirmed_primary(fact))
        .map(primary_evidence_uri)
        .unwrap_or_else(|| labels.no_primary.to_owned());
    if language == ReportLanguage::English {
        section.push_str(&format!(
            "\n- {}: {}",
            labels.evidence_basis, evidence_basis
        ));
    } else {
        section.push_str(&format!(
            "\n- {}：{}",
            labels.evidence_basis, evidence_basis
        ));
    }
    section
}

fn render_structural_changes(
    input: &RuntimeReportInput,
    facts: &[&NormalizedFact],
    language: ReportLanguage,
) -> Option<String> {
    let labels = labels(language);
    let mut changes = facts
        .iter()
        .copied()
        .filter(|fact| is_structural_change(fact.kind()) && fact.status() == &FactStatus::Known)
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

    let mut section = format!("## {}", labels.changes);
    for (index, fact) in changes.iter().enumerate() {
        let company = company_label(input, fact.company_id());
        let value = fact_value(fact);
        let evidence = evidence_line(fact, labels);
        if language == ReportLanguage::English {
            section.push_str(&format!(
                "\n### {}. {}\n- {}: {}\n- {}",
                index + 1,
                company,
                labels.field_structural_change,
                value,
                evidence
            ));
        } else {
            section.push_str(&format!(
                "\n### {}. {}\n- {}：{}\n- {}",
                index + 1,
                company,
                labels.field_structural_change,
                value,
                evidence
            ));
        }
    }
    Some(section)
}

fn render_top5(
    input: &RuntimeReportInput,
    facts: &[&NormalizedFact],
    language: ReportLanguage,
) -> Option<String> {
    let labels = labels(language);
    let mut companies = BTreeMap::<&str, Vec<&NormalizedFact>>::new();
    for fact in facts
        .iter()
        .copied()
        .filter(|fact| fact.status() == &FactStatus::Known)
    {
        companies.entry(fact.company_id()).or_default().push(fact);
    }
    if companies.is_empty() || companies.len() > MAX_COMPANY_CARDS {
        return None;
    }

    let mut section = format!("## {}", labels.company_observation);
    for (company_index, (company, mut company_facts)) in
        companies.into_iter().take(MAX_COMPANY_CARDS).enumerate()
    {
        company_facts.sort_by(|left, right| left.kind().cmp(right.kind()));
        section.push_str(&format!("\n### {}", company_label(input, company)));
        for fact in company_facts {
            let separator = if language == ReportLanguage::English {
                ":"
            } else {
                "："
            };
            section.push_str(&format!(
                "\n- {}{}{}\n  {}",
                fact_label(fact.kind(), labels),
                separator,
                fact_value(fact),
                evidence_line(fact, labels)
            ));
        }
        if company_index + 1 == MAX_COMPANY_CARDS {
            break;
        }
    }
    Some(section)
}

fn friendly_failure_reason(reason: &str, language: ReportLanguage) -> &'static str {
    let lower = reason.to_ascii_lowercase();
    match language {
        ReportLanguage::Chinese => {
            if lower.contains("json") {
                "返回资料格式无法识别"
            } else if lower.contains("response") {
                "返回资料不可用"
            } else {
                "请求未成功"
            }
        }
        ReportLanguage::Japanese => {
            if lower.contains("json") {
                "応答データを解釈できません"
            } else if lower.contains("response") {
                "応答データを取得できません"
            } else {
                "リクエストに失敗しました"
            }
        }
        ReportLanguage::English => {
            if lower.contains("json") {
                "response format could not be parsed"
            } else if lower.contains("response") {
                "response data was unavailable"
            } else {
                "request failed"
            }
        }
    }
}

fn coverage_line(item: &SourceCoverage, labels: Labels, language: ReportLanguage) -> String {
    let source = source_label(item.source(), labels);
    let total = item.expected();
    if item.available() == 0 && item.not_configured() == total {
        return match language {
            ReportLanguage::English => {
                format!(
                    "  - {source}: {} ({total} companies)",
                    labels.source_not_configured
                )
            }
            ReportLanguage::Japanese => {
                format!(
                    "  - {source}：{}（{} 社）",
                    labels.source_not_configured, total
                )
            }
            ReportLanguage::Chinese => {
                format!(
                    "  - {source}：{}（{} 家）",
                    labels.source_not_configured, total
                )
            }
        };
    }
    let separator = if language == ReportLanguage::English {
        ":"
    } else {
        "："
    };
    let mut detail = format!(
        "{source}{separator}{} {}",
        item.available(),
        labels.source_available
    );
    if item.unavailable() > 0 {
        let joiner = if language == ReportLanguage::English {
            ", "
        } else {
            "，"
        };
        detail.push_str(&format!(
            "{joiner}{} {}",
            item.unavailable(),
            labels.source_unavailable
        ));
    }
    if item.not_configured() > 0 {
        let joiner = if language == ReportLanguage::English {
            ", "
        } else {
            "，"
        };
        detail.push_str(&format!(
            "{joiner}{} {}",
            item.not_configured(),
            labels.source_not_configured
        ));
    }
    format!("  - {detail}")
}

fn render_health(
    input: &RuntimeReportInput,
    facts: &[&NormalizedFact],
    health: &SourceHealthFacts,
    language: ReportLanguage,
) -> String {
    let labels = labels(language);
    let mut section = match language {
        ReportLanguage::Chinese => format!(
            "## {}\n- {}：{} 条\n- {}：{} 条\n- {}：{} 条\n- {}：{} 条",
            labels.health,
            labels.confirmed_facts,
            health.confirmed,
            labels.pending_leads,
            health.unconfirmed,
            labels.unknown_facts,
            health.unknown,
            labels.unavailable_facts,
            health.unavailable
        ),
        ReportLanguage::Japanese => format!(
            "## {}\n- {}：{} 件\n- {}：{} 件\n- {}：{} 件\n- {}：{} 件",
            labels.health,
            labels.confirmed_facts,
            health.confirmed,
            labels.pending_leads,
            health.unconfirmed,
            labels.unknown_facts,
            health.unknown,
            labels.unavailable_facts,
            health.unavailable
        ),
        ReportLanguage::English => format!(
            "## {}\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}",
            labels.health,
            labels.confirmed_facts,
            health.confirmed,
            labels.pending_leads,
            health.unconfirmed,
            labels.unknown_facts,
            health.unknown,
            labels.unavailable_facts,
            health.unavailable
        ),
    };
    let colon = if language == ReportLanguage::English {
        ":"
    } else {
        "："
    };
    section.push_str(&format!("\n- {}{colon}", labels.source_status));
    let mut coverage = input.source_coverage().iter().collect::<Vec<_>>();
    coverage.sort_by(|left, right| left.source().cmp(right.source()));
    if coverage.is_empty() {
        section.push_str("\n  - —");
    } else {
        for item in coverage {
            section.push('\n');
            section.push_str(&coverage_line(item, labels, language));
        }
    }
    if health.discovery_only > 0 {
        section.push_str(&format!("\n- {}", labels.discovery_note));
    }
    if health.top5_selection_missing {
        section.push_str(&format!("\n- {}", labels.ranking_missing));
    }

    let mut groups = BTreeMap::<String, usize>::new();
    let mut not_configured = 0usize;
    for fact in facts {
        if fact.status() == &FactStatus::Unavailable {
            let passage = fact
                .provenance()
                .source_field_or_passage()
                .to_ascii_lowercase();
            if passage.contains("not configured") {
                not_configured += 1;
            } else {
                *groups
                    .entry(source_label(
                        source_kind_for_fact(fact.kind()).unwrap_or("other"),
                        labels,
                    ))
                    .or_default() += 1;
            }
        }
    }
    if !health.source_failures.is_empty() {
        let mut failures = BTreeMap::<(String, String), usize>::new();
        for failure in &health.source_failures {
            *failures
                .entry((
                    source_label(failure.source(), labels),
                    friendly_failure_reason(failure.reason(), language).to_owned(),
                ))
                .or_default() += 1;
        }
        for ((source, reason), count) in failures {
            *groups.entry(format!("{source}（{reason}）")).or_default() += count;
        }
    }
    section.push_str(&format!("\n- {}{colon}", labels.attention));
    let mut group_items = groups.into_iter().collect::<Vec<_>>();
    group_items.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    if group_items.is_empty() && not_configured == 0 {
        section.push_str(&format!("\n  - {}", labels.no_attention));
    } else {
        for (group, count) in group_items.into_iter().take(MAX_HEALTH_GROUPS) {
            let unit = match language {
                ReportLanguage::Chinese => "项",
                ReportLanguage::Japanese => "件",
                ReportLanguage::English => "items",
            };
            let separator = if language == ReportLanguage::English {
                ":"
            } else {
                "："
            };
            section.push_str(&format!("\n  - {group}{separator}{count} {unit}"));
        }
        if not_configured > 0 {
            let unit = match language {
                ReportLanguage::Chinese => "项",
                ReportLanguage::Japanese => "件",
                ReportLanguage::English => "items",
            };
            section.push_str(&format!(
                "\n  - {}{colon}{} {}",
                labels.source_not_configured, not_configured, unit
            ));
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

fn snapshot_company(company: &CompanyIdentity) -> SnapshotCompany {
    SnapshotCompany {
        id: safe_text(company.id()),
        name: safe_text(company.name()),
        ticker: safe_text(company.ticker()),
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

/// Renders the default Chinese Weekly Radar report.
pub fn render_report(input: &RuntimeReportInput) -> RenderedReport {
    render_report_in_language(input, ReportLanguage::default())
}

/// Renders one deterministic human-first report in the requested language.
pub fn render_report_in_language(
    input: &RuntimeReportInput,
    language: ReportLanguage,
) -> RenderedReport {
    let mut facts = input.facts().iter().collect::<Vec<_>>();
    facts.sort_by(|left, right| {
        left.company_id()
            .cmp(right.company_id())
            .then_with(|| left.kind().cmp(right.kind()))
    });
    let company_count = if input.companies().is_empty() {
        facts
            .iter()
            .map(|fact| fact.company_id())
            .collect::<BTreeSet<_>>()
            .len()
    } else {
        input.companies().len()
    };
    let health = build_health(input, &facts);
    let structural_count = facts
        .iter()
        .filter(|fact| is_structural_change(fact.kind()) && fact.status() == &FactStatus::Known)
        .count();
    let mut sections = vec![render_executive_summary(
        input,
        &facts,
        &health,
        structural_count,
        company_count,
        language,
    )];
    if let Some(section) = render_structural_changes(input, &facts, language) {
        sections.push(section);
    }
    if let Some(section) = render_top5(input, &facts, language) {
        sections.push(section);
    }
    sections.push(render_health(input, &facts, &health, language));
    let markdown = sections.join("\n\n") + "\n";

    let metadata = SnapshotMetadata {
        as_of: input.as_of(),
        fact_count: facts.len(),
        company_count,
    };
    let mut snapshot_facts = facts.into_iter().map(snapshot_fact).collect::<Vec<_>>();
    snapshot_facts.sort_by(|left, right| {
        left.company_id
            .cmp(&right.company_id)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    let mut companies = input
        .companies()
        .iter()
        .map(snapshot_company)
        .collect::<Vec<_>>();
    companies.sort_by(|left, right| left.id.cmp(&right.id));
    let mut source_coverage = input
        .source_coverage()
        .iter()
        .map(|item| SnapshotCoverage {
            source: safe_text(item.source()),
            expected: item.expected(),
            available: item.available(),
            not_configured: item.not_configured(),
            unavailable: item.unavailable(),
        })
        .collect::<Vec<_>>();
    source_coverage.sort_by(|left, right| left.source.cmp(&right.source));
    let snapshot_json = serde_json::to_string_pretty(&SnapshotDocument {
        language: language.as_str().to_owned(),
        metadata: metadata.clone(),
        companies,
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

impl fmt::Display for ReportLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
