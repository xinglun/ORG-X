# ORG-X Bounded Contexts

| Context | Responsibility | Representative future concepts |
| --- | --- | --- |
| Universe | 判断哪些公司进入观察范围 | Company, Security, Listing, UniverseSnapshot, EligibilityPolicy |
| Ingestion | 可靠带入外部事实，不解释意义 | Provider port, observation, ingestion receipt |
| Evidence | 保存可追溯、可验证的事实 | EvidenceRecord, source, polarity, confidence, freshness |
| Production System | 重建企业如何创造核心价值 | ProductionSystem, ProductionUnit, Workflow, HumanRole, AgentRole |
| Organization | 记录与生产系统重写相关的组织适配 | ManagementCommitment, responsibility, budget, decision rights |
| Productivity | 验证生产方式的经济结果 | RevenuePerEmployee, OperatingIncomePerEmployee, FCFPerEmployee |
| Transformation | 管理六阶段与状态转换原因 | Stage, transition, supporting/counter/missing proof |
| Diffusion | 观察新生产方式是否被行业模仿 | CompetitorImitation, job taxonomy, benchmark, diffusion |
| Ranking | 在同 Stage 内按证据质量排序 | EvidenceConfidence, TransformationScore, CounterEvidenceRisk |
| Reporting | 生成研究优先级视图 | Top5, Rising, Watch, Dropped, research packet |

Context 之间只通过显式的 Application/Domain ports 或只读快照交流。外部结构必须在 ACL 中转换。
