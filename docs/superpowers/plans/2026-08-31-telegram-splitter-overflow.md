# Telegram semantic splitter overflow repair

## Goal

Restore Weekly Radar delivery when a valid rendered report section is longer
than Telegram's per-message limits but is composed of complete level-three
entries.

## Design

1. Keep the existing top-level heading aliases and fail-closed unknown-heading
   behavior.
2. When a top-level section exceeds the caller's character or line limit, split
   only at complete level-three headings outside fenced Markdown blocks.
3. Preserve each source slice byte-for-byte, retain the section's semantic
   boundary for every slice, pack adjacent slices only when both limits allow,
   and keep an oversized individual entry as a hard error.
4. Add regression coverage for the production-shaped reference-model section,
   then run the focused test and repository quality gate.

## Verification

- The new regression fails before the nested-entry fallback with
  `AtomicSectionTooLarge`.
- The focused splitter test proves all chunks stay within both limits and
  concatenation reproduces the original Markdown exactly.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test --locked --all` pass.
- After the reviewed PR is merged, rerun the authorized non-dry-run workflow
  for `2026-08-31` and verify the Telegram/data publication evidence.
