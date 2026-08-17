# WI-WR-006 Rising / Dropped Implementation Plan

## 1. Governed setup

- Start from `origin/main` at `c90349ff42b8a87ba6146bbd62bc9e357951786c`.
- Keep all changes within the Contract's exclusive source, test, docs, and
  Work Item evidence paths.
- Run the canonical `before_edit` checkpoint after the Contract is ready.

## 2. TDD implementation

- Add `tests/weekly_radar_rising_dropped.rs` using a path import so the shared
  `weekly_radar/domain/mod.rs` remains unchanged.
- Write failing tests for strengthened → Rising, weakened/invalidated →
  Dropped, non-Top5 context, price/rank/score-only no-event behavior,
  unchanged no-event behavior, fact order, proof overlap, and identity
  conflicts.
- Implement the minimum standalone domain model in
  `src/features/weekly_radar/domain/rising_dropped.rs`.
- Run focused tests, then all project tests and formatter/linter checks.

## 3. Evidence and governance

- Add reference-impact JSON proving no shared module or cross-feature import is
  required.
- Update Summary with changed files, scenario evidence, checks, risks, and
  current-WI issue resolution.
- Run the required `before_finish` checkpoint and `make ai-finish TASK=wi-wr-006
  REPORT_LANGUAGE=zh-CN`.
- Deliver the active Outcome in the conversation before archive.
- Run `make archive-work-item TASK=wi-wr-006`, `make check`, and the local AI
  quality gates. Commit the complete archive bundle only.

## 4. Handoff boundary

Stop after local commit. Do not push, open a PR, merge, or run
`make ai-close-work-item`; the parent agent owns those provider and closure
actions.
