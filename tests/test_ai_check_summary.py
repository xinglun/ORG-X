import json
from copy import deepcopy

import ai_check_summary
import ai_finish
from ai_common import PROJECT_ROOT

ARCHIVE_SUMMARY = (
    PROJECT_ROOT / "tests" / "fixtures" / "realign_ai_cockpit_v2.summary.json"
)
ALIGNMENT_AREAS = {
    "plan",
    "contractSummaryEvidence",
    "documentationCommandsCapability",
    "multilingualSemantics",
    "limitationsUnknownsHistory",
}


def test_registered_command_matches_only_supported_finish_quality_profiles():
    expected = "make ai-cockpit-quality"

    assert ai_check_summary.registered_command_matches(
        "quality", "make ai-cockpit-quality GOVERNANCE_PROFILE=light", expected
    )
    assert ai_check_summary.registered_command_matches(
        "quality", "make ai-cockpit-quality GOVERNANCE_PROFILE=strict", expected
    )
    assert not ai_check_summary.registered_command_matches(
        "quality", "make ai-cockpit-quality GOVERNANCE_PROFILE=release", expected
    )
    assert not ai_check_summary.registered_command_matches(
        "quality", "make arbitrary-quality GOVERNANCE_PROFILE=light", expected
    )


def aligned_documentation_summary(
    *,
    evidence: str = "docs/contract-fields.md",
    changed_files: list[dict[str, str]] | None = None,
) -> dict:
    changed = changed_files or [{"path": evidence, "reason": "fixture"}]
    return {
        "changedFiles": changed,
        "sourcesUsed": [evidence],
        "documentationAlignment": {
            "schemaVersion": 1,
            "status": "aligned",
            "checkedAt": "2026-07-28T04:00:00+09:00",
            "checks": [
                {
                    "area": area,
                    "status": "aligned",
                    "evidence": [evidence],
                    "reason": f"{area} is aligned in the fixture.",
                }
                for area in sorted(ALIGNMENT_AREAS)
            ],
        },
    }


def test_summary_validator_orchestrates_focused_validation_helpers(monkeypatch):
    monkeypatch.setattr(
        ai_check_summary,
        "_validate_summary_structure",
        lambda *_args, **_kwargs: ["structure"],
    )
    monkeypatch.setattr(
        ai_check_summary,
        "_validate_verification_entries",
        lambda *_args, **_kwargs: ["verification"],
    )
    monkeypatch.setattr(
        ai_check_summary,
        "_validate_summary_metadata",
        lambda *_args, **_kwargs: ["metadata"],
    )
    monkeypatch.setattr(
        ai_check_summary,
        "_validate_required_verification",
        lambda *_args, **_kwargs: ["required"],
    )
    monkeypatch.setattr(
        ai_check_summary,
        "validate_documentation_alignment",
        lambda *_args, **_kwargs: ["documentation"],
    )

    assert ai_check_summary.validate_summary({}, None) == [
        "structure",
        "verification",
        "metadata",
        "documentation",
        "required",
    ]


def test_summary_rejects_missing_evidence_without_matching_forbidden_claim():
    contract = {
        "requestedOperation": {
            "action": "modify",
            "environment": "repository",
        },
        "riskAssessment": {"riskTypes": ["destructive_change"]},
        "governanceProfile": {"selected": "standard"},
        "scope": ["scripts/legacy_api.py"],
        "requiredEvidenceContext": {
            "destructiveLevel": "delete",
            "availableEvidence": ["usage_analysis", "reference_search"],
        },
    }
    summary = {"overclaimPrevention": "Do not report checks that were not verified."}

    issues = ai_check_summary.validate_required_evidence_claims(contract, summary)

    assert issues == [
        (
            "derived missing evidence requires forbidden claim: "
            "Do not claim deletion safety or compatibility preservation."
        )
    ]


def test_documentation_alignment_accepts_complete_source_bound_record():
    summary = aligned_documentation_summary()

    assert ai_check_summary.validate_documentation_alignment(summary) == []


