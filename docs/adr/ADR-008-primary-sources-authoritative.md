# ADR-008: Primary Sources Have Highest Authority

## Status

Accepted

## Context

SEC、公司正式披露和第三方估算可能冲突。平均冲突会抹去来源权威和变化本身。

## Decision

Primary sources 拥有最高 authority。冲突不平均：保存 authoritative fact，同时保存 conflicting secondary observation 及其质量信息。

## Consequences

系统更能解释数据冲突和时间变化，但必须保存来源 tier、effective date、freshness 和 confidence。

## Enforcement

`docs/data/DATA_SOURCE_POLICY.md`、`docs/data/DATA_QUALITY_POLICY.md` 和 EvidenceRecord contract 执行。
