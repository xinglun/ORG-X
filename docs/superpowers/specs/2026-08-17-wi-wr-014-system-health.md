# WI-WR-014 System Health Integration

## Intent

Add a provider-agnostic, typed System Health section to the Weekly Radar publication boundary. The section carries explicit health facts for later renderers and delivery adapters without deriving HealthStatus, Stage, Ranking, Top5, or Threshold Distance.

## Boundary

The Weekly Radar Domain owns only validated facts and deterministic collection invariants:

- `HealthStatus` and `Freshness` are supplied enum values.
- Evidence coverage and source coverage preserve supplied counts and percentages.
- Degraded companies and extraction failures preserve supplied opaque identities, sources, and reasons.
- Collection order is stable and duplicate identities are rejected before mutation.
- `WeeklyRadarPublication` accepts at most one supplied `SystemHealth` section.

The Domain does not import or depend on Telegram, HTTP, secrets, network clients, databases, persistence, schedulers, renderers, retry logic, or external providers. Source acquisition, extraction execution, freshness calculation, rendering, and delivery remain outside this WI.

## Acceptance

1. Explicit health facts are retained exactly, including a supplied status that does not match a derived interpretation of coverage.
2. Blank text values, percentages above 100, duplicate companies, duplicate sources, and duplicate extraction failures are rejected deterministically.
3. Publication attachment is single-assignment and does not replace an existing health section.
4. Unit, module-local, integration, architecture, and AI Cockpit checks pass.

## Verification

```text
make check
make ai-cockpit-quality GOVERNANCE_PROFILE=strict
make check-ai-coverage-guard
make check-ai-pr AI_BASE_COMMIT=<base>
```

