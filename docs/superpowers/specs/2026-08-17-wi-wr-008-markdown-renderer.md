# WI-WR-008 Markdown Renderer

## Goal

Provide a deterministic, complete archival Markdown boundary for one Weekly
Radar snapshot. The renderer presents explicit upstream facts in a stable
document without calculating or changing research conclusions.

## Approved approach

Use the existing Weekly Radar domain read models as the source of truth and
register them in `src/features/weekly_radar/domain/mod.rs`:

- `Top5WeeklyReadModel` supplies the ordered Top5 entries and their seven
  already-decided fields.
- `WeeklyChangeCompression` supplies the five explicit change collections and
  the stable `NO_CHANGE` output.
- `SystemHealth` supplies explicit status, freshness, coverage, degraded
  companies, source coverage, and extraction failures.
- `ResearchPacket` supplies the ordered Research Card sections and each card's
  headline, evidence, counter evidence, missing proof, and next step.
- `WeeklyRadarSnapshot` supplies immutable report identity and metadata.

The renderer is registered in the Weekly Radar Interface layer. It owns only
the two additional explicit record types required by the archival report:
`StageHistoryEntry` and `RankChange`. These records carry supplied values and
are validated for nonblank textual identity/period/company/fact fields, but
their meanings are not inferred.

The renderer accepts borrowed read models and borrowed ordered slices. It
returns an in-memory `MarkdownDocument`; it does not persist, publish, split,
retry, or send the document.

## Public boundary

The interface exposes the following documented shapes:

```rust
pub struct StageHistoryEntry {
    // stable supplied identity, period, company, previous stage,
    // current stage, and opaque history fact
}

pub struct RankChange {
    // stable supplied identity, period, company, optional previous/current
    // rank values, and opaque rank-change fact
}

pub struct MarkdownReportInput<'a> {
    snapshot: &'a WeeklyRadarSnapshot,
    top5: &'a Top5WeeklyReadModel,
    research: &'a ResearchPacket,
    compression: &'a WeeklyChangeCompression,
    stage_history: &'a [StageHistoryEntry],
    rank_changes: &'a [RankChange],
    system_health: Option<&'a SystemHealth>,
}

pub struct MarkdownDocument(String);

pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn render(input: &MarkdownReportInput<'_>) -> MarkdownDocument;
}
```

`StageHistoryEntry::new` and `RankChange::new` reject blank textual fields
with a typed `MarkdownRenderError`. Rank values are retained as supplied
optional values; the renderer never calculates a delta, direction, rank, or
membership.

## Markdown document contract

For identical immutable inputs, rendering produces byte-identical output. The
document always uses these section markers in this order:

1. `# Weekly Radar Markdown Report`
2. `## Snapshot`
3. `## Change Compression`
4. `## Top5`
5. `## Research Cards`
6. `## Evidence`
7. `## Counter Evidence`
8. `## Missing Proof`
9. `## Stage History`
10. `## Rank Changes`
11. `## System Health`

The Snapshot section prints all supplied snapshot metadata. Change Compression
prints Important Structural Change, Top5 Change, Stage Transition, Rising,
Dropped, and No Change in the upstream fixed order. Each event is emitted in
the order returned by its read model. When the upstream compression emits
`NoChange`, its `NO_CHANGE` label and all five supplied zero counts are shown;
when events exist, the renderer prints that No Change was not emitted instead
of manufacturing one.

Top5 prints every supplied `Top5Entry` in input order. Research Cards prints
the supplied executive summary and the packet's Top5, Rising, Watch, and
Dropped sections in that fixed packet order. Evidence, Counter Evidence, and
Missing Proof each visit those same card sections and card orders, exposing the
corresponding supplied field without replacing blank/unknown semantics.

Stage History and Rank Changes print every supplied entry in slice order. Rank
values are shown as `NOT_SUPPLIED` when the explicit input contains `None`; no
comparison is performed. System Health prints all supplied fields and ordered
collections. If the optional health input is absent, the section prints
`NOT_SUPPLIED` and no health state is derived.

The output contains no BUY/SELL, target-price, portfolio, capital-action, or
agent-instruction behavior. Opaque supplied facts remain data in the report.

## Data flow

```text
WeeklyRadarSnapshot ─┐
Top5WeeklyReadModel ─┤
ResearchPacket ──────┤
WeeklyChangeCompression ─┤──> MarkdownReportInput ──> MarkdownRenderer
Stage History ───────┤                                      │
Rank Changes ────────┤                                      ▼
SystemHealth ────────┘                             MarkdownDocument
```

No source performs sorting, rank/stage/distance/score calculation, section
membership detection, duplicate merging, evidence acquisition, or external
I/O. The only transformation is deterministic text formatting and conversion
of explicitly supplied enum/optional values to stable labels.

## Errors and empty states

The upstream read models remain responsible for their own validation. The
renderer validates only its newly owned Stage History and Rank Changes text
fields and returns a typed error before those records exist when a required
field is whitespace-only. Empty collections remain visible with stable empty
markers. Optional System Health absence is rendered as `NOT_SUPPLIED`; it is
not converted into Healthy, Degraded, Unavailable, or Unknown.

## Testing strategy

Module-local tests cover:

- Stage History and Rank Change text validation and exact accessors.
- Full section marker order and exact supplied values.
- Non-sorted input order for every ordered collection.
- `NO_CHANGE`, zero counts, empty history/rank collections, and absent health.
- Repeated rendering equality.
- Source-level assertions excluding sort/rank/stage/distance/score inference,
  external delivery terms, and module-registration bypasses.

Integration tests load the public Weekly Radar module tree and verify that the
registered renderer consumes the registered Top5/Compression/System Health
modules and the existing Reporting read model. They also verify that no
shared coverage policy or global architecture test is required.

## Explicit non-goals

This WI does not implement persistence, publication, Telegram/HTTP adapters,
secret or credential handling, scheduler behavior, retry, message splitting,
snapshot construction, report calculation, or any trading/capital action.
