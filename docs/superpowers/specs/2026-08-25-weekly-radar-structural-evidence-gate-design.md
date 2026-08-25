# Weekly Radar Structural Evidence Gate Design

## Context

The post-merge Weekly Radar dry-run on 2026-08-25 completed acquisition and
claim extraction for ten companies. It discovered 76 document candidates and
promoted five Meta Engineering claims as validated evidence, but the five
claims were rendered as one undifferentiated “confirmed information” group.
The runtime therefore proved that a source document contained a bounded claim,
but did not yet state whether that claim describes an enterprise organization,
workflow, production-system, deployment, or measurable operating change.

The same run reported SEC source coverage as available while also listing SEC
response data as unavailable for individual facts. Those are different facts:
the SEC endpoint stages may be reachable even when one or more normalized facts
cannot be produced. The report must expose both levels without implying full
financial coverage.

## Goal

Add one deterministic classification boundary after `ValidatedEvidence`:

```text
SourceObservation
  → EvidenceCandidate
  → ValidatedEvidence
  → {ValidatedFact, StructuralEvidence}
```

The report will show the two validated classes separately, retain source
availability and pending leads as separate acquisition outcomes, and expose
SEC stage reachability separately from usable SEC facts. Existing judgment and
Ranking gates remain fail-closed and unchanged.

## Non-goals

- No SEC parser, CIK registry, response-limit, or provider URL changes.
- No new source provider, crawl policy, LLM, probabilistic model, or dependency.
- No change to Transformation Stage definitions, judgment rules, Ranking
  thresholds, Counter Evidence, Telegram delivery, archive persistence, or the
  Weekly Radar workflow.
- No requirement that a live week contain a structural change. A live zero is
  valid when coverage is explicit and not degraded.

## Evidence semantics

`ValidatedEvidence` means a dated authoritative document contains a bounded
sentence-level claim with a concrete change action and a production-system
signal. It is a verified fact, not automatically a structural change.

`StructuralEvidence` is a validated claim whose sentence also contains a
structural-relevance signal. The signal must identify at least one of:

- organization, team, responsibility, reporting line, division, or operating
  model change;
- workflow, process, operations, production, deployment, rollout, automation,
  agent, platform, or infrastructure change;
- a measurable production-system impact such as utilization, latency,
  throughput, capacity, cost, headcount, margin, or cash flow.

The existing action gate remains mandatory. A sentence that merely describes
architecture, research, a model, or a product capability without an explicit
change plus structural-relevance signal remains a regular validated fact or a
pending lead; it cannot become `StructuralEvidence`.

The classification is represented in the runtime evidence module and mapped to
stable normalized fact kinds:

- `evidence_official_material_<index>` for a regular validated fact;
- `evidence_structural_change_<index>` for `StructuralEvidence`.

The latter kind is visible to reporting but is not parsed as a judgment signal,
so it cannot create a Stage or Ranking entry by itself.

## SEC health semantics

`ResearchMetrics` keeps its existing counters and adds backward-compatible
zero-default counters for:

- SEC collection stages expected and available (`submissions` and
  `company_facts` per configured CIK);
- SEC normalized facts expected and available (`FactStatus::Known`).

The acquisition flow records stage health by distinct stage, not by duplicate
failure messages. It counts usable facts from the normalized SEC result. The
report displays these counters separately from the existing source-family
coverage line and never calls all SEC facts available merely because the HTTP
stages were reachable.

## Report contract

The localized report keeps the existing executive-summary, judgment, ranking,
and system-health structure while replacing the ambiguous evidence wording with
explicit categories:

- validated facts;
- structural evidence;
- source availability;
- leads to verify;
- unavailable facts.

The Chinese, Japanese, and English labels carry the same metric values. New
headings are accepted by the semantic splitter; legacy “Confirmed Information”
aliases remain accepted so older rendered messages remain splittable.

The executive summary reports both total validated evidence and structural
evidence. If structural evidence is zero, it states that no structural change
was confirmed; if source or fact coverage is degraded, it also states that this
is not proof that no change occurred. It never promotes a regular validated
fact into an organizational-change conclusion.

## Error handling and compatibility

- Missing classification signals fail closed to regular validated fact or
  pending lead; they never default to structural evidence.
- New serialized metric fields use `serde(default)` and default to zero for
  legacy input snapshots.
- Existing `evidence_*` facts, SEC facts, source observations, and archive
  boundaries remain valid.
- The existing publication guard may still use any primary SEC fact or
  validated document evidence; this change only adds structural counting and
  report semantics.

## Verification design

TDD coverage will include:

1. a dated organization/workflow or production-impact claim classified as
   structural evidence;
2. a generic engineering/research description rejected from structural
   classification;
3. an ordinary validated fact remaining visible outside structural evidence;
4. partial SEC facts showing separate stage and fact counters;
5. legacy metrics deserializing with zero defaults;
6. localized report labels and both new and legacy semantic-splitter headings;
7. the existing judgment and Ranking fail-closed tests remaining green.

The final live dry-run is operational validation only. Its outcome may be zero
structural evidence; readiness is determined by classification correctness,
coverage honesty, and fail-closed behavior rather than by manufacturing a
positive weekly event.
