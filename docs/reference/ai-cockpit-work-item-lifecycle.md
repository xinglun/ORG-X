---
author: Ray
title: AI Cockpit Work Item Lifecycle
description: Isolated Work Item lifecycle, agent-orchestrated parallelism, budget, and release-evidence rules.
---

# AI Cockpit Work Item Lifecycle

The default execution unit is one Work Item, one dedicated branch, and one PR.
Within one Work Item its lifecycle is serial and evidence-bound:

```text
remote base → dedicated branch → Contract/Preflight → implement → ai-finish/archive
  → push → PR/review → merge → ai-close-work-item → synchronize and clean base
  → close and clean that Work Item
```

## Outcome diagnostic boundary

Task Outcome is the canonical lifecycle decision record, not merely a prose
summary. Newly generated records carry `humanStatusColor`: `green` only for a
completed Outcome, `yellow` for warning or human-confirmation states, and
`red` for blocked or cancelled states. A blocked Outcome additionally requires
the structured `failedGate` and an actionable `recoveryCondition`; the
canonical Markdown renders all three fields. Validators reject missing or
contradictory fields for new generator versions, so a blocked delivery cannot
be displayed as green. Historical archived Outcomes remain read-compatible and
are never rewritten solely to add these fields.

An agent or subagent may orchestrate multiple independent Work Items in parallel
when each has a separate linked worktree, `codex/<work-item-id>` branch,
Contract/Summary pair, PR, archive, and closure receipt. AI Cockpit does not
schedule those tasks and a Work Item never becomes a shared global runtime
lock. It validates the local record shape and fails closed for malformed,
unpaired, mismatched, or non-dedicated active branches.

### Template quality-shard worktrees

The template repository's own `project-test-shard-%` targets use isolated,
temporary Git worktrees because its pytest shards write generated governance
evidence. `scripts/quality_shard_workspace.py` serializes only the short
Git-common-directory operations (`worktree list`, `add`, and `remove`) with a
cross-process lock. Each `make` invocation also uses a unique parent-process
run directory, so an interrupted invocation cannot share a worktree path with
a later retry. Evidence copying, regeneration, shard execution, and artifact publication remain parallel. A lifecycle failure names the shard and
phase on stderr; if both the runner and cleanup fail, the runner exit status is
preserved and the cleanup failure is also reported.

This is a template-repository quality implementation, not an automatic claim
that every installed adopter uses pytest sharding. The installed `Makefile.ai`
executes the adopter-defined `PROJECT_TEST` command. Adding an adopter-facing,
stack-neutral parallel-test runtime requires its own Contract, installer
catalog delivery, and fresh-adopter parity evidence; it must not be inferred
from this template-internal helper.

### Branch-integrated generated projections

Active Contract, Summary, Outcome, and Start Receipt files are task-namespaced
worktree evidence. In contrast, `.ai/cockpit/current_status.md`, both
`.ai/cockpit/task_report.*` projections, and `.ai/work-items/archive/index.json`
are shared branch-integrated facts. An active status is therefore provisional:
it cannot establish archive or PR ownership by itself.

When a linked Work Item is active, `ai-start` requires a candidate
`concurrencyBoundary` that declares the complete serialized-projection
inventory plus exclusive implementation, generated-evidence, and quality paths.
The Start Receipt binds the boundary digest. Finish and archive acquire one
persistent projection lease, require the latest `origin/main`, and hold it
through merged-PR lifecycle closure. A successor must rebase after the prior
owner closes before it can regenerate shared projections.

For a start request with a different Work Item ID, a malformed foreign
linked-worktree identity is isolated rather than becoming a global start
deadlock. The foreign Work Item is never edited or cleaned automatically;
starting that same Work Item remains fail-closed until its owner repairs the
identity. Run `make ai-doctor` to see the isolated identity and its recovery
boundary before deciding on a corrective Work Item.

