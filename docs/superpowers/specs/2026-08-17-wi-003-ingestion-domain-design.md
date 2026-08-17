# WI-003 Ingestion Domain and Observation Contract

## Boundary

The Ingestion Domain accepts source observations as opaque facts. It preserves
the observation identity, URI, title, observation time, optional effective date,
content hash, source tier, source classification, and payload. It does not
interpret claims, calculate scores, contact external systems, or persist data.

The Application layer exposes `ObservationSource` as a collection port. An
external adapter may implement that port in a later WI; this WI contains no
adapter implementation.

## Decisions

- Required textual metadata uses small validated value objects and rejects
  blank input.
- Observation time, effective date, and content hash remain opaque strings at
  this boundary. Parsing and hash computation belong to a later contract with
  explicit evidence.
- `IngestionReceipt` keeps accepted observations in insertion order and uses a
  set only to reject repeated observation identities.
- Payload is retained as bytes so ingestion does not alter or infer source
  content.
- The implementation uses only the Rust standard library.

## No-goals

- Network acquisition, source authentication, retries, scheduling, and
  persistence.
- Evidence extraction, production-system state, ranking, rendering, Telegram,
  or trading behavior.

## Authorization

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The same authorization is recorded in the Work Item Contract, including its
scope and raw-request digest.

## Verification

- `cargo test --test ingestion_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-003 REPORT_LANGUAGE=zh-CN`
