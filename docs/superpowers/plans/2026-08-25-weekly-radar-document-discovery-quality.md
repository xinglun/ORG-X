# Weekly Radar Document Discovery and Claim Extraction Quality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Increase the conversion of bounded official document candidates into complete, traceable ValidatedEvidence without weakening fail-closed judgment.

**Architecture:** Extend the existing discovery metadata parser and carry `DocumentKind` through `SourceObservation`. Keep Claim promotion in the existing deterministic EvidenceCandidate gate, and expose only acquisition-quality counters through ResearchMetrics and the localized report.

**Tech Stack:** Rust stable, `chrono`, `regex`, `serde`, existing injected `HttpClient`, Cargo tests, and AI Cockpit Make targets.

**Spec:** `docs/superpowers/specs/2026-08-25-weekly-radar-document-discovery-quality-design.md`

## Global Constraints

- Use deterministic, bounded, provider-neutral, rule-only parsing.
- Do not add providers, LLM/probabilistic extraction, or unbounded crawling.
- Preserve entry-point, discovery-only, incomplete-Claim, and ambiguous-date fail-closed behavior.
- Preserve legacy JSON defaults for `ResearchMetrics`, `SourceObservation`, and existing report snapshots.
- Do not change SEC collection, judgment, Stage, Ranking, Counter Evidence, Telegram, archive, data branch, or workflow behavior.

---

### Task 1: Establish the failing metadata and source-context contract

**Files:**
- Modify: `tests/weekly_radar_evidence_quality.rs`
- Modify: `tests/weekly_radar_runtime.rs`
- Modify: `.ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json`

**Interfaces:**
- Consumes: `document_metadata`, `collect_configured_sources`, `SourceMaterialKind`.
- Produces: failing tests that define metadata precedence and `DocumentKind` propagation.

- [ ] **Step 1: Write the failing tests**

  Add tests named `document_metadata_prefers_published_metadata_over_modified_date`,
  `document_metadata_reads_json_ld_and_iso_datetime`,
  `document_metadata_rejects_malformed_dates`, and
  `discovered_document_retains_document_kind_without_promoting_entry_point`.
  The fixtures must assert the exact `NaiveDate`, `DocumentKind`, and
  `SourceMaterialKind` values.

- [ ] **Step 2: Run the focused tests to verify RED**

  Run:

  ```bash
  cargo test --test weekly_radar_evidence_quality document_metadata_ -- --exact
  cargo test --test weekly_radar_runtime discovered_document_retains_document_kind_without_promoting_entry_point -- --exact
  ```

  Expected: the new tests fail because metadata precedence and source-context
  accessors do not yet exist.

- [ ] **Step 3: Record the RED evidence**

  Add the failing command, failure reason, and test names to the active Summary;
  do not mark any verification check passed.

- [ ] **Step 4: Commit the test contract**

  ```bash
  git add tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs .ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json
  git commit -m "test: define document metadata and context boundaries"
  ```

### Task 2: Implement bounded document metadata and context propagation

**Files:**
- Modify: `src/features/weekly_radar/runtime/discovery.rs`
- Modify: `src/features/weekly_radar/runtime/sources.rs`
- Modify: `src/features/weekly_radar/runtime.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`
- Test: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Consumes: Task 1 metadata and propagation tests.
- Produces: `document_metadata` date precedence and `SourceObservation::document_kind()`.

- [ ] **Step 1: Implement only the date parser needed by the RED tests**

  Add bounded helpers in `discovery.rs` that parse ISO date prefixes from
  `article:published_time`, `name=date`, JSON-LD `datePublished`, `<time
  datetime>`, and finally JSON-LD `dateModified`. Return `None` for malformed
  values and never infer a date from arbitrary prose.

- [ ] **Step 2: Carry `DocumentKind` into source observations**

  Add an optional `document_kind` field to `SourceObservationInput` and
  `SourceObservation`, expose a documented getter, serialize it optionally, and
  set it only in `collect_discovered_documents` and its unavailable status
  record. Leave entry points, hiring records, GDELT articles, and status-only
  observations as `None`.

