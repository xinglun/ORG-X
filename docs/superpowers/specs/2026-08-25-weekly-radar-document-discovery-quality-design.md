# Weekly Radar Document Discovery and Claim Extraction Quality

## Problem

The merged Weekly Radar runtime now follows bounded official links and keeps
homepage observations separate from document observations. The first post-merge
dry-run nevertheless produced 76 document candidates, only 5 validated facts,
and 71 pending leads. The main loss occurs between a fetched document and a
complete `EvidenceCandidate`: common publication metadata is not fully read,
the discovered `DocumentKind` is discarded when the candidate becomes a source
observation, and the rule-only sentence extractor has little context for
reader-facing quality counters.

The system must improve evidence yield by recovering facts already present in
bounded official documents, while keeping false positives fail-closed. A page
being reachable, a secondary article existing, or a document having a title is
not enough to become validated evidence.

## Goals

1. Recover a document publication/effective date from bounded deterministic
   metadata sources with explicit precedence.
2. Preserve the deterministic document class from discovery through
   `SourceObservation` and evidence provenance.
3. Promote dated, authoritative, actionable production claims when all required
   Claim fields are present.
4. Keep the report honest about document candidates, validated evidence, and
   pending leads, including a deterministic document-kind breakdown.
5. Preserve legacy snapshots and the existing Stage, Ranking, publication, and
   archive boundaries.

## Non-goals

- No SEC parser or transport changes.
- No new provider, broader crawl, LLM, probabilistic model, or hiring-source
  configuration.
- No judgment, Stage, Ranking, Counter Evidence, Telegram, archive, data
  branch, or workflow changes.

## Design

### Date recovery

`document_metadata` returns the first valid date in this order:

1. `<meta property="article:published_time" content="...">`;
2. `<meta name="date" content="...">`;
3. JSON-LD `datePublished`;
4. `<time datetime="...">`;
5. JSON-LD `dateModified` only when no publication date exists.

Values may contain an ISO date-time, but only the `YYYY-MM-DD` date is retained.
Malformed, empty, ambiguous, and future dates are not guessed. The existing
cutoff validator remains the final date gate.

### Document context

`DocumentCandidate.document_kind` is copied into
`SourceObservation.document_kind`. Entry points, hiring records, discovery
articles, and status observations use `None`; only official discovered
documents carry a `DocumentKind`. The field is serialized as an optional value
so old snapshots continue to deserialize.

### Claim extraction

The existing `SourceObservation -> EvidenceCandidate -> ValidatedEvidence`
pipeline remains the only promotion path. Extraction continues to require:

- official authoritative document material;
- a non-empty title and body passage;
- a valid date;
- an explicit change/action signal;
- a production-area signal.

The extracted candidate retains the document kind in its provenance text. The
rule set may add precise action terms needed by observed official documents but
must not promote generic architecture descriptions, title-only pages, or
sentences that merely mention a production area without an action.

### Metrics and report

`ResearchMetrics` gains an optional, backward-compatible map of document-kind
counts. The acquisition loop increments it only for discovered document
observations. The localized report renders the sorted breakdown next to the
existing overall counts. Existing `validated_evidence`, `pending_leads`, and
source availability counters remain separate and unchanged in meaning.

### Compatibility and judgment boundary

New serialized fields default to empty/`None`. Existing `NormalizedFact`,
`RuntimeReportInput`, and report snapshots remain readable. No new metric or
document context is consumed by judgment or ranking; it is visibility and audit
information only.

## Acceptance evidence

- Metadata fixtures prove precedence, ISO date-time parsing, and malformed-date
  rejection.
- Source fixtures prove document-kind propagation and entry-point separation.
- Evidence fixtures prove a dated actionable claim promotes and generic,
  missing-date, title-only, and non-actionable claims do not.
- Report fixtures prove document-kind counts are localized and legacy metrics
  JSON defaults remain valid.
- Full project quality and AI Cockpit gates pass without changing SEC,
  judgment, Ranking, publication, or archive behavior.
