# AI Cockpit repository usage

This repository uses one externally installed `ai-cockpit` Runtime. The binary
is shared; this `.ai/` directory is private to this repository. Never infer
current repository or Work Item state from process state, the working directory,
or Agent prose.

## Agent route

1. Inspect the repository with `ai-cockpit inspect --repo <repository>` and
   `ai-cockpit status --repo <repository>`.
2. Confirm readiness with `ai-cockpit doctor --repo <repository>` and
   `ai-cockpit agent doctor --repo <repository> --json`.
3. For a new repository, use `ai-cockpit attach --repo <repository>`.
4. Agent discovery is explicit and repository-local:
   `ai-cockpit agent list/install/repair/detach --repo <repository> --provider <provider>`.
5. Create a Work Item with `ai-cockpit work-item new --repo <repository> --id
   <id> --mode code`, then provide human-owned intent, scope, acceptance, and
   authority before implementation.
6. For an authorized Work Item, use `start → preflight → checkpoint → verify →
   finish → archive → close`. Every command carries `--repo`.

The Runtime has no global active Work Item, current repository, or project
profile. Repository protocol, Contract, evidence, knowledge, and adapter
ownership records remain isolated under this repository's `.ai/`.

## Evidence discipline

Do not claim `green`, `passed`, `approved`, `verified`, or `completed` from this
file. Query the Runtime and read current repository evidence. Missing, stale,
contradictory, or unknown evidence requires a rerun, human decision, or stop.

Generated status, receipt, and archive files are written by Runtime commands;
do not hand-edit them. Do not copy V1 Python runtime code, Make commands,
installers, or schemas into this repository. Do not edit global Agent or MCP
configuration.

The visible human Outcome is a terminal handoff. It must retain its
`Outcome: 🟢`, `Outcome: 🟡`, or `Outcome: 🔴` marker, unknowns, evidence,
decision, and next action. A missing, stale, contradictory, or malformed
Outcome does not authorize finish, archive, merge, close, or release.
