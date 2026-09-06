# Scoring Specification

本篇描述 `src/features/weekly_radar/runtime/judgment.rs` 中 `build_ranking_candidate` 实际计算的四个值，供读者对照代码复核。这不是独立设计的目标式，而是现状实现的镜像；如需更换算法，应先在此修改设计再改代码，避免文档与实现再次分叉。

## Transformation Score

`transformation_score = min(100, Stage.rank() * 20 + supporting_count * 10)`

- `Stage.rank()`：`docs/domain/TRANSFORMATION_STAGE_MODEL.md` 中六个 Stage 的序号（`TOOL` 起始）。
- `supporting_count`：判断链中已验证的 Supporting Evidence 数量。
- Score 只能在证据存在后计算，并且只能在同一 Stage 内作为辅助排序信号（见 `docs/domain/RANKING_MODEL.md` 的排序顺序）。它不能掩盖 Stage 差异，也不能把低 Stage 的高分排到高置信 Stage 之前。

## 独立维度

- `Evidence Confidence = min(100, supporting_count * 25 + 25)`
- `Counter Evidence Risk = min(100, counter_count * 25)`
- `Evidence Freshness`：按最新证据的 `effective_date` 与评估截止日 `cutoff` 的天数差分档 —— `<=30` 天为 100，`<=90` 天为 80，`<=365` 天为 60，否则 30；没有任何带日期的证据时取 50。

以上三项与 `Transformation Score` 一样独立保存，不合并成一个总分。任何正面分数都必须同时检查 Supporting Evidence、Counter Evidence 和 Missing Evidence；没有反证审查的候选不能进入 Top5（由 `docs/domain/EVIDENCE_MODEL.md` 的三类证据规则保证）。

## Theater control（尚未实现）

`AI_THEATER_RISK` 标记（当 AI mentions、partnerships 或 press releases 增长，但 Workflow、Organization、headcount structure、productivity 和 margin 没有可解释变化时降级候选）目前只是设计意图，代码中未找到任何实现（`rg AI_THEATER_RISK src/` 无匹配）。在实现前，不应假设产出报告已具备这一层反 Hype 保护。
