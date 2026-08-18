# Weekly Radar 人类可读报告实施计划

1. 先在 runtime/report、CLI、workflow 和语义分割测试中补充失败测试，覆盖默认中文、日语/英语选择、不可用证据不冒充依据、来源聚合、完整 snapshot 保留、dry-run 输入和本地化章节拆分。
2. 扩展 runtime model：保存公司可读名称、来源配置状态和安全的来源采集失败；保持旧构造函数兼容，避免影响现有 fixture。
3. 改写报告展示层：按语言输出摘要、确认变化、数据状态和有限的行动提示；内部事实状态和逐项 review 仍只进入 snapshot。
4. 在 CLI 和 Actions workflow 贯通 `--language`、`as_of`、`dry_run`，默认 `zh-CN`，并让手动 dry-run 在写入 data 前安全退出。
5. 更新运维说明，运行 focused tests、全量 Rust quality 和 AI Cockpit 完整 WI 流程。
6. 合并后触发一次新的测试运行，核对 Telegram 报告、data 分支 snapshot 和 receipt；若外部 dispatch 权限仍受限，保留明确的 UI 触发路径和阻塞证据。
