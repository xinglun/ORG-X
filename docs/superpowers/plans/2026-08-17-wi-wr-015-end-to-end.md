# WR-015 Implementation Plan

- [x] Record the authorized Contract and resolve the initial preflight evidence.
- [x] Record the before-edit checkpoint and revalidate the Contract amendment before coding.
- [x] Add red public E2E assertions for workflow version, ordered flow, retry, and isolation.
- [x] Replace the test-only archive ledger with the provider-agnostic append-only `InMemoryWeeklyRadarArchive` boundary.
- [x] Correct the E2E failure injector so the configured message index fails exactly once.
- [x] Pin both workflow checkout actions to `actions/checkout@v5` while retaining depth 0.
- [ ] Run focused tests, full repository quality, strict Finish, Archive, PR checks, merge, close, and branch cleanup.

## Verification notes

The E2E test uses a fixed snapshot/cutoff, an injected recording transport, and a fixed publication clock. It does not exercise Telegram network calls or durable archive storage. The retry assertion follows the existing receipt contract: retry resends the retained publication from its first chunk and preserves exact payload bytes and source order.
