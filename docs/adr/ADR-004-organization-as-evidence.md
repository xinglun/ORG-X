# ADR-004: Organization Rewrite Is Evidence, Not the North Star

## Status

Accepted

## Context

裁员、减少管理层或增加 Agent 可能是成本控制、实验或宣传，并不必然表示生产系统变化。

## Decision

Organization Context 只作为 Production System Rewrite 的重要证据。任何组织变化必须解释其如何服务新的核心生产方式。

## Consequences

组织事实仍然重要，但不能单独升级 Stage；系统会保留 management commitment、responsibility、budget、approval 和 decision rights 的上下文。

## Enforcement

`docs/domain/PRODUCTION_SYSTEM_MODEL.md`、`docs/architecture/BOUNDED_CONTEXTS.md` 和 Stage Gate 规则共同执行。
