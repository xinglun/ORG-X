# ORG-X Reader Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize ORG-X documentation into a reader-first product and operations guide without deleting internal engineering records.

**Architecture:** Keep the existing stable document paths and add one reader index at `docs/README.md`. Use `README.md` as the short entry point, focused documents as the canonical detail pages, and links rather than duplicated process narratives.

**Tech Stack:** Markdown, repository links, existing Rust/Actions/runtime documentation as factual sources, AI Cockpit Make checks.

## Global Constraints

- Reader-facing prose is primarily Chinese; stable code identifiers and domain terms remain English.
- Reader docs must not contain delivery status, Work Item progress, next-step planning, or implementation chronology.
- Rule-only extraction, `UNKNOWN`/`UNAVAILABLE`, primary-source authority, no paid APIs, non-trading scope, Monday 09:00 JST, `actions/checkout@v5`, Telegram secrets, and `data`-branch retention must remain factually aligned.
- Existing `docs/adr/**`, `docs/superpowers/**` records, source code, tests, CI, secrets, and runtime behavior are not changed except the new design/plan records declared by the Contract.

---

### Task 1: Build the reader entry path

**Files:**
- Modify: `README.md`
- Modify: `NORTH_STAR.md`
- Modify: `ENGINEERING_PRINCIPLES.md`
- Create: `docs/README.md`

- [ ] **Step 1: Replace the root delivery-status section with purpose, boundary, method, output, and reader links.**
- [ ] **Step 2: Align root North Star and engineering principles with rule-only extraction and current evidence boundaries.**
- [ ] **Step 3: Add `docs/README.md` with ordered reading routes and links to existing detail documents.**
- [ ] **Step 4: Check every new relative link against the repository file list.**

### Task 2: Rewrite product, evidence, and data explanations

**Files:**
- Modify: `docs/product/NORTH_STAR.md`
- Modify: `docs/product/PRD.md`
- Modify: `docs/product/SCOPE.md`
- Modify: `docs/data/DATA_QUALITY_POLICY.md`
- Modify: `docs/data/DATA_SOURCE_POLICY.md`
- Modify: `docs/domain/EVIDENCE_MODEL.md`
- Modify: `docs/domain/PRODUCTION_SYSTEM_MODEL.md`
- Modify: `docs/domain/TRANSFORMATION_STAGE_MODEL.md`
- Modify: `docs/domain/RANKING_MODEL.md`

- [ ] **Step 1: Give each product document one reader question and remove implementation-history language.**
- [ ] **Step 2: Replace the generic LLM extraction sequence in reader documentation with the approved rule-only extraction boundary.**
- [ ] **Step 3: Make authority, provenance, `UNKNOWN`, `UNAVAILABLE`, counter-evidence, Stage, and Ranking terminology consistent.**
- [ ] **Step 4: Keep domain documents provider-neutral while linking source policy details to the data documents.**

### Task 3: Rewrite architecture, scoring, validation, and operations explanations

**Files:**
- Modify: `docs/architecture/ARCHITECTURE.md`
- Modify: `docs/architecture/BOUNDED_CONTEXTS.md`
- Modify: `docs/architecture/DEPENDENCY_RULES.md`
- Modify: `docs/scoring/SCORING_SPEC.md`
- Modify: `docs/scoring/STAGE_GATE_SPEC.md`
- Modify: `docs/validation/VALIDATION_STRATEGY.md`
- Modify: `docs/operations/WEEKLY_RADAR.md`

- [ ] **Step 1: Remove Phase/Work Item narration from architecture and explain current boundaries directly.**
- [ ] **Step 2: Present Stage gates and Score as separate reader concepts with explicit ordering.**
- [ ] **Step 3: Organize Weekly Radar around prerequisites, commands, configuration, source behavior, Telegram output, and data retention.**
- [ ] **Step 4: Compare operations text with `.github/workflows/weekly-radar.yml` and runtime source.**

### Task 4: Review and verify the documentation surface

**Files:**
- Modify: `.ai/work-items/active/wi-docs-reader.summary.json`
- Generated: `.ai/cockpit/current_status.md`

- [ ] **Step 1: Run the five independent documentation review strategies and record only consensus Critical/High findings.**
- [ ] **Step 2: Run Markdown metadata checks, link/path checks, contradiction searches, and the full project quality gate.**
- [ ] **Step 3: Update Summary with changed files, evidence, guideline compliance, scenario coverage, risks, and residual unknowns.**
- [ ] **Step 4: Run `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-docs-reader.contract.json SUMMARY=.ai/work-items/active/wi-docs-reader.summary.json STAGE=before_finish`.**

### Task 5: Complete the governed lifecycle

**Files:**
- Generated: `.ai/work-items/archive/**`
- Generated: `target/task-closure-receipts/**`

- [ ] **Step 1: Run `make ai-finish TASK=wi-docs-reader REPORT_LANGUAGE=zh-CN`.**
- [ ] **Step 2: Archive the active Work Item and run `make check-ai-pr AI_BASE_COMMIT=65531d9ffa874dc386d366473ad07dbe893bcdd5`.**
- [ ] **Step 3: Push the dedicated branch, open one PR, wait for hosted checks, and merge it.**
- [ ] **Step 4: Run `make ai-close-work-item TASK=wi-docs-reader`, then verify the merged base, archive, closure receipt, clean worktrees, and absent remote branch.**
