# WI-WR-007 Weekly Change Compression — Design Spec

## Boundary

This Work Item creates a standalone Weekly Radar domain boundary that accepts
explicit upstream facts for five weekly-change sections:

1. Important Structural Change
2. Top5 Change
3. Stage Transition
4. Rising
5. Dropped

The boundary returns those sections in a deterministic order and preserves each
event's supplied identity, period, company, opaque fact value, and insertion
order. The source is deliberately not registered through the shared Weekly
Radar `mod.rs`; focused integration tests load it by path so this Work Item
does not take ownership of shared module registration.

## Input model

Each section has its own public newtype. Every section event contains:

- `EventId` — the stable event identity supplied by the caller;
- `PeriodId` — the supplied weekly period;
- `CompanyReference` — the supplied company identity; and
- `FactValue` — an opaque, non-empty supplied fact value.

The compression input also receives the expected period. Construction rejects
blank boundary values, any event whose period differs from the expected
period, and any event identity repeated across sections. No event field is
normalized, interpreted, ranked, merged, or overwritten.

## Output model

`WeeklyChangeCompression` owns the five ordered event vectors and exposes a
stable six-slot `sections()` view in this order:

```text
Important Structural Change
Top5 Change
Stage Transition
Rising
Dropped
No Change
```

The first five slots always expose their vectors, including empty vectors.
`NoChange` is `Some` only when every event vector is empty. It contains the
supplied period, the stable label `NO_CHANGE`, and zero counts for all five
sections. When any event exists, the No Change slot is `None`; this makes the
absence explicit without fabricating a no-change event. No natural-language
narrative is generated.

## Invariants and errors

- Accepted event values are returned exactly as supplied.
- Within-section order is the input order, and section order is fixed by the
  output boundary rather than by sorting or ranking.
- Duplicate event identity returns `DuplicateIdentity` before compression is
  built.
- A period mismatch returns `PeriodMismatch` before compression is built.
- Blank values return `EmptyValue`.
- The module uses standard-library facilities only and does not import or
  register shared Weekly Radar modules.

## Explicit non-goals

This Work Item does not calculate or infer Stage, Ranking, Distance, score,
Top5 membership, Rising, Dropped, or Important Structural Change. It does not
consume raw evidence, render Markdown or Telegram, persist snapshots, schedule
runs, retry delivery, call providers, or perform trading/capital actions.
