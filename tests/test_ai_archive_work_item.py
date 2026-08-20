import hashlib
import json
from argparse import Namespace

import ai_archive_work_item
import ai_check_status_consistency
import ai_generate_human_report
import ai_lifecycle_truth
import pytest


def write_superseded_predecessor(tmp_path, *, work_item_id="task"):
    contract = tmp_path / f"{work_item_id}.contract.json"
    outcome = tmp_path / f"{work_item_id}.outcome.json"
    outcome.write_text(
        json.dumps({"workItemId": work_item_id, "status": "blocked"}), encoding="utf-8"
    )
    successor = {
        "workItemId": "successor",
        "branch": "codex/successor",
        "baseCommit": "a" * 40,
    }
    receipt = {
        "schemaVersion": 1,
        "transition": "superseded",
        "predecessor": {"workItemId": work_item_id},
        "predecessorOutcomeDigest": hashlib.sha256(outcome.read_bytes()).hexdigest(),
        "successor": successor,
        "successorWorkItemId": successor["workItemId"],
        "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/1",
        "authority": "user",
        "reason": "A verified successor owns the corrective delivery.",
    }
    receipt_path = tmp_path / f"{work_item_id}.successor-receipt.json"
    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
    return contract, outcome, receipt_path, receipt


def test_archive_growth_requires_projected_same_work_item_reservation():
    contract = {"workItemId": "task"}
    issues = ai_archive_work_item.validate_archive_growth_reservation(
        contract, 487, {"max": {"archiveGrowth": 488}}
    )
    assert any("reservation is required" in issue for issue in issues)


def test_archive_inputs_reject_non_green_outcome_before_mutation(tmp_path, monkeypatch):
    active = tmp_path / ".ai/work-items/active"
    active.mkdir(parents=True)
    contract_path = active / "task.contract.json"
    summary_path = active / "task.summary.json"
    contract_path.write_text(
        json.dumps({"workItemId": "task", "baseCommit": "a" * 40}), encoding="utf-8"
    )
    summary_path.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    (active / "task.outcome.json").write_text(
        json.dumps(
            {
                "workItemId": "task",
                "status": "needs_human_confirmation",
                "humanStatusColor": "yellow",
            }
        ),
        encoding="utf-8",
    )
    (active / "task.outcome.md").write_text("# Task Outcome\n", encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda _contract: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])

    issues = ai_archive_work_item._validate_archive_inputs(
        contract_path,
        json.loads(contract_path.read_text(encoding="utf-8")),
        summary_path,
        json.loads(summary_path.read_text(encoding="utf-8")),
    )

    assert any("completed" in issue for issue in issues)
    assert any("green" in issue for issue in issues)


def test_archive_growth_reservation_accepts_projected_count_and_repayment():
    contract = {
        "workItemId": "task",
        "budgetImpact": {
            "expectedMetrics": {"archiveGrowth": 488},
            "approved": True,
            "repaymentWorkItem": "task",
            "repaymentRecords": [".ai/guards/governance_complexity_policy.yaml"],
        },
    }
    assert (
        ai_archive_work_item.validate_archive_growth_reservation(
            contract, 487, {"max": {"archiveGrowth": 488}}
        )
        == []
    )


def test_archive_growth_accepts_bounded_future_reservation():
    contract = {
        "workItemId": "budget-window",
        "budgetImpact": {
            "expectedMetrics": {"archiveGrowth": 496},
            "reservedFutureMetrics": {"archiveGrowth": 497},
            "approved": True,
            "repaymentWorkItem": "budget-window",
            "repaymentRecords": ["policy"],
        },
    }
    assert (
        ai_archive_work_item.validate_archive_growth_reservation(
            contract, 495, {"max": {"archiveGrowth": 497}}
        )
        == []
    )


def test_archive_growth_string_policy_limit_rejects_unapproved_overrun():
    contract = {
        "workItemId": "task",
        "budgetImpact": {"expectedMetrics": {"archiveGrowth": 493}},
    }
    issues = ai_archive_work_item.validate_archive_growth_reservation(
        contract, 492, {"max": {"archiveGrowth": "492"}}
    )
    assert any(
        "projected archiveGrowth=493 exceeds configured maximum 492" in issue for issue in issues
    )
    assert any("requires budgetImpact.approved=true" in issue for issue in issues)


def test_archive_growth_reservation_rejects_stale_projection():
    contract = {
        "workItemId": "task",
        "budgetImpact": {"expectedMetrics": {"archiveGrowth": 487}},
    }
    issues = ai_archive_work_item.validate_archive_growth_reservation(
        contract, 487, {"max": {"archiveGrowth": 488}}
    )
    assert any("reservation is stale" in issue for issue in issues)


def test_archive_growth_overrun_is_warning_only_when_policy_declares_warning_mode():
    contract = {"workItemId": "task", "budgetImpact": {}}
    policy = {"max": {"archiveGrowth": 200}, "enforcement": {"archiveGrowth": "warning"}}

    assert ai_archive_work_item.validate_archive_growth_reservation(contract, 527, policy) == []
    assert ai_archive_work_item.archive_growth_warnings(contract, 527, policy) == [
        "projected archiveGrowth=528 exceeds configured maximum 200 (warning)"
    ]


