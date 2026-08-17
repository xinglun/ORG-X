# WI-WR-010 Implementation Plan

1. Register the Weekly Radar infrastructure publisher module and define the documented injected transport, destination/request types, and typed adapter errors.
2. Add tests first for exact chunk forwarding, source order, empty input, blank destination, and stop-on-first-failure behavior.
3. Implement the adapter against WR-011 complete chunks and the existing `WeeklyRadarPublisher` application port without adding network, secret, receipt, retry, scheduler, or domain-calculation behavior.
4. Add module registration and public integration coverage, then run focused tests, `make check`, reference-impact checks, strict AI Cockpit Finish/Archive, and the authorized publish/merge/close lifecycle.

The implementation remains provider-agnostic. Telegram environment names are documented for the later concrete transport but no values are read or stored here.
