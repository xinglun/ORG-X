# ADR-003: Production System Is the Core Domain

## Status

Accepted

## Context

使用 AI 的员工数量、模型能力或宣传声量都不能直接说明企业生产方式改变。North Star 要寻找核心价值创造过程的结构性重构。

## Decision

Production System Context 是 ORG-X 核心 Domain。所有组织、生产率、Stage 和扩散判断都必须能回到核心生产过程如何改变。

## Consequences

系统优先重建 workflow、decision、roles、control、verification 和 exception path；单点工具部署不会自动获得高 Stage。

## Enforcement

`docs/domain/PRODUCTION_SYSTEM_MODEL.md` 与 `src/features/production_system/` 是核心边界；架构规则禁止核心域依赖输出层。
