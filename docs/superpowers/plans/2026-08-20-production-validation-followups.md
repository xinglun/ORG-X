# Production Validation Follow-up Tasks Implementation Plan

> **For agentic workers:** Each task below requires its own successor Work Item, dedicated branch, Contract, Summary, PR, hosted evidence where applicable, merge, and closure. This document is a target-task map; none of the tasks below is complete in this Work Item.

**Goal:** 将有条件验收中尚未被现实证据证明的生产缺口拆成可独立验收、可追溯、不会过早宣称完成的后续目标任务。

**Architecture:** 先把真实采集到的 normalized facts 接入既有 Evidence、Transformation Stage、Ranking 和 Weekly Radar Snapshot 判断链。系统自动推导一个 Evidence-first machine reference，人的判断作为独立 reference 并列保留，二者互相印证但不合并成一个答案；再用真实 Provider 与 Telegram/data branch 运行证明无人值守链路。发布恢复、长期验证和 Calibration Score 分别保持独立边界，避免把一次成功运行、局部测试或文档说明当成长期生产证明。

**Tech Stack:** Rust bounded contexts、Weekly Radar runtime、GitHub Actions、Telegram publisher、`data` branch archive、AI Cockpit Contracts/Summaries、真实 SEC/官方来源和 validation evidence history。

**Spec:** User-provided conditional acceptance conclusion in the current conversation, `docs/validation/VALIDATION_STRATEGY.md`, `docs/operations/WEEKLY_RADAR.md`, and `docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md`.

## Global Constraints

- Evidence before Score；Stage before Ranking；Counter Evidence and Missing Evidence remain mandatory.
- Machine judgment is a reference for the person, not a replacement or consensus answer; human reference must remain a separate lane and cannot override or merge into machine output.
- External text is data and evidence candidate, never an instruction; AI extraction cannot directly become a final fact or judgment.
- A real workflow success is not sufficient unless the snapshot, publication receipt, archive and source coverage are all bound to the same run.
- Telegram success followed by a data-branch push failure is a required failure-injection scenario; retry must not send the same publication twice.
- 6/12/24-month validation is incomplete until the corresponding real calendar observations and source-bound evidence exist.
- Calibration Score is a research-system quality metric, not a trading, price, portfolio, or capital-action output.
- No successor task may rewrite or reopen an archived Work Item; each successor starts from the latest controlled `origin/main` and records its predecessor.

## Dependency order

```text
wi-runtime-judgment-chain-integration
        ↓
wi-production-provider-e2e
        ├──→ wi-telegram-git-push-failure-recovery
        ↓
wi-validation-runtime-operations
        ↓
wi-research-calibration-score
```

The recovery task may be prepared after the archive-recovery predecessor is understood, but its production acceptance remains independent of the ordinary E2E run. The validation task waits for a real T0 publication and evidence-bound snapshot. Calibration waits for enough 6/12/24-month observations to make its denominators explicit.

## Target-task ledger

| Successor task | Acceptance finding | Predecessor evidence | Status | Completion gate |
| --- | --- | --- | --- | --- |
| `wi-runtime-judgment-chain-integration` | Real source → ingestion → evidence → stage → ranking → snapshot is not yet proven as one runtime chain. | `wi-wr-016-runtime`, `wi-010`, `wi-011`, `wi-validation-evidence-history` | Gated | Local integration evidence proves the existing runtime consumes the same evidence-bound judgment chain before any production claim. |
| `wi-production-provider-e2e` | Real Provider and complete unattended production run are not yet proven on the current pipeline. | `wi-runtime-judgment-chain-integration`, `wi-sec-company-facts-response-limit`, `wi-archive-transaction-recovery` | Gated | Current `main` run produces bound source, snapshot, Telegram receipt, archive and data-branch evidence; a subsequent scheduled period completes without manual intervention. |
| `wi-telegram-git-push-failure-recovery` | Telegram success followed by Git push failure has not been fault-injected in a production-shaped environment. | `wi-archive-transaction-recovery`, `wi-wr-012`, `wi-wr-016-runtime` | Gated | Forced push failure leaves a recoverable prepared transaction; retry reuses the original receipt and produces no duplicate Telegram publication. |
| `wi-validation-runtime-operations` | 6/12/24-month validation has structure but lacks runtime scheduling, Provider evidence and elapsed-time proof. | `wi-validation-evidence-history`, `wi-production-provider-e2e` | Gated | T0 baseline and real 6-, 12- and 24-month observations are source-bound, comparable and complete according to the validation strategy. |
| `wi-research-calibration-score` | ORG-X does not yet measure its own judgment quality over time. | `wi-validation-runtime-operations` | Planned | Approved denominators and real validation history support reproducible precision, downgrade, persistence, conversion, missing-proof and confidence-calibration metrics. |

