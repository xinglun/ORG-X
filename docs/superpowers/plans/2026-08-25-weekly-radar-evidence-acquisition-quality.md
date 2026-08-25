# Weekly Radar Evidence Acquisition Quality Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Weekly Radar distinguish source availability, discovered documents, pending claims, and validated enterprise evidence while restoring diagnosable SEC acquisition and preserving fail-closed Stage/Ranking behavior.

**Architecture:** Keep provider-specific parsing inside `weekly_radar::runtime`. Add explicit material roles to source observations, bounded document discovery, and a deterministic `EvidenceCandidate` → `ValidatedEvidence` gate. Map only validated evidence and structured SEC facts into the existing `NormalizedFact` snapshot boundary; expose acquisition counters through a backward-compatible `ResearchMetrics` envelope consumed read-only by the report renderer.

**Tech Stack:** Rust 2024, `chrono`, `regex`, `serde`, `serde_json`, `url`, injected `HttpClient`/`FixtureHttpClient`, Cargo tests, Makefile AI Cockpit gates. No new dependency.

**Spec:** `docs/superpowers/specs/2026-08-25-weekly-radar-evidence-acquisition-quality-design.md`

## Global Constraints

- Source availability is not confirmed evidence; an official homepage cannot set `has_primary_evidence`.
- SEC submissions, Company Facts, and filing-document requests remain finite, User-Agent-bound, response-size-bound, and safe-diagnostic-only.
- Discovery uses same-origin links, fixed per-entry/per-company limits, stable deduplication, and no guessed URLs.
- `EvidenceCandidate` must include subject, concrete change/fact, usable date on or before cutoff, production-system area, source identity/kind, authority, polarity, passage, and provenance before validation.
- GDELT/news remains discovery-only and cannot become authoritative evidence.
- Existing Stage-before-Ranking, Counter Evidence, Missing Proof, publication, and legacy snapshot behavior remain fail-closed and backward-compatible.
- Every production change follows RED → observed failure → GREEN → observed pass; no implementation code precedes its failing test.
- Tests use injected fixtures and fixed timestamps; no external provider call, new auth material, or live publication is added.

---

### Task 1: Add explicit research metrics and source-material roles

**Files:**
- Modify: `src/features/weekly_radar/runtime/sources.rs` — add a serialized material-role enum to distinguish entry points, discovered documents, structured hiring records, and discovery articles; retain existing source status/tier.
- Modify: `src/features/weekly_radar/runtime/model.rs` — add backward-compatible `ResearchMetrics` and `RuntimeReportInput` accessors/defaults.
- Modify: `src/features/weekly_radar/runtime.rs` — stop promoting entry-point observations to `FactStatus::Known`; add the conversion boundary for validated evidence in Task 4.
- Test: `tests/weekly_radar_evidence_quality.rs` — create the public regression test file and assert the new source role/metrics API.

**Interfaces:**
- Consumes: existing `SourceObservation`, `SourceStatus`, `SourceTier`, `NormalizedFact`, and `RuntimeReportInput`.
- Produces: `SourceMaterialKind::{EntryPoint, Document, HiringRecord, DiscoveryArticle, Status}`, `SourceObservation::material_kind()`, `ResearchMetrics::{source_available, document_candidates, validated_evidence, pending_leads, unavailable_sources}`, `RuntimeReportInput::research_metrics()`, and a default-zero deserialization path for legacy snapshots.

- [x] **Step 1: Write the failing tests**

```rust
#[test]
fn official_entry_point_is_not_a_confirmed_fact() {
    let observation = official_observation(SourceMaterialKind::EntryPoint, "Investor Relations");
    let fact = normalize_source_observation(&observation, 1).unwrap();

    assert_eq!(fact.status(), &FactStatus::Unconfirmed);
    assert_eq!(fact.value(), None);
}

#[test]
fn legacy_runtime_input_defaults_research_metrics_to_zero() {
    let legacy = serde_json::json!({
        "as_of": "2026-08-25",
        "companies": [],
        "facts": [],
        "source_coverage": [],
        "source_failures": []
    });
    let input: RuntimeReportInput = serde_json::from_value(legacy).unwrap();

    assert_eq!(input.research_metrics().validated_evidence(), 0);
    assert_eq!(input.research_metrics().source_available(), 0);
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality official_entry_point_is_not_a_confirmed_fact legacy_runtime_input_defaults_research_metrics_to_zero`

