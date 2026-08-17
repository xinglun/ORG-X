#[path = "../src/features/weekly_radar/domain/change_compression.rs"]
mod change_compression;

use change_compression::{
    ChangeCompressionError, CompanyReference, CompressionSection, DroppedChange, EventId,
    FactValue, ImportantStructuralChange, PeriodId, RisingChange, StageTransitionChange,
    Top5Change, WeeklyChangeCompression, WeeklyChangeInput,
};

fn period(value: &str) -> PeriodId {
    PeriodId::new(value).expect("period should be valid")
}

fn event(value: &str) -> EventId {
    EventId::new(value).expect("event should be valid")
}

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company should be valid")
}

fn fact(value: &str) -> FactValue {
    FactValue::new(value).expect("fact should be valid")
}

fn important(
    id: &str,
    period_value: &str,
    company_value: &str,
    fact_value: &str,
) -> ImportantStructuralChange {
    ImportantStructuralChange::new(
        event(id),
        period(period_value),
        company(company_value),
        fact(fact_value),
    )
    .expect("important structural change should be valid")
}

fn top5(id: &str, period_value: &str, company_value: &str, fact_value: &str) -> Top5Change {
    Top5Change::new(
        event(id),
        period(period_value),
        company(company_value),
        fact(fact_value),
    )
    .expect("Top5 change should be valid")
}

fn transition(
    id: &str,
    period_value: &str,
    company_value: &str,
    fact_value: &str,
) -> StageTransitionChange {
    StageTransitionChange::new(
        event(id),
        period(period_value),
        company(company_value),
        fact(fact_value),
    )
    .expect("stage transition should be valid")
}

fn rising(id: &str, period_value: &str, company_value: &str, fact_value: &str) -> RisingChange {
    RisingChange::new(
        event(id),
        period(period_value),
        company(company_value),
        fact(fact_value),
    )
    .expect("Rising change should be valid")
}

fn dropped(id: &str, period_value: &str, company_value: &str, fact_value: &str) -> DroppedChange {
    DroppedChange::new(
        event(id),
        period(period_value),
        company(company_value),
        fact(fact_value),
    )
    .expect("Dropped change should be valid")
}

#[test]
fn compression_preserves_each_explicit_section_and_input_order() {
    let input = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![
            important("structural-1", "2026-W33", "Acme", "workflow transfer"),
            important("structural-2", "2026-W33", "Beta", "agent supervision"),
        ],
        vec![top5("top5-1", "2026-W33", "Gamma", "entered supplied Top5")],
        vec![transition(
            "transition-1",
            "2026-W33",
            "Delta",
            "WORKFLOW -> PRODUCTION_SYSTEM",
        )],
        vec![rising(
            "rising-1",
            "2026-W33",
            "Epsilon",
            "supporting evidence strengthened",
        )],
        vec![dropped(
            "dropped-1",
            "2026-W33",
            "Foxtrot",
            "counter evidence invalidated",
        )],
    )
    .expect("explicit input should validate");

    let compression =
        WeeklyChangeCompression::from_input(input).expect("compression should validate");

    assert_eq!(compression.period().as_str(), "2026-W33");
    assert_eq!(
        compression.important_structural()[0].event_id().as_str(),
        "structural-1"
    );
    assert_eq!(
        compression.important_structural()[0].period().as_str(),
        "2026-W33"
    );
    assert_eq!(
        compression.important_structural()[0].company().as_str(),
        "Acme"
    );
    assert_eq!(
        compression.important_structural()[1].fact().as_str(),
        "agent supervision"
    );
    assert_eq!(compression.top5()[0].event_id().as_str(), "top5-1");
    assert_eq!(compression.top5()[0].period().as_str(), "2026-W33");
    assert_eq!(compression.top5()[0].company().as_str(), "Gamma");
    assert_eq!(
        compression.top5()[0].fact().as_str(),
        "entered supplied Top5"
    );
    assert_eq!(
        compression.stage_transitions()[0].event_id().as_str(),
        "transition-1"
    );
    assert_eq!(
        compression.stage_transitions()[0].period().as_str(),
        "2026-W33"
    );
    assert_eq!(
        compression.stage_transitions()[0].company().as_str(),
        "Delta"
    );
    assert_eq!(
        compression.stage_transitions()[0].fact().as_str(),
        "WORKFLOW -> PRODUCTION_SYSTEM"
    );
    assert_eq!(compression.rising()[0].event_id().as_str(), "rising-1");
    assert_eq!(compression.rising()[0].period().as_str(), "2026-W33");
    assert_eq!(compression.rising()[0].company().as_str(), "Epsilon");
    assert_eq!(
        compression.rising()[0].fact().as_str(),
        "supporting evidence strengthened"
    );
    assert_eq!(compression.dropped()[0].event_id().as_str(), "dropped-1");
    assert_eq!(compression.dropped()[0].period().as_str(), "2026-W33");
    assert_eq!(compression.dropped()[0].company().as_str(), "Foxtrot");
    assert_eq!(
        compression.dropped()[0].fact().as_str(),
        "counter evidence invalidated"
    );
    assert!(compression.no_change().is_none());

    let sections = compression.sections();
    assert_eq!(sections.len(), 6);
    assert!(matches!(
        sections[0],
        CompressionSection::ImportantStructural(_)
    ));
    assert!(matches!(sections[1], CompressionSection::Top5(_)));
    assert!(matches!(
        sections[2],
        CompressionSection::StageTransition(_)
    ));
    assert!(matches!(sections[3], CompressionSection::Rising(_)));
    assert!(matches!(sections[4], CompressionSection::Dropped(_)));
    assert!(matches!(sections[5], CompressionSection::NoChange(None)));
}

