//! Pure measured productivity facts retained for comparable research history.

use std::fmt;

#[cfg(test)]
mod mod_test;

fn duplicate_identity(entity: &'static str, id: impl Into<String>) -> ProductivityDomainError {
    ProductivityDomainError::DuplicateIdentity {
        entity,
        id: id.into(),
    }
}

use crate::shared::domain::text_value as shared_text_value;

macro_rules! text_value {
    ($name:ident, $field:literal, $description:literal) => {
        shared_text_value!($name, $field, $description, ProductivityDomainError);
    };
}

text_value!(
    CompanyReference,
    "company reference",
    "Opaque company reference for a productivity history."
);
text_value!(
    ProductivitySnapshotId,
    "productivity snapshot id",
    "Stable identity for one productivity observation."
);
text_value!(
    Period,
    "period",
    "Opaque reporting period retained with an observation."
);
text_value!(MetricValue, "metric value", "Opaque measured metric value.");
text_value!(
    MetricUnit,
    "metric unit",
    "Unit retained with a measured metric value."
);
text_value!(
    EmployeeCount,
    "employee count",
    "Opaque employee denominator retained with a metric."
);
text_value!(
    GrowthRate,
    "growth rate",
    "Opaque growth fact retained with a snapshot."
);
text_value!(
    HeadcountChange,
    "headcount change",
    "Opaque headcount-change fact retained with a snapshot."
);

/// Validation and collection failures for productivity facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductivityDomainError {
    /// A required boundary value contained only whitespace.
    EmptyValue { field: &'static str },
    /// A snapshot identity already exists in a history.
    DuplicateIdentity { entity: &'static str, id: String },
}

impl fmt::Display for ProductivityDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyValue { field } => write!(formatter, "{field} cannot be empty"),
            Self::DuplicateIdentity { entity, id } => {
                write!(formatter, "duplicate {entity} identity {id}")
            }
        }
    }
}

impl std::error::Error for ProductivityDomainError {}

macro_rules! per_employee_metric {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name {
            value: MetricValue,
            unit: MetricUnit,
            employee_count: EmployeeCount,
        }

        impl $name {
            /// Creates a per-employee fact without arithmetic or unit conversion.
            pub fn new(
                value: impl Into<String>,
                unit: impl Into<String>,
                employee_count: impl Into<String>,
            ) -> Result<Self, ProductivityDomainError> {
                Ok(Self {
                    value: MetricValue::new(value)?,
                    unit: MetricUnit::new(unit)?,
                    employee_count: EmployeeCount::new(employee_count)?,
                })
            }

            /// Returns the measured value.
            pub fn value(&self) -> &MetricValue {
                &self.value
            }

            /// Returns the supplied unit.
            pub fn unit(&self) -> &MetricUnit {
                &self.unit
            }

            /// Returns the employee denominator supplied with the fact.
            pub fn employee_count(&self) -> &EmployeeCount {
                &self.employee_count
            }
        }
    };
}

per_employee_metric!(
    RevenuePerEmployee,
    "Revenue per employee measured fact with value, unit, and denominator."
);
per_employee_metric!(
    OperatingIncomePerEmployee,
    "Operating income per employee measured fact with value, unit, and denominator."
);
per_employee_metric!(
    FreeCashFlowPerEmployee,
    "Free cash flow per employee measured fact with value, unit, and denominator."
);

/// Growth and headcount facts retained together for one observation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GrowthAndHeadcount {
    revenue_growth: Option<GrowthRate>,
    operating_income_growth: Option<GrowthRate>,
    free_cash_flow_growth: Option<GrowthRate>,
    headcount_change: HeadcountChange,
}

impl GrowthAndHeadcount {
    /// Creates supplied growth facts and a headcount change fact.
    pub fn new(
        revenue_growth: Option<GrowthRate>,
        operating_income_growth: Option<GrowthRate>,
        free_cash_flow_growth: Option<GrowthRate>,
        headcount_change: HeadcountChange,
    ) -> Result<Self, ProductivityDomainError> {
        Ok(Self {
            revenue_growth,
            operating_income_growth,
            free_cash_flow_growth,
            headcount_change,
        })
    }

