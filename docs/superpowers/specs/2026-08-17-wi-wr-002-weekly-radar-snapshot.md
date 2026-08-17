# WI-WR-002 Weekly Radar Snapshot Store

## Intent

Weekly Radar already has a typed snapshot identity, but later rendering, delivery,
and retry work needs one stable historical record that can be reused without
recomputing the run. This Work Item adds the application boundary for that
record.

## Contract

- `WeeklyRadarSnapshotStore` accepts a fully formed `WeeklyRadarSnapshot`.
- `InMemoryWeeklyRadarSnapshotStore` appends each identity once and rejects a
  duplicate instead of overwriting the first record.
- Retrieval preserves the supplied metadata and append order exactly.
- The boundary is provider-agnostic and contains no Telegram, renderer,
  scheduler, retry, receipt, external persistence, credential, or calculation
  behavior.

## Design constraints

The store does not recalculate `as_of`, evidence cutoff, universe snapshot,
model version, or scoring version. It also does not infer Stage, Ranking,
Threshold Distance, Top5, Rising, Dropped, or System Health. External durable
persistence is a later boundary; this Work Item uses an in-memory implementation
to make identity and historical ordering deterministic and testable.

Issues discovered during implementation are resolved in this Work Item when
they remain within this Contract. A successor is only appropriate for a
distinct or materially expanded boundary.

## Authorization and verification

Execution is covered by the user's explicit authorization for all 24 roadmap
Work Items and is recorded in the active Contract. Verification includes the
focused snapshot integration tests, `make check`, and the AI Cockpit lifecycle
checks before finish.
