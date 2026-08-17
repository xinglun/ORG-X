use super::*;

fn commitment() -> ManagementCommitment {
    ManagementCommitment::new(
        ManagementCommitmentId::new("commitment").unwrap(),
        "Leadership commits to the operating-model change",
        "Executive team",
    )
    .unwrap()
}

fn responsibility() -> Responsibility {
    Responsibility::new(
        ResponsibilityId::new("responsibility").unwrap(),
        "Owns the workflow outcome",
        "Operations lead",
    )
    .unwrap()
}

fn budget() -> Budget {
    Budget::new(
        BudgetId::new("budget").unwrap(),
        "Funds workflow redesign",
        "100000",
        "USD",
    )
    .unwrap()
}

fn decision_right() -> DecisionRight {
    DecisionRight::new(
        DecisionRightId::new("decision").unwrap(),
        "Approve the production workflow",
        "Operations lead",
        "Workflow design and exception handling",
    )
    .unwrap()
}

fn adaptation() -> OrganizationAdaptation {
    OrganizationAdaptation::new(
        AdaptationId::new("adaptation").unwrap(),
        "Move responsibility from handoffs to supervised workflow ownership",
        "production-system:core",
    )
    .unwrap()
}

#[test]
fn organization_evidence_preserves_facts_and_collection_order() {
    let mut evidence = OrganizationEvidence::new(
        OrganizationId::new("org").unwrap(),
        "Operating Model Evidence",
        "Organization facts related to the core production system",
    )
    .unwrap();
    let commitment = commitment();
    let responsibility = responsibility();
    let budget = budget();
    let decision_right = decision_right();
    let adaptation = adaptation();

    evidence.add_commitment(commitment.clone()).unwrap();
    evidence.add_responsibility(responsibility.clone()).unwrap();
    evidence.add_budget(budget.clone()).unwrap();
    evidence.add_decision_right(decision_right.clone()).unwrap();
    evidence.add_adaptation(adaptation.clone()).unwrap();

    assert_eq!(evidence.commitments(), std::slice::from_ref(&commitment));
    assert_eq!(
        evidence.responsibilities(),
        std::slice::from_ref(&responsibility)
    );
    assert_eq!(evidence.budgets(), std::slice::from_ref(&budget));
    assert_eq!(
        evidence.decision_rights(),
        std::slice::from_ref(&decision_right)
    );
    assert_eq!(evidence.adaptations(), std::slice::from_ref(&adaptation));
    assert_eq!(evidence.budgets()[0].amount().as_str(), "100000");
    assert_eq!(
        evidence.adaptations()[0]
            .production_system_target()
            .as_str(),
        "production-system:core"
    );
}

#[test]
fn organization_evidence_rejects_blank_fields_and_duplicate_identities() {
    assert!(matches!(
        OrganizationId::new(" "),
        Err(OrganizationDomainError::EmptyValue {
            field: "organization id"
        })
    ));
    assert!(matches!(
        ManagementCommitment::new(
            ManagementCommitmentId::new("commitment").unwrap(),
            "",
            "Executive team"
        ),
        Err(OrganizationDomainError::EmptyValue { field: "statement" })
    ));

    let mut evidence = OrganizationEvidence::new(
        OrganizationId::new("org").unwrap(),
        "Evidence",
        "Description",
    )
    .unwrap();
    let commitment = commitment();
    evidence.add_commitment(commitment.clone()).unwrap();

    assert!(matches!(
        evidence.add_commitment(commitment),
        Err(OrganizationDomainError::DuplicateIdentity {
            entity: "management commitment",
            ..
        })
    ));
}
