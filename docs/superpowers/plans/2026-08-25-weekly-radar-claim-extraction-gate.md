# Weekly Radar Claim Extraction Gate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent title, metadata, and full-page document text from becoming ValidatedEvidence unless a bounded sentence-level production-system claim is present.

**Architecture:** Keep discovery responsible for document identity, title, date, and clean body text. Keep evidence extraction responsible for selecting one bounded body sentence with a change signal and production-system signal, then use the existing validation gate for authority, cutoff, provenance, and required fields. SEC facts and report/Raking boundaries remain unchanged.

**Tech Stack:** Rust, `chrono`, `regex`, existing provider-neutral Weekly Radar runtime, fixture-driven Rust integration tests, Make/AI Cockpit quality gates.

**Spec:** Approved bounded design in the conversation on 2026-08-25; governed by `.ai/work-items/active/wi-weekly-radar-claim-extraction-gate.contract.json`.

## Global Constraints

- Use deterministic rule-only extraction; never add LLM or probabilistic claim extraction.
- Title text, script/JSON metadata, entry-point pages, and ambiguous material must not promote to ValidatedEvidence.
- Preserve SEC financial facts, source availability, document candidate metrics, and fail-closed Ranking behavior.
- Keep the implementation bounded to `discovery.rs`, `evidence.rs`, the focused evidence-quality tests, and the Weekly Radar operations note.
- Do not use external network, credentials, Telegram, data-branch persistence, or workflow changes in tests or local verification.

---

### Task 1: Establish the failing body-content and sentence-claim tests

**Files:**
- Modify: `tests/weekly_radar_evidence_quality.rs`
- Read: `src/features/weekly_radar/runtime/discovery.rs`
- Read: `src/features/weekly_radar/runtime/evidence.rs`

**Interfaces:**
- Consumes: existing `CompanyConfig`, fixture HTTP client, `collect_configured_sources`, `extract_evidence_candidate`, and `document_metadata` behavior.
- Produces: failing regression tests that define clean body extraction and sentence-level claim promotion.

- [ ] **Step 1: Write the failing tests**

Add tests with these exact behaviors:

```rust
#[test]
fn document_body_excludes_title_script_and_metadata_before_claim_extraction() {
    let (title, date, body) = document_metadata(
        r#"<html><head><title>Acme engineering update</title>
        <meta name="description" content="Acme adopted an agent workflow.">
        <script>window.claim = "Acme adopted an agent workflow.";</script></head>
        <body><p>Acme moved its engineering workflow to an agent-assisted scheduler.</p></body></html>"#,
        "fallback",
    );

    assert_eq!(title, "Acme engineering update");
    assert_eq!(date, None);
    assert_eq!(body, "Acme moved its engineering workflow to an agent-assisted scheduler.");
}

#[test]
fn title_only_document_does_not_create_a_claim_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Acme moved its engineering workflow.</title></head></html>"#,
        "https://ir.example.test/engineering/update",
    );

    assert!(extract_evidence_candidate(&observation).is_none());
}

#[test]
fn body_sentence_with_change_and_production_signals_creates_a_bounded_candidate() {
    let observation = document_observation_from_html(
        r#"<html><head><title>Engineering update</title></head>
        <body><p>Acme moved its engineering workflow to an agent-assisted scheduler.</p>
        <p>The page also contains implementation details.</p></body></html>"#,
        "https://ir.example.test/engineering/update",
    );

    let candidate = extract_evidence_candidate(&observation).expect("body claim should qualify");

    assert_eq!(candidate.concrete_change(), "Acme moved its engineering workflow to an agent-assisted scheduler.");
    assert!(!candidate.concrete_change().contains("implementation details"));
}
```

The helper must construct an official `SourceObservation` through the existing fixture path; do not deserialize or hand-construct a production observation with a test-only bypass.

- [ ] **Step 2: Run the focused tests to verify RED**

Run:

```bash
cargo test --test weekly_radar_evidence_quality document_body_excludes_title_script_and_metadata_before_claim_extraction -- --exact
cargo test --test weekly_radar_evidence_quality title_only_document_does_not_create_a_claim_candidate -- --exact
cargo test --test weekly_radar_evidence_quality body_sentence_with_change_and_production_signals_creates_a_bounded_candidate -- --exact
```

Expected: compilation or assertion failure because the current metadata normalizer retains title/script text and the current extractor promotes combined title/full-page text rather than a bounded body sentence.

- [ ] **Step 3: Commit the red tests**

```bash
git add tests/weekly_radar_evidence_quality.rs
git commit -m "test: define weekly radar claim extraction boundary"
```

### Task 2: Strip non-content metadata and preserve document identity

