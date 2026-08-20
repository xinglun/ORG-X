import hashlib
import json
import signal
import subprocess
import sys
from datetime import UTC, datetime, timedelta
from types import SimpleNamespace

import ai_check_review_policy
import ai_check_scope
import ai_check_status
import ai_check_status_consistency
import ai_check_summary
import ai_checkpoint
import ai_common
import ai_finish
import ai_generate_human_report
import ai_generate_status
import ai_governance_compression
import pytest


def finish_summary_with_alignment():
    return {
        "verification": [],
        "documentationAlignment": ai_check_summary.complete_generated_documentation_alignment([]),
    }


def test_governance_entrypoints_can_clean_ambient_git_environment():
    assert all(not key.startswith("GIT_") for key in ai_common.clean_git_environment())


def test_nested_make_command_uses_validated_repository_entrypoint(tmp_path):
    (tmp_path / "Makefile.ai").write_text("target:\n\t@true\n", encoding="utf-8")
    environment = {"AI_COCKPIT_MAKE_ENTRYPOINT": "Makefile.ai"}

    assert ai_common.nested_make_command(
        ["make", "target"], root=tmp_path, environment=environment
    ) == ["make", "-f", "Makefile.ai", "target"]
    assert ai_common.nested_make_command(
        ["make", "-f", "Makefile.ai", "target"], root=tmp_path, environment=environment
    ) == ["make", "-f", "Makefile.ai", "target"]


def test_nested_make_command_supports_an_including_gnumakefile(tmp_path):
    (tmp_path / "GNUmakefile").write_text("include Makefile.ai\n", encoding="utf-8")

    assert ai_common.nested_make_command(
        ["make", "target"],
        root=tmp_path,
        environment={"AI_COCKPIT_MAKE_ENTRYPOINT": "GNUmakefile"},
    ) == ["make", "-f", "GNUmakefile", "target"]


@pytest.mark.parametrize(
    ("entrypoint", "command"),
    [
        ("../Makefile.ai", ["make", "target"]),
        ("/tmp/Makefile.ai", ["make", "target"]),
        ("missing/Makefile.ai", ["make", "target"]),
        ("project.mk", ["make", "target"]),
        ("Makefile.ai", ["make", "-f", "Makefile", "target"]),
        ("Makefile.ai", ["make", "--file=Makefile", "target"]),
    ],
)
def test_nested_make_command_rejects_untrusted_or_conflicting_entrypoint(
    tmp_path, entrypoint, command
):
    (tmp_path / "Makefile.ai").write_text("target:\n\t@true\n", encoding="utf-8")
    (tmp_path / "Makefile").write_text("target:\n\t@true\n", encoding="utf-8")

    with pytest.raises(ValueError, match="Make entrypoint"):
        ai_common.nested_make_command(
            command,
            root=tmp_path,
            environment={"AI_COCKPIT_MAKE_ENTRYPOINT": entrypoint},
        )


def test_finish_run_merges_stabilization_environment(monkeypatch):
    captured = {}

    class FakeProcess:
        pid = 901
        returncode = 0

        def communicate(self, timeout=None):
            return "passed\n", None

        def poll(self):
            return self.returncode

        def wait(self, timeout=None):
            return self.returncode

    def fake_popen(command, **kwargs):
        captured["command"] = command
        captured["env"] = kwargs["env"]
        captured["start_new_session"] = kwargs["start_new_session"]
        return FakeProcess()

    monkeypatch.setattr(ai_finish.subprocess, "Popen", fake_popen)
    monkeypatch.delenv("AI_COCKPIT_MAKE_ENTRYPOINT", raising=False)
    code, _, output = ai_finish.run(
        ["make", "check-ai-agent-risk"], extra_env={"AI_FINISH_STABILIZING": "1"}
    )
    assert code == 0
    assert output == "passed\n"
    assert captured["command"] == ["make", "check-ai-agent-risk"]
    assert captured["env"]["AI_FINISH_STABILIZING"] == "1"
    assert captured["start_new_session"] is True


def test_finish_discards_verification_evidence_bound_to_a_prior_contract(tmp_path):
    summary_path = tmp_path / "task.summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "verification": [
                    {"check": "quality", "contractHash": "old-contract"},
                    {"check": "aiScope", "contractHash": "current-contract"},
                    {"check": "legacy", "result": "passed"},
                ]
            }
        ),
        encoding="utf-8",
    )

    removed = ai_finish.discard_stale_contract_verification(summary_path, "current-contract")

    assert removed == 2
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert summary["verification"] == [{"check": "aiScope", "contractHash": "current-contract"}]


def test_finish_summary_text_list_keeps_only_non_empty_strings():
    assert ai_finish._summary_text_list([" completed ", "", 7, "  "]) == ["completed"]


def test_finish_metadata_stale_detects_expired_timezone_aware_owner():
    now = datetime.now(UTC)
    started = now - timedelta(seconds=ai_finish.FINISH_LOCK_MAX_AGE_SECONDS + 1)
    assert ai_finish._metadata_is_stale({"startedAt": started.isoformat()}, now=now)


def test_finish_console_output_is_bounded_but_marks_truncation():
    output = "x" * (ai_finish.CONSOLE_OUTPUT_LIMIT + 10)

    displayed = ai_finish.console_output(output)

    assert displayed.startswith("x" * ai_finish.CONSOLE_OUTPUT_LIMIT)
    assert "output truncated: 10 character(s)" in displayed


def test_finish_run_bounds_large_console_output(monkeypatch, capsys):
    payload = "x" * (ai_finish.CONSOLE_OUTPUT_LIMIT + 10)

    class FakeProcess:
        pid = 902
        returncode = 0

        def communicate(self, timeout=None):
            return payload, None

        def poll(self):
            return self.returncode

        def wait(self, timeout=None):
            return self.returncode

    monkeypatch.setattr(
        ai_finish.subprocess,
        "Popen",
        lambda *_args, **_kwargs: FakeProcess(),
    )

    code, _, output = ai_finish.run(["make", "check-ai-agent-risk"])

    assert code == 0
    assert output == payload
    assert "output truncated: 10 character(s)" in capsys.readouterr().out


class ProcessCleanupFakeProcess:
    def __init__(self, *, timeout=False):
        self.pid = 901
        self.returncode = None
        self.timeout = timeout
        self.communicate_calls = 0
        self.wait_calls = 0

    def communicate(self, timeout=None):
        self.communicate_calls += 1
        if self.timeout and self.communicate_calls == 1:
            raise subprocess.TimeoutExpired(["make", "quality"], timeout)
        self.returncode = 0
        return "captured\n", None

    def poll(self):
        return self.returncode

    def wait(self, timeout=None):
        self.wait_calls += 1
        if self.timeout and self.wait_calls == 1:
            raise subprocess.TimeoutExpired(["make", "quality"], timeout)
        self.returncode = 0
        return self.returncode


def test_finish_timeout_terminates_and_escalates_owned_process_group(monkeypatch):
    process = ProcessCleanupFakeProcess(timeout=True)
    signals = []

    monkeypatch.setattr(ai_finish.subprocess, "Popen", lambda *_args, **_kwargs: process)
    monkeypatch.setattr(ai_finish, "_owned_process_groups", lambda _pid: {901})
    monkeypatch.setattr(
        ai_finish.os,
        "killpg",
        lambda pid, signum: signals.append((pid, signum)),
    )
    monkeypatch.setenv(ai_finish.FINISH_COMMAND_TIMEOUT_ENV, "1")

    code, _duration, output = ai_finish.run(["make", "quality"])

    assert code == 124
    assert "timed out after 1 second(s)" in output
    assert signals == [(901, signal.SIGTERM), (901, signal.SIGKILL)]
    assert process.communicate_calls == 2
    assert process.wait_calls == 2


