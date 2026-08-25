# Weekly Radar Structural Evidence Gate Implementation Plan

> For agentic workers: REQUIRED SUB-SKILL: Use superpowers:executing-plans or superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Make Weekly Radar distinguish regular validated facts from structural evidence, report SEC stage reachability separately from usable SEC facts, and preserve fail-closed judgment/Ranking behavior.

**Architecture:** Keep the existing SourceObservation → EvidenceCandidate → ValidatedEvidence flow. Add a deterministic EvidenceClass decision inside the evidence runtime, map the decision to stable normalized-fact prefixes, and expose backward-compatible research counters through ResearchMetrics. The report renderer consumes those counters and prefixes read-only; the semantic splitter accepts both new and legacy headings.

**Tech Stack:** Rust stable, serde, chrono, existing fixture HTTP client, Cargo tests, AI Cockpit Make targets.

**Spec:** docs/superpowers/specs/2026-08-25-weekly-radar-structural-evidence-gate-design.md

## Global Constraints

- Use deterministic, bounded, provider-neutral lexical rules; add no dependency, LLM, provider, or unbounded crawl.
- Preserve SourceObservation → EvidenceCandidate → ValidatedEvidence; StructuralEvidence is a second classification, not a replacement for validation.
- New ResearchMetrics fields deserialize as zero for legacy snapshots.
- Keep judgment rules, Transformation Stage definitions, Ranking thresholds, Telegram delivery, archive persistence, and workflow publication unchanged.
- Prefer false negatives to promoting generic technical prose as structural enterprise change.
- Every production behavior change gets a failing test before implementation and a focused green run immediately afterward.

---

### Task 1: Add the StructuralEvidence classification boundary

**Files:**
- Modify: src/features/weekly_radar/runtime/evidence.rs
- Test: tests/weekly_radar_evidence_quality.rs
- Test: tests/evidence_test.rs

**Interfaces:**
- Consumes: existing EvidenceCandidate fields concrete_change, production_area, source_kind, source_tier, source_title, and passage.
- Produces: public EvidenceClass::{ValidatedFact, StructuralEvidence}, ValidatedEvidence::evidence_class(), and normalized fact kinds evidence_official_material_<index> or evidence_structural_change_<index>.

- [ ] Write a failing structural-positive test. Add a dated candidate with source details “Acme consolidated production scheduling under one platform.” Assert evidence_class() is StructuralEvidence and to_normalized_fact(1).kind() starts with evidence_structural_change_.
- [ ] Run the focused test:
  
  ~~~bash
  cargo test --test weekly_radar_evidence_quality explicit_production_system_change_becomes_structural_evidence -- --exact
  ~~~
  
  Expected: compilation failure because EvidenceClass and evidence_class() do not exist.
- [ ] Write a failing structural-negative test. Add a dated candidate with “The research model shifted representation modeling for long-range graph topologies.” Assert evidence_class() is ValidatedFact and the normalized kind starts with evidence_official_material_.
- [ ] Run the negative test and confirm the same missing-API RED failure.
- [ ] Implement EvidenceClass, a private structural-signal classifier, ValidatedEvidence::evidence_class(), and the kind selection in to_normalized_fact(). Require an existing change action plus organization/operating-model, production/workflow/deployment, or measurable operating-impact signals. Do not treat research, architecture, model, or product alone as structural.
- [ ] Run:
  
  ~~~bash
  cargo test --test weekly_radar_evidence_quality explicit_production_system_change_becomes_structural_evidence generic_research_description_remains_a_regular_validated_fact -- --exact
  cargo test --test evidence_test
  ~~~
- [ ] Commit with git add src/features/weekly_radar/runtime/evidence.rs tests/weekly_radar_evidence_quality.rs tests/evidence_test.rs && git commit -m "feat: classify structural weekly radar evidence".

### Task 2: Add backward-compatible structural and SEC health metrics

**Files:**
- Modify: src/features/weekly_radar/runtime/model.rs
- Test: tests/weekly_radar_evidence_quality.rs
- Test: tests/weekly_radar_runtime.rs

**Interfaces:**
- Consumes: existing ResearchMetrics::new(source_available, document_candidates, validated_evidence, pending_leads, unavailable_sources).
- Produces: with_structural_evidence, with_sec_health, and getters for structural evidence, SEC stage expected/available, and SEC fact expected/available. The old constructor and old snapshots remain valid.

