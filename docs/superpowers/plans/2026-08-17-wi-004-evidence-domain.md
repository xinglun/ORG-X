# WI-004 Implementation Plan

1. Record the authorized Contract, evidence sources, acceptance, scenario
   coverage, and `before_edit` checkpoint.
2. Add failing tests for provenance retention, required-field rejection,
   quality classifications, polarity routing, duplicates, company mismatch, and
   explicit missing evidence.
3. Add pure Evidence value objects and `EvidenceRecord`.
4. Add `MissingEvidence` and `EvidenceSet` with supporting/counter/missing
   collections and deterministic membership errors.
5. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and final checkpoint.
6. Finish, report the Outcome, archive evidence, publish a Draft PR, merge it,
   and close the Work Item before starting WI-005.

## Current-WI issue policy

Defects inside this evidence boundary are fixed and verified in WI-004. A new
WI is opened only when the requested change is a distinct boundary or a
material scope expansion.
