# SEC Company Facts Response Limit Implementation Plan

## Goal

Restore the SEC Company Facts evidence path observed to fail in the latest
hosted Weekly Radar run, without weakening the generic response-size guard for
other sources.

## Evidence

- `origin/data:weekly-radar/reports/2026-08-18.md` reports `SEC` coverage as
  `0 available, 10 unavailable`.
- `origin/data:weekly-radar/snapshots/2026-08-18.json` records ten failures with
  `HTTP response body exceeded configured limit`.
- `src/features/weekly_radar/runtime/http.rs` currently applies a 1 MiB limit
  to every transport request.
- `src/features/weekly_radar/runtime/sec.rs` requests Company Facts through that
  generic path.

## Scope and design

1. Add a documented, finite per-request body-limit capability to the runtime
   HTTP boundary while keeping `HttpClient::get` on the existing 1 MiB default.
2. Make `FixtureHttpClient` and `UreqHttpClient` enforce the supplied limit with
   bounded reads and the existing secret-safe `RuntimeError`.
3. Give SEC Company Facts a source-specific limit sized for the observed public
   payload class; submissions and filing requests keep the default limit.
4. Add fixture and local HTTP regression coverage for a valid payload above 1
   MiB, the SEC boundary, the default boundary, and User-Agent propagation.
5. Document the distinction in the operations guide.

## Verification

- Focused SEC/runtime tests, including the large-payload regression.
- `make quality`.
- `make ai-finish TASK=wi-sec-company-facts-response-limit REPORT_LANGUAGE=zh-CN`.
- `make archive-work-item TASK=wi-sec-company-facts-response-limit`.
- `make check-ai-pr AI_BASE_COMMIT=82d57672c2f6130620be4a73380a0a2aaf02b03f`.
- Hosted PR checks, merge, `make ai-close-work-item`, and final clean-state
  audit.

## Explicit non-goals

- No change to the company registry, source selection, report ranking, Stage
  logic, Telegram behavior, workflow schedule, dependencies, or historical
  `origin/data` files.
- No live provider request is required to prove the code-level boundary; the
  hosted run is retained as the motivating production evidence.
