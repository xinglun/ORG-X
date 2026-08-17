# WI-WR-009 Telegram Renderer — Design Spec

## Intent and boundary

WI-WR-009 adds a provider-agnostic Weekly Radar interface boundary that turns
explicit upstream facts into one short Markdown message suitable for a later
Telegram publisher. WR-007 is the upstream source of the fixed change-section
semantics; this renderer does not recalculate or reinterpret those facts.

The renderer is responsible for deterministic section ordering, Markdown
assembly, atomic item limits, and message-size validation. It is not
responsible for acquiring evidence, calculating Stage, Ranking, Threshold
Distance, Important Structural Change, Stage Transition, Top5, Rising,
Dropped, or System Health, or for inferring No Change.

## Input model

The standalone module accepts explicit view facts so it does not import
internal modules from other bounded contexts:

- `PeriodId` identifies the supplied Weekly Radar period.
- `SummaryItem` carries an upstream identity and a complete Markdown fragment
  for Important Structural Change or Stage Transition.
- `CompanyCard` carries an upstream identity, company reference, and complete
  Markdown card body for Top5, Threshold Distance, Rising, or Dropped.
- `SystemHealthSummary` carries an explicit status label and complete Markdown
  details; the renderer never derives the status from the details.
- `NoChangeSummary` is an explicit upstream no-change fact with the stable
  `NO_CHANGE` label and supplied Markdown statement.
- `TelegramSummaryInput` groups the explicit sections and rejects blank
  values, duplicate identities, period mismatch, a missing change state, or a
  contradictory No Change plus non-empty change sections.

The renderer does not normalize Markdown, escape Markdown syntax, sort items,
or merge duplicate content. It preserves each supplied fragment as a complete
substring of the output.

## Output and ordering

`TelegramRenderer::render` returns an immutable `TelegramMessage` containing
the assembled Markdown and measured character/line counts. Non-empty sections
are emitted in this fixed order:

1. Important Structural Change
2. Stage Transition
3. Top5
4. Threshold Distance
5. Rising
6. Dropped
7. System Health
8. No Change

An explicit No Change input is rendered only when all six change collections
are empty. The renderer does not create No Change from an empty input. System
Health may accompany either an explicit No Change state or change sections.

## Length and atomicity

`TelegramRenderLimits` is supplied by the caller because this request does not
provide product-specific numeric values. It contains maximum characters,
maximum lines, maximum items per section, and maximum total company cards.
Zero limits are invalid. The renderer validates all limits before assembling a
message and returns a typed error for any violation. It never truncates a
Markdown fragment, truncates a company card, or emits a partial message.

The message is assembled from complete section blocks and then measured. A
character or line overflow returns an error with the observed and allowed
values; no alternative message is returned.

## Error handling

Constructors reject blank values and duplicate identities deterministically.
`TelegramSummaryInput` rejects period mismatch and contradictory explicit
change state. `TelegramRenderer::render` rejects item/card/character/line
limit violations before returning `TelegramMessage`. All failures are local,
typed, and provider-independent.

## Testing strategy

Module-local tests cover value validation, input-state invariants, exact
fragment retention, fixed ordering, No Change, and all limit errors. A same-
stem companion target keeps the implementation file associated with coverage.
Public integration tests cover every requested section, provider-boundary
source assertions, and the immutable output contract. Formatter, Clippy,
all Rust tests, architecture checks, and AI Cockpit verification provide the
remaining evidence.

## Explicit non-goals

This WI does not implement a full Markdown archive renderer, Telegram
Publisher, HTTP, secrets, retry, scheduling, persistence, message splitting,
receipt handling, or any domain calculation.
