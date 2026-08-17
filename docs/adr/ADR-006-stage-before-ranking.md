# ADR-006: Stage Before Ranking

## Status

Accepted

## Context

总分会把已经跨过生产系统临界点的公司与仍处在工具阶段的公司混在一起，造成不可解释的排名。

## Decision

先判断 Stage，再按 Evidence Confidence、Transformation Score、Counter Evidence Risk 和 Freshness 在同一 Stage 内排序。

## Consequences

高置信 Stage 4 可以优先于高分 Stage 1；研究排序更少迎合单一叙事，Stage 判定质量成为第一优先级。

## Enforcement

`docs/domain/RANKING_MODEL.md`、`docs/scoring/STAGE_GATE_SPEC.md` 与未来 Ranking Engine Contract 执行。
