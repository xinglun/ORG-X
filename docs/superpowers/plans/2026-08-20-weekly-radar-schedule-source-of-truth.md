# Weekly Radar Schedule Source-of-Truth Plan

## Goal

Align the reusable Weekly Radar scheduler default with the already authoritative
production schedule: Monday at 09:00 JST (`0 0 * * 1` UTC).

## Steps

1. Record the current split: library/tests/spec use Sunday while workflow and
   operations use Monday.
2. Add or update regression coverage so the default is Monday and explicit
   configured weekdays retain their existing behavior.
3. Change only the scheduler default and all declared schedule documentation;
   preserve the pure application boundary and production workflow file.
4. Run focused scheduler/end-to-end tests, then the full AI Cockpit and project
   quality checks.
5. Finish, archive, commit, `check-ai-pr`, push, hosted-check, merge, close,
   and final clean-state verification.

## Verification

- `make ai-preflight`
- `make check-ai-contract`
- `make ai-checkpoint ... STAGE=before_edit`
- focused scheduler and end-to-end tests
- `make ai-finish TASK=wi-weekly-radar-schedule-source-of-truth REPORT_LANGUAGE=zh-CN`
- `make archive-work-item TASK=wi-weekly-radar-schedule-source-of-truth`
- `make check-ai-pr AI_BASE_COMMIT=48aa5396563bc80d8ec7af8c1cc2e72dcf79e2f8`
- hosted `ai-cockpit-quality`, `check-ai-pr`, and `task-list-completed`
- `make ai-close-work-item TASK=wi-weekly-radar-schedule-source-of-truth`
- final `make quality` and branch/archive audits
