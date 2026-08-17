# WR-013 Weekly Scheduler Specification

## Purpose

The Weekly Radar execution sequence needs one explicit application boundary
that selects the weekly publication day. The default is Sunday, with
`day_of_week` configurable by the caller.

## Boundary

`WeeklyScheduler` belongs to the Weekly Radar Application layer. An outer
runtime supplies a `Weekday`; the scheduler returns a structured
`ScheduleDecision` containing the configured and observed weekdays.

The scheduler is deliberately pure. It does not read system time, timezone,
environment variables, or CI configuration. It does not calculate facts,
persist snapshots, render reports, publish Telegram messages, or retry a
publication. The caller remains responsible for invoking the Weekly Radar use
case only when the decision is `Due`.

## Safety invariants

- The default schedule is Sunday, one weekend day.
- A configured weekday is the only weekday that returns `Due`.
- Repeated evaluation does not consume or mutate scheduler state.
- Domain modules do not import or mention the Scheduler boundary.
- Runtime timers, cron wiring, persisted run history, and external services
  remain outside WR-013.

## Verification

Module-local and public integration tests cover the default, every configurable
weekday, structured decision values, repeated evaluation, and the Domain
boundary. No clock, network, token, chat ID, or provider is required.