def test_generated_documentation_alignment_completes_bounded_installer_record():
    changed = [
        {
            "path": "tests/fixtures/documentation-alignment-summary-schema-20260728.contract.json",
            "reason": "durable contract fixture",
        },
        {"path": "docs/contract-fields.md", "reason": "Japanese field guide"},
        {
            "path": "docs/reference/ai-cockpit-work-item-lifecycle.md",
            "reason": "English lifecycle guide",
        },
    ]
    summary = {
        "changedFiles": changed,
        "sourcesUsed": [],
        "documentationAlignment": ai_check_summary.complete_generated_documentation_alignment(
            changed
        ),
    }

    assert summary["documentationAlignment"]["status"] == "aligned"
    assert ai_check_summary.validate_documentation_alignment(summary) == []


def test_generated_documentation_alignment_handles_empty_and_multilingual_write_sets():
    empty = ai_check_summary.complete_generated_documentation_alignment([])
    empty_checks = {item["area"]: item for item in empty["checks"]}

    assert empty["status"] == "aligned"
    assert empty_checks["contractSummaryEvidence"]["status"] == "not_applicable"
    assert empty_checks["documentationCommandsCapability"]["status"] == "not_applicable"
    assert empty_checks["multilingualSemantics"]["status"] == "not_applicable"
    assert empty_checks["limitationsUnknownsHistory"]["status"] == "not_applicable"

    changed = [
        {"path": "docs/guide.ja.md", "reason": "Japanese guide"},
        {"path": "docs/guide.zh-CN.md", "reason": "Chinese guide"},
    ]
    multilingual = ai_check_summary.complete_generated_documentation_alignment(changed)
    checks = {item["area"]: item for item in multilingual["checks"]}

    assert checks["contractSummaryEvidence"]["evidence"] == ["docs/guide.ja.md"]
    assert checks["documentationCommandsCapability"]["evidence"] == [
        "docs/guide.ja.md",
        "docs/guide.zh-CN.md",
    ]
    assert checks["multilingualSemantics"]["evidence"] == [
        "docs/guide.ja.md",
        "docs/guide.zh-CN.md",
    ]
    assert checks["limitationsUnknownsHistory"]["evidence"] == ["docs/guide.ja.md"]


def test_documentation_alignment_requires_complete_aligned_domain_set():
    summary = aligned_documentation_summary()
    alignment = summary["documentationAlignment"]
    alignment["status"] = "not_checked"
    alignment["checkedAt"] = None
    alignment["checks"][0]["status"] = "misaligned"
    alignment["checks"].pop()
    alignment["checks"].append(deepcopy(alignment["checks"][0]))

    issues = ai_check_summary.validate_documentation_alignment(summary)

    assert "documentationAlignment.status must be aligned before finish" in issues
    assert "documentationAlignment.checkedAt must be an offset-aware ISO-8601 timestamp" in issues
    assert any("duplicate area" in issue for issue in issues)
    assert any("missing required area" in issue for issue in issues)
    assert any("status must be aligned or not_applicable" in issue for issue in issues)


def test_documentation_alignment_reports_malformed_nested_structure():
    assert ai_check_summary.changed_file_paths({"changedFiles": "invalid"}) == set()
    assert ai_check_summary.validate_documentation_alignment(
        {"documentationAlignment": "invalid"}
    ) == ["documentationAlignment must be an object"]

    summary = {
        "changedFiles": [],
        "sourcesUsed": [],
        "documentationAlignment": {
            "schemaVersion": 1,
            "status": "aligned",
            "checkedAt": "not-a-timestamp",
            "checks": [
                "invalid",
                {
                    "area": "unknown",
                    "status": "aligned",
                    "evidence": [],
                    "extra": True,
                },
                {
                    "area": "plan",
                    "status": "not_applicable",
                    "evidence": ["docs/contract-fields.md", "docs/contract-fields.md"],
                    "reason": "",
                },
                {
                    "area": "contractSummaryEvidence",
                    "status": "aligned",
                    "evidence": "invalid",
                    "reason": "invalid evidence shape",
                },
            ],
        },
    }

    issues = ai_check_summary.validate_documentation_alignment(summary)

    assert any("checkedAt must be an offset-aware" in issue for issue in issues)
    assert any("checks[0] must be an object" in issue for issue in issues)
    assert any(".extra is not a recognized field" in issue for issue in issues)
    assert any(".reason is required" in issue for issue in issues)
    assert any(".area must be one of" in issue for issue in issues)
    assert any("must not contain duplicate paths" in issue for issue in issues)
    assert any("must be empty when not_applicable" in issue for issue in issues)
    assert any("evidence must be a list" in issue for issue in issues)

    non_list = deepcopy(summary)
    non_list["documentationAlignment"]["checks"] = "invalid"
    assert any(
        "documentationAlignment.checks must be a list" in issue
        for issue in ai_check_summary.validate_documentation_alignment(non_list)
    )