**Files:**
- Modify: `src/features/weekly_radar/runtime/discovery.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Consumes: `document_metadata(html, fallback_title) -> (String, Option<NaiveDate>, String)`.
- Produces: the same public return shape, with the third value containing only bounded body/content text and never title, `script`, `style`, `noscript`, or `meta` payloads.

- [ ] **Step 1: Implement the minimal body-content normalizer**

Keep title and date extraction unchanged. Before tag normalization, remove non-content blocks and metadata. Rust's `regex` crate does not support backreferences, so remove each block tag family with its own bounded expression:

```rust
fn normalize_document_body(value: &str) -> String {
    let mut without_non_content = value.to_owned();
    for tag in ["script", "style", "noscript"] {
        let regex = Regex::new(&format!(r"(?is)<{tag}\\b[^>]*>.*?</{tag}\\s*>"))
            .expect("valid non-content regex");
        without_non_content = regex.replace_all(&without_non_content, " ").into_owned();
    }
    let without_metadata = Regex::new(r"(?is)<title\\b[^>]*>.*?</title\\s*>|<meta\\b[^>]*>")
        .expect("valid metadata regex")
        .replace_all(&without_non_content, " ");
    normalize_markup(&without_metadata)
}
```

Use this helper only for the body text returned by `document_metadata`; keep the separate title extraction so the report retains source identity.

- [ ] **Step 2: Run the new tests to verify GREEN for metadata behavior**

Run:

```bash
cargo test --test weekly_radar_evidence_quality document_body_excludes_title_script_and_metadata_before_claim_extraction -- --exact
cargo test --test weekly_radar_evidence_quality title_only_document_does_not_create_a_claim_candidate -- --exact
```

Expected: the body-content test passes; the sentence extraction test remains red until Task 3.

- [ ] **Step 3: Run the existing discovery regression tests**

Run:

```bash
cargo test --test weekly_radar_evidence_quality official_entry_point_discovers_relevant_same_origin_documents_only -- --exact
cargo test --test weekly_radar_evidence_quality document_discovery_deduplicates_and_caps_followed_links -- --exact
```

Expected: both pass, proving URL discovery, same-origin bounds, and candidate caps are unchanged.

- [ ] **Step 4: Commit the content-boundary implementation**

```bash
git add src/features/weekly_radar/runtime/discovery.rs tests/weekly_radar_evidence_quality.rs
git commit -m "fix: exclude metadata from weekly radar document bodies"
```

### Task 3: Extract one sentence-level production claim

**Files:**
- Modify: `src/features/weekly_radar/runtime/evidence.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Consumes: authoritative `SourceObservation` with `material_kind=Document`, `status=Known`, an effective date, title, and clean body text.
- Produces: `Option<EvidenceCandidate>` whose `concrete_change` and `passage` are one bounded sentence, not a title plus full document.

- [ ] **Step 1: Add the sentence-level extraction helper and signal rules**

Replace the current `title + observation.text()` signal scan with a deterministic helper that first receives body text with `script`, `style`, `noscript`, `title`, `meta`, and heading blocks removed, then:

1. Normalizes whitespace.
2. Splits on terminal punctuation (`.`, `!`, `?`, `。`, `！`, `？`).
3. Requires a complete sentence with at least eight whitespace-separated tokens.
4. Requires one explicit change action signal such as `announc`, `reorganiz`, `restructur`, `consolidat`, `appoint`, `moved`, `shift`, `launch`, `adopt`, `automat`, `replaced`, `moderniz`, `built`, `doubled`, `reduced`, or `increased`.
5. Requires one production-system signal such as `engineering`, `platform`, `product`, `operations`, `workflow`, `automation`, `ai`, `agent`, `data`, `cloud`, `research`, `development`, `scheduler`, `model`, `storage`, or `infrastructure`.
6. Returns the first bounded sentence and the matched production signal.

The candidate must use that sentence for `concrete_change`, `passage`, and `Provenance::source_field_or_passage`; retain the observed title separately in `source_title`.

- [ ] **Step 2: Run the sentence tests to verify GREEN**

Run:

```bash
cargo test --test weekly_radar_evidence_quality body_sentence_with_change_and_production_signals_creates_a_bounded_candidate -- --exact
cargo test --test weekly_radar_evidence_quality title_only_document_does_not_create_a_claim_candidate -- --exact
```

Expected: both pass, with the candidate containing only the first claim sentence.

- [ ] **Step 3: Add and run rejection cases**

Add tests for a heading-only body, a script-only body, a sentence with a production signal but no change signal, and an authoritative document with no effective date. Heading blocks (`h1` through `h6`) are document labels rather than body claims and must be removed with the other non-content blocks. Each must return `None` from extraction or fail validation at the correct existing gate. Run:

```bash
cargo test --test weekly_radar_evidence_quality -- evidence
```

Expected: all evidence extraction and validation tests pass, and no rejection path fabricates a value.

