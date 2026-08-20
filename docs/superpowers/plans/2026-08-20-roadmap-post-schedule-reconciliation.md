# Roadmap Post-Schedule Reconciliation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Synchronize the human-facing ORG-X roadmap with the 40 archived Work Items that exist after the merged schedule source-of-truth correction, while recording the governed reconciliation itself as the next lifecycle item.

**Architecture:** This is a documentation-only governance reconciliation. `.ai/work-items/archive/index.json` and the archived Contract/Summary/Outcome records remain authoritative; the roadmap is updated only as a timestamped pre/post-archive snapshot and does not redefine product or dependency boundaries.

**Tech Stack:** Markdown, JSON, `jq`, Git, AI Cockpit Make targets, and GitHub PR checks.

**Spec:** `.ai/work-items/archive/2026/wi-weekly-radar-schedule-source-of-truth.contract.json` and `.ai/work-items/archive/index.json`

## Global Constraints

- Archive authority: use `.ai/work-items/archive/index.json` and archived records for status and count claims.
- Scope: modify only the roadmap, this plan, and governed Work Item evidence.
- Product boundary: do not change Rust, tests, workflows, configuration, dependencies, product scope, or dependency semantics.
- Snapshot wording: state 40 archived items before this Work Item and 41 after it archives.
- Lifecycle: complete Contract, checkpoint, finish, archive, PR, merge, close, and clean-state verification.

---

### Task 1: Validate the stale snapshot against archive evidence

**Files:**
- Read: `.ai/work-items/archive/index.json`
- Read: `.ai/work-items/archive/2026/wi-weekly-radar-schedule-source-of-truth.contract.json`
- Read: `.ai/work-items/archive/2026/wi-weekly-radar-schedule-source-of-truth.outcome.md`
- Read: `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md`

**Interfaces:**
- Consumes: archive sequence 40 and the merged schedule Work Item evidence.
- Produces: exact count/table edits for Task 2.

- [ ] **Step 1: Confirm archive sequence and latest Work Item identity**

Run:

```bash
jq '.entries[-1] | {workItemId, archiveSequence, contractPath, summaryPath, manifestPath}' .ai/work-items/archive/index.json
```

Expected: `workItemId` is `wi-weekly-radar-schedule-source-of-truth` and `archiveSequence` is `40`.

- [ ] **Step 2: Confirm the roadmap is stale**

Run:

```bash
rg -n '38|39|Weekly Radar WI|治理、产品与维护|Completed Work Items|Execution handoff' docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md
```

Expected: the roadmap contains the pre-schedule count snapshot and does not list the schedule Work Item.

### Task 2: Update the roadmap snapshot and completed table

**Files:**
- Modify: `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md`
- Read: `.ai/work-items/archive/index.json`

**Interfaces:**
- Consumes: Task 1's archive sequence and exact latest Work Item identity.
- Produces: a roadmap whose displayed snapshot is 14/15 governance items, 9 core items, 17 Weekly Radar items, and 40/41 total archived items.

- [ ] **Step 1: Update the status table and snapshot paragraph**

Change only the current snapshot values:

```markdown
| 已完成治理、产品与维护 WI | 14（本 WI 归档后为 15） | Completed / archived |
| Weekly Radar WI | 17 | Completed / archived |
| 已归档 Work Item 合计 | 40（本 WI 归档后为 41） | 以 `archive/index.json` 为准 |
```

The following paragraph must state that 40 items are confirmed before this Work Item and 41 after its archive, while retaining `archive/index.json` as the authority.

- [ ] **Step 2: Add the completed schedule row and this reconciliation row**

Add these rows to the Completed Work Items table:

```markdown
| `wi-weekly-radar-schedule-source-of-truth` | Align reusable Weekly Radar default schedule with production Monday 09:00 JST | Completed | `.ai/work-items/archive/2026/wi-weekly-radar-schedule-source-of-truth.*`, PR #40 |
| `wi-roadmap-post-schedule-reconciliation` | Reconcile roadmap after the schedule source-of-truth correction | Completed | `.ai/work-items/archive/2026/wi-roadmap-post-schedule-reconciliation.*` |
```

Do not rewrite historical rows or alter the Core Research Pipeline and Weekly Radar boundary text.

- [ ] **Step 3: Update the execution handoff and checklist wording**

Make the handoff state that the roadmap lists 40 archived Work Items before this reconciliation, will contain 41 after archive, and that Active Work Item returns to zero only after the lifecycle closes.

### Task 3: Verify the documentation-only diff and lifecycle evidence

**Files:**
- Read: `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md`
- Read: `.ai/work-items/archive/index.json`
- Verify: `.ai/cockpit/current_status.md`

**Interfaces:**
- Consumes: Task 2's edited roadmap.
- Produces: evidence for the Contract Summary and finish gate.

- [ ] **Step 1: Check for scope drift**

Run:

```bash
git diff --check
git diff --name-only
```

Expected: only the declared roadmap, plan, and Work Item evidence paths are changed.

- [ ] **Step 2: Check counts and latest table entries**

Run:

```bash
jq '.entries | length' .ai/work-items/archive/index.json
rg -n '14（本 WI 归档后为 15）|17|40（本 WI 归档后为 41）|wi-weekly-radar-schedule-source-of-truth|wi-roadmap-post-schedule-reconciliation' docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md
```

Expected: archive length is `40` before this Work Item archives and every required roadmap identity/count is present.

- [ ] **Step 3: Run repository governance checks**

Run:

```bash
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-roadmap-post-schedule-reconciliation.contract.json SUMMARY=.ai/work-items/active/wi-roadmap-post-schedule-reconciliation.summary.json STAGE=before_finish
make ai-finish TASK=wi-roadmap-post-schedule-reconciliation REPORT_LANGUAGE=zh-CN
make check-ai-pr AI_BASE_COMMIT=c0a9c7df9fc16d05ea4f2d56f0fea3600b690c62
```

Expected: all required checks pass before archive/PR closure.

- [ ] **Step 4: Commit only the governed reconciliation**

Run:

```bash
git add docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md docs/superpowers/plans/2026-08-20-roadmap-post-schedule-reconciliation.md .ai
git commit -m "docs: reconcile roadmap after schedule fix"
```

Expected: one commit contains only this Work Item's declared scope.

### Task 4: Complete lifecycle closure

**Files:**
- Modify: `.ai/work-items/archive/**`
- Modify: `.ai/cockpit/current_status.md`
- Read: `target/task-closure-receipts/**`

**Interfaces:**
- Consumes: Task 3's verified and committed reconciliation.
- Produces: merged PR, archive sequence 41, closure receipt, and a clean root worktree.

- [ ] **Step 1: Archive after direct Outcome handoff**

Run `make archive-work-item TASK=wi-roadmap-post-schedule-reconciliation` only after the active Outcome is delivered in the conversation.

- [ ] **Step 2: Push, check, merge, and delete remote branch**

Create one PR from `codex/wi-roadmap-post-schedule-reconciliation`, wait for all hosted checks, merge it, and delete the merged remote branch.

- [ ] **Step 3: Close and audit**

Run:

```bash
make ai-close-work-item TASK=wi-roadmap-post-schedule-reconciliation
git worktree remove /Users/sei-rinn/dev/workspace_rust/ORG-X/.worktrees/roadmap-post-schedule-reconciliation
git fetch origin
git merge --ff-only origin/main
git status --short --branch
git worktree list
```

Expected: archive sequence 41 exists, no active Work Item remains, the remote/local work branch is absent, and the root worktree is clean and synchronized.