def test_archive_moves_task_owned_success_criteria_sibling(tmp_path):
    contract = tmp_path / ".ai" / "work-items" / "active" / "task.contract.json"
    assert ai_archive_work_item.owned_success_criteria_path(contract) == contract.with_name(
        "task.success.json"
    )


def test_outcome_artifact_paths_are_stable_and_ordered(tmp_path):
    contract = tmp_path / "task.contract.json"
    assert [path.name for path in ai_archive_work_item.outcome_artifact_paths(contract)] == [
        "task.outcome.json",
        "task.outcome.md",
        "task.events.jsonl",
        "task.successor-receipt.json",
    ]


def test_bound_superseded_receipt_is_the_only_failed_verification_archive_exception(tmp_path):
    contract, _, receipt, receipt_data = write_superseded_predecessor(tmp_path)
    issues = [
        "Summary is missing required verification: aiStatus",
        "required verification is not passed: quality",
    ]
    assert ai_archive_work_item.superseded_archive_validation_exception(
        contract_path=contract, work_item_id="task", summary_issues=issues
    )

    receipt_data["transition"] = "quarantined"
    receipt.write_text(json.dumps(receipt_data), encoding="utf-8")
    assert not ai_archive_work_item.superseded_archive_validation_exception(
        contract_path=contract, work_item_id="task", summary_issues=issues
    )
    assert not ai_archive_work_item.superseded_archive_validation_exception(
        contract_path=contract,
        work_item_id="task",
        summary_issues=["summary contractHash does not match Contract"],
    )


def test_canonical_superseded_summary_exception_accepts_only_bound_red_evidence(tmp_path):
    contract, _, _, _ = write_superseded_predecessor(tmp_path)
    issues = [
        "Summary is missing required verification: aiStatus",
        "required verification is not passed: quality",
    ]

    assert ai_lifecycle_truth.superseded_summary_validation_exception(
        contract_path=contract,
        work_item_id="task",
        summary_issues=issues,
    )
    assert not ai_lifecycle_truth.superseded_summary_validation_exception(
        contract_path=contract,
        work_item_id="task",
        summary_issues=[*issues, "summary contractHash does not match Contract"],
    )


def test_canonical_superseded_transition_predicate_accepts_bound_red_evidence(tmp_path):
    contract, _, _, _ = write_superseded_predecessor(tmp_path)

    assert ai_lifecycle_truth.is_valid_superseded_transition(
        contract_path=contract,
        work_item_id="task",
    )


@pytest.mark.parametrize(
    "invalid_case",
    [
        "missing_receipt",
        "malformed_receipt",
        "quarantined",
        "wrong_digest",
        "wrong_predecessor",
        "non_blocked",
        "foreign_issue",
        "missing_authority",
        "missing_reason",
    ],
)
def test_canonical_superseded_summary_exception_rejects_invalid_evidence(tmp_path, invalid_case):
    contract, outcome_path, receipt_path, receipt = write_superseded_predecessor(tmp_path)
    if invalid_case == "missing_receipt":
        receipt_path.unlink()
    elif invalid_case == "malformed_receipt":
        receipt_path.write_text("{", encoding="utf-8")
    elif invalid_case == "quarantined":
        receipt["transition"] = "quarantined"
    elif invalid_case == "wrong_digest":
        receipt["predecessorOutcomeDigest"] = "0" * 64
    elif invalid_case == "wrong_predecessor":
        receipt["predecessor"] = {"workItemId": "other"}
    elif invalid_case == "non_blocked":
        outcome_path.write_text(
            json.dumps({"workItemId": "task", "status": "completed"}), encoding="utf-8"
        )
        receipt["predecessorOutcomeDigest"] = hashlib.sha256(outcome_path.read_bytes()).hexdigest()
    elif invalid_case == "foreign_issue":
        receipt["issue"] = "https://example.test/issues/1"
    elif invalid_case == "missing_authority":
        receipt["authority"] = ""
    elif invalid_case == "missing_reason":
        receipt["reason"] = ""
    if invalid_case not in {"missing_receipt", "malformed_receipt"}:
        receipt_path.write_text(json.dumps(receipt), encoding="utf-8")

    assert not ai_lifecycle_truth.superseded_summary_validation_exception(
        contract_path=contract,
        work_item_id="task",
        summary_issues=["required verification is not passed: quality"],
    )
    assert not ai_lifecycle_truth.is_valid_superseded_transition(
        contract_path=contract,
        work_item_id="task",
    )


def test_next_archive_sequence_prefers_existing_index(tmp_path, monkeypatch):
    archive = tmp_path / "archive"
    archive.mkdir()
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive)
    (archive / "index.json").write_text(
        '{"indexVersion": 1, "entries": [{"archiveSequence": 41}]}',
        encoding="utf-8",
    )

    assert ai_archive_work_item._next_archive_sequence() == 42


