# WI-002 Universe Domain Design

## Goal

Implement the first pure Universe bounded-context contract for ORG-X. The domain
will represent company identity, security identity, listings, a supplied
universe snapshot, and deterministic eligibility over facts already provided by
an outer layer.

## Scope

In scope:

- `Company`, `Security`, `Listing`, `UniverseSnapshot`, and `EligibilityPolicy`.
- Strong identity/value types with explicit validation for empty values.
- Common-equity and supported-exchange facts needed by the MVP filter.
- Snapshot referential-integrity validation and deterministic de-duplicated
  eligible-security output.
- Focused tests for positive and negative eligibility paths.

Out of scope:

- Provider clients, network calls, CSV parsing, databases, ingestion receipts,
  and external data acquisition.
- Evidence extraction, scoring, ranking, stage transitions, reporting, and any
  trading or capital-action decision.
- Application, infrastructure, interface, or ACL behavior beyond retaining the
  existing five-layer boundary markers.

## Domain rules

1. A `Company`, `Security`, `Listing`, or `SnapshotId` cannot be created with an
   empty identity value; a company name and ticker also cannot be empty.
2. A `Security` must reference a company present in the snapshot.
3. A `Listing` must reference a security present in the snapshot.
4. Eligibility requires all of:
   - the instrument is common equity;
   - the listing is active;
   - the exchange is NYSE or Nasdaq;
   - the security is a member of S&P 500 or Nasdaq 100.
5. Index membership is a fact supplied to the policy. The policy does not fetch,
   infer, or refresh membership.
6. A security represented in both supported indexes is returned once, ordered by
   stable security identity rather than input ordering.

## Proposed Rust surface

The Domain module will expose documented types and constructors using only the
Rust standard library:

- `CompanyId`, `SecurityId`, `ListingId`, and `SnapshotId` as validated identity
  values.
- `Company` with an immutable identity and legal name.
- `InstrumentType`, `Security`, and `Listing` for the security and listing facts.
- `UniverseIndex` and `IndexMembership` for supplied index facts.
- `EligibilityFacts` and `EligibilityPolicy` for a pure deterministic rule.
- `UniverseSnapshot` for validated aggregate input and de-duplicated output.
- `UniverseDomainError` for invalid values and broken references.

The API will return `Result` for construction and snapshot validation. It will
not expose mutable collections or provider-specific fields.

## Alternatives rejected

- A provider-backed repository was rejected because ingestion belongs to another
  bounded context and would violate the Domain dependency rule.
- A generic score or ranking field was rejected because Stage precedes Ranking
  and Universe only answers eligibility.
- A trading status such as `READY` or `NO_TRADE` was rejected because capital
  actions are outside ORG-X.
- A date/time dependency was rejected for this first slice because snapshot
  identity is sufficient; temporal semantics can be introduced by a later
  Contract with an explicit requirement.

## Verification

- Focused Universe integration tests cover eligible and rejected facts,
  duplicate memberships, invalid identities, and invalid references.
- Existing architecture tests must continue to prove Domain has no provider or
  infrastructure dependency.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all`, `make check`, and `make ai-cockpit-quality
  GOVERNANCE_PROFILE=strict` must pass.
