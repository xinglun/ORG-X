use org_x::features::transformation::domain::{
    CompanyReference, MissingProof, MissingProofId, PersistenceFact, ProofId, ProofReference,
    Stage, StageTransition, StageTransitionId, TransformationDomainError, TransformationProofSet,
};

#[test]
fn public_transformation_api_retains_explicit_proof_categories() {
    let mut proofs = TransformationProofSet::new();
    let supporting = ProofReference::new(
        ProofId::new("support").unwrap(),
        "Production-system redesign evidence",
    )
    .unwrap();
    let counter = ProofReference::new(
        ProofId::new("counter").unwrap(),
        "Old workflow remains in one region",
    )
    .unwrap();
    let missing = MissingProof::new(
        MissingProofId::new("missing").unwrap(),
        "Independent peer evidence",
    )
    .unwrap();

    proofs.add_supporting(supporting).unwrap();
    proofs.add_counter(counter).unwrap();
    proofs.add_missing(missing).unwrap();

    assert_eq!(proofs.supporting().len(), 1);
    assert_eq!(proofs.counter().len(), 1);
    assert_eq!(proofs.missing().len(), 1);
}

#[test]
fn public_transformation_api_supports_correction_transitions_without_recommendation() {
    let transition = StageTransition::new(
        StageTransitionId::new("downgrade").unwrap(),
        Stage::ProductivityBreakout,
        Stage::ProductionSystem,
        "2026-08-17",
    )
    .unwrap();
    let persistence = PersistenceFact::new("2026-Q1..Q2", "2").unwrap();

    assert_eq!(transition.from(), &Stage::ProductivityBreakout);
    assert_eq!(transition.to(), &Stage::ProductionSystem);
    assert_eq!(persistence.observation_count().as_str(), "2");
    assert!(matches!(
        StageTransition::new(
            StageTransitionId::new("same").unwrap(),
            Stage::Tool,
            Stage::Tool,
            "2026-08-17"
        ),
        Err(TransformationDomainError::SameStageTransition { .. })
    ));
    assert_eq!(
        CompanyReference::new("company").unwrap().as_str(),
        "company"
    );
}
