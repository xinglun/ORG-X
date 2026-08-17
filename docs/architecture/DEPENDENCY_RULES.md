# ORG-X Dependency Rules

## 允许的方向

`interface -> application -> domain`。Infrastructure 只能实现由 Application 或 Domain 定义的 port；ACL 只负责隔离和转换外部数据结构。反向引用属于架构违规。

## 禁止耦合

- Domain 不依赖 Infrastructure、Interface、网络客户端、数据库、LLM SDK 或外部 provider。
- Transformation 不依赖外部 provider 或网络实现。
- Ranking 不依赖外部 provider、网络、数据库或 renderer。
- Production System 不依赖 Reporting、renderer 或 interface。
- Evidence Domain 不依赖具体 source provider、provider JSON 或 SEC/news/fundamental implementation。

## 如何验证

架构测试逐文件扫描禁止令牌，并验证所有 Context 的五层目录和根 module exports。测试是架构规则的一部分，不是事后说明。

## Provider isolation

Provider 的 JSON、SDK 类型和错误模型不得传播到 Domain。它们必须在 ACL 中翻译为 Application/Domain 能理解的端口输入；无法翻译的数据保持 `UNKNOWN` 或 `UNAVAILABLE`。
