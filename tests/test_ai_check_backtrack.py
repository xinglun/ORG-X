"""Regression coverage for approved Work Item evidence cleanup."""

from __future__ import annotations

import json
import sys
from types import SimpleNamespace

import ai_check_backtrack as backtrack


def test_approved_work_item_record_deletion_is_not_reported() -> None:
    path = ".ai/work-items/active/closed.receipt.json"
    assert backtrack.detect_items([("D", path)], authorized_deletions={path}) == []


def test_unapproved_work_item_record_deletion_remains_fail_closed() -> None:
    path = ".ai/work-items/active/unapproved.receipt.json"
    findings = backtrack.detect_items([("D", path)], authorized_deletions=set())
    assert len(findings) == 1
    assert findings[0].kind == "removed_work_item_record"
    assert findings[0].path == path


def test_authorization_requires_matching_approved_contract_and_summary(
    monkeypatch, tmp_path
) -> None:
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    target = ".ai/work-items/active/closed.receipt.json"
    (active / "cleanup.contract.json").write_text(
        json.dumps(
            {
                "destructiveChangePolicy": {
                    "allowed": True,
                    "allowPatterns": [".ai/work-items/active/closed.*.json"],
                    "approvalEvidence": {"approved": True},
                }
            }
        ),
        encoding="utf-8",
    )
    (active / "cleanup.summary.json").write_text(
        json.dumps({"destructiveChanges": [{"path": target, "action": "delete"}]}),
        encoding="utf-8",
    )
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    assert backtrack.authorized_deletion_paths() == {target}


def test_authorization_fails_closed_for_mismatch_or_invalid_shapes(monkeypatch, tmp_path) -> None:
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    (active / "cleanup.contract.json").write_text("{}", encoding="utf-8")
    (active / "other.summary.json").write_text("{}", encoding="utf-8")
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    assert backtrack.authorized_deletion_paths() == set()

    (active / "other.summary.json").rename(active / "cleanup.summary.json")
    (active / "cleanup.contract.json").write_text(
        json.dumps(
            {
                "destructiveChangePolicy": {
                    "allowed": True,
                    "approvalEvidence": {"approved": True},
                    "allowPatterns": [".ai/**", 1],
                }
            }
        ),
        encoding="utf-8",
    )
    assert backtrack.authorized_deletion_paths() == set()


def test_authorization_fails_closed_for_ambiguous_malformed_and_unapproved_inputs(
    monkeypatch, tmp_path
) -> None:
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    summary = active / "cleanup.summary.json"
    summary.write_text(json.dumps({"destructiveChanges": []}), encoding="utf-8")
    contract = active / "cleanup.contract.json"
    contract.write_text("{}", encoding="utf-8")
    (active / "extra.contract.json").write_text("{}", encoding="utf-8")
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)

    assert backtrack.authorized_deletion_paths() == set()

    (active / "extra.contract.json").unlink()
    contract.write_text("not-json", encoding="utf-8")
    assert backtrack.authorized_deletion_paths() == set()

    contract.write_text(
        json.dumps({"destructiveChangePolicy": {"allowed": False}}), encoding="utf-8"
    )
    assert backtrack.authorized_deletion_paths() == set()

    contract.write_text(
        json.dumps(
            {
                "destructiveChangePolicy": {
                    "allowed": True,
                    "approvalEvidence": {"approved": False},
                    "allowPatterns": [".ai/**"],
                }
            }
        ),
        encoding="utf-8",
    )
    assert backtrack.authorized_deletion_paths() == set()

    contract.write_text(
        json.dumps(
            {
                "destructiveChangePolicy": {
                    "allowed": True,
                    "approvalEvidence": {"approved": True},
                    "allowPatterns": [".ai/**"],
                }
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"destructiveChanges": "invalid"}), encoding="utf-8")
    assert backtrack.authorized_deletion_paths() == set()