Expected: compile failure because `SourceMaterialKind`, `material_kind`, and `research_metrics` do not yet exist; no production code is changed before this failure is observed.

- [x] **Step 3: Implement the minimal source-role and metrics model**

Add a `SourceMaterialKind` enum and a `material_kind` field to the private `SourceObservationInput` and public serialized `SourceObservation`. Set existing observations explicitly: official page = `EntryPoint`, Greenhouse/Lever record = `HiringRecord`, GDELT article = `DiscoveryArticle`; use `Document` for future discovered pages and `Status` for unavailable/not-configured records.

Add a serializable `ResearchMetrics` value object with checked non-negative `usize` counters, `Default`, getters, and a `RuntimeReportInput` field with `#[serde(default)]`. Add `set_research_metrics` so acquisition orchestration can bind one complete metric envelope before judgment/reporting. Change only the official-entry-point branch of `normalize_source_observation` to return `Unconfirmed`; keep SEC facts and future validated-evidence conversion on the confirmed path.

- [x] **Step 4: Run the focused tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality official_entry_point_is_not_a_confirmed_fact legacy_runtime_input_defaults_research_metrics_to_zero`

Expected: both tests pass, and the existing `cargo test --all` suite remains green for this model-only change.

- [x] **Step 5: Commit the bounded model change**

```bash
git add src/features/weekly_radar/runtime/sources.rs src/features/weekly_radar/runtime/model.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_evidence_quality.rs
git commit -m "feat: separate source material from confirmed facts"
```

### Task 2: Make SEC acquisition independent, bounded, and document-aware

**Files:**
- Modify: `src/features/weekly_radar/runtime/sec.rs` — return partial collection facts, safe stage failures, and bounded filing document candidates while retaining current aliases and response limits.
- Modify: `src/features/weekly_radar/runtime/model.rs` — add provider-neutral source failure stage detail only if the existing `SourceFailure` reason cannot carry it without exposing payloads.
- Modify: `src/main.rs` — retain successful SEC facts, bind per-stage failures, count SEC availability/unavailability, and add SEC document candidates to the same evidence path.
- Test: `tests/weekly_radar_evidence_quality.rs` — add SEC fixtures for success, submissions-only failure, facts-only failure, optional filing-document failure, and finite document selection.

**Interfaces:**
- Consumes: `CompanyConfig`, `HttpClient`, `NormalizedFact`, existing `SecClient::collect` fixtures, and `RuntimeError` safe display.
- Produces: `CompanyEvidence::documents()`, `CompanyEvidence::failures()`, `SecDocumentCandidate`, and a stable stage label set `{submissions, company_facts, filing_document}`.

- [x] **Step 1: Write the failing SEC partial-result tests**

```rust
#[test]
fn sec_keeps_company_facts_when_submissions_request_fails() {
    let client = FixtureHttpClient::new();
    client.insert(facts_url(), HttpResponse::ok(company_facts_fixture()));

    let evidence = SecClient::collect(&sec_test_company(), &client, TEST_USER_AGENT).unwrap();

    assert_eq!(evidence.fact("revenue").unwrap().status(), &FactStatus::Known);
    assert!(evidence.failures().iter().any(|failure| failure.stage() == "submissions"));
}