- [ ] **Step 4: Run SEC isolation and report metric regressions**

Run:

```bash
cargo test --test weekly_radar_evidence_quality sec_keeps_company_facts_when_submissions_request_fails -- --exact
cargo test --test weekly_radar_evidence_quality degraded_report_separates_evidence_and_source_availability_counts -- --exact
cargo test --test weekly_radar_evidence_quality localized_reports_keep_the_same_metric_values -- --exact
```

Expected: SEC Company Facts and report metric separation remain green.

- [ ] **Step 5: Commit the claim gate**

```bash
git add src/features/weekly_radar/runtime/evidence.rs tests/weekly_radar_evidence_quality.rs
git commit -m "fix: require sentence-level weekly radar claims"
```

### Task 4: Document the operator-visible evidence boundary

**Files:**
- Modify: `docs/operations/WEEKLY_RADAR.md`

**Interfaces:**
- Consumes: the implemented distinction between `DocumentCandidate`, `SourceObservation`, `EvidenceCandidate`, and `ValidatedEvidence`.
- Produces: operator guidance that explains why discovered documents can exceed validated evidence and why title/page availability is not a claim.

- [ ] **Step 1: Add the evidence promotion rules**

Document the exact flow:

```text
EntryPoint -> DocumentCandidate -> clean body passage -> EvidenceCandidate -> ValidatedEvidence
```

State that title-only pages, metadata/script payloads, headings, and ambiguous passages stay as discovery material; SEC facts follow their independent structured path.

- [ ] **Step 2: Run documentation checks**

Run:

```bash
make check-docs-metadata
```

Expected: documentation metadata validation passes.

- [ ] **Step 3: Commit the documentation**

```bash
git add docs/operations/WEEKLY_RADAR.md
git commit -m "docs: clarify weekly radar claim promotion"
```

### Task 5: Run full verification and update governed evidence

**Files:**
- Modify: `.ai/work-items/active/wi-weekly-radar-claim-extraction-gate.summary.json`
- Modify: `src/main.rs` (update the existing integration fixture to use a dated body claim)
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`, `.ai/knowledge/**`, `.ai/work-items/archive/**`

**Interfaces:**
- Consumes: all implementation, test, documentation, checkpoint, and CI evidence from Tasks 1–4.
- Produces: a validator-backed Summary and complete AI Cockpit Finish/Archive/PR/merge/close lifecycle.

- [ ] **Step 1: Run focused and project verification**

Run:

```bash
cargo test --test weekly_radar_evidence_quality
cargo test --lib
cargo test
make quality
```

Expected: exit code 0 with no test failures; record exact counts and command outputs in the Summary.

- [ ] **Step 2: Run the required AI Cockpit checks**

Run:

```bash
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-claim-extraction-gate.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-claim-extraction-gate.summary.json STAGE=before_finish
make ai-finish TASK=wi-weekly-radar-claim-extraction-gate REPORT_LANGUAGE=zh-CN
```

Expected: a separate localized Outcome begins with `Outcome: 🟢` only if all required checks and the Summary evidence are complete. If a gate fails, preserve the blocked Outcome and resolve the in-scope issue before retrying.

- [ ] **Step 3: Run pre-merge checks, commit, push, PR, and hosted CI**

Run:

```bash
make ai-pre-merge AI_BASE_COMMIT=ad641e0d9f1fad1d7f13d9676f8800cfd6abb14c
git status --short
git push -u origin codex/weekly-radar-claim-gate
gh pr create --base main --head codex/weekly-radar-claim-gate --title "收紧 Weekly Radar Claim 提取门" --body-file <(printf '%s\n' '## Summary' '' 'Require sentence-level production claims before evidence promotion.' '' '## Verification' '' '- cargo test' '- make quality' '- AI Cockpit lifecycle')
gh pr checks <PR_NUMBER> --watch
```

Expected: the PR head is the exact Work Item head; required CI jobs pass; no unrelated files are present.

- [ ] **Step 4: Merge and close the Work Item**

After hosted checks pass, merge the single PR and run:

```bash
gh pr merge <PR_NUMBER> --squash --delete-branch=false
make ai-close-work-item TASK=wi-weekly-radar-claim-extraction-gate
```

Expected: closure verifies the archived evidence, merged PR head, synchronized `main`, clean worktrees, and absent remote Work Item branch. Report the exact closure state and any residual risk.

- [ ] **Step 5: Trigger one post-merge Weekly Radar dry-run**

Run exactly once after merge:

```bash
gh workflow run weekly-radar.yml --repo xinglun/ORG-X --ref main \
  -f language=zh-CN -f as_of=2026-08-25 -f dry_run=true -f republish_published=false
```

Inspect the resulting run without rerunning it. Expected: the report still shows SEC availability and fail-closed Ranking, while title/page-only materials no longer count as promoted claim evidence.
