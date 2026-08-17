# ADR-010: Historical Snapshots Are Immutable Evidence

## Status

Accepted

## Context

ORG-X 必须能回答公司何时首次进入某个 Stage、当时有哪些证据，以及后来的事实是否证明判断正确或错误。可变的当前状态无法支持验证。

## Decision

持久化 UniverseSnapshot、EvidenceSnapshot、TransformationProfileSnapshot、RankingSnapshot 和 Top5Snapshot。快照及 Stage Transition 原因作为不可变历史证据保存。

## Consequences

系统可以重放历史判断、解释排名变化并进行 6/12/24 个月验证；后续修正通过新快照和降级记录表达，不覆盖旧证据。

## Enforcement

`docs/validation/VALIDATION_STRATEGY.md`、`docs/scoring/STAGE_GATE_SPEC.md` 和未来 Historical Snapshots Work Item 的验收标准执行。
