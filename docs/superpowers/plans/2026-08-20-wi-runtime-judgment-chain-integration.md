# 接入 Weekly Radar 自动判断链执行计划

## 目标与范围

在当前 Work Item `wi-runtime-judgment-chain-integration` 中，把 Weekly Radar runtime 接入 Evidence-first automatic Stage Engine。系统产生机器参考；人的参考作为独立可选视图保留，二者不合并、不投票、不互相覆盖。

## 实施顺序

### 1. 建立失败测试

先新增 `tests/weekly_radar_judgment_chain.rs`，覆盖：

1. 固定 source fixture 经过 `NormalizedFact`、Evidence proof binding、automatic Stage Engine、same-Stage Ranking 后进入 snapshot；
2. 两个不同 Stage 的候选不会形成一个跨 Stage 排序；
3. supporting/counter/missing 不足或 provenance/company 不匹配时，输出 `UNDETERMINED` 或 typed error，不产生 machine Ranking；
4. human reference 与 machine judgment 并列保留，修改人参考不会改变 machine judgment；
5. renderer 两次输出稳定，且 snapshot 中的 machine/human 字段与输入一致。

先运行：

```text
cargo test --test weekly_radar_judgment_chain
```

预期第一次为失败，因为 runtime 尚未有 judgment boundary。

### 2. 实现 runtime judgment boundary

新增 `src/features/weekly_radar/runtime/judgment.rs`：

- 定义可序列化的 machine/human 双轨 snapshot 类型；
- 从 `NormalizedFact` 构造 EvidenceRecord/EvidenceSet 与 proof references；
- 实现版本化、保守的 Stage Engine；
- 显式返回 `Assigned` 或 `Undetermined`；
- 只在 selected Stage 上建立 RankingCandidate/RankingReadModel；
- 用 ACL 转换 transformation Stage 与 ranking Stage，不把 conversion 散落进 renderer；
- 仅使用 fixed rule version，不调用网络、不读取 secret、不引入随机性。

### 3. 接入输入、归档和报告

- `runtime/model.rs` 在 `RuntimeReportInput` 中保存经过验证的 `JudgmentSnapshot`，并提供只读访问；
- `runtime/report.rs` 仅序列化 judgment 并渲染两个独立参考视图，不从 facts 重算 Stage/Ranking；
- `runtime.rs` 注册并导出新边界；
- `runtime/error.rs` 增加 typed judgment failures；
- `main.rs` 在 acquisition 后调用 automatic Stage Engine，再进入现有 snapshot/archive/publication path；
- dry-run 可以显示 `UNDETERMINED`，但不得把它变成排名或确定 Stage；
- 更新 `docs/operations/WEEKLY_RADAR.md`，只描述用户可看到的系统参考、人的独立判断和当前限制。

### 4. 绿色验证与回归

按顺序运行 focused test、全量 Rust tests、质量命令、AI Cockpit checks。补齐 Contract/Summary 的 scenario、guideline、documentation 和 residual-risk 证据。真实 Provider、无人值守、Telegram/Git recovery、longitudinal validation、calibration 不在本 Work Item 内，不能在 Outcome 中宣称完成。

### 5. 生命周期

完成实现和验证后：

```text
make ai-checkpoint CONTRACT=.ai/work-items/active/wi-runtime-judgment-chain-integration.contract.json SUMMARY=.ai/work-items/active/wi-runtime-judgment-chain-integration.summary.json STAGE=before_finish
make ai-finish TASK=wi-runtime-judgment-chain-integration REPORT_LANGUAGE=zh-CN
make check-ai-pr AI_BASE_COMMIT=1c7c443cd13410d186a4c3e178a3eb655388c731
```

随后按仓库规则创建 PR、跑 hosted checks、合并、归档、关闭 Work Item，并确认本地/远端分支和 worktree 无残留。任何未验证的机器判断质量或真实 Provider 结果都保留为 successor gap。
