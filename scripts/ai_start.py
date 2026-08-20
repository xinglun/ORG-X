#!/usr/bin/env python3
"""Create a new Work Item Contract and Summary skeleton."""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from collections.abc import Iterator
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

from ai_calibration_corrective import calibration_start_issue
from ai_check_diff_ownership import format_preview, preview
from ai_check_status_consistency import DEFAULT_STATUS, validate_status_consistency
from ai_check_summary import documentation_alignment_skeleton
from ai_check_work_item import validate_concurrency_boundary
from ai_common import (
    PROJECT_ROOT,
    capture_dirty_baseline,
    clean_git_environment,
    current_head,
    discover_remote_default_candidates,
    nested_make_command,
    save_json,
)
from ai_external_handoff import HandoffError, ingest_receipt
from ai_generate_status import write_active_status, write_no_active_status
from ai_lifecycle_truth import validate_successor_receipt
from ai_observability import create_observability
from ai_readiness_policy import readiness_state
from ai_start_receipt import build_receipt, current_branch, receipt_binding, receipt_path
from ai_work_item_intelligence import record_fact_once

ACTIVE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "active"
START_LOCK_FILENAME = ".ai-start.lock"
MODES = ["investigate", "author_todo", "code", "review", "cleanup"]
JOURNEYS = ["feature", "bugfix", "refactor", "cleanup"]
DEFAULT_CHECKPOINT_STAGES = ["before_edit", "before_finish"]
DEFAULT_VERIFICATION_CHECKS = [
    "aiWorkItem",
    "aiScope",
    "aiGuards",
    "aiCheckpoint",
    "aiAgentRisk",
    "aiReviewPolicy",
    "aiBacktrack",
    "aiCoverage",
    "aiScenarioCoverage",
    "aiGuidelines",
    "aiSummary",
    "aiStatus",
    "aiStatusCheck",
    "aiStatusConsistency",
    "aiDiffOwnership",
    "quality",
]


def default_verification() -> list[dict[str, object]]:
    return [{"check": check, "required": True} for check in DEFAULT_VERIFICATION_CHECKS]


def projected_archive_growth() -> int:
    """Return the archive count after this newly started Work Item closes."""
    archive_dir = PROJECT_ROOT / ".ai" / "work-items" / "archive"
    return len(list(archive_dir.rglob("*.contract.json"))) + 1


def slug(value: str) -> str:
    normalized = re.sub(r"[^a-zA-Z0-9_-]+", "_", value.strip().lower()).strip("_")
    if not normalized:
        raise ValueError("TASK cannot be empty")
    return normalized


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Create an AI Work Item skeleton.")
    parser.add_argument("--task", required=True, help="Task id, for example: add_health_check")
    parser.add_argument("--title", help="Human-readable title. Defaults to the task id.")
    parser.add_argument("--mode", default="investigate", choices=MODES)
    parser.add_argument(
        "--journey", default="feature", choices=JOURNEYS, help="Work journey preset."
    )
    parser.add_argument("--force", action="store_true", help="Overwrite an existing skeleton.")
    parser.add_argument(
        "--concurrency-boundary",
        help="JSON ownership declaration required when starting beside a linked Work Item.",
    )
    parser.add_argument(
        "--calibration-corrective",
        help="JSON declaration for the bounded corrective route while calibration is live.",
    )
    return parser.parse_args()


def refresh_stale_no_active_status(issues: list[str]) -> list[str]:
    stale_messages = {
        "cockpit status Changed Files do not match current Git changes; run `make repair-ai-status`",
        "cockpit status no-active state must not persist changed files; run `make repair-ai-status`",
    }
    if len(issues) == 1 and issues[0] in stale_messages:
        previous_status = DEFAULT_STATUS.read_bytes() if DEFAULT_STATUS.exists() else None
        try:
            write_no_active_status(DEFAULT_STATUS)
            refreshed_issues = validate_status_consistency()
        except (OSError, RuntimeError, ValueError):
            if previous_status is None:
                DEFAULT_STATUS.unlink(missing_ok=True)
            else:
                DEFAULT_STATUS.parent.mkdir(parents=True, exist_ok=True)
                DEFAULT_STATUS.write_bytes(previous_status)
            raise
        if refreshed_issues:
            if previous_status is None:
                DEFAULT_STATUS.unlink(missing_ok=True)
            else:
                DEFAULT_STATUS.parent.mkdir(parents=True, exist_ok=True)
                DEFAULT_STATUS.write_bytes(previous_status)
        return refreshed_issues
    return issues


