# ORG-X 能力一览

本页是 ORG-X 的能力地图和阅读入口。它回答“现在具备什么能力、能力边界是什么、详细证据在哪里”，不替代各领域的完整规范。

状态只根据当前仓库中的源码、自动化测试和读者文档判断：

- **已具备（源码/测试）**：仓库内已有明确实现，并有对应测试或结构证据。
- **边界已具备**：规则、接口或适配边界已存在，但能力仍受来源、fixture、配置或运行环境限制。
- **外部验证边界**：本地证据不足以证明真实 provider 或生产环境行为；这里不把它写成已验证。
- **非目标**：项目明确不承担的能力，不应从名称或输出格式中推导出来。

## 产品与研究能力

表格中的“规范/使用说明”用于继续阅读，“实现/测试证据”用于核对当前仓库事实；二者都不代表真实 provider 或生产业务结果已经取得。

| 能力 | 状态 | 能力边界 | 规范/使用说明 | 实现/测试证据 |
| --- | --- | --- | --- | --- |
| 研究问题与输出定义 | 已具备（源码/测试） | 研究企业生产方式变化，不把研究排序当作资本行动。 | [North Star](product/NORTH_STAR.md)、[产品要求](product/PRD.md)、[范围与边界](product/SCOPE.md) | [Weekly Radar domain](../src/features/weekly_radar/domain/mod.rs)、[domain tests](../tests/weekly_radar_contract.rs) |
| 研究对象与 Universe 规则 | 已具备（源码/测试） | 只评估输入中明确提供且满足规则的对象，不凭经验补全外部名单。 | [范围与边界](product/SCOPE.md)、[Evidence Model](domain/EVIDENCE_MODEL.md) | [Universe domain](../src/features/universe/domain/mod.rs)、[Universe tests](../tests/universe_domain.rs) |
| Production System 建模 | 已具备（源码/测试） | 保留企业核心生产方式、责任和证据，不把单一 AI 使用描述升级为生产方式重写。 | [Production System Model](domain/PRODUCTION_SYSTEM_MODEL.md)、[产品要求](product/PRD.md) | [Production System domain](../src/features/production_system/domain/mod.rs)、[tests](../tests/production_system_domain.rs) |
| Transformation Stage | 已具备（源码/测试） | Stage 是基于证据的状态，不等同于 Score；状态转换需要明确证据。 | [Transformation Stage Model](domain/TRANSFORMATION_STAGE_MODEL.md)、[Stage Gate Specification](scoring/STAGE_GATE_SPEC.md) | [Transformation domain](../src/features/transformation/domain/mod.rs)、[tests](../tests/transformation_domain.rs) |
| Organization 组织适配 | 已具备（源码/测试） | 能记录管理承诺、责任、预算和决策权等证据；当前不代表已接入真实企业组织数据。 | [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Organization design](superpowers/plans/2026-08-17-wi-006-organization.md) | [Organization domain](../src/features/organization/domain/mod.rs)、[tests](../tests/organization_domain.rs) |
| Productivity 生产率事实 | 已具备（源码/测试） | 能保留可比较的生产率指标和人力变化事实；当前不自动计算经济显著性，也不输出投资结论。 | [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Productivity design](superpowers/specs/2026-08-17-wi-007-productivity-design.md) | [Productivity domain](../src/features/productivity/domain/mod.rs)、[tests](../tests/productivity_domain.rs) |
| Diffusion 行业扩散 | 已具备（源码/测试） | 能记录竞品模仿、职位分类和基准观察等扩散事实；当前不代表已取得行业真实面板数据。 | [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Diffusion design](superpowers/specs/2026-08-17-wi-009-diffusion-design.md) | [Diffusion domain](../src/features/diffusion/domain/mod.rs)、[tests](../tests/diffusion_domain.rs) |
| Ranking、Score 与 Top5 研究优先级 | 已具备（源码/测试） | 只表达研究资源优先级，不输出买入、持有、卖出、价格或仓位结论。 | [Ranking Model](domain/RANKING_MODEL.md)、[Scoring Specification](scoring/SCORING_SPEC.md) | [Ranking domain](../src/features/ranking/domain/mod.rs)、[Top5 tests](../tests/weekly_radar_top5.rs) |
| Rising / Dropped 结构变化 | 已具备（源码/测试） | 只根据可比较的结构事实产生变化，不把价格或排名变化当作生产方式变化。 | [范围与边界](product/SCOPE.md)、[Evidence Model](domain/EVIDENCE_MODEL.md) | [Rising/Dropped](../src/features/weekly_radar/domain/rising_dropped.rs)、[tests](../tests/weekly_radar_rising_dropped.rs) |

