# WI-WR-012 Publication Receipt + Retry

## Boundary

WR-012 extends the provider-agnostic Telegram delivery boundary with validated message IDs and adds an immutable `PublicationReceipt` for one precomputed `WeeklyRadarPublication`. A receipt records the Telegram channel, the original snapshot identity, an explicit timestamp supplied by a clock boundary, ordered message IDs, attempt number, and typed delivery status.

The retry service retains the original publication value and reuses its opaque fact payloads byte-for-byte. It performs delivery only: it does not create a new snapshot, recalculate facts, rerender Markdown, split messages, or mutate the saved publication. A receipt for a different snapshot is rejected before transport invocation.

All tests use an injected in-memory transport/clock. No Telegram HTTP, bot token, chat ID, SDK, persistence, scheduler, backoff loop, polling, or webhook behavior is included.

## Failure semantics

- Analysis and snapshot persistence remain successful even when delivery fails.
- A partial/failed attempt returns the successful message IDs already observed, the original snapshot ID, failed message index, and typed reason.
- Retry is explicit and bounded to the same retained publication; a later scheduler may decide when to call it.

## Acceptance

1. Successful publication returns a complete Telegram `PublicationReceipt` with ordered message IDs and `Published` status.
2. Failed publication returns a receipt-bearing typed failure without changing the source snapshot or publication facts.
3. Retry sends the exact original payloads with the same snapshot ID and increments the attempt number.
4. Mismatched snapshot receipts fail locally before any transport call.
5. Module-local, same-stem, public integration, architecture registration, `make check`, strict Finish/Archive, and the authorized repository lifecycle pass.
