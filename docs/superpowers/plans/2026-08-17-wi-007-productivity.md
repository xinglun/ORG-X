# WI-007 Implementation Plan

1. Record the authorized Contract, productivity sources, acceptance, scenario
   coverage, and the `before_edit` checkpoint.
2. Add failing tests for validated periods and metric facts, blank-field
   rejection, duplicate snapshots, insertion order, growth/headcount facts,
   and the no-calculation boundary.
3. Add pure per-employee metric value objects, `GrowthAndHeadcount`,
   `ProductivitySnapshot`, and ordered `ProductivityHistory` operations.
4. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and the final checkpoint.
5. Finish, report the Outcome, archive evidence, publish a Draft PR, merge it,
   and close the Work Item before starting WI-008.

## Current-WI issue policy

Defects inside this productivity boundary are fixed and verified in WI-007. A
new WI is opened only when the requested change is a distinct boundary or a
material scope expansion.