`make ai-doctor` is the single read-only diagnostic surface for installation
and lifecycle facts. It presents immutable version/commit/tag/digest evidence,
available `ai-*` targets, Outcome color and recovery, and hosted snapshot
readiness together. If facts conflict, it reports the conflict and recovery
instead of selecting a preferred value; it never performs provider polling or
changes a Work Item, Outcome, snapshot, branch, or installation record.

When a Work Item is a declared successor, it must wait for its predecessor to
have evidence for PR merge, archive, local/remote branch deletion, and base
synchronization. `make check-ai-serial-order` fails closed when that declared
dependency is incomplete; independent Work Items do not acquire that edge.

Create the dedicated Work Item branch before running `make ai-start`.
When one remote default branch can be identified, `ai-start` rejects execution
from that base branch before it writes a Contract, Summary, Start Receipt,
Cockpit Status, or task event. Repositories without one discoverable remote
default branch retain fixture/bootstrap behavior, but that absence is not
evidence that the current branch is safe.

## Controlled corrective route during live calibration

An `in_progress` or `paused` calibration Session blocks an ordinary
`ai-start`: calibration readiness cannot be bypassed by creating a routine
Work Item. When that Session itself exposes a template or workflow defect, use
the explicit, bounded corrective declaration instead:

```text
make ai-start TASK=<task> MODE=code \
  AI_START_CALIBRATION_CORRECTIVE='<JSON declaration>'
```

The declaration has schema version `1` and must contain exactly
`sessionPath`, `sessionId`, `sessionState`, `sessionDigest`, `findingId`,
`findingSummary`, `authority`, `repairPaths`, and `resumeCondition`. Its
Session identity, live state, and SHA-256 digest must match
`.ai/calibration/session.json` byte-for-byte. `repairPaths` must be unique,
repository-relative, Contract-scoped paths and may not change the Session or
activation state. The complete declaration is persisted in the Contract; its
canonical JSON digest is bound into the immutable Start Receipt and checked
again by active Contract validation. A missing, changed, inactive, or
unreadable Session therefore fails closed after start as well as before it.

The generated Cockpit Status shows this route as yellow. It records the
Session, finding, repair paths, and resume condition, but is neither
calibration completion nor authority to activate calibration, archive, merge,
release, or skip the normal Work Item lifecycle. Complete the corrective Work
Item, then resume calibration only through the Session workflow.

## Pre-finish hosted verification snapshot

### Active-evidence commit boundary

Active Contract, Summary, Start Receipt, and any generated active Outcome are
dedicated Work Item evidence. They are ordinary branch content, not ignored
local state: include them with the same dedicated-branch commit as the
implementation candidate. This makes their snapshot digests reproducible
without `git add -f`.

When Finish starts, it records the exact active Contract and Summary in its
Change Summary automatically, so retry and archive validation retain the same
evidence boundary without hand-written duplicate entries.

Diff ownership recognizes only the exact active Work Item's Contract, Summary,
and Outcome as its intrinsic evidence. A different Work Item's active evidence
remains unowned unless its own Contract covers it.

The snapshot command remains read-only with respect to Git: it never stages,
commits, pushes, or selects unrelated files. It still rejects every dirty or
untracked path. Resolve unrelated changes before measuring; do not weaken the
clean-tree boundary to accommodate active evidence.

Some performance or environment-specific acceptance criteria require hosted
execution from a committed source before the final Summary can truthfully
report completion. For only that case, the active Contract must explicitly
require hosted verification and register pending `hostedPerformanceEvidence`.
After local implementation and verification, create a local snapshot commit
with explicit human authorization and run:

```text
make ai-prepare-hosted-verification-snapshot \
  CONTRACT=.ai/work-items/active/<task>.contract.json
```

The validator reruns local quality, binds a receipt to the branch, base,
commit, tree, active Contract and Summary, and confirms that Git refs and the
worktree were not mutated. The receipt identifies only pushing that exact
branch for hosted measurement as eligible and provides no human authorization.
It is not review readiness and cannot authorize a PR,
merge, tag, release, archive mutation, closure, or branch deletion. Release

