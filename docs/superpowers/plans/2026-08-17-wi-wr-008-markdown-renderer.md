# WI-WR-008 Markdown Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Register the delivered Weekly Radar read models and build a deterministic archival Markdown renderer that preserves Top5, Research Cards, evidence/proof fields, change compression, explicit Stage History, Rank Changes, and System Health.

**Architecture:** The existing Top5 Weekly Read Model and Weekly Change Compression remain fact-owning domain modules and become publicly registered under Weekly Radar Domain. The renderer lives under Weekly Radar Interface and consumes borrowed `WeeklyRadarSnapshot`, `Top5WeeklyReadModel`, `ResearchPacket`, `WeeklyChangeCompression`, explicit ordered Stage History/Rank Changes records, and optional `SystemHealth`; it only formats supplied facts into one in-memory `MarkdownDocument`.

**Tech Stack:** Rust 2021, Rust standard library, Cargo unit/integration tests, existing Weekly Radar and Reporting read models, AI Cockpit Make targets, Markdown governance documents, and JSON reference-impact evidence. No new dependency.

## Global Constraints

- "Register the existing Top5 Weekly Read Model and Weekly Change Compression modules in the Weekly Radar Domain module tree, and register the Markdown Renderer in the Weekly Radar Interface layer; do not alter their supplied fact semantics."
- "The renderer consumes explicit Weekly Radar read models and compression/system-health facts plus explicit Stage History and Rank Changes records. It preserves input order and values and never calculates Stage, Ranking, Distance, score, Top5 membership, section membership, or change meaning."
- "The report is an in-memory deterministic Markdown document only. Do not add persistence, HTTP, Telegram, Scheduler, retry, publication, access-material transport, network, database, or external-provider behavior."
- "Only this Work Item's implementation, focused tests, design/plan, reference-impact record, Contract/Summary, and generated governance/archive evidence may change. Shared coverage policy, global architecture tests, unrelated Work Items, and unrelated project policy remain untouched."
- "The local lifecycle ends after strict AI Finish, Archive, and a local commit. Do not push, open a PR, merge, or close the Work Item."
- Preserve the existing `Top5WeeklyReadModel`, `WeeklyChangeCompression`, `SystemHealth`, and `ResearchPacket` APIs and facts; this Work Item only registers and consumes them.
- Every new public Rust API has doc comments and no external crate import.

---

## File map

- Modify `src/features/weekly_radar/domain/mod.rs` to register `change_compression` and `top5_weekly_read_model` without editing either source.
- Modify `src/features/weekly_radar/interface/mod.rs` to register `markdown_renderer`.
- Create `src/features/weekly_radar/interface/markdown_renderer.rs` with the renderer, explicit Stage History/Rank Changes records, report input, Markdown document, typed constructor error, and fixed-order formatting helpers.
- Create `src/features/weekly_radar/interface/markdown_renderer_test.rs` with module-local tests loaded from the renderer source.
- Create `tests/weekly_radar_markdown_renderer.rs` with public-module integration tests for full report order, fact/order retention, empty states, determinism, and source boundary checks.
- Create `tests/markdown_renderer_test.rs` as the same-stem test companion so coverage association remains local to this Work Item.
- Create `.ai/evidence/reference-impact/wi-wr-008-markdown-renderer.json` to bind the registered composition paths and unchanged shared policy paths.
- Update `.ai/work-items/active/wi-wr-008.summary.json` with implementation and verification evidence before Finish.
- Create `docs/superpowers/specs/2026-08-17-wi-wr-008-markdown-renderer.md` and this plan; the spec is already committed in design checkpoint `49d17aa`.
- Generated lifecycle files remain within the Contract scope: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.*`, `.ai/work-items/active/wi-wr-008.outcome.*`, and `.ai/work-items/archive/**/wi-wr-008.*`.

## Public interfaces used by every implementation task

The renderer source must expose these exact signatures:

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarkdownRenderError {
    EmptyValue {
        entity: &'static str,
        field: &'static str,
    },
}

impl StageHistoryEntry {
    pub fn new(
        id: impl Into<String>,
        period: impl Into<String>,
        company: impl Into<String>,
        previous_stage: impl Into<String>,
        current_stage: impl Into<String>,
        fact: impl Into<String>,
    ) -> Result<Self, MarkdownRenderError>;
}

impl RankChange {
    pub fn new(
        id: impl Into<String>,
        period: impl Into<String>,
        company: impl Into<String>,
        previous_rank: Option<u32>,
        current_rank: Option<u32>,
        fact: impl Into<String>,
    ) -> Result<Self, MarkdownRenderError>;
}

impl<'a> MarkdownReportInput<'a> {
    pub fn new(
        snapshot: &'a WeeklyRadarSnapshot,
        top5: &'a Top5WeeklyReadModel,
        research: &'a ResearchPacket,
        compression: &'a WeeklyChangeCompression,
        stage_history: &'a [StageHistoryEntry],
        rank_changes: &'a [RankChange],
        system_health: Option<&'a SystemHealth>,
    ) -> Self;
}

impl MarkdownRenderer {
    pub fn render(input: &MarkdownReportInput<'_>) -> MarkdownDocument;
}

impl MarkdownDocument {
    pub fn as_str(&self) -> &str;
}
```

