# WI-007 Productivity Metrics Domain

## Boundary

The Productivity Domain retains comparable measured facts for a company over
opaque reporting periods. A `ProductivitySnapshot` can preserve Revenue per
Employee, Operating Income per Employee, Free Cash Flow per Employee, growth
facts, and a headcount change. `ProductivityHistory` keeps snapshots in
insertion order and rejects duplicate identities.

Each per-employee fact retains its measured value, unit, and employee
denominator. Periods, values, units, growth rates, and headcount changes remain
opaque strings at this boundary so source-specific normalization does not leak
into the Domain.

## Decisions

- Required identities, periods, values, units, denominators, growth rates, and
  headcount changes reject blank input.
- Missing metric values remain `None`; absence is not converted into zero.
- The Domain preserves supplied facts but performs no arithmetic, currency
  conversion, forecasting, peer selection, causal inference, or stage update.
- The implementation uses only the Rust standard library and imports no other
  feature module.

## No-goals

- External acquisition, source adapters, persistence, scheduling, reporting,
  Telegram delivery, ranking, scoring, or stage transitions.
- Metric normalization, arithmetic, currency conversion, forecasts, peer
  comparisons, investment conclusions, trading, price, or capital behavior.

## Authorization and issue policy

This WI is executed under the user's explicit authorization: `完成24 个WI，需要我授权的，授权给你并请写入Contract。`
The authorization is recorded in the Contract. If verification finds an issue
inside this productivity boundary, it is resolved in WI-007; a successor is
reserved for a distinct boundary or a material scope expansion.

## Verification

- `cargo test --test productivity_domain`
- `cargo test --all`
- `make check`
- `make ai-finish TASK=wi-007 REPORT_LANGUAGE=zh-CN`
