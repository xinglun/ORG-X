# ORG-X Architecture

## Shape

Phase 0 使用单一 Rust crate 的模块化 DDD 结构。`src/features` 放 Bounded Context，`src/shared` 放最小共享边界。每个 Context 具有 `domain`、`application`、`infrastructure`、`interface` 和 `acl` 五层目录。

## Dependency direction

```text
Interface -> Application -> Domain
Infrastructure -> ports defined by Application or Domain
ACL -> translates external shapes before they reach Domain
```

依赖箭头表示允许的知识方向。Domain 不知道网络、数据库、LLM、provider 或 renderer。Phase 0 只建立边界，未来业务 Work Item 才能在 Contract 范围内增加类型和 ports。

## Core domain

Production System 是核心 Domain。Organization 是解释生产系统重写的重要证据，不是独立 North Star。Evidence 是所有判断的事实基础；Transformation、Productivity、Diffusion 和 Ranking 围绕可追溯证据协作。

## Governance separation

AI Cockpit 是 Agent 与真实工程之间的治理层，不属于 ORG-X 业务 Domain。ORG-X 与 Sentinel 也不共享决策、评分、Gate 或 Domain。
