# Weekly Radar Snapshot Lifecycle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persist the exact Weekly Radar runtime input before delivery, support source-free delivery retry, and make final date archives atomic and non-overwriting.

**Architecture:** Keep `RuntimeReportInput` as the serialized compute boundary. Add a versioned input envelope and safe archive primitives in `runtime/archive.rs`; the CLI branches before acquisition for retry, persists the envelope before rendering for normal publish, and passes the envelope into the final archive manifest. Preserve the existing renderer, Telegram adapter, data-branch guard, and fixture-driven tests.

**Tech Stack:** Rust stable, Cargo, `serde`/`serde_json`, `chrono`, filesystem `OpenOptions`/`rename`, existing CLI and Telegram transport seams, repository Make/AI Cockpit gates.

**Spec:** `docs/superpowers/specs/2026-08-20-weekly-radar-snapshot-lifecycle.md`

## Global Constraints

- Keep the runtime deterministic, rule-only, evidence-first, and provider-neutral.
- Use the existing `RuntimeReportInput` serialization boundary; add no dependency and no new source semantics.
- Keep `--dry-run` archive- and delivery-free, and keep archive writes guarded to the literal `data` branch.
- Do not add Stage, Score, Rank, Top5, investment conclusions, LLM extraction, or production provider operations.
- Do not expose secret-bearing filesystem details in new errors or documentation.
- Every new public Rust item has a documentation comment.

---

### Task 1: Lock the input-snapshot contract with failing tests

**Files:**
- Modify: `tests/weekly_radar_runtime.rs` near the existing archive tests
- Modify: `src/features/weekly_radar/runtime.rs` only for the imports required by test compilation after the API exists

**Interfaces:**
- Consumes: existing `RuntimeReportInput`, `ReportLanguage`, `render_report`, `send_rendered_report_with_transport`, and `write_run` fixture helpers.
- Produces: test expectations for `persist_input_snapshot`, `load_input_snapshot`, `InputSnapshot`, and same-date conflict behavior.

- [ ] **Step 1: Write the failing round-trip and conflict tests.**

Add tests with these behaviors and concrete assertions:

```rust
#[test]
fn task5_input_snapshot_round_trips_and_is_idempotent() {
    let root = task4_temp_root("input-snapshot");
    let input = task4_report_input();
    let first = persist_input_snapshot(
        &root, "data", &input, ReportLanguage::Japanese, true,
    ).expect("input snapshot should persist");
    let second = persist_input_snapshot(
        &root, "data", &input, ReportLanguage::Japanese, true,
    ).expect("identical input snapshot should be idempotent");
    assert_eq!(first, second);
    let loaded = load_input_snapshot(&root, "data", input.as_of())
        .expect("input snapshot should load");
    assert_eq!(loaded.input(), &input);
    assert_eq!(loaded.language(), ReportLanguage::Japanese);
    assert!(loaded.has_primary_evidence());
}

#[test]
fn task5_input_snapshot_rejects_a_same_date_conflict_without_mutation() {
    let root = task4_temp_root("input-snapshot-conflict");
    let original = task4_report_input();
    persist_input_snapshot(&root, "data", &original, ReportLanguage::Chinese, true)
        .expect("original input snapshot should persist");
    let before = fs::read(root.join("weekly-radar/snapshots/2026-08-17.input.json"))
        .expect("original bytes should exist");
    let mut different = original.clone();
    different.add_fact(
        NormalizedFact::new(
            "omega",
            "revenue",
            "99000000",
            FactStatus::Known,
            Confidence::Medium,
            task4_provenance("facts.revenue"),
        )
        .expect("distinct fixture fact should be valid"),
    )
    .expect("distinct fixture fact should be unique");
    let error = persist_input_snapshot(&root, "data", &different, ReportLanguage::Chinese, true)
        .expect_err("conflicting input must be rejected");
    assert!(matches!(error, ArchiveError::InputSnapshotConflict { .. }));
    assert_eq!(fs::read(root.join("weekly-radar/snapshots/2026-08-17.input.json")).unwrap(), before);
}
```

Use the concrete `omega`/`revenue` fixture above; it must remain a unique company/kind pair in the input.

- [ ] **Step 2: Run the focused tests and confirm the red state.**

Run:

```sh
cargo test --test weekly_radar_runtime task5_input_snapshot -- --nocapture
```

Expected: compilation fails because the input-snapshot APIs and conflict variant are not yet defined. Do not modify production code before recording this failure.

- [ ] **Step 3: Commit the failing-test checkpoint.**

```sh
git add tests/weekly_radar_runtime.rs
git commit -m "test: specify weekly radar input snapshot lifecycle"
```

