# WI-WR-009 Telegram Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a deterministic Weekly Radar Telegram Markdown summary boundary that formats explicit facts atomically without recalculating domain values.

**Architecture:** Keep the renderer in `features/weekly_radar/interface` as a standalone provider-agnostic module. `TelegramSummaryInput` owns explicit view facts, `TelegramRenderLimits` describes caller-supplied constraints, and `TelegramRenderer` builds complete Markdown blocks in fixed priority order. Limit failures return typed errors before any truncated or partial output is exposed.

**Tech Stack:** Rust 2021, standard library only, Cargo unit/integration tests, repository AI Cockpit Make targets.

## Global Constraints

- Consume only explicit period, section item, company-card, System Health, and No Change facts.
- Preserve every supplied Markdown fragment and company-card body; never truncate, split, sort, normalize, or merge them.
- Render sections in the order Important Structural Change, Stage Transition, Top5, Threshold Distance, Rising, Dropped, System Health, No Change.
- Reject caller-supplied item, company-card, character, and line limit violations with typed errors.
- Do not calculate Stage, Ranking, Threshold Distance, Important Structural Change, Stage Transition, Top5, Rising, Dropped, System Health, or No Change.
- Do not implement Publisher, HTTP, sensitive runtime configuration, retry, scheduling, persistence, message splitting, receipts, or external providers.
- Do not modify shared architecture tests, global coverage policy, unrelated Work Items, or other bounded contexts.

---

## File map

- Create `src/features/weekly_radar/interface/telegram_renderer.rs`: explicit input types, typed validation errors, atomic limits, deterministic Markdown renderer, and immutable message output.
- Create `src/features/weekly_radar/interface/telegram_renderer_test.rs`: module-local tests for the renderer boundary.
- Modify `src/features/weekly_radar/interface/mod.rs`: export the renderer module through the existing Weekly Radar interface boundary.
- Create `tests/weekly_radar_telegram_renderer.rs`: public integration tests for all sections, ordering, limits, and source isolation.
- Create `tests/telegram_renderer_test.rs`: same-stem companion target loading the public integration target for coverage association.
- Create `.ai/evidence/reference-impact/wi-wr-009-telegram-renderer.json`: evidence of unchanged global architecture/coverage policy and absent provider dependencies.
- Create `docs/superpowers/specs/2026-08-17-wi-wr-009-telegram-renderer.md`: approved design boundary.
- Create this plan and update governed AI Cockpit evidence through the active Summary, Finish, and Archive targets.

### Task 1: Write the failing API and behavior tests

**Files:**
- Create: `tests/weekly_radar_telegram_renderer.rs`
- Create: `tests/telegram_renderer_test.rs`
- Do not create production implementation before the focused test has failed.

**Interfaces:**
- Consumes the planned `PeriodId`, `ItemId`, `CompanyReference`, `SummaryItem`, `CompanyCard`, `SystemHealthSummary`, `NoChangeSummary`, `TelegramSummaryInput`, `TelegramRenderLimits`, `TelegramRenderer`, and `TelegramMessage` API.
- Produces red tests that specify exact section markers, full content retention, and error behavior.

- [ ] **Step 1: Add test helpers and the all-sections scenario.**

  Load `src/features/weekly_radar/interface/telegram_renderer.rs` with a path module so the initial test reports the missing implementation. Construct one explicit item for Important Structural Change and Stage Transition, one complete company card for each of Top5, Threshold Distance, Rising, and Dropped, one System Health summary, and one period. Build `TelegramSummaryInput`, render it with limits larger than the fixture, and assert that all supplied Markdown fragments occur exactly in the output and that section markers occur in the fixed priority order.

- [ ] **Step 2: Add explicit No Change and validation scenarios.**

  Build an input with empty change collections and `Some(NoChangeSummary)`; assert the stable `NO_CHANGE` marker and full no-change statement. Add assertions for blank identities/Markdown, duplicate item identities, missing change state, and contradictory No Change plus a non-empty change section.

- [ ] **Step 3: Add atomic limit scenarios.**

  Use small caller-supplied limits to assert `ItemLimitExceeded`, `CompanyCardLimitExceeded`, `MessageTooLong`, and `LineLimitExceeded`. Assert that the error is returned instead of a partial message and that a long company-card body is not shortened.

- [ ] **Step 4: Add the same-stem coverage companion.**

  Create `tests/telegram_renderer_test.rs` with:

  ```rust
  #[path = "weekly_radar_telegram_renderer.rs"]
  mod weekly_radar_telegram_renderer;
  ```

