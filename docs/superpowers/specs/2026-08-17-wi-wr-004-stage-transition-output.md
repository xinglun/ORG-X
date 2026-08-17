# WI-WR-004 Stage Transition Detection Output

## Goal

Provide a standalone Weekly Radar read-only output boundary for Stage Transition facts that an upstream evaluator has already confirmed or explicitly marked as `Candidate`.

## Design

`StageTransitionOutput` owns only supplied facts: event identity, company, opaque `from_stage` and `to_stage` labels, transition date, explicit status, ordered supporting/counter/missing references, and confidence. `TransitionStatus` has exactly `Confirmed` and `Candidate`; constructing a Candidate never promotes it. `TransitionPriority::ProductivityBreakoutHigh` is exposed only for the explicit `PRODUCTION_SYSTEM → PRODUCTIVITY_BREAKOUT` label pair, with all other pairs remaining `Normal`.

The module is intentionally standalone. It uses standard-library types only, is not registered in the shared Weekly Radar module files, and is loaded by the focused integration test with `#[path]` so parallel Work Items can own the shared module registration. It does not inspect raw Evidence, mutate Stage, compare historical Snapshots, rank or score events, persist, render, send, schedule, retry, or invoke external systems.

## Invariants

- Required text values reject blank input.
- Event identity is stable and required.
- Supporting and counter references preserve insertion order and cannot reuse an identity across either collection.
- Missing references preserve insertion order and cannot reuse an identity with another missing reference or with supporting/counter references.
- The output accepts corrective downgrades and does not reject a same-stage pair because it is organizing supplied facts rather than deciding a transition.
- Priority is a pure mapping over the two supplied stage labels; no evidence, history, score, or external state is read.

## Explicit exclusions

Stage detection, Evidence interpretation, historical comparison, ranking, scoring, Snapshot construction, persistence, Markdown/Telegram rendering, publishing, scheduling, retry, module registration, and trading/capital-action behavior remain outside this Work Item.

## Current-WI issue policy

Validation, test, documentation, Clippy, or governance evidence issues that remain inside this boundary are resolved in WI-WR-004, with Contract amendments and checkpoint evidence when scope must expand. A new WI is reserved for a distinct or materially expanded boundary.