### Task 2: Implement immutable input persistence and safe final archive primitives

**Files:**
- Modify: `src/features/weekly_radar/runtime/archive.rs`
- Modify: `src/features/weekly_radar/runtime.rs`
- Modify: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Consumes: `RuntimeReportInput`, `ReportLanguage`, `RenderedReport`, `TelegramDeliveryReceipt`.
- Produces: `InputSnapshot`, `persist_input_snapshot`, `load_input_snapshot`, `ensure_run_available`, `write_run_with_input_snapshot`, `ArchiveError::{ExistingRun, InputSnapshotConflict, MissingInputSnapshot, InvalidInputSnapshot}`.

- [ ] **Step 1: Add the failing same-date archive and retention tests.**

Add a test that writes one final fixture run, captures the bytes of `reports/YYYY-MM-DD.md`, `snapshots/YYYY-MM-DD.json`, `receipts/YYYY-MM-DD.json`, and `manifest.json`, then attempts a second different report for the same date. Assert `ArchiveError::ExistingRun` and byte-for-byte equality after the error. Add an old date-prefixed file and exercise an invalid receipt before a successful run; assert the old file remains after the invalid receipt and is removed only after the successful run.

The test calls the new API with an input envelope:

```rust
let input_snapshot = persist_input_snapshot(
    &root, "data", &input, ReportLanguage::Chinese, true,
).expect("input snapshot should persist");
write_run_with_input_snapshot(
    &root, "data", &report, &receipt, Some(&input_snapshot),
).expect("first run should archive");
```

- [ ] **Step 2: Run the focused archive tests and confirm the red state.**

Run:

```sh
cargo test --test weekly_radar_runtime task5_archive -- --nocapture
```

Expected: compilation fails for the new API and/or the assertions fail against the current overwrite behavior.

- [ ] **Step 3: Implement the versioned input envelope.**

In `archive.rs`, add documented public types and functions with these signatures:

```rust
pub const INPUT_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

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
```

Serialize a deterministic envelope containing `schema_version`, `as_of`, `language.as_str()`, `has_primary_evidence`, `snapshot_id`, and `input`. Compute `snapshot_id` from the serialized `RuntimeReportInput` using the existing non-cryptographic FNV-style identity pattern. Validate schema version, date agreement, supported language, and the computed identity on load. Write to `weekly-radar/snapshots/YYYY-MM-DD.input.json`; return success without rewriting when bytes are identical and return `InputSnapshotConflict` when the path already contains different bytes.

- [ ] **Step 4: Implement atomic file writes and pre-mutation final-run checks.**

Replace plain `fs::write` for final archive files with a helper that opens a unique sibling temporary file using `create_new`, writes all bytes, flushes and syncs it, then renames it to the final path; remove only the temporary file on failure. Add `ensure_run_available(root, branch, as_of)` and check the three date-specific final paths before directory creation, retention, or final writes. Treat any existing report, rendered snapshot, or receipt as `ExistingRun`.

Add `write_run_with_input_snapshot` and keep `write_run` as the documented compatibility wrapper passing `None`. Validate branch, receipt identity, receipt cardinality, input-snapshot date, and final-run availability before writing. Stage report, snapshot, receipt, then manifest last. Add optional manifest fields for `input_snapshot` and `snapshot_id`; use the input envelope when supplied. Call `retain_recent` only after all four writes succeed.

- [ ] **Step 5: Run focused tests until green.**

Run:

```sh
cargo test --test weekly_radar_runtime task5_input_snapshot -- --nocapture
cargo test --test weekly_radar_runtime task5_archive -- --nocapture
```

Expected: all new input, conflict, atomicity, and retention tests pass, and the existing archive tests continue to pass.

- [ ] **Step 6: Commit the archive implementation.**

```sh
git add src/features/weekly_radar/runtime/archive.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_runtime.rs
git commit -m "feat: make weekly radar archives immutable by date"
```

### Task 3: Add source-free CLI retry and enforce normal lifecycle ordering

**Files:**
- Modify: `src/main.rs`
- Modify: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Consumes: `persist_input_snapshot`, `load_input_snapshot`, `ensure_run_available`, `write_run_with_input_snapshot`.
- Produces: `CliOptions.retry_as_of`, `--retry-as-of YYYY-MM-DD`, normal Compute → Persist → Render → Publish → Archive execution, and source-free retry.

- [ ] **Step 1: Write failing CLI parser and retry tests.**

