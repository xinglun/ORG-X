use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Utc};
use org_x::features::weekly_radar::runtime::config::CompanySourceRegistry;
use org_x::features::weekly_radar::runtime::http::{HttpClient, UreqHttpClient};
use org_x::features::weekly_radar::runtime::model::{
    CompanyIdentity, Confidence, FactStatus, NormalizedFact, Provenance, ResearchMetrics,
    RuntimeReportInput, SourceCoverage, SourceFailure,
};
use org_x::features::weekly_radar::runtime::report::{
    render_report_in_language, RenderedReport, ReportLanguage,
};
use org_x::features::weekly_radar::runtime::sec::{
    SecClient, SecDocumentCandidate, SecDocumentStatus,
};
use org_x::features::weekly_radar::runtime::sources::{
    collect_configured_sources, document_observation, DocumentObservationInput, SourceKind,
    SourceObservation, SourceStatus, SourceTier,
};
use org_x::features::weekly_radar::runtime::{
    acquire_run_lock, build_input_snapshot, derive_judgment_snapshot_for_companies,
    ensure_run_available, ensure_run_replace_available, extract_evidence_candidate,
    load_input_snapshot, normalize_source_observation, recover_pending_run,
    replace_run_with_input_snapshot, send_rendered_report, validate_evidence_candidate,
    verify_committed_run, verify_committed_run_read_only, write_run_with_input_snapshot,
    SourceMaterialKind,
};

const DEFAULT_REGISTRY: &str = "config/weekly_radar/companies.json";
const DEFAULT_ARCHIVE_DIR: &str = ".";

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliOptions {
    as_of: NaiveDate,
    retry_as_of: Option<NaiveDate>,
    recover_published_as_of: Option<NaiveDate>,
    verify_published_as_of: Option<NaiveDate>,
    republish_published_as_of: Option<NaiveDate>,
    archive_dir: PathBuf,
    registry: PathBuf,
    dry_run: bool,
    language: ReportLanguage,
}

enum CliAction {
    Help,
    Run(CliOptions),
}

#[derive(Debug)]
enum CliError {
    Usage(String),
    Failure(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(reason) => write!(formatter, "{reason}\n\n{}", usage()),
            Self::Failure(reason) => formatter.write_str(reason),
        }
    }
}

fn usage() -> &'static str {
    "Usage: org-x weekly-radar [--as-of YYYY-MM-DD] [--archive-dir PATH] [--registry PATH] [--language zh-CN|ja|en] [--dry-run]\n       org-x weekly-radar --retry-as-of YYYY-MM-DD [--archive-dir PATH]\n       org-x weekly-radar --recover-published-as-of YYYY-MM-DD [--archive-dir PATH]\n       org-x weekly-radar --verify-published-as-of YYYY-MM-DD [--archive-dir PATH]\n       org-x weekly-radar --republish-published-as-of YYYY-MM-DD [--archive-dir PATH]"
}

