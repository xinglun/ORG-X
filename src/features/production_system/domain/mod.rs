//! Pure facts for how a production system creates and verifies value.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn duplicate_identity(entity: &'static str, id: impl Into<String>) -> ProductionDomainError {
    ProductionDomainError::DuplicateIdentity {
        entity,
        id: id.into(),
    }
}

use crate::shared::domain::text_value as shared_text_value;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        shared_text_value!($name, $field, $description, ProductionDomainError);
    };
}

text_value!(
    ProductionSystemId,
    "production system id",
    "Stable identity for a production system."
);
text_value!(
    ProductionUnitId,
    "production unit id",
    "Stable identity for a production unit."
);
text_value!(WorkflowId, "workflow id", "Stable identity for a workflow.");
text_value!(
    HumanRoleId,
    "human role id",
    "Stable identity for a human role."
);
text_value!(
    AgentRoleId,
    "agent role id",
    "Stable identity for an agent role."
);
text_value!(
    StepId,
    "workflow step id",
    "Stable identity for a workflow step."
);
text_value!(
    ControlPointId,
    "control point id",
    "Stable identity for a control point."
);
text_value!(
    VerificationPointId,
    "verification point id",
    "Stable identity for a verification point."
);
text_value!(
    DecisionPointId,
    "decision point id",
    "Stable identity for a decision point."
);
text_value!(
    ExceptionPathId,
    "exception path id",
    "Stable identity for an exception path."
);
text_value!(Name, "name", "A validated display name.");
text_value!(Purpose, "purpose", "A validated purpose statement.");
text_value!(
    Description,
    "description",
    "A validated descriptive statement."
);
text_value!(
    Responsibility,
    "responsibility",
    "A validated responsibility statement."
);
text_value!(
    Capability,
    "capability",
    "A validated capability statement."
);

/// Validation and collection failures for production-system facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductionDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// An entity identity already exists in its owning collection.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for ProductionDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for ProductionDomainError {}

/// Human responsibility level recorded for an agent-supported workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SupervisionMode {
    /// A human owns the outcome and the agent contributes bounded work.
    HumanOwned,
    /// A human reviews the agent result before it is accepted.
    HumanReviewed,
    /// The agent works until a defined exception requires human attention.
    HumanEscalated,
}

/// A role reference carried by a workflow element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RoleReference {
    /// Reference to a human role.
    Human(HumanRoleId),
    /// Reference to an agent role.
    Agent(AgentRoleId),
}

/// A human role with responsibility retained as a domain fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HumanRole {
    id: HumanRoleId,
    name: Name,
    responsibility: Responsibility,
}

impl HumanRole {
    /// Creates a human role with validated descriptive fields.
    pub fn new(
        id: HumanRoleId,
        name: impl Into<String>,
        responsibility: impl Into<String>,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            responsibility: Responsibility::new(responsibility)?,
        })
    }

    /// Returns the role identity.
    pub fn id(&self) -> &HumanRoleId {
        &self.id
    }

    /// Returns the role name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the responsibility statement.
    pub fn responsibility(&self) -> &Responsibility {
        &self.responsibility
    }
}

/// An agent role with capability and human-supervision facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRole {
    id: AgentRoleId,
    name: Name,
    capability: Capability,
    supervision: SupervisionMode,
}

impl AgentRole {
    /// Creates an agent role with an explicit supervision mode.
    pub fn new(
        id: AgentRoleId,
        name: impl Into<String>,
        capability: impl Into<String>,
        supervision: SupervisionMode,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            capability: Capability::new(capability)?,
            supervision,
        })
    }

    /// Returns the role identity.
    pub fn id(&self) -> &AgentRoleId {
        &self.id
    }

    /// Returns the role name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the capability statement.
    pub fn capability(&self) -> &Capability {
        &self.capability
    }

    /// Returns the human-supervision mode.
    pub fn supervision(&self) -> &SupervisionMode {
        &self.supervision
    }
}

/// A bounded unit of output created by a production system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionUnit {
    id: ProductionUnitId,
    name: Name,
    output: Description,
}

impl ProductionUnit {
    /// Creates a production unit with a validated output description.
    pub fn new(
        id: ProductionUnitId,
        name: impl Into<String>,
        output: impl Into<String>,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            output: Description::new(output)?,
        })
    }

    /// Returns the unit identity.
    pub fn id(&self) -> &ProductionUnitId {
        &self.id
    }

    /// Returns the unit name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the output description.
    pub fn output(&self) -> &Description {
        &self.output
    }
}

/// An ordered step in a workflow.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowStep {
    id: StepId,
    description: Description,
    role: RoleReference,
}

