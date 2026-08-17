# ORG-X Reader Documentation Design

## Goal

将 ORG-X 的公开阅读路径从“开发过程记录”整理为“面向读者的产品、方法和使用说明”，同时保留既有工程边界和内部审计记录。

## Audience

- 想快速理解 ORG-X 研究对象、判断方法和输出含义的读者。
- 想运行 Weekly Radar、配置环境并理解数据保留边界的操作者。
- 需要查阅架构、证据、Stage、Score 和验证规则的维护者。

## Design

### Reader entry

`README.md` 只回答“ORG-X 是什么、研究什么、明确不做什么、从哪里继续阅读”。`docs/README.md` 作为完整导航，按照“产品 → 判断模型 → 证据与数据 → 架构 → 验证 → Weekly Radar”组织文档，不展示 WI、进度、下一步或实现历史。

### Stable document boundaries

保留现有产品、架构、数据、领域、评分、验证和运维文件路径；通过重写标题、开头说明、术语和链接，使每个文件只回答一个主要读者问题。不会把所有内容合并成一篇长文，也不会删除 `docs/adr/` 或 `docs/superpowers/` 内部资料。

### Factual alignment

读者文档以当前仓库实现为事实来源：规则抽取、歧义为 `UNKNOWN`、缺少可选来源为 `UNAVAILABLE`、一手证据优先、无付费数据 API、非交易边界、周一 09:00 JST、`actions/checkout@v5`、Telegram 配置和 `data` 分支 365 天保留。

### Language and terminology

正文以中文为主；`Evidence Candidate`、`EvidenceRecord`、`Stage`、`Production System`、`UNKNOWN`、`UNAVAILABLE`、代码路径和环境变量保留稳定英文形式。重复定义以导航链接代替复制。

## Non-goals

- 不改变 Rust 运行时、测试、CI、Secrets 或数据分支行为。
- 不删除或重写既有 ADR、计划、spec 或 Work Item 证据。
- 不把内部工程资料伪装成产品功能说明。

## Acceptance

读者从 `README.md` 出发，可以在两层导航内找到产品目的、范围、证据规则、Stage/Ranking、架构、验证和 Weekly Radar 使用方法；文档中不存在过时的交付状态或下一 WI 指令，且当前运行时事实在相关文档中一致。
