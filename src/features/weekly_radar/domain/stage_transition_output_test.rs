use super::*;

fn output(from_stage: &str, to_stage: &str) -> StageTransitionOutput {
    StageTransitionOutput::new(
        TransitionEventId::new("module-event").expect("event id should be valid"),
        CompanyReference::new("acme").expect("company should be valid"),
        StageLabel::new(from_stage).expect("from stage should be valid"),
        StageLabel::new(to_stage).expect("to stage should be valid"),
        TransitionDate::new("2026-08-20").expect("date should be valid"),
        TransitionStatus::Candidate,
        Confidence::new("MEDIUM").expect("confidence should be valid"),
    )
}

#[test]
fn module_local_test_preserves_candidate_and_explicit_productivity_breakout_priority() {
    let output = output("PRODUCTION_SYSTEM", "PRODUCTIVITY_BREAKOUT");

    assert_eq!(output.status(), &TransitionStatus::Candidate);
    assert_eq!(
        output.priority(),
        TransitionPriority::ProductivityBreakoutHigh
    );
}

#[test]
fn module_local_test_rejects_evidence_identity_overlap() {
    let mut output = output("WORKFLOW", "PRODUCTION_SYSTEM");
    output
        .add_supporting(EvidenceReference::new("e-1", "support").unwrap())
        .unwrap();

    let result = output.add_missing(MissingEvidence::new("e-1", "missing").unwrap());

    assert!(matches!(
        result,
        Err(StageTransitionOutputError::DuplicateIdentity {
            entity: "transition evidence",
            id
        }) if id == "e-1"
    ));
}