The renderer must import `ResearchPacket` only as a read-only Reporting domain
input. It must import the registered Weekly Radar modules through their public
paths and must not duplicate their types.

### Task 1: Write the public integration RED tests

**Files:**
- Create: `tests/weekly_radar_markdown_renderer.rs`
- Create: `tests/markdown_renderer_test.rs`

**Interfaces:**
- Consumes: The approved public signatures above and existing constructors in `WeeklyRadarSnapshot`, `Top5WeeklyReadModel`, `WeeklyChangeCompression`, `ResearchPacket`, and `SystemHealth`.
- Produces: Failing tests that prove the missing public module registration and renderer behavior before production code exists.

- [ ] **Step 1: Add fixture imports and constructors.**

Use the public paths that must exist after implementation:

```rust
use org_x::features::reporting::domain::{
    ReportSection, ResearchCard, ResearchPacket, Top5 as ResearchTop5,
};
use org_x::features::weekly_radar::domain::change_compression::{
    CompanyReference as ChangeCompany, EventId, FactValue as ChangeFact,
    ImportantStructuralChange, PeriodId, WeeklyChangeCompression, WeeklyChangeInput,
};
use org_x::features::weekly_radar::domain::system_health::{
    DegradedCompany, EvidenceCoverage, ExtractionFailure, FailureId, Freshness,
    HealthStatus, Reason, SourceCoverage, SourceReference, SystemHealth,
};
use org_x::features::weekly_radar::domain::top5_weekly_read_model::{
    CandidateId, Company, Confidence, Direction, KeyChange, NextStep, Stage,
    Top5Entry, Top5WeeklyReadModel,
};
use org_x::features::weekly_radar::interface::markdown_renderer::{
    MarkdownReportInput, MarkdownRenderer, RankChange, StageHistoryEntry,
};
```

Build a snapshot with id `snapshot-renderer`, an ordered Top5 containing
`candidate-2` then `candidate-1`, a Research Packet with one card in each of
Top5/Rising/Watch/Dropped, two Stage History entries ordered `history-2` then
`history-1`, and two Rank Changes ordered `rank-2` then `rank-1`. Use opaque
values containing their field names, such as `evidence-card-2`, so a dropped
field cannot pass unnoticed.

Build non-empty compression input in the existing fixed order with one event
in each of Important Structural Change, Top5 Change, Stage Transition, Rising,
and Dropped. Build System Health with two source coverage records in supplied
order, one degraded company, and one extraction failure.

- [ ] **Step 2: Add the fixed-order and fact-retention test.**

