# Weekly Radar 同日最后一次成功更新为正本 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 允许 Weekly Radar 在同一天由 schedule 或手动触发再次成功发布，并让最后一次成功更新成为该日期的唯一正本；失败不覆盖旧正本。

**Architecture:** 为 archive transaction 增加显式的同日替换模式，并把本次输入快照纳入成功发布后的 transaction staged artifacts。保留现有创建型 API 的严格防重复语义；CLI 普通发布使用替换模式，retry/verify/republish 保持各自边界。Workflow 删除普通同日运行的提前 `ALREADY-PUBLISHED` 分支，但保留显式 republish 分支。

**Tech Stack:** Rust stable、Serde、Cargo tests、GitHub Actions YAML、AI Cockpit。

**Spec:** `docs/superpowers/specs/2026-08-24-weekly-radar-same-day-canonical-update.md`

## Global Constraints

- 不改动 `data`、`weekly-radar-pending`、Secret 或任何生产分支内容。
- 所有公共 API 变更必须有 Rustdoc；transaction 失败必须 fail closed。
- 每个任务先写失败测试，再写最小实现；每完成一个任务运行对应 focused test。
- 旧的创建型 archive、retry、verify、republish 和 pending recovery 测试不得因方便而删除。

## Task 1: 建立可恢复的同日替换 archive transaction

**Files:** `src/features/weekly_radar/runtime/archive.rs`, `src/features/weekly_radar/runtime.rs`, `tests/weekly_radar_runtime.rs`

- [x] 为同日替换新增失败测试：第二个不同输入可以提交；最终 manifest、report、snapshot、receipt、input snapshot 的 identity 全部指向新运行。
- [x] 新增失败恢复测试：替换 transaction 在每个 promotion stage 中断时，旧正本不会被误判为新正本，恢复后五个 artifact 一致；发送前未提交的新输入快照不会泄漏。
- [x] 新增失败关闭测试：旧正本不完整、manifest 不一致或存在 prepared transaction 时，替换被拒绝且不覆盖已有文件。
- [x] 扩展 transaction artifact/校验模型，兼容既有四 artifact transaction，同时支持含输入快照的五 artifact replacement transaction。
- [x] 增加显式、文档化的 replacement archive API；让输入快照和四个最终文件在成功发送后同一逻辑 transaction 提交。
- [x] 运行 `cargo test --test weekly_radar_runtime task5_` 与 archive 模块 focused tests。

## Task 2: 让 CLI 普通运行使用同日替换，保持 retry/verify/republish 边界

**Files:** `src/main.rs`, `src/features/weekly_radar/runtime.rs`, `tests/weekly_radar_runtime.rs`

- [x] 添加失败测试：普通第二次运行不会在 archive 已有当天正本时返回 `ExistingRun`；retry 仍不覆盖已完成正本；verify/republish 语义不变。
- [x] 将普通发布改为在内存构造输入快照，Telegram 成功后通过 replacement API 一起提交，避免发送前覆盖旧输入快照。
- [x] 保留 `persist_input_snapshot` 的严格创建/幂等 API 给旧调用方和 retry 兼容路径使用。
- [x] 更新 CLI 相关测试，验证同日成功更新、发送失败后的旧正本和输入快照绑定。
- [x] 运行 `cargo test --test weekly_radar_runtime task5_cli`、`cargo test --bin org-x`。

## Task 3: 更新 Actions 与用户说明

**Files:** `.github/workflows/weekly-radar.yml`, `tests/weekly_radar_runtime.rs`, `docs/operations/WEEKLY_RADAR.md`

- [x] 添加失败静态测试：普通同日运行继续调用普通 CLI，不能无条件转为 verify；显式 republish 仍不更新正本。
- [x] 删除普通同日 final archive 的提前退出；保留 pending/recovery、日期顺序、data 目标和 lease 保护。
- [x] 更新用户文档：手动触发不受同日限制；最后一次成功更新为正本；失败保留旧正本；retry/verify/republish 的区别。
- [x] 运行 workflow 静态测试和文档检查。

## Task 4: 完整验证与 WI 收口

- [ ] 更新 Contract/Summary 的验收、场景、残余风险和证据。
- [ ] 运行 `cargo test --all-targets`、`make ai-checkpoint ... STAGE=before_finish`、`make ai-finish TASK=... REPORT_LANGUAGE=zh-CN`。
- [ ] 归档、提交、推送并运行 `make check-ai-pr AI_BASE_COMMIT=ecd17580be6c234ef0208ae9f8ace07323af2bfd`。
- [ ] 完成 PR hosted checks、合并、`make ai-close-work-item TASK=...`，并确认本地/远端分支与 worktree 无残留。
