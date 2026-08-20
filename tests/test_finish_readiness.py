import ai_finish


def test_checkpoint_recovery_guidance_uses_before_finish_for_a_missing_finish_stage():
    guidance = ai_finish.checkpoint_recovery_guidance(
        ["missing checkpointEvidence for required stage(s): before_finish"],
        contract=".ai/work-items/active/example.contract.json",
        summary=".ai/work-items/active/example.summary.json",
    )

    assert "make ai-checkpoint" in guidance
    assert "STAGE=before_finish" in guidance
    assert "ai-revalidate-contract-amendment" not in guidance


def test_checkpoint_recovery_guidance_uses_append_only_amendment_for_stale_before_edit():
    guidance = ai_finish.checkpoint_recovery_guidance(
        ["missing contract_amendment_revalidation for stale before_edit Contract"],
        contract=".ai/work-items/active/example.contract.json",
        summary=".ai/work-items/active/example.summary.json",
    )

    assert "make ai-revalidate-contract-amendment" in guidance
    assert "PREVIOUS_CONTRACT_HASH=<immutable-before-edit-hash>" in guidance


def test_checkpoint_recovery_guidance_keeps_unknown_validation_failures_non_bypassing():
    guidance = ai_finish.checkpoint_recovery_guidance(
        ["checkpointEvidence[manual].contractHash is malformed"],
        contract=".ai/work-items/active/example.contract.json",
        summary=".ai/work-items/active/example.summary.json",
    )

    assert "inspect every reported checkpoint issue" in guidance
    assert "preserve active evidence" in guidance
    assert "ai-revalidate-contract-amendment" not in guidance
    assert "STAGE=before_finish" not in guidance


def summary(*, verification="passed", unknowns=None, residual_risks=None):
    return {
        "verification": [
            {"check": "quality", "result": verification},
        ],
        "unknownsRemaining": [] if unknowns is None else unknowns,
        "residualRisks": [] if residual_risks is None else residual_risks,
        "reviewReadiness": {
            "status": "not_ready",
            "reason": "Initial skeleton.",
            "expectedReviewFocus": ["review"],
        },
    }


def test_promote_review_readiness_marks_fully_verified_summary_ready():
    result = ai_finish.promote_review_readiness(summary())

    assert result["status"] == "ready"
    assert "required verification" in result["reason"]
    assert result["expectedReviewFocus"] == ["review"]


def test_promote_review_readiness_preserves_residual_risk_signal():
    result = ai_finish.promote_review_readiness(
        summary(residual_risks=[{"level": "medium", "area": "review", "detail": "focus"}])
    )

    assert result["status"] == "ready_with_risks"
    assert "residual risk" in result["reason"]


def test_promote_review_readiness_remains_not_ready_for_incomplete_evidence():
    failed = ai_finish.promote_review_readiness(summary(verification="failed"))
    unknown = ai_finish.promote_review_readiness(summary(unknowns=["external review"]))

    assert failed["status"] == "not_ready"
    assert unknown["status"] == "not_ready"


def test_promote_review_readiness_allows_only_contract_optional_not_run_checks():
    candidate = summary()
    candidate["verification"] = [
        {"check": "scope", "result": "passed"},
        {"check": "quality", "result": "not_run"},
    ]
    contract = {
        "contractVersion": 2,
        "acceptance": ["Optional verification is declared in the Contract."],
        "verification": [
            {"check": "scope", "required": True},
            {"check": "quality", "required": False},
        ],
    }

    result = ai_finish.promote_review_readiness(candidate, contract)

    assert result["status"] == "ready"


def test_promote_review_readiness_requires_acceptance_evidence_for_v2():
    result = ai_finish.promote_review_readiness(
        summary(),
        {
            "contractVersion": 2,
            "acceptance": ["A1: behavior is mapped"],
            "riskAssessment": {"level": "low"},
        },
    )

    assert result["status"] == "not_ready"
    assert "Acceptance evidence" in result["reason"]


def test_finish_archive_message_is_not_lifecycle_closure():
    output = ai_finish.archive_next_steps("example")

    assert "lifecycle is not closed" in output
    assert "make ai-close-work-item TASK=example" in output


