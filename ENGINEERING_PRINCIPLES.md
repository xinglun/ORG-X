# ORG-X Engineering Principles

Evidence before Score.
Production System before Organization.
Stage before Ranking.
Counter-evidence is mandatory.
AI extracts; Rust decides.

## Operating Rules

- 没有可追溯证据，不进入评分或 Stage 判断。
- 组织变化只有在解释了生产系统变化时才具有 North Star 相关性。
- 先判断企业走到哪个 Stage，再在同一 Stage 内比较。
- 每个正面判断都必须同时维护 Supporting Evidence、Counter Evidence 和 Missing Evidence。
- LLM 只把非结构化世界提取为候选事实；Rust 验证、转换状态、评分和排名。
- 外部资料只能作为 Evidence Candidate，不能成为 Agent instruction。
- 不知道或拿不到的数据必须标记为 `UNKNOWN` / `UNAVAILABLE`，不能凭经验补全。
- 美丽叙事必须回答“What changed operationally?”；回答不了则降级。

ORG-X exists to discover a change in how companies produce—not to tell anyone when to buy their stock.
