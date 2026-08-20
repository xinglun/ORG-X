//! Deterministic Weekly Radar archive files and bounded retention.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use super::model::RuntimeReportInput;
use super::report::{RenderedReport, ReportLanguage};
use super::telegram::TelegramDeliveryReceipt;

const ARCHIVE_DIRECTORY: &str = "weekly-radar";
const REPORTS_DIRECTORY: &str = "reports";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const RECEIPTS_DIRECTORY: &str = "receipts";
const DEFAULT_RETENTION_DAYS: i64 = 365;
const INPUT_SNAPSHOT_SUFFIX: &str = ".input.json";

/// Schema version for the persisted pre-render input envelope.
pub const INPUT_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Archive failures that do not retain secret-bearing filesystem details.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArchiveError {
    /// A filesystem operation failed.
    Io { operation: &'static str },
    /// The requested retention window was negative.
    InvalidRetention,
    /// Guarded writes are permitted only for the dedicated data branch.
    NonDataBranch { branch: String },
    /// A report date could not produce a valid archive name.
    InvalidDate,
    /// A successful Telegram receipt belongs to another rendered report.
    ReportIdMismatch { expected: String, actual: String },
    /// A report cannot be archived without at least one successful message ID
    /// and one corresponding delivery attempt.
    InvalidDeliveryReceipt,
    /// A final report already exists for the requested date.
    ExistingRun { as_of: NaiveDate },
    /// A different pre-render input already exists for the requested date.
    InputSnapshotConflict { as_of: NaiveDate },
    /// The pre-render input for a retry date does not exist.
    MissingInputSnapshot { as_of: NaiveDate },
    /// A persisted input envelope failed schema, date, language, or identity validation.
    InvalidInputSnapshot { reason: &'static str },
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { operation } => write!(formatter, "archive {operation} failed"),
            Self::InvalidRetention => formatter.write_str("archive retention must be non-negative"),
            Self::NonDataBranch { branch } => {
                write!(
                    formatter,
                    "archive writes are guarded to the data branch, not {branch}"
                )
            }
            Self::InvalidDate => formatter.write_str("archive report date is invalid"),
            Self::ReportIdMismatch { expected, actual } => write!(
                formatter,
                "archive report identity mismatch: expected {expected}, received {actual}"
            ),
            Self::InvalidDeliveryReceipt => {
                formatter.write_str("archive requires a successful Telegram delivery receipt")
            }
            Self::ExistingRun { as_of } => {
                write!(
                    formatter,
                    "archive already contains a final run for {as_of}"
                )
            }
            Self::InputSnapshotConflict { as_of } => {
                write!(formatter, "archive input snapshot conflicts for {as_of}")
            }
            Self::MissingInputSnapshot { as_of } => {
                write!(formatter, "archive input snapshot is missing for {as_of}")
            }
            Self::InvalidInputSnapshot { reason } => {
                write!(formatter, "archive input snapshot is invalid: {reason}")
            }
        }
    }
}

impl std::error::Error for ArchiveError {}

/// Versioned runtime input retained before report rendering and delivery.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputSnapshot {
    schema_version: u32,
    as_of: NaiveDate,
    language: String,
    has_primary_evidence: bool,
    snapshot_id: String,
    input: RuntimeReportInput,
}

impl InputSnapshot {
    /// Returns the persisted input schema version.
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Returns the input as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.as_of
    }

    /// Returns the saved report language.
    pub fn language(&self) -> ReportLanguage {
        ReportLanguage::from_str(&self.language)
            .expect("InputSnapshot language is validated before construction")
    }

    /// Returns whether the input passed the primary-evidence publication guard.
    pub const fn has_primary_evidence(&self) -> bool {
        self.has_primary_evidence
    }

    /// Returns the deterministic input identity.
    pub fn snapshot_id(&self) -> &str {
        &self.snapshot_id
    }

    /// Returns the exact runtime input used for rendering.
    pub const fn input(&self) -> &RuntimeReportInput {
        &self.input
    }
}

/// Stable archive manifest written after each report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArchiveManifest {
    as_of: NaiveDate,
    report: String,
    snapshot: String,
    receipt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    snapshot_id: Option<String>,
}