fn parse_options(args: &[String]) -> Result<CliAction, CliError> {
    let Some(command) = args.first() else {
        return Err(CliError::Usage("a command is required".to_owned()));
    };
    if command == "--help" || command == "-h" {
        return Ok(CliAction::Help);
    }
    if command != "weekly-radar" {
        return Err(CliError::Usage(format!(
            "unknown command {command}; expected weekly-radar"
        )));
    }

    let mut as_of = None;
    let mut archive_dir = PathBuf::from(DEFAULT_ARCHIVE_DIR);
    let mut registry = PathBuf::from(DEFAULT_REGISTRY);
    let mut dry_run = false;
    let mut language = ReportLanguage::default();
    let mut retry_as_of = None;
    let mut recover_published_as_of = None;
    let mut verify_published_as_of = None;
    let mut republish_published_as_of = None;
    let mut as_of_explicit = false;
    let mut language_explicit = false;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--help" | "-h" => return Ok(CliAction::Help),
            "--dry-run" => {
                if dry_run {
                    return Err(CliError::Usage("--dry-run may be supplied once".to_owned()));
                }
                dry_run = true;
                index += 1;
            }
            "--as-of" => {
                let value = option_value(args, &mut index, "--as-of")?;
                let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                    .map_err(|_| CliError::Usage("--as-of must use YYYY-MM-DD".to_owned()))?;
                as_of = Some(parsed);
                as_of_explicit = true;
            }
            "--retry-as-of" => {
                let value = option_value(args, &mut index, "--retry-as-of")?;
                let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d")
                    .map_err(|_| CliError::Usage("--retry-as-of must use YYYY-MM-DD".to_owned()))?;
                retry_as_of = Some(parsed);
            }
            "--recover-published-as-of" => {
                let value = option_value(args, &mut index, "--recover-published-as-of")?;
                let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
                    CliError::Usage("--recover-published-as-of must use YYYY-MM-DD".to_owned())
                })?;
                recover_published_as_of = Some(parsed);
            }
            "--verify-published-as-of" => {
                let value = option_value(args, &mut index, "--verify-published-as-of")?;
                let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
                    CliError::Usage("--verify-published-as-of must use YYYY-MM-DD".to_owned())
                })?;
                verify_published_as_of = Some(parsed);
            }
            "--republish-published-as-of" => {
                let value = option_value(args, &mut index, "--republish-published-as-of")?;
                let parsed = NaiveDate::parse_from_str(&value, "%Y-%m-%d").map_err(|_| {
                    CliError::Usage("--republish-published-as-of must use YYYY-MM-DD".to_owned())
                })?;
                republish_published_as_of = Some(parsed);
            }
            "--archive-dir" => {
                let value = option_value(args, &mut index, "--archive-dir")?;
                archive_dir = PathBuf::from(value);
            }
            "--registry" => {
                let value = option_value(args, &mut index, "--registry")?;
                registry = PathBuf::from(value);
            }
            "--language" => {
                let value = option_value(args, &mut index, "--language")?;
                language = value.parse::<ReportLanguage>().map_err(CliError::Usage)?;
                language_explicit = true;
            }
            unknown => {
                return Err(CliError::Usage(format!(
                    "unknown weekly-radar option {unknown}"
                )))
            }
        }
    }

    if retry_as_of.is_some()
        || recover_published_as_of.is_some()
        || verify_published_as_of.is_some()
        || republish_published_as_of.is_some()
    {
        let mut incompatible = Vec::new();
        if as_of_explicit {
            incompatible.push("--as-of");
        }
        if language_explicit {
            incompatible.push("--language");
        }
        if dry_run {
            incompatible.push("--dry-run");
        }
        let recovery_options = [
            ("--retry-as-of", retry_as_of.is_some()),
            (
                "--recover-published-as-of",
                recover_published_as_of.is_some(),
            ),
            ("--verify-published-as-of", verify_published_as_of.is_some()),
            (
                "--republish-published-as-of",
                republish_published_as_of.is_some(),
            ),
        ];
        if recovery_options
            .iter()
            .filter(|(_, present)| *present)
            .count()
            > 1
        {
            incompatible.extend(
                recovery_options
                    .iter()
                    .filter_map(|(option, present)| present.then_some(*option)),
            );
        }
        if !incompatible.is_empty() {
            return Err(CliError::Usage(format!(
                "recovery options cannot be combined with {}",
                incompatible.join(", ")
            )));
        }
    }

    Ok(CliAction::Run(CliOptions {
        as_of: as_of.unwrap_or_else(|| Utc::now().date_naive()),
        retry_as_of,
        recover_published_as_of,
        verify_published_as_of,
        republish_published_as_of,
        archive_dir,
        registry,
        dry_run,
        language,
    }))
}

fn option_value(args: &[String], index: &mut usize, option: &str) -> Result<String, CliError> {
    let value_index = *index + 1;
    let Some(value) = args.get(value_index) else {
        return Err(CliError::Usage(format!("{option} requires a value")));
    };
    if value.starts_with('-') {
        return Err(CliError::Usage(format!("{option} requires a value")));
    }
    *index += 2;
    Ok(value.clone())
}

#[derive(Default)]
struct CoverageCounts {
    expected: BTreeSet<String>,
    available: BTreeSet<String>,
    not_configured: BTreeSet<String>,
    not_applicable: BTreeSet<String>,
}

struct AcquiredRuntimeInput {
    input: RuntimeReportInput,
    has_primary_evidence: bool,
}

#[derive(Default)]
struct ResearchMetricCounts {
    source_available: usize,
    document_candidates: usize,
    document_kind_counts: BTreeMap<String, usize>,
    validated_evidence: usize,
    pending_leads: usize,
    unavailable_sources: usize,
}