def test_finish_sigterm_cancellation_cleans_owned_process_group(monkeypatch):
    process = ProcessCleanupFakeProcess()
    signals = []

    monkeypatch.setattr(ai_finish.subprocess, "Popen", lambda *_args, **_kwargs: process)
    monkeypatch.setattr(ai_finish, "_owned_process_groups", lambda _pid: {901})
    monkeypatch.setattr(
        ai_finish.os,
        "killpg",
        lambda pid, signum: signals.append((pid, signum)),
    )

    def communicate(timeout=None):
        handler = signal.getsignal(signal.SIGTERM)
        assert callable(handler)
        handler(signal.SIGTERM, None)
        return "cancelled\n", None

    process.communicate = communicate

    code, _duration, output = ai_finish.run(["make", "quality"])

    assert code == 128 + signal.SIGTERM
    assert "cancelled by signal" in output
    assert signals == [(901, signal.SIGTERM)]


def test_finish_rejects_invalid_timeout_before_spawning(monkeypatch):
    monkeypatch.setenv(ai_finish.FINISH_COMMAND_TIMEOUT_ENV, "0")
    monkeypatch.setattr(
        ai_finish.subprocess,
        "Popen",
        lambda *_args, **_kwargs: pytest.fail("invalid timeout must not spawn a command"),
    )

    code, duration, output = ai_finish.run(["make", "quality"])

    assert code == 2
    assert duration == 0
    assert ai_finish.FINISH_COMMAND_TIMEOUT_ENV in output


def test_finish_signal_targets_only_owned_process_group(monkeypatch):
    process = ProcessCleanupFakeProcess()
    signals = []
    monkeypatch.setattr(
        ai_finish.os,
        "killpg",
        lambda pid, signum: signals.append((pid, signum)),
    )

    ai_finish._signal_owned_process_group(process, signal.SIGTERM)

    assert signals == [(901, signal.SIGTERM)]


def test_finish_process_group_discovery_collects_descendants(monkeypatch):
    class Snapshot:
        stdout = "901 1 901\n902 901 902\n903 902 903\n904 1 904\n"

    monkeypatch.setattr(ai_finish.subprocess, "run", lambda *_args, **_kwargs: Snapshot())

    assert ai_finish._owned_process_groups(901) == {901, 902, 903}


@pytest.fixture(autouse=True)
def isolate_diff_ownership_preview(monkeypatch):
    monkeypatch.setattr(ai_finish, "preview", lambda **_kwargs: [])
    monkeypatch.setattr(ai_finish, "ensure_work_item_branch", lambda: None)


class ObservabilityStub:
    def check_started(self, **kwargs):
        return None

    def check_passed(self, **kwargs):
        return None

    def check_failed(self, **kwargs):
        return None

    def guard_violation(self, **kwargs):
        return None

    def work_item_finished(self, **kwargs):
        return None


def test_review_policy_helpers_parse_focus_and_paths(tmp_path, monkeypatch):
    policy = tmp_path / "review.yaml"
    policy.write_text(
        "requiredReviewChecklist:\n  include:\n    - .ai/**\n  exclude:\n    - .ai/work-items/archive/**\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_review_policy, "POLICY", policy)

    include, exclude = ai_check_review_policy.review_patterns()
    assert ai_check_review_policy.detect(
        [".ai/guards/scope.yaml", ".ai/work-items/archive/task.json", "src/app.py"],
        include=include,
        exclude=exclude,
    ) == [".ai/guards/scope.yaml"]
    assert ai_check_review_policy.review_focus(None) == []
    assert ai_check_review_policy.review_focus(
        {"reviewReadiness": {"expectedReviewFocus": ["CI", ""]}}
    ) == ["CI"]


def test_status_check_main_accepts_matching_ready_status(tmp_path, monkeypatch):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    status = tmp_path / "status.md"
    contract.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "mode": "code",
                "acceptance": ["done"],
                "riskAssessment": {"level": "low", "riskTypes": [], "reason": "fixture"},
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(
        json.dumps(
            {
                "verification": [{"check": "quality", "result": "passed"}],
                "reviewReadiness": {
                    "status": "ready",
                    "reason": "fixture",
                    "expectedReviewFocus": [],
                },
                "unknownsRemaining": [],
                "risk": {"level": "low", "detail": "fixture"},
                "guidelinesCompliance": [],
                "checkpointEvidence": [],
                "residualRisks": [],
            }
        ),
        encoding="utf-8",
    )
    model = ai_governance_compression.derive_governance_status(
        json.loads(contract.read_text(encoding="utf-8")),
        json.loads(summary.read_text(encoding="utf-8")),
    )
    status.write_text(
        ai_governance_compression.render_active_status(
            model,
            work_item_id="task",
            mode="code",
            contract_path=str(contract),
            summary_path=str(summary),
            generated_at="<timestamp>",
            ownership_counts={},
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(
        ai_check_status, "create_observability", lambda **kwargs: ObservabilityStub()
    )
    monkeypatch.setattr(ai_check_status, "BACKTRACK_REPORT", tmp_path / "backtrack.json")
    monkeypatch.setattr(ai_check_status, "ownership_preview", lambda **_kwargs: [])
    monkeypatch.setattr(
        sys,
        "argv",
        ["ai_check_status.py", str(status), "--contract", str(contract), "--summary", str(summary)],
    )

    assert ai_check_status.main() == 0
    assert ai_check_status.required_commands(json.loads(contract.read_text(encoding="utf-8"))) == [
        "quality"
    ]


def test_status_check_rejects_stale_japanese_projection(tmp_path, monkeypatch):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    status = tmp_path / "status.ja.md"
    contract_data = {
        "workItemId": "task",
        "mode": "code",
        "acceptance": ["specific acceptance evidence"],
        "riskAssessment": {"level": "low", "riskTypes": [], "reason": "fixture"},
        "verification": [{"check": "quality", "required": True}],
    }
    summary_data = {
        "verification": [{"check": "quality", "result": "passed"}],
        "reviewReadiness": {
            "status": "ready",
            "reason": "fixture",
            "expectedReviewFocus": [],
        },
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "guidelinesCompliance": [],
        "checkpointEvidence": [],
        "residualRisks": [],
    }
    contract.write_text(json.dumps(contract_data), encoding="utf-8")
    summary.write_text(json.dumps(summary_data), encoding="utf-8")
    model = ai_governance_compression.derive_governance_status(contract_data, summary_data)
    english = ai_governance_compression.render_active_status(
        model,
        work_item_id="task",
        mode="code",
        contract_path=str(contract),
        summary_path=str(summary),
        generated_at="2026-07-29T00:00:00+00:00",
        ownership_counts={},
    )
    status.write_text(
        ai_generate_status.localize_status_markdown(english, "ja"),
        encoding="utf-8",
    )
    monkeypatch.setattr(
        ai_check_status, "create_observability", lambda **kwargs: ObservabilityStub()
    )
    monkeypatch.setattr(ai_check_status, "BACKTRACK_REPORT", tmp_path / "backtrack.json")
    monkeypatch.setattr(ai_check_status, "ownership_preview", lambda **_kwargs: [])
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_check_status.py",
            str(status),
            "--contract",
            str(contract),
            "--summary",
            str(summary),
            "--language",
            "ja",
        ],
    )

    assert ai_check_status.main() == 0
    status.write_text(status.read_text(encoding="utf-8") + "\n手編集\n", encoding="utf-8")
    assert ai_check_status.main() == 1


def test_status_check_main_accepts_generated_status_with_unresolved_ownership(
    tmp_path, monkeypatch
):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    status = tmp_path / "status.md"
    contract.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "mode": "code",
                "acceptance": ["done"],
                "riskAssessment": {"level": "low", "riskTypes": [], "reason": "fixture"},
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(
        json.dumps(
            {
                "verification": [{"check": "quality", "result": "passed"}],
                "reviewReadiness": {
                    "status": "ready",
                    "reason": "fixture",
                    "expectedReviewFocus": [],
                },
                "unknownsRemaining": [],
                "risk": {"level": "low", "detail": "fixture"},
                "guidelinesCompliance": [],
                "checkpointEvidence": [],
                "residualRisks": [],
            }
        ),
        encoding="utf-8",
    )
    unresolved_preview = [
        SimpleNamespace(path="src/app.py", state="unowned"),
    ]

    monkeypatch.setattr(ai_generate_status, "PROJECT_ROOT", tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(
        ai_generate_status, "ownership_preview", lambda **_kwargs: unresolved_preview
    )
    monkeypatch.setattr(ai_check_status, "ownership_preview", lambda **_kwargs: unresolved_preview)
    monkeypatch.setattr(
        ai_generate_status,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"status_generated": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(
        ai_check_status,
        "create_observability",
        lambda **_kwargs: ObservabilityStub(),
    )

    ai_generate_status.write_active_status(
        contract, summary, output=status, observability_log=tmp_path / "events.jsonl"
    )
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_check_status.py",
            "status.md",
            "--contract",
            "task.contract.json",
            "--summary",
            "task.summary.json",
        ],
    )

    assert ai_check_status.main() == 0
    text = status.read_text(encoding="utf-8")
    assert "Recommendation: `needs_investigation`" in text
    assert "diff ownership unresolved: 1" in text


def test_status_check_accepts_generated_bound_external_handoff_and_rejects_stale_content(
    tmp_path, monkeypatch
):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    status = tmp_path / "status.md"
    contract_data = {
        "workItemId": "task",
        "mode": "code",
        "acceptance": ["done"],
        "riskAssessment": {"level": "low", "riskTypes": [], "reason": "fixture"},
        "verification": [{"check": "quality", "required": True}],
    }
    summary_data = {
        "verification": [{"check": "quality", "result": "passed"}],
        "reviewReadiness": {
            "status": "ready",
            "reason": "fixture",
            "expectedReviewFocus": [],
        },
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "guidelinesCompliance": [],
        "checkpointEvidence": [],
        "residualRisks": [],
        "externalHandoff": {
            "handoffVersion": 1,
            "state": "awaiting_external_receipt",
            "bindings": {
                "workItemId": "task",
                "branch": "codex/task",
                "headCommit": "a" * 40,
                "tree": "b" * 40,
                "contractDigest": "c" * 64,
                "summaryDigest": "d" * 64,
            },
            "action": "provider_release.publish",
            "fulfiller": "provider_release",
            "receiptKind": "github_release_result",
            "deadline": "2099-08-10T00:00:00Z",
        },
    }
    contract.write_text(json.dumps(contract_data), encoding="utf-8")
    summary.write_text(json.dumps(summary_data), encoding="utf-8")
    monkeypatch.setattr(ai_generate_status, "PROJECT_ROOT", tmp_path)
    monkeypatch.chdir(tmp_path)
    monkeypatch.setattr(ai_generate_status, "ownership_preview", lambda **_kwargs: [])
    monkeypatch.setattr(ai_check_status, "ownership_preview", lambda **_kwargs: [])
    monkeypatch.setattr(
        ai_generate_status,
        "create_observability",
        lambda **_kwargs: type("Obs", (), {"status_generated": lambda *_args, **_kwargs: None})(),
    )
    monkeypatch.setattr(
        ai_check_status, "create_observability", lambda **_kwargs: ObservabilityStub()
    )
    monkeypatch.setattr(ai_generate_status, "BACKTRACK_REPORT", tmp_path / "backtrack.json")
    monkeypatch.setattr(ai_check_status, "BACKTRACK_REPORT", tmp_path / "backtrack.json")

    ai_generate_status.write_active_status(
        contract, summary, output=status, observability_log=tmp_path / "events.jsonl"
    )
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_check_status.py",
            "status.md",
            "--contract",
            "task.contract.json",
            "--summary",
            "task.summary.json",
        ],
    )

    assert ai_check_status.main() == 0
    status.write_text(status.read_text(encoding="utf-8") + "\nmanual drift\n", encoding="utf-8")
    assert ai_check_status.main() == 1


