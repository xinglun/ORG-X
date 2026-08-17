# Ranking Model

## Ordering

排名顺序固定为：

1. Stage
2. Evidence Confidence
3. Transformation Score
4. Counter Evidence Risk
5. Evidence Freshness

高置信 Stage 4 优先于高分 Stage 1。不得使用单一总分掩盖阶段差异。

## Score relationship

Transformation Score 是同 Stage 内的辅助比较，不是 Stage 的替代品。独立记录 Evidence Confidence、Counter Evidence Risk 和 Freshness；它们不能被隐藏在一个总分里。

## Research views

系统维护 Top5、Rising、Watch 和 Dropped。排名变化必须说明证据变化，例如 workflow transfer strengthened、management commitment confirmed、productivity divergence emerging 或 counter evidence remains。Dropped 是允许纠错的正式结果。