def test_documentation_alignment_rejects_untrusted_evidence_paths(tmp_path, monkeypatch):
    summary = aligned_documentation_summary()
    evidence = summary["documentationAlignment"]["checks"][0]["evidence"]

    for invalid, expected in (
        ("/tmp/private.md", "must be repository-relative"),
        ("https://example.invalid/evidence", "must be a repository-relative path, not a URL"),
        ("docs/does-not-exist.md", "does not exist"),
        ("docs/trust-layer.md", "is not declared in changedFiles or sourcesUsed"),
    ):
        evidence[0] = invalid
        issues = ai_check_summary.validate_documentation_alignment(summary)
        assert any(expected in issue for issue in issues), (invalid, issues)

    local_only = "target/local-closure-receipt.md"
    (tmp_path / local_only).parent.mkdir()
    (tmp_path / local_only).write_text("local receipt", encoding="utf-8")
    monkeypatch.setattr(ai_check_summary, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_summary, "_is_git_tracked_repository_path", lambda _path: False)
    evidence[0] = local_only

    issues = ai_check_summary.validate_documentation_alignment(summary)

    assert any(
        "must be a Git-tracked repository file or an active Contract-scoped file" in issue
        for issue in issues
    )


def test_documentation_alignment_accepts_untracked_active_contract_scope(tmp_path, monkeypatch):
    evidence = ".ai/cockpit/README.md"
    (tmp_path / evidence).parent.mkdir(parents=True)
    (tmp_path / evidence).write_text("runtime documentation", encoding="utf-8")
    monkeypatch.setattr(ai_check_summary, "PROJECT_ROOT", tmp_path)
    monkeypatch.setattr(ai_check_summary, "_is_git_tracked_repository_path", lambda _path: False)

    assert (
        ai_check_summary.validate_documentation_alignment(
            aligned_documentation_summary(evidence=evidence),
            {"contractVersion": 2, "scope": [".ai/cockpit/**"]},
        )
        == []
    )


def test_documentation_alignment_reverse_maps_changed_documentation_surfaces():
    summary = aligned_documentation_summary(
        evidence="scripts/ai_check_summary.py",
        changed_files=[
            {"path": "scripts/ai_check_summary.py", "reason": "validator"},
            {"path": "docs/contract-fields.md", "reason": "field semantics"},
            {"path": "Makefile", "reason": "command surface"},
        ],
    )
    summary["sourcesUsed"].extend(["docs/contract-fields.md", "Makefile"])

    issues = ai_check_summary.validate_documentation_alignment(summary)

    assert (
        "documentationAlignment evidence is missing changed documentation/command surface: "
        "docs/contract-fields.md"
    ) in issues
    assert (
        "documentationAlignment evidence is missing changed documentation/command surface: Makefile"
    ) in issues


def test_documentation_alignment_is_required_for_active_v2_but_not_legacy_archive():
    summary = aligned_documentation_summary()
    summary.pop("documentationAlignment")

    active_issues = ai_check_summary._validate_summary_structure(
        summary,
        {"contractVersion": 2},
        contract_path="",
        summary_path="",
        legacy_archive=False,
    )
    archive_issues = ai_check_summary.validate_documentation_alignment(summary, legacy_archive=True)

    assert "missing field: documentationAlignment" in active_issues
    assert archive_issues == []