- [ ] Write a failing test constructing ResearchMetrics::new(9, 10, 5, 71, 32).with_structural_evidence(2).with_sec_health(20, 18, 80, 74), then assert all five new getters.
- [ ] Add a legacy JSON fixture that omits all new fields and asserts all new counters deserialize as zero.
- [ ] Run the focused tests and verify RED because the methods do not exist.
- [ ] Add serde-default, skip-zero fields; keep the existing five-argument new constructor; add const builder/getter methods. Preserve RuntimeReportInput legacy deserialization.
- [ ] Run:
  
  ~~~bash
  cargo test --test weekly_radar_evidence_quality research_metrics_retain_structural_and_sec_health_counts legacy_runtime_input_defaults_research_metrics_to_zero -- --exact
  cargo test --test weekly_radar_runtime task5_input_snapshot_round_trips_and_is_idempotent -- --exact
  ~~~
- [ ] Commit with git add src/features/weekly_radar/runtime/model.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs && git commit -m "feat: expose weekly radar evidence health metrics".

### Task 3: Bind classification and SEC health during acquisition

**Files:**
- Modify: src/main.rs
- Test: tests/weekly_radar_evidence_quality.rs

**Interfaces:**
- Consumes: ValidatedEvidence::evidence_class(), SecClient::collect(), normalized SEC facts, and the current acquisition loop.
- Produces: ResearchMetrics with structural count and distinct SEC stage/fact counters; has_primary_evidence behavior remains unchanged.

- [ ] Extend the validated-document integration fixture with assertions for validated_evidence() == 1 and structural_evidence() == 1.
- [ ] Add a partial SEC fixture assertion for SEC stage expected/available and fact expected/available, including a fact-level unavailable result when the filing document is absent.
- [ ] Run the new acquisition tests and verify RED because acquisition does not populate new counters.
- [ ] In acquire_runtime_input, track structural_evidence separately, count two SEC stages per configured CIK, count a stage once based on distinct submissions/company_facts failure stages, count SEC fact expected from evidence.facts().len(), and count SEC fact available from FactStatus::Known. Preserve existing source coverage, source failures, fact insertion, and primary-evidence behavior.
- [ ] Run:
  
  ~~~bash
  cargo test --test weekly_radar_evidence_quality validated_document_claim_is_counted_and_can_feed_judgment sec_health_distinguishes_reachable_stages_from_usable_facts -- --exact
  cargo test --test weekly_radar_judgment_chain insufficient_evidence_is_undetermined_and_has_no_machine_ranking -- --exact
  ~~~
- [ ] Commit with git add src/main.rs tests/weekly_radar_evidence_quality.rs && git commit -m "feat: bind structural and SEC health metrics".

### Task 4: Separate validated facts and structural evidence in reports

**Files:**
- Modify: src/features/weekly_radar/runtime/report.rs
- Test: tests/weekly_radar_evidence_quality.rs
- Test: tests/weekly_radar_runtime.rs

**Interfaces:**
- Consumes: fact-kind prefixes, ResearchMetrics, FactStatus, and localized report inputs.
- Produces: distinct validated-fact and structural-evidence sections, equal zh-CN/ja/en metric values, explicit SEC stage/fact health lines, and calibrated no-change/degraded wording.

- [ ] Update the report fixture with one evidence_official_material_001 and one evidence_structural_change_002. First assert the new headings and metric labels:
  
  ~~~rust
  assert!(report.markdown().contains("## 已验证事实"));
  assert!(report.markdown().contains("## 结构性证据"));
  assert!(report.markdown().contains("本周新增已验证事实：2"));
  assert!(report.markdown().contains("本周新增结构性证据：1"));
  assert!(report.markdown().contains("SEC 可用事实"));
  ~~~
  
  Also assert the structural claim is absent from the ordinary validated-fact section.
- [ ] Add Japanese and English metric assertions plus a degraded report assertion preserving the sentence that absence is not proof of no change.
- [ ] Run the report tests and verify RED because the old headings/labels are still rendered.
- [ ] Add helpers separating regular evidence_official_material_ facts from evidence_structural_change_ facts. Render regular validated facts under localized Validated Facts headings and structural facts under localized Structural Evidence headings. Keep SEC and source observations out of both sections.
- [ ] Add structural count and SEC stage/fact lines to all three language branches. Change the executive summary wording from confirmed information to validated facts and from confirmed organizational changes to structural change evidence. Keep existing domain structural_change and Ranking reference rendering unchanged.
- [ ] Run:
  
  ~~~bash
  cargo test --test weekly_radar_evidence_quality confirmed_information_contains_only_validated_evidence localized_reports_keep_validated_evidence_separate_from_known_facts degraded_report_separates_evidence_and_source_availability_counts -- --exact
  cargo test --test weekly_radar_runtime task7_default_report_is_chinese_and_hides_runtime_diagnostics_from_readers task8_report_does_not_turn_unavailable_structural_evidence_into_no_change -- --exact
  ~~~
