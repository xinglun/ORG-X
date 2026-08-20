import argparse
import hashlib
import json
import sys

import ai_check_agent_risk
import ai_finish
import ai_generate_human_report as human
import ai_generate_status
import pytest
from ai_check_task_outcome import validate_outcome
from ai_generate_task_outcome import generate_outcome, render_markdown
from ai_governance_compression import render_active_status


class _FinishObservation:
    def __init__(self):
        self.started = []
        self.passed = []
        self.failed = []

    def check_started(self, **kwargs):
        self.started.append(kwargs)

    def check_passed(self, **kwargs):
        self.passed.append(kwargs)

    def check_failed(self, **kwargs):
        self.failed.append(kwargs)


def test_finish_refreshes_capability_truth_before_source_bound_evidence(tmp_path, monkeypatch):
    matrix_path = tmp_path / "docs" / "reference" / "capability-truth-matrix.json"
    generator_path = tmp_path / "scripts" / "ai_capability_truth.py"
    summary_path = tmp_path / "summary.json"
    contract_path = tmp_path / "contract.json"
    matrix_path.parent.mkdir(parents=True)
    generator_path.parent.mkdir(parents=True)
    matrix_path.write_text(
        json.dumps({"capabilities": [{"evidenceSource": "stale", "digest": "stale"}]}),
        encoding="utf-8",
    )
    generator_path.write_text("# test generator\n", encoding="utf-8")
    summary_path.write_text(
        json.dumps({"changedFiles": [], "generatedFiles": [], "verification": []}),
        encoding="utf-8",
    )
    contract_path.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish,
        "changed_paths",
        lambda _contract_data: ["docs/reference/capability-truth-matrix.json"],
    )
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda *_args, **_kwargs: (
            "make check-source-bound-evidence",
            ["make", "check-source-bound-evidence"],
        ),
    )
    calls = []

    def fake_run(command, **_kwargs):
        calls.append(command)
        if "--write" in command:
            matrix_path.write_text(
                json.dumps(
                    {
                        "capabilities": [
                            {
                                "evidenceSource": {"digest": "fresh-evidence"},
                                "digest": "fresh-row",
                            }
                        ]
                    }
                ),
                encoding="utf-8",
            )
            return 0, 11, "capability truth matrix refreshed"
        assert command == ["make", "check-source-bound-evidence"]
        assert calls[0][-1] == "--write"
        return 0, 19, "source-bound evidence passed"

    monkeypatch.setattr(ai_finish, "run", fake_run)
    observation = _FinishObservation()

    result = ai_finish.run_declared_checks(
        [{"check": "sourceBoundEvidence", "required": True}],
        args=argparse.Namespace(skip_quality=False),
        contract="contract.json",
        summary="summary.json",
        contract_data={},
        contract_path=contract_path,
        summary_path=summary_path,
        contract_hash="contract-hash",
        commit_sha="commit-sha",
        obs=observation,
    )

    assert result == 0
    assert calls[0][0] == sys.executable
    assert calls[0][1:] == ["scripts/ai_capability_truth.py", "--write"]
    assert json.loads(matrix_path.read_text(encoding="utf-8"))["capabilities"][0] == {
        "evidenceSource": {"digest": "fresh-evidence"},
        "digest": "fresh-row",
    }
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert any(
        item.get("path") == "docs/reference/capability-truth-matrix.json"
        for item in summary["changedFiles"]
    )
    assert "docs/reference/capability-truth-matrix.json" in summary["generatedFiles"]
    evidence = summary["verification"][0]
    assert evidence["check"] == "sourceBoundEvidence"
    assert evidence["result"] == "passed"
    assert "capability truth matrix refreshed" in evidence["outputTail"]
    assert observation.failed == []


def test_finish_fails_closed_when_source_bound_refresh_fails(tmp_path, monkeypatch):
    summary_path = tmp_path / "summary.json"
    contract_path = tmp_path / "contract.json"
    summary_path.write_text(
        json.dumps({"changedFiles": [], "generatedFiles": [], "verification": []}),
        encoding="utf-8",
    )
    contract_path.write_text("{}", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish,
        "render_check_command",
        lambda *_args, **_kwargs: (
            "make check-source-bound-evidence",
            ["make", "check-source-bound-evidence"],
        ),
    )
    monkeypatch.setattr(
        ai_finish,
        "refresh_source_bound_evidence",
        lambda *, summary_path: (7, 13, "capability truth refresh failed"),
    )
    check_calls = []
    monkeypatch.setattr(ai_finish, "run", lambda command, **_kwargs: check_calls.append(command))
    observation = _FinishObservation()

    result = ai_finish.run_declared_checks(
        [{"check": "sourceBoundEvidence", "required": True}],
        args=argparse.Namespace(skip_quality=False),
        contract="contract.json",
        summary="summary.json",
        contract_data={},
        contract_path=contract_path,
        summary_path=summary_path,
        contract_hash="contract-hash",
        commit_sha="commit-sha",
        obs=observation,
    )

    assert result == 7
    assert check_calls == []
    evidence = json.loads(summary_path.read_text(encoding="utf-8"))["verification"][0]
    assert evidence["check"] == "sourceBoundEvidence"
    assert evidence["result"] == "failed"
    assert "capability truth refresh failed" in evidence["outputTail"]
    assert observation.failed[0]["check_id"] == "sourceBoundEvidence"


def test_adoption_finish_projects_bootstrap_as_not_applicable_approach(tmp_path, monkeypatch):
    contract_path = tmp_path / "task.contract.json"
    summary_path = tmp_path / "task.summary.json"
    contract_path.write_text(
        json.dumps(
            {
                "workItemId": "adopt_ai_cockpit",
                "baseCommit": "a" * 40,
                "adoptionBootstrapPaths": ["scripts/ai_*.py"],
                "scope": ["scripts/ai_*.py"],
                "verification": [],
            }
        ),
        encoding="utf-8",
    )
    summary_path.write_text(
        json.dumps({"changedFiles": [], "verification": [], "unknownsRemaining": []}),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)
    monkeypatch.chdir(tmp_path)

    payload = ai_finish._pre_merge_outcome_input(
        "adopt_ai_cockpit", contract_path, summary_path, "en"
    )

    assert payload["evidence"]["implementationApproach"]["status"] == "not_applicable"


