# Weekly Radar 自动判断链设计

## 目标

把 Weekly Radar 当前的

`Provider facts → report`

改为可审计的：

`Provider facts → Evidence → automatic machine reference → Stage → same-Stage Ranking → Snapshot/report`

系统自动给出一个可复核的参考判断。人的判断是独立参考，不是系统的输入，也不与系统合并成一个“共识答案”。报告中两种视图并列展示，允许人确认、质疑或保留不同结论。

## Stage authority

本 Work Item 采用用户选择的 B：由 Evidence-first Stage Engine 自动推导机器参考。

Stage Engine 只消费已规范化且带 provenance 的 `NormalizedFact`。它不从一条新闻、单个字段或缺少来源的文本跳级。当前版本采用显式的 provider-neutral stage signal 命名约定：

- `judgment.supporting.<STAGE>.<CATEGORY>`：支持某 Stage 的已确认事实；
- `judgment.counter.<STAGE>.<CATEGORY>`：反向或削弱某 Stage 的事实；
- `judgment.missing.<STAGE>.<REQUIREMENT>`：仍需补齐的证明要求。

每个 Stage 的自动参考至少需要两个不同 `source_uri` 的 supporting signal、一个 counter signal 和一个明确的 missing-proof inventory。缺任一门槛时输出 `UNDETERMINED`，不猜测 Stage，也不生成该公司的 machine Ranking。该规则是可版本化的机器参考规则，不是经过长期历史数据校准的真理；校准属于后续 Work Item。

Ranking 只在已经通过 Stage gate 的同一 Stage 内执行，沿用 Ranking Domain 的四个独立维度：Evidence Confidence、Transformation Score、Counter Evidence Risk、Evidence Freshness。Stage Engine 负责提供这些保留下来的维度；renderer 只序列化，不重新计算。

## 双轨视图

`MachineJudgment` 包含：

- machine Stage（或 `UNDETERMINED` 及原因）；
- supporting/counter/missing proof references；
- evidence cutoff 与每条 proof 的 source provenance；
- 同一 Stage 内的 Ranking read model；
- rule version。

`HumanReference` 是可选的独立视图，只包含人的公司判断、Stage 参考、备注和记录时间。它不参与 machine Stage、score、Ranking 的计算；系统不投票、不平均、不覆盖、不训练、不合并这两个视图。

## Snapshot 与报告

`RuntimeReportInput` 持有可选的、已经验证的 `JudgmentSnapshot`。JSON snapshot 保留：事实截止日、Evidence cutoff、事实 provenance、supporting/counter/missing proof、machine judgment、machine ranking、human reference（如有）和既有 source health/no-change 数据。

Markdown/Telegram 视图把 machine reference 与 human reference 分开标识。没有足够 Evidence 时显示“系统暂无法判断”，而不是把事实缺口渲染成 Stage 或排名。报告 renderer 不接受原始 Stage/score 参数，也不从 facts 重新推导 Stage/Ranking。

## 失败边界

- fact 的 company、kind、status、provenance 无效：返回 typed runtime error；
- supporting/counter/missing proof 无法绑定到现有 fact：返回 typed runtime error；
- Evidence gate 不足：返回 `UNDETERMINED`，不生成 machine Ranking；
- 跨 Stage 候选：由 Ranking Domain 的 `ranked_within_stage` 隔离，不形成跨 Stage 总序；
- renderer 收到没有验证过的 judgment：拒绝构造 report；
- 普通报告仍可保留 source health 和事实观察，但不能把 `UNDETERMINED` 伪装成确定 Stage。

## 不在本 Work Item 内

真实 Provider 如何产生 stage signals、长期历史校准、无人值守定时、Telegram/Git 失败恢复，以及人的参考判断录入界面，分别由后续 successor Work Item 提供真实证据。本 Work Item 先把自动参考的边界接入真实 runtime，并确保后续 Provider E2E 不能绕过该边界。
