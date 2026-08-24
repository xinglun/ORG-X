# Weekly Radar 使用说明

Weekly Radar 是确定性的 evidence-first 周报：获取 SEC 和明确配置的公司来源，只保留来源中实际提供的事实，在内存中准备完整运行输入，生成默认中文、也可切换日语或英语的面向人的报告，发送到 Telegram；只有发送成功后，才把输入快照以及可追溯的 report、snapshot、receipt 和 manifest 一起写入受保护的 `data` 分支。

## 运行流程

```text
获取来源 → 保留证据 → 系统自动给出参考判断 → 准备输入快照
                              ↓
          报告并列呈现系统参考与人的独立判断 → Telegram → 成功后提交 archive → data branch → retention
```

发布只有在报告通过一手证据检查、Telegram receipt 与 report ID 绑定后才会把输入快照、report、snapshot、receipt 和 manifest 作为一个可恢复的逻辑 transaction 写入 archive。输入快照位于 `weekly-radar/snapshots/YYYY-MM-DD.input.json`，最终渲染 snapshot 仍位于 `weekly-radar/snapshots/YYYY-MM-DD.json`。

### 已有输入快照的兼容性

以前保存的输入快照仍可用于重试和“已发布”只读验证。旧快照中没有记录“该来源不适用”计数时，系统按 `0` 处理；新快照在确实存在该状态时会保留计数。这样不会改变旧报告的身份，也不会要求重新获取来源。

输入快照的身份仍然必须与内容一致。只要日期、事实、来源覆盖、判断或其他内容被改动，或者快照格式损坏，系统就会在重试、Telegram 发送和 archive/data 写入之前停止。不要手工编辑历史快照；如果校验失败，应保留原文件并检查对应的 `manifest`、receipt 和 transaction 记录。

## 准备工作

- 本地运行需要 Rust stable、Cargo、Git，以及仓库中的 `config/weekly_radar/companies.json`。
- dry-run 需要 `ORGX_SEC_USER_AGENT`；真实发布还需要 Telegram 的两个环境变量。
- GitHub Actions 会安装 Rust stable，并从 Actions Secrets 读取同名变量。

## 调度与命令

生产调度和本地 `WeeklyScheduler::default` 均为每周一 09:00 JST（UTC `0 0 * * 1`），也支持 `workflow_dispatch` 手动执行。显式配置 `day_of_week` 仍可用于测试或调用方，不改变生产默认值。手动执行可选择语言（`zh-CN`、`ja`、`en`）、日期和 `dry_run`。Actions 使用 `actions/checkout@v5`，从 `data` 分支重建已有的 `weekly-radar/` 树，并运行与本地相同的 CLI。

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

如果只想确认某个日期当前正本是否完整，可做只读验证：

```sh
cargo run --release -- weekly-radar \
  --archive-dir . \
  --verify-published-as-of 2026-08-24
```

输出 `ALREADY-PUBLISHED:` 表示 report、snapshot、Telegram receipt、manifest 和可用的输入快照已互相验证；该命令不会联系 Telegram，也不会新建 `data` 提交。它是明确的只读检查，不会阻止随后一次普通手动或定时发布。

普通发布不区分定时触发和手动触发。同一天已经有正本时，新的普通运行仍会获取资料、生成报告并发送 Telegram；只有这次发送成功且 archive transaction 完成后，新的 report、snapshot、receipt、manifest 和输入快照才会整体替换当天正本。因此，当天最后一次成功更新就是当天正本，紧急情况下可以直接手动触发。

如果已经确认该日期的报告存在，但配置的 Telegram 目标没有看到它，可以明确选择一次“重新发送已生成报告”。这会再次发送同一份已保存输入生成的报告，因此目标中会出现一条有意的重复消息；它不会重新获取资料，不会改变已有 report、snapshot、receipt、manifest，也不会写入 `data` 或 pending ref。

Actions 手动运行时，把 `republish_published` 设为 `true`，并填写已经完成发布的 `as_of` 日期；不要同时选择 `dry_run`。本地等价入口是：

```sh
cargo run --release -- weekly-radar \
  --archive-dir . \
  --republish-published-as-of 2026-08-24
```

只有明确的手动运行可以使用这个选项，定时运行不会启用它。输出中的 report ID、message IDs 和 attempts 是发送服务返回的凭据，只能说明服务接受了这次发送，不能证明你的 Telegram 客户端已经显示、通知或被人阅读。

如果进程在 Telegram 成功后、归档提交完成前退出，归档器会把已序列化的 report、snapshot、receipt、manifest 和输入快照保存在 `weekly-radar/.transactions/` 的 prepared transaction 中。下一次非 dry-run 或同日期 retry 会先校验并完成这个 transaction，复用已有 receipt，不再发送第二次 Telegram。若 transaction 缺失、损坏、或已有公共文件既不是旧正本也不是 staged bytes，运行会以 `IncompleteRun` fail closed；它不会猜测、覆盖或删除冲突文件，需要人工检查归档残留。这里保证的是可审计的逻辑提交点，不是跨多个公共文件的物理原子事务。