- [ ] **Step 3: Run the focused tests to verify GREEN**

  ```bash
  cargo test --test weekly_radar_evidence_quality document_metadata_ -- --exact
  cargo test --test weekly_radar_runtime discovered_document_retains_document_kind_without_promoting_entry_point -- --exact
  ```

  Expected: all Task 1 tests pass with no change to existing negative cases.

- [ ] **Step 4: Commit the bounded metadata/context implementation**

  ```bash
  git add src/features/weekly_radar/runtime/discovery.rs src/features/weekly_radar/runtime/sources.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs
  git commit -m "feat: retain document metadata and discovery context"
  ```

### Task 3: Improve Claim extraction using document context

**Files:**
- Modify: `src/features/weekly_radar/runtime/evidence.rs`
- Modify: `tests/weekly_radar_evidence_quality.rs`
- Modify: `.ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json`

**Interfaces:**
- Consumes: `SourceObservation::document_kind()` and Task 2 metadata.
- Produces: complete candidates with document context in provenance while
  preserving `validate_evidence_candidate` as the promotion gate.

- [ ] **Step 1: Write the failing Claim tests**

  Add `dated_engineering_document_promotes_a_complete_claim`,
  `document_kind_context_is_retained_in_claim_provenance`, and
  `generic_or_non_actionable_document_remains_a_pending_lead` using injected
  fixture observations. Assert exact source title, URI, passage, date, and
  production area for the positive case.

- [ ] **Step 2: Run the Claim tests to verify RED**

  ```bash
  cargo test --test weekly_radar_evidence_quality dated_engineering_document_ -- --exact
  cargo test --test weekly_radar_evidence_quality document_kind_context_ -- --exact
  cargo test --test weekly_radar_evidence_quality generic_or_non_actionable_document_remains_a_pending_lead -- --exact
  ```

  Expected: the positive context assertion fails because the current candidate
  provenance does not include the discovered document kind.

- [ ] **Step 3: Implement the minimal context-aware extraction change**

  Keep the existing sentence/action/production signal gate. Add only the
  precise action signals demonstrated by the fixtures, and append a bounded
  `document_kind=<kind>` marker to the candidate provenance passage. Do not
  relax date, source-tier, title, passage, or production-area requirements.

- [ ] **Step 4: Run all evidence-quality tests**

  ```bash
  cargo test --test weekly_radar_evidence_quality
  ```

  Expected: all existing and new positive/negative tests pass.

- [ ] **Step 5: Commit the Claim extraction change**

  ```bash
  git add src/features/weekly_radar/runtime/evidence.rs tests/weekly_radar_evidence_quality.rs .ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json
  git commit -m "feat: improve document claim extraction provenance"
  ```

### Task 4: Add document-kind metrics and localized report visibility

**Files:**
- Modify: `src/main.rs`
- Modify: `src/features/weekly_radar/runtime/model.rs`
- Modify: `src/features/weekly_radar/runtime/report.rs`
- Modify: `tests/weekly_radar_runtime.rs`
- Modify: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Consumes: `SourceObservation::document_kind()` and validated/pending counts.
- Produces: optional `ResearchMetrics.document_kind_counts` and localized
  report lines with deterministic sorted counts.

- [ ] **Step 1: Write the failing report/compatibility tests**

  Add `research_metrics_document_kind_counts_default_for_legacy_json` and
  `localized_reports_render_document_kind_counts_without_ranking`.
  Assert old JSON deserializes with an empty map, Chinese/Japanese/English
  output includes the same sorted counts, and the report still contains no
  machine Ranking when no explicit selection is supplied.

- [ ] **Step 2: Run the tests to verify RED**

  ```bash
  cargo test --test weekly_radar_runtime research_metrics_document_kind_counts_default_for_legacy_json -- --exact
  cargo test --test weekly_radar_runtime localized_reports_render_document_kind_counts_without_ranking -- --exact
  ```

  Expected: the new metric accessor and report text are missing.

- [ ] **Step 3: Implement the optional metric envelope**

  Add a `BTreeMap<String, usize>` field with `#[serde(default,
  skip_serializing_if = "BTreeMap::is_empty")]`, a documented builder/getter,
  and increment it only for `SourceMaterialKind::Document` observations.
  Render the stable sorted entries in all three report languages. Do not feed
  this map to judgment or Ranking.

