# Weekly Radar Evidence Dimension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add explicit Organization, Workflow, ProductionSystem, and OperatingMetric dimensions to validated structural evidence without changing judgment or Ranking behavior.

**Architecture:** Keep the existing `SourceObservation → EvidenceCandidate → ValidatedEvidence` pipeline. Add a provider-neutral optional dimension to `NormalizedFact`; classify validated passages with a fixed, ordered signal table; render dimension-specific labels while retaining old normalized kinds and JSON compatibility.

**Tech Stack:** Rust stable, serde, chrono, Cargo tests, Markdown report renderer, AI Cockpit Make gates.

**Spec:** `docs/superpowers/specs/2026-08-25-weekly-radar-evidence-dimension-design.md`

## Global Constraints

- Keep evidence classification deterministic, provider-neutral, bounded, and rule-only.
- Prefer false negatives to promoting technical prose as structural enterprise change.
- Preserve legacy snapshot deserialization by defaulting new fields to zero or `None`.
- Do not change judgment, Ranking, Telegram, archive, or workflow behavior outside the structural evidence dimension boundary.
- Do not import internal modules from other features.

---

### Task 1: Add the backward-compatible structural dimension model

**Files:**
- Modify: `src/features/weekly_radar/runtime/model.rs`
- Modify: `src/features/weekly_radar/runtime.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Produces `StructuralDimension::{Organization, Workflow, ProductionSystem, OperatingMetric}`.
- Produces `NormalizedFact::new_with_structural_dimension(...)` and
  `NormalizedFact::structural_dimension()`.
- Keeps `NormalizedFact::new(...)` and old JSON inputs unchanged.

- [ ] **Step 1: Write the failing compatibility and constructor tests**

Add tests that import `StructuralDimension`, construct a fact with
`new_with_structural_dimension`, assert the getter, and deserialize a legacy
JSON object that has no `structural_dimension` field and returns `None`.

```rust
let fact = NormalizedFact::new_with_structural_dimension(
    "acme",
    "evidence_structural_change_001",
    "Acme reduced serving latency.",
    Some(StructuralDimension::OperatingMetric),
    FactStatus::Known,
    Confidence::High,
    provenance,
).unwrap();
assert_eq!(fact.structural_dimension(), Some(StructuralDimension::OperatingMetric));
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test --test weekly_radar_evidence_quality structural_dimension`

Expected: compilation failure because `StructuralDimension`, the dimension-aware constructor, and the getter do not exist yet.

- [ ] **Step 3: Implement the minimal model and public re-export**

Define the documented serde-compatible enum in `runtime/model.rs`, add an
optional skipped field to `NormalizedFact`, extend the custom wire
deserializer with `#[serde(default)]`, add the dimension-aware constructor and
getter, and re-export the enum from `runtime.rs`. Preserve the existing
constructor by passing `None`.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality structural_dimension`

Expected: PASS, including legacy JSON deserialization.

- [ ] **Step 5: Commit the model boundary**

```bash
git add src/features/weekly_radar/runtime/model.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_evidence_quality.rs
git commit -m "feat: add structural evidence dimensions"
```

### Task 2: Classify validated evidence dimensions with complete claims

**Files:**
- Modify: `src/features/weekly_radar/runtime/evidence.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Produces `ValidatedEvidence::structural_dimension() -> Option<StructuralDimension>`.
- Keeps `ValidatedEvidence::evidence_class()` as `StructuralEvidence` exactly when a dimension exists.
- Keeps `to_normalized_fact()` on the existing `evidence_structural_change_<index>` prefix.

- [ ] **Step 1: Write failing dimension and completeness tests**

Add one test for each dimension, using dated authoritative candidates. Use
fixtures matching the live run: GPU utilization and latency must be
`OperatingMetric`; deployment/platform/storage must be `ProductionSystem`; a
reporting-line or responsibility claim must be `Organization`; an approval or
manual-to-automated process claim must be `Workflow`.

Add a negative test proving generic research prose remains
`EvidenceClass::ValidatedFact`, and a completeness test proving a structural
signal without date, production area, source title, or passage cannot be
promoted.

```rust
let validated = validate_evidence_candidate(
    &complete_candidate("2026-08-19", "model serving").with_source_details(
        "Serving optimization",
        "Acme reduced model serving latency by 68%.",
    ),
    cutoff(),
).unwrap();
assert_eq!(validated.structural_dimension(), Some(StructuralDimension::OperatingMetric));
```

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test --test weekly_radar_evidence_quality structural_dimension`

Expected: the new assertions fail because the current classifier only exposes a boolean structural class and does not distinguish dimensions.

- [ ] **Step 3: Implement the minimal deterministic classifier**

Add fixed signal tables and a private classifier with precedence
`OperatingMetric > ProductionSystem > Workflow > Organization`. Reuse the
existing validation checks as the completeness boundary; compute the optional
dimension only after all required fields and authority checks pass. Use the
dimension to drive `evidence_class()` and pass it to the new normalized-fact
constructor.

- [ ] **Step 4: Run the focused tests and verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality structural_dimension`

Expected: all dimension positives, generic negatives, and incomplete-claim tests pass.

- [ ] **Step 5: Commit the classifier**

```bash
git add src/features/weekly_radar/runtime/evidence.rs tests/weekly_radar_evidence_quality.rs
git commit -m "feat: classify structural evidence dimensions"
```

### Task 3: Render dimension-specific localized report labels

**Files:**
- Modify: `src/features/weekly_radar/runtime/report.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`
- Test: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- `fact_label` consumes a `NormalizedFact` so it can read the optional dimension.
- Existing generic structural kinds without a dimension render a generic structural label.
- Existing report headings, metrics, splitter aliases, Stage, and Ranking output remain unchanged.