- [ ] Commit with git add src/features/weekly_radar/runtime/report.rs tests/weekly_radar_evidence_quality.rs tests/weekly_radar_runtime.rs && git commit -m "feat: separate validated facts from structural evidence".

### Task 5: Preserve semantic Telegram splitting for new and legacy headings

**Files:**
- Modify: src/features/weekly_radar/interface/semantic_message_splitter.rs
- Test: tests/weekly_radar_semantic_message_splitter.rs
- Test: tests/semantic_message_splitter_test.rs

**Interfaces:**
- Consumes: localized report headings produced by Task 4.
- Produces: complete semantic chunks for new validated-fact and structural-evidence headings, plus compatibility for previous confirmed-information aliases.

- [ ] Write a failing fixture containing ## 已验证事实 and ## 结构性证据. Assert the first chunk remains ExecutiveSummary, the structural section is ImportantTransition, and source Markdown is unchanged. Repeat with Japanese and English aliases.
- [ ] Run cargo test --test weekly_radar_semantic_message_splitter --test semantic_message_splitter_test and verify UnknownSection for new headings.
- [ ] Map Validated Facts/已验证事实/検証済み事実 to ExecutiveSummary; map Structural Evidence/结构性证据/構造的証拠 to ImportantTransition; retain every existing Confirmed Information alias. Do not parse or rewrite fact content.
- [ ] Run the two splitter integration tests and verify GREEN.
- [ ] Commit with git add src/features/weekly_radar/interface/semantic_message_splitter.rs tests/weekly_radar_semantic_message_splitter.rs tests/semantic_message_splitter_test.rs && git commit -m "feat: accept structural evidence report sections".

### Task 6: Update operations documentation and governance evidence

**Files:**
- Modify: docs/operations/WEEKLY_RADAR.md
- Modify: docs/superpowers/specs/2026-08-25-weekly-radar-structural-evidence-gate-design.md
- Modify: docs/superpowers/plans/2026-08-25-weekly-radar-structural-evidence-gate.md
- Modify: .ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json

**Interfaces:**
- Consumes: implemented evidence/report/SEC behavior and focused test evidence.
- Produces: operator documentation stating the four evidence layers, structural gate, SEC stage/fact semantics, exact verification results, and residual risks.

- [ ] Add the new terms StructuralEvidence, evidence_structural_change_, SEC stage, usable facts, 待验证线索, and 不等于 to the operations guide.
- [ ] Update the active Summary with changed files, exact checks, guideline compliance, risks, unknowns, and evidence-bound limitations. Do not claim the post-merge dry-run until it actually runs.
- [ ] Run cargo fmt --all -- --check, the focused suites, and make quality.
- [ ] Run make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-structural-evidence-gate.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-structural-evidence-gate.summary.json STAGE=before_finish.
- [ ] Run make ai-finish TASK=wi-weekly-radar-structural-evidence-gate REPORT_LANGUAGE=zh-CN and keep its Outcome as the completion authority.
- [ ] Commit the documentation and generated governance projections only after their checks pass.

### Task 7: Hosted lifecycle and authorized post-merge dry-run

**Files:**
- Generated: .ai/work-items/archive/**
- Generated: .ai/knowledge/**

**Interfaces:**
- Consumes: finished branch, local verification, hosted CI, merged PR, and explicit user-authorized Weekly Radar dry_run dispatch.
- Produces: one merged Work Item, clean closure, and one linked post-merge dry-run result with no Telegram/data-branch writes.

- [ ] Run make check-ai-pr against the recorded base commit.
- [ ] Push the dedicated branch and create one PR.
- [ ] Wait for required hosted checks and merge the PR without provider-side branch deletion.
- [ ] Run make ai-close-work-item TASK=wi-weekly-radar-structural-evidence-gate and verify clean branch/worktree/remote state.
- [ ] From synchronized main, dispatch Weekly Radar with language=zh-CN and dry_run=true; record the workflow URL, conclusion, report metrics, SEC stage/fact metrics, structural evidence count, and Ranking behavior in the final handoff.