def test_source_bound_refresh_registers_all_declared_generated_documents(tmp_path, monkeypatch):
    generated = (
        "docs/reference/capability-truth-matrix.json",
        "docs/reference/capability-truth-matrix.md",
        "docs/reference/pre-release-documentation-alignment.json",
        "docs/reference/pre-release-documentation-alignment.md",
        "docs/reference/japanese-capability-assessment.json",
        "docs/reference/japanese-capability-assessment.md",
    )
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "generatedFiles": [],
                "verification": [],
                "documentationAlignment": {
                    "checks": [{"area": "documentationCommandsCapability", "evidence": []}]
                },
            }
        ),
        encoding="utf-8",
    )
    for relative in generated:
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(f"old:{relative}\n", encoding="utf-8")
    for relative in (
        "scripts/ai_capability_truth.py",
        "scripts/ai_japanese_capability.py",
        "scripts/check_pre_release_documentation_alignment.py",
    ):
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("# test generator\n", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    calls = []

    def fake_run(command, **_kwargs):
        calls.append(command)
        script = command[1]
        if script == "scripts/ai_capability_truth.py":
            (tmp_path / generated[0]).write_text("fresh:capability\n", encoding="utf-8")
        elif script == "scripts/ai_japanese_capability.py":
            (tmp_path / generated[4]).write_text("fresh:japanese-json\n", encoding="utf-8")
            (tmp_path / generated[5]).write_text("fresh:japanese-md\n", encoding="utf-8")
        elif script == "scripts/check_pre_release_documentation_alignment.py":
            (tmp_path / generated[2]).write_text("fresh:alignment-json\n", encoding="utf-8")
            (tmp_path / generated[3]).write_text("fresh:alignment-md\n", encoding="utf-8")
        else:
            raise AssertionError(command)
        return 0, 3, f"{script} refreshed"

    monkeypatch.setattr(ai_finish, "run", fake_run)

    code, _duration, _detail = ai_finish.refresh_source_bound_evidence(summary_path=summary_path)

    assert code == 0
    assert [command[1] for command in calls] == [
        "scripts/ai_capability_truth.py",
        "scripts/ai_japanese_capability.py",
        "scripts/check_pre_release_documentation_alignment.py",
    ]
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert {item["path"] for item in summary["changedFiles"]} == set(generated)
    assert set(summary["generatedFiles"]) == set(generated)
    assert set(summary["documentationAlignment"]["checks"][0]["evidence"]) == set(generated)


def test_source_bound_refresh_rebuilds_existing_knowledge_projections(tmp_path, monkeypatch):
    matrix_path = tmp_path / "docs" / "reference" / "capability-truth-matrix.json"
    generator_path = tmp_path / "scripts" / "ai_capability_truth.py"
    summary_path = tmp_path / "summary.json"
    matrix_path.parent.mkdir(parents=True)
    generator_path.parent.mkdir(parents=True)
    matrix_path.write_text("{}\n", encoding="utf-8")
    generator_path.write_text("# test generator\n", encoding="utf-8")
    summary_path.write_text(
        json.dumps({"changedFiles": [], "generatedFiles": [], "verification": []}),
        encoding="utf-8",
    )
    knowledge_file = tmp_path / ".ai" / "knowledge" / "work-items" / "old.json"
    knowledge_file.parent.mkdir(parents=True)
    knowledge_file.write_text("old\n", encoding="utf-8")
    index_file = knowledge_file.parent.parent / "index.json"
    index_file.write_text("old-index\n", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    captured = {}

    def fake_run(command, **_kwargs):
        if "--write" in command:
            matrix_path.write_text("fresh\n", encoding="utf-8")
            return 0, 3, "capability truth matrix refreshed"
        return 0, 3, "source-bound evidence passed"

    monkeypatch.setattr(ai_finish, "run", fake_run)
    monkeypatch.setattr(
        "ai_generate_knowledge_record.rebuild_existing_projections",
        lambda *, repo_root, changed_paths: (
            captured.update({"repo_root": repo_root, "changed_paths": tuple(changed_paths)})
            or [
                ".ai/knowledge/work-items/old.json",
                ".ai/knowledge/index.json",
            ]
        ),
    )

    code, _duration, detail = ai_finish.refresh_source_bound_evidence(summary_path=summary_path)

    assert code == 0
    assert "Implementation Knowledge projections refreshed" in detail
    assert captured == {
        "repo_root": tmp_path,
        "changed_paths": ("docs/reference/capability-truth-matrix.json",),
    }
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert {item["path"] for item in summary["changedFiles"]} == {
        "docs/reference/capability-truth-matrix.json",
        ".ai/knowledge/work-items/old.json",
        ".ai/knowledge/index.json",
    }


def test_finish_digest_excludes_derived_lifecycle_projections(monkeypatch):
    monkeypatch.setattr(ai_finish, "path_fingerprint", lambda path: f"digest:{path}")

    digest = ai_finish.worktree_digest_for_finish(
        [
            "scripts/example.py",
            ".ai/work-items/active/task.summary.json",
            ".ai/work-items/active/task.outcome.json",
            ".ai/work-items/active/task.outcome.md",
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
        ],
        ".ai/work-items/active/task.summary.json",
    )

    assert digest == ai_finish.worktree_digest(["scripts/example.py"])


def test_pre_merge_handoff_projects_evidence_bound_observed_issue_resolutions(
    tmp_path, monkeypatch
):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [],
                "observedIssues": [
                    {
                        "area": "projection-lag",
                        "detail": "The published projection was stale.",
                        "status": "resolved_by_provider_projection_sync",
                        "action": "Synchronized the provider projection and candidate metadata.",
                        "evidence": [
                            {"source": "release.json", "subject": "publishedVersion"},
                            {"source": "command://check-release-distribution", "subject": "pass"},
                        ],
                    },
                    {
                        "area": "remaining-gap",
                        "detail": "A separate limitation remains.",
                        "status": "open",
                        "evidence": [{"source": "summary", "subject": "remaining-gap"}],
                    },
                    {
                        "area": "malformed-resolution",
                        "detail": "Marked resolved without evidence.",
                        "status": "resolved_by_manual_note",
                        "action": "Manual note only.",
                        "evidence": [],
                    },
                ],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)

    payload = ai_finish._pre_merge_outcome_input(task, contract_path, summary_path, "en")
    questions = payload["evidence"]["handoffQuestions"]

    assert [item["claim"] for item in questions["resolvedProblems"]] == [
        "The published projection was stale."
    ]
    assert questions["resolvedProblems"][0]["evidenceRefs"] == [
        {"source": "release.json", "subject": "publishedVersion"},
        {"source": "command://check-release-distribution", "subject": "pass"},
    ]
    assert [item["claim"] for item in questions["resolutionApproach"]] == [
        "Synchronized the provider projection and candidate metadata."
    ]
    remaining = questions["remainingRisks"]
    assert any(item["claim"] == "A separate limitation remains." for item in remaining)
    assert any(
        "Marked resolved without evidence" in item["claim"] and item["inference"]
        for item in remaining
    )
    assert payload["evidence"]["resolutions"] == [
        {
            "problem": "The published projection was stale.",
            "action": "Synchronized the provider projection and candidate metadata.",
            "verification": "Evidence review",
            "result": "resolved",
            "evidenceRefs": [
                {"source": "release.json", "subject": "publishedVersion"},
                {"source": "command://check-release-distribution", "subject": "pass"},
            ],
            "evidence": [
                {"source": "release.json", "subject": "publishedVersion"},
                {"source": "command://check-release-distribution", "subject": "pass"},
            ],
        }
    ]


def test_pre_merge_handoff_consumes_summary_evidence_refs_for_resolutions(tmp_path, monkeypatch):
    task = "example-evidence-refs"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    refs = [
        {"source": "archive/wi-22.summary.json", "subject": "observedIssues[0].evidenceRefs"},
        {"source": "tests/test_task_outcome_ai_finish_integration.py", "subject": "pass"},
    ]
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [],
                "observedIssues": [
                    {
                        "area": "evidence-ref-adapter",
                        "detail": "Summary evidenceRefs must reach Outcome resolutions.",
                        "status": "resolved_by_adapter_fix",
                        "action": "Read evidenceRefs as the canonical source.",
                        "evidenceRefs": refs,
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)

    payload = ai_finish._pre_merge_outcome_input(task, contract_path, summary_path, "en")

    assert payload["evidence"]["resolutions"][0]["evidenceRefs"] == refs
    assert payload["evidence"]["handoffQuestions"]["resolvedProblems"][0]["evidenceRefs"] == refs
    assert payload["evidence"]["handoffQuestions"]["resolutionApproach"][0]["evidenceRefs"] == refs


def test_pre_merge_handoff_preserves_high_residual_risk_controls(tmp_path, monkeypatch):
    task = "residual-risk-controls"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [],
                "observedIssues": [],
                "residualRisks": [
                    {
                        "level": "high",
                        "area": "provider-publication",
                        "detail": "Provider evidence is pending.",
                        "decisionOwner": "release maintainer",
                        "requiredEvidence": ["same-SHA rehearsal receipt"],
                        "mitigation": "Do not publish until the receipt passes.",
                        "acceptanceStatus": "pending",
                    }
                ],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)

    payload = ai_finish._pre_merge_outcome_input(task, contract_path, summary_path, "en")
    risk = payload["evidence"]["handoffRisks"][0]

    assert risk["decisionOwner"] == "release maintainer"
    assert risk["requiredEvidence"] == ["same-SHA rehearsal receipt"]
    assert risk["mitigation"] == "Do not publish until the receipt passes."
    assert risk["acceptanceStatus"] == "pending"


def test_record_result_preserves_failed_attempt_when_retry_passes(tmp_path):
    summary_path = tmp_path / "summary.json"
    failed = {"check": "aiSummary", "result": "failed", "outputDigest": "a" * 64}
    passed = {"check": "aiSummary", "result": "passed", "outputDigest": "b" * 64}
    summary_path.write_text(json.dumps({"verification": [failed]}), encoding="utf-8")

    ai_finish.record_result(summary_path, passed)

    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert summary["verification"] == [passed]
    assert summary["verificationHistory"] == [failed]


def test_pre_merge_handoff_projects_retry_stop_and_resolution(tmp_path, monkeypatch):
    task = "retry-projection"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    failed = {
        "check": "aiSummary",
        "result": "failed",
        "executedAt": "2026-08-16T20:00:00Z",
        "outputDigest": "a" * 64,
    }
    passed = {
        "check": "aiSummary",
        "result": "passed",
        "executedAt": "2026-08-16T20:05:00Z",
        "outputDigest": "b" * 64,
    }
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [passed],
                "verificationHistory": [failed],
                "observedIssues": [],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)

    payload = ai_finish._pre_merge_outcome_input(task, contract_path, summary_path, "en")
    questions = payload["evidence"]["handoffQuestions"]

    assert questions["blockedProblems"] == []
    assert questions["problemCount"] == 1
    assert questions["resolvedProblems"][0]["inference"] is False
    assert questions["resolvedProblems"][0]["evidenceRefs"]
    assert [event["eventType"] for event in payload["evidence"]["events"]] == [
        "stop",
        "resolution",
    ]
    assert all(event["evidence"] for event in payload["evidence"]["events"])


