use super::*;

fn transition(id: &str, from: Stage, to: Stage) -> StageTransition {
    StageTransition::new(StageTransitionId::new(id).unwrap(), from, to, "2026-08-17").unwrap()
}

#[test]
fn stage_catalog_has_six_ordered_variants() {
    assert_eq!(Stage::Tool.rank(), 0);
    assert_eq!(Stage::Substitution.rank(), 1);
    assert_eq!(Stage::Workflow.rank(), 2);
    assert_eq!(Stage::ProductionSystem.rank(), 3);
    assert_eq!(Stage::ProductivityBreakout.rank(), 4);
    assert_eq!(Stage::ReferenceModel.rank(), 5);
    assert_eq!(Stage::ReferenceModel.label(), "REFERENCE_MODEL");
}

#[test]
fn transitions_preserve_upgrade_and_downgrade_direction() {
    let upgrade = transition("upgrade", Stage::Workflow, Stage::ProductionSystem);
    let correction = transition("correction", Stage::ProductionSystem, Stage::Workflow);

    assert_eq!(upgrade.from(), &Stage::Workflow);
    assert_eq!(upgrade.to(), &Stage::ProductionSystem);
    assert_eq!(correction.from(), &Stage::ProductionSystem);
    assert_eq!(correction.to(), &Stage::Workflow);
    assert_eq!(upgrade.transition_date().as_str(), "2026-08-17");
}

#[test]
fn same_stage_transition_is_rejected() {
    assert!(matches!(
        StageTransition::new(
            StageTransitionId::new("same").unwrap(),
            Stage::Workflow,
            Stage::Workflow,
            "2026-08-17"
        ),
        Err(TransformationDomainError::SameStageTransition {
            stage: Stage::Workflow
        })
    ));
}

#[test]
fn proof_set_preserves_polarity_missing_requirements_and_order() {
    let supporting =
        ProofReference::new(ProofId::new("supporting").unwrap(), "Workflow evidence").unwrap();
    let counter =
        ProofReference::new(ProofId::new("counter").unwrap(), "Counter evidence").unwrap();
    let missing = MissingProof::new(
        MissingProofId::new("missing").unwrap(),
        "Two quarters of persistence",
    )
    .unwrap();
    let mut proofs = TransformationProofSet::new();

    proofs.add_supporting(supporting.clone()).unwrap();
    proofs.add_counter(counter.clone()).unwrap();
    proofs.add_missing(missing.clone()).unwrap();

    assert_eq!(proofs.supporting(), std::slice::from_ref(&supporting));
    assert_eq!(proofs.counter(), std::slice::from_ref(&counter));
    assert_eq!(proofs.missing(), std::slice::from_ref(&missing));
    assert!(matches!(
        proofs.add_counter(supporting),
        Err(TransformationDomainError::DuplicateIdentity {
            entity: "transformation proof",
            ..
        })
    ));
}

#[test]
fn persistence_and_assessment_preserve_supplied_facts() {
    let persistence = PersistenceFact::new("2025-Q1..2026-Q2", "6").unwrap();
    let mut assessment = TransformationAssessment::new(
        CompanyReference::new("company-1").unwrap(),
        Stage::ProductionSystem,
    );
    assessment.set_persistence(persistence.clone());
    assessment
        .add_transition(transition(
            "transition",
            Stage::Workflow,
            Stage::ProductionSystem,
        ))
        .unwrap();

    assert_eq!(assessment.current_stage(), &Stage::ProductionSystem);
    assert_eq!(assessment.persistence(), Some(&persistence));
    assert_eq!(assessment.transitions().len(), 1);
}
