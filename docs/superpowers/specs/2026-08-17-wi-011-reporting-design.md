# WI-011 Reporting Read Model Design

## Boundary

WI-011 packages upstream read-model facts into a stable research packet with
four sections: Top5, Rising, Watch, and Dropped. It owns section capacity,
identity validation, and insertion order. It does not decide section
membership or calculate any upstream fact.

## Model

- `ResearchCard` retains identity, company, supplied Stage label, headline, evidence summary, counter evidence, missing proof, and next research step.
- `Top5` accepts at most five unique cards.
- `ReportSection` represents Rising, Watch, or Dropped and preserves unique cards in insertion order.
- `ResearchPacket` groups the executive summary and four supplied sections.
- `ReportingReadModel` is an assembly boundary that accepts cards and creates a packet without recomputation.

## Explicit no-goals

This slice does not assign Stage, calculate Ranking, calculate threshold
distance, acquire or validate evidence, persist snapshots, render Markdown or
Telegram output, schedule work, deliver messages, or retry publication.

## Design decisions

1. Card fields are opaque validated text values; Reporting does not interpret them.
2. Duplicate identities are rejected within each section.
3. Top5 checks duplicate identity before capacity so the error remains deterministic.
4. Packet accessors return immutable references; downstream renderers consume the same packet.
5. The Domain uses only the Rust standard library and imports no other feature module.

## Authorization and issue policy

The user authorized execution, verification, publication, merge, closure, and
archive for all 24 roadmap WIs. That authorization is recorded in the Contract.
Issues discovered during this WI are fixed here whenever they remain within the
Contract scope; a new WI is reserved for a distinct or materially expanded
boundary.

## Verification

- `cargo test --test reporting_domain`
- `cargo test reporting::domain`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all`
- `make check`
- `make ai-cockpit-quality GOVERNANCE_PROFILE=strict`