def test_pre_merge_handoff_binds_current_failed_check_to_stop_evidence(tmp_path, monkeypatch):
    task = "current-failure"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    failed = {
        "check": "quality",
        "result": "failed",
        "executedAt": "2026-08-16T20:00:00Z",
        "outputDigest": "a" * 64,
    }
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [failed],
                "observedIssues": [],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)

    payload = ai_finish._pre_merge_outcome_input(task, contract_path, summary_path, "en")
    questions = payload["evidence"]["handoffQuestions"]

    assert questions["blockedProblems"][0]["claim"] == "quality failed on the latest attempt."
    assert questions["blockedProblems"][0]["inference"] is False
    assert payload["evidence"]["events"][0]["state"] == "unresolved"
    assert payload["evidence"]["events"][0]["evidence"]


def test_pre_merge_outcome_renders_retry_stop_as_resolved(tmp_path, monkeypatch):
    task = "retry-outcome"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    outcome_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    contract_path.write_text(json.dumps({"baseCommit": "a" * 40}), encoding="utf-8")
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "knownGaps": [],
                "verification": [
                    {"check": "quality", "result": "passed", "outputDigest": "b" * 64}
                ],
                "verificationHistory": [
                    {"check": "quality", "result": "failed", "outputDigest": "a" * 64}
                ],
                "observedIssues": [],
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _task: (outcome_path, markdown_path))

    ok, message = ai_finish._write_and_validate_pre_merge_outcome(
        task, contract_path, summary_path, outcome_path, markdown_path, "en"
    )

    assert ok, message
    outcome = json.loads(outcome_path.read_text(encoding="utf-8"))
    assert outcome["humanHandoff"]["questions"]["blockedProblems"] == []
    assert outcome["sections"]["forcedStops"][0]["result"] == "resolved"
    assert outcome["sections"]["resolutions"][0]["evidence"]


def _outcome(task: str) -> dict:
    return generate_outcome(
        task,
        {
            "taskId": task,
            "contractDigest": "a" * 64,
            "summaryDigest": "b" * 64,
            "verificationDigest": "c" * 64,
            "baseCommit": "1" * 40,
            "headCommit": "2" * 40,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "1.0",
            "generatorVersion": "1.2",
        },
        evidence={
            "outcomeSummary": "Completed from structured evidence.",
            "sources": [{"source": "fixture", "subject": "evidence"}],
        },
    )


def test_outcome_pipeline_orders_generation_validation_render_validation_and_records_link(
    tmp_path, monkeypatch
):
    task = "example-task"
    summary_path = tmp_path / "summary.json"
    raw_path = tmp_path / "raw.json"
    raw_path.write_text(json.dumps({"taskId": task}), encoding="utf-8")
    summary_path.write_text(json.dumps({"taskOutcomeInput": "raw.json"}), encoding="utf-8")
    json_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (json_path, markdown_path))
    calls = []

    def fake_run(command, **_kwargs):
        calls.append(command[1] if len(command) > 1 else command[0])
        if "generate_task_outcome" in " ".join(command):
            json_path.write_text(json.dumps(_outcome(task)), encoding="utf-8")
            markdown_path.write_text("# Task Outcome\n", encoding="utf-8")
        return 0, 1, "valid"

    monkeypatch.setattr(ai_finish, "run", fake_run)
    ok, message = ai_finish.run_task_outcome_pipeline(task, summary_path)

    assert ok
    assert message == "Outcome pipeline passed"
    assert calls[0].endswith("ai_generate_task_outcome.py")
    assert calls[1] == "-c"
    assert calls[2].endswith("ai_render_task_outcome.py")
    assert calls[3] == "-c"
    state = json.loads(summary_path.read_text(encoding="utf-8"))["taskOutcome"]
    assert state["markdownPath"] == "outcome.md"
    assert state["evidenceCount"] == 1
    assert state["humanStatusColor"] == "green"
    assert state["completionFact"] == "All declared finish checks passed."