def test_summary_residual_risk_rejects_generated_skeleton_text():
    summary = {
        "summaryVersion": 2,
        "residualRisks": [
            {
                "level": "medium",
                "area": "scope",
                "detail": "Initial skeleton; replace with actual residual risks before finishing.",
            }
        ],
    }

    issues = ai_check_summary.validate_residual_risk_semantics(summary)

    assert issues == ["residualRisks[0].detail contains generated placeholder text"]


def test_summary_residual_risk_accepts_concrete_detail_and_preserves_legacy_archive():
    concrete = {
        "summaryVersion": 2,
        "residualRisks": [
            {
                "level": "medium",
                "area": "historical-evidence",
                "detail": "Historical archive evidence remains immutable and is not rewritten.",
            }
        ],
    }
    legacy = {
        "summaryVersion": 2,
        "residualRisks": [
            {
                "level": "medium",
                "area": "scope",
                "detail": "Initial skeleton; replace with actual residual risks before finishing.",
            }
        ],
    }

    assert ai_check_summary.validate_residual_risk_semantics(concrete) == []
    assert (
        ai_check_summary.validate_residual_risk_semantics(
            legacy,
            legacy_archive=True,
            summary_path=".ai/work-items/archive/2026/old.summary.json",
        )
        == []
    )


def test_required_verification_does_not_self_block_ai_summary():
    contract = {
        "verification": [
            {"check": "aiSummary", "required": True},
            {"check": "aiWorkItem", "required": True},
        ]
    }
    summary = {"verification": [{"check": "aiWorkItem", "result": "passed"}]}

    assert ai_check_summary._validate_required_verification(summary, contract) == []


def test_receipt_binding_exempts_receipt_from_changed_file_diff(monkeypatch):
    summary = {"changedFiles": []}
    contract = {
        "scope": ["scripts/ai_start.py"],
        "startReceipt": {"path": ".ai/work-items/starts/task.json"},
    }
    monkeypatch.setattr(
        ai_check_summary,
        "changed_paths",
        lambda _contract: [".ai/work-items/starts/task.json"],
    )
    assert ai_check_summary.validate_changed_files_cover_diff(summary, contract) == []


def test_passed_v2_evidence_requires_worktree_digest():
    item = ai_finish.evidence(
        "projectTest",
        "make ai-cockpit-project-test",
        0,
        1,
        "passed\n",
        contract_hash="b" * 64,
        commit_sha="a" * 40,
        execution_contract_path=".ai/work-items/active/task.contract.json",
        execution_summary_path=".ai/work-items/active/task.summary.json",
        worktree_digest="c" * 64,
    )
    item.pop("worktreeDigest")
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/ai_check_summary.py", "reason": "fixture"}],
        "verification": [item],
        "risk": {"level": "low", "detail": "fixture"},
    }
    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "projectTest", "required": True}],
        },
    )
    assert "verification[0].worktreeDigest is required for passed result" in issues


def test_hosted_performance_evidence_requires_registered_structured_shape():
    summary = {
        "hostedPerformanceEvidence": {
            "schemaVersion": 1,
            "status": "not_run",
            "baselineWorkItem": "wi-20",
            "comparisonRule": "No improvement claim without comparable source-bound runs.",
            "scenarios": [
                {
                    "scenario": "pull_request_quality_gate",
                    "status": "not_run",
                    "reason": "No comparable hosted run.",
                    "evidence": [],
                }
            ],
        }
    }
    assert ai_check_summary.validate_hosted_performance_evidence(summary) == []


def test_hosted_performance_evidence_rejects_missing_not_run_reason():
    summary = {
        "hostedPerformanceEvidence": {
            "schemaVersion": 1,
            "status": "not_run",
            "baselineWorkItem": "wi-20",
            "comparisonRule": "No improvement claim without comparable source-bound runs.",
            "scenarios": [
                {"scenario": "pull_request_quality_gate", "status": "not_run", "evidence": []}
            ],
        }
    }
    assert "hostedPerformanceEvidence.scenarios[0].reason is required" in (
        ai_check_summary.validate_hosted_performance_evidence(summary)
    )


