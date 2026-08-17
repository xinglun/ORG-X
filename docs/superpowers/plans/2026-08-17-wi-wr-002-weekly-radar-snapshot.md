# Plan: WI-WR-002 Weekly Radar Snapshot Store

## Steps

1. Record the Contract, scope, authorization, risk review, and before-edit
   checkpoint.
2. Add failing tests for empty history, exact metadata retention, append order,
   and duplicate identity rejection.
3. Implement the provider-agnostic `WeeklyRadarSnapshotStore` boundary and its
   in-memory append-only implementation without changing the WR-001 Domain
   types or adding dependencies.
4. Run formatting, focused tests, full tests, and AI Cockpit checks.
5. Complete the Work Item summary, archive its evidence, publish through the
   authorized GitHub workflow, and verify the merged result.

## Acceptance mapping

| Acceptance | Evidence |
| --- | --- |
| Append once, reject duplicates, preserve order | `tests/weekly_radar_snapshot.rs` |
| Retain supplied metadata exactly | `tests/weekly_radar_snapshot.rs` and store API |
| Remain provider-agnostic | `src/features/weekly_radar/application/snapshot_store.rs` |
| Register the application boundary without dependencies | `src/features/weekly_radar/application/mod.rs` |
| Cover empty and historical behavior | focused integration tests and `make check` |
| Record design, exclusions, authorization, and verification | this spec, this plan, and the Contract/Summary |

## Out of scope

Durable database/filesystem persistence, scheduling, rendering, Telegram
delivery, retry orchestration, publication receipts, typed weekly calculations,
credentials, and trading or capital-action behavior remain outside this Work
Item.
