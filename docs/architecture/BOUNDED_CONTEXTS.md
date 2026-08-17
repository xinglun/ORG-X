# ORG-X Bounded Contexts

| Context | 读者问题 | 代表概念 |
| --- | --- | --- |
| Universe | 哪些公司进入观察范围？ | Company, Security, Listing, UniverseSnapshot, EligibilityPolicy |
| Ingestion | 如何可靠带入外部事实？ | Provider port, observation, ingestion receipt |
| Evidence | 事实能否追溯和验证？ | EvidenceRecord, source, polarity, confidence, freshness |
| Production System | 企业如何创造核心价值？ | ProductionSystem, ProductionUnit, Workflow, HumanRole, AgentRole |
| Organization | 组织如何适配生产系统重写？ | ManagementCommitment, responsibility, budget, decision rights |
| Productivity | 生产方式是否产生经济结果？ | RevenuePerEmployee, OperatingIncomePerEmployee, FCFPerEmployee |
| Transformation | 企业处于哪个 Stage？ | Stage, transition, supporting/counter/missing proof |
| Diffusion | 新生产方式是否被行业模仿？ | CompetitorImitation, job taxonomy, benchmark, diffusion |
| Ranking | 同一 Stage 内先研究谁？ | EvidenceConfidence, TransformationScore, CounterEvidenceRisk |
| Reporting | 如何形成研究优先级视图？ | Top5, Rising, Watch, Dropped, research packet |

Context 之间只通过显式的 Application/Domain ports 或只读快照交流。外部结构必须在 ACL 中转换。
