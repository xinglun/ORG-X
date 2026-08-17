#[path = "../src/features/weekly_radar/domain/stage_transition_output.rs"]
mod stage_transition_output;

use stage_transition_output::{
    CompanyReference, Confidence, EvidenceReference, MissingEvidence, StageLabel,
    StageTransitionOutput, TransitionDate, TransitionEventId, TransitionPriority, TransitionStatus,
};

fn event_id(value: &str) -> TransitionEventId {
    TransitionEventId::new(value).expect("event identity should be valid")
}

fn company(value: &str) -> CompanyReference {
    CompanyReference::new(value).expect("company reference should be valid")
}

fn stage(value: &str) -> StageLabel {
    StageLabel::new(value).expect("stage label should be valid")
}

fn date(value: &str) -> TransitionDate {
    TransitionDate::new(value).expect("transition date should be valid")
}

fn confidence(value: &str) -> Confidence {
    Confidence::new(value).expect("confidence should be valid")
}

fn supporting(id: &str, description: &str) -> EvidenceReference {
    EvidenceReference::new(id, description).expect("supporting evidence should be valid")
}

fn missing(id: &str, requirement: &str) -> MissingEvidence {
    MissingEvidence::new(id, requirement).expect("missing evidence should be valid")
}

#[test]
fn explicit_status_and_all_supplied_fields_are_retained() {
    let mut confirmed = StageTransitionOutput::new(
        event_id("event-confirmed"),
        company("acme"),
        stage("WORKFLOW"),
        stage("PRODUCTION_SYSTEM"),
        date("2026-08-17"),
        TransitionStatus::Confirmed,
        confidence("HIGH"),
    );
    confirmed
        .add_supporting(supporting("support-1", "workflow transfer"))
        .unwrap();
    confirmed
        .add_counter(supporting("counter-1", "legacy path remains"))
        .unwrap();
    confirmed
        .add_missing(missing("missing-1", "multiple quarters"))
        .unwrap();

    assert_eq!(confirmed.event_id().as_str(), "event-confirmed");
    assert_eq!(confirmed.company().as_str(), "acme");
    assert_eq!(confirmed.prior_stage().as_str(), "WORKFLOW");
    assert_eq!(confirmed.to_stage().as_str(), "PRODUCTION_SYSTEM");
    assert_eq!(confirmed.transition_date().as_str(), "2026-08-17");
    assert_eq!(confirmed.status(), &TransitionStatus::Confirmed);
    assert_eq!(confirmed.status().label(), "CONFIRMED");
    assert_eq!(confirmed.confidence().as_str(), "HIGH");
    assert_eq!(confirmed.supporting()[0].id().as_str(), "support-1");
    assert_eq!(
        confirmed.counter()[0].description().as_str(),
        "legacy path remains"
    );
    assert_eq!(
        confirmed.missing()[0].requirement().as_str(),
        "multiple quarters"
    );

    let candidate = StageTransitionOutput::new(
        event_id("event-candidate"),
        company("acme"),
        stage("PRODUCTION_SYSTEM"),
        stage("PRODUCTIVITY_BREAKOUT"),
        date("2026-08-18"),
        TransitionStatus::Candidate,
        confidence("MEDIUM"),
    );

    assert_eq!(candidate.status(), &TransitionStatus::Candidate);
    assert_eq!(candidate.status().label(), "CANDIDATE");
    assert_ne!(candidate.status(), &TransitionStatus::Confirmed);
}

#[test]
fn collections_preserve_order_and_reject_duplicate_or_overlapping_evidence() {
    let mut output = StageTransitionOutput::new(
        event_id("event-order"),
        company("acme"),
        stage("WORKFLOW"),
        stage("PRODUCTION_SYSTEM"),
        date("2026-08-17"),
        TransitionStatus::Candidate,
        confidence("MEDIUM"),
    );

    output
        .add_supporting(supporting("support-1", "first"))
        .unwrap();
    output
        .add_supporting(supporting("support-2", "second"))
        .unwrap();
    assert_eq!(
        output
            .supporting()
            .iter()
            .map(|item| item.id().as_str())
            .collect::<Vec<_>>(),
        vec!["support-1", "support-2"]
    );

    assert!(output
        .add_counter(supporting("support-1", "same identity"))
        .is_err());
    assert!(output
        .add_supporting(supporting("support-2", "same collection"))
        .is_err());
    output
        .add_counter(supporting("counter-1", "counter"))
        .unwrap();
    assert!(output.add_missing(missing("counter-1", "overlap")).is_err());
    output
        .add_missing(missing("missing-1", "first missing"))
        .unwrap();
    assert!(output
        .add_missing(missing("missing-1", "same missing identity"))
        .is_err());
}

#[test]
fn priority_marks_only_productivity_breakout_and_accepts_correction_facts() {
    let productivity_breakout = StageTransitionOutput::new(
        event_id("event-breakout"),
        company("acme"),
        stage("PRODUCTION_SYSTEM"),
        stage("PRODUCTIVITY_BREAKOUT"),
        date("2026-08-17"),
        TransitionStatus::Candidate,
        confidence("LOW"),
    );
    assert_eq!(
        productivity_breakout.priority(),
        TransitionPriority::ProductivityBreakoutHigh
    );
    assert_eq!(
        productivity_breakout.priority().label(),
        "PRODUCTIVITY_BREAKOUT_HIGH"
    );

    let normal = StageTransitionOutput::new(
        event_id("event-normal"),
        company("acme"),
        stage("WORKFLOW"),
        stage("PRODUCTION_SYSTEM"),
        date("2026-08-17"),
        TransitionStatus::Confirmed,
        confidence("HIGH"),
    );
    assert_eq!(normal.priority(), TransitionPriority::Normal);

    let correction = StageTransitionOutput::new(
        event_id("event-correction"),
        company("acme"),
        stage("PRODUCTION_SYSTEM"),
        stage("WORKFLOW"),
        date("2026-08-18"),
        TransitionStatus::Confirmed,
        confidence("MEDIUM"),
    );
    assert_eq!(correction.prior_stage().as_str(), "PRODUCTION_SYSTEM");
    assert_eq!(correction.to_stage().as_str(), "WORKFLOW");
    assert_eq!(correction.priority(), TransitionPriority::Normal);

    let same_stage_fact = StageTransitionOutput::new(
        event_id("event-same-stage"),
        company("acme"),
        stage("WORKFLOW"),
        stage("WORKFLOW"),
        date("2026-08-19"),
        TransitionStatus::Candidate,
        confidence("UNKNOWN"),
    );
    assert_eq!(same_stage_fact.prior_stage(), same_stage_fact.to_stage());
}

#[test]
fn blank_boundary_values_are_rejected_and_source_has_no_cross_feature_inference() {
    assert!(TransitionEventId::new(" ").is_err());
    assert!(CompanyReference::new("").is_err());
    assert!(StageLabel::new("\t").is_err());
    assert!(TransitionDate::new(" ").is_err());
    assert!(Confidence::new("").is_err());
    assert!(EvidenceReference::new("evidence-1", "").is_err());
    assert!(MissingEvidence::new("", "requirement").is_err());

    let source = include_str!("../src/features/weekly_radar/domain/stage_transition_output.rs");
    assert!(!source.contains("features::transformation"));
    assert!(!source.contains("WeeklyRadarSnapshot"));
    assert!(!source.contains("telegram"));
}
