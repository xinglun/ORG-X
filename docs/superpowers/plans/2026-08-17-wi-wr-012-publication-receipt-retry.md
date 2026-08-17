# WI-WR-012 Implementation Plan

1. Add tests first for message ID validation, successful receipts, partial failure receipts, exact retry payload reuse, attempt progression, and snapshot mismatch rejection.
2. Extend the WR-010 injected transport to return typed message IDs while preserving its existing no-network/provider-agnostic behavior.
3. Implement the immutable `PublicationReceipt`, typed delivery failure, injected timestamp clock, and explicit retry service around a retained `WeeklyRadarPublication`.
4. Register module and integration tests, add reference-impact evidence, then run focused tests, `make check`, strict AI Cockpit Finish/Archive, and the authorized publish/merge/close lifecycle.

The implementation deliberately excludes Scheduler, E2E orchestration, real Telegram configuration/network, and any new snapshot calculation.