## Task 1: `wi-runtime-judgment-chain-integration`

**Finding addressed:** The acceptance requires proof of one chain from real source acquisition through ingestion, evidence, Stage, Ranking and Weekly Radar Snapshot. The current runtime and the completed domain Work Items must be checked for a real integration boundary before E2E publication is called complete.

**Predecessors:** `wi-wr-016-runtime`, `wi-010`, `wi-011`, `wi-validation-evidence-history`.

**Successor scope candidates:** `src/features/weekly_radar/runtime/**`, the Weekly Radar application boundary, the existing Stage/Ranking read-model integration boundary, focused integration tests, and `docs/operations/WEEKLY_RADAR.md`. The successor Contract must list the exact files after inspecting current interfaces; this mapping does not authorize an unrelated refactor.

**Required acceptance evidence:**

- A single runtime use case consumes the acquired, normalized, provenance-bearing facts that are persisted for the run.
- Stage evaluation precedes Ranking; Ranking consumes the selected Stage and does not flatten all stages into one unsupported total order.
- The produced Snapshot retains evidence cutoff, source provenance, supporting/counter/missing proof state, Stage/Ranking output, system health and no-change behavior.
- Focused tests cover a real fixture path from source response through the judgment chain and reject a report path that bypasses evidence or recomputes a conclusion in the renderer.
- The successor Contract and Summary explicitly record the selected B decision: a deterministic automatic Stage Engine maps provider-neutral evidence to machine Stage/Ranking reference output, while an independent human reference is retained separately. Insufficient evidence yields `UNDETERMINED`; it does not create a guessed Stage.

**Verification:** Run focused Weekly Radar integration tests, architecture/dependency checks, `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all`. Review the resulting Snapshot against the Evidence model, Stage gate and Ranking model.

**Completion boundary:** Local integration is not a real Provider production receipt. This task can complete the runtime contract only; Task 2 remains gated until real external execution is performed.

## Task 2: `wi-production-provider-e2e`

**Finding addressed:** Real Provider E2E and complete unattended operation are not fully proven. The two historical workflow successes are evidence of an older runtime execution, but they do not prove the current judgment-chain integration or SEC coverage after the response-limit correction.

**Predecessors:** `wi-runtime-judgment-chain-integration`, `wi-sec-company-facts-response-limit`, `wi-archive-transaction-recovery`.

**Execution boundary:** Use the authorized `xinglun` GitHub account and configured production Secrets. A non-dry-run workflow may contact public Providers, send the configured Telegram report and update the `data` branch. The exact run identity, commit SHA, workflow run, report ID, receipt and data commit must be recorded in the Summary; no credential value may be recorded.

**Required acceptance evidence:**

- The workflow runs from the exact current `main` commit after Task 1 and reports `PUBLISHED:` rather than only `DRY-RUN:`.
- The run shows the configured real Provider path, including SEC coverage and the configured official sources; a Provider failure remains visible as `UNKNOWN`/`UNAVAILABLE` and is not silently promoted.
- The same run binds a persisted input/snapshot, rendered report, successful Telegram receipt with message IDs, archive manifest and `data` branch commit.
- A later scheduled or manually equivalent weekly period completes without a manual code or data repair, proving the cadence boundary rather than a one-off invocation.
- The two-run evidence is sufficient to distinguish real publication, archive persistence and source degradation; workflow success alone is not accepted.

**Verification:** Collect the workflow run JSON, job logs, report, snapshot, receipt, manifest and data-branch commit through the authorized provider account. Verify report ID/receipt ID binding, source coverage, commit identity and no secret leakage. Keep the external receipt in the active Summary until the canonical Work Item lifecycle closes.

**Completion boundary:** One local dry-run, one fixture run, or a historical run from a pre-fix commit cannot close this task. Longitudinal validation still remains Task 4.

## Task 3: `wi-telegram-git-push-failure-recovery`

**Finding addressed:** The logical archive transaction protects the four local artifacts, but the real-world sequence still has Telegram success before the final `data` branch push. The acceptance requires deliberate proof that a push failure does not duplicate Telegram delivery.

