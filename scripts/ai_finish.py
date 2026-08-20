#!/usr/bin/env python3
"""Run finish checks for a Work Item through the Makefile."""

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import re
import shlex
import signal
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from ai_acceptance_policy import validate_acceptance_evidence
from ai_check_diff_ownership import format_preview, preview
from ai_common import (
    PROJECT_ROOT,
    changed_paths,
    clean_git_environment,
    current_head,
    discover_remote_default_candidates,
    included,
    load_json,
    nested_make_command,
    path_fingerprint,
    redact_machine_paths,
    redact_sensitive_output,
    render_check_command,
    run_git,
    save_json,
    verification_key,
)
from ai_evidence_dependencies import (
    EvidenceDependencyError,
    load_capability_evidence_dependencies,
    source_bound_evidence_is_affected,
)
from ai_observability import create_observability, elapsed_ms
from ai_projection_lease import ProjectionLeaseError, requires_lease
from ai_projection_lease import acquire as acquire_projection_lease
from ai_verification_policy import (
    classify_immutable_workflow_pin_change,
    finish_quality_route_for_contract,
)
from ai_work_item_intelligence import record_fact_once

ACTIVE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "active"
FINISH_LOCK_MAX_AGE_SECONDS = 24 * 60 * 60
FINISH_COMMAND_TIMEOUT_ENV = "AI_FINISH_COMMAND_TIMEOUT_SECONDS"
FINISH_COMMAND_TIMEOUT_DEFAULT_SECONDS = 60 * 60
FINISH_COMMAND_TIMEOUT_MAX_SECONDS = 24 * 60 * 60
FINISH_COMMAND_TERMINATION_GRACE_SECONDS = 5
REPORT_BOUNDARY_TEXT = {
    "en": (
        "## Task Outcome Report (active; relay to the human before archive)",
        "CLI output cannot authenticate human receipt or approval.",
        "Next lifecycle action: archive is explicit and must follow the direct human report.",
    ),
    "zh-CN": (
        "## 工单结果报告（active；归档前必须直接告知相关人员）",
        "CLI 输出不能证明人类已阅读或批准报告。 (CLI output cannot authenticate human receipt or approval.)",
        "下一生命周期动作：归档必须显式执行，并且只能在直接报告之后进行。",
    ),
    "ja": (
        "## タスク結果レポート（active。アーカイブ前に直接人へ報告してください）",
        "CLI 出力は人間による受領または承認を認証できません。 (CLI output cannot authenticate human receipt or approval.)",
        "次のライフサイクル操作：アーカイブは明示的に実行し、直接報告の後にのみ行います。",
    ),
}

CURRENT_REPORT_LANGUAGE = "en"
PRE_MERGE_MERGE_IDENTITY_GAP_PREFIX = (
    "Merged commit identity is intentionally null until a post-merge binding exists;"
)
PROJECT_TEST_AGGREGATE_RECEIPT = Path("target/quality/project-test-aggregate/receipt.json")


def restore_tracked_project_test_receipt(*, root: Path = PROJECT_ROOT) -> bool:
    """Restore the tracked aggregate receipt after quality mutates its worktree copy."""
    receipt = PROJECT_TEST_AGGREGATE_RECEIPT.as_posix()
    tracked = subprocess.run(  # nosec B603 B607 - fixed Git argv and bounded repository path
        ["git", "ls-files", "--error-unmatch", "--", receipt],
        cwd=root,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )
    if tracked.returncode != 0:
        repository = subprocess.run(  # nosec B603 B607 - fixed Git argv and bounded repository path
            ["git", "rev-parse", "--is-inside-work-tree"],
            cwd=root,
            env=clean_git_environment(),
            text=True,
            capture_output=True,
            check=False,
        )
        if repository.returncode != 0:
            detail = repository.stderr.strip() or tracked.stderr.strip() or "unknown Git error"
            raise RuntimeError(f"cannot inspect tracked quality receipt: {detail}")
        return False

    restored = subprocess.run(  # nosec B603 B607 - fixed Git argv and bounded repository path
        ["git", "restore", "--source=HEAD", "--worktree", "--", receipt],
        cwd=root,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )
    if restored.returncode != 0:
        detail = restored.stderr.strip() or "git restore failed"
        raise RuntimeError(f"cannot restore tracked quality receipt: {detail}")
    return True


def checkpoint_recovery_guidance(issues: list[str], *, contract: str, summary: str) -> str:
    """Return the canonical non-bypass recovery command for checkpoint failures."""
    missing_before_finish = any(
        "missing checkpointEvidence for required stage(s):" in issue and "before_finish" in issue
        for issue in issues
    )
    if missing_before_finish:
        return (
            "ERROR: Required before_finish checkpoint evidence is missing; run "
            f"make ai-checkpoint CONTRACT={contract} SUMMARY={summary} "
            "STAGE=before_finish before retrying ai-finish."
        )
    stale_before_edit = any(
        "contract_amendment_revalidation" in issue or "before_edit" in issue and "stale" in issue
        for issue in issues
    )
    if stale_before_edit:
        return (
            "ERROR: Immutable before_edit Contract binding is stale; run "
            f"make ai-revalidate-contract-amendment CONTRACT={contract} SUMMARY={summary} "
            "PREVIOUS_CONTRACT_HASH=<immutable-before-edit-hash> "
            "AMENDMENT_REASON='<why the Contract changed>' before retrying ai-finish."
        )
    return (
        "ERROR: Checkpoint validation failed; inspect every reported checkpoint issue, "
        "preserve active evidence, and use only the stage-specific canonical recovery "
        "command before retrying ai-finish."
    )


class FinishMutexError(RuntimeError):
    """Raised when another Finish owns this Work Item worktree."""


