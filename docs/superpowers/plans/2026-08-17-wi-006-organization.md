# WI-006 Implementation Plan

1. Record the authorized Contract, organization sources, acceptance, scenario
   coverage, and the `before_edit` checkpoint.
2. Add failing tests for validated organization identities, blank-field
   rejection, duplicate collection identities, fact preservation, collection
   order, and the no-stage/no-score boundary.
3. Add pure Organization value objects and the ordered `OrganizationEvidence`
   aggregate for commitment, responsibility, budget, decision-right, and
   adaptation facts.
4. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and the final checkpoint.
5. Finish, report the Outcome, archive evidence, publish a Draft PR, merge it,
   and close the Work Item before starting WI-007.

## Current-WI issue policy

Defects inside this organization-evidence boundary are fixed and verified in
WI-006. A new WI is opened only when the requested change is a distinct
boundary or a material scope expansion.
