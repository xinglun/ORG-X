# Weekly Radar SEC/IR Deep Document Discovery Design

## Status

Approved for implementation by the user on 2026-08-25. The implementation
stops when the local design, tests, runtime behavior, and governance checks
have no remaining discovered problems; hosted PR lifecycle is not required by
this request.

## Problem

Weekly Radar currently acquires SEC submissions metadata and Company Facts,
but filing metadata is retained only as pending candidates. Apart from the
employee-count fallback, filing bodies do not enter the document-to-claim
pipeline. Official IR sources follow one bounded set of same-origin links, so
an index page can be observed without reaching the dated release or filing it
indexes.

The target path is:

```text
SEC submissions / IR entry point
    -> bounded document discovery
    -> bounded document fetch
    -> SourceObservation::Document
    -> EvidenceCandidate
    -> ValidatedEvidence
```

Homepage availability, filing metadata, and failed document fetches remain
separate from validated enterprise evidence.

## Goals

1. Fetch a bounded set of recent SEC filing documents selected from validated
   submissions metadata and expose their body, filing date, form, accession,
   title, and safe retrieval status to the existing document evidence path.
2. Preserve partial SEC success: submissions, Company Facts, and each filing
   document have independent failure accounting; one failed filing must not
   discard usable facts or other documents.
3. Extend official IR discovery by at most one additional same-origin hop,
   with stable deduplication and a finite per-entry total. Index pages remain
   documents/leads and cannot become evidence without a dated claim passage.
4. Keep Stage-before-Ranking, source availability semantics, legacy snapshot
   decoding, and the existing EvidenceCandidate validation gate unchanged.
5. Make the new behavior deterministic and testable entirely through the
   injected HTTP fixture boundary.

## Non-goals

- No Stage definitions, Ranking thresholds, Top5 policy, Counter Evidence,
  Missing Proof, or publication behavior changes.
- No guessed URLs, unrestricted crawling, robots bypass, search provider,
  news promotion, LLM, probabilistic extraction, or new credentials.
- No SEC financial metric derivation beyond the existing normalized Company
  Facts rules and employee-count fallback.
- No migration of historical snapshots and no breaking deserialization change.
- No hosted PR, merge, release, or production publication requirement for this
  Work Item.

## Architecture

### SEC adapter

`SecClient` keeps submissions and Company Facts as independent stages. It
selects the existing bounded recent filing set, validates accession/document
names, fetches each primary document with a finite SEC filing body limit, and
retains a provider-private `SecDocumentCandidate` status of `Known`, `Unknown`,
or `Unavailable`. The candidate carries normalized text and authoritative
filing metadata when available. A failed filing adds only a safe
`filing_document` failure and remains visible as a document lead.

The runtime converts each SEC candidate into a provider-neutral
`SourceObservation` with `SourceKind::Sec` and `DocumentKind::Filing`. This
factory lives at the source-observation boundary; `sec.rs` does not import
the CLI or report layers.

### IR deep discovery

The existing same-origin link parser remains the only URL construction path.
The source adapter fetches the direct candidates, then parses at most one
additional level from a bounded number of successfully fetched direct pages.
It maintains a global same-origin URL set and a fixed total candidate cap per
entry. Stable document classification, URL ordering, and finite body limits
remain deterministic. A nested document is emitted with the same official
primary tier and provenance that identifies its discovery parent.

### Runtime integration

The CLI combines SEC document observations with configured IR/Careers/
Engineering observations before the existing normalization and evidence loop.
Document metrics are counted once at this shared boundary. SEC stage health
continues to count only submissions and Company Facts; filing failures are
represented by document observations and safe source failures without double
counting.

## Failure and compatibility semantics

- Invalid SEC metadata is skipped without constructing a URL.
- A filing response that fails, exceeds the finite limit, or has no usable
  text produces an unavailable/unknown document observation and a pending
  lead; it cannot produce a validated fact.
- A failed SEC filing does not change the status of successful Company Facts or
  other filing documents.
- IR cross-origin links, fragments, duplicate URLs, and candidates beyond the
  finite depth/total cap are ignored.
- Entry points and index pages remain `Unconfirmed`/pending unless the body
  contains a complete dated claim accepted by the existing evidence gate.
- Existing serialized `SourceKind`/`SourceObservation` snapshots remain
  readable because source observations are output-only and the new source kind
  is emitted only by the new runtime path.

## Verification strategy

- SEC fixture tests prove bounded recent-document fetching, form/accession
  provenance, response-size failure, partial filing failure, and retention of
  Company Facts.
- IR fixture tests prove one-hop expansion, same-origin filtering, duplicate
  elimination, finite total requests, and index-page non-promotion.
- Runtime tests prove SEC documents enter the common evidence loop once,
  metrics do not double count them, and Stage/Ranking suppression remains
  unchanged.
- Run `cargo fmt --all -- --check`, Clippy with warnings denied,
  `cargo test --all`, `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`,
  the required AI Cockpit checks, and the final local dry-run command. No
  hosted CI or PR merge is part of the stopping condition.