For the hosted execution, dispatch `smoke.yml` with `purpose=hosted_measurement`
and the snapshot branch. A successful `ci-evidence` job uploads exactly one
`hosted-measurement-receipt-<run-id>-<attempt>` artifact. Its JSON schema records
the repository, workflow/run URL and identity, ref, exact `commitSha`, required
job names and conclusions, and artifact name. Copy its facts into the active
Summary only after independently confirming that its `commitSha` equals the
snapshot receipt. The hosted receipt is evidence, not authority: it cannot
authorize a PR, merge, release, archive mutation, closure, or branch deletion.
intent, an archived Work Item, complete hosted evidence, a dirty/detached/base
state, baseline mismatch, or failed quality stops the stage. Once hosted
results are recorded in the active Summary, the Work Item must return to the
full `ai-finish`/archive → final push → PR → merge → `ai-close-work-item` →
cleanup lifecycle.

## Resume after a corrective predecessor

## Record a successor or quarantine route

When an active Work Item already has a red blocked Outcome and an authorized
corrective successor, do not write a receipt by hand. Record the limited route
with `make ai-transition-to-successor PREDECESSOR_TASK=<blocked-task>
SUCCESSOR_TASK=<new-task> SUCCESSOR_BRANCH=codex/<new-task>
SUCCESSOR_BASE=<base-sha> ISSUE=https://github.com/<owner>/<repo>/issues/<n>
AUTHORITY='<recorded human authority>' MODE=quarantined REASON='<why>'`.
The command validates the blocked Outcome, exact identities, same-repository
Issue, authority, mode, and receipt location. Status/doctor show a yellow route
while the predecessor Outcome remains red. It never authorizes archive, merge,
release, branch deletion, provider mutation, or predecessor evidence rewrite.
During `ai-start`, a valid quarantined receipt admits its named successor only
when the requested task, dedicated branch, and current base commit all match
the receipt. A valid or invalid receipt remains fail-closed for its predecessor
and named successor, but cannot block an unrelated Work Item from starting. A
malformed or stale bound-task receipt, or a branch/base mismatch, fails before
lifecycle writes; this narrow bootstrap exception does
not enable general concurrent startup.

## Retry-versus-successor boundary

### Resolve current-Work-Item problems in place

When implementation, verification, finish, or handoff discovers a problem,
the default is to repair it in the current Work Item. This is allowed when
the current Contract still covers the scope, authority, and base: amend the
Contract before adding paths or authority, revalidate it, preserve the retry
evidence, and keep the blocked Outcome visible. Do not create another Work
Item or Issue merely to avoid the repair or to expand the work.

Create a successor or independent Work Item only when the scope, authority, or
base genuinely differs, the change is genuinely independent, safe in-scope
resolution is impossible, immutable failed-delivery evidence requires a new
delivery, or a human explicitly directs it. The reason and predecessor/linkage
must be recorded in the new Contract and Start Receipt. This boundary applies
to the template repository and to every adopter repository receiving the
installed AI Cockpit rules.

A blocked active Work Item retries in place when it retains the same active
Contract and scope and needs only an active schema/evidence correction. Preserve
the blocked Outcome, append the correction, and rerun the affected gates. Do
not create another Issue or successor for a missing Summary field or missing
`before_finish` checkpoint.

A governed successor/quarantine is required only if the delivery must start
from a changed base, the Contract/scope is invalidated, or immutable
failed-delivery evidence requires an independently re-delivered change. The
successor route retains, rather than rewrites, predecessor evidence.

Checkpoint recovery is stage-specific. A missing `before_finish` record is
recovered with `make ai-checkpoint CONTRACT=<contract> SUMMARY=<summary>
STAGE=before_finish`. A stale immutable `before_edit` Contract binding is
recovered only through append-only
`make ai-revalidate-contract-amendment`; it must never be replaced by a second
`before_edit` record.

When a process defect pauses a Work Item, complete and close the corrective
Work Item first. Rebase the paused dedicated branch onto the latest discovered
remote default branch, replace its `predecessorWorkItem` with that corrective's
closed evidence, and run:

