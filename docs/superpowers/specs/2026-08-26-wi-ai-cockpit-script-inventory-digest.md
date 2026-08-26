# AI Cockpit Script Inventory Digest

## Problem

The clean `main` baseline fails `tests/ai_cockpit/upgrade_feature_checklist_test.py` because the installed checklist's `scriptInventoryDigest` is stale. The test computes the expected digest from the unchanged `scripts/ai_installer_catalog.json` and 115 on-disk scripts.

## Decision

Refresh only `.ai/evidence/ai-cockpit-upgrade-feature-checklist.json` to the deterministic digest computed by the existing test. Do not modify scripts, catalog membership, test assertions, or governance policy semantics.

## Acceptance

- The focused and full Python suites pass.
- `make quality` and AI Cockpit gates pass.
- The corrective PR merges and the Work Item closes with clean local/remote state.
