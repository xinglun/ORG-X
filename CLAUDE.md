<!-- AI_COCKPIT_SECTION -->

## AI Cockpit Rules

This repository uses AI Cockpit as a collaborative engineering environment for AI-assisted changes. AI Change Governance is the core control mechanism inside that environment.

### Repository and Review Unit

The default unit of governed work is one Work Item, one dedicated work branch, and one pull or merge request. Do not combine unrelated Work Items on one branch or use one PR to deliver multiple independent Work Items.

For template-maintenance work, create the branch from the latest `origin/main`. For an adopter project, create the branch from the latest commit on that project's own remote default branch. The adopter remote and branch may have any names; discover them instead of assuming `origin/main`, and record the remote, default branch, and base commit in the Work Item Contract.

Installation and upgrade changes are changes in the adopter project's history. Use a published template release tag and record its source identity; do not install from a moving template work branch. After the PR is merged, delete the remote and local work branch unless a documented recovery exception applies.

Work Item completion is a lifecycle closure, not merely branch deletion. Run `make ai-close-work-item TASK=<task>` only after the Work Item is archived and the corresponding PR is merged. Closure must verify archived evidence, local branch/PR Head SHA ownership, fast-forward-only base synchronization, clean worktrees, and remote branch absence before deleting the local retry identity. Any failed step is fail closed and must not report the Work Item as closed. A remote failure must retain or restore the Work Item checkout for retry.

Only `ready_on_base` means the invoking worktree can start the next Work Item. `closed_but_current_worktree_detached` means closure succeeded while another worktree owns the synchronized base; continue from the reported base worktree and do not treat the detached invoking worktree as ready.

### Required Workflow

1. Create or identify a version 2 Work Item Contract in `.ai/work-items/active/`.
2. Before implementation, complete `scope`, `outOfScope`, `sources`, `acceptance`, `verification`, risk, capability, and execution-decision fields.
   Treat the Contract as delegation plus description: it assigns boundaries and explains the work before changes begin.
   If the Contract contains an `intent` section, read it before implementing. When context is available, fill in at least `intent.problem` (detailed background and gap), `intent.constraints` (constraints to respect), and `intent.rationale` (why this approach). All `intent` fields are optional — do not invent content when context is not provided; leave them empty or mark them as `not provided`.
3. Read `.ai/glossary.md` and follow the Contract `guidelines`.
4. Do not change files outside the declared scope. Update the Contract first if the required scope changes.
5. Do not remove tests, snapshots, or Work Item records without documenting the reason in the Summary.
6. Update the matching AI Change Summary with changed files, verification evidence, guideline compliance, residual risks, and optional `intentAlignment` evidence when it is genuinely available.
   `unknowns` and `notCodable` are valid outputs when coding should not continue. Summary is a collaboration handoff, not only an audit artifact.
7. Run `make ai-finish TASK=<task> REPORT_LANGUAGE=<conversation-locale>` and treat failures as blockers for completion or archive. The locale must follow the conversation (`en`, `ja`, or `zh-CN`); never silently fall back to English.
   Use checkpoints to keep long-running tasks from drifting.
   A same-active-Contract, same-scope correction of active schema or evidence
   retries in that Work Item and preserves each blocked Outcome; do not create a
   successor or duplicate Issue merely for that correction. Use a governed
   successor/quarantine route only when the base changed, the Contract/scope is
   invalidated, or immutable failed-delivery evidence must be re-delivered.
   For a missing `before_finish` checkpoint, run the canonical
   `make ai-checkpoint CONTRACT=<contract> SUMMARY=<summary> STAGE=before_finish`;
   use `make ai-revalidate-contract-amendment` only for a stale immutable
   `before_edit` Contract binding.
8. Before archive, every agent and subagent must deliver the active Outcome into the conversation. The human handoff must state: what was completed; problem totals, blocking problems, and warnings; stops with reason/stage/resolution; resolved problems with solutions and evidence; avoided risks; remaining risks; unknowns; human decisions; verification; impact; and next action. A file path alone is not delivery. Every factual claim is evidence-bound; unsupported benefit statements are explicitly marked `inference`, and self-praise is forbidden without quantitative evidence.
9. Work Item closure must verify the archived record and inspect local branches, local worktrees, and remote branches. Any residue is a blocking condition with a concrete location and recovery action; only a clean local/remote state may be reported closed.
8. If you need a pre-implementation readiness view, run `make ai-preflight`. Use `make generate-ai-preflight-review` when you want generation only, and `make check-ai-preflight-review` as the report validator. `make ai-start` in `MODE=code` should surface the same review before implementation begins.
   The rule is **Evidence over Self-Declaration**: readiness is derived from Contract evidence, not from agent confidence. When that review reports `needs_human_confirmation` or `not_ready`, pause and report the Preflight Review to the user before any coding continues. Advisory mode means the command can exit successfully; it does not mean the agent may silently continue.

For a completed archive whose local `check-ai-pr` passes but whose exact GitHub
Actions `template-smoke` job failed aggregate coverage, use only the documented
same-Work-Item recovery command with all `HOSTED_REPOSITORY`,
`HOSTED_PULL_REQUEST`, `HOSTED_CANDIDATE_HEAD`, `HOSTED_RUN_ID`, and
`HOSTED_JOB_ID` arguments. The runtime and PR audit must independently verify
the provider facts and canonical coverage failure log. Never paste a log,
hand-write a receipt, lower the coverage floor, rewrite the archive, or commit
an ad-hoc repair to the archived candidate. The receipt does not authorize
provider mutation, merge, release, closure, or deletion.

Before editing, use Empathy, Design, Architecture, Implementation, Judgment, and Shipping as review lenses.
Do not invent missing product context. Prefer explicit "not provided" over inferred explanations.
If the user did not provide motivation or user impact, record that plainly in `problemStatement` or `unknowns` when relevant.
Treat `executionDecision` as the judgment point: continue only when scope, acceptance, verification, and unresolved unknowns support implementation.

### Safety Rules

- Never revert user changes unless the user explicitly requests it.
- Never store secrets, credentials, API keys, or machine-specific paths in governance templates.
- Do not hand-edit `.ai/cockpit/current_status.md`; generate it through the provided Make targets.
- Repository approval fields are workflow records, not trusted identity proof. Use protected platform review for trusted approval.
- AI Cockpit checks detect policy violations and block workflow completion; they do not prevent a process with filesystem access from writing files.

### Finish Criteria

Before reporting a Work Item ready for review, run every required check declared in its Contract, including scope, guards, checkpoint, agent risk, backtrack, coverage, guidelines, Summary, status, and configured project quality checks.

<!-- /AI_COCKPIT_SECTION -->