```rust
#[test]
fn full_report_preserves_supplied_facts_and_fixed_section_order() {
    let fixtures = full_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &fixtures.stage_history,
        &fixtures.rank_changes,
        Some(&fixtures.system_health),
    );

    let markdown = MarkdownRenderer::render(&input).as_str().to_owned();
    let markers = [
        "# Weekly Radar Markdown Report",
        "## Snapshot",
        "## Change Compression",
        "## Top5",
        "## Research Cards",
        "## Evidence",
        "## Counter Evidence",
        "## Missing Proof",
        "## Stage History",
        "## Rank Changes",
        "## System Health",
    ];
    let positions: Vec<_> = markers
        .iter()
        .map(|marker| markdown.find(marker).expect("section marker must exist"))
        .collect();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(markdown.contains("candidate-2"));
    assert!(markdown.find("candidate-2").unwrap() < markdown.find("candidate-1").unwrap());
    assert!(markdown.contains("evidence-card-2"));
    assert!(markdown.find("history-2").unwrap() < markdown.find("history-1").unwrap());
    assert!(markdown.find("rank-2").unwrap() < markdown.find("rank-1").unwrap());
    assert!(markdown.contains("source-2"));
    assert!(markdown.contains("counter-card-1"));
    assert!(markdown.contains("missing-card-1"));
}
```

- [ ] **Step 3: Add empty/no-change and deterministic rendering tests.**

Use an empty `WeeklyChangeInput`, empty Stage History/Rank Changes, and
`system_health: None`:

```rust
#[test]
fn empty_report_keeps_no_change_and_absent_health_explicit() {
    let fixtures = empty_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &[],
        &[],
        None,
    );
    let markdown = MarkdownRenderer::render(&input).as_str().to_owned();

    assert!(markdown.contains("NO_CHANGE"));
    assert!(markdown.contains("Important Structural Change Count: 0"));
    assert!(markdown.contains("Top5 Change Count: 0"));
    assert!(markdown.contains("Stage Transition Count: 0"));
    assert!(markdown.contains("Rising Count: 0"));
    assert!(markdown.contains("Dropped Count: 0"));
    assert!(markdown.contains("Stage History: EMPTY"));
    assert!(markdown.contains("Rank Changes: EMPTY"));
    assert!(markdown.contains("NOT_SUPPLIED"));
}

#[test]
fn rendering_the_same_input_twice_is_byte_identical() {
    let fixtures = full_fixtures();
    let input = MarkdownReportInput::new(
        &fixtures.snapshot,
        &fixtures.top5,
        &fixtures.research,
        &fixtures.compression,
        &fixtures.stage_history,
        &fixtures.rank_changes,
        Some(&fixtures.system_health),
    );
    let first = MarkdownRenderer::render(&input);
    let second = MarkdownRenderer::render(&input);

    assert_eq!(first.as_str(), second.as_str());
}
```

- [ ] **Step 4: Add source and registration boundary assertions.**

```rust
#[test]
fn renderer_source_has_no_recomputation_or_external_delivery_boundary() {
    let source = include_str!("../src/features/weekly_radar/interface/markdown_renderer.rs");
    let lowered = source.to_ascii_lowercase();
    for forbidden in [
        "sort_by", "sort_unstable", "rank_by", "calculate_stage",
        "calculate_rank", "calculate_distance", "calculate_score", "telegram",
        "http", "reqwest", "sqlx", "std::net", "std::fs", "credential",
    ] {
        assert!(!lowered.contains(forbidden), "forbidden token: {forbidden}");
    }
}
```

- [ ] **Step 5: Add the same-stem coverage companion.**

Create `tests/markdown_renderer_test.rs` with:

```rust
#[path = "weekly_radar_markdown_renderer.rs"]
mod weekly_radar_markdown_renderer;
```

- [ ] **Step 6: Run the focused tests and verify RED.**

Run:

```bash
cargo test --test weekly_radar_markdown_renderer
```

Expected result: compilation fails because `domain::change_compression`,
`domain::top5_weekly_read_model`, and
`interface::markdown_renderer` are not registered yet. The failure must be an
unresolved public module/API failure, not a test assertion typo.

### Task 2: Implement registrations, explicit records, and report input

**Files:**
- Modify: `src/features/weekly_radar/domain/mod.rs`
- Modify: `src/features/weekly_radar/interface/mod.rs`
- Create: `src/features/weekly_radar/interface/markdown_renderer.rs`
- Create: `src/features/weekly_radar/interface/markdown_renderer_test.rs`

**Interfaces:**
- Consumes: Task 1 failing public integration tests and the registered upstream read models.
- Produces: The exact public types and constructors listed in the Public interfaces section.

