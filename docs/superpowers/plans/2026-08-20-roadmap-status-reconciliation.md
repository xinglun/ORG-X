# Roadmap Status Reconciliation Plan

## Objective

Reconcile the Work Item roadmap's status counts and completed/candidate tables with the immutable evidence in `.ai/work-items/archive/index.json`, without changing product code or archived Work Item records.

## Evidence boundary

- The archive index and archived Contract filenames are authoritative for lifecycle status.
- The roadmap remains a planning document; it must not claim completion from source-code presence alone.
- The current state is no active Work Item after the merged Weekly Radar snapshot lifecycle change.

## Implementation tasks

1. Enumerate completed Work Items from the archive index and archived Contract/Summary/Outcome pairs.
2. Update the roadmap status/count section and completed/candidate tables to distinguish completed evidence from genuinely uncreated candidates.
3. Add an explicit status snapshot date and archive-index authority note so future drift is detectable.
4. Run scope, governance, documentation alignment, quality, PR, merge, archive, and close checks.

## Acceptance evidence

- No source, test, dependency, workflow, evidence record, or archived Work Item file changes.
- The roadmap count is reproducible from `.ai/work-items/archive/index.json`.
- `make quality` passes after the documentation correction.
- The full Work Item lifecycle is closed with clean local and remote state.
