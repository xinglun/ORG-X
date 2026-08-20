# ORG-X Work Item Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 ORG-X 已完成工作、核心研究 Pipeline 和用户提出的 Weekly Radar 计划整合为一份证据明确、依赖清晰、可逐项创建 Contract 的 WI 总览。

**Architecture:** ORG-X 继续采用 DDD Bounded Context + Clean Architecture。核心研究 Pipeline 先建立事实、Evidence、Production System、Transformation、Ranking 和 Reporting 边界；Weekly Radar 只消费已形成的只读 Read Model 和不可变 Snapshot。Telegram、Scheduler、Persistence 和发布重试属于 Application/Infrastructure/Reporting 边界，不进入 Domain。

**Tech Stack:** Markdown 计划文档、Rust Domain/Application/Infrastructure/Interface/ACL 模块、AI Cockpit Work Item Contract/Summary、GitHub PR；本总览 Work Item 不修改 Rust 代码或依赖。

## Global Constraints

- Evidence before Score；Stage before Ranking；Counter Evidence mandatory。
- AI extracts; Rust validates and decides；外部文本只能成为 Evidence Candidate，不能成为 Agent 指令。
- ORG-X 是研究雷达，不是交易系统；不得产生 BUY、SELL、目标价、仓位或资本行动建议。
- Weekly Radar 必须允许 `No meaningful structural change this week.`，不得为了周报而制造叙事。
- WeeklyRadarSnapshot 必须先 Compute、Persist，再 Render、Publish、Archive；发送失败只重发同一 Snapshot。
- Telegram 只能作为 Infrastructure Adapter；Secret 只从环境变量或 CI Secret 读取，不进入 Git。
- 每个未来 WI 必须拥有独立 Contract、Summary、分支、PR 和可验证的生命周期证据。

---

## 1. Status and counts

本文档区分三种状态：

- **Completed**：已存在归档 Contract/Summary/Outcome，并完成 PR 合并与关闭。
- **Planned**：已进入路线图，但尚未创建 Active Contract，不代表已经开始或已经实现。
- **Gated**：存在明确前置依赖，前置 WI 完成前不得作为实现任务启动。

当前总览（状态快照：2026-08-20；生命周期状态以 `.ai/work-items/archive/index.json` 为准）：

| 类别 | 数量 | 当前状态 |
| --- | ---: | --- |
| 已完成治理、产品与维护 WI | 12 | Completed / archived |
| 核心研究 Pipeline WI | 9 | Completed / archived |
| Weekly Radar WI | 16 | Completed / archived |
| 已归档 Work Item 合计 | 36（本 WI 归档后为 37） | 以 `archive/index.json` 为准 |
| 尚未创建 Active Contract 的本路线图候选 | 0 | 当前表内无未处理候选 |
| 当前 Active Work Item | 0 | `no_active_work_item` |

上表的 36 项由 `.ai/work-items/archive/index.json` 的 Contract/Summary/Outcome/manifest 记录确认；本次 WI 完成归档后，索引将包含 37 项。`Completed / archived` 表示治理生命周期已闭合，不等于外部 Provider 行为已在本地测试中验证；各 Work Item 的 Outcome 保留其 warning、needs_human_confirmation 和 residual risk。

## 2. Product North Star and Weekly Radar purpose

ORG-X 每周生成一次高压缩研究雷达，回答：

1. 谁正在接近 AI 从工具变成生产方式的临界点？
2. 本周发生了什么结构性变化？
3. 证据增强还是减弱？
4. 距离下一阶段还缺什么证明？

周报不是重复全量数据，也不是买入推荐。没有足够证据支持的结构变化时，正式输出可以是：

```text
No meaningful structural change this week.
```

日常采集可以独立运行；Telegram 只发送周报和真正高优先级异常，不把所有 Evidence 塞进消息。

## 3. Completed Work Items