- [ ] **Step 1: Add module-local tests before implementation details.**

Create `src/features/weekly_radar/interface/markdown_renderer_test.rs` with
tests that will fail until the new types exist:

```rust
use super::{MarkdownRenderError, RankChange, StageHistoryEntry};

#[test]
fn stage_history_rejects_blank_required_values() {
    assert_eq!(
        StageHistoryEntry::new(" ", "2026-W33", "Acme", "WORKFLOW", "PRODUCTION", "fact"),
        Err(MarkdownRenderError::EmptyValue {
            entity: "stage history",
            field: "id",
        })
    );
}

#[test]
fn rank_change_retains_optional_positions_and_fact() {
    let change = RankChange::new(
        "rank-1", "2026-W33", "Acme", Some(4), None, "left supplied Top5",
    )
    .expect("rank change should validate");

    assert_eq!(change.previous_rank(), Some(4));
    assert_eq!(change.current_rank(), None);
    assert_eq!(change.fact(), "left supplied Top5");
}
```

Add `#[cfg(test)] #[path = "markdown_renderer_test.rs"] mod module_tests;`
to the renderer source after its public type declarations.

- [ ] **Step 2: Register the existing domain modules.**

Add only these public module declarations to the existing module files:

```rust
// src/features/weekly_radar/domain/mod.rs
pub mod change_compression;
pub mod system_health;
pub mod top5_weekly_read_model;

// src/features/weekly_radar/interface/mod.rs
pub mod markdown_renderer;
```

Do not edit the existing `change_compression.rs`, `top5_weekly_read_model.rs`,
or `system_health.rs` bodies.

- [ ] **Step 3: Add typed validation and explicit ordered records.**

Implement `MarkdownRenderError`, a private `non_empty` helper, and the two
records as private fields. Validate in constructor order and return the first
blank field without trimming accepted values. Accessors return the exact
stored strings; rank accessors return the exact `Option<u32>` values.

The validation shape is:

```rust
fn non_empty(
    entity: &'static str,
    field: &'static str,
    value: impl Into<String>,
) -> Result<String, MarkdownRenderError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(MarkdownRenderError::EmptyValue { entity, field });
    }
    Ok(value)
}
```

- [ ] **Step 4: Add borrowed report input and document access.**

Define `MarkdownReportInput<'a>` with the seven borrowed fields from the
Public interfaces section, `MarkdownReportInput::new`, and accessors only if
the renderer helpers need them. Define `MarkdownDocument(String)` with
`as_str(&self) -> &str`. Keep both types immutable after construction.

- [ ] **Step 5: Run module-local and integration tests to verify the first GREEN cycle.**

Run:

```bash
cargo test --test weekly_radar_markdown_renderer
cargo test --lib features::weekly_radar::interface::markdown_renderer
```

Expected result: the module registration, constructor validation, and input
type compile; the focused integration assertions may still fail until the
formatter from Task 3 is present. If a failure is a compile/type error, fix
the production signature rather than weakening the test.

### Task 3: Implement fixed-order Markdown rendering

**Files:**
- Modify: `src/features/weekly_radar/interface/markdown_renderer.rs`
- Modify: `src/features/weekly_radar/interface/markdown_renderer_test.rs`
- Modify: `tests/weekly_radar_markdown_renderer.rs`

**Interfaces:**
- Consumes: `MarkdownReportInput<'_>` and all registered upstream accessors.
- Produces: `MarkdownRenderer::render(&MarkdownReportInput<'_>) -> MarkdownDocument` with all declared sections.

- [ ] **Step 1: Add module-local empty-state assertions.**

Extend the module-local tests with a minimal input and assert that rendering
contains `NO_CHANGE`, the five upstream zero-count labels, `Stage History:
EMPTY`, `Rank Changes: EMPTY`, and `NOT_SUPPLIED` for absent health. This test
must be added before writing the formatter body.

- [ ] **Step 2: Add deterministic line and field helpers.**

Use only `String::push_str`/`push` or `std::fmt::Write` to append lines. The
helpers must iterate borrowed slices directly and must not call sorting,
ranking, comparison, de-duplication, or calculation functions. Use these
stable labels:

