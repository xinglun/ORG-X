# Runtime Source URL and Redirect Safety Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enforce a fail-closed URL baseline for configured Weekly Radar sources and disable automatic redirects on source and Telegram HTTP agents.

**Architecture:** Keep URL validation at the runtime configuration boundary, before source adapters receive configured URLs. Keep redirect behavior inside infrastructure transports; adapters continue to classify non-success responses without interpreting redirect destinations.

**Tech Stack:** Rust 2021, `url`, `ureq`, `serde`, existing Rust integration tests, AI Cockpit Work Item governance.

**Spec:** `docs/superpowers/specs/2026-08-20-wi-runtime-http-source-safety.md`

## Global Constraints

- Only configured source URL validation and redirect behavior change.
- No DNS resolution, full host allowlist, proxy policy, retry/backoff, Stage, score, ranking, universe, or production-operation changes.
- Validation errors must not retain or display complete URLs, credentials, queries, or fragments.
- Fixture transports remain permissive because they are injected test doubles; production source URLs are validated by `CompanyConfig` before collection.
- All changes remain inside the declared Contract scope and complete the full AI Cockpit lifecycle.

---

### Task 1: Bind the governed scope and source-of-truth design

**Files:**
- Modify: `.ai/work-items/active/wi-runtime-http-source-safety.contract.json`
- Modify: `.ai/work-items/active/wi-runtime-http-source-safety.summary.json`
- Create: `docs/superpowers/specs/2026-08-20-wi-runtime-http-source-safety.md`
- Create: `docs/superpowers/plans/2026-08-20-wi-runtime-http-source-safety.md`

**Interfaces:**
- Consumes: current `main` at `1feefd5cf1b4e2cc50adba0c844277678ebce4e4`, runtime source/config/http/Telegram code, and the approved bounded design.
- Produces: a ready Contract with explicit scope, exclusions, scenarios, acceptance, risks, and verification.

- [x] **Step 1: Record the current base and no-dirty-path baseline**

Run: `git status --short --branch` and `git rev-parse HEAD`.

Expected: clean dedicated branch at the recorded base commit.

- [x] **Step 2: Write the design and lifecycle Contract**

The Contract records the explicit user authorization for repository changes, PR creation, hosted checks, merge, archive, and close; it excludes production operations and product-judgment policy.

- [ ] **Step 3: Run preflight and the before-edit checkpoint**

Run: `make ai-preflight CONTRACT=.ai/work-items/active/wi-runtime-http-source-safety.contract.json` and then `make ai-checkpoint CONTRACT=.ai/work-items/active/wi-runtime-http-source-safety.contract.json SUMMARY=.ai/work-items/active/wi-runtime-http-source-safety.summary.json STAGE=before_edit`.

Expected: preflight `ready`; checkpoint records the final Contract hash and required checks.

### Task 2: Add failing URL-policy regression tests

**Files:**
- Modify: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Consumes: `CompanyConfig::new`, `CompanySourceRegistry::from_json`, `UreqHttpClient`, and a local redirect fixture.
- Produces: executable proof for malformed/unsafe configured URLs, secret-safe errors, and no-follow redirect behavior.

- [ ] **Step 1: Add URL validation tests first**

Add tests that assert configuration rejects credentials, fragments, malformed URLs, `localhost`, local-only suffixes, loopback/private/link-local IP literals, and accepts the existing public registry shape plus `example.test` fixture URLs.

- [ ] **Step 2: Run the focused tests and verify the expected RED failures**

Run: `cargo test --test weekly_radar_runtime configured_source_urls -- --nocapture`.

Expected: the new tests fail because the current prefix-only validator accepts the unsafe cases.

- [ ] **Step 3: Add a redirect regression test**

Use a local TCP listener whose first response is `302 Location: /final`; assert `UreqHttpClient` returns the 302 response and the server observes only one request.

- [ ] **Step 4: Run the redirect test and verify the expected RED failure**

Run: `cargo test --test weekly_radar_runtime ureq_does_not_follow_redirects -- --nocapture`.

Expected: the test fails because `ureq` currently follows its default redirect budget.

### Task 3: Implement the minimum production safety boundary

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `src/features/weekly_radar/runtime/config.rs`
- Modify: `src/features/weekly_radar/runtime/http.rs`
- Modify: `src/features/weekly_radar/runtime/telegram.rs`
- Modify: `tests/weekly_radar_runtime.rs`