def test_status_consistency_covers_empty_paired_and_unpaired_states(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / "current_status.md"
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_status_consistency, "ACTIVE_DIR", active)

    status.write_text("- State: `no_active_work_item`\n", encoding="utf-8")
    assert ai_check_status_consistency.validate_status_consistency(status) == []
    assert ai_check_status_consistency.live_no_active_changed_files(status) == []

    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text("{}\n", encoding="utf-8")
    issues = ai_check_status_consistency.validate_status_consistency(status)
    assert any("no matching Summary" in issue for issue in issues)

    summary.write_text("{}\n", encoding="utf-8")
    status.write_text(
        "- State: `in_progress`\n- Task: `task`\n"
        "- Contract Path: `.ai/work-items/active/task.contract.json`\n"
        "- Summary Path: `.ai/work-items/active/task.summary.json`\n",
        encoding="utf-8",
    )
    assert ai_check_status_consistency.validate_status_consistency(status) == []


def test_quality_session_filters_only_transient_project_test_receipt(monkeypatch):
    changed = [
        "Makefile",
        "target/quality/project-test-aggregate/receipt.json",
        "tests/test_core_gates.py",
    ]

    monkeypatch.setenv("QUALITY_SESSION_ID", "quality-session-1")
    assert ai_check_status_consistency.filter_quality_session_transient_paths(changed) == [
        "Makefile",
        "tests/test_core_gates.py",
    ]

    monkeypatch.setenv("QUALITY_SESSION_ID", "legacy")
    assert ai_check_status_consistency.filter_quality_session_transient_paths(changed) == changed


def test_status_consistency_rejects_live_no_active_changes(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / "current_status.md"
    status.write_text(
        "- State: `no_active_work_item`\n\n## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_status_consistency, "ACTIVE_DIR", active)

    def fake_run(command, **kwargs):
        if command[:3] == ["git", "rev-parse", "--verify"]:
            return SimpleNamespace(returncode=0, stdout="head\n")
        if command[:3] == ["git", "diff", "--name-only"]:
            return SimpleNamespace(returncode=0, stdout="src/app.py\n")
        if command[:3] == ["git", "ls-files", "--others"]:
            return SimpleNamespace(returncode=0, stdout="")
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)

    issues = ai_check_status_consistency.validate_status_consistency(status)

    assert issues == [
        (
            "no active Work Item has uncommitted paths outside a complete current archive "
            "transaction: src/app.py; repair-ai-status cannot establish ownership; "
            "restore the paths or create/resume a Work Item"
        )
    ]


def test_status_consistency_rejects_incomplete_uncommitted_archive_evidence(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / "current_status.md"
    status.write_text(
        "- State: `no_active_work_item`\n\n## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_status_consistency, "ACTIVE_DIR", active)

    def fake_run(command, **kwargs):
        if command[:3] == ["git", "rev-parse", "--verify"]:
            return SimpleNamespace(returncode=0, stdout="head\n")
        if command[:3] == ["git", "diff", "--name-only"]:
            return SimpleNamespace(
                returncode=0,
                stdout=".ai/work-items/archive/2026/task.summary.json\n",
            )
        if command[:3] == ["git", "ls-files", "--others"]:
            return SimpleNamespace(returncode=0, stdout="")
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)

    issues = ai_check_status_consistency.validate_status_consistency(status)
    assert len(issues) == 1
    assert ".ai/work-items/archive/2026/task.summary.json" in issues[0]
    assert "repair-ai-status cannot establish ownership" in issues[0]


def _archive_bundle_paths(task: str = "task") -> dict[str, str]:
    archive = f".ai/work-items/archive/2026/{task}"
    return {
        "contract": f"{archive}.contract.json",
        "summary": f"{archive}.summary.json",
        "manifest": f"{archive}.archive-manifest.json",
        "index": ".ai/work-items/archive/index.json",
        "receipt": f".ai/work-items/starts/{task}.json",
    }


def _stub_changed_paths(monkeypatch, changed: list[str]) -> None:
    def fake_run(command, **kwargs):
        if command[:3] == ["git", "rev-parse", "--verify"]:
            return SimpleNamespace(returncode=0, stdout="head\n")
        if command[:3] == ["git", "diff", "--name-only"]:
            return SimpleNamespace(returncode=0, stdout="")
        if command[:3] == ["git", "ls-files", "--others"]:
            return SimpleNamespace(returncode=0, stdout="\n".join(changed))
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)


def test_finish_quality_paths_excludes_all_generated_lifecycle_projections(monkeypatch):
    monkeypatch.setattr(
        ai_finish,
        "changed_paths",
        lambda _contract: [
            "scripts/ai_finish.py",
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
            ".ai/work-items/starts/task.json",
            ".ai/work-items/active/task.contract.json",
            ".ai/work-items/active/task.summary.json",
            ".ai/work-items/active/task.outcome.json",
            ".ai/work-items/active/task.outcome.md",
        ],
    )

    assert ai_finish.finish_quality_paths({"workItemId": "task"}) == ["scripts/ai_finish.py"]


def test_finish_pin_route_binds_to_contract_base_and_current_file(tmp_path, monkeypatch):
    workflow = tmp_path / ".github" / "workflows" / "compatibility.yml"
    workflow.parent.mkdir(parents=True)
    workflow.write_text(
        "jobs:\n  quality:\n    steps:\n"
        "      - uses: dtolnay/rust-toolchain@6c977a6ca4077a0ceb28ffbe03f59d46e9ac8772\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(
            returncode=0,
            stdout=(
                "jobs:\n  quality:\n    steps:\n"
                "      - uses: dtolnay/rust-toolchain@e97e2d8cc328f1b50210efc529dca0028893a2d9\n"
            ),
            stderr="",
        ),
    )

    facts = ai_finish.immutable_pin_facts_for_finish(
        {"baseCommit": "a" * 40}, [".github/workflows/compatibility.yml"]
    )

    assert facts is not None
    assert facts["eligible"] is True


