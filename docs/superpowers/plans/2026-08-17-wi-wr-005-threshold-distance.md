# WI-WR-005 Threshold Distance — Execution Plan

## Controlled scope

Write only the standalone threshold source, its `#[path]` integration test,
the two dedicated documents, and WR-005 Contract/Summary/Outcome/start/archive
evidence. Do not modify shared Weekly Radar module exports, architecture
guards, or other Work Items. Do not push, open a PR, merge, or close; the
parent agent owns those later lifecycle steps.

## TDD sequence

1. Complete the v2 Contract and Summary with the explicit 24-WI authorization,
   supplied-value/no-formula decision, exclusive path set, acceptance,
   scenarios, and current-WI issue policy.
2. Run Preflight and the `before_edit` checkpoint.
3. Add focused tests first for Distance labels, fact/order retention, no
   inference, blank/empty input, duplicate evidence, and Confirmed/Missing
   overlap; observe the missing-module Red result.
4. Add the minimum standalone implementation and run the focused Green suite.
5. If the repository coverage association does not recognize the required
   integration-test filename, keep the issue in this WI by adding a legitimate
   module-local `threshold_distance_test.rs` under the same Domain source,
   registering it through `threshold_distance.rs`, and recording the Contract
   amendment; do not modify the shared coverage policy or weaken the guard.
6. Run format, Clippy, full tests, `make check`, Finish, archive, and the
   repository quality/PR checks required for a local commit.
7. Deliver the active Outcome in the conversation before archive. Commit the
   archive bundle on `codex/wi-wr-005`; leave provider operations to the parent.

## Current-WI issue policy

In-scope issues stay in WR-005. For example, a source-level validation defect,
test gap, formatting/Clippy issue, documentation mismatch, or governance
evidence ownership problem is corrected here, with the Summary and Outcome
updated. A successor is reserved for a distinct boundary or a materially
expanded scope. Shared-module interface gaps are recorded as limitations when
they are outside this exclusive write set rather than being silently changed.

## Verification evidence

- Focused: `cargo test --test weekly_radar_threshold_distance`
- Project quality: `make check`
- Governance: `make ai-preflight`, `make ai-checkpoint`, `make ai-finish`
- Archive and local PR-quality validation: `make archive-work-item`,
  `make check-ai-pr AI_BASE_COMMIT=<recorded base>`
- Scope proof: inspect `git diff --name-only` and ensure no shared module path
  is changed.
