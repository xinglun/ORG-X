# Weekly Radar 使用说明

Weekly Radar 是确定性的 evidence-first 周报：获取 SEC 和明确配置的公司来源，从入口页发现有限的实际文档，再把具体主张通过 EvidenceCandidate → ValidatedEvidence 门槛，并进一步通过组织重写、生产系统重写、持续结果、行业扩散四族证据门。通过门槛的内容还会继续区分为“已验证事实”和“结构性证据”；入口页可访问性本身不会被写成企业变化。SEC 指标、来源可用性和其他 `Known` facts 仍会保留在输入快照、系统状态和判断上下文中，但不会因为状态是 `Known` 就自动变成企业变化证据。生成默认中文、也可切换日语或英语的面向人的报告，发送到 Telegram；只有发送成功后，才把输入快照以及可追溯的 report、snapshot、receipt 和 manifest 一起写入受保护的 `data` 分支。

## 运行流程

```text
来源入口 → 有限文档发现 → 主张抽取/日期验证 → ValidatedEvidence → 四族范本证据门 → 系统自动给出参考判断 → 准备输入快照
                              ↓
          报告并列呈现系统参考与人的独立判断 → Telegram → 成功后提交 archive → data branch → retention
```

发布只有在报告通过一手证据检查、Telegram receipt 与 report ID 绑定后才会把输入快照、report、snapshot、receipt 和 manifest 作为一个可恢复的逻辑 transaction 写入 archive。输入快照位于 `weekly-radar/snapshots/YYYY-MM-DD.input.json`，最终渲染 snapshot 仍位于 `weekly-radar/snapshots/YYYY-MM-DD.json`。

### 已有输入快照的兼容性

以前保存的输入快照仍可用于重试和“已发布”只读验证。旧快照中没有记录“该来源不适用”计数或 `research_metrics` 时，系统分别按 `0` 处理；新快照会保留来源获取、文档类型和证据验证计数。这样不会改变旧报告的身份，也不会要求重新获取来源。

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

来源状态会分别说明：资料可用、已配置但暂时不可用、尚未配置、不适用、资料可读但没有可确认内容，以及仅用于发现线索。尚未配置不等于请求失败；不适用也不计入暂不可用；新闻发现即使可用，仍只表示有线索，不能单独证明事实。SEC submissions、Company Facts 和可选 filing document 是独立阶段：一阶段失败不会丢弃另一阶段成功结果，阶段名与安全原因会进入 source failure；申报候选来自有限的最近 `10-K`、`10-Q`、`8-K` 元数据，URL 只能由校验后的 accession 和 primary document 拼接。

SEC 申报候选在保留元数据后，还会逐个读取最多 3 个最近的校验后 primary document；每个 filing body 使用独立的有限上限（8 MiB），并保留标题、正文、accession、form、filing date、report date 和 archive URI。`KNOWN` 表示正文可用于现有 claim gate，`UNKNOWN` 表示响应成功但没有可用正文，`UNAVAILABLE` 表示请求失败或超过上限。文档失败不会抹掉 Company Facts 或其他 filing；employee fallback 会复用已经读取的最新 10-K，只有最新 10-K 不在这组有限候选中时才单独读取它。

官方 IR 入口先发现最多 8 个直接同源文档，再从成功读取的直接文档执行一次额外同源发现；两层合计最多保留 12 个文档。URL 会去掉 fragment、去重并拒绝跨 origin 链接，目录页、archive/index 页和入口页仍只是线索，不能因为被抓到或可访问就成为企业变化证据。超过深度、重复或总量上限的链接不会继续请求。

对 `official_research_sources` 明确配置的内容 URL，系统只在 URL 路径表现为文章、博客、新闻、press 或 customer story 时把该入口同时作为一个有界文档读取；普通首页和分类页仍只保留为 `EntryPoint`。`independent_research_sources` 是另一组显式、可跨 origin 的采用者自有披露或 IR 文档 URL；它们使用独立的 authoritative tier，仍遵守相同的有限正文和文档发现边界。系统不会猜测未配置 URL，也不会把供应商域名上的另一个页面自动视为独立来源。这允许研究配置绑定实际发布内容，同时不把“页面可访问”升级为证据。内容路径优先于通用导航链接，避免有限 discovery budget 被菜单耗尽。

站点如果使用受限的发布元数据（例如 PwC 的 `pwcReleaseDate`），适配器只允许显式登记的发布日期字段，并优先于修改日期；未知或格式错误的字段不会被猜测。当前 Microsoft 独立扩散配置绑定 PwC 自有案例和 Atos Group 自有 press disclosure；Microsoft 域名上的客户案例仍然只属于供应商归因。

研究状态必须按以下层次解释：

