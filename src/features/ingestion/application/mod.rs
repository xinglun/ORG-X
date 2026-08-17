//! Application ports for collecting observations into the Ingestion Domain.

use super::domain::{IngestionDomainError, Observation};

#[cfg(test)]
mod mod_test;

/// Request context passed from an application use case to a source adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestionRequest {
    source_key: String,
}

impl IngestionRequest {
    /// Creates a request for a named source boundary.
    pub fn new(source_key: impl Into<String>) -> Self {
        Self {
            source_key: source_key.into(),
        }
    }

    /// Returns the source key without interpreting it.
    pub fn source_key(&self) -> &str {
        &self.source_key
    }
}

/// Errors returned by an observation collection port.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceCollectionError {
    /// The source returned a domain-invalid observation.
    Domain(IngestionDomainError),
    /// The source could not provide observations for this request.
    Unavailable { source: String },
}

impl From<IngestionDomainError> for SourceCollectionError {
    fn from(error: IngestionDomainError) -> Self {
        Self::Domain(error)
    }
}

/// Port implemented by an external source adapter outside the Domain layer.
pub trait ObservationSource {
    /// Collects opaque observations for one application request.
    fn collect(
        &self,
        request: &IngestionRequest,
    ) -> Result<Vec<Observation>, SourceCollectionError>;
}
