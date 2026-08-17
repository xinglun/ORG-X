# WI-010 Ranking Read Model Design

## Boundary

WI-010 stores research candidates and provides deterministic ordering only
within a caller-selected Transformation Stage. The fixed key order is:

1. Evidence Confidence descending
2. Transformation Score descending
3. Counter Evidence Risk ascending
4. Evidence Freshness descending
5. Candidate identity ascending as a deterministic tie-break

Stage remains the first boundary. The API does not expose a cross-Stage total
ranking, and Transformation Score is never a replacement for Stage.

## Model

- `Stage` owns six explicit grouping labels for this Read Model.
- `RankingCandidate` retains company, Stage, and four independent bounded values.
- `RankingReadModel` rejects duplicate candidate identities and preserves insertion order.
- `ranked_within_stage(stage)` filters first, then applies only the fixed key order.

## Explicit no-goals

This slice does not assign a Stage, calculate a score, acquire or validate
Evidence, calculate freshness, persist snapshots, render reports, schedule
work, publish messages, or produce trading/capital-action behavior.

## Design decisions

1. Each ranking dimension is a separate 0–100 value object; no hidden composite score exists.
2. Counter Evidence Risk is ordered ascending so lower risk is preferred.
3. Freshness is ordered descending so fresher supplied facts are preferred.
4. A caller must select the Stage, preventing accidental cross-Stage comparison.
5. The Domain uses only the Rust standard library and does not import other feature modules.

## Authorization and issue policy

The user authorized execution, verification, publication, merge, closure, and
archive for all 24 roadmap WIs. That authorization is recorded in the Contract.
Issues discovered during this WI are fixed here whenever they remain within the
Contract scope; a new WI is reserved for a distinct or materially expanded
boundary.

## Verification

- `cargo test --test ranking_domain`
- `cargo test ranking::domain`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `make check`
- `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`