#[test]
fn empty_input_emits_stable_no_change_without_narrative() {
    let compression = WeeklyChangeCompression::from_input(
        WeeklyChangeInput::new(
            period("2026-W33"),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("empty input should validate"),
    )
    .expect("empty compression should validate");

    let no_change = compression
        .no_change()
        .expect("No Change should be emitted");
    assert_eq!(no_change.label(), "NO_CHANGE");
    assert_eq!(no_change.period().as_str(), "2026-W33");
    assert_eq!(no_change.counts().important_structural(), 0);
    assert_eq!(no_change.counts().top5(), 0);
    assert_eq!(no_change.counts().stage_transitions(), 0);
    assert_eq!(no_change.counts().rising(), 0);
    assert_eq!(no_change.counts().dropped(), 0);
    assert!(matches!(
        compression.sections()[5],
        CompressionSection::NoChange(Some(_))
    ));
}

#[test]
fn duplicate_identity_and_period_mismatch_are_rejected_before_compression() {
    let duplicate = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![important("same-event", "2026-W33", "Acme", "first")],
        vec![top5("same-event", "2026-W33", "Beta", "second")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("duplicate identity should be rejected");
    assert_eq!(
        duplicate,
        ChangeCompressionError::DuplicateIdentity {
            id: "same-event".to_owned(),
        }
    );

    let mismatch = WeeklyChangeInput::new(
        period("2026-W33"),
        vec![important("wrong-period", "2026-W34", "Acme", "fact")],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("period mismatch should be rejected");
    assert_eq!(
        mismatch,
        ChangeCompressionError::PeriodMismatch {
            expected: "2026-W33".to_owned(),
            actual: "2026-W34".to_owned(),
        }
    );
}

#[test]
fn blank_values_are_rejected_and_standalone_source_has_no_cross_feature_boundary() {
    assert_eq!(
        EventId::new(" ").expect_err("blank event must be rejected"),
        ChangeCompressionError::EmptyValue { field: "event id" }
    );
    assert!(CompanyReference::new("\t").is_err());

    let source = include_str!("../src/features/weekly_radar/domain/change_compression.rs");
    assert!(!source.contains("use crate::"));
    assert!(!source.contains("features::"));
    assert!(!source.contains("WeeklyRadarSnapshot"));
    assert!(!source.contains("telegram"));
}