@pytest.mark.parametrize("contract", [{}, {"baseCommit": "a" * 40}])
def test_finish_pin_route_fails_closed_when_base_or_current_evidence_is_unavailable(
    tmp_path, monkeypatch, contract
):
    workflow = tmp_path / ".github" / "workflows" / "compatibility.yml"
    workflow.parent.mkdir(parents=True)
    if contract:
        workflow.write_text("name: compatibility\n", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(returncode=1, stdout="", stderr="missing base"),
    )

    facts = ai_finish.immutable_pin_facts_for_finish(
        contract, [".github/workflows/compatibility.yml"]
    )

    assert facts is not None
    assert facts["eligible"] is False


def _write_archive_manifest(root, paths: dict[str, str], **overrides) -> None:
    contract_target = root / paths["contract"]
    contract_target.parent.mkdir(parents=True, exist_ok=True)
    if not contract_target.exists():
        contract_target.write_text(
            json.dumps({"contractVersion": 2, "workItemId": "task"}),
            encoding="utf-8",
        )
    summary_target = root / paths["summary"]
    if not summary_target.exists():
        _write_archive_summary(root, paths, list(paths.values()))
    manifest = {
        "format": "ai-cockpit-archive-manifest",
        "manifestVersion": 1,
        "workItemId": "task",
        "contractPath": paths["contract"],
        "summaryPath": paths["summary"],
        "contractSha256": hashlib.sha256(contract_target.read_bytes()).hexdigest(),
        "summarySha256": hashlib.sha256(summary_target.read_bytes()).hexdigest(),
        **overrides,
    }
    target = root / paths["manifest"]
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(manifest), encoding="utf-8")


def _write_archive_summary(root, paths: dict[str, str], changed_files: list[str] | object) -> None:
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "changedFiles": (
            [{"path": path, "reason": "fixture"} for path in changed_files]
            if isinstance(changed_files, list)
            else changed_files
        ),
    }
    target = root / paths["summary"]
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(summary), encoding="utf-8")


def _completed_outcome(task: str) -> dict[str, object]:
    return {
        "format": "ai-cockpit-task-outcome",
        "schemaVersion": 1,
        "workItemId": task,
        "status": "completed",
        "bindings": {
            "taskId": task,
            "contractDigest": "a" * 64,
            "summaryDigest": "b" * 64,
            "verificationDigest": "c" * 64,
            "baseCommit": "d" * 40,
            "headCommit": "e" * 40,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "repository-governance",
            "generatorVersion": "1.0",
        },
        "sections": {
            "outcomeSummary": "Implemented the governed change.",
            "taskOverview": f"Governed Work Item: {task}",
            "deliveredChanges": ["scripts/example.py"],
            "findings": [],
            "risks": [],
            "warnings": [],
            "limitations": [],
            "nonRiskExplanations": [],
            "forbiddenClaims": [],
            "interventions": [],
            "forcedStops": [],
            "resolutions": [],
            "recurrencePrevention": [],
            "avoidedImpact": [],
            "residualRisks": [],
            "humanDecisions": [],
            "evidence": [{"source": "contract.json", "subject": "Contract"}],
        },
    }


def test_status_consistency_accepts_transaction_owned_archive_start_receipt(tmp_path, monkeypatch):
    paths = _archive_bundle_paths()
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    _write_archive_summary(tmp_path, paths, list(paths.values()))
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, list(paths.values()))

    assert ai_check_status_consistency.live_no_active_changed_files(status) == []
    assert ai_check_status_consistency.validate_status_consistency(status) == []
    assert ai_check_status_consistency.repair_status(status) == 0


