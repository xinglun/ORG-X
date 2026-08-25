# Weekly Radar Independent Diffusion Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Require independently corroborated adoption evidence before ORG-X can label an AI-era company an industry reference model.

**Architecture:** Add typed provenance roles at the Rust source/evidence boundary, carry the optional role through normalized facts and the existing four-family gate, and render precomputed role counts. Explicit customer-owned URLs are collected through the existing bounded document path; supplier URLs remain available only as attribution.

**Tech Stack:** Rust 2024, `serde`, `chrono`, existing injected HTTP fixtures, Cargo tests, Markdown report rendering, Python/Shell AI Cockpit orchestration.

**Spec:** `docs/superpowers/specs/2026-08-26-weekly-radar-independent-diffusion.md`

## Global Constraints

- Supplier-controlled material never satisfies independent diffusion by itself.
- Independent diffusion requires two authoritative independent source URIs and two named adopters/peers.
- SEC/IR outcome evidence remains separate from customer diffusion evidence.
- Only explicitly configured URLs may be fetched; no guessed cross-origin URL or unbounded crawl.
- Legacy JSON defaults the new optional provenance role to unknown/non-independent.
- Rust owns classification, evidence metadata, gate semantics, Stage assignment, and report values.
- Tests use injected fixtures; no live network, credentials, secret values, Telegram, or data-branch writes.
- Every behavior change follows RED → observed failure → GREEN → observed pass.

### Task 1: Add source-role semantics to the domain gate

**Files:**
- Modify: `src/features/transformation/domain/mod.rs`
- Test: `src/features/transformation/domain/mod_test.rs`

**Interfaces:**
- Add public serializable `ReferenceModelSourceRole` with supplier, independent customer, regulatory/filing, and discovery-only variants.
- Add `source_role()` to `ReferenceModelEvidence` and a role-aware constructor while preserving `new()` as a legacy unknown-role constructor.
- Add `supplier_attribution_sources()` to `ReferenceModelAssessment`.

- [ ] **Step 1: Write RED tests.** Add a supplier-only diffusion fixture with two named peers and assert `Candidate` plus `independent_diffusion_sources` missing; add two independent customer-role claims and assert the complete bundle is `Confirmed`; assert role serialization and legacy constructor behavior.
- [ ] **Step 2: Run RED.** Run `cargo test transformation::domain::mod_test --lib` and confirm the new role APIs/expectations fail before production changes.
- [ ] **Step 3: Implement minimal domain changes.** Store the role on evidence, default legacy `new()` to unknown, count only independent customer claims for diffusion, count supplier attribution separately, and preserve the existing family/outcome/counter rules.
- [ ] **Step 4: Run GREEN.** Re-run the focused domain tests and the existing transformation-domain integration tests; confirm all pass.
- [ ] **Step 5: Commit.** `git add src/features/transformation/domain/mod.rs src/features/transformation/domain/mod_test.rs && git commit -m "feat: gate diffusion on independent source roles"`

### Task 2: Carry provenance roles through runtime evidence

