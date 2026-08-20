# Weekly Radar Archive Transaction and Recovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a logical archive commit point and deterministic recovery for Weekly Radar while preserving current public paths and APIs.

**Architecture:** `runtime/archive.rs` will stage four serialized artifacts in a hidden transaction directory, record their digests and paths in a prepared date-keyed record, promote the existing public files with per-file atomic writes, and mark the record committed last. A public recovery function verifies staged and public bytes before completing a prepared record; `main.rs` invokes it before acquisition or retry delivery. Existing complete archives without transaction records remain legacy committed runs.

**Tech Stack:** Rust stable, `serde`/`serde_json`, `chrono`, existing filesystem primitives, fixture-driven Cargo tests, and repository AI Cockpit Make targets.

**Spec:** `docs/superpowers/specs/2026-08-20-wi-archive-transaction-recovery.md`

## Global Constraints

- Keep `weekly-radar/reports/YYYY-MM-DD.md`, `weekly-radar/snapshots/YYYY-MM-DD.json`, `weekly-radar/receipts/YYYY-MM-DD.json`, and `weekly-radar/manifest.json` unchanged.
- Keep `write_run` and `write_run_with_input_snapshot` signatures unchanged; add only documented recovery/error API surface.
- Call no live source, Telegram, workflow, or data-branch operation; use local fixtures, injected failures, and hosted CI only.
- Do not add dependencies or introduce Stage, Score, Ranking, Top5, investment, or capital-action behavior.
- Do not claim physical atomicity across multiple public files; document the logical commit point and fail-closed residue behavior.
- Keep all new public Rust items documented and keep input snapshot identity, data-branch, duplicate, retention, and dry-run behavior intact.

---

### Task 1: Specify transaction states and recovery with failing tests

**Files:**
- Modify: `src/features/weekly_radar/runtime/archive.rs` at the archive test module location
- Modify: `tests/weekly_radar_runtime.rs` near the existing archive tests

**Interfaces:**
- Consumes: existing `ArchiveManifest`, `RenderedReport`, archive fixture helpers, and the current per-file archive paths.
- Produces: failing tests for `IncompleteRun`, `recover_pending_run`, prepared transaction recovery, committed visibility, and injected promotion failures.

- [ ] **Step 1: Add a unit test for each commit failure stage.**

  Add a `#[cfg(test)]` module in `archive.rs` that invokes the planned private transaction helper with plain fixture strings and each failure stage (`prepared`, `report`, `snapshot`, `receipt`, and `manifest`). For every failure, assert `ArchiveError::Io`, a prepared transaction record exists, and `recover_pending_run` later creates all four public artifacts and a committed record without changing the staged bytes.

- [ ] **Step 2: Add the fail-closed and legacy compatibility tests.**

  In `tests/weekly_radar_runtime.rs`, add tests that seed one date-specific final file and assert `ensure_run_available` returns `ArchiveError::IncompleteRun` without mutation; seed all three date-specific files without a transaction record and assert `ExistingRun`; seed a prepared record whose staged digest differs from a public file and assert `recover_pending_run` returns `IncompleteRun` while preserving the public bytes.

- [ ] **Step 3: Run the focused tests and record the red state.**

  Run:

  ```sh
  cargo test --lib archive::tests -- --nocapture
  cargo test --test weekly_radar_runtime archive_transaction -- --nocapture
  ```

  Expected: compilation fails because `IncompleteRun`, `recover_pending_run`, and the private transaction helper/test stage do not exist. Do not add production implementation before this failure is observed.

### Task 2: Implement logical commit, recovery, and pre-delivery guards

**Files:**
- Modify: `src/features/weekly_radar/runtime/archive.rs`
- Modify: `src/features/weekly_radar/runtime.rs`
- Modify: `src/main.rs`
- Modify: `tests/weekly_radar_runtime.rs`
- Modify: `tests/weekly_radar_end_to_end.rs`

**Interfaces:**
- Consumes: existing rendered report, receipt, input snapshot, and `write_atomic` behavior.
- Produces: documented `ArchiveError::IncompleteRun`, documented `recover_pending_run`, a date-keyed prepared/committed transaction record, and pre-acquisition recovery/duplicate guards.

- [ ] **Step 1: Define the transaction record and error surface.**

  Add `IncompleteRun { as_of: NaiveDate }`, derive `Deserialize` for `ArchiveManifest`, and define private serde records for transaction state, staged relative paths, and digests. Validate generated relative paths before joining them to the archive root; malformed records must become `IncompleteRun` without exposing filesystem details.