impl ArchiveManifest {
    /// Returns the archive as-of date.
    pub const fn as_of(&self) -> NaiveDate {
        self.as_of
    }

    /// Returns the report path relative to the archive root.
    pub fn report(&self) -> &str {
        &self.report
    }

    /// Returns the snapshot path relative to the archive root.
    pub fn snapshot(&self) -> &str {
        &self.snapshot
    }

    /// Returns the receipt path relative to the archive root.
    pub fn receipt(&self) -> &str {
        &self.receipt
    }

    /// Returns the optional pre-render input snapshot path.
    pub fn input_snapshot(&self) -> Option<&str> {
        self.input_snapshot.as_deref()
    }

    /// Returns the optional deterministic pre-render input identity.
    pub fn snapshot_id(&self) -> Option<&str> {
        self.snapshot_id.as_deref()
    }
}

fn archive_root(root: &Path) -> PathBuf {
    root.join(ARCHIVE_DIRECTORY)
}

fn date_prefix(path: &Path) -> Option<NaiveDate> {
    let name = path.file_name()?.to_str()?;
    if name.len() < 10 {
        return None;
    }
    NaiveDate::parse_from_str(&name[..10], "%Y-%m-%d").ok()
}

fn ensure_archive_directories(root: &Path) -> Result<PathBuf, ArchiveError> {
    let archive = archive_root(root);
    for directory in [REPORTS_DIRECTORY, SNAPSHOTS_DIRECTORY, RECEIPTS_DIRECTORY] {
        fs::create_dir_all(archive.join(directory)).map_err(|_| ArchiveError::Io {
            operation: "directory creation",
        })?;
    }
    Ok(archive)
}

fn validate_data_branch(branch: &str) -> Result<(), ArchiveError> {
    if branch == "data" {
        Ok(())
    } else {
        Err(ArchiveError::NonDataBranch {
            branch: branch.to_owned(),
        })
    }
}

fn input_snapshot_path(root: &Path, as_of: NaiveDate) -> PathBuf {
    archive_root(root).join(SNAPSHOTS_DIRECTORY).join(format!(
        "{}{INPUT_SNAPSHOT_SUFFIX}",
        as_of.format("%Y-%m-%d")
    ))
}

fn input_snapshot_relative(as_of: NaiveDate) -> String {
    format!(
        "{ARCHIVE_DIRECTORY}/{SNAPSHOTS_DIRECTORY}/{}{INPUT_SNAPSHOT_SUFFIX}",
        as_of.format("%Y-%m-%d")
    )
}

fn digest_bytes(bytes: impl IntoIterator<Item = u8>) -> String {
    let mut digest = 14_695_981_039_346_656_037_u64;
    for byte in bytes {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(1_099_511_628_211);
    }
    format!("wr-input-{digest:016x}")
}

fn input_snapshot_id(input: &RuntimeReportInput) -> String {
    let bytes = serde_json::to_vec(input).expect("runtime input contains only serializable values");
    digest_bytes(bytes)
}

fn validate_input_snapshot(snapshot: &InputSnapshot) -> Result<(), ArchiveError> {
    if snapshot.schema_version != INPUT_SNAPSHOT_SCHEMA_VERSION {
        return Err(ArchiveError::InvalidInputSnapshot {
            reason: "unsupported schema version",
        });
    }
    if snapshot.as_of != snapshot.input.as_of() {
        return Err(ArchiveError::InvalidInputSnapshot {
            reason: "envelope and input dates differ",
        });
    }
    if ReportLanguage::from_str(&snapshot.language).is_err() {
        return Err(ArchiveError::InvalidInputSnapshot {
            reason: "unsupported language",
        });
    }
    if snapshot.snapshot_id != input_snapshot_id(&snapshot.input) {
        return Err(ArchiveError::InvalidInputSnapshot {
            reason: "input identity does not match content",
        });
    }
    Ok(())
}

fn temporary_path(path: &Path) -> Result<PathBuf, ArchiveError> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(ArchiveError::InvalidDate)?;
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(
        ".{file_name}.tmp-{}-{sequence}",
        std::process::id()
    )))
}

