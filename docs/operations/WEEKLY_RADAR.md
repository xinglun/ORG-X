# Weekly Radar 使用说明

Weekly Radar 是确定性的 evidence-first 周报：获取 SEC 和明确配置的公司来源，只保留来源中实际提供的事实，先把完整的运行输入持久化为不可变快照，再生成默认中文、也可切换日语或英语的面向人的报告，发送到 Telegram，并把可追溯的 report、snapshot、receipt 和 manifest 写入受保护的 `data` 分支。

## 运行流程

```text
获取来源 → 规则抽取 → 保留证据 → 持久化输入快照
                                      ↓
                         生成人类周报 → Telegram → data branch archive → retention
```

发布只有在报告通过一手证据检查、输入快照已经持久化、Telegram receipt 与 report ID 绑定后才会写入 archive。输入快照位于 `weekly-radar/snapshots/YYYY-MM-DD.input.json`，最终渲染 snapshot 仍位于 `weekly-radar/snapshots/YYYY-MM-DD.json`。

## 准备工作

- 本地运行需要 Rust stable、Cargo、Git，以及仓库中的 `config/weekly_radar/companies.json`。
- dry-run 需要 `ORGX_SEC_USER_AGENT`；真实发布还需要 Telegram 的两个环境变量。
- GitHub Actions 会安装 Rust stable，并从 Actions Secrets 读取同名变量。

## 调度与命令

生产调度是每周一 09:00 JST（UTC `0 0 * * 1`），也支持 `workflow_dispatch` 手动执行。手动执行可选择语言（`zh-CN`、`ja`、`en`）、日期和 `dry_run`。Actions 使用 `actions/checkout@v5`，从 `data` 分支重建已有的 `weekly-radar/` 树，并运行与本地相同的 CLI。

本地发布入口：

```sh
cargo run --release -- weekly-radar \
  --as-of "$(date -u +%F)" \
  --archive-dir . \
  --registry config/weekly_radar/companies.json \
  --language zh-CN
```

如果发送过程中进程退出或 Telegram 返回失败，保留已经写入的输入快照，并使用它做 delivery-only retry：

```sh
cargo run --release -- weekly-radar \
  --archive-dir . \
  --retry-as-of 2026-08-17
```

重试从 `weekly-radar/snapshots/YYYY-MM-DD.input.json` 读取原始 `RuntimeReportInput` 和保存的语言，不读取 registry、不重新获取来源，也不需要 `ORGX_SEC_USER_AGENT`。`--retry-as-of` 不能和 `--as-of`、`--language` 或 `--dry-run` 一起使用；若该日期已经有最终 report、snapshot 或 receipt，命令会在发送 Telegram 前拒绝，避免重复归档和重复发送。

Actions 只把结果提交到字面值为 `data` 的 orphan 分支，并使用 lease 保护并发更新。`main` 和其他分支不是 archive 或 retention 的目标。

## 环境变量

- `ORGX_SEC_USER_AGENT`：获取 SEC 数据前必填。写明应用名和维护者联系地址；不要把它当作 Secret，也不要打印实际值。
- `ORGX_TELEGRAM_BOT_TOKEN`：Telegram Bot token，只由 Telegram transport 使用。
- `ORGX_TELEGRAM_CHAT_ID`：目标 chat ID，只由 Telegram transport 使用。

实际值只能放在本机运行环境或 GitHub Actions Secrets 中，不能提交到仓库。

配置 Telegram 时，先在 BotFather 创建 Bot 并保存 token，再确认目标 chat ID。最后把以下两个名称配置到 GitHub Actions Secrets 或本机运行环境：

```text
ORGX_TELEGRAM_BOT_TOKEN
ORGX_TELEGRAM_CHAT_ID
```

不要把 token、chat ID 或包含它们的 URL 写入仓库、命令历史、日志或报告。

## 本地 dry-run

`--dry-run` 执行正常的来源获取和报告验证，但不发送 Telegram，不持久化输入快照，也不创建、删除或修改 archive 文件。默认输出中文；需要检查其他语言时使用 `--language ja` 或 `--language en`：

```sh
ORGX_SEC_USER_AGENT='ORG-X local dry-run contact@example.test' \
cargo run -- weekly-radar \
  --as-of "$(date -u +%F)" \
  --archive-dir /tmp/org-x-weekly-radar \
  --registry config/weekly_radar/companies.json \
  --language zh-CN \
  --dry-run
```

离线测试可以注入 fixture HTTP client；没有来源配置的 registry 会跳过 discovery acquisition，因此不会发起网络请求。真实发布要求一手证据可达、Telegram 凭据存在，并取得成功的 bound delivery receipt。

## 来源和事实状态

SEC EDGAR、SEC XBRL 和明确配置的官方公司页面优先；Greenhouse 和 Lever 是有边界的招聘来源；GDELT 只用于发现，不能直接成为权威证据。

规则抽取保留来源、字段或原文片段和日期。缺少、歧义、冲突、日期不明、对象不相关或格式错误不会被猜测；无法取得的资料会在周报中按来源归纳，完整明细保留在 snapshot。运行时不使用付费 API、LLM 抽取、未经提供的 Stage/Rank/Score 推断或投资结论。

## Telegram 报告

报告按“本周摘要 → 重要组织变化 → 重点公司（有明确选择时）→ 系统状态”组织，再在安全的语义边界拆分消息。报告正文不显示 `source_*`、内部状态枚举、覆盖率分数或逐项采集诊断；读者只看到确认过的结论、待核实线索数量、来源是否可用及需要关注的聚合问题。完整事实、来源、状态和逐项 review 明细仍在 snapshot。Publisher 有限重试、保留消息顺序和 message IDs，并在失败时记录已接受的部分 ID。

## data 分支保留

每次成功运行写入 report、sanitized snapshot、绑定的 `PUBLISHED` receipt 和 manifest；manifest 会记录输入快照路径及其稳定 `snapshot_id`。同一日期的最终文件是不可覆盖的；冲突或写入失败不会先执行 retention。retention 只删除日期前缀文件中超过 365 天的 input snapshot、report、snapshot 和 receipt；最近文件保持不动。dry-run 不执行 retention。
