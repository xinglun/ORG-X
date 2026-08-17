# WI-WR-010 Telegram Publisher Adapter

## Boundary

The infrastructure adapter consumes the complete, already-rendered `SemanticMessageSplit` produced by WR-011 and forwards each `SemanticMessageChunk` to an injected, provider-agnostic `TelegramTransport`. The adapter owns destination validation and ordered transport invocation; it does not own Telegram HTTP, credentials, provider SDKs, or publication lifecycle state.

The transport receives the exact chunk Markdown and destination in source order. A transport error is returned as a typed adapter error containing the zero-based chunk index and semantic boundary. The adapter stops at the first failure and does not retry, create receipts, expose message IDs, or replay a partial delivery; those behaviors belong to WR-012.

`ORGX_TELEGRAM_BOT_TOKEN` and `ORGX_TELEGRAM_CHAT_ID` are reserved configuration names for the later concrete Telegram infrastructure. No token, chat ID, network call, or CI secret wiring is included in this WI.

## Non-goals

This WI does not render or re-render Markdown, split content, calculate domain facts, alter the WR-009 renderer or WR-011 splitter, call Telegram, persist snapshots, schedule publication, implement receipts, retry, polling, webhooks, or add provider SDK dependencies.

## Acceptance

1. `TelegramTransport` is public, documented, and accepts a destination plus complete Markdown without prescribing a client or provider.
2. `TelegramPublisherAdapter` forwards every complete WR-011 chunk exactly once and in source order without truncation, merging, normalization, or recalculation.
3. Blank destinations and empty splits fail before transport invocation; the first transport failure includes its chunk index and semantic boundary and prevents later sends.
4. Module-local, same-stem companion, and public integration tests use only an in-memory recording/failing transport and prove exact payload and order.
5. `make check`, focused tests, strict AI Cockpit gates, archive, and the authorized publish/merge/close lifecycle pass.
