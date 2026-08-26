<!-- AI_COCKPIT_ADAPTER_BEGIN provider=codex adapterVersion=1 repositoryId=sha256:c1643cea4f2eb905bfd6694cfdbea37a08f6b7d79ad18a4047063ccc53c2bf68 -->

This repository is attached to AI Cockpit.

Canonical interface: .ai/agent-interface.json
Read .ai/README.md before acting; read .ai/glossary.md for the repository-local Agent route and vocabulary.

Use the installed shared Rust Runtime as the repository-governance interface.
Every repository-bound command must include an explicit --repo <path>.
Prefer MCP when available; CLI remains the fallback. Do not infer AI Cockpit state from this file. Query the Runtime for current governance state.

Before editing, query inspect, status, doctor, and agent doctor. Use one bounded Work Item, branch, and worktree. Keep all edits inside the Contract scope; amend and re-run preflight before expanding it.

Contract first: intent, scope, outOfScope, sources, unknowns, acceptance criteria, verification, and authority are human-owned. For code mode, unresolved unknowns or notCodable conditions stop implementation. Do not invent intent, approval, evidence, or completion.

A preflight result of not_ready or needs_human_confirmation is a mandatory human pause. Show the humanDecisionRequest and resume condition; a successful command or yellow result is not authorization.

For authorized changes use: start or work-item new → preflight → checkpoint → verify → finish → archive → close. Keep the Summary current with changed paths and reasons, sources, verification commands/results, guideline compliance, unknowns, risk, generated/destructive changes, and observed issues.

Before archive, present a visible human Outcome with 🟢/🟡/🔴, facts, unknowns, evidence, human decision, and next action. A raw MCP record or folded-only output is not a human handoff. Close only after the merged PR, archive, decision, default-branch synchronization, clean worktrees, and exact branch removal are verified.

Canonical delivery order is latest remote default base → dedicated branch/worktree → implement → finish/archive → push → reviewed PR → merge → close → synchronize and clean. Never merge a feature branch into local main before PR review, delete its branch before merge, or let a provider auto-delete it to bypass finalization. If a remote step fails, preserve the retry checkout and identity until recovery is complete.

A terminal green Outcome is the Rust equivalent of status=completed plus humanStatusColor=green: it requires state=Verified, decisionState=green, current Contract/Summary/evidence bindings, and direct human-visible delivery. Include issue count, blockers/stopping reason, resolved issues, risks, unknowns, verification, impact, human decision, and next action; every factual claim needs evidence, and unproven benefit is an inference.

When a defect is found in the current Work Item, repair it there by amending and revalidating its Contract before opening another Work Item or Issue. A successor is allowed only for a genuinely different scope, authority, or base, an independent compatible change, an unsafe in-scope repair, immutable failed delivery, or explicit human direction.

Never edit global Agent or MCP configuration, secrets, or credentials. Do not copy V1 runtime code, Python modules, Make commands, installers, or schemas into this repository.

<!-- AI_COCKPIT_ADAPTER_END -->