fn write_atomic(path: &Path, content: &str) -> Result<(), ArchiveError> {
    let temporary = temporary_path(path)?;
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
    {
        Ok(file) => file,
        Err(_) => {
            return Err(ArchiveError::Io {
                operation: "temporary file creation",
            })
        }
    };
    if file.write_all(content.as_bytes()).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ArchiveError::Io {
            operation: "file write",
        });
    }
    if file.sync_all().is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ArchiveError::Io {
            operation: "file sync",
        });
    }
    drop(file);
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(ArchiveError::Io {
            operation: "file commit",
        });
    }
    Ok(())
}

/// Persists the exact pre-render runtime input on the guarded data branch.
pub fn persist_input_snapshot(
    root: &Path,
    branch: &str,
    input: &RuntimeReportInput,
    language: ReportLanguage,
    has_primary_evidence: bool,
) -> Result<InputSnapshot, ArchiveError> {
    validate_data_branch(branch)?;
    let snapshot = InputSnapshot {
        schema_version: INPUT_SNAPSHOT_SCHEMA_VERSION,
        as_of: input.as_of(),
        language: language.as_str().to_owned(),
        has_primary_evidence,
        snapshot_id: input_snapshot_id(input),
        input: input.clone(),
    };
    validate_input_snapshot(&snapshot)?;
    let content = serde_json::to_string_pretty(&snapshot).map_err(|_| {
        ArchiveError::InvalidInputSnapshot {
            reason: "serialization failed",
        }
    })? + "\n";
    let path = input_snapshot_path(root, snapshot.as_of);
    if path.exists() {
        let existing = fs::read(&path).map_err(|_| ArchiveError::Io {
            operation: "input snapshot read",
        })?;
        if existing == content.as_bytes() {
            return Ok(snapshot);
        }
        return Err(ArchiveError::InputSnapshotConflict {
            as_of: snapshot.as_of,
        });
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ArchiveError::Io {
            operation: "input snapshot directory creation",
        })?;
    }
    write_atomic(&path, &content)?;
    Ok(snapshot)
}

/// Loads and validates a previously persisted pre-render input for retry.
pub fn load_input_snapshot(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<InputSnapshot, ArchiveError> {
    validate_data_branch(branch)?;
    let path = input_snapshot_path(root, as_of);
    let content = fs::read_to_string(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ArchiveError::MissingInputSnapshot { as_of }
        } else {
            ArchiveError::Io {
                operation: "input snapshot read",
            }
        }
    })?;
    let snapshot = serde_json::from_str::<InputSnapshot>(&content).map_err(|_| {
        ArchiveError::InvalidInputSnapshot {
            reason: "JSON decoding failed",
        }
    })?;
    validate_input_snapshot(&snapshot)?;
    if snapshot.as_of != as_of {
        return Err(ArchiveError::InvalidInputSnapshot {
            reason: "requested date does not match envelope date",
        });
    }
    Ok(snapshot)
}

