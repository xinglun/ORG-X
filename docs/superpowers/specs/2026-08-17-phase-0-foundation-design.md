# ORG-X Phase 0 Foundation Design

## Status

Approved for implementation under the user's explicit authorization to proceed through confirmation points.

## Goal

建立 ORG-X 的 Rust 工程基础、DDD Bounded Context 边界、Clean Architecture 依赖规则、核心项目文档、首批 ADR 和可执行 Architecture Tests，使仓库具备进入 AI Cockpit 安装验收的条件，同时不实现任何正式业务功能。

## Governing North Star

ORG-X 寻找 AI 从工具变成生产方式的临界点：持续观察美国上市公司，寻找那些率先围绕 AI 重构核心生产系统、实现结构性生产率跃迁，并可能成为下一代企业范式的公司。

ORG-X 是生产方式研究系统，不是交易决策系统。它不输出买卖、目标价、仓位、交易 Gate 或 NO_TRADE / PROBE / READY 等资本行动结论。

## Scope

本阶段交付：

1. 最小 Rust crate：`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`src/lib.rs`、`src/main.rs`。
2. 以单 crate 模块化方式建立 Universe、Ingestion、Evidence、Production System、Organization、Productivity、Transformation、Diffusion、Ranking、Reporting 及 shared 层的目录边界。
3. 为每个 Bounded Context 建立 `domain`、`application`、`infrastructure`、`interface`、`acl` 五层入口；仅定义边界说明和最小编译模块，不定义业务状态、评分或数据抓取行为。
4. 建立根文档 `NORTH_STAR.md`、`ENGINEERING_PRINCIPLES.md`、`README.md`，以及 `docs/product`、`docs/architecture`、`docs/domain`、`docs/data`、`docs/scoring`、`docs/validation` 下的 Phase 0 文档。
5. 建立 ADR-001 至 ADR-010，固定 DDD、Evidence First、Production System 核心域、LLM/Rust 边界、Stage/Ranks 顺序、反证、来源权威、交易边界和不可变历史快照等决策。
6. 建立不依赖额外运行时库的 Architecture Tests，通过读取源文件并检查禁止依赖令牌，验证依赖方向与 provider-agnostic 规则。
7. 提供本地质量命令，使 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test --all` 可执行。

## Explicitly Out of Scope

- SEC、Investor Relations、新闻、职位或基本面 provider 的实现。
- LLM SDK、EvidenceCandidate、EvidenceRecord、Stage、Score、Ranking、Top5 等业务行为。
- 数据库、迁移、网络调用、定时任务、渲染器和外部 API。
- AI Cockpit 的安装、Work Item 生命周期执行和 WI-001 验证；这些在本阶段所有基础文件与架构测试通过后作为下一阶段进行。
- 任何股票价格预测、技术分析、交易信号、仓位管理或投资建议。

## Design Options Considered

### Option A — 每个 Bounded Context 独立 crate

优点是编译器天然强化边界，适合未来团队并行开发。缺点是空仓库阶段会引入 workspace、跨 crate API 与版本管理成本，容易把工程基础误认为业务进展。

### Option B — 单 crate 的模块化 Bounded Context（selected）

每个 Context 以目录和 Rust module 作为边界，Domain 保持纯 Rust，外层通过未来的 ports/adapters 扩展。Architecture Tests 对关键禁依赖进行可执行守护。该方案适合当前单一 foundation Work Item，并保留未来按稳定边界拆 crate 的选择。

### Option C — 只写文档，推迟代码骨架

能够最快产出思想文档，但无法验证 Rust module 结构、编译入口和架构测试命令，不符合“先建立工程与 Architecture Tests”的硬约束。

选择 Option B，因为它以最低工程复杂度同时满足 DDD/Clean Architecture 的可验证性和后续演进空间。

## Architecture

依赖方向固定为：

```text
interface -> application -> domain
infrastructure -> application/domain ports
acl -> translates external provider shapes at the boundary
```

Phase 0 不创建 provider 或持久化实现；因此 `infrastructure` 与 `acl` 只保留编译安全的模块说明入口。未来外部 JSON、SEC schema 或 LLM 输出必须先经过 ACL 和 Application/Domain port，禁止传播到 Domain。

### Bounded Contexts