- [ ] **Step 2: Stage serialized artifacts and write the prepared record.**

  Refactor the existing receipt serialization into a string before the final writes. Create `weekly-radar/.transactions/<date>-<pid>-<sequence>/`, write report, rendered snapshot, receipt, and manifest there, calculate deterministic byte identities with the existing digest helper, then atomically write `weekly-radar/.transactions/<date>.json` with `state: prepared`. The transaction record must be durable before any public final path is promoted.

- [ ] **Step 3: Promote matching public files and commit last.**

  Read staged bytes, write each public final path through `write_atomic`, and after every promotion verify the staged/public digest. Write the compatibility manifest as the final public artifact, then atomically rewrite the transaction record with `state: committed`. Run `retain_recent` only after the committed record succeeds. Preserve same-date checks before directory creation and do not overwrite any final date path.

- [ ] **Step 4: Implement recovery and state detection.**

  Implement `recover_pending_run` to validate a prepared record, staged files, and recorded digests; fill only absent public files; accept existing public files only when bytes match; reject mismatches with `IncompleteRun`; update the compatibility manifest only when its existing manifest is absent, equal, or not newer than the prepared date; write the committed record last; then run retention. Update `ensure_run_available` to distinguish valid committed/legacy complete runs from prepared, corrupted, and partial states.

- [ ] **Step 5: Guard normal and retry CLI paths before delivery.**

  Export `recover_pending_run`. In `run_weekly_radar`, keep dry-run before all archive calls; for non-dry runs, recover the requested date before acquisition, reject an existing date before acquisition, and in `--retry-as-of` recover before any Telegram call. A successful recovery returns a `RECOVERED` message and does not acquire or send.

- [ ] **Step 6: Run focused tests and extend no-duplicate coverage.**

  Run:

  ```sh
  cargo fmt --check
  cargo test --lib archive::tests -- --nocapture
  cargo test --test weekly_radar_runtime archive_transaction -- --nocapture
  cargo test --test weekly_radar_runtime task5_archive -- --nocapture
  cargo test --test weekly_radar_end_to_end durable_input_survives_failed_delivery_and_supports_exact_retry -- --nocapture
  ```

  Add a provider-free assertion around the recovery path that the recording transport count does not increase while the prepared transaction is completed.

### Task 3: Document, finish, review, and close the governed Work Item

**Files:**
- Modify: `docs/superpowers/specs/2026-08-20-weekly-radar-snapshot-lifecycle.md`
- Modify: `docs/superpowers/specs/2026-08-20-wi-archive-transaction-recovery.md`
- Modify: `docs/operations/WEEKLY_RADAR.md`
- Modify: `.ai/work-items/active/wi-archive-transaction-recovery.summary.json`
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`, active Outcome files, archive records

**Interfaces:**
- Consumes: implementation/test evidence from Tasks 1–2 and the Contract acceptance/scenario list.
- Produces: evidence-bound operational documentation and a complete archived Work Item.

- [ ] **Step 1: Update lifecycle wording.**

  Replace the existing claim that the four independent final files are collectively atomic with logical transaction/commit wording. Document prepared recovery, fail-closed mismatches, legacy archive compatibility, and the no-second-send rule. Keep provider/production verification explicitly outside this Work Item.

- [ ] **Step 2: Update the Summary with exact evidence.**

  Record each changed file, focused and full test commands, AI Cockpit checks, PR/hosted/merge/close evidence, resolved problems, residual provider boundaries, unknowns, impact, and next action. Do not claim a production Telegram or workflow receipt.

- [ ] **Step 3: Run the mandatory governance and quality checks.**

  Run the required before-finish checkpoint, `make ai-finish TASK=wi-archive-transaction-recovery REPORT_LANGUAGE=zh-CN`, `make check-ai-pr`, and the repository quality commands including `cargo fmt --check`, `cargo test --all`, `make check-ai-status-consistency`, and `make check-ai-coverage-guard`. Resolve every failure within this same Contract scope.

- [ ] **Step 4: Commit, push, create the PR, and verify hosted checks.**

  Commit only declared files on `codex/wi-archive-transaction-recovery`, push the branch, create one PR for this Work Item, and wait for all required hosted checks. Do not merge if any required check is red.

- [ ] **Step 5: Merge, close, and audit residue.**

  Merge the approved PR, run `make ai-close-work-item TASK=wi-archive-transaction-recovery`, and verify archived evidence, base synchronization, no local/remote `codex/*` branch residue, one clean root worktree, and no active Work Item.
