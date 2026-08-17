# WI-WR-003 Top5 Weekly Read Model Implementation Plan

> **For agentic workers:** This plan is executed inline in the isolated WI worktree. Keep the Work Item Contract and Summary authoritative, and run the AI Cockpit checkpoints and Finish gates before the local commit.

**Goal:** Build a deterministic Top5 read-only collection that preserves seven supplied facts, insertion order, and candidate identity while enforcing a five-entry limit.

**Architecture:** The independent Rust module defines validated opaque text values, one immutable entry value, and one ordered collection. It is intentionally not exported through shared `mod.rs` files; the focused integration test imports it with `#[path]` so this WI does not collide with the parent composition Work Item.

**Tech Stack:** Rust 2021, standard library only, Cargo tests, AI Cockpit Make targets.

## Global Constraints

- Only `src/features/weekly_radar/domain/top5_weekly_read_model.rs`, `tests/weekly_radar_top5.rs`, dedicated docs, and this WI's lifecycle evidence may change.
- Preserve candidate, company, stage, direction, confidence, key_change, and next values exactly after blank-only validation.
- Accept at most five entries, preserve supplied order, and reject repeated candidate identity without mutating the collection.
- Do not calculate or infer Top5 membership, Stage, Direction, Confidence, Key Change, Distance, ranking, persistence, rendering, delivery, or capital action.
- If a defect remains inside this Contract, solve it in this WI and record the correction; do not create a new WI.

## Task 1: Establish the failing focused tests

**Files:**

- Create: `tests/weekly_radar_top5.rs`
- Create: `src/features/weekly_radar/domain/top5_weekly_read_model.rs`
- Create: `src/features/weekly_radar/domain/top5_weekly_read_model_test.rs`

**Interfaces:**

- The integration test imports the not-yet-implemented module with `#[path = "../src/features/weekly_radar/domain/top5_weekly_read_model.rs"]`.
- The source module loads its owned companion test only under `cfg(test)`, matching the existing coverage association without changing shared coverage policy.
- The wished-for public API is `CandidateId`, `Company`, `Stage`, `Direction`, `Confidence`, `KeyChange`, `NextStep`, `Top5Entry`, `Top5WeeklyReadModel`, and `Top5DomainError`.

- [ ] Write tests for exact seven-field retention and empty/ordered input.
- [ ] Write tests for five-entry capacity, sixth-entry rejection, duplicate rejection, and non-mutating errors.
- [ ] Run `cargo test --test weekly_radar_top5` and observe the expected missing-symbol failure before implementing production behavior.

## Task 2: Implement the minimum typed boundary

**Files:**

- Modify: `src/features/weekly_radar/domain/top5_weekly_read_model.rs`

**Interfaces:**

- Each text wrapper exposes `new` and `as_str`.
- `Top5Entry::new` validates and stores the seven wrappers.
- `Top5WeeklyReadModel::new`, `add`, `from_entries`, `entries`, `len`, and `is_empty` expose ordered collection behavior.

- [ ] Implement blank-only validation that retains accepted strings unchanged.
- [ ] Implement `Top5Entry` accessors with no derived fields.
- [ ] Implement `Top5WeeklyReadModel::add` with duplicate-before-capacity validation, then append without sorting.
- [ ] Run `cargo test --test weekly_radar_top5` and the module unit tests until green.

## Task 3: Verify boundaries and document evidence

**Files:**

- Modify: `.ai/work-items/active/wi-wr-003.summary.json`
- Modify: `docs/superpowers/specs/2026-08-17-wi-wr-003-top5-weekly-read-model.md`
- Modify: `docs/superpowers/plans/2026-08-17-wi-wr-003-top5-weekly-read-model.md`

- [ ] Run `cargo fmt -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test`/`make check`.
- [ ] Confirm `git diff --check` and inspect `git diff --name-only` against the exclusive write set.
- [ ] Update Summary with fresh command evidence, scenario coverage, guideline compliance, residual risks, and documentation alignment.
- [ ] Run `make ai-checkpoint ... STAGE=before_finish`, then `make ai-finish TASK=wi-wr-003 REPORT_LANGUAGE=zh-CN`.
- [ ] Deliver the active Outcome in the conversation before `make archive-work-item TASK=wi-wr-003`.
- [ ] Archive locally, run final quality/AI checks, and commit only this WI. Do not push, open a PR, merge, or close.