- `SourceObservation`：入口页或接口是否可访问；入口页的可访问性不是企业变化证据。
- `DocumentCandidate`：从同源入口发现的有限文档，保留 URL、标题、`filing`/`earnings`/`engineering`/`organization` 等文档类型、日期和发现 provenance；IR 只增加一次嵌套发现，SEC filing 只使用有限 submissions 元数据构造 archive URI；不会猜测 URL，也不会无限爬取。入口页仍是 `SourceMaterialKind::EntryPoint`，不会继承文档类型或被晋升为证据。
- `EvidenceCandidate`：必须有公司 ID、公司名称、具体变化/事实、有效日期、生产环节、来源 URI、来源标题、权威级别、正文段落和 cutoff 关系；任一字段缺失都保持待验证，不进入 confirmed 或 StructuralEvidence。
- `ValidatedEvidence`：通过确定性 gate 的主张，才会成为 `evidence_*` normalized fact；它随后按主张段落中的结构性信号分为普通已验证事实或 StructuralEvidence。GDELT、新闻、招聘记录和页面级 observation 不会绕过该 gate。
- `StructuralDimension`：StructuralEvidence 的维度只描述主张涉及的变化域，不改变其 evidence kind 前缀、Stage、Ranking、Counter Evidence、Telegram 或 archive 行为。固定优先级为 `OperatingMetric`（利用率、延迟、吞吐、产能、成本、利润/现金流、GPU 等）→ `ProductionSystem`（部署、发布、平台、基础设施、存储、云、自动化、agent 等）→ `Workflow`（工作流、流程、审批、排程、交接等）→ `Organization`（组织、职责、汇报关系、团队、部门、人员规模等）。
- `NormalizedFact.structural_dimension`：新事实会保存可选的 `organization`、`workflow`、`production_system` 或 `operating_metric`；旧快照缺少该字段时按 `None` 读取，旧的 `evidence_structural_change_<index>` kind 保持可读，并在报告中使用通用“结构性证据”标签。
- `NormalizedFact.reference_model_family`：只有文档级 Claim 经过 ValidatedEvidence，或 SEC 适配器保留了明确的历史经营周期，才会携带 `organization_rewrite`、`production_system_rewrite`、`sustained_outcome` 或 `industry_diffusion`。同时，范本 Claim 可携带 `SupplierAttribution`、`IndependentCustomerDisclosure`、`RegulatoryOrFiling` 或 `DiscoveryOnly` 来源角色。SEC 结果周期最多保留有限的不同日期，不复制同一事实 identity；旧快照缺少这些字段时按空值读取并保持非独立。
- `ValidatedFact`：已验证但未满足结构性变化信号的事实，使用 `evidence_official_material_<index>`，进入报告的“已验证事实”。
- `StructuralEvidence`：已验证且明确涉及组织、责任、生产系统、工作流、部署、利用率、延迟、产能、成本、人员或经营指标变化的事实，使用 `evidence_structural_change_<index>`，进入报告单独的“结构性证据”；报告会按维度显示中/日/英标签，但它不会绕过既有 Stage 或 Ranking gate。
- Careers 文档默认仍是招聘线索：通用雇主介绍、能力描述或“AI/数据/云/基础设施”宣传文案不能生成 `EvidenceCandidate`。只有正文明确说明招聘、招募、headcount、员工规模或新增岗位等变化时，才允许形成普通 `ValidatedFact`；Careers 来源不会因为这些生产词汇被分类为 `StructuralEvidence`。

报告渲染还会把事实状态和证据语义分开：只有 `status=Known` 且 `kind` 以 `evidence_` 开头的 fact 才能进入证据明细，并按上述两类分别出现在“已验证事实”或“结构性证据”；SEC 的 `revenue`、`headcount`、`capex` 等原始指标，以及 `source_*`、`pending_evidence_*` 和其他非证据 fact，不会进入这两节。系统状态中的“已知事实”只是输入中所有 `Known` facts 的可观测计数，不等于已确认企业变化的数量。

文档晋升前会先按以下确定性顺序恢复日期：`article:published_time`、`meta name=date`、JSON-LD `datePublished`、`<time datetime>`，最后才使用 JSON-LD `dateModified`；只取可解析的 ISO 日期前缀，畸形或无法解析的值不会被猜测。随后系统会把 `<title>`、`meta`、`script`、`style`、`noscript`、标题标签以及 `nav`、`header`、`footer`、`aside`、`form` 等非正文块从候选正文中移除；带有 `share`、`social`、`menu`、`breadcrumb`、`sidebar`、`navigation` 等标记的常见容器也会被排除。若页面包含段落，抽取只使用清洗后的 `<p>` 内容，避免把“Skip to content”、分享链接、页脚或菜单拼进主张。正文必须提供一个有终止标点的完整句子（至少 8 个词），同时命中明确的生产系统变化动作和生产环节信号，且保留文档类型上下文，才会生成 `EvidenceCandidate`；Careers 文档还必须命中明确招聘变化信号。该类型会写入最终事实 provenance。仅描述既有架构、接口、系统组成或雇主能力而没有变化动作的句子不会晋升。因此，文章标题、目录标题、页面 JSON、整页拼接文本、只有关键词的句子、通用 Careers 文案和没有有效日期的文档，都会停留在 `DocumentCandidate`/待验证线索层，不会进入“已确认信息”。

