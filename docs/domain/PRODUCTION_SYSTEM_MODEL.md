# Production System Model

## 要观察的问题

公司创造核心价值的方式是否因为 AI 改变？关注商品、软件、广告、研究成果、金融服务或物流服务如何被生产，而不是员工是否打开了 AI 工具。

## 核心概念

- `ProductionSystem`：创造核心价值的整体生产方式。
- `ProductionUnit`：承担可识别产出的生产单元。
- `ProductionWorkflow`：从意图到交付、验证和例外处理的路径。
- `HumanRole` / `AgentRole`：人和 Agent 各自执行、监督、指导和承担责任的角色。
- `ControlPoint`：人类或系统保留控制的节点。
- `VerificationPoint`：产出被检查、验证或拒绝的节点。
- `DecisionPoint`：目标、取舍、批准和责任发生的节点。
- `ExceptionPath`：正常自动化路径无法处理时的人工介入路径。

## 什么才算重写

传统流程“需求 → PM → 设计 → 开发 → 测试 → 发布”，只有在能够证明 AI 执行、Human supervision、责任和验证结构一起改变时，才可能被判断为 AI-native production system。单独部署 Agent、宣传 digital workforce 或裁员都不够。

## 组织证据的关系

Organization Rewrite 是 Production System Rewrite 的证据，而不是 North Star 本身。每个组织变化都必须解释它如何服务核心生产方式的变化。

## AI 时代行业范本的更高门槛

`ReferenceModel` 不是组织公告或单次生产系统改造的同义词。要进入这一层，必须分别保留四类来源绑定的主张：

1. `OrganizationRewrite`：职责、汇报关系、决策权或组织边界发生了具体重写；
2. `ProductionSystemRewrite`：核心生产工作流、Agent 执行、人类监督或控制点发生了具体重写；
3. `SustainedOutcome`：至少两个不同有效周期的经营或生产结果；
4. `IndustryDiffusion`：至少两个独立来源、带有明确同行/采用者的模仿或扩散证据。

这四类证据通过 `ReferenceModelEvidenceBundle` 进入 `Candidate / Confirmed / NotEligible` 门禁。来源可访问、公司自述、单次发布、招聘页面或新闻线索都不能单独满足任何范本结论；反证复核和缺失证明必须分别保留。`Candidate` 只表示核心重写已出现但完整证据仍未闭合，不表示行业范本。

### 扩散来源的独立性边界

`IndustryDiffusion` 还必须保留来源角色，不能把“供应商发布了客户案例”当成独立验证：

- `SupplierAttribution`：供应商控制的客户案例或技术归因，只能证明供应商叙述了某个采用案例；
- `IndependentCustomerDisclosure`：采用者自己的案例、IR 或经营披露，可用于独立扩散的命名采用者与采用行为；
- `RegulatoryOrFiling`：SEC/IR 的申报或经营结果，主要服务 `SustainedOutcome`，不自动变成扩散证据；
- `DiscoveryOnly`：新闻或二手发现材料，只是线索。

独立扩散门只计入 authoritative 的 `IndependentCustomerDisclosure` 主张，并要求至少两个不同来源 URI 与两个命名采用者/同行。供应商来源数量与独立来源数量必须在评估和报告中分开；旧快照缺少来源角色时按未知处理，不能追溯性地升级为独立证据。
