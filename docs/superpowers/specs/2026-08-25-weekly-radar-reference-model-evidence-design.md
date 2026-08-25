# Weekly Radar AI-era Reference Model Evidence Design

## Status

Approved in conversation on 2026-08-25 for governed implementation. The
design is intentionally stricter than a production-system candidate: it must
be possible to say both “this company is a serious candidate” and “this is
not yet proven to be an industry exemplar.”

## Goal

Identify a company that has crossed the AI-era reference-model boundary:

```text
organization rewrite
  -> production-system rewrite
  -> persistent operating outcome
  -> independent industry diffusion
```

The target is analogous to an industry-defining production paradigm, not a
company with a large AI budget, a public reorganization, or a successful
product launch.

## Evidence families

Every candidate is represented by a provider-neutral evidence packet with four
required supporting families:

1. `OrganizationRewrite`: responsibility, reporting lines, decision rights,
   team/division structure, or operating-model governance changed.
2. `ProductionSystemRewrite`: AI execution, human supervision, verification,
   control points, exception paths, or the core production workflow changed.
3. `SustainedOutcome`: at least two distinct effective periods with comparable
   operating or economic measures. Stock price is not an outcome. A single
   company claim without a comparable period is insufficient.
4. `IndustryDiffusion`: at least two independent source URIs, named peers or
   adopters, and an explicit imitation/adoption signal. The candidate's own
   description of its framework is not diffusion evidence.

Every family retains claim text, effective date, source URI, source title,
authority tier, and content identity. A fact can belong to at most one family
at this boundary; multiple source claims remain separate records.

## Fail-closed eligibility

The deterministic gate has three reader-facing results:

- `Candidate`: organization and production-system rewrite are supported, but
  outcome, diffusion, counter-review, or critical proof is incomplete.
- `Confirmed`: all four families pass, outcome periods are distinct, diffusion
  sources are independent, authoritative-source requirements pass, counter
  evidence review is represented, and no critical missing proof remains.
- `NotEligible`: evidence is present but the packet fails a hard condition, or
  counter evidence directly invalidates the claimed model.

Missing or unavailable data is never converted into a negative claim. A
degraded source produces `MissingProof` or `Unknown` and prevents confirmation.
The gate is not a score and cannot be bypassed by Ranking confidence,
transformation score, freshness, or a compelling narrative.

## Domain boundary

The existing transformation domain owns the family enum and pure eligibility
rules. Weekly Radar runtime owns:

- mapping validated evidence to a family;
- preserving optional family metadata in `NormalizedFact` with legacy defaults;
- deriving the assessment from facts at the evidence cutoff;
- preventing generic `REFERENCE_MODEL` judgment signals from assigning the
  highest stage without a confirmed packet;
- serializing the assessment through the existing judgment snapshot; and
- rendering the assessment without recomputing it in the renderer.

Python and Shell remain orchestration-only. They may run the Rust binary,
fixture servers, bounded dry-runs, and verification commands. They may not
classify claims, infer periods, decide source authority, or assign eligibility.

## Report contract

The localized Weekly Radar report adds an AI-era reference-model validation
section. For each relevant company it shows:

- status (`Candidate`, `Confirmed`, or `NotEligible`);
- the four-family evidence matrix;
- distinct outcome-period count;
- independent diffusion-source count;
- counter evidence count and review state;
- missing proof; and
- explicit degraded-data wording when source coverage is insufficient.

The report must never call a Candidate an industry exemplar. Existing sections
for validated facts, structural evidence, source availability, pending leads,
unavailable facts, judgment, and Ranking remain separate.

## Stage and Ranking boundary

The existing Stage-before-Ranking architecture remains. Lower stages and their
ranking behavior are unchanged. `REFERENCE_MODEL` is the only stage whose
assignment additionally requires a `Confirmed` reference-model assessment.
An incomplete packet leaves the machine result `UNDETERMINED` and emits no
machine Ranking for that company.

## Compatibility and safety

- New serialized fields use `serde(default)` and preserve old snapshots.
- Source and claim records remain bounded and deterministic.
- No live network call is used in tests.
- News/discovery material can create a lead but cannot satisfy authoritative
  outcome or diffusion proof without the required validation path.
- Counter evidence and missing proof are retained independently.
- No secret, credential, Telegram, data branch, or external system is mutated.

## Verification boundary

TDD must prove the following before live execution:

1. complete bundle confirms;
2. organization/production-only packet remains Candidate;
3. one outcome period fails;
4. one diffusion source fails;
5. same URI duplicates fail independent-source counting;
6. future evidence is excluded by cutoff;
7. unavailable evidence never becomes counter evidence;
8. self-description and homepage evidence cannot confirm;
9. the highest stage gate is fail-closed;
10. old snapshots deserialize and all three report languages carry identical
    decision values.

The live run is a research validation step, not permission to manufacture a
positive result. If no company passes, the correct output is a bounded
Candidate or data-insufficient result with missing proof.
