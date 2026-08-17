#[path = "../src/features/weekly_radar/domain/rising_dropped.rs"]
mod rising_dropped;

use rising_dropped::{
    derive_event, CompanyReference, DomainError, EventId, EventKind, EvidenceId, EvidenceSet,
    NextStep, PeriodId, Reason, ResearchState, StageLabel, StructuralChangeKind,
    StructuralEvidenceDelta, WeeklyChangeSet,
};

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("test company should be valid")
}

fn period(value: &str) -> PeriodId {
    PeriodId::new(value).expect("test period should be valid")
}

fn event_id(value: &str) -> EventId {
    EventId::new(value).expect("test event id should be valid")
}

fn evidence(value: &str) -> EvidenceId {
    EvidenceId::new(value).expect("test evidence id should be valid")
}

fn stage(value: &str) -> StageLabel {
    StageLabel::new(value).expect("test stage should be valid")
}

fn details(prefix: &str) -> rising_dropped::EvidenceDeltaDetails {
    let supporting = EvidenceSet::new(vec![evidence(&format!("{prefix}-supporting-1"))])
        .expect("supporting evidence should be valid");
    let counter = EvidenceSet::new(vec![evidence(&format!("{prefix}-counter-1"))])
        .expect("counter evidence should be valid");
    let missing = EvidenceSet::new(vec![evidence(&format!("{prefix}-missing-1"))])
        .expect("missing evidence should be valid");
    let details = rising_dropped::EvidenceDeltaDetails::new(
        Reason::new(format!("{prefix} structural evidence changed"))
            .expect("reason should be valid"),
        supporting,
        counter,
        missing,
        NextStep::new(format!("{prefix} next research step")).expect("next step should be valid"),
    )
    .expect("evidence details should be valid");
    let _ = (
        details.reason(),
        details.supporting(),
        details.counter(),
        details.missing(),
        details.next(),
    );
    details
}

fn state(company_name: &str, stage_name: &str) -> ResearchState {
    let state = ResearchState::new(company(company_name), stage(stage_name));
    let _ = (state.company(), state.stage());
    state
}

#[test]
fn strengthened_delta_emits_rising_without_ranking_inputs_and_preserves_facts() {
    let event = derive_event(
        event_id("evt-rising-1"),
        period("2026-W33"),
        state("Acme", "WORKFLOW"),
        state("Acme", "PRODUCTION_SYSTEM"),
        StructuralEvidenceDelta::strengthened(details("workflow transfer")),
    )
    .expect("matching states should produce a result")
    .expect("structural strengthening should produce Rising");

    assert_eq!(event.kind(), EventKind::Rising);
    assert_eq!(event.event_id().as_str(), "evt-rising-1");
    assert_eq!(event.period().as_str(), "2026-W33");
    assert_eq!(event.company().as_str(), "Acme");
    assert_eq!(event.previous_stage().as_str(), "WORKFLOW");
    assert_eq!(event.current_stage().as_str(), "PRODUCTION_SYSTEM");
    assert_eq!(
        event.reason().as_str(),
        "workflow transfer structural evidence changed"
    );
    assert_eq!(
        event.supporting().ids()[0].as_str(),
        "workflow transfer-supporting-1"
    );
    assert_eq!(
        event.counter().ids()[0].as_str(),
        "workflow transfer-counter-1"
    );
    assert_eq!(
        event.missing().ids()[0].as_str(),
        "workflow transfer-missing-1"
    );
    assert_eq!(
        event.next().as_str(),
        "workflow transfer next research step"
    );
}

#[test]
fn weakened_structural_delta_emits_dropped_and_preserves_counter_and_missing_facts() {
    let event = derive_event(
        event_id("evt-dropped-1"),
        period("2026-W33"),
        state("Beta", "PRODUCTION_SYSTEM"),
        state("Beta", "WORKFLOW"),
        StructuralEvidenceDelta::weakened(details("production system persistence")),
    )
    .expect("matching states should produce a result")
    .expect("structural weakening should produce Dropped");

    assert_eq!(event.kind(), EventKind::Dropped);
    assert_eq!(event.company().as_str(), "Beta");
    assert_eq!(event.previous_stage().as_str(), "PRODUCTION_SYSTEM");
    assert_eq!(event.current_stage().as_str(), "WORKFLOW");
    assert_eq!(
        event.counter().ids()[0].as_str(),
        "production system persistence-counter-1"
    );
    assert_eq!(
        event.missing().ids()[0].as_str(),
        "production system persistence-missing-1"
    );
}

#[test]
fn invalidated_structural_delta_emits_dropped_for_counter_evidence() {
    let event = derive_event(
        event_id("evt-invalidated-1"),
        period("2026-W33"),
        state("Gamma", "PRODUCTIVITY_BREAKOUT"),
        state("Gamma", "PRODUCTIVITY_BREAKOUT"),
        StructuralEvidenceDelta::invalidated(details("counter evidence")),
    )
    .expect("matching states should produce a result")
    .expect("invalidated evidence should produce Dropped");

    assert_eq!(event.kind(), EventKind::Dropped);
    assert_eq!(
        event.reason().as_str(),
        "counter evidence structural evidence changed"
    );
}

