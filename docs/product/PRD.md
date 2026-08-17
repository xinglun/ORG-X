# ORG-X Product Model

## Purpose

ORG-X 建立一个 evidence-first radar，持续重建企业核心价值创造过程，判断 AI 是否从局部工具变成新的生产系统，并用后续企业事实验证早期判断。

## Judgment chain

```text
External World
  -> Rule Extraction
  -> Evidence Candidate
  -> Rust Validation
  -> EvidenceRecord
  -> Domain Engine
  -> Stage
  -> Score
  -> Ranking
```

规则抽取只产生候选事实；Rust 负责验证、保留溯源、判断状态和生成排序。没有证据不评分，Stage 不能被总分替代。

## Six stages

| Stage | 读者含义 |
| --- | --- |
| `TOOL` | AI 是工具，尚未显示特殊生产方式意义。 |
| `SUBSTITUTION` | AI 替代局部人工任务，但旧组织和 Workflow 仍主导。 |
| `WORKFLOW` | 完整 Workflow 围绕 AI 重构，人从执行者转为监督者，但核心生产系统尚未根本改变。 |
| `PRODUCTION_SYSTEM` | 核心价值创造过程围绕 AI 重设计，并出现 Workflow、Decision、Human/Agent Responsibility 和 Organization Adaptation 的多类证据。 |
| `PRODUCTIVITY_BREAKOUT` | 新生产方式产生持续的同行相对生产率差异，并被经济结果捕获。 |
| `REFERENCE_MODEL` | 优势持续、竞争者模仿、行业扩散，企业成为新的生产范式示范。 |

## Research funnel

系统从去重后的 S&P 500 + Nasdaq 100 观察宇宙开始，先做确定性过滤，再逐层提高财务、行业、劳动力、filing、招聘、交叉来源、持续性和反证要求。

## Calibration universe

META、PLTR、MSFT、GOOG、AMZN、NVDA、CRM、ADBE、IBM、WMT 只作为人工已知候选，用于校准不同状态。系统不内置任何公司必须进入 Top5 的结论。
