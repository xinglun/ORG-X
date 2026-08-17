# Scoring Specification

## Transformation Score

| Dimension | Weight |
| --- | ---: |
| Production System Rewrite | 30 |
| Workflow Rewrite | 15 |
| Organizational Adaptation | 15 |
| Management Commitment | 10 |
| Productivity Divergence | 20 |
| Economic Capture | 5 |
| Diffusion / Reinforcement | 5 |
| **Total** | **100** |

Score 只能在证据存在后计算，并且只能在同一 Stage 内作为辅助排序信号。它不能掩盖 Stage 差异，也不能把低 Stage 的高叙事分数排到高置信 Stage 之前。

## 独立维度

`Evidence Confidence`、`Counter Evidence Risk` 和 `Evidence Freshness` 必须独立保存。任何正面分数都必须同时检查 Supporting Evidence、Counter Evidence 和 Missing Evidence。没有反证审查的候选不能进入 Top5。

## Theater control

当 AI mentions、partnerships 或 press releases 增长，但 Workflow、Organization、headcount structure、productivity 和 margin 没有可解释变化时，标记 `AI_THEATER_RISK = HIGH` 并降级。