```text
make ai-resume-work-item CONTRACT=<active-contract> \
  BASE_REMOTE=<remote> BASE_BRANCH=<default-branch>
```

This is the supported baseline transition when a completed corrective predecessor
is the source of the new baseline. The
writer preserves the immutable Start Receipt, verifies the exact remote ref,
Git ancestry, original dedicated branch, predecessor merge identity, closure
postconditions, archive-manifest identity and digests, then atomically appends
one `resumeHistory` edge and advances Contract `baseCommit`. Repeated corrective
cycles append edges; they do not rewrite prior edges. Direct edits, broken
chains, or incomplete closure evidence fail closed. Afterward, rerun Preflight
and every verification made stale by the new baseline.

Version-1 Start Receipts created before the dedicated-branch start guard may
truthfully record the remote default branch in `baseBranch`. They are not
rewritten. A bounded compatibility resume accepts such evidence only when the
requested base branch equals the Receipt value, the current non-base branch is
exactly `codex/<work-item-id>`, every resume edge retains that same work branch,
and all ordinary ancestry, predecessor, archive-manifest,
digest, and closure checks pass. This recovery is not the normal start path and
does not permit new Work Items to start on the default branch.

## Synchronize an active Work Item to current main

When an active dedicated Work Item is behind the live remote default branch but
does not have a completed corrective predecessor, do not run Git rebase by
hand. First fetch the target so the local tracking ref equals the live remote
head, then run:

```text
make ai-synchronize-work-item CONTRACT=<active-contract> \
  BASE_REMOTE=<remote> BASE_BRANCH=<default-branch> \
  TARGET_ROOT=<target-worktree-root>
```

The command has local-only authority: it verifies the immutable Start Receipt,
active Contract/Summary pair, dedicated branch identity, tracking-ref freshness,
and ancestry. A clean Work Item rebases directly. A dirty Work Item may proceed
only with an explicit Contract `synchronizationCheckpoint` authorization and
only when every dirty path is Contract-owned or a governed generated artifact;
it first creates a recoverable local checkpoint and records its identity and
paths in the digest-bound `synchronizationHistory` transition. It never pushes,
force-pushes, opens a PR, changes provider state, rewrites the Start Receipt,
or changes archive evidence. A conflict is automatically aborted before any
active evidence write. A successful synchronization marks prior verification
`not_run`; rerun Preflight and all required current-generation checks before
Finish. Replay, stale tracking state, detached/base/foreign branches, dirty
worktrees, unrelated histories, and malformed evidence fail closed.

## Conflict successor after an automatic synchronization abort

Do not resolve a proven synchronization conflict manually. Create a clean,
dedicated successor from the live default branch through normal `ai-start`,
then bind it without modifying the preserved source worktree:

```text
make ai-transition-conflict-successor SOURCE_ROOT=<source-worktree> \
  SOURCE_CONTRACT=<source-contract> SUCCESSOR_CONTRACT=<successor-contract> \
  BASE_REMOTE=<remote> BASE_BRANCH=<default-branch> ISSUE=<issue-url> \
  AUTHORITY='<recorded human authority>' REASON='<conflict recovery reason>'
```

The successor-only receipt binds both Start Receipts, source Contract/Summary/
blocked Outcome digests, source checkpoint HEAD, and the re-proven live target
base. It never authorizes manual rebase, merge, release, archive mutation,
source-branch deletion, or provider action.

`TARGET_ROOT` is optional when the target is the current checkout; it is
required for a caller acting on a distinct target worktree. The runtime treats
that root as the sole source for Contract/Summary resolution, Git operations,
validation, and evidence writes, and never reads caller active evidence as a
fallback.

## Contract readiness

Active v2 code Contracts must contain concrete problem, constraints, rationale, sources, acceptance, and verification content. Generic starter phrases are rejected by the Contract check before implementation. If Preflight reports `needs_human_confirmation` or `not_ready`, stop and report the reason; do not continue by treating advisory output as authorization.

