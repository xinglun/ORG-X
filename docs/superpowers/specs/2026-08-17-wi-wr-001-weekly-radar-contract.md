# WI-WR-001 Weekly Radar Domain Contract

## Boundary

This Work Item establishes the smallest provider-agnostic Weekly Radar boundary:

- `WeeklyRadarSnapshot` retains `snapshot_id`, `as_of`, `universe_snapshot_id`, `evidence_cutoff`, `model_version`, and `scoring_version`.
- `WeeklyRadarPublication` binds ordered, opaque precomputed publication facts to exactly one snapshot.
- `WeeklyRadarPublisher` is an Application port that receives one publication and reports a typed delivery boundary error.

The boundary is read-only from the consumer perspective. It does not infer Stage, rank candidates, calculate Threshold Distance, detect Rising/Dropped, render Markdown or Telegram, persist history, schedule a run, retry delivery, or create a receipt.

## Invariants

1. Required snapshot and fact values reject blank input.
2. A publication exposes only the snapshot it was created with.
3. Publication facts preserve insertion order and reject duplicate fact identity.
4. The Domain and Application layers use standard-library types only.
5. External channels must implement the Application port outside this context; they cannot redefine publication facts.

## Safety and handoff

The Weekly Radar is a research publication boundary, not a trading system. No BUY, SELL, target price, position, or capital-action behavior is introduced. Later WIs may add typed read models, persistence, renderers, publisher adapters, retry, scheduling, and health integration only within their own Contracts.

If a problem is found while implementing this WI and it remains in this boundary, fix it here and amend the current Contract when needed. Do not open a new WI merely to move an in-scope correction.

## Authorization

Execution, verification, publication, merge, close, and archive are authorized for all 24 roadmap Work Items by the recorded user authorization in the Contract.
