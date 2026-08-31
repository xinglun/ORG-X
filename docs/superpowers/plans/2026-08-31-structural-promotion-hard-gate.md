# Structural Promotion Hard Gate

> Execute this plan in the isolated Work Item worktree after the Contract
> checkpoint. Keep the change fail-closed and scoped to the evidence promotion
> boundary.

## Goal

Prevent document passages that do not prove a structural change from entering
`StructuralEvidence`, while preserving genuine structural claims with a
complete semantic contract.

## Steps

1. Add regression fixtures using the exact 2026-08-31 production passages for
   Amazon, Alphabet, Walmart, and Microsoft. Assert each is retained as a
   validated fact but is not structural evidence.
2. Add a positive regression for a genuine production-system sentence and a
   report-level regression for a dimension-only fact without a contract.
3. Run the focused tests and confirm the new tests fail against the current
   promotion path (RED).
4. Implement the smallest runtime change that makes structural promotion
   require a present, valid, complete structural contract and prevents the
   report/count path from bypassing that gate.
5. Run focused tests, formatting, clippy, and the locked workspace test suite.
6. Re-run AI Cockpit verification, expose the Outcome, archive, push the
   reviewed PR as `xinglun`, merge it, then verify synchronization and cleanup.

## Verification commands

```text
cargo test --locked --test weekly_radar_evidence_quality
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --locked --workspace
```

## Stop conditions

- Do not implement WI-2 attribution behavior in this Work Item.
- Do not weaken or delete existing positive structural tests.
- Stop before implementation if Runtime preflight reports `not_ready` or
  `needs_human_confirmation`.