For a code Contract that declares an `implementationSurface`, list every
planned repository-relative path under `production`, `tests`, `generated`, or
`documentation`. Every path must be Contract-owned, must not match
`outOfScope`, and a non-empty `production` list requires a non-empty `tests`
list. A declared Makefile, guard, workflow, or other restricted path also
requires a complete `restrictedWriteApproval`. `Implementation Surface` is a
first-class Preflight signal: malformed, unowned, excluded, forbidden, or
unapproved paths make `make ai-preflight` and therefore
`make ai-prepare-implementation` fail before the immutable `before_edit`
checkpoint. `ai-start` may still create and display an incomplete skeleton; it
does not authorize editing until the Contract becomes ready.

### Contract amendment revalidation

`before_edit` proves the phase boundary at which implementation was first
authorized. It is immutable evidence: do not rerun
`make ai-prepare-implementation` after that record exists, and do not edit the
Summary to replace it. If a legitimate scope or Contract amendment is needed,
first amend the Contract through the governed review path, then run:

```text
make ai-revalidate-contract-amendment \
  CONTRACT=.ai/work-items/active/<task>.contract.json \
  SUMMARY=.ai/work-items/active/<task>.summary.json \
  PREVIOUS_CONTRACT_HASH=<immutable-before-edit-hash> \
  AMENDMENT_REASON='<why the Contract changed>'
```

The command appends a `contract_amendment_revalidation` checkpoint that binds
the original `before_edit` hash, the preceding Contract hash, the amended
Contract hash, the reason, and whether required verification had already
started. If verification had started, the revalidation must additionally
invalidate every required gate and record the prior passed-gate count; Finish
then reruns the full required gate set for the amended Contract. Missing,
malformed, stale, or cross-Contract revalidation evidence fails closed. The
record does not authorize a merge,
release, provider mutation, or a manual rewrite of lifecycle evidence.

## Complexity budget

Before implementation, estimate expected changes in the Contract's `budgetImpact`. At finish, `make check-ai-budget-impact` compares the generated complexity report with `.ai/guards/governance_complexity_policy.yaml`. An overrun is permitted only when the Contract explicitly records approval, a repayment Work Item, and repayment records. A separate budget-repair Work Item/PR is the appropriate repayment path when the current Work Item cannot repay its own increase.

## Release evidence states

Release evidence uses three distinct states:

- Historical: an existing archived Work Item or prior release record; preserve it as evidence and do not rewrite it.
- Candidate: a release commit/tag and its generated artifacts are prepared, but publication and source binding are not yet proven.
- Published: the public tag, source commit, release assets, checksums, SBOM, provenance, and release-state checks all point to the same source-bound release.

Do not report a candidate as published. `check-release-distribution` remains the source-bound verification for public release evidence.

## Closure rule

Only after the PR is merged and the Work Item is archived may `make ai-close-work-item TASK=<task>` run. The command owns branch deletion and must fail closed on any lifecycle mismatch. After closure, verify the local base equals the remote base and only then begin the next serial Work Item.
## Preflight hard gates before PR and release

After `make ai-finish TASK=<task> REPORT_LANGUAGE=<conversation-locale>` archives the Work Item, commit the complete
Work Item bundle, then run `make check-ai-pr AI_BASE_COMMIT=<latest-default-branch-sha>`.
Do not run the aggregate PR check against an uncommitted archive or generated
release evidence. Independent review must finish while evidence is active. The order is:

```text
independent review → ai-finish/archive → commit bundle → check-ai-pr → push → PR
```

Before reporting a successful `ai-finish` result—whether archive is requested
inline or as the later explicit `archive-work-item` step—Finish runs
`make check-changed-critical-coverage AI_BASE_COMMIT=<Contract baseCommit>`.
The resulting report binds the immutable Contract base, candidate HEAD, a
content-addressed candidate tree digest, and a binary diff digest. The
candidate includes committed, staged, unstaged, deleted, renamed, and
untracked Contract-owned delivery paths without creating an ordinary commit
before archive. It rejects an unowned dirty path instead of silently absorbing
another Work Item's changes.

