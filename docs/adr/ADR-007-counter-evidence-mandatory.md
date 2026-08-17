# ADR-007: Counter Evidence Is Mandatory

## Status

Accepted

## Context

只收集 Supporting Evidence 会把研究系统变成漂亮叙事生成器，无法承认 workflow 回退、生产率不持续或实验失败。

## Decision

任何正面判断必须主动寻找 Counter Evidence 和 Missing Evidence。没有 Counter Evidence Review，候选不得进入 Top5。

## Consequences

研究 packet 会同时呈现支持、反证和缺失证明；候选可能降级或从 Stage 3 回到 Stage 2。

## Enforcement

`docs/domain/EVIDENCE_MODEL.md`、`docs/scoring/SCORING_SPEC.md` 和未来 Counter Evidence Engine 的验收标准执行。
