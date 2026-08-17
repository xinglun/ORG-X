# WI-004 Evidence Domain and Provenance

## Boundary

Evidence is a validated record of a source claim, not an instruction and not a
score. The Domain retains identity, company reference, observation/effective
dates, source URI/title, claim, optional normalized value, polarity, confidence,
freshness, extractor version, and content hash.

`EvidenceSet` groups records for one company into supporting and counter
collections and keeps explicit missing requirements separately. A missing
requirement carries `Unknown`, `Unavailable`, or `NotCollected`; it never turns
absence into a fabricated fact.

## Decisions

- Small value objects reject blank identity and provenance fields.
- Dates, normalized values, and hashes remain opaque strings at this boundary.
- Quality dimensions are independent classifications and are not averaged.
- Record insertion order is preserved while sets reject duplicate identities.
- Company ownership is checked when a record enters an `EvidenceSet`.
- The implementation uses only the Rust standard library and imports no other
  feature module.

## No-goals

- Source acquisition, authentication, external adapters, LLM extraction,
  persistence, retries, or scheduling.
- Stage, score, ranking, reporting, Telegram, or trading behavior.

## Authorization and issue policy

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The authorization is recorded in the Contract. If verification finds an issue
inside this boundary, it is resolved in WI-004; a successor is reserved for a
distinct boundary or material scope expansion.

## Verification

- `cargo test --test evidence_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-004 REPORT_LANGUAGE=zh-CN`