def active_work_item_paths() -> list[Path]:
    if not ACTIVE_DIR.exists():
        return []
    paths = sorted(path for path in ACTIVE_DIR.glob("*.json") if path.is_file())
    resolved: set[Path] = set()
    now = datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    for handoff_path in paths:
        if not handoff_path.name.endswith(".handoff.json"):
            continue
        task = handoff_path.name.removesuffix(".handoff.json")
        receipt_path = ACTIVE_DIR / f"{task}.receipt.json"
        if not receipt_path.is_file():
            continue
        try:
            handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            if not has_complete_archived_work_item(PROJECT_ROOT, task):
                continue
            ingest_receipt(handoff, receipt, now=now)
        except (HandoffError, OSError, json.JSONDecodeError, TypeError):
            continue
        resolved.update({handoff_path, receipt_path})
    return [path for path in paths if path not in resolved]


def linked_worktree_records(*, root: Path = PROJECT_ROOT) -> list[tuple[Path, str | None]]:
    """Return linked worktree paths and their checked-out branches.

    ``git worktree list --porcelain`` is intentionally the only discovery
    source: arbitrary neighbouring directories must not influence lifecycle
    state. Detached worktrees are represented with ``None`` and are handled by
    the caller as historical/non-active checkouts.
    """
    result = subprocess.run(  # nosec B603 B607 - fixed list-form Git interrogation
        ["git", "worktree", "list", "--porcelain"],
        cwd=root,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        # Unit-level Contract construction uses a temporary non-Git root. It
        # cannot have linked worktrees, so preserve that isolated behavior;
        # real repository discovery errors remain fail-closed below.
        if not (root / ".git").exists():
            return [(root, None)]
        raise RuntimeError("cannot enumerate linked worktrees")
    records: list[tuple[Path, str | None]] = []
    for block in result.stdout.split("\n\n"):
        lines = block.splitlines()
        location = next(
            (line.removeprefix("worktree ") for line in lines if line.startswith("worktree ")), None
        )
        if not location:
            continue
        ref = next(
            (
                line.removeprefix("branch refs/heads/")
                for line in lines
                if line.startswith("branch refs/heads/")
            ),
            None,
        )
        records.append((Path(location), ref))
    return records


def has_complete_archived_work_item(worktree: Path, task: str) -> bool:
    """Return whether an active-record orphan is fully represented in its archive."""
    archive_root = worktree / ".ai" / "work-items" / "archive"
    required = ("contract.json", "summary.json", "outcome.json", "archive-manifest.json")
    return (
        any(
            all((year / f"{task}.{suffix}").is_file() for suffix in required)
            for year in archive_root.iterdir()
            if year.is_dir()
        )
        if archive_root.is_dir()
        else False
    )


@dataclass(frozen=True)
class LinkedWorktreeIdentity:
    """A validated active Work Item identity observed in a linked checkout."""

    worktree: Path
    branch: str
    task: str


BOUNDARY_PATH_KEYS = ("implementationPaths", "generatedEvidencePaths", "verificationOutputPaths")


def parse_concurrency_boundary(
    raw: str | None, task: str
) -> tuple[dict[str, object] | None, str | None]:
    """Parse and validate a candidate boundary before lifecycle writes."""
    if raw is None:
        return None, None
    try:
        boundary = json.loads(raw)
    except json.JSONDecodeError as exc:
        return None, f"ERROR: --concurrency-boundary must be valid JSON: {exc.msg}"
    if not isinstance(boundary, dict):
        return None, "ERROR: --concurrency-boundary must be a JSON object"
    issues = validate_concurrency_boundary({"workItemId": task, "concurrencyBoundary": boundary})
    if issues:
        return None, "ERROR: invalid --concurrency-boundary: " + "; ".join(issues)
    return boundary, None


def parse_calibration_corrective(raw: str | None) -> tuple[dict[str, object] | None, str | None]:
    """Parse the explicit corrective declaration before any lifecycle write."""
    if raw is None:
        return None, None
    try:
        corrective = json.loads(raw)
    except json.JSONDecodeError as exc:
        return None, f"ERROR: --calibration-corrective must be valid JSON: {exc.msg}"
    if not isinstance(corrective, dict):
        return None, "ERROR: --calibration-corrective must be a JSON object"
    return corrective, None


def boundary_overlap(
    candidate: dict[str, object], foreign: dict[str, object]
) -> tuple[str, str] | None:
    """Return any non-serialized path collision across declared ownership."""
    for candidate_key in BOUNDARY_PATH_KEYS:
        for foreign_key in BOUNDARY_PATH_KEYS:
            candidate_paths = candidate.get(candidate_key)
            foreign_paths = foreign.get(foreign_key)
            if not isinstance(candidate_paths, list) or not isinstance(foreign_paths, list):
                return candidate_key, "<invalid-boundary>"
            for candidate_path in candidate_paths:
                for foreign_path in foreign_paths:
                    if not isinstance(candidate_path, str) or not isinstance(foreign_path, str):
                        return candidate_key, "<invalid-boundary>"
                    candidate_prefix = candidate_path.removesuffix("/**")
                    foreign_prefix = foreign_path.removesuffix("/**")
                    if (
                        candidate_prefix == foreign_prefix
                        or candidate_prefix.startswith(foreign_prefix + "/")
                        or foreign_prefix.startswith(candidate_prefix + "/")
                    ):
                        return (
                            f"{candidate_key}->{foreign_key}",
                            f"{candidate_path} overlaps {foreign_path}",
                        )
    return None


def linked_worktree_boundary_issue(
    identities: list[LinkedWorktreeIdentity],
    candidate: dict[str, object] | None,
    *,
    require_boundary: bool,
) -> str | None:
    """Require and compare boundaries whenever a linked Work Item is active."""
    if not identities:
        return None
    if candidate is None and require_boundary:
        return "ERROR: linked Work Items are active; supply a valid --concurrency-boundary before lifecycle writes."
    if candidate is None:
        return None
    for identity in identities:
        path = (
            identity.worktree / ".ai" / "work-items" / "active" / f"{identity.task}.contract.json"
        )
        try:
            contract = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return f"ERROR: linked Work Item boundary is unreadable: {identity.worktree}"
        boundary = contract.get("concurrencyBoundary")
        issues = validate_concurrency_boundary(contract)
        if boundary is None or issues or not isinstance(boundary, dict):
            detail = "; ".join(issues) if issues else "concurrencyBoundary is missing"
            return f"ERROR: linked Work Item has no provable concurrency boundary: {identity.task}: {detail}"
        overlap = boundary_overlap(candidate, boundary)
        if overlap is not None:
            category, detail = overlap
            return (
                f"ERROR: planned concurrency boundary overlaps linked Work Item "
                f"{identity.task}: {category}: {detail}"
            )
    return None


def linked_worktree_identity_report(
    *, root: Path = PROJECT_ROOT
) -> tuple[list[LinkedWorktreeIdentity], list[str]]:
    """Discover linked active identities without mutating any checkout."""
    """Reject malformed foreign state while permitting isolated Work Items.

    Concurrency belongs to the agent orchestrator, not to a shared active-WI
    lock.  A linked worktree can therefore own one independently governed
    Work Item when its branch and both records unambiguously bind to that ID.
    Every other shape fails closed.
    """
    try:
        records = linked_worktree_records(root=root)
    except (OSError, RuntimeError):
        return [], ["ERROR: cannot enumerate linked worktrees before starting a Work Item"]
    identities: list[LinkedWorktreeIdentity] = []
    errors: list[str] = []
    for worktree, branch in records:
        try:
            if worktree.resolve() == root.resolve() or branch is None:
                continue
        except OSError:
            errors.append(
                f"ERROR: cannot resolve linked worktree before starting a Work Item: {worktree}"
            )
            continue
        active_dir = worktree / ".ai" / "work-items" / "active"
        contracts = {
            path.name.removesuffix(".contract.json") for path in active_dir.glob("*.contract.json")
        }
        summaries = {
            path.name.removesuffix(".summary.json") for path in active_dir.glob("*.summary.json")
        }
        summaries -= {
            task
            for task in summaries - contracts
            if has_complete_archived_work_item(worktree, task)
        }
        if not contracts and not summaries:
            continue
        if contracts != summaries:
            errors.append(
                "ERROR: linked worktree has malformed active Work Item records on branch "
                f"{branch}: {worktree} (contract/summary pair required)"
            )
            continue
        if len(contracts) != 1:
            errors.append(
                "ERROR: linked worktree has multiple active Work Items on branch "
                f"{branch}: {worktree}"
            )
            continue
        task = next(iter(contracts))
        try:
            contract = json.loads(
                (active_dir / f"{task}.contract.json").read_text(encoding="utf-8")
            )
            summary = json.loads((active_dir / f"{task}.summary.json").read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            errors.append(
                f"ERROR: linked worktree has unreadable active Work Item records: {worktree}"
            )
            continue
        if contract.get("workItemId") != task or summary.get("workItemId") != task:
            errors.append(
                f"ERROR: linked worktree active Work Item IDs do not match record paths: {worktree}"
            )
            continue
        identities.append(LinkedWorktreeIdentity(worktree, branch, task))
    return identities, errors


def recoverable_foreign_duplicate_identities(
    identities: list[LinkedWorktreeIdentity],
) -> set[LinkedWorktreeIdentity]:
    """Classify only a noncanonical duplicate with one canonical owner."""
    recoverable: set[LinkedWorktreeIdentity] = set()
    for identity in identities:
        canonical = [
            candidate
            for candidate in identities
            if candidate.task == identity.task and candidate.branch == f"codex/{identity.task}"
        ]
        if identity.branch != f"codex/{identity.task}" and len(canonical) == 1:
            recoverable.add(identity)
    return recoverable


def quarantined_successor_issue(
    requested_task: str | None,
    identities: list[LinkedWorktreeIdentity],
    *,
    root: Path,
) -> str | None:
    """Allow only the exact current-base successor named by a valid quarantine receipt."""
    for identity in identities:
        active_dir = identity.worktree / ".ai" / "work-items" / "active"
        outcome = active_dir / f"{identity.task}.outcome.json"
        receipt_path = active_dir / f"{identity.task}.successor-receipt.json"
        if not receipt_path.exists():
            continue
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            return f"ERROR: linked worktree has unreadable quarantined successor receipt: {receipt_path}"
        reason = validate_successor_receipt(
            predecessor_outcome=outcome,
            predecessor_work_item_id=identity.task,
            receipt=receipt,
        )
        if reason:
            successor = receipt.get("successor")
            successor_task = (
                successor.get("workItemId")
                if isinstance(successor, dict) and isinstance(successor.get("workItemId"), str)
                else None
            )
            if requested_task not in {identity.task, successor_task}:
                continue
            return (
                "ERROR: linked worktree has invalid quarantined successor receipt "
                f"({reason}): {receipt_path}"
            )
        successor = receipt["successor"]
        if (
            requested_task == successor["workItemId"]
            and current_branch() == successor["branch"]
            and current_head() == successor["baseCommit"]
        ):
            continue
        if requested_task not in {identity.task, successor["workItemId"]}:
            continue
        return (
            "ERROR: linked worktree quarantined successor receipt permits only "
            f"{successor['workItemId']} on {successor['branch']} at {successor['baseCommit']}: "
            f"{receipt_path}"
        )
    return None


def linked_worktree_active_issue(
    requested_task: str | None = None,
    *,
    candidate_boundary: dict[str, object] | None = None,
    require_boundary: bool = False,
    root: Path = PROJECT_ROOT,
) -> str | None:
    """Reject unsafe foreign state while allowing unrelated recoverable duplicates."""
    identities, errors = linked_worktree_identity_report(root=root)
    if errors:
        return errors[0]
    quarantine_issue = quarantined_successor_issue(requested_task, identities, root=root)
    if quarantine_issue:
        return quarantine_issue
    recoverable = recoverable_foreign_duplicate_identities(identities)
    for identity in identities:
        if identity in recoverable:
            if requested_task != identity.task:
                continue
            return (
                "ERROR: linked worktree has a recoverable foreign duplicate Work Item identity: "
                f"{identity.branch} carries {identity.task} while codex/{identity.task} is the canonical owner: "
                f"requested task {requested_task} conflicts with that active identity. "
                f"{identity.worktree}. Run `python3 scripts/ai_linked_worktree_recovery.py --task {identity.task}` "
                "for the read-only owner repair route."
            )
        if identity.branch != f"codex/{identity.task}":
            if requested_task is not None and requested_task != identity.task:
                continue
            return (
                "ERROR: linked worktree active Work Item branch does not match its task: "
                f"{identity.branch} != codex/{identity.task}: {identity.worktree}"
            )
    return linked_worktree_boundary_issue(
        identities, candidate_boundary, require_boundary=require_boundary
    )


def existing_work_item_ids() -> set[str]:
    """Return active and archived Work Item IDs before creating a new skeleton."""
    paths = list(ACTIVE_DIR.glob("*.contract.json"))
    paths.extend((PROJECT_ROOT / ".ai" / "work-items" / "archive").rglob("*.contract.json"))
    return {path.name.removesuffix(".contract.json") for path in paths}


def next_available_task_id(task: str, occupied_ids: set[str], *, date: str | None = None) -> str:
    """Choose a deterministic collision-free ID without overwriting history."""
    if task not in occupied_ids:
        return task
    stamp = date or datetime.now(UTC).astimezone().strftime("%Y%m%d")
    candidate = f"{task}-{stamp}"
    if candidate not in occupied_ids:
        return candidate
    suffix = 2
    while f"{candidate}-{suffix}" in occupied_ids:
        suffix += 1
    return f"{candidate}-{suffix}"


def configuration_gate_issue(
    task: str, *, root: Path = PROJECT_ROOT, mode: str = "code"
) -> str | None:
    """Block ordinary Work Items after adoption until configuration is ready."""
    if task == "configure_ai_cockpit" or mode != "code":
        return None
    evidence_path = root / ".ai" / "cockpit" / "adoption-runtime-verification.json"
    if not evidence_path.is_file():
        return None
    try:
        evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return "ERROR: Configuration Required; adoption Runtime Verification is unreadable."
    if evidence.get("readiness") == "ready" and evidence.get("projectQualityState") == "configured":
        return None
    if readiness_state(root).get("productionReady") is True:
        return None
    return (
        "ERROR: Configuration Required; finish configure_ai_cockpit and reach Adoption Ready "
        "before starting ordinary governed development."
    )


def default_branch_start_issue(*, root: Path = PROJECT_ROOT) -> str | None:
    """Reject a new Work Item on a uniquely discovered remote default branch."""

    def run_git(args: list[str]) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["git", *args],  # nosec B603 B607
            cwd=root,
            env=clean_git_environment(),
            text=True,
            capture_output=True,
            check=False,
        )

    try:
        candidates = discover_remote_default_candidates(run_git)
    except (OSError, RuntimeError):
        return None
    if len(candidates) != 1:
        return None
    remote, base_branch = candidates[0]
    if current_branch(project_root=root) != base_branch:
        return None
    return (
        "ERROR: ai-start requires a dedicated Work Item branch; "
        f"current branch {base_branch!r} is the discovered default branch "
        f"{remote}/{base_branch}. Create the dedicated branch from the latest remote base, "
        "then run ai-start again."
    )


def start_lock_path() -> Path:
    repo_hash = hashlib.sha256(str(PROJECT_ROOT.resolve()).encode("utf-8")).hexdigest()[:16]
    return Path(tempfile.gettempdir()) / f"codex-ai-start-{repo_hash}{START_LOCK_FILENAME}"


@contextlib.contextmanager
def acquire_start_lock() -> Iterator[None]:
    lock_path = start_lock_path()
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as lock_file:
        try:
            fcntl.flock(lock_file.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError as exc:
            raise RuntimeError(
                "another ai-start is already in progress; wait for it to finish before creating a new Work Item"
            ) from exc
        lock_file.seek(0)
        lock_file.truncate()
        lock_file.write(f"pid={os.getpid()}\n")
        lock_file.flush()
        try:
            yield
        finally:
            try:
                fcntl.flock(lock_file.fileno(), fcntl.LOCK_UN)
            except OSError:
                pass
            lock_path.unlink(missing_ok=True)


def run_make(
    target: str, *, contract: str | None = None, variables: list[str] | None = None
) -> tuple[int, str]:
    try:
        command = nested_make_command(["make", target], root=PROJECT_ROOT)
    except ValueError as exc:
        return 2, str(exc)
    if contract:
        command.append(f"CONTRACT={contract}")
    command.extend(variables or [])
    try:
        result = subprocess.run(
            command,
            cwd=PROJECT_ROOT,
            env=clean_git_environment(),
            text=True,
            capture_output=True,
            check=False,
        )
    except OSError as exc:
        return 127, str(exc)
    return result.returncode, (result.stdout or "") + (result.stderr or "")


def journey_policy(
    journey: str,
) -> tuple[list[str], list[str], list[str], dict[str, object]]:
    """Return acceptance, guidelines, exclusions, and destructive policy for a journey."""
    acceptance = ["The Work Item Contract is updated for the actual task."]
    guidelines: list[str] = []
    out_of_scope: list[str] = []
    destructive_policy: dict[str, object] = {
        "allowed": False,
        "requiresHumanApproval": True,
        "allowPatterns": [],
    }
    if journey == "feature":
        acceptance.extend(
            [
                "The new feature is implemented according to requirements.",
                "Unit tests are added to verify the new feature.",
                "User documentation or comments are updated.",
            ]
        )
        guidelines.extend(
            [
                "New public APIs must be documented.",
                "Do not import internal modules from other features.",
            ]
        )
    elif journey == "bugfix":
        acceptance.extend(
            [
                "The bug is reproduced by a test case.",
                "The fix resolves the bug and the test passes.",
                "No regression is introduced in existing functionality.",
            ]
        )
        guidelines.extend(
            [
                "Fix must target the root cause, not just the symptom.",
                "Avoid side effects on other components.",
            ]
        )
    elif journey == "refactor":
        acceptance.extend(
            [
                "Code structural changes are completed without changing functional behavior.",
                "All existing unit tests pass without modifications.",
                "API backwards compatibility is maintained.",
            ]
        )
        guidelines.extend(
            [
                "Zero functional changes allowed.",
                "Do not add new dependencies.",
                "Ensure clippy/linter produces zero warnings on changed code.",
            ]
        )
        out_of_scope.extend(["Adding new features", "Modifying existing public API signatures"])
    elif journey == "cleanup":
        acceptance.extend(
            [
                "Unused code, assets, or dependencies are removed.",
                "Documentation or formatting is cleaned up.",
                "Existing tests still pass.",
            ]
        )
        guidelines.extend(
            [
                "Do not modify active production code logic.",
                "Only delete dead code that is verified to have no callers.",
            ]
        )
        out_of_scope.extend(["Modifying business logic", "Adding new features"])
    return acceptance, guidelines, out_of_scope, destructive_policy


def persist_work_item(
    contract_path: Path,
    summary_path: Path,
    contract: dict[str, object],
    summary: dict[str, object],
) -> bool:
    """Persist a new Work Item and roll back if active status generation fails."""
    status_path = PROJECT_ROOT / ".ai" / "cockpit" / "current_status.md"
    previous_status = status_path.read_bytes() if status_path.exists() else None
    start_receipt_path = receipt_path(str(contract["workItemId"]), project_root=PROJECT_ROOT)
    save_json(contract_path, contract)
    save_json(summary_path, summary)
    try:
        start_receipt = build_receipt(contract, project_root=PROJECT_ROOT)
        contract["startReceipt"] = receipt_binding(start_receipt)
        changed_files = summary.get("changedFiles")
        if isinstance(changed_files, list):
            changed_files.append(
                {
                    "path": start_receipt["receiptPath"],
                    "reason": "Immutable Work Item Start Receipt created before implementation.",
                }
            )
        save_json(contract_path, contract)
        save_json(start_receipt_path, start_receipt)
    except (OSError, ValueError, KeyError) as exc:
        contract_path.unlink(missing_ok=True)
        summary_path.unlink(missing_ok=True)
        start_receipt_path.unlink(missing_ok=True)
        print(
            f"ERROR: failed to create Start Receipt; Work Item creation rolled back: {exc}",
            file=sys.stderr,
        )
        return False
    try:
        write_active_status(contract_path, summary_path)
    except (OSError, RuntimeError, ValueError) as exc:
        contract_path.unlink(missing_ok=True)
        summary_path.unlink(missing_ok=True)
        start_receipt_path.unlink(missing_ok=True)
        if previous_status is None:
            status_path.unlink(missing_ok=True)
        else:
            status_path.parent.mkdir(parents=True, exist_ok=True)
            status_path.write_bytes(previous_status)
        print(
            f"ERROR: failed to generate Cockpit status; Work Item creation rolled back: {exc}",
            file=sys.stderr,
        )
        return False
    return True


def run_code_preflight(contract_path: Path, summary_path: Path, contract_rel: str) -> int:
    """Run code-mode preflight and refresh status with its result."""
    code, output = run_make(
        "ai-preflight", contract=contract_rel, variables=["AI_PREFLIGHT_VALIDATE_CONTRACT=false"]
    )
    if output.strip():
        print(output.rstrip())
    try:
        write_active_status(contract_path, summary_path, announce=False)
    except (OSError, RuntimeError, ValueError) as exc:
        print(
            f"ERROR: failed to refresh Cockpit status after Preflight Review: {exc}",
            file=sys.stderr,
        )
        return 1
    return code


def validate_start_state(
    task: str,
    *,
    force: bool,
    mode: str = "code",
    candidate_boundary: dict[str, object] | None = None,
    calibration_corrective: dict[str, object] | None = None,
) -> tuple[Path, Path, str] | None:
    """Validate lifecycle state and return target paths plus trusted base commit."""
    branch_issue = default_branch_start_issue(root=PROJECT_ROOT)
    if branch_issue:
        print(branch_issue, file=sys.stderr)
        return None

    calibration_issue = calibration_start_issue(calibration_corrective, root=PROJECT_ROOT)
    if calibration_issue:
        print(calibration_issue, file=sys.stderr)
        return None

    if (
        candidate_boundary is not None
        and current_branch(project_root=PROJECT_ROOT) != f"codex/{task}"
    ):
        print(
            "ERROR: --concurrency-boundary requires the requested task's dedicated branch before lifecycle writes.",
            file=sys.stderr,
        )
        return None
    linked_issue = linked_worktree_active_issue(
        task,
        candidate_boundary=candidate_boundary,
        require_boundary=candidate_boundary is not None,
        root=PROJECT_ROOT,
    )
    if linked_issue:
        print(linked_issue, file=sys.stderr)
        return None

    consistency_issues = refresh_stale_no_active_status(validate_status_consistency())
    if consistency_issues:
        for issue in consistency_issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        print(
            "ERROR: fix Work Item lifecycle/status consistency before creating a new Work Item. "
            "Run `make repair-ai-status` when the active files are paired; otherwise clean up active Work Item files manually.",
            file=sys.stderr,
        )
        return None

    active_paths = active_work_item_paths()
    if active_paths:
        active_items = ", ".join(path.stem for path in active_paths)
        print(
            "ERROR: an active Work Item already exists: "
            f"{active_items}. Finish or archive it before creating a new Work Item.",
            file=sys.stderr,
        )
        return None

    gate_issue = configuration_gate_issue(task, mode=mode)
    if gate_issue:
        print(gate_issue, file=sys.stderr)
        return None

    contract_path = ACTIVE_DIR / f"{task}.contract.json"
    summary_path = ACTIVE_DIR / f"{task}.summary.json"
    if not force and (contract_path.exists() or summary_path.exists()):
        print(f"ERROR: Work Item already exists: {task}", file=sys.stderr)
        return None

    base_commit = current_head()
    if not base_commit:
        print(
            "ERROR: ai-start requires an initial Git commit so baseCommit is trustworthy.",
            file=sys.stderr,
        )
        return None
    return contract_path, summary_path, base_commit


def main() -> int:
    phase_start = time.time()
    args = parse_args()
    try:
        task = slug(args.task)
    except ValueError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    candidate_boundary, boundary_error = parse_concurrency_boundary(args.concurrency_boundary, task)
    if boundary_error:
        print(boundary_error, file=sys.stderr)
        return 2
    calibration_corrective, calibration_error = parse_calibration_corrective(
        args.calibration_corrective
    )
    if calibration_error:
        print(calibration_error, file=sys.stderr)
        return 2

    try:
        lock_context: contextlib.AbstractContextManager[None] = acquire_start_lock()
        lock_context.__enter__()
    except RuntimeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    try:
        active_paths = active_work_item_paths()
        if not active_paths:
            resolved_task = next_available_task_id(task, existing_work_item_ids())
            if resolved_task != task:
                print(
                    f"NOTICE: Work Item ID {task!r} already exists in history; using {resolved_task!r}.",
                    file=sys.stderr,
                )
                task = resolved_task
        start_state = validate_start_state(
            task,
            force=args.force,
            mode=args.mode,
            candidate_boundary=candidate_boundary,
            calibration_corrective=calibration_corrective,
        )
        if start_state is None:
            return 1
        contract_path, summary_path, base_commit = start_state
        title = args.title or task.replace("_", " ")
        baseline_dirty_paths = capture_dirty_baseline()
        contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
        summary_rel = summary_path.relative_to(PROJECT_ROOT).as_posix()

        acceptance_criteria, guidelines_list, out_of_scope_list, destructive_change_policy = (
            journey_policy(args.journey)
        )

        contract = {
            "contractVersion": 2,
            "workItemId": task,
            "mode": args.mode,
            "title": title,
            "baseCommit": base_commit,
            "baselineDirtyPaths": baseline_dirty_paths,
            "scope": [
                contract_rel,
                summary_rel,
                ".ai/cockpit/current_status.md",
                ".ai/cockpit/task_report.json",
                ".ai/cockpit/task_report.md",
                ".ai/work-items/starts/**",
                ".ai/work-items/archive/**",
                ".ai/knowledge/**",
                f".ai/work-items/active/{task}.outcome.json",
                f".ai/work-items/active/{task}.outcome.md",
            ],
            "outOfScope": out_of_scope_list,
            "sources": [{"path": contract_rel, "reason": "Initial Work Item skeleton."}],
            "unknowns": [
                "Replace this with concrete open questions, or clear it before mode code."
            ],
            "notCodable": False,
            "riskAssessment": {
                "level": "medium",
                "riskTypes": ["scope_unclear"],
                "reason": "Initial skeleton; replace with task-specific implementation and review risks.",
            },
            "governanceProfile": {
                "selected": "standard",
                "source": "automatic",
                "reasons": [
                    "Initial Work Item skeleton defaults to Standard until scope is classified."
                ],
                "override": None,
            },
            "agentCapability": {
                "canImplement": False,
                "canVerify": False,
                "needsHumanDecision": True,
                "blockedReason": "Initial skeleton; clear unknowns and confirm verification before coding.",
            },
            "executionDecision": {
                "status": "needs_human_decision",
                "reason": "Initial skeleton must be completed before execution.",
            },
            "preReviewWarnings": [
                "Replace with task-specific review focus, or clear when no special review focus remains."
            ],
            "checkpointPolicy": {
                "requiredBeforeFinish": True,
                "requiredStages": list(DEFAULT_CHECKPOINT_STAGES),
                "reason": "Record at least one checkpoint before finishing to reduce mid-task drift.",
            },
            "acceptance": acceptance_criteria,
            "guidelines": guidelines_list,
            # intent セクション（V2 以降）: AI が「なぜこの変更が存在するか」を理解するための文脈。
            # 全フィールドは任意。None は「未記入」を意味し、バリデーターに空文字列エラーを起こさない。
            # 現在の AI ワークフローで最も自然に記入されるのは problem / constraints / rationale の 3 フィールド。
            # businessGoal / userGoal / nonGoals はセクションに存在するが、文脈が提供されない限り記入しない。
            "intent": {
                "problem": None,
                "constraints": [],
                "rationale": None,
            },
            "verification": default_verification(),
            "destructiveChangePolicy": destructive_change_policy,
            "restrictedWriteApproval": {
                "approved": False,
                "approvedBy": "",
                "reason": "Set only when a human explicitly approves restricted governance paths.",
            },
            "rollbackNote": "Revert this Work Item diff and restore related tests and docs.",
            "budgetImpact": {"expectedMetrics": {"archiveGrowth": projected_archive_growth()}},
        }
        if candidate_boundary is not None:
            contract["concurrencyBoundary"] = candidate_boundary
        if calibration_corrective is not None:
            contract["calibrationCorrective"] = calibration_corrective
        summary = {
            "summaryVersion": 2,
            "workItemId": task,
            "contractPath": contract_rel,
            "changedFiles": [
                {"path": contract_rel, "reason": "Created the Work Item Contract skeleton."},
                {"path": summary_rel, "reason": "Created the AI Change Summary skeleton."},
            ],
            "sourcesUsed": [contract_rel],
            "verification": [
                {"check": item["check"], "result": "not_run"} for item in contract["verification"]
            ],
            "guidelinesCompliance": [
                {"guideline": item, "compliant": False, "evidence": "Not verified."}
                for item in guidelines_list
            ],
            "unknownsRemaining": ["Replace this before finishing the Work Item."],
            "risk": {
                "level": "medium",
                "detail": "Initial skeleton; scope and acceptance still need task-specific review.",
            },
            "generatedFiles": [],
            "destructiveChanges": [],
            "observedIssues": [],
            "residualRisks": [
                {
                    "level": "medium",
                    "area": "scope",
                    "detail": "Initial skeleton; replace with actual residual risks before finishing.",
                    "reviewRecommended": True,
                    "followUpCandidate": False,
                }
            ],
            "reviewReadiness": {
                "status": "not_ready",
                "reason": "Initial skeleton; required checks have not run.",
                "expectedReviewFocus": [],
            },
            "boundaryChecks": {
                "runtimeEntrypoints": "not_applicable",
                "userVisibleOutput": "not_applicable",
                "persistence": "not_applicable",
                "localization": "not_applicable",
                "generatedArtifacts": "not_applicable",
                "makeEntrypoints": "not_applicable",
            },
            "userCorrectionsCaptured": [],
            "userCorrectionSolidification": [],
            "checkpointEvidence": [],
            "knownGaps": ["Replace this before finishing the Work Item."],
            "overclaimPrevention": "Do not report completion for checks or behavior that were not verified.",
            "documentationAlignment": documentation_alignment_skeleton(),
        }
        if not persist_work_item(contract_path, summary_path, contract, summary):
            return 1
        observability = create_observability(work_item_id=task)
        observability.work_item_started(fields={"mode": args.mode, "title": title})
        record_fact_once(
            task,
            "contract_created",
            {"contractPath": contract_rel, "summaryPath": summary_rel, "mode": args.mode},
            root=PROJECT_ROOT,
        )

        # This deliberately uses the complete local diff, not the Contract-aware
        # task delta: files dirty before ai-start are not adopted by the new task.
        print("\n".join(format_preview(preview())))

        if args.mode == "code":
            code = run_code_preflight(contract_path, summary_path, contract_rel)
            if code != 0:
                return code

        print(f"Work Item skeleton created: {task}")
        print(f"contract: {contract_rel}")
        print(f"summary: {summary_rel}")
        getattr(observability, "lifecycle_phase_finished", lambda *_args, **_kwargs: None)(
            "planning", duration_ms=int((time.time() - phase_start) * 1000), cache_outcome="miss"
        )
        return 0
    finally:
        lock_context.__exit__(None, None, None)


if __name__ == "__main__":
    sys.exit(main())
