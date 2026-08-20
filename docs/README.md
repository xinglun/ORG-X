# ORG-X 文档导航

这组文档说明 ORG-X 研究什么、如何判断企业生产方式变化、数据如何进入证据链，以及如何运行 Weekly Radar。阅读路径按概念和使用场景组织，读者可以按需选择阅读深度。

## 快速查看能力

- [能力一览](CAPABILITIES.md)：按产品与研究能力、证据与数据能力、系统与运行能力、工程质量与治理能力分组，查看能力状态、边界及详细证据链接。

## 按目标阅读

- 研究读者：先看 [North Star](product/NORTH_STAR.md)、[产品要求](product/PRD.md) 和 [范围与边界](product/SCOPE.md)。
- 贡献者：先看 [架构](architecture/ARCHITECTURE.md)、[Bounded Contexts](architecture/BOUNDED_CONTEXTS.md) 和 [依赖规则](architecture/DEPENDENCY_RULES.md)。
- 运维者：直接看 [Weekly Radar 使用说明](operations/WEEKLY_RADAR.md)，再用 [能力一览](CAPABILITIES.md) 核对运行边界。
- 审阅者：先看 [能力一览](CAPABILITIES.md)，再沿每行的实现、测试和规范链接核对证据。

## 推荐阅读路径

### 1. 先理解产品

- [North Star](product/NORTH_STAR.md)：研究问题和输出含义。
- [产品要求](product/PRD.md)：从外部资料到研究排序的判断链。
- [范围与边界](product/SCOPE.md)：研究什么，以及明确不做什么。

### 2. 再理解判断模型

- [Evidence Model](domain/EVIDENCE_MODEL.md)：证据、溯源、反证和缺失证据。
- [Production System Model](domain/PRODUCTION_SYSTEM_MODEL.md)：如何识别核心生产方式重写。
- [Transformation Stage Model](domain/TRANSFORMATION_STAGE_MODEL.md)：六个 Stage 及其升级边界。
- [Ranking Model](domain/RANKING_MODEL.md)：Stage 内如何形成研究优先级。
- [Scoring Specification](scoring/SCORING_SPEC.md)：辅助排序分数的维度和限制。
- [Stage Gate Specification](scoring/STAGE_GATE_SPEC.md)：状态转换需要哪些证据。

### 3. 了解证据和数据

- [Data Source Policy](data/DATA_SOURCE_POLICY.md)：来源优先级、免费数据栈和权威性。
- [Data Quality Policy](data/DATA_QUALITY_POLICY.md)：可用性（Availability）、时效性（Freshness）、权威性（Authority）、完整性（Completeness）和可信度（Confidence）。
- [Validation Strategy](validation/VALIDATION_STRATEGY.md)：如何用 6/12/24 个月事实验证早期判断。

### 4. 了解系统边界

- [Architecture](architecture/ARCHITECTURE.md)：模块结构和依赖方向。
- [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)：各研究上下文的职责。
- [Dependency Rules](architecture/DEPENDENCY_RULES.md)：Domain、Provider、ACL 和 Infrastructure 的隔离规则。

### 5. 运行 Weekly Radar

- [Weekly Radar Operations](operations/WEEKLY_RADAR.md)：命令、环境变量、Telegram、调度和 `data` 分支保留策略。

## 统一术语

| 术语 | 含义 |
| --- | --- |
| Evidence Candidate | 从外部资料中抽取、尚未完成验证的候选事实。 |
| EvidenceRecord | 带有来源、日期、内容 hash、置信度和极性的可追溯事实。 |
| `UNKNOWN` | 资料存在但规则无法可靠确认，或证据存在歧义/冲突。 |
| `UNAVAILABLE` | 所需来源或字段没有提供、无法取得或没有配置。 |
| Stage | 企业 AI 生产方式转型所处的状态，不等同于 Score。 |
| Production System | 企业持续创造核心价值的整体生产方式。 |

## 阅读原则

- 先看证据，再看 Stage 和 Score。
- 一手来源优先；发现型来源不能直接升级权威性。
- 不把“使用 AI”、裁员、合作公告或单一新闻直接当成生产方式革命。
- 不能可靠确认的事实保留 `UNKNOWN`，不做经验推断。