#[test]
fn price_rank_and_score_only_changes_do_not_emit_events() {
    for change in [
        StructuralChangeKind::PriceOnly,
        StructuralChangeKind::RankOnly,
        StructuralChangeKind::ScoreOnly,
    ] {
        assert_eq!(
            derive_event(
                event_id("evt-non-structural"),
                period("2026-W33"),
                state("Delta", "WORKFLOW"),
                state("Delta", "WORKFLOW"),
                StructuralEvidenceDelta::non_structural(change),
            )
            .expect("non-structural changes should be accepted"),
            None
        );
    }
}

#[test]
fn unchanged_state_does_not_fabricate_a_change() {
    assert_eq!(
        derive_event(
            event_id("evt-unchanged"),
            period("2026-W33"),
            state("Epsilon", "WORKFLOW"),
            state("Epsilon", "WORKFLOW"),
            StructuralEvidenceDelta::unchanged(),
        )
        .expect("unchanged state should be accepted"),
        None
    );
}

#[test]
fn mismatched_previous_and_current_company_is_rejected() {
    assert_eq!(
        derive_event(
            event_id("evt-company-mismatch"),
            period("2026-W33"),
            state("Foxtrot", "WORKFLOW"),
            state("Golf", "WORKFLOW"),
            StructuralEvidenceDelta::strengthened(details("company mismatch")),
        ),
        Err(DomainError::CompanyMismatch {
            previous: "Foxtrot".to_owned(),
            current: "Golf".to_owned(),
        })
    );
}

#[test]
fn weekly_change_set_preserves_section_order_and_rejects_same_period_conflicts() {
    let first = derive_event(
        event_id("evt-order-1"),
        period("2026-W33"),
        state("Hotel", "WORKFLOW"),
        state("Hotel", "PRODUCTION_SYSTEM"),
        StructuralEvidenceDelta::strengthened(details("first")),
    )
    .expect("first event should be valid")
    .expect("first event should be Rising");
    let second = derive_event(
        event_id("evt-order-2"),
        period("2026-W33"),
        state("India", "PRODUCTION_SYSTEM"),
        state("India", "WORKFLOW"),
        StructuralEvidenceDelta::weakened(details("second")),
    )
    .expect("second event should be valid")
    .expect("second event should be Dropped");
    let duplicate_event_id = derive_event(
        event_id("evt-order-1"),
        period("2026-W33"),
        state("Juliet", "WORKFLOW"),
        state("Juliet", "PRODUCTION_SYSTEM"),
        StructuralEvidenceDelta::strengthened(details("duplicate event")),
    )
    .expect("duplicate event candidate should be valid")
    .expect("duplicate event candidate should be Rising");
    let duplicate_company = derive_event(
        event_id("evt-order-3"),
        period("2026-W33"),
        state("Hotel", "PRODUCTION_SYSTEM"),
        state("Hotel", "WORKFLOW"),
        StructuralEvidenceDelta::weakened(details("cross-kind conflict")),
    )
    .expect("conflicting company candidate should be valid")
    .expect("conflicting company candidate should be Dropped");

    let mut changes = WeeklyChangeSet::new(period("2026-W33"));
    assert_eq!(changes.period().as_str(), "2026-W33");
    changes.add(first).expect("first event should be accepted");
    changes
        .add(second)
        .expect("second event should be accepted");
    assert_eq!(changes.rising()[0].event_id().as_str(), "evt-order-1");
    assert_eq!(changes.dropped()[0].event_id().as_str(), "evt-order-2");
    assert_eq!(
        changes.add(duplicate_event_id),
        Err(DomainError::DuplicateIdentity {
            entity: "weekly change event",
            id: "evt-order-1".to_owned(),
        })
    );
    assert_eq!(
        changes.add(duplicate_company),
        Err(DomainError::CompanyPeriodConflict {
            period: "2026-W33".to_owned(),
            company: "Hotel".to_owned(),
        })
    );
}

#[test]
fn evidence_duplicates_and_cross_collection_overlap_are_rejected() {
    assert_eq!(
        EvidenceSet::new(vec![evidence("same"), evidence("same")]),
        Err(DomainError::DuplicateIdentity {
            entity: "evidence",
            id: "same".to_owned()
        })
    );

    let supporting =
        EvidenceSet::new(vec![evidence("shared")]).expect("supporting should be valid");
    let counter = EvidenceSet::new(vec![evidence("shared")]).expect("counter should be valid");
    let missing = EvidenceSet::empty();
    assert_eq!(
        rising_dropped::EvidenceDeltaDetails::new(
            Reason::new("overlap reason").expect("reason should be valid"),
            supporting,
            counter,
            missing,
            NextStep::new("overlap next").expect("next should be valid"),
        ),
        Err(DomainError::OverlappingEvidence {
            id: "shared".to_owned()
        })
    );
}

#[test]
fn adding_an_event_for_another_period_is_rejected() {
    let event = derive_event(
        event_id("evt-period-mismatch"),
        period("2026-W32"),
        state("Kilo", "WORKFLOW"),
        state("Kilo", "PRODUCTION_SYSTEM"),
        StructuralEvidenceDelta::strengthened(details("period mismatch")),
    )
    .expect("period mismatch candidate should be valid")
    .expect("period mismatch candidate should be Rising");
    let mut changes = WeeklyChangeSet::new(period("2026-W33"));

    assert_eq!(
        changes.add(event),
        Err(DomainError::PeriodMismatch {
            expected: "2026-W33".to_owned(),
            actual: "2026-W32".to_owned(),
        })
    );
}
