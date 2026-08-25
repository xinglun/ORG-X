# Weekly Radar Independent Diffusion Source Normalization

## Problem

The 2026-08-26 live dry-run reached validated and structural evidence, but Microsoft remained a Candidate because independent diffusion was counted as zero. The PwC-owned customer disclosure was reachable and substantive, but its `pwcReleaseDate` metadata was not recognized. The configured NIQ URL returned HTTP 403.

## Decision

- Keep the four-family reference-model gate and its fail-closed behavior unchanged.
- Add a bounded allowlist of site-specific publication metadata names, with release/published metadata ahead of modified metadata.
- Treat the explicit Atos Group press URL as a customer-owned independent source and retain PwC as the second explicit independent source.
- Recognize bounded `/press/` AI document paths through deterministic classification; do not guess sibling URLs.
- For independent documents only, allow the bounded title to provide the named-adopter context for a substantive body claim. Entry points and title-only pages remain non-evidence.
- Supplier Microsoft customer stories remain `SupplierAttribution` and cannot satisfy the independent diffusion condition.

## Evidence boundary

`SourceObservation` remains the acquisition record. Only a dated authoritative document can become an `EvidenceCandidate`, and only a validated candidate can reach the judgment chain. The independent source role is attached by source kind, not inferred from claim wording or URL similarity.

## Acceptance

The PwC date fixture, Atos press-document fixture, named-adopter fixture, supplier separation tests, full local/hosted lifecycle, and one post-merge dry-run must all provide evidence. A passing parser test alone does not establish that an AI-era reference model exists.
