# Schedule Plan Residue Reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the last stale Sunday-default sentence from the historical Weekly Radar scheduler plan and keep the roadmap snapshot synchronized with archive sequence 41 before this Work Item becomes sequence 42.

**Architecture:** Documentation-only cleanup. The current scheduler specification, production workflow, and implementation are authoritative for Monday 09:00 JST; the historical plan is corrected to match them, while `.ai/work-items/archive/index.json` remains authoritative for lifecycle counts.

**Tech Stack:** Markdown, JSON, `rg`, `jq`, Git, AI Cockpit Make targets, and GitHub PR checks.

**Spec:** `docs/superpowers/specs/2026-08-17-wi-wr-013-weekly-scheduler.md` and `.ai/work-items/archive/index.json`

## Global Constraints

- Modify only the declared historical plan, roadmap snapshot, plan, and governed evidence.
- Do not change Rust, tests, workflows, configuration, dependencies, product scope, or dependency semantics.
- Use Monday 09:00 JST / UTC `0 0 * * 1` as the schedule wording authority.
- State 41 archived items before this Work Item and 42 after it archives.
- Complete the full Work Item lifecycle and verify archive sequence 42.

---

### Task 1: Correct the stale scheduler-plan wording

**Files:**
- Modify: `docs/superpowers/plans/2026-08-17-wi-wr-013-weekly-scheduler.md`
- Read: `docs/superpowers/specs/2026-08-17-wi-wr-013-weekly-scheduler.md`
- Read: `.github/workflows/weekly-radar.yml`

- [ ] **Step 1: Locate the stale statement**

Run:

```bash
rg -n -i 'default.*sunday|sunday.*default' docs/superpowers/plans/2026-08-17-wi-wr-013-weekly-scheduler.md
```

Expected: the plan's purpose sentence identifies Sunday as the default.

- [ ] **Step 2: Replace the statement with the current contract**

Change the purpose wording to state that the default is Monday, matching production 09:00 JST / 00:00 UTC, while retaining caller configurability.

- [ ] **Step 3: Verify schedule residue is gone**

Run:

```bash
! rg -n -i 'default.*sunday|sunday.*default' docs/superpowers/plans/2026-08-17-wi-wr-013-weekly-scheduler.md docs/superpowers/specs/2026-08-17-wi-wr-013-weekly-scheduler.md docs/operations/WEEKLY_RADAR.md .github/workflows/weekly-radar.yml
```

Expected: no stale Sunday-default match remains in the governed schedule source documents; descriptive text in this cleanup plan is outside that search surface.

### Task 2: Advance the roadmap snapshot to archive sequence 42

**Files:**
- Modify: `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md`
- Read: `.ai/work-items/archive/index.json`

- [ ] **Step 1: Confirm the current archive baseline**

Run:

```bash
jq '.entries | {count:length, latest:.[-1].archiveSequence}' .ai/work-items/archive/index.json
```

Expected: count and latest sequence are both `41`.

- [ ] **Step 2: Update the status table and snapshot paragraph**

Set the roadmap values to governance 15 before / 16 after, Weekly Radar 17, and total 41 before / 42 after. Retain the timestamped pre/post snapshot explanation and archive authority.

- [ ] **Step 3: Add this Work Item to the completed table**

Add:

```markdown
| `wi-schedule-plan-residue-reconciliation` | Remove stale Sunday-default scheduler-plan wording and reconcile the roadmap | Completed | `.ai/work-items/archive/2026/wi-schedule-plan-residue-reconciliation.*` |
```

Do not change historical completed rows or product/dependency sections.

### Task 3: Run governance verification and close the lifecycle

**Files:**
- Modify: `.ai/work-items/archive/**`
- Modify: `.ai/cockpit/current_status.md`
- Read: `.ai/work-items/archive/index.json`

- [ ] **Step 1: Verify the diff before finish**

Run:

```bash
git diff --check
git diff --name-only
jq '.entries | length' .ai/work-items/archive/index.json
```

Expected: no whitespace errors; only declared paths change; archive length is 41 before archive.

- [ ] **Step 2: Run finish and PR checks**

Run:

```bash
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-schedule-plan-residue-reconciliation.contract.json SUMMARY=.ai/work-items/active/wi-schedule-plan-residue-reconciliation.summary.json STAGE=before_finish
make ai-finish TASK=wi-schedule-plan-residue-reconciliation REPORT_LANGUAGE=zh-CN
make check-ai-pr AI_BASE_COMMIT=0eaa2c4a15b807821c540a1b9ba1218c3ffafbf0
```

Expected: required governance and documentation-quality checks pass.

- [ ] **Step 3: Commit, archive, merge, and close**

Use one commit and one PR, then run `make archive-work-item`, wait for hosted checks, merge, delete the remote branch, and run `make ai-close-work-item TASK=wi-schedule-plan-residue-reconciliation`.

- [ ] **Step 4: Verify sequence 42 and clean state**

Run:

```bash
git fetch origin
git merge --ff-only origin/main
jq '.entries | {count:length, latest:.[-1]}' .ai/work-items/archive/index.json
git status --short --branch
git worktree list
```

Expected: archive count and latest sequence are 42, root is synchronized and clean, and no Work Item worktree or branch residue remains.
