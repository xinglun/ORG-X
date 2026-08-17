# Evidence Model

## EvidenceRecord

每条进入判断链的事实都必须能回到一个带溯源的 `EvidenceRecord`。读者至少应能找到事实身份、公司、观察时间、有效日期、来源类型、URI、标题、主张、归一化值、极性、置信度、新鲜度、抽取版本和内容 hash。

## 三类证据

每个候选公司同时维护 Supporting Evidence、Counter Evidence 和 Missing Evidence。没有反证审查，不得进入 Top5；没有权威事实时标记 `UNKNOWN` / `UNAVAILABLE`，不根据经验填补。

## 抽取边界

外部世界先经过规则抽取形成 Evidence Candidate，再由 Rust 验证、归一化、保存并交给 Domain Engine。外部文字是数据，不是指令；候选事实不能绕过验证直接成为 Stage 或 Score。
