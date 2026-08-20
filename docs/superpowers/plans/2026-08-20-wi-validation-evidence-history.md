# Validation Evidence History Implementation Plan

> **For agentic workers:** Execute this plan inside the governed `wi-validation-evidence-history` Work Item. Keep the Contract and Summary authoritative for scope and evidence.

## Goal

将 `docs/validation/VALIDATION_STRATEGY.md` 中的 T0 → 6/12/24 个月验证要求落成一个只保存证据与完整性状态的 Rust Bounded Context；不擅自推导 Stage、评分、排名或投资结论。

## Constraints

- 只保存调用方已经提供的 opaque strings、metrics、source quality 和 evidence references。
- 基线保存 Stage 文本、Evidence IDs、hypotheses、counter evidence、missing proof、peer baseline。
- 后续观察只覆盖 `SixMonths`、`TwelveMonths`、`TwentyFourMonths`，重复 horizon 必须拒绝。
- Evaluator 只报告缺失 horizon / completeness，不计算经济结果或研究判断。
- 遵守五层 bounded-context 结构，不导入其他 feature 的内部模块。
- 不调用外部 Provider，不触发生产工作流，不修改 Telegram 或生产数据。

## Tasks

- [x] 完成 Contract、Summary、preflight 和 `before_edit` checkpoint。
- [x] 先写 domain/store/evaluator 测试，验证保存、拒绝重复、拒绝空值和完整性状态。
- [x] 实现 `validation` context 的五层结构、opaque domain record、in-memory store 和 completeness evaluator。
- [x] 更新 architecture module list、validation strategy、implementation spec 和 roadmap count/handoff；归档前核心研究计数仍为 9，归档后为 10。
- [x] 运行 focused tests、`make quality`、reference-impact 检查，并修复所有真实失败。
- [ ] 记录 active Outcome，归档，提交/推送 PR，验证 hosted checks，合并并运行 `ai-close-work-item`。
- [ ] 最终检查 root/worktree/branch/remote/archive/active residue，并交付 evidence-bound handoff。

## Verification Matrix

| Concern | Evidence | Command / inspection |
| --- | --- | --- |
| Opaque validation retention | `src/features/validation/domain/mod.rs`, focused tests | `cargo test --test validation_domain` |
| Store duplicate safety | in-memory store and tests | `cargo test --test validation_store` |
| Five-layer architecture | `src/features/mod.rs`, architecture test | `cargo test --test architecture` |
| Docs/roadmap alignment | validation strategy, spec, roadmap | targeted `rg`, diff review |
| Project quality | repository Make targets | `make quality` |
| Lifecycle closure | Contract/Summary/Outcome/archive/hosted PR | `make ai-finish`, archive, hosted checks, `make ai-close-work-item` |

## Review Focus

1. Evidence retention must not become hidden judgment logic.
2. Invalid duplicate/blank additions must leave prior state unchanged.
3. Production receipt, full universe authority, runtime judgment-chain integration, and source security policy remain explicit follow-up decisions.
