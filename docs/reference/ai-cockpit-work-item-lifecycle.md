# AI Cockpit Work Item Lifecycle

This repository uses the externally installed Rust `ai-cockpit` Runtime. The
runtime is shared across repositories, while repository protocol, Contracts,
evidence, and adapter ownership remain under `.ai/`.

## 1. Inspect and attach

Bind every command to the intended repository:

```bash
ai-cockpit inspect --repo <repo>
ai-cockpit status --repo <repo>
ai-cockpit compatibility --repo <repo>
ai-cockpit doctor --repo <repo>
ai-cockpit agent doctor --repo <repo> --json
```

For a new checkout:

```bash
ai-cockpit attach --repo <repo>
ai-cockpit profile confirm --repo <repo> \
  --program cargo --args test,--workspace
```

`attach` creates only repository-owned Runtime protocol files. Agent adapter
installation is explicit and repository-local:

```bash
ai-cockpit agent list --repo <repo>
ai-cockpit agent install --repo <repo> --provider codex
```

Do not copy Runtime source, Python modules, installers, commands, or schemas
into the repository. Do not edit global Agent or MCP configuration.

## 2. Create and authorize a Work Item

```bash
ai-cockpit work-item new --repo <repo> --id <id> --mode code
ai-cockpit start --repo <repo> --id <id> \
  --intent "<problem and constraints>" \
  --goal "<bounded goal>" \
  --scope 'docs/**' \
  --authority authorized
```

The Contract must state intent, scope, out-of-scope paths, sources, acceptance,
risk, authority, and verification. High-risk changes also declare scenario
coverage. Human-owned decisions cannot be inferred from a successful command.

## 3. Preflight and checkpoint

```bash
ai-cockpit preflight --repo <repo> \
  --contract .ai/work-items/active/<id>.contract.json
ai-cockpit checkpoint --repo <repo> --id <id>
```

`not_ready` and `needs_human_confirmation` are mandatory pauses. A yellow
result may proceed only when the Runtime identifies a safe verification action.
Contract amendments require a fresh preflight. A checkpoint is an ordered,
repository-bound lifecycle transition and must not be duplicated or bypassed.

## 4. Implement and verify

Edit only paths declared by the Contract. Run the project checks through the
Runtime or directly as declared evidence:

```bash
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args fmt,--all,--,--check --workers 1
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args clippy,--all-targets,--all-features,--,-D,warnings --workers 1
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args test,--all --workers 1
```

Verification must bind to the current Work Item and snapshot. A failure keeps
the Work Item recoverable; do not delete its records or overwrite old evidence.

## 5. Finish and human handoff

```bash
ai-cockpit finish --repo <repo> --id <id>
ai-cockpit work-item outcome --repo <repo> --id <id>
```

`finish` requires current verification, a refreshed green decision, and exactly
one checkpoint. The Outcome must be delivered visibly in the conversation and
include status, problems, stop or resolution facts, risks, unknowns, human
decision, verification, impact, and next action. A file-only or folded result
is not a delivery confirmation.

## 6. Archive and close

```bash
ai-cockpit archive --repo <repo> --id <id>
ai-cockpit close --repo <repo> --id <id> --human-decision approved
```

Archive is allowed only after the visible Outcome and terminal evidence are
valid. Close is a post-merge operation: the reviewed PR must be merged, the
default branch synchronized, and local/remote branches and worktrees verified
clean. Until then, keep the Work Item open and recoverable.

Historical records under `.ai/work-items/archive/`, recovery receipts, evidence,
and `.ai/install/` release identity are immutable context. They are not a source
for current Runtime state and must not be rewritten during a migration.