Derived active lifecycle projections (the same Work Item's Summary, Outcome,
status, and Human Benefit Report) are excluded from both the candidate tree and
the Summary worktree digest to avoid a self-reference cycle: a successful gate
must be recorded in those files after the snapshot. Their bytes remain
independently bound by the Outcome and immutable archive manifest. A later
explicit archive re-computes the candidate
immediately before mutation, requires the matching report and Outcome binding,
and records the report digest plus candidate binding in the manifest. A
missing, stale, mismatched, or failing result produces a blocked active Outcome
and denies archive. This guard complements, rather than replaces, the clean
committed `check-ai-pr` gate.

An archive retry may reuse a prior `ai-finish` verification only when its final
`aiSummary` attestation binds both the unchanged Work Item state and the
Summary inputs consumed to derive the Task Outcome and Human Benefit Report.
If any such input changes—for example `knownGaps`, changed-file evidence,
verification evidence, non-risk explanations, or recorded human decisions—the
retry reruns the normal Finish verification and regenerates the derived Outcome
and report. Do not delete or hand-edit derived artifacts to turn a yellow or
red Outcome green.

The installer-created `adopt_ai_cockpit` Contract is the only bounded
not-applicable case: its explicit `adoptionBootstrapPaths` identify template
runtime files whose mapped template tests are intentionally not copied into an
adopter. The command records the Contract-bound applicability result and still
fails closed for malformed adoption metadata, ordinary Work Items, and missing
ordinary mappings. Its generated report remains local state and must not alter
the archive worktree digest.

If `check-ai-pr` discovers a changed-critical-coverage or archive-evidence
failure only after archive, do not rewrite its Contract, Summary, Outcome, or
manifest and do not create a duplicate Work Item solely for that repair. Start
the narrow same-Work-Item route from the clean committed candidate:

```sh
make ai-open-post-archive-recovery \
  TASK=<task> AI_BASE_COMMIT=<merge-base-sha> \
  ISSUE=<repository-issue-url> AUTHORITY='<recorded human authority>' \
  RECOVERY_PATHS='scripts/example.py tests/test_example.py'
```

The command first reproduces the failing aggregate PR audit, then writes one
append-only receipt binding the archive digests, PR base, failure output,
authority, Issue, and finite repair paths. `check-ai-pr` independently
revalidates it and grants ownership only to those paths. The receipt never
authorizes archive rewrite, merge, release, branch deletion, closure, or new
scope. Use a successor only when the Contract/base/scope itself is invalid or
a new delivery is required.

When the failure occurred only in hosted GitHub Actions aggregate coverage and
the local audit passes, use the same target with all hosted arguments together:

```sh
make ai-open-post-archive-recovery \
  TASK=<task> AI_BASE_COMMIT=<merge-base-sha> \
  ISSUE=https://github.com/<owner>/<repo>/issues/<number> \
  AUTHORITY='<recorded human authority>' RECOVERY_PATHS='tests/test_example.py' \
  HOSTED_REPOSITORY=<owner>/<repo> HOSTED_PULL_REQUEST=<number> \
  HOSTED_CANDIDATE_HEAD=<40-character-sha> HOSTED_RUN_ID=<run-id> \
  HOSTED_JOB_ID=<template-smoke-job-id>
```

The runtime and `check-ai-pr` each query GitHub again. They require the exact
repository, pull request, candidate Head, pull-request workflow event,
completed failed `template-smoke` job, and the canonical `pytest-cov`
below-floor line in that job's log. Missing authentication, unavailable logs,
reruns, changed provider facts, a successful or non-coverage failure, a
different base, or a changed archive fails closed. Do not supply a copied log,
write a receipt by hand, reduce the coverage floor, or add a patch directly to
the archived candidate.

Before the `before_finish` checkpoint, complete the current v2 Summary's
`documentationAlignment` record. It must cover the plan,
Contract/Summary evidence relationship, documentation/commands/capability
language, multilingual semantics, and limitations/unknowns/history. Every
aligned evidence path must exist in the repository and also be declared by
`changedFiles` or `sourcesUsed`; changed documentation and command surfaces
must be recoverable in the opposite direction from that evidence. An
unreviewed, incomplete, misaligned, machine-local, missing, or undeclared
record blocks Finish.

