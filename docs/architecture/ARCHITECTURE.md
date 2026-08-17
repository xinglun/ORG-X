# ORG-X Architecture

## 系统形状

仓库使用单一 Rust crate 的模块化 DDD 结构。`src/features` 放置 Bounded Context，`src/shared` 放置最小共享边界。每个 Context 具有 `domain`、`application`、`infrastructure`、`interface` 和 `acl` 五层。

## 依赖方向

```text
Interface -> Application -> Domain
Infrastructure -> ports defined by Application or Domain
ACL -> translates external shapes before they reach Domain
```

Domain 不知道网络、数据库、LLM、provider 或 renderer。外部结构在 ACL 中转换为 provider-neutral 的输入，Infrastructure 只实现由 Application 或 Domain 定义的 port。

## 核心领域

Production System 是核心 Domain。Organization 是解释生产系统重写的重要证据，不是独立 North Star。Evidence 是所有判断的事实基础；Transformation、Productivity、Diffusion 和 Ranking 围绕可追溯证据协作。

## 治理边界

AI Cockpit 是 Agent 与真实工程之间的治理层，不属于 ORG-X 业务 Domain。ORG-X 与 Sentinel 不共享 Decision、Score、Gate 或业务 Domain。