def test_outcome_pipeline_failure_preserves_raw_evidence_and_records_structured_failure(
    tmp_path, monkeypatch
):
    task = "example-task"
    summary_path = tmp_path / "summary.json"
    raw_path = tmp_path / "raw.json"
    raw = '{"taskId":"example-task","events":[{"eventType":"finding"}]}'
    raw_path.write_text(raw, encoding="utf-8")
    summary_path.write_text(json.dumps({"taskOutcomeInput": "raw.json"}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish,
        "_outcome_paths",
        lambda _: (tmp_path / "outcome.json", tmp_path / "outcome.md"),
    )
    monkeypatch.setattr(ai_finish, "run", lambda *_args, **_kwargs: (1, 4, "schema: invalid"))

    ok, message = ai_finish.run_task_outcome_pipeline(task, summary_path)

    assert not ok
    assert "schema: invalid" in message
    assert raw_path.read_text(encoding="utf-8") == raw
    state = json.loads(summary_path.read_text(encoding="utf-8"))["taskOutcome"]
    assert state["status"] == "failed"
    assert state["rawEvidencePath"] == "raw.json"
    assert "error" in state


def test_outcome_pipeline_without_contract_fails_closed(tmp_path):
    summary_path = tmp_path / "summary.json"
    summary_path.write_text("{}", encoding="utf-8")
    assert ai_finish.run_task_outcome_pipeline("example-task", summary_path) == (
        False,
        "mandatory Task Outcome requires the active Contract",
    )


def test_pre_archive_critical_coverage_records_success_and_failure(monkeypatch):
    class Observer:
        def __init__(self):
            self.events = []

        def check_started(self, **kwargs):
            self.events.append(("started", kwargs))

        def check_passed(self, **kwargs):
            self.events.append(("passed", kwargs))

        def check_failed(self, **kwargs):
            self.events.append(("failed", kwargs))

    contract = {"workItemId": "example-task", "baseCommit": "a" * 40}
    observer = Observer()
    monkeypatch.setattr(ai_finish, "run", lambda _command: (0, 17, "coverage passed"))

    assert ai_finish.run_pre_archive_critical_coverage(contract, obs=observer) == (
        0,
        "coverage passed",
    )
    assert observer.events[0][0] == "started"
    assert observer.events[0][1]["command"] == (
        "make check-changed-critical-coverage AI_BASE_COMMIT="
        + "a" * 40
        + " CONTRACT=.ai/work-items/active/example-task.contract.json"
    )
    assert observer.events[1][0] == "passed"

    monkeypatch.setattr(ai_finish, "run", lambda _command: (1, 19, "coverage failed"))
    assert ai_finish.run_pre_archive_critical_coverage(contract, obs=observer) == (
        1,
        "coverage failed",
    )
    assert observer.events[-1][0] == "failed"


def test_pre_archive_critical_coverage_requires_contract_base():
    assert ai_finish.pre_archive_critical_coverage_command({"workItemId": "example-task"}) == (
        None,
        "Contract baseCommit is required for pre-archive critical coverage",
    )


def test_pre_archive_critical_coverage_requires_work_item_and_preserves_plain_failure_text():
    class Observer:
        def check_started(self, **_kwargs):
            raise AssertionError("missing Contract identity must not invoke the gate")

    assert ai_finish.pre_archive_critical_coverage_command({"baseCommit": "a" * 40}) == (
        None,
        "pre-archive changed-critical coverage requires a Work Item id",
    )
    assert ai_finish.run_pre_archive_critical_coverage(
        {"baseCommit": "a" * 40}, obs=Observer()
    ) == (2, "pre-archive changed-critical coverage requires a Work Item id")
    assert ai_finish.outcome_failure_message("quality", "lint command failed") == (
        "Finish blocked at quality: lint command failed"
    )
    assert ai_finish.verification_priority({"check": "aiStatusCheck"}) == 30


def test_prepare_pre_archive_candidate_coverage_fails_closed_for_gate_or_binding_failure(
    monkeypatch,
):
    contract = {"workItemId": "example-task", "baseCommit": "a" * 40}
    observer = object()
    monkeypatch.setattr(
        ai_finish,
        "run_pre_archive_critical_coverage",
        lambda *_args, **_kwargs: (7, "coverage failed"),
    )

    assert ai_finish.prepare_pre_archive_candidate_coverage(
        "example-task", contract, obs=observer
    ) == (7, "coverage failed")

    monkeypatch.setattr(
        ai_finish, "run_pre_archive_critical_coverage", lambda *_args, **_kwargs: (0, "")
    )
    monkeypatch.setattr(
        ai_finish,
        "bind_pre_archive_candidate_coverage_to_outcome",
        lambda _task: (False, "binding failed"),
    )

    assert ai_finish.prepare_pre_archive_candidate_coverage(
        "example-task", contract, obs=observer
    ) == (1, "binding failed")


def test_pre_archive_candidate_coverage_is_projected_into_outcome(tmp_path, monkeypatch):
    task = "example-task"
    report_path = tmp_path / "target/changed-critical-coverage.json"
    report_path.parent.mkdir(parents=True)
    binding = {
        "baseCommit": "a" * 40,
        "candidateHead": "b" * 40,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    report_path.write_text(json.dumps({"binding": binding}), encoding="utf-8")
    outcome_path = tmp_path / "task.outcome.json"
    markdown_path = tmp_path / "task.outcome.md"
    outcome = _outcome(task)
    outcome["bindings"] = {}
    outcome_path.write_text(json.dumps(outcome), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _task: (outcome_path, markdown_path))
    import ai_check_task_outcome

    monkeypatch.setattr(
        ai_check_task_outcome,
        "validate_outcome",
        lambda *_args, **_kwargs: type("Report", (), {"valid": True, "errors": []})(),
    )

    assert ai_finish.bind_pre_archive_candidate_coverage_to_outcome(task) == (
        True,
        "Task Outcome binds pre-archive candidate coverage",
    )
    actual = json.loads(outcome_path.read_text(encoding="utf-8"))["bindings"]
    assert actual["preArchiveCandidateCoverage"]["binding"] == binding


def test_failed_check_selection_uses_latest_failure_and_fails_closed_on_bad_summary(tmp_path):
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "verification": [
                    {"check": "quality", "result": "failed"},
                    {"check": "aiSummary", "result": "passed"},
                ]
            }
        ),
        encoding="utf-8",
    )
    assert ai_finish.failed_check_from_summary(summary_path, "verification") == "quality"
    summary_path.write_text(json.dumps({"verification": {}}), encoding="utf-8")
    assert ai_finish.failed_check_from_summary(summary_path, "verification") == "verification"
    summary_path.write_text(
        json.dumps({"verification": [{"check": "", "result": "failed"}]}), encoding="utf-8"
    )
    assert ai_finish.failed_check_from_summary(summary_path, "verification") == "verification"
    assert (
        ai_finish.failed_check_from_summary(tmp_path / "missing.json", "verification")
        == "verification"
    )


def test_blocked_finish_failure_preserves_gate_exit_status(monkeypatch, tmp_path):
    monkeypatch.setattr(
        ai_finish, "write_blocked_outcome", lambda *_args, **_kwargs: (True, "persisted")
    )

    assert (
        ai_finish.return_blocked_finish_failure(
            task="example-task",
            contract_path=tmp_path / "contract.json",
            summary_path=tmp_path / "summary.json",
            failed_check="preArchiveCriticalCoverage",
            failure_message="gate failed",
            code=2,
        )
        == 2
    )
    monkeypatch.setattr(
        ai_finish,
        "write_blocked_outcome",
        lambda *_args, **_kwargs: (False, "report refresh failed"),
    )
    assert (
        ai_finish.return_blocked_finish_failure(
            task="example-task",
            contract_path=tmp_path / "contract.json",
            summary_path=tmp_path / "summary.json",
            failed_check="preArchiveCriticalCoverage",
            failure_message="gate failed",
            code=1,
        )
        == 1
    )


def test_blocked_finish_failure_delivers_persisted_outcome_to_conversation(
    monkeypatch, tmp_path, capsys
):
    task = "example-task"
    outcome_path = tmp_path / "outcome.json"
    outcome_path.write_text(json.dumps(_outcome(task)), encoding="utf-8")
    monkeypatch.setattr(
        ai_finish,
        "_outcome_paths",
        lambda _task: (outcome_path, tmp_path / "outcome.md"),
    )
    monkeypatch.setattr(
        ai_finish, "write_blocked_outcome", lambda *_args, **_kwargs: (True, "persisted")
    )

    exit_code = ai_finish.return_blocked_finish_failure(
        task=task,
        contract_path=tmp_path / "contract.json",
        summary_path=tmp_path / "summary.json",
        failed_check="quality",
        failure_message="gate failed",
        code=2,
        language="zh-CN",
    )

    captured = capsys.readouterr()
    assert exit_code == 2
    assert "工单结果报告" in captured.out
    assert "Outcome: 🟢 completed" in captured.out
    assert "归档必须显式执行" in captured.out
    assert "CLI output cannot authenticate human receipt or approval" in captured.out
    assert "Blocked Task Outcome persisted" in captured.err


def test_blocked_finish_default_uses_bound_conversation_locale(monkeypatch, tmp_path):
    captured: dict[str, object] = {}
    monkeypatch.setattr(ai_finish, "CURRENT_REPORT_LANGUAGE", "zh-CN")

    def persist(*_args, **kwargs):
        captured["language"] = kwargs.get("language")
        return True, "persisted"

    monkeypatch.setattr(ai_finish, "write_blocked_outcome", persist)
    monkeypatch.setattr(
        ai_finish, "deliver_direct_outcome_report", lambda *_args: (True, "delivered")
    )

    assert (
        ai_finish.return_blocked_finish_failure(
            task="example-task",
            contract_path=tmp_path / "contract.json",
            summary_path=tmp_path / "summary.json",
            failed_check="quality",
            failure_message="gate failed",
            code=2,
        )
        == 2
    )
    assert captured["language"] == "zh-CN"


def test_documentation_alignment_failure_is_reported_without_raising(tmp_path):
    summary_path = tmp_path / "summary.json"
    summary_path.write_text("not-json", encoding="utf-8")

    errors = ai_finish.documentation_alignment_issues(summary_path, {})

    assert len(errors) == 1
    assert errors[0].startswith("documentationAlignment could not be validated:")


