# Weekly Radar SEC/IR Deep Document Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn bounded SEC filing metadata and shallow official IR links into fetched, provider-neutral document observations that can enter the existing deterministic evidence gate.

**Architecture:** Keep SEC parsing in `runtime::sec`, same-origin link parsing in `runtime::discovery`, and observation construction/IR traversal in `runtime::sources`. The CLI merges SEC document observations with existing official observations before the unchanged `EvidenceCandidate` validation loop.

**Tech Stack:** Rust 2024, `chrono`, `regex`, `serde`, `serde_json`, `url`, injected `HttpClient`/`FixtureHttpClient`, Cargo tests, and AI Cockpit Make gates. No new dependency.

**Spec:** `docs/superpowers/specs/2026-08-25-weekly-radar-sec-ir-deep-discovery-design.md`

## Global Constraints

- SEC document requests remain User-Agent-bound, finite-body-bound, bounded by `MAX_SEC_DOCUMENT_CANDIDATES`, and fail closed.
- IR discovery remains same-origin, fragment-free, deduplicated, one-hop deep, and globally capped per configured entry point.
- Homepage/index availability is not evidence; only a complete dated document claim reaches `ValidatedEvidence`.
- GDELT/news, Stage, Ranking, publication, and legacy snapshot semantics remain unchanged.
- Every behavior change follows RED → observed failure → GREEN → observed pass.
- No live provider call is used by tests and no secret or credential is added.

---

### Task 1: Fetch and classify SEC filing documents

**Files:**
- Modify: `src/features/weekly_radar/runtime/sec.rs` — add bounded filing-body retrieval, document status, title/text retention, and safe per-document failures.
- Modify: `src/features/weekly_radar/runtime.rs` — export the new SEC document status type without changing existing public fact behavior.
- Test: `tests/weekly_radar_runtime.rs` — add SEC document success, partial failure, and finite-body tests.

**Interfaces:**
- Consumes: existing `SubmissionsDocument`, `SecDocumentCandidate` metadata validation, `HttpClient`, and `document_metadata`.
- Produces: `SecDocumentStatus::{Known, Unknown, Unavailable}`, `SecDocumentCandidate::title()`, `text()`, `status()`, and `status_reason()`; `CompanyEvidence::documents()` remains bounded and ordered.

- [ ] **Step 1: Write the failing SEC document tests.**

  Add fixtures that register submissions, Company Facts, and three filing URLs. Assert that the candidate exposes `DocumentKind::Filing`-compatible title/text/status information and that one filing failure does not remove the other two or the Company Facts. Add a response over `SEC_FILING_DOCUMENT_MAX_RESPONSE_BODY_BYTES` and assert `Unavailable` with a safe `filing_document` failure.

- [ ] **Step 2: Run the SEC tests and observe RED.**

  Run:

  ```bash
  cargo test --test weekly_radar_runtime sec_filing_document
  ```

  Expected: compile or assertion failures because SEC candidates do not yet retain fetched text/status and filing documents are not fetched as a bounded set.

- [ ] **Step 3: Implement the smallest SEC fetch path.**

  Add the finite filing-body limit, fetch each bounded candidate once, normalize the HTML through the existing document metadata helper, and preserve provider-private status. Reuse the fetched latest 10-K body for the employee fallback when available; retain the existing fallback behavior when the latest 10-K is outside the bounded candidate set. Do not change Company Facts alias selection.

- [ ] **Step 4: Run the SEC tests and observe GREEN.**

  Run:

  ```bash
  cargo test --test weekly_radar_runtime sec_filing_document
  cargo test --test weekly_radar_runtime sec_selects_latest_10k_metadata_and_preserves_employee_passage
  ```

  Expected: all selected tests pass, with no response body or request credential in failure text.

- [ ] **Step 5: Commit the SEC adapter slice.**

  ```bash
  git add src/features/weekly_radar/runtime/sec.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_runtime.rs
  git commit -m "feat: fetch bounded SEC filing documents"
  ```

### Task 2: Add bounded one-hop IR discovery

