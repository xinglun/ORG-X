//! Deterministic in-memory validation-history adapter for tests and local use.

use crate::features::validation::application::validation_store::{
    ValidationStore, ValidationStoreError,
};
use crate::features::validation::domain::ValidationRecord;

#[cfg(test)]
#[path = "in_memory_store_test.rs"]
mod in_memory_store_test;

/// In-memory validation store that rejects company overwrites.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryValidationStore {
    records: Vec<ValidationRecord>,
}

impl InMemoryValidationStore {
    /// Creates an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ValidationStore for InMemoryValidationStore {
    fn save(&mut self, record: ValidationRecord) -> Result<(), ValidationStoreError> {
        if self
            .records
            .iter()
            .any(|existing| existing.company_id() == record.company_id())
        {
            return Err(ValidationStoreError::DuplicateCompany {
                company_id: record.company_id().to_owned(),
            });
        }
        self.records.push(record);
        Ok(())
    }

    fn get(&self, company_id: &str) -> Option<&ValidationRecord> {
        self.records
            .iter()
            .find(|record| record.company_id() == company_id)
    }

    fn records(&self) -> &[ValidationRecord] {
        &self.records
    }
}
