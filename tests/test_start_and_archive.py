import fcntl
import hashlib
import json
import runpy
import subprocess
import sys
from datetime import UTC, datetime
from pathlib import Path

import ai_archive_work_item
import ai_calibration_corrective
import ai_check_pr
import ai_check_scope
import ai_common
import ai_generate_human_report
import ai_lifecycle_truth
import ai_linked_worktree_recovery
import ai_resume_work_item
import ai_start
import ai_start_receipt
import pytest
from ai_acceptance_policy import validate_acceptance_evidence
from ai_external_handoff import build_handoff
from ai_generate_task_outcome import generate_outcome
from ai_observability import AiEventType
from ai_render_task_outcome import render_task_outcome
from ai_resume_work_item import ResumeError, resume_contract, synchronize_contract
from ai_start_receipt import (
    build_receipt,
    current_branch,
    receipt_binding,
    receipt_path,
    scope_digest,
    skeleton_digest,
    validate_receipt,
    validate_resume_history,
)


def test_ai_start_make_target_exposes_calibration_corrective_declaration() -> None:
    makefile = Path(__file__).resolve().parents[1] / "Makefile"
    target = makefile.read_text(encoding="utf-8").split("ai-resume-work-item:", 1)[0]

    assert "AI_START_CALIBRATION_CORRECTIVE" in target
    assert "--calibration-corrective" in target


def test_lifecycle_phase_event_type_is_available_to_start_and_archive() -> None:
    assert AiEventType.LIFECYCLE_PHASE_FINISHED.value == "lifecycle_phase_finished"


def test_live_calibration_session_rejects_ordinary_start_before_lifecycle_writes(
    tmp_path: Path,
) -> None:
    session_path = tmp_path / ".ai" / "calibration" / "session.json"
    session_path.parent.mkdir(parents=True)
    session_path.write_text(
        json.dumps({"sessionId": "calibration-1", "state": "in_progress"}),
        encoding="utf-8",
    )

    issue = ai_start.calibration_start_issue(root=tmp_path)

    assert issue == (
        "ERROR: live calibration Session calibration-1 is in_progress; "
        "start requires a valid --calibration-corrective declaration before lifecycle writes."
    )
    assert not (tmp_path / ".ai" / "work-items" / "active").exists()


def test_live_calibration_session_admits_only_exact_bound_corrective_declaration(
    tmp_path: Path,
) -> None:
    session_path = tmp_path / ".ai" / "calibration" / "session.json"
    session_path.parent.mkdir(parents=True)
    session_path.write_text(
        json.dumps({"sessionId": "calibration-1", "state": "paused"}),
        encoding="utf-8",
    )
    corrective = {
        "schemaVersion": 1,
        "sessionPath": ".ai/calibration/session.json",
        "sessionId": "calibration-1",
        "sessionState": "paused",
        "sessionDigest": hashlib.sha256(session_path.read_bytes()).hexdigest(),
        "findingId": "CAL-614-001",
        "findingSummary": "Start must expose a bounded corrective route.",
        "authority": "user authorization recorded in issue #614",
        "repairPaths": ["scripts/ai_start.py"],
        "resumeCondition": "Resume calibration through its own Session workflow after closure.",
    }

    assert ai_start.calibration_start_issue(corrective, root=tmp_path) is None


def test_calibration_corrective_shape_rejects_incomplete_or_unsafe_declarations() -> None:
    assert (
        ai_calibration_corrective.validate_calibration_corrective_shape(None)
        == "calibrationCorrective must be a JSON object"
    )
    assert "missing" in ai_calibration_corrective.validate_calibration_corrective_shape({})

    declaration = {
        "schemaVersion": 1,
        "sessionPath": ".ai/calibration/session.json",
        "sessionId": "calibration-1",
        "sessionState": "paused",
        "sessionDigest": "a" * 64,
        "findingId": "CAL-614-001",
        "findingSummary": "bounded corrective route",
        "authority": "direct user authorization",
        "repairPaths": ["scripts/ai_start.py"],
        "resumeCondition": "resume calibration after closure",
    }
    assert (
        ai_calibration_corrective.validate_calibration_corrective_shape(
            {**declaration, "schemaVersion": 2}
        )
        == "calibrationCorrective.schemaVersion must be 1"
    )
    assert (
        ai_calibration_corrective.validate_calibration_corrective_shape(
            {**declaration, "sessionPath": "/absolute"}
        )
        == "calibrationCorrective.sessionPath must be .ai/calibration/session.json"
    )
    assert (
        ai_calibration_corrective.validate_calibration_corrective_shape(
            {**declaration, "repairPaths": ["../outside.py"]}
        )
        == "calibrationCorrective.repairPaths must be unique repository-relative paths"
    )
    assert (
        ai_calibration_corrective.validate_calibration_corrective_shape(
            {**declaration, "repairPaths": [".ai/calibration/session.json"]}
        )
        == "calibrationCorrective.repairPaths cannot modify calibration Session state"
    )


def test_calibration_corrective_rejects_invalid_stale_and_non_live_sessions(tmp_path: Path) -> None:
    session_path = tmp_path / ".ai" / "calibration" / "session.json"
    session_path.parent.mkdir(parents=True)
    session_path.write_text("not json", encoding="utf-8")
    assert "Session is unreadable" in ai_calibration_corrective.calibration_start_issue(
        root=tmp_path
    )

    session_path.write_text("[]", encoding="utf-8")
    assert ai_calibration_corrective.calibration_start_issue(root=tmp_path) == (
        "ERROR: calibration Session must be a JSON object"
    )
    session_path.write_text('{"sessionId": "", "state": "paused"}', encoding="utf-8")
    assert (
        "sessionId must be a non-empty string"
        in ai_calibration_corrective.calibration_start_issue(root=tmp_path)
    )

    session_path.write_text('{"sessionId": "calibration-1", "state": "complete"}', encoding="utf-8")
    assert ai_calibration_corrective.calibration_start_issue(root=tmp_path) is None
    assert ai_calibration_corrective.calibration_start_issue({}, root=tmp_path) == (
        "ERROR: calibration corrective requires a live in_progress or paused Session"
    )
    assert ai_calibration_corrective.calibration_corrective_binding_issue({}, root=tmp_path) == (
        "ERROR: calibration corrective requires a live in_progress or paused Session"
    )
    session_path.unlink()
    assert ai_calibration_corrective.calibration_corrective_binding_issue({}, root=tmp_path) == (
        "ERROR: calibrationCorrective requires its bound live calibration Session"
    )


def test_linked_worktree_foreign_duplicate_allows_unrelated_task_with_recovery_route(
    tmp_path, monkeypatch
):
    current, canonical, foreign = (tmp_path / name for name in ("current", "canonical", "foreign"))
    for root in (current, canonical, foreign):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    for root in (canonical, foreign):
        for suffix in ("contract", "summary"):
            (root / ".ai" / "work-items" / "active" / f"other-task.{suffix}.json").write_text(
                '{"workItemId":"other-task"}', encoding="utf-8"
            )
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(canonical, "codex/other-task"), (foreign, "codex/other-task-refresh")],
    )
    issue = ai_start.linked_worktree_active_issue("new-task")
    assert issue is None
    assert (
        foreign / ".ai" / "work-items" / "active" / "other-task.contract.json"
    ).read_bytes() == (b'{"workItemId":"other-task"}')


def test_linked_worktree_foreign_duplicate_for_requested_task_stays_fail_closed(
    tmp_path, monkeypatch
):
    canonical, foreign = tmp_path / "canonical", tmp_path / "foreign"
    for root in (canonical, foreign):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
        for suffix in ("contract", "summary"):
            (root / ".ai" / "work-items" / "active" / f"other-task.{suffix}.json").write_text(
                '{"workItemId":"other-task"}', encoding="utf-8"
            )
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(canonical, "codex/other-task"), (foreign, "codex/other-task-refresh")],
    )

    issue = ai_start.linked_worktree_active_issue("other-task")

    assert "recoverable foreign duplicate Work Item identity" in issue
    assert "requested task other-task" in issue


def test_real_linked_worktree_duplicate_does_not_block_unrelated_start(tmp_path, monkeypatch):
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    _write_commit(root, "seed.txt", "start\n")
    canonical = tmp_path / "canonical"
    foreign = tmp_path / "foreign"
    _git(root, "worktree", "add", "-b", "codex/other-task", str(canonical))
    _git(root, "worktree", "add", "-b", "codex/other-task-refresh", str(foreign))
    for worktree in (canonical, foreign):
        active = worktree / ".ai" / "work-items" / "active"
        active.mkdir(parents=True)
        for suffix in ("contract", "summary"):
            (active / f"other-task.{suffix}.json").write_text(
                '{"workItemId":"other-task"}', encoding="utf-8"
            )
    foreign_contract = (foreign / ".ai/work-items/active/other-task.contract.json").read_bytes()
    foreign_summary = (foreign / ".ai/work-items/active/other-task.summary.json").read_bytes()
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", root)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", root / ".ai" / "work-items" / "active")

    assert ai_start.linked_worktree_active_issue("new-task", root=root) is None
    assert (
        foreign / ".ai/work-items/active/other-task.contract.json"
    ).read_bytes() == foreign_contract
    assert (
        foreign / ".ai/work-items/active/other-task.summary.json"
    ).read_bytes() == foreign_summary


def test_ai_start_main_creates_unrelated_contract_despite_real_foreign_duplicate(
    tmp_path, monkeypatch
):
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    _write_commit(root, "seed.txt", "start\n")
    canonical = tmp_path / "canonical"
    foreign = tmp_path / "foreign"
    _git(root, "worktree", "add", "-b", "codex/other-task", str(canonical))
    _git(root, "worktree", "add", "-b", "codex/other-task-refresh", str(foreign))
    for worktree in (canonical, foreign):
        active = worktree / ".ai" / "work-items" / "active"
        active.mkdir(parents=True)
        for suffix in ("contract", "summary"):
            (active / f"other-task.{suffix}.json").write_text(
                '{"workItemId":"other-task"}', encoding="utf-8"
            )
    foreign_contract = (foreign / ".ai/work-items/active/other-task.contract.json").read_bytes()
    foreign_summary = (foreign / ".ai/work-items/active/other-task.summary.json").read_bytes()
    active = root / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", root)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "DEFAULT_STATUS", root / ".ai/cockpit/current_status.md")
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *args, **kwargs: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "new-task", "--mode", "investigate"])

    assert ai_start.main() == 0
    assert (active / "new-task.contract.json").exists()
    assert (active / "new-task.summary.json").exists()
    contract = json.loads((active / "new-task.contract.json").read_text(encoding="utf-8"))
    assert {
        ".ai/cockpit/task_report.json",
        ".ai/cockpit/task_report.md",
        ".ai/knowledge/**",
    }.issubset(contract["scope"])
    assert (
        foreign / ".ai/work-items/active/other-task.contract.json"
    ).read_bytes() == foreign_contract
    assert (
        foreign / ".ai/work-items/active/other-task.summary.json"
    ).read_bytes() == foreign_summary


def test_ai_start_main_creates_unrelated_contract_despite_foreign_canonical_active_work_item(
    tmp_path, monkeypatch
):
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    _write_commit(root, "seed.txt", "start\n")
    foreign = tmp_path / "foreign"
    _git(root, "worktree", "add", "-b", "codex/other-task", str(foreign))
    foreign_active = foreign / ".ai" / "work-items" / "active"
    foreign_active.mkdir(parents=True)
    for suffix in ("contract", "summary"):
        (foreign_active / f"other-task.{suffix}.json").write_text(
            '{"workItemId":"other-task"}', encoding="utf-8"
        )
    foreign_contract = (foreign_active / "other-task.contract.json").read_bytes()
    foreign_summary = (foreign_active / "other-task.summary.json").read_bytes()
    active = root / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", root)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "DEFAULT_STATUS", root / ".ai/cockpit/current_status.md")
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *args, **kwargs: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "new-task", "--mode", "investigate"])

    assert ai_start.main() == 0
    assert (active / "new-task.contract.json").exists()
    assert (active / "new-task.summary.json").exists()
    assert (root / ".ai/work-items/starts/new-task.json").exists()
    assert (foreign_active / "other-task.contract.json").read_bytes() == foreign_contract
    assert (foreign_active / "other-task.summary.json").read_bytes() == foreign_summary


def test_foreign_duplicate_diagnostic_is_read_only_and_requires_canonical_owner(
    tmp_path, monkeypatch
):
    canonical, foreign = tmp_path / "canonical", tmp_path / "foreign"
    canonical.mkdir()
    foreign.mkdir()
    identities = [
        ai_start.LinkedWorktreeIdentity(canonical, "codex/other-task", "other-task"),
        ai_start.LinkedWorktreeIdentity(foreign, "codex/other-task-refresh", "other-task"),
    ]
    monkeypatch.setattr(ai_start, "linked_worktree_identity_report", lambda: (identities, []))
    code, value = ai_linked_worktree_recovery.report("other-task")
    assert code == 0
    assert value["status"] == "recoverable_foreign_duplicate"
    assert value["authorization"] == "diagnostic_only_no_mutation"
    assert foreign.exists()


def test_start_and_archive_use_clean_git_environment():
    assert all(not key.startswith("GIT_") for key in ai_common.clean_git_environment())