impl WorkflowStep {
    /// Creates a workflow step assigned to a role reference.
    pub fn new(
        id: StepId,
        description: impl Into<String>,
        role: RoleReference,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            role,
        })
    }

    /// Returns the step identity.
    pub fn id(&self) -> &StepId {
        &self.id
    }

    /// Returns the step description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the role responsible for this step.
    pub fn role(&self) -> &RoleReference {
        &self.role
    }
}

/// A workflow control point and its owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPoint {
    id: ControlPointId,
    description: Description,
    owner: RoleReference,
}

impl ControlPoint {
    /// Creates a control point with an explicit owner.
    pub fn new(
        id: ControlPointId,
        description: impl Into<String>,
        owner: RoleReference,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            owner,
        })
    }

    /// Returns the control-point identity.
    pub fn id(&self) -> &ControlPointId {
        &self.id
    }

    /// Returns the control-point description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the role that owns this point.
    pub fn owner(&self) -> &RoleReference {
        &self.owner
    }
}

/// A workflow verification point and its verifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationPoint {
    id: VerificationPointId,
    description: Description,
    verifier: RoleReference,
}

impl VerificationPoint {
    /// Creates a verification point with an explicit verifier.
    pub fn new(
        id: VerificationPointId,
        description: impl Into<String>,
        verifier: RoleReference,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            verifier,
        })
    }

    /// Returns the verification-point identity.
    pub fn id(&self) -> &VerificationPointId {
        &self.id
    }

    /// Returns the verification-point description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the role that verifies this point.
    pub fn verifier(&self) -> &RoleReference {
        &self.verifier
    }
}

/// A workflow decision point and its owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionPoint {
    id: DecisionPointId,
    description: Description,
    owner: RoleReference,
}

impl DecisionPoint {
    /// Creates a decision point with an explicit owner.
    pub fn new(
        id: DecisionPointId,
        description: impl Into<String>,
        owner: RoleReference,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            owner,
        })
    }

    /// Returns the decision-point identity.
    pub fn id(&self) -> &DecisionPointId {
        &self.id
    }

    /// Returns the decision-point description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the role that owns this point.
    pub fn owner(&self) -> &RoleReference {
        &self.owner
    }
}

/// A workflow exception path and its human escalation target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExceptionPath {
    id: ExceptionPathId,
    description: Description,
    escalates_to: HumanRoleId,
}

impl ExceptionPath {
    /// Creates an exception path with an explicit human escalation target.
    pub fn new(
        id: ExceptionPathId,
        description: impl Into<String>,
        escalates_to: HumanRoleId,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            escalates_to,
        })
    }

    /// Returns the exception-path identity.
    pub fn id(&self) -> &ExceptionPathId {
        &self.id
    }

    /// Returns the exception-path description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the human role that receives the escalation.
    pub fn escalates_to(&self) -> &HumanRoleId {
        &self.escalates_to
    }
}

/// An ordered production workflow with explicit control structures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workflow {
    id: WorkflowId,
    name: Name,
    purpose: Purpose,
    steps: Vec<WorkflowStep>,
    control_points: Vec<ControlPoint>,
    verification_points: Vec<VerificationPoint>,
    decision_points: Vec<DecisionPoint>,
    exception_paths: Vec<ExceptionPath>,
}

