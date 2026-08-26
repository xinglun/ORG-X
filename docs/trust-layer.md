---
title: "AI Cockpit Trust Boundary"
description: Repository-local evidence and lifecycle boundaries for the installed AI Cockpit Runtime.
keywords: [ai-cockpit, trust, evidence, human-decision]
---

# AI Cockpit Trust Boundary

ORG-X uses an externally installed Rust `ai-cockpit` Runtime. The Runtime is a
repository-bound governance interface; it is not a security sandbox, an Agent
Runtime, a proof of provider identity, or a substitute for hosted review and
branch protection.

## Evidence over self-declaration

The Runtime binds a human request to a Work Item Contract, evaluates scope and
authority, records verification, and exposes the next safe action. Agent prose,
confidence, a repository approval field, or a successful command alone is not
independent authorization. Missing, stale, contradictory, or high-risk evidence
returns control to a human or stops the lifecycle.

The trust boundary is composed of separate facts:

1. Git history binds repository ancestry and the Work Item base revision.
2. Contract and scope bind the requested change and allowed paths.
3. Runtime receipts bind preflight, checkpoint, verification, finish, and archive
   to the Contract and current snapshot.
4. Hosted review, branch protection, release checks, identity, signatures, SBOM,
   and provenance remain external facts when they are required.

The Runtime can record and validate these bindings when evidence exists; it does
not manufacture a missing provider, security, release, or enterprise fact.

## Repository-local lifecycle

```text
human request → Contract → preflight → checkpoint → change
→ Runtime verification → human Outcome → finish/archive → reviewed close
```

Use the installed binary explicitly:

```bash
ai-cockpit inspect --repo <repo>
ai-cockpit status --repo <repo>
ai-cockpit doctor --repo <repo>
ai-cockpit preflight --repo <repo> \
  --contract .ai/work-items/active/<id>.contract.json
ai-cockpit verify --repo <repo> --work-item <id> \
  --command cargo --args test,--workspace --workers 1
ai-cockpit finish --repo <repo> --id <id>
ai-cockpit work-item outcome --repo <repo> --id <id>
```

Every command must include `--repo`. Generated records are written by the
Runtime and must not be hand-edited. A visible `Outcome: 🟢`, `Outcome: 🟡`, or
`Outcome: 🔴` is a human handoff, not merely a file path. A green Outcome still
requires current verification, a matching Contract/Summary, and a valid
repository state.

## Boundaries and history

The installed Runtime owns current protocol state under `.ai/`. Historical V1
archives, recovery receipts, evidence, and release/install records remain
immutable context and are not rewritten during a migration. Global Agent, MCP,
shell, and Python configuration are outside the repository boundary and must
not be modified by repository governance.