- [ ] **Step 1: Write failing localized report assertions**

Construct structural facts for all four dimensions and assert Chinese,
Japanese, and English reports render the corresponding labels. Assert a legacy
`evidence_structural_change_` fact without a dimension does not render as
Organization. Retain the existing assertions that all ten companies produce
no machine Ranking when the gate is not satisfied.

- [ ] **Step 2: Run the focused tests and verify RED**

Run: `cargo test --test weekly_radar_evidence_quality localized_reports && cargo test --test weekly_radar_runtime task8_report`

Expected: the new dimension-label assertions fail, while the existing report tests remain green.

- [ ] **Step 3: Implement labels and fact-aware rendering**

Extend the localized `Labels` table with the four dimension labels and a
generic structural label. Change the item label helper to inspect
`fact.structural_dimension()` and use the generic fallback when it is `None`.
Do not alter section headings or ranking predicates.

- [ ] **Step 4: Run focused runtime and report tests**

Run: `cargo test --test weekly_radar_evidence_quality && cargo test --test weekly_radar_runtime`

Expected: PASS with stable report output and unchanged legacy behavior.

- [ ] **Step 5: Commit the report behavior**

```bash
git add src/features/weekly_radar/runtime/report.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs
git commit -m "feat: render structural evidence dimensions"
```

### Task 4: Align operations documentation and integration regressions

**Files:**
- Modify: `docs/operations/WEEKLY_RADAR.md`
- Modify: `tests/weekly_radar_runtime.rs`
- Modify: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Documents the dimension taxonomy and completeness boundary without changing operator commands.
- Confirms the current `StructuralEvidence` section, legacy aliases, and fail-closed Ranking remain stable.

- [ ] **Step 1: Write documentation/integration regression assertions**

Add a runtime fixture that serializes a dimensioned fact and verifies the
snapshot retains the dimension while a legacy fact still round-trips without
one. Add a report assertion that no dimension metadata creates a Stage or
Ranking entry by itself.

- [ ] **Step 2: Run the integration tests and verify RED**

Run: `cargo test --test weekly_radar_runtime task8 && cargo test --test weekly_radar_evidence_quality report`

Expected: the new snapshot/dimension assertions fail before the final documentation and integration updates.

- [ ] **Step 3: Update the operator guide**

Document `Organization`, `Workflow`, `ProductionSystem`, and `OperatingMetric`,
the precedence rule, the complete-claim fields, the generic legacy fallback,
and the fact that dimensions are evidence metadata rather than Stage/Ranking
proof.

- [ ] **Step 4: Run the full focused Weekly Radar suite**

Run: `cargo test --test weekly_radar_evidence_quality && cargo test --test weekly_radar_runtime && cargo test --test weekly_radar_semantic_message_splitter && cargo test --test semantic_message_splitter_test`

Expected: PASS with no changed splitter or Ranking behavior.

- [ ] **Step 5: Commit documentation and integration coverage**

```bash
git add docs/operations/WEEKLY_RADAR.md tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs
git commit -m "docs: document weekly radar evidence dimensions"
```

### Task 5: Governance verification and lifecycle handoff

**Files:**
- Modify: `.ai/work-items/active/wi-weekly-radar-evidence-dimension.summary.json`
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`, `.ai/work-items/active/wi-weekly-radar-evidence-dimension.outcome.json`, `.ai/work-items/active/wi-weekly-radar-evidence-dimension.outcome.md`

- [ ] **Step 1: Run formatting and project quality**

Run: `cargo fmt --all -- --check` and `make quality`.

Expected: all formatting, lint, unit, integration, architecture, documentation, and test-weakening checks pass.

- [ ] **Step 2: Record verification evidence in Summary**

Update the Summary with every changed path, RED/GREEN TDD evidence, focused
commands, quality output, guideline compliance, residual risks, and the exact
dry-run requirement. Do not claim a live result before the post-merge run.

- [ ] **Step 3: Run the governed finish checks**

Run: `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-evidence-dimension.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-evidence-dimension.summary.json STAGE=before_finish` and then `make ai-finish TASK=wi-weekly-radar-evidence-dimension REPORT_LANGUAGE=zh-CN`.

Expected: Outcome is green; any failed retry remains recorded and is resolved before archive.

- [ ] **Step 4: Archive, commit, and pass the PR gate**

Run: `make archive-work-item CONTRACT=.ai/work-items/active/wi-weekly-radar-evidence-dimension.contract.json`, commit the archive bundle, then run `make check-ai-pr AI_BASE_COMMIT=8d0d38137cd2c0fc04e19767bf298bdc6a5bd212`.

Expected: exactly one archived Work Item is owned by the PR and the knowledge index is valid.

- [ ] **Step 5: Push, merge, close, and trigger one safe dry-run**

Push the dedicated branch, create one PR, wait for all required CI checks,
merge without provider-side branch deletion, and run
`make ai-close-work-item TASK=wi-weekly-radar-evidence-dimension`. From the
synchronized base, trigger exactly one `weekly-radar.yml` dispatch with
`language=zh-CN`, the current `as_of`, `dry_run=true`, and
`republish_published=false`. Record the run URL, conclusion, dimension counts,
SEC/source health, and Ranking behavior.

## Self-review

- Spec coverage: Tasks 1-4 cover the data model, deterministic classifier,
  complete-claim boundary, localized reports, compatibility, and operations
  documentation; Task 5 covers every Contract lifecycle gate.
- Placeholder scan: no TODO, TBD, or unspecified implementation step remains.
- Type consistency: `StructuralDimension` is defined in `runtime/model.rs`,
  re-exported from `runtime.rs`, stored by `NormalizedFact`, returned by
  `ValidatedEvidence`, and consumed by `report.rs` exactly as referenced above.
