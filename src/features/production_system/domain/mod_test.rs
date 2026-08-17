use super::*;

fn human_role() -> HumanRole {
    HumanRole::new(
        HumanRoleId::new("operator").unwrap(),
        "Operator",
        "Owns customer exception decisions",
    )
    .unwrap()
}

fn agent_role() -> AgentRole {
    AgentRole::new(
        AgentRoleId::new("review-agent").unwrap(),
        "Review Agent",
        "Produces a first-pass review",
        SupervisionMode::HumanReviewed,
    )
    .unwrap()
}

#[test]
fn production_system_rejects_blank_values_and_duplicate_roots() {
    assert!(matches!(
        ProductionSystemId::new(" "),
        Err(ProductionDomainError::EmptyValue {
            field: "production system id"
        })
    ));
    assert!(matches!(
        ProductionSystem::new(ProductionSystemId::new("system").unwrap(), "", "purpose"),
        Err(ProductionDomainError::EmptyValue { field: "name" })
    ));

    let mut system = ProductionSystem::new(
        ProductionSystemId::new("system").unwrap(),
        "Core Production",
        "Creates verified customer outcomes",
    )
    .unwrap();
    let unit = ProductionUnit::new(
        ProductionUnitId::new("unit").unwrap(),
        "Verified Output",
        "A customer-ready result",
    )
    .unwrap();

    system.add_unit(unit.clone()).unwrap();
    assert!(matches!(
        system.add_unit(unit),
        Err(ProductionDomainError::DuplicateIdentity {
            entity: "production unit",
            ..
        })
    ));
}

#[test]
fn workflow_preserves_ordered_steps_and_explicit_control_structures() {
    let human = human_role();
    let agent = agent_role();
    let human_ref = RoleReference::Human(human.id().clone());
    let agent_ref = RoleReference::Agent(agent.id().clone());
    let mut workflow = Workflow::new(
        WorkflowId::new("review").unwrap(),
        "Review Workflow",
        "Turns an input into a verified result",
    )
    .unwrap();

    workflow
        .add_step(
            WorkflowStep::new(
                StepId::new("one").unwrap(),
                "Prepare the input",
                human_ref.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    workflow
        .add_step(
            WorkflowStep::new(
                StepId::new("two").unwrap(),
                "Draft the review",
                agent_ref.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    workflow
        .add_control_point(
            ControlPoint::new(
                ControlPointId::new("control").unwrap(),
                "Confirm the input is complete",
                human_ref.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    workflow
        .add_verification_point(
            VerificationPoint::new(
                VerificationPointId::new("verify").unwrap(),
                "Check the draft against the input",
                human_ref.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    workflow
        .add_decision_point(
            DecisionPoint::new(
                DecisionPointId::new("decide").unwrap(),
                "Accept or return the draft",
                human_ref.clone(),
            )
            .unwrap(),
        )
        .unwrap();
    workflow
        .add_exception_path(
            ExceptionPath::new(
                ExceptionPathId::new("exception").unwrap(),
                "Escalate an unverifiable draft",
                human.id().clone(),
            )
            .unwrap(),
        )
        .unwrap();

    assert_eq!(workflow.steps()[0].id().as_str(), "one");
    assert_eq!(workflow.steps()[1].role(), &agent_ref);
    assert_eq!(workflow.control_points()[0].owner(), &human_ref);
    assert_eq!(workflow.verification_points()[0].verifier(), &human_ref);
    assert_eq!(workflow.decision_points()[0].owner(), &human_ref);
    assert_eq!(workflow.exception_paths()[0].escalates_to(), human.id());
    assert_eq!(agent.supervision(), &SupervisionMode::HumanReviewed);
}

#[test]
fn workflow_rejects_duplicate_step_identity() {
    let role = RoleReference::Human(human_role().id().clone());
    let mut workflow = Workflow::new(
        WorkflowId::new("review").unwrap(),
        "Review Workflow",
        "Purpose",
    )
    .unwrap();
    let step = WorkflowStep::new(StepId::new("step").unwrap(), "Do work", role).unwrap();

    workflow.add_step(step.clone()).unwrap();
    assert!(matches!(
        workflow.add_step(step),
        Err(ProductionDomainError::DuplicateIdentity {
            entity: "workflow step",
            ..
        })
    ));
}
