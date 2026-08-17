# ORG-X

ORG-X searches for the point where AI stops being a tool and becomes a new mode of production. It observes US-listed companies for evidence that AI is restructuring a core production system, creating persistent productivity divergence, and becoming a reference model for peers.

## Delivery status

Foundation、AI Cockpit adoption/configuration、Engineering Foundation Verification 和 Universe Domain 已完成并归档。当前没有 Active Work Item；下一项推荐是 `WI-003 Ingestion Domain & Observation Contract`。

剩余 Core Research Pipeline 与 Weekly Radar 的 WI 清单、依赖顺序、输出结构和边界见 [ORG-X Work Item Roadmap](docs/superpowers/plans/2026-08-17-orgx-wi-roadmap.md)。Weekly Radar 实现必须等核心 Reporting Read Model（`WI-011`）完成后再启动。

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

从最新 `origin/main` 创建专用 Work Item 分支，先运行 AI Cockpit Preflight，再开始 `WI-003`。每个后续 WI 都必须独立完成 Contract、Summary、验证、PR 和关闭生命周期。
