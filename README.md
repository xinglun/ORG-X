# ORG-X

ORG-X searches for the point where AI stops being a tool and becomes a new mode of production. It observes US-listed companies for evidence that AI is restructuring a core production system, creating persistent productivity divergence, and becoming a reference model for peers.

## Phase 0 status

当前仓库是 Foundation 阶段：Rust crate、DDD Bounded Context、Clean Architecture 目录、North Star / PRD / Architecture / Domain / Data / Scoring / Validation 文档、ADR-001 至 ADR-010 和 Architecture Tests 已建立。正式业务功能尚未开始。

## Local checks

```bash
make check
```

The check runs formatting, Clippy with warnings denied, and all tests. The required direct commands are:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
```

## Scope

ORG-X is a production-system research radar, not a trading system. It does not produce price forecasts, buy/sell decisions, position sizing, portfolio management, or trading gates. Top5 is a research priority list.

The initial calibration universe is S&P 500 plus Nasdaq 100, deduplicated. META, PLTR, MSFT, GOOG, AMZN, NVDA, CRM, ADBE, IBM, and WMT are candidates for calibration, not expected winners.

## Architecture

The repository uses one Rust crate with ten Bounded Contexts. Each Context exposes `domain`, `application`, `infrastructure`, `interface`, and `acl` boundaries. Architecture Tests enforce that pure domain code stays independent of future external implementations.

## Evidence boundary

SEC filings, company material, news, job postings, and other external text are data and become Evidence Candidates only. They are never executable instructions to an Agent. AI extracts; Rust decides.

## Next gate

After Phase 0 checks pass, the next step is installation and acceptance of AI Cockpit. Its first Work Item is WI-001 Engineering Foundation Verification; no business Work Item may start before that verification is archived.
