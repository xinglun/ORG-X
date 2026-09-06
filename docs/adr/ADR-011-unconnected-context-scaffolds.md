# ADR-011: Ten Bounded Contexts Remain Domain-Only Scaffolds

## Status

Accepted

## Context

`src/features/` 下有 12 个按 ADR-001 边界划分的 Context。目前只有 `weekly_radar` 和 `validation` 在 `application` / `infrastructure` / `interface` 层有真实实现；其余 10 个（`diffusion`、`evidence`、`ingestion`、`organization`、`production_system`、`productivity`、`ranking`、`reporting`、`transformation`、`universe`）只在 `domain` 层定义了值对象和实体，其余层仅有 `//! Boundary marker for this layer.` 的占位符。一次全项目评审将这一现状标记为潜在的"外观完整但未接线"的风险。

## Decision

保留这 10 个 Context 的域层作为**有意为之的未来扩展地基**，不在本 ADR 下删除或强行实现它们。`weekly_radar` 正是通过组合这些域类型（如 `evidence::domain::EvidenceRecord`、`production_system::domain` 的概念）来实现的，说明这些域模型不是死代码，而是尚未获得独立 application/infrastructure/interface 层的预置词汇表。

## Consequences

- 新增功能若需要 evidence、production_system 等概念的独立生命周期（而不是通过 weekly_radar 内部逻辑组合），应优先扩展对应 Context 的 application/infrastructure/interface 层，而不是在 `weekly_radar` 内继续堆积职责。
- 在没有具体功能需求前，不应为了"看起来完整"而向这些空占位层添加代码；空占位符本身就是准确的状态声明。
- 若某个 Context 在合理时间内（例如跨越多个季度）始终没有被任何具体功能引用，应重新评审是否降级为文档化概念或彻底移除，而不是无限期保留。

## Enforcement

`tests/architecture/module_boundaries.rs` 强制每个 Context 具备五层目录结构（含占位符），`.ai/glossary.md` 记录当前实现边界，供后续读者和 Agent 在扩展前先确认目标 Context 的真实状态。
