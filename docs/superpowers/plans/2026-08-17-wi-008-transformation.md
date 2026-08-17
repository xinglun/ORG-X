# WI-008 Implementation Plan

1. Record the authorized Contract, transformation sources, acceptance, scenario
   coverage, and the `before_edit` checkpoint.
2. Add failing tests for the six stages, explicit upgrade/downgrade
   transitions, same-stage rejection, proof polarity, missing proof,
   persistence facts, and assessment ordering.
3. Add pure Stage, StageTransition, TransformationProofSet, PersistenceFact,
   and TransformationAssessment models.
4. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and the final checkpoint.
5. Finish, report the Outcome, archive evidence, publish a Draft PR, merge it,
   and close the Work Item before starting WI-009.

## Current-WI issue policy

Defects inside this transformation boundary are fixed and verified in WI-008.
A new WI is opened only when the requested change is a distinct boundary or a
material scope expansion.
