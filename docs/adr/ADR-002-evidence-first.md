# ADR-002: Evidence Before Score

## Status

Accepted

## Context

漂亮叙事、AI 宣传和单篇新闻容易制造虚假的转型信号。ORG-X 必须能解释每个判断由哪些事实支持。

## Decision

没有 EvidenceRecord 不评分、不升级 Stage、不进入 Top5。每个候选必须维护 Supporting Evidence、Counter Evidence 和 Missing Evidence。

## Consequences

研究结果更慢但可复核；数据缺失会保留为 UNKNOWN / UNAVAILABLE，而不是被推测填满。

## Enforcement

`docs/domain/EVIDENCE_MODEL.md`、`docs/scoring/SCORING_SPEC.md` 和未来 Evidence Domain Contract 执行该规则。
