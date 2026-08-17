//! Deterministic Weekly Radar archive files and bounded retention.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDate};
use serde::Serialize;

use super::report::RenderedReport;

const ARCHIVE_DIRECTORY: &str = "weekly-radar";
const REPORTS_DIRECTORY: &str = "reports";
const SNAPSHOTS_DIRECTORY: &str = "snapshots";
const RECEIPTS_DIRECTORY: &str = "receipts";
const DEFAULT_RETENTION_DAYS: i64 = 365;

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
        }
    }
}

impl std::error::Error for ArchiveError {}

/// Stable archive manifest written after each report.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArchiveManifest {
    as_of: NaiveDate,
    report: String,
    snapshot: String,
    receipt: String,
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

/// Removes only date-prefixed files older than the requested retention window.
pub fn retain_recent(
    root: &Path,
    as_of: NaiveDate,
    retention_days: i64,
) -> Result<usize, ArchiveError> {
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

fn write_file(path: &Path, content: &str) -> Result<(), ArchiveError> {
    fs::write(path, content).map_err(|_| ArchiveError::Io {
        operation: "file write",
    })
}

/// Writes the deterministic report, snapshot, receipt, and manifest files.
pub fn write_run(
    root: &Path,
    rendered_report: &RenderedReport,
) -> Result<ArchiveManifest, ArchiveError> {
    let date = rendered_report.as_of();
    retain_recent(root, date, DEFAULT_RETENTION_DAYS)?;
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
    };
    write_file(
        &archive
            .join(REPORTS_DIRECTORY)
            .join(format!("{date_text}.md")),
        rendered_report.markdown(),
    )?;
    write_file(
        &archive
            .join(SNAPSHOTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        rendered_report.snapshot_json(),
    )?;
    let receipt = format!(
        "{{\n  \"as_of\": \"{date_text}\",\n  \"status\": \"NOT_PUBLISHED\",\n  \"message_ids\": []\n}}\n"
    );
    write_file(
        &archive
            .join(RECEIPTS_DIRECTORY)
            .join(format!("{date_text}.json")),
        &receipt,
    )?;
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|_| ArchiveError::InvalidDate)? + "\n";
    write_file(&archive.join("manifest.json"), &manifest_json)?;
    Ok(manifest)
}

/// Performs a write only when the caller explicitly identifies the data branch.
pub fn write_run_guarded(
    root: &Path,
    branch: &str,
    rendered_report: &RenderedReport,
) -> Result<ArchiveManifest, ArchiveError> {
    if branch != "data" {
        return Err(ArchiveError::NonDataBranch {
            branch: branch.to_owned(),
        });
    }
    write_run(root, rendered_report)
}
