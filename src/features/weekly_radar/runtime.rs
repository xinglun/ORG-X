//! Provider-neutral runtime boundaries for Weekly Radar collection and reporting.
//!
//! Provider-specific payloads belong in the runtime adapters that consume these
//! boundaries. The normalized model is intentionally independent of any one
//! external service.

pub mod archive;
pub mod config;
pub mod error;
pub mod http;
pub mod model;
pub mod report;
pub mod rules;
pub mod sec;
pub mod sources;
pub mod telegram;

pub use archive::{retain_recent, write_run, ArchiveError, ArchiveManifest};
pub use config::{CompanyConfig, CompanySourceRegistry};
pub use error::RuntimeError;
pub use http::{
    FixtureHttpClient, HttpClient, HttpResponse, HttpTimeouts, UreqHttpClient,
    MAX_HTTP_RESPONSE_BODY_BYTES,
};
pub use model::{
    Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput, SourceCoverage,
};
pub use report::{render_report, RenderedReport, SnapshotMetadata, SourceHealthFacts};
pub use rules::extract_employee_count;
pub use sec::{CompanyEvidence, SecClient};
pub use sources::{
    collect_configured_sources, SourceKind, SourceObservation, SourceStatus, SourceTier,
    MAX_HIRING_RECORDS, MAX_SOURCE_BODY_BYTES,
};
pub use telegram::{
    send_rendered_report, send_rendered_report_with_transport, EnvTelegramTransport,
    TelegramDeliveryReceipt, TelegramError, TelegramRetryPolicy,
};