- [ ] **Step 5: Run the focused target and verify RED.**

  Run `cargo test --test weekly_radar_telegram_renderer`.

  Expected result: compilation fails because the path-loaded renderer source and its public API do not exist. Fix only test typos if needed; do not add production code until the failure identifies the missing implementation.

### Task 2: Implement explicit input facts and validation

**Files:**
- Create: `src/features/weekly_radar/interface/telegram_renderer.rs`
- Create: `src/features/weekly_radar/interface/telegram_renderer_test.rs`

**Interfaces:**
- Consumes the red tests from Task 1.
- Produces validated `PeriodId`, `ItemId`, `CompanyReference`, `SummaryItem`, `CompanyCard`, `SystemHealthSummary`, `NoChangeSummary`, and `TelegramSummaryInput` values.

- [ ] **Step 1: Add typed errors and non-empty text newtypes.**

  Define `TelegramRenderError` with `EmptyValue { field }`, `DuplicateIdentity { id }`, `PeriodMismatch { expected, actual }`, `MissingChangeState`, `ConflictingNoChange`, `InvalidLimit { field }`, `ItemLimitExceeded { section, limit, actual }`, `CompanyCardLimitExceeded { limit, actual }`, `MessageTooLong { limit, actual }`, and `LineLimitExceeded { limit, actual }`. Implement `Display` and `Error`.

  Define private-string newtypes `PeriodId`, `ItemId`, `CompanyReference`, and `MarkdownFragment`. Each `new` rejects whitespace-only input and each `as_str` returns the exact supplied value without trimming or normalization.

- [ ] **Step 2: Add explicit section records.**

  Define `SummaryItem { id, markdown }` and `CompanyCard { id, company, markdown }`, each with constructors and immutable accessors. Define `SystemHealthSummary { status, markdown }` and `NoChangeSummary { period, markdown }`; `NoChangeSummary::LABEL` must be the stable `NO_CHANGE` string and its constructor must retain the supplied statement.

- [ ] **Step 3: Validate the grouped input without mutation or recomputation.**

  Define `TelegramSummaryInput::new` with the eight explicit section arguments. Validate the input period against `NoChangeSummary.period`, reject duplicate identities across all change vectors, reject a missing change state when all six change vectors are empty and no explicit No Change exists, and reject explicit No Change when any change vector is non-empty. Keep every vector in caller order and expose immutable slice accessors.

- [ ] **Step 4: Add module-local tests and verify GREEN for input behavior.**

  In `telegram_renderer_test.rs`, test exact value retention, duplicate identity rejection, period mismatch, explicit No Change, contradictory state, and blank-value errors. Run `cargo test --test weekly_radar_telegram_renderer` and `cargo test --test telegram_renderer_test`; the input portions must pass before moving to rendering.

### Task 3: Implement atomic deterministic Markdown rendering

**Files:**
- Modify: `src/features/weekly_radar/interface/telegram_renderer.rs`
- Modify: `src/features/weekly_radar/interface/telegram_renderer_test.rs`
- Modify: `src/features/weekly_radar/interface/mod.rs`

**Interfaces:**
- Consumes validated `TelegramSummaryInput`.
- Produces `TelegramRenderLimits`, `TelegramRenderer::render`, and immutable `TelegramMessage`.

- [ ] **Step 1: Add caller-supplied limit validation.**

  Define `TelegramRenderLimits::new(max_characters, max_lines, max_items_per_section, max_company_cards)` and reject zero values with `InvalidLimit`. Validate each change section length against `max_items_per_section` and the combined Top5/Threshold Distance/Rising/Dropped card count against `max_company_cards` before assembling output.

- [ ] **Step 2: Add fixed section block assembly.**

  Define `TelegramRenderer::render(input, limits)` to build complete blocks in the exact priority order. Use a header containing the supplied period, section headings, `- ` prefixes for summary items, and `- **company** — card` prefixes for company cards. Include System Health when supplied and No Change only when explicitly supplied. Never inspect a fragment to derive domain meaning, and append each fragment in full.

- [ ] **Step 3: Add final character and line checks.**

  Join complete blocks with blank lines, measure Unicode scalar characters and lines, and return `MessageTooLong` or `LineLimitExceeded` when limits are exceeded. Return `TelegramMessage { markdown, character_count, line_count, company_card_count }` only after all checks pass. Do not return a partial buffer on failure.

