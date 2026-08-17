use org_x::features::organization::domain::{
    Budget, BudgetId, DecisionRight, DecisionRightId, ManagementCommitment, ManagementCommitmentId,
    OrganizationDomainError, OrganizationEvidence, OrganizationId, Responsibility,
    ResponsibilityId,
};

#[test]
fn public_organization_api_retains_responsibility_budget_and_decision_scope() {
    let mut evidence = OrganizationEvidence::new(
        OrganizationId::new("org").unwrap(),
        "Organization Evidence",
        "Facts about the operating model",
    )
    .unwrap();
    let responsibility = Responsibility::new(
        ResponsibilityId::new("responsibility").unwrap(),
        "Owns the outcome",
        "Operations lead",
    )
    .unwrap();
    let budget = Budget::new(
        BudgetId::new("budget").unwrap(),
        "Funds redesign",
        "250000",
        "USD",
    )
    .unwrap();
    let decision = DecisionRight::new(
        DecisionRightId::new("decision").unwrap(),
        "Approve workflow exceptions",
        "Operations lead",
        "Core workflow",
    )
    .unwrap();

    evidence.add_responsibility(responsibility).unwrap();
    evidence.add_budget(budget).unwrap();
    evidence.add_decision_right(decision).unwrap();

    assert_eq!(
        evidence.responsibilities()[0].owner().as_str(),
        "Operations lead"
    );
    assert_eq!(evidence.budgets()[0].unit().as_str(), "USD");
    assert_eq!(
        evidence.decision_rights()[0].scope().as_str(),
        "Core workflow"
    );
}

#[test]
fn public_organization_api_rejects_duplicate_decision_rights() {
    let mut evidence = OrganizationEvidence::new(
        OrganizationId::new("org").unwrap(),
        "Organization Evidence",
        "Facts",
    )
    .unwrap();
    let decision = DecisionRight::new(
        DecisionRightId::new("decision").unwrap(),
        "Approve",
        "Operations lead",
        "Workflow",
    )
    .unwrap();

    evidence.add_decision_right(decision.clone()).unwrap();
    assert!(matches!(
        evidence.add_decision_right(decision),
        Err(OrganizationDomainError::DuplicateIdentity {
            entity: "decision right",
            ..
        })
    ));
}

#[test]
fn public_organization_api_does_not_require_stage_or_score_inputs() {
    let commitment = ManagementCommitment::new(
        ManagementCommitmentId::new("commitment").unwrap(),
        "Leadership commits",
        "Executive team",
    )
    .unwrap();

    assert_eq!(commitment.statement().as_str(), "Leadership commits");
    assert_eq!(commitment.committed_by().as_str(), "Executive team");
}