每份报告的 `research_metrics` 与首页摘要分别展示：`本周新增已验证事实`、`本周新增结构性证据`、`发现文档候选`、按 `engineering`、`earnings` 等稳定类型排序的文档类型计数、`来源可用性确认`、`待验证线索`、`关键数据源不可用`，以及 SEC 的“阶段可用/期望”和“可用事实/期望”。旧快照没有文档类型计数时按空映射读取，报告不显示该附加行。这些计数互不替代：来源可用不等于企业发生变化，文档候选不等于 Claim，SEC 阶段可达也不等于 SEC normalized facts 可用。当结构性证据为零且仍有待验证线索、不可用来源或不可用事实时，报告使用“数据不足/无法据此确认本周没有组织变化”的 calibrated wording，而不是把它写成没有变化。

规则抽取保留来源、字段或原文片段和日期。系统会在证据达到门槛时自动给出一个可复核的参考判断；缺少、歧义、冲突、日期不明、对象不相关或格式错误不会被猜测，证据不足时显示“系统暂无法判断”。人的判断保持独立，系统只提供参考，报告不会把两者合并成一个答案，方便人自行核验、保留不同意见或继续补证。运行时不使用付费 API、LLM 抽取或投资结论。

## Telegram 报告

报告按“本周摘要 → 已验证事实（如有）→ 结构性证据（如有）→ 系统参考判断 → 结构性变化证据 → 重点公司（有明确选择时）→ 系统状态”组织，再按完整章节拆分消息；旧版“已确认信息”等标题仍作为输入兼容别名接受。

你会在“已验证事实”和“结构性证据”中逐条看到经过 ValidatedEvidence gate 的事实：公司、信息类型、事实内容、事实日期（资料没有日期时不补写）和直接证据链接。报告不会用一个总入口链接代替每家公司或每条事实的依据。SEC 指标等“已知事实”会在快照和系统状态中保留，用于审计和判断上下文，但不应被解读为结构性变化结论。

“系统参考判断”会逐家公司说明系统参考是什么、为什么得到这个状态、哪些资料支持它、哪些反向资料需要注意、还缺少什么证据以及对应链接。系统参考只是给人的一个可复核参考；人的判断仍然独立，可以同意、保留不同意见或继续补证，二者不会合并、投票或协作生成一个答案。

“AI 时代范本验证”会逐家公司显示 `Candidate`、`Confirmed` 或 `NotEligible`，并列出组织重写、生产系统重写、持续结果、行业扩散四类证据矩阵、结果周期数、独立扩散来源数、供应商归因来源数、来源角色摘要、反证复核数量和开放条件。只有四类证据、跨周期结果、独立同行扩散和反证复核全部通过时，才允许最高 `REFERENCE_MODEL` Stage；`Candidate` 不是行业范本，也不会进入该 Stage 的 Ranking。

范本验证的来源语义必须保持分层：来源可用性不是企业变化，供应商客户案例是
`SupplierAttribution`，客户自有披露是 `IndependentCustomerDisclosure`，SEC/IR
申报和经营结果是 `RegulatoryOrFiling`，新闻只属于 `DiscoveryOnly` 线索。供应商
材料可以保留技术归因，但不能单独满足 `IndustryDiffusion`；独立扩散至少需要两个
authoritative 的客户/采用者来源 URI 和两个命名采用者。报告分别显示独立扩散来源数、
供应商归因来源数、来源角色摘要和尚缺条件。

“没有确认到组织变化”“证据不足”和“来源暂不可用”含义不同：前者表示当前可用资料中没有确认事实，后两者表示不能据此判断没有变化。没有明确选择重点公司时，不显示排名，也不生成交易或资本行动结论。报告正文不显示 `source_*`、内部状态枚举、覆盖率分数或逐项采集诊断；完整事实、来源、状态、`research_metrics`、系统参考依据和逐项 review 明细仍在 snapshot。Publisher 有限重试、保留消息顺序和 message IDs，并在失败时记录已接受的部分 ID。

## data 分支保留

每次成功运行写入 report、sanitized snapshot、绑定的 `PUBLISHED` receipt 和 manifest；manifest 会记录输入快照路径及其稳定 `snapshot_id`。同一日期允许由普通 schedule 或手动运行更新，但只有完整成功 transaction 才会替换旧正本；冲突或写入失败不会先执行 retention。transaction 只有在全部公共 artifact（包括有输入快照时的输入文件）完成后才变为 committed，retention 也只在该提交点之后执行。retention 只删除日期前缀文件中超过 365 天的 input snapshot、report、snapshot 和 receipt；最近文件保持不动。dry-run 不执行 recovery、归档或 retention。