    /// Returns the revenue growth fact when supplied.
    pub fn revenue_growth(&self) -> Option<&GrowthRate> {
        self.revenue_growth.as_ref()
    }

    /// Returns the operating-income growth fact when supplied.
    pub fn operating_income_growth(&self) -> Option<&GrowthRate> {
        self.operating_income_growth.as_ref()
    }

    /// Returns the free-cash-flow growth fact when supplied.
    pub fn free_cash_flow_growth(&self) -> Option<&GrowthRate> {
        self.free_cash_flow_growth.as_ref()
    }

    /// Returns the supplied headcount change fact.
    pub fn headcount_change(&self) -> &HeadcountChange {
        &self.headcount_change
    }
}

/// One period of measured productivity facts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductivitySnapshot {
    id: ProductivitySnapshotId,
    period: Period,
    revenue_per_employee: Option<RevenuePerEmployee>,
    operating_income_per_employee: Option<OperatingIncomePerEmployee>,
    free_cash_flow_per_employee: Option<FreeCashFlowPerEmployee>,
    growth_and_headcount: GrowthAndHeadcount,
}

impl ProductivitySnapshot {
    /// Creates a snapshot while preserving missing metric values as `None`.
    pub fn new(
        id: ProductivitySnapshotId,
        period: impl Into<String>,
        revenue_per_employee: Option<RevenuePerEmployee>,
        operating_income_per_employee: Option<OperatingIncomePerEmployee>,
        free_cash_flow_per_employee: Option<FreeCashFlowPerEmployee>,
        growth_and_headcount: GrowthAndHeadcount,
    ) -> Result<Self, ProductivityDomainError> {
        Ok(Self {
            id,
            period: Period::new(period)?,
            revenue_per_employee,
            operating_income_per_employee,
            free_cash_flow_per_employee,
            growth_and_headcount,
        })
    }

    /// Returns the snapshot identity.
    pub fn id(&self) -> &ProductivitySnapshotId {
        &self.id
    }

    /// Returns the opaque observation period.
    pub fn period(&self) -> &Period {
        &self.period
    }

    /// Returns the revenue-per-employee fact when supplied.
    pub fn revenue_per_employee(&self) -> Option<&RevenuePerEmployee> {
        self.revenue_per_employee.as_ref()
    }

    /// Returns the operating-income-per-employee fact when supplied.
    pub fn operating_income_per_employee(&self) -> Option<&OperatingIncomePerEmployee> {
        self.operating_income_per_employee.as_ref()
    }

    /// Returns the free-cash-flow-per-employee fact when supplied.
    pub fn free_cash_flow_per_employee(&self) -> Option<&FreeCashFlowPerEmployee> {
        self.free_cash_flow_per_employee.as_ref()
    }

    /// Returns the growth and headcount facts.
    pub fn growth_and_headcount(&self) -> &GrowthAndHeadcount {
        &self.growth_and_headcount
    }
}

/// An ordered productivity history for one company.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductivityHistory {
    company: CompanyReference,
    snapshots: Vec<ProductivitySnapshot>,
}

impl ProductivityHistory {
    /// Creates an empty history for a validated company reference.
    pub fn new(company: CompanyReference) -> Self {
        Self {
            company,
            snapshots: Vec::new(),
        }
    }

    /// Returns the company reference.
    pub fn company(&self) -> &CompanyReference {
        &self.company
    }

    /// Returns snapshots in insertion order.
    pub fn snapshots(&self) -> &[ProductivitySnapshot] {
        &self.snapshots
    }

    /// Adds a snapshot unless its identity already exists.
    pub fn add_snapshot(
        &mut self,
        snapshot: ProductivitySnapshot,
    ) -> Result<(), ProductivityDomainError> {
        if self
            .snapshots
            .iter()
            .any(|existing| existing.id == snapshot.id)
        {
            return Err(duplicate_identity(
                "productivity snapshot",
                snapshot.id.as_str(),
            ));
        }
        self.snapshots.push(snapshot);
        Ok(())
    }
}
