//! Pure organization facts retained as evidence for production-system review.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn non_empty(
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, OrganizationDomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(OrganizationDomainError::EmptyValue { field });
    }
    Ok(value)
}

fn duplicate_identity(entity: &'static str, id: impl Into<String>) -> OrganizationDomainError {
    OrganizationDomainError::DuplicateIdentity {
        entity,
        id: id.into(),
    }
}

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Creates the value and rejects blank input.
            pub fn new(value: impl Into<String>) -> Result<Self, OrganizationDomainError> {
                Ok(Self(non_empty($field, value)?))
            }

            /// Returns the original value supplied at the boundary.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_value!(
    OrganizationId,
    "organization id",
    "Stable identity for an organization evidence aggregate."
);
text_value!(
    ManagementCommitmentId,
    "management commitment id",
    "Stable identity for a management commitment fact."
);
text_value!(
    ResponsibilityId,
    "responsibility id",
    "Stable identity for a responsibility fact."
);
text_value!(BudgetId, "budget id", "Stable identity for a budget fact.");
text_value!(
    DecisionRightId,
    "decision right id",
    "Stable identity for a decision-right fact."
);
text_value!(
    AdaptationId,
    "adaptation id",
    "Stable identity for an organization adaptation fact."
);
text_value!(Name, "name", "A validated organization evidence name.");
text_value!(
    Statement,
    "statement",
    "A validated organization statement."
);
text_value!(
    Description,
    "description",
    "A validated organization description."
);
text_value!(
    Amount,
    "amount",
    "An opaque amount retained with a budget fact."
);
text_value!(Unit, "unit", "The unit retained with a budget amount.");
text_value!(Scope, "scope", "A validated decision-right scope.");

/// Validation and collection failures for organization facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OrganizationDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// An entity identity already exists in its owning collection.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for OrganizationDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for OrganizationDomainError {}

/// A management commitment retained as an auditable statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagementCommitment {
    id: ManagementCommitmentId,
    statement: Statement,
    committed_by: Description,
}

impl ManagementCommitment {
    /// Creates a commitment statement with its supplied authoring group.
    pub fn new(
        id: ManagementCommitmentId,
        statement: impl Into<String>,
        committed_by: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            statement: Statement::new(statement)?,
            committed_by: Description::new(committed_by)?,
        })
    }

    /// Returns the commitment identity.
    pub fn id(&self) -> &ManagementCommitmentId {
        &self.id
    }

    /// Returns the commitment statement.
    pub fn statement(&self) -> &Statement {
        &self.statement
    }

    /// Returns the group or role recorded as the commitment source.
    pub fn committed_by(&self) -> &Description {
        &self.committed_by
    }
}

/// A responsibility assignment retained without enforcing it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Responsibility {
    id: ResponsibilityId,
    statement: Statement,
    owner: Description,
}

impl Responsibility {
    /// Creates a responsibility statement and its recorded owner.
    pub fn new(
        id: ResponsibilityId,
        statement: impl Into<String>,
        owner: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            statement: Statement::new(statement)?,
            owner: Description::new(owner)?,
        })
    }

    /// Returns the responsibility identity.
    pub fn id(&self) -> &ResponsibilityId {
        &self.id
    }

    /// Returns the responsibility statement.
    pub fn statement(&self) -> &Statement {
        &self.statement
    }

    /// Returns the recorded responsibility owner.
    pub fn owner(&self) -> &Description {
        &self.owner
    }
}

/// A budget fact with an opaque amount and unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Budget {
    id: BudgetId,
    purpose: Description,
    amount: Amount,
    unit: Unit,
}

impl Budget {
    /// Creates a budget fact without performing arithmetic or allocation.
    pub fn new(
        id: BudgetId,
        purpose: impl Into<String>,
        amount: impl Into<String>,
        unit: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            purpose: Description::new(purpose)?,
            amount: Amount::new(amount)?,
            unit: Unit::new(unit)?,
        })
    }

    /// Returns the budget identity.
    pub fn id(&self) -> &BudgetId {
        &self.id
    }

    /// Returns the budget purpose.
    pub fn purpose(&self) -> &Description {
        &self.purpose
    }

    /// Returns the opaque amount.
    pub fn amount(&self) -> &Amount {
        &self.amount
    }

    /// Returns the amount unit.
    pub fn unit(&self) -> &Unit {
        &self.unit
    }
}

/// A decision right fact with holder and scope preserved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionRight {
    id: DecisionRightId,
    decision: Description,
    holder: Description,
    scope: Scope,
}

impl DecisionRight {
    /// Creates a decision-right fact without enforcing the permission.
    pub fn new(
        id: DecisionRightId,
        decision: impl Into<String>,
        holder: impl Into<String>,
        scope: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            decision: Description::new(decision)?,
            holder: Description::new(holder)?,
            scope: Scope::new(scope)?,
        })
    }

    /// Returns the decision-right identity.
    pub fn id(&self) -> &DecisionRightId {
        &self.id
    }

    /// Returns the decision description.
    pub fn decision(&self) -> &Description {
        &self.decision
    }

    /// Returns the recorded right holder.
    pub fn holder(&self) -> &Description {
        &self.holder
    }

    /// Returns the scope of the right.
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
}