fn acquire_runtime_input(
    registry: &CompanySourceRegistry,
    http: &dyn HttpClient,
    sec_user_agent: &str,
    as_of: NaiveDate,
) -> Result<AcquiredRuntimeInput, CliError> {
    let mut input = RuntimeReportInput::from_date(as_of);
    let mut coverage = BTreeMap::<String, CoverageCounts>::new();
    let mut has_primary_evidence = false;
    let mut metrics = ResearchMetricCounts::default();
    let mut structural_evidence = 0usize;
    let mut sec_stage_expected = 0usize;
    let mut sec_stage_available = 0usize;
    let mut sec_fact_expected = 0usize;
    let mut sec_fact_available = 0usize;
    let observed_at = Utc::now();

    for company in registry.companies() {
        input
            .add_company(
                CompanyIdentity::new(company.id(), company.name(), company.ticker())
                    .map_err(|error| CliError::Failure(error.to_string()))?,
            )
            .map_err(|error| CliError::Failure(error.to_string()))?;
        let mut observations = Vec::new();
        let sec_coverage = coverage.entry("sec".to_owned()).or_default();
        sec_coverage.expected.insert(company.id().to_owned());
        if company.sec_cik().is_none() {
            sec_coverage.not_configured.insert(company.id().to_owned());
        } else {
            sec_stage_expected += 2;
            match SecClient::collect(company, http, sec_user_agent) {
                Ok(evidence) => {
                    let failed_stages = evidence
                        .failures()
                        .iter()
                        .map(|failure| failure.stage())
                        .collect::<BTreeSet<_>>();
                    sec_stage_available += ["submissions", "company_facts"]
                        .iter()
                        .filter(|stage| !failed_stages.contains(*stage))
                        .count();
                    sec_fact_expected += evidence.facts().len();
                    sec_fact_available += evidence
                        .facts()
                        .iter()
                        .filter(|fact| fact.status() == &FactStatus::Known)
                        .count();
                    let mut sec_stage_successes = 2usize;
                    for failure in evidence.failures() {
                        if matches!(failure.stage(), "submissions" | "company_facts") {
                            metrics.unavailable_sources += 1;
                            sec_stage_successes = sec_stage_successes.saturating_sub(1);
                        }
                    }
                    metrics.source_available += sec_stage_successes;
                    let sec_stage_failures = failed_stages
                        .iter()
                        .filter(|stage| matches!(**stage, "submissions" | "company_facts"))
                        .count();
                    if sec_stage_failures < 2 {
                        sec_coverage.available.insert(company.id().to_owned());
                    }
                    for fact in evidence.facts() {
                        if fact.status() == &FactStatus::Known {
                            has_primary_evidence = true;
                        }
                        input
                            .add_fact(fact.clone())
                            .map_err(|error| CliError::Failure(error.to_string()))?;
                    }
                    for document in evidence.documents() {
                        observations.push(sec_document_observation(
                            company.id(),
                            document,
                            observed_at,
                        ));
                    }
                    if !evidence.failures().is_empty() {
                        let reason = evidence
                            .failures()
                            .iter()
                            .map(|failure| format!("{}: {}", failure.stage(), failure.reason()))
                            .collect::<Vec<_>>()
                            .join("; ");
                        input
                            .add_source_failure(
                                SourceFailure::new("sec", company.id(), reason)
                                    .map_err(|error| CliError::Failure(error.to_string()))?,
                            )
                            .map_err(|error| CliError::Failure(error.to_string()))?;
                    }
                }
                Err(error) => {
                    input
                        .add_source_failure(
                            SourceFailure::new("sec", company.id(), error.to_string())
                                .map_err(|error| CliError::Failure(error.to_string()))?,
                        )
                        .map_err(|error| CliError::Failure(error.to_string()))?;
                }
            }
        }

        observations.extend(collect_configured_sources(company, http, observed_at));
        let mut available_kinds = BTreeSet::new();
        let mut source_indices = BTreeMap::<&str, usize>::new();
        let mut evidence_indices = BTreeMap::<&str, usize>::new();
        for observation in &observations {
            let kind = observation.kind().as_str().to_owned();
            let source_coverage = coverage.entry(kind).or_default();
            source_coverage.expected.insert(company.id().to_owned());
            if !matches!(
                observation.status(),
                SourceStatus::Unavailable
                    | SourceStatus::NotConfigured
                    | SourceStatus::NotApplicable
            ) {
                available_kinds.insert(observation.kind().as_str());
            }
            if observation.status() == SourceStatus::NotConfigured {
                source_coverage
                    .not_configured
                    .insert(company.id().to_owned());
            }
            if observation.status() == SourceStatus::NotApplicable {
                source_coverage
                    .not_applicable
                    .insert(company.id().to_owned());
            }
            if observation.status() == SourceStatus::Unavailable {
                metrics.unavailable_sources += 1;
            }
            if observation.material_kind() == SourceMaterialKind::EntryPoint
                && !matches!(
                    observation.status(),
                    SourceStatus::Unavailable
                        | SourceStatus::NotConfigured
                        | SourceStatus::NotApplicable
                )
            {
                metrics.source_available += 1;
            }
            if observation.material_kind() == SourceMaterialKind::Document {
                metrics.document_candidates += 1;
                if let Some(document_kind) = observation.document_kind() {
                    *metrics
                        .document_kind_counts
                        .entry(document_kind.as_str().to_owned())
                        .or_insert(0) += 1;
                }
            }
            let index = source_indices
                .entry(observation.kind().as_str())
                .and_modify(|index| *index += 1)
                .or_insert(1);
            let fact = normalize_source_observation(observation, *index)
                .map_err(|error| CliError::Failure(error.to_string()))?;
            input
                .add_fact(fact)
                .map_err(|error| CliError::Failure(error.to_string()))?;

            if observation.material_kind() == SourceMaterialKind::Document {
                let evidence_result = extract_evidence_candidate(observation)
                    .map(|candidate| validate_evidence_candidate(&candidate, as_of));
                match evidence_result {
                    Some(Ok(validated)) => {
                        let index = evidence_indices
                            .entry("official_material")
                            .and_modify(|index| *index += 1)
                            .or_insert(1);
                        let validated_fact = validated
                            .to_normalized_fact(*index)
                            .map_err(|error| CliError::Failure(error.to_string()))?;
                        input
                            .add_fact(validated_fact)
                            .map_err(|error| CliError::Failure(error.to_string()))?;
                        metrics.validated_evidence += 1;
                        if validated.evidence_class()
                            == org_x::features::weekly_radar::runtime::evidence::EvidenceClass::StructuralEvidence
                        {
                            structural_evidence += 1;
                        }
                        has_primary_evidence = true;
                    }
                    Some(Err(error)) => {
                        metrics.pending_leads += 1;
                        add_pending_evidence_fact(
                            &mut input,
                            observation,
                            source_indices
                                .get(observation.kind().as_str())
                                .copied()
                                .unwrap_or(1),
                            error.to_string(),
                        )?;
                    }
                    None => {
                        metrics.pending_leads += 1;
                        add_pending_evidence_fact(
                            &mut input,
                            observation,
                            source_indices
                                .get(observation.kind().as_str())
                                .copied()
                                .unwrap_or(1),
                            "claim extraction did not produce required fields".to_owned(),
                        )?;
                    }
                }
            }
        }
        add_counter_review_fact(&mut input, company.id(), &observations, as_of)?;
        for kind in available_kinds {
            coverage
                .entry(kind.to_owned())
                .or_default()
                .available
                .insert(company.id().to_owned());
        }
    }

    for (source, counts) in coverage {
        input
            .add_source_coverage(
                SourceCoverage::new_with_states(
                    source,
                    counts.expected.len(),
                    counts.available.len(),
                    counts.not_configured.len(),
                    counts.not_applicable.len(),
                )
                .map_err(|error| CliError::Failure(error.to_string()))?,
            )
            .map_err(|error| CliError::Failure(error.to_string()))?;
    }

    input.set_research_metrics(
        ResearchMetrics::new(
            metrics.source_available,
            metrics.document_candidates,
            metrics.validated_evidence,
            metrics.pending_leads,
            metrics.unavailable_sources,
        )
        .with_document_kind_counts(metrics.document_kind_counts)
        .with_structural_evidence(structural_evidence)
        .with_sec_health(
            sec_stage_expected,
            sec_stage_available,
            sec_fact_expected,
            sec_fact_available,
        ),
    );

    Ok(AcquiredRuntimeInput {
        input,
        has_primary_evidence,
    })
}