def test_hosted_performance_evidence_rejects_invalid_root_and_scenarios():
    assert ai_check_summary.validate_hosted_performance_evidence(
        {"hostedPerformanceEvidence": "legacy prose"}
    ) == ["hostedPerformanceEvidence must be an object"]
    issues = ai_check_summary.validate_hosted_performance_evidence(
        {
            "hostedPerformanceEvidence": {
                "schemaVersion": 2,
                "status": "wrong",
                "baselineWorkItem": "",
                "comparisonRule": "",
                "scenarios": "not-a-list",
            }
        }
    )
    assert "hostedPerformanceEvidence.schemaVersion must be 1" in issues
    assert "hostedPerformanceEvidence.scenarios must be a non-empty list" in issues


def test_legacy_archive_summary_without_v2_fields_remains_readable():
    item = ai_finish.evidence(
        "projectTest",
        "make ai-cockpit-project-test",
        0,
        1,
        "passed\n",
        contract_hash="b" * 64,
        commit_sha="a" * 40,
        execution_contract_path=".ai/work-items/archive/2026/task.contract.json",
        execution_summary_path=".ai/work-items/archive/2026/task.summary.json",
        worktree_digest="c" * 64,
    )
    item.pop("worktreeDigest")
    summary = {
        "workItemId": "task",
        "contractPath": ".ai/work-items/archive/2026/task.contract.json",
        "changedFiles": [{"path": "scripts/ai_check_summary.py", "reason": "fixture"}],
        "sourcesUsed": ["fixture"],
        "verification": [item],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }
    issues = ai_check_summary.validate_summary(
        summary,
        {"contractVersion": 2, "workItemId": "task"},
        legacy_archive=True,
    )
    assert issues == []


def test_intent_alignment_validator_accepts_empty_and_partial_payloads():
    assert ai_check_summary.validate_intent_alignment({"intentAlignment": {}}) == []
    assert ai_check_summary.validate_intent_alignment({"intentAlignment": None}) == []
    assert (
        ai_check_summary.validate_intent_alignment({"intentAlignment": {"problemResolved": True}})
        == []
    )
    assert (
        ai_check_summary.validate_intent_alignment(
            {"intentAlignment": {"problemResolutionEvidence": "legacy evidence text"}}
        )
        == []
    )
    assert (
        ai_check_summary.validate_intent_alignment(
            {"intentAlignment": {"constraintsRespectEvidence": "legacy evidence text"}}
        )
        == []
    )


def test_intent_alignment_validator_accepts_legacy_archive_payload():
    archive_summary = json.loads(ARCHIVE_SUMMARY.read_text(encoding="utf-8"))
    assert (
        ai_check_summary.validate_intent_alignment(
            {"intentAlignment": archive_summary["intentAlignment"]}
        )
        == []
    )


def test_intent_alignment_validator_rejects_unknown_keys():
    issues = ai_check_summary.validate_intent_alignment(
        {"intentAlignment": {"problemResolved": True, "unknownKey": False}}
    )
    assert "intentAlignment.unknownKey is not a recognized field" in issues


def test_active_summary_rejects_a_predicted_archive_sequence():
    issues = ai_check_summary._validate_summary_metadata(
        {
            "risk": {"level": "medium", "detail": "fixture"},
            "knownGaps": ["Archive sequence 626 and hosted CI remain pending."],
        }
    )

    assert (
        "knownGaps must not predict a numeric archive sequence before the generator allocates it"
        in issues
    )


def test_summary_accepts_nonnumeric_or_generator_owned_archive_sequence():
    for summary in (
        {
            "risk": {"level": "medium", "detail": "fixture"},
            "knownGaps": ["The next archive sequence and hosted CI remain pending."],
        },
        {
            "risk": {"level": "medium", "detail": "fixture"},
            "knownGaps": ["Archive sequence 625 was generated."],
            "archiveSequence": 625,
        },
    ):
        assert (
            "knownGaps must not predict a numeric archive sequence before the generator allocates it"
            not in ai_check_summary._validate_summary_metadata(summary)
        )