## 证据与数据能力

| 能力 | 状态 | 能力边界 | 规范/使用说明 | 实现/测试证据 |
| --- | --- | --- | --- | --- |
| Ingestion 外部事实接入边界 | 边界已具备 | 能把 observation 和 ingestion receipt 作为 provider-neutral 边界输入；当前不等于已接入某个真实 provider。 | [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Data Source Policy](data/DATA_SOURCE_POLICY.md) | [Ingestion domain](../src/features/ingestion/domain/mod.rs)、[tests](../tests/ingestion_domain.rs) |
| Evidence Candidate、溯源与反证 | 已具备（源码/测试） | 外部资料先作为候选证据；来源、日期、内容和极性必须可追溯。 | [Evidence Model](domain/EVIDENCE_MODEL.md)、[Data Source Policy](data/DATA_SOURCE_POLICY.md) | [Evidence domain](../src/features/evidence/domain/mod.rs)、[tests](../tests/evidence_domain.rs) |
| 一手来源优先与 discovery 隔离 | 边界已具备 | 发现型来源可以帮助定位材料，但不能直接升级为权威证据。 | [Data Source Policy](data/DATA_SOURCE_POLICY.md) | [Runtime sources](../src/features/weekly_radar/runtime/sources.rs)、[runtime tests](../tests/weekly_radar_runtime.rs) |
| UNKNOWN / UNAVAILABLE 与数据质量 | 已具备（源码/测试） | 资料歧义、来源冲突或字段缺失时保留不确定性，不做经验推断。 | [Data Quality Policy](data/DATA_QUALITY_POLICY.md)、[Evidence Model](domain/EVIDENCE_MODEL.md) | [Runtime rules](../src/features/weekly_radar/runtime/rules.rs)、[runtime tests](../tests/weekly_radar_runtime.rs) |
| 长期验证与历史事实 | 边界已具备 | 能按 6/12/24 个月保存和评估 supplied facts；当前是内存 store 的 application/domain 边界，不接入 Weekly Radar runtime、不调度真实验证周期、不连接外部 Provider，也不声称已有生产验证结果。 | [Validation Strategy](validation/VALIDATION_STRATEGY.md) | [Validation domain](../src/features/validation/domain/mod.rs)、[validation tests](../tests/validation_domain.rs) |

## 系统与运行能力

本节面向需要理解架构边界、运行顺序和失败恢复的贡献者与运维者；不熟悉术语时，优先打开每行的规范链接。