- [ ] **Step 4: Register the public interface module.**

  Add `pub mod telegram_renderer;` to `src/features/weekly_radar/interface/mod.rs` and keep the module free of imports from other features, providers, network clients, or runtime configuration.

- [ ] **Step 5: Run focused tests and refactor only after GREEN.**

  Run `cargo test --test weekly_radar_telegram_renderer` and `cargo test --test telegram_renderer_test`. Refactor duplicated block/measurement helpers only while both targets remain green.

### Task 4: Record reference impact and run project verification

**Files:**
- Create: `.ai/evidence/reference-impact/wi-wr-009-telegram-renderer.json`
- Update: `.ai/work-items/active/wi-wr-009.summary.json`
- Do not modify: architecture tests or `.ai/guards/coverage_policy.yaml`.

**Interfaces:**
- Consumes the renderer implementation and focused tests.
- Produces machine-readable boundary evidence and verified Summary entries.

- [ ] **Step 1: Add reference-impact evidence.**

  Record the renderer and test paths, the unchanged architecture-test and global-coverage-policy paths, the no-cross-feature-import result, the no-provider-integration result, and the fact that WR-007 is consumed semantically through explicit view facts rather than recalculated.

- [ ] **Step 2: Run focused, module-local, integration, and project checks.**

  Run:

  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --test weekly_radar_telegram_renderer
  cargo test --test telegram_renderer_test
  cargo test --all
  make check
  make check-ai-reference-impact CONTRACT=.ai/work-items/active/wi-wr-009.contract.json
  make check-ai-coverage-guard
  ```

  Record exact command results and output digests in the Summary; do not claim scenario verification before the commands pass.

- [ ] **Step 3: Update Summary for Finish.**

  Replace skeleton placeholders with changed files, scenario evidence, guideline compliance, residual risks, known gaps, boundary checks, no destructive changes, and the local-only next action. Mark the four Contract scenarios verified only with matching test evidence.

### Task 5: Finish, archive, and commit locally

**Files:**
- Modify: `.ai/work-items/active/wi-wr-009.summary.json`
- Generated/archived by Make targets: `.ai/cockpit/*`, `.ai/work-items/archive/**/wi-wr-009.*`, `.ai/work-items/active/wi-wr-009.outcome.*`

**Interfaces:**
- Consumes verified implementation, active Contract, Summary, and reference-impact evidence.
- Produces strict AI Cockpit Finish evidence, archived Work Item evidence, and one local commit; it does not push, open a PR, merge, close, or delete a remote branch.

- [ ] **Step 1: Run the before-edit checkpoint if not already recorded.**

  Run `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-wr-009.contract.json SUMMARY=.ai/work-items/active/wi-wr-009.summary.json STAGE=before_edit` before production edits, then preserve its evidence in the Summary.

- [ ] **Step 2: Run the canonical before-finish checkpoint.**

  Run `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-wr-009.contract.json SUMMARY=.ai/work-items/active/wi-wr-009.summary.json STAGE=before_finish` after all verification and Summary updates.

- [ ] **Step 3: Run strict AI Cockpit Finish.**

  Run `make ai-finish TASK=wi-wr-009 REPORT_LANGUAGE=zh-CN`. Treat any failed gate as a same-WI correction inside the declared scope.

- [ ] **Step 4: Archive the finished Work Item.**

  Run `make archive-work-item TASK=wi-wr-009` and inspect the generated archive manifest, outcome, and current status. Do not run `ai-close-work-item`; this request explicitly stops before close.

- [ ] **Step 5: Verify ownership and create the local commit.**

  Inspect `git status --short`, `git diff --check`, the Contract scope/diff-ownership report, archived evidence, and the final test/Finish output. Stage only WI-WR-009 files and commit with:

  ```bash
  git add .ai docs/superpowers/specs/2026-08-17-wi-wr-009-telegram-renderer.md docs/superpowers/plans/2026-08-17-wi-wr-009-telegram-renderer.md src/features/weekly_radar/interface/mod.rs src/features/weekly_radar/interface/telegram_renderer.rs src/features/weekly_radar/interface/telegram_renderer_test.rs tests/weekly_radar_telegram_renderer.rs tests/telegram_renderer_test.rs
  git commit -m "feat: add Telegram weekly radar renderer"
  ```

  Confirm the commit SHA, branch, and worktree path. Leave the branch/worktree intact for the user's later provider lifecycle.