def test_blocked_outcome_refreshes_the_exact_active_review_report(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    summary_path.write_text(json.dumps({"changedFiles": [], "verification": []}), encoding="utf-8")
    outcome_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    report_json = tmp_path / "task_report.json"
    report_markdown = tmp_path / "task_report.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _task: (outcome_path, markdown_path))
    monkeypatch.setattr(ai_finish, "_human_report_paths", lambda: (report_json, report_markdown))
    refreshed = []
    monkeypatch.setattr(
        ai_finish,
        "refresh_active_status_after_blocked_outcome",
        lambda actual_contract, actual_summary: (
            refreshed.append((actual_contract, actual_summary)) or (True, "active status refreshed")
        ),
    )

    ok, message = ai_finish.write_blocked_outcome(
        task,
        contract_path,
        summary_path,
        failed_check="quality",
        failure_message="quality gate failed",
    )

    assert ok, message
    outcome = json.loads(outcome_path.read_text(encoding="utf-8"))
    assert outcome["status"] == "blocked"
    assert outcome["humanStatusColor"] == "red"
    assert outcome["failedGate"] == "quality"
    assert outcome["recoveryCondition"] == "Run a passing quality retry."
    assert "Human Status: `red`" in markdown_path.read_text(encoding="utf-8")
    assert "Failed Gate: `quality`" in markdown_path.read_text(encoding="utf-8")
    assert validate_outcome(
        outcome, markdown_path.read_text(encoding="utf-8"), expected_task_id=task
    ).valid
    report = json.loads(report_json.read_text(encoding="utf-8"))
    assert human.validate_human_report(report, outcome) == []
    assert report_markdown.read_text(encoding="utf-8") == human.render_human_report(report)
    assert any("quality gate failed" in warning for warning in outcome["sections"]["warnings"])
    assert refreshed == [(contract_path, summary_path)]


def test_blocked_outcome_fails_closed_and_removes_stale_status_when_refresh_fails(
    tmp_path, monkeypatch
):
    stale_status = tmp_path / ".ai" / "cockpit" / "current_status.md"
    stale_status.parent.mkdir(parents=True)
    stale_status.write_text("- Traffic Light: `green`\n", encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    calls = []

    def failed_run(command, **_kwargs):
        calls.append(command)
        return 2, 1, "status generator failed"

    monkeypatch.setattr(ai_finish, "run", failed_run)

    ok, message = ai_finish.refresh_active_status_after_blocked_outcome(
        tmp_path / "contract.json", tmp_path / "summary.json"
    )

    assert not ok
    assert "status generator failed" in message
    assert calls[0][:2] == ["make", "generate-cockpit-status"]
    assert not stale_status.exists()


def test_blocked_outcome_normalizes_coverage_metrics_for_valid_report(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    summary_path.write_text(json.dumps({"changedFiles": [], "verification": []}), encoding="utf-8")
    outcome_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    report_json = tmp_path / "task_report.json"
    report_markdown = tmp_path / "task_report.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _task: (outcome_path, markdown_path))
    monkeypatch.setattr(ai_finish, "_human_report_paths", lambda: (report_json, report_markdown))
    monkeypatch.setattr(
        ai_finish, "refresh_active_status_after_blocked_outcome", lambda *_args: (True, "ok")
    )

    ok, message = ai_finish.write_blocked_outcome(
        task,
        contract_path,
        summary_path,
        failed_check="preArchiveCriticalCoverage",
        failure_message="scripts/ai_finish.py: 83.50% is below 85%",
    )

    assert ok, message
    outcome = json.loads(outcome_path.read_text(encoding="utf-8"))
    assert validate_outcome(
        outcome, markdown_path.read_text(encoding="utf-8"), expected_task_id=task
    ).valid
    assert "preArchiveCriticalCoverage" in outcome["sections"]["warnings"][0]
    assert "%" not in outcome["sections"]["warnings"][0]


def test_blocked_outcome_survives_report_refresh_failure(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    summary_path.write_text(json.dumps({"changedFiles": [], "verification": []}), encoding="utf-8")
    outcome_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "b" * 40)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _task: (outcome_path, markdown_path))
    monkeypatch.setattr(
        ai_finish, "run_human_report_pipeline", lambda *_args: (False, "report writer unavailable")
    )
    monkeypatch.setattr(
        ai_finish, "refresh_active_status_after_blocked_outcome", lambda *_args: (True, "ok")
    )

    ok, message = ai_finish.write_blocked_outcome(
        task,
        contract_path,
        summary_path,
        failed_check="aiDiffOwnership",
        failure_message="stale report blocks retry",
    )

    assert not ok
    assert "report writer unavailable" in message
    outcome = json.loads(outcome_path.read_text(encoding="utf-8"))
    assert outcome["status"] == "blocked"
    assert validate_outcome(
        outcome, markdown_path.read_text(encoding="utf-8"), expected_task_id=task
    ).valid


def test_human_report_pipeline_generates_review_artifacts_and_summary_binding(
    tmp_path, monkeypatch
):
    task = "example-task"
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [],
                "documentationAlignment": {
                    "checks": [{"area": "documentationCommandsCapability", "evidence": []}]
                },
            }
        ),
        encoding="utf-8",
    )
    outcome_path = tmp_path / "outcome.json"
    outcome_value = _outcome(task)
    outcome_value.update(
        {
            "format": "ai-cockpit-task-outcome",
            "schemaVersion": 1,
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
        }
    )
    outcome_path.write_text(json.dumps(outcome_value), encoding="utf-8")
    json_path = tmp_path / "task_report.json"
    markdown_path = tmp_path / "task_report.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    contract_path = tmp_path / ".ai/work-items/active" / f"{task}.contract.json"
    contract_path.parent.mkdir(parents=True)
    contract_path.write_text(
        json.dumps({"scope": ["task_report.json", "task_report.md"]}), encoding="utf-8"
    )
    monkeypatch.setattr(
        ai_finish, "_outcome_paths", lambda _: (outcome_path, tmp_path / "outcome.md")
    )
    monkeypatch.setattr(ai_finish, "_human_report_paths", lambda: (json_path, markdown_path))

    ok, message = ai_finish.run_human_report_pipeline(task, summary_path)

    assert ok
    assert message == "Human Benefit Report pipeline passed"
    report = json.loads(json_path.read_text(encoding="utf-8"))
    assert human.validate_human_report(report, outcome_value) == []
    assert markdown_path.read_text(encoding="utf-8") == human.render_human_report(report)
    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert {item["path"] for item in summary["changedFiles"]} == {
        "task_report.json",
        "task_report.md",
    }
    assert summary["documentationAlignment"]["checks"][0]["evidence"] == ["task_report.md"]


def test_prepare_documentation_alignment_binds_existing_human_report_before_finish(
    tmp_path, monkeypatch
):
    task = "example-task"
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [
                    {"path": ".ai/cockpit/task_report.md", "reason": "prior report"},
                    {
                        "path": ".ai/work-items/active/example-task.outcome.md",
                        "reason": "future outcome",
                    },
                ],
                "documentationAlignment": {
                    "checks": [{"area": "documentationCommandsCapability", "evidence": []}]
                },
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    contract_path = tmp_path / ".ai/work-items/active" / f"{task}.contract.json"
    contract_path.parent.mkdir(parents=True)
    contract_path.write_text(
        json.dumps({"scope": [".ai/cockpit/task_report.md"]}), encoding="utf-8"
    )
    monkeypatch.setattr(
        ai_finish,
        "_human_report_paths",
        lambda: (
            tmp_path / ".ai/cockpit/task_report.json",
            tmp_path / ".ai/cockpit/task_report.md",
        ),
    )
    monkeypatch.setattr(
        ai_finish,
        "_outcome_paths",
        lambda _: (
            tmp_path / ".ai/work-items/active/outcome.json",
            tmp_path / ".ai/work-items/active/outcome.md",
        ),
    )
    (tmp_path / ".ai/cockpit/task_report.md").parent.mkdir(parents=True, exist_ok=True)
    (tmp_path / ".ai/cockpit/task_report.md").write_text("# prior report\n", encoding="utf-8")

    ai_finish.prepare_documentation_alignment_evidence(task, summary_path)

    summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert summary["documentationAlignment"]["checks"][0]["evidence"] == [
        ".ai/cockpit/task_report.md"
    ]