| 能力 | 状态 | 能力边界 | 规范/使用说明 | 实现/测试证据 |
| --- | --- | --- | --- | --- |
| DDD / Clean Architecture 与上下文隔离 | 已具备（源码/测试） | Domain 不依赖 Infrastructure；provider 细节通过边界隔离。 | [Architecture](architecture/ARCHITECTURE.md)、[Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Dependency Rules](architecture/DEPENDENCY_RULES.md) | [Architecture tests](../tests/architecture/dependency_rules.rs)、[module-boundary tests](../tests/architecture/module_boundaries.rs) |
| Weekly Radar 采集、规则化、渲染 | 已具备（源码/测试） | 计算、持久化、渲染和发布按固定顺序执行；dry-run 不发送、不归档。 | [Weekly Radar Operations](operations/WEEKLY_RADAR.md) | [Runtime entrypoint](../src/main.rs)、[end-to-end tests](../tests/weekly_radar_end_to_end.rs) |
| 本地化报告与 Telegram 消息边界 | 边界已具备 | 支持中文、日文、英文和语义分片；真实 Telegram provider 行为不由本地 fixture 代替。 | [Weekly Radar Operations](operations/WEEKLY_RADAR.md) | [Report renderer](../src/features/weekly_radar/runtime/report.rs)、[Telegram renderer tests](../tests/weekly_radar_telegram_renderer.rs) |
| Publication Receipt 与同输入 retry | 边界已具备 | 保留已接受的 message IDs 和尝试次数，retry 使用保存的输入；不会凭空生成成功 receipt。 | [Weekly Radar Operations](operations/WEEKLY_RADAR.md) | [Publication receipt](../src/features/weekly_radar/infrastructure/publication_receipt.rs)、[receipt tests](../tests/weekly_radar_publication_receipt.rs) |
| 日期归档、逻辑提交与崩溃恢复 | 边界已具备 | 文件仍逐个 rename；先写 prepared 记录，全部完成后才写 committed。中断或不一致时停止并保留状态供 recovery 处理；不宣称跨文件物理原子事务。 | [Weekly Radar Operations](operations/WEEKLY_RADAR.md) | [Archive runtime](../src/features/weekly_radar/runtime/archive.rs)、[archive tests](../tests/weekly_radar_runtime.rs) |
| Weekly schedule 与 data 分支保留 | 边界已具备 | 调度、orphan data 分支和保留策略已声明；生产运行结果仍需外部环境验证。 | [Weekly Radar Operations](operations/WEEKLY_RADAR.md) | [Workflow](../.github/workflows/weekly-radar.yml)、[scheduler tests](../tests/weekly_radar_scheduler.rs) |
| Reporting 研究优先级视图 | 已具备（源码/测试） | 能从研究卡片组装 Top5、Rising、Watch、Dropped 的 read model；当前是确定性组装边界，不代表已有真实 provider 数据。 | [Bounded Contexts](architecture/BOUNDED_CONTEXTS.md)、[Ranking Model](domain/RANKING_MODEL.md) | [Reporting domain](../src/features/reporting/domain/mod.rs)、[tests](../tests/reporting_domain.rs) |

## 工程质量与治理能力

本节面向贡献者、审阅者和发布维护者；这些条目说明仓库的质量与治理门禁，不是产品功能承诺。

| 能力 | 状态 | 能力边界 | 规范/使用说明 | 实现/测试证据 |
| --- | --- | --- | --- | --- |
| Rust 格式、Clippy 与完整测试 | 已具备（源码/测试） | 本地质量门禁覆盖格式、禁止警告的 Clippy 和完整 Cargo 测试。 | [根目录检查说明](../README.md) | [Makefile](../Makefile)、[Cargo manifest](../Cargo.toml) |
| 文档元数据与内部链接检查 | 已具备（源码/测试） | reader 文档检查一级标题、内部链接、过程标记和 Weekly Radar 事实对齐。 | [文档导航](README.md) | [Documentation checker](../scripts/check_docs_metadata.py)、[Makefile.ai](../Makefile.ai) |
| Governed Work Item 与证据交接 | 边界已具备 | Contract、Summary、Finish、archive、PR 和 closure 形成可审计交接；它是工程治理能力，不是产品功能。 | [AI Cockpit README](../.ai/cockpit/README.md)、[工程规则](../AGENTS.md) | [Work Item records](../.ai/work-items/archive/index.json) |

## 明确的外部验证边界

以下事实不能仅凭本地源码或 fixture 测试宣称已经完成：

- 真实 Telegram、SEC、IR、招聘来源或其他 provider 的生产调用结果。
- 生产环境凭据、真实数据分支运行和 hosted workflow 的业务数据结果。
- 归档多文件之间的物理原子性；当前实现提供的是逻辑提交点和可恢复的 fail-closed 行为。

操作命令、环境变量和 provider 限制以 [Weekly Radar Operations](operations/WEEKLY_RADAR.md) 为准。

## 明确的非目标

- LLM 直接替代规则判断或未经溯源的事实抽取；详见 [Data Source Policy](data/DATA_SOURCE_POLICY.md)。
- 买入、持有、卖出、价格预测、仓位、资本行动或其他交易决策；详见 [范围与边界](product/SCOPE.md)。
- 将单一新闻、AI 工具使用、裁员或合作公告自动升级为生产方式重写；详见 [Evidence Model](domain/EVIDENCE_MODEL.md)。
