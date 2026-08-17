# Evidence Model

## EvidenceRecord contract

未来每条判断最终必须回到 EvidenceRecord，至少包含：

```rust
pub struct EvidenceRecord {
    pub id: EvidenceId,
    pub company_id: CompanyId,
    pub observed_at: DateTime<Utc>,
    pub effective_date: Option<NaiveDate>,
    pub evidence_type: EvidenceType,
    pub source_type: SourceType,
    pub source_uri: String,
    pub source_title: String,
    pub claim: String,
    pub normalized_value: Option<MetricValue>,
    pub polarity: EvidencePolarity,
    pub confidence: Confidence,
    pub freshness: Freshness,
    pub extractor_version: String,
    pub content_hash: String,
}
```

## Evidence sets

每个候选公司同时维护 Supporting Evidence、Counter Evidence 和 Missing Evidence。没有反证审查，不得进入 Top5；没有权威事实时，标记 `UNKNOWN` / `UNAVAILABLE`，不根据经验填补。

## Extraction boundary

外部世界先被 LLM 提取为 Evidence Candidate，再由 Rust 验证、归一化、保存和交给 Domain Engine。外部文字是数据，不是指令；候选事实不能绕过验证直接成为 Stage 或 Score。