#[test]
fn sec_discovers_only_bounded_recent_filings_with_provenance() {
    let client = FixtureHttpClient::new();
    client.insert(submissions_url(), HttpResponse::ok(submissions_fixture_with_recent_filings()));
    client.insert(facts_url(), HttpResponse::ok(company_facts_fixture()));

    let evidence = SecClient::collect(&sec_test_company(), &client, TEST_USER_AGENT).unwrap();

    assert_eq!(evidence.documents().len(), 3);
    assert!(evidence.documents().iter().all(|document| document.source_uri().starts_with("https://www.sec.gov/Archives/")));
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality sec_keeps_company_facts_when_submissions_request_fails sec_discovers_only_bounded_recent_filings_with_provenance`

Expected: compile failure because `CompanyEvidence::failures`, `documents`, and `SecDocumentCandidate` do not exist; existing SEC behavior is not treated as a passing substitute.

- [x] **Step 3: Implement independent SEC stages**

Introduce private stage helpers that return `Result<T, SecStageFailure>` with fixed stage names and safe reasons derived only from `RuntimeError` variants. Fetch Company Facts and submissions independently. For a failed Company Facts request, emit the existing normalized unavailable/unknown fact state for each configured fact kind; for a failed submissions request, retain Company Facts facts and record the submissions failure.

Use submissions metadata to select at most `MAX_SEC_DOCUMENT_CANDIDATES` recent `10-K`, `10-Q`, and `8-K` records with complete accession, filing date, report date, and primary document. Construct URLs only from the SEC archive root plus validated metadata; never accept an external URL from fixture payloads. Keep the latest 10-K employee passage extraction bounded; when that optional document request fails, retain existing facts and add a `filing_document` failure instead of aborting the company result.

- [x] **Step 4: Update the acquisition orchestration**

In `acquire_runtime_input`, add all returned SEC facts, map each safe SEC stage failure to `SourceFailure::new("sec", company.id(), stage_reason)`, count SEC source availability only when a SEC stage returns usable data, and add discovered SEC documents as pending candidates for Task 4. Set `has_primary_evidence` only when a SEC fact is `FactStatus::Known`; do not set it from source-entry availability.

- [x] **Step 5: Run the focused SEC tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality sec_` and `cargo test --test weekly_radar_runtime sec_`

Expected: all new partial-result/document tests and existing SEC alias, limit, User-Agent, conflict, and employee extraction tests pass.

- [x] **Step 6: Commit the SEC acquisition change**

```bash
git add src/features/weekly_radar/runtime/sec.rs src/features/weekly_radar/runtime/model.rs src/main.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs
git commit -m "feat: make sec acquisition partially observable"
```

### Task 3: Add bounded same-origin document discovery

**Files:**
- Modify: `src/features/weekly_radar/runtime/sources.rs` — extract and classify bounded links from official entry-point HTML, fetch only same-origin relevant documents, and preserve document title/date/provenance.
- Create: `src/features/weekly_radar/runtime/discovery.rs` — provider-neutral document candidate and deterministic date/kind classification helpers.
- Modify: `src/features/weekly_radar/runtime.rs` — export the discovery types required by tests and evidence extraction.
- Test: `tests/weekly_radar_evidence_quality.rs` — add same-origin, cross-origin, deduplication, request-limit, and homepage-only fixtures.

**Interfaces:**
- Consumes: an official `SourceObservation` with `EntryPoint` material, validated configured URL, `HttpClient`, and fixed `DateTime<Utc>`.
- Produces: `DocumentCandidate { company_id, source_kind, url, title, document_kind, published_or_effective_date, provenance }`, `DocumentKind`, and `MAX_DOCUMENT_CANDIDATES_PER_ENTRY`.

- [x] **Step 1: Write the failing discovery tests**

```rust
#[test]
fn official_entry_point_discovers_relevant_same_origin_documents_only() {
    let company = source_test_company();
    let client = FixtureHttpClient::new();
    client.insert(company.official_ir_url().unwrap(), HttpResponse::ok(ir_homepage_with_links()));
    client.insert("https://ir.example.test/earnings/q2", HttpResponse::ok("<title>Q2 Earnings Release</title><time datetime=\"2026-08-20\">"));
    client.insert("https://ir.example.test/organization/update", HttpResponse::ok("<title>Organization update</title><time datetime=\"2026-08-19\">"));

    let observations = collect_configured_sources(&company, &client, observed_at());
    let documents: Vec<_> = observations.iter().filter(|observation| observation.material_kind() == SourceMaterialKind::Document).collect();

    assert_eq!(documents.len(), 2);
    assert!(documents.iter().all(|observation| observation.url().unwrap().starts_with("https://ir.example.test/")));
}

#[test]
fn homepage_only_is_available_but_never_a_document_or_confirmed_fact() {
    let company = source_test_company();
    let client = FixtureHttpClient::with_response(company.official_ir_url().unwrap(), HttpResponse::ok("<title>Investor Relations</title>"));

    let observations = collect_configured_sources(&company, &client, observed_at());
    let entry = observations.iter().find(|observation| observation.kind() == SourceKind::OfficialIr).unwrap();

    assert_eq!(entry.material_kind(), SourceMaterialKind::EntryPoint);
    assert_eq!(normalize_source_observation(entry, 1).unwrap().status(), &FactStatus::Unconfirmed);
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality official_entry_point_discovers_relevant_same_origin_documents_only homepage_only_is_available_but_never_a_document_or_confirmed_fact`

Expected: compile failure because `DocumentCandidate`, `DocumentKind`, and document material observations are not implemented.

- [x] **Step 3: Implement link extraction and bounded selection**

Extract `href` values from the already bounded official HTML response. Resolve relative URLs against the configured entry-point URL, reject fragments, non-HTTP(S), cross-origin hosts/ports/schemes, and duplicate canonical URLs. Keep only links whose title/URL classifies as filing, earnings, investor-day, engineering, AI/automation, organization, product, platform, or career material. Sort by canonical URL before applying `MAX_DOCUMENT_CANDIDATES_PER_ENTRY`.

Fetch each selected document through the existing `HttpClient` body limit. Parse `<title>` and the first explicit `<time datetime>`/ISO date; keep absent dates as `None`. Emit a `Document` observation with `SourceTier::OfficialPrimary`; a failed selected document becomes an explicit unavailable document observation and does not abort other links. Preserve the entry-point observation separately.

- [x] **Step 4: Run the focused discovery tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality official_entry_point_discovers_relevant_same_origin_documents_only homepage_only_is_available_but_never_a_document_or_confirmed_fact`

Expected: both tests pass, including same-origin rejection, deterministic deduplication, and homepage non-promotion.

- [x] **Step 5: Commit the discovery change**

```bash
git add src/features/weekly_radar/runtime/sources.rs src/features/weekly_radar/runtime/discovery.rs src/features/weekly_radar/runtime.rs tests/weekly_radar_evidence_quality.rs
git commit -m "feat: discover bounded official documents"
```

### Task 4: Implement the EvidenceCandidate validation gate

**Files:**
- Create: `src/features/weekly_radar/runtime/evidence.rs` — define candidate fields, validation errors, `ValidatedEvidence`, deterministic rule extraction, and normalized-fact conversion.
- Modify: `src/features/weekly_radar/runtime.rs` — export the gate and use it for document observations.
- Modify: `src/features/weekly_radar/runtime/model.rs` — add any provider-neutral fields required to preserve evidence metrics and provenance without changing existing fact wire semantics.
- Test: `tests/weekly_radar_evidence_quality.rs` — add table-driven rejection/promotion tests.

**Interfaces:**
- Consumes: `CompanyConfig`, `SourceObservation`, `DocumentCandidate`/document observations, `NaiveDate` cutoff, and existing `Provenance`.
- Produces: `EvidenceCandidate`, `ValidatedEvidence`, `EvidenceValidationError`, `extract_evidence_candidate`, `validate_evidence_candidate`, and `NormalizedFact::from_validated_evidence`/equivalent runtime helper.

- [x] **Step 1: Write the failing gate tests**

```rust
#[test]
fn evidence_gate_rejects_missing_production_area_and_cutoff_date() {
    let candidate = EvidenceCandidate::new(
        "acme", "Acme", "Acme changed responsibility", None,
        "", EvidenceSourceKind::OfficialMaterial, SourceTier::OfficialPrimary,
        EvidencePolarity::Supporting, "https://ir.example.test/update",
    ).unwrap();

    let error = validate_evidence_candidate(&candidate, cutoff()).unwrap_err();

    assert_eq!(error, EvidenceValidationError::MissingRequiredField { field: "effective_date" });
}

#[test]
fn complete_authoritative_candidate_becomes_validated_evidence() {
    let candidate = complete_candidate("2026-08-19", "engineering workflow");

    let validated = validate_evidence_candidate(&candidate, cutoff()).unwrap();

    assert_eq!(validated.company_id(), "acme");
    assert_eq!(validated.production_area(), "engineering workflow");
    assert_eq!(validated.effective_date(), Some(&date("2026-08-19")));
}

#[test]
fn page_level_observation_cannot_create_an_evidence_candidate() {
    let entry = official_observation(SourceMaterialKind::EntryPoint, "Investor Relations");

    assert!(extract_evidence_candidate(&entry).is_none());
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality evidence_gate_rejects_missing_production_area_and_cutoff_date complete_authoritative_candidate_becomes_validated_evidence page_level_observation_cannot_create_an_evidence_candidate`

Expected: compile failure because the candidate/gate types and extraction function do not exist.

- [x] **Step 3: Implement deterministic candidate validation**

Define owned bounded strings for subject, change, production area, source URI/title, and passage. `EvidenceCandidate::new` rejects blank required strings. `validate_evidence_candidate` checks effective date presence, cutoff ordering, `SourceTier::OfficialPrimary`, source URI/title, and content passage; returns a stable enum error naming the first failed field in declaration order. Preserve `EvidencePolarity` and an extractor version string.

Implement `extract_evidence_candidate` only for `Document` material with official authority. Use the document title plus normalized text as the bounded passage. Require an explicit date from provenance, a change signal (`reorgan`, `restructur`, `consolidat`, `appoint`, `shift`, `launch`, `adopt`, `automat`, `agent`, `responsibility`, or `operating model`), and a production-area signal (`engineering`, `platform`, `product`, `operations`, `workflow`, `automation`, `AI`, `agent`, `data`, `cloud`, `research`, or `development`). The configured company is the subject; the selected signal phrase is the production area; the bounded title/text is the concrete change claim. Non-matching documents remain pending and are never confirmed.

Add `ValidatedEvidence::to_normalized_fact(index)` with a stable `evidence_<source_kind>_<index>` kind, `FactStatus::Known`, `Confidence::High`, and provenance containing the source title, production area, date, passage, and extractor version. Use a deterministic standard-library content hash helper; do not add a hashing dependency.

- [x] **Step 4: Run the focused gate tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality evidence_gate_`

Expected: all candidate rejection, promotion, and page-level non-promotion tests pass.

- [x] **Step 5: Commit the evidence gate**

```bash
git add src/features/weekly_radar/runtime/evidence.rs src/features/weekly_radar/runtime.rs src/features/weekly_radar/runtime/model.rs tests/weekly_radar_evidence_quality.rs
git commit -m "feat: gate source claims into validated evidence"
```

### Task 5: Wire validated evidence, metrics, and primary-evidence gating

**Files:**
- Modify: `src/main.rs` — run extraction/validation for discovered official documents, retain invalid candidates as pending/unknown, increment `ResearchMetrics`, and set `has_primary_evidence` only from structured SEC facts or validated evidence.
- Modify: `src/features/weekly_radar/runtime.rs` — keep source normalization non-confirmed and expose validated-evidence conversion.
- Modify: `src/features/weekly_radar/runtime/model.rs` — bind the final metrics envelope and preserve snapshot round trips.
- Test: `src/main.rs` unit-test module plus `tests/weekly_radar_judgment_chain.rs` and `tests/weekly_radar_runtime.rs` — add orchestration and no-ranking regressions. The private acquisition boundary is tested in its owning binary module.

**Interfaces:**
- Consumes: source observations, SEC `CompanyEvidence`, `extract_evidence_candidate`, `validate_evidence_candidate`, and existing judgment derivation.
- Produces: runtime input where `research_metrics.validated_evidence` counts only promoted records; `source_available` counts reachable configured entries; `document_candidates` counts bounded discovered/SEC documents; `pending_leads` counts non-promoted candidates; `unavailable_sources` counts configured acquisition failures.

- [x] **Step 1: Write the failing orchestration tests**

```rust
#[test]
fn homepage_availability_does_not_satisfy_primary_evidence_guard() {
    let registry = registry_with_only_official_homepages();
    let acquired = acquire_fixture_runtime_input(&registry, homepage_only_client());

    assert!(!acquired.has_primary_evidence);
    assert_eq!(acquired.input.research_metrics().source_available(), 3);
    assert_eq!(acquired.input.research_metrics().validated_evidence(), 0);
}

#[test]
fn validated_document_claim_is_counted_and_can_feed_judgment() {
    let acquired = acquire_fixture_runtime_input(&registry_with_valid_document(), valid_document_client());

    assert!(acquired.has_primary_evidence);
    assert_eq!(acquired.input.research_metrics().validated_evidence(), 1);
    assert!(acquired.input.facts().iter().any(|fact| fact.kind().starts_with("evidence_")));
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality homepage_availability_does_not_satisfy_primary_evidence_guard validated_document_claim_is_counted_and_can_feed_judgment`

Expected: compile failure or assertion failure because acquisition currently treats official source availability as primary evidence and does not bind research metrics.

- [x] **Step 3: Implement the acquisition wiring**

For each SEC result, add facts first, record stage failures, and increment structured availability/unavailability counters. For each source observation, increment entry-point availability only for reachable configured entry points; increment document-candidate count for `Document` observations and SEC document candidates; always add the source-status fact for audit, but invoke extraction only for authoritative documents. On successful validation, add the validated fact and increment `validated_evidence`; on a rejected candidate, add one `Unconfirmed` pending fact with a safe validation reason and increment `pending_leads`.

Remove the current `has_primary_evidence` assignment from `observation.is_authoritative() && observation.status() == SourceStatus::Known`. Set it only when a SEC fact is `Known` or a `ValidatedEvidence` fact is added. Keep GDELT, careers postings, and page-level entries out of that guard.

- [x] **Step 4: Run the focused orchestration and judgment tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality homepage_availability_does_not_satisfy_primary_evidence_guard validated_document_claim_is_counted_and_can_feed_judgment && cargo test --test weekly_radar_judgment_chain`

Expected: new orchestration tests pass; existing insufficient-evidence tests still prove no machine Ranking; existing structured SEC evidence tests remain green.

- [x] **Step 5: Commit the runtime wiring**

```bash
git add src/main.rs src/features/weekly_radar/runtime.rs src/features/weekly_radar/runtime/model.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_judgment_chain.rs tests/weekly_radar_runtime.rs
git commit -m "feat: bind validated evidence to weekly radar input"
```

### Task 6: Reframe Weekly Radar report output and document the runtime contract

**Files:**
- Modify: `src/features/weekly_radar/runtime/report.rs` — render four explicit counters, keep `已确认信息` limited to validated/structured facts, and use degraded wording when source coverage is insufficient.
- Modify: `tests/weekly_radar_evidence_quality.rs` and `tests/weekly_radar_runtime.rs` — add Chinese/Japanese/English output and legacy snapshot assertions.
- Modify: `docs/operations/WEEKLY_RADAR.md` — document source availability versus pending lead versus validated evidence and SEC degraded behavior.
- Create: `docs/superpowers/plans/2026-08-25-weekly-radar-evidence-acquisition-quality.md` — this plan, already declared in the Contract.

**Interfaces:**
- Consumes: `RuntimeReportInput::research_metrics`, existing `SourceHealthFacts`, `NormalizedFact`, and localized report labels.
- Produces: deterministic report sections/labels for validated evidence, source availability, pending leads, and unavailable sources; no new ranking computation.

- [x] **Step 1: Write the failing report tests**

```rust
#[test]
fn degraded_report_separates_evidence_and_source_availability_counts() {
    let input = input_with_metrics(ResearchMetrics::new(9, 10, 0, 10, 50));
    let report = render_report_in_language(&input, ReportLanguage::Chinese);

    assert!(report.markdown().contains("本周新增有效证据：0"));
    assert!(report.markdown().contains("来源可用性确认：9"));
    assert!(report.markdown().contains("待验证线索：10"));
    assert!(report.markdown().contains("关键数据源不可用：50"));
    assert!(report.markdown().contains("数据不足"));
    assert!(!report.markdown().contains("Investor Relations"));
}

#[test]
fn localized_reports_keep_the_same_metric_values() {
    let input = input_with_metrics(ResearchMetrics::new(9, 10, 1, 9, 50));

    for language in [ReportLanguage::Chinese, ReportLanguage::Japanese, ReportLanguage::English] {
        let report = render_report_in_language(&input, language);
        assert!(report.markdown().contains("9"));
        assert!(report.markdown().contains("10"));
        assert!(report.markdown().contains("50"));
    }
}
```

- [x] **Step 2: Run the focused tests to verify RED**

Run: `cargo test --test weekly_radar_evidence_quality degraded_report_separates_evidence_and_source_availability_counts localized_reports_keep_the_same_metric_values`

Expected: assertion failure because the current report calls page-level known facts confirmed and has no four-counter vocabulary.

- [x] **Step 3: Implement localized report semantics**

Add labels for the four counters in all three languages. Build the summary from `ResearchMetrics`; retain existing `SourceHealthFacts` detail for legacy facts and source failures. Render `已确认信息` only from `FactStatus::Known` facts whose kind is a structured SEC fact or `evidence_` kind. When validated structural count is zero and unavailable/pending metrics are non-zero, use the calibrated degraded sentence and never use the plain no-change sentence. Keep all existing localized headings, snapshot JSON, report IDs, and ranking reference sections deterministic.

- [x] **Step 4: Update operations documentation and legacy fixtures**

Document the acquisition state machine, SEC stage diagnostics, bounded discovery limits, candidate gate fields, and the exact interpretation of the four report counters in `docs/operations/WEEKLY_RADAR.md`. Add a legacy JSON fixture without `research_metrics` and assert it decodes with zero defaults and renders without a fabricated evidence count.

- [x] **Step 5: Run the focused report tests to verify GREEN**

Run: `cargo test --test weekly_radar_evidence_quality degraded_report_ && cargo test --test weekly_radar_runtime task7_report_ task8_report_`

Expected: new localized counter/degraded-output tests and existing report compatibility tests pass.

- [x] **Step 6: Commit report semantics and documentation**

```bash
git add src/features/weekly_radar/runtime/report.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs docs/operations/WEEKLY_RADAR.md
git commit -m "feat: distinguish validated evidence in weekly reports"
```

### Task 7: Run the complete verification and Work Item handoff

**Files:**
- Modify: `.ai/work-items/active/wi-weekly-radar-evidence-quality.summary.json` — record every changed file, test result, checkpoint, residual risk, and CI evidence.
- Modify: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md` — generate through Make targets only.
- Create: `.ai/evidence/reference-impact/wi-weekly-radar-evidence-quality-*.json` — generated reference-impact evidence if required by guards.
- Modify: `.ai/work-items/active/wi-weekly-radar-evidence-quality.outcome.*` — generated by Finish only.

**Interfaces:**
- Consumes: all production/tests/docs changes from Tasks 1–6 and the Contract's 16 required checks.
- Produces: checkpoint-bound Summary, green/red Outcome evidence, archive bundle, and exact-head CI/PR evidence.

- [x] **Step 1: Run the pre-finish checkpoint**

Run: `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-evidence-quality.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-evidence-quality.summary.json STAGE=before_finish`

Expected: checkpoint evidence binds the current Contract hash, Summary acceptance/scenario counts, and required-check inventory.

- [x] **Step 2: Run local formatting, lint, tests, and repository quality**

Run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
make quality
```

Expected: all commands pass with no warnings, no changed generated files outside declared scope, and no test weakening.

- [x] **Step 3: Update Summary with evidence and run Finish**

Record exact commands and results in Summary, set all required scenarios to `verified` only where output exists, keep any real external uncertainty in `knownGaps`/`residualRisks`, then run:

```bash
make ai-finish TASK=wi-weekly-radar-evidence-quality REPORT_LANGUAGE=zh-CN
```

Expected: a separate `Outcome: 🟢` result is produced only if all required bindings and checks pass; otherwise retain the blocked Outcome and fix the in-scope issue.

- [ ] **Step 4: Archive, commit, and validate the exact PR candidate**

Run the repository's canonical archive target, commit the complete archive bundle, then run:

```bash
make check-ai-pr AI_BASE_COMMIT=7ea05a9a464657f4bf8c8ec391f1efa66d9e28ae
```

Expected: the committed diff is owned by the active Work Item, archive/Contract/Summary/Outcome bindings are exact, and no out-of-scope path is present.

- [ ] **Step 5: Push, open the PR, and trigger exactly one CI run**

Push only `codex/weekly-radar-evidence-quality`, open one PR against `main`, and trigger one non-release CI run for the exact candidate head. Record workflow/run/job identity and result in the Summary; do not infer success from a queued run.

- [ ] **Step 6: Merge and close only after hosted gates pass**

After required hosted checks pass, merge the PR through the repository workflow, then run `make ai-close-work-item TASK=wi-weekly-radar-evidence-quality`. Verify the archived record, merged PR head SHA, synchronized `main`, clean worktrees, and absent remote/local Work Item branch before reporting closure.

## Plan self-review

- SEC recovery is covered by Task 2; the plan explicitly tests both independent endpoint success/failure and bounded filings.
- Document discovery is covered by Task 3; homepage availability is tested separately from documents.
- Evidence extraction/validation is covered by Task 4; Task 5 binds only validated output to primary evidence and metrics.
- Report semantics, localization, legacy decoding, and degraded wording are covered by Task 6.
- Stage/Ranking non-regression and full governance/CI lifecycle are covered by Tasks 5 and 7.
- No task requires a new dependency, live provider call, unrestricted crawl, or secret-bearing fixture.
- Placeholder scan completed: no `TBD`, `TODO`, `FIXME`, or unspecified implementation step remains in this plan.
