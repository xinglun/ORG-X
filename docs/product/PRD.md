# ORG-X Product Requirements

## Purpose

ORG-X 建立一个 evidence-first radar，持续重建企业核心价值创造过程，判断 AI 是否从局部工具变成新的生产系统，并用后续企业事实验证早期判断。

## Required sequence

`External World -> LLM Extraction -> EvidenceCandidate -> Rust Validation -> EvidenceRecord -> Domain Engine -> Stage -> Score -> Ranking`

LLM 不拥有最终判断权。没有证据不评分；Stage 不由总分替代。

## Six stages

1. `TOOL`：AI 是工具，没有特殊生产方式意义。
2. `SUBSTITUTION`：AI 替代局部人工任务，但旧组织和工作流仍主导。
3. `WORKFLOW`：完整工作流围绕 AI 重构，人从执行者变为监督者，但核心生产系统尚未根本改变。
4. `PRODUCTION_SYSTEM`：核心价值创造过程围绕 AI 重新设计，并出现 workflow、decision、human/agent responsibility 和 organizational adaptation 的多类证据。
5. `PRODUCTIVITY_BREAKOUT`：Stage 3 之外，生产率相对同行持续分化并被经济结果捕获。
6. `REFERENCE_MODEL`：优势持续、竞争者模仿、行业扩散，企业成为新的生产范式示范。

## Funnel

先用结构化数据和确定性过滤压缩 US-listed universe，再逐层提高财务、行业、劳动力、filing、transcript、job data、交叉来源和反证要求。MVP 从去重后的 S&P 500 + Nasdaq 100 开始。

## Calibration

META、PLTR、MSFT、GOOG、AMZN、NVDA、CRM、ADBE、IBM、WMT 只作为不同状态的人工已知候选。系统不得内置任何公司必须进入 Top5 的结论。