impl Workflow {
    /// Creates an empty workflow with validated identity and descriptions.
    pub fn new(
        id: WorkflowId,
        name: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            purpose: Purpose::new(purpose)?,
            steps: Vec::new(),
            control_points: Vec::new(),
            verification_points: Vec::new(),
            decision_points: Vec::new(),
            exception_paths: Vec::new(),
        })
    }

    /// Returns the workflow identity.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    /// Returns the workflow name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the workflow purpose.
    pub fn purpose(&self) -> &Purpose {
        &self.purpose
    }

    /// Returns steps in insertion order.
    pub fn steps(&self) -> &[WorkflowStep] {
        &self.steps
    }

    /// Returns control points in insertion order.
    pub fn control_points(&self) -> &[ControlPoint] {
        &self.control_points
    }

    /// Returns verification points in insertion order.
    pub fn verification_points(&self) -> &[VerificationPoint] {
        &self.verification_points
    }

    /// Returns decision points in insertion order.
    pub fn decision_points(&self) -> &[DecisionPoint] {
        &self.decision_points
    }

    /// Returns exception paths in insertion order.
    pub fn exception_paths(&self) -> &[ExceptionPath] {
        &self.exception_paths
    }

    /// Appends a step unless its identity is already present.
    pub fn add_step(&mut self, step: WorkflowStep) -> Result<(), ProductionDomainError> {
        if self.steps.iter().any(|existing| existing.id == step.id) {
            return Err(duplicate_identity("workflow step", step.id.as_str()));
        }
        self.steps.push(step);
        Ok(())
    }

    /// Appends a control point unless its identity is already present.
    pub fn add_control_point(&mut self, point: ControlPoint) -> Result<(), ProductionDomainError> {
        if self
            .control_points
            .iter()
            .any(|existing| existing.id == point.id)
        {
            return Err(duplicate_identity("control point", point.id.as_str()));
        }
        self.control_points.push(point);
        Ok(())
    }

    /// Appends a verification point unless its identity is already present.
    pub fn add_verification_point(
        &mut self,
        point: VerificationPoint,
    ) -> Result<(), ProductionDomainError> {
        if self
            .verification_points
            .iter()
            .any(|existing| existing.id == point.id)
        {
            return Err(duplicate_identity("verification point", point.id.as_str()));
        }
        self.verification_points.push(point);
        Ok(())
    }

    /// Appends a decision point unless its identity is already present.
    pub fn add_decision_point(
        &mut self,
        point: DecisionPoint,
    ) -> Result<(), ProductionDomainError> {
        if self
            .decision_points
            .iter()
            .any(|existing| existing.id == point.id)
        {
            return Err(duplicate_identity("decision point", point.id.as_str()));
        }
        self.decision_points.push(point);
        Ok(())
    }

    /// Appends an exception path unless its identity is already present.
    pub fn add_exception_path(&mut self, path: ExceptionPath) -> Result<(), ProductionDomainError> {
        if self
            .exception_paths
            .iter()
            .any(|existing| existing.id == path.id)
        {
            return Err(duplicate_identity("exception path", path.id.as_str()));
        }
        self.exception_paths.push(path);
        Ok(())
    }
}

/// The overall way a company creates verified core value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductionSystem {
    id: ProductionSystemId,
    name: Name,
    purpose: Purpose,
    units: Vec<ProductionUnit>,
    workflows: Vec<Workflow>,
    human_roles: Vec<HumanRole>,
    agent_roles: Vec<AgentRole>,
}

impl ProductionSystem {
    /// Creates an empty production system with validated identity and purpose.
    pub fn new(
        id: ProductionSystemId,
        name: impl Into<String>,
        purpose: impl Into<String>,
    ) -> Result<Self, ProductionDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            purpose: Purpose::new(purpose)?,
            units: Vec::new(),
            workflows: Vec::new(),
            human_roles: Vec::new(),
            agent_roles: Vec::new(),
        })
    }

    /// Returns the production-system identity.
    pub fn id(&self) -> &ProductionSystemId {
        &self.id
    }

    /// Returns the production-system name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the production-system purpose.
    pub fn purpose(&self) -> &Purpose {
        &self.purpose
    }

    /// Returns production units in insertion order.
    pub fn units(&self) -> &[ProductionUnit] {
        &self.units
    }

    /// Returns workflows in insertion order.
    pub fn workflows(&self) -> &[Workflow] {
        &self.workflows
    }

    /// Returns human roles in insertion order.
    pub fn human_roles(&self) -> &[HumanRole] {
        &self.human_roles
    }

    /// Returns agent roles in insertion order.
    pub fn agent_roles(&self) -> &[AgentRole] {
        &self.agent_roles
    }

    /// Adds a production unit unless its identity is already present.
    pub fn add_unit(&mut self, unit: ProductionUnit) -> Result<(), ProductionDomainError> {
        if self.units.iter().any(|existing| existing.id == unit.id) {
            return Err(duplicate_identity("production unit", unit.id.as_str()));
        }
        self.units.push(unit);
        Ok(())
    }

    /// Adds a workflow unless its identity is already present.
    pub fn add_workflow(&mut self, workflow: Workflow) -> Result<(), ProductionDomainError> {
        if self
            .workflows
            .iter()
            .any(|existing| existing.id == workflow.id)
        {
            return Err(duplicate_identity("workflow", workflow.id.as_str()));
        }
        self.workflows.push(workflow);
        Ok(())
    }

    /// Adds a human role unless its identity is already present.
    pub fn add_human_role(&mut self, role: HumanRole) -> Result<(), ProductionDomainError> {
        if self
            .human_roles
            .iter()
            .any(|existing| existing.id == role.id)
        {
            return Err(duplicate_identity("human role", role.id.as_str()));
        }
        self.human_roles.push(role);
        Ok(())
    }

    /// Adds an agent role unless its identity is already present.
    pub fn add_agent_role(&mut self, role: AgentRole) -> Result<(), ProductionDomainError> {
        if self
            .agent_roles
            .iter()
            .any(|existing| existing.id == role.id)
        {
            return Err(duplicate_identity("agent role", role.id.as_str()));
        }
        self.agent_roles.push(role);
        Ok(())
    }
}