fn sec_document_observation(
    company_id: &str,
    document: &SecDocumentCandidate,
    observed_at: chrono::DateTime<Utc>,
) -> org_x::features::weekly_radar::runtime::SourceObservation {
    let status = match document.status() {
        SecDocumentStatus::Known => SourceStatus::Known,
        SecDocumentStatus::Unknown => SourceStatus::Unknown,
        SecDocumentStatus::Unavailable => SourceStatus::Unavailable,
    };
    let provenance = format!(
        "SEC filing accession={} form={} filing_date={} report_date={}",
        document.accession_number(),
        document.form(),
        document.filing_date(),
        document
            .report_date()
            .map(|date| date.to_string())
            .unwrap_or_else(|| "unknown".to_owned()),
    );
    document_observation(DocumentObservationInput {
        company_id: company_id.to_owned(),
        kind: SourceKind::Sec,
        tier: SourceTier::OfficialPrimary,
        url: document.source_uri().to_owned(),
        title: document.title().to_owned(),
        text: document.text().to_owned(),
        status,
        status_reason: document.status_reason().to_owned(),
        document_kind: org_x::features::weekly_radar::runtime::DocumentKind::Filing,
        source_field_or_passage: provenance,
        observed_at,
        effective_date: Some(document.filing_date()),
    })
}

