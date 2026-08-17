use org_x::features::reporting::domain::{
    CompanyReference as ReportingCompany, ReportSection, ResearchCard, ResearchCardId,
    ResearchPacket, Top5 as ResearchTop5,
};
use org_x::features::weekly_radar::domain::change_compression::{
    CompanyReference as ChangeCompany, DroppedChange, EventId, FactValue as ChangeFact,
    ImportantStructuralChange, PeriodId, RisingChange, StageTransitionChange, Top5Change,
    WeeklyChangeCompression, WeeklyChangeInput,
};
use org_x::features::weekly_radar::domain::system_health::{
    CompanyReference as HealthCompany, DegradedCompany, EvidenceCoverage, ExtractionFailure,
    FailureId, Freshness, HealthStatus, Reason, SourceCoverage, SourceReference, SystemHealth,
};
use org_x::features::weekly_radar::domain::top5_weekly_read_model::{
    CandidateId, Company, Confidence, Direction, KeyChange, NextStep, Stage, Top5Entry,
    Top5WeeklyReadModel,
};
use org_x::features::weekly_radar::domain::{
    AsOf, EvidenceCutoff, ModelVersion, ScoringVersion, SnapshotId, UniverseSnapshotId,
    WeeklyRadarSnapshot,
};
use org_x::features::weekly_radar::interface::markdown_renderer::{
    MarkdownRenderer, MarkdownReportInput, RankChange, StageHistoryEntry,
};

struct Fixtures {
    snapshot: WeeklyRadarSnapshot,
    top5: Top5WeeklyReadModel,
    research: ResearchPacket,
    compression: WeeklyChangeCompression,
    stage_history: Vec<StageHistoryEntry>,
    rank_changes: Vec<RankChange>,
    system_health: SystemHealth,
}

fn snapshot() -> WeeklyRadarSnapshot {
    WeeklyRadarSnapshot::new(
        SnapshotId::new("snapshot-renderer").unwrap(),
        AsOf::new("2026-08-17").unwrap(),
        UniverseSnapshotId::new("universe-renderer").unwrap(),
        EvidenceCutoff::new("cutoff-renderer").unwrap(),
        ModelVersion::new("model-renderer").unwrap(),
        ScoringVersion::new("scoring-renderer").unwrap(),
    )
    .unwrap()
}

fn top5_entry(id: &str) -> Top5Entry {
    Top5Entry::new(
        CandidateId::new(id).unwrap(),
        Company::new(format!("company-{id}")).unwrap(),
        Stage::new(format!("stage-{id}")).unwrap(),
        Direction::new(format!("direction-{id}")).unwrap(),
        Confidence::new(format!("confidence-{id}")).unwrap(),
        KeyChange::new(format!("key-change-{id}")).unwrap(),
        NextStep::new(format!("next-{id}")).unwrap(),
    )
    .unwrap()
}

fn research_card(id: &str) -> ResearchCard {
    ResearchCard::new(
        ResearchCardId::new(id).unwrap(),
        ReportingCompany::new(format!("research-company-{id}")).unwrap(),
        format!("research-stage-{id}"),
        format!("headline-{id}"),
        format!("evidence-{id}"),
        format!("counter-{id}"),
        format!("missing-{id}"),
        format!("next-research-{id}"),
    )
    .unwrap()
}

fn change_period() -> PeriodId {
    PeriodId::new("2026-W33").unwrap()
}

fn change_event_id(id: &str) -> EventId {
    EventId::new(id).unwrap()
}

fn change_company(value: &str) -> ChangeCompany {
    ChangeCompany::new(value).unwrap()
}

fn change_fact(value: &str) -> ChangeFact {
    ChangeFact::new(value).unwrap()
}

