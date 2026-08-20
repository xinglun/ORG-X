# Weekly Radar Snapshot Lifecycle Specification

## Goal

Make the runtime recovery-safe and archive append-only by date while preserving the existing deterministic, rule-only, evidence-first behavior.

## Current gap

The CLI currently executes acquisition, rendering, Telegram delivery, and durable archive writing in that order. A delivery failure can therefore leave no durable copy of the exact `RuntimeReportInput` that produced the attempted report. The archive writer also performs retention before writing and overwrites same-date final files with ordinary filesystem writes. These behaviors make recovery and historical immutability weaker than the roadmap contract.

## Required normal lifecycle

For a non-dry run, the CLI must execute this sequence:

```text
Compute/acquire RuntimeReportInput
  → validate primary-evidence eligibility
  → persist immutable input snapshot
  → render from the persisted input
  → validate rendered report
  → publish to Telegram
  → logically commit staged report, rendered snapshot, receipt, and manifest
  → retain only the configured recent archive window
```

`--dry-run` remains non-mutating: it may acquire, render, and validate, but it must not persist an input snapshot, send Telegram, write archive files, or run retention.

## Persisted input snapshot

The pre-render input is stored at:

```text
weekly-radar/snapshots/YYYY-MM-DD.input.json
```

The envelope is versioned and contains:

```json
{
  "schema_version": 1,
  "as_of": "YYYY-MM-DD",
  "language": "zh-CN|ja|en",
  "has_primary_evidence": true,
  "snapshot_id": "wr-input-...",
  "input": {"as_of": "YYYY-MM-DD", "companies": [], "facts": [], "source_coverage": [], "source_failures": []}
}
```

The `input` value is the existing serializable `RuntimeReportInput`; no new fact semantics are introduced. `snapshot_id` is a deterministic non-cryptographic identity derived from the serialized input and is used for archive traceability, not for security.

Persisting an identical envelope for the same date is idempotent. Persisting different bytes for a date that already has an input snapshot returns an explicit conflict error and does not replace the existing snapshot. This forces a later attempt to use the saved input rather than silently recomputing a mutable historical run.

## Delivery-only retry

The CLI accepts:

```sh
cargo run -- weekly-radar \
  --archive-dir . \
  --retry-as-of YYYY-MM-DD
```

Retry loads the input envelope from the archive, uses its saved date and language, renders from its saved `RuntimeReportInput`, validates the report, sends Telegram, and writes the final archive. It does not read the registry, require `ORGX_SEC_USER_AGENT`, or call source acquisition. It must reject `--retry-as-of` together with `--as-of`, `--language`, or `--dry-run` because those options would make the retry non-identical or non-publishing. A retry checks for an existing final run before sending so an already archived date cannot create a duplicate delivery.

## Final archive integrity

The final report, rendered snapshot, receipt, and manifest are staged in a unique hidden transaction directory. Each public file is still written through a unique sibling temporary file followed by rename, but the date-keyed transaction record is written as `prepared` before promotion and rewritten as `committed` last. Only the committed record is a new transaction's visibility point; this is logical transactional visibility, not physical multi-file filesystem atomicity.

If a process stops between promotions, the next non-dry run or retry first validates and completes the prepared transaction from its staged bytes and persisted delivery receipt, without another Telegram call. Existing public files are accepted only when their bytes match the staged digest. A malformed transaction, a digest mismatch, or a partial date set without a valid transaction record returns `IncompleteRun` without overwriting the conflicting bytes. Complete pre-transaction archives with all three date-specific final files remain legacy committed runs and return `ExistingRun`.

The manifest continues to be `weekly-radar/manifest.json` and additionally records the optional input-snapshot path and deterministic `snapshot_id` when the caller supplies the persisted input. The existing `write_run` API remains available for fixture callers without an input envelope; the CLI uses `write_run_with_input_snapshot`. Transaction metadata stays under `weekly-radar/.transactions/` and is excluded from date-prefixed retention.

Retention runs only after all final files and the manifest have been committed. Invalid branch, receipt, identity, conflict, input-snapshot, and final-write failures occur before retention and do not delete old files.

## Public runtime interfaces

`src/features/weekly_radar/runtime/archive.rs` provides:

```rust
pub const INPUT_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

pub struct InputSnapshot { /* versioned envelope with validated accessors */ }

pub fn persist_input_snapshot(
    root: &Path,
    branch: &str,
    input: &RuntimeReportInput,
    language: ReportLanguage,
    has_primary_evidence: bool,
) -> Result<InputSnapshot, ArchiveError>;

pub fn load_input_snapshot(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<InputSnapshot, ArchiveError>;

pub fn ensure_run_available(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<(), ArchiveError>;

pub fn acquire_run_lock(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<ArchiveRunLock, ArchiveError>;
pub fn recover_pending_run(
    root: &Path,
    branch: &str,
    as_of: NaiveDate,
) -> Result<Option<ArchiveManifest>, ArchiveError>;

pub fn write_run_with_input_snapshot(
    root: &Path,
    branch: &str,
    rendered_report: &RenderedReport,
    delivery_receipt: &TelegramDeliveryReceipt,
    input_snapshot: Option<&InputSnapshot>,
) -> Result<ArchiveManifest, ArchiveError>;
```

All new public items have Rust documentation. `ReportLanguage` remains an enum with its existing stable string values; serialization of the envelope uses those strings.

## Verification requirements

Tests must demonstrate:

1. Input snapshots round-trip exactly, are idempotent for identical bytes, and reject same-date conflicts without mutation.
2. Normal lifecycle persistence precedes rendering/publishing at the CLI boundary; dry-run remains non-mutating.
3. Retry loads the saved language and input without acquisition credentials or source HTTP calls.
4. Same-date final archive writes are rejected before mutation, and each public file uses a temporary sibling.
5. Failure injection after transaction preparation and each public promotion leaves no false committed state; recovery completes matching staged bytes without another provider send.
6. Malformed, mismatched, and partial transaction residue returns `IncompleteRun` without overwriting existing bytes, while complete legacy archives remain protected by `ExistingRun`.
7. Retention does not run for pre-commit failures and does run after a successful complete archive.
8. Existing data-branch guards, receipt identity checks, localized rendering, and deterministic E2E lifecycle behavior remain intact.

## Non-goals

This change does not add Stage, Score, Rank, Top5, investment conclusions, LLM extraction, source-authority reinterpretation, a new dependency, external provider integration tests, or production data-branch operations.
