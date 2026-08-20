# Weekly Radar Archive Transaction and Recovery Specification

## Goal

Make a delivered Weekly Radar archive logically committed and recoverable across the existing report, rendered snapshot, receipt, and compatibility manifest paths without changing those public paths or claiming a filesystem transaction across independent files.

## Current gap

`runtime/archive.rs` currently writes each final file through its own sibling temporary file and rename. That protects an individual file, but a process interruption between renames can leave only part of a date's final set. The next retry then sees an existing final file and cannot distinguish a committed run from an incomplete archive. Retrying after a successful Telegram delivery can also send the same report a second time.

## Design

Each new archive run has a unique hidden transaction directory under `weekly-radar/.transactions/`. The directory contains the fully serialized report, rendered snapshot, receipt, and compatibility manifest before any public final path is promoted. A date-keyed transaction record is written with `state: prepared` after staging. Public paths are then promoted one at a time using the existing per-file atomic write primitive. The same record is rewritten with `state: committed` only after all public artifacts match the staged bytes. That committed record is the logical visibility/commit point.

The transaction record contains only generated relative paths, the date, the staged artifact digests, the manifest, and the transaction state. Recovery accepts a prepared record only when every staged artifact is present and matches its recorded digest, and it validates all staged bytes plus all existing date-specific public bytes before promoting any new public file. It fills absent public files, verifies existing public files byte-for-byte, updates the compatibility manifest only when its current manifest is not newer than the prepared date, and writes the committed record last. The compatibility manifest is a global latest-date pointer rather than a historical artifact digest; committed older transactions remain valid after a newer date updates it. It reuses the already persisted delivery receipt; recovery never calls Telegram.

If a prepared record is malformed, a staged or final artifact differs, a committed record no longer matches its public files, or a partial final set exists without a valid transaction record, the runtime returns IncompleteRun and does not overwrite or delete the conflicting bytes. A per-date Unix file lock covers the CLI from recovery/duplicate checks through delivery and archive commit; a separate commit lock covers direct archive API calls. Complete archives written by older versions without a transaction record remain legacy committed runs when all three date-specific final files exist and therefore still produce ExistingRun.

## Public behavior

- `write_run` and `write_run_with_input_snapshot` keep their signatures and existing public paths.
- `ensure_run_available` recognizes a valid committed transaction or a complete legacy final set as an existing run; it rejects prepared, corrupted, and partial states with `IncompleteRun`.
- `recover_pending_run` is a documented public recovery entrypoint. It returns `Some(manifest)` only when it completes a prepared transaction and `None` when no recovery is pending. A valid committed run is left for the existing duplicate guard.
- The CLI checks pending recovery and same-date availability before acquisition for non-dry runs. Retry and normal rerun therefore do not send Telegram again when a prepared archive can be completed locally.
- Retry checks recovery before loading the input snapshot, so a complete prepared transaction can be recovered from its staged bytes even if the input envelope is unavailable.
- Dry-run remains non-mutating and does not recover or create archive files.
- Retention runs only after the logical commit record is durable. Transaction directories are not retention data and are not date-prefixed final files.

## Failure and recovery boundary

The design provides logical transactional visibility and deterministic recovery, not physical multi-file atomicity. A crash can still leave intermediate public files, but those files are either completed from matching staged bytes or reported as an explicit incomplete state. No provider behavior, real Telegram run, workflow execution, DNS policy, or product Stage/Score/Ranking semantics are introduced by this change.

## Acceptance evidence

1. Failure-injection tests cover preparation and every public promotion stage; no failure is reported as a committed run and existing bytes are not silently overwritten.
2. Recovery tests complete a prepared transaction from staged bytes, preserve the delivery receipt, and prove no second transport call is needed.
3. Malformed/mismatched transaction and partial legacy residue fail closed with `IncompleteRun`; complete legacy archives remain `ExistingRun`.
4. Existing input snapshot, report identity, data-branch, retention, dry-run, and public API tests remain green.
5. Operations documentation states the recovery command/behavior and the remaining external-provider verification boundary.
