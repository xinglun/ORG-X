# ORG-X

ORG-X 是一个研究雷达，用来寻找 AI 从工具变成新生产方式的临界点。它观察美国上市公司如何重构核心生产系统、工作流、组织责任和经济结果，并把值得继续研究的案例整理出来。

## 先读什么

- [能力一览](docs/CAPABILITIES.md)：按能力域查看当前边界、状态和详细证据链接。
- [读者导航](docs/README.md)：完整的产品、方法、架构和运维说明。
- [产品 North Star](docs/product/NORTH_STAR.md)：ORG-X 要回答的问题，以及什么才算生产方式变化。
- [证据与数据规则](docs/data/DATA_SOURCE_POLICY.md)：来源优先级、溯源和 `UNKNOWN` / `UNAVAILABLE` 的含义。
- [Weekly Radar 使用说明](docs/operations/WEEKLY_RADAR.md)：本地运行、Telegram 配置、调度和 `data` 分支保留规则。

## ORG-X 研究什么

ORG-X 关注的不是“谁拥有最强 AI”，而是：

1. 谁正在改变？
2. 它改变了什么生产方式？
3. 这种变化是否已经产生结构性的生产率优势？

研究对象从 AI 工具、局部替代和工作流重构开始，逐步检查核心生产系统、组织适配、生产率突破和行业扩散。

## 研究边界

ORG-X 是生产方式研究系统，不是交易系统。`Top5`（研究优先级前五项）、`Rising`（结构证据增强）、`Watch`（待继续研究）和 `Dropped`（结构证据减弱或失效）只表示研究资源优先级，不代表买入、持有、卖出、价格预测、仓位或任何资本行动。定义详见 [Ranking Model](docs/domain/RANKING_MODEL.md)。

外部资料首先是数据和 Evidence Candidate。系统优先使用一手来源，只用规则抽取；无法确认、存在歧义或缺少必要来源时保留 `UNKNOWN` 或 `UNAVAILABLE`，不根据经验补全。

## 本地检查

环境要求见仓库的 [Rust toolchain 配置](rust-toolchain.toml)。安装工具链后，可先运行以下命令验证本地环境；Weekly Radar 的参数、环境变量和运行限制见 [使用说明](docs/operations/WEEKLY_RADAR.md)。

```bash
make check
```

该命令运行格式检查、禁止警告的 Clippy 检查和完整测试。