**Files:**
- Modify: `src/features/weekly_radar/runtime/discovery.rs` — preserve same-origin canonicalization and stable classification while supporting bounded parent-link traversal metadata.
- Modify: `src/features/weekly_radar/runtime/sources.rs` — add `SourceKind::Sec`, a shared document-observation factory, and one-hop IR traversal with a fixed total cap.
- Test: `tests/weekly_radar_evidence_quality.rs` — add nested IR document and cap/non-promotion fixtures.

**Interfaces:**
- Consumes: `discover_documents`, `DocumentCandidate`, `DocumentKind`, `SourceObservation`, and existing finite HTTP helpers.
- Produces: `SourceKind::Sec`, `document_observation(...)`, and `collect_configured_sources` behavior that reaches one bounded nested official IR document while preserving direct-document behavior.

- [ ] **Step 1: Write the failing IR deep-discovery tests.**

  Add an IR fixture where the entry page links to an `Earnings archive` index, and the index links to a dated `Q3 earnings release`. Assert the nested release is returned as a document observation with the release URL and body. Add cross-origin, duplicate, and more-than-cap links and assert they are never fetched. Assert the index alone does not create a validated candidate.

- [ ] **Step 2: Run the IR tests and observe RED.**

  ```bash
  cargo test --test weekly_radar_evidence_quality ir_deep_discovery
  ```

  Expected: the nested release is absent because current collection follows only direct entry links.

- [ ] **Step 3: Implement the one-hop traversal and source boundary.**

  Add a deterministic queue of direct candidates and at most one nested discovery pass, with a global same-origin URL set and a finite total cap. Add `SourceKind::Sec`. Refactor the existing document constructor into a documented `document_observation` factory used by IR and later by SEC. Preserve `SourceObservation` output-only serialization and all existing status mappings.

- [ ] **Step 4: Run the IR tests and observe GREEN.**

  ```bash
  cargo test --test weekly_radar_evidence_quality ir_deep_discovery
  cargo test --test weekly_radar_evidence_quality document_discovery_deduplicates_and_caps_followed_links
  ```

  Expected: nested same-origin documents are discovered within the cap, cross-origin/duplicate links are excluded, and existing direct discovery remains green.

- [ ] **Step 5: Commit the IR discovery slice.**

  ```bash
  git add src/features/weekly_radar/runtime/discovery.rs src/features/weekly_radar/runtime/sources.rs tests/weekly_radar_evidence_quality.rs
  git commit -m "feat: deepen bounded official IR discovery"
  ```

### Task 3: Feed SEC documents through the common evidence loop

The source-kind mapping in `src/features/weekly_radar/runtime/evidence.rs` is
part of this slice so SEC filing observations receive an explicit official
evidence classification.

**Files:**
- Modify: `src/main.rs` — convert SEC candidates to provider-neutral document observations, merge them with official observations, and remove duplicate SEC candidate/pending metric increments.
- Modify: `tests/weekly_radar_runtime.rs` — test SEC document observation conversion and safe status mapping.
- Modify: `tests/weekly_radar_evidence_quality.rs` — assert an actionable SEC filing body reaches the existing candidate/validation boundary without changing ranking gates.

**Interfaces:**
- Consumes: `CompanyEvidence::documents()`, `document_observation`, `SecDocumentStatus`, `SourceKind::Sec`, and the existing observation processing loop.
- Produces: one shared document path for SEC and IR, correct source/document metrics, and unchanged Stage/Ranking fail-closed behavior.

- [ ] **Step 1: Write the failing integration tests.**

  Add a SEC document fixture containing a dated production-system change and assert the common runtime input contains one validated evidence fact from the filing. Add a metric assertion proving the SEC candidate is counted once, and preserve the no-ranking assertion when the stage gate remains incomplete.

- [ ] **Step 2: Run the integration tests and observe RED.**

  ```bash
  cargo test --test weekly_radar_evidence_quality sec_document_enters_common_evidence_loop
  ```

  Expected: the filing remains only a metadata/pending lead because the CLI does not yet convert SEC candidates into `SourceObservation` values.

