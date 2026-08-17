# WI-WR-005 Threshold Distance — Design Spec

## Boundary

This Work Item adds one standalone, read-only Threshold Distance boundary at
`src/features/weekly_radar/domain/threshold_distance.rs`. It retains a company
reference, opaque `current_stage` and `next_stage` labels, ordered Confirmed
and Missing evidence identities, and one `Distance` value.

`Distance` has exactly four stable labels: `FAR`, `DEVELOPING`, `NEAR`, and
`CANDIDATE`. The constructor receives that value from an upstream producer.
The repository does not provide an authoritative Distance formula, so this WI
does not guess one, compare stages, inspect evidence content, or recalculate
the supplied value.

## Invariants

- Company, stage, and evidence identity wrappers reject blank values.
- Confirmed and Missing evidence collections must each contain at least one
  identity.
- Duplicate identities within one collection are rejected.
- An identity present in Confirmed cannot also be present in Missing.
- Accepted evidence remains in supplied order and is exposed through immutable
  slices.
- The supplied Distance is returned unchanged, even when opaque stage labels
  would suggest a different relationship.

## Explicit no-goals

This WI does not infer Stage, detect Stage Transition, calculate Distance,
calculate a score, decide Ranking or Top5 membership, acquire or persist
Evidence, render Markdown or Telegram, publish, schedule, retry, or perform a
trading/capital action. It does not modify shared `weekly_radar` module files,
the architecture module-boundary test, or another Work Item.

## API shape

```text
CompanyReference::new(value)
StageLabel::new(value)
EvidenceId::new(value)
Distance::{Far, Developing, Near, Candidate}
ThresholdDistance::new(
  company,
  current_stage,
  next_stage,
  confirmed_evidence,
  missing_evidence,
  distance,
)
```

The integration test imports the file with `#[path]` so this isolated WI does
not edit `src/features/weekly_radar/domain/mod.rs` or any other shared export.

## Authorization and issue handling

The raw authorization recorded in the Contract is:

> 完成24 个WI，需要我授权的，授权给你并请写入Contract。

The corresponding user policy is also recorded here: if a validation, test,
Clippy, documentation, or governance issue is discovered and remains inside
WR-005, resolve it in this WI and update its evidence; do not casually create
a successor WI. A distinct or materially expanded boundary remains the only
reason to open a successor.
