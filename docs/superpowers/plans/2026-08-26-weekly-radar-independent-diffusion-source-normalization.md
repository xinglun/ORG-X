# Implementation Plan: Independent Diffusion Source Normalization

1. RED: add fixtures for PwC site-specific publication metadata, the explicit Atos press URL, and independent title/body named-adopter extraction.
2. GREEN: implement the bounded metadata allowlist, `agentic-ai` document classification, independent title context, and bounded adoption-verb extraction in Rust.
3. Update both Weekly Radar registries to replace the inaccessible NIQ URL with the explicit Atos Group disclosure; keep supplier URLs unchanged.
4. Run targeted tests, then Rust format/clippy/tests, Python tests, and all AI Cockpit gates. Repair any in-scope failure before finish.
5. Run `ai-finish`, `check-ai-pr`, hosted CI, merge, `ai-close-work-item`, and verify clean local/remote state.
6. Trigger the documented local dry-run from the merged `main`; inspect Confirmed/Candidate status, source roles, missing proof, and Ranking output without publishing or archiving.