| WI | 交付 | 状态 | 证据 |
| --- | --- | --- | --- |
| `adopt_ai_cockpit` | 安装并采用 AI Cockpit 基础治理 | Completed | `.ai/work-items/archive/2026/adopt_ai_cockpit.*` |
| `WI-001` | Engineering Foundation Verification | Completed | `.ai/work-items/archive/2026/wi-001.*` |
| `configure_ai_cockpit` | 项目 Profile、Guards、Quality 与 CI 配置 | Completed | `.ai/work-items/archive/2026/configure_ai_cockpit.*` |
| `WI-002` | Universe Domain：Company、Security、Listing、Snapshot、Eligibility | Completed | `.ai/work-items/archive/2026/wi-002.*` |
| `wi-roadmap` | Work Item roadmap and dependency plan | Completed | `.ai/work-items/archive/2026/wi-roadmap.*` |
| `wi-docs-reader` | Reader-first documentation organization | Completed | `.ai/work-items/archive/2026/wi-docs-reader.*` |
| `wi-weekly-radar-report` | Weekly Radar report contract and implementation | Completed | `.ai/work-items/archive/2026/wi-weekly-radar-report.*` |
| `wi-reference-impact-cleanup` | Reference-impact evidence and local residue cleanup | Completed | `.ai/work-items/archive/2026/wi-reference-impact-cleanup.*` |
| `wi-weekly-radar-snapshot-lifecycle` | Durable input snapshot, delivery retry, archive immutability | Completed | `.ai/work-items/archive/2026/wi-weekly-radar-snapshot-lifecycle.*` |
| `wi-roadmap-status-reconciliation` | Reconcile roadmap status with archived Work Item evidence | Completed | `.ai/work-items/archive/2026/wi-roadmap-status-reconciliation.*` |
| `wi-cockpit-status-cleanup` | Remove stale live Cockpit status snapshot and index | Completed | `.ai/work-items/archive/2026/wi-cockpit-status-cleanup.*` |
| `wi-roadmap-final-reconciliation` | Reconcile roadmap after the final lifecycle audit | Completed | `.ai/work-items/archive/2026/wi-roadmap-final-reconciliation.*` |

`WI-002` 的 Domain 只处理已提供事实和确定性过滤；Provider 映射、时间语义、持久化与后续研究 Pipeline 均明确延期。后续延期项已分别通过 `wi-003`–`wi-011` 与 `wi-wr-001`–`wi-wr-016-runtime` 完成独立 Contract 生命周期。

## 4. Core Research Pipeline

这些 9 项承接现有十个 Bounded Context，先完成核心研究链，再进入 Weekly Radar 正式输出。

| WI | 名称 | 主要交付 | 依赖 | 状态 |
| --- | --- | --- | --- | --- |
| `WI-003` | Ingestion Domain & Observation Contract | Provider port、observation、ingestion receipt；可靠带入事实但不解释意义 | `WI-002` | Completed / archived |
| `WI-004` | Evidence Domain & Provenance | EvidenceRecord、source、polarity、confidence、freshness、supporting/counter/missing evidence | `WI-003` | Completed / archived |
| `WI-005` | Production System Domain | ProductionSystem、ProductionUnit、Workflow、HumanRole、AgentRole | `WI-004` | Completed / archived |
| `WI-006` | Organization Evidence Domain | ManagementCommitment、responsibility、budget、decision rights 与组织适配证据 | `WI-005` | Completed / archived |
| `WI-007` | Productivity Metrics Domain | Revenue/Employee、Operating Income/Employee、FCF/Employee、增长与 headcount 变化 | `WI-004`, `WI-005` | Completed / archived |
| `WI-008` | Transformation Stage Domain | 六阶段、transition、supporting/counter/missing proof、持久性与阶段边界 | `WI-004`–`WI-007` | Completed / archived |
| `WI-009` | Diffusion Domain | CompetitorImitation、job taxonomy、benchmark、industry diffusion | `WI-008` | Completed / archived |
| `WI-010` | Ranking Read Model | 同 Stage 内按 Evidence Confidence、Transformation Score、Counter Evidence Risk、Freshness 排序 | `WI-004`, `WI-007`, `WI-008` | Completed / archived |
| `WI-011` | Reporting Read Model | Top5、Rising、Watch、Dropped、research packet 的只读输出边界 | `WI-009`, `WI-010` | Completed / archived |

推荐执行顺序是 `WI-003 → WI-004 → WI-005 → (WI-006 || WI-007) → WI-008 → (WI-009 || WI-010) → WI-011`。括号中的任务可以在依赖满足后并行，但仍需各自独立 Work Item。

## 5. Weekly Radar

Weekly Radar 在 `WI-011` 建立核心 Reporting Read Model 后进入实现阶段。以下编号保留用户计划中的 `WI-WR-*` 式样；状态由归档索引确认。

