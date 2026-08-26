# Switch to Installed AI Cockpit Runtime Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the repository-bound Python AI Cockpit runtime and make ORG-X use the installed `ai-cockpit` Rust Runtime through explicit repository attachment and CLI-bound governance commands.

**Architecture:** Keep only repository-local Protocol, Contract, evidence, and adapter facts under `.ai/`; keep the executable external at `/Users/sei-rinn/.local/bin/ai-cockpit`. Retain Rust quality targets and CI while removing Python governance entrypoints.

**Tech Stack:** Rust/Cargo, installed `ai-cockpit` 0.2.33 CLI, GitHub Actions, repository-local JSON/TOML governance state.

**Spec:** User request in the conversation, installed Runtime command surface, `.ai/README.md`, `.ai/glossary.md`, and repository governance rules.

## Global Constraints

- Every repository-bound Runtime command includes explicit `--repo <path>`.
- Preserve archived Work Item, evidence, decision, and knowledge records byte-for-byte.
- Do not copy Runtime implementation, Python modules, Make orchestration, installers, schemas, secrets, or global Agent/MCP configuration into the repository.
- Keep Cargo formatting, linting, and tests available; verify `cargo test --workspace` through the installed Runtime profile.
- Remove only the old Python runtime and live references; leave Rust source, Rust tests, and historical governance records intact.

---

### Task 1: Establish the Runtime-native repository context and Contract

**Files:**
- Create via Runtime: `.ai/cockpit.toml`, `.ai/project.json`, `.ai/agent-interface.json`, `.ai/decisions/profile-v1.json`, `.ai/decisions/profile-v2.json`.
- Create via Runtime: `.ai/work-items/active/wi-switch-installed-ai-cockpit.contract.json` and `.ai/work-items/active/wi-switch-installed-ai-cockpit.summary.json`.
- Create: `docs/superpowers/plans/2026-08-27-switch-installed-ai-cockpit.md`.
- Update: the Contract with human-owned intent, exact scope, exclusions, acceptance, sources, risk, authority, and verification.

**Interfaces:**
- Consumes: installed Runtime 0.2.33, `origin/main`, base `4d36b13c437db4cfaac9181ac2ce8f4ad3d63c5c`, and user authorization for an isolated worktree.
- Produces: a Runtime-native Contract that binds the old runtime removal, replacements, preservation rules, and verification.

- [ ] Attach and calibrate the repository:

```bash
ai-cockpit attach --repo "$PWD"
ai-cockpit profile confirm --repo "$PWD" --program cargo --args test,--workspace
```

Expected: Protocol v1/schema v2 is attached and the Cargo test profile is `calibrated`.

- [ ] Create the Runtime-native Work Item skeleton:

```bash
ai-cockpit work-item new --repo "$PWD" --id wi-switch-installed-ai-cockpit --mode code
```

Expected: a `not_ready` skeleton without invented intent or authority.

- [ ] Complete the Contract, including `intent`, exact old runtime paths, preservation of archived evidence, the stale-manifest warning, and scenario coverage for clean removal, historical preservation, CLI operation, CI, and Rust quality checks.

- [ ] Run and pass:

```bash
ai-cockpit preflight --repo "$PWD" --contract .ai/work-items/active/wi-switch-installed-ai-cockpit.contract.json
```

### Task 2: Replace live instructions, entrypoints, and CI references

**Files:**
- Modify via Runtime: `AGENTS.md` with the explicit `codex` adapter; update `CLAUDE.md` and `GEMINI.md` to the installed-runtime route while preserving unrelated content.
- Modify: `.ai/README.md`, `.ai/cockpit/README.md`, and live governance documentation to use explicit installed CLI commands.
- Modify: `Makefile` to retain only Rust targets and remove `include Makefile.ai`.
- Delete: `Makefile.ai`, `Makefile.ai.stack`, `pyproject.toml`.
- Modify: `.github/workflows/ai-cockpit.yml` so it does not invoke Python governance scripts or `make ai-*` targets.

**Interfaces:**
- Consumes: the attached Protocol and calibrated profile from Task 1.
- Produces: live instructions and CI with no Python AI Cockpit dependency.

- [ ] Install the repository-local adapter:

```bash
ai-cockpit agent install --repo "$PWD" --provider codex
```

Expected: `.ai/adapters/codex.json` and a marked managed section in `AGENTS.md`; no global configuration writes.

- [ ] Remove obsolete Make/Python entrypoints and rewrite the root Makefile to keep `fmt-check`, `clippy`, `test`, and `check`.

- [ ] Update live docs and CI to use `ai-cockpit --repo "$PWD"` plus Cargo checks; retain historical records without rewriting their original commands.

- [ ] Prove no live reference remains:

```bash
rg -n --hidden -g '!.git/**' -g '!target/**' -g '!.ai/work-items/archive/**' -g '!.ai/work-items/recovery-receipts/**' -g '!.ai/work-items/conflict-successor-receipts/**' -g '!.worktrees/**' 'Makefile\.ai|scripts/ai_|make (ai|check-ai)|PYTHON|python3|pyproject\.toml'
```

Expected: no live project-runtime or Python-governance references.

### Task 3: Remove old runtime code and validate installed Runtime operation

**Files:**
- Delete: all old Python runtime files under `scripts/` and Python-only tests under `tests/**/*.py`, as listed in the Contract.
- Preserve: `.ai/work-items/archive/**`, `.ai/work-items/recovery-receipts/**`, `.ai/work-items/conflict-successor-receipts/**`, and Rust tests.
- Generate: current Runtime verification evidence through installed CLI commands only.

**Interfaces:**
- Consumes: the exact removal scope and replacements from Task 2.
- Produces: no bound Python implementation, an attached/calibrated installed Runtime, and current verification evidence.

- [ ] Check identity and state:

```bash
ai-cockpit --version
ai-cockpit inspect --repo "$PWD"
ai-cockpit status --repo "$PWD"
ai-cockpit compatibility --repo "$PWD"
ai-cockpit doctor --repo "$PWD"
ai-cockpit agent doctor --repo "$PWD" --json
```

Expected: Runtime 0.2.33, `COMPATIBLE`, attached repository, and conflict-free Codex adapter.

- [ ] Record fresh checkpoint and verification:

```bash
ai-cockpit checkpoint --repo "$PWD" --id wi-switch-installed-ai-cockpit
ai-cockpit verify --repo "$PWD" --work-item wi-switch-installed-ai-cockpit --command cargo --args test,--workspace --workers 1
ai-cockpit verify --repo "$PWD" --work-item wi-switch-installed-ai-cockpit --command cargo --args fmt,--all,--,--check --workers 1
ai-cockpit verify --repo "$PWD" --work-item wi-switch-installed-ai-cockpit --command cargo --args clippy,--all-targets,--all-features,--,--deny,warnings --workers 1
ai-cockpit work-item validate --repo "$PWD" --id wi-switch-installed-ai-cockpit --json
```

- [ ] Finish and deliver the visible Outcome:

```bash
ai-cockpit finish --repo "$PWD" --id wi-switch-installed-ai-cockpit
ai-cockpit work-item outcome --repo "$PWD" --id wi-switch-installed-ai-cockpit
```

Expected: `Outcome: 🟢` only if every acceptance and evidence requirement passes; otherwise retain the reported recovery condition.

- [ ] Run final diff and state checks:

```bash
git diff --check
git status --short
ai-cockpit inspect --repo "$PWD"
ai-cockpit status --repo "$PWD"
ai-cockpit doctor --repo "$PWD"
```