**Interfaces:**
- Consumes: failing tests from Task 2.
- Produces: `CompanyConfig` URL validation and redirect-disabled source/Telegram agents, with existing runtime APIs preserved.

- [ ] **Step 1: Add the direct `url` dependency**

Add `url = "2.5"` to `[dependencies]` so the runtime owns the URL parser it uses rather than depending on a transitive crate.

- [ ] **Step 2: Implement secret-safe URL validation**

Replace the prefix-only check with parsed scheme/host/credential/fragment checks and local/private host checks. Return `RuntimeError::InvalidConfiguration` with only the field name and a generic reason.

- [ ] **Step 3: Disable redirects on both production agents**

Set `.redirects(0)` on the source `ureq::AgentBuilder` and the Telegram `AgentBuilder`; do not alter timeout, body-size, payload, or retry settings.

- [ ] **Step 4: Run focused tests and verify GREEN**

Run: `cargo test --test weekly_radar_runtime configured_source_urls ureq_does_not_follow_redirects -- --nocapture`.

Expected: all new tests pass, including secret-safe error assertions.

### Task 4: Complete documentation and repository verification

**Files:**
- Modify: `docs/superpowers/specs/2026-08-17-wi-wr-016-runtime.md`
- Modify: `.ai/evidence/reference-impact/wi-runtime-http-source-safety-config.json`
- Modify: `.ai/evidence/reference-impact/wi-runtime-http-source-safety-cargo.json`
- Modify: `.ai/evidence/reference-impact/wi-runtime-http-source-safety-lock.json`
- Modify: `.ai/evidence/reference-impact/wi-runtime-http-source-safety-http.json`
- Modify: `.ai/evidence/reference-impact/wi-runtime-http-source-safety-telegram.json`
- Modify: `.ai/work-items/active/wi-runtime-http-source-safety.summary.json`

**Interfaces:**
- Consumes: implemented code and focused test evidence.
- Produces: documentation alignment, reference-impact evidence, and a verification-ready Summary.

- [ ] **Step 1: Document the URL and redirect boundary**

Update the runtime specification with the configured-source URL policy, redirect behavior, and explicit DNS/allowlist non-goals.

- [ ] **Step 2: Record reference-impact evidence**

Add one record for each changed runtime production file family, explicitly documenting no external provider or dynamic reference impact beyond the existing runtime boundary.

- [ ] **Step 3: Run focused, architecture, quality, and governance checks**

Run: `cargo fmt --check`, focused runtime tests, `cargo test --all`, `make quality`, `make check-ai-coverage-guard`, `make check-ai-reference-impact`, and every Contract-required AI Cockpit check.

- [ ] **Step 4: Update Summary only from command evidence**

Record exact commands, results, warnings, residual DNS/allowlist risk, and review readiness; do not claim real external provider behavior was exercised.

### Task 5: Finish and close the governed Work Item

**Files:**
- Modify: `.ai/work-items/active/wi-runtime-http-source-safety.summary.json`
- Generated: `.ai/cockpit/current_status.md`, `.ai/cockpit/task_report.json`, `.ai/cockpit/task_report.md`
- Generated/archived: `.ai/work-items/archive/**`

- [ ] **Step 1: Run before-finish checkpoint and `make ai-finish`**
- [ ] **Step 2: Archive the Contract, Summary, and Outcome**
- [ ] **Step 3: Commit, run `make check-ai-pr`, push one branch, and create one PR**
- [ ] **Step 4: Wait for every hosted check, request code review, and merge without provider-side branch deletion**
- [ ] **Step 5: Run `make ai-close-work-item TASK=wi-runtime-http-source-safety`**
- [ ] **Step 6: Remove the exact closed worktree and audit main/branches/remotes/archive/active state**

## Verification Matrix

- URL config rejects unsafe forms without leaking URL values.
- Existing public registry and fixture URLs remain accepted.
- Source and Telegram agents return 3xx responses instead of following them.
- Response-size, timeout, source parsing, and Telegram retry behavior remain green.
- Full Rust quality/test suite passes.
- AI Cockpit finish, archive, hosted PR checks, merge, close, and final clean-state evidence pass.

## Review Focus

- Confirm the policy is applied at the actual configured-source boundary and not only in tests.
- Confirm redirect disabling does not alter payloads, timeout budgets, or retry semantics.
- Confirm DNS rebinding and complete host allowlist remain explicit residual risks rather than being overclaimed as solved.
