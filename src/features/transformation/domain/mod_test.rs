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

fn reference_evidence(
    id: &str,
    family: ReferenceModelEvidenceFamily,
    uri: &str,
    period: Option<&str>,
    peer: Option<&str>,
) -> ReferenceModelEvidence {
    ReferenceModelEvidence::new(
        id,
        family,
        format!("{id} claim"),
        uri,
        period.map(str::to_owned),
        peer.map(str::to_owned),
        true,
    )
    .unwrap()
}

fn complete_reference_bundle() -> ReferenceModelEvidenceBundle {
    let mut bundle = ReferenceModelEvidenceBundle::new();
    bundle
        .add_supporting(reference_evidence(
            "org",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            "https://example.test/org",
            Some("2025-01-13"),
            None,
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "production",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            "https://example.test/production",
            Some("2025-02-18"),
            None,
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "outcome-1",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            "https://example.test/10q-1",
            Some("2025-06-30"),
            None,
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "outcome-2",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            "https://example.test/10q-2",
            Some("2025-12-31"),
            None,
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "diffusion-1",
            ReferenceModelEvidenceFamily::IndustryDiffusion,
            "https://peer-a.test/adoption",
            Some("2026-01-10"),
            Some("Peer A"),
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "diffusion-2",
            ReferenceModelEvidenceFamily::IndustryDiffusion,
            "https://peer-b.test/adoption",
            Some("2026-02-10"),
            Some("Peer B"),
        ))
        .unwrap();
    bundle.set_counter_reviewed(true);
    bundle
}

#[test]
fn complete_reference_model_bundle_is_confirmed() {
    let assessment = complete_reference_bundle().assess();

    assert_eq!(
        assessment.eligibility(),
        ReferenceModelEligibility::Confirmed
    );
    assert_eq!(assessment.distinct_outcome_periods(), 2);
    assert_eq!(assessment.independent_diffusion_sources(), 2);
    assert!(assessment.missing().is_empty());
}

#[test]
fn organization_and_production_without_outcome_or_diffusion_is_only_a_candidate() {
    let mut bundle = ReferenceModelEvidenceBundle::new();
    bundle
        .add_supporting(reference_evidence(
            "org",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            "https://example.test/org",
            Some("2025-01-13"),
            None,
        ))
        .unwrap();
    bundle
        .add_supporting(reference_evidence(
            "production",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            "https://example.test/production",
            Some("2025-02-18"),
            None,
        ))
        .unwrap();

    let assessment = bundle.assess();

    assert_eq!(
        assessment.eligibility(),
        ReferenceModelEligibility::Candidate
    );
    assert!(assessment
        .missing()
        .iter()
        .any(|item| item == "sustained_outcome"));
    assert!(assessment
        .missing()
        .iter()
        .any(|item| item == "industry_diffusion"));
}

#[test]
fn missing_core_rewrite_is_not_a_reference_model_candidate() {
    let mut bundle = ReferenceModelEvidenceBundle::new();
    bundle
        .add_supporting(reference_evidence(
            "org",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            "https://example.test/org",
            Some("2025-01-13"),
            None,
        ))
        .unwrap();

    assert_eq!(
        bundle.assess().eligibility(),
        ReferenceModelEligibility::NotEligible
    );
}

#[test]
fn one_outcome_period_or_one_diffusion_source_cannot_confirm() {
    let mut bundle = complete_reference_bundle();
    bundle
        .add_supporting(reference_evidence(
            "duplicate-period",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            "https://example.test/10q-3",
            Some("2025-12-31"),
            None,
        ))
        .unwrap();
    let assessment = bundle.assess();
    assert_eq!(
        assessment.eligibility(),
        ReferenceModelEligibility::Confirmed
    );

    let mut incomplete = ReferenceModelEvidenceBundle::new();
    for evidence in [
        reference_evidence(
            "org",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            "https://example.test/org",
            Some("2025-01-13"),
            None,
        ),
        reference_evidence(
            "production",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            "https://example.test/production",
            Some("2025-02-18"),
            None,
        ),
        reference_evidence(
            "outcome-1",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            "https://example.test/10q-1",
            Some("2025-06-30"),
            None,
        ),
        reference_evidence(
            "outcome-2",
            ReferenceModelEvidenceFamily::SustainedOutcome,
            "https://example.test/10q-2",
            Some("2025-12-31"),
            None,
        ),
        reference_evidence(
            "diffusion-1",
            ReferenceModelEvidenceFamily::IndustryDiffusion,
            "https://peer-a.test/adoption",
            Some("2026-01-10"),
            Some("Peer A"),
        ),
    ] {
        incomplete.add_supporting(evidence).unwrap();
    }
    incomplete.set_counter_reviewed(true);

    let assessment = incomplete.assess();
    assert_eq!(
        assessment.eligibility(),
        ReferenceModelEligibility::Candidate
    );
    assert!(assessment
        .missing()
        .iter()
        .any(|item| item == "independent_diffusion_sources"));
}

#[test]
fn non_authoritative_evidence_never_satisfies_the_gate() {
    let mut bundle = complete_reference_bundle();
    let self_description = ReferenceModelEvidence::new(
        "self-description",
        ReferenceModelEvidenceFamily::IndustryDiffusion,
        "company says competitors should follow it",
        "https://candidate.test/strategy",
        Some("2026-03-01".to_owned()),
        Some("Unnamed industry".to_owned()),
        false,
    )
    .unwrap();
    bundle.add_supporting(self_description.clone()).unwrap();
    assert_eq!(
        bundle.assess().eligibility(),
        ReferenceModelEligibility::Confirmed
    );

    let mut only_self_description = ReferenceModelEvidenceBundle::new();
    only_self_description
        .add_supporting(reference_evidence(
            "org",
            ReferenceModelEvidenceFamily::OrganizationRewrite,
            "https://example.test/org",
            Some("2025-01-13"),
            None,
        ))
        .unwrap();
    only_self_description
        .add_supporting(reference_evidence(
            "production",
            ReferenceModelEvidenceFamily::ProductionSystemRewrite,
            "https://example.test/production",
            Some("2025-02-18"),
            None,
        ))
        .unwrap();
    only_self_description
        .add_supporting(self_description)
        .unwrap();
    only_self_description.set_counter_reviewed(true);
    assert_eq!(
        only_self_description.assess().eligibility(),
        ReferenceModelEligibility::Candidate
    );
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
