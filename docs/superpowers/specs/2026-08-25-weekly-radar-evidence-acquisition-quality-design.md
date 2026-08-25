# Weekly Radar Evidence Acquisition Quality Design

## Status

Approved in conversation for implementation on 2026-08-25. This document is
the design boundary for `wi-weekly-radar-evidence-quality`.

## Problem

The current Weekly Radar runtime can prove that an official homepage is
reachable, but it can promote that page-level observation to a confirmed
fact. The current path is effectively:

```text
URL -> successful page fetch -> normalized page text -> CONFIRMED fact
```

That makes `Confirmed Information` describe source availability rather than a
company or production-system change. The runtime also has a SEC adapter, but
one submissions or Company Facts failure aborts the complete SEC collection
for a company, and SEC material is not yet used as a document-discovery input.

The research path must instead preserve the distinction between an available
source, a discovered document, an extracted claim, and validated evidence:

```text
source entry -> bounded document discovery -> claim candidate -> validation
-> ValidatedEvidence -> normalized fact/judgment input -> Weekly Radar
```

The user-facing report must expose those states separately. No Stage or
Ranking behavior is being relaxed; insufficient evidence remains fail-closed.

## Goals

1. Restore reliable SEC acquisition with endpoint-specific, safe diagnostics
   and filing/document provenance.
2. Discover bounded, date-aware documents from configured official entry
   points instead of treating a homepage as evidence.
3. Add an explicit `EvidenceCandidate` to `ValidatedEvidence` gate. A record
   cannot become confirmed without a subject, concrete change, effective or
   publication date, production-system area, source identity/type, and a
   supported authority classification.
4. Ensure page availability, pending leads, validated facts, and unavailable
   sources are counted and rendered separately.
5. Preserve the existing Stage-before-Ranking and no-fabrication behavior.

## Non-goals

- No change to Stage definitions, Ranking order, Top5 selection, Counter
  Evidence rules, Missing Proof rules, or investment/trading behavior.
- No LLM dependency, external search SDK, paid data provider, or new secret.
- No guessed company URLs or unrestricted crawling.
- No promotion of news/GDELT material to authoritative evidence. Discovery
  material may seed a lead and corroboration request only.
- No claim that a source gap proves that no company change occurred.
- No migration of historical snapshots beyond backward-compatible decoding and
  explicit semantic labels.

## Architecture

### 1. Source observation

`SourceObservation` remains the acquisition-level record. It answers:

> Was this configured source reachable, and what bounded material did it
> return?

An official homepage with usable text is `source_available`, not confirmed
research evidence. Its provenance is retained for discovery and audit but it
does not satisfy `has_primary_evidence` by itself.

### 2. Document candidate

Official IR, careers, engineering, and SEC entry points feed a bounded
discovery step. A document candidate retains:

- company identity;
- canonical URL and source family;
- title and document kind;
- publication/effective date when explicit;
- discovery provenance and retrieval status.

Discovery is finite: same-origin official links only, a fixed per-entry and
per-company candidate limit, finite response bodies, and stable deduplication.
SEC submissions are the authoritative index for filing URLs and may select
recent 10-K, 10-Q, 8-K, earnings-release, shareholder-letter, and investor-day
documents when the filing metadata provides them. A document candidate is not
itself a claim.

### 3. Evidence candidate and validation gate

The extraction boundary creates an `EvidenceCandidate` from a document or a
structured SEC fact. It retains the source passage and explicit fields:

- subject/company;
- concrete change or measured fact;
- effective or publication date;
- affected production-system area;
- source type and authority tier;
- polarity and extraction version;
- source URI/title and content hash.

The validator creates `ValidatedEvidence` only when required fields are
non-empty, the date is usable and on/before the run cutoff, the source is
permitted for the claim type, and the passage is tied to the document or SEC
fact. Candidates that fail remain visible as pending/unknown with a safe
reason; they never become `FactStatus::Known`.

The runtime's existing `NormalizedFact` remains the snapshot compatibility
boundary. Validated evidence is mapped into it with a stable evidence kind and
provenance. A page-level `SourceObservation` maps to availability/pending
state, never to confirmed state. SEC quantitative facts that already satisfy
the structured fact contract remain confirmed facts.

### 4. Weekly report semantics

The report separates four counters:

- `新增有效证据`: validated claim/fact records eligible for judgment;
- `来源可用性确认`: reachable configured source observations;
- `待验证线索`: discovered documents or extracted candidates not yet valid;
- `关键数据源不可用`: configured sources whose acquisition failed.

`已确认信息` is rendered only from validated facts/evidence. When no
structural change is confirmed and source coverage is degraded, the summary
states that no change was confirmed and that the result is data-insufficient,
not proof of no change. Existing ranking suppression remains unchanged.

## SEC recovery design

The SEC adapter will preserve the current finite User-Agent and response-size
guards while making the acquisition stages observable independently:

1. Validate CIK and User-Agent.
2. Fetch submissions and company facts with endpoint-specific safe failure
   context.
3. Normalize available company facts without discarding successful facts when
   a separate optional document fetch fails.
4. Use submissions metadata to discover bounded recent filings/documents.
5. Use a filing document only for a rule-approved extraction such as the
   employee-count fallback; otherwise preserve it as a document candidate.

The production report must show whether SEC failed at submissions, facts, or a
document fetch without retaining response bodies, headers, credentials, or
provider tokens. Fixtures will cover success, one-endpoint failure, bounded
document discovery, and all-ten-company coverage accounting.

## Failure and compatibility behavior

- A missing or failed source produces `UNAVAILABLE`; a reachable but
  ambiguous/insufficient document produces `UNKNOWN` or pending lead.
- A discovery-only source remains discovery-only and cannot set
  `has_primary_evidence`.
- An official homepage can increase source availability but cannot set
  `has_primary_evidence`.
- SEC partial success retains confirmed facts already acquired and records the
  failed sub-operation explicitly; if no usable SEC fact/document remains,
  publication still fails the existing primary-evidence guard.
- Existing snapshot fields remain readable. New fields use stable defaults for
  legacy snapshots, and rendering remains deterministic across Chinese,
  Japanese, and English.

## Verification strategy

Focused tests will cover:

- SEC endpoint success, endpoint-specific failures, User-Agent propagation,
  finite body limits, filing discovery, and partial results;
- same-origin bounded link discovery and homepage/non-evidence semantics;
- candidate extraction and validation rejection for missing subject, change,
  date, production area, source, cutoff, or authority;
- validated evidence promotion and page-level non-promotion;
- `has_primary_evidence` and ranking suppression behavior;
- four report counters, degraded summary wording, localized headings, and
  legacy snapshot compatibility.

The Work Item will then run `cargo fmt --all -- --check`, Clippy with warnings
denied, `cargo test --all`, repository quality, AI Cockpit Finish, PR checks,
and one hosted CI run. CI results are evidence about this exact branch/head;
they do not replace local verification.

