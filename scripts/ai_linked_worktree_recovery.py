"""Diagnose a recoverable foreign linked-Work-Item duplicate without mutation."""

from __future__ import annotations

import argparse
import json

import ai_start


def report(task: str) -> tuple[int, dict[str, object]]:
    """Return a fail-closed diagnosis and an owner-only recovery route."""
    identities, errors = ai_start.linked_worktree_identity_report()
    if errors:
        return 1, {"status": "blocked", "task": task, "errors": errors}
    matching = [identity for identity in identities if identity.task == task]
    canonical = [identity for identity in matching if identity.branch == f"codex/{task}"]
    foreign = [identity for identity in matching if identity.branch != f"codex/{task}"]
    recoverable = ai_start.recoverable_foreign_duplicate_identities(identities)
    if len(canonical) != 1 or not foreign or not all(item in recoverable for item in foreign):
        return 1, {
            "status": "not_recoverable",
            "task": task,
            "canonicalOwnerCount": len(canonical),
            "foreignIdentityCount": len(foreign),
            "recovery": "Do not mutate any checkout. Resolve this identity through its owning Work Item.",
        }
    return 0, {
        "status": "recoverable_foreign_duplicate",
        "task": task,
        "canonicalOwner": {
            "branch": canonical[0].branch,
            "worktree": canonical[0].worktree.as_posix(),
        },
        "foreignDuplicates": [
            {"branch": item.branch, "worktree": item.worktree.as_posix()} for item in foreign
        ],
        "authorization": "diagnostic_only_no_mutation",
        "ownerRepair": "The canonical owner must finish or explicitly resume its own Work Item. The foreign checkout must be repaired only by its owning corrective Work Item after it establishes a fresh branch and Contract identity.",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--task", required=True)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    code, value = report(args.task)
    print(json.dumps(value, ensure_ascii=False, sort_keys=True, indent=None if args.json else 2))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
