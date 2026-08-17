//! Provider-neutral runtime boundaries for Weekly Radar collection and reporting.
//!
//! Provider-specific payloads belong in the runtime adapters that consume these
//! boundaries. The normalized model is intentionally independent of any one
//! external service.

pub mod config;
pub mod error;
pub mod http;
pub mod model;

pub use config::{CompanyConfig, CompanySourceRegistry};
pub use error::RuntimeError;
pub use http::{FixtureHttpClient, HttpClient, HttpResponse, UreqHttpClient};
pub use model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};