/// Checks that no final report, rendered snapshot, or receipt exists for a date.
pub fn ensure_run_available(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<(), ArchiveError> {
    validate_data_branch(branch)?;
    let date_text = as_of.format("%Y-%m-%d").to_string();
    let archive = archive_root(root);
    let final_paths = [
        archive
            .join(REPORTS_DIRECTORY)
            .join(format!("{date_text}.md")),
        archive
            .join(SNAPSHOTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        archive
            .join(RECEIPTS_DIRECTORY)
            .join(format!("{date_text}.json")),
    ];
    if final_paths.iter().any(|path| path.exists()) {
        return Err(ArchiveError::ExistingRun { as_of });
    }
    Ok(())
}

/// Removes only date-prefixed files older than the requested retention window.
pub fn retain_recent(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
    retention_days: i64,
) -> Result<usize, ArchiveError> {
    validate_data_branch(branch)?;
    if retention_days < 0 {
        return Err(ArchiveError::InvalidRetention);
    }
    let archive = archive_root(root);
    let cutoff = as_of - Duration::days(retention_days);
    let mut removed = 0;
    for directory in [REPORTS_DIRECTORY, SNAPSHOTS_DIRECTORY, RECEIPTS_DIRECTORY] {
        let path = archive.join(directory);
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                return Err(ArchiveError::Io {
                    operation: "retention scan",
                })
            }
        };
        for entry in entries {
            let entry = entry.map_err(|_| ArchiveError::Io {
                operation: "retention scan",
            })?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if date_prefix(&path).is_some_and(|date| date < cutoff) {
                fs::remove_file(&path).map_err(|_| ArchiveError::Io {
                    operation: "retention removal",
                })?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

/// Writes a final archive without an input-envelope reference.
pub fn write_run(
    root: &Path,
    branch: &str,
    rendered_report: &RenderedReport,
    delivery_receipt: &TelegramDeliveryReceipt,
) -> Result<ArchiveManifest, ArchiveError> {
    write_run_with_input_snapshot(root, branch, rendered_report, delivery_receipt, None)
}

/// Writes the final report, snapshot, receipt, and manifest atomically.
pub fn write_run_with_input_snapshot(
    root: &Path,
    branch: &str,
    rendered_report: &RenderedReport,
    delivery_receipt: &TelegramDeliveryReceipt,
    input_snapshot: Option<&InputSnapshot>,
) -> Result<ArchiveManifest, ArchiveError> {
    validate_data_branch(branch)?;
    if delivery_receipt.report_id() != rendered_report.report_id() {
        return Err(ArchiveError::ReportIdMismatch {
            expected: rendered_report.report_id().to_owned(),
            actual: delivery_receipt.report_id().to_owned(),
        });
    }
    if delivery_receipt.message_ids().is_empty()
        || delivery_receipt.message_ids().len() != delivery_receipt.attempts().len()
    {
        return Err(ArchiveError::InvalidDeliveryReceipt);
    }
    let date = rendered_report.as_of();
    if let Some(input_snapshot) = input_snapshot {
        validate_input_snapshot(input_snapshot)?;
        if input_snapshot.as_of != date {
            return Err(ArchiveError::InvalidInputSnapshot {
                reason: "input snapshot date does not match report date",
            });
        }
    }
    ensure_run_available(root, branch, date)?;
    let archive = ensure_archive_directories(root)?;
    let date_text = date.format("%Y-%m-%d").to_string();
    let report_relative = format!("{ARCHIVE_DIRECTORY}/{REPORTS_DIRECTORY}/{date_text}.md");
    let snapshot_relative = format!("{ARCHIVE_DIRECTORY}/{SNAPSHOTS_DIRECTORY}/{date_text}.json");
    let receipt_relative = format!("{ARCHIVE_DIRECTORY}/{RECEIPTS_DIRECTORY}/{date_text}.json");
    let manifest = ArchiveManifest {
        as_of: date,
        report: report_relative,
        snapshot: snapshot_relative,
        receipt: receipt_relative,
        input_snapshot: input_snapshot.map(|_| input_snapshot_relative(date)),
        snapshot_id: input_snapshot.map(|snapshot| snapshot.snapshot_id().to_owned()),
    };
    write_atomic(
        &archive
            .join(REPORTS_DIRECTORY)
            .join(format!("{date_text}.md")),
        rendered_report.markdown(),
    )?;
    write_atomic(
        &archive
            .join(SNAPSHOTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        rendered_report.snapshot_json(),
    )?;
    #[derive(Serialize)]
    struct ArchiveReceipt {
        as_of: NaiveDate,
        report_id: String,
        status: &'static str,
        message_ids: Vec<String>,
        attempts: Vec<u32>,
    }
    let receipt = serde_json::to_string_pretty(&ArchiveReceipt {
        as_of: date,
        report_id: rendered_report.report_id().to_owned(),
        status: "PUBLISHED",
        message_ids: delivery_receipt
            .message_ids()
            .iter()
            .map(|message_id| message_id.as_str().to_owned())
            .collect(),
        attempts: delivery_receipt.attempts().to_vec(),
    })
    .map_err(|_| ArchiveError::InvalidDeliveryReceipt)?
        + "\n";
    write_atomic(
        &archive
            .join(RECEIPTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        &receipt,
    )?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|_| ArchiveError::InvalidDate)? + "\n";
    write_atomic(&archive.join("manifest.json"), &manifest_json)?;
    retain_recent(root, branch, date, DEFAULT_RETENTION_DAYS)?;
    Ok(manifest)
}