Documentation alignment is a close-out evidence map, not a capability claim or
a replacement for tests, hosted evidence, provider controls, or the Capability
Truth Matrix. Historical archived Summaries that predate the field remain
immutable and readable; they are not backfilled.

During the current Work Item's archive transaction, exact active artifact paths
in `documentationAlignment` are migrated to their durable archive locations.
Execution-time evidence such as recorded commands and `executionContractPath`
remains unchanged because it describes the actual check context rather than a
current resolvable documentation reference.

When that transaction refreshes the current Human Benefit Report, the JSON and
Markdown files are admitted only as an exact pair. Both files must be currently
changed, named by the archived Summary, and validate against the one currently
changed archived `completed` Task Outcome for the same Work Item. A missing,
stale, malformed, cross-task, non-completed, or historical-only report remains
outside transaction ownership and blocks pre-merge validation.

The same complete transaction is the only no-active ownership projection used
by `check-ai-diff-ownership` during `ai-pre-merge`. Its archive index, start
receipt, manifest, Contract, Summary, and every Summary-declared changed path
are accepted together only after all bindings validate. This does not exempt
other historical evidence or incomplete transactions: those paths remain
unowned and fail closed.

The same archive transaction loads the registered instruction-traceability
manifest before moving any active artifact. If that JSON is malformed, archive
fails closed without moving the Contract or Summary. Every value exactly equal
to the current active Contract path is migrated to the generated archive
Contract path; lookalike paths, command strings that merely contain the path,
unrelated Work Items, and historical archive paths are not rewritten. A
manifest with no exact reference is a byte-for-byte no-op. When a rewrite is
needed, the archived Summary owns that generated change. If any later archive,
index, manifest, or status step fails, the transaction restores the original
active artifacts and traceability bytes before reporting failure.

This gate first runs the project formatter and, when the governance script and policy
are installed, the governance complexity/budget check; only then does it validate PR
ownership. This catches formatting drift and budget overflow before remote CI.
The PR must contain exactly one newly maintained Work Item and must be based on the
latest remote default branch; a branch derived from another unmerged Work Item is
invalid even when its tests pass.

When CI or PR checks block a change, pause before retrying. Perform a process-root-
cause review for missing preflight gates, wrong ordering, late formatter or budget
checks, template/adopter boundary errors, and source-bound evidence design. If the
failure is preventable in the workflow, open and complete a corrective Work Item
that adds an executable fail-closed gate before resuming the original operation.

Hosted quality failures must preserve their diagnostic payload before runner
teardown. A workflow that buffers individual Gate output may keep heartbeat
notices for liveness, but on failure it must also emit every non-passing Gate's
durable log. If the wrapper exits before per-Gate timing is written, it must emit
the wrapper log as the fallback. Timing metadata without the exact failing output
is not sufficient evidence for root-cause analysis.

Before release evidence is generated, run
`make finalize-release-freeze-premerge TASK=<task>` on the dedicated Work Item
branch after `ai-finish` has archived the Work Item and before committing the
release metadata. This is the only supported premerge freeze writer for a release
preparation PR: it requires the archived Work Item evidence, a clean branch, and
source-bound candidate metadata. Its canonical `sourceTree` and `archiveSha256`
are calculated from the clean candidate branch `HEAD`. The controlled
`SOURCE_COMMIT` reference is retained separately so the hosted release workflow
can resolve the exact merged default-branch identity. Both `.ai/work-items/active` and
`.ai/work-items/archive` are export-ignored, so moving evidence during Finish does
not change canonical content.

Do not run `make check-release-preflight` on the premerge metadata commit. That
commit carries the candidate freeze records but is not the release source identity;
the exact-source gate would correctly reject it. The gate runs only after runtime
freeze on the exact merged `SOURCE_COMMIT`, in the hosted detached checkout. After
merge, that checkout must regenerate the same tree and archive or stop before tag
mutation. `make check-release-preflight RELEASE_PREFLIGHT_SOURCE_COMMIT="$SOURCE_COMMIT"`
then fails closed when lifecycle evidence is absent or inconsistent, archive policy
blocks, or regenerated content differs.