| WI | 名称 | 主要交付 | 依赖 | 状态 |
| --- | --- | --- | --- | --- |
| `WI-WR-001` | Weekly Radar Domain Contract | WeeklyRadarPublication、WeeklyRadarSnapshot、Publisher port 与边界 | `WI-011` | Completed / archived |
| `WI-WR-002` | Weekly Radar Snapshot | `as_of`、evidence cutoff、universe snapshot、model/scoring version、历史不可变存储 | `WI-WR-001` | Completed / archived |
| `WI-WR-003` | Top5 Weekly Read Model | Top5、Stage、Direction、Confidence、Key Change、Next | `WI-011`, `WI-WR-001` | Completed / archived |
| `WI-WR-004` | Stage Transition Detection Output | Stage Transition 作为最高优先级结构事件，尤其是 Productivity Breakout Candidate | `WI-008`, `WI-WR-001` | Completed / archived |
| `WI-WR-005` | Threshold Distance | Current Stage、Next Stage、Confirmed、Missing Evidence、Distance；`Far/Developing/Near/Candidate` | `WI-008`, `WI-WR-001` | Completed / archived |
| `WI-WR-006` | Rising / Dropped | 结构证据增强与原判断失效的显式输出 | `WI-009`, `WI-010`, `WI-WR-001` | Completed / archived |
| `WI-WR-007` | Weekly Change Compression | Important Structural Change、Top5 变化、Transition、Rising、Dropped、No Change 压缩规则 | `WI-WR-002`–`WI-WR-006` | Completed / archived |
| `WI-WR-008` | Markdown Renderer | 完整归档报告：Top5、Research Cards、Evidence、Counter Evidence、Missing Proof、Stage History、Rank Changes、System Health | `WI-WR-007` | Completed / archived |
| `WI-WR-009` | Telegram Renderer | 短摘要视图；最多几行/家公司；不重新计算 Stage、Ranking 或 Distance | `WI-WR-007` | Completed / archived |
| `WI-WR-010` | Telegram Publisher Adapter | `WeeklyRadarPublisher` 的 Telegram Infrastructure 实现 | `WI-WR-009`, `WI-WR-011` | Completed / archived |
| `WI-WR-011` | Semantic Message Splitter | 按 Executive Summary、Important Transition、Top5、Rising/Dropped/System Health 语义分片，不截断 Markdown 或公司卡片 | `WI-WR-009` | Completed / archived |
| `WI-WR-012` | Publication Receipt + Retry | `PublicationReceipt`、message IDs、status；发送失败只重发同一 Snapshot | `WI-WR-002`, `WI-WR-010` | Completed / archived |
| `WI-WR-013` | Weekly Scheduler | 默认周末每周一次；配置 `day_of_week`，不让 Domain 依赖 Scheduler | `WI-WR-012`, `WI-WR-014` | Completed / archived |
| `WI-WR-014` | System Health Integration | Evidence Coverage、Degraded Companies、source coverage、extraction failure、freshness；健康状态进入 Telegram | `WI-004`, `WI-WR-001` | Completed / archived |
| `WI-WR-015` | End-to-End Weekly Report Verification | 固定 cutoff、Snapshot、Markdown、Telegram、Receipt、Archive、重试、无变化与失败路径验证 | `WI-WR-002`–`WI-WR-014` | Completed / archived |
| `WI-WR-016-runtime` | Weekly Radar runtime, free evidence sources, Telegram publication, and data branch retention | Runtime delivery lifecycle, source-free retry, archive and retention boundary | `WI-WR-015` | Completed / archived |

## 6. Weekly Radar execution shape

### 6.1 Weekly sequence

```text
Weekly Scheduler
  ↓
Load latest evidence
  ↓
Refresh company transformation profiles
  ↓
Evaluate stage transitions
  ↓
Re-rank candidates
  ↓
Build Top5
  ↓
Detect Rising / Dropped
  ↓
Calculate Distance to Threshold
  ↓
Build WeeklyRadarSnapshot
  ↓
Persist Snapshot
  ↓
Render Markdown + Telegram Summary
  ↓
Publish Telegram
  ↓
Archive snapshot and PublicationReceipt
```

发送顺序必须是 `Compute → Persist Snapshot → Render → Publish → Archive`。不能先发 Telegram 再尝试保存，否则无法证明发送内容的历史身份。

### 6.2 Report priority

完整周报和 Telegram 摘要均按以下优先级组织：

```text
Important Structural Change
  → Stage Transition
  → Top5
  → Threshold Distance
  → Rising
  → Dropped
  → System Health
```