def test_authorization_ignores_malformed_summary_entries(monkeypatch, tmp_path) -> None:
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    target = ".ai/work-items/active/closed.receipt.json"
    (active / "cleanup.contract.json").write_text(
        json.dumps(
            {
                "destructiveChangePolicy": {
                    "allowed": True,
                    "approvalEvidence": {"approved": True},
                    "allowPatterns": [".ai/work-items/active/**"],
                }
            }
        ),
        encoding="utf-8",
    )
    (active / "cleanup.summary.json").write_text(
        json.dumps(
            {
                "destructiveChanges": [
                    "invalid",
                    {"path": target, "action": "delete"},
                ]
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    assert backtrack.authorized_deletion_paths() == {target}


def test_detect_items_reports_test_snapshot_and_work_item_deletions() -> None:
    findings = backtrack.detect_items(
        [
            ("D", "tests/removed_test.py"),
            ("D", "fixtures/example.snapshot"),
            ("D", ".ai/work-items/active/removed.receipt.json"),
        ]
    )
    assert [finding.kind for finding in findings] == [
        "deleted_test",
        "deleted_snapshot",
        "removed_work_item_record",
    ]


def test_parse_args_accepts_verbose(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["ai_check_backtrack.py", "--verbose"])
    assert backtrack.parse_args().verbose is True


def test_main_reports_no_issues_and_passes_observability(monkeypatch, tmp_path) -> None:
    report = tmp_path / "target" / "backtrack.json"
    policy = tmp_path / "policy.yaml"
    policy.write_text("reportOnly: true\n", encoding="utf-8")
    events: list[str] = []
    observer = SimpleNamespace(
        guard_violation=lambda **_kwargs: events.append("guard"),
        check_failed=lambda **_kwargs: events.append("failed"),
        check_passed=lambda **_kwargs: events.append("passed"),
    )
    monkeypatch.setattr(backtrack, "REPORT_PATH", report)
    monkeypatch.setattr(backtrack, "POLICY_PATH", policy)
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(backtrack, "changed_name_status", list)
    monkeypatch.setattr(backtrack, "authorized_deletion_paths", lambda: set())
    monkeypatch.setattr(backtrack, "create_observability", lambda: observer)
    monkeypatch.setattr(backtrack, "parse_args", lambda: SimpleNamespace(verbose=False))
    assert backtrack.main() == 0
    assert events == ["passed"]
    assert report.exists()


def test_main_verbose_reports_warning_in_report_only_mode(monkeypatch, tmp_path, capsys) -> None:
    report = tmp_path / "target" / "backtrack.json"
    policy = tmp_path / "policy.yaml"
    policy.write_text("reportOnly: true\n", encoding="utf-8")
    events: list[str] = []
    observer = SimpleNamespace(
        guard_violation=lambda **_kwargs: events.append("guard"),
        check_failed=lambda **_kwargs: events.append("failed"),
        check_passed=lambda **_kwargs: events.append("passed"),
    )
    monkeypatch.setattr(backtrack, "REPORT_PATH", report)
    monkeypatch.setattr(backtrack, "POLICY_PATH", policy)
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        backtrack,
        "changed_name_status",
        lambda: [("D", "tests/removed_test.py")],
    )
    monkeypatch.setattr(backtrack, "authorized_deletion_paths", lambda: set())
    monkeypatch.setattr(backtrack, "create_observability", lambda: observer)
    monkeypatch.setattr(backtrack, "parse_args", lambda: SimpleNamespace(verbose=True))

    assert backtrack.main() == 0
    assert events == ["guard", "passed"]
    assert "scanning 1 changed path(s)" in capsys.readouterr().out


def test_main_blocks_warning_when_policy_is_not_report_only(monkeypatch, tmp_path) -> None:
    report = tmp_path / "target" / "backtrack.json"
    policy = tmp_path / "policy.yaml"
    policy.write_text("reportOnly: false\n", encoding="utf-8")
    events: list[str] = []
    observer = SimpleNamespace(
        guard_violation=lambda **_kwargs: events.append("guard"),
        check_failed=lambda **_kwargs: events.append("failed"),
        check_passed=lambda **_kwargs: events.append("passed"),
    )
    monkeypatch.setattr(backtrack, "REPORT_PATH", report)
    monkeypatch.setattr(backtrack, "POLICY_PATH", policy)
    monkeypatch.setattr(backtrack, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(backtrack, "changed_name_status", lambda: [("D", ".ai/work-items/x.json")])
    monkeypatch.setattr(backtrack, "authorized_deletion_paths", lambda: set())
    monkeypatch.setattr(backtrack, "create_observability", lambda: observer)
    monkeypatch.setattr(backtrack, "parse_args", lambda: SimpleNamespace(verbose=False))

    assert backtrack.main() == 1
    assert events == ["guard", "failed"]