def test_finish_quality_paths_excludes_only_current_generated_evidence(monkeypatch):
    contract = {"workItemId": "example"}
    monkeypatch.setattr(
        ai_finish,
        "changed_paths",
        lambda _contract: [
            "docs/guide.md",
            ".ai/cockpit/current_status.md",
            ".ai/work-items/active/example.summary.json",
            ".ai/guards/policy.yaml",
        ],
    )

    assert ai_finish.finish_quality_paths(contract) == ["docs/guide.md", ".ai/guards/policy.yaml"]


def test_archive_reuse_requires_a_same_state_final_summary_attestation(monkeypatch):
    contract = {"scope": ["scripts/ai_finish.py"]}
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: contract["scope"])
    contract_hash = "contract-hash"
    commit_sha = "commit-sha"
    contract_path = ".ai/work-items/active/example.contract.json"
    summary_path = ".ai/work-items/active/example.summary.json"
    digest = ai_finish.worktree_digest_for_finish(contract["scope"], summary_path)
    evidence = {
        "check": "aiSummary",
        "result": "passed",
        "runner": "ai_finish",
        "contractHash": contract_hash,
        "commitSha": commit_sha,
        "executionContractPath": contract_path,
        "executionSummaryPath": summary_path,
        "worktreeDigest": digest,
    }
    summary_data = {"verification": [evidence]}
    evidence["outcomeInputDigest"] = ai_finish.outcome_input_digest(summary_data)

    assert ai_finish.reusable_archive_verification(
        summary_data,
        contract,
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        contract=contract_path,
        summary_path=summary_path,
    )

    evidence["commitSha"] = "stale-commit"
    assert not ai_finish.reusable_archive_verification(
        summary_data,
        contract,
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        contract=contract_path,
        summary_path=summary_path,
    )


def test_archive_reuse_rejects_changed_outcome_inputs_after_summary_attestation(monkeypatch):
    contract = {"scope": ["scripts/ai_finish.py"]}
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: contract["scope"])
    contract_hash = "contract-hash"
    commit_sha = "commit-sha"
    contract_path = ".ai/work-items/active/example.contract.json"
    summary_path = ".ai/work-items/active/example.summary.json"
    summary_data = {
        "changedFiles": [],
        "knownGaps": [],
        "nonRiskExplanations": [],
        "userCorrectionsCaptured": [],
        "verification": [
            {
                "check": "aiSummary",
                "result": "passed",
                "runner": "ai_finish",
                "contractHash": contract_hash,
                "commitSha": commit_sha,
                "executionContractPath": contract_path,
                "executionSummaryPath": summary_path,
                "worktreeDigest": ai_finish.worktree_digest_for_finish(
                    contract["scope"], summary_path
                ),
            }
        ],
    }
    summary_data["verification"][0]["outcomeInputDigest"] = ai_finish.outcome_input_digest(
        summary_data
    )

    summary_data["knownGaps"] = ["A previously generated warning is now stale."]

    assert not ai_finish.reusable_archive_verification(
        summary_data,
        contract,
        contract_hash=contract_hash,
        commit_sha=commit_sha,
        contract=contract_path,
        summary_path=summary_path,
    )


def test_archive_reuse_rejects_malformed_verification_evidence(monkeypatch):
    contract = {"scope": ["scripts/ai_finish.py"]}
    monkeypatch.setattr(ai_finish, "changed_paths", lambda _contract: contract["scope"])

    assert not ai_finish.reusable_archive_verification(
        {"verification": {"check": "aiSummary", "result": "passed"}},
        contract,
        contract_hash="contract-hash",
        commit_sha="commit-sha",
        contract=".ai/work-items/active/example.contract.json",
        summary_path=".ai/work-items/active/example.summary.json",
    )


def test_promote_review_readiness_does_not_override_failed_stabilization_evidence():
    result = ai_finish.promote_review_readiness(
        summary(verification="failed"),
        {"contractVersion": 2, "acceptance": []},
    )

    assert result["status"] == "not_ready"