- `universe`：公司、证券、上市资格和 Universe snapshot 的候选边界。
- `ingestion`：外部事实可靠进入系统的 provider 边界，不解释意义。
- `evidence`：EvidenceRecord、来源、时间、置信度、freshness 和反证的事实边界。
- `production_system`：核心生产方式、ProductionWorkflow、Agent/Human roles、control/verification/decision points 的核心域。
- `organization`：管理承诺、团队、责任、预算与组织适配，作为生产系统重写的证据。
- `productivity`：Revenue/Employee、Operating Income/Employee、FCF/Employee、持续性与同行差异的验证边界。
- `transformation`：六阶段 TOOL、SUBSTITUTION、WORKFLOW、PRODUCTION_SYSTEM、PRODUCTIVITY_BREAKOUT、REFERENCE_MODEL 的状态转换边界。
- `diffusion`：竞争者模仿、行业重构、工作分类、基准与资本再配置的扩散边界。
- `ranking`：先 Stage，再 Evidence Confidence、Transformation Score、Counter Evidence Risk、Freshness 的排序边界。
- `reporting`：Top5、Rising、Watch、Dropped 和研究资料包的只读输出边界。

### Shared Boundaries

`shared/domain` 只放稳定的跨 Context 值对象/标识符约束；`shared/application` 只放 ports/use-case 约定；`shared/infrastructure` 和 `shared/interface` 不可反向成为 Domain 依赖。Phase 0 只建立目录和模块入口，不预先发明跨领域模型。

## Architecture Test Strategy

测试位于 `tests/architecture/`，使用标准库读取仓库源文件。它们不加载 provider、数据库或 LLM，也不通过命名约定掩盖违反依赖的内容。

每项测试明确对应一个项目宪法约束：

- `domain_does_not_depend_on_infrastructure`：所有 `src/**/domain/**/*.rs` 不得引用 infrastructure、interface、reqwest、sqlx 或 LLM SDK。
- `transformation_does_not_depend_on_llm`：transformation 源文件不得引用 LLM SDK、provider 和 infrastructure。
- `ranking_does_not_depend_on_external_provider`：ranking 源文件不得引用 provider、HTTP、数据库或 renderer。
- `production_system_does_not_depend_on_renderer`：production_system 源文件不得引用 reporting、renderer 或 interface。
- `evidence_domain_is_provider_agnostic`：evidence domain 不得引用 SEC、news、fundamental provider 或 provider JSON 类型。

另外验证 Context 的层目录存在，并验证 `src/lib.rs` 只导出 Context 模块而不从 `main`、infrastructure 或外部 provider 反向引入实现。

## Documentation Structure

根文档提供第一屏原则；产品文档固定 Mission Boundary 和 MVP Universe；架构文档固定层次、Context 和依赖；领域文档固定生产系统、六阶段、Evidence 与 Ranking 模型；数据文档固定来源层级和质量维度；评分文档固定 Stage Gate 与非 Stage 替代的 Score；验证文档固定 T0 后 6/12/24 个月的事实验证路径。

## Acceptance Criteria

1. 从仓库根目录执行 `cargo fmt --check` 成功。
2. 从仓库根目录执行 `cargo clippy --all-targets --all-features -- -D warnings` 成功。
3. 从仓库根目录执行 `cargo test --all` 成功，包含全部 Architecture Tests。
4. 10 个 Architecture Tests 均能在违规令牌出现时失败，并在当前骨架中通过。
5. 所有要求的目录、Context 层入口、根文档、Phase 0 文档和 ADR-001 至 ADR-010 均存在且无占位标记、空泛事项或自相矛盾的实现要求。
6. 文档明确说明 ORG-X 与 Sentinel 的边界不共享 Decision、Score、Gate 或业务 Domain。
7. 文档与代码均不引入 reqwest、sqlx、LLM SDK、SEC implementation、news provider、renderer 或交易决策。
8. Phase 0 完成后，下一步明确为 AI Cockpit 安装前环境确认，而不是任何业务 Work Item。

## Risks and Controls

- 风险：空模块被误解为已实现业务。控制：所有入口仅含边界说明，README 和 Scope 明确“无业务行为”。
- 风险：文档叙事先于证据。控制：将 Evidence before Score、Counter-evidence mandatory、AI extracts; Rust decides 写入根原则和 ADR。
- 风险：未来 provider 模型泄漏到 Domain。控制：ACL 目录、provider-agnostic Architecture Tests 和依赖规则文档同时落地。
- 风险：Foundation 变成无止境的大设计。控制：本阶段只建立可编译、可测试的边界，业务模型进入后续独立 Work Item。

## Transition

完成本文档自检并提交后，写出实施计划。实施完成并通过所有本地质量检查后，才执行 AI Cockpit 安装；安装后的第一项仅为 WI-001 Engineering Foundation Verification。