如果 Telegram 已经成功、但随后 `data` 分支 push 没有完成，系统会先尝试保留一个带有原始 report、snapshot、receipt 和 manifest 的 pending publication，再更新 `data`。下一次运行会先校验 pending publication 的日期和全部文件身份；校验通过后只把同一组已发布文件推到 `data`，不会重新获取来源、重新生成报告或再次调用 Telegram。若 pending 本身也无法保存、校验失败、日期顺序冲突或文件被改动，系统会停止并要求人工检查，不会猜测、覆盖或重复发送。`data` 已经成功更新但 pending 清理暂时失败时，下一次运行会先比较身份，再安全地清理冗余 pending 状态。

运维人员按以下结果判断：`PUBLISHED:` 表示新报告已发布并成为当天正本；`ALREADY-PUBLISHED:` 只表示显式只读验证成功；`READY-TO-PUSH:` 表示 Telegram 已接受、等待把同一组文件推进 `data`；`RECOVERED:` 表示本地 prepared transaction 已恢复。其他 archive 错误应保持停止并检查证据。不应手工复制 receipt、改写历史周报或直接修复 `data` 分支。若需在一个已取回的 archive 上做无发送验证，可使用：

```sh
cargo run --release -- weekly-radar \
  --archive-dir . \
  --recover-published-as-of 2026-08-17
```

该命令只做身份校验，不访问来源、不读取 Telegram 配置，也不发送消息。

同一日期的非 dry-run CLI 会持有从 recovery、重复日期检查、来源获取、Telegram 发送到 archive commit 的 Unix 文件锁；直接 archive API 另有 commit 锁。锁在进程异常退出时由操作系统释放，避免并发运行在发送前互相通过检查。

Actions 把最终结果提交到字面值为 `data` 的 orphan 分支；Telegram 成功到 `data` push 完成之间会短暂使用 `weekly-radar-pending` publication ref，并使用 lease 保护两个 ref 的并发更新。`main` 和其他分支不是 archive 或 retention 的目标。

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

SEC submissions 和 SEC Company Facts 都是包含完整申报历史的 JSON，使用独立的有限响应上限；普通网页、招聘接口和发现接口仍使用通用的 1 MiB 响应保护。超过各自上限的响应会失败关闭并进入来源健康状态，不会以部分或猜测数据继续发布。

来源状态会分别说明：资料可用、已配置但暂时不可用、尚未配置、不适用、资料可读但没有可确认内容，以及仅用于发现线索。尚未配置不等于请求失败；不适用也不计入暂不可用；新闻发现即使可用，仍只表示有线索，不能单独证明事实。SEC 或其他来源暂不可用时，报告会保留不含凭据、响应正文和敏感请求信息的原因，不能据此推断“没有变化”。

规则抽取保留来源、字段或原文片段和日期。系统会在证据达到门槛时自动给出一个可复核的参考判断；缺少、歧义、冲突、日期不明、对象不相关或格式错误不会被猜测，证据不足时显示“系统暂无法判断”。人的判断保持独立，系统只提供参考，报告不会把两者合并成一个答案，方便人自行核验、保留不同意见或继续补证。运行时不使用付费 API、LLM 抽取或投资结论。

## Telegram 报告

报告按“本周摘要 → 已确认信息 → 系统参考判断 → 重要组织变化 → 重点公司（有明确选择时）→ 系统状态”组织，再按完整章节拆分消息。

你会在“已确认信息”中逐条看到：公司、信息类型、事实内容、事实日期（资料没有日期时不补写）和直接证据链接。报告不会用一个总入口链接代替每家公司或每条事实的依据。

“系统参考判断”会逐家公司说明系统参考是什么、为什么得到这个状态、哪些资料支持它、哪些反向资料需要注意、还缺少什么证据以及对应链接。系统参考只是给人的一个可复核参考；人的判断仍然独立，可以同意、保留不同意见或继续补证，二者不会合并、投票或协作生成一个答案。

“没有确认到组织变化”“证据不足”和“来源暂不可用”含义不同：前者表示当前可用资料中没有确认事实，后两者表示不能据此判断没有变化。没有明确选择重点公司时，不显示排名，也不生成交易或资本行动结论。报告正文不显示 `source_*`、内部状态枚举、覆盖率分数或逐项采集诊断；完整事实、来源、状态、系统参考依据和逐项 review 明细仍在 snapshot。Publisher 有限重试、保留消息顺序和 message IDs，并在失败时记录已接受的部分 ID。

## data 分支保留

每次成功运行写入 report、sanitized snapshot、绑定的 `PUBLISHED` receipt 和 manifest；manifest 会记录输入快照路径及其稳定 `snapshot_id`。同一日期允许由普通 schedule 或手动运行更新，但只有完整成功 transaction 才会替换旧正本；冲突或写入失败不会先执行 retention。transaction 只有在全部公共 artifact（包括有输入快照时的输入文件）完成后才变为 committed，retention 也只在该提交点之后执行。retention 只删除日期前缀文件中超过 365 天的 input snapshot、report、snapshot 和 receipt；最近文件保持不动。dry-run 不执行 recovery、归档或 retention。
