# WI-003 Implementation Plan

1. Record the authorized Contract, sources, acceptance, scenario coverage, and
   `before_edit` checkpoint.
2. Add failing tests for provenance preservation, blank metadata rejection,
   receipt ordering and duplicate identities, and the Application collection
   port.
3. Add pure Ingestion value objects, `Observation`, and `IngestionReceipt`.
4. Add the Application `ObservationSource` port and request/error types without
   an external implementation.
5. Run focused tests, the complete Rust suite, architecture checks, AI Cockpit
   gates, and final checkpoint.
6. Finish, report the Outcome, archive the evidence, publish a draft PR, merge
   it, and close the Work Item before starting WI-004.

## Current-WI issue policy

If verification finds a defect inside this boundary, fix it here and rerun the
affected checks. A successor WI is reserved for a distinct boundary or a
material scope expansion.
