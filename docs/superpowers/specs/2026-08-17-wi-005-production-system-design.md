# WI-005 Production System Domain

## Boundary

The Production System Domain describes how a company creates verified core
value. It retains a `ProductionSystem`, its `ProductionUnit` outputs, ordered
`Workflow` steps, and the `HumanRole` and `AgentRole` responsibilities that
participate in those workflows. Workflow control, verification, decision, and
exception structures remain explicit facts with role references.

The model is intentionally provider-agnostic and uses validated value objects,
insertion-ordered collections, and deterministic duplicate-identity errors.
Agent roles always carry an explicit human-supervision mode.

## Decisions

- Required identities, names, purposes, descriptions, responsibilities, and
  capabilities reject blank input at the domain boundary.
- Production-system collections reject duplicate unit, workflow, human-role,
  and agent-role identities.
- Workflow steps and all four control structures preserve insertion order.
- Workflow elements preserve role references without performing runtime work or
  inferring responsibility beyond the supplied facts.
- Exception paths retain a human escalation target.
- The implementation uses only the Rust standard library and imports no other
  feature module.

## No-goals

- Evidence acquisition, organization adaptation, productivity metrics,
  transformation stages, ranking, reporting, or Telegram delivery.
- Persistence, network access, scheduling, runtime agent execution, or live
  deployment actions.
- Provider selection, model evaluation, scoring, trading, price, or capital
  behavior.

## Authorization and issue policy

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The authorization is recorded in the Contract. If verification finds an issue
inside this production-system boundary, it is resolved in WI-005; a successor
is reserved for a distinct boundary or material scope expansion.

## Verification

- `cargo test --test production_system_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-005 REPORT_LANGUAGE=zh-CN`
