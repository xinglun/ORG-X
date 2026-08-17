# WI-WR-003 Top5 Weekly Read Model

## Goal

Provide a small, read-only Weekly Radar boundary that consumes already supplied
Top5 entry facts and preserves them for later snapshot and renderer Work Items.

## Contract

Each entry contains seven opaque, nonblank text facts:

```text
candidate | company | stage | direction | confidence | key_change | next
```

`candidate` is the stable entry identity. `Top5WeeklyReadModel` accepts zero to
five entries, keeps insertion order, and rejects a repeated candidate identity.
The model is a projection boundary: it does not calculate membership, rank,
Stage, Direction, Confidence, Key Change, or Distance.

Nonblank values are retained exactly. Validation rejects only values whose
contents are empty or all whitespace; it does not trim or normalize accepted
values.

## Errors

- `EmptyValue { field }`: a required fact is blank or all whitespace.
- `DuplicateIdentity { entity, id }`: the candidate identity already exists.
- `Top5LimitExceeded { limit }`: a sixth entry was supplied.

Duplicate detection runs before capacity detection so a duplicate candidate is
reported as an identity error even when the collection is already full. Failed
additions do not mutate the existing ordered collection.

## Explicit non-goals

This WI does not edit shared module registration, ranking, transformation,
reporting, snapshot persistence, rendering, Telegram delivery, scheduling,
retry, external adapters, or infrastructure. It does not import other feature
implementations and adds no dependencies. A later composition WI may register
the independent module without changing its semantics.

The module owns a companion `top5_weekly_read_model_test.rs` file. The source
module loads it only under `cfg(test)`. This satisfies the repository's existing
module-based coverage association without modifying shared coverage policy or
shared module registration.

## Safety boundary

The model is not a trading or capital-action recommendation. It retains
research display facts only. No price, target price, portfolio, or execution
decision is represented.

## Current-WI issue policy

If implementation, tests, documentation, or AI Cockpit evidence reveal a
defect that remains inside this Contract, resolve it here, amend the Contract
first when scope/evidence must expand, and preserve the correction in the
Summary/Outcome. Do not open a new WI merely to contain a current-WI issue.