def test_archive_preflight_rejects_trailing_whitespace_in_active_artifacts(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    artifact = tmp_path / ".ai/work-items/active/task.contract.json"
    artifact.parent.mkdir(parents=True)
    artifact.write_text('{"workItemId": "task"} \n', encoding="utf-8")

    assert ai_archive_work_item.archive_text_whitespace_issues([artifact]) == [
        ".ai/work-items/active/task.contract.json:1: trailing whitespace"
    ]


def test_archive_manifest_is_stable_and_excludes_generated_status(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    contract.write_text(json.dumps({"workItemId": "task", "baseCommit": "base"}), encoding="utf-8")
    summary.write_text(
        json.dumps({"workItemId": "task", "contractPath": "task.contract.json"}), encoding="utf-8"
    )

    manifest = ai_archive_work_item._archive_manifest(
        contract_target=contract, summary_target=summary, archive_sequence=7
    )

    assert manifest["manifestVersion"] == 1
    assert manifest["archiveSequence"] == 7
    assert manifest["generatedStatusExcluded"] is True
    assert "manifestSha256" not in manifest
    assert (
        manifest["contractSha256"]
        == __import__("hashlib").sha256(contract.read_bytes()).hexdigest()
    )
    assert (
        manifest["summarySha256"] == __import__("hashlib").sha256(summary.read_bytes()).hexdigest()
    )


def test_archive_manifest_binds_outcome_artifact_digests(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    outcome = tmp_path / "task.outcome.json"
    contract.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    summary.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    outcome.write_text("{}\n", encoding="utf-8")
    manifest = ai_archive_work_item._archive_manifest(
        contract_target=contract,
        summary_target=summary,
        archive_sequence=8,
        outcome_targets=[outcome],
    )
    assert manifest["outcomeArtifacts"][0]["path"] == "task.outcome.json"
    assert len(manifest["outcomeArtifacts"][0]["sha256"]) == 64


def test_superseded_archive_transaction_preserves_receipt_bound_outcome_bytes(
    tmp_path, monkeypatch
):
    active = tmp_path / ".ai/work-items/active"
    cockpit = tmp_path / ".ai/cockpit"
    target = tmp_path / ".ai/work-items/archive/2026"
    active.mkdir(parents=True)
    cockpit.mkdir(parents=True)
    target.mkdir(parents=True)
    (target.parent / "index.json").write_text('{"indexVersion": 1, "entries": []}\n')
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    outcome = active / "task.outcome.json"
    receipt = active / "task.successor-receipt.json"
    contract.write_text('{"workItemId":"task"}\n', encoding="utf-8")
    summary.write_text('{"changedFiles":[]}\n', encoding="utf-8")
    outcome.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "status": "blocked",
                "sections": {
                    "evidence": [
                        {
                            "source": ".ai/work-items/active/task.contract.json",
                            "subject": "Contract",
                        }
                    ]
                },
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    outcome_before = outcome.read_bytes()
    receipt.write_text(
        json.dumps(
            {
                "schemaVersion": 1,
                "transition": "superseded",
                "predecessor": {"workItemId": "task"},
                "predecessorOutcomeDigest": hashlib.sha256(outcome_before).hexdigest(),
                "successor": {
                    "workItemId": "successor",
                    "branch": "codex/successor",
                    "baseCommit": "a" * 40,
                },
                "successorWorkItemId": "successor",
                "issue": "https://github.com/spirex-ds-dev/ai-cockpit-template/issues/1",
                "authority": "user",
                "reason": "A verified successor owns the corrective delivery.",
            }
        )
        + "\n",
        encoding="utf-8",
    )

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_archive_work_item, "ARCHIVE_BASE_DIR", tmp_path / ".ai/work-items/archive"
    )
    monkeypatch.setattr(ai_archive_work_item, "_generate_status", lambda _command: None)

    sources = (contract, summary, outcome, receipt)
    ai_archive_work_item._execute_archive_transaction(
        contract_path=contract,
        summary_path=summary,
        review_path=active / "task.review.json",
        success_path=active / "task.success.json",
        outcome_paths=[outcome, receipt],
        files_to_move=[(path, target / path.name) for path in sources],
        target_dir=target,
        summary_tmp=target / ".task.summary.tmp",
        manifest_target=target / "task.archive-manifest.json",
        has_summary=True,
        has_review=False,
        has_success=False,
        archive_sequence=1,
        traceability_path=tmp_path / "docs/reference/traceability.json",
        traceability_backup=None,
        traceability_payload=None,
        preserve_superseded_outcome=True,
    )

    archived_outcome = target / outcome.name
    assert archived_outcome.read_bytes() == outcome_before
    assert ai_lifecycle_truth.is_valid_superseded_transition(
        contract_path=target / contract.name,
        work_item_id="task",
    )
    manifest = json.loads((target / "task.archive-manifest.json").read_text(encoding="utf-8"))
    artifact_digests = {item["path"]: item["sha256"] for item in manifest["outcomeArtifacts"]}
    assert (
        artifact_digests[archived_outcome.relative_to(tmp_path).as_posix()]
        == hashlib.sha256(outcome_before).hexdigest()
    )


def test_archived_outcome_projection_rewrites_only_existing_transaction_artifacts(tmp_path):
    task = "task"
    active = tmp_path / ".ai/work-items/active"
    archive = tmp_path / ".ai/work-items/archive/2026"
    archive.mkdir(parents=True)
    contract = archive / f"{task}.contract.json"
    outcome_path = archive / f"{task}.outcome.json"
    contract.write_text("{}", encoding="utf-8")
    outcome_path.write_text("{}", encoding="utf-8")
    active_contract = (active / contract.name).relative_to(tmp_path).as_posix()
    active_outcome = (active / outcome_path.name).relative_to(tmp_path).as_posix()
    absent_summary = (active / f"{task}.summary.json").relative_to(tmp_path).as_posix()
    outcome = {
        "evidence": [active_contract, active_outcome, absent_summary],
        "embedded": f"prefix:{active_outcome}",
    }

    projected = ai_lifecycle_truth.archived_outcome_projection(
        outcome,
        root=tmp_path,
        contract_path=contract,
        work_item_id=task,
    )

    assert projected["evidence"] == [
        contract.relative_to(tmp_path).as_posix(),
        outcome_path.relative_to(tmp_path).as_posix(),
        absent_summary,
    ]
    assert projected["embedded"] == f"prefix:{active_outcome}"
    assert outcome["evidence"][0] == active_contract


def test_archive_manifest_binds_content_addressed_pre_archive_coverage(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    contract = tmp_path / "task.contract.json"
    summary = tmp_path / "task.summary.json"
    contract.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    summary.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")

    coverage = {
        "reportSha256": "a" * 64,
        "binding": {
            "baseCommit": "b" * 40,
            "candidateHead": "c" * 40,
            "candidateTreeDigest": "d" * 64,
            "candidateDiffDigest": "e" * 64,
        },
    }
    manifest = ai_archive_work_item._archive_manifest(
        contract_target=contract,
        summary_target=summary,
        archive_sequence=9,
        pre_archive_candidate_coverage=coverage,
    )

    assert manifest["preArchiveCandidateCoverage"] == coverage


def test_pre_archive_candidate_coverage_requires_matching_outcome_binding(tmp_path, monkeypatch):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / "target"
    active.mkdir(parents=True)
    target.mkdir()
    contract = active / "task.contract.json"
    contract.write_text(
        json.dumps({"workItemId": "task", "baseCommit": "a" * 40}), encoding="utf-8"
    )
    binding = {
        "baseCommit": "a" * 40,
        "candidateHead": "b" * 40,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    report = {"binding": binding}
    report_bytes = (json.dumps(report, sort_keys=True) + "\n").encode("utf-8")
    (target / "changed-critical-coverage.json").write_bytes(report_bytes)
    coverage = {"reportSha256": hashlib.sha256(report_bytes).hexdigest(), "binding": binding}
    (active / "task.outcome.json").write_text(
        json.dumps({"bindings": {"preArchiveCandidateCoverage": coverage}}), encoding="utf-8"
    )

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    import check_changed_critical_coverage

    monkeypatch.setattr(
        check_changed_critical_coverage, "candidate_snapshot", lambda **_kwargs: binding
    )

    assert (
        ai_archive_work_item.load_pre_archive_candidate_coverage(
            contract_path=contract, contract=json.loads(contract.read_text(encoding="utf-8"))
        )
        == coverage
    )


def test_pre_archive_coverage_accepts_only_missing_historical_binding_for_superseded(
    tmp_path, monkeypatch
):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / "target"
    active.mkdir(parents=True)
    target.mkdir()
    contract, outcome, _, _ = write_superseded_predecessor(active)
    contract_payload = {"workItemId": "task", "baseCommit": "a" * 40}
    contract.write_text(json.dumps(contract_payload), encoding="utf-8")
    outcome_before = outcome.read_bytes()
    binding = {
        "baseCommit": "a" * 40,
        "candidateHead": "b" * 40,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    report_bytes = (json.dumps({"binding": binding}, sort_keys=True) + "\n").encode()
    (target / "changed-critical-coverage.json").write_bytes(report_bytes)
    coverage = {"reportSha256": hashlib.sha256(report_bytes).hexdigest(), "binding": binding}

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    import check_changed_critical_coverage

    monkeypatch.setattr(
        check_changed_critical_coverage, "candidate_snapshot", lambda **_kwargs: binding
    )

    assert (
        ai_archive_work_item.load_pre_archive_candidate_coverage(
            contract_path=contract,
            contract=contract_payload,
        )
        == coverage
    )
    assert outcome.read_bytes() == outcome_before


def test_pre_archive_coverage_accepts_superseded_remote_default_tip_base(tmp_path, monkeypatch):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / "target"
    active.mkdir(parents=True)
    target.mkdir()
    contract, outcome, _, _ = write_superseded_predecessor(active)
    contract_payload = {"workItemId": "task", "baseCommit": "a" * 40}
    contract.write_text(json.dumps(contract_payload), encoding="utf-8")
    outcome_before = outcome.read_bytes()
    binding = {
        "baseCommit": "b" * 40,
        "candidateHead": "b" * 40,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    report_bytes = (json.dumps({"binding": binding}, sort_keys=True) + "\n").encode()
    (target / "changed-critical-coverage.json").write_bytes(report_bytes)
    coverage = {"reportSha256": hashlib.sha256(report_bytes).hexdigest(), "binding": binding}

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "_unique_remote_default_tip", lambda: "b" * 40)
    import check_changed_critical_coverage

    def snapshot(**kwargs):
        assert kwargs["base"] == "b" * 40
        return binding

    monkeypatch.setattr(check_changed_critical_coverage, "candidate_snapshot", snapshot)

    assert (
        ai_archive_work_item.load_pre_archive_candidate_coverage(
            contract_path=contract,
            contract=contract_payload,
        )
        == coverage
    )
    assert outcome.read_bytes() == outcome_before


@pytest.mark.parametrize(
    ("report_base", "candidate_head", "remote_tip"),
    [
        ("b" * 40, "c" * 40, "b" * 40),
        ("b" * 40, "b" * 40, "c" * 40),
    ],
)
def test_pre_archive_coverage_rejects_untrusted_superseded_alternate_base(
    tmp_path, monkeypatch, report_base, candidate_head, remote_tip
):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / "target"
    active.mkdir(parents=True)
    target.mkdir()
    contract, _, _, _ = write_superseded_predecessor(active)
    contract_payload = {"workItemId": "task", "baseCommit": "a" * 40}
    contract.write_text(json.dumps(contract_payload), encoding="utf-8")
    binding = {
        "baseCommit": report_base,
        "candidateHead": candidate_head,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    (target / "changed-critical-coverage.json").write_text(
        json.dumps({"binding": binding}) + "\n", encoding="utf-8"
    )
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(ai_archive_work_item, "_unique_remote_default_tip", lambda: remote_tip)

    with pytest.raises(ValueError, match="remote default tip"):
        ai_archive_work_item.load_pre_archive_candidate_coverage(
            contract_path=contract,
            contract=contract_payload,
        )


@pytest.mark.parametrize(
    "candidates",
    [[], [("origin", "main"), ("upstream", "main")]],
)
def test_unique_remote_default_tip_rejects_missing_or_ambiguous_identity(monkeypatch, candidates):
    monkeypatch.setattr(
        ai_archive_work_item,
        "discover_remote_default_candidates",
        lambda _runner: candidates,
    )

    with pytest.raises(ValueError, match="one unique remote default tip"):
        ai_archive_work_item._unique_remote_default_tip()


def test_pre_archive_coverage_rejects_existing_mismatch_even_when_superseded(tmp_path, monkeypatch):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / "target"
    active.mkdir(parents=True)
    target.mkdir()
    contract, outcome, receipt_path, receipt = write_superseded_predecessor(active)
    contract_payload = {"workItemId": "task", "baseCommit": "a" * 40}
    contract.write_text(json.dumps(contract_payload), encoding="utf-8")
    binding = {
        "baseCommit": "a" * 40,
        "candidateHead": "b" * 40,
        "candidateTreeDigest": "c" * 64,
        "candidateDiffDigest": "d" * 64,
    }
    report_bytes = (json.dumps({"binding": binding}, sort_keys=True) + "\n").encode()
    (target / "changed-critical-coverage.json").write_bytes(report_bytes)
    wrong_coverage = {"reportSha256": "0" * 64, "binding": binding}
    outcome.write_text(
        json.dumps(
            {
                "workItemId": "task",
                "status": "blocked",
                "bindings": {"preArchiveCandidateCoverage": wrong_coverage},
            }
        ),
        encoding="utf-8",
    )
    receipt["predecessorOutcomeDigest"] = hashlib.sha256(outcome.read_bytes()).hexdigest()
    receipt_path.write_text(json.dumps(receipt), encoding="utf-8")

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    import check_changed_critical_coverage

    monkeypatch.setattr(
        check_changed_critical_coverage, "candidate_snapshot", lambda **_kwargs: binding
    )

    with pytest.raises(ValueError, match="does not bind"):
        ai_archive_work_item.load_pre_archive_candidate_coverage(
            contract_path=contract,
            contract=contract_payload,
        )


def test_current_worktree_digest_excludes_self_referential_lifecycle_projections(monkeypatch):
    monkeypatch.setattr(
        ai_archive_work_item,
        "changed_paths",
        lambda _contract: [
            "src/app.py",
            ".ai/work-items/active/task.summary.json",
            ".ai/work-items/active/task.outcome.json",
            ".ai/work-items/active/task.outcome.md",
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
        ],
    )
    monkeypatch.setattr(ai_archive_work_item, "path_fingerprint", lambda path: f"digest:{path}")

    digest = ai_archive_work_item._current_worktree_digest(
        {
            "workItemId": "task",
            "summaryPath": ".ai/work-items/active/task.summary.json",
        }
    )

    assert digest == ai_archive_work_item._worktree_digest(["src/app.py"])


def test_archive_entry_references_manifest_digest(tmp_path, monkeypatch):
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    target = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    target.mkdir(parents=True)
    contract_path = target / "task.contract.json"
    summary_path = target / "task.summary.json"
    manifest_path = target / "task.archive-manifest.json"
    contract_path.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    summary_path.write_text(json.dumps({"workItemId": "task"}), encoding="utf-8")
    manifest_path.write_text(
        json.dumps({"format": "ai-cockpit-archive-manifest"}), encoding="utf-8"
    )

    entry = ai_archive_work_item._archive_entry(
        contract_path=contract_path,
        summary_path=summary_path,
        target_dir=target,
        archive_sequence=1,
    )

    assert entry["manifestPath"].endswith("task.archive-manifest.json")
    assert len(entry["manifestSha256"]) == 64


def test_is_ignored_matches_gitignore_archive_patterns(tmp_path, monkeypatch):
    (tmp_path / ".gitignore").write_text("local/*.json\n!local/kept.json\n", encoding="utf-8")
    local = tmp_path / "local"
    local.mkdir()
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)

    assert ai_archive_work_item._is_ignored(local / "old.json")
    assert not ai_archive_work_item._is_ignored(local / "kept.json")
    assert not ai_archive_work_item._is_ignored(tmp_path / "other.txt")


def test_restore_files_moves_archive_inputs_back(tmp_path):
    active = tmp_path / ".ai" / "work-items" / "active"
    archive = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    active.mkdir(parents=True)
    archive.mkdir(parents=True)

    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    archived_contract = archive / contract.name
    archived_summary = archive / summary.name
    archived_contract.write_text("contract", encoding="utf-8")
    archived_summary.write_text("summary", encoding="utf-8")

    ai_archive_work_item._restore_files(
        [(contract, archived_contract), (summary, archived_summary)]
    )

    assert contract.read_text(encoding="utf-8") == "contract"
    assert summary.read_text(encoding="utf-8") == "summary"
    assert not archived_contract.exists()
    assert not archived_summary.exists()


def test_load_archive_index_adds_unindexed_authoritative_pair(tmp_path, monkeypatch):
    archive = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    archive.mkdir(parents=True)
    contract = archive / "legacy.contract.json"
    summary = archive / "legacy.summary.json"
    contract.write_text(json.dumps({"workItemId": "legacy"}), encoding="utf-8")
    summary.write_text(
        json.dumps(
            {
                "workItemId": "legacy",
                "contractPath": ".ai/work-items/archive/2026/legacy.contract.json",
            }
        ),
        encoding="utf-8",
    )
    (archive.parent / "index.json").write_text('{"entries": []}', encoding="utf-8")
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive.parent)

    result = ai_archive_work_item._load_archive_index()

    assert len(result["entries"]) == 1
    assert result["entries"][0]["contractPath"].endswith("legacy.contract.json")


def test_load_archive_index_deduplicates_pair_and_prefers_strict_entry(tmp_path, monkeypatch):
    archive = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    archive.mkdir(parents=True)
    (archive / "task.contract.json").write_text("{}", encoding="utf-8")
    (archive / "task.summary.json").write_text("{}", encoding="utf-8")
    index = archive.parent / "index.json"
    pair = {
        "contractPath": ".ai/work-items/archive/2026/task.contract.json",
        "summaryPath": ".ai/work-items/archive/2026/task.summary.json",
    }
    index.write_text(
        json.dumps(
            {
                "entries": [
                    {**pair, "workItemId": "task", "archivedAt": "legacy"},
                    {
                        **pair,
                        "workItemId": "task",
                        "contractSha256": "a" * 64,
                        "summarySha256": "b" * 64,
                        "archivedAt": "current",
                    },
                ]
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive.parent)

    result = ai_archive_work_item._load_archive_index()

    assert len(result["entries"]) == 1
    assert result["entries"][0]["archivedAt"] == "current"


def test_load_archive_index_drops_stale_pair(tmp_path, monkeypatch):
    archive = tmp_path / ".ai" / "work-items" / "archive" / "2026"
    archive.mkdir(parents=True)
    (archive.parent / "index.json").write_text(
        json.dumps(
            {
                "entries": [
                    {
                        "contractPath": ".ai/work-items/archive/2026/removed.contract.json",
                        "summaryPath": ".ai/work-items/archive/2026/removed.summary.json",
                    }
                ]
            }
        ),
        encoding="utf-8",
    )
    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_archive_work_item, "ARCHIVE_BASE_DIR", archive.parent)

    result = ai_archive_work_item._load_archive_index()

    assert result["entries"] == []


def test_main_dry_run_validates_summary_and_current_digest(tmp_path, monkeypatch):
    project_root = tmp_path / "project"
    active = project_root / ".ai" / "work-items" / "active"
    archive = project_root / ".ai" / "work-items" / "archive" / "2026"
    active.mkdir(parents=True)
    archive.mkdir(parents=True)

    contract_path = active / "task.contract.json"
    summary_path = active / "task.summary.json"
    contract_path.write_text(
        json.dumps(
            {
                "summaryVersion": 2,
                "workItemId": "task",
                "mode": "code",
                "scope": ["src/app.py"],
                "budgetImpact": {
                    "expectedMetrics": {"archiveGrowth": 1},
                    "approved": True,
                    "repaymentWorkItem": "task",
                    "repaymentRecords": ["policy"],
                },
            }
        ),
        encoding="utf-8",
    )

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", project_root)
    monkeypatch.setattr(ai_archive_work_item, "ACTIVE_DIR", active)
    monkeypatch.setattr(
        ai_archive_work_item, "ARCHIVE_BASE_DIR", project_root / ".ai" / "work-items" / "archive"
    )
    monkeypatch.setattr(ai_archive_work_item, "validate_contract", lambda _contract: [])
    monkeypatch.setattr(ai_archive_work_item, "validate_summary", lambda *_args, **_kwargs: [])
    monkeypatch.setattr(
        ai_archive_work_item, "changed_paths", lambda _contract: ["src/app.py", "src/app.py"]
    )
    monkeypatch.setattr(ai_archive_work_item, "path_fingerprint", lambda path: f"digest:{path}")

    current_digest = ai_archive_work_item._current_worktree_digest({"scope": ["src/app.py"]})

    class DummyObservability:
        def record(self, *_args, **_kwargs):
            return None

    monkeypatch.setattr(
        ai_archive_work_item, "create_observability", lambda *_args, **_kwargs: DummyObservability()
    )
    monkeypatch.setattr(
        ai_archive_work_item,
        "parse_args",
        lambda: Namespace(contract=str(contract_path), dry_run=True),
    )
    summary_path.write_text(
        json.dumps(
            {
                "verification": [
                    {"check": "aiSummary", "result": "passed", "worktreeDigest": current_digest}
                ]
            }
        ),
        encoding="utf-8",
    )

    assert ai_archive_work_item.main() == 0
    assert contract_path.exists()
    assert summary_path.exists()


def test_rewrite_traceability_paths_rewrites_every_archived_evidence_value():
    active_prefix = ".ai/work-items/active/task"
    archive_prefix = ".ai/work-items/archive/2026/task"
    replacements = {
        f"{active_prefix}.contract.json": f"{archive_prefix}.contract.json",
        f"{active_prefix}.summary.json": f"{archive_prefix}.summary.json",
        f"{active_prefix}.review.json": f"{archive_prefix}.review.json",
        f"{active_prefix}.success.json": f"{archive_prefix}.success.json",
        f"{active_prefix}.outcome.json": f"{archive_prefix}.outcome.json",
    }
    payload = {
        "instructions": [
            {
                "contractPaths": [f"{active_prefix}.contract.json"],
                "implementationEvidence": [f"{active_prefix}.review.json"],
                "acceptanceEvidence": [
                    f"{active_prefix}.summary.json",
                    f"{active_prefix}.success.json",
                    f"{active_prefix}.outcome.json",
                ],
            }
        ]
    }

    rewritten, count = ai_archive_work_item._rewrite_traceability_paths(payload, replacements)

    serialized = json.dumps(rewritten, sort_keys=True)
    assert count == len(replacements)
    assert ".ai/work-items/active/task" not in serialized
    assert all(target in serialized for target in replacements.values())


def test_archive_failure_rolls_back_rewritten_traceability_bytes(tmp_path, monkeypatch):
    active = tmp_path / ".ai/work-items/active"
    target = tmp_path / ".ai/work-items/archive/2026"
    active.mkdir(parents=True)
    target.mkdir(parents=True)
    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    review = active / "task.review.json"
    outcome = active / "task.outcome.json"
    contract.write_text('{"workItemId":"task"}\n', encoding="utf-8")
    summary.write_text('{"changedFiles":[]}\n', encoding="utf-8")
    review.write_text("{}\n", encoding="utf-8")
    outcome.write_text(
        json.dumps(
            {
                "format": "ai-cockpit-task-outcome",
                "schemaVersion": 1,
                "workItemId": "task",
                "status": "completed",
                "humanStatusColor": "green",
                "bindings": {
                    "taskId": "task",
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
                    "outcomeSummary": "Completed.",
                    "taskOverview": "Task.",
                    "deliveredChanges": [],
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
        ),
        encoding="utf-8",
    )
    sources = (contract, summary, review, outcome)
    original_source_bytes = {path: path.read_bytes() for path in sources}
    traceability = tmp_path / "docs/reference/remediation-instruction-traceability.json"
    traceability.parent.mkdir(parents=True)
    traceability.write_text(
        json.dumps(
            {
                "contractPaths": [".ai/work-items/active/task.contract.json"],
                "acceptanceEvidence": [
                    ".ai/work-items/active/task.summary.json",
                    ".ai/work-items/active/task.outcome.json",
                ],
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    original_traceability = traceability.read_bytes()

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_archive_work_item,
        "ARCHIVE_BASE_DIR",
        tmp_path / ".ai/work-items/archive",
    )
    monkeypatch.setattr(ai_archive_work_item, "_generate_status", lambda _command: None)
    monkeypatch.setattr(
        ai_archive_work_item,
        "_load_archive_index",
        lambda: (_ for _ in ()).throw(RuntimeError("post-rewrite failure")),
    )

    with pytest.raises(RuntimeError, match="post-rewrite failure"):
        ai_archive_work_item._execute_archive_transaction(
            contract_path=contract,
            summary_path=summary,
            review_path=review,
            success_path=active / "task.success.json",
            outcome_paths=[outcome],
            files_to_move=[(path, target / path.name) for path in sources],
            target_dir=target,
            summary_tmp=target / ".task.summary.tmp",
            manifest_target=target / "task.archive-manifest.json",
            has_summary=True,
            has_review=True,
            has_success=False,
            archive_sequence=1,
            traceability_path=traceability,
            traceability_backup=original_traceability,
            traceability_payload=json.loads(original_traceability),
        )

    assert traceability.read_bytes() == original_traceability
    assert all(path.read_bytes() == original_source_bytes[path] for path in sources)


def test_archive_transaction_records_refreshed_human_report_paths_in_archived_summary(
    tmp_path, monkeypatch
):
    active = tmp_path / ".ai/work-items/active"
    cockpit = tmp_path / ".ai/cockpit"
    target = tmp_path / ".ai/work-items/archive/2026"
    active.mkdir(parents=True)
    cockpit.mkdir(parents=True)
    target.mkdir(parents=True)
    (target.parent / "index.json").write_text('{"indexVersion": 1, "entries": []}\n')

    contract = active / "task.contract.json"
    summary = active / "task.summary.json"
    review = active / "task.review.json"
    outcome = active / "task.outcome.json"
    contract.write_text('{"workItemId":"task"}\n', encoding="utf-8")
    summary.write_text(
        json.dumps(
            {
                "summaryVersion": 2,
                "workItemId": "task",
                "changedFiles": [{"path": ".ai/work-items/starts/task.json", "reason": "fixture"}],
                "documentationAlignment": {
                    "schemaVersion": 1,
                    "status": "aligned",
                    "checkedAt": "2026-08-08T00:00:00+00:00",
                    "checks": [
                        {
                            "area": "documentationCommandsCapability",
                            "status": "aligned",
                            "evidence": [".ai/work-items/starts/task.json"],
                            "reason": "fixture",
                        }
                    ],
                },
            }
        )
        + "\n",
        encoding="utf-8",
    )
    review.write_text("{}\n", encoding="utf-8")
    outcome_payload = {
        "format": "ai-cockpit-task-outcome",
        "schemaVersion": 1,
        "workItemId": "task",
        "status": "completed",
        "bindings": {
            "taskId": "task",
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
            "outcomeSummary": "Completed.",
            "taskOverview": "Task.",
            "deliveredChanges": [],
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
    outcome.write_text(json.dumps(outcome_payload), encoding="utf-8")
    report = ai_generate_human_report.generate_human_report(outcome_payload)
    (cockpit / "task_report.json").write_text(json.dumps(report), encoding="utf-8")
    (cockpit / "task_report.md").write_text(
        ai_generate_human_report.render_human_report(report), encoding="utf-8"
    )

    monkeypatch.setattr(ai_archive_work_item, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(
        ai_archive_work_item, "ARCHIVE_BASE_DIR", tmp_path / ".ai/work-items/archive"
    )
    monkeypatch.setattr(ai_archive_work_item, "_generate_status", lambda _command: None)

    sources = (contract, summary, review, outcome)
    ai_archive_work_item._execute_archive_transaction(
        contract_path=contract,
        summary_path=summary,
        review_path=review,
        success_path=active / "task.success.json",
        outcome_paths=[outcome],
        files_to_move=[(path, target / path.name) for path in sources],
        target_dir=target,
        summary_tmp=target / ".task.summary.tmp",
        manifest_target=target / "task.archive-manifest.json",
        has_summary=True,
        has_review=True,
        has_success=False,
        archive_sequence=1,
        traceability_path=tmp_path / "docs/reference/traceability.json",
        traceability_backup=None,
        traceability_payload=None,
    )

    archived_summary = json.loads((target / "task.summary.json").read_text(encoding="utf-8"))
    changed_paths = {item["path"] for item in archived_summary["changedFiles"]}
    assert ".ai/cockpit/task_report.json" in changed_paths
    assert ".ai/cockpit/task_report.md" in changed_paths
    command_evidence = next(
        check["evidence"]
        for check in archived_summary["documentationAlignment"]["checks"]
        if check["area"] == "documentationCommandsCapability"
    )
    assert ".ai/cockpit/task_report.md" in command_evidence

    monkeypatch.setattr(ai_check_status_consistency, "PROJECT_ROOT", tmp_path)
    transaction_paths = {
        ".ai/work-items/archive/index.json",
        ".ai/work-items/starts/task.json",
        ".ai/work-items/archive/2026/task.archive-manifest.json",
        ".ai/work-items/archive/2026/task.contract.json",
        ".ai/work-items/archive/2026/task.summary.json",
        ".ai/work-items/archive/2026/task.outcome.json",
        ".ai/cockpit/task_report.json",
        ".ai/cockpit/task_report.md",
    }
    assert {
        ".ai/cockpit/task_report.json",
        ".ai/cockpit/task_report.md",
    }.issubset(ai_check_status_consistency.transaction_owned_paths(transaction_paths))

    (cockpit / "task_report.md").write_text("tampered\n", encoding="utf-8")
    assert not ai_check_status_consistency.transaction_owned_paths(transaction_paths)