```rust
const EMPTY: &str = "EMPTY";
const NOT_SUPPLIED: &str = "NOT_SUPPLIED";
const NOT_EMITTED: &str = "NOT_EMITTED";

fn push_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn push_field(output: &mut String, label: &str, value: &str) {
    output.push_str("- ");
    output.push_str(label);
    output.push_str(": ");
    output.push_str(value);
    output.push('\n');
}
```

Formatting may map already-supplied enum variants to stable uppercase labels,
but it must not derive one field from another.

- [ ] **Step 3: Render snapshot and compression in fixed upstream order.**

Render these markers in order: `# Weekly Radar Markdown Report`, `## Snapshot`,
then `## Change Compression`. Snapshot must print id, as-of, universe
snapshot id, evidence cutoff, model version, and scoring version.

For compression, iterate `important_structural()`, `top5()`,
`stage_transitions()`, `rising()`, and `dropped()` in that exact order. For
each event print event id, period, company, and fact. For `no_change()` print
the supplied `NO_CHANGE`, period, and five supplied counts; when it is `None`
print `No Change: NOT_EMITTED` without fabricating counts.

- [ ] **Step 4: Render Top5 and Research Card sections without reclassification.**

Render `## Top5` from `Top5WeeklyReadModel::entries()` in supplied order and
print candidate, company, stage, direction, confidence, key change, and next.
Render `## Research Cards` from the supplied packet executive summary and
its Top5/Rising/Watch/Dropped sections in that fixed packet order. For every
card print id, company, stage, headline, next step, and the section label
already supplied by the packet.

- [ ] **Step 5: Render Evidence, Counter Evidence, and Missing Proof.**

Visit the same packet section order and card order three times. The Evidence
section prints each card's `evidence()`, Counter Evidence prints
`counter_evidence()`, and Missing Proof prints `missing_proof()`. Do not
deduplicate cards across packet sections and do not infer a section from any
card field.

- [ ] **Step 6: Render explicit Stage History, Rank Changes, and System Health.**

Render Stage History and Rank Changes in supplied slice order. Print both rank
positions independently; map `None` to the literal `NOT_SUPPLIED` and do not
compute rank delta or direction. For System Health, map the explicit enum
variants to uppercase labels, print aggregate/source coverage values exactly,
then iterate degraded companies, source coverage, and extraction failures in
their stored order. With `None`, print one `NOT_SUPPLIED` health marker and no
derived status.

- [ ] **Step 7: Run the focused GREEN tests.**

Run:

```bash
cargo fmt --all -- --check
cargo test --test weekly_radar_markdown_renderer
cargo test --test markdown_renderer_test
cargo test --lib features::weekly_radar::interface::markdown_renderer
```

Expected result: all focused tests pass, including the same-stem companion,
without warnings or source-boundary assertion failures.

### Task 4: Record reference impact and verify project quality

**Files:**
- Create: `.ai/evidence/reference-impact/wi-wr-008-markdown-renderer.json`
- Modify: `.ai/work-items/active/wi-wr-008.summary.json`
- Do not modify: `.ai/guards/coverage_policy.yaml`, global architecture tests, or upstream read-model source files.

**Interfaces:**
- Consumes: Focused renderer implementation and test evidence.
- Produces: Reference-impact evidence, Summary scenario results, guideline evidence, and quality command results.

- [ ] **Step 1: Write reference-impact evidence.**

Record that `domain/mod.rs` and `interface/mod.rs` are the only module
registration changes; the existing Top5, Compression, System Health, and
Reporting read-model sources are consumed unchanged; no shared coverage or
architecture policy is modified; and the renderer imports only standard
library plus read-only domain types.

- [ ] **Step 2: Run project format, lint, and all tests.**

Run the complete project checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

Also run architecture tests explicitly:

```bash
cargo test --test dependency_rules
cargo test --test module_boundaries
```

- [ ] **Step 3: Update Summary with exact evidence.**

Set `changedFiles` to the actual implementation, tests, docs, reference-impact,
Contract/Summary, and generated paths. Mark all five Contract scenarios
`verified` with command evidence. Record every required check command and
result, `guidelinesCompliance`, `boundaryChecks`, residual fact-order risk,
zero destructive changes, and local-only next action. Keep `unknownsRemaining`
empty and set `reviewReadiness.status` to `ready_for_review` only after all
required checks have fresh evidence.