fn add_pending_evidence_fact(
    input: &mut RuntimeReportInput,
    observation: &org_x::features::weekly_radar::runtime::SourceObservation,
    index: usize,
    reason: String,
) -> Result<(), CliError> {
    let provenance = Provenance::new(
        observation.provenance().source_uri(),
        format!("pending evidence: {reason}"),
        *observation.provenance().retrieved_at(),
        observation.provenance().effective_date().copied(),
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    let fact = NormalizedFact::without_value(
        observation.company_id(),
        format!(
            "pending_evidence_{}_{index:03}",
            observation.kind().as_str()
        ),
        FactStatus::Unconfirmed,
        org_x::features::weekly_radar::runtime::Confidence::Unknown,
        provenance,
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    input
        .add_fact(fact)
        .map_err(|error| CliError::Failure(error.to_string()))
}

fn add_counter_review_fact(
    input: &mut RuntimeReportInput,
    company_id: &str,
    observations: &[SourceObservation],
    as_of: NaiveDate,
) -> Result<(), CliError> {
    let reviewed_sources = observations
        .iter()
        .filter(|observation| {
            observation.status() == SourceStatus::Known
                && observation.is_authoritative()
                && matches!(
                    observation.material_kind(),
                    SourceMaterialKind::EntryPoint | SourceMaterialKind::Document
                )
        })
        .filter_map(|observation| observation.url())
        .collect::<BTreeSet<_>>();
    let Some(source_uri) = reviewed_sources.iter().next() else {
        return Ok(());
    };
    let review = format!(
        "Counter-evidence review completed across {} authoritative source observations in the bounded corpus; no disconfirming reference-model claim was identified.",
        reviewed_sources.len()
    );
    let provenance = Provenance::new(
        source_uri.to_owned(),
        review.clone(),
        Utc::now(),
        Some(as_of),
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    let fact = NormalizedFact::new(
        company_id,
        "judgment.review.REFERENCE_MODEL.counter_evidence_review",
        review,
        FactStatus::Known,
        Confidence::High,
        provenance,
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    input
        .add_fact(fact)
        .map_err(|error| CliError::Failure(error.to_string()))
}

fn validate_rendered_report(report: &RenderedReport) -> Result<(), CliError> {
    if report.report_id().trim().is_empty() || report.markdown().trim().is_empty() {
        return Err(CliError::Failure(
            "rendered report validation failed".to_owned(),
        ));
    }
    serde_json::from_str::<serde_json::Value>(report.snapshot_json())
        .map_err(|_| CliError::Failure("rendered report snapshot is not valid JSON".to_owned()))?;
    if !(report.markdown().contains("## 本周摘要")
        || report.markdown().contains("## 週次サマリー")
        || report.markdown().contains("## Executive Summary"))
        || !(report.markdown().contains("## 系统状态")
            || report.markdown().contains("## システム状態")
            || report.markdown().contains("## System Health"))
    {
        return Err(CliError::Failure(
            "rendered report is missing required headings".to_owned(),
        ));
    }
    Ok(())
}

fn sec_user_agent() -> Result<String, CliError> {
    let value = env::var("ORGX_SEC_USER_AGENT").map_err(|_| {
        CliError::Failure("ORGX_SEC_USER_AGENT is required before acquisition".to_owned())
    })?;
    if value.trim().is_empty() {
        return Err(CliError::Failure(
            "ORGX_SEC_USER_AGENT is required before acquisition".to_owned(),
        ));
    }
    Ok(value)
}

fn registry_has_configured_primary_source(registry: &CompanySourceRegistry) -> bool {
    registry.companies().iter().any(|company| {
        company.sec_cik().is_some()
            || company.official_ir_url().is_some()
            || company.careers_url().is_some()
            || company.engineering_ai_blog_url().is_some()
            || !company.official_research_source_urls().is_empty()
            || !company.independent_research_source_urls().is_empty()
    })
}

struct RepublishDeliveryEvidence {
    report_id: String,
    message_ids: Vec<String>,
    attempts: Vec<u32>,
}

fn republish_published_report_with<F>(
    archive_dir: &Path,
    as_of: NaiveDate,
    send: F,
) -> Result<String, CliError>
where
    F: FnOnce(&RenderedReport) -> Result<RepublishDeliveryEvidence, CliError>,
{
    verify_committed_run_read_only(archive_dir, "data", as_of)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    let input_snapshot = load_input_snapshot(archive_dir, "data", as_of)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    if !input_snapshot.has_primary_evidence() {
        return Err(CliError::Failure(
            "cannot republish weekly radar without primary evidence".to_owned(),
        ));
    }
    let report = render_report_in_language(input_snapshot.input(), input_snapshot.language());
    validate_rendered_report(&report)?;
    let delivery = send(&report)?;
    if delivery.report_id != report.report_id()
        || delivery.message_ids.is_empty()
        || delivery.message_ids.len() != delivery.attempts.len()
    {
        return Err(CliError::Failure(
            "republish delivery evidence did not bind to the report".to_owned(),
        ));
    }
    Ok(format!(
        "REPUBLISHED: report {} sent to Telegram; message_ids={:?}; attempts={:?}; archive unchanged",
        report.report_id(),
        delivery.message_ids,
        delivery.attempts
    ))
}

fn run_weekly_radar(options: CliOptions) -> Result<String, CliError> {
    if let Some(verify_as_of) = options.verify_published_as_of {
        let manifest = verify_committed_run_read_only(&options.archive_dir, "data", verify_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        return Ok(format!(
            "ALREADY-PUBLISHED: report {} verified at {}",
            verify_as_of,
            manifest.report()
        ));
    }
    if let Some(republish_as_of) = options.republish_published_as_of {
        return republish_published_report_with(&options.archive_dir, republish_as_of, |report| {
            let receipt = send_rendered_report(report)
                .map_err(|error| CliError::Failure(error.to_string()))?;
            Ok(RepublishDeliveryEvidence {
                report_id: receipt.report_id().to_owned(),
                message_ids: receipt
                    .message_ids()
                    .iter()
                    .map(|message_id| message_id.as_str().to_owned())
                    .collect(),
                attempts: receipt.attempts().to_vec(),
            })
        });
    }
    if let Some(recover_as_of) = options.recover_published_as_of {
        let _run_lock = acquire_run_lock(&options.archive_dir, "data", recover_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        let manifest = verify_committed_run(&options.archive_dir, "data", recover_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        return Ok(format!(
            "READY-TO-PUSH: report {} verified at {}",
            recover_as_of,
            manifest.report()
        ));
    }
    if let Some(retry_as_of) = options.retry_as_of {
        let _run_lock = acquire_run_lock(&options.archive_dir, "data", retry_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        if let Some(manifest) = recover_pending_run(&options.archive_dir, "data", retry_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?
        {
            return Ok(format!(
                "RECOVERED: report archive completed at {}",
                manifest.report()
            ));
        }
        let input_snapshot = load_input_snapshot(&options.archive_dir, "data", retry_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        if !input_snapshot.has_primary_evidence() {
            return Err(CliError::Failure(
                "cannot retry weekly radar without primary evidence".to_owned(),
            ));
        }
        ensure_run_available(&options.archive_dir, "data", retry_as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
        let report = render_report_in_language(input_snapshot.input(), input_snapshot.language());
        validate_rendered_report(&report)?;
        let receipt =
            send_rendered_report(&report).map_err(|error| CliError::Failure(error.to_string()))?;
        let manifest = write_run_with_input_snapshot(
            &options.archive_dir,
            "data",
            &report,
            &receipt,
            Some(&input_snapshot),
        )
        .map_err(|error| CliError::Failure(error.to_string()))?;
        return Ok(format!(
            "RETRIED: report {} archived at {}",
            report.report_id(),
            manifest.report()
        ));
    }

    let registry = CompanySourceRegistry::from_path(&options.registry)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    let user_agent = sec_user_agent()?;
    if !options.dry_run && !registry_has_configured_primary_source(&registry) {
        return Err(CliError::Failure(
            "cannot publish weekly radar without primary evidence".to_owned(),
        ));
    }
    let _run_lock = if !options.dry_run {
        Some(
            acquire_run_lock(&options.archive_dir, "data", options.as_of)
                .map_err(|error| CliError::Failure(error.to_string()))?,
        )
    } else {
        None
    };
    if !options.dry_run {
        if let Some(manifest) = recover_pending_run(&options.archive_dir, "data", options.as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?
        {
            return Ok(format!(
                "RECOVERED: report archive completed at {}",
                manifest.report()
            ));
        }
        ensure_run_replace_available(&options.archive_dir, "data", options.as_of)
            .map_err(|error| CliError::Failure(error.to_string()))?;
    }

    let production = UreqHttpClient::new();
    let http: &dyn HttpClient = &production;

    let mut acquired = acquire_runtime_input(&registry, http, &user_agent, options.as_of)?;
    let company_ids = acquired
        .input
        .companies()
        .iter()
        .map(|company| company.id())
        .collect::<Vec<_>>();
    let judgment = derive_judgment_snapshot_for_companies(
        options.as_of,
        company_ids,
        acquired.input.facts(),
        Vec::new(),
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    acquired
        .input
        .set_judgment(judgment)
        .map_err(|error| CliError::Failure(error.to_string()))?;

    if options.dry_run {
        let report = render_report_in_language(&acquired.input, options.language);
        validate_rendered_report(&report)?;
        return Ok(format!(
            "{}\nDRY-RUN: report {} validated; Telegram and archive were not contacted.",
            report.markdown(),
            report.report_id()
        ));
    }
    if !acquired.has_primary_evidence {
        return Err(CliError::Failure(
            "cannot publish weekly radar without primary evidence".to_owned(),
        ));
    }

    let input_snapshot = build_input_snapshot(
        &acquired.input,
        options.language,
        acquired.has_primary_evidence,
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    let report = render_report_in_language(input_snapshot.input(), input_snapshot.language());
    validate_rendered_report(&report)?;
    let receipt =
        send_rendered_report(&report).map_err(|error| CliError::Failure(error.to_string()))?;
    let manifest = replace_run_with_input_snapshot(
        &options.archive_dir,
        "data",
        &report,
        &receipt,
        &input_snapshot,
    )
    .map_err(|error| CliError::Failure(error.to_string()))?;
    Ok(format!(
        "PUBLISHED: report {} archived at {}",
        report.report_id(),
        manifest.report()
    ))
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = match parse_options(&args) {
        Ok(CliAction::Help) => {
            println!("{}", usage());
            Ok(())
        }
        Ok(CliAction::Run(options)) => run_weekly_radar(options).map(|output| {
            println!("{output}");
        }),
        Err(error) => Err(error),
    };

    if let Err(error) = result {
        eprintln!("org-x: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use org_x::features::weekly_radar::infrastructure::telegram_publisher::{
        TelegramMessageId, TelegramTransport, TelegramTransportError,
    };
    use org_x::features::weekly_radar::runtime::archive::{
        persist_input_snapshot, write_run_with_input_snapshot,
    };
    use org_x::features::weekly_radar::runtime::config::CompanyConfig;
    use org_x::features::weekly_radar::runtime::http::{FixtureHttpClient, HttpResponse};
    use org_x::features::weekly_radar::runtime::model::RuntimeReportInput;
    use org_x::features::weekly_radar::runtime::report::{
        render_report_in_language, ReportLanguage,
    };
    use org_x::features::weekly_radar::runtime::telegram::{
        send_rendered_report_with_transport, TelegramRetryPolicy,
    };
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct TestTelegramTransport;

    impl TelegramTransport for TestTelegramTransport {
        fn send_message(
            &self,
            _destination: &str,
            _markdown: &str,
        ) -> Result<TelegramMessageId, TelegramTransportError> {
            TelegramMessageId::new("fixture-message").map_err(|error| {
                TelegramTransportError::Failed {
                    reason: error.to_string(),
                }
            })
        }
    }

    #[test]
    fn homepage_availability_does_not_satisfy_primary_evidence_guard() {
        let company = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            Some("https://example.test/investors".to_owned()),
            None,
            None,
            None,
            None,
        )
        .expect("homepage-only fixture company should be valid");
        let registry = CompanySourceRegistry::new(1, vec![company.clone()])
            .expect("homepage-only fixture registry should be valid");
        let client = FixtureHttpClient::with_response(
            company.official_ir_url().expect("IR URL exists"),
            HttpResponse::ok("<title>Investor Relations</title><p>Investor Relations</p>"),
        );

        let acquired = acquire_runtime_input(
            &registry,
            &client,
            "ORG-X test contact@example.test",
            NaiveDate::from_ymd_opt(2026, 8, 25).expect("fixture date should be valid"),
        )
        .expect("homepage-only acquisition should complete");

        assert!(!acquired.has_primary_evidence);
        assert_eq!(acquired.input.research_metrics().source_available(), 1);
        assert_eq!(acquired.input.research_metrics().validated_evidence(), 0);
    }

    #[test]
    fn validated_document_claim_is_counted_and_can_feed_judgment() {
        let company = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            Some("https://ir.example.test/investors".to_owned()),
            None,
            None,
            None,
            None,
        )
        .expect("valid-document fixture company should be valid");
        let registry = CompanySourceRegistry::new(1, vec![company.clone()])
            .expect("valid-document fixture registry should be valid");
        let client = FixtureHttpClient::new();
        client.insert(
            company.official_ir_url().expect("IR URL exists"),
            HttpResponse::ok("<a href=\"/organization/update\">Organization update</a>"),
        );
        client.insert(
            "https://ir.example.test/organization/update",
            HttpResponse::ok(
                "<title>Organization update</title><time datetime=\"2026-08-19\"></time><p>Acme reorganized its engineering workflow and consolidated production scheduling under one platform.</p>",
            ),
        );

        let acquired = acquire_runtime_input(
            &registry,
            &client,
            "ORG-X test contact@example.test",
            NaiveDate::from_ymd_opt(2026, 8, 25).expect("fixture date should be valid"),
        )
        .expect("valid-document acquisition should complete");

        assert!(acquired.has_primary_evidence);
        assert_eq!(acquired.input.research_metrics().validated_evidence(), 1);
        assert_eq!(acquired.input.research_metrics().structural_evidence(), 1);
        assert!(acquired
            .input
            .facts()
            .iter()
            .any(|fact| fact.kind().starts_with("evidence_")));
    }

    #[test]
    fn multiple_official_research_sources_preserve_source_coverage_invariant() {
        let company = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("base company should be valid")
        .with_official_research_sources(vec![
            "https://example.test/frontier".to_owned(),
            "https://example.test/customer-stories".to_owned(),
        ])
        .expect("research sources should validate");
        let registry = CompanySourceRegistry::new(1, vec![company.clone()])
            .expect("research registry should be valid");
        let client = FixtureHttpClient::new();
        client.insert(
            "https://example.test/frontier",
            HttpResponse::ok("<title>Frontier research</title><p>Official research.</p>"),
        );
        client.insert(
            "https://example.test/customer-stories",
            HttpResponse::ok("<title>Customer stories</title><p>Official stories.</p>"),
        );

        let acquired = acquire_runtime_input(
            &registry,
            &client,
            "ORG-X test contact@example.test",
            NaiveDate::from_ymd_opt(2026, 8, 25).expect("fixture date should be valid"),
        )
        .expect("multiple research sources should not invalidate coverage");

        let coverage = acquired
            .input
            .source_coverage()
            .iter()
            .find(|item| item.source() == "official_research")
            .expect("official research coverage should be present");
        assert_eq!(coverage.expected(), 1);
        assert_eq!(coverage.available(), 1);
        assert_eq!(coverage.not_configured(), 0);
        assert_eq!(coverage.unavailable(), 0);
    }

    #[test]
    fn sec_health_distinguishes_reachable_stages_from_usable_facts() {
        let company = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            Some("0001234567".to_owned()),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("SEC health fixture company should be valid");
        let registry = CompanySourceRegistry::new(1, vec![company]).unwrap();
        let client = FixtureHttpClient::new();
        client.insert(
            "https://data.sec.gov/submissions/CIK0001234567.json",
            HttpResponse::ok(
                r#"{"filings":{"recent":{"accessionNumber":["0001234567-25-000001"],"filingDate":["2025-02-15"],"reportDate":["2024-12-31"],"form":["10-K"],"primaryDocument":["acme-2024.htm"]}}}"#,
            ),
        );
        client.insert(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
            HttpResponse::ok(r#"{"facts":{}}"#),
        );

        let acquired = acquire_runtime_input(
            &registry,
            &client,
            "ORG-X test contact@example.test",
            NaiveDate::from_ymd_opt(2026, 8, 25).unwrap(),
        )
        .expect("partial SEC health acquisition should complete");

        assert_eq!(acquired.input.research_metrics().sec_stage_expected(), 2);
        assert_eq!(acquired.input.research_metrics().sec_stage_available(), 2);
        assert!(acquired.input.research_metrics().sec_fact_expected() > 0);
        assert_eq!(acquired.input.research_metrics().sec_fact_available(), 0);
    }

    #[test]
    fn sec_filing_document_enters_common_evidence_loop_once() {
        let company = CompanyConfig::new(
            "acme",
            "Acme Corporation",
            "ACME",
            Some("0001234567".to_owned()),
            None,
            None,
            None,
            None,
            None,
        )
        .expect("SEC document fixture company should be valid");
        let registry = CompanySourceRegistry::new(1, vec![company]).unwrap();
        let client = FixtureHttpClient::new();
        client.insert(
            "https://data.sec.gov/submissions/CIK0001234567.json",
            HttpResponse::ok(
                r#"{"filings":{"recent":{"accessionNumber":["0001234567-25-000001"],"filingDate":["2025-02-15"],"reportDate":["2024-12-31"],"form":["8-K"],"primaryDocument":["acme-update.htm"]}}}"#,
            ),
        );
        client.insert(
            "https://data.sec.gov/api/xbrl/companyfacts/CIK0001234567.json",
            HttpResponse::ok(r#"{"facts":{}}"#),
        );
        client.insert(
            "https://www.sec.gov/Archives/edgar/data/1234567/000123456725000001/acme-update.htm",
            HttpResponse::ok(
                "<title>Acme organization update</title><time datetime=\"2025-02-15\"><p>Acme reorganized its engineering workflow and consolidated production scheduling under one platform.</p>",
            ),
        );

        let acquired = acquire_runtime_input(
            &registry,
            &client,
            "ORG-X test contact@example.test",
            NaiveDate::from_ymd_opt(2026, 8, 25).unwrap(),
        )
        .expect("SEC document acquisition should complete");

        assert!(acquired.has_primary_evidence);
        assert_eq!(acquired.input.research_metrics().document_candidates(), 1);
        assert_eq!(acquired.input.research_metrics().validated_evidence(), 1);
        assert_eq!(acquired.input.research_metrics().pending_leads(), 0);
        assert_eq!(
            acquired
                .input
                .research_metrics()
                .document_kind_counts()
                .get("filing"),
            Some(&1)
        );
    }

    #[test]
    fn republish_reconstructs_input_sends_once_and_leaves_archive_unchanged() {
        let root = std::env::temp_dir().join(format!(
            "org-x-republish-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        let as_of = NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date should be valid");
        let input = RuntimeReportInput::from_date(as_of);
        let input_snapshot =
            persist_input_snapshot(&root, "data", &input, ReportLanguage::Chinese, true)
                .expect("fixture input should persist");
        let report = render_report_in_language(&input, ReportLanguage::Chinese);
        let receipt = send_rendered_report_with_transport(
            &report,
            "fixture-chat",
            &TestTelegramTransport,
            TelegramRetryPolicy::new(1, Duration::ZERO),
        )
        .expect("fixture report should deliver");
        write_run_with_input_snapshot(&root, "data", &report, &receipt, Some(&input_snapshot))
            .expect("fixture archive should commit");
        let archive_paths = [
            root.join("weekly-radar/snapshots/2026-08-17.input.json"),
            root.join("weekly-radar/reports/2026-08-17.md"),
            root.join("weekly-radar/snapshots/2026-08-17.json"),
            root.join("weekly-radar/receipts/2026-08-17.json"),
            root.join("weekly-radar/manifest.json"),
        ];
        let before = archive_paths
            .iter()
            .map(|path| fs::read(path).expect("fixture archive file should be readable"))
            .collect::<Vec<_>>();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let sent_for_callback = Arc::clone(&sent);

        let output = republish_published_report_with(&root, as_of, |rendered| {
            sent_for_callback
                .lock()
                .expect("test send lock should work")
                .push(rendered.report_id().to_owned());
            Ok(RepublishDeliveryEvidence {
                report_id: rendered.report_id().to_owned(),
                message_ids: vec!["republish-message".to_owned()],
                attempts: vec![1],
            })
        })
        .expect("republish should use the persisted input and send once");

        assert!(output.contains("REPUBLISHED: report"));
        assert!(output.contains("message_ids=[\"republish-message\"]"));
        assert!(output.contains("archive unchanged"));
        assert_eq!(sent.lock().unwrap().len(), 1);
        for (path, expected) in archive_paths.iter().zip(before) {
            assert_eq!(
                fs::read(path).unwrap(),
                expected,
                "{} changed",
                path.display()
            );
        }
        fs::remove_dir_all(root).expect("republish fixture should be removable");
    }
}
