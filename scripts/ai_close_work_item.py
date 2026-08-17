#!/usr/bin/env python3
"""Close a completed Work Item by restoring a clean, synchronized repository."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import time
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from pathlib import Path

from ai_check_summary import validate_summary
from ai_check_work_item import validate_contract
from ai_common import (
    PROJECT_ROOT,
    InvalidProviderPayloadError,
    clean_git_environment,
    discover_remote_default_candidates,
    load_json,
    run_git,
)
from ai_lifecycle_truth import superseded_summary_validation_exception
from ai_projection_lease import release as release_projection_lease
from ai_projection_lease import requires_lease
from ai_work_item_intelligence import record_fact_once

ARCHIVE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "archive"
ACTIVE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "active"
STATUS_PATH = PROJECT_ROOT / ".ai" / "cockpit" / "current_status.md"
CLOSURE_RECEIPTS_DIR = PROJECT_ROOT / "target" / "task-closure-receipts"


@dataclass(frozen=True)
class CommandResult:
    returncode: int
    stdout: str = ""
    stderr: str = ""


Runner = Callable[[Sequence[str], bool], CommandResult]


def _run_git(args: Sequence[str], check: bool = False) -> CommandResult:
    result = run_git(list(args))
    if check and result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or f"git {' '.join(args)} failed")
    return CommandResult(result.returncode, result.stdout, result.stderr)


def _run_external(args: Sequence[str], check: bool = False) -> CommandResult:
    executable = shutil.which(args[0])
    if executable is None:
        raise RuntimeError(f"required command is unavailable: {args[0]}")
    result = subprocess.run(
        [executable, *args[1:]],
        cwd=PROJECT_ROOT,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )
    if check and result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or f"{' '.join(args)} failed")
    return CommandResult(result.returncode, result.stdout, result.stderr)


def _default_runner(args: Sequence[str], check: bool = False) -> CommandResult:
    if args and args[0] == "gh":
        return _run_external(args, check)
    return _run_git(args, check)


def _find_archived_contract(task: str) -> Path:
    matches = sorted(ARCHIVE_DIR.glob(f"*/{task}.contract.json"))
    if len(matches) != 1:
        raise RuntimeError(
            f"expected exactly one archived Contract for {task}, found {len(matches)}"
        )
    return matches[0]


def _release_projection_lease_if_required(task: str, branch: str, contract_path: Path) -> None:
    """Release the exact owner only after the full closure succeeds."""
    try:
        contract = load_json(contract_path)
    except (OSError, ValueError):
        return
    if requires_lease(contract):
        release_projection_lease(task, branch, root=PROJECT_ROOT)


def _recorded_start_branch(task: str) -> str | None:
    """Return a bounded legacy branch identity recorded at Work Item start."""
    receipt = PROJECT_ROOT / ".ai" / "work-items" / "starts" / f"{task}.json"
    if not receipt.is_file():
        return None
    data = load_json(receipt)
    branch = data.get("baseBranch") if isinstance(data, dict) else None
    if not isinstance(branch, str) or not branch.startswith("codex/"):
        return None
    return branch


def _archived_outcome_path(contract_path: Path) -> Path:
    outcome_path = contract_path.with_name(
        contract_path.name.replace(".contract.json", ".outcome.json")
    )
    if not outcome_path.is_file():
        raise RuntimeError(
            f"archived Task Outcome is missing: {outcome_path.relative_to(PROJECT_ROOT)}"
        )
    return outcome_path


def generate_final_human_report(
    task: str, contract_path: Path, closure_facts: dict[str, str]
) -> tuple[Path, Path]:
    """Write the provider-bound Final Report outside synchronized source history."""

    from ai_generate_human_report import (
        generate_human_report,
        render_human_report,
        validate_human_report,
    )

    outcome_path = _archived_outcome_path(contract_path)
    outcome = load_json(outcome_path)
    report = generate_human_report(outcome, phase="final", closure_facts=closure_facts)
    markdown = render_human_report(report)
    issues = validate_human_report(
        report,
        outcome,
        phase="final",
        closure_facts=closure_facts,
        markdown=markdown,
    )
    if issues:
        raise RuntimeError("Final Human Benefit Report is invalid: " + "; ".join(issues))
    json_path = CLOSURE_RECEIPTS_DIR / f"{task}.task-report.json"
    markdown_path = CLOSURE_RECEIPTS_DIR / f"{task}.task-report.md"
    json_path.parent.mkdir(parents=True, exist_ok=True)
    json_path.write_text(
        json.dumps(report, ensure_ascii=False, sort_keys=True, indent=2) + "\n",
        encoding="utf-8",
    )
    markdown_path.write_text(markdown, encoding="utf-8")
    return json_path, markdown_path


def generate_closure_receipt(
    task: str,
    contract_path: Path,
    pr: dict[str, object],
    *,
    work_branch: str,
    base_remote: str,
    base_branch: str,
    base_commit: str,
    base_worktree: str | None,
) -> Path:
    """Write a human-readable receipt only from verified closure facts."""
    outcome_path = _archived_outcome_path(contract_path)
    merge = pr.get("mergeCommit")
    merge_commit = merge.get("oid") if isinstance(merge, dict) else None
    if not isinstance(merge_commit, str) or len(merge_commit) != 40:
        raise RuntimeError("Closure Receipt requires authoritative merge commit")
    url = pr.get("url")
    if not isinstance(url, str) or not url.startswith("https://"):
        raise RuntimeError("Closure Receipt requires authoritative pull request URL")
    receipt_path = CLOSURE_RECEIPTS_DIR / f"{task}.closure.md"
    json_receipt_path = CLOSURE_RECEIPTS_DIR / f"{task}.closure.json"
    receipt_path.parent.mkdir(parents=True, exist_ok=True)
    next_worktree = base_worktree or PROJECT_ROOT.as_posix()
    final_json, final_markdown = generate_final_human_report(
        task,
        contract_path,
        {
            "pullRequest": url,
            "mergeCommit": merge_commit,
            "base": f"{base_remote}/{base_branch}",
            "baseCommit": base_commit,
            "workBranch": work_branch,
            "cleanup": "scheduled",
            "continueFrom": next_worktree,
        },
    )
    outcome_digest = hashlib.sha256(outcome_path.read_bytes()).hexdigest()
    json_receipt_path.write_text(
        json.dumps(
            {
                "workItemId": task,
                "outcomeDigest": outcome_digest,
                "pullRequest": {
                    "number": pr.get("number"),
                    "url": url,
                    "state": "merged",
                    "headSha": pr.get("headRefOid"),
                    "mergeSha": merge_commit,
                },
                "branch": {"name": work_branch, "remoteDeleted": False, "localDeleted": False},
                "defaultBranch": {
                    "name": base_branch,
                    "containsMerge": True,
                    "verifiedAt": "repository_recorded",
                },
                "closureState": "closing",
                "providerEvidence": [],
            },
            ensure_ascii=False,
            sort_keys=True,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    receipt_path.write_text(
        "\n".join(
            [
                f"# Work Item Closure Receipt: {task}",
                "",
                "## Evidence",
                f"- Archived Task Outcome: `{outcome_path.relative_to(PROJECT_ROOT).as_posix()}`",
                f"- Final Human Benefit Report: `{final_markdown}`",
                f"- Final Human Benefit JSON: `{final_json}`",
                f"- Pull Request: {url}",
                f"- Merge Commit: `{merge_commit}`",
                "",
                "## Closure facts",
                f"- Work branch scheduled for cleanup: `{work_branch}`",
                f"- Base synchronized: `{base_remote}/{base_branch}` at `{base_commit}`",
                f"- Continue from: `{next_worktree}`",
                "",
                "Branch cleanup is performed only after this receipt is written.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return receipt_path


def validate_closure_receipt(receipt_path: Path, task: str) -> None:
    """Reject a missing or incomplete user-visible receipt before cleanup."""
    if not receipt_path.is_file():
        raise RuntimeError("Closure Receipt is invalid: file is missing")
    text = receipt_path.read_text(encoding="utf-8")
    required = (
        f"# Work Item Closure Receipt: {task}",
        "## Evidence",
        "- Archived Task Outcome:",
        "- Final Human Benefit Report:",
        "- Final Human Benefit JSON:",
        "- Pull Request: https://",
        "- Merge Commit: `",
        "## Closure facts",
        "- Work branch scheduled for cleanup:",
        "- Base synchronized:",
        "- Continue from:",
    )
    if any(marker not in text for marker in required):
        raise RuntimeError("Closure Receipt is invalid: required closure facts are missing")


def finalize_closure_receipt(task: str) -> None:
    """Mark the persisted receipt closed only after both branch deletions succeed."""
    path = CLOSURE_RECEIPTS_DIR / f"{task}.closure.json"
    if not path.is_file():
        raise RuntimeError("Closure Receipt JSON is missing before branch cleanup")
    receipt = load_json(path)
    branch = receipt.get("branch")
    if not isinstance(branch, dict):
        raise TypeError("Closure Receipt JSON has invalid branch facts")
    branch["remoteDeleted"] = True
    branch["localDeleted"] = True
    receipt["closureState"] = "closed"
    path.write_text(
        json.dumps(receipt, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8"
    )


def _verify_archived_evidence(task: str) -> Path:
    if list(ACTIVE_DIR.glob("*.contract.json")) or list(ACTIVE_DIR.glob("*.summary.json")):
        raise RuntimeError("active Work Item evidence remains; archive the Work Item first")
    contract_path = _find_archived_contract(task)
    summary_path = contract_path.with_name(
        contract_path.name.replace(".contract.json", ".summary.json")
    )
    if not summary_path.is_file():
        raise RuntimeError(f"archived Summary is missing: {summary_path.relative_to(PROJECT_ROOT)}")
    contract = load_json(contract_path)
    summary = load_json(summary_path)
    contract_issues = validate_contract(contract)
    summary_issues = validate_summary(
        summary,
        contract,
        contract_path=contract_path.relative_to(PROJECT_ROOT).as_posix(),
        summary_path=summary_path.relative_to(PROJECT_ROOT).as_posix(),
        legacy_archive=False,
    )
    issues = list(contract_issues)
    if not superseded_summary_validation_exception(
        contract_path=contract_path,
        work_item_id=task,
        summary_issues=summary_issues,
    ):
        issues.extend(summary_issues)
    if issues:
        raise RuntimeError("archived Work Item evidence is invalid: " + "; ".join(issues))
    if "- State: `no_active_work_item`" not in STATUS_PATH.read_text(encoding="utf-8"):
        raise RuntimeError("Cockpit Status is not no_active_work_item")
    return contract_path


def _discover_base(runner: Runner) -> tuple[str, str]:
    candidates = discover_remote_default_candidates(lambda args: runner(args, False))
    if len(candidates) != 1:
        raise RuntimeError(
            "could not uniquely discover the repository remote default branch; "
            "set the remote HEAD or provide an adapter-specific configuration"
        )
    return candidates[0]


def _verify_pr(
    runner: Runner,
    branch: str,
    base_branch: str,
    branch_commit: str,
    *,
    allow_stacked_base: bool = False,
) -> dict[str, object]:
    try:
        result = runner(
            [
                "gh",
                "pr",
                "view",
                branch,
                "--json",
                "state,headRefName,headRefOid,baseRefName,mergedAt,mergeCommit,url",
            ],
            True,
        )
        data = json.loads(result.stdout)
    except (RuntimeError, json.JSONDecodeError) as exc:
        raise RuntimeError(
            f"cannot verify the pull request through the platform adapter: {exc}"
        ) from exc
    if not isinstance(data, dict):
        raise InvalidProviderPayloadError("pull request adapter returned a non-object response")
    if data.get("state") != "MERGED":
        raise RuntimeError("pull request is not merged; no cleanup was attempted")
    if data.get("headRefName") != branch:
        raise RuntimeError("pull request head branch does not match the current Work Item branch")
    if data.get("headRefOid") != branch_commit:
        raise RuntimeError("pull request Head SHA does not match the local Work Item branch")
    if not allow_stacked_base and data.get("baseRefName") != base_branch:
        raise RuntimeError("pull request base branch does not match the discovered repository base")
    merge_commit = data.get("mergeCommit")
    if not isinstance(merge_commit, dict) or not merge_commit.get("oid"):
        raise RuntimeError("merged pull request has no authoritative merge commit")
    if not data.get("mergedAt"):
        raise RuntimeError("merged pull request has no merge timestamp")
    return data


def _verify_stacked_base(
    runner: Runner,
    *,
    remote: str,
    default_branch: str,
    stacked_base: str,
    merge_commit: str,
) -> None:
    """Verify the narrow parent-Work-Item exception to default-base closure.

    A merged corrective may target an open parent Work Item branch, but no other
    arbitrary base is eligible for branch deletion. The parent must have
    archived evidence, remain unclosed, still exist remotely, and still retain
    the authoritative corrective merge commit. The latter direction matters:
    the parent branch can legitimately advance after the corrective PR merges.
    """
    if stacked_base == default_branch:
        return
    if not stacked_base.startswith("codex/") or not stacked_base.removeprefix("codex/"):
        raise RuntimeError("stacked pull request base is not a Work Item branch")
    parent_task = stacked_base.removeprefix("codex/")
    parent_contract_path = _find_archived_contract(parent_task)
    parent_contract = load_json(parent_contract_path)
    if parent_contract.get("workItemId") != parent_task:
        raise RuntimeError("stacked pull request base does not match its archived Work Item")
    parent_receipt = CLOSURE_RECEIPTS_DIR / f"{parent_task}.closure.md"
    if parent_receipt.exists():
        raise RuntimeError("stacked pull request parent Work Item is already closed")
    remote_parent = runner(["ls-remote", "--exit-code", "--heads", remote, stacked_base], False)
    if remote_parent.returncode != 0:
        raise RuntimeError("stacked pull request parent Work Item branch is absent from remote")
    runner(["fetch", remote, stacked_base], True)
    ancestry = runner(
        ["merge-base", "--is-ancestor", merge_commit, f"{remote}/{stacked_base}"], False
    )
    if ancestry.returncode != 0:
        raise RuntimeError("stacked pull request merge commit is not retained by its parent branch")


def _require_clean_worktree(runner: Runner) -> None:
    status = runner(["status", "--porcelain", "--untracked-files=all"], False)
    if status.returncode != 0:
        raise RuntimeError("cannot inspect repository worktree")
    if status.stdout.strip():
        raise RuntimeError("worktree or index is not clean; cleanup stopped")


def _base_worktree_path(runner: Runner, base_branch: str) -> str | None:
    """Find a worktree that currently owns the repository base branch."""
    result = runner(["worktree", "list", "--porcelain"], False)
    if result.returncode != 0:
        raise RuntimeError("cannot inspect Git worktrees")
    path: str | None = None
    for block in result.stdout.split("\n\n"):
        lines = block.splitlines()
        if not lines or not lines[0].startswith("worktree "):
            continue
        branch = next(
            (
                line.removeprefix("branch refs/heads/")
                for line in lines
                if line.startswith("branch ")
            ),
            None,
        )
        if branch == base_branch:
            path = lines[0].removeprefix("worktree ")
            break
    return path


def _in_worktree(runner: Runner, path: str) -> Runner:
    """Run Git commands against a designated worktree without changing branches."""

    def scoped(args: Sequence[str], check: bool = False) -> CommandResult:
        if args and args[0] == "gh":
            return runner(args, check)
        return runner(["-C", path, *args], check)

    return scoped


def _registered_target_worktree(path: str) -> Runner:
    """Return a runner for a registered worktree in this repository only."""
    target = Path(path).expanduser().resolve()
    if not target.is_dir():
        raise RuntimeError("target worktree path does not exist")
    runner = _in_worktree(_default_runner, str(target))
    top_level = runner(["rev-parse", "--show-toplevel"], False)
    if top_level.returncode != 0 or not top_level.stdout.strip():
        raise RuntimeError("target path is not a Git worktree")
    if Path(top_level.stdout.strip()).resolve() != target:
        raise RuntimeError("target path is not the root of a Git worktree")
    worktrees = runner(["worktree", "list", "--porcelain"], False)
    if worktrees.returncode != 0:
        raise RuntimeError("cannot inspect target Git worktrees")
    source_root = PROJECT_ROOT.resolve().as_posix()
    if f"worktree {source_root}" not in worktrees.stdout.splitlines():
        raise RuntimeError("target worktree is not registered in this repository")
    return runner


def _remote_branch_absent(runner: Runner, remote: str, branch: str) -> None:
    result = runner(["ls-remote", "--exit-code", "--heads", remote, branch], False)
    if result.returncode == 0:
        raise RuntimeError("remote work branch still exists")
    if result.returncode != 2:
        raise RuntimeError("could not verify remote work branch deletion")
    tracking = runner(["branch", "--remotes", "--list", f"{remote}/{branch}"], False)
    if tracking.returncode != 0:
        raise RuntimeError("could not verify local remote-tracking branch cleanup")
    if tracking.stdout.strip():
        raise RuntimeError("local remote-tracking Work Item branch still exists")


def _delete_remote_branch(runner: Runner, remote: str, branch: str) -> None:
    """Delete a remote branch and accept an externally completed deletion."""
    runner(["push", remote, "--delete", branch], False)
    runner(["fetch", remote, "--prune"], True)
    _remote_branch_absent(runner, remote, branch)


def _delete_local_branch(
    runner: Runner,
    branch: str,
    *,
    detach_required: bool,
) -> None:
    """Delete the local branch while preserving a retryable linked checkout."""
    if not detach_required:
        runner(["branch", "-D", branch], True)
    else:
        runner(["switch", "--detach", "HEAD"], True)
        try:
            runner(["branch", "-D", branch], True)
        except RuntimeError as exc:
            restored = runner(["switch", branch], False)
            if restored.returncode != 0:
                raise RuntimeError(
                    "local Work Item branch deletion failed after detach; "
                    "checkout restoration also failed"
                ) from exc
            raise RuntimeError(
                "local Work Item branch deletion failed after detach; "
                "the Work Item checkout was restored for retry"
            ) from exc

    result = runner(["branch", "--list", branch], False)
    if result.returncode != 0:
        raise RuntimeError("could not verify local Work Item branch cleanup")
    if result.stdout.strip():
        raise RuntimeError("local Work Item branch still exists")


def close_work_item(task: str, runner: Runner = _run_git) -> dict[str, object]:
    contract_path = _verify_archived_evidence(task)
    branch_result = runner(["branch", "--show-current"], False)
    if branch_result.returncode != 0 or not branch_result.stdout.strip():
        raise RuntimeError("closure must start from the Work Item branch, not a detached HEAD")
    work_branch = branch_result.stdout.strip()
    remote, base_branch = _discover_base(runner)
    if work_branch == base_branch:
        raise RuntimeError(
            "current branch is the repository base branch, not the still-identifiable Work Item branch; "
            "run ai-close-work-item from the merged Work Item branch before deleting it, then let closure "
            "synchronize the base and remove local/remote branches"
        )
    expected_branch = f"codex/{task}"
    recorded_branch = _recorded_start_branch(task)
    if work_branch != expected_branch and work_branch != recorded_branch:
        raise RuntimeError(
            "requested Work Item does not match the selected worktree branch; "
            f"expected {expected_branch}, found {work_branch}"
        )
    _require_clean_worktree(runner)
    work_commit = runner(["rev-parse", work_branch], True).stdout.strip()
    if not work_commit:
        raise RuntimeError("cannot resolve the local Work Item branch commit")
    pr = _verify_pr(
        runner,
        work_branch,
        base_branch,
        work_commit,
        allow_stacked_base=True,
    )
    pr_base = pr.get("baseRefName")
    if not isinstance(pr_base, str) or not pr_base:
        raise RuntimeError("merged pull request has no authoritative base branch")
    merge = pr.get("mergeCommit")
    merge_commit = merge.get("oid") if isinstance(merge, dict) else None
    if not isinstance(merge_commit, str) or not merge_commit:
        raise RuntimeError("merged pull request has no authoritative merge commit")
    if pr_base != base_branch:
        _verify_stacked_base(
            runner,
            remote=remote,
            default_branch=base_branch,
            stacked_base=pr_base,
            merge_commit=merge_commit,
        )
    closure_base = pr_base

    base_path = _base_worktree_path(runner, closure_base)
    base_runner = _in_worktree(runner, base_path) if base_path else runner
    if base_path:
        _require_clean_worktree(base_runner)
    else:
        runner(["switch", closure_base], True)
    base_runner(["fetch", remote, "--prune"], True)
    base_runner(["merge", "--ff-only", f"{remote}/{closure_base}"], True)
    local_base = base_runner(["rev-parse", closure_base], True).stdout.strip()
    remote_base = runner(["rev-parse", f"{remote}/{closure_base}"], True).stdout.strip()
    if local_base != remote_base:
        raise RuntimeError("base branch is not synchronized with the remote after fast-forward")

    final_branch = base_runner(["branch", "--show-current"], True).stdout.strip()
    if final_branch != closure_base:
        raise RuntimeError("repository is not on the synchronized base branch")
    _require_clean_worktree(base_runner)
    final_local = base_runner(["rev-parse", closure_base], True).stdout.strip()
    final_remote = runner(["rev-parse", f"{remote}/{closure_base}"], True).stdout.strip()
    if final_local != final_remote:
        raise RuntimeError("local base branch no longer matches the remote base branch")

    receipt_path = generate_closure_receipt(
        task,
        contract_path,
        pr,
        work_branch=work_branch,
        base_remote=remote,
        base_branch=closure_base,
        base_commit=final_local,
        base_worktree=base_path,
    )
    validate_closure_receipt(receipt_path, task)

    try:
        _delete_remote_branch(runner, remote, work_branch)
    except RuntimeError as exc:
        if base_path is None:
            restored = runner(["switch", work_branch], False)
            if restored.returncode != 0:
                raise RuntimeError(
                    f"{exc}; local Work Item branch remains, but checkout restoration also failed"
                ) from exc
            raise RuntimeError(f"{exc}; the Work Item checkout was restored for retry") from exc
        raise

    # A merged PR is the authority for deleting a branch. -D is intentional here:
    # squash and rebase merges do not make the source ref an ancestor of base.
    _delete_local_branch(
        runner,
        work_branch,
        detach_required=base_path is not None,
    )
    if (CLOSURE_RECEIPTS_DIR / f"{task}.closure.json").is_file():
        finalize_closure_receipt(task)
    _release_projection_lease_if_required(task, work_branch, contract_path)

    linked_base = base_path is not None
    repository_state = "closed_but_current_worktree_detached" if linked_base else "ready_on_base"
    return {
        "task": task,
        "contract": contract_path.relative_to(PROJECT_ROOT).as_posix(),
        "pullRequest": str(pr.get("url", "")),
        "workBranch": work_branch,
        "baseRemote": remote,
        "baseBranch": closure_base,
        "baseCommit": final_local,
        "closureReceipt": str(receipt_path),
        "finalHumanReport": str(CLOSURE_RECEIPTS_DIR / f"{task}.task-report.md"),
        "state": "closed",
        "repositoryState": repository_state,
        "nextWorkItemReady": not linked_base,
        "baseWorktree": base_path or "",
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Close a completed Work Item safely.")
    parser.add_argument("--task", required=True)
    parser.add_argument(
        "--worktree",
        help="registered child Work Item worktree for exceptional stacked-PR closure",
    )
    return parser.parse_args()


def main() -> int:
    phase_start = time.time()
    args = parse_args()
    try:
        target_worktree = getattr(args, "worktree", None)
        runner = (
            _registered_target_worktree(target_worktree) if target_worktree else _default_runner
        )
        result = close_work_item(args.task, runner)
    except (OSError, RuntimeError, ValueError) as exc:
        print(f"Work Item lifecycle: not closed\nReason: {exc}", file=sys.stderr)
        return 1
    print("Work Item lifecycle: closed")
    record_fact_once(args.task, "closed", {"closureReceipt": str(result["closureReceipt"])})
    print(f"Pull request: merged ({result['pullRequest']})")
    print(
        f"Archived Task Outcome: {Path(str(result['contract'])).with_name(Path(str(result['contract'])).name.replace('.contract.json', '.outcome.md'))}"
    )
    print(f"Closure Receipt: {result['closureReceipt']}")
    if result.get("finalHumanReport"):
        print(f"Final Human Benefit Report: {result['finalHumanReport']}")
    print(f"Local work branch: deleted ({result['workBranch']})")
    print(f"Remote work branch: deleted ({result['workBranch']})")
    print(
        f"Local {result['baseBranch']}: synchronized with {result['baseRemote']}/{result['baseBranch']}"
    )
    from ai_observability import create_observability

    create_observability(work_item_id=args.task).lifecycle_phase_finished(
        "closure", duration_ms=int((time.time() - phase_start) * 1000), cache_outcome="miss"
    )
    if result["nextWorkItemReady"] is True:
        print("Repository state: ready for next Work Item")
    else:
        print("Current worktree: detached; not ready for the next Work Item")
        print(f"Continue from synchronized base worktree: {result['baseWorktree']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
