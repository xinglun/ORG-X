# ADR-001: Use DDD and Clean Architecture

## Status

Accepted

## Context

ORG-X 同时处理公司、证据、生产系统、组织、生产率、阶段和扩散。若 provider、数据库和研究逻辑混在一起，North Star 无法保持可审计。

## Decision

采用 DDD Bounded Context 与 Clean Architecture。每个 Context 通过 domain、application、infrastructure、interface 和 acl 边界表达职责，依赖由外向内。

## Consequences

Domain 可以独立验证，外部来源可以替换；早期需要维护目录和 Architecture Tests，业务实现必须遵守边界。

## Enforcement

`docs/architecture/ARCHITECTURE.md`、`docs/architecture/BOUNDED_CONTEXTS.md`、`docs/architecture/DEPENDENCY_RULES.md` 与 `tests/architecture/` 共同执行。
