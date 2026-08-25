# Weekly Radar Independent Diffusion Evidence Design

## Status

Approved in the current Work Item conversation on 2026-08-26. The user
requires the complete local, PR, hosted-CI, merge, and close lifecycle, with
same-Work-Item repair when a check finds a problem.

## Goal

Make the reference-model gate answer the product question: which company has
rewritten its organization and production system around AI, produced durable
results, and become an independently corroborated industry model.

## Problem

The predecessor packet treated two Microsoft-hosted customer stories as the
diffusion family. Those pages establish supplier attribution and named
customer examples, but they do not independently corroborate diffusion. The
runtime currently has no typed boundary separating supplier material,
customer self-disclosure, regulatory/IR operating results, and discovery-only
leads.

## Design

Add a provider-neutral `ReferenceModelSourceRole` to validated evidence and
reference-model claims:

- `SupplierAttribution`: supplier-controlled customer stories or technical
  attribution; retained for context, never sufficient for independent
  diffusion.
- `IndependentCustomerDisclosure`: adopter-owned case studies, investor
  disclosures, or operating communications; eligible for the independent
  diffusion hard condition when they contain a named adopter and adoption or
  imitation claim.
- `RegulatoryOrFiling`: SEC/IR result material; eligible for its own outcome
  family and retained separately from customer diffusion.
- `DiscoveryOnly`: news or secondary material; lead only.

The independent diffusion gate counts only authoritative claims with the
`IndependentCustomerDisclosure` role, distinct source URIs, and distinct
named adopters/peers. Existing four-family logic, two distinct outcome
periods, counter review, Stage-before-Ranking, and fail-closed behavior remain
unchanged. Legacy JSON defaults the optional role to unknown/non-independent.

Add an explicit `independent_research_sources` list to the bounded company
configuration. Microsoft receives only the two adopter-owned URLs identified
by this Work Item. Supplier URLs remain in `official_research_sources` and
are classified as supplier attribution. No cross-origin URL is guessed and no
unbounded crawl is introduced.

## Data flow

```text
configured URL
  -> SourceObservation(kind, tier, material kind)
  -> EvidenceCandidate(source role)
  -> ValidatedEvidence(source role)
  -> NormalizedFact(source role)
  -> ReferenceModelEvidence(source role)
  -> fail-closed family assessment
  -> localized report provenance and missing proof
```

Rust owns role classification, evidence metadata, the gate, Stage assignment,
and report values. Python and Shell remain orchestration and verification
boundaries only.

## Report contract

The reference-model section reports independent diffusion sources, supplier
attribution sources, source-role provenance, and missing proof separately. A
supplier-only packet remains `Candidate` with `independent_diffusion_sources`
missing. The report must not infer that an unavailable source proves no
organizational change.

## Verification boundary

TDD must prove supplier-only suppression, independent customer promotion,
legacy deserialization, explicit cross-origin collection, role propagation
through judgment, localized report rendering, and unchanged ranking
fail-closed behavior. Tests use deterministic fixtures and never call live
providers.