### Task 5: Strict Finish, Archive, and local commit

**Files:**
- Modify: `.ai/work-items/active/wi-wr-008.summary.json`
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.*`, `.ai/work-items/active/wi-wr-008.outcome.*`, `.ai/work-items/archive/**/wi-wr-008.*`

**Interfaces:**
- Consumes: Verified implementation, reference-impact record, Contract, and Summary.
- Produces: Strict Finish evidence, archived Work Item bundle, and one or more local commits. No provider-side lifecycle action is allowed.

- [ ] **Step 1: Run the canonical before-finish checkpoint.**

Run:

```bash
make ai-checkpoint \
  CONTRACT=.ai/work-items/active/wi-wr-008.contract.json \
  SUMMARY=.ai/work-items/active/wi-wr-008.summary.json \
  STAGE=before_finish
```

- [ ] **Step 2: Run strict AI Cockpit Finish.**

Run:

```bash
make ai-finish TASK=wi-wr-008 REPORT_LANGUAGE=zh-CN
```

Treat every failed gate as an in-scope correction. If coverage association
fails, add tests under `src/features/weekly_radar/interface/markdown_renderer_test.rs`
or the two declared renderer integration test files and rerun the affected
checks; do not edit `.ai/guards/coverage_policy.yaml`.

- [ ] **Step 3: Archive the finished Work Item.**

Run:

```bash
make archive-work-item TASK=wi-wr-008
```

Verify the archive manifest binds the exact Contract, Summary, Outcome, and
archive sequence for `wi-wr-008`.

- [ ] **Step 4: Verify local ownership and final repository state.**

Run:

```bash
make check-ai-diff-ownership CONTRACT=.ai/work-items/active/wi-wr-008.contract.json
git diff --check
git status --short --branch
git diff --name-only origin/main...HEAD
```

After Archive, use the archived Contract/Summary paths required by the current
Make target if `check-ai-diff-ownership` reports the active pair no longer
exists. Do not run `make check-ai-pr` as a claim of PR readiness unless the
local archive transaction is committed and the target accepts the local-only
state. Do not push, create a PR, merge, or close.

- [ ] **Step 5: Commit the complete local bundle.**

After fresh verification shows no out-of-scope files, stage the full governed
bundle and commit:

```bash
git add .ai docs/superpowers/specs/2026-08-17-wi-wr-008-markdown-renderer.md docs/superpowers/plans/2026-08-17-wi-wr-008-markdown-renderer.md src/features/weekly_radar/domain/mod.rs src/features/weekly_radar/interface/mod.rs src/features/weekly_radar/interface/markdown_renderer.rs src/features/weekly_radar/interface/markdown_renderer_test.rs tests/weekly_radar_markdown_renderer.rs tests/markdown_renderer_test.rs
git commit -m "feat: add deterministic weekly radar markdown renderer"
```

Confirm the final commit SHA and retain the dedicated branch/worktree for the
user's later provider lifecycle. The final handoff must state completed work,
problem totals, blocking problems/warnings, stops and resolutions, resolved
problems with evidence, avoided and remaining risks, unknowns, human decision,
verification, impact, and next action. It must explicitly state that push,
PR, merge, and close were not performed.

## Plan self-review

- Spec coverage: Tasks 1–3 cover fixed section order, all required report
  sections, exact input order/value retention, explicit no-change/empty states,
  deterministic repeat rendering, Stage History, Rank Changes, and optional
  System Health. Task 4 covers reference impact, coverage-policy preservation,
  and project quality. Task 5 covers checkpoint, strict Finish, Archive, and
  local-only commit.
- Placeholder scan: no `TODO`, `TBD`, `TBC`, vague "appropriate" step, or
  unspecified test step is present; each verification command and expected
  result is named.
- Type consistency: the signatures in the Public interfaces section match the
  Task 1 fixtures, Task 2 constructors/input, and Task 3 renderer calls.
- Scope check: only one bounded interface composition boundary is implemented;
  persistence and delivery remain explicit future Work Items.
