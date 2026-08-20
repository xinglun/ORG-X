# Roadmap Archive Count Reconciliation

## Goal

Reconcile the roadmap's dated archive-count snapshot with the authoritative archive index after `wi-runtime-http-source-safety` closed as archive sequence 44, and document the current Work Item as the expected post-archive sequence 45.

## Scope

- Modify `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md` only for count snapshot text and completed-row evidence.
- Modify the active Contract and Summary plus generated lifecycle artifacts.
- Do not change archived records, Rust code, tests, workflows, product scope, or dependency semantics.

## Evidence boundary

- Current authority: `.ai/work-items/archive/index.json` contains 44 contiguous archived records, with `wi-runtime-http-source-safety` at sequence 44.
- Latest completion evidence: `.ai/work-items/archive/2026/wi-runtime-http-source-safety.outcome.md` and its archive manifest.
- The roadmap count is a timestamped pre/post-archive snapshot, not a replacement for the archive index.

## Implementation

1. Update the overview counts to 17 maintenance, 10 core research, 17 Weekly Radar, and 44 archived before / 45 after this Work Item.
2. Add `wi-runtime-http-source-safety` to the completed maintenance and Weekly Radar rows with its archived evidence path.
3. Add this reconciliation Work Item to the completed maintenance table with its eventual archived evidence path.
4. Correct the WI-WR-013 row from the stale weekend wording to the Monday default proved by the archived schedule source-of-truth WI.
5. Preserve all historical rows, product boundaries, dependencies, and explicit non-production limitations.

## Verification

- `jq empty` for Contract and Summary.
- `make ai-preflight` and Contract/schema/scope/guard/status checks.
- Diff review proving roadmap-only semantic changes.
- `make ai-finish TASK=wi-roadmap-count-reconciliation REPORT_LANGUAGE=zh-CN`.
- Explicit archive, commit, `make check-ai-pr`, hosted checks, PR merge, `make ai-close-work-item`, and final clean-state audit.

## Non-goals

No runtime changes, external provider calls, production workflows, Telegram publication, data writes, product-policy decisions, or archived-record rewrites.
