# WR-015 Weekly Radar End-to-End Verification

## Purpose

WR-015 binds the already-implemented Weekly Radar boundaries into one deterministic, provider-isolated execution proof. It also pins both repository workflow checkout actions to `actions/checkout@v5` while retaining `fetch-depth: 0`.

## Execution contract

The public integration test supplies a fixed cutoff and snapshot identity, then composes:

1. `WeeklyScheduler` for the due-day decision.
2. `InMemoryWeeklyRadarSnapshotStore` for append-only snapshot persistence.
3. Markdown and Telegram renderers for stable views, including explicit No Change output.
4. `SemanticMessageSplitter` for complete, ordered Telegram sections.
5. `TelegramPublisherAdapter` with an injected recording transport.
6. `PublicationReceiptService` for typed partial delivery and explicit retry.
7. `InMemoryWeeklyRadarArchive` for an append-only snapshot/Published-receipt pair.

The success scenario proves `Compute → Persist Snapshot → Render → Publish → Archive`, matching snapshot and receipt identity, stable Markdown output, and an archived Published receipt. The failure scenario injects one transport failure, verifies the typed partial receipt, retries at attempt two, and proves that the retained publication payloads are resent byte-for-byte in their original order. No-change and workflow-isolation assertions use no network, credentials, provider SDK, timer, external database, or persisted scheduler state.

## Archive boundary

The archive is intentionally provider-agnostic and in-memory for this Work Item. It accepts only a Published receipt whose snapshot identity matches the supplied snapshot, rejects partial/failed receipts, rejects duplicates, and exposes append order for verification. Durable storage and runtime integration remain outside WR-015.

## Acceptance evidence

- `tests/weekly_radar_end_to_end.rs` is the public end-to-end evidence.
- `src/features/weekly_radar/infrastructure/archive_store.rs` is the production archive boundary.
- `src/features/weekly_radar/infrastructure/archive_store_test.rs` covers append, mismatch, unpublished, and duplicate guards.
- `.github/workflows/ai-cockpit.yml` contains exactly two `actions/checkout@v5` references and no `@v4` reference.