/// An organization adaptation fact related to an opaque production target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrganizationAdaptation {
    id: AdaptationId,
    description: Description,
    production_system_target: Description,
}

impl OrganizationAdaptation {
    /// Creates an adaptation fact without inferring a stage or score.
    pub fn new(
        id: AdaptationId,
        description: impl Into<String>,
        production_system_target: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            description: Description::new(description)?,
            production_system_target: Description::new(production_system_target)?,
        })
    }

    /// Returns the adaptation identity.
    pub fn id(&self) -> &AdaptationId {
        &self.id
    }

    /// Returns the adaptation description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns the opaque target reference.
    pub fn production_system_target(&self) -> &Description {
        &self.production_system_target
    }
}

/// An ordered collection of organization facts related to one organization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrganizationEvidence {
    id: OrganizationId,
    name: Name,
    description: Description,
    commitments: Vec<ManagementCommitment>,
    responsibilities: Vec<Responsibility>,
    budgets: Vec<Budget>,
    decision_rights: Vec<DecisionRight>,
    adaptations: Vec<OrganizationAdaptation>,
}

impl OrganizationEvidence {
    /// Creates an empty organization-evidence collection with validated facts.
    pub fn new(
        id: OrganizationId,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<Self, OrganizationDomainError> {
        Ok(Self {
            id,
            name: Name::new(name)?,
            description: Description::new(description)?,
            commitments: Vec::new(),
            responsibilities: Vec::new(),
            budgets: Vec::new(),
            decision_rights: Vec::new(),
            adaptations: Vec::new(),
        })
    }

    /// Returns the organization identity.
    pub fn id(&self) -> &OrganizationId {
        &self.id
    }

    /// Returns the evidence collection name.
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Returns the evidence collection description.
    pub fn description(&self) -> &Description {
        &self.description
    }

    /// Returns commitments in insertion order.
    pub fn commitments(&self) -> &[ManagementCommitment] {
        &self.commitments
    }

    /// Returns responsibilities in insertion order.
    pub fn responsibilities(&self) -> &[Responsibility] {
        &self.responsibilities
    }

    /// Returns budgets in insertion order.
    pub fn budgets(&self) -> &[Budget] {
        &self.budgets
    }

    /// Returns decision rights in insertion order.
    pub fn decision_rights(&self) -> &[DecisionRight] {
        &self.decision_rights
    }

    /// Returns adaptations in insertion order.
    pub fn adaptations(&self) -> &[OrganizationAdaptation] {
        &self.adaptations
    }

    /// Adds a commitment unless its identity already exists.
    pub fn add_commitment(
        &mut self,
        commitment: ManagementCommitment,
    ) -> Result<(), OrganizationDomainError> {
        if self
            .commitments
            .iter()
            .any(|existing| existing.id == commitment.id)
        {
            return Err(duplicate_identity(
                "management commitment",
                commitment.id.as_str(),
            ));
        }
        self.commitments.push(commitment);
        Ok(())
    }

    /// Adds a responsibility unless its identity already exists.
    pub fn add_responsibility(
        &mut self,
        responsibility: Responsibility,
    ) -> Result<(), OrganizationDomainError> {
        if self
            .responsibilities
            .iter()
            .any(|existing| existing.id == responsibility.id)
        {
            return Err(duplicate_identity(
                "responsibility",
                responsibility.id.as_str(),
            ));
        }
        self.responsibilities.push(responsibility);
        Ok(())
    }

    /// Adds a budget unless its identity already exists.
    pub fn add_budget(&mut self, budget: Budget) -> Result<(), OrganizationDomainError> {
        if self.budgets.iter().any(|existing| existing.id == budget.id) {
            return Err(duplicate_identity("budget", budget.id.as_str()));
        }
        self.budgets.push(budget);
        Ok(())
    }

    /// Adds a decision right unless its identity already exists.
    pub fn add_decision_right(
        &mut self,
        decision_right: DecisionRight,
    ) -> Result<(), OrganizationDomainError> {
        if self
            .decision_rights
            .iter()
            .any(|existing| existing.id == decision_right.id)
        {
            return Err(duplicate_identity(
                "decision right",
                decision_right.id.as_str(),
            ));
        }
        self.decision_rights.push(decision_right);
        Ok(())
    }

    /// Adds an adaptation unless its identity already exists.
    pub fn add_adaptation(
        &mut self,
        adaptation: OrganizationAdaptation,
    ) -> Result<(), OrganizationDomainError> {
        if self
            .adaptations
            .iter()
            .any(|existing| existing.id == adaptation.id)
        {
            return Err(duplicate_identity(
                "organization adaptation",
                adaptation.id.as_str(),
            ));
        }
        self.adaptations.push(adaptation);
        Ok(())
    }
}
