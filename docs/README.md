# ORG-X 文档导航

这里的文档帮助你理解 ORG-X 研究什么、如何阅读研究结果、如何核对证据，以及使用每周研究报告时需要注意什么。建议先按自己的目标选择入口，再按需深入。

## 先看能力一览

[能力一览](CAPABILITIES.md)用用户能理解的方式说明：你可以完成什么、会得到什么、哪些地方有边界，以及每项能力的详情入口。

## 按目标阅读

- **想知道 ORG-X 是什么**：先看[研究目的（North Star）](product/NORTH_STAR.md)、[产品要求](product/PRD.md)和[范围与边界](product/SCOPE.md)。
- **想研究一家企业**：看[生产系统模型](domain/PRODUCTION_SYSTEM_MODEL.md)、[转型阶段模型](domain/TRANSFORMATION_STAGE_MODEL.md)和[排名模型](domain/RANKING_MODEL.md)。
- **想核对一条判断**：看[证据模型](domain/EVIDENCE_MODEL.md)、[数据来源规则](data/DATA_SOURCE_POLICY.md)和[数据质量规则](data/DATA_QUALITY_POLICY.md)。
- **想理解研究标签**：Top5（优先阅读）、Rising（证据增强）、Watch（继续观察）、Dropped（证据减弱）；详情看[排名模型](domain/RANKING_MODEL.md)和[评分说明](scoring/SCORING_SPEC.md)。
- **想了解每周报告如何生成和阅读**：看[Weekly Radar 使用说明](operations/WEEKLY_RADAR.md)。
- **想了解判断能否被后续事实支持**：看[长期验证策略](validation/VALIDATION_STRATEGY.md)。
- **想确认不提供什么**：看[能力一览](CAPABILITIES.md#明确不提供什么)和[范围与边界](product/SCOPE.md)。

## 推荐阅读顺序

### 1. 先理解研究问题

- [研究目的（North Star）](product/NORTH_STAR.md)：ORG-X 要回答什么问题。
- [产品要求](product/PRD.md)：从资料到研究排序的判断路径。
- [范围与边界](product/SCOPE.md)：研究什么，以及明确不做什么。

### 2. 再理解判断依据

- [证据模型](domain/EVIDENCE_MODEL.md)：证据、来源、反证和缺失资料。
- [生产系统模型](domain/PRODUCTION_SYSTEM_MODEL.md)：如何识别核心生产方式变化。
- [转型阶段模型](domain/TRANSFORMATION_STAGE_MODEL.md)：变化所处的阶段和升级边界。
- [排名模型](domain/RANKING_MODEL.md)：如何形成研究优先级。
- [评分说明](scoring/SCORING_SPEC.md)：排序分数的维度和限制。
- [阶段门槛](scoring/STAGE_GATE_SPEC.md)：阶段判断需要哪些证据。

### 3. 最后查看资料和跟踪边界

- [数据来源规则](data/DATA_SOURCE_POLICY.md)：来源优先级和权威性。
- [数据质量规则](data/DATA_QUALITY_POLICY.md)：可用性、时效性、权威性、完整性和可信度。
- [长期验证策略](validation/VALIDATION_STRATEGY.md)：如何用 6、12、24 个月的后续事实回看判断。
- [Weekly Radar 使用说明](operations/WEEKLY_RADAR.md)：每周报告、消息发布和外部服务边界。

## 统一术语

| 术语 | 面向读者的含义 |
| --- | --- |
| 研究对象 | 被纳入当前研究范围、可以继续核对资料的企业或案例。 |
| 证据 | 能够支持或反驳某个判断、并且可以追溯到资料来源的事实。 |
| 未知 | 有相关资料，但目前无法可靠确认。 |
| 不可用 | 需要的资料、字段或服务没有提供、无法取得或没有配置。 |
| 转型阶段（Transformation Stage） | 企业 AI 生产方式变化所处的阶段，不等同于分数。 |
| 生产系统（Production System） | 企业持续创造核心价值的整体生产方式。 |
| 研究优先级 | 帮助安排阅读顺序的结果，不是公司好坏排名或投资建议。 |

## 阅读原则

- 先看资料和证据，再看阶段和研究优先级。
- 一手来源优先；发现型资料用于定位线索，不能直接升级为权威结论。
- 不把“使用 AI”、裁员、合作公告或单一新闻直接当成生产方式变化。
- 不能可靠确认的事实保留“未知”，不做经验推断。
