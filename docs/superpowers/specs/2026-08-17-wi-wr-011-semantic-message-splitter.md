# WI-WR-011 Semantic Message Splitter

## Boundary

The splitter consumes one already-rendered Weekly Radar Markdown string and emits ordered, typed chunks. It recognizes only top-level rendered sections outside fenced Markdown blocks:

- Executive Summary: the rendered title and explicit No Change section;
- Important Transition: Important Structural Change and Stage Transition;
- Top5: Top5 and Threshold Distance;
- Rising/Dropped: Rising and Dropped;
- System Health: System Health.

Sections are atomic. The splitter preserves exact source text, nested Markdown, code fences, company cards, and source order. It starts a new chunk only between complete sections and returns a typed error when one section cannot fit within caller-supplied character or line limits.

## Non-goals

This WI does not render Markdown, infer No Change, calculate Stage/Ranking/Distance, access Telegram or HTTP, read secrets, publish, retry, persist, schedule, or create receipts.

## Acceptance

1. All five semantic boundaries are recognized in the WR-009 output order.
2. Headings inside fenced blocks remain content.
3. No Markdown or company card is truncated or split.
4. Unknown headings, empty input, zero limits, and unclosed fences return deterministic errors.
5. Formatting, Clippy, full tests, and strict AI Cockpit checks pass.
