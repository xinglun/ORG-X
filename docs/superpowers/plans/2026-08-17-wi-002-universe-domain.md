# WI-002 Universe Domain Implementation Plan

> For the implementation agent: follow the Work Item Contract and keep the
> pure-domain boundary intact. Execute this plan in the dedicated `codex/wi-002`
> worktree.

**Goal:** Implement and verify the minimal deterministic Universe Domain model.

**Architecture:** Keep all business behavior in `src/features/universe/domain`.
Export the domain module through the existing Universe context. Use standard
library collections only. Verify behavior through `tests/universe_domain.rs`
and the existing architecture tests.

## Task 1: Lock the Contract and design evidence

1. Validate the WI-002 Contract and Summary with `make ai-preflight`.
2. Record the required `before_edit` checkpoint.
3. Self-review the design spec for scope, assumptions, non-goals, and
   testability.

## Task 2: Write failing domain tests first

1. Add tests for valid construction of Company, Security, Listing, and
   SnapshotId.
2. Add tests for rejection of empty identities/names/tickers.
3. Add tests for policy acceptance and each rejection rule.
4. Add tests for snapshot reference validation and membership deduplication.
5. Run the focused test target and confirm it fails for the missing Domain API.

## Task 3: Implement the smallest pure Domain

1. Add documented validated identity types and domain error variants.
2. Add immutable Company, Security, Listing, and index-membership facts.
3. Add EligibilityPolicy and deterministic EligibilityFacts evaluation.
4. Add UniverseSnapshot construction, referential-integrity validation, and
   stable de-duplicated eligible-security output.
5. Export the Domain module without touching other bounded contexts or adding
   dependencies.

## Task 4: Verify, finish, archive, and publish

1. Run focused tests, architecture tests, formatting, clippy, all tests, and
   strict AI Cockpit quality.
2. Update Summary scenario evidence, guideline compliance, residual risks, and
   documentation alignment.
3. Record the `before_finish` checkpoint and run non-archive `ai-finish`.
4. Directly report the Outcome, archive the Work Item, commit intentionally,
   run `check-ai-pr`, push, create a PR, inspect hosted checks, merge, and run
   `ai-close-work-item`.
