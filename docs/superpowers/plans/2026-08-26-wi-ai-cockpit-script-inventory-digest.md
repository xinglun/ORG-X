# AI Cockpit Script Inventory Digest Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the generated AI Cockpit checklist digest to the value verified by the unchanged installer catalog and on-disk scripts.

**Architecture:** Treat `scripts/ai_installer_catalog.json` plus the script bytes as the source of truth. Update only the installed evidence projection, then run the existing Python test and repository gates.

**Tech Stack:** Python pytest, JSON evidence projection, Make AI Cockpit quality and lifecycle gates.

**Spec:** `docs/superpowers/specs/2026-08-26-wi-ai-cockpit-script-inventory-digest.md`

## Global Constraints

- Do not change scripts, catalog contents, tests, or policy semantics.
- The executable test remains the authority for the expected digest.
- Keep the corrective Work Item independent from Weekly Radar evidence extraction.

### Task 1: Refresh and verify the generated digest

**Files:**
- Modify: `.ai/evidence/ai-cockpit-upgrade-feature-checklist.json`
- Test: `tests/ai_cockpit/upgrade_feature_checklist_test.py`

- [ ] **Step 1: Run the focused test to reproduce RED**

```bash
PYTHONDONTWRITEBYTECODE=1 pytest -q tests/ai_cockpit/upgrade_feature_checklist_test.py
```

Expected: the inventory test fails only on `scriptInventoryDigest`.

- [ ] **Step 2: Update only the installed digest**

Set `installerCatalog.installed.scriptInventoryDigest` to the digest printed by the test failure (`9730f66ff9fa5158a07e6f461479f69bd5c20acef518184fbed7e43544e0caf3`) and leave catalog/script/test bytes unchanged.

- [ ] **Step 3: Run the focused test to verify GREEN**

Run the same command and expect 2 passed.

### Task 2: Complete governed verification

- [ ] **Step 1:** Run `pytest -q`, `make quality`, all Contract checks, and `make ai-finish ... REPORT_LANGUAGE=zh-CN`.
- [ ] **Step 2:** Archive, run `check-ai-pr`, push, wait for CI, merge, and run `ai-close-work-item`.
- [ ] **Step 3:** Verify the Weekly Radar Work Item can be synchronized to the new `main` and rerun its Python/Shell/merged-main acceptance.