def test_status_consistency_accepts_current_report_pair_bound_to_archived_completed_outcome(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    outcome_path = ".ai/work-items/archive/2026/task.outcome.json"
    report_paths = [".ai/cockpit/task_report.json", ".ai/cockpit/task_report.md"]
    changed = [*paths.values(), outcome_path, *report_paths]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    outcome = _completed_outcome("task")
    outcome_target = tmp_path / outcome_path
    outcome_target.parent.mkdir(parents=True, exist_ok=True)
    outcome_target.write_text(json.dumps(outcome), encoding="utf-8")
    report = ai_generate_human_report.generate_human_report(outcome)
    (tmp_path / report_paths[0]).write_text(json.dumps(report), encoding="utf-8")
    (tmp_path / report_paths[1]).write_text(
        ai_generate_human_report.render_human_report(report), encoding="utf-8"
    )
    _write_archive_summary(tmp_path, paths, changed)
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == []


def test_status_consistency_rejects_stale_current_report_pair_in_archive_transaction(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    outcome_path = ".ai/work-items/archive/2026/task.outcome.json"
    report_paths = [".ai/cockpit/task_report.json", ".ai/cockpit/task_report.md"]
    changed = [*paths.values(), outcome_path, *report_paths]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    outcome = _completed_outcome("task")
    outcome_target = tmp_path / outcome_path
    outcome_target.parent.mkdir(parents=True, exist_ok=True)
    outcome_target.write_text(json.dumps(outcome), encoding="utf-8")
    report = ai_generate_human_report.generate_human_report(outcome)
    (tmp_path / report_paths[0]).write_text(json.dumps(report), encoding="utf-8")
    (tmp_path / report_paths[1]).write_text("stale report\n", encoding="utf-8")
    _write_archive_summary(tmp_path, paths, changed)
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == sorted(changed)


def test_status_consistency_rejects_single_current_report_file_in_archive_transaction(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    outcome_path = ".ai/work-items/archive/2026/task.outcome.json"
    report_path = ".ai/cockpit/task_report.json"
    changed = [*paths.values(), outcome_path, report_path]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    outcome = _completed_outcome("task")
    outcome_target = tmp_path / outcome_path
    outcome_target.parent.mkdir(parents=True, exist_ok=True)
    outcome_target.write_text(json.dumps(outcome), encoding="utf-8")
    (tmp_path / report_path).write_text(
        json.dumps(ai_generate_human_report.generate_human_report(outcome)), encoding="utf-8"
    )
    _write_archive_summary(tmp_path, paths, changed)
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == sorted(changed)


def test_status_consistency_accepts_summary_owned_post_archive_implementation_changes(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    implementation_paths = ["src/app.py", "docs/guide.md"]
    changed = [*paths.values(), *implementation_paths]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    _write_archive_summary(tmp_path, paths, changed)
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == []
    assert ai_check_status_consistency.validate_status_consistency(status) == []


def test_status_consistency_accepts_receipt_owned_post_archive_recovery_change(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    outcome_path = ".ai/work-items/archive/2026/task.outcome.json"
    recovery_path = "target/quality/project-test-aggregate/receipt.json"
    changed = [*paths.values(), outcome_path, recovery_path]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    outcome_target = tmp_path / outcome_path
    outcome_target.parent.mkdir(parents=True, exist_ok=True)
    outcome_target.write_text(json.dumps(_completed_outcome("task")), encoding="utf-8")
    _write_archive_summary(tmp_path, paths, changed)
    _write_archive_manifest(tmp_path, paths)
    recovery_target = tmp_path / ".ai/work-items/recovery-receipts/task.json"
    recovery_target.parent.mkdir(parents=True, exist_ok=True)
    recovery_target.write_text(
        json.dumps(
            {
                "receiptVersion": 1,
                "kind": "same_work_item_post_archive_recovery",
                "workItemId": "task",
                "prBaseCommit": "a" * 40,
                "issue": "https://github.com/example/repo/issues/1",
                "humanAuthorization": {
                    "type": "human",
                    "reference": "same Work Item recovery",
                },
                "failure": {"gate": "changedCriticalCoverage"},
                "archive": {
                    "contract": {
                        "path": paths["contract"],
                        "sha256": hashlib.sha256(
                            (tmp_path / paths["contract"]).read_bytes()
                        ).hexdigest(),
                    },
                    "summary": {
                        "path": paths["summary"],
                        "sha256": hashlib.sha256(
                            (tmp_path / paths["summary"]).read_bytes()
                        ).hexdigest(),
                    },
                    "outcome": {
                        "path": outcome_path,
                        "sha256": hashlib.sha256(outcome_target.read_bytes()).hexdigest(),
                    },
                    "archive-manifest": {
                        "path": paths["manifest"],
                        "sha256": hashlib.sha256(
                            (tmp_path / paths["manifest"]).read_bytes()
                        ).hexdigest(),
                    },
                },
                "recoveryPaths": [recovery_path],
            }
        ),
        encoding="utf-8",
    )
    (tmp_path / recovery_path).parent.mkdir(parents=True, exist_ok=True)
    (tmp_path / recovery_path).write_text("generated\n", encoding="utf-8")
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    monkeypatch.setenv("AI_BASE_COMMIT", "a" * 40)

    def fake_run(command, **kwargs):
        if command[:3] == ["git", "rev-parse", "--verify"]:
            return SimpleNamespace(returncode=0, stdout="head\n")
        if command[:3] == ["git", "diff", "--name-only"]:
            return SimpleNamespace(returncode=0, stdout=f"{recovery_path}\n")
        if command[:3] == ["git", "ls-files", "--others"]:
            return SimpleNamespace(returncode=0, stdout="")
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == []
    assert ai_check_status_consistency.validate_status_consistency(status) == []


def test_status_consistency_rejects_summary_omitted_path_without_false_repair(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    unowned = "src/unowned.py"
    changed = [*paths.values(), unowned]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    original = (
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n"
    )
    status.write_text(original, encoding="utf-8")
    _write_archive_summary(tmp_path, paths, list(paths.values()))
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    commands: list[list[str]] = []

    def fake_run(command, **kwargs):
        commands.append(command)
        if command[:3] == ["git", "rev-parse", "--verify"]:
            return SimpleNamespace(returncode=0, stdout="head\n")
        if command[:3] == ["git", "diff", "--name-only"]:
            return SimpleNamespace(returncode=0, stdout="")
        if command[:3] == ["git", "ls-files", "--others"]:
            return SimpleNamespace(returncode=0, stdout="\n".join(changed))
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)

    issues = ai_check_status_consistency.validate_status_consistency(status)

    assert issues == [
        (
            "no active Work Item has uncommitted paths outside a complete current archive "
            "transaction: src/unowned.py; repair-ai-status cannot establish ownership; "
            "restore the paths or create/resume a Work Item"
        )
    ]
    assert ai_check_status_consistency.repair_status(status) == 1
    assert status.read_text(encoding="utf-8") == original
    assert not any("ai_generate_status.py" in command for command in commands)


def test_status_consistency_rejects_malformed_archive_summary_changed_files(tmp_path, monkeypatch):
    paths = _archive_bundle_paths()
    changed = [*paths.values(), "src/app.py"]
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    _write_archive_summary(tmp_path, paths, "not-a-list")
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == sorted(changed)
    assert ai_check_status_consistency.validate_status_consistency(status)


@pytest.mark.parametrize(
    ("missing_key", "manifest_overrides"),
    [
        ("summary", {}),
        ("index", {}),
        ("contract", {}),
        ("manifest", {}),
        (None, {"workItemId": "other"}),
        (None, {"contractPath": ".ai/work-items/archive/2026/other.contract.json"}),
        (None, {"summaryPath": ".ai/work-items/archive/2026/other.summary.json"}),
        (None, {"format": "unsupported"}),
        (None, {"contractSha256": "0" * 64}),
        (None, {"summarySha256": "0" * 64}),
    ],
)
def test_status_consistency_rejects_unowned_archive_start_receipt(
    tmp_path, monkeypatch, missing_key, manifest_overrides
):
    paths = _archive_bundle_paths()
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    _write_archive_summary(tmp_path, paths, list(paths.values()))
    _write_archive_manifest(tmp_path, paths, **manifest_overrides)
    changed = [path for key, path in paths.items() if key != missing_key]
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, changed)

    assert ai_check_status_consistency.live_no_active_changed_files(status) == sorted(changed)
    assert ai_check_status_consistency.validate_status_consistency(status)


def test_status_consistency_rejects_receipt_paired_only_with_historical_archive(
    tmp_path, monkeypatch
):
    paths = _archive_bundle_paths()
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    _write_archive_manifest(tmp_path, paths)
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, [paths["receipt"]])

    assert ai_check_status_consistency.live_no_active_changed_files(status) == [paths["receipt"]]


def test_status_consistency_rejects_malformed_changed_archive_manifest(tmp_path, monkeypatch):
    paths = _archive_bundle_paths()
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    manifest = tmp_path / paths["manifest"]
    manifest.parent.mkdir(parents=True, exist_ok=True)
    manifest.write_text("{", encoding="utf-8")
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, list(paths.values()))

    assert ai_check_status_consistency.live_no_active_changed_files(status) == sorted(
        paths.values()
    )


def test_status_consistency_accepts_clean_post_commit_no_active_state(tmp_path, monkeypatch):
    status = tmp_path / ".ai/cockpit/current_status.md"
    status.parent.mkdir(parents=True)
    status.write_text(
        "- State: `no_active_work_item`\n"
        "- Worktree Change Count: `0`\n\n"
        "## Changed Files\n\n- none\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_check_status_consistency, "ACTIVE_DIR", tmp_path / ".ai/work-items/active"
    )
    _stub_changed_paths(monkeypatch, [])

    assert ai_check_status_consistency.validate_status_consistency(status) == []


def test_checkpoint_main_reports_required_state(tmp_path, monkeypatch, capsys):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "mode": "code",
                "notCodable": False,
                "executionDecision": {"status": "continue"},
                "scope": ["src/**"],
                "outOfScope": [],
                "unknowns": [],
                "acceptance": ["done"],
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(
        json.dumps(
            {
                "verification": [{"check": "quality", "result": "passed"}],
                "reviewReadiness": {"expectedReviewFocus": ["quality"]},
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "ai_checkpoint.py",
            "--contract",
            str(contract),
            "--summary",
            str(summary),
            "--stage",
            "before_finish",
        ],
    )

    assert ai_checkpoint.main() == 0
    recorded = json.loads(summary.read_text(encoding="utf-8"))["checkpointEvidence"][0]
    assert recorded["stage"] == "before_finish"
    assert recorded["recorded"] is True
    assert recorded["requiredChecks"] == 1
    output = capsys.readouterr().out
    assert "Required Checks Passed: `1`" in output
    assert "problem: not provided" in output
    assert "constraint: not provided" in output
    assert "rationale: not provided" in output
    assert "Ready for final status generation" in output


def test_finish_evidence_redacts_and_replaces_existing_result(tmp_path, monkeypatch):
    summary = tmp_path / "task.summary.json"
    summary.write_text(
        json.dumps(
            {
                "verification": [{"check": "quality", "result": "not_run"}],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    item = ai_finish.evidence(
        "quality",
        "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
        0,
        12,
        "token=secret-value /Users/example/project passed",
        contract_hash="a" * 64,
        commit_sha="b" * 40,
        execution_contract_path=".ai/work-items/active/task.contract.json",
        execution_summary_path=".ai/work-items/active/task.summary.json",
        worktree_digest="c" * 64,
    )
    ai_finish.record_result(summary, item)

    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    assert recorded == [item]
    assert "secret-value" not in item["outputSummary"]
    assert "<LOCAL_PATH>" in item["outputSummary"]
    assert item["outputTail"]
    assert item["outputBytes"] > 0
    truncated_private_key = "".join(["-" * 5, "BEGIN PRIVATE KEY", "-" * 5, "\n", "A" * 40])
    truncated_item = ai_finish.evidence(
        "quality",
        "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
        0,
        12,
        f"prefix {truncated_private_key}",
        contract_hash="a" * 64,
        commit_sha="b" * 40,
        execution_contract_path=".ai/work-items/active/task.contract.json",
        execution_summary_path=".ai/work-items/active/task.summary.json",
        worktree_digest="c" * 64,
    )
    assert "[PRIVATE_KEY_REDACTED]" in truncated_item["outputSummary"]
    assert "BEGIN PRIVATE KEY" not in truncated_item["outputSummary"]
    for key_kind in ("RSA" + " PRIVATE KEY", "OPENSSH" + " PRIVATE KEY"):
        fragment = "".join(["-" * 5, "BEGIN ", key_kind, "-" * 5, "\n", key_kind, "-body-fragment"])
        fragment_item = ai_finish.evidence(
            "quality",
            "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
            0,
            12,
            f"prefix {fragment}",
            contract_hash="a" * 64,
            commit_sha="b" * 40,
            execution_contract_path=".ai/work-items/active/task.contract.json",
            execution_summary_path=".ai/work-items/active/task.summary.json",
            worktree_digest="c" * 64,
        )
        assert fragment_item["outputSummary"] == "prefix [PRIVATE_KEY_REDACTED]"
        assert f"{key_kind}-body-fragment" not in fragment_item["outputSummary"]
    long_private_key = "".join(
        [
            "-" * 5,
            "BEGIN PRIVATE KEY",
            "-" * 5,
            "\n",
            "A" * 800,
            "\n",
            "-" * 5,
            "END PRIVATE KEY",
            "-" * 5,
        ]
    )
    long_item = ai_finish.evidence(
        "quality",
        "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
        0,
        12,
        f"prefix {long_private_key} suffix",
        contract_hash="a" * 64,
        commit_sha="b" * 40,
        execution_contract_path=".ai/work-items/active/task.contract.json",
        execution_summary_path=".ai/work-items/active/task.summary.json",
        worktree_digest="c" * 64,
    )
    assert "[PRIVATE_KEY_REDACTED]" in long_item["outputSummary"]
    assert "BEGIN PRIVATE KEY" not in long_item["outputSummary"]
    assert (
        ai_finish.pending_evidence(
            "quality",
            "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
            contract_hash="a" * 64,
            commit_sha="b" * 40,
            execution_contract_path="contract.json",
            execution_summary_path="summary.json",
            worktree_digest="c" * 64,
        )["runner"]
        == "ai_finish_pending"
    )


def test_finish_worktree_digest_is_stable_and_path_sensitive(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    (tmp_path / "a.txt").write_text("a", encoding="utf-8")
    (tmp_path / "b.txt").write_text("b", encoding="utf-8")

    both = ai_finish.worktree_digest(["b.txt", "a.txt", "a.txt"])

    assert both == ai_finish.worktree_digest(["a.txt", "b.txt"])
    assert both != ai_finish.worktree_digest(["a.txt"])


def test_finish_record_result_requires_active_summary(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    archive.mkdir(parents=True)
    summary = active / "task.summary.json"
    archived_summary = archive / summary.name
    archived_summary.write_text(
        json.dumps({"verification": [{"check": "quality", "result": "not_run"}]}), encoding="utf-8"
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)

    item = ai_finish.evidence(
        "quality",
        "make ai-cockpit-quality GOVERNANCE_PROFILE=standard",
        0,
        1,
        "passed",
        contract_hash="a" * 64,
        commit_sha="b" * 40,
        execution_contract_path=".ai/work-items/active/task.contract.json",
        execution_summary_path=".ai/work-items/active/task.summary.json",
        worktree_digest="c" * 64,
    )
    with pytest.raises(FileNotFoundError, match="summary not found"):
        ai_finish.record_result(summary, item)

    recorded = json.loads(archived_summary.read_text(encoding="utf-8"))["verification"]
    assert recorded == [{"check": "quality", "result": "not_run"}]
    assert not summary.exists()


def test_finish_main_fails_when_contract_is_missing(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(sys, "argv", ["ai_finish.py", "--task", "missing", "--language", "en"])

    assert ai_finish.main() == 1


def test_finish_refuses_repository_base_branch_before_running_checks(tmp_path, monkeypatch, capsys):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    (active / "task.contract.json").write_text(
        json.dumps({"contractVersion": 2, "workItemId": "task", "verification": []}),
        encoding="utf-8",
    )
    (active / "task.summary.json").write_text(json.dumps({"verification": []}), encoding="utf-8")

    def reject_base_branch():
        raise RuntimeError(
            "ai-finish must run on the dedicated Work Item branch; current branch is the repository base branch"
        )

    monkeypatch.setattr(ai_finish, "ensure_work_item_branch", reject_base_branch)
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--archive", "--language", "en"]
    )

    assert ai_finish.main() == 2
    assert "dedicated Work Item branch" in capsys.readouterr().err


def test_finish_branch_guard_compares_current_branch_with_discovered_base(monkeypatch):
    monkeypatch.setattr(ai_finish, "repository_base_branch", lambda: "main")

    with pytest.raises(RuntimeError, match="repository base branch"):
        ai_finish.validate_work_item_branch("main", "main")


def test_finish_branch_discovery_handles_remote_head_and_no_remote_head(monkeypatch):
    def one_remote(args):
        if args == ["remote"]:
            return SimpleNamespace(returncode=0, stdout="origin\n", stderr="")
        return SimpleNamespace(returncode=0, stdout="origin/main\n", stderr="")

    monkeypatch.setattr(ai_finish, "run_git", one_remote)

    assert ai_finish.repository_base_branch() == "main"

    monkeypatch.setattr(
        ai_finish,
        "run_git",
        lambda args: SimpleNamespace(
            returncode=0,
            stdout="",
            stderr="",
        ),
    )
    assert ai_finish.repository_base_branch() is None


@pytest.mark.parametrize("branches", [("main", "trunk"), ("main", "main")])
def test_finish_branch_discovery_rejects_ambiguous_remote_heads(monkeypatch, branches):
    def two_remotes(args):
        if args == ["remote"]:
            return SimpleNamespace(returncode=0, stdout="origin\nupstream\n", stderr="")
        remote = args[-1].split("/")[2]
        branch = branches[0] if remote == "origin" else branches[1]
        return SimpleNamespace(returncode=0, stdout=f"{remote}/{branch}\n", stderr="")

    monkeypatch.setattr(ai_finish, "run_git", two_remotes)

    with pytest.raises(RuntimeError, match="multiple remote HEAD targets"):
        ai_finish.repository_base_branch()


def test_finish_branch_helpers_fail_closed_for_git_errors_and_detached_head(monkeypatch):
    monkeypatch.setattr(
        ai_finish,
        "run_git",
        lambda _args: SimpleNamespace(returncode=1, stdout="", stderr="bad git"),
    )
    with pytest.raises(RuntimeError, match="cannot enumerate Git remotes"):
        ai_finish.repository_base_branch()


def test_finish_allows_branch_when_no_remote_head_is_configured(monkeypatch):
    monkeypatch.setattr(ai_finish, "repository_base_branch", lambda: None)
    monkeypatch.setattr(ai_finish, "_git_output", lambda _args: "codex/task")

    ai_finish.ensure_work_item_branch()


def test_finish_main_records_required_check_failure(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []

    def fail_quality(command, **_kwargs):
        executed.append(command)
        if command == ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"]:
            return 3, 7, "quality failed"
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", fail_quality)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 3
    assert executed == [
        ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"],
        [
            "make",
            "generate-cockpit-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        [
            "make",
            "check-ai-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        ["make", "check-ai-status-consistency"],
    ]
    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    assert [item["check"] for item in recorded] == ["quality"]
    assert recorded[0]["result"] == "failed"
    assert recorded[0]["exitCode"] == 3


def test_finish_main_does_not_inject_release_source_evidence_into_work_item_checks(
    tmp_path, monkeypatch
):
    """A normal Work Item may finish before the post-merge final reassessment."""
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []

    def record_run(command, **_kwargs):
        executed.append(command)
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", record_run)
    monkeypatch.setattr(
        ai_finish, "prepare_pre_archive_candidate_coverage", lambda *_args, **_kwargs: (0, "")
    )
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 0
    assert executed[0] == ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"]
    assert ["make", "sourceBoundEvidence"] not in executed


def test_finish_verification_normalization_ignores_entries_without_a_check_key():
    """Malformed optional entries cannot become executable verification checks."""
    assert ai_finish.inject_mandatory_verification_checks(
        [{"required": False}, {"check": "quality", "required": True}]
    ) == [{"check": "quality", "required": True}]


def test_pre_merge_outcome_requires_a_valid_contract_base_commit(tmp_path, monkeypatch):
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    contract.write_text(json.dumps({"baseCommit": "invalid"}), encoding="utf-8")
    summary.write_text(json.dumps({}), encoding="utf-8")

    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)

    with pytest.raises(ValueError, match="baseCommit"):
        ai_finish._pre_merge_outcome_input("task", contract, summary)


def test_finish_main_rejects_stale_checkpoint_before_declared_checks(tmp_path, monkeypatch, capsys):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "acceptance": ["done"],
                "unknowns": [],
                "checkpointPolicy": {
                    "requiredBeforeFinish": True,
                    "requiredStages": ["before_edit"],
                },
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(
        json.dumps(
            {
                "verification": [],
                "checkpointEvidence": [
                    {
                        "stage": "before_edit",
                        "recorded": True,
                        "contractHash": "stale",
                        "acceptanceCount": 1,
                        "unknownCount": 0,
                        "requiredChecks": 1,
                        "requiredChecksPassed": 0,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "ensure_work_item_branch", lambda: None)
    monkeypatch.setattr(ai_finish, "preview", lambda **_kwargs: [])
    executed = []

    def run(command, **_kwargs):
        executed.append(command)
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", run)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 2
    assert executed == []
    error = capsys.readouterr().err
    assert "checkpointEvidence[before_edit] contractHash is stale" in error
    assert "make ai-revalidate-contract-amendment" in error
    assert "make ai-prepare-implementation" not in error


def test_finish_main_source_bound_failure_stops_quality_and_outcome(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [
                    {"check": "sourceBoundEvidence", "required": True},
                    {"check": "quality", "required": True},
                ],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []

    def fail_source_bound(command, **_kwargs):
        executed.append(command)
        if command == ["make", "sourceBoundEvidence"]:
            return 4, 2, "stale evidence"
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", fail_source_bound)
    monkeypatch.setattr(
        ai_finish,
        "run_task_outcome_pipeline",
        lambda *_args, **_kwargs: pytest.fail("Outcome must not run after source-bound failure"),
    )
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 4
    assert executed == [
        ["make", "sourceBoundEvidence"],
        [
            "make",
            "generate-cockpit-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        [
            "make",
            "check-ai-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        ["make", "check-ai-status-consistency"],
    ]
    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    assert [(item["check"], item["result"]) for item in recorded] == [
        ("sourceBoundEvidence", "failed")
    ]


def test_finish_injects_source_bound_check_only_for_affected_evidence(monkeypatch):
    dependencies = type(
        "Dependencies",
        (),
        {
            "matrix_path": "docs/reference/capability-truth-matrix.json",
            "capability_ids_by_path": {
                "Makefile": ("human_benefit_report",),
                "tests/test_finish.py": ("human_benefit_report",),
            },
        },
    )()
    monkeypatch.setattr(
        ai_finish, "load_capability_evidence_dependencies", lambda _root: dependencies
    )
    monkeypatch.setattr(
        ai_finish,
        "changed_paths",
        lambda _contract: ["Makefile", "scripts/ai_finish.py"],
    )

    checks = ai_finish.inject_mandatory_verification_checks(
        [{"check": "quality", "required": True}],
        contract_data={"workItemId": "task"},
    )

    assert [item["check"] for item in checks] == ["sourceBoundEvidence", "quality"]
    assert checks[-1]["required"] is True


def test_finish_does_not_inject_source_bound_check_for_unrelated_change(monkeypatch):
    dependencies = type(
        "Dependencies",
        (),
        {
            "matrix_path": "docs/reference/capability-truth-matrix.json",
            "capability_ids_by_path": {"Makefile": ("human_benefit_report",)},
        },
    )()
    monkeypatch.setattr(
        ai_finish, "load_capability_evidence_dependencies", lambda _root: dependencies
    )
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: ["scripts/ai_finish.py"])

    checks = ai_finish.inject_mandatory_verification_checks(
        [{"check": "quality", "required": True}],
        contract_data={"workItemId": "task"},
    )

    assert [item["check"] for item in checks] == ["quality"]


def test_finish_main_stabilizes_successful_work_item(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []
    monkeypatch.setattr(
        ai_finish,
        "run",
        lambda command, **_kwargs: executed.append(command) or (0, 2, "passed"),
    )
    monkeypatch.setattr(
        ai_finish, "prepare_pre_archive_candidate_coverage", lambda *_args, **_kwargs: (0, "")
    )
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 0
    # Status is regenerated before each status-derived assertion so persisted
    # verification evidence cannot invalidate the projection it is checking.
    assert len(executed) == 16
    assert executed[0] == ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"]
    assert sum(command[:2] == ["make", "generate-cockpit-status"] for command in executed) == 5
    assert executed[-1][:2] == ["make", "check-ai-status-consistency"]
    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    assert all(item["result"] == "passed" for item in recorded)
    assert {item["check"] for item in recorded} >= {
        "quality",
        "aiStatus",
        "aiSummary",
    }


def test_finish_main_deduplicates_explicit_source_bound_check(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [
                    {"check": "sourceBoundEvidence", "required": False},
                    {"check": "quality", "required": True},
                    {"check": "sourceBoundEvidence", "required": True},
                ],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []

    def fail_quality(command, **_kwargs):
        executed.append(command)
        return (
            (3, 7, "quality failed")
            if command == ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"]
            else (0, 1, "passed")
        )

    monkeypatch.setattr(ai_finish, "run", fail_quality)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 3
    assert executed == [
        ["make", "sourceBoundEvidence"],
        ["make", "ai-cockpit-quality", "GOVERNANCE_PROFILE=standard"],
        [
            "make",
            "generate-cockpit-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        [
            "make",
            "check-ai-status",
            "CONTRACT=.ai/work-items/active/task.contract.json",
            "SUMMARY=.ai/work-items/active/task.summary.json",
        ],
        ["make", "check-ai-status-consistency"],
    ]


def test_finish_main_demotes_readiness_when_final_status_check_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    executed = []

    def fail_final_status(command, **_kwargs):
        executed.append(command)
        is_final_status = len(executed) > 6 and command[:2] == ["make", "check-ai-status"]
        return (1, 2, "status failed") if is_final_status else (0, 2, "passed")

    monkeypatch.setattr(ai_finish, "run", fail_final_status)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 1
    readiness = json.loads(summary.read_text(encoding="utf-8"))["reviewReadiness"]
    assert readiness["status"] == "not_ready"


def test_finish_main_fails_when_summary_is_missing(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    contract.write_text(
        json.dumps({"contractVersion": 2, "workItemId": "task", "verification": []}),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(sys, "argv", ["ai_finish.py", "--task", "task", "--language", "en"])

    assert ai_finish.main() == 1


def test_finish_main_rejects_invalid_verification_list(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps({"contractVersion": 2, "workItemId": "task", "verification": "bad"}),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(sys, "argv", ["ai_finish.py", "--task", "task", "--language", "en"])

    assert ai_finish.main() == 1


def test_finish_main_rejects_skip_quality_for_required_check(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(
        sys,
        "argv",
        ["ai_finish.py", "--task", "task", "--skip-quality", "--no-archive", "--language", "en"],
    )

    assert ai_finish.main() == 2


def test_finish_main_reports_unknown_check_id(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "verification": [{"check": "missingCheck", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    rendered = []

    def render(check, **_kwargs):
        rendered.append(check)
        if check == "missingCheck":
            raise ValueError("unknown check")
        return f"make {check}", ["make", check]

    monkeypatch.setattr(ai_finish, "render_check_command", render)
    executed = []
    monkeypatch.setattr(
        ai_finish,
        "run",
        lambda command, **_kwargs: executed.append(command) or (0, 1, "passed"),
    )
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 2
    assert rendered == ["missingCheck"]
    assert executed == []


def test_finish_main_fails_when_archive_step_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )

    def run(command, **_kwargs):
        if command[:2] == ["make", "archive-work-item"]:
            return 5, 3, "archive failed"
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", run)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        ai_finish, "bind_pre_archive_candidate_coverage_to_outcome", lambda _task: (True, "ok")
    )
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--archive", "--language", "en"]
    )

    assert ai_finish.main() == 5


def test_finish_main_fails_when_stabilization_check_fails(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )

    def run(command, **_kwargs):
        if command[:2] == ["make", "check-ai-status"]:
            return 4, 2, "status failed"
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", run)
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 4


def test_finish_main_allows_optional_check_failure(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [
                    {"check": "quality", "required": True},
                    {"check": "aiReviewPolicy", "required": False},
                ],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )

    def run(command, **_kwargs):
        if command[-1] == "aiReviewPolicy":
            return 1, 1, "optional failed"
        return 0, 1, "passed"

    monkeypatch.setattr(ai_finish, "run", run)
    monkeypatch.setattr(
        ai_finish, "prepare_pre_archive_candidate_coverage", lambda *_args, **_kwargs: (0, "")
    )
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 0
    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    optional = next(item for item in recorded if item["check"] == "aiReviewPolicy")
    assert optional["result"] == "failed"


def test_finish_main_rejects_contract_version_one(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps({"contractVersion": 1, "workItemId": "task", "verification": []}),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 2


def test_finish_main_rejects_inline_command_verification(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "verification": [{"command": "make evil", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps(finish_summary_with_alignment()), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(
        ai_finish,
        "run",
        lambda *_args, **_kwargs: pytest.fail(
            "Malformed verification must be rejected before any command executes"
        ),
    )
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", "task", "--no-archive", "--language", "en"]
    )

    assert ai_finish.main() == 2


def test_finish_main_archives_on_success(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    contract.write_text(
        json.dumps(
            {
                "contractVersion": 2,
                "workItemId": "task",
                "baseCommit": "b" * 40,
                "verification": [{"check": "quality", "required": True}],
            }
        ),
        encoding="utf-8",
    )
    summary.write_text(json.dumps({"verification": []}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda check, **_kwargs: (f"make {check}", ["make", check]),
    )
    monkeypatch.setattr(ai_finish, "run", lambda command, **_kwargs: (0, 1, "passed"))
    monkeypatch.setattr(
        ai_finish, "prepare_pre_archive_candidate_coverage", lambda *_args, **_kwargs: (0, "")
    )
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: ObservabilityStub())
    monkeypatch.setattr(sys, "argv", ["ai_finish.py", "--task", "task", "--language", "en"])

    assert ai_finish.main() == 0


def test_finish_run_executes_command_and_prints_output(tmp_path, monkeypatch, capsys):
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    code, duration, output = ai_finish.run(["printf", "passed"])
    assert code == 0
    assert "passed" in output
    assert duration >= 0
    assert "passed" in capsys.readouterr().out


def test_finish_record_result_replaces_non_list_verification(tmp_path, monkeypatch):
    summary = tmp_path / "task.summary.json"
    summary.write_text('{"verification": "bad"}\n', encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    item = {"check": "quality", "result": "passed"}
    ai_finish.record_result(summary, item)
    recorded = json.loads(summary.read_text(encoding="utf-8"))["verification"]
    assert recorded == [item]


def test_scope_main_reports_out_of_scope_and_dependency_failures(tmp_path, monkeypatch, capsys):
    contract = tmp_path / "task.contract.json"
    contract.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "scope": ["src/**"],
                "outOfScope": ["src/private/**"],
                "destructiveChangePolicy": {"allowed": False},
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(
        ai_check_scope, "changed_paths", lambda _contract: ["src/private/key.py", "README.md"]
    )
    monkeypatch.setattr(
        ai_check_scope,
        "simple_yaml_lists",
        lambda _path: {"dependencyScopeRules.src/**": ["tests/**"]},
    )
    monkeypatch.setattr(
        ai_check_scope, "create_observability", lambda **_kwargs: ObservabilityStub()
    )
    monkeypatch.setattr(sys, "argv", ["ai_check_scope.py", str(contract)])

    assert ai_check_scope.main() == 1
    errors = capsys.readouterr().err
    assert "matches outOfScope" in errors
    assert "dependency scope rule requires tests/**" in errors


def test_review_policy_main_writes_warning_report(tmp_path, monkeypatch):
    policy = tmp_path / ".ai" / "guards" / "ai_review_policy.yaml"
    policy.parent.mkdir(parents=True)
    policy.write_text("requiredReviewChecklist:\n  include:\n    - .ai/**\n", encoding="utf-8")
    summary = tmp_path / "summary.json"
    summary.write_text(
        json.dumps({"workItemId": "task", "reviewReadiness": {"expectedReviewFocus": []}}),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_review_policy, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_review_policy, "POLICY", policy)
    monkeypatch.setattr(ai_check_review_policy, "REPORT", tmp_path / "target" / "review.json")
    monkeypatch.setattr(ai_check_review_policy, "changed_paths", lambda: [".ai/guards/scope.yaml"])
    monkeypatch.setattr(
        ai_check_review_policy, "create_observability", lambda **_kwargs: ObservabilityStub()
    )
    monkeypatch.setattr(sys, "argv", ["ai_check_review_policy.py", "--summary", str(summary)])

    assert ai_check_review_policy.main() == 0
    report = json.loads(ai_check_review_policy.REPORT.read_text(encoding="utf-8"))
    assert report["status"] == "warning"
    assert report["matchedPaths"] == [".ai/guards/scope.yaml"]


def test_status_consistency_repair_no_active_state(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_status_consistency, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_check_status_consistency, "DEFAULT_STATUS", status)

    def fake_run(_command, **_kwargs):
        status.parent.mkdir(parents=True, exist_ok=True)
        status.write_text("- State: `no_active_work_item`\n", encoding="utf-8")
        return SimpleNamespace(returncode=0, stdout="")

    monkeypatch.setattr(ai_check_status_consistency.subprocess, "run", fake_run)
    assert ai_check_status_consistency.repair_status(status) == 0


def test_status_consistency_rejects_no_active_changed_files(tmp_path, monkeypatch):
    active = tmp_path / ".ai" / "work-items" / "active"
    active.mkdir(parents=True)
    status = tmp_path / "current_status.md"
    status.write_text(
        "- State: `no_active_work_item`\n\n## Changed Files\n\n- `src/old.py`\n\n## Next Action\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_check_status_consistency, "ACTIVE_DIR", active)
    issues = ai_check_status_consistency.validate_status_consistency(status)
    assert any("must not persist changed files" in issue for issue in issues)