fn full_compression() -> WeeklyChangeCompression {
    WeeklyChangeCompression::from_input(
        WeeklyChangeInput::new(
            change_period(),
            vec![ImportantStructuralChange::new(
                change_event_id("important-1"),
                change_period(),
                change_company("important-company-1"),
                change_fact("important-fact-1"),
            )
            .unwrap()],
            vec![Top5Change::new(
                change_event_id("top5-change-1"),
                change_period(),
                change_company("top5-change-company-1"),
                change_fact("top5-change-fact-1"),
            )
            .unwrap()],
            vec![StageTransitionChange::new(
                change_event_id("transition-1"),
                change_period(),
                change_company("transition-company-1"),
                change_fact("transition-fact-1"),
            )
            .unwrap()],
            vec![RisingChange::new(
                change_event_id("rising-1"),
                change_period(),
                change_company("rising-company-1"),
                change_fact("rising-fact-1"),
            )
            .unwrap()],
            vec![DroppedChange::new(
                change_event_id("dropped-1"),
                change_period(),
                change_company("dropped-company-1"),
                change_fact("dropped-fact-1"),
            )
            .unwrap()],
        )
        .unwrap(),
    )
    .unwrap()
}

fn empty_compression() -> WeeklyChangeCompression {
    WeeklyChangeCompression::from_input(
        WeeklyChangeInput::new(
            change_period(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    )
    .unwrap()
}

fn full_research() -> ResearchPacket {
    let mut top5 = ResearchTop5::new();
    top5.add(research_card("card-2")).unwrap();
    top5.add(research_card("card-1")).unwrap();

    let mut rising = ReportSection::new();
    rising.add(research_card("rising-card-1")).unwrap();

    let mut watch = ReportSection::new();
    watch.add(research_card("watch-card-1")).unwrap();

    let mut dropped = ReportSection::new();
    dropped.add(research_card("dropped-card-1")).unwrap();

    ResearchPacket::new("executive-summary-renderer", top5, rising, watch, dropped).unwrap()
}

fn empty_research() -> ResearchPacket {
    ResearchPacket::new(
        "empty-executive-summary",
        ResearchTop5::new(),
        ReportSection::new(),
        ReportSection::new(),
        ReportSection::new(),
    )
    .unwrap()
}

fn full_system_health() -> SystemHealth {
    let mut health = SystemHealth::new(
        HealthStatus::Degraded,
        EvidenceCoverage::new(2, 3, 88).unwrap(),
        Freshness::Aging,
    );
    health
        .add_degraded_company(DegradedCompany::new(
            HealthCompany::new("degraded-company-1").unwrap(),
            Reason::new("degraded-reason-1").unwrap(),
        ))
        .unwrap();
    health
        .add_source_coverage(
            SourceCoverage::new(SourceReference::new("source-2").unwrap(), 2, 3, 66).unwrap(),
        )
        .unwrap();
    health
        .add_source_coverage(
            SourceCoverage::new(SourceReference::new("source-1").unwrap(), 1, 3, 33).unwrap(),
        )
        .unwrap();
    health
        .add_extraction_failure(ExtractionFailure::new(
            FailureId::new("failure-1").unwrap(),
            SourceReference::new("failure-source-1").unwrap(),
            Reason::new("failure-reason-1").unwrap(),
        ))
        .unwrap();
    health
}

fn full_fixtures() -> Fixtures {
    Fixtures {
        snapshot: snapshot(),
        top5: Top5WeeklyReadModel::from_entries([
            top5_entry("candidate-2"),
            top5_entry("candidate-1"),
        ])
        .unwrap(),
        research: full_research(),
        compression: full_compression(),
        stage_history: vec![
            StageHistoryEntry::new(
                "history-2",
                "2026-W33",
                "history-company-2",
                "stage-before-2",
                "stage-after-2",
                "stage-history-fact-2",
            )
            .unwrap(),
            StageHistoryEntry::new(
                "history-1",
                "2026-W33",
                "history-company-1",
                "stage-before-1",
                "stage-after-1",
                "stage-history-fact-1",
            )
            .unwrap(),
        ],
        rank_changes: vec![
            RankChange::new(
                "rank-2",
                "2026-W33",
                "rank-company-2",
                Some(4),
                Some(2),
                "rank-change-fact-2",
            )
            .unwrap(),
            RankChange::new(
                "rank-1",
                "2026-W33",
                "rank-company-1",
                None,
                Some(5),
                "rank-change-fact-1",
            )
            .unwrap(),
        ],
        system_health: full_system_health(),
    }
}

fn empty_fixtures() -> Fixtures {
    Fixtures {
        snapshot: snapshot(),
        top5: Top5WeeklyReadModel::new(),
        research: empty_research(),
        compression: empty_compression(),
        stage_history: Vec::new(),
        rank_changes: Vec::new(),
        system_health: full_system_health(),
    }
}

#[test]
fn full_report_preserves_supplied_facts_and_fixed_section_order() {
    let fixtures = full_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &fixtures.stage_history,
        &fixtures.rank_changes,
        Some(&fixtures.system_health),
    );

    let markdown = MarkdownRenderer::render(&input).as_str().to_owned();
    let markers = [
        "# Weekly Radar Markdown Report",
        "## Snapshot",
        "## Change Compression",
        "## Top5",
        "## Research Cards",
        "## Evidence",
        "## Counter Evidence",
        "## Missing Proof",
        "## Stage History",
        "## Rank Changes",
        "## System Health",
    ];
    let positions: Vec<_> = markers
        .iter()
        .map(|marker| markdown.find(marker).expect("section marker must exist"))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(markdown.contains("candidate-2"));
    assert!(markdown.find("candidate-2").unwrap() < markdown.find("candidate-1").unwrap());
    assert!(markdown.contains("important-fact-1"));
    assert!(markdown.contains("top5-change-fact-1"));
    assert!(markdown.contains("transition-fact-1"));
    assert!(markdown.contains("rising-fact-1"));
    assert!(markdown.contains("dropped-fact-1"));
    assert!(markdown.contains("headline-card-2"));
    assert!(markdown.contains("evidence-card-2"));
    assert!(markdown.contains("counter-card-1"));
    assert!(markdown.contains("missing-card-1"));
    assert!(markdown.find("history-2").unwrap() < markdown.find("history-1").unwrap());
    assert!(markdown.find("rank-2").unwrap() < markdown.find("rank-1").unwrap());
    assert!(markdown.find("source-2").unwrap() < markdown.find("source-1").unwrap());
    assert!(markdown.contains("stage-history-fact-2"));
    assert!(markdown.contains("rank-change-fact-2"));
    assert!(markdown.contains("DEGRADED"));
    assert!(markdown.contains("AGING"));
}

#[test]
fn empty_report_keeps_no_change_and_absent_health_explicit() {
    let fixtures = empty_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &[],
        &[],
        None,
    );
    let markdown = MarkdownRenderer::render(&input).as_str().to_owned();

    assert!(markdown.contains("NO_CHANGE"));
    assert!(markdown.contains("Important Structural Change Count: 0"));
    assert!(markdown.contains("Top5 Change Count: 0"));
    assert!(markdown.contains("Stage Transition Count: 0"));
    assert!(markdown.contains("Rising Count: 0"));
    assert!(markdown.contains("Dropped Count: 0"));
    assert!(markdown.contains("Stage History: EMPTY"));
    assert!(markdown.contains("Rank Changes: EMPTY"));
    assert!(markdown.contains("NOT_SUPPLIED"));
}

#[test]
fn rendering_the_same_input_twice_is_byte_identical() {
    let fixtures = full_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &fixtures.stage_history,
        &fixtures.rank_changes,
        Some(&fixtures.system_health),
    );
    let first = MarkdownRenderer::render(&input);
    let second = MarkdownRenderer::render(&input);

    assert_eq!(first.as_str(), second.as_str());
}

#[test]
fn renderer_source_has_no_recomputation_or_external_delivery_boundary() {
    let source = include_str!("../src/features/weekly_radar/interface/markdown_renderer.rs");
    let lowered = source.to_ascii_lowercase();
    for forbidden in [
        "sort_by",
        "sort_unstable",
        "rank_by",
        "calculate_stage",
        "calculate_rank",
        "calculate_distance",
        "calculate_score",
        "telegram",
        "http",
        "reqwest",
        "sqlx",
        "std::net",
        "std::fs",
        "credential",
    ] {
        assert!(!lowered.contains(forbidden), "forbidden token: {forbidden}");
    }
}
