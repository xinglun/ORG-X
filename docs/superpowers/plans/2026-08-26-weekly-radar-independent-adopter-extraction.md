# Weekly Radar Independent Adopter Extraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the already-configured Atos independent disclosure count as a named independent diffusion source without weakening ORG-X's evidence or Ranking gates.

**Architecture:** Keep the change in the Rust evidence-extraction boundary. Add a sentence-local, bounded named-adopter pattern for infinitive deployment prose, prove it with a deterministic red/green fixture, and leave source roles, the four-family gate, counter review, and Ranking untouched.

**Tech Stack:** Rust (`regex`, existing Weekly Radar evidence model), Cargo integration tests, Markdown operations/spec documentation, Python/Shell AI Cockpit verification.

**Spec:** `docs/superpowers/specs/2026-08-26-weekly-radar-independent-adopter-extraction.md`

## Global Constraints

- Only explicitly configured bounded sources may enter the runtime.
- `IndependentCustomerDisclosure` remains distinct from `SupplierAttribution`.
- No source availability observation may become validated evidence without the existing date, substance, and claim gates.
- No Ranking is emitted unless the existing complete reference-model contract passes.
- Python and Shell are limited to orchestration and governance verification; evidence semantics remain in Rust.

### Task 1: Reproduce the Atos named-adopter defect

**Files:**
- Modify: `tests/weekly_radar_evidence_quality.rs`
- Read: `src/features/weekly_radar/runtime/evidence.rs`

**Interfaces:**
- Consumes: `extract_evidence_candidate`, `validate_evidence_candidate`, `DocumentObservationInput`.
- Produces: A focused regression test for an independent Atos disclosure.

- [ ] **Step 1: Write the failing test**

Add one fixture with title/body context matching the live disclosure, including `Atos Group becomes the first French Global System Integrator to deploy Microsoft 365 Copilot`. Assert the candidate validates, is `IndustryDiffusion`, and currently fails because `reference_model_named_peer()` is `None`.

- [ ] **Step 2: Run the focused test to verify RED**

Run:

```bash
cargo test --test weekly_radar_evidence_quality independent_customer_infinitive_deployment_promotes_named_adopter -- --exact
```

Expected: FAIL at the named-adopter assertion, proving the fixture catches the live defect rather than passing against existing behavior.

### Task 2: Implement the minimal bounded matcher change

**Files:**
- Modify: `src/features/weekly_radar/runtime/evidence.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Consumes: Existing `reference_model_named_peer_for_text` direct-verb matcher and explicit diffusion vocabulary.
- Produces: Named-adopter extraction for a bounded copular/change phrase followed by an infinitive adoption/deployment verb.

- [ ] **Step 1: Add the smallest production pattern**

Extend `reference_model_named_peer_for_text` with a sentence-local regex that captures a capitalized adopter name of at most five words, followed by a bounded `is/are/was/were/becomes/became` phrase and an explicit `to deploy`, `to roll out`, `to adopt`, `to implement`, or `to use` action. Keep the existing direct-verb pattern unchanged.

- [ ] **Step 2: Run the focused test to verify GREEN**

Run the same exact Cargo command. Expected: PASS with named peer `Atos Group`, `IndustryDiffusion`, `IndependentCustomerDisclosure`, configured URI, and effective date preserved.

- [ ] **Step 3: Run regression tests**

Run:

```bash
cargo test --test weekly_radar_evidence_quality --quiet
cargo test --test weekly_radar_judgment_chain --quiet
```

Expected: all tests pass, including supplier-only suppression and no-Ranking behavior.

### Task 3: Document and verify the boundary

**Files:**
- Modify: `docs/operations/WEEKLY_RADAR.md`
- Read: `docs/superpowers/specs/2026-08-26-weekly-radar-independent-adopter-extraction.md`

**Interfaces:**
- Consumes: The accepted matcher boundary and existing source-role semantics.
- Produces: Operational documentation stating the supported infinitive deployment construction and its limits.

- [ ] **Step 1: Update the operational note**

Document that independent adopter extraction supports bounded direct verbs and sentence-local copular phrases leading to explicit infinitive deployment/adoption verbs; state that this does not alter source roles or the gate.

- [ ] **Step 2: Run project verification**

Run the Contract-declared Rust, Python, Shell, and AI Cockpit checks. Record exact outputs in the Summary and use `make ai-finish TASK=wi-weekly-radar-independent-adopter-extraction REPORT_LANGUAGE=zh-CN` before archive.

### Task 4: Complete the governed delivery lifecycle

**Files:**
- Modify: governed Work Item records and generated projections only.

**Interfaces:**
- Consumes: Passing local checks, archived evidence, exact base/head SHA bindings, and hosted CI.
- Produces: Merged PR, closure receipt, clean synchronized `main`, and a merged-main dry-run result.

- [ ] **Step 1: Run `ai-finish` and archive the Work Item**
- [ ] **Step 2: Run `check-ai-pr` against base `13a445579389a9cd530a7c97037671f92e5aae73`**
- [ ] **Step 3: Push, open PR, wait for all required CI checks, and merge**
- [ ] **Step 4: Run `make ai-close-work-item TASK=wi-weekly-radar-independent-adopter-extraction`**
- [ ] **Step 5: Trigger the documented dry-run from merged `main` and inspect Microsoft source counts, missing proof, qualification, and Ranking output**
