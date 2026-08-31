# Telegram reference-model heading splitter repair

## Goal

Restore safe Weekly Radar delivery after the report renderer added the AI-era
reference model validation section in three report languages.

## Design

1. Add the Chinese, Japanese, and English section headings to the existing
   `JudgmentReference` semantic boundary in the splitter.
2. Add one regression test that runs all three headings through the public
   splitter and checks both the boundary and source preservation.
3. Run the focused regression and repository quality checks. The production
   workflow, renderer, transport, credentials, and unknown-heading rejection
   remain unchanged.

## Verification

- The new regression fails before the alias mapping is added with
  `UnknownSection`.
- The focused splitter test passes after the mapping is added.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and `cargo test --all` pass before delivery.
- After the reviewed PR is merged, manually rerun the authorized production
  workflow for `2026-08-31` and verify the successful publication evidence.