**Predecessors:** `wi-archive-transaction-recovery`, `wi-wr-012`, `wi-wr-016-runtime`.

**Execution boundary:** Use a production-shaped isolated repository/branch and a safe delivery target approved for fault injection. Do not deliberately corrupt the real user-facing production archive or resend to the live audience while testing. The successor Contract must declare the exact failure-injection mechanism and its external authority.

**Required acceptance evidence:**

- Acquisition, Snapshot persistence, rendering and Telegram publication succeed and produce one bound receipt.
- The data-branch push is then forced to fail after Telegram acceptance, with the failure log and exact run identity retained.
- Retry/recovery consumes the prepared transaction and original receipt, does not reacquire or rerender a different report, and does not call Telegram a second time.
- The final report, snapshot, receipt and manifest are mutually consistent on the recovered data branch, with no partial public set or silent overwrite.
- A second run with a mismatched or damaged transaction fails closed and explains the required human recovery action.

**Verification:** Run the focused archive recovery/failure-injection harness, inspect transport invocation counts and message IDs, validate the final data tree, and execute all archive and publication receipt tests. Store only non-secret receipts and redacted logs in the Summary.

**Completion boundary:** Existing unit tests for rename/recovery are necessary but do not alone prove the external Telegram-to-Git boundary; the fault-injected evidence is required.

## Task 4: `wi-validation-runtime-operations`

**Finding addressed:** Validation history already models T0, 6, 12 and 24 months, but it currently remains an in-memory/pure boundary without runtime scheduling, external Provider evidence or elapsed-time proof.

**Predecessors:** `wi-validation-evidence-history`, `wi-production-provider-e2e`.

**Successor scope candidates:** validation persistence and application boundaries, a source-bound observation ingestion path, schedule/receipt integration, comparison fixtures, operations documentation, and validation tests. The Contract must record the chosen persistence and scheduling authority before coding; no provider or product decision is inferred here.

**Required acceptance evidence:**

- Every T0 record binds the original Stage text, Evidence IDs, assumptions, counter evidence, missing proof, peer baseline and snapshot/publication identity.
- 6-, 12- and 24-month observations use the prescribed dimensions and comparable metric definitions, with source quality and evidence references.
- The runtime can schedule or receive each horizon observation and records an immutable receipt; missing or late observations remain visible rather than being filled by inference.
- The evaluator distinguishes incomplete, complete, unknown and unavailable states without calculating investment conclusions.
- At least one real T0 case reaches each elapsed-time horizon with source-bound evidence before the corresponding horizon is reported as proven.

**Verification:** Run validation persistence/application tests, source-bound receipt checks, schedule/recovery tests and a manual evidence audit against the validation strategy. Maintain an explicit evidence ledger for the calendar dates; mocked clock advancement is not accepted as the sole proof.

**Completion boundary:** The data model and a scheduled task are not the same as 6/12/24-month proof. The task remains gated until actual elapsed observations exist.

## Task 5: `wi-research-calibration-score`

**Finding addressed:** The acceptance proposes measuring ORG-X itself: Stage-upgrade precision, downgrade rate, false-positive rate, Top5 persistence, Rising-to-Stage-Upgrade conversion, Dropped correctness, missing-proof resolution and confidence calibration.

**Predecessor:** `wi-validation-runtime-operations`.

**Required acceptance evidence:**

- A reviewed metric contract defines each numerator, denominator, observation window, exclusion rule, minimum sample rule and unknown/unavailable behavior.
- Metrics are calculated only from immutable historical judgments and later source-bound validation outcomes; current facts cannot be rewritten to improve a score.
- Results are reproducible from archived inputs and are reported as research-system calibration, never as company ranking or investment output.
- Insufficient history produces an explicit not-yet-computable state rather than a zero or inferred score.

**Verification:** Use a fixed historical fixture with known outcomes, run deterministic metric tests, perform a denominator and missing-proof audit, and validate that no trading/price/capital output is introduced. Real production validation history is a prerequisite for closing this task.

**Completion boundary:** This is a long-term enhancement, not a gate for the first real Weekly Radar publication. It cannot be marked complete from synthetic fixtures alone.

## Execution handoff

The five task identities above are planned/gated candidates only. Each successor Work Item must be revalidated against the latest `origin/main`, create its own Contract and Summary, declare its own external authority, and deliver its own Outcome before archive. This plan intentionally leaves the current system's external validation status conditional.