Add parser assertions for `--retry-as-of`, rejection with `--as-of`, rejection with `--language`, and rejection with `--dry-run`. Add a CLI retry fixture test that writes a valid input snapshot, removes `ORGX_SEC_USER_AGENT` and Telegram environment values, invokes `weekly-radar --archive-dir /tmp/org-x-weekly-radar-retry-fixture --retry-as-of 2026-08-17`, and asserts the failure is a missing Telegram configuration rather than an acquisition/user-agent error. Add a normal-run sequence test using an archive input file assertion before the delivery seam where that seam is available; preserve the existing dry-run assertion that the archive root is absent.

Expected parser shape:

```rust
assert!(matches!(
    parse_options(&args("weekly-radar --retry-as-of 2026-08-17")),
    Ok(CliAction::Run(CliOptions { retry_as_of: Some(date), .. })) if date == NaiveDate::from_ymd_opt(2026, 8, 17).unwrap()
));
```

- [ ] **Step 2: Run the CLI tests and confirm the red state.**

Run:

```sh
cargo test --test weekly_radar_runtime task5_cli_retry -- --nocapture
```

Expected: compilation or assertion failure because the option and retry branch do not exist.

- [ ] **Step 3: Implement parser state and usage text.**

Add `retry_as_of: Option<NaiveDate>` and explicit-option tracking for `--as-of` and `--language`. Parse `--retry-as-of` with the same `YYYY-MM-DD` validation. At the end of parsing reject combinations with explicit `--as-of`, explicit `--language`, or `--dry-run`; retain the existing defaults for normal runs. Update the usage string and add parser documentation through the existing help output.

- [ ] **Step 4: Implement the normal lifecycle branch.**

Keep registry loading, user-agent validation, acquisition, and the existing primary-evidence guard in the normal branch. For dry-run, render and validate without persistence as before. For non-dry-run, call:

```rust
let input_snapshot = persist_input_snapshot(
    &options.archive_dir,
    "data",
    &acquired.input,
    options.language,
    acquired.has_primary_evidence,
)?;
let report = render_report_in_language(input_snapshot.input(), input_snapshot.language());
validate_rendered_report(&report)?;
let receipt = send_rendered_report(&report)?;
write_run_with_input_snapshot(
    &options.archive_dir,
    "data",
    &report,
    &receipt,
    Some(&input_snapshot),
)?;
```

Use the saved input and language returned by the envelope; do not recompute input for rendering.

- [ ] **Step 5: Implement the retry branch before acquisition.**

When `retry_as_of` is present, load the envelope from `archive_dir` on the `data` branch, reject an envelope without primary evidence, call `ensure_run_available` before Telegram, render with the envelope’s saved `RuntimeReportInput` and `ReportLanguage`, validate, publish, and call `write_run_with_input_snapshot`. Do not load the registry or call `sec_user_agent` in this branch. Keep output wording explicit that the report was retried from the persisted input.

- [ ] **Step 6: Run CLI and full runtime tests until green.**

Run:

```sh
cargo test --test weekly_radar_runtime task5_cli -- --nocapture
cargo test --test weekly_radar_runtime
```

Expected: retry, parser, dry-run, archive, and existing runtime tests pass.

- [ ] **Step 7: Commit the CLI lifecycle implementation.**

```sh
git add src/main.rs tests/weekly_radar_runtime.rs
git commit -m "feat: add weekly radar delivery-only retry"
```

### Task 4: Extend lifecycle E2E coverage and operational documentation

**Files:**
- Modify: `tests/weekly_radar_end_to_end.rs`
- Modify: `docs/operations/WEEKLY_RADAR.md`

**Interfaces:**
- Consumes: the existing in-memory Compute → Persist → Render → Publish → Archive E2E fixture and the runtime input/archive APIs.
- Produces: E2E evidence that a persisted input survives a failed delivery and that a later retry uses the same input identity; operator instructions for normal execution and recovery.

- [ ] **Step 1: Add the failing durable-input E2E assertion.**

Add a fixture-driven E2E test that creates a valid `RuntimeReportInput`, persists it before invoking the existing recording transport, forces one transport failure, asserts the input snapshot file remains and reloads equal to the original, then sends the same rendered report through a successful transport and archives it with `write_run_with_input_snapshot`. Assert the manifest contains the input-snapshot path and snapshot ID, and assert the final report bytes are derived from the reloaded input.

- [ ] **Step 2: Run the E2E test and confirm the red state.**

Run:

```sh
cargo test --test weekly_radar_end_to_end durable_input -- --nocapture
```

Expected: the test fails to compile or fails because the runtime E2E fixture does not yet exercise the durable input API.

- [ ] **Step 3: Implement the E2E assertions and run the suite.**

Use only temporary local paths and the existing fake transport. Do not contact SEC, Telegram, GitHub, or a live data branch. Run:

```sh
cargo test --test weekly_radar_end_to_end
```

Expected: all existing and new E2E tests pass.

- [ ] **Step 4: Update the operations guide.**

Document the exact normal sequence, `weekly-radar/snapshots/YYYY-MM-DD.input.json`, the `--retry-as-of` command, the saved-language behavior, same-date rejection, and the fact that retry must be used after a delivery failure instead of reacquiring the date. State that dry-run does not persist input snapshots and that no provider values belong in the repository. Keep the existing scheduling, environment, source-boundary, and retention guidance.

- [ ] **Step 5: Commit the E2E and documentation changes.**

```sh
git add tests/weekly_radar_end_to_end.rs docs/operations/WEEKLY_RADAR.md
git commit -m "docs: describe weekly radar snapshot recovery"
```

### Task 5: Run repository verification and governed lifecycle closure

**Files:**
- Modify: `.ai/work-items/active/wi-weekly-radar-snapshot-lifecycle.summary.json` through `make ai-finish`
- Generate: `.ai/cockpit/current_status.md`, `.ai/work-items/starts/**`, `.ai/work-items/archive/**`, and the active Outcome through repository Make targets

**Interfaces:**
- Consumes: all implementation, test, spec, and documentation changes from Tasks 1–4.
- Produces: evidence-bound Work Item Summary, archived Work Item record, merged PR, closed lifecycle, and clean local/remote state.

- [ ] **Step 1: Run formatting and focused verification.**

Run:

```sh
cargo fmt --all -- --check
cargo test --test weekly_radar_runtime
cargo test --test weekly_radar_end_to_end
```

Expected: all commands exit successfully.

- [ ] **Step 2: Inspect scope and behavior.**

Run:

```sh
git diff --check
git diff --stat
git diff -- src/main.rs src/features/weekly_radar/runtime/archive.rs src/features/weekly_radar/runtime.rs src/features/weekly_radar/runtime/report.rs tests/weekly_radar_runtime.rs tests/weekly_radar_end_to_end.rs docs/operations/WEEKLY_RADAR.md
```

Confirm no Stage/Score/Rank/Top5 or investment behavior was added, no external credentials or provider values were added, and all changed paths belong to the Contract.

- [ ] **Step 3: Run the mandatory governance checkpoints and quality gate.**

Run:

```sh
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-snapshot-lifecycle.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-snapshot-lifecycle.summary.json STAGE=before_finish
make quality
make ai-finish TASK=wi-weekly-radar-snapshot-lifecycle REPORT_LANGUAGE=zh-CN
```

If the same-scope evidence validator reports a documentation or summary mismatch, correct only the Contract/Summary evidence, rerun the checkpoint, and rerun `ai-finish`; preserve the blocked Outcome as required by repository governance.

- [ ] **Step 4: Archive, publish, merge, and close the Work Item.**

Deliver the active Outcome into the conversation before archive, then run:

```sh
make archive-work-item TASK=wi-weekly-radar-snapshot-lifecycle
git add .ai/work-items/archive .ai/work-items/active .ai/cockpit/current_status.md
git commit -m "chore: archive weekly radar snapshot lifecycle work item"
make check-ai-pr AI_BASE_COMMIT=f13383979839ecb4a40fe38503a43ce3e043b591
git push -u origin codex/wi-weekly-radar-snapshot-lifecycle
gh pr create --base main --head codex/wi-weekly-radar-snapshot-lifecycle --title "Persist Weekly Radar snapshots before delivery" --body "Persist the exact runtime input before delivery, add delivery-only retry, and make date archives immutable."
gh pr checks --watch
gh pr merge --merge
make ai-close-work-item TASK=wi-weekly-radar-snapshot-lifecycle
```

Record hosted-check warnings as residual risks when the provider scenario cannot be executed locally; do not claim provider behavior that was not observed. After close, inspect `git status --short --branch`, `git worktree list --porcelain`, local branches, and remote branches and report any residue as a blocker.

## Self-review

- Spec coverage: Tasks 1–2 cover versioned persistence, idempotency, conflicts, atomic writes, same-date protection, and retention timing; Task 3 covers normal and retry CLI ordering; Task 4 covers E2E and operations; Task 5 covers all governed verification and closure.
- Placeholder scan: this plan contains no `TBD`, `TODO`, “implement later”, or unspecified test-only step; each code task includes a concrete signature, assertion, or command.
- Type consistency: `InputSnapshot` is returned by `persist_input_snapshot` and consumed by `load_input_snapshot`, `write_run_with_input_snapshot`, and the CLI; `ReportLanguage` is saved as its stable string and restored before rendering.
