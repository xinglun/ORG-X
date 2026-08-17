# Weekly Change Compression Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a standalone, deterministic Weekly Radar compression boundary for five explicit change sections plus stable No Change output.

**Architecture:** Keep the source independent of shared `weekly_radar` module registration. Use section-specific newtypes around one opaque fact-preserving record, validate period and event identity at input construction, and return a fixed section view without calculating any upstream domain meaning.

**Tech Stack:** Rust 2021, standard library only, Cargo unit/integration tests, repository AI Cockpit Make targets.

## Global Constraints

- Preserve every supplied event identity, period, company, opaque fact, and per-section insertion order exactly.
- Do not recompute or infer Stage, Ranking, Distance, score, Top5 membership, Rising, Dropped, or Important Structural Change.
- Do not edit `src/features/weekly_radar/domain/mod.rs`, architecture tests, or `.ai/guards/coverage_policy.yaml`.
- Use module-local and integration tests; add same-boundary tests inside this WI if coverage association requires them.
- No dependencies, cross-feature imports, renderers, persistence, scheduling, delivery, provider calls, or trading/capital behavior.

## File map

- Create `src/features/weekly_radar/domain/change_compression.rs`: public value objects, five explicit event newtypes, input validation, stable No Change, and fixed section output.
- Create `src/features/weekly_radar/domain/change_compression_test.rs`: module-local tests compiled through the standalone source path.
- Create `tests/weekly_radar_change_compression.rs`: focused integration tests for every acceptance scenario and source-boundary assertions.
- Create `tests/change_compression_test.rs`: same-stem coverage companion that loads the focused test target.
- Create `docs/superpowers/specs/2026-08-17-wi-wr-007-weekly-change-compression.md`: approved design boundary.
- Create `docs/superpowers/plans/2026-08-17-wi-wr-007-weekly-change-compression.md`: this TDD execution plan.
- Create `.ai/evidence/reference-impact/wi-wr-007-weekly-change-compression.json`: evidence that shared module registration and cross-feature imports are intentionally unchanged.
- Update `.ai/work-items/active/wi-wr-007.summary.json` and generated AI Cockpit evidence during implementation and Finish.

### Task 1: Establish the failing focused tests

**Files:**
- Create: `tests/weekly_radar_change_compression.rs`
- Create: `tests/change_compression_test.rs`
- Modify: `src/features/weekly_radar/domain/change_compression.rs` only to add the module-local test path after the first production API exists.

**Interfaces:**
- Consumes: the planned public types `PeriodId`, `EventId`, `CompanyReference`, `FactValue`, five section event types, `WeeklyChangeInput`, `WeeklyChangeCompression`, `CompressionSection`, and `ChangeCompressionError`.
- Produces: failing behavioral tests that define the exact construction and accessors used by the implementation.

- [ ] **Step 1: Write the focused integration test with the desired API.**

  The test imports the standalone source with `#[path = "../src/features/weekly_radar/domain/change_compression.rs"]`, constructs one event in each section, asserts exact fields and within-section order, asserts `sections()` returns the six fixed variants, asserts empty input emits `NO_CHANGE` with zero counts, and asserts duplicate identity/period mismatch errors. Add a source assertion for absence of `use crate::`, `features::`, `WeeklyRadarSnapshot`, and `telegram`.

- [ ] **Step 2: Add the same-stem coverage companion.**

  Create `tests/change_compression_test.rs` with:

  ```rust
  #[path = "weekly_radar_change_compression.rs"]
  mod weekly_radar_change_compression;
  ```

- [ ] **Step 3: Run the focused test and verify the expected RED failure.**

  Run `cargo test --test weekly_radar_change_compression`.

  Expected result: compilation fails because `change_compression.rs` and its exported API do not yet exist; the failure must be a missing module/API failure rather than an assertion typo.

### Task 2: Implement the fact-preserving compression boundary

**Files:**
- Create: `src/features/weekly_radar/domain/change_compression.rs`
- Create: `src/features/weekly_radar/domain/change_compression_test.rs`

**Interfaces:**
- Consumes: failing integration tests from Task 1.
- Produces: `WeeklyChangeInput::new`, `WeeklyChangeCompression::from_input`, all public value objects and event newtypes, `CompressionSection<'a>`, `NoChange`, and typed deterministic errors.

- [ ] **Step 1: Add non-empty value objects and typed errors.**

  Define `PeriodId`, `EventId`, `CompanyReference`, and `FactValue` as private-string newtypes with `new` and `as_str`. Define `ChangeCompressionError::{EmptyValue, PeriodMismatch, DuplicateIdentity}` with `Display` and `Error` implementations. Preserve nonblank values exactly; only whitespace-only values are rejected.

- [ ] **Step 2: Add five explicit event newtypes.**

  Define `ImportantStructuralChange`, `Top5Change`, `StageTransitionChange`, `RisingChange`, and `DroppedChange` as public wrappers over a private record containing `event_id`, `period`, `company`, and `fact`. Each `new` validates value objects and each accessor returns a reference to the supplied field.