- [ ] **Step 4: Run focused runtime/report tests**

  ```bash
  cargo test --test weekly_radar_runtime
  cargo test --test weekly_radar_evidence_quality
  ```

  Expected: all focused tests pass, including legacy snapshot compatibility.

- [ ] **Step 5: Commit the metrics/report change**

  ```bash
  git add src/main.rs src/features/weekly_radar/runtime/model.rs src/features/weekly_radar/runtime/report.rs tests/weekly_radar_runtime.rs tests/weekly_radar_evidence_quality.rs
  git commit -m "feat: report document discovery quality metrics"
  ```

### Task 5: Document boundaries and run complete verification

**Files:**
- Modify: `docs/operations/WEEKLY_RADAR.md`
- Modify: `.ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json`
- Modify: `docs/superpowers/specs/2026-08-25-weekly-radar-document-discovery-quality-design.md`
- Modify: `docs/superpowers/plans/2026-08-25-weekly-radar-document-discovery-quality.md`

**Interfaces:**
- Consumes: Tasks 1–4 behavior and test evidence.
- Produces: operator documentation, updated Summary evidence, and a clean
  governed candidate for `ai-finish`.

- [ ] **Step 1: Update the operations documentation**

  Explain the metadata precedence, DocumentKind values, Claim completeness
  boundary, report counters, and the rule that source availability or document
  discovery is not itself a validated fact.

- [ ] **Step 2: Run the complete local quality suite**

  ```bash
  make check
  make check-ai-coverage-guard
  make check-ai-scenario-coverage CONTRACT=.ai/work-items/active/wi-weekly-radar-document-discovery-quality.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json
  ```

  Expected: formatter, clippy, all Cargo tests, coverage guard, and scenario
  coverage pass.

- [ ] **Step 3: Record the before-finish checkpoint**

  ```bash
  make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-document-discovery-quality.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-document-discovery-quality.summary.json STAGE=before_finish
  ```

- [ ] **Step 4: Run Finish and inspect the Outcome**

  ```bash
  make ai-finish TASK=wi-weekly-radar-document-discovery-quality REPORT_LANGUAGE=zh-CN
  ```

  Proceed only with a complete green Outcome and direct human handoff.

- [ ] **Step 5: Commit the final governed bundle**

  ```bash
  git add docs/ .ai/ src/ tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs
  git commit -m "docs: finalize document discovery quality work item"
  ```

### Task 6: PR, CI, merge, closure, and safe dry-run

**Files:**
- No new product files; use the archived Work Item projections and closure receipts generated by the governed lifecycle.

**Interfaces:**
- Consumes: green Finish Outcome and committed archive bundle.
- Produces: passed `check-ai-pr`, hosted CI, merged PR, closed WI, and one
  post-merge dry-run with no Telegram/archive mutation.

- [ ] **Step 1: Validate the committed PR boundary**

  ```bash
  make check-ai-pr AI_BASE_COMMIT=c7f0f3c466164cf7f104940d57de5d732eb60c1e
  ```

- [ ] **Step 2: Push one Work Item branch and open one PR**

  ```bash
  git push --set-upstream origin codex/wi-weekly-radar-document-discovery-quality
  gh pr create --base main --head codex/wi-weekly-radar-document-discovery-quality
  ```

- [ ] **Step 3: Wait for every required hosted check**

  ```bash
  gh pr checks <PR_NUMBER> --watch --interval 10
  ```

- [ ] **Step 4: Merge without provider-side branch deletion and close the WI**

  ```bash
  gh pr merge <PR_NUMBER> --merge --delete-branch=false
  make ai-close-work-item TASK=wi-weekly-radar-document-discovery-quality
  ```

- [ ] **Step 5: Run one safe post-merge dry-run**

  ```bash
  DRYRUN_ARCHIVE_DIR=$(mktemp -d /tmp/org-x-weekly-radar-document-discovery-quality.XXXXXX)
  ORGX_SEC_USER_AGENT='ORG-X local dry-run contact@example.test' cargo run --release -- weekly-radar --as-of 2026-08-25 --archive-dir "$DRYRUN_ARCHIVE_DIR" --registry config/weekly_radar/companies.json --language zh-CN --dry-run
  ```

  Expected: a validated report, no Telegram call, no archive files, explicit
  separation of document candidates, pending leads, and validated evidence.
