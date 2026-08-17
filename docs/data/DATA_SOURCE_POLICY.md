# Data Source Policy

## 来源优先级

| 优先级 | 来源 | 用途 |
| --- | --- | --- |
| P0 | SEC EDGAR、SEC XBRL Company Facts、公司 Investor Relations、官方 filings/releases | 财务、员工线索、战略和组织事实；作为主要证据。 |
| P1 | 公司 Careers、Greenhouse Job Board、Lever Postings、官方 Engineering/AI Blog | 招聘结构、职位描述、生产系统和工作方式线索。 |
| P2 | GDELT 等免费发现型来源 | 广泛发现事件，再回到一手资料验证。 |

一手来源优先于二手来源；少量高质量证据优先于大量未经验证的叙事。ORG-X v0 不依赖付费数据 API。

## 运行时边界

- SEC EDGAR 和配置过的官方公司页面是 primary evidence candidates。
- Greenhouse 和 Lever 是有边界的结构化招聘来源，不能替代官方一手证据。
- GDELT 只用于 discovery，不能直接提升证据权威性。
- 缺少来源或可选配置为 `UNAVAILABLE`；歧义、冲突、日期缺失、对象不相关或格式错误为 `UNKNOWN`。
- 运行时只使用规则抽取；Provider JSON 留在 adapter 内，归一化事实保留 provider-neutral provenance。

当观察宇宙没有可用的一手证据时，发布流程 fail closed。Dry-run 使用相同的受限公开来源获取和验证路径，但不发送 Telegram、不改变 archive。

## Ingestion 边界

Ingestion 只负责可靠带入外部事实、去重并保存来源信息，不解释意义。每个来源至少保留 URI、标题、观察时间、有效日期和 content hash。外部文字永远先成为 Evidence Candidate。