def test_summary_metadata_rejects_malformed_non_risk_explanations():
    issues = ai_check_summary._validate_summary_metadata(
        {
            "risk": {"level": "medium", "detail": "fixture"},
            "nonRiskExplanations": [
                {
                    "sourceWarning": "Hosted verification is not required.",
                    "reason": "Contract does not require it.",
                    "evidence": "not-a-list",
                }
            ],
        }
    )

    assert "nonRiskExplanations[0].evidence must be a list" in issues


def test_scenario_coverage_validator_accepts_valid_payload():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "src/app.py", "reason": "fixture"}],
        "sourcesUsed": ["spec"],
        "scenarioCoverage": [
            {
                "scenario": "example verified scenario",
                "required": True,
                "status": "verified",
                "evidence": ["make example-check"],
            },
            {
                "scenario": "example unverified scenario",
                "required": True,
                "status": "unverified",
                "evidence": [],
                "reason": "Waiting on an external run.",
            },
            {
                "scenario": "example not applicable scenario",
                "required": False,
                "status": "not_applicable",
                "evidence": [],
                "reason": "Legacy path not touched.",
            },
        ],
        "verification": [{"check": "quality", "result": "not_run"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "reviewReadiness": {"status": "ready", "reason": "fixture", "expectedReviewFocus": []},
        "boundaryChecks": {
            "runtimeEntrypoints": "not_applicable",
            "userVisibleOutput": "not_applicable",
            "persistence": "not_applicable",
            "localization": "not_applicable",
            "generatedArtifacts": "not_applicable",
            "makeEntrypoints": "not_applicable",
        },
        "knownGaps": [],
        "overclaimPrevention": "fixture",
        "documentationAlignment": aligned_documentation_summary(
            evidence="scripts/ai_check_summary.py",
            changed_files=[{"path": "src/app.py", "reason": "fixture"}],
        )["documentationAlignment"],
    }
    summary["sourcesUsed"].append("scripts/ai_check_summary.py")

    assert (
        ai_check_summary.validate_summary(summary, {"workItemId": "task", "contractVersion": 2})
        == []
    )


def test_scenario_coverage_validator_rejects_invalid_required_entries():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "src/app.py", "reason": "fixture"}],
        "sourcesUsed": ["spec"],
        "scenarioCoverage": [
            {
                "scenario": "example verified scenario",
                "required": True,
                "status": "verified",
                "evidence": [],
            },
            {
                "scenario": "example not applicable scenario",
                "required": True,
                "status": "not_applicable",
                "evidence": [],
            },
        ],
        "verification": [{"check": "quality", "result": "not_run"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "reviewReadiness": {"status": "ready", "reason": "fixture", "expectedReviewFocus": []},
        "boundaryChecks": {
            "runtimeEntrypoints": "not_applicable",
            "userVisibleOutput": "not_applicable",
            "persistence": "not_applicable",
            "localization": "not_applicable",
            "generatedArtifacts": "not_applicable",
            "makeEntrypoints": "not_applicable",
        },
        "knownGaps": [],
        "overclaimPrevention": "fixture",
    }

    issues = ai_check_summary.validate_summary(
        summary, {"workItemId": "task", "contractVersion": 2}
    )
    assert (
        "scenarioCoverage[0].evidence must contain at least one item when status is verified"
        in issues
    )
    assert "scenarioCoverage[1].reason is required when status is not_applicable" in issues


def test_summary_validator_rejects_summary_filename_mismatch():
    summary = {
        "summaryVersion": 2,
        "workItemId": "wrong",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }

    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "wrong",
            "verification": [{"check": "quality", "required": True}],
        },
        summary_path=".ai/work-items/active/right.summary.json",
    )

    assert "workItemId does not match the Summary filename" in issues


