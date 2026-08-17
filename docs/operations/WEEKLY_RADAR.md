# Weekly Radar Operations

Weekly Radar is a deterministic, evidence-first runtime. It acquires SEC and
explicitly configured company sources, normalizes only supplied facts, renders
the human-first report, publishes ordered Telegram messages, and writes the
matching report/snapshot/receipt to the guarded `data` archive.

## Schedule and command

The intended production schedule is Monday at 09:00 JST (`0 0 * * 1` UTC),
with manual dispatch available for an operator-approved rerun. The local
entrypoint is:

```sh
cargo run --release -- weekly-radar \
  --as-of 2026-08-17 \
  --archive-dir . \
  --registry config/weekly_radar/companies.json
```

The CLI composes acquisition, normalization, deterministic report rendering,
primary-evidence validation, Telegram publication, and the data-branch archive
in that order. A successful publication archives only after the Telegram
receipt is bound to the rendered report ID.

## Environment variables

- `ORGX_SEC_USER_AGENT` is required before acquisition. Use an identifying
  application name and maintainer contact; do not use a secret or print the
  value in logs.
- `ORGX_TELEGRAM_BOT_TOKEN` is read only by the Telegram transport.
- `ORGX_TELEGRAM_CHAT_ID` is read only by the Telegram transport.

Actual secret values belong in the runtime environment or CI secret store and
must not be committed to this repository.

## Local dry-run

Use `--dry-run` to validate the registry, fixture-safe source boundaries, the
normalized report shape, and sanitized deterministic output without sending
Telegram or creating, deleting, or changing archive files:

```sh
ORGX_SEC_USER_AGENT='ORG-X local dry-run contact@example.test' \
cargo run -- weekly-radar \
  --as-of 2026-08-17 \
  --archive-dir /tmp/org-x-weekly-radar \
  --registry config/weekly_radar/companies.json \
  --dry-run
```

Dry-run uses the injected fixture-safe HTTP boundary. A real publication
requires reachable primary evidence, Telegram credentials, and a successful
bound delivery receipt; dry-run does not bypass those publication gates.

## Source priorities and UNKNOWN

Primary evidence has priority: SEC EDGAR and explicitly configured official
company pages are authoritative candidates. Greenhouse and Lever are bounded
structured hiring sources. GDELT is discovery-only and cannot become
authoritative. Optional sources are never guessed when their registry entry is
absent.

Rule-only extraction preserves source provenance and passage text. Missing,
ambiguous, conflicting, undated, unrelated, or malformed evidence remains
`UNKNOWN`; an unreachable optional source is `UNAVAILABLE`, and discovery-only
material remains non-authoritative. The runtime does not use paid APIs, LLMs,
stage inference, rank inference, score inference, or investment conclusions.

## Telegram setup

Provide `ORGX_TELEGRAM_BOT_TOKEN` and `ORGX_TELEGRAM_CHAT_ID` through the CI
secret/environment mechanism. The publisher splits only at safe semantic
boundaries, retries a chunk within its finite policy, preserves ordered message
IDs and attempts, and records partial accepted IDs on failure. Tokens, chat IDs,
query strings, fragments, and userinfo are excluded from public diagnostics and
rendered report output.

## Data-branch retention

Archive writes are guarded to the literal `data` branch. Each successful run
writes the report, sanitized snapshot, bound `PUBLISHED` receipt, and manifest
under `weekly-radar/`; retention removes only date-prefixed report, snapshot,
and receipt files older than 365 days. `main` and other branches are never
retention or archive targets, and dry-run never performs retention.
