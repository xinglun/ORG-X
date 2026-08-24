//! Provider-neutral runtime boundaries for Weekly Radar collection and reporting.
//!
//! Provider-specific payloads belong in the runtime adapters that consume these
//! boundaries. The normalized model is intentionally independent of any one
//! external service.

pub mod archive;
pub mod config;
pub mod error;
pub mod http;
pub mod judgment;
pub mod model;
pub mod report;
pub mod rules;
pub mod sec;
pub mod sources;
pub mod telegram;

pub use archive::{
    acquire_run_lock, ensure_run_available, load_input_snapshot, persist_input_snapshot,
    recover_pending_run, retain_recent, verify_committed_run, verify_committed_run_read_only,
    write_run, write_run_with_input_snapshot, ArchiveError, ArchiveManifest, ArchiveRunLock,
    InputSnapshot, INPUT_SNAPSHOT_SCHEMA_VERSION,
};
pub use config::{CompanyConfig, CompanySourceRegistry};
pub use error::RuntimeError;
pub use http::{
    FixtureHttpClient, HttpClient, HttpResponse, HttpTimeouts, UreqHttpClient,
    MAX_HTTP_RESPONSE_BODY_BYTES,
};
pub use judgment::{
    derive_judgment_snapshot, derive_judgment_snapshot_for_companies, HumanReference,
    JudgmentSnapshot, MachineStage,
};
pub use model::{
    CompanyIdentity, Confidence, FactStatus, NormalizedFact, Provenance, RuntimeReportInput,
    SourceCoverage, SourceFailure,
};
pub use report::{
    render_report, render_report_in_language, RenderedReport, ReportLanguage, SnapshotMetadata,
    SourceHealthFacts,
};
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

/// Converts one acquired source observation into a provider-neutral fact.
///
/// `index` is a one-based, stable index within the observation's source kind
/// for one company. Official known observations retain their text as a
/// confirmed value. Structured hiring and discovery observations retain their
/// text in provenance while remaining unconfirmed, and missing/ambiguous
/// observations retain no concrete value.
pub fn normalize_source_observation(
    observation: &SourceObservation,
    index: usize,
) -> Result<NormalizedFact, RuntimeError> {
    if index == 0 {
        return Err(RuntimeError::invalid_model(
            "source observation index must be one-based",
        ));
    }

    let kind = format!("source_{}_{index:03}", observation.kind().as_str());
    let status = match observation.status() {
        SourceStatus::Known if observation.tier() == SourceTier::OfficialPrimary => {
            FactStatus::Known
        }
        SourceStatus::Known | SourceStatus::DiscoveryOnly => FactStatus::Unconfirmed,
        SourceStatus::Unknown => FactStatus::Unknown,
        SourceStatus::Unavailable | SourceStatus::NotConfigured | SourceStatus::NotApplicable => {
            FactStatus::Unavailable
        }
    };
    let confidence = match status {
        FactStatus::Known => Confidence::High,
        FactStatus::Unconfirmed => match observation.tier() {
            SourceTier::StructuredHiring => Confidence::Medium,
            SourceTier::DiscoveryOnly => Confidence::Low,
            SourceTier::OfficialPrimary => Confidence::Unknown,
        },
        FactStatus::Unknown | FactStatus::Unavailable => Confidence::Unknown,
    };
    let passage = if status == FactStatus::Known || observation.text().trim().is_empty() {
        observation
            .provenance()
            .source_field_or_passage()
            .to_owned()
    } else {
        format!(
            "{}; passage: {}",
            observation.provenance().source_field_or_passage(),
            observation.text()
        )
    };
    let provenance = Provenance::new(
        observation.provenance().source_uri(),
        passage,
        *observation.provenance().retrieved_at(),
        observation.provenance().effective_date().copied(),
    )?;

    if status == FactStatus::Known {
        if observation.text().trim().is_empty() {
            return Err(RuntimeError::invalid_model(
                "confirmed source observation text cannot be blank",
            ));
        }
        NormalizedFact::new(
            observation.company_id(),
            kind,
            observation.text(),
            status,
            confidence,
            provenance,
        )
    } else {
        NormalizedFact::without_value(
            observation.company_id(),
            kind,
            status,
            confidence,
            provenance,
        )
    }
}