def test_summary_validator_rejects_contract_path_mismatch():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/archive/2026/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }

    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "quality", "required": True}],
        },
        contract_path=".ai/work-items/active/task.contract.json",
        summary_path=".ai/work-items/active/task.summary.json",
    )

    assert "contractPath does not match the Contract path" in issues


def test_summary_validator_rejects_unknown_active_fields():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }
    summary["unexpectedField"] = True

    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "quality", "required": True}],
        },
        summary_path=".ai/work-items/active/task.summary.json",
    )

    assert "unknown field: unexpectedField" in issues


def test_summary_validator_accepts_structured_preflight_decision_evidence():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "decisionEvidence": {
            "decisionId": "HD-test",
            "decision": "A",
            "workItemId": "task",
            "contractHash": "a" * 16,
            "preflightHash": "b" * 16,
            "recordedAt": "2026-07-24T00:00:00Z",
            "recordedBy": "user",
        },
    }
    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "quality", "required": True}],
        },
        summary_path=".ai/work-items/active/task.summary.json",
    )
    assert not any("unknown field: decisionEvidence" in issue for issue in issues)


def test_summary_validator_accepts_optional_task_outcome_input():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "taskOutcomeInput": "target/task.evidence.json",
    }
    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "quality", "required": True}],
        },
        summary_path=".ai/work-items/active/task.summary.json",
    )
    assert not any("taskOutcomeInput" in issue for issue in issues)


def test_summary_validator_accepts_mandatory_task_outcome_state():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [
            {"path": "scripts/app.py", "reason": "changed"},
            {"path": ".ai/work-items/active/task.outcome.json", "reason": "Outcome"},
            {"path": ".ai/work-items/active/task.outcome.md", "reason": "Outcome"},
        ],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "passed"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "taskOutcome": {
            "status": "completed",
            "jsonPath": ".ai/work-items/active/task.outcome.json",
            "markdownPath": ".ai/work-items/active/task.outcome.md",
            "rawEvidencePath": "derived:pre_merge",
            "evidenceCount": 2,
        },
    }
    issues = ai_check_summary.validate_summary(
        summary,
        {
            "contractVersion": 2,
            "workItemId": "task",
            "verification": [{"check": "quality", "required": True}],
        },
        summary_path=".ai/work-items/active/task.summary.json",
    )

    assert not any("taskOutcome" in issue for issue in issues)


def test_summary_validator_accepts_positive_archive_sequence_and_rejects_invalid_value():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/archive/2026/task.contract.json",
        "changedFiles": [{"path": "scripts/app.py", "reason": "changed"}],
        "sourcesUsed": ["spec"],
        "verification": [{"check": "quality", "result": "not_run"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "archiveSequence": 3,
    }
    contract = {"contractVersion": 2, "workItemId": "task", "verification": []}

    assert ai_check_summary.validate_summary(summary, contract, legacy_archive=True) == []
    summary["archiveSequence"] = 0
    issues = ai_check_summary.validate_summary(summary, contract, legacy_archive=True)
    assert "archiveSequence must be a positive integer when present" in issues


def test_summary_validator_requires_v2_acceptance_evidence_mapping():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "tests/test_acceptance_policy.py", "reason": "fixture"}],
        "sourcesUsed": ["fixture"],
        "verification": [{"check": "quality", "result": "not_run"}],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
    }
    contract = {
        "contractVersion": 2,
        "workItemId": "task",
        "acceptance": ["A1: behavior is mapped"],
        "verification": [],
    }

    issues = ai_check_summary.validate_summary(summary, contract)

    assert "summary.acceptanceEvidence must be a list" in issues