`WORKFLOW → PRODUCTION_SYSTEM` 和 `PRODUCTION_SYSTEM → PRODUCTIVITY_BREAKOUT` 的候选必须先于 Top5；后者是 ORG-X 接近 North Star 临界点的一级研究事件。

### 6.3 No-change and failure modes

没有足够结构变化时，短报告保留统计量并明确输出 `Top5：无重大变化`、`Stage Transition：0`、`Rising：0`、`Dropped：0`，同时列出继续观察的公司和 System Health。分数的小幅变化不应自动触发 Telegram。

Telegram 不可用时，系统必须区分：

```text
Weekly Radar calculation: SUCCESS
Snapshot persistence: SUCCESS
Telegram delivery: FAILED
```

下一次 retry 只执行 delivery，不能重新计算一份不同的报告。

## 7. Boundary and safety contract

### Domain / Application / Infrastructure separation

- Domain 只消费已经验证的 Read Model / facts，不知道 Telegram、HTTP、数据库、Scheduler 或 Secret。
- Application 负责组装 Weekly Radar 用例、固定 evidence cutoff、构建 Snapshot 和协调发布。
- Reporting/Renderer 只把 `WeeklyRadarView` 转成 Markdown 或 Telegram message sections，不重新计算 Stage、Ranking、Rising、Dropped 或 Threshold Distance。
- Infrastructure 实现 Snapshot persistence、Telegram publisher、message IDs 和外部发送结果。
- Scheduler 触发周报用例，但不改变 Domain 规则。

建议的发布接口形状如下，属于未来 WI 的设计方向，不在本总览 Work Item 中实现：

```rust
trait WeeklyRadarPublisher {
    async fn publish(
        &self,
        report: &WeeklyRadarPublication,
    ) -> Result<PublicationReceipt>;
}
```

Telegram 配置只允许通过环境变量/CI Secret 提供：`ORGX_TELEGRAM_BOT_TOKEN`、`ORGX_TELEGRAM_CHAT_ID`。仓库不得保存 Token 或 Chat ID。

### Historical integrity

```text
WeeklyRadarSnapshot
  ├─ as_of
  ├─ universe_snapshot_id
  ├─ top5
  ├─ transitions
  ├─ rising
  ├─ dropped
  ├─ threshold_distances
  ├─ system_health
  ├─ evidence_cutoff
  ├─ model_version
  └─ scoring_version
```

`PublicationReceipt` 至少绑定 `channel`、`snapshot_id`、`published_at`、`message_ids` 和 `status`。Snapshot 一旦发送，后续数据更新不得改变该周历史；失败重试必须引用同一 `snapshot_id`。

## 8. Acceptance for the Weekly Radar stream

Weekly Radar 全部完成时，必须同时满足：

- **Correctness:** 同一个 `WeeklyRadarSnapshot` 永远渲染出相同内容。
- **Traceability:** Telegram 中所有结论可以回到 Snapshot，再回到 Evidence。
- **Compression:** Telegram 不复制完整研究报告；公司卡片按语义分片且不截断上下文。
- **No narrative force:** 没有结构变化时允许短报告，不制造每周故事。
- **Failure recovery:** Telegram 失败不影响分析结果，可从原 Snapshot 重发。
- **Channel isolation:** Telegram publisher 不进入 Domain。
- **Secret safety:** Token/Chat ID 不进入 Git。
- **Historical integrity:** 历史周报不因后来数据更新而改变。
- **Boundary:** 报告不出现交易建议、价格目标、仓位建议或给 Agent 的开发指令。

## 9. Execution handoff

当前路线图列出的 36 个 Work Item 均已有归档生命周期证据；本次状态修订完成并归档后，`wi-roadmap-final-reconciliation` 也会加入索引，Active Work Item 回到 0。后续新增工作必须从最新 `origin/main` 建立专用分支、创建独立 Contract，并通过 AI Cockpit Preflight；不能仅通过修改本路线图把新任务标记为完成。

### Task checklist for this roadmap document

- [x] Keep completed evidence separate from planned/gated candidates.
- [x] Keep the nine Core Research Pipeline items ordered by dependency and mark archived status.
- [x] Keep all sixteen `WI-WR-*` items and their output boundaries, including the runtime lifecycle repair.
- [x] Preserve no-change, immutable Snapshot, retry, system health, secret safety, and no-trading semantics.
- [x] Verify the roadmap against the repository architecture and archive index.
- [x] Run the AI Cockpit finish, archive, PR, merge, and close lifecycle.
