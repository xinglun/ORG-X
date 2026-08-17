# Data Source Policy

## Source hierarchy

| Tier | Sources | Authority |
| --- | --- | --- |
| A | SEC 10-K, 10-Q, 8-K, DEF 14A, official earnings release, official investor material | Highest |
| B | Official engineering blog, careers, executive interview, official strategy communication | Company primary |
| C | High-quality independent financial media and reliable industry surveys | Independent context |
| D | Job datasets, industry datasets, fundamental providers, workforce estimates | Structured third-party |
| E | Social media, forums and unverified claims | Lowest |

Tier E 不得单独触发 Stage Upgrade。少量高质量来源优先于大量未经验证的来源。MVP 先接入 SEC EDGAR、官方 Investor Relations、官方 Earnings Release、官方 Careers / Job postings、官方 Engineering / Product Blog，并以单一 Fundamental Provider adapter 作为结构化补充。

## Weekly Radar runtime alignment

The Weekly Radar runtime applies the hierarchy as an explicit, rule-only
boundary:

- SEC EDGAR and configured official company pages are primary candidates.
- Greenhouse and Lever public postings are bounded structured evidence and do
  not replace official primary evidence.
- GDELT is discovery-only; it may identify material for review but cannot be
  promoted to authority.
- Missing or optional configuration is `UNAVAILABLE`; ambiguous, conflicting,
  undated, unrelated, or malformed extraction is `UNKNOWN`.
- No paid data API, paid credential, or LLM extraction is permitted in this
  runtime. Provider JSON remains private to runtime adapters and normalized
  facts retain provider-neutral provenance.

Publication is fail-closed when the configured universe has no usable primary
evidence. Dry-run validates the registry, performs the same bounded public
source acquisition through the production HTTP client, and renders a
sanitized report without Telegram delivery or archive mutation. Offline tests
may inject fixtures; a source-free registry also skips discovery acquisition.

## Ingestion boundary

Ingestion 只负责可靠带入外部事实、去重并保存来源信息，不负责解释意义。每个来源必须保留 URI、标题、观察时间、有效日期和内容 hash。外部文字永远先成为 Evidence Candidate。