def implementation_approach_fixture(*, approach_type: str = "implementation") -> dict:
    evidence = [{"source": "scripts/ai_check_summary.py", "subject": "implementation path"}]
    return {
        "approachType": approach_type,
        "status": "complete",
        "summary": {
            "text": "The change records the governed path for producing the review result.",
            "status": "verified",
            "evidence": evidence,
        },
        "mechanism": {
            "text": "The Summary record is projected into Outcome and Human Report views.",
            "status": "verified",
            "evidence": evidence,
        },
        "affectedComponents": [
            {
                "component": "Summary to Outcome projection",
                "detail": "The approach is carried as structured evidence.",
                "status": "verified",
                "evidence": evidence,
            }
        ],
        "designDecisions": [
            {
                "decision": "Keep Summary as the source of truth.",
                "reason": "Projections stay deterministic and reviewable.",
                "status": "verified",
                "evidence": evidence,
            }
        ],
        "technicalDetails": [
            {
                "topic": "Evidence binding",
                "detail": "Each verified statement points to an existing repository reference.",
                "status": "verified",
                "evidence": evidence,
            }
        ],
        "evidence": [
            {
                "claim": "The projection path is represented in repository code.",
                "status": "verified",
                "source": "scripts/ai_check_summary.py",
                "subject": "implementation path",
            }
        ],
    }


def test_summary_structure_accepts_implementation_approach_as_a_known_field():
    summary = {
        "summaryVersion": 2,
        "workItemId": "task",
        "contractPath": ".ai/work-items/active/task.contract.json",
        "changedFiles": [{"path": "scripts/example.py", "reason": "fixture"}],
        "sourcesUsed": ["scripts/example.py"],
        "verification": [],
        "unknownsRemaining": [],
        "risk": {"level": "low", "detail": "fixture"},
        "generatedFiles": [],
        "destructiveChanges": [],
        "observedIssues": [],
        "implementationApproach": implementation_approach_fixture(),
    }
    contract = {"contractVersion": 2, "workItemId": "task"}

    issues = ai_check_summary._validate_summary_structure(
        summary,
        contract,
        contract_path=".ai/work-items/active/task.contract.json",
        summary_path=".ai/work-items/active/task.summary.json",
        legacy_archive=False,
    )

    assert not any("implementationApproach" in issue for issue in issues)


def test_implementation_approach_assessment_requires_code_and_keeps_missing_data_yellow():
    assessor = getattr(ai_check_summary, "assess_implementation_approach", None)
    assert callable(assessor)

    assessment = assessor(
        {"changedFiles": [{"path": "scripts/example.py", "reason": "fixture"}]},
        {"scope": ["scripts/example.py"]},
    )

    assert assessment["status"] == "incomplete"
    assert assessment["humanStatusColor"] == "yellow"
    assert assessment["warnings"]


def test_configuration_scope_accepts_configuration_approach_without_security_red():
    assessor = getattr(ai_check_summary, "assess_implementation_approach", None)
    assert callable(assessor)

    assessment = assessor(
        {
            "changedFiles": [{"path": "config/settings.yaml", "reason": "fixture"}],
            "configurationApproach": implementation_approach_fixture(approach_type="configuration"),
        },
        {"scope": ["config/settings.yaml"]},
    )

    assert assessment["status"] == "complete"
    assert assessment["requiredField"] == "configurationApproach"


def test_adoption_bootstrap_does_not_require_product_implementation_approach():
    assessor = getattr(ai_check_summary, "assess_implementation_approach", None)
    assert callable(assessor)

    assessment = assessor(
        {"changedFiles": [{"path": "scripts/ai_finish.py", "reason": "installer bootstrap"}]},
        {
            "workItemId": "adopt_ai_cockpit",
            "scope": ["scripts/ai_*.py", "scripts/bootstrap_*.py"],
            "adoptionBootstrapPaths": ["scripts/ai_*.py", "scripts/bootstrap_*.py"],
        },
    )

    assert assessment["status"] == "not_applicable"
    assert assessment["humanStatusColor"] == "unknown"


def test_verified_approach_requires_an_existing_repository_evidence_path():
    approach = implementation_approach_fixture()
    approach["summary"]["evidence"] = [
        {"source": "scripts/does_not_exist.py", "subject": "missing implementation"}
    ]

    issues = ai_check_summary.validate_implementation_approach(
        approach, {"scope": ["scripts/ai_check_summary.py"]}
    )

    assert any("does not exist" in issue for issue in issues)
    assert any("verified claims require" in issue for issue in issues)
