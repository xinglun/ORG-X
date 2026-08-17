# Ranking Model

## 排序顺序

研究优先级固定按以下顺序比较：

1. `Stage`
2. `Evidence Confidence`
3. `Transformation Score`
4. `Counter Evidence Risk`
5. `Evidence Freshness`

高置信 Stage 4 优先于高分 Stage 1。不能用单一总分掩盖阶段差异。

## Score 的位置

Transformation Score 只在同一 Stage 内作为辅助比较，不是 Stage 的替代品。`Evidence Confidence`、`Counter Evidence Risk` 和 `Freshness` 独立保存，不能隐藏在一个总分里。

## 研究视图

系统维护 `Top5`、`Rising`、`Watch` 和 `Dropped`。排名变化必须说明证据变化，例如 Workflow transfer strengthened、Management commitment confirmed、Productivity divergence emerging 或 Counter Evidence remains。`Dropped` 是允许纠错的正式结果。
