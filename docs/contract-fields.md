# AI Cockpit Contract 与 Summary 字段

本项目使用外部安装的 Rust `ai-cockpit` Runtime。下面只记录当前 Runtime
读取的 repository-local Work Item 格式；旧 Python/Make 格式仅存在于历史
归档中，不应为新任务复制。

## Contract

Contract 位于 `.ai/work-items/active/<id>.contract.json`，由 Runtime 绑定
repository 和当前快照。`ai-cockpit work-item new` 生成事实字段，Agent 或人
只补充人类拥有的决策字段。

必需或关键字段包括：

- `protocolVersion`、`workItemId`、`mode`：协议版本、唯一 Work Item ID 和模式。
- `repositoryId`、`baseRevision`、`repositorySnapshotDigest`：repository、基线
  和创建时快照绑定。
- `intent`、`goal`：问题背景和目标；不能用 Agent 自信替代人类意图。
- `scope`、`outOfScope`：允许修改和明确禁止修改的路径或 glob。
- `authority`、`risk`：授权状态和风险级别。
- `acceptanceCriteria`：带稳定 ID（例如 `A1:`）的验收条件。
- `sources`、`verification`、`requiredEvidenceClasses`：来源和所需验证证据。
- `scenarioCoverage`：高风险 Work Item 的场景、预期结果和验证计划。
- `resourceContext`：branch、remote、base 和 PR 等外部上下文；未知事实必须保留
  为未知，不得猜测。

`preflight` 从这些事实计算状态。`not_ready` 或
`needs_human_confirmation` 必须暂停；`yellow` 只有在 Runtime 给出安全的验证
动作时才可继续。Contract 变更后必须重新运行 preflight。

## Summary 与生成证据

`.summary.json`、checkpoint、verification、finish、archive 和 outcome 记录由
Runtime 生成。不要手工改写生成的 Summary 或状态投影。验证必须绑定到当前
Work Item、当前 Contract digest 和当前 repository snapshot；旧归档不回填、不
重写。

## 生命周期命令

```bash
ai-cockpit work-item new --repo <repo> --id <id> --mode code
ai-cockpit start --repo <repo> --id <id> --intent "..." --goal "..." \
  --scope 'docs/**' --authority authorized
ai-cockpit preflight --repo <repo> \
  --contract .ai/work-items/active/<id>.contract.json
ai-cockpit checkpoint --repo <repo> --id <id>
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args test,--workspace --workers 1
ai-cockpit finish --repo <repo> --id <id>
ai-cockpit work-item outcome --repo <repo> --id <id>
ai-cockpit archive --repo <repo> --id <id>
```

所有命令都必须显式带 `--repo`。`close` 只在 reviewed PR 已 merge、默认分支已
同步、归档和清理事实都验证后运行。本页不授权删除历史归档、recovery receipt
或 `.ai/install` 发布证据。
