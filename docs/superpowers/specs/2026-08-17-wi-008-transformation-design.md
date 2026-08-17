# WI-008 Transformation Stage Domain

## Boundary

The Transformation Domain retains the six documented stages: `TOOL`,
`SUBSTITUTION`, `WORKFLOW`, `PRODUCTION_SYSTEM`, `PRODUCTIVITY_BREAKOUT`, and
`REFERENCE_MODEL`. A `StageTransition` records explicit from/to stages and a
transition date. Both upgrades and corrective downgrades are valid; a
same-stage no-op is rejected.

`TransformationProofSet` keeps supporting proof, counter proof, and missing
proof requirements as separate ordered collections. `PersistenceFact` retains
the supplied persistence window and observation count without calculating
sufficiency. `TransformationAssessment` groups these facts but never
recommends a stage or substitutes a score for one.

## Decisions

- Stage order is stable metadata, not a score.
- Transition direction is explicit and supports corrections/downgrades.
- Supporting, counter, and missing proof remain separate and ordered.
- Missing proof is an explicit requirement; absence is never converted into
  support.
- Persistence window and observation count remain opaque supplied facts.
- The implementation uses only the Rust standard library and imports no other
  feature module.

## No-goals

- Evidence acquisition, source adapters, persistence, scheduling, ranking,
  reporting, Telegram delivery, scoring, or stage recommendation.
- Proof quality calculation, source resolution, automatic inference, or runtime
  transition enforcement.
- Trading, price prediction, capital-action behavior, or external operations.

## Authorization and issue policy

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The authorization is recorded in the Contract. If verification finds an issue
inside this transformation boundary, it is resolved in WI-008; a successor is
reserved for a distinct boundary or a material scope expansion.

## Verification

- `cargo test --test transformation_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-008 REPORT_LANGUAGE=zh-CN`
