use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fmt;
use std::path::PathBuf;

use chrono::{NaiveDate, Utc};
use org_x::features::weekly_radar::runtime::config::CompanySourceRegistry;
use org_x::features::weekly_radar::runtime::http::{HttpClient, UreqHttpClient};
use org_x::features::weekly_radar::runtime::model::{
    FactStatus, RuntimeReportInput, SourceCoverage,
};
use org_x::features::weekly_radar::runtime::report::{render_report, RenderedReport};
use org_x::features::weekly_radar::runtime::sec::SecClient;
use org_x::features::weekly_radar::runtime::sources::{collect_configured_sources, SourceStatus};
use org_x::features::weekly_radar::runtime::{
    normalize_source_observation, send_rendered_report, write_run,
};

const DEFAULT_REGISTRY: &str = "config/weekly_radar/companies.json";
const DEFAULT_ARCHIVE_DIR: &str = ".";

#[derive(Clone, Debug, Eq, PartialEq)]
struct CliOptions {
    as_of: NaiveDate,
    archive_dir: PathBuf,
    registry: PathBuf,
    dry_run: bool,
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
    "Usage: org-x weekly-radar [--as-of YYYY-MM-DD] [--archive-dir PATH] [--registry PATH] [--dry-run]"
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
            }
            "--archive-dir" => {
                let value = option_value(args, &mut index, "--archive-dir")?;
                archive_dir = PathBuf::from(value);
            }
            "--registry" => {
                let value = option_value(args, &mut index, "--registry")?;
                registry = PathBuf::from(value);
            }
            unknown => {
                return Err(CliError::Usage(format!(
                    "unknown weekly-radar option {unknown}"
                )))
            }
        }
    }

    Ok(CliAction::Run(CliOptions {
        as_of: as_of.unwrap_or_else(|| Utc::now().date_naive()),
        archive_dir,
        registry,
        dry_run,
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
}

struct AcquiredRuntimeInput {
    input: RuntimeReportInput,
    has_primary_evidence: bool,
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
    let observed_at = Utc::now();

    for company in registry.companies() {
        let sec_coverage = coverage.entry("sec".to_owned()).or_default();
        sec_coverage.expected.insert(company.id().to_owned());
        if let Ok(evidence) = SecClient::collect(company, http, sec_user_agent) {
            sec_coverage.available.insert(company.id().to_owned());
            for fact in evidence.facts() {
                if fact.status() == &FactStatus::Known {
                    has_primary_evidence = true;
                }
                input
                    .add_fact(fact.clone())
                    .map_err(|error| CliError::Failure(error.to_string()))?;
            }
        }

        let observations = collect_configured_sources(company, http, observed_at);
        let mut available_kinds = BTreeSet::new();
        let mut source_indices = BTreeMap::<&str, usize>::new();
        for observation in &observations {
            let kind = observation.kind().as_str().to_owned();
            let source_coverage = coverage.entry(kind).or_default();
            source_coverage.expected.insert(company.id().to_owned());
            if observation.status() != SourceStatus::Unavailable {
                available_kinds.insert(observation.kind().as_str());
            }
            if observation.is_authoritative() && observation.status() == SourceStatus::Known {
                has_primary_evidence = true;
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
        }
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
                SourceCoverage::new(source, counts.expected.len(), counts.available.len())
                    .map_err(|error| CliError::Failure(error.to_string()))?,
            )
            .map_err(|error| CliError::Failure(error.to_string()))?;
    }

    Ok(AcquiredRuntimeInput {
        input,
        has_primary_evidence,
    })
}

fn validate_rendered_report(report: &RenderedReport) -> Result<(), CliError> {
    if report.report_id().trim().is_empty() || report.markdown().trim().is_empty() {
        return Err(CliError::Failure(
            "rendered report validation failed".to_owned(),
        ));
    }
    serde_json::from_str::<serde_json::Value>(report.snapshot_json())
        .map_err(|_| CliError::Failure("rendered report snapshot is not valid JSON".to_owned()))?;
    if !report.markdown().contains("## Executive Summary")
        || !report.markdown().contains("## System Health")
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
    })
}

fn run_weekly_radar(options: CliOptions) -> Result<String, CliError> {
    let registry = CompanySourceRegistry::from_path(&options.registry)
        .map_err(|error| CliError::Failure(error.to_string()))?;
    let user_agent = sec_user_agent()?;
    if !options.dry_run && !registry_has_configured_primary_source(&registry) {
        return Err(CliError::Failure(
            "cannot publish weekly radar without primary evidence".to_owned(),
        ));
    }
    let production = UreqHttpClient::new();
    let http: &dyn HttpClient = &production;

    let acquired = acquire_runtime_input(&registry, http, &user_agent, options.as_of)?;
    let report = render_report(&acquired.input);
    validate_rendered_report(&report)?;

    if options.dry_run {
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

    let receipt =
        send_rendered_report(&report).map_err(|error| CliError::Failure(error.to_string()))?;
    let manifest = write_run(&options.archive_dir, "data", &report, &receipt)
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
