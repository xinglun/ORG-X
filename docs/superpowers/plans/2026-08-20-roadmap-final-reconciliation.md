# Roadmap Final Reconciliation Plan

## Goal

Reconcile the roadmap snapshot with the authoritative archive after PR #36, and
leave the merged main tree with a truthful post-archive count.

## Boundaries

- Use `.ai/work-items/archive/index.json` as the only authority for completed
  Work Item counts and status.
- Preserve every existing archive record and do not modify source, tests,
  dependencies, workflows, or provider behavior.
- Update only the roadmap, this plan, and generated lifecycle evidence.

## Execution

1. Record the before state: 36 archive entries, the stale roadmap count, no
   active Work Item, and clean base main.
2. Run the required `before_edit` checkpoint and update the roadmap to state
   the 36-entry current snapshot and the expected 37-entry post-archive state.
3. Run all local governance checks, `make quality`, and `before_finish`; resolve
   every failure in this same WI before Finish.
4. Run `make ai-finish`, archive the Work Item, commit the complete archive
   transaction, run `make check-ai-pr`, push the PR, wait for hosted checks,
   merge, and run `make ai-close-work-item`.
5. Re-audit the merged main tree: archive count 37, roadmap count 37, no active
   Work Item, clean main, no local/remote work branch, and one worktree.

## Acceptance Evidence

- Roadmap snapshot and completed tables are archive-index-derived.
- No existing archived record is changed.
- All local and hosted lifecycle checks pass.
- Main and origin/main converge after closure.