def test_prepare_documentation_alignment_binds_source_bound_generated_reports(
    tmp_path, monkeypatch
):
    task = "source-bound-alignment"
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [
                    {
                        "path": "docs/reference/pre-release-documentation-alignment.json",
                        "reason": "generated",
                    },
                    {
                        "path": "docs/reference/pre-release-documentation-alignment.md",
                        "reason": "generated",
                    },
                ],
                "documentationAlignment": {
                    "checks": [{"area": "documentationCommandsCapability", "evidence": []}]
                },
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    contract_path = tmp_path / ".ai/work-items/active" / f"{task}.contract.json"
    contract_path.parent.mkdir(parents=True)
    contract_path.write_text(
        json.dumps(
            {
                "scope": [
                    "docs/reference/pre-release-documentation-alignment.json",
                    "docs/reference/pre-release-documentation-alignment.md",
                ]
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(
        ai_finish,
        "_outcome_paths",
        lambda _: (
            tmp_path / ".ai/work-items/active/source-bound-alignment.outcome.json",
            tmp_path / ".ai/work-items/active/source-bound-alignment.outcome.md",
        ),
    )
    monkeypatch.setattr(
        ai_finish,
        "_human_report_paths",
        lambda: (
            tmp_path / ".ai/cockpit/task_report.json",
            tmp_path / ".ai/cockpit/task_report.md",
        ),
    )
    for relative in ai_finish.SOURCE_BOUND_ALIGNMENT_RELATIVES:
        path = tmp_path / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("generated\n", encoding="utf-8")

    ai_finish.prepare_documentation_alignment_evidence(task, summary_path)

    evidence = json.loads(summary_path.read_text(encoding="utf-8"))["documentationAlignment"][
        "checks"
    ][0]["evidence"]
    assert evidence == list(ai_finish.SOURCE_BOUND_ALIGNMENT_RELATIVES)


def test_human_report_pipeline_binds_generated_outcome_markdown_before_finish_recheck(
    tmp_path, monkeypatch
):
    task = "example-task"
    outcome_path = tmp_path / "outcome.json"
    outcome_markdown = tmp_path / "outcome.md"
    summary_path = tmp_path / "summary.json"
    outcome_value = _outcome(task)
    outcome_value.update(
        {
            "format": "ai-cockpit-task-outcome",
            "schemaVersion": 1,
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
        }
    )
    outcome_path.write_text(json.dumps(outcome_value), encoding="utf-8")
    outcome_markdown.write_text("# Task Outcome\n", encoding="utf-8")
    summary_path.write_text(
        json.dumps(
            {
                "changedFiles": [{"path": "outcome.md", "reason": "generated Outcome"}],
                "documentationAlignment": {
                    "checks": [{"area": "documentationCommandsCapability", "evidence": []}]
                },
            }
        ),
        encoding="utf-8",
    )
    report_json = tmp_path / "task_report.json"
    report_markdown = tmp_path / "task_report.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    active_contract = tmp_path / ".ai/work-items/active" / f"{task}.contract.json"
    active_contract.parent.mkdir(parents=True)
    active_contract.write_text(
        json.dumps({"scope": ["outcome.md", "task_report.json", "task_report.md"]}),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (outcome_path, outcome_markdown))
    monkeypatch.setattr(ai_finish, "_human_report_paths", lambda: (report_json, report_markdown))

    ok, message = ai_finish.run_human_report_pipeline(task, summary_path)

    assert ok, message
    evidence = json.loads(summary_path.read_text(encoding="utf-8"))["documentationAlignment"][
        "checks"
    ][0]["evidence"]
    assert evidence == ["outcome.md", "task_report.md"]


def test_archived_human_report_refreshes_after_outcome_path_rewrite(tmp_path, monkeypatch):
    task = "example-task"
    archive = tmp_path / ".ai/work-items/archive/2026"
    archive.mkdir(parents=True)
    outcome_value = _outcome(task)
    outcome_value.update(
        {
            "format": "ai-cockpit-task-outcome",
            "schemaVersion": 1,
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
        }
    )
    outcome_path = archive / f"{task}.outcome.json"
    outcome_path.write_text(json.dumps(outcome_value), encoding="utf-8")
    json_path = tmp_path / ".ai/cockpit/task_report.json"
    markdown_path = tmp_path / ".ai/cockpit/task_report.md"
    json_path.parent.mkdir(parents=True)
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_human_report_paths", lambda: (json_path, markdown_path))

    ok, message = ai_finish.refresh_archived_human_report(task)

    assert ok
    assert message == "Archived Human Benefit Report binding passed"
    report = json.loads(json_path.read_text(encoding="utf-8"))
    assert human.validate_human_report(report, outcome_value) == []


def test_unscoped_current_report_remains_generated_evidence_not_summary_ownership(
    tmp_path, monkeypatch
):
    task = "example-task"
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(json.dumps({"changedFiles": []}), encoding="utf-8")
    outcome_path = tmp_path / "outcome.json"
    outcome_value = _outcome(task)
    outcome_value.update(
        {
            "format": "ai-cockpit-task-outcome",
            "schemaVersion": 1,
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
        }
    )
    outcome_path.write_text(json.dumps(outcome_value), encoding="utf-8")
    contract_path = tmp_path / ".ai/work-items/active" / f"{task}.contract.json"
    contract_path.parent.mkdir(parents=True)
    contract_path.write_text(json.dumps({"scope": ["fixture.txt"]}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_finish, "_outcome_paths", lambda _: (outcome_path, tmp_path / "outcome.md")
    )

    ok, _ = ai_finish.run_human_report_pipeline(task, summary_path)

    assert ok
    assert json.loads(summary_path.read_text(encoding="utf-8"))["changedFiles"] == []


def test_outcome_pipeline_without_opt_in_derives_a_pre_merge_report(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps(
            {
                "workItemId": task,
                "baseCommit": "a" * 40,
                "verification": [],
            }
        ),
        encoding="utf-8",
    )
    summary_path.write_text(
        json.dumps(
            {
                "verification": [],
                "changedFiles": [{"path": "fixture.txt", "reason": "fixture"}],
                "observedIssues": [],
            }
        ),
        encoding="utf-8",
    )
    json_path = tmp_path / "outcome.json"
    markdown_path = tmp_path / "outcome.md"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (json_path, markdown_path))

    ok, _ = ai_finish.run_task_outcome_pipeline(task, summary_path, contract_path)

    assert ok
    outcome = json.loads(json_path.read_text(encoding="utf-8"))
    assert outcome["bindings"]["lifecycleStage"] == "pre_merge"
    assert outcome["bindings"]["pullRequest"] == {"state": "not_created"}
    assert markdown_path.exists()
    recorded_summary = json.loads(summary_path.read_text(encoding="utf-8"))
    assert recorded_summary["taskOutcome"]["markdownPath"] == "outcome.md"
    assert {item["path"] for item in recorded_summary["changedFiles"]} == {
        "fixture.txt",
        "outcome.json",
        "outcome.md",
    }
    assert (
        outcome["bindings"]["summaryDigest"]
        == hashlib.sha256(summary_path.read_bytes()).hexdigest()
    )


def test_outcome_pipeline_structures_legacy_known_gaps_as_limitations(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    warning = "Hosted provider checks were not_run."
    summary_path.write_text(
        json.dumps({"verification": [], "changedFiles": [], "knownGaps": [warning]}),
        encoding="utf-8",
    )
    json_path = tmp_path / "outcome.json"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (json_path, tmp_path / "outcome.md"))

    ok, message = ai_finish.run_task_outcome_pipeline(task, summary_path, contract_path)

    assert ok, message
    sections = json.loads(json_path.read_text(encoding="utf-8"))["sections"]
    assert sections["limitations"][0]["sourceWarning"] == warning
    assert sections["nonRiskExplanations"][0]["sourceWarning"] == warning
    assert sections["forbiddenClaims"]


def test_outcome_pipeline_preserves_non_risk_explanation_without_warning_or_yellow_status(
    tmp_path, monkeypatch
):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    explanation = {
        "sourceWarning": "Hosted verification is not required by the Contract.",
        "reason": "The Contract does not require hosted verification for this Work Item.",
        "evidence": [{"source": "contract", "subject": "verification"}],
    }
    summary_path.write_text(
        json.dumps(
            {
                "verification": [],
                "changedFiles": [],
                "knownGaps": [],
                "nonRiskExplanations": [explanation],
            }
        ),
        encoding="utf-8",
    )
    json_path = tmp_path / "outcome.json"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (json_path, tmp_path / "outcome.md"))

    ok, message = ai_finish.run_task_outcome_pipeline(task, summary_path, contract_path)

    assert ok, message
    outcome = json.loads(json_path.read_text(encoding="utf-8"))
    assert outcome["status"] == "completed"
    assert outcome["sections"]["warnings"] == []
    assert outcome["sections"]["limitations"] == []
    assert outcome["sections"]["nonRiskExplanations"] == [explanation]


def test_outcome_pipeline_classifies_evidenced_known_gap_as_non_risk(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / "contract.json"
    summary_path = tmp_path / "summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": task, "baseCommit": "a" * 40, "verification": []}),
        encoding="utf-8",
    )
    explanation = {
        "sourceWarning": "Provider-hosted timing is outside this Work Item's acceptance boundary.",
        "reason": "The Contract requires portable source/template parity, not provider-hosted timing.",
        "evidence": [{"source": "contract", "subject": "acceptance"}],
    }
    summary_path.write_text(
        json.dumps(
            {
                "verification": [],
                "changedFiles": [],
                "knownGaps": [explanation["sourceWarning"]],
                "nonRiskExplanations": [explanation],
            }
        ),
        encoding="utf-8",
    )
    json_path = tmp_path / "outcome.json"
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "_outcome_paths", lambda _: (json_path, tmp_path / "outcome.md"))

    ok, message = ai_finish.run_task_outcome_pipeline(task, summary_path, contract_path)

    assert ok, message
    outcome = json.loads(json_path.read_text(encoding="utf-8"))
    assert outcome["status"] == "completed"
    assert outcome["sections"]["warnings"] == []
    assert outcome["sections"]["limitations"] == []
    assert outcome["sections"]["nonRiskExplanations"] == [explanation]


