# WI-005 Implementation Plan

1. Record the authorized Contract, production-system sources, acceptance,
   scenario coverage, and the `before_edit` checkpoint.
2. Add failing tests for validated identities, blank-field rejection,
   duplicate roots, role preservation, supervision mode, ordered workflow
   steps, role references, and all four explicit control structures.
3. Add pure Production System value objects, roles, units, workflows, and
   deterministic collection operations.
4. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and the final checkpoint.
5. Finish, report the Outcome, archive evidence, publish a Draft PR, merge it,
   and close the Work Item before starting WI-006.

## Current-WI issue policy

Defects inside this production-system boundary are fixed and verified in
WI-005. A new WI is opened only when the requested change is a distinct
boundary or a material scope expansion.
