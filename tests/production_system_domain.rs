use org_x::features::production_system::domain::{
    AgentRole, AgentRoleId, HumanRole, HumanRoleId, ProductionDomainError, ProductionSystem,
    ProductionSystemId, SupervisionMode,
};

#[test]
fn public_domain_api_preserves_roles_and_supervision() {
    let human = HumanRole::new(
        HumanRoleId::new("owner").unwrap(),
        "Owner",
        "Owns the final responsibility",
    )
    .unwrap();
    let agent = AgentRole::new(
        AgentRoleId::new("assistant").unwrap(),
        "Assistant",
        "Prepares a bounded draft",
        SupervisionMode::HumanEscalated,
    )
    .unwrap();
    let mut system = ProductionSystem::new(
        ProductionSystemId::new("system").unwrap(),
        "Production System",
        "Creates a verified output",
    )
    .unwrap();

    system.add_human_role(human.clone()).unwrap();
    system.add_agent_role(agent.clone()).unwrap();

    assert_eq!(system.human_roles(), &[human]);
    assert_eq!(system.agent_roles(), &[agent]);
}

#[test]
fn public_domain_api_rejects_duplicate_role_identity() {
    let mut system = ProductionSystem::new(
        ProductionSystemId::new("system").unwrap(),
        "Production System",
        "Creates a verified output",
    )
    .unwrap();
    let role = HumanRole::new(
        HumanRoleId::new("owner").unwrap(),
        "Owner",
        "Owns the final responsibility",
    )
    .unwrap();

    system.add_human_role(role.clone()).unwrap();
    assert!(matches!(
        system.add_human_role(role),
        Err(ProductionDomainError::DuplicateIdentity {
            entity: "human role",
            ..
        })
    ));
}
