# Data Quality Policy

## Required dimensions

每个关键数据都必须记录：

- Availability
- Freshness
- Authority
- Completeness
- Confidence

值本身不能脱离来源、有效日期和置信度存在。没有权威来源时使用 `UNAVAILABLE`；未知事实使用 `UNKNOWN`，不得根据经验估算并伪装成事实。

## Conflicts

冲突数据不得平均。例如 SEC 的员工数与第三方估算不同，应保存 primary authoritative fact，同时保存 conflicting secondary observation 和冲突原因。冲突本身可能是组织变化的证据。

## Quality effects

Freshness、Authority、Completeness 和 Confidence 独立影响证据质量。低质量数据可以作为上下文，但不能绕过 Stage Gate 或单独推动 Stage Upgrade。每次质量变化都必须可追溯到来源和采集时间。