- [ ] **Step 3: Add input validation without sorting or merging.**

  Define:

  ```rust
  pub fn new(
      period: PeriodId,
      important_structural: Vec<ImportantStructuralChange>,
      top5: Vec<Top5Change>,
      stage_transitions: Vec<StageTransitionChange>,
      rising: Vec<RisingChange>,
      dropped: Vec<DroppedChange>,
  ) -> Result<Self, ChangeCompressionError>
  ```

  Validate every event period against `period` and insert each event ID into a standard-library identity set in input section order. Return the first deterministic duplicate or mismatch and retain original vector order for accepted input.

- [ ] **Step 4: Add stable No Change and fixed section output.**

  Define `ChangeCounts` with five zero-valued counters, `NoChange` with `LABEL == "NO_CHANGE"`, and `CompressionSection<'a>` with variants for the five slices plus `NoChange(Option<&'a NoChange>)`. `WeeklyChangeCompression::from_input` moves the vectors without rewriting them, creates `Some(NoChange)` only when all five vectors are empty, and `sections()` returns the six variants in the specified order.

- [ ] **Step 5: Add module-local tests.**

  Test typed value rejection, exact fact retention, deterministic zero-count No Change, and duplicate identity rejection from inside `change_compression_test.rs`. Keep the tests independent of shared module registration.

- [ ] **Step 6: Run the focused tests and verify GREEN.**

  Run `cargo test --test weekly_radar_change_compression` and `cargo test --test change_compression_test`.

  Expected result: all focused integration and module-local tests pass with no warnings.

### Task 3: Record reference impact and run quality checks

**Files:**
- Create: `.ai/evidence/reference-impact/wi-wr-007-weekly-change-compression.json`
- Do not modify: `src/features/weekly_radar/domain/mod.rs`, `tests/architecture/module_boundaries.rs`, `.ai/guards/coverage_policy.yaml`.

**Interfaces:**
- Consumes: the standalone source and focused tests from Task 2.
- Produces: machine-readable evidence that shared registration and architecture/coverage policy paths remain unchanged.

- [ ] **Step 1: Write reference-impact evidence.**

  Record the Work Item, source path, focused test paths, unchanged shared paths, and the conclusion that no cross-feature or shared-module reference is required.

- [ ] **Step 2: Run formatter, Clippy, all tests, and architecture checks.**

  Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`.

- [ ] **Step 3: Update Summary with implementation evidence.**

  Record changed files, exact focused/all-quality commands, scenario evidence, guideline compliance, residual risks, no destructive changes, and the local-only next action. Keep scenario statuses unverified until the final verification commands have run.

### Task 4: Finish, archive, and commit locally

**Files:**
- Modify: `.ai/work-items/active/wi-wr-007.summary.json`
- Generated/archived by Make targets: `.ai/cockpit/*`, `.ai/work-items/archive/**/wi-wr-007.*`, `.ai/work-items/active/wi-wr-007.outcome.*`

**Interfaces:**
- Consumes: verified implementation, tests, reference-impact evidence, active Contract, and updated Summary.
- Produces: strict AI Cockpit Finish evidence, archived Work Item bundle, and one local commit with no provider-side actions.

- [ ] **Step 1: Run the canonical before-finish checkpoint.**

  Run `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-wr-007.contract.json SUMMARY=.ai/work-items/active/wi-wr-007.summary.json STAGE=before_finish`.

- [ ] **Step 2: Run strict AI Cockpit Finish.**

  Run `make ai-finish TASK=wi-wr-007 REPORT_LANGUAGE=zh-CN` and treat any failed gate as a same-WI correction.

- [ ] **Step 3: Archive the finished Work Item.**

  Run `make archive-work-item TASK=wi-wr-007` using the repository's current Make entrypoint.

- [ ] **Step 4: Run post-archive local validation.**

  Run the repository quality checks and `make check-ai-pr AI_BASE_COMMIT=ba346c2bbd538d2f734951ca05f3ad0322979cfa` only if the local archive workflow declares it required; do not push, open a PR, merge, or close.

- [ ] **Step 5: Commit the complete local bundle.**

  Verify `git status`, the diff ownership report, the absence of out-of-scope files, and the final test/Finish evidence. Commit with:

  ```bash
  git add .ai docs/superpowers/specs/2026-08-17-wi-wr-007-weekly-change-compression.md docs/superpowers/plans/2026-08-17-wi-wr-007-weekly-change-compression.md src/features/weekly_radar/domain/change_compression.rs src/features/weekly_radar/domain/change_compression_test.rs tests/weekly_radar_change_compression.rs tests/change_compression_test.rs
  git commit -m "feat: add weekly change compression boundary"
  ```

  Confirm the commit SHA and leave the dedicated branch/worktree intact for the user's later provider lifecycle.
