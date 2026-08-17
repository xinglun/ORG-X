# WI-WR-001 implementation plan

1. Complete the governed Contract with the Weekly Radar boundary, dependencies, exclusions, authorization, and scenario coverage.
2. Add red tests for snapshot metadata retention, publication binding, ordered facts, duplicate rejection, and the publisher port.
3. Add the five-layer `weekly_radar` context with pure Domain metadata/publication types and the provider-agnostic Application port.
4. Register the context in the repository module and architecture checks.
5. Run focused tests, `make check`, AI Cockpit finish checks, archive evidence, commit, run `make check-ai-pr`, push, open the PR, wait for hosted checks, merge, and close the Work Item.

Out of scope: persistence, rendering, Telegram, retry, receipt, scheduling, typed weekly calculations, secrets/credential values, and trading semantics.

Current-WI issue policy: resolve defects inside this Contract in the current WI; amend the Contract and revalidate if the owned boundary must expand. Open a successor only for a distinct boundary.
