//! Deterministic Weekly Radar archive files and bounded retention.

use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::fd::AsRawFd;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use super::model::RuntimeReportInput;
use super::report::{RenderedReport, ReportLanguage};
use super::telegram::TelegramDeliveryReceipt;

const ARCHIVE_DIRECTORY: &str = "weekly-radar";
const REPORTS_DIRECTORY: &str = "reports";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const RECEIPTS_DIRECTORY: &str = "receipts";
const TRANSACTIONS_DIRECTORY: &str = ".transactions";
const DEFAULT_RETENTION_DAYS: i64 = 365;
const INPUT_SNAPSHOT_SUFFIX: &str = ".input.json";
const TRANSACTION_RECORD_SCHEMA_VERSION: u32 = 1;
const RUN_LOCK_SUFFIX: &str = ".run.lock";
const COMMIT_LOCK_SUFFIX: &str = ".commit.lock";

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
    /// The archive contains an incomplete or unverifiable transaction.
    IncompleteRun { as_of: NaiveDate },
    /// Another process is already handling the requested report date.
    ConcurrentRun { as_of: NaiveDate },
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
            Self::IncompleteRun { as_of } => {
                write!(formatter, "archive contains an incomplete run for {as_of}")
            }
            Self::ConcurrentRun { as_of } => {
                write!(formatter, "archive run is already in progress for {as_of}")
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArchiveTransactionState {
    Prepared,
    Committed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ArchiveTransactionArtifact {
    final_path: String,
    staged_path: String,
    digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ArchiveTransactionRecord {
    schema_version: u32,
    as_of: NaiveDate,
    transaction_id: String,
    state: ArchiveTransactionState,
    staging_directory: String,
    artifacts: Vec<ArchiveTransactionArtifact>,
    manifest: ArchiveManifest,
}

/// Holds the per-date lock used by the CLI across recovery, delivery, and archive commit.
///
/// On Unix, the lock is advisory and is released automatically when this value is dropped,
/// including when the process terminates unexpectedly. The lock file itself is metadata under
/// `weekly-radar/.transactions/` and is not part of the public archive.
pub struct ArchiveRunLock {
    file: File,
}

impl Drop for ArchiveRunLock {
    fn drop(&mut self) {
        unlock_file(&self.file);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArchiveCommitStage {
    Prepared,
    Report,
    Snapshot,
    Receipt,
    Manifest,
}

#[cfg(test)]
impl ArchiveCommitStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Prepared => "prepared",
            Self::Report => "report",
            Self::Snapshot => "snapshot",
            Self::Receipt => "receipt",
            Self::Manifest => "manifest",
        }
    }
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

fn lock_file(file: &File, as_of: NaiveDate) -> Result<(), ArchiveError> {
    #[cfg(unix)]
    {
        const LOCK_EXCLUSIVE: i32 = 2;
        const LOCK_NONBLOCKING: i32 = 4;
        unsafe extern "C" {
            fn flock(file_descriptor: i32, operation: i32) -> i32;
        }
        let result = unsafe { flock(file.as_raw_fd(), LOCK_EXCLUSIVE | LOCK_NONBLOCKING) };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if matches!(error.raw_os_error(), Some(11) | Some(35)) {
            return Err(ArchiveError::ConcurrentRun { as_of });
        }
        Err(ArchiveError::Io {
            operation: "archive lock",
        })
    }

    #[cfg(not(unix))]
    {
        let _ = (file, as_of);
        Ok(())
    }
}

fn unlock_file(file: &File) {
    #[cfg(unix)]
    {
        const UNLOCK: i32 = 8;
        unsafe extern "C" {
            fn flock(file_descriptor: i32, operation: i32) -> i32;
        }
        let _ = unsafe { flock(file.as_raw_fd(), UNLOCK) };
    }

    #[cfg(not(unix))]
    {
        let _ = file;
    }
}

fn acquire_lock(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
    suffix: &str,
) -> Result<ArchiveRunLock, ArchiveError> {
    validate_data_branch(branch)?;
    let archive = ensure_archive_directories(root)?;
    let path = archive.join(TRANSACTIONS_DIRECTORY).join(format!(
        "{}{}",
        as_of.format("%Y-%m-%d"),
        suffix
    ));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|_| ArchiveError::Io {
            operation: "archive lock creation",
        })?;
    lock_file(&file, as_of)?;
    Ok(ArchiveRunLock { file })
}

/// Acquires the per-date execution lock used to prevent a second delivery while a run is active.
pub fn acquire_run_lock(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<ArchiveRunLock, ArchiveError> {
    acquire_lock(root, branch, as_of, RUN_LOCK_SUFFIX)
}

fn acquire_commit_lock(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<ArchiveRunLock, ArchiveError> {
    acquire_lock(root, branch, as_of, COMMIT_LOCK_SUFFIX)
}

fn transaction_record_path(root: &Path, as_of: NaiveDate) -> PathBuf {
    archive_root(root)
        .join(TRANSACTIONS_DIRECTORY)
        .join(format!("{}.json", as_of.format("%Y-%m-%d")))
}

fn transaction_id(as_of: NaiveDate) -> String {
    let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!(
        "{}-{}-{timestamp}-{sequence}",
        as_of.format("%Y-%m-%d"),
        std::process::id()
    )
}

fn archive_relative_path(path: &str) -> Option<PathBuf> {
    let relative = Path::new(path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(relative.to_owned())
}

fn archive_join_relative(archive: &Path, relative: &str) -> Result<PathBuf, ArchiveError> {
    let relative = archive_relative_path(relative).ok_or(ArchiveError::InvalidDate)?;
    Ok(archive.join(relative))
}

fn final_relative_paths(as_of: NaiveDate) -> [String; 4] {
    let date_text = as_of.format("%Y-%m-%d");
    [
        format!("{REPORTS_DIRECTORY}/{date_text}.md"),
        format!("{SNAPSHOTS_DIRECTORY}/{date_text}.json"),
        format!("{RECEIPTS_DIRECTORY}/{date_text}.json"),
        "manifest.json".to_owned(),
    ]
}

fn final_paths(root: &Path, as_of: NaiveDate) -> [PathBuf; 3] {
    let archive = archive_root(root);
    let date_text = as_of.format("%Y-%m-%d");
    [
        archive
            .join(REPORTS_DIRECTORY)
            .join(format!("{date_text}.md")),
        archive
            .join(SNAPSHOTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        archive
            .join(RECEIPTS_DIRECTORY)
            .join(format!("{date_text}.json")),
    ]
}

fn transaction_artifacts(
    as_of: NaiveDate,
    staging_directory: &str,
    report: &str,
    snapshot: &str,
    receipt: &str,
    manifest: &str,
) -> Vec<ArchiveTransactionArtifact> {
    let final_paths = final_relative_paths(as_of);
    let staged_names = [
        "report.md",
        "snapshot.json",
        "receipt.json",
        "manifest.json",
    ];
    [report, snapshot, receipt, manifest]
        .into_iter()
        .zip(final_paths)
        .zip(staged_names)
        .map(
            |((content, final_path), staged_name)| ArchiveTransactionArtifact {
                final_path,
                staged_path: format!("{staging_directory}/{staged_name}"),
                digest: digest_bytes(content.as_bytes().iter().copied()),
            },
        )
        .collect()
}

fn validate_transaction_record(
    record: &ArchiveTransactionRecord,
    as_of: NaiveDate,
) -> Result<(), ArchiveError> {
    let expected_paths = final_relative_paths(as_of);
    let expected_manifest = ArchiveManifest {
        as_of,
        report: format!("{ARCHIVE_DIRECTORY}/{}", expected_paths[0]),
        snapshot: format!("{ARCHIVE_DIRECTORY}/{}", expected_paths[1]),
        receipt: format!("{ARCHIVE_DIRECTORY}/{}", expected_paths[2]),
        input_snapshot: record
            .manifest
            .input_snapshot
            .as_ref()
            .map(|_| input_snapshot_relative(as_of)),
        snapshot_id: record.manifest.snapshot_id.clone(),
    };
    let transaction_id_is_single_component = !record.transaction_id.is_empty()
        && Path::new(&record.transaction_id)
            .components()
            .all(|component| matches!(component, Component::Normal(_)));
    if record.schema_version != TRANSACTION_RECORD_SCHEMA_VERSION
        || record.as_of != as_of
        || record.manifest != expected_manifest
        || record.manifest.input_snapshot.is_some() != record.manifest.snapshot_id.is_some()
        || record
            .manifest
            .snapshot_id
            .as_deref()
            .is_some_and(str::is_empty)
        || !transaction_id_is_single_component
        || record.staging_directory != format!("{TRANSACTIONS_DIRECTORY}/{}", record.transaction_id)
        || archive_relative_path(&record.staging_directory).is_none()
        || record.artifacts.len() != 4
    {
        return Err(ArchiveError::IncompleteRun { as_of });
    }
    for expected_path in expected_paths {
        let Some(artifact) = record
            .artifacts
            .iter()
            .find(|artifact| artifact.final_path == expected_path)
        else {
            return Err(ArchiveError::IncompleteRun { as_of });
        };
        if artifact.digest.is_empty()
            || archive_relative_path(&artifact.final_path).is_none()
            || archive_relative_path(&artifact.staged_path).is_none()
            || !artifact
                .staged_path
                .starts_with(&format!("{}/", record.staging_directory))
        {
            return Err(ArchiveError::IncompleteRun { as_of });
        }
    }
    Ok(())
}

fn read_transaction_record(
    root: &Path,
    as_of: NaiveDate,
) -> Result<Option<ArchiveTransactionRecord>, ArchiveError> {
    let path = transaction_record_path(root, as_of);
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(ArchiveError::IncompleteRun { as_of }),
    };
    let record = serde_json::from_str::<ArchiveTransactionRecord>(&content)
        .map_err(|_| ArchiveError::IncompleteRun { as_of })?;
    validate_transaction_record(&record, as_of)?;
    Ok(Some(record))
}

fn write_transaction_record(
    root: &Path,
    record: &ArchiveTransactionRecord,
) -> Result<(), ArchiveError> {
    let content =
        serde_json::to_string_pretty(record).map_err(|_| ArchiveError::InvalidDate)? + "\n";
    write_atomic(&transaction_record_path(root, record.as_of), &content)
}

fn maybe_injected_failure(
    requested: Option<ArchiveCommitStage>,
    stage: ArchiveCommitStage,
) -> Result<(), ArchiveError> {
    if requested == Some(stage) {
        return Err(ArchiveError::Io {
            operation: "injected transaction failure",
        });
    }
    Ok(())
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
    for directory in [
        REPORTS_DIRECTORY,
        SNAPSHOTS_DIRECTORY,
        RECEIPTS_DIRECTORY,
        TRANSACTIONS_DIRECTORY,
    ] {
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
    snapshot
        .input
        .validate()
        .map_err(|_| ArchiveError::InvalidInputSnapshot {
            reason: "runtime input validation failed",
        })?;
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

fn transaction_artifact<'a>(
    record: &'a ArchiveTransactionRecord,
    final_path: &str,
) -> Result<&'a ArchiveTransactionArtifact, ArchiveError> {
    record
        .artifacts
        .iter()
        .find(|artifact| artifact.final_path == final_path)
        .ok_or(ArchiveError::IncompleteRun {
            as_of: record.as_of,
        })
}

fn read_staged_artifact(
    root: &Path,
    record: &ArchiveTransactionRecord,
    artifact: &ArchiveTransactionArtifact,
) -> Result<String, ArchiveError> {
    let archive = archive_root(root);
    let path = archive_join_relative(&archive, &artifact.staged_path).map_err(|_| {
        ArchiveError::IncompleteRun {
            as_of: record.as_of,
        }
    })?;
    let bytes = fs::read(path).map_err(|_| ArchiveError::IncompleteRun {
        as_of: record.as_of,
    })?;
    if digest_bytes(bytes.iter().copied()) != artifact.digest {
        return Err(ArchiveError::IncompleteRun {
            as_of: record.as_of,
        });
    }
    String::from_utf8(bytes).map_err(|_| ArchiveError::IncompleteRun {
        as_of: record.as_of,
    })
}

fn verify_committed_transaction(
    root: &Path,
    record: &ArchiveTransactionRecord,
) -> Result<(), ArchiveError> {
    let archive = archive_root(root);
    for artifact in &record.artifacts {
        if artifact.final_path == "manifest.json" {
            continue;
        }
        let path = archive_join_relative(&archive, &artifact.final_path).map_err(|_| {
            ArchiveError::IncompleteRun {
                as_of: record.as_of,
            }
        })?;
        let bytes = fs::read(path).map_err(|_| ArchiveError::IncompleteRun {
            as_of: record.as_of,
        })?;
        if digest_bytes(bytes.iter().copied()) != artifact.digest {
            return Err(ArchiveError::IncompleteRun {
                as_of: record.as_of,
            });
        }
    }
    let manifest_path = archive.join("manifest.json");
    let manifest_content =
        fs::read_to_string(manifest_path).map_err(|_| ArchiveError::IncompleteRun {
            as_of: record.as_of,
        })?;
    let current_manifest =
        serde_json::from_str::<ArchiveManifest>(&manifest_content).map_err(|_| {
            ArchiveError::IncompleteRun {
                as_of: record.as_of,
            }
        })?;
    if current_manifest.as_of < record.as_of {
        return Err(ArchiveError::IncompleteRun {
            as_of: record.as_of,
        });
    }
    Ok(())
}

fn compatibility_manifest_allows_update(root: &Path, as_of: NaiveDate) -> Result<(), ArchiveError> {
    let path = archive_root(root).join("manifest.json");
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(ArchiveError::IncompleteRun { as_of }),
    };
    let existing = serde_json::from_str::<ArchiveManifest>(&content)
        .map_err(|_| ArchiveError::IncompleteRun { as_of })?;
    if existing.as_of > as_of {
        return Err(ArchiveError::IncompleteRun { as_of });
    }
    Ok(())
}

fn promote_staged_artifact(
    root: &Path,
    record: &ArchiveTransactionRecord,
    artifact: &ArchiveTransactionArtifact,
    content: &str,
    allow_existing: bool,
) -> Result<(), ArchiveError> {
    let archive = archive_root(root);
    let final_path = archive_join_relative(&archive, &artifact.final_path).map_err(|_| {
        ArchiveError::IncompleteRun {
            as_of: record.as_of,
        }
    })?;
    if final_path.exists() {
        let existing = fs::read(&final_path).map_err(|_| ArchiveError::IncompleteRun {
            as_of: record.as_of,
        })?;
        if artifact.final_path == "manifest.json" && allow_existing {
            write_atomic(&final_path, content)?;
        } else if existing != content.as_bytes() || !allow_existing {
            return Err(ArchiveError::IncompleteRun {
                as_of: record.as_of,
            });
        }
    } else {
        write_atomic(&final_path, content)?;
    }
    let final_bytes = fs::read(&final_path).map_err(|_| ArchiveError::IncompleteRun {
        as_of: record.as_of,
    })?;
    if digest_bytes(final_bytes.iter().copied()) != artifact.digest {
        return Err(ArchiveError::IncompleteRun {
            as_of: record.as_of,
        });
    }
    Ok(())
}

fn validate_existing_public_artifacts(
    root: &Path,
    record: &ArchiveTransactionRecord,
    artifacts: &[(ArchiveTransactionArtifact, String)],
) -> Result<(), ArchiveError> {
    let archive = archive_root(root);
    for (artifact, content) in artifacts {
        if artifact.final_path == "manifest.json" {
            continue;
        }
        let final_path = archive_join_relative(&archive, &artifact.final_path).map_err(|_| {
            ArchiveError::IncompleteRun {
                as_of: record.as_of,
            }
        })?;
        if final_path.exists() {
            let existing = fs::read(final_path).map_err(|_| ArchiveError::IncompleteRun {
                as_of: record.as_of,
            })?;
            if existing != content.as_bytes() {
                return Err(ArchiveError::IncompleteRun {
                    as_of: record.as_of,
                });
            }
        }
    }
    Ok(())
}

fn complete_prepared_transaction(
    root: &Path,
    branch: &str,
    mut record: ArchiveTransactionRecord,
) -> Result<ArchiveManifest, ArchiveError> {
    validate_data_branch(branch)?;
    validate_transaction_record(&record, record.as_of)?;
    let _ = ensure_archive_directories(root)?;
    let date = record.as_of;
    compatibility_manifest_allows_update(root, date)?;
    let final_paths = [
        format!("{REPORTS_DIRECTORY}/{}.md", date.format("%Y-%m-%d")),
        format!("{SNAPSHOTS_DIRECTORY}/{}.json", date.format("%Y-%m-%d")),
        format!("{RECEIPTS_DIRECTORY}/{}.json", date.format("%Y-%m-%d")),
        "manifest.json".to_owned(),
    ];
    let staged_artifacts = final_paths
        .iter()
        .map(|final_path| {
            let artifact = transaction_artifact(&record, final_path)?.clone();
            let content = read_staged_artifact(root, &record, &artifact)?;
            Ok((artifact, content))
        })
        .collect::<Result<Vec<_>, ArchiveError>>()?;
    validate_existing_public_artifacts(root, &record, &staged_artifacts)?;
    for (artifact, content) in staged_artifacts {
        promote_staged_artifact(root, &record, &artifact, &content, true)?;
    }

    record.state = ArchiveTransactionState::Committed;
    write_transaction_record(root, &record)?;
    let _ = fs::remove_dir_all(archive_root(root).join(&record.staging_directory));
    retain_recent(root, branch, date, DEFAULT_RETENTION_DAYS)?;
    Ok(record.manifest)
}

/// Recovers a prepared archive transaction without sending Telegram again.
///
/// `Some` is returned only when a prepared transaction was completed. `None`
/// means that no prepared transaction exists; a valid committed or legacy run
/// remains subject to the normal [`ensure_run_available`] duplicate guard.
pub fn recover_pending_run(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<Option<ArchiveManifest>, ArchiveError> {
    let _lock = acquire_commit_lock(root, branch, as_of)?;
    recover_pending_run_unlocked(root, branch, as_of)
}

fn recover_pending_run_unlocked(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<Option<ArchiveManifest>, ArchiveError> {
    validate_data_branch(branch)?;
    let Some(record) = read_transaction_record(root, as_of)? else {
        let final_paths = final_paths(root, as_of);
        let present_count = final_paths.iter().filter(|path| path.exists()).count();
        if present_count > 0 && present_count < final_paths.len() {
            return Err(ArchiveError::IncompleteRun { as_of });
        }
        return Ok(None);
    };
    match record.state {
        ArchiveTransactionState::Prepared => {
            complete_prepared_transaction(root, branch, record).map(Some)
        }
        ArchiveTransactionState::Committed => {
            verify_committed_transaction(root, &record)?;
            Ok(None)
        }
    }
}

/// Checks that a date has no committed or incomplete final archive state.
pub fn ensure_run_available(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<(), ArchiveError> {
    let _lock = acquire_commit_lock(root, branch, as_of)?;
    ensure_run_available_unlocked(root, branch, as_of)
}

fn ensure_run_available_unlocked(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<(), ArchiveError> {
    validate_data_branch(branch)?;
    if let Some(record) = read_transaction_record(root, as_of)? {
        match record.state {
            ArchiveTransactionState::Prepared => {
                return Err(ArchiveError::IncompleteRun { as_of });
            }
            ArchiveTransactionState::Committed => {
                verify_committed_transaction(root, &record)?;
                return Err(ArchiveError::ExistingRun { as_of });
            }
        }
    }
    let final_paths = final_paths(root, as_of);
    let present_count = final_paths.iter().filter(|path| path.exists()).count();
    if present_count == final_paths.len() {
        return Err(ArchiveError::ExistingRun { as_of });
    }
    if present_count > 0 {
        return Err(ArchiveError::IncompleteRun { as_of });
    }
    compatibility_manifest_allows_update(root, as_of)?;
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

fn commit_archive_transaction(
    root: &Path,
    branch: &str,
    manifest: &ArchiveManifest,
    report: &str,
    snapshot: &str,
    receipt: &str,
    fail_after: Option<ArchiveCommitStage>,
) -> Result<ArchiveManifest, ArchiveError> {
    validate_data_branch(branch)?;
    let _lock = acquire_commit_lock(root, branch, manifest.as_of)?;
    let archive = ensure_archive_directories(root)?;
    ensure_run_available_unlocked(root, branch, manifest.as_of)?;
    let transaction_id = transaction_id(manifest.as_of);
    let staging_directory = format!("{TRANSACTIONS_DIRECTORY}/{transaction_id}");
    let staging_root = archive.join(&staging_directory);
    fs::create_dir(&staging_root).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            ArchiveError::IncompleteRun {
                as_of: manifest.as_of,
            }
        } else {
            ArchiveError::Io {
                operation: "transaction staging directory creation",
            }
        }
    })?;
    let manifest_json =
        serde_json::to_string_pretty(manifest).map_err(|_| ArchiveError::InvalidDate)? + "\n";
    let contents = [report, snapshot, receipt, manifest_json.as_str()];
    let artifacts = transaction_artifacts(
        manifest.as_of,
        &staging_directory,
        report,
        snapshot,
        receipt,
        &manifest_json,
    );
    for (artifact, content) in artifacts.iter().zip(contents) {
        let staged_path = archive_join_relative(&archive, &artifact.staged_path).map_err(|_| {
            ArchiveError::IncompleteRun {
                as_of: manifest.as_of,
            }
        })?;
        write_atomic(&staged_path, content)?;
    }
    let mut record = ArchiveTransactionRecord {
        schema_version: TRANSACTION_RECORD_SCHEMA_VERSION,
        as_of: manifest.as_of,
        transaction_id,
        state: ArchiveTransactionState::Prepared,
        staging_directory,
        artifacts,
        manifest: manifest.clone(),
    };
    write_transaction_record(root, &record)?;
    maybe_injected_failure(fail_after, ArchiveCommitStage::Prepared)?;
    compatibility_manifest_allows_update(root, manifest.as_of)?;

    let staged_artifacts = record
        .artifacts
        .iter()
        .map(|artifact| {
            let content = read_staged_artifact(root, &record, artifact)?;
            Ok((artifact.clone(), content))
        })
        .collect::<Result<Vec<_>, ArchiveError>>()?;
    validate_existing_public_artifacts(root, &record, &staged_artifacts)?;
    let promotion_stages = [
        (
            ArchiveCommitStage::Report,
            false,
            format!(
                "{REPORTS_DIRECTORY}/{}.md",
                manifest.as_of.format("%Y-%m-%d")
            ),
        ),
        (
            ArchiveCommitStage::Snapshot,
            false,
            format!(
                "{SNAPSHOTS_DIRECTORY}/{}.json",
                manifest.as_of.format("%Y-%m-%d")
            ),
        ),
        (
            ArchiveCommitStage::Receipt,
            false,
            format!(
                "{RECEIPTS_DIRECTORY}/{}.json",
                manifest.as_of.format("%Y-%m-%d")
            ),
        ),
        (
            ArchiveCommitStage::Manifest,
            true,
            "manifest.json".to_owned(),
        ),
    ];
    for (stage, allow_existing, final_path) in promotion_stages {
        let (artifact, content) = staged_artifacts
            .iter()
            .find(|(artifact, _)| artifact.final_path == final_path)
            .ok_or(ArchiveError::IncompleteRun {
                as_of: manifest.as_of,
            })?;
        promote_staged_artifact(root, &record, artifact, content, allow_existing)?;
        maybe_injected_failure(fail_after, stage)?;
    }

    record.state = ArchiveTransactionState::Committed;
    write_transaction_record(root, &record)?;
    let _ = fs::remove_dir_all(archive.join(&record.staging_directory));
    retain_recent(root, branch, manifest.as_of, DEFAULT_RETENTION_DAYS)?;
    Ok(manifest.clone())
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

/// Stages and logically commits the final report, snapshot, receipt, and manifest.
///
/// Each individual public file uses an atomic sibling rename, while the
/// date-keyed transaction record is the cross-file commit point. This is not a
/// physical filesystem transaction across the public paths.
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
    commit_archive_transaction(
        root,
        branch,
        &manifest,
        rendered_report.markdown(),
        rendered_report.snapshot_json(),
        &receipt,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "org-x-archive-transaction-{label}-{}",
            TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        root
    }

    fn test_manifest() -> ArchiveManifest {
        let as_of = NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date is valid");
        ArchiveManifest {
            as_of,
            report: "weekly-radar/reports/2026-08-17.md".to_owned(),
            snapshot: "weekly-radar/snapshots/2026-08-17.json".to_owned(),
            receipt: "weekly-radar/receipts/2026-08-17.json".to_owned(),
            input_snapshot: None,
            snapshot_id: None,
        }
    }

    #[test]
    fn archive_transaction_recovers_after_each_public_promotion_failure() {
        for stage in [
            ArchiveCommitStage::Prepared,
            ArchiveCommitStage::Report,
            ArchiveCommitStage::Snapshot,
            ArchiveCommitStage::Receipt,
            ArchiveCommitStage::Manifest,
        ] {
            let root = test_root(stage.as_str());
            let manifest = test_manifest();
            let error = commit_archive_transaction(
                &root,
                "data",
                &manifest,
                "report bytes",
                "snapshot bytes",
                "receipt bytes",
                Some(stage),
            )
            .expect_err("injected failure must stop before commit");
            assert!(matches!(error, ArchiveError::Io { .. }));

            let recovered = recover_pending_run(&root, "data", manifest.as_of())
                .expect("prepared transaction should be recoverable")
                .expect("recovery should complete the prepared transaction");
            assert_eq!(recovered, manifest);
            assert_eq!(
                fs::read_to_string(root.join("weekly-radar/reports/2026-08-17.md"))
                    .expect("report should be recovered"),
                "report bytes"
            );
            assert_eq!(
                fs::read_to_string(root.join("weekly-radar/snapshots/2026-08-17.json"))
                    .expect("snapshot should be recovered"),
                "snapshot bytes"
            );
            assert_eq!(
                fs::read_to_string(root.join("weekly-radar/receipts/2026-08-17.json"))
                    .expect("receipt should be recovered"),
                "receipt bytes"
            );
            assert!(
                root.join("weekly-radar/manifest.json").exists(),
                "compatibility manifest should be recovered"
            );
            fs::remove_dir_all(root).expect("transaction fixture should be removable");
        }
    }

    #[test]
    fn archive_transaction_recovery_rejects_mismatched_public_bytes() {
        let root = test_root("mismatch");
        let manifest = test_manifest();
        commit_archive_transaction(
            &root,
            "data",
            &manifest,
            "report bytes",
            "snapshot bytes",
            "receipt bytes",
            Some(ArchiveCommitStage::Snapshot),
        )
        .expect_err("injected failure must leave a prepared transaction");
        let report_path = root.join("weekly-radar/reports/2026-08-17.md");
        fs::write(&report_path, "unrelated bytes").expect("fixture should create mismatch");

        let error = recover_pending_run(&root, "data", manifest.as_of())
            .expect_err("recovery must fail closed on mismatched public bytes");
        assert!(matches!(error, ArchiveError::IncompleteRun { .. }));
        assert_eq!(
            fs::read_to_string(report_path).expect("mismatched bytes should remain"),
            "unrelated bytes"
        );
        fs::remove_dir_all(root).expect("mismatch fixture should be removable");
    }

    #[test]
    fn archive_recovery_validates_all_staged_bytes_before_promoting_any_public_file() {
        let root = test_root("staged-corruption");
        let manifest = test_manifest();
        commit_archive_transaction(
            &root,
            "data",
            &manifest,
            "report bytes",
            "snapshot bytes",
            "receipt bytes",
            Some(ArchiveCommitStage::Prepared),
        )
        .expect_err("injected failure must leave a prepared transaction");
        let record = read_transaction_record(&root, manifest.as_of())
            .expect("transaction record should be readable")
            .expect("prepared record should exist");
        let snapshot = record
            .artifacts
            .iter()
            .find(|artifact| artifact.final_path == "snapshots/2026-08-17.json")
            .expect("snapshot artifact should exist");
        fs::write(
            archive_root(&root).join(&snapshot.staged_path),
            "corrupted snapshot",
        )
        .expect("fixture should corrupt the staged snapshot");

        let error = recover_pending_run(&root, "data", manifest.as_of())
            .expect_err("recovery must fail before promoting a later-corrupted artifact");
        assert!(matches!(error, ArchiveError::IncompleteRun { .. }));
        assert!(
            !archive_root(&root).join("reports/2026-08-17.md").exists(),
            "no public artifact should be promoted when preflight validation fails"
        );
        fs::remove_dir_all(root).expect("staged corruption fixture should be removable");
    }

    #[test]
    fn committed_transactions_remain_valid_when_a_newer_date_updates_the_manifest() {
        let root = test_root("manifest-history");
        let first = test_manifest();
        commit_archive_transaction(
            &root,
            "data",
            &first,
            "first report",
            "first snapshot",
            "first receipt",
            None,
        )
        .expect("first date should commit");

        let second_date = NaiveDate::from_ymd_opt(2026, 8, 18).expect("fixture date is valid");
        let second = ArchiveManifest {
            as_of: second_date,
            report: "weekly-radar/reports/2026-08-18.md".to_owned(),
            snapshot: "weekly-radar/snapshots/2026-08-18.json".to_owned(),
            receipt: "weekly-radar/receipts/2026-08-18.json".to_owned(),
            input_snapshot: None,
            snapshot_id: None,
        };
        commit_archive_transaction(
            &root,
            "data",
            &second,
            "second report",
            "second snapshot",
            "second receipt",
            None,
        )
        .expect("second date should commit");

        assert_eq!(
            ensure_run_available(&root, "data", first.as_of()),
            Err(ArchiveError::ExistingRun {
                as_of: first.as_of()
            })
        );
        assert_eq!(
            ensure_run_available(&root, "data", second_date),
            Err(ArchiveError::ExistingRun { as_of: second_date })
        );
        assert_eq!(
            recover_pending_run(&root, "data", first.as_of()).expect("old transaction is valid"),
            None
        );
        fs::remove_dir_all(root).expect("manifest history fixture should be removable");
    }

    #[test]
    fn older_date_is_rejected_before_delivery_when_newer_manifest_exists() {
        let root = test_root("manifest-ordering");
        let newer = NaiveDate::from_ymd_opt(2026, 8, 18).expect("fixture date is valid");
        let newer_manifest = ArchiveManifest {
            as_of: newer,
            report: "weekly-radar/reports/2026-08-18.md".to_owned(),
            snapshot: "weekly-radar/snapshots/2026-08-18.json".to_owned(),
            receipt: "weekly-radar/receipts/2026-08-18.json".to_owned(),
            input_snapshot: None,
            snapshot_id: None,
        };
        commit_archive_transaction(
            &root,
            "data",
            &newer_manifest,
            "newer report",
            "newer snapshot",
            "newer receipt",
            None,
        )
        .expect("newer date should commit");

        let older = NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date is valid");
        assert_eq!(
            ensure_run_available(&root, "data", older),
            Err(ArchiveError::IncompleteRun { as_of: older })
        );
        fs::remove_dir_all(root).expect("manifest ordering fixture should be removable");
    }

    #[cfg(unix)]
    #[test]
    fn per_date_execution_lock_rejects_concurrent_owners() {
        let root = test_root("run-lock");
        let as_of = NaiveDate::from_ymd_opt(2026, 8, 17).expect("fixture date is valid");
        let first = acquire_run_lock(&root, "data", as_of).expect("first owner should lock");
        assert!(matches!(
            acquire_run_lock(&root, "data", as_of),
            Err(ArchiveError::ConcurrentRun { as_of: date }) if date == as_of
        ));
        drop(first);
        let _second = acquire_run_lock(&root, "data", as_of).expect("lock should be released");
        fs::remove_dir_all(root).expect("lock fixture should be removable");
    }
}
