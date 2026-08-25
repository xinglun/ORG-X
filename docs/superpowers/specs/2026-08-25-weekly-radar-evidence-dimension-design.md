# Weekly Radar Evidence Dimension Design

## Context

The 2026-08-25 post-merge Weekly Radar dry-run covered ten companies and
promoted five dated, authoritative engineering claims. Two claims were
classified as structural evidence, but the reader-facing report rendered both
as `组织变化` even though the claims described GPU utilization and latency.
The existing `StructuralEvidence` class answers whether a validated claim is
structurally relevant, but it does not say which structural dimension changed.

The current validation boundary already requires a company identity, concrete
claim, effective date, production area, authoritative source, source title,
source URI, and bounded passage before promotion. This Work Item makes that
completeness boundary explicit in the design and regression tests while adding
the missing dimension without creating a second evidence pipeline.

## Goal

Preserve the existing flow:

```text
SourceObservation → EvidenceCandidate → ValidatedEvidence
  → {ValidatedFact, StructuralEvidence}
```

and enrich `StructuralEvidence` with one deterministic dimension:

```text
Organization | Workflow | ProductionSystem | OperatingMetric
```

The dimension is visible in normalized facts and localized reports. Existing
Stage, Ranking, Counter Evidence, Telegram, archive, and workflow behavior
remains unchanged.

## Non-goals

- No new source provider, broader document crawling, LLM, probabilistic model,
  or dependency.
- No SEC HTTP/parser, CIK, response-limit, or provider URL changes.
- No changes to Transformation Stage definitions, judgment rules, Ranking
  thresholds, Counter Evidence, Telegram delivery, archive persistence, data
  branch publication, or workflow configuration.
- No requirement that a live week contain a structural event. A truthful zero
  remains valid when coverage is explicit.

## Data model

Add a public, provider-neutral `StructuralDimension` enum in the runtime model:

- `Organization`: organization, responsibility, reporting line, team, division,
  leadership, or operating-model change.
- `Workflow`: process, approval, manual step, decision, exception, or workflow
  change.
- `ProductionSystem`: production, deployment, rollout, platform,
  infrastructure, storage, serving, kernel, compute, or system change.
- `OperatingMetric`: utilization, latency, throughput, capacity, cost, margin,
  cash flow, productivity, or another bounded operating metric change.

`NormalizedFact` receives an optional `structural_dimension` field. The field
is omitted when absent and defaults to `None` during deserialization, so old
snapshots remain readable. Existing constructors remain source-compatible;
validated evidence uses a dimension-aware constructor only when promoting a
structural claim. The existing `evidence_structural_change_<index>` kind prefix
is retained for stable identity and old consumers.

`ValidatedEvidence::structural_dimension()` returns the deterministic result.
`evidence_class()` is StructuralEvidence exactly when the result is `Some`;
otherwise it remains ValidatedFact.

## Deterministic classification

Classification examines only the bounded validated passage. The candidate must
already pass the existing completeness checks: company identity, concrete
change, effective date not after the cutoff, production area, authoritative
source tier, source title, source URI, and non-empty passage. Missing required
data fails closed before dimension classification.

The rule table is fixed and provider-neutral. When multiple dimensions appear,
the more specific measurable impact wins in this order:

1. OperatingMetric
2. ProductionSystem
3. Workflow
4. Organization

This precedence makes `reduced latency` an operating-metric claim and
`maximizing GPU utilization` an operating-metric claim, while a platform or
deployment statement without a metric remains production-system evidence. A
generic architecture or research description without one of these explicit
signals remains a regular validated fact or pending lead.

## Report behavior

The existing structural evidence section remains in the same position and
retains its heading for splitter compatibility. Its item-level information
type uses the normalized dimension:

| Dimension | Chinese | Japanese | English |
| --- | --- | --- | --- |
| Organization | 组织变化 | 組織変化 | Organization Change |
| Workflow | 工作流变化 | ワークフロー変化 | Workflow Change |
| ProductionSystem | 生产系统变化 | 生産システム変化 | Production-System Change |
| OperatingMetric | 运营指标变化 | 運用指標変化 | Operating-Metric Change |

Legacy structural facts with no serialized dimension use a generic structural
evidence label rather than being guessed as organization change. Existing
validated-fact, structural-evidence, and confirmed-information headings remain
accepted by the semantic splitter.

## Compatibility and failure behavior

- Old `NormalizedFact` JSON without `structural_dimension` deserializes with
  `None`.
- Existing normalized fact kinds and identity prefixes do not change.
- Missing or ambiguous dimension signals fail closed to ValidatedFact; the
  system never invents Organization.
- Structural dimensions are presentation and evidence metadata only. They do
  not satisfy a Stage, create Ranking, or alter thresholds.
- No data branch, Telegram, archive, or workflow side effect is introduced.

## Verification design

TDD fixtures will cover:

1. organization evidence → `Organization`;
2. workflow evidence → `Workflow`;
3. platform/deployment/storage evidence → `ProductionSystem`;
4. GPU utilization and latency evidence → `OperatingMetric`;
5. generic research/architecture prose → regular ValidatedFact;
6. incomplete claim data → rejection or non-structural result;
7. legacy normalized-fact JSON → `None` dimension;
8. Chinese, Japanese, and English report labels;
9. unchanged fail-closed Ranking behavior.

Focused evidence/runtime tests run before the full project quality graph.
Operations documentation is updated only after the implementation and tests
agree on the dimension semantics.
