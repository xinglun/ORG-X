# ADR-005: LLM Extracts; Rust Decides

## Status

Accepted

## Context

外部世界包含大量非结构化 filing、transcript、新闻和职位信息。LLM 适合理解文本，但不应拥有不可审计的评分或 Stage 权力。

## Decision

LLM 只能把外部内容提取为 Evidence Candidate；Rust 负责验证、状态转换、评分、排名和规则。候选内容不能绕过 EvidenceRecord 进入 Domain Engine。

## Consequences

提取器可以迭代并记录 extractor_version，决策仍保持确定性；系统必须处理候选无法验证的情况。

## Enforcement

`docs/domain/EVIDENCE_MODEL.md`、`docs/architecture/DEPENDENCY_RULES.md` 和 transformation 架构测试执行。