- [ ] **Step 3: Implement the integration with no judgment changes.**

  Build SEC document observations before the existing configured-source loop, process both collections through the same normalization/evidence code, count document candidates and pending leads only there, and count SEC stage failures separately from document-observation failures. Keep Company Facts facts and all ranking inputs unchanged.

- [ ] **Step 4: Run the focused integration and regression suites.**

  ```bash
  cargo test --test weekly_radar_evidence_quality sec_document_enters_common_evidence_loop
  cargo test --test weekly_radar_judgment_chain
  cargo test --test weekly_radar_runtime
  ```

  Expected: filing evidence is visible, partial failures remain explicit, and all Stage/Ranking regression tests pass.

- [ ] **Step 5: Commit the runtime integration slice.**

  ```bash
  git add src/main.rs tests/weekly_radar_runtime.rs tests/weekly_radar_evidence_quality.rs
  git commit -m "feat: route SEC documents through evidence validation"
  ```

### Task 4: Document the acquisition boundary and close the local problem loop

**Files:**
- Modify: `docs/operations/WEEKLY_RADAR.md` — document SEC filing-body discovery, IR one-hop limits, failure semantics, and the distinction between metadata, documents, leads, and validated evidence.
- Modify: `.ai/work-items/active/wi-weekly-radar-sec-ir-deep-discovery.summary.json` — record changed files, tests, observed issues, resolutions, residual risks, and local stop evidence.
- Modify: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md` — update only through Make targets.
- Create: `.ai/evidence/reference-impact/wi-weekly-radar-sec-ir-deep-discovery*.json` — add only if the reference-impact guard requires a current record.

**Interfaces:**
- Consumes: completed SEC, IR, and runtime slices plus the Contract scenarios.
- Produces: reader-facing semantics and a local, evidence-bound Work Item state with no unresolved discovered implementation or governance issue.

- [ ] **Step 1: Update operations documentation.**

  Explain that SEC submissions metadata is not filing evidence until the bounded body fetch succeeds, IR index pages are leads, nested discovery is one hop and same-origin, and all failures remain visible without proving “no change”.

- [ ] **Step 2: Run formatting, lint, and all tests.**

  ```bash
  cargo fmt --all -- --check
  cargo clippy --all-targets --all-features -- -D warnings
  cargo test --all
  ```

  Expected: all commands pass with no warnings.

- [ ] **Step 3: Run repository quality and AI Cockpit checks.**

  ```bash
  make ai-cockpit-quality GOVERNANCE_PROFILE=strict
  make check-ai-serial-order CONTRACT=.ai/work-items/active/wi-weekly-radar-sec-ir-deep-discovery.contract.json
  make check-ai-budget-impact CONTRACT=.ai/work-items/active/wi-weekly-radar-sec-ir-deep-discovery.contract.json
  make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-sec-ir-deep-discovery.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-sec-ir-deep-discovery.summary.json STAGE=before_finish
  ```

  Expected: all local gates pass; any discovered issue is recorded and fixed in this WI before stopping.

- [ ] **Step 4: Run one final dry-run and inspect the report.**

  ```bash
  ORGX_SEC_USER_AGENT='ORG-X local dry-run contact@example.test' cargo run -- weekly-radar --as-of 2026-08-25 --archive-dir /tmp/org-x-weekly-radar-sec-ir-deep-discovery-20260825 --registry config/weekly_radar/companies.json --language zh-CN --dry-run
  ```

  Expected: the report shows SEC filing/document observations separately from SEC stage health, IR documents are not homepage facts, Ranking remains suppressed when its gate is unmet, and no formal archive or Telegram delivery is contacted.

- [ ] **Step 5: Record the final local Outcome and stop without PR lifecycle.**

  Run `make ai-finish TASK=wi-weekly-radar-sec-ir-deep-discovery REPORT_LANGUAGE=zh-CN` only after every required local check and scenario is verified. If it passes, deliver the green Outcome and leave the branch ready for human review; do not push, merge, or close unless separately requested.