def test_outcome_pipeline_missing_input_fails_closed(tmp_path, monkeypatch):
    summary_path = tmp_path / "summary.json"
    summary_path.write_text(json.dumps({"taskOutcomeInput": "missing-raw.json"}), encoding="utf-8")
    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    ok, message = ai_finish.run_task_outcome_pipeline("example-task", summary_path)
    assert not ok
    assert "does not exist" in message


def test_finish_execution_priority_runs_summary_after_mandatory_outcome_and_quality():
    assert ai_finish.finish_execution_priority(
        {"check": "aiSummary"}
    ) > ai_finish.finish_execution_priority({"check": "aiStatus"})
    assert ai_finish.finish_execution_priority(
        {"check": "aiSummary"}
    ) > ai_finish.finish_execution_priority({"check": "quality"})


def test_finish_defaults_to_active_outcome_and_accepts_conversation_language(monkeypatch):
    monkeypatch.setattr(sys, "argv", ["ai_finish.py", "--task", "example-task"])

    args = ai_finish.parse_args()

    assert args.archive is False
    assert args.language is None


def test_direct_outcome_report_is_localized_and_explicit_about_archive_boundary():
    outcome = generate_outcome(
        "example-task",
        {
            "taskId": "example-task",
            "contractDigest": "a" * 64,
            "summaryDigest": "b" * 64,
            "verificationDigest": "c" * 64,
            "baseCommit": "1" * 40,
            "headCommit": "2" * 40,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "1.0",
            "generatorVersion": "1.2",
        },
    )

    report = ai_finish.render_direct_outcome_report(outcome, "zh-CN")

    assert "工单结果报告" in report
    assert "任务结果: example-task" in report
    assert "归档必须显式执行" in report


def test_direct_outcome_report_fails_closed_when_full_report_is_missing():
    outcome = _outcome("example-task")
    outcome.pop("format")

    with pytest.raises(ValueError, match="Task Outcome is invalid"):
        ai_finish.render_direct_outcome_report(outcome, "zh-CN")


def test_direct_outcome_report_contains_the_complete_conversation_surface():
    outcome = generate_outcome(
        "example-task",
        {
            "taskId": "example-task",
            "contractDigest": "a" * 64,
            "summaryDigest": "b" * 64,
            "verificationDigest": "c" * 64,
            "baseCommit": "1" * 40,
            "headCommit": "2" * 40,
            "lifecycleStage": "pre_merge",
            "pullRequest": {"state": "not_created"},
            "aiCockpitVersion": "1.0",
            "generatorVersion": "1.2",
        },
        evidence={
            "locale": "zh-CN",
            "completed": [
                {
                    "title": "Direct delivery",
                    "detail": "The complete Outcome is rendered into the conversation stream.",
                    "evidence": [{"source": "pytest", "subject": "direct-report"}],
                }
            ],
            "passedChecks": [
                {
                    "title": "Conversation delivery",
                    "detail": "The full report surface is present.",
                    "evidence": [{"source": "pytest", "subject": "direct-report"}],
                }
            ],
            "handoffQuestions": {
                "problemCount": 0,
                "blockedProblems": [],
                "resolvedProblems": [],
                "resolutionApproach": [],
                "avoidedRisks": [],
                "remainingRisks": [],
                "agentUnknowns": [],
                "humanConfirmations": [],
                "recurrenceLikelihood": "low",
                "nextTime": "Keep the complete report in the conversation stream.",
            },
        },
    )

    report = ai_finish.render_direct_outcome_report(outcome, "zh-CN")

    for marker in (
        "Outcome: 🟢 completed",
        "状态: 🟢 `completed`",
        "## 结果摘要",
        "## 已完成",
        "# AI Cockpit Task Report",
        "What was completed",
        "Problems found",
        "Problems resolved",
        "Risks avoided",
        "Remaining risks",
        "Human decisions",
        "Verification",
        "Next action",
        "归档必须显式执行",
    ):
        assert marker in report


@pytest.mark.parametrize("archive", [False, True])
def test_finish_prepares_candidate_coverage_for_separate_or_inline_archive(
    tmp_path, monkeypatch, archive
):
    task = "example-task"
    active = tmp_path / ".ai/work-items/active"
    active.mkdir(parents=True)
    contract_path = active / f"{task}.contract.json"
    summary_path = active / f"{task}.summary.json"
    contract = {
        "contractVersion": 2,
        "workItemId": task,
        "baseCommit": "d" * 40,
        "scope": [],
        "verification": [],
    }
    contract_path.write_text(json.dumps(contract), encoding="utf-8")
    digest = ai_finish.worktree_digest_for_finish([], summary_path.relative_to(tmp_path).as_posix())
    summary_data = {
        "verification": [
            {
                "check": "aiSummary",
                "result": "passed",
                "runner": "ai_finish",
                "contractHash": __import__("hashlib")
                .sha256(contract_path.read_bytes())
                .hexdigest(),
                "commitSha": "a" * 40,
                "executionContractPath": contract_path.relative_to(tmp_path).as_posix(),
                "executionSummaryPath": summary_path.relative_to(tmp_path).as_posix(),
                "worktreeDigest": digest,
            }
        ]
    }
    summary_data["verification"][0]["outcomeInputDigest"] = ai_finish.outcome_input_digest(
        summary_data
    )
    summary_path.write_text(json.dumps(summary_data), encoding="utf-8")
    outcome_path = active / f"{task}.outcome.json"
    outcome_value = _outcome(task)
    outcome_path.write_text(json.dumps(outcome_value), encoding="utf-8")
    outcome_path.with_suffix(".md").write_text(render_markdown(outcome_value), encoding="utf-8")

    class Observer:
        def lifecycle_phase_finished(self, *_args, **_kwargs):
            pass

        def check_started(self, **_kwargs):
            pass

        def check_passed(self, **_kwargs):
            pass

        def check_failed(self, **_kwargs):
            pass

        def work_item_finished(self, **_kwargs):
            pass

    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "ensure_work_item_branch", lambda: None)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "preview", lambda **_kwargs: [])
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: Observer())
    monkeypatch.setattr(
        ai_check_agent_risk, "validate_checkpoint_bindings", lambda *_args, **_kwargs: []
    )
    monkeypatch.setattr(ai_finish, "documentation_alignment_issues", lambda *_args: [])
    monkeypatch.setattr(ai_finish, "active_terminal_outcome_issues", lambda *_args: ())
    monkeypatch.setattr(
        ai_finish, "_outcome_paths", lambda _task: (outcome_path, active / f"{task}.outcome.md")
    )
    monkeypatch.setattr(ai_finish, "render_direct_outcome_report", lambda *_args: "report\n")
    monkeypatch.setattr(ai_finish, "refresh_archived_human_report", lambda _task: (True, "ok"))
    report_refreshes = []
    monkeypatch.setattr(
        ai_finish,
        "run_human_report_pipeline",
        lambda _task, _summary: report_refreshes.append((_task, _summary)) or (True, "ok"),
    )
    monkeypatch.setattr(
        ai_finish, "bind_pre_archive_candidate_coverage_to_outcome", lambda _task: (True, "ok")
    )
    commands = []
    monkeypatch.setattr(
        ai_finish,
        "run",
        lambda command, **_kwargs: commands.append(command) or (0, 1, "ok"),
    )
    argv = ["ai_finish.py", "--task", task, "--language", "en"]
    if archive:
        argv.append("--archive")
    monkeypatch.setattr(sys, "argv", argv)

    assert ai_finish.main() == 0
    coverage_command = [
        "make",
        "check-changed-critical-coverage",
        "AI_BASE_COMMIT=" + "d" * 40,
        "CONTRACT=.ai/work-items/active/example-task.contract.json",
    ]
    archive_command = [
        "make",
        "archive-work-item",
        f"CONTRACT={contract_path.relative_to(tmp_path).as_posix()}",
    ]
    assert coverage_command in commands
    assert report_refreshes
    assert report_refreshes[-1] == (task, summary_path)
    if archive:
        assert archive_command in commands
    else:
        assert archive_command not in commands