```json
{
  "state": "frozen",
  "sourceTree": "<exact-default-branch-tree-sha>",
  "archiveSha256": "<regenerated-canonical-archive-sha256>",
  "lifecycle": {
    "state": "closed_and_synchronized",
    "command": "make ai-close-work-item",
    "baseCommit": "<exact-default-branch-tree-sha>",
    "worktreeClean": true
  }
}
```

Historical premerge markers and release metadata remain preparation evidence;
they do not authorize publication of a later source tree. If a later correction
changes included source bytes, preserve the old record and follow the readiness
and rehearsal sequence below rather than mutating or repeatedly regenerating it.

## Release readiness and exact-source rehearsal

For new release attempts, the committed premerge marker is historical preparation
evidence, not the authority for a later default-branch source tree. Run
`make check-release-readiness` normally runs only after there are no active Work
Items. It checks stable candidate, policy, archive-growth, and mandatory Japanese evidence,
but deliberately does not compare a historical `release-freeze.json` to current
source bytes.

One narrow exception prevents a lifecycle deadlock: the single active Work Item
that performs the public release may carry this readiness evidence when its
Contract declares the exact `repository_release.publish` operation, has
identity-bound user authorization, and is explicitly allowed to continue. Any
ordinary, additional, malformed, or unauthorized active Work Item remains a
hard blocker. This exception does not authorize a tag or Release; the same-SHA
rehearsal and actual release gates remain mandatory.

The required sequence is: synchronized default branch → repository readiness →
successful same-SHA rehearsal → actual hosted release. The rehearsal uses the
same exact-source checkout, runtime freeze, strict preflight, locked dependency,
required-CI, supply-chain-evidence, and strict-smoke path as publication. It
creates a private Actions receipt, never a tag, GitHub Release, or public asset.
It is not a published release.

The actual hosted release receives the rehearsal run id, resolves the default
branch again, and rejects a missing, failed, wrong-workflow, wrong-SHA, or
wrong-tag receipt before runtime finalization or immutable mutation. The gate
still runs only after runtime freeze on the exact merged `SOURCE_COMMIT`; exact
archive, digest, installer, and identity checks remain mandatory at that boundary.

The receipt is a result, not a cache hit. It may replace a second strict-smoke
dispatch only when the workflow rechecks its source SHA, Git tree digest,
release tag, strict-smoke workflow/run/job conclusions, collection and shard
receipt, coverage and source set, provider artifact digests, integrity digest,
and unexpired validity window. Any missing, cancelled, failed, stale, or
mismatched field fails closed before tag creation; runtime freeze, preflight,
release evidence, Draft asset verification, and Quick Install still run.

A later included-source change invalidates the rehearsal SHA and requires a new
same-SHA rehearsal, not another committed freeze. If exact-source validation
fails after a successful rehearsal, stop, preserve diagnostics, and open a
corrective Work Item; do not create a new freeze Work Item as a substitute for
root-cause repair.

Template and adopter boundary: template-maintenance branches use the template
repository's `project-format-check` and governance policy from the latest template
default branch. An installed adopter uses its own configured formatter, remote
default branch, base commit, and governance policy; it must not copy the template's
absolute line or archive budgets.

## External handoff and receipt

When a Work Item needs hosted CI, provider release, human confirmation, or adopter execution, create a versioned handoff bound to the Work Item, branch, HEAD, tree, Contract and Summary digests, action, fulfiller, receipt kind, and deadline. The visible state is `awaiting_external_receipt` (yellow). Do not poll or claim external completion.

Only a receipt with every matching binding and the declared fulfiller/kind may resolve the handoff. On expiry, project `blocked` (red), preserve the recovery condition, and create a new bound handoff if the external action is still required. Timeout alone never resumes a Work Item.