def _finish_lock_path(task: str, *, root: Path) -> Path:
    """Return an untracked lock location unique to this worktree and task."""
    result = subprocess.run(  # nosec B603 B607 - fixed list-form Git metadata lookup
        ["git", "rev-parse", "--git-dir"],
        cwd=root,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode == 0 and result.stdout.strip():
        git_dir = Path(result.stdout.strip())
        if not git_dir.is_absolute():
            git_dir = root / git_dir
        return git_dir / f"ai-finish-{task}.lock"
    return root / ".ai" / "work-items" / "runtime" / f"ai-finish-{task}.lock"


def _load_finish_lock_metadata(lock_path: Path) -> dict[str, Any]:
    try:
        value = json.loads(lock_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return value if isinstance(value, dict) else {}


def _metadata_is_stale(metadata: dict[str, Any], *, now: datetime) -> bool:
    value = metadata.get("startedAt")
    if not isinstance(value, str):
        return False
    try:
        started = datetime.fromisoformat(value)
    except ValueError:
        return False
    if started.tzinfo is None:
        return False
    return (now - started.astimezone(UTC)).total_seconds() > FINISH_LOCK_MAX_AGE_SECONDS


@contextlib.contextmanager
def finish_mutex(
    task: str,
    *,
    archive: bool,
    root: Path = PROJECT_ROOT,
) -> Any:
    """Serialize Finish only for one task in one worktree.

    Advisory metadata makes the owner/retry route observable.  The kernel lock
    is authoritative, automatically released on process death, and therefore
    cannot turn a crashed Finish into an unbounded stale lock.
    """
    lock_path = _finish_lock_path(task, root=root)
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    with lock_path.open("a+", encoding="utf-8") as handle:
        try:
            fcntl.flock(handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            handle.seek(0)
            owner = _load_finish_lock_metadata(lock_path)
            owner_pid = owner.get("pid", "unknown")
            owner_mode = "archive" if owner.get("archive") is True else "normal"
            requested_mode = "archive" if archive else "normal"
            raise FinishMutexError(
                "ai-finish is already running for this Work Item worktree "
                f"(task={task}, owner_pid={owner_pid}, owner_mode={owner_mode}, "
                f"requested_mode={requested_mode}). Wait for the owner to finish, then retry; "
                "do not remove or edit active evidence or the mutex file."
            ) from exc
        now = datetime.now(UTC)
        previous = _load_finish_lock_metadata(lock_path)
        if _metadata_is_stale(previous, now=now):
            print(
                "Recovered stale ai-finish mutex metadata after kernel-lock acquisition "
                f"(task={task}, previous_pid={previous.get('pid', 'unknown')}).",
                file=sys.stderr,
            )
        metadata = {
            "task": task,
            "pid": os.getpid(),
            "startedAt": now.isoformat(),
            "archive": archive,
            "worktree": root.resolve().as_posix(),
        }
        handle.seek(0)
        handle.truncate()
        json.dump(metadata, handle, ensure_ascii=False, sort_keys=True)
        handle.write("\n")
        handle.flush()
        os.fsync(handle.fileno())
        try:
            yield
        finally:
            handle.seek(0)
            handle.truncate()
            handle.flush()
            os.fsync(handle.fileno())
            fcntl.flock(handle.fileno(), fcntl.LOCK_UN)


def _git_output(args: list[str]) -> str:
    result = run_git(args)
    if result.returncode != 0:
        raise RuntimeError(
            f"git {' '.join(args)} failed: {(result.stderr or result.stdout).strip()}"
        )
    return result.stdout.strip()


def repository_base_branch() -> str | None:
    candidates = discover_remote_default_candidates(run_git)
    if len(candidates) > 1:
        raise RuntimeError(
            "could not uniquely discover the repository remote default branch; "
            "multiple remote HEAD targets were found"
        )
    return candidates[0][1] if candidates else None


def ensure_work_item_branch() -> None:
    current = _git_output(["branch", "--show-current"])
    base = repository_base_branch()
    if base is not None:
        validate_work_item_branch(current, base)


def validate_work_item_branch(current: str, base: str) -> None:
    if current == base:
        raise RuntimeError(
            "ai-finish must run on the dedicated Work Item branch; current branch is the repository "
            f"base branch ({base}). Finish/archive on the Work Item branch before pushing and opening the PR."
        )


def task_paths(task: str) -> tuple[str, str]:
    contract = ACTIVE_DIR / f"{task}.contract.json"
    summary = ACTIVE_DIR / f"{task}.summary.json"
    return contract.relative_to(PROJECT_ROOT).as_posix(), summary.relative_to(
        PROJECT_ROOT
    ).as_posix()


def finish_command_timeout_seconds() -> float:
    """Return the finite command timeout, rejecting unsafe overrides."""
    raw = os.environ.get(FINISH_COMMAND_TIMEOUT_ENV)
    if raw is None or not raw.strip():
        return float(FINISH_COMMAND_TIMEOUT_DEFAULT_SECONDS)
    try:
        value = float(raw)
    except ValueError as exc:
        raise ValueError(
            f"{FINISH_COMMAND_TIMEOUT_ENV} must be a finite number of seconds"
        ) from exc
    if not value.is_integer() or not 1 <= value <= FINISH_COMMAND_TIMEOUT_MAX_SECONDS:
        raise ValueError(
            f"{FINISH_COMMAND_TIMEOUT_ENV} must be an integer from 1 through "
            f"{FINISH_COMMAND_TIMEOUT_MAX_SECONDS}"
        )
    return value


def _owned_process_groups(root_pid: int) -> set[int]:
    """Return process groups belonging to one command's process tree."""
    if os.name != "posix":
        return {root_pid}
    try:
        snapshot = subprocess.run(  # nosec B603 B607 - fixed local ps inspection command
            ["/bin/ps", "-eo", "pid=,ppid=,pgid="],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.SubprocessError):
        return {root_pid}
    children: dict[int, list[tuple[int, int]]] = {}
    for line in snapshot.stdout.splitlines():
        try:
            pid, parent_pid, process_group = (int(value) for value in line.split())
        except ValueError:
            continue
        children.setdefault(parent_pid, []).append((pid, process_group))
    groups = {root_pid}
    pending = [root_pid]
    while pending:
        parent_pid = pending.pop()
        for child_pid, process_group in children.get(parent_pid, []):
            groups.add(process_group)
            pending.append(child_pid)
    return groups


def _signal_owned_process_group(
    process: subprocess.Popen[str], signum: int, groups: set[int] | None = None
) -> None:
    """Signal only process groups in one Finish command's process tree."""
    if os.name == "posix":
        for process_group in sorted(groups or _owned_process_groups(process.pid)):
            try:
                os.killpg(process_group, signum)
            except (ProcessLookupError, OSError):
                pass
    else:
        if signum == signal.SIGKILL:
            process.kill()
        else:
            process.terminate()


def _terminate_owned_process_group(process: subprocess.Popen[str]) -> None:
    """Terminate, escalate, and reap one Finish command and its descendants."""
    groups = _owned_process_groups(process.pid)
    if process.poll() is not None and groups == {process.pid}:
        return
    _signal_owned_process_group(process, signal.SIGTERM, groups)
    try:
        process.wait(timeout=FINISH_COMMAND_TERMINATION_GRACE_SECONDS)
    except subprocess.TimeoutExpired:
        _signal_owned_process_group(process, signal.SIGKILL, groups)
        process.wait()


def run(command: list[str], *, extra_env: dict[str, str] | None = None) -> tuple[int, int, str]:
    command = list(command)
    if command and command[0] == "make":
        for name in ("PROJECT_FORMAT_CHECK", "PROJECT_TEST", "PROJECT_LINT"):
            if name in os.environ and not any(item.startswith(f"{name}=") for item in command):
                command.append(f"{name}={os.environ[name]}")
    try:
        command = nested_make_command(command, root=PROJECT_ROOT)
    except ValueError as exc:
        output = f"ERROR: {exc}\n"
        print(output, end="", file=sys.stderr)
        return 2, 0, output
    try:
        timeout_seconds = finish_command_timeout_seconds()
    except ValueError as exc:
        output = f"ERROR: {exc}\n"
        print(output, end="", file=sys.stderr)
        return 2, 0, output
    print("$ " + " ".join(command))
    start = time.time()
    environment = clean_git_environment()
    if extra_env:
        environment.update(extra_env)
    process = subprocess.Popen(  # validated argv, no shell, owned session
        command,
        cwd=PROJECT_ROOT,
        env=environment,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        start_new_session=os.name == "posix",
    )
    cancelled_signal: int | None = None
    timed_out = False
    previous_handlers: dict[int, Any] = {}

    def cancel(signum: int, _frame: Any) -> None:
        nonlocal cancelled_signal
        cancelled_signal = signum
        _terminate_owned_process_group(process)

    if os.name == "posix":
        for signum in (signal.SIGINT, signal.SIGTERM):
            previous_handlers[signum] = signal.getsignal(signum)
            signal.signal(signum, cancel)
    try:
        try:
            captured, _ = process.communicate(timeout=timeout_seconds)
        except subprocess.TimeoutExpired:
            timed_out = True
            _terminate_owned_process_group(process)
            captured, _ = process.communicate()
    finally:
        for restored_signum, handler in previous_handlers.items():
            signal.signal(restored_signum, handler)
    output = captured or ""
    if timed_out:
        output += (
            f"\n🔴 ai-finish command timed out after {int(timeout_seconds)} second(s); "
            "owned process group terminated.\n"
        )
        code = 124
    elif cancelled_signal is not None:
        output += (
            f"\n🔴 ai-finish command cancelled by signal {cancelled_signal}; "
            "owned process group terminated.\n"
        )
        code = 128 + cancelled_signal
    else:
        code = process.returncode
    if output:
        displayed = console_output(output)
        print(displayed, end="" if displayed.endswith("\n") else "\n")
    return code, elapsed_ms(start), output


def evidence(
    check_id: str,
    command: str,
    code: int,
    duration: int,
    output: str,
    *,
    contract_hash: str,
    commit_sha: str,
    execution_contract_path: str,
    execution_summary_path: str,
    worktree_digest: str,
) -> dict[str, Any]:
    compact = redact_sensitive_output(output)
    compact = redact_machine_paths(compact)
    compact = " ".join(compact.split())[:500]
    tail = redact_sensitive_output(output)
    tail = redact_machine_paths(tail)
    tail = " ".join(tail.split())[-2000:]
    return {
        "check": check_id,
        "command": command,
        "result": "passed" if code == 0 else "failed",
        "runner": "ai_finish",
        "executedAt": datetime.now(UTC).isoformat(),
        "exitCode": code,
        "durationMs": duration,
        "outputDigest": hashlib.sha256(output.encode("utf-8")).hexdigest(),
        "commandHash": hashlib.sha256(" ".join(command.split()).encode("utf-8")).hexdigest(),
        "contractHash": contract_hash,
        "commitSha": commit_sha,
        "executionContractPath": execution_contract_path,
        "executionSummaryPath": execution_summary_path,
        "worktreeDigest": worktree_digest,
        "outputSummary": compact,
        "outputTail": tail,
        "outputBytes": len(output.encode("utf-8")),
    }


def pending_evidence(
    check_id: str,
    command: str,
    *,
    contract_hash: str,
    commit_sha: str,
    execution_contract_path: str,
    execution_summary_path: str,
    worktree_digest: str,
) -> dict[str, Any]:
    item = evidence(
        check_id,
        command,
        0,
        0,
        "pending transactional validation",
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        execution_contract_path=execution_contract_path,
        execution_summary_path=execution_summary_path,
        worktree_digest=worktree_digest,
    )
    item["runner"] = "ai_finish_pending"
    return item


def worktree_digest(paths: list[str]) -> str:
    digest = hashlib.sha256()
    for path in sorted(set(paths)):
        digest.update(path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path_fingerprint(path).encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()


def worktree_digest_for_finish(paths: list[str], summary_path: str) -> str:
    """Hash source state without lifecycle projections written after verification."""
    outcome_stem = summary_path.removesuffix(".summary.json")
    derived_paths = {
        summary_path,
        f"{outcome_stem}.outcome.json",
        f"{outcome_stem}.outcome.md",
        ".ai/cockpit/current_status.md",
        ".ai/cockpit/task_report.json",
        ".ai/cockpit/task_report.md",
    }
    return worktree_digest([path for path in paths if path not in derived_paths])


def outcome_input_digest(summary: dict[str, Any]) -> str:
    """Bind archive reuse to every Summary field consumed by Outcome generation.

    The final ``aiSummary`` record is deliberately excluded: it is written
    after Outcome generation to attest the stabilized state and therefore
    cannot be an Outcome input without creating a self-reference cycle.
    """
    verification = summary.get("verification", [])
    return _sha256_json(
        {
            "changedFiles": summary.get("changedFiles", []),
            "knownGaps": summary.get("knownGaps", []),
            "nonRiskExplanations": summary.get("nonRiskExplanations", []),
            "userCorrectionsCaptured": summary.get("userCorrectionsCaptured", []),
            "verification": [
                item
                for item in verification
                if isinstance(item, dict) and item.get("check") != "aiSummary"
            ]
            if isinstance(verification, list)
            else verification,
            "verificationHistory": summary.get("verificationHistory", []),
        }
    )


def reusable_archive_verification(
    summary: dict[str, Any],
    contract_data: dict[str, Any],
    *,
    contract_hash: str,
    commit_sha: str,
    contract: str,
    summary_path: str,
) -> bool:
    """Return whether archive can consume a same-state finish attestation.

    `aiSummary` is written last and fingerprints every owned path except the
    self-referential Summary.  It is therefore the only reusable anchor: a
    prior pass from another commit, Contract, path, or worktree state is never
    sufficient to skip strict verification.
    """
    values = summary.get("verification")
    if not isinstance(values, list):
        return False
    expected_digest = worktree_digest_for_finish(changed_paths(contract_data), summary_path)
    return any(
        isinstance(item, dict)
        and item.get("check") == "aiSummary"
        and item.get("result") == "passed"
        and item.get("runner") == "ai_finish"
        and item.get("contractHash") == contract_hash
        and item.get("commitSha") == commit_sha
        and item.get("executionContractPath") == contract
        and item.get("executionSummaryPath") == summary_path
        and item.get("worktreeDigest") == expected_digest
        and item.get("outcomeInputDigest") == outcome_input_digest(summary)
        for item in values
    )


def record_result(summary_path: Path, item: dict[str, Any]) -> None:
    if not summary_path.exists():
        raise FileNotFoundError(f"summary not found: {summary_path.relative_to(PROJECT_ROOT)}")
    summary = load_json(summary_path)
    values = summary.get("verification", [])
    if not isinstance(values, list):
        values = []
    key = verification_key(item)
    history = summary.get("verificationHistory", [])
    if not isinstance(history, list):
        history = []
    for entry in values:
        if not isinstance(entry, dict) or verification_key(entry) != key:
            continue
        if entry != item and entry.get("result") == "failed":
            digest = _sha256_json(entry)
            if not any(
                _sha256_json(previous) == digest
                for previous in history
                if isinstance(previous, dict)
            ):
                history.append(entry)
        # Same-check records are projections of the latest attempt; they are
        # intentionally removed from the current verification view.
    summary["verification"] = [
        entry
        for entry in values
        if not (isinstance(entry, dict) and verification_key(entry) == key)
    ] + [item]
    if history:
        summary["verificationHistory"] = history
    save_json(summary_path, summary)


def discard_stale_contract_verification(summary_path: Path, contract_hash: str) -> int:
    """Remove active verification records that cannot attest the current Contract.

    Active Summary evidence is a projection of the current Contract, not a
    historical ledger.  A Contract amendment invalidates every verification
    record bound to its prior hash; retaining those records makes ``aiSummary``
    fail before Finish can record the replacement evidence, forcing a needless
    retry.  Archive evidence preserves the historical record separately.
    """
    summary = load_json(summary_path)
    values = summary.get("verification", [])
    if not isinstance(values, list):
        return 0
    retained = [
        item
        for item in values
        if isinstance(item, dict) and item.get("contractHash") == contract_hash
    ]
    removed = len(values) - len(retained)
    if removed:
        summary["verification"] = retained
        save_json(summary_path, summary)
    return removed


def promote_review_readiness(
    summary: dict[str, Any], contract: dict[str, Any] | None = None
) -> dict[str, Any]:
    """Derive review readiness from recorded verification and residual risk."""
    verification = summary.get("verification")
    unknowns = summary.get("unknownsRemaining")
    required_checks: set[str] | None = None
    if isinstance(contract, dict) and contract.get("contractVersion") == 2:
        declared = contract.get("verification")
        if isinstance(declared, list):
            required_checks = {
                item["check"]
                for item in declared
                if isinstance(item, dict)
                and isinstance(item.get("check"), str)
                and item.get("required", True) is True
            }
    if isinstance(verification, list) and required_checks is not None:
        passed_required_checks = {
            item.get("check")
            for item in verification
            if isinstance(item, dict) and item.get("result") == "passed"
        }
        verification_complete = required_checks <= passed_required_checks
    else:
        verification_complete = (
            isinstance(verification, list)
            and bool(verification)
            and all(
                isinstance(item, dict) and item.get("result") == "passed" for item in verification
            )
        )
    complete = verification_complete and isinstance(unknowns, list) and not unknowns
    existing = summary.get("reviewReadiness")
    expected_focus = (
        existing.get("expectedReviewFocus", [])
        if isinstance(existing, dict) and isinstance(existing.get("expectedReviewFocus"), list)
        else []
    )
    if isinstance(contract, dict):
        acceptance_issues = validate_acceptance_evidence(
            contract,
            summary,
            summary.get("verification", [])
            if isinstance(summary.get("verification"), list)
            else [],
        )
        if acceptance_issues:
            return {
                "status": "not_ready",
                "reason": "Acceptance evidence is incomplete: " + "; ".join(acceptance_issues[:3]),
                "expectedReviewFocus": expected_focus,
            }
    if not complete:
        return {
            "status": "not_ready",
            "reason": "Required verification or known-unknown evidence is incomplete.",
            "expectedReviewFocus": expected_focus,
        }
    residual_risks = summary.get("residualRisks")
    has_residual_risk = isinstance(residual_risks, list) and bool(residual_risks)
    return {
        "status": "ready_with_risks" if has_residual_risk else "ready",
        "reason": (
            "All required verification passed; residual risk remains documented."
            if has_residual_risk
            else "All required verification passed and no residual risk remains."
        ),
        "expectedReviewFocus": expected_focus,
    }


def archive_next_steps(task: str) -> str:
    return (
        "Work Item archived; lifecycle is not closed. "
        "Next steps: push this Work Item branch, open and merge its PR, "
        f"then run make ai-close-work-item TASK={task}."
    )


def pre_archive_critical_coverage_command(
    contract_data: dict[str, Any],
) -> tuple[list[str] | None, str | None]:
    """Return the base-bound coverage gate that must precede archive mutation."""
    base_commit = contract_data.get("baseCommit")
    if not isinstance(base_commit, str) or not base_commit.strip():
        return None, "Contract baseCommit is required for pre-archive critical coverage"
    work_item_id = contract_data.get("workItemId")
    if not isinstance(work_item_id, str) or not work_item_id:
        return None, "pre-archive changed-critical coverage requires a Work Item id"
    return [
        "make",
        "check-changed-critical-coverage",
        f"AI_BASE_COMMIT={base_commit}",
        f"CONTRACT=.ai/work-items/active/{work_item_id}.contract.json",
    ], None


def run_pre_archive_critical_coverage(
    contract_data: dict[str, Any], *, obs: Any
) -> tuple[int, str]:
    """Run and record the archive-blocking critical coverage gate."""
    command, error = pre_archive_critical_coverage_command(contract_data)
    if command is None:
        return 2, error or "pre-archive critical coverage is unavailable"
    command_text = " ".join(command)
    obs.check_started(check_id="preArchiveCriticalCoverage", command=command_text)
    code, duration, output = run(command)
    if code:
        obs.check_failed(
            check_id="preArchiveCriticalCoverage", command=command_text, duration_ms=duration
        )
    else:
        obs.check_passed(
            check_id="preArchiveCriticalCoverage", command=command_text, duration_ms=duration
        )
    return code, output


def prepare_pre_archive_candidate_coverage(
    task: str, contract_data: dict[str, Any], *, obs: Any
) -> tuple[int, str]:
    """Produce and bind current candidate evidence for either archive lifecycle.

    ``ai-finish`` may be followed by a separate explicit archive command.  The
    same evidence is therefore required for ordinary finish and inline archive;
    archive remains an independent stale-binding verifier.
    """
    code, output = run_pre_archive_critical_coverage(contract_data, obs=obs)
    if code:
        return code, output or "pre-archive critical coverage failed"
    outcome_ok, outcome_message = bind_pre_archive_candidate_coverage_to_outcome(task)
    if not outcome_ok:
        return 1, outcome_message
    # Binding candidate coverage mutates the already-derived Outcome. Refresh
    # its human report immediately so the report digest and next action remain
    # bound to the exact persisted Outcome before either finish or archive
    # returns control to a human.
    summary_path = PROJECT_ROOT / task_paths(task)[1]
    report_ok, report_message = run_human_report_pipeline(task, summary_path)
    if not report_ok:
        return 1, f"post-candidate Human Benefit Report refresh failed: {report_message}"
    return 0, ""


def bind_pre_archive_candidate_coverage_to_outcome(task: str) -> tuple[bool, str]:
    """Project the successful candidate report into the already-derived Outcome."""
    from ai_check_task_outcome import validate_outcome
    from ai_render_task_outcome import render_task_outcome

    report_path = PROJECT_ROOT / "target" / "changed-critical-coverage.json"
    json_path, markdown_path = _outcome_paths(task)
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
        contract_path = PROJECT_ROOT / ".ai" / "work-items" / "active" / f"{task}.contract.json"
        contract = load_json(contract_path) if contract_path.is_file() else None
        outcome = load_json(json_path)
        binding = report.get("binding") if isinstance(report, dict) else None
        if not isinstance(binding, dict):
            return False, "pre-archive candidate coverage report is missing a binding"
        required = ("baseCommit", "candidateHead", "candidateTreeDigest", "candidateDiffDigest")
        if any(not isinstance(binding.get(key), str) or not binding[key] for key in required):
            return False, "pre-archive candidate coverage binding is incomplete"
        bindings = outcome.get("bindings")
        if not isinstance(bindings, dict):
            return False, "Task Outcome bindings are missing"
        bindings["preArchiveCandidateCoverage"] = {
            "reportSha256": hashlib.sha256(report_bytes).hexdigest(),
            "binding": {key: binding[key] for key in required},
        }
        markdown = render_task_outcome(outcome)
        validation = validate_outcome(outcome, markdown, expected_task_id=task, contract=contract)
        if not validation.valid:
            return False, "; ".join(f"{item.code}: {item.message}" for item in validation.errors)
        save_json(json_path, outcome)
        markdown_path.write_text(markdown, encoding="utf-8")
    except (OSError, TypeError, ValueError) as exc:
        return False, str(exc)
    return True, "Task Outcome binds pre-archive candidate coverage"


def verification_priority(item: dict[str, Any]) -> int:
    check_id = verification_key(item)
    if check_id == "sourceBoundEvidence":
        return 0
    if check_id == "aiStatus":
        return 20
    if check_id == "aiStatusCheck":
        return 30
    if check_id == "aiStatusConsistency":
        return 40
    if check_id == "aiAgentRisk":
        return 50
    if check_id == "aiSummary":
        return 51
    return 10


def finish_execution_priority(item: dict[str, Any]) -> int:
    """Order ai-finish's self-referential gates around Outcome integration."""
    check_id = verification_key(item)
    if check_id == "aiSummary":
        return 100
    return verification_priority(item) + 10


STABILIZATION_CHECKS = frozenset(
    {"aiStatus", "aiStatusCheck", "aiStatusConsistency", "aiAgentRisk", "aiSummary"}
)
# Final source-bound reassessment is release evidence.  It must be requested
# by the release-stage Contract/target, not injected into every ordinary Work
# Item finish: a source-changing corrective must be able to complete its PR
# lifecycle before the post-merge reassessment can truthfully bind to HEAD.
MANDATORY_VERIFICATION_CHECKS: tuple[str, ...] = ()
CONSOLE_OUTPUT_LIMIT = 12_000


def console_output(output: str) -> str:
    """Keep terminal diagnostics bounded without weakening stored evidence."""
    if len(output) <= CONSOLE_OUTPUT_LIMIT:
        return output
    omitted = len(output) - CONSOLE_OUTPUT_LIMIT
    return (
        output[:CONSOLE_OUTPUT_LIMIT]
        + f"\n[output truncated: {omitted} character(s) retained in verification evidence]\n"
    )


def source_bound_check_required(contract_data: dict[str, Any]) -> bool:
    """Require source-bound evidence validation only for affected Work Items."""
    try:
        dependencies = load_capability_evidence_dependencies(PROJECT_ROOT)
    except EvidenceDependencyError:
        # A malformed dependency graph must fail through the registered gate,
        # rather than silently bypassing it and paying for quality first.
        return True
    if dependencies is None:
        return False
    return source_bound_evidence_is_affected(changed_paths(contract_data), dependencies)


def inject_mandatory_verification_checks(
    declared_items: list[dict[str, Any]],
    *,
    contract_data: dict[str, Any] | None = None,
) -> list[dict[str, Any]]:
    """Normalize checks and add the bounded source gate for affected changes."""
    normalized: dict[str, dict[str, Any]] = {
        check_id: {"check": check_id, "required": True}
        for check_id in MANDATORY_VERIFICATION_CHECKS
    }
    if contract_data is not None and source_bound_check_required(contract_data):
        normalized["sourceBoundEvidence"] = {
            "check": "sourceBoundEvidence",
            "required": True,
        }
    for item in declared_items:
        check_id = verification_key(item)
        if not check_id:
            continue
        current = normalized.get(check_id)
        if current is None:
            normalized[check_id] = dict(item)
            continue
        replacement = dict(item)
        replacement["required"] = current.get("required") is True or item.get("required") is True
        normalized[check_id] = replacement
    return list(normalized.values())


def _outcome_paths(task: str) -> tuple[Path, Path]:
    root = ACTIVE_DIR / task
    return root.with_suffix(".outcome.json"), root.with_suffix(".outcome.md")


def _human_report_paths() -> tuple[Path, Path]:
    root = PROJECT_ROOT / ".ai" / "cockpit" / "task_report"
    return root.with_suffix(".json"), root.with_suffix(".md")


def ensure_active_evidence_changed_files(task: str, summary_path: Path) -> None:
    """Declare the active Contract and Summary as committed Work Item evidence.

    These exact paths are intrinsic to the current Work Item. Recording them
    here lets normal staging carry the active evidence through retries and
    archive validation without asking users or legacy fixtures to force-add or
    duplicate the paths in every hand-written changedFiles list.
    """
    summary = load_json(summary_path)
    changed = summary.get("changedFiles")
    if not isinstance(changed, list):
        return
    existing = {item.get("path") for item in changed if isinstance(item, dict)}
    for path, reason in (
        (
            f".ai/work-items/active/{task}.contract.json",
            "Active Work Item Contract is committed snapshot evidence.",
        ),
        (
            f".ai/work-items/active/{task}.summary.json",
            "Active AI Change Summary is committed snapshot evidence.",
        ),
    ):
        if path not in existing:
            changed.append({"path": path, "reason": reason})
    save_json(summary_path, summary)


SOURCE_BOUND_MATRIX_RELATIVE = "docs/reference/capability-truth-matrix.json"
SOURCE_BOUND_MATRIX_MARKDOWN_RELATIVE = "docs/reference/capability-truth-matrix.md"
SOURCE_BOUND_GENERATOR_RELATIVE = "scripts/ai_capability_truth.py"
SOURCE_BOUND_JAPANESE_GENERATOR_RELATIVE = "scripts/ai_japanese_capability.py"
SOURCE_BOUND_JAPANESE_RELATIVES = (
    "docs/reference/japanese-capability-assessment.json",
    "docs/reference/japanese-capability-assessment.md",
)
SOURCE_BOUND_ALIGNMENT_GENERATOR_RELATIVE = "scripts/check_pre_release_documentation_alignment.py"
SOURCE_BOUND_ALIGNMENT_RELATIVES = (
    "docs/reference/pre-release-documentation-alignment.json",
    "docs/reference/pre-release-documentation-alignment.md",
)
SOURCE_BOUND_DECLARED_GENERATED_RELATIVES = (
    SOURCE_BOUND_MATRIX_RELATIVE,
    SOURCE_BOUND_MATRIX_MARKDOWN_RELATIVE,
    *SOURCE_BOUND_ALIGNMENT_RELATIVES,
    *SOURCE_BOUND_JAPANESE_RELATIVES,
)


def _file_sha256(path: Path) -> str:
    if not path.is_file():
        return "missing"
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _record_source_bound_generation(
    summary_path: Path,
    *,
    path: str,
    before_sha256: str,
    after_sha256: str,
) -> None:
    """Bind a source-bound generated document to the active Summary evidence."""
    _record_generated_output(
        summary_path,
        path=path,
        before_sha256=before_sha256,
        after_sha256=after_sha256,
    )
    summary = load_json(summary_path)
    alignment = summary.get("documentationAlignment")
    checks = alignment.get("checks") if isinstance(alignment, dict) else None
    if isinstance(checks, list):
        for check in checks:
            if (
                not isinstance(check, dict)
                or check.get("area") != "documentationCommandsCapability"
            ):
                continue
            evidence = check.get("evidence")
            if isinstance(evidence, list) and path not in evidence:
                evidence.append(path)
            break
    save_json(summary_path, summary)


def _record_generated_output(
    summary_path: Path,
    *,
    path: str,
    before_sha256: str,
    after_sha256: str,
) -> None:
    """Register a generated output without misclassifying its evidence type."""
    summary = load_json(summary_path)
    changed = summary.get("changedFiles")
    if not isinstance(changed, list):
        changed = []
        summary["changedFiles"] = changed
    if not any(isinstance(item, dict) and item.get("path") == path for item in changed):
        changed.append(
            {
                "path": path,
                "reason": (
                    "Generated source-bound evidence during ai_finish; "
                    f"sha256 before={before_sha256}, after={after_sha256}."
                ),
            }
        )
    generated = summary.get("generatedFiles")
    if not isinstance(generated, list):
        generated = []
        summary["generatedFiles"] = generated
    if path not in generated:
        generated.append(path)
    save_json(summary_path, summary)


def _record_existing_source_bound_outputs(
    summary_path: Path,
    paths: tuple[str, ...],
    before_sha256: dict[str, str],
) -> None:
    """Register every declared output, including unchanged projections."""
    for relative in paths:
        path = PROJECT_ROOT / relative
        if path.is_file():
            after = _file_sha256(path)
            _record_source_bound_generation(
                summary_path,
                path=relative,
                before_sha256=before_sha256.get(relative, after),
                after_sha256=after,
            )


def refresh_source_bound_evidence(*, summary_path: Path) -> tuple[int, int, str]:
    """Refresh capability evidence before the source-bound gate runs.

    The refresh is intentionally fail-closed when configured.  Repositories
    without the capability truth pair retain the historical no-op behavior.
    """
    matrix_path = PROJECT_ROOT / SOURCE_BOUND_MATRIX_RELATIVE
    generator_path = PROJECT_ROOT / SOURCE_BOUND_GENERATOR_RELATIVE
    if not matrix_path.is_file() or not generator_path.is_file():
        return (
            0,
            0,
            "sourceBoundEvidence refresh skipped: capability truth configuration is absent",
        )

    details: list[str] = []
    changed_source_bound_paths: set[str] = set()
    total_duration = 0

    def run_generator(
        relative_generator: str,
        outputs: tuple[str, ...],
        label: str,
        *,
        required_outputs: tuple[str, ...] | None = None,
    ) -> int:
        nonlocal total_duration
        before = {relative: _file_sha256(PROJECT_ROOT / relative) for relative in outputs}
        code, duration, output = run([sys.executable, relative_generator, "--write"])
        total_duration += duration
        after = {relative: _file_sha256(PROJECT_ROOT / relative) for relative in outputs}
        changed_source_bound_paths.update(
            relative for relative in outputs if before[relative] != after[relative]
        )
        details.append(
            f"{label} generated: "
            + "; ".join(
                f"{relative} sha256 before={before[relative]}, after={after[relative]}"
                for relative in outputs
            )
        )
        if output:
            details.append(output)
        _record_existing_source_bound_outputs(summary_path, outputs, before)
        required = required_outputs if required_outputs is not None else outputs
        missing = [relative for relative in required if after[relative] == "missing"]
        if code == 0 and missing:
            details.append(f"{label} refresh failed: missing output(s): {', '.join(missing)}")
            return 1
        return code

    matrix_outputs = (
        SOURCE_BOUND_MATRIX_RELATIVE,
        SOURCE_BOUND_MATRIX_MARKDOWN_RELATIVE,
    )
    code = run_generator(
        SOURCE_BOUND_GENERATOR_RELATIVE,
        matrix_outputs,
        "capability truth",
        required_outputs=(SOURCE_BOUND_MATRIX_RELATIVE,),
    )
    if code != 0:
        return code, total_duration, "\n".join(details)

    japanese_generator = PROJECT_ROOT / SOURCE_BOUND_JAPANESE_GENERATOR_RELATIVE
    if japanese_generator.is_file():
        code = run_generator(
            SOURCE_BOUND_JAPANESE_GENERATOR_RELATIVE,
            SOURCE_BOUND_JAPANESE_RELATIVES,
            "Japanese capability assessment",
        )
        if code != 0:
            return code, total_duration, "\n".join(details)

    alignment_generator = PROJECT_ROOT / SOURCE_BOUND_ALIGNMENT_GENERATOR_RELATIVE
    if alignment_generator.is_file():
        code = run_generator(
            SOURCE_BOUND_ALIGNMENT_GENERATOR_RELATIVE,
            SOURCE_BOUND_ALIGNMENT_RELATIVES,
            "pre-release documentation alignment",
        )
        if code != 0:
            return code, total_duration, "\n".join(details)

    knowledge_root = PROJECT_ROOT / ".ai" / "knowledge"
    knowledge_paths = []
    if knowledge_root.is_dir():
        index_path = knowledge_root / "index.json"
        if index_path.is_file():
            knowledge_paths.append(index_path)
        knowledge_paths.extend(sorted((knowledge_root / "work-items").glob("*.json")))
    before = {
        path.relative_to(PROJECT_ROOT).as_posix(): _file_sha256(path) for path in knowledge_paths
    }
    try:
        from ai_generate_knowledge_record import rebuild_existing_projections

        refreshed = rebuild_existing_projections(
            repo_root=PROJECT_ROOT,
            changed_paths=sorted(changed_source_bound_paths),
        )
    except (OSError, TypeError, ValueError) as exc:
        details.append(f"Implementation Knowledge refresh failed: {exc}")
        return 1, total_duration, "\n".join(details)
    for relative in refreshed:
        _record_generated_output(
            summary_path,
            path=relative,
            before_sha256=before.get(relative, "missing"),
            after_sha256=_file_sha256(PROJECT_ROOT / relative),
        )
    if refreshed:
        details.append("Implementation Knowledge projections refreshed: " + ", ".join(refreshed))

    return 0, total_duration, "\n".join(details)


def prepare_documentation_alignment_evidence(task: str, summary_path: Path) -> None:
    """Bind already-declared generated Markdown before Finish validates docs.

    A prior blocked Finish can already have produced its compact report.  The
    next Finish must recognize that declared report as documentation evidence
    before its first alignment check, rather than requiring a second retry.
    """

    summary = load_json(summary_path)
    contract_path = PROJECT_ROOT / ".ai" / "work-items" / "active" / f"{task}.contract.json"
    contract = load_json(contract_path) if contract_path.is_file() else {}
    scope = contract.get("scope", []) if isinstance(contract, dict) else []
    alignment = summary.get("documentationAlignment")
    if not isinstance(alignment, dict):
        return
    checks = alignment.get("checks")
    if not isinstance(checks, list):
        return
    report_markdown = _human_report_paths()[1].relative_to(PROJECT_ROOT).as_posix()
    outcome_markdown = _outcome_paths(task)[1].relative_to(PROJECT_ROOT).as_posix()
    documented_generated_paths = {
        report_markdown,
        outcome_markdown,
        *SOURCE_BOUND_DECLARED_GENERATED_RELATIVES,
    }
    changed = summary.get("changedFiles", [])
    declared_paths = {item.get("path") for item in changed if isinstance(item, dict)}
    for check in checks:
        if not isinstance(check, dict) or check.get("area") != "documentationCommandsCapability":
            continue
        evidence_paths = check.setdefault("evidence", [])
        if isinstance(evidence_paths, list):
            for generated_path in sorted(
                candidate
                for candidate in documented_generated_paths & declared_paths
                if included(candidate, scope)
                and (PROJECT_ROOT / candidate).is_file()
                and candidate not in evidence_paths
            ):
                evidence_paths.append(generated_path)
        break
    save_json(summary_path, summary)


def run_human_report_pipeline(task: str, summary_path: Path) -> tuple[bool, str]:
    """Generate the compact review view from the validated Task Outcome."""

    from ai_generate_human_report import generate_human_report, render_human_report

    outcome_path, _outcome_markdown_path = _outcome_paths(task)
    json_path, markdown_path = _human_report_paths()
    try:
        outcome = load_json(outcome_path)
        contract_path = PROJECT_ROOT / ".ai" / "work-items" / "active" / f"{task}.contract.json"
        contract = load_json(contract_path) if contract_path.is_file() else None
        report = generate_human_report(outcome, phase="review", contract=contract)
        save_json(json_path, report)
        markdown_path.write_text(render_human_report(report), encoding="utf-8")
        summary = load_json(summary_path)
        changed = summary.setdefault("changedFiles", [])
        contract_path = PROJECT_ROOT / ".ai" / "work-items" / "active" / f"{task}.contract.json"
        contract = load_json(contract_path) if contract_path.is_file() else {}
        scope = contract.get("scope", []) if isinstance(contract, dict) else []
        existing = {item.get("path") for item in changed if isinstance(item, dict)}
        for path, reason in (
            (json_path, "Generated machine-readable Human Benefit Review Report."),
            (markdown_path, "Generated human-readable Human Benefit Review Report."),
        ):
            relative = path.relative_to(PROJECT_ROOT).as_posix()
            if included(relative, scope) and relative not in existing:
                changed.append({"path": relative, "reason": reason})
        save_json(summary_path, summary)
        prepare_documentation_alignment_evidence(task, summary_path)
    except (OSError, KeyError, TypeError, ValueError) as exc:
        return False, str(exc)
    return True, "Human Benefit Report pipeline passed"


def refresh_active_status_after_blocked_outcome(
    contract_path: Path, summary_path: Path
) -> tuple[bool, str]:
    """Regenerate and validate status after a Finish failure changes Outcome facts.

    A successful earlier stabilization can have rendered a green completion
    projection.  Once a later Finish gate persists a blocked Outcome, leaving
    that projection in place would present contradictory lifecycle facts.  A
    refresh failure removes the obsolete generated status rather than retaining
    a stale green report; the task-bound blocked Outcome remains the recovery
    record.
    """
    status_path = PROJECT_ROOT / ".ai" / "cockpit" / "current_status.md"
    try:
        contract = contract_path.relative_to(PROJECT_ROOT).as_posix()
        summary = summary_path.relative_to(PROJECT_ROOT).as_posix()
    except ValueError as exc:
        return False, str(exc)
    commands = [
        ["make", "generate-cockpit-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status-consistency"],
    ]
    for command in commands:
        code, _duration, output = run(command)
        if code == 0:
            continue
        try:
            status_path.unlink(missing_ok=True)
        except OSError as exc:
            return False, f"{' '.join(command)} failed and stale status removal failed: {exc}"
        return False, output or f"{' '.join(command)} failed"
    return True, "active status refreshed from the blocked Outcome"


def refresh_archived_human_report(task: str) -> tuple[bool, str]:
    """Rebind the Review Report after archive rewrites Outcome evidence paths."""

    from ai_generate_human_report import generate_human_report, render_human_report

    matches = sorted(
        (PROJECT_ROOT / ".ai" / "work-items" / "archive").glob(f"*/{task}.outcome.json")
    )
    if len(matches) != 1:
        return False, f"expected exactly one archived Task Outcome for {task}, found {len(matches)}"
    json_path, markdown_path = _human_report_paths()
    try:
        contract_path = matches[0].with_name(
            matches[0].name.replace(".outcome.json", ".contract.json")
        )
        contract = load_json(contract_path) if contract_path.is_file() else None
        report = generate_human_report(
            load_json(matches[0]),
            phase="review",
            contract=contract,
        )
        save_json(json_path, report)
        markdown_path.write_text(render_human_report(report), encoding="utf-8")
    except (OSError, KeyError, TypeError, ValueError) as exc:
        return False, str(exc)
    return True, "Archived Human Benefit Report binding passed"


def _record_outcome_state(summary_path: Path, state: dict[str, Any]) -> None:
    summary = load_json(summary_path)
    status = state.get("status")
    if status == "completed":
        state.setdefault("humanStatusColor", "green")
        state.setdefault("completionFact", "All declared finish checks passed.")
        state.setdefault("recoveryCondition", "")
    elif status in {"blocked", "failed"}:
        state.setdefault("humanStatusColor", "red")
    else:
        state.setdefault("humanStatusColor", "yellow")
    summary["taskOutcome"] = state
    changed = summary.get("changedFiles")
    output_paths = (state.get("jsonPath"), state.get("markdownPath"))
    if isinstance(changed, list):
        declared = {
            item.get("path")
            for item in changed
            if isinstance(item, dict) and isinstance(item.get("path"), str)
        }
        for path in output_paths:
            if isinstance(path, str) and path not in declared:
                changed.append(
                    {
                        "path": path,
                        "reason": "Mandatory Task Outcome evidence generated by ai-finish.",
                    }
                )
    save_json(summary_path, summary)


def _sha256_json(value: Any) -> str:
    return hashlib.sha256(
        json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()


def _summary_text_list(value: Any) -> list[str]:
    if not isinstance(value, list):
        return []
    return [item.strip() for item in value if isinstance(item, str) and item.strip()]


def _observed_issue_evidence_refs(
    item: dict[str, Any], *, summary_path: Path, index: int
) -> list[dict[str, str]]:
    """Normalize one Summary observed-issue evidence list for human claims."""
    raw = item.get("evidenceRefs", item.get("evidence"))
    if not isinstance(raw, list):
        return []
    refs: list[dict[str, str]] = []
    area = item.get("area") if isinstance(item.get("area"), str) else "observed issue"
    fallback_subject = f"observedIssues[{index}] {area}"
    for value in raw:
        if isinstance(value, str) and value.strip():
            refs.append({"source": value.strip(), "subject": fallback_subject})
        elif isinstance(value, dict):
            source = value.get("source")
            subject = value.get("subject")
            if isinstance(source, str) and source.strip():
                ref = {
                    "source": source.strip(),
                    "subject": subject.strip()
                    if isinstance(subject, str) and subject.strip()
                    else fallback_subject,
                }
                digest = value.get("digest")
                if isinstance(digest, str) and re.fullmatch(r"[a-f0-9]{64}", digest):
                    ref["digest"] = digest
                refs.append(ref)
    return refs


def _observed_issue_handoff(
    observed: Any, *, summary_path: Path
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]]]:
    """Project resolved and unresolved Summary issues without inventing evidence."""
    resolved: list[dict[str, Any]] = []
    approach: list[dict[str, Any]] = []
    remaining: list[dict[str, Any]] = []
    if not isinstance(observed, list):
        return resolved, approach, remaining
    resolved_prefixes = ("resolved", "fixed", "mitigated", "accepted")
    for index, value in enumerate(observed):
        if not isinstance(value, dict):
            continue
        raw_area = value.get("area")
        area = (
            raw_area.strip() if isinstance(raw_area, str) and raw_area.strip() else "observed issue"
        )
        raw_detail = value.get("detail")
        detail = raw_detail.strip() if isinstance(raw_detail, str) else ""
        claim = detail or area or "Observed issue lacks a detail record."
        raw_status = value.get("status")
        status_text = (
            raw_status.strip()
            if isinstance(raw_status, str) and raw_status.strip()
            else "unresolved"
        )
        refs = _observed_issue_evidence_refs(value, summary_path=summary_path, index=index)
        is_resolved = status_text.lower().startswith(resolved_prefixes)
        if is_resolved and refs:
            resolved.append(
                {
                    "claim": claim,
                    "title": area,
                    "detail": f"Status: {status_text}",
                    "evidenceRefs": refs,
                    "inference": False,
                }
            )
            action = f"Resolution status: {status_text}"
            for key in ("action", "resolution", "solution"):
                candidate = value.get(key)
                if isinstance(candidate, str) and candidate.strip():
                    action = candidate.strip()
                    break
            approach.append(
                {
                    "claim": action,
                    "title": f"Resolution for {area}",
                    "evidenceRefs": refs,
                    "inference": False,
                }
            )
            continue
        if is_resolved and not refs:
            remaining.append(
                {
                    "claim": f"{claim} Status {status_text} has no evidence references; resolution is not reported as verified.",
                    "evidenceRefs": [],
                    "inference": True,
                }
            )
            continue
        remaining.append(
            {
                "claim": claim,
                "title": area,
                "detail": f"Status: {status_text}",
                "evidenceRefs": refs,
                "inference": not bool(refs),
            }
        )
    return resolved, approach, remaining


def _verification_evidence_ref(
    summary_path: Path, subject: str, item: dict[str, Any]
) -> dict[str, str]:
    ref: dict[str, str] = {
        "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
        "subject": subject,
    }
    digest = item.get("outputDigest")
    if isinstance(digest, str) and re.fullmatch(r"[a-f0-9]{64}", digest):
        ref["digest"] = digest
    return ref


def _verification_retry_projection(
    summary_path: Path,
    verification: Any,
    history: Any,
    required_checks: dict[str, bool] | None = None,
) -> tuple[list[dict[str, Any]], list[dict[str, Any]], list[dict[str, Any]], list[str]]:
    """Build stop/resolution events from append-only failed verification attempts."""
    current = (
        [item for item in verification if isinstance(item, dict)]
        if isinstance(verification, list)
        else []
    )
    prior = (
        [item for item in history if isinstance(item, dict)] if isinstance(history, list) else []
    )
    current_by_check = {
        item.get("check"): item
        for item in current
        if isinstance(item.get("check"), str) and item.get("check")
    }
    events: list[dict[str, Any]] = []
    resolved_claims: list[dict[str, Any]] = []
    resolution_claims: list[dict[str, Any]] = []
    historical_failed_checks: list[str] = []
    seen_prior: set[str] = set()

    for index, failed in enumerate(prior):
        check = failed.get("check")
        if not isinstance(check, str) or failed.get("result") != "failed":
            continue
        latest = current_by_check.get(check)
        if not isinstance(latest, dict) or latest.get("result") != "passed":
            continue
        failure_digest = _sha256_json(failed)
        if failure_digest in seen_prior:
            continue
        seen_prior.add(failure_digest)
        historical_failed_checks.append(check)
        refs = [
            _verification_evidence_ref(
                summary_path, f"verificationHistory[{index}] {check} failed", failed
            ),
            _verification_evidence_ref(summary_path, f"verification[{check}] retry passed", latest),
        ]
        stop_reason = f"{check} failed before the retry."
        recovery = f"Retry {check} after correcting the recorded failure."
        events.append(
            {
                "eventId": f"retry-stop-{check}-{index}",
                "eventType": "stop",
                "occurredAt": failed.get("executedAt", ""),
                "stage": "verification",
                "reason": stop_reason,
                "policyOrGuard": f"required verification: {check}",
                "attemptedAction": "complete the Work Item",
                "avoidedImpact": "a stale completion claim",
                "recovery": recovery,
                "state": "resolved",
                "evidence": refs,
            }
        )
        events.append(
            {
                "eventId": f"retry-resolution-{check}-{index}",
                "eventType": "resolution",
                "occurredAt": latest.get("executedAt", ""),
                "problem": stop_reason,
                "action": f"Re-ran {check} after the correction; the latest attempt passed.",
                "verification": f"{check} latest verification result is passed.",
                "state": "resolved",
                "evidence": refs,
            }
        )
        resolved_claims.append(
            {
                "claim": stop_reason,
                "title": check,
                "detail": "The earlier stop was resolved by a later passing attempt.",
                "evidenceRefs": refs,
                "inference": False,
            }
        )
        resolution_claims.append(
            {
                "claim": f"Re-ran {check} after the correction; the latest attempt passed.",
                "title": f"Resolution for {check}",
                "evidenceRefs": refs,
                "inference": False,
            }
        )

    for index, failed in enumerate(current):
        check = failed.get("check")
        if not isinstance(check, str) or failed.get("result") != "failed":
            continue
        if required_checks is not None and required_checks.get(check, True) is not True:
            continue
        refs = [_verification_evidence_ref(summary_path, f"verification[{check}] failed", failed)]
        events.append(
            {
                "eventId": f"current-stop-{check}-{index}",
                "eventType": "stop",
                "occurredAt": failed.get("executedAt", ""),
                "stage": "verification",
                "reason": f"{check} failed on the latest attempt.",
                "policyOrGuard": f"required verification: {check}",
                "attemptedAction": "complete the Work Item",
                "avoidedImpact": "an unsupported completion claim",
                "recovery": f"Run a passing {check} retry.",
                "state": "unresolved",
                "evidence": refs,
            }
        )

    return events, resolved_claims, resolution_claims, historical_failed_checks


def _pre_merge_outcome_input(
    task: str, contract_path: Path, summary_path: Path, language: str | None = "en"
) -> dict[str, Any]:
    """Derive truthful Outcome evidence before a provider PR can exist."""
    contract = load_json(contract_path)
    summary = load_json(summary_path)
    from ai_render_task_outcome_multilingual import normalize_locale

    if not language:
        raise ValueError("conversation language is required for human Outcome delivery")
    locale = normalize_locale(language)
    head_commit = current_head()
    base_commit = contract.get("baseCommit")
    if not isinstance(base_commit, str) or len(base_commit) != 40:
        raise ValueError("Contract baseCommit is required for mandatory Task Outcome")
    if len(head_commit) != 40:
        raise ValueError("current HEAD is required for mandatory Task Outcome")
    changed = summary.get("changedFiles", [])
    delivered = [
        item["path"]
        for item in changed
        if isinstance(item, dict) and isinstance(item.get("path"), str)
    ]
    non_risk_explanations = [
        dict(item) for item in summary.get("nonRiskExplanations", []) if isinstance(item, dict)
    ]
    verification = summary.get("verification", [])
    declared_required = {
        item["check"]: item.get("required", True) is True
        for item in contract.get("verification", [])
        if isinstance(item, dict) and isinstance(item.get("check"), str)
    }

    def is_required_check(item: dict[str, Any]) -> bool:
        check = item.get("check")
        return declared_required.get(check, True)

    optional_failed_checks = [
        item
        for item in verification
        if isinstance(item, dict)
        and item.get("result") == "failed"
        and item.get("check")
        and not is_required_check(item)
    ]
    non_risk_explanations.extend(
        {
            "sourceWarning": f"{item['check']} failed on the latest attempt.",
            "reason": (
                "The Contract explicitly declares this verification check as optional; "
                "the failure remains visible but does not block the required acceptance boundary."
            ),
            "evidence": [
                {
                    "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": f"verification[{item['check']}] failed",
                },
                {
                    "source": contract_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": f"verification[{item['check']}].required=false",
                },
            ],
        }
        for item in optional_failed_checks
    )
    evidenced_non_risk_warnings = {
        item["sourceWarning"]
        for item in non_risk_explanations
        if isinstance(item.get("sourceWarning"), str)
        and isinstance(item.get("evidence"), list)
        and item["evidence"]
        and all(
            isinstance(reference, dict)
            and isinstance(reference.get("source"), str)
            and reference["source"].strip()
            and isinstance(reference.get("subject"), str)
            and reference["subject"].strip()
            for reference in item["evidence"]
        )
    }
    residual_risks = summary.get("residualRisks", [])
    merge_identity_risk_is_explicit = isinstance(residual_risks, list) and any(
        isinstance(item, dict) and item.get("area") == "merge_identity" for item in residual_risks
    )
    warnings = [
        item
        for item in summary.get("knownGaps", [])
        if isinstance(item, str)
        and item not in evidenced_non_risk_warnings
        and not (
            merge_identity_risk_is_explicit and item.startswith(PRE_MERGE_MERGE_IDENTITY_GAP_PREFIX)
        )
    ]
    limitations = [
        {
            "sourceWarning": warning,
            "title": "Unresolved evidence is explicitly limited",
            "affectedClaims": ["completion_claim"],
            "requiredEvidence": ["fresh verification evidence"],
            "forbiddenClaims": ["Do not claim this warning was verified or resolved."],
        }
        for warning in warnings
    ]
    known_gap_non_risk_explanations = [
        {
            "sourceWarning": warning,
            "reason": "The Summary records this item as an unresolved gap rather than a verified result.",
            "evidence": [],
        }
        for warning in warnings
    ]
    human_decisions = [
        item["instruction"]
        for item in summary.get("userCorrectionsCaptured", [])
        if isinstance(item, dict) and isinstance(item.get("instruction"), str)
    ]
    completed = [
        {
            "title": f"Changed {item.get('path')}",
            "detail": item.get("reason", "The declared Work Item change was recorded."),
            "evidence": [
                {
                    "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": item.get("path", "changed file"),
                }
            ],
        }
        for item in changed
        if isinstance(item, dict) and isinstance(item.get("path"), str)
    ]
    passed_checks = [
        {
            "title": str(item.get("check")),
            "detail": item.get("outputSummary") or f"{item.get('check')} passed.",
            "evidence": [
                {
                    "source": item.get(
                        "executionContractPath", summary_path.relative_to(PROJECT_ROOT).as_posix()
                    ),
                    "subject": item.get("check", "verification"),
                    **(
                        {"digest": item["outputDigest"]}
                        if isinstance(item.get("outputDigest"), str)
                        else {}
                    ),
                }
            ],
        }
        for item in verification
        if isinstance(item, dict) and item.get("result") == "passed"
    ]
    retained = [
        {
            "title": "Retained limitation",
            "detail": warning,
            "evidence": [
                {
                    "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": "knownGaps",
                }
            ],
        }
        for warning in warnings
    ]
    residual_risks = [
        {
            "severity": item.get("level", "medium"),
            "title": item.get("area", "Residual risk"),
            "detail": item.get("detail", "Residual risk remains under review."),
            "state": "unresolved",
            "evidence": [
                {
                    "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": "residualRisks",
                }
            ],
            **{
                key: item[key]
                for key in (
                    "sourceWarning",
                    "affectedClaims",
                    "requiredEvidence",
                    "decisionOwner",
                    "mitigation",
                    "acceptanceStatus",
                    "blockingFor",
                )
                if key in item
            },
        }
        for item in summary.get("residualRisks", [])
        if isinstance(item, dict)
    ]
    failed_checks = [
        item.get("check")
        for item in verification
        if isinstance(item, dict)
        and item.get("result") == "failed"
        and item.get("check")
        and is_required_check(item)
    ]
    retry_events, retry_resolved, retry_approach, historical_failed_checks = (
        _verification_retry_projection(
            summary_path,
            verification,
            summary.get("verificationHistory", []),
            declared_required,
        )
    )
    failed_check_claims = []
    for item in verification if isinstance(verification, list) else []:
        if not isinstance(item, dict) or item.get("result") != "failed":
            continue
        if not is_required_check(item):
            continue
        check = item.get("check")
        if not isinstance(check, str) or not check:
            continue
        failed_check_claims.append(
            {
                "claim": f"{check} failed on the latest attempt.",
                "evidenceRefs": [
                    _verification_evidence_ref(summary_path, f"verification[{check}] failed", item)
                ],
                "inference": False,
            }
        )
    observed_resolved, observed_approach, observed_remaining = _observed_issue_handoff(
        summary.get("observedIssues"), summary_path=summary_path
    )
    observed_resolved.extend(retry_resolved)
    observed_approach.extend(retry_approach)
    observed_resolutions: list[dict[str, Any]] = []
    observed_issues = summary.get("observedIssues")
    if isinstance(observed_issues, list):
        for index, issue in enumerate(observed_issues):
            if not isinstance(issue, dict):
                continue
            status_text = issue.get("status")
            if not isinstance(status_text, str) or not status_text.lower().startswith(
                ("resolved", "fixed", "mitigated", "accepted")
            ):
                continue
            refs = _observed_issue_evidence_refs(issue, summary_path=summary_path, index=index)
            if not refs:
                continue
            area = issue.get("area") if isinstance(issue.get("area"), str) else "observed issue"
            detail = issue.get("detail") if isinstance(issue.get("detail"), str) else area
            detail_text = detail.strip() if isinstance(detail, str) else area
            action = f"Resolution status: {status_text}"
            for key in ("action", "resolution", "solution"):
                candidate = issue.get(key)
                if isinstance(candidate, str) and candidate.strip():
                    action = candidate.strip()
                    break
            observed_resolutions.append(
                {
                    "problem": detail_text or area,
                    "action": action,
                    "verification": "Evidence review",
                    "result": "resolved",
                    "evidenceRefs": refs,
                    "evidence": refs,
                }
            )
    remaining_issue_claims = [
        *observed_remaining,
        *[
            {
                "claim": item["detail"],
                "title": item["area"],
                "evidenceRefs": [
                    {
                        "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                        "subject": "residualRisks",
                    }
                ],
                "inference": False,
            }
            for item in summary.get("residualRisks", [])
            if isinstance(item, dict)
            and isinstance(item.get("detail"), str)
            and isinstance(item.get("area"), str)
        ],
    ]
    problem_count_refs = []
    if isinstance(summary.get("observedIssues"), list) and summary.get("observedIssues"):
        problem_count_refs.append(
            {
                "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                "subject": "observedIssues",
            }
        )
    if warnings:
        problem_count_refs.append(
            {
                "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                "subject": "knownGaps",
            }
        )
    if failed_checks or historical_failed_checks:
        problem_count_refs.append(
            {
                "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                "subject": "verification and verificationHistory",
            }
        )
    user_decisions: list[str] = []
    for item in summary.get("userCorrectionsCaptured", []):
        if isinstance(item, str) and item.strip():
            user_decisions.append(item.strip())
        elif isinstance(item, dict) and isinstance(item.get("instruction"), str):
            user_decisions.append(item["instruction"])
    approach_projection: dict[str, Any] = {}
    has_new_approach_contract_signal = isinstance(contract.get("rawUserRequest"), str) and bool(
        contract["rawUserRequest"].strip()
    )
    if (
        not has_new_approach_contract_signal
        and not isinstance(summary.get("implementationApproach"), dict)
        and not isinstance(summary.get("configurationApproach"), dict)
    ):
        # Historic Contracts predate the applicability signal. Preserve their
        # terminal behavior while making the projection explicit; new v2
        # Contracts carry rawUserRequest and therefore remain fail-visible when
        # a code/config approach is missing.
        from ai_generate_task_outcome import _not_applicable_approach

        approach_projection["implementationApproach"] = _not_applicable_approach()
    problem_count = (
        len(summary.get("observedIssues", []))
        + len(warnings)
        + len(failed_checks)
        + len(historical_failed_checks)
    )
    return {
        "taskId": task,
        "bindings": {
            "taskId": task,
            "contractDigest": hashlib.sha256(contract_path.read_bytes()).hexdigest(),
            "summaryDigest": hashlib.sha256(summary_path.read_bytes()).hexdigest(),
            "verificationDigest": _sha256_json(verification),
            "baseCommit": base_commit,
            "headCommit": head_commit,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "repository-governance",
            "generatorVersion": "1.2",
            "locale": locale,
        },
        "evidence": {
            "deliveredChanges": delivered,
            "locale": locale,
            "completed": completed,
            "passedChecks": passed_checks,
            "retained": retained,
            "handoffRisks": residual_risks,
            "resolutions": observed_resolutions,
            "events": retry_events,
            "handoffQuestions": {
                "problemCount": problem_count,
                "problemCountEvidenceRefs": problem_count_refs,
                "blockedProblems": failed_check_claims,
                "resolvedProblems": observed_resolved,
                "resolutionApproach": observed_approach,
                "avoidedRisks": _summary_text_list(summary.get("avoidedRisks")),
                "remainingRisks": remaining_issue_claims,
                "agentUnknowns": [
                    *[item for item in contract.get("unknowns", []) if isinstance(item, str)],
                    *[
                        item
                        for item in summary.get("unknownsRemaining", [])
                        if isinstance(item, str)
                    ],
                ],
                "humanConfirmations": user_decisions,
                "recurrenceLikelihood": "unknown: no direct recurrence probability evidence was recorded.",
                "nextTime": "Bind conversation locale and preserve evidence details before the next Work Item starts.",
            },
            "warnings": warnings,
            "limitations": limitations,
            "nonRiskExplanations": [
                *non_risk_explanations,
                *known_gap_non_risk_explanations,
            ],
            "forbiddenClaims": ["Do not claim an unresolved warning was verified or resolved."]
            if warnings
            else [],
            "humanDecisions": [*human_decisions, *user_decisions],
            **approach_projection,
            "sources": [
                {
                    "source": contract_path.relative_to(PROJECT_ROOT).as_posix(),
                    "subject": "Contract",
                },
                {"source": summary_path.relative_to(PROJECT_ROOT).as_posix(), "subject": "Summary"},
            ],
        },
    }


def _write_and_validate_pre_merge_outcome(
    task: str,
    contract_path: Path,
    summary_path: Path,
    json_path: Path,
    markdown_path: Path,
    language: str | None = "en",
) -> tuple[bool, str]:
    from ai_check_task_outcome import validate_outcome
    from ai_generate_task_outcome import generate_outcome
    from ai_render_task_outcome import render_task_outcome

    try:
        payload = _pre_merge_outcome_input(task, contract_path, summary_path, language)
        outcome = generate_outcome(
            task,
            payload["bindings"],
            events=payload["evidence"].get("events", []),
            evidence=payload["evidence"],
        )
        markdown = render_task_outcome(outcome)
        report = validate_outcome(
            outcome,
            markdown,
            expected_task_id=task,
            contract=load_json(contract_path),
        )
        if not report.valid:
            return False, "; ".join(f"{item.code}: {item.message}" for item in report.errors)
        save_json(json_path, outcome)
        markdown_path.write_text(markdown, encoding="utf-8")
    except (OSError, ValueError) as exc:
        return False, str(exc)
    return True, "Outcome pipeline passed"


def write_blocked_outcome(
    task: str,
    contract_path: Path,
    summary_path: Path,
    *,
    failed_check: str,
    failure_message: str,
    language: str | None = "en",
) -> tuple[bool, str]:
    """Persist a valid blocked Outcome, then derive its exact review report.

    The Outcome is the recovery fact.  Its report is deliberately derived only
    after the Outcome has been persisted, so report-generation failure cannot
    erase the usable blocked record.  The caller still fails closed when this
    function returns ``False``.
    """
    from ai_check_task_outcome import validate_outcome
    from ai_generate_task_outcome import generate_outcome
    from ai_render_task_outcome import render_task_outcome

    json_path, markdown_path = _outcome_paths(task)
    message = outcome_failure_message(failed_check, failure_message)
    try:
        payload = _pre_merge_outcome_input(task, contract_path, summary_path, language)
        evidence = dict(payload["evidence"])
        warnings = list(evidence.get("warnings", []))
        warnings.append(message)
        evidence["warnings"] = warnings
        evidence["status"] = "blocked"
        evidence["failedGate"] = failed_check
        evidence["recoveryCondition"] = f"Run a passing {failed_check} retry."
        evidence["redReasons"] = [
            {
                "gate": failed_check,
                "cause": message,
                "location": f"verification:{failed_check}",
                "recovery": evidence["recoveryCondition"],
                "evidence": [
                    {
                        "source": summary_path.relative_to(PROJECT_ROOT).as_posix(),
                        "subject": failed_check,
                    }
                ],
            }
        ]
        evidence["limitations"] = [
            *list(evidence.get("limitations", [])),
            {
                "sourceWarning": message,
                "title": "Finish verification is blocked",
                "affectedClaims": ["completion_claim", "archive_readiness"],
                "requiredEvidence": [f"a passing {failed_check} retry"],
                "forbiddenClaims": ["Do not claim this Work Item is complete or archive-ready."],
            },
        ]
        evidence["nonRiskExplanations"] = [
            *list(evidence.get("nonRiskExplanations", [])),
            {
                "sourceWarning": message,
                "reason": "The failed Finish gate is recorded as a recovery condition, not a completed result.",
                "evidence": [],
            },
        ]
        evidence["forbiddenClaims"] = [
            *list(evidence.get("forbiddenClaims", [])),
            "Do not claim a blocked Work Item has completed verification or may be archived.",
        ]
        outcome = generate_outcome(
            task,
            payload["bindings"],
            events=payload["evidence"].get("events", []),
            evidence=evidence,
        )
        markdown = render_task_outcome(outcome)
        report = validate_outcome(
            outcome,
            markdown,
            expected_task_id=task,
            contract=load_json(contract_path),
        )
        if not report.valid:
            return False, "; ".join(f"{item.code}: {item.message}" for item in report.errors)
        save_json(json_path, outcome)
        markdown_path.write_text(markdown, encoding="utf-8")
        _record_outcome_state(
            summary_path,
            {
                "status": "blocked",
                "jsonPath": json_path.relative_to(PROJECT_ROOT).as_posix(),
                "markdownPath": markdown_path.relative_to(PROJECT_ROOT).as_posix(),
                "rawEvidencePath": "derived:blocked_finish",
                "failedCheck": failed_check,
                "error": failure_message,
            },
        )
    except (OSError, KeyError, TypeError, ValueError) as exc:
        return False, str(exc)

    report_ok, report_message = run_human_report_pipeline(task, summary_path)
    if not report_ok:
        return (
            False,
            f"blocked Outcome persisted but Human Benefit Report refresh failed: {report_message}",
        )
    status_ok, status_message = refresh_active_status_after_blocked_outcome(
        contract_path, summary_path
    )
    if not status_ok:
        return (
            False,
            f"blocked Outcome persisted but active status refresh failed: {status_message}",
        )
    return True, "blocked Outcome and Human Benefit Report persisted"


def outcome_failure_message(failed_check: str, failure_message: str) -> str:
    """Keep persisted blocked-Outcome text actionable and validator-safe.

    Detailed command output remains in the Summary verification record.  The
    Outcome is a human decision surface and deliberately carries only the gate
    identity and recovery direction when a tool error contains raw metrics.
    """
    normalized = " ".join(failure_message.split())
    if re.search(r"\b\d+(?:\.\d+)?\s*%", normalized):
        return (
            f"Finish blocked at {failed_check}: verification threshold was not met; "
            "inspect recorded verification output and rerun the gate."
        )
    return f"Finish blocked at {failed_check}: {normalized}"


def failed_check_from_summary(summary_path: Path, fallback: str) -> str:
    """Return the most recently recorded failed check without guessing success."""
    try:
        verification = load_json(summary_path).get("verification", [])
    except (OSError, ValueError, TypeError):
        return fallback
    if not isinstance(verification, list):
        return fallback
    for item in reversed(verification):
        if isinstance(item, dict) and item.get("result") == "failed":
            check = item.get("check")
            if isinstance(check, str) and check:
                return check
    return fallback


def documentation_alignment_issues(summary_path: Path, contract_data: dict[str, Any]) -> list[str]:
    """Return archive-required documentation-alignment defects without mutation.

    A completed Outcome is an archive prerequisite.  Revalidate the active
    Summary even when `--archive` reuses a prior Finish attestation because the
    Summary itself is intentionally excluded from that attestation's worktree
    digest to avoid a self-reference cycle.
    """
    from ai_check_summary import validate_documentation_alignment

    try:
        return validate_documentation_alignment(load_json(summary_path), contract_data)
    except (OSError, TypeError, ValueError) as exc:
        return [f"documentationAlignment could not be validated: {exc}"]


def return_blocked_finish_failure(
    *,
    task: str,
    contract_path: Path,
    summary_path: Path,
    failed_check: str,
    failure_message: str,
    code: int,
    language: str | None = None,
) -> int:
    """Fail closed while retaining the standard blocked recovery evidence."""
    blocked_ok, blocked_message = write_blocked_outcome(
        task,
        contract_path,
        summary_path,
        failed_check=failed_check,
        failure_message=failure_message,
        language=language or CURRENT_REPORT_LANGUAGE,
    )
    if blocked_ok:
        print(f"Blocked Task Outcome persisted: {blocked_message}", file=sys.stderr)
        report_ok, report_message = deliver_direct_outcome_report(
            task, language or CURRENT_REPORT_LANGUAGE
        )
        if not report_ok:
            print(
                "ERROR: active Task Outcome conversation delivery failed: " + report_message,
                file=sys.stderr,
            )
    else:
        print(
            "ERROR: blocked Task Outcome/report recovery failed: " + blocked_message,
            file=sys.stderr,
        )
    return code if code else 1


def run_task_outcome_pipeline(
    task: str,
    summary_path: Path,
    contract_path: Path | None = None,
    language: str | None = "en",
) -> tuple[bool, str]:
    """Generate a mandatory pre-merge Outcome or validate explicit raw evidence."""
    summary = load_json(summary_path)
    input_value = summary.get("taskOutcomeInput")
    if not isinstance(input_value, str) or not input_value:
        if contract_path is None:
            return False, "mandatory Task Outcome requires the active Contract"
        json_path, markdown_path = _outcome_paths(task)
        ok, message = _write_and_validate_pre_merge_outcome(
            task, contract_path, summary_path, json_path, markdown_path, language
        )
        if not ok:
            _record_outcome_state(summary_path, {"status": "failed", "error": message})
            return False, message
        outcome = load_json(json_path)
        sections = outcome.get("sections", {})
        evidence_count = len(sections.get("evidence", [])) if isinstance(sections, dict) else 0
        _record_outcome_state(
            summary_path,
            {
                "status": outcome.get("status", "unknown"),
                "jsonPath": json_path.relative_to(PROJECT_ROOT).as_posix(),
                "markdownPath": markdown_path.relative_to(PROJECT_ROOT).as_posix(),
                "rawEvidencePath": "derived:pre_merge",
                "evidenceCount": evidence_count,
            },
        )
        # Recording taskOutcome also declares the generated Outcome files in
        # Summary.changedFiles.  Rebind once after that bookkeeping mutation;
        # otherwise the freshly written Outcome can never satisfy the
        # terminal Summary digest gate.
        rebound_ok, rebound_message = _write_and_validate_pre_merge_outcome(
            task, contract_path, summary_path, json_path, markdown_path, language
        )
        if not rebound_ok:
            _record_outcome_state(summary_path, {"status": "failed", "error": rebound_message})
            return False, rebound_message
        return True, rebound_message
    input_path = PROJECT_ROOT / input_value
    json_path, markdown_path = _outcome_paths(task)
    if not input_path.exists():
        message = f"raw Evidence input does not exist: {input_value}"
        _record_outcome_state(
            summary_path, {"status": "failed", "rawEvidencePath": input_value, "error": message}
        )
        return False, message

    python = sys.executable
    contract_argument = (
        str(contract_path.relative_to(PROJECT_ROOT)) if contract_path is not None else ""
    )
    commands = [
        [
            python,
            "scripts/ai_generate_task_outcome.py",
            input_value,
            str(json_path.relative_to(PROJECT_ROOT)),
            str(markdown_path.relative_to(PROJECT_ROOT)),
        ],
        [
            python,
            "-c",
            "from pathlib import Path; import sys; sys.path.insert(0, 'scripts'); from ai_check_task_outcome import validate_outcome; import json; outcome=json.loads(Path(sys.argv[1]).read_text()); contract=json.loads(Path(sys.argv[4]).read_text()) if sys.argv[4] else None; report=validate_outcome(outcome, expected_task_id=sys.argv[3], contract=contract); print('valid' if report.valid else '\\n'.join(f'{e.code}: {e.message}' for e in report.errors)); raise SystemExit(0 if report.valid else 1)",
            str(json_path.relative_to(PROJECT_ROOT)),
            str(markdown_path.relative_to(PROJECT_ROOT)),
            task,
            contract_argument,
        ],
        [
            python,
            "scripts/ai_render_task_outcome.py",
            str(json_path.relative_to(PROJECT_ROOT)),
            str(markdown_path.relative_to(PROJECT_ROOT)),
        ],
        [
            python,
            "-c",
            "from pathlib import Path; import sys; sys.path.insert(0, 'scripts'); from ai_check_task_outcome import validate_outcome; import json; outcome=json.loads(Path(sys.argv[1]).read_text()); contract=json.loads(Path(sys.argv[4]).read_text()) if sys.argv[4] else None; report=validate_outcome(outcome, Path(sys.argv[2]).read_text(), expected_task_id=sys.argv[3], contract=contract); print('valid' if report.valid else '\\n'.join(f'{e.code}: {e.message}' for e in report.errors)); raise SystemExit(0 if report.valid else 1)",
            str(json_path.relative_to(PROJECT_ROOT)),
            str(markdown_path.relative_to(PROJECT_ROOT)),
            task,
            contract_argument,
        ],
    ]
    for command in commands:
        code, _, output = run(command)
        if code != 0:
            message = " ".join((output or "Outcome pipeline command failed").split())[:500]
            _record_outcome_state(
                summary_path,
                {"status": "failed", "rawEvidencePath": input_value, "error": message},
            )
            return False, message
    outcome = json.loads(json_path.read_text(encoding="utf-8"))
    sections = outcome.get("sections", {})
    evidence_count = len(sections.get("evidence", [])) if isinstance(sections, dict) else 0
    _record_outcome_state(
        summary_path,
        {
            "status": outcome.get("status", "unknown"),
            "jsonPath": json_path.relative_to(PROJECT_ROOT).as_posix(),
            "markdownPath": markdown_path.relative_to(PROJECT_ROOT).as_posix(),
            "rawEvidencePath": input_value,
            "evidenceCount": evidence_count,
        },
    )
    return True, "Outcome pipeline passed"


def refresh_final_outcome_after_stabilization(
    task: str, contract_path: Path, summary_path: Path, language: str
) -> tuple[bool, str]:
    """Rebuild human projections from the final verification state before archive."""
    # The Human Benefit Report pipeline records its own generated paths in the
    # Summary.  Establish that final Summary shape before binding the Outcome
    # digest; generating Outcome first would make the report path mutation
    # immediately stale the terminal binding and trip the green gate.
    report_ok, report_message = run_human_report_pipeline(task, summary_path)
    if not report_ok:
        return False, f"final Human Benefit Report regeneration failed: {report_message}"
    outcome_ok, outcome_message = run_task_outcome_pipeline(
        task, summary_path, contract_path, language
    )
    if not outcome_ok:
        return False, f"final Outcome regeneration failed: {outcome_message}"
    contract = contract_path.relative_to(PROJECT_ROOT).as_posix()
    summary = summary_path.relative_to(PROJECT_ROOT).as_posix()
    for command in (
        ["make", "check-ai-change-summary", f"SUMMARY={summary}", f"CONTRACT={contract}"],
        ["make", "generate-cockpit-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status-consistency"],
    ):
        code, _duration, output = run(command)
        if code != 0:
            return False, output or f"final projection validation failed: {' '.join(command)}"
    return True, "final Outcome and Human Benefit Report regenerated from stabilized verification"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run AI Work Item finish checks.")
    parser.add_argument("--task", required=True)
    parser.add_argument(
        "--skip-quality", action="store_true", help="Skip the project quality gate."
    )
    parser.add_argument(
        "--archive",
        action=argparse.BooleanOptionalAction,
        default=False,
        help="Archive only after the agent has relayed the active Task Outcome to the human.",
    )
    parser.add_argument(
        "--language",
        default=None,
        help="Explicit conversation language for the direct human report (en, zh-CN, or ja).",
    )
    return parser.parse_args()


def render_direct_outcome_report(outcome: dict[str, Any], language: str) -> str:
    """Render the active Outcome and the explicit archive boundary for the human."""
    from ai_generate_human_report import generate_human_report, render_human_report
    from ai_render_task_outcome_multilingual import normalize_locale, render_localized_outcome

    locale = normalize_locale(language)
    heading, limitation, next_action = REPORT_BOUNDARY_TEXT[locale]
    status_color = outcome.get("humanStatusColor")
    traffic_light = {
        "green": "🟢",
        "yellow": "🟡",
        "red": "🔴",
    }.get(status_color if isinstance(status_color, str) else "", "🔴")
    # A conversation delivery is only valid when both the localized Outcome
    # and the complete human-benefit report are renderable.  Do not silently
    # downgrade to a status-only or localized-only surface: that would make a
    # missing report look like a successful human handoff.
    contract = None
    work_item_id = outcome.get("workItemId")
    if isinstance(work_item_id, str):
        contract_path = (
            PROJECT_ROOT / ".ai" / "work-items" / "active" / f"{work_item_id}.contract.json"
        )
        if contract_path.is_file():
            contract = load_json(contract_path)
    human_summary = render_human_report(generate_human_report(outcome, contract=contract))
    return (
        f"Outcome: {traffic_light} {outcome.get('status', 'unknown')}\n"
        f"{heading}\n{render_localized_outcome(outcome, locale)}\n"
        f"{human_summary}{limitation}\n{next_action}\n"
    )


def active_terminal_outcome_issues(
    task: str, contract_path: Path, summary_path: Path
) -> tuple[str, ...]:
    """Return terminal-gate issues for the current active candidate."""
    from ai_outcome_gate import validate_terminal_outcome

    outcome_path, outcome_markdown_path = _outcome_paths(task)
    result = validate_terminal_outcome(
        outcome_path,
        outcome_markdown_path,
        expected_task_id=task,
        contract_path=contract_path,
        summary_path=summary_path,
        expected_base_commit=load_json(contract_path).get("baseCommit"),
        expected_head_commit=current_head(),
    )
    return result.issues


def deliver_direct_outcome_report(task: str, language: str) -> tuple[bool, str]:
    """Relay the persisted active Outcome to the conversation-facing stream."""

    outcome_path, _ = _outcome_paths(task)
    try:
        print(render_direct_outcome_report(load_json(outcome_path), language), end="")
    except (OSError, TypeError, ValueError) as exc:
        return False, str(exc)
    return True, "active Task Outcome delivered"


def finish_quality_paths(contract_data: dict[str, Any]) -> list[str]:
    """Exclude only this Work Item's generated projections from risk routing."""
    task = str(contract_data.get("workItemId", ""))
    generated = {
        ".ai/cockpit/current_status.md",
        ".ai/cockpit/task_report.json",
        ".ai/cockpit/task_report.md",
        f".ai/work-items/starts/{task}.json",
        f".ai/work-items/active/{task}.contract.json",
        f".ai/work-items/active/{task}.summary.json",
        f".ai/work-items/active/{task}.outcome.json",
        f".ai/work-items/active/{task}.outcome.md",
    }
    return [path for path in changed_paths(contract_data) if path not in generated]


def immutable_pin_facts_for_finish(
    contract_data: dict[str, Any], paths: list[str]
) -> dict[str, Any] | None:
    """Bind immutable-pin routing to the active Contract base and current file."""
    if len(paths) != 1:
        return None
    base = str(contract_data.get("baseCommit", ""))
    path = paths[0]
    if not base:
        return {
            "path": path,
            "kind": "immutable_workflow_pin",
            "eligible": False,
            "reason": "Contract baseCommit is unavailable",
            "replacementCount": 0,
        }
    try:
        base_result = subprocess.run(  # nosec B603 B607 - fixed list-form Git evidence lookup
            ["git", "show", f"{base}:{path}"],
            cwd=PROJECT_ROOT,
            env=clean_git_environment(),
            text=True,
            capture_output=True,
            check=False,
        )
        if base_result.returncode != 0:
            raise ValueError(base_result.stderr.strip() or "base file is unavailable")
        current_path = PROJECT_ROOT / path
        if not current_path.is_file():
            raise ValueError("current file is unavailable")
        return classify_immutable_workflow_pin_change(
            path,
            base_result.stdout,
            current_path.read_text(encoding="utf-8"),
        )
    except (OSError, ValueError) as exc:
        return {
            "path": path,
            "kind": "immutable_workflow_pin",
            "eligible": False,
            "reason": f"base/current evidence unavailable: {exc}",
            "replacementCount": 0,
        }


def run_declared_checks(
    declared_items: list[dict[str, Any]],
    *,
    args: argparse.Namespace,
    contract: str,
    summary: str,
    contract_data: dict[str, Any],
    contract_path: Path,
    summary_path: Path,
    contract_hash: str,
    commit_sha: str,
    obs: Any,
) -> int:
    """Run declared checks and persist transactional verification evidence."""
    for item in declared_items:
        if not verification_key(item) or "command" in item:
            print(
                "ERROR: contractVersion 2 verification must use registered check IDs only",
                file=sys.stderr,
            )
            return 2
    transactional_markers_written = False
    outcome_requested = True
    for item in declared_items:
        check_id = verification_key(item)
        # These checks attest self-referential Summary/Status artifacts.  They
        # run together after ordinary verification has been recorded, where
        # each state write can be followed by a fresh Status projection.
        if check_id in STABILIZATION_CHECKS:
            continue
        if args.skip_quality and check_id == "quality":
            if item.get("required") is True:
                print(
                    "ERROR: --skip-quality cannot skip required Contract verification",
                    file=sys.stderr,
                )
                return 2
            continue
        route: dict[str, Any] | None = None
        try:
            if check_id == "quality":
                governance_profile = contract_data.get("governanceProfile", {})
                quality_paths = finish_quality_paths(contract_data)
                route = finish_quality_route_for_contract(
                    quality_paths,
                    governance_profile if isinstance(governance_profile, dict) else None,
                    immutable_pin_facts=immutable_pin_facts_for_finish(
                        contract_data, quality_paths
                    ),
                )
                cmd_str = str(route["command"])
                command = shlex.split(cmd_str)
            else:
                cmd_str, command = render_check_command(
                    check_id, contract_path=contract, summary_path=summary
                )
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2
        obs.check_started(check_id=check_id, command=cmd_str)
        # Outcome-enabled Summaries run aiSummary before Status. Pre-writing
        # pending markers for later self-referential checks would make aiSummary
        # reject its own Summary as incomplete; stabilization records them after
        # their real execution.
        if (
            not outcome_requested
            and not transactional_markers_written
            and verification_priority(item) >= 20
        ):
            current_digest = worktree_digest(changed_paths(contract_data))
            for candidate in declared_items:
                if verification_priority(candidate) >= 20:
                    candidate_id = verification_key(candidate)
                    candidate_command, _ = render_check_command(
                        candidate_id, contract_path=contract, summary_path=summary
                    )
                    record_result(
                        summary_path,
                        pending_evidence(
                            candidate_id,
                            candidate_command,
                            contract_hash=contract_hash,
                            commit_sha=commit_sha,
                            execution_contract_path=contract,
                            execution_summary_path=summary,
                            worktree_digest=current_digest,
                        ),
                    )
            transactional_markers_written = True
        if check_id == "aiSummary":
            current_digest = worktree_digest(changed_paths(contract_data))
            record_result(
                summary_path,
                evidence(
                    check_id,
                    cmd_str,
                    0,
                    0,
                    "pending transactional validation",
                    contract_hash=contract_hash,
                    commit_sha=commit_sha,
                    execution_contract_path=contract,
                    execution_summary_path=summary,
                    worktree_digest=current_digest,
                ),
            )
        refresh_output = ""
        refresh_duration = 0
        if check_id == "sourceBoundEvidence":
            refresh_code, refresh_duration, refresh_output = refresh_source_bound_evidence(
                summary_path=summary_path
            )
            if refresh_code != 0:
                current_digest = worktree_digest(changed_paths(contract_data))
                record_result(
                    summary_path,
                    evidence(
                        check_id,
                        cmd_str,
                        refresh_code,
                        refresh_duration,
                        refresh_output,
                        contract_hash=contract_hash,
                        commit_sha=commit_sha,
                        execution_contract_path=contract,
                        execution_summary_path=summary,
                        worktree_digest=current_digest,
                    ),
                )
                obs.check_failed(
                    check_id=check_id,
                    command=cmd_str,
                    duration_ms=refresh_duration,
                    detail="source-bound evidence refresh failed",
                )
                return refresh_code
        code, duration, output = run(command)
        duration += refresh_duration
        if refresh_output:
            output = refresh_output + ("\n" + output if output else "")
        if check_id in {"quality", "projectTest"}:
            try:
                if restore_tracked_project_test_receipt():
                    output = (
                        output.rstrip()
                        + "\nRestored tracked project-test aggregate receipt before Summary stabilization.\n"
                    )
            except RuntimeError as exc:
                output = output.rstrip() + f"\nERROR: {exc}\n"
                code = code or 1
        if route is not None:
            output = json.dumps({"finishQualityRoute": route}, sort_keys=True) + "\n" + output
        current_digest = worktree_digest(changed_paths(contract_data))
        record_result(
            summary_path,
            evidence(
                check_id,
                cmd_str,
                code,
                duration,
                output,
                contract_hash=contract_hash,
                commit_sha=commit_sha,
                execution_contract_path=contract,
                execution_summary_path=summary,
                worktree_digest=current_digest,
            ),
        )
        if code != 0 and item.get("required") is True:
            obs.check_failed(check_id=check_id, command=cmd_str, duration_ms=duration)
            return code
        if code == 0:
            obs.check_passed(check_id=check_id, command=cmd_str, duration_ms=duration)
        else:
            obs.check_failed(
                check_id=check_id,
                command=cmd_str,
                duration_ms=duration,
                detail="optional verification failed",
            )
    return 0


def _main_with_mutex(args: argparse.Namespace) -> int:
    contract, summary = task_paths(args.task)
    if not (PROJECT_ROOT / contract).exists():
        print(f"ERROR: Contract does not exist: {contract}", file=sys.stderr)
        return 1
    if not (PROJECT_ROOT / summary).exists():
        print(f"ERROR: Summary does not exist: {summary}", file=sys.stderr)
        return 1

    try:
        ensure_work_item_branch()
    except RuntimeError as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2

    contract_path = PROJECT_ROOT / contract
    summary_path = PROJECT_ROOT / summary
    contract_data = load_json(contract_path)
    if contract_data.get("contractVersion") != 2:
        print(
            "ERROR: ai-finish executes only contractVersion 2 check-ID Contracts", file=sys.stderr
        )
        return 2
    if (PROJECT_ROOT / "Makefile").exists():
        preflight_code, _, _ = run(["make", "ai-preflight", f"CONTRACT={contract}"])
        if preflight_code != 0:
            print(
                "ERROR: Work Item finish is blocked by the Human Decision Gate; "
                "record valid Decision Evidence and rerun Preflight until status is ready.",
                file=sys.stderr,
            )
            return return_blocked_finish_failure(
                task=args.task,
                contract_path=contract_path,
                summary_path=summary_path,
                failed_check="aiPreflight",
                failure_message="the Work Item preflight review is not ready",
                code=preflight_code,
            )
    ensure_active_evidence_changed_files(args.task, summary_path)
    contract_hash = hashlib.sha256(contract_path.read_bytes()).hexdigest()
    commit_sha = current_head()
    from ai_check_agent_risk import validate_checkpoint_bindings

    checkpoint_issues = validate_checkpoint_bindings(
        contract_data,
        load_json(summary_path),
        expected_contract_hash=contract_hash,
    )
    if checkpoint_issues:
        for issue in checkpoint_issues:
            print(f"ERROR: {issue}", file=sys.stderr)
        print(
            checkpoint_recovery_guidance(checkpoint_issues, contract=contract, summary=summary),
            file=sys.stderr,
        )
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="aiCheckpoint",
            failure_message="Contract/checkpoint binding is stale",
            code=2,
        )
    stale_verification_count = discard_stale_contract_verification(summary_path, contract_hash)
    if stale_verification_count:
        print(
            "Discarded "
            f"{stale_verification_count} stale active verification record(s) "
            "bound to a prior Contract."
        )
    declared = contract_data.get("verification", [])
    if not isinstance(declared, list):
        print("ERROR: Contract verification must be a list", file=sys.stderr)
        return 1

    obs = create_observability(work_item_id=args.task)
    total_start = time.time()
    declared_items = inject_mandatory_verification_checks(
        [item for item in declared if isinstance(item, dict)],
        contract_data=contract_data,
    )
    summary_requests_outcome = True
    declared_items.sort(
        key=finish_execution_priority if summary_requests_outcome else verification_priority
    )
    ownership = preview(contract=contract_data)
    print("\n".join(format_preview(ownership)))
    ownership_failures = [
        item for item in ownership if item.state not in {"active_owned", "archived_owned"}
    ]
    if ownership_failures:
        print(
            "ERROR: finish is blocked until every task-era changed path has Work Item ownership.",
            file=sys.stderr,
        )
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="aiDiffOwnership",
            failure_message="one or more task-era changed paths are unowned, ambiguous, restricted, or out of scope",
            code=1,
        )
    # Evidence shape is deterministic and inexpensive to validate.  Check it
    # before the required quality route so an active Work Item gets its
    # canonical red Outcome without paying for a quality run that cannot make
    # malformed documentation evidence archive-ready.  The same validation is
    # deliberately repeated after Outcome/report generation below because that
    # pipeline mutates self-referential documentation surfaces.
    prepare_documentation_alignment_evidence(args.task, summary_path)
    alignment_issues = documentation_alignment_issues(summary_path, contract_data)
    if alignment_issues:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="documentationAlignment",
            failure_message="; ".join(alignment_issues[:3]),
            code=1,
        )
    existing_summary = load_json(summary_path)
    reuse_archive_verification = args.archive and reusable_archive_verification(
        existing_summary,
        contract_data,
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        contract=contract,
        summary_path=summary,
    )
    verification_start = time.time()
    code = 0
    if reuse_archive_verification:
        print("Reusing same-state ai-finish verification for archive")
    else:
        code = run_declared_checks(
            declared_items,
            args=args,
            contract=contract,
            summary=summary,
            contract_data=contract_data,
            contract_path=contract_path,
            summary_path=summary_path,
            contract_hash=contract_hash,
            commit_sha=commit_sha,
            obs=obs,
        )
    getattr(obs, "lifecycle_phase_finished", lambda *_args, **_kwargs: None)(
        "verification",
        duration_ms=elapsed_ms(verification_start),
        cache_outcome="hit" if reuse_archive_verification else "miss",
    )
    if code:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check=failed_check_from_summary(summary_path, "verification"),
            failure_message="a required declared verification check failed",
            code=code,
        )

    prepare_documentation_alignment_evidence(args.task, summary_path)
    alignment_issues = documentation_alignment_issues(summary_path, contract_data)
    if alignment_issues:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="documentationAlignment",
            failure_message="; ".join(alignment_issues[:3]),
            code=1,
        )

    if reuse_archive_verification:
        outcome_ok = _outcome_paths(args.task)[0].is_file()
        outcome_message = "existing outcome is bound by same-state verification"
        human_report_ok, human_report_message = outcome_ok, outcome_message
        if outcome_ok:
            gate_issues = active_terminal_outcome_issues(args.task, contract_path, summary_path)
            if gate_issues:
                human_report_ok = False
                human_report_message = "; ".join(gate_issues)
    else:
        outcome_ok, outcome_message = run_task_outcome_pipeline(
            contract_data["workItemId"], summary_path, contract_path, args.language
        )
        if outcome_ok:
            human_report_ok, human_report_message = run_human_report_pipeline(
                contract_data["workItemId"], summary_path
            )
        else:
            human_report_ok, human_report_message = False, outcome_message
    if not outcome_ok:
        print(f"ERROR: Task Outcome integration failed: {outcome_message}", file=sys.stderr)
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="taskOutcome",
            failure_message=outcome_message,
            code=1,
        )
    if not human_report_ok:
        print(
            f"ERROR: Human Benefit Report integration failed: {human_report_message}",
            file=sys.stderr,
        )
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="humanBenefitReport",
            failure_message=human_report_message,
            code=1,
        )

    # The Outcome/report pipeline expands the active Summary's declared
    # documentation surfaces. Recheck before any completed state is reported
    # or archive is invoked, rather than leaving archive as the first consumer
    # able to discover that the completion claim is stale.
    alignment_issues = documentation_alignment_issues(summary_path, contract_data)
    if alignment_issues:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="documentationAlignment",
            failure_message="; ".join(alignment_issues[:3]),
            code=1,
        )

    if reuse_archive_verification:
        print("Work Item finish checks reused from same-state evidence")
        outcome_json, _outcome_markdown = _outcome_paths(args.task)
        try:
            print(render_direct_outcome_report(load_json(outcome_json), args.language), end="")
        except ValueError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return return_blocked_finish_failure(
                task=args.task,
                contract_path=contract_path,
                summary_path=summary_path,
                failed_check="taskOutcomeReport",
                failure_message=str(exc),
                code=1,
            )
        code, coverage_message = prepare_pre_archive_candidate_coverage(
            args.task, contract_data, obs=obs
        )
        if code != 0:
            obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
            return return_blocked_finish_failure(
                task=args.task,
                contract_path=contract_path,
                summary_path=summary_path,
                failed_check="preArchiveCandidateCoverage",
                failure_message=coverage_message,
                code=code,
            )
        if args.archive:
            archive_command = ["make", "archive-work-item", f"CONTRACT={contract}"]
            cmd_str = " ".join(archive_command)
            obs.check_started(check_id="archive-work-item", command=cmd_str)
            code, duration, _ = run(archive_command)
            if code != 0:
                obs.check_failed(
                    check_id="archive-work-item", command=cmd_str, duration_ms=duration
                )
                obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
                return return_blocked_finish_failure(
                    task=args.task,
                    contract_path=contract_path,
                    summary_path=summary_path,
                    failed_check="archive-work-item",
                    failure_message="archive command failed",
                    code=code,
                )
            obs.check_passed(check_id="archive-work-item", command=cmd_str, duration_ms=duration)
            report_ok, report_message = refresh_archived_human_report(args.task)
            if not report_ok:
                print(
                    f"ERROR: archived Human Benefit Report integration failed: {report_message}",
                    file=sys.stderr,
                )
                obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
                return return_blocked_finish_failure(
                    task=args.task,
                    contract_path=contract_path,
                    summary_path=summary_path,
                    failed_check="archivedHumanBenefitReport",
                    failure_message=report_message,
                    code=1,
                )
            print(archive_next_steps(args.task))
        obs.work_item_finished(result="passed", duration_ms=elapsed_ms(total_start))
        return 0

    # Establish a fail-closed readiness baseline before self-referential
    # stabilization. Positive readiness is persisted only after the first
    # stabilization and final Summary validation have passed.
    summary_data = load_json(summary_path)
    existing_readiness = summary_data.get("reviewReadiness")
    expected_focus = (
        existing_readiness.get("expectedReviewFocus", [])
        if isinstance(existing_readiness, dict)
        and isinstance(existing_readiness.get("expectedReviewFocus"), list)
        else []
    )
    summary_data["reviewReadiness"] = {
        "status": "not_ready",
        "reason": "Final stabilization and status checks are still pending.",
        "expectedReviewFocus": expected_focus,
    }
    save_json(summary_path, summary_data)

    # Summary/status are self-referential artifacts. Stabilize them after all
    # declared result evidence has been written, then attest without mutating.
    stabilization = [
        (
            "aiStatus",
            ["make", "generate-cockpit-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ),
        (
            "aiStatusCheck",
            ["make", "check-ai-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ),
        ("aiStatusConsistency", ["make", "check-ai-status-consistency"]),
        (
            "aiAgentRisk",
            ["make", "check-ai-agent-risk", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ),
        (
            "aiSummary",
            ["make", "check-ai-change-summary", f"SUMMARY={summary}", f"CONTRACT={contract}"],
        ),
    ]
    for check_id, command in stabilization:
        if check_id in {"aiStatusCheck", "aiStatusConsistency"}:
            refresh_command = [
                "make",
                "generate-cockpit-status",
                f"CONTRACT={contract}",
                f"SUMMARY={summary}",
            ]
            refresh_code, refresh_duration, _refresh_output = run(refresh_command)
            if refresh_code != 0:
                obs.check_failed(
                    check_id="aiStatus",
                    command=" ".join(refresh_command),
                    duration_ms=refresh_duration,
                )
                obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
                return return_blocked_finish_failure(
                    task=args.task,
                    contract_path=contract_path,
                    summary_path=summary_path,
                    failed_check="aiStatus",
                    failure_message="status refresh failed during stabilization",
                    code=refresh_code,
                )
        obs.check_started(check_id=check_id, command=" ".join(command))
        if check_id == "aiAgentRisk":
            code, duration, output = run(command, extra_env={"AI_FINISH_STABILIZING": "1"})
        else:
            code, duration, output = run(command)
        # Record actual result of stabilization check to Summary for debugging.
        current_worktree_digest = worktree_digest(changed_paths(contract_data))
        record_result(
            summary_path,
            evidence(
                check_id,
                " ".join(command),
                code,
                duration,
                output,
                contract_hash=contract_hash,
                commit_sha=commit_sha,
                execution_contract_path=contract,
                execution_summary_path=summary,
                worktree_digest=current_worktree_digest,
            ),
        )
        if code != 0:
            obs.check_failed(check_id=check_id, command=" ".join(command), duration_ms=duration)
            obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
            return return_blocked_finish_failure(
                task=args.task,
                contract_path=contract_path,
                summary_path=summary_path,
                failed_check=check_id,
                failure_message="stabilization check failed",
                code=code,
            )
        obs.check_passed(check_id=check_id, command=" ".join(command), duration_ms=duration)

    # Promote only after the declared checks, stabilization, and final Summary
    # validation have all passed. Promotion itself changes the Summary, so
    # status must be regenerated and checked once more against the promoted
    # state. These final checks intentionally do not mutate verification
    # evidence after the status has been generated.
    summary_data = load_json(summary_path)
    summary_data["reviewReadiness"] = promote_review_readiness(summary_data, contract_data)
    save_json(summary_path, summary_data)
    final_status_checks = [
        ["make", "generate-cockpit-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status", f"CONTRACT={contract}", f"SUMMARY={summary}"],
        ["make", "check-ai-status-consistency"],
    ]
    for command in final_status_checks:
        code, duration, output = run(command)
        if code != 0:
            failed_summary = load_json(summary_path)
            failed_summary["reviewReadiness"] = {
                "status": "not_ready",
                "reason": f"Final status validation failed: {' '.join(command)}",
                "expectedReviewFocus": expected_focus,
            }
            save_json(summary_path, failed_summary)
            print(output, file=sys.stderr)
            obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
            return return_blocked_finish_failure(
                task=args.task,
                contract_path=contract_path,
                summary_path=summary_path,
                failed_check="finalStatus",
                failure_message="final status validation failed",
                code=code,
            )

    # Revalidate the promoted Summary last and retain its evidence as the
    # archive's final worktree-digest anchor.
    summary_command = [
        "make",
        "check-ai-change-summary",
        f"SUMMARY={summary}",
        f"CONTRACT={contract}",
    ]
    code, duration, output = run(summary_command)
    if code != 0:
        failed_summary = load_json(summary_path)
        failed_summary["reviewReadiness"] = {
            "status": "not_ready",
            "reason": "Final Summary validation failed after Readiness promotion.",
            "expectedReviewFocus": expected_focus,
        }
        save_json(summary_path, failed_summary)
        print(output, file=sys.stderr)
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="aiSummary",
            failure_message="final Summary validation failed",
            code=code,
        )
    final_summary_evidence = evidence(
        "aiSummary",
        " ".join(summary_command),
        code,
        duration,
        output,
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        execution_contract_path=contract,
        execution_summary_path=summary,
        worktree_digest=worktree_digest_for_finish(changed_paths(contract_data), summary),
    )
    final_summary_evidence["outcomeInputDigest"] = outcome_input_digest(load_json(summary_path))
    record_result(summary_path, final_summary_evidence)

    final_outcome_ok, final_outcome_message = refresh_final_outcome_after_stabilization(
        contract_data["workItemId"], contract_path, summary_path, args.language
    )
    if not final_outcome_ok:
        print(f"ERROR: {final_outcome_message}", file=sys.stderr)
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="taskOutcomeProjection",
            failure_message=final_outcome_message,
            code=1,
        )

    code, coverage_message = prepare_pre_archive_candidate_coverage(
        args.task, contract_data, obs=obs
    )
    if code != 0:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="preArchiveCandidateCoverage",
            failure_message=coverage_message,
            code=code,
        )
    gate_issues = active_terminal_outcome_issues(args.task, contract_path, summary_path)
    if gate_issues:
        obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="taskOutcomeGreenGate",
            failure_message="; ".join(gate_issues),
            code=1,
        )
    record_fact_once(
        args.task,
        "finish_passed",
        {"contractPath": contract, "summaryPath": summary, "commitSha": commit_sha},
    )
    outcome_json, _outcome_markdown = _outcome_paths(args.task)
    try:
        print(render_direct_outcome_report(load_json(outcome_json), args.language), end="")
    except (OSError, TypeError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return return_blocked_finish_failure(
            task=args.task,
            contract_path=contract_path,
            summary_path=summary_path,
            failed_check="taskOutcomeReport",
            failure_message=str(exc),
            code=1,
        )
    print("Work Item finish checks passed")
    if args.archive:
        archive_command = ["make", "archive-work-item", f"CONTRACT={contract}"]
        cmd_str = " ".join(archive_command)
        obs.check_started(check_id="archive-work-item", command=cmd_str)
        code, duration, _ = run(archive_command)
        if code != 0:
            obs.check_failed(check_id="archive-work-item", command=cmd_str, duration_ms=duration)
            obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
            return code
        obs.check_passed(check_id="archive-work-item", command=cmd_str, duration_ms=duration)
        report_ok, report_message = refresh_archived_human_report(args.task)
        if not report_ok:
            print(
                f"ERROR: archived Human Benefit Report integration failed: {report_message}",
                file=sys.stderr,
            )
            obs.work_item_finished(result="failed", duration_ms=elapsed_ms(total_start))
            return 1
        print(archive_next_steps(args.task))
    duration_ms = elapsed_ms(total_start)
    getattr(obs, "lifecycle_phase_finished", lambda *_args, **_kwargs: None)(
        "finish", duration_ms=duration_ms, cache_outcome="miss"
    )
    obs.work_item_finished(result="passed", duration_ms=duration_ms)
    return 0


def main() -> int:
    args = parse_args()
    if not args.language:
        print(
            "ERROR: --language is required; bind the Outcome to the conversation locale (en, ja, or zh-CN).",
            file=sys.stderr,
        )
        return 2
    global CURRENT_REPORT_LANGUAGE
    CURRENT_REPORT_LANGUAGE = args.language
    try:
        # Acquire before Preflight, verification, Outcome/report/status, or
        # archive paths can perform mutable lifecycle work. Legacy serial
        # Work Items retain their established route; an explicit concurrency
        # boundary is the sole opt-in to the persistent projection lease.
        with finish_mutex(args.task, archive=args.archive):
            contract_path = ACTIVE_DIR / f"{args.task}.contract.json"
            try:
                contract = load_json(contract_path)
            except (OSError, ValueError):
                contract = None
            if requires_lease(contract):
                acquire_projection_lease(args.task, root=PROJECT_ROOT)
            return _main_with_mutex(args)
    except (FinishMutexError, ProjectionLeaseError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