def test_reused_finish_verification_blocks_archive_when_documentation_alignment_is_incomplete(
    tmp_path, monkeypatch
):
    """A stale completed Outcome must not reach archive through the reuse path."""
    task = "example-task"
    active = tmp_path / ".ai/work-items/active"
    active.mkdir(parents=True)
    contract_path = active / f"{task}.contract.json"
    summary_path = active / f"{task}.summary.json"
    contract = {
        "contractVersion": 2,
        "workItemId": task,
        "baseCommit": "a" * 40,
        "scope": [],
        "verification": [],
    }
    contract_path.write_text(json.dumps(contract), encoding="utf-8")
    digest = ai_finish.worktree_digest_for_finish([], summary_path.relative_to(tmp_path).as_posix())
    summary_path.write_text(
        json.dumps(
            {
                "verification": [
                    {
                        "check": "aiSummary",
                        "result": "passed",
                        "runner": "ai_finish",
                        "contractHash": __import__("hashlib")
                        .sha256(contract_path.read_bytes())
                        .hexdigest(),
                        "commitSha": "a" * 40,
                        "executionContractPath": contract_path.relative_to(tmp_path).as_posix(),
                        "executionSummaryPath": summary_path.relative_to(tmp_path).as_posix(),
                        "worktreeDigest": digest,
                    }
                ],
                "documentationAlignment": {
                    "schemaVersion": 1,
                    "status": "not_checked",
                    "checkedAt": None,
                    "checks": [],
                },
            }
        ),
        encoding="utf-8",
    )
    (active / f"{task}.outcome.json").write_text(json.dumps(_outcome(task)), encoding="utf-8")
    (active / f"{task}.outcome.md").write_text("# Task Outcome\n", encoding="utf-8")

    class Observer:
        def lifecycle_phase_finished(self, *_args, **_kwargs):
            pass

        def check_started(self, **_kwargs):
            pass

        def check_passed(self, **_kwargs):
            pass

        def check_failed(self, **_kwargs):
            pass

        def work_item_finished(self, **_kwargs):
            pass

    monkeypatch.setattr(ai_finish, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_finish, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_finish, "ensure_work_item_branch", lambda: None)
    monkeypatch.setattr(ai_finish, "current_head", lambda: "a" * 40)
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: [])
    monkeypatch.setattr(ai_finish, "preview", lambda **_kwargs: [])
    monkeypatch.setattr(ai_finish, "create_observability", lambda **_kwargs: Observer())
    monkeypatch.setattr(
        ai_check_agent_risk, "validate_checkpoint_bindings", lambda *_args, **_kwargs: []
    )
    blocked = {}
    monkeypatch.setattr(
        ai_finish,
        "return_blocked_finish_failure",
        lambda **kwargs: blocked.update(kwargs) or kwargs["code"],
    )
    commands = []
    monkeypatch.setattr(
        ai_finish,
        "run",
        lambda command, **_kwargs: commands.append(command) or (0, 1, "ok"),
    )
    monkeypatch.setattr(
        sys, "argv", ["ai_finish.py", "--task", task, "--archive", "--language", "en"]
    )

    assert ai_finish.main() == 1
    assert blocked["failed_check"] == "documentationAlignment"
    assert commands == []


def test_status_contains_only_outcome_link_count_and_status_not_full_report():
    status = render_active_status(
        {
            "recommendation": "needs_investigation",
            "signals": [],
            "evidence": {},
            "decisionDrivers": [],
        },
        work_item_id="example-task",
        mode="code",
        contract_path="contract.json",
        summary_path="summary.json",
        task_outcome={
            "status": "completed",
            "markdownPath": ".ai/work-items/active/example-task.outcome.md",
            "evidenceCount": 3,
        },
    )

    assert "Task Outcome" in status
    assert "- Signal Domain: `work_item_lifecycle`" in status
    assert "Presence: `present`" in status
    assert "example-task.outcome.md" in status
    assert "Evidence Count: `3`" in status
    assert "Full Outcome" not in status
    assert "score" not in status.lower()


def test_status_projects_missing_active_outcome_as_yellow_with_recovery_action():
    status = render_active_status(
        {
            "recommendation": "needs_investigation",
            "signals": [],
            "evidence": {},
            "decisionDrivers": [],
        },
        work_item_id="example-task",
        mode="code",
        contract_path="contract.json",
        summary_path="summary.json",
    )

    assert "## Task Outcome" in status
    assert "- Presence: `absent`" in status
    assert "- Traffic Light: `yellow`" in status
    assert "- Next Action: `continue verification or run make ai-finish`" in status


def test_status_projects_blocked_outcome_as_red_with_gate_and_recovery():
    status = render_active_status(
        {"recommendation": "blocked", "signals": [], "evidence": {}, "decisionDrivers": []},
        work_item_id="example-task",
        mode="code",
        contract_path="contract.json",
        summary_path="summary.json",
        task_outcome={
            "status": "blocked",
            "humanStatusColor": "red",
            "failedCheck": "quality",
            "recoveryCondition": "Run a passing quality retry.",
            "markdownPath": ".ai/work-items/active/example-task.outcome.md",
            "evidenceCount": 2,
        },
    )

    assert "- Presence: `present`" in status
    assert "- Traffic Light: `red`" in status
    assert "- Failed Gate: `quality`" in status
    assert "- Recovery Condition: `Run a passing quality retry.`" in status


def test_outcome_projection_rejects_cross_task_outcome_file(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / f"{task}.contract.json"
    outcome_path = tmp_path / f"{task}.outcome.json"
    contract_path.write_text(json.dumps({"workItemId": task}), encoding="utf-8")
    outcome_path.write_text(json.dumps({"workItemId": "other-task"}), encoding="utf-8")
    monkeypatch.setattr(ai_generate_status, "PROJECT_ROOT", tmp_path)

    with pytest.raises(RuntimeError, match="does not match active Work Item"):
        ai_generate_status.project_active_task_outcome(
            {"workItemId": task},
            {
                "taskOutcome": {
                    "status": "completed",
                    "jsonPath": outcome_path.name,
                    "markdownPath": f"{task}.outcome.md",
                }
            },
            contract_path,
        )


def test_outcome_projection_derives_green_from_bound_completed_outcome(tmp_path, monkeypatch):
    task = "example-task"
    contract_path = tmp_path / f"{task}.contract.json"
    outcome_path = tmp_path / f"{task}.outcome.json"
    markdown_path = tmp_path / f"{task}.outcome.md"
    bindings = {
        "taskId": task,
        "contractDigest": "a" * 64,
        "summaryDigest": "b" * 64,
        "verificationDigest": "c" * 64,
        "baseCommit": "d" * 40,
        "headCommit": "e" * 40,
        "lifecycleStage": "pre_merge",
        "pullRequest": {"state": "not_created"},
        "aiCockpitVersion": "0.5.48",
    }
    outcome = generate_outcome(task, bindings)
    outcome_path.write_text(json.dumps(outcome), encoding="utf-8")
    markdown_path.write_text(render_markdown(outcome), encoding="utf-8")
    monkeypatch.setattr(ai_generate_status, "PROJECT_ROOT", tmp_path)

    projection = ai_generate_status.project_active_task_outcome(
        {"workItemId": task},
        {
            "taskOutcome": {
                "status": "completed",
                "jsonPath": outcome_path.name,
                "markdownPath": markdown_path.name,
            }
        },
        contract_path,
    )

    assert projection == {
        "status": "completed",
        "humanStatusColor": "green",
        "failedCheck": "",
        "recoveryCondition": "",
        "markdownPath": markdown_path.name,
        "evidenceCount": 0,
    }