**Files:**
- Modify: `src/features/weekly_radar/runtime/model.rs`
- Modify: `src/features/weekly_radar/runtime/evidence.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Add optional `reference_model_source_role` to `NormalizedFact`, with `serde(default)` and accessors.
- Classify `SourceKind::OfficialResearch` as supplier attribution and the new independent source kind as independent customer disclosure.
- Preserve role metadata when `ValidatedEvidence` becomes a normalized fact.

- [ ] **Step 1: Write RED tests.** Add tests proving supplier customer stories retain attribution but are not independent, independent customer documents become independent-role validated facts, and old JSON without the role deserializes to `None`.
- [ ] **Step 2: Run RED.** Run `cargo test --test weekly_radar_evidence_quality reference_model_source_role` and confirm failure on the missing type/metadata.
- [ ] **Step 3: Implement minimal propagation.** Add the optional field and builder/accessor, role-aware evidence candidate metadata, source-kind mapping, and legacy-safe deserialization without changing claim extraction predicates.
- [ ] **Step 4: Run GREEN.** Run the focused evidence tests plus the existing claim-extraction tests.
- [ ] **Step 5: Commit.** `git add src/features/weekly_radar/runtime/model.rs src/features/weekly_radar/runtime/evidence.rs tests/weekly_radar_evidence_quality.rs && git commit -m "feat: preserve evidence provenance roles"`

### Task 3: Add explicit bounded independent source collection

**Files:**
- Modify: `src/features/weekly_radar/runtime/config.rs`
- Modify: `src/features/weekly_radar/runtime/sources.rs`
- Modify: `config/weekly_radar/reference_model_candidates.json`
- Test: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Add `CompanyConfig.independent_research_sources` with validation and accessor.
- Add `SourceKind::IndependentResearch` and authoritative `SourceTier::IndependentPrimary`.
- Parameterize the existing document observation/collection path by tier, preserving the bounded discovery limits and explicit URL validation.

- [ ] **Step 1: Write RED tests.** Add a fixture with a cross-origin explicitly configured independent URL and assert it is collected as `IndependentResearch`/`IndependentPrimary`; assert an undeclared URL is never guessed or fetched and legacy config without the list remains valid.
- [ ] **Step 2: Run RED.** Run `cargo test --test weekly_radar_runtime independent_research` and confirm the config field/source tier/path is absent.
- [ ] **Step 3: Implement minimal collection.** Add the list and validation, call the existing bounded official-document path with `IndependentPrimary`, and pass the tier into document/status observations. Keep supplier URLs in `official_research_sources`.
- [ ] **Step 4: Update configuration.** Add the PwC self-disclosure and NIQ IR URL only to Microsoft's explicit independent list; do not add guessed URLs to other companies.
- [ ] **Step 5: Run GREEN.** Run focused runtime/config tests and existing source collection tests.
- [ ] **Step 6: Commit.** `git add src/features/weekly_radar/runtime/config.rs src/features/weekly_radar/runtime/sources.rs config/weekly_radar/reference_model_candidates.json tests/weekly_radar_runtime.rs && git commit -m "feat: collect bounded independent research sources"`

### Task 4: Apply roles in judgment and report

**Files:**
- Modify: `src/features/weekly_radar/runtime/judgment.rs`
- Modify: `src/features/weekly_radar/runtime/report.rs`
- Test: `tests/weekly_radar_judgment_chain.rs`
- Test: `tests/weekly_radar_evidence_quality.rs`

**Interfaces:**
- Pass `NormalizedFact::reference_model_source_role()` to the domain evidence constructor.
- Render supplier and independent counts/provenance from `ReferenceModelAssessment`; never recompute the gate in the renderer.

- [ ] **Step 1: Write RED tests.** Add supplier-only and independent-role judgment fixtures; assert only the latter can reach `Confirmed`/`REFERENCE_MODEL`, and assert Chinese, Japanese, and English reports expose separate counts and missing proof.
- [ ] **Step 2: Run RED.** Run `cargo test --test weekly_radar_judgment_chain independent_diffusion` and confirm the supplier-only fixture incorrectly passes or the new report fields are absent.
- [ ] **Step 3: Implement minimal judgment/report changes.** Construct role-aware evidence, retain assessment counts, add localized labels, and keep `ReferenceModelEligibility::Candidate`/no-ranking behavior unchanged for incomplete packets.
- [ ] **Step 4: Run GREEN.** Run focused judgment/report tests and all existing report localization tests.
- [ ] **Step 5: Commit.** `git add src/features/weekly_radar/runtime/judgment.rs src/features/weekly_radar/runtime/report.rs tests/weekly_radar_judgment_chain.rs tests/weekly_radar_evidence_quality.rs && git commit -m "feat: expose independent diffusion provenance"`

### Task 5: Documentation, governance evidence, and full verification

**Files:**
- Modify: `docs/domain/PRODUCTION_SYSTEM_MODEL.md`
- Modify: `docs/validation/VALIDATION_STRATEGY.md`
- Modify: `docs/operations/WEEKLY_RADAR.md`
- Create: `.ai/evidence/reference-impact/wi-weekly-radar-independent-diffusion-config.json`
- Modify: `.ai/work-items/active/wi-weekly-radar-independent-diffusion.summary.json`
- Modify: generated Cockpit projections only through Make targets.

- [ ] **Step 1: Document the semantic boundary.** State that source availability, supplier attribution, independent customer disclosure, regulatory/IR result, validated fact, structural evidence, and reference-model confirmation are separate states.
- [ ] **Step 2: Run local quality.** Run `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, `pytest -q`, and `git diff --check`; fix any issue in this Work Item and rerun the failing check.
- [ ] **Step 3: Run governance quality.** Run `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`, all Contract-required checks, and `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-independent-diffusion.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-independent-diffusion.summary.json STAGE=before_finish`.
- [ ] **Step 4: Run a bounded local radar result.** Execute the existing dry-run against fixture/configured inputs, inspect role counts and the fail-closed result, and record evidence without claiming a positive reference model unless the independent gate actually passes.
- [ ] **Step 5: Finish and archive.** Run `make ai-finish TASK=wi-weekly-radar-independent-diffusion REPORT_LANGUAGE=zh-CN`; resolve every reported problem before archive, then archive the Work Item.

### Task 6: Complete hosted lifecycle

- [ ] **Step 1:** Run archived-branch `make check-ai-pr` and push the branch as `xinglun`.
- [ ] **Step 2:** Create one PR for this Work Item and wait for required GitHub Actions checks.
- [ ] **Step 3:** If CI fails, inspect the failing job, amend the current Work Item, rerun local checks, finish evidence, and push the repair.
- [ ] **Step 4:** Merge only after required checks are green; verify the merged head SHA.
- [ ] **Step 5:** Delete the merged remote branch, synchronize the base fast-forward-only, run `make ai-close-work-item TASK=wi-weekly-radar-independent-diffusion`, and verify clean worktrees/branches.
