# WI-WR-006 Rising / Dropped Design

## Boundary

This Work Item creates a standalone Weekly Radar domain output boundary for two
research-change sections: `Rising` and `Dropped`. It consumes an upstream
`ResearchState` pair and an explicit `StructuralEvidenceDelta`. It does not
inspect raw evidence, prices, rank, scores, or the meaning of a Stage label.

The source is intentionally not registered in `weekly_radar/domain/mod.rs` or
any shared module because the Work Item's exclusive-write rule forbids those
paths. The later composition Work Item may wire this boundary through an
explicit adapter.

## Decision model

The caller supplies one of these explicit delta kinds:

- `Strengthened` → `Rising`
- `Weakened` or `Invalidated` → `Dropped`
- `Unchanged`, `PriceOnly`, `RankOnly`, or `ScoreOnly` → no event

This is routing of a structured upstream fact, not inference. A company can be
Rising without being Top5. A Stage change is retained as previous/current
context, but this module does not decide whether the Stage change is valid.

## Data and integrity

`ResearchState` retains company and Stage. `StructuralEvidenceDelta` retains a
reason, ordered supporting/counter/missing evidence IDs, and next step.
`RisingDroppedEvent` adds period and event identity while preserving all those
values. Evidence IDs must be unique both within and across proof collections.

`WeeklyChangeSet` retains Rising and Dropped insertion order. It rejects a
duplicate event identity and any company identity that already appears in the
same period, including a Rising/Dropped conflict. It never overwrites an
existing event.

## Error behavior

Blank boundary values, mismatched previous/current companies, mismatched
periods, duplicate proof identity, overlapping proof identity, duplicate event
identity, and same-period company conflicts return typed deterministic errors.

## Non-goals

The design does not implement Stage transition detection, Ranking, Top5,
Threshold Distance, Snapshot persistence, reporting assembly, renderers,
Telegram delivery, scheduling, retry, price interpretation, or capital action.