def test_synchronization_git_queries_use_the_resolved_absolute_executable(tmp_path, monkeypatch):
    observed: list[list[str]] = []

    def fake_run(command, **_kwargs):
        observed.append(command)
        return subprocess.CompletedProcess(command, 0, stdout="ok\n", stderr="")

    monkeypatch.setattr(ai_resume_work_item, "governed_git_executable", lambda: "/trusted/git")
    monkeypatch.setattr(ai_resume_work_item.subprocess, "run", fake_run)

    assert ai_resume_work_item._git(tmp_path, "status", "--porcelain") == "ok"
    assert observed == [["/trusted/git", "status", "--porcelain"]]


def test_linked_worktree_valid_isolated_active_pair_allows_agent_parallelism(tmp_path, monkeypatch):
    current = tmp_path / "current"
    other = tmp_path / "other"
    for root in (current, other):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    (other / ".ai" / "work-items" / "active" / "other-task.contract.json").write_text(
        '{"workItemId": "other-task"}', encoding="utf-8"
    )
    (other / ".ai" / "work-items" / "active" / "other-task.summary.json").write_text(
        '{"workItemId": "other-task"}', encoding="utf-8"
    )
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(other, "codex/other-task")],
    )

    issue = ai_start.linked_worktree_active_issue()

    assert issue is None
    assert not list((current / ".ai" / "work-items" / "active").glob("*.json"))


def test_linked_worktree_quarantined_receipt_allows_only_its_bound_successor(tmp_path, monkeypatch):
    current, predecessor = (tmp_path / name for name in ("current", "predecessor"))
    active = predecessor / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    outcome = active / "blocked-task.outcome.json"
    outcome.write_text(
        json.dumps({"workItemId": "blocked-task", "status": "blocked"}), encoding="utf-8"
    )
    transition = ai_lifecycle_truth.transition_to_successor(
        predecessorOutcome=outcome,
        predecessor={"workItemId": "blocked-task"},
        successor={
            "workItemId": "bound-successor",
            "branch": "codex/bound-successor",
            "baseCommit": "a" * 40,
        },
        issue="https://github.com/spirex-ds-dev/ai-cockpit-template/issues/737",
        authority="explicit human authorization",
        mode="quarantined",
        reason="start only the exact successor",
    )
    assert transition.accepted
    for suffix in ("contract", "summary"):
        (active / f"blocked-task.{suffix}.json").write_text(
            json.dumps({"workItemId": "blocked-task"}), encoding="utf-8"
        )
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(predecessor, "codex/blocked-task")],
    )
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "current_branch", lambda: "codex/bound-successor")

    assert ai_start.linked_worktree_active_issue("bound-successor", root=current) is None
    assert ai_start.linked_worktree_active_issue("unrelated-task", root=current) is None
    monkeypatch.setattr(ai_start, "current_branch", lambda: "codex/unrelated-task")
    assert "quarantined successor" in ai_start.linked_worktree_active_issue(
        "bound-successor", root=current
    )
    monkeypatch.setattr(ai_start, "current_branch", lambda: "codex/bound-successor")
    monkeypatch.setattr(ai_start, "current_head", lambda: "b" * 40)
    assert "quarantined successor" in ai_start.linked_worktree_active_issue(
        "bound-successor", root=current
    )
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    receipt_path = active / "blocked-task.successor-receipt.json"
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    receipt["authority"] = ""
    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
    assert "invalid quarantined successor receipt (missing_authority)" in (
        ai_start.linked_worktree_active_issue("bound-successor", root=current) or ""
    )
    assert ai_start.linked_worktree_active_issue("unrelated-task", root=current) is None


def test_linked_worktree_malformed_active_pair_fails_closed(tmp_path, monkeypatch):
    current = tmp_path / "current"
    other = tmp_path / "other"
    for root in (current, other):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    (other / ".ai" / "work-items" / "active" / "other-task.contract.json").write_text(
        "{}", encoding="utf-8"
    )
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(other, "codex/other-task")],
    )

    issue = ai_start.linked_worktree_active_issue()

    assert issue == (
        "ERROR: linked worktree has malformed active Work Item records on branch "
        f"codex/other-task: {other} (contract/summary pair required)"
    )


def test_linked_worktree_active_pair_with_non_dedicated_branch_fails_closed(tmp_path, monkeypatch):
    current = tmp_path / "current"
    other = tmp_path / "other"
    for root in (current, other):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    for suffix in ("contract", "summary"):
        (other / ".ai" / "work-items" / "active" / f"other-task.{suffix}.json").write_text(
            '{"workItemId": "other-task"}', encoding="utf-8"
        )
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(ai_start, "linked_worktree_records", lambda **_kwargs: [(other, "main")])

    assert ai_start.linked_worktree_active_issue() == (
        "ERROR: linked worktree active Work Item branch does not match its task: "
        f"main != codex/other-task: {other}"
    )


def test_unrelated_malformed_linked_worktree_isolated_but_own_identity_stays_closed(
    tmp_path, monkeypatch
):
    current = tmp_path / "current"
    other = tmp_path / "other"
    for root in (current, other):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    for suffix in ("contract", "summary"):
        (other / ".ai" / "work-items" / "active" / f"other-task.{suffix}.json").write_text(
            '{"workItemId": "other-task"}', encoding="utf-8"
        )
    foreign_contract = (
        other / ".ai" / "work-items" / "active" / "other-task.contract.json"
    ).read_bytes()
    foreign_summary = (
        other / ".ai" / "work-items" / "active" / "other-task.summary.json"
    ).read_bytes()
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(ai_start, "linked_worktree_records", lambda **_kwargs: [(other, "main")])

    assert ai_start.linked_worktree_active_issue("new-task") is None
    assert "branch does not match its task" in ai_start.linked_worktree_active_issue("other-task")
    assert (
        other / ".ai" / "work-items" / "active" / "other-task.contract.json"
    ).read_bytes() == foreign_contract
    assert (
        other / ".ai" / "work-items" / "active" / "other-task.summary.json"
    ).read_bytes() == foreign_summary


def test_linked_worktree_ignores_summary_orphan_with_complete_archive(tmp_path, monkeypatch):
    current = tmp_path / "current"
    other = tmp_path / "other"
    task = "closed-task"
    for root in (current, other):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    (other / ".ai" / "work-items" / "active" / f"{task}.summary.json").write_text(
        "{}", encoding="utf-8"
    )
    archive = other / ".ai" / "work-items" / "archive" / "2026"
    archive.mkdir(parents=True)
    for suffix in ("contract.json", "summary.json", "outcome.json", "archive-manifest.json"):
        (archive / f"{task}.{suffix}").write_text("{}", encoding="utf-8")
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(
        ai_start,
        "linked_worktree_records",
        lambda **_kwargs: [(other, "main")],
    )

    assert ai_start.linked_worktree_active_issue() is None


def test_linked_worktree_check_ignores_detached_or_empty_worktrees(tmp_path, monkeypatch):
    current = tmp_path / "current"
    detached = tmp_path / "detached"
    for root in (current, detached):
        (root / ".ai" / "work-items" / "active").mkdir(parents=True)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", current)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", current / ".ai" / "work-items" / "active")
    monkeypatch.setattr(ai_start, "linked_worktree_records", lambda **_kwargs: [(detached, None)])

    assert ai_start.linked_worktree_active_issue() is None


def test_rewrite_archived_path_references_preserves_non_path_scalars():
    active = ".ai/work-items/active/task.contract.json"
    archived = ".ai/work-items/archive/2026/task.contract.json"
    evidence = {
        "path": active,
        "nested": [active, "make ai-finish CONTRACT=" + active, 622, True, None],
    }

    rewritten = ai_archive_work_item._rewrite_archived_path_references(evidence, {active: archived})

    assert rewritten == {
        "path": archived,
        "nested": [archived, "make ai-finish CONTRACT=" + active, 622, True, None],
    }


def test_start_preflight_can_skip_contract_validation_for_new_skeleton(monkeypatch):
    observed = {}

    class Result:
        returncode = 0
        stdout = ""
        stderr = ""

    def fake_run(command, **_kwargs):
        observed["command"] = command
        return Result()

    monkeypatch.setattr(ai_start.subprocess, "run", fake_run)
    assert (
        ai_start.run_make(
            "ai-preflight",
            contract=".ai/work-items/active/example.contract.json",
            variables=["AI_PREFLIGHT_VALIDATE_CONTRACT=false"],
        )[0]
        == 0
    )
    assert observed["command"][-1] == "AI_PREFLIGHT_VALIDATE_CONTRACT=false"


def test_start_nested_make_uses_selected_explicit_entrypoint(tmp_path, monkeypatch):
    (tmp_path / "Makefile.ai").write_text("ai-preflight:\n\t@true\n", encoding="utf-8")
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setenv("AI_COCKPIT_MAKE_ENTRYPOINT", "Makefile.ai")
    observed = {}

    class Result:
        returncode = 0
        stdout = "ready"
        stderr = ""

    def fake_run(command, **_kwargs):
        observed["command"] = command
        return Result()

    monkeypatch.setattr(ai_start.subprocess, "run", fake_run)

    assert ai_start.run_make("ai-preflight") == (0, "ready")
    assert observed["command"] == ["make", "-f", "Makefile.ai", "ai-preflight"]


def test_next_available_task_id_resolves_archive_collision_before_creation():
    assert (
        ai_start.next_available_task_id(
            "publish-new-version",
            {"publish-new-version", "publish-new-version-20260725"},
            date="20260725",
        )
        == "publish-new-version-20260725-2"
    )


def test_next_available_task_id_uses_module_utc_reference_for_generated_stamp(monkeypatch):
    class FixedDatetime:
        @staticmethod
        def now(tz):
            assert tz is UTC
            return datetime(2026, 7, 30, tzinfo=UTC)

    monkeypatch.setattr(ai_start, "datetime", FixedDatetime)

    assert ai_start.next_available_task_id("task", {"task"}) == "task-20260730"


def test_start_receipt_binds_contract_and_rejects_tampering(tmp_path):
    contract = {
        "contractVersion": 2,
        "workItemId": "receipt_task",
        "mode": "code",
        "title": "Receipt",
        "baseCommit": "a" * 40,
        "scope": ["src", "tests"],
    }
    receipt = build_receipt(contract, timestamp="2026-07-17T00:00:00+00:00")
    contract["startReceipt"] = receipt_binding(receipt)
    assert receipt["contractSkeletonDigest"] == skeleton_digest(contract)
    assert validate_receipt(contract, receipt, project_root=tmp_path) == []

    tampered = dict(receipt)
    tampered["baseCommit"] = "b" * 40
    assert "Start Receipt baseCommit does not match Contract" in validate_receipt(
        contract, tampered, project_root=tmp_path
    )


def _git(root: Path, *args: str) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=root,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def _write_commit(root: Path, name: str, content: str) -> str:
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")
    _git(root, "add", name)
    _git(root, "commit", "-m", f"write {name}")
    return _git(root, "rev-parse", "HEAD")


