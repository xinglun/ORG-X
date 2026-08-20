# Roadmap Post-SEC-Fix Reconciliation Plan

## Goal

Bring the roadmap snapshot into agreement with the immutable archive after
`wi-sec-company-facts-response-limit` merged as PR #38, without changing any
product, dependency, or runtime scope.

## Steps

1. Inspect the current roadmap counts and completed table against
   `.ai/work-items/archive/index.json` and the archived #38 evidence.
2. Update the roadmap's status snapshot and completed table:
   - add `wi-sec-company-facts-response-limit`;
   - add this reconciliation Work Item as the post-archive maintenance row;
   - state 38 archived entries before this WI and 39 after archive;
   - preserve all existing research and Weekly Radar rows and boundaries.
3. Run JSON, scope, guard, status, documentation, and project quality checks.
4. Finish and archive the Work Item, then commit, run `check-ai-pr`, push,
   create one draft PR, wait for hosted checks, mark ready, merge, and close.
5. Verify a clean main worktree, no active Work Item, contiguous archive
   sequence through 39, no local or remote `codex/*` branches, and no
   unmerged PRs.

## Verification

- `make ai-preflight`
- `make check-ai-contract`
- `make ai-checkpoint CONTRACT=... SUMMARY=... STAGE=before_edit`
- `make ai-finish TASK=... REPORT_LANGUAGE=zh-CN`
- `make archive-work-item TASK=...`
- `make check-ai-pr AI_BASE_COMMIT=64ea52d537db47a9640cf46860465f2cd61bede6`
- hosted `ai-cockpit-quality`, `check-ai-pr`, and `task-list-completed`
- `make ai-close-work-item TASK=...`
- final `make quality` and archive/branch/worktree/PR audits
