//! Completeness assessment without research judgment or arithmetic.

use crate::features::validation::domain::{ValidationHorizon, ValidationRecord};

#[cfg(test)]
#[path = "validation_evaluator_test.rs"]
mod validation_evaluator_test;

/// Whether all three follow-up horizons have been retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationReadiness {
    /// T0 plus all required follow-up horizons are present.
    Complete,
    /// At least one required follow-up horizon is absent.
    Incomplete,
}

/// A validation-history completeness result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationAssessment {
    company_id: String,
    missing_horizons: Vec<ValidationHorizon>,
    readiness: ValidationReadiness,
}

impl ValidationAssessment {
    /// Returns the company identity assessed.
    pub fn company_id(&self) -> &str {
        &self.company_id
    }

    /// Returns required follow-up horizons absent from the record.
    pub fn missing_horizons(&self) -> &[ValidationHorizon] {
        &self.missing_horizons
    }

    /// Returns completeness only; no Stage or score is inferred.
    pub const fn readiness(&self) -> ValidationReadiness {
        self.readiness
    }
}

/// Application service that reports retention completeness only.
pub struct ValidationEvaluator;

impl ValidationEvaluator {
    /// Assesses missing horizons without evaluating the supplied facts.
    pub fn assess(record: &ValidationRecord) -> ValidationAssessment {
        let missing_horizons = record.missing_horizons();
        let readiness = if missing_horizons.is_empty() {
            ValidationReadiness::Complete
        } else {
            ValidationReadiness::Incomplete
        };
        ValidationAssessment {
            company_id: record.company_id().to_owned(),
            missing_horizons,
            readiness,
        }
    }
}