def _write_predecessor_archive(root: Path, work_item_id: str, sequence: int) -> str:
    archive = root / ".ai/work-items/archive/2026"
    archive.mkdir(parents=True, exist_ok=True)
    predecessor_contract = archive / f"{work_item_id}.contract.json"
    predecessor_summary = archive / f"{work_item_id}.summary.json"
    predecessor_contract.write_text(
        json.dumps({"workItemId": work_item_id}) + "\n", encoding="utf-8"
    )
    predecessor_summary.write_text(
        json.dumps({"workItemId": work_item_id}) + "\n", encoding="utf-8"
    )
    manifest = archive / f"{work_item_id}.archive-manifest.json"
    manifest.write_text(
        json.dumps(
            {
                "format": "ai-cockpit-archive-manifest",
                "manifestVersion": 1,
                "workItemId": work_item_id,
                "archiveSequence": sequence,
                "contractPath": (f".ai/work-items/archive/2026/{work_item_id}.contract.json"),
                "summaryPath": (f".ai/work-items/archive/2026/{work_item_id}.summary.json"),
                "contractSha256": hashlib.sha256(predecessor_contract.read_bytes()).hexdigest(),
                "summarySha256": hashlib.sha256(predecessor_summary.read_bytes()).hexdigest(),
                "generatedStatusExcluded": True,
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    return f".ai/work-items/archive/2026/{work_item_id}.archive-manifest.json"


def _closed_predecessor(work_item_id: str, merge_commit: str, manifest_path: str) -> dict:
    return {
        "workItemId": work_item_id,
        "status": "closed",
        "pr": {"merged": True, "mergeCommit": merge_commit},
        "closure": {
            "succeeded": True,
            "localBranchDeleted": True,
            "remoteBranchDeleted": True,
            "baseSynchronized": True,
            "evidence": manifest_path,
        },
    }


def _resume_fixture(tmp_path: Path) -> tuple[Path, Path, Path, str, str]:
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    start = _write_commit(root, "seed.txt", "start\n")
    remote = tmp_path / "origin.git"
    subprocess.run(
        ["git", "init", "--bare", str(remote)], check=True, capture_output=True, text=True
    )
    _git(root, "remote", "add", "origin", str(remote))
    _git(root, "push", "-u", "origin", "main")
    _git(root, "switch", "-c", "codex/paused-task")

    contract_path = root / ".ai/work-items/active/paused-task.contract.json"
    receipt_file = root / ".ai/work-items/starts/paused-task.json"
    contract_path.parent.mkdir(parents=True)
    receipt_file.parent.mkdir(parents=True)
    contract = {
        "contractVersion": 2,
        "workItemId": "paused-task",
        "mode": "code",
        "title": "Paused task",
        "baseCommit": start,
        "scope": ["src/**"],
    }
    receipt = build_receipt(
        contract,
        timestamp="2026-07-28T00:00:00+00:00",
        project_root=root,
    )
    contract["startReceipt"] = receipt_binding(receipt)
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")

    _git(root, "switch", "main")
    target = _write_commit(root, "corrective.txt", "fixed\n")
    _git(root, "push", "origin", "main")
    _git(root, "switch", "codex/paused-task")
    _git(root, "rebase", target)

    manifest = _write_predecessor_archive(root, "corrective", 1)
    contract["predecessorWorkItem"] = _closed_predecessor("corrective", target, manifest)
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    return root, contract_path, receipt_file, start, target


def _synchronization_fixture(tmp_path: Path) -> tuple[Path, Path, Path, str, str]:
    """Create an active, clean dedicated branch that is behind origin/main."""
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    start = _write_commit(root, "seed.txt", "start\n")
    remote = tmp_path / "origin.git"
    subprocess.run(
        ["git", "init", "--bare", str(remote)], check=True, capture_output=True, text=True
    )
    _git(root, "remote", "add", "origin", str(remote))
    _git(root, "push", "-u", "origin", "main")
    _git(root, "switch", "-c", "codex/paused-task")

    contract_path = root / ".ai/work-items/active/paused-task.contract.json"
    summary_path = root / ".ai/work-items/active/paused-task.summary.json"
    receipt_file = root / ".ai/work-items/starts/paused-task.json"
    contract_path.parent.mkdir(parents=True)
    receipt_file.parent.mkdir(parents=True)
    contract = {
        "contractVersion": 2,
        "workItemId": "paused-task",
        "mode": "code",
        "title": "Paused task",
        "baseCommit": start,
        "scope": ["src/**"],
    }
    receipt = build_receipt(
        contract,
        timestamp="2026-08-06T00:00:00+00:00",
        project_root=root,
    )
    contract["startReceipt"] = receipt_binding(receipt)
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    summary_path.write_text(
        json.dumps(
            {
                "workItemId": "paused-task",
                "verification": [{"check": "quality", "result": "passed"}],
            }
        )
        + "\n",
        encoding="utf-8",
    )
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    _git(root, "add", ".ai")
    _git(root, "commit", "-m", "record active evidence")

    _git(root, "switch", "main")
    target = _write_commit(root, "corrective.txt", "fixed\n")
    _git(root, "push", "origin", "main")
    _git(root, "switch", "codex/paused-task")
    return root, contract_path, summary_path, start, target


def test_synchronize_contract_rebases_clean_active_branch_and_records_one_transition(tmp_path):
    root, contract_path, summary_path, start, target = _synchronization_fixture(tmp_path)
    original_receipt = (root / ".ai/work-items/starts/paused-task.json").read_bytes()

    transition = synchronize_contract(
        contract_path,
        summary_path=summary_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-08-06T01:00:00+00:00",
        project_root=root,
    )

    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert transition["fromBaseCommit"] == start
    assert transition["toBaseCommit"] == target
    assert transition["workBranch"] == "codex/paused-task"
    assert contract["baseCommit"] == target
    assert contract["synchronizationHistory"] == [transition]
    assert _git(root, "merge-base", "--is-ancestor", target, "HEAD") == ""
    assert summary["verification"][0]["result"] == "not_run"
    assert (root / ".ai/work-items/starts/paused-task.json").read_bytes() == original_receipt


def test_synchronize_contract_checkpoints_contract_authorized_owned_dirty_worktree(tmp_path):
    root, contract_path, summary_path, start, target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    source = root / "src" / "implementation.py"
    source.parent.mkdir()
    source.write_text("VALUE = 'active work item change'\n", encoding="utf-8")

    transition = synchronize_contract(
        contract_path,
        summary_path=summary_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-08-06T01:00:00+00:00",
        project_root=root,
    )

    synchronized = json.loads(contract_path.read_text(encoding="utf-8"))
    assert transition["fromBaseCommit"] == start
    assert transition["toBaseCommit"] == target
    assert transition["checkpointHeadBefore"] != transition["checkpointHeadAfter"]
    assert _git(root, "merge-base", "--is-ancestor", target, "HEAD") == ""
    assert _git(root, "show", "HEAD:src/implementation.py") == "VALUE = 'active work item change'"
    assert synchronized["synchronizationHistory"] == [transition]


def test_synchronize_cli_uses_explicit_target_root_for_dirty_worktree(tmp_path, monkeypatch):
    target_root, contract_path, _summary_path, _start, target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    source = target_root / "src" / "implementation.py"
    source.parent.mkdir()
    source.write_text("VALUE = 'active work item change'\n", encoding="utf-8")
    caller_root = tmp_path / "current-capability-caller"
    caller_root.mkdir()
    monkeypatch.setattr(ai_resume_work_item, "PROJECT_ROOT", caller_root)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_resume_work_item.py",
            "--synchronize",
            "--project-root",
            str(target_root),
            "--contract",
            ".ai/work-items/active/paused-task.contract.json",
            "--base-remote",
            "origin",
            "--base-branch",
            "main",
        ],
    )

    assert ai_resume_work_item.main() == 0
    assert json.loads(contract_path.read_text(encoding="utf-8"))["baseCommit"] == target
    assert not (caller_root / ".ai").exists()


def test_synchronize_contract_checkpoints_modified_owned_path(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    _write_commit(root, "src/implementation.py", "VALUE = 'initial'\n")
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    (root / "src" / "implementation.py").write_text(
        "VALUE = 'active work item change'\n", encoding="utf-8"
    )

    transition = synchronize_contract(
        contract_path,
        summary_path=summary_path,
        base_remote="origin",
        base_branch="main",
        project_root=root,
    )

    assert transition["checkpointPaths"] == [
        ".ai/work-items/active/paused-task.contract.json",
        "src/implementation.py",
    ]
    assert _git(root, "show", "HEAD:src/implementation.py") == "VALUE = 'active work item change'"


def test_synchronization_history_rejects_malformed_checkpoint_evidence(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    source = root / "src" / "implementation.py"
    source.parent.mkdir()
    source.write_text("VALUE = 'active work item change'\n", encoding="utf-8")
    synchronize_contract(
        contract_path,
        summary_path=summary_path,
        base_remote="origin",
        base_branch="main",
        project_root=root,
    )
    synchronized = json.loads(contract_path.read_text(encoding="utf-8"))
    synchronized["synchronizationHistory"][0]["checkpointPaths"] = []

    issues = ai_start_receipt.validate_synchronization_history_structure(
        synchronized, synchronized["startReceipt"]["baseCommit"]
    )

    assert "synchronizationHistory[0].checkpointPaths must be a non-empty list" in issues


def test_synchronize_contract_aborts_conflict_without_evidence_write(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    _write_commit(root, "seed.txt", "work item edit\n")
    _git(root, "switch", "main")
    _write_commit(root, "seed.txt", "corrective edit\n")
    _git(root, "push", "origin", "main")
    _git(root, "switch", "codex/paused-task")
    before_contract = contract_path.read_bytes()
    before_summary = summary_path.read_bytes()
    before_head = _git(root, "rev-parse", "HEAD")

    with pytest.raises(ResumeError, match="rebase conflicted and was aborted"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert contract_path.read_bytes() == before_contract
    assert summary_path.read_bytes() == before_summary
    assert _git(root, "rev-parse", "HEAD") == before_head
    assert _git(root, "status", "--porcelain") == ""


def test_synchronize_contract_keeps_authorized_checkpoint_after_rebase_conflict(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    source = root / "src" / "implementation.py"
    source.parent.mkdir()
    source.write_text("VALUE = 'active work item change'\n", encoding="utf-8")
    writer = tmp_path / "provider-writer"
    _git(
        tmp_path,
        "clone",
        "--branch",
        "main",
        _git(root, "remote", "get-url", "origin"),
        str(writer),
    )
    _git(writer, "config", "user.name", "Provider")
    _git(writer, "config", "user.email", "provider@example.com")
    _write_commit(writer, "src/implementation.py", "VALUE = 'corrective change'\n")
    _git(writer, "push", "origin", "main")
    _git(root, "fetch", "origin", "main")
    before_summary = summary_path.read_bytes()
    before_head = _git(root, "rev-parse", "HEAD")

    with pytest.raises(ResumeError, match="rebase conflicted and was aborted"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    checkpoint_head = _git(root, "rev-parse", "HEAD")
    retained = json.loads(contract_path.read_text(encoding="utf-8"))
    assert checkpoint_head != before_head
    assert retained["synchronizationCheckpoint"]["authorized"] is True
    assert "synchronizationHistory" not in retained
    assert summary_path.read_bytes() == before_summary
    assert _git(root, "status", "--porcelain") == ""


def test_conflicted_synchronization_binds_a_current_main_successor_without_source_mutation(
    tmp_path,
):
    source, contract_path, summary_path, start, target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Preserve owned evidence before governed conflict transition.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    (source / "src").mkdir()
    (source / "src" / "implementation.py").write_text("SOURCE\n", encoding="utf-8")
    outcome = source / ".ai/work-items/active/paused-task.outcome.json"
    outcome.write_text(
        json.dumps(
            {
                "workItemId": "paused-task",
                "status": "blocked",
                "humanStatusColor": "red",
                "failedGate": "synchronization_conflict",
            }
        )
        + "\n",
        encoding="utf-8",
    )
    _git(source, "add", ".ai")
    _git(source, "commit", "-m", "record blocked source evidence")
    _git(source, "add", "src/implementation.py")
    _git(source, "commit", "-m", "source change")
    _git(source, "switch", "main")
    target = _write_commit(source, "src/implementation.py", "TARGET\n")
    _git(source, "push", "origin", "main")
    _git(source, "switch", "codex/paused-task")

    with pytest.raises(ResumeError, match="rebase conflicted and was aborted"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=source,
        )
    source_head = _git(source, "rev-parse", "HEAD")
    source_contract = contract_path.read_bytes()
    source_summary = summary_path.read_bytes()
    source_outcome = outcome.read_bytes()

    successor = tmp_path / "successor"
    _git(
        tmp_path,
        "clone",
        "--branch",
        "main",
        _git(source, "remote", "get-url", "origin"),
        str(successor),
    )
    _git(successor, "config", "user.name", "Test")
    _git(successor, "config", "user.email", "test@example.com")
    _git(successor, "switch", "-c", "codex/recovered-task")
    active = successor / ".ai/work-items/active"
    starts = successor / ".ai/work-items/starts"
    active.mkdir(parents=True)
    starts.mkdir(parents=True)
    successor_contract = {
        "contractVersion": 2,
        "workItemId": "recovered-task",
        "mode": "code",
        "title": "Recovered task",
        "baseCommit": target,
        "scope": ["src/**"],
    }
    successor_receipt = build_receipt(successor_contract, project_root=successor)
    successor_contract["startReceipt"] = receipt_binding(successor_receipt)
    successor_contract_path = active / "recovered-task.contract.json"
    successor_contract_path.write_text(
        json.dumps(successor_contract, indent=2) + "\n", encoding="utf-8"
    )
    (active / "recovered-task.summary.json").write_text(
        json.dumps({"workItemId": "recovered-task", "verification": []}) + "\n",
        encoding="utf-8",
    )
    (starts / "recovered-task.json").write_text(
        json.dumps(successor_receipt, indent=2) + "\n", encoding="utf-8"
    )
    _git(successor, "add", ".ai")
    _git(successor, "commit", "-m", "start successor")

    receipt = ai_resume_work_item.transition_conflicted_synchronization_to_successor(
        source_root=source,
        source_contract_path=contract_path,
        successor_root=successor,
        successor_contract_path=successor_contract_path,
        base_remote="origin",
        base_branch="main",
        issue="https://github.com/spirex-ds-dev/ai-cockpit-template/issues/709",
        authority="user standing authorization recorded in Contract",
        reason="The governed synchronization conflict requires a current-main successor.",
    )

    assert receipt["source"]["checkpointHead"] == source_head
    assert receipt["source"]["baseCommit"] == start
    assert receipt["targetBaseCommit"] == target
    assert receipt["successor"]["workItemId"] == "recovered-task"
    assert contract_path.read_bytes() == source_contract
    assert summary_path.read_bytes() == source_summary
    assert outcome.read_bytes() == source_outcome
    assert _git(source, "rev-parse", "HEAD") == source_head
    assert (successor / ".ai/work-items/conflict-successor-receipts/paused-task.json").is_file()


def test_conflict_successor_rejects_shared_root_and_foreign_issue_before_evidence_reads(tmp_path):
    common = tmp_path / "common"
    common.mkdir()
    kwargs = {
        "source_contract_path": common / "source.contract.json",
        "successor_contract_path": common / "successor.contract.json",
        "base_remote": "origin",
        "base_branch": "main",
        "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/709",
        "authority": "user",
        "reason": "governed recovery",
    }
    with pytest.raises(ResumeError, match="distinct source and successor"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(
            source_root=common, successor_root=common, **kwargs
        )

    successor = tmp_path / "successor"
    successor.mkdir()
    with pytest.raises(ResumeError, match="repository Issue URL"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(
            source_root=common,
            successor_root=successor,
            issue="https://example.invalid/issues/709",
            **{key: value for key, value in kwargs.items() if key != "issue"},
        )


def test_conflict_successor_rejects_blank_authority_and_dirty_worktrees(tmp_path):
    source = tmp_path / "source"
    successor = tmp_path / "successor"
    source.mkdir()
    successor.mkdir()
    _git(source, "init", "-b", "main")
    _git(successor, "init", "-b", "main")
    (successor / "untracked.txt").write_text("dirty\n", encoding="utf-8")
    kwargs = {
        "source_contract_path": source / "source.contract.json",
        "successor_contract_path": successor / "successor.contract.json",
        "base_remote": "origin",
        "base_branch": "main",
        "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/709",
        "reason": "governed recovery",
    }

    with pytest.raises(ResumeError, match="authority and reason"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(
            source_root=source, successor_root=successor, authority=" ", **kwargs
        )

    with pytest.raises(ResumeError, match="clean committed"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(
            source_root=source, successor_root=successor, authority="user", **kwargs
        )


def test_conflict_successor_rejects_missing_duplicate_and_mismatched_source_evidence(
    tmp_path, monkeypatch
):
    source = tmp_path / "source"
    successor = tmp_path / "successor"
    source.mkdir()
    successor.mkdir()
    source_contract_path = source / "source.contract.json"
    successor_contract_path = successor / "successor.contract.json"
    monkeypatch.setattr(ai_resume_work_item, "_clean_worktree", lambda _root: True)
    kwargs = {
        "source_root": source,
        "source_contract_path": source_contract_path,
        "successor_root": successor,
        "successor_contract_path": successor_contract_path,
        "base_remote": "origin",
        "base_branch": "main",
        "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/709",
        "authority": "user",
        "reason": "governed recovery",
    }

    source_contract_path.write_text("{}", encoding="utf-8")
    successor_contract_path.write_text("{}", encoding="utf-8")
    with pytest.raises(ResumeError, match="source Work Item ID is missing"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    source_contract_path.write_text('{"workItemId": "source"}', encoding="utf-8")
    successor_contract_path.write_text('{"workItemId": "source"}', encoding="utf-8")
    with pytest.raises(ResumeError, match="successor Work Item ID must be distinct"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    successor_contract_path.write_text('{"workItemId": "successor"}', encoding="utf-8")
    (source / "source.summary.json").write_text('{"workItemId": "other"}', encoding="utf-8")
    (source / "source.outcome.json").write_text('{"workItemId": "source"}', encoding="utf-8")
    with pytest.raises(ResumeError, match="source Summary Work Item ID does not match"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    (source / "source.summary.json").write_text('{"workItemId": "source"}', encoding="utf-8")
    with pytest.raises(ResumeError, match="retain a blocked Outcome"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    (source / "source.outcome.json").write_text(
        '{"workItemId": "source", "status": "blocked", "failedGate": "other"}',
        encoding="utf-8",
    )
    with pytest.raises(ResumeError, match="not a synchronization conflict"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    (source / "source.outcome.json").write_text(
        '{"workItemId": "source", "status": "blocked", "failedGate": "synchronization_conflict"}',
        encoding="utf-8",
    )
    source_contract_path.write_text(
        '{"workItemId": "source", "synchronizationHistory": {}}', encoding="utf-8"
    )
    with pytest.raises(ResumeError, match="already has a synchronization transition"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    external_contract = tmp_path / "foreign.contract.json"
    external_contract.write_text('{"workItemId": "source"}', encoding="utf-8")
    with pytest.raises(ResumeError, match="source Contract must be inside"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(
            **{**kwargs, "source_contract_path": external_contract}
        )


def test_conflict_successor_rejects_invalid_receipts_and_branch_identities(tmp_path, monkeypatch):
    source = tmp_path / "source"
    successor = tmp_path / "successor"
    source.mkdir()
    successor.mkdir()
    source_contract_path = source / "source.contract.json"
    successor_contract_path = successor / "successor.contract.json"
    source_contract = {"workItemId": "source", "baseCommit": "a" * 40}
    successor_contract = {"workItemId": "successor", "baseCommit": "a" * 40}
    source_summary = {"workItemId": "source"}
    source_outcome = {
        "workItemId": "source",
        "status": "blocked",
        "failedGate": "synchronization_conflict",
    }

    def load_json(_path, description):
        values = {
            "source Contract": source_contract,
            "successor Contract": successor_contract,
            "source Summary": source_summary,
            "source Outcome": source_outcome,
            "source Start Receipt": {},
            "successor Start Receipt": {},
        }
        return values[description]

    monkeypatch.setattr(ai_resume_work_item, "_clean_worktree", lambda _root: True)
    monkeypatch.setattr(ai_resume_work_item, "_load_json", load_json)
    monkeypatch.setattr(
        ai_resume_work_item,
        "receipt_path",
        lambda task, project_root: project_root / f"{task}.json",
    )
    kwargs = {
        "source_root": source,
        "source_contract_path": source_contract_path,
        "successor_root": successor,
        "successor_contract_path": successor_contract_path,
        "base_remote": "origin",
        "base_branch": "main",
        "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/709",
        "authority": "user",
        "reason": "governed recovery",
    }

    monkeypatch.setattr(ai_resume_work_item, "validate_receipt", lambda *_args, **_kwargs: ["bad"])
    with pytest.raises(ResumeError, match="source Work Item evidence is invalid"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    receipt_results = iter(([], ["bad"]))
    monkeypatch.setattr(
        ai_resume_work_item, "validate_receipt", lambda *_args, **_kwargs: next(receipt_results)
    )
    with pytest.raises(ResumeError, match="successor Work Item evidence is invalid"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    monkeypatch.setattr(ai_resume_work_item, "validate_receipt", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_resume_work_item, "_governed_git", lambda *_args, **_kwargs: "codex/other"
    )
    with pytest.raises(ResumeError, match="source branch does not identify"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)

    def branches(root, *_args, **_kwargs):
        return "codex/source" if root == source else "codex/other"

    monkeypatch.setattr(ai_resume_work_item, "_governed_git", branches)
    with pytest.raises(ResumeError, match="successor must be on its dedicated"):
        ai_resume_work_item.transition_conflicted_synchronization_to_successor(**kwargs)


def test_synchronize_contract_rejects_dirty_worktree_before_rebase_or_write(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    (root / "unrelated.txt").write_text("dirty\n", encoding="utf-8")
    before_contract = contract_path.read_bytes()
    before_summary = summary_path.read_bytes()
    before_head = _git(root, "rev-parse", "HEAD")

    with pytest.raises(ResumeError, match="clean dedicated Work Item worktree"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert contract_path.read_bytes() == before_contract
    assert summary_path.read_bytes() == before_summary
    assert _git(root, "rev-parse", "HEAD") == before_head


def test_synchronize_contract_rejects_checkpoint_with_unowned_dirty_path(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["synchronizationCheckpoint"] = {
        "authorized": True,
        "reason": "Record the governed active Work Item synchronization checkpoint.",
    }
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    (root / "unowned.txt").write_text("not governed\n", encoding="utf-8")
    before_head = _git(root, "rev-parse", "HEAD")

    with pytest.raises(ResumeError, match="not Contract-owned: unowned.txt"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert _git(root, "rev-parse", "HEAD") == before_head


def test_synchronize_contract_rejects_replay_without_rewriting_transition(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    synchronize_contract(
        contract_path,
        summary_path=summary_path,
        base_remote="origin",
        base_branch="main",
        project_root=root,
    )
    _git(root, "add", ".ai")
    _git(root, "commit", "-m", "record synchronization")
    before_contract = contract_path.read_bytes()
    before_summary = summary_path.read_bytes()

    with pytest.raises(ResumeError, match="already has a synchronization transition"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert contract_path.read_bytes() == before_contract
    assert summary_path.read_bytes() == before_summary


def test_synchronize_contract_rejects_stale_remote_tracking_ref_without_rebase(tmp_path):
    root, contract_path, summary_path, _start, _target = _synchronization_fixture(tmp_path)
    writer = tmp_path / "provider-writer"
    _git(
        tmp_path,
        "clone",
        "--branch",
        "main",
        _git(root, "remote", "get-url", "origin"),
        str(writer),
    )
    _git(writer, "config", "user.name", "Provider")
    _git(writer, "config", "user.email", "provider@example.com")
    _write_commit(writer, "provider-main.txt", "new remote state\n")
    _git(writer, "push", "origin", "main")
    before_contract = contract_path.read_bytes()
    before_summary = summary_path.read_bytes()
    before_head = _git(root, "rev-parse", "HEAD")

    with pytest.raises(ResumeError, match="remote tracking ref is stale"):
        synchronize_contract(
            contract_path,
            summary_path=summary_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert contract_path.read_bytes() == before_contract
    assert summary_path.read_bytes() == before_summary
    assert _git(root, "rev-parse", "HEAD") == before_head


@pytest.mark.parametrize("resolved", [None, "git"])
def test_governed_git_executable_rejects_missing_or_relative_resolution(monkeypatch, resolved):
    monkeypatch.setattr(ai_resume_work_item.shutil, "which", lambda _name: resolved)

    with pytest.raises(ResumeError, match="absolute Git executable"):
        ai_resume_work_item.governed_git_executable()


def test_resume_contract_appends_source_bound_lineage_without_rewriting_receipt(tmp_path):
    root, contract_path, receipt_file, start, target = _resume_fixture(tmp_path)
    original_receipt = receipt_file.read_bytes()
    original_binding = json.loads(contract_path.read_text(encoding="utf-8"))["startReceipt"]

    transition = resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T01:00:00+00:00",
        project_root=root,
    )

    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    assert transition["fromBaseCommit"] == start
    assert transition["toBaseCommit"] == target
    assert transition["predecessorMergeCommit"] == target
    assert transition["workBranch"] == "codex/paused-task"
    assert len(transition["priorContractDigest"]) == 64
    assert contract["baseCommit"] == target
    assert contract["resumeHistory"] == [transition]
    assert contract["startReceipt"] == original_binding
    assert receipt_file.read_bytes() == original_receipt
    assert validate_receipt(contract, receipt, project_root=root) == []


def test_resume_contract_recovers_base_branch_receipt_without_rewriting_it(tmp_path):
    root, contract_path, receipt_file, _start, target = _resume_fixture(tmp_path)
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    receipt["baseBranch"] = "main"
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    original_receipt = receipt_file.read_bytes()

    transition = resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T01:00:00+00:00",
        project_root=root,
    )

    resumed = json.loads(contract_path.read_text(encoding="utf-8"))
    assert transition["toBaseCommit"] == target
    assert transition["baseBranch"] == "main"
    assert transition["workBranch"] == "codex/paused-task"
    assert receipt_file.read_bytes() == original_receipt
    assert validate_receipt(resumed, receipt, project_root=root) == []


@pytest.mark.parametrize("work_branch", ["codex/unrelated-task", "agent/paused-task"])
def test_resume_contract_rejects_unrelated_branch_for_base_branch_receipt(tmp_path, work_branch):
    root, contract_path, receipt_file, _start, _target = _resume_fixture(tmp_path)
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    receipt["baseBranch"] = "main"
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    _git(root, "switch", "-c", work_branch)
    original_contract = contract_path.read_bytes()
    original_receipt = receipt_file.read_bytes()

    with pytest.raises(ResumeError, match="does not identify this Work Item"):
        resume_contract(
            contract_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )

    assert contract_path.read_bytes() == original_contract
    assert receipt_file.read_bytes() == original_receipt


def test_resume_contract_appends_second_transition_without_rewriting_first(tmp_path):
    root, contract_path, receipt_file, _start, first_target = _resume_fixture(tmp_path)
    first = resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T01:00:00+00:00",
        project_root=root,
    )

    _git(root, "switch", "main")
    second_target = _write_commit(root, "corrective-2.txt", "fixed again\n")
    _git(root, "update-ref", "refs/remotes/origin/main", second_target)
    _git(root, "switch", "codex/paused-task")
    _git(root, "rebase", second_target)
    manifest = _write_predecessor_archive(root, "corrective-2", 2)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    contract["predecessorWorkItem"] = _closed_predecessor("corrective-2", second_target, manifest)
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")

    second = resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T02:00:00+00:00",
        project_root=root,
    )

    resumed = json.loads(contract_path.read_text(encoding="utf-8"))
    assert resumed["resumeHistory"][0] == first
    assert resumed["resumeHistory"][1] == second
    assert second["fromBaseCommit"] == first_target
    assert second["toBaseCommit"] == second_target
    assert (
        validate_receipt(
            resumed,
            json.loads(receipt_file.read_text(encoding="utf-8")),
            project_root=root,
        )
        == []
    )


@pytest.mark.parametrize(
    ("mutation", "expected"),
    [
        (lambda contract: contract.pop("resumeHistory"), "resumeHistory is required"),
        (
            lambda contract: contract["resumeHistory"][0].update({"resumeVersion": 99}),
            "resumeHistory[0].resumeVersion is unsupported",
        ),
        (
            lambda contract: contract["resumeHistory"][0].pop("priorContractDigest"),
            "resumeHistory[0] missing field: priorContractDigest",
        ),
        (
            lambda contract: contract["resumeHistory"][0].update({"fromBaseCommit": "f" * 40}),
            "resumeHistory[0].fromBaseCommit does not continue from the immutable Start Receipt",
        ),
        (
            lambda contract: contract.update({"baseCommit": "e" * 40}),
            "resumeHistory final toBaseCommit does not match Contract baseCommit",
        ),
        (
            lambda contract: contract["resumeHistory"][0].update(
                {"workBranch": "codex/different-task"}
            ),
            "workBranch does not match immutable Start Receipt",
        ),
    ],
)
def test_resume_history_rejects_direct_or_malformed_baseline_transition(
    tmp_path, mutation, expected
):
    root, contract_path, receipt_file, _start, _target = _resume_fixture(tmp_path)
    resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T01:00:00+00:00",
        project_root=root,
    )
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    mutation(contract)
    assert any(
        expected in issue for issue in validate_resume_history(contract, receipt, project_root=root)
    )


@pytest.mark.parametrize(
    ("mutation", "expected"),
    [
        (
            lambda contract: contract["predecessorWorkItem"].update({"status": "open"}),
            "predecessor status must be closed",
        ),
        (
            lambda contract: contract["predecessorWorkItem"]["pr"].update(
                {"mergeCommit": "f" * 40}
            ),
            "predecessor merge commit must equal resume target",
        ),
        (
            lambda contract: contract["predecessorWorkItem"]["closure"].update(
                {"evidence": ".ai/work-items/archive/2026/missing.archive-manifest.json"}
            ),
            "predecessor archive manifest is missing",
        ),
    ],
)
def test_resume_contract_is_atomic_when_source_binding_fails(tmp_path, mutation, expected):
    root, contract_path, receipt_file, _start, _target = _resume_fixture(tmp_path)
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    mutation(contract)
    contract_path.write_text(json.dumps(contract, indent=2) + "\n", encoding="utf-8")
    before_contract = contract_path.read_bytes()
    before_receipt = receipt_file.read_bytes()

    with pytest.raises(ResumeError, match=expected):
        resume_contract(
            contract_path,
            base_remote="origin",
            base_branch="main",
            timestamp="2026-07-28T01:00:00+00:00",
            project_root=root,
        )

    assert contract_path.read_bytes() == before_contract
    assert receipt_file.read_bytes() == before_receipt


def test_resume_history_rejects_non_ancestor_and_manifest_digest_mismatch(tmp_path, monkeypatch):
    root, contract_path, receipt_file, _start, _target = _resume_fixture(tmp_path)
    resume_contract(
        contract_path,
        base_remote="origin",
        base_branch="main",
        timestamp="2026-07-28T01:00:00+00:00",
        project_root=root,
    )
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    manifest_path = root / contract["resumeHistory"][0]["predecessorManifestPath"]
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    manifest["summarySha256"] = "f" * 64
    manifest_path.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
    monkeypatch.setattr(ai_start_receipt, "_git_is_ancestor", lambda *_args: False)

    issues = validate_resume_history(contract, receipt, project_root=root)

    assert "resumeHistory[0]: fromBaseCommit is not an ancestor of toBaseCommit" in issues
    assert "resumeHistory[0]: predecessor manifest summarySha256 does not match" in issues


def test_resume_contract_rejects_wrong_original_branch_and_missing_remote_atomically(
    tmp_path,
):
    root, contract_path, receipt_file, _start, _target = _resume_fixture(tmp_path)
    receipt = json.loads(receipt_file.read_text(encoding="utf-8"))
    receipt["baseBranch"] = "codex/other-task"
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    before = contract_path.read_bytes()

    with pytest.raises(ResumeError, match="current branch does not match immutable Start Receipt"):
        resume_contract(
            contract_path,
            base_remote="origin",
            base_branch="main",
            project_root=root,
        )
    assert contract_path.read_bytes() == before

    receipt["baseBranch"] = "codex/paused-task"
    receipt_file.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
    with pytest.raises(ResumeError, match="Needed a single revision"):
        resume_contract(
            contract_path,
            base_remote="missing",
            base_branch="main",
            project_root=root,
        )
    assert contract_path.read_bytes() == before


def test_resume_cli_reports_success_and_failure(tmp_path, monkeypatch, capsys):
    root, contract_path, _receipt_file, _start, _target = _resume_fixture(tmp_path)
    monkeypatch.setattr(ai_resume_work_item, "PROJECT_ROOT", root)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_resume_work_item.py",
            "--contract",
            str(contract_path.relative_to(root)),
            "--base-remote",
            "origin",
            "--base-branch",
            "main",
        ],
    )
    assert ai_resume_work_item.main() == 0
    assert "Work Item resume recorded:" in capsys.readouterr().out

    def reject(*_args, **_kwargs):
        raise ResumeError("rejected")

    monkeypatch.setattr(ai_resume_work_item, "resume_contract", reject)
    assert ai_resume_work_item.main() == 1
    assert "Work Item resume failed: rejected" in capsys.readouterr().out


def test_synchronize_cli_uses_explicit_target_root_not_caller_root(tmp_path, monkeypatch, capsys):
    target_root, contract_path, summary_path, _start, target = _synchronization_fixture(tmp_path)
    caller_root = tmp_path / "caller"
    caller_root.mkdir()
    caller_contract = caller_root / ".ai" / "work-items" / "active" / "paused-task.contract.json"
    caller_contract.parent.mkdir(parents=True)
    caller_contract.write_text('{"workItemId": "caller-task"}\n', encoding="utf-8")
    caller_before = caller_contract.read_bytes()
    monkeypatch.setattr(ai_resume_work_item, "PROJECT_ROOT", caller_root)
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_resume_work_item.py",
            "--synchronize",
            "--project-root",
            str(target_root),
            "--contract",
            str(contract_path.relative_to(target_root)),
            "--summary",
            str(summary_path.relative_to(target_root)),
            "--base-remote",
            "origin",
            "--base-branch",
            "main",
        ],
    )

    assert ai_resume_work_item.main() == 0
    synchronized = json.loads(contract_path.read_text(encoding="utf-8"))
    assert synchronized["baseCommit"] == target
    assert caller_contract.read_bytes() == caller_before
    assert "Work Item synchronization recorded:" in capsys.readouterr().out


def test_resume_helpers_reject_malformed_inputs_with_specific_diagnostics(tmp_path):
    malformed = tmp_path / "malformed.json"
    malformed.write_text("{", encoding="utf-8")
    with pytest.raises(ResumeError, match="Contract cannot be read"):
        ai_resume_work_item._load_json(malformed, "Contract")

    malformed.write_text("[]", encoding="utf-8")
    with pytest.raises(ResumeError, match="Contract must be a JSON object"):
        ai_resume_work_item._load_json(malformed, "Contract")

    target = "a" * 40
    with pytest.raises(ResumeError, match="predecessorWorkItem must be an evidence object"):
        ai_resume_work_item._predecessor_transition_fields({}, target)

    predecessor = _closed_predecessor("corrective", target, "manifest.json")
    predecessor["closure"]["localBranchDeleted"] = False
    with pytest.raises(ResumeError, match="predecessor closure is incomplete"):
        ai_resume_work_item._predecessor_transition_fields(
            {"predecessorWorkItem": predecessor}, target
        )

    predecessor = _closed_predecessor("", target, "manifest.json")
    with pytest.raises(ResumeError, match="predecessor Work Item ID is missing"):
        ai_resume_work_item._predecessor_transition_fields(
            {"predecessorWorkItem": predecessor}, target
        )

    predecessor = _closed_predecessor("corrective", target, "")
    with pytest.raises(ResumeError, match="predecessor archive manifest path is missing"):
        ai_resume_work_item._predecessor_transition_fields(
            {"predecessorWorkItem": predecessor}, target
        )


def test_resume_contract_rejects_contract_outside_repository(tmp_path):
    repository = tmp_path / "repository"
    repository.mkdir()
    outside = tmp_path / "outside.contract.json"
    outside.write_text('{"workItemId":"outside"}\n', encoding="utf-8")

    with pytest.raises(ResumeError, match="Contract must be inside the repository"):
        resume_contract(
            outside,
            base_remote="origin",
            base_branch="main",
            project_root=repository,
        )


def test_start_receipt_rejects_missing_binding_and_receipt():
    contract = {
        "contractVersion": 2,
        "workItemId": "receipt_task",
        "baseCommit": "a" * 40,
        "scope": [],
    }
    assert validate_receipt(contract, None) == ["Start Receipt is missing"]
    receipt = build_receipt(contract)
    assert "Contract startReceipt binding is missing" in validate_receipt(contract, receipt)


def test_start_receipt_rejects_malformed_fields_and_binding():
    contract = {
        "contractVersion": 2,
        "workItemId": "receipt_task",
        "mode": "code",
        "title": "Receipt",
        "baseCommit": "a" * 40,
        "scope": [],
    }
    receipt = build_receipt(contract, timestamp="not-a-timestamp")
    receipt.update(
        {
            "receiptVersion": 99,
            "workItemId": "other",
            "receiptPath": "wrong.json",
            "baseCommit": "b" * 40,
            "initialScopeDigest": "short",
            "contractSkeletonDigest": "short",
        }
    )
    contract["startReceipt"] = {"path": "wrong.json"}
    issues = validate_receipt(contract, receipt)
    assert len(issues) >= 7
    assert "Start Receipt receiptVersion is unsupported" in issues
    assert "Start Receipt startTimestamp is not ISO-8601" in issues
    assert "Start Receipt initialScopeDigest must be a SHA-256 digest" in issues
    assert "Start Receipt contractSkeletonDigest must be a SHA-256 digest" in issues


def test_start_receipt_helpers_and_tracked_validation(monkeypatch, tmp_path):
    contract = {
        "contractVersion": 2,
        "workItemId": "receipt_task",
        "mode": "code",
        "title": "Receipt",
        "baseCommit": "a" * 40,
        "scope": ["src"],
    }
    receipt = build_receipt(contract, timestamp="2026-07-17T00:00:00+00:00", project_root=tmp_path)
    contract["startReceipt"] = receipt_binding(receipt)
    assert len(scope_digest(contract["scope"])) == 64
    assert receipt_path("receipt_task", project_root=tmp_path).name == "receipt_task.json"
    assert isinstance(current_branch(project_root=tmp_path), str)

    class Result:
        returncode = 1

    monkeypatch.setattr("ai_start_receipt.subprocess.run", lambda *args, **kwargs: Result())
    assert "Start Receipt is not Git-tracked" in validate_receipt(
        contract, receipt, project_root=tmp_path, require_tracked=True
    )


def test_start_receipt_cli_success_and_fail_closed_paths(monkeypatch, tmp_path):
    contract_path = tmp_path / "contract.json"
    receipt_file = tmp_path / "receipt.json"
    contract_path.write_text(json.dumps({"workItemId": "receipt_task"}), encoding="utf-8")
    receipt_file.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(ai_start_receipt, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start_receipt, "receipt_path", lambda _work_item_id: receipt_file)
    monkeypatch.setattr(ai_start_receipt, "validate_receipt", lambda *args, **kwargs: [])
    monkeypatch.setattr(
        sys,
        "argv",
        ["ai_start_receipt.py", "--contract", "contract.json", "--receipt", "receipt.json"],
    )
    assert ai_start_receipt.main() == 0

    monkeypatch.setattr(ai_start_receipt, "validate_receipt", lambda *args, **kwargs: ["bad"])
    assert ai_start_receipt.main() == 1

    monkeypatch.setattr(sys, "argv", ["ai_start_receipt.py", "--contract", "missing.json"])
    assert ai_start_receipt.main() == 1


def test_start_receipt_rejects_invalid_contract_shapes_and_bad_file(monkeypatch, tmp_path):
    for contract in (
        {},
        {"workItemId": "task", "scope": "bad", "baseCommit": "a" * 40},
        {"workItemId": "task", "scope": [1], "baseCommit": ""},
        {"workItemId": "task", "scope": [], "baseCommit": ""},
    ):
        with pytest.raises(ValueError):
            build_receipt(contract, project_root=tmp_path)

    contract_path = tmp_path / "contract.json"
    receipt_file = tmp_path / "receipt.json"
    contract_path.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    receipt_file.write_text("not-json", encoding="utf-8")
    monkeypatch.setattr(ai_start_receipt, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start_receipt, "receipt_path", lambda _work_item_id: receipt_file)
    monkeypatch.setattr(sys, "argv", ["ai_start_receipt.py", "--contract", "contract.json"])
    assert ai_start_receipt.main() == 1

    monkeypatch.setattr(sys, "argv", ["ai_start_receipt.py", "--contract", "missing.json"])
    with pytest.raises(SystemExit):
        runpy.run_path(ai_start_receipt.__file__, run_name="__main__")


def test_scope_guard_adds_bound_receipt_path(monkeypatch):
    class Observation:
        def check_passed(self, **_kwargs):
            return None

        def check_failed(self, **_kwargs):
            return None

        def guard_violation(self, **_kwargs):
            return None

    contract = {
        "workItemId": "receipt_task",
        "scope": ["scripts/ai_start.py"],
        "outOfScope": [],
        "startReceipt": {"path": ".ai/work-items/starts/receipt_task.json"},
    }
    monkeypatch.setattr(ai_check_scope, "load_json", lambda _path: contract)
    monkeypatch.setattr(
        ai_check_scope,
        "changed_paths",
        lambda _contract: [".ai/work-items/starts/receipt_task.json"],
    )
    monkeypatch.setattr(ai_check_scope, "simple_yaml_lists", lambda _path: {})
    monkeypatch.setattr(ai_check_scope, "create_observability", lambda **_kwargs: Observation())
    monkeypatch.setattr(ai_check_scope, "elapsed_ms", lambda _start: 1)
    monkeypatch.setattr(sys, "argv", ["ai_check_scope.py", "contract.json"])
    assert ai_check_scope.main() == 0

    contract["outOfScope"] = [".ai/work-items/starts/**"]
    assert ai_check_scope.main() == 1

    contract["outOfScope"] = []
    contract["destructiveChangePolicy"] = {
        "allowed": True,
        "requiresHumanApproval": False,
        "allowPatterns": [".ai/work-items/starts/**"],
    }
    monkeypatch.setattr(sys, "argv", ["ai_check_scope.py", "contract.json", "--verbose"])
    assert ai_check_scope.main() == 0

    contract["destructiveChangePolicy"]["allowPatterns"] = []
    monkeypatch.setattr(sys, "argv", ["ai_check_scope.py", "contract.json", "--verbose"])
    assert ai_check_scope.main() == 0


def test_scope_guard_adds_bound_active_evidence_paths(monkeypatch):
    class Observation:
        def check_passed(self, **_kwargs):
            return None

        def check_failed(self, **_kwargs):
            return None

        def guard_violation(self, **_kwargs):
            return None

    contract = {"workItemId": "outcome_task", "scope": [], "outOfScope": []}
    monkeypatch.setattr(ai_check_scope, "load_json", lambda _path: contract)
    monkeypatch.setattr(
        ai_check_scope,
        "changed_paths",
        lambda _contract: [
            ".ai/work-items/active/outcome_task.contract.json",
            ".ai/work-items/active/outcome_task.summary.json",
            ".ai/work-items/active/outcome_task.outcome.json",
            ".ai/work-items/active/outcome_task.outcome.md",
        ],
    )
    monkeypatch.setattr(ai_check_scope, "simple_yaml_lists", lambda _path: {})
    monkeypatch.setattr(ai_check_scope, "create_observability", lambda **_kwargs: Observation())
    monkeypatch.setattr(ai_check_scope, "elapsed_ms", lambda _start: 1)
    monkeypatch.setattr(sys, "argv", ["ai_check_scope.py", "contract.json"])
    assert ai_check_scope.main() == 0

    contract["outOfScope"] = [".ai/work-items/active/**"]
    assert ai_check_scope.main() == 1


def test_start_receipt_missing_fields_fails_closed():
    contract = {"workItemId": "receipt_task", "baseCommit": "a" * 40, "scope": []}
    issues = validate_receipt(contract, {})
    assert "Start Receipt missing field: receiptVersion" in issues
    assert "Start Receipt missing field: contractSkeletonDigest" in issues


def test_journey_policy_keeps_refactor_contract_boundaries():
    acceptance, guidelines, out_of_scope, destructive = ai_start.journey_policy("refactor")

    assert (
        "Code structural changes are completed without changing functional behavior." in acceptance
    )
    assert "Zero functional changes allowed." in guidelines
    assert "Adding new features" in out_of_scope
    assert destructive["allowed"] is False


def archive_contract(mode: str = "review") -> dict[str, object]:
    return {
        "contractVersion": 2,
        "workItemId": "task",
        "mode": mode,
        "title": "Task",
        "baseCommit": "a" * 40,
        "baselineDirtyPaths": [],
        "scope": [
            "scripts/ai_archive_work_item.py",
            "tests/test_start_and_archive.py",
            ".ai/cockpit/current_status.md",
            ".ai/work-items/archive/**",
        ],
        "outOfScope": ["Product source changes"],
        "sources": [{"path": "scripts/ai_archive_work_item.py", "reason": "fixture"}],
        "unknowns": [],
        "notCodable": False,
        "acceptance": ["done"],
        "verification": [{"check": "quality", "required": True}],
        "riskAssessment": {"level": "low", "riskTypes": [], "reason": "fixture"},
        "agentCapability": {
            "canImplement": True,
            "canVerify": True,
            "needsHumanDecision": False,
            "blockedReason": "",
        },
        "executionDecision": {"status": "continue", "reason": "fixture"},
        "checkpointPolicy": {
            "requiredBeforeFinish": False,
            "requiredStages": [],
            "reason": "fixture",
        },
        "destructiveChangePolicy": {
            "allowed": False,
            "requiresHumanApproval": True,
            "allowPatterns": [],
        },
        "rollbackNote": "revert",
        "budgetImpact": {"expectedMetrics": {"archiveGrowth": 1}},
    }


def archive_summary(*, verification_result: str = "passed") -> dict[str, object]:
    active_summary = ".ai/work-items/active/task.summary.json"
    return {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [
            {"path": ".ai/work-items/active/task.contract.json", "reason": "contract"},
            {"path": ".ai/work-items/active/task.summary.json", "reason": "summary"},
            {"path": ".ai/work-items/active/task.review.json", "reason": "review"},
        ],
        "sourcesUsed": ["scripts/ai_archive_work_item.py"],
        "acceptanceEvidence": [
            {
                "acceptanceId": "A1",
                "kind": "lifecycle",
                "evidence": [
                    {
                        "type": "lifecycle",
                        "path": active_summary,
                        "locator": "verification and hosted evidence",
                        "verification": "quality",
                    }
                ],
            }
        ],
        "scenarioCoverage": [
            {
                "scenario": "Archived evidence remains resolvable.",
                "required": True,
                "status": "verified",
                "evidence": [active_summary],
            }
        ],
        "userCorrectionSolidification": [
            {
                "correction": "Archive evidence paths.",
                "solidifiedIn": [
                    active_summary,
                    {"nested": [active_summary]},
                ],
                "location": ".ai/work-items/active/task.contract.json",
            }
        ],
        "documentationAlignment": {
            "status": "aligned",
            "checkedAt": "2026-07-28T00:00:00+00:00",
            "checks": [
                {
                    "area": "plan",
                    "status": "not_applicable",
                    "evidence": [],
                    "reason": "fixture",
                },
                {
                    "area": "contractSummaryEvidence",
                    "status": "aligned",
                    "evidence": [".ai/work-items/active/task.contract.json"],
                    "reason": "fixture",
                },
                {
                    "area": "documentationCommandsCapability",
                    "status": "not_applicable",
                    "evidence": [],
                    "reason": "fixture",
                },
                {
                    "area": "multilingualSemantics",
                    "status": "not_applicable",
                    "evidence": [],
                    "reason": "fixture",
                },
                {
                    "area": "limitationsUnknownsHistory",
                    "status": "aligned",
                    "evidence": [".ai/work-items/active/task.contract.json"],
                    "reason": "fixture",
                },
            ],
        },
        "verification": [
            {"check": "quality", "result": verification_result},
            {
                "check": "aiSummary",
                "result": "passed",
                "worktreeDigest": "a" * 64,
                "command": active_summary,
                "executionContractPath": ".ai/work-items/active/task.contract.json",
                "executionSummaryPath": active_summary,
                "outputSummary": active_summary,
                "outputTail": active_summary,
                "futureCapturedOutput": active_summary,
            },
        ],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": active_summary},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }


def write_valid_archive_outcome(
    contract_path: Path,
    summary_path: Path,
    *,
    sources: list[dict[str, str]] | None = None,
) -> None:
    contract = json.loads(contract_path.read_text(encoding="utf-8"))
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    verification = summary.get("verification", [])
    verification_digest = hashlib.sha256(
        json.dumps(verification, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    outcome = generate_outcome(
        contract["workItemId"],
        {
            "taskId": contract["workItemId"],
            "contractDigest": hashlib.sha256(contract_path.read_bytes()).hexdigest(),
            "summaryDigest": hashlib.sha256(summary_path.read_bytes()).hexdigest(),
            "verificationDigest": verification_digest,
            "baseCommit": contract["baseCommit"],
            "headCommit": "b" * 40,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "repository-governance",
            "generatorVersion": "1.2",
        },
        evidence={"locale": "en", "sources": sources or []},
    )
    outcome_path = contract_path.with_name(
        contract_path.name.replace(".contract.json", ".outcome.json")
    )
    outcome_path.write_text(json.dumps(outcome), encoding="utf-8")
    outcome_path.with_suffix(".md").write_text(render_task_outcome(outcome), encoding="utf-8")


def prepare_archive_transaction(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    traceability = tmp_path / "docs" / "reference" / "remediation-instruction-traceability.json"
    active.mkdir(parents=True)
    traceability.parent.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(json.dumps(archive_contract("code")), encoding="utf-8")
    summary.write_text(json.dumps(archive_summary()), encoding="utf-8")
    write_valid_archive_outcome(
        contract,
        summary,
        sources=[
            {"source": ".ai/work-items/active/task.contract.json", "subject": "Contract"},
            {"source": ".ai/work-items/active/task.summary.json", "subject": "Summary"},
        ],
    )
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_archive_work_item,
        "_current_worktree_digest",
        lambda _contract: "a" * 64,
    )
    monkeypatch.setattr(ai_archive_work_item.subprocess, "run", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        ai_archive_work_item,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"record": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])
    return contract, summary, archive, traceability


def stub_active_status(monkeypatch):
    monkeypatch.setattr(ai_start, "write_active_status", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(ai_start, "run_make", lambda *_args, **_kwargs: (0, ""))


def stub_ownership_preview(monkeypatch):
    """Keep lifecycle unit tests focused on their own temporary repository state."""
    monkeypatch.setattr(ai_start, "preview", list)


def test_ai_start_refreshes_only_stale_no_active_status(monkeypatch):
    stale = (
        "cockpit status Changed Files do not match current Git changes; run `make repair-ai-status`"
    )
    no_active_stale = (
        "cockpit status no-active state must not persist changed files; run `make repair-ai-status`"
    )
    calls = []
    monkeypatch.setattr(ai_start, "write_no_active_status", lambda path: calls.append(path))
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)

    assert ai_start.refresh_stale_no_active_status([stale]) == []
    assert calls == [ai_start.DEFAULT_STATUS]
    assert ai_start.refresh_stale_no_active_status([no_active_stale]) == []
    assert calls == [ai_start.DEFAULT_STATUS, ai_start.DEFAULT_STATUS]
    assert ai_start.refresh_stale_no_active_status(["different lifecycle error"]) == [
        "different lifecycle error"
    ]


def test_ai_start_failed_no_active_refresh_restores_status_bytes(tmp_path, monkeypatch):
    status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    status.parent.mkdir(parents=True)
    status.write_bytes(b"original status\n")
    stale = (
        "cockpit status no-active state must not persist changed files; run `make repair-ai-status`"
    )
    persistent = "worktree remains dirty"

    monkeypatch.setattr(ai_start, "DEFAULT_STATUS", status)
    monkeypatch.setattr(
        ai_start,
        "write_no_active_status",
        lambda path: path.write_bytes(b"regenerated status\n"),
    )
    monkeypatch.setattr(ai_start, "validate_status_consistency", lambda: [persistent])

    assert ai_start.refresh_stale_no_active_status([stale]) == [persistent]
    assert status.read_bytes() == b"original status\n"


def test_ai_start_failed_no_active_refresh_removes_new_status(tmp_path, monkeypatch):
    status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    stale = (
        "cockpit status no-active state must not persist changed files; run `make repair-ai-status`"
    )

    monkeypatch.setattr(ai_start, "DEFAULT_STATUS", status)

    def write_status(path):
        path.parent.mkdir(parents=True)
        path.write_bytes(b"regenerated status\n")

    monkeypatch.setattr(ai_start, "write_no_active_status", write_status)
    monkeypatch.setattr(ai_start, "validate_status_consistency", lambda: ["worktree remains dirty"])

    assert ai_start.refresh_stale_no_active_status([stale]) == ["worktree remains dirty"]
    assert not status.exists()


def test_ai_start_default_contains_agent_risk_gate(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *a, **k: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "sample", "--mode", "code"])

    assert ai_start.main() == 0
    contract = json.loads((active / "sample.contract.json").read_text(encoding="utf-8"))
    summary = json.loads((active / "sample.summary.json").read_text(encoding="utf-8"))
    checks = [item["check"] for item in contract["verification"]]
    assert "aiAgentRisk" in checks
    assert "aiCheckpoint" in checks
    assert "aiReviewPolicy" in checks
    assert "aiDiffOwnership" in checks
    assert contract["contractVersion"] == 2
    assert contract["notCodable"] is False
    assert contract["baseCommit"] == "a" * 40
    assert contract["checkpointPolicy"]["requiredStages"] == ["before_edit", "before_finish"]
    assert contract["governanceProfile"] == {
        "selected": "standard",
        "source": "automatic",
        "reasons": ["Initial Work Item skeleton defaults to Standard until scope is classified."],
        "override": None,
    }
    assert ".ai/cockpit/current_status.md" in contract["scope"]
    assert ".ai/work-items/active/sample.outcome.json" in contract["scope"]
    assert ".ai/work-items/active/sample.outcome.md" in contract["scope"]
    assert summary["documentationAlignment"]["status"] == "not_checked"
    assert {item["area"] for item in summary["documentationAlignment"]["checks"]} == {
        "plan",
        "contractSummaryEvidence",
        "documentationCommandsCapability",
        "multilingualSemantics",
        "limitationsUnknownsHistory",
    }
    receipt = tmp_path / ".ai" / "work-items" / "starts" / "sample.json"
    assert receipt.exists()
    assert json.loads(receipt.read_text(encoding="utf-8"))["workItemId"] == "sample"


def test_ai_start_persists_bound_calibration_corrective_in_contract_and_receipt(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    session_path = tmp_path / ".ai" / "calibration" / "session.json"
    session_path.parent.mkdir(parents=True)
    session_path.write_text(
        json.dumps({"sessionId": "calibration-1", "state": "in_progress"}),
        encoding="utf-8",
    )
    corrective = {
        "schemaVersion": 1,
        "sessionPath": ".ai/calibration/session.json",
        "sessionId": "calibration-1",
        "sessionState": "in_progress",
        "sessionDigest": hashlib.sha256(session_path.read_bytes()).hexdigest(),
        "findingId": "CAL-614-001",
        "findingSummary": "Start must expose a bounded corrective route.",
        "authority": "user authorization recorded in issue #614",
        "repairPaths": ["scripts/ai_start.py"],
        "resumeCondition": "Resume calibration through its own Session workflow after closure.",
    }
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *a, **k: None})(),
    )
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_start.py",
            "--task",
            "corrective",
            "--mode",
            "code",
            "--calibration-corrective",
            json.dumps(corrective),
        ],
    )

    assert ai_start.main() == 0

    contract = json.loads((active / "corrective.contract.json").read_text(encoding="utf-8"))
    receipt = json.loads(
        (tmp_path / ".ai" / "work-items" / "starts" / "corrective.json").read_text(encoding="utf-8")
    )
    assert contract["calibrationCorrective"] == corrective
    assert receipt["calibrationCorrectiveDigest"] == ai_start_receipt._digest(corrective)
    assert validate_receipt(contract, receipt, project_root=tmp_path) == []
    receipt["calibrationCorrectiveDigest"] = "0" * 64
    assert "Start Receipt calibrationCorrectiveDigest does not match Contract" in validate_receipt(
        contract, receipt, project_root=tmp_path
    )


def test_ai_start_fails_closed_when_preflight_gate_blocks(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    monkeypatch.setattr(ai_start, "write_active_status", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(ai_start, "run_make", lambda *_args, **_kwargs: (1, "gate blocked"))
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *a, **k: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "blocked", "--mode", "code"])

    assert ai_start.main() == 1
    assert (active / "blocked.contract.json").exists()


def test_ai_start_requires_initial_commit(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "")
    stub_active_status(monkeypatch)
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "sample"])

    assert ai_start.validate_start_state("sample", force=False) is None
    assert ai_start.main() == 1
    assert not (active / "sample.contract.json").exists()


def test_ai_start_refuses_discovered_remote_default_branch_before_writes(tmp_path, monkeypatch):
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    _write_commit(root, "seed.txt", "start\n")
    _git(root, "remote", "add", "origin", str(tmp_path / "remote.git"))
    _git(root, "update-ref", "refs/remotes/origin/main", "HEAD")
    _git(root, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
    active = root / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = root / ".ai" / "cockpit" / "current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text("unchanged\n", encoding="utf-8")
    before_status = status.read_bytes()

    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", root)
    monkeypatch.setattr(ai_start, "DEFAULT_STATUS", status)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "must-not-start"])

    assert ai_start.main() == 1
    assert not (active / "must-not-start.contract.json").exists()
    assert not (active / "must-not-start.summary.json").exists()
    assert not (root / ".ai/work-items/starts/must-not-start.json").exists()
    assert status.read_bytes() == before_status


def test_default_branch_start_guard_allows_matching_dedicated_branch(tmp_path):
    root = tmp_path / "repository"
    root.mkdir()
    _git(root, "init", "-b", "main")
    _git(root, "config", "user.name", "Test")
    _git(root, "config", "user.email", "test@example.com")
    _write_commit(root, "seed.txt", "start\n")
    _git(root, "remote", "add", "origin", str(tmp_path / "remote.git"))
    _git(root, "update-ref", "refs/remotes/origin/main", "HEAD")
    _git(root, "symbolic-ref", "refs/remotes/origin/HEAD", "refs/remotes/origin/main")
    _git(root, "switch", "-c", "codex/sample")

    assert ai_start.default_branch_start_issue(root=root) is None


def test_ai_start_refuses_when_an_active_work_item_already_exists(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    (active / "existing.contract.json").write_text(
        json.dumps({"workItemId": "existing"}), encoding="utf-8"
    )
    (active / "existing.summary.json").write_text(
        json.dumps({"workItemId": "existing"}), encoding="utf-8"
    )
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "sample"])

    assert ai_start.main() == 1
    assert not (active / "sample.contract.json").exists()
    assert not (active / "sample.summary.json").exists()


def test_ai_start_refuses_when_start_lock_is_held(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    lock_path = ai_start.start_lock_path()
    lock_handle = lock_path.open("a+", encoding="utf-8")
    fcntl.flock(lock_handle.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)

    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "sample"])

    try:
        assert ai_start.main() == 1
        assert not (active / "sample.contract.json").exists()
        assert not (active / "sample.summary.json").exists()
    finally:
        lock_handle.close()
        lock_path.unlink(missing_ok=True)


def test_archive_refuses_to_overwrite_existing_audit_record(tmp_path, monkeypatch):
    active = tmp_path / "active"
    archive = tmp_path / "archive"
    active.mkdir()
    contract = active / "task.contract.json"
    contract.write_text(json.dumps(archive_contract("review")), encoding="utf-8")
    year_dir = archive / str(__import__("datetime").datetime.now().year)
    year_dir.mkdir(parents=True)
    (year_dir / contract.name).write_text("existing", encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 1


def test_archive_dry_run_and_successful_review_item(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    contract.write_text(json.dumps(archive_contract("review")), encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract), "--dry-run"])
    assert ai_archive_work_item.main() == 0
    assert contract.exists()

    calls = []

    def fake_run(cmd, cwd=None, check=False, **kwargs):
        calls.append(cmd)

    observer = type("Obs", (), {"record": lambda *_args, **_kwargs: None})()
    monkeypatch.setattr(ai_archive_work_item, "create_observability", lambda **_kwargs: observer)
    monkeypatch.setattr(ai_archive_work_item.subprocess, "run", fake_run)
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])
    assert ai_archive_work_item.main() == 0
    assert not contract.exists()
    assert list(archive.glob("*/task.contract.json"))
    assert any(
        any(str(part).endswith("ai_generate_status.py") for part in cmd) and "--no-active" in cmd
        for cmd in calls
    )
    index = json.loads((archive / "index.json").read_text(encoding="utf-8"))
    assert index["indexVersion"] == 1
    assert index["entries"][0]["workItemId"] == "task"
    assert index["entries"][0]["contractPath"].endswith("task.contract.json")


def test_archive_code_item_rewrites_summary_paths(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    review = active / "task.review.json"
    success = active / "task.success.json"
    events = active / "task.events.jsonl"
    contract_payload = archive_contract("code")
    contract_payload["acceptance"] = ["A1: Archived lifecycle evidence remains resolvable."]
    contract.write_text(json.dumps(contract_payload), encoding="utf-8")
    summary.write_text(json.dumps(archive_summary()), encoding="utf-8")
    review.write_text(json.dumps({"workItemId": "task", "result": "ok"}), encoding="utf-8")
    success.write_text(
        json.dumps(
            {
                "schemaVersion": 1,
                "workItemId": "task",
                "criteria": [
                    {
                        "id": "SC-TASK",
                        "statement": "Archived with the Work Item.",
                        "evidenceHints": ["tests/test_start_and_archive.py"],
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    write_valid_archive_outcome(
        contract,
        summary,
        sources=[
            {"source": ".ai/work-items/active/task.contract.json", "subject": "Contract"},
            {"source": ".ai/work-items/active/task.summary.json", "subject": "Summary"},
        ],
    )
    events.write_text('{"eventType":"completed"}\n', encoding="utf-8")
    report_dir = tmp_path / ".ai" / "cockpit"
    report_dir.mkdir(parents=True)
    report_json = report_dir / "task_report.json"
    report_markdown = report_dir / "task_report.md"
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_archive_work_item,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"record": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(ai_archive_work_item.subprocess, "run", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        ai_generate_human_report,
        "generate_human_report",
        lambda value, *, phase, closure_facts=None, contract=None: {
            "workItemId": value["workItemId"],
            "source": value["sections"]["evidence"][0]["source"],
            "phase": phase,
        },
    )
    monkeypatch.setattr(
        ai_generate_human_report,
        "render_human_report",
        lambda value: f"# {value['workItemId']} ({value['phase']})\n",
    )
    monkeypatch.setattr(
        ai_archive_work_item, "_current_worktree_digest", lambda _contract: "a" * 64
    )
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 0
    archived_summary = next(archive.glob("*/task.summary.json"))
    assert next(archive.glob("*/task.success.json")).exists()
    archived_outcome = json.loads(
        next(archive.glob("*/task.outcome.json")).read_text(encoding="utf-8")
    )
    assert (
        archived_outcome["bindings"]["summaryDigest"]
        == hashlib.sha256(archived_summary.read_bytes()).hexdigest()
    )
    outcome_sources = [item["source"] for item in archived_outcome["sections"]["evidence"]]
    assert outcome_sources == [
        ".ai/work-items/archive/2026/task.contract.json",
        ".ai/work-items/archive/2026/task.summary.json",
    ]
    assert next(archive.glob("*/task.outcome.md")).exists()
    assert next(archive.glob("*/task.events.jsonl")).exists()
    refreshed_report = json.loads(report_json.read_text(encoding="utf-8"))
    assert refreshed_report == {
        "phase": "review",
        "source": ".ai/work-items/archive/2026/task.contract.json",
        "workItemId": "task",
    }
    assert report_markdown.read_text(encoding="utf-8") == "# task (review)\n"
    data = json.loads(archived_summary.read_text(encoding="utf-8"))
    assert data["archiveSequence"] == 1
    assert "/active/" not in data["contractPath"]
    assert all(
        "/active/" not in evidence
        for check in data["documentationAlignment"]["checks"]
        for evidence in check["evidence"]
    )
    assert any(
        evidence.endswith("/archive/2026/task.contract.json")
        for check in data["documentationAlignment"]["checks"]
        for evidence in check["evidence"]
    )
    archived_summary_path = ".ai/work-items/archive/2026/task.summary.json"
    assert data["acceptanceEvidence"][0]["evidence"][0]["path"] == archived_summary_path
    assert data["scenarioCoverage"][0]["evidence"] == [archived_summary_path]
    solidification = data["userCorrectionSolidification"][0]
    assert solidification["solidifiedIn"] == [
        archived_summary_path,
        {"nested": [archived_summary_path]},
    ]
    assert solidification["location"] == ".ai/work-items/archive/2026/task.contract.json"
    immutable_verification = data["verification"][1]
    assert immutable_verification["command"] == ".ai/work-items/active/task.summary.json"
    assert (
        immutable_verification["executionContractPath"]
        == ".ai/work-items/active/task.contract.json"
    )
    assert (
        immutable_verification["executionSummaryPath"] == ".ai/work-items/active/task.summary.json"
    )
    assert immutable_verification["outputSummary"] == ".ai/work-items/active/task.summary.json"
    assert immutable_verification["outputTail"] == ".ai/work-items/active/task.summary.json"
    assert (
        immutable_verification["futureCapturedOutput"] == ".ai/work-items/active/task.summary.json"
    )
    assert data["risk"]["detail"] == ".ai/work-items/active/task.summary.json"
    policy = tmp_path / "scope.yaml"
    policy.write_text("allowAlways:\n", encoding="utf-8")
    changed = [item["path"] for item in data["changedFiles"]]
    monkeypatch.setattr(ai_check_pr, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_pr, "SCOPE_POLICY", policy)
    monkeypatch.setattr(ai_check_pr, "changed_paths", lambda *_args, **_kwargs: changed)
    monkeypatch.setattr(
        ai_check_pr,
        "changed_name_status",
        lambda *_args, **_kwargs: [("A", path) for path in changed],
    )
    monkeypatch.setattr(
        ai_check_pr,
        "run_git",
        lambda *_args: type(
            "Result",
            (),
            {"returncode": 0, "stdout": "", "stderr": ""},
        )(),
    )
    monkeypatch.setattr(ai_check_pr, "validate_contract", lambda _contract: [])
    monkeypatch.setattr(ai_check_pr, "human_benefit_report_issues", lambda _contract: [])
    monkeypatch.setattr(
        ai_check_pr,
        "validate_summary",
        lambda checked_summary, checked_contract, **_kwargs: validate_acceptance_evidence(
            checked_contract,
            checked_summary,
            checked_summary["verification"],
            project_root=tmp_path,
        ),
    )
    aggregate_issues = ai_check_pr.validate_pr_bundle(
        "a" * 40,
        [archived_summary.with_name("task.contract.json")],
    )
    assert not aggregate_issues, aggregate_issues
    assert (
        validate_acceptance_evidence(
            contract_payload,
            data,
            data["verification"],
            project_root=tmp_path,
        )
        == []
    )
    missing_evidence = json.loads(json.dumps(data))
    missing_evidence["acceptanceEvidence"][0]["evidence"][0]["path"] = "missing-evidence.json"
    archived_summary.write_text(json.dumps(missing_evidence), encoding="utf-8")
    aggregate_missing_issues = ai_check_pr.validate_pr_bundle(
        "a" * 40,
        [archived_summary.with_name("task.contract.json")],
    )
    assert any(
        "acceptanceEvidence[0].evidence[0].path does not exist" in issue
        for issue in aggregate_missing_issues
    )
    assert all(
        "/archive/" in item["path"]
        or item["path"]
        in {
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
            ".ai/knowledge/index.json",
            ".ai/knowledge/work-items/task.json",
        }
        for item in data["changedFiles"]
    )
    assert any(item["path"].endswith("task.review.json") for item in data["changedFiles"])
    assert any(item["path"].endswith("task.outcome.json") for item in data["changedFiles"])
    assert any(item["path"] == ".ai/cockpit/current_status.md" for item in data["changedFiles"])
    index = json.loads((archive / "index.json").read_text(encoding="utf-8"))
    assert index["entries"][0]["summaryPath"].endswith("task.summary.json")
    assert len(index["entries"][0]["contractSha256"]) == 64
    assert len(index["entries"][0]["summarySha256"]) == 64
    manifest = next(archive.glob("*/task.archive-manifest.json"))
    manifest_data = json.loads(manifest.read_text(encoding="utf-8"))
    assert manifest_data["format"] == "ai-cockpit-archive-manifest"
    assert manifest_data["generatedStatusExcluded"] is True
    assert {item["path"].split("/")[-1] for item in manifest_data["outcomeArtifacts"]} == {
        "task.outcome.json",
        "task.outcome.md",
        "task.events.jsonl",
    }
    assert len(index["entries"][0]["manifestSha256"]) == 64
    assert index["entries"][0]["manifestPath"].endswith("task.archive-manifest.json")


def test_archive_atomically_rewrites_exact_registered_traceability_contract_paths(
    tmp_path, monkeypatch
):
    contract, _summary, archive, traceability = prepare_archive_transaction(tmp_path, monkeypatch)
    active_contract = ".ai/work-items/active/task.contract.json"
    lookalike = ".ai/work-items/active/task-copy.contract.json"
    command = f"make check-ai-contract CONTRACT={active_contract}"
    traceability.write_text(
        json.dumps(
            {
                "schemaVersion": 1,
                "planPath": "plan.md",
                "instructions": [
                    {
                        "id": "CURRENT",
                        "contractPaths": [active_contract, active_contract, lookalike],
                        "verificationCommands": [command],
                    }
                ],
            }
        ),
        encoding="utf-8",
    )

    assert ai_archive_work_item.main() == 0

    archived_contract = next(archive.glob("*/task.contract.json"))
    archived_relative = archived_contract.relative_to(tmp_path).as_posix()
    payload = json.loads(traceability.read_text(encoding="utf-8"))
    instruction = payload["instructions"][0]
    assert instruction["contractPaths"] == [archived_relative, archived_relative, lookalike]
    assert instruction["verificationCommands"] == [command]
    assert not contract.exists()

    archived_summary = json.loads(
        next(archive.glob("*/task.summary.json")).read_text(encoding="utf-8")
    )
    assert any(
        item["path"] == "docs/reference/remediation-instruction-traceability.json"
        and "archive" in item["reason"].lower()
        for item in archived_summary["changedFiles"]
    )
    assert archived_summary["verification"][0]["result"] == "passed"


def test_archive_rejects_malformed_registered_traceability_before_moving_files(
    tmp_path, monkeypatch
):
    contract, summary, archive, traceability = prepare_archive_transaction(tmp_path, monkeypatch)
    original_index = b'{"indexVersion":1,"entries":[]}\n'
    (archive / "index.json").parent.mkdir(parents=True, exist_ok=True)
    (archive / "index.json").write_bytes(original_index)
    traceability.write_bytes(b"{not-json\n")

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert traceability.read_bytes() == b"{not-json\n"
    assert (archive / "index.json").read_bytes() == original_index
    assert not list(archive.glob("*/task.contract.json"))


def test_archive_traceability_no_reference_is_byte_for_byte_noop(tmp_path, monkeypatch):
    _contract, _summary, _archive, traceability = prepare_archive_transaction(tmp_path, monkeypatch)
    original = (
        b'{\n  "schemaVersion": 1,\n  "contractPaths": '
        b'[".ai/work-items/active/other.contract.json"]\n}\n'
    )
    traceability.write_bytes(original)

    assert ai_archive_work_item.main() == 0
    assert traceability.read_bytes() == original


def test_archive_failure_restores_registered_traceability_bytes(tmp_path, monkeypatch):
    contract, summary, archive, traceability = prepare_archive_transaction(tmp_path, monkeypatch)
    original_contract = contract.read_bytes()
    original_summary = summary.read_bytes()
    active_contract = ".ai/work-items/active/task.contract.json"
    original = json.dumps({"contractPaths": [active_contract]}, indent=2).encode() + b"\n"
    traceability.write_bytes(original)
    status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    status.parent.mkdir(parents=True)
    original_status = b"active status before archive\n"
    status.write_bytes(original_status)
    monkeypatch.setattr(
        ai_archive_work_item,
        "_generate_status",
        lambda _command: status.write_bytes(b"no active work item\n"),
    )
    monkeypatch.setattr(
        ai_archive_work_item,
        "_write_archive_index",
        lambda _index: (_ for _ in ()).throw(OSError("disk full after rewrite")),
    )

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert contract.read_bytes() == original_contract
    assert summary.read_bytes() == original_summary
    assert traceability.read_bytes() == original
    assert status.read_bytes() == original_status
    assert not list(archive.glob("*/task.contract.json"))
    assert not list(archive.glob("*/task.summary.json"))
    assert not list(archive.glob("*/task.archive-manifest.json"))


def test_archive_rolls_back_when_status_regeneration_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(json.dumps(archive_contract("code")), encoding="utf-8")
    summary.write_text(json.dumps(archive_summary()), encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_archive_work_item, "_current_worktree_digest", lambda _contract: "a" * 64
    )

    def fake_run(cmd, cwd=None, check=False):
        if any(str(part).endswith("ai_generate_status.py") for part in cmd):
            raise subprocess.CalledProcessError(returncode=1, cmd=cmd)

    monkeypatch.setattr(ai_archive_work_item.subprocess, "run", fake_run)
    monkeypatch.setattr(
        ai_archive_work_item,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"record": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert not list(archive.glob("*/task.contract.json"))


def test_archive_rolls_back_when_index_write_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(json.dumps(archive_contract("code")), encoding="utf-8")
    summary.write_text(json.dumps(archive_summary()), encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_archive_work_item, "_current_worktree_digest", lambda _contract: "a" * 64
    )
    monkeypatch.setattr(ai_archive_work_item.subprocess, "run", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        ai_archive_work_item,
        "_write_archive_index",
        lambda _index: (_ for _ in ()).throw(OSError("disk full")),
    )
    monkeypatch.setattr(
        ai_archive_work_item,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"record": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert not list(archive.glob("*/task.contract.json"))


def test_archive_rejects_invalid_summary_before_moving_files(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(json.dumps(archive_contract("code")), encoding="utf-8")
    summary.write_text(json.dumps(archive_summary(verification_result="not_run")), encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert not list(archive.rglob("task.contract.json"))


def test_archive_rejects_stale_worktree_digest_before_moving_files(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(json.dumps(archive_contract("code")), encoding="utf-8")
    summary_data = archive_summary()
    summary_data["verification"] = [
        {"check": "quality", "result": "passed"},
        {"check": "aiSummary", "result": "passed", "worktreeDigest": "b" * 64},
    ]
    summary.write_text(json.dumps(summary_data), encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_archive_work_item, "_current_worktree_digest", lambda _contract: "a" * 64
    )
    monkeypatch.setattr(sys, "argv", ["ai_archive_work_item.py", str(contract)])

    assert ai_archive_work_item.main() == 1
    assert contract.exists()
    assert summary.exists()
    assert not list(archive.rglob("task.contract.json"))


def test_ai_start_journeys(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_active_status(monkeypatch)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *a, **k: None})(),
    )

    # Test refactor journey
    monkeypatch.setattr(
        sys,
        "argv",
        ["ai_start.py", "--task", "refactor_task", "--mode", "code", "--journey", "refactor"],
    )
    assert ai_start.main() == 0
    contract = json.loads((active / "refactor_task.contract.json").read_text(encoding="utf-8"))
    summary = json.loads((active / "refactor_task.summary.json").read_text(encoding="utf-8"))
    assert "Zero functional changes allowed." in contract["guidelines"]
    assert "Adding new features" in contract["outOfScope"]
    assert contract["destructiveChangePolicy"]["allowed"] is False
    assert any(
        item["guideline"] == "Zero functional changes allowed."
        for item in summary["guidelinesCompliance"]
    )

    for path in active.glob("*.json"):
        path.unlink()

    # Test cleanup journey
    monkeypatch.setattr(
        sys,
        "argv",
        ["ai_start.py", "--task", "cleanup_task", "--mode", "code", "--journey", "cleanup"],
    )
    assert ai_start.main() == 0
    contract_c = json.loads((active / "cleanup_task.contract.json").read_text(encoding="utf-8"))
    assert contract_c["destructiveChangePolicy"]["allowed"] is False
    assert contract_c["destructiveChangePolicy"]["requiresHumanApproval"] is True


def test_ai_start_generates_active_status(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    generated = []
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    stub_ownership_preview(monkeypatch)
    monkeypatch.setattr(
        ai_start,
        "write_active_status",
        lambda contract, summary, **_kwargs: generated.append((contract, summary)),
    )
    monkeypatch.setattr(ai_start, "run_make", lambda *_args, **_kwargs: (0, ""))
    monkeypatch.setattr(
        ai_start,
        "create_observability",
        lambda **_: type("Obs", (), {"work_item_started": lambda *a, **k: None})(),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "status_task", "--mode", "code"])

    assert ai_start.main() == 0
    assert generated == [
        (active / "status_task.contract.json", active / "status_task.summary.json"),
        (active / "status_task.contract.json", active / "status_task.summary.json"),
    ]


def test_ai_start_rolls_back_pair_when_status_generation_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text("previous status\n", encoding="utf-8")
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_start, "validate_status_consistency", list)
    monkeypatch.setattr(ai_start, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_start, "capture_dirty_baseline", list)
    monkeypatch.setattr(
        ai_start,
        "write_active_status",
        lambda *_: (_ for _ in ()).throw(RuntimeError("status failed")),
    )
    monkeypatch.setattr(sys, "argv", ["ai_start.py", "--task", "status_task", "--mode", "code"])

    assert ai_start.main() == 1
    assert not list(active.glob("status_task.*.json"))
    assert status.read_text(encoding="utf-8") == "previous status\n"


RESOLVED_HANDOFF_TASK = "previous-work-item"


def _resolved_handoff_bindings() -> dict[str, str]:
    return {
        "workItemId": RESOLVED_HANDOFF_TASK,
        "branch": "codex/previous-work-item",
        "headCommit": "a" * 40,
        "tree": "b" * 40,
        "contractDigest": "c" * 64,
        "summaryDigest": "d" * 64,
    }


def _write_resolved_handoff_records(
    root: Path, *, archive: bool, receipt_update: dict[str, object] | None = None
) -> None:
    active = root / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    handoff = build_handoff(
        _resolved_handoff_bindings(),
        action="human.confirm",
        fulfiller="human",
        receipt_kind="human_confirmation",
        deadline="2099-01-01T00:00:00Z",
    )
    receipt: dict[str, object] = {
        "receiptVersion": 1,
        "kind": "human_confirmation",
        "fulfilledBy": "human",
        "bindings": _resolved_handoff_bindings(),
    }
    if receipt_update:
        receipt.update(receipt_update)
    (active / f"{RESOLVED_HANDOFF_TASK}.handoff.json").write_text(
        json.dumps(handoff), encoding="utf-8"
    )
    (active / f"{RESOLVED_HANDOFF_TASK}.receipt.json").write_text(
        json.dumps(receipt), encoding="utf-8"
    )
    if archive:
        archive_dir = root / ".ai" / "work-items" / "archive" / "2026"
        archive_dir.mkdir(parents=True)
        for suffix in ("contract.json", "summary.json", "outcome.json", "archive-manifest.json"):
            (archive_dir / f"{RESOLVED_HANDOFF_TASK}.{suffix}").write_text("{}\n", encoding="utf-8")


def _configure_resolved_handoff_start(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(ai_start, "ACTIVE_DIR", tmp_path / ".ai" / "work-items" / "active")
    monkeypatch.setattr(ai_start, "PROJECT_ROOT", tmp_path)


def test_ai_start_ignores_resolved_handoff_with_complete_archive(tmp_path, monkeypatch):
    _configure_resolved_handoff_start(tmp_path, monkeypatch)
    _write_resolved_handoff_records(tmp_path, archive=True)

    assert ai_start.active_work_item_paths() == []


@pytest.mark.parametrize(
    "archive,receipt_update",
    [
        (False, None),
        (True, {"bindings": {**_resolved_handoff_bindings(), "tree": "f" * 40}}),
        (True, {"receiptVersion": 2}),
    ],
)
def test_ai_start_keeps_unresolved_or_untrusted_handoff_active(
    tmp_path, monkeypatch, archive, receipt_update
):
    _configure_resolved_handoff_start(tmp_path, monkeypatch)
    _write_resolved_handoff_records(tmp_path, archive=archive, receipt_update=receipt_update)

    assert len(ai_start.active_work_item_paths()) == 2


def test_ai_start_keeps_missing_receipt_active(tmp_path, monkeypatch):
    _configure_resolved_handoff_start(tmp_path, monkeypatch)
    _write_resolved_handoff_records(tmp_path, archive=True)
    (tmp_path / ".ai" / "work-items" / "active" / f"{RESOLVED_HANDOFF_TASK}.receipt.json").unlink()

    assert len(ai_start.active_work_item_paths()) == 1


def test_ai_start_keeps_expired_handoff_active(tmp_path, monkeypatch):
    _configure_resolved_handoff_start(tmp_path, monkeypatch)
    _write_resolved_handoff_records(tmp_path, archive=True)
    handoff_path = (
        tmp_path / ".ai" / "work-items" / "active" / f"{RESOLVED_HANDOFF_TASK}.handoff.json"
    )
    handoff = json.loads(handoff_path.read_text(encoding="utf-8"))
    handoff["deadline"] = "2000-01-01T00:00:00Z"
    handoff_path.write_text(json.dumps(handoff), encoding="utf-8")

    assert len(ai_start.active_work_item_paths()) == 2
