# WI-WR-004 Stage Transition Detection Output Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build and verify a standalone typed output boundary for explicit Weekly Radar Stage Transition facts.

**Architecture:** Keep the implementation in one exclusive source file under the Weekly Radar Domain path without editing shared module registration. Load it from an integration test with `#[path]`; use opaque Stage labels and explicit status values so the module organizes upstream facts without importing Transformation or Evidence internals.

**Tech Stack:** Rust standard library, Cargo tests, repository Make/AI Cockpit gates, JSON governance evidence, and Markdown design documentation.

## Global Constraints

- Write only the exclusive WI-WR-004 paths in the Contract.
- Preserve event identity, company, from/to stage, date, status, supporting, counter, missing, and confidence.
- Mark only explicit `PRODUCTION_SYSTEM → PRODUCTIVITY_BREAKOUT` as `ProductivityBreakoutHigh`.
- Do not infer from raw Evidence, mutate Stage, compare Snapshots, rank, score, persist, render, send, schedule, retry, or add dependencies.
- Resolve in-scope implementation, test, documentation, and governance issues in this WI; do not create a new WI for them.

---

### Task 1: Establish governance evidence and design

**Files:**
- Modify: `.ai/work-items/active/wi-wr-004.contract.json`
- Modify: `.ai/work-items/active/wi-wr-004.summary.json`
- Create: `docs/superpowers/specs/2026-08-17-wi-wr-004-stage-transition-output.md`
- Create: `docs/superpowers/plans/2026-08-17-wi-wr-004-stage-transition-output.md`
- Create: `.ai/evidence/reference-impact/wi-wr-004-stage-transition-output.json`

**Interfaces:**
- Consumes: roadmap, Stage model, Evidence model, Weekly Radar boundary, and user authorization.
- Produces: bounded Contract/Summary, explicit acceptance/scenarios, and a reference-impact record proving shared module paths remain untouched.

- [x] **Step 1: Record the exact authorization and current-WI issue policy**

  Contract records the exact authorization sentence `完成24 个WI，需要我授权的，授权给你并请写入Contract。`, its digest and capture time, plus the rule to resolve in-scope issues in this WI.

- [x] **Step 2: Run Preflight and require ready evidence**

  Run `make ai-preflight CONTRACT=.ai/work-items/active/wi-wr-004.contract.json` after Contract completion. Do not write production code while the report is `not_ready` or `needs_human_confirmation`.

### Task 2: Define the failing output-boundary tests

**Files:**
- Create: `tests/weekly_radar_stage_transition.rs`
- Create: `src/features/weekly_radar/domain/stage_transition_output.rs`

**Interfaces:**
- Consumes: no shared module import; the integration test loads the exclusive source with `#[path = "../src/features/weekly_radar/domain/stage_transition_output.rs"]`.
- Produces: test-defined API for `StageTransitionOutput`, `TransitionStatus`, `TransitionPriority`, and ordered evidence references.

- [ ] **Step 1: Write tests for explicit status and all supplied fields**

  Construct one `Confirmed` and one `Candidate` output and assert event/company/from/to/date/status/supporting/counter/missing/confidence accessors return the supplied values unchanged.

- [ ] **Step 2: Run the focused test and verify the feature-missing failure**

  Run `cargo test --test weekly_radar_stage_transition`. Expected result: compilation fails because the standalone module and its public types do not yet exist.

- [ ] **Step 3: Add tests for priority, ordering, duplicates, blanks, and downgrade**

  Assert the explicit Productivity Breakout pair is high priority, other pairs are normal, corrective downgrade facts are retained, ordered collections stay ordered, and invalid blank/duplicate/overlapping references return deterministic errors.

### Task 3: Implement the minimal standalone domain boundary

**Files:**
- Modify: `src/features/weekly_radar/domain/stage_transition_output.rs`

**Interfaces:**
- Consumes: owned values passed by callers; no other feature module.
- Produces: documented public value objects, `StageTransitionOutput::new`, evidence collection append methods, accessors, and pure priority mapping.

- [ ] **Step 1: Add documented opaque text values and enums**

  Implement non-empty wrappers for event/company/stage/date/evidence/confidence text, `TransitionStatus::{Confirmed,Candidate}`, and `TransitionPriority::{Normal,ProductivityBreakoutHigh}`.

- [ ] **Step 2: Add ordered evidence collections with deterministic validation**

  Store supporting/counter/missing vectors. Reject blank values at value construction, reject duplicate event identities at the output collection boundary, and reject any evidence identity overlap across collections.

- [ ] **Step 3: Add output construction and accessor behavior**

  Store all supplied fields unchanged. Expose `priority()` as a pure explicit-label mapping for `PRODUCTION_SYSTEM → PRODUCTIVITY_BREAKOUT`; do not inspect evidence or historical state.

- [ ] **Step 4: Run the focused tests and refactor only after green**

  Run `cargo test --test weekly_radar_stage_transition`; preserve green tests while improving names or documentation without adding behavior.

### Task 4: Complete governance verification and local commit

**Files:**
- Modify: `.ai/work-items/active/wi-wr-004.summary.json`
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`, `.ai/work-items/active/wi-wr-004.outcome.json`, `.ai/work-items/active/wi-wr-004.outcome.md`
- Archive: `.ai/work-items/archive/**`

**Interfaces:**
- Consumes: implementation, tests, Contract, Summary, and local command output.
- Produces: archived Work Item evidence and one local commit; no push, PR, merge, or close.

- [ ] **Step 1: Run focused and full checks**

  Run `cargo test --test weekly_radar_stage_transition`, `make check`, and `git diff --check`. Confirm no forbidden shared path changed and `Cargo.toml` is unchanged.

- [ ] **Step 2: Record checkpoint and run Finish in Chinese locale**

  Run `make ai-checkpoint TASK=wi-wr-004 STAGE=before_finish CONTRACT=.ai/work-items/active/wi-wr-004.contract.json SUMMARY=.ai/work-items/active/wi-wr-004.summary.json`, then `make ai-finish TASK=wi-wr-004 REPORT_LANGUAGE=zh-CN`.

- [ ] **Step 3: Deliver the active Outcome before archive**

  Report completed facts, problem totals, warnings, resolved issues and evidence, avoided risks, remaining risks, unknowns, human decisions, verification, impact, and next action in the conversation. Do not claim external publication or closure.

- [ ] **Step 4: Archive and commit only this Work Item**

  Run `make archive-work-item TASK=wi-wr-004`, rerun `make check` and `git diff --check`, stage only declared paths, and commit with `feat: add stage transition output`.

