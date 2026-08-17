# WI-009 Diffusion Domain Design

## Boundary

WI-009 records diffusion facts that can later support reference-model review:
competitor imitation, job taxonomy change, benchmark comparison, industry
diffusion, and categorized diffusion signals. The Domain preserves supplied
facts and insertion order; it does not decide whether diffusion is sufficient
for a stage, score, ranking, or publication.

## Model

- `CompetitorImitation` retains subject company, imitator company, scope, and observation date.
- `JobTaxonomyChange` retains company, role label, change description, and date.
- `BenchmarkObservation` retains benchmark, opaque comparison, period, company, and date.
- `IndustryDiffusion` retains industry, description, and date without aggregating adoption.
- `DiffusionSignalKind` classifies workflow redesign, job taxonomy, productivity benchmark, advisory adoption, and capital reallocation.
- `DiffusionProfile` groups ordered facts for one company and rejects duplicate fact identities across collections.

## Explicit no-goals

This slice does not acquire external data, resolve company identity, infer
industry adoption, calculate productivity, assign a Transformation Stage,
calculate a score, rank companies, render reports, persist snapshots, or send
messages. One observation remains one observation.

## Design decisions

1. Text values are validated at construction and remain opaque; the Domain performs no normalization beyond rejecting blank input.
2. Fact collections expose read-only slices in insertion order.
3. A fact identity is unique across the whole profile, preventing the same observation from being silently reused under another category.
4. `DiffusionSignalKind` is a category, not a score or stage decision.
5. The Domain uses only the Rust standard library and imports no other feature module.

## Authorization and issue policy

The user authorized execution, verification, publication, merge, closure, and
archive for all 24 roadmap WIs. That authorization is recorded in the Contract.
If a problem is found during this WI and remains inside the Contract scope, it
must be resolved in this WI; a new WI is reserved for a distinct or materially
expanded boundary.

## Verification

- `cargo test --test diffusion_domain`
- `cargo test diffusion::domain`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `make check`
- `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`
