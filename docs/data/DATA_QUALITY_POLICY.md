# Data Quality Policy

## 每个关键事实要回答什么

每个关键数据都保留以下维度：

- `Availability`：来源或字段是否可取得。
- `Freshness`：资料的有效日期和采集时间。
- `Authority`：来源在证据层级中的权威性。
- `Completeness`：所需字段和上下文是否齐全。
- `Confidence`：规则验证后对事实身份的信心。

值不能脱离来源、有效日期、内容片段或结构化字段和置信度单独存在。

## `UNKNOWN` 与 `UNAVAILABLE`

- `UNKNOWN`：资料存在，但规则无法可靠确认；例如歧义、冲突、日期不明、对象不相关或格式不完整。
- `UNAVAILABLE`：所需来源、字段或可选配置没有提供或无法取得。

两者都不能用经验估算、默认值或缺失推断替代。

## 冲突处理

冲突数据不平均。例如 SEC 员工数与第三方估算不同，应保留一手权威事实、冲突观察和冲突原因。冲突本身可能是组织变化的线索，但不能未经验证升级 Stage。

## 对研究判断的影响

Freshness、Authority、Completeness 和 Confidence 独立影响证据质量。低质量数据可以作为上下文，但不能绕过 Stage Gate 或单独推动 Stage Upgrade。每次质量变化都必须能追溯到来源和采集时间。
