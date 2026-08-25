# Reference-Model Evidence Implementation Plan

> Execute in the isolated Work Item worktree. Every phase must record RED,
> GREEN, and a hard acceptance checkpoint before the next phase starts.

## Goal

Add a deterministic, fail-closed evidence bundle that can identify an AI-era
reference-model candidate without confusing an organization announcement with
an industry-proven production paradigm.

## Constraints

- Rust owns domain classification, eligibility, Stage suppression, serialization,
  and report values.
- Python and Shell only orchestrate bounded fixtures, the Rust binary, and
  verification commands.
- No new provider, unbounded crawl, LLM, score substitution, or live-network
  test.
- Preserve old snapshots and the existing validated-fact/structural-evidence
  report separation.

## Phase 1: Domain family and gate

**Files**

- Modify `src/features/transformation/domain/mod.rs`.
- Test `src/features/transformation/domain/mod_test.rs` and
  `tests/transformation_domain.rs`.

**Step 1 — RED**

Add tests for `ReferenceModelEvidenceFamily`, evidence identity, duplicate
source rejection, distinct outcome periods, independent diffusion sources,
counter review, and the three eligibility states. Run the focused domain test
and record the expected compile/assertion failure.

**Step 2 — GREEN**

Implement only the typed family values, evidence packet, missing-proof values,
and pure gate. The gate must not import runtime, provider, report, or Ranking
modules. It must expose exact missing requirements rather than a score.

**Hard acceptance**

- Complete four-family fixture is `Confirmed`.
- Organization + production only is `Candidate`.
- One period, one diffusion URI, no counter review, or duplicate identities
  cannot be `Confirmed`.
- Domain tests and architecture checks pass.

## Phase 2: Runtime family metadata

**Files**

- Modify `src/features/weekly_radar/runtime/model.rs`.
- Modify `src/features/weekly_radar/runtime/evidence.rs`.
- Modify `src/features/weekly_radar/runtime.rs`.
- Test `tests/weekly_radar_evidence_quality.rs` and
  `tests/weekly_radar_runtime.rs`.

**Step 1 — RED**

Add tests that a validated organization claim, production-system claim,
outcome claim, and diffusion claim carry the correct optional family, while a
homepage, generic engineering article, or self-description carries none.
Add legacy JSON fixtures without the new field.

**Step 2 — GREEN**

Add the provider-neutral family enum at the runtime serialization boundary,
default it to `None` for legacy JSON, and classify only validated evidence.
Keep the lexical classifier conservative and source/document-aware. Do not
use the classifier to infer a missing outcome or diffusion claim.

**Hard acceptance**

- Page-level source observations remain non-evidence.
- Generic technical prose remains a regular fact or pending lead.
- Legacy JSON round-trips with `None`.
- Family values retain source URI, title, date, and content identity.

## Phase 3: Judgment assessment and Stage gate

**Files**

- Modify `src/features/weekly_radar/runtime/judgment.rs`.
- Modify `src/features/weekly_radar/runtime/model.rs` if the snapshot boundary
  needs a compatibility field.
- Test `tests/weekly_radar_judgment_chain.rs`.

**Step 1 — RED**

Add integration tests for a complete packet, a Candidate packet, counter
evidence, missing proof, future dates, and an existing generic
`REFERENCE_MODEL` judgment fact. Assert the generic fact cannot assign the
highest stage without a confirmed packet.

**Step 2 — GREEN**

Add a serialized `ReferenceModelAssessment` to `JudgmentSnapshot`. Derive it
once from cutoff-eligible normalized facts. Keep the assessment separate from
Ranking. Update only the `REFERENCE_MODEL` branch of stage evaluation to
require `Confirmed`; leave lower stages and same-stage ranking keys unchanged.

**Hard acceptance**

- A confirmed packet yields `REFERENCE_MODEL` only through the explicit gate.
- Candidate, NotEligible, Unknown, or degraded packets emit no highest-stage
  Ranking entry.
- Counter and MissingProof remain separate arrays/fields.
- Existing lower-stage judgment tests remain green.

## Phase 4: Report and localization

**Files**

- Modify `src/features/weekly_radar/runtime/report.rs`.
- Modify `tests/semantic_message_splitter_test.rs` if a new top-level heading
  is introduced.
- Modify `tests/weekly_radar_runtime.rs`.
- Modify `docs/operations/WEEKLY_RADAR.md`.

**Step 1 — RED**

Add Chinese, Japanese, and English assertions for the reference-model matrix,
Candidate wording, Confirmed wording, missing proof, counter review, and
degraded coverage. Assert no Candidate is rendered as an exemplar.

**Step 2 — GREEN**

Render only the precomputed assessment. Keep validated facts, structural
evidence, source availability, pending leads, and unavailable facts in their
existing sections. Update semantic splitting and operations documentation.

**Hard acceptance**

- All languages render identical decision values and counts.
- The report says `Candidate`/`候选` until the gate confirms.
- Degraded coverage says “not confirmed” rather than “did not happen”.
- Existing legacy headings remain splittable.

## Phase 5: Runtime integration and bounded run

**Files**

- Modify `src/main.rs` only where the existing acquisition loop needs to pass
  family metadata or assessment inputs.
- Test `tests/weekly_radar_runtime.rs` and
  `tests/weekly_radar_evidence_quality.rs`.

**Step 1 — RED**

Add a deterministic registry fixture with a Microsoft-CoreAI-shaped
organization/production packet, missing outcomes and diffusion, plus a
complete synthetic reference-model packet. Assert the report matrix and no
fabricated confirmation.

**Step 2 — GREEN**

Feed validated family metadata through the existing shared evidence loop,
derive the assessment once, and render the read model. Do not duplicate source
requests, loosen authority, or move logic into Python/Shell.

**Hard acceptance**

- Every configured company receives a four-family status.
- The strongest candidate is deterministic and auditable.
- No complete-looking narrative can bypass missing proof.
- Metrics are counted once and old source-health semantics remain unchanged.

## Phase 6: Governance and historical-debt sweep

Run in this order:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
pytest -q
make ai-cockpit-quality GOVERNANCE_PROFILE=strict
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-weekly-radar-reference-model-evidence.contract.json SUMMARY=.ai/work-items/active/wi-weekly-radar-reference-model-evidence.summary.json STAGE=before_finish
make ai-finish TASK=wi-weekly-radar-reference-model-evidence REPORT_LANGUAGE=zh-CN
```

The finish stage must also verify stale Knowledge Projection records, generated
status ownership, scope ownership, localization, and deterministic report
output. If any historical projection is stale, rebuild it using the supplied
knowledge projection tooling and keep the correction in this Work Item.

## Final live validation

Trigger one bounded dry-run only after all local hard gates pass. Preserve the
report and input snapshot evidence, inspect each company matrix, and iterate in
this Work Item for every defect. The research target is complete only when a
company has a Confirmed four-family packet; otherwise report the strongest
Candidate and exact missing proof without overclaiming.
