# Installed AI Cockpit Runtime

ORG-X is governed by the externally installed Rust `ai-cockpit` Runtime. This
directory no longer contains the former Python/Make runtime.

## Repository-bound commands

Every command must bind this repository explicitly:

```bash
ai-cockpit inspect --repo <repo>
ai-cockpit status --repo <repo>
ai-cockpit compatibility --repo <repo>
ai-cockpit doctor --repo <repo>
ai-cockpit agent doctor --repo <repo> --json
```

For a new checkout, attach and calibrate the repository-owned protocol:

```bash
ai-cockpit attach --repo <repo>
ai-cockpit profile confirm --repo <repo> --program cargo --args test,--workspace
```

Agent adapter installation is explicit and repository-local. It does not modify
home-directory configuration:

```bash
ai-cockpit agent list --repo <repo>
ai-cockpit agent install --repo <repo> --provider codex
```

## Work Item lifecycle

```bash
ai-cockpit work-item new --repo <repo> --id <id> --mode code
ai-cockpit start --repo <repo> --id <id> --intent "..." --goal "..." \
  --scope 'src/**' --authority authorized
ai-cockpit preflight --repo <repo> \
  --contract .ai/work-items/active/<id>.contract.json
ai-cockpit checkpoint --repo <repo> --id <id>
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args test,--workspace --workers 1
ai-cockpit finish --repo <repo> --id <id>
ai-cockpit work-item outcome --repo <repo> --id <id>
ai-cockpit archive --repo <repo> --id <id>
```

Preflight is evidence-derived. `not_ready` and `needs_human_confirmation` are
mandatory pauses; a yellow result can proceed only when its safe action is the
declared verification step. `finish` requires current verification evidence,
a green refreshed decision, and exactly one checkpoint. Surface the human
Outcome in the conversation before archive. Close only after the reviewed PR
is merged and the default branch is synchronized.

Historical V1 records under `.ai/work-items/archive/`, recovery receipts, and
install evidence remain immutable context. They are not current Runtime state
and must not be rewritten.
