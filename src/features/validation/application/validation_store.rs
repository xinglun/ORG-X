//! Application port for validation-history persistence.

use crate::features::validation::domain::ValidationRecord;
use std::fmt;

#[cfg(test)]
#[path = "validation_store_test.rs"]
mod validation_store_test;

/// Persistence failures for validation histories.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValidationStoreError {
    /// A store already contains a record for the company.
    DuplicateCompany { company_id: String },
}

impl fmt::Display for ValidationStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCompany { company_id } => {
                write!(
                    formatter,
                    "validation record already exists for company {company_id}"
                )
            }
        }
    }
}

impl std::error::Error for ValidationStoreError {}

/// Persistence port for immutable-by-company validation histories.
pub trait ValidationStore {
    /// Saves a new company history without overwriting an existing one.
    fn save(&mut self, record: ValidationRecord) -> Result<(), ValidationStoreError>;

    /// Finds a company history by its opaque company identity.
    fn get(&self, company_id: &str) -> Option<&ValidationRecord>;

    /// Returns histories in deterministic insertion order.
    fn records(&self) -> &[ValidationRecord];
}
