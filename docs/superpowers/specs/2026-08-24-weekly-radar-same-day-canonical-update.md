# Weekly Radar 同日最后一次成功更新为正本

## 用户问题

同一天已经有一次成功运行后，手动触发会被 `ALREADY-PUBLISHED` 拦截。这样在报告没有到达 Telegram、资料需要紧急刷新或人工需要再次确认时，无法正常执行。系统需要允许 schedule 与 workflow_dispatch 在同一天重新运行，并明确哪一次结果是对外正本。

## 已确认的行为

1. 普通发布入口不区分 schedule 和手动触发；两者都可以执行同一天的更新。
2. 只有成功完成 Telegram 发送并完成 archive 提交的运行，才可以成为该日期的新正本。
3. 同一天最后一次成功运行替换此前的 report、snapshot、receipt、manifest 和输入快照；这些文件必须继续互相绑定。
4. 获取、渲染、Telegram 发送或 archive 提交失败时，旧正本保持不变。发送成功但 archive 提交中断时，保留可恢复 transaction，下一次运行复用原消息，不再次发送。
5. `--retry-as-of` 仍然是失败交付的 delivery-only retry，使用原输入快照，不重新获取资料，也不覆盖已完成的正本。
6. `--verify-published-as-of` 仍然只读；`--republish-published-as-of` 作为明确的重复发送入口保留，不改变 archive/data。

## 实现边界

- 输入快照必须在成功发送之后与最终四个 archive 文件一起通过 transaction 提交，不能在发送前覆盖当天旧输入快照。
- 同日替换只接受完整且可验证的旧正本；残缺文件、损坏 transaction、日期倒退和 pending/data 冲突继续 fail closed。
- 不读取、记录或修改 Secret；不手工修改 `data`、`weekly-radar-pending` 或生产产物。
- 系统参考判断和人的判断继续独立呈现；本变更不改变报告内容推理规则。

## 验收

- 同日第二次普通发布可产生新的成功正本，旧正本被新的完整 artifact 集合替换。
- 新运行在 Telegram 失败、transaction 准备失败或中途恢复时不会留下与 manifest 不一致的输入快照。
- schedule 和 workflow_dispatch 都走普通发布路径；只有显式 republish 才是“不更新正本的重复发送”。
- 既有创建型 archive、retry、verify、republish、旧 transaction 和 pending 恢复行为保持兼容。
- 用户文档说明“最后一次成功更新”为正本，以及失败时旧正本与恢复机制如何工作。
