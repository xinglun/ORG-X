# ADR-009: ORG-X Does Not Produce Trading Decisions

## Status

Accepted

## Context

ORG-X 研究企业生产方式的演化；把研究优先级混入买卖、价格或仓位决策会跨越 Mission Boundary，并与 Sentinel 混淆。

## Decision

ORG-X 不产生技术分析、价格预测、买卖时点、目标价、仓位、Portfolio Management、NO_TRADE / PROBE / READY 或 Trading Gate。Top5 是 Research Priority。

## Consequences

系统可以深入研究结构性生产率变化而不承担资本行动结论；任何资本决策必须在系统外由人或独立系统处理。

## Enforcement

`NORTH_STAR.md`、`docs/product/SCOPE.md`、`README.md` 和与 Sentinel 的边界声明共同执行。
