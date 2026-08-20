#!/usr/bin/env python3
"""Validate an AI Change Summary against a Work Item Contract."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess  # nosec B404 - used only for fixed list-form Git tracking interrogation
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from ai_acceptance_policy import validate_acceptance_evidence
from ai_common import (
    PROJECT_ROOT,
    changed_paths,
    contains_machine_path,
    included,
    load_json,
    non_empty_string,
    render_check_command,
    simple_yaml_lists,
    validate_scenario_coverage,
    verification_key,
)
from ai_observability import create_observability, elapsed_ms
from ai_required_evidence import EvidenceContext, derive_required_evidence

SCOPE_POLICY = PROJECT_ROOT / ".ai" / "guards" / "scope_policy.yaml"
REQUIRED_FIELDS = (
    "workItemId",
    "contractPath",
    "changedFiles",
    "sourcesUsed",
    "verification",
    "unknownsRemaining",
    "risk",
    "generatedFiles",
    "destructiveChanges",
    "observedIssues",
    "documentationAlignment",
)
ALLOWED_FIELDS = set(REQUIRED_FIELDS) | {
    "archiveSequence",
    "acceptanceEvidence",
    "boundaryChecks",
    "checkpointEvidence",
    "checkpointReview",
    "followUps",
    "generatedFiles",
    "guidelinesCompliance",
    "knownGaps",
    "nonRiskExplanations",
    "issuesObserved",
    "overclaimPrevention",
    "residualRisks",
    "reviewReadiness",
    "rollbackEvidence",
    "scenarioCoverage",
    "sourcesUsed",
    "summaryVersion",
    "title",
    "unverifiedScenarios",
    "userCorrectionSolidification",
    "userCorrectionsCaptured",
    "ownershipDecisions",
    "intentAlignment",
    "decisionEvidence",
    "taskOutcomeInput",
    "taskOutcome",
    "hostedPerformanceEvidence",
    "documentationAlignment",
    "verificationHistory",
    "implementationApproach",
    "configurationApproach",
}
RESULTS = {"passed", "failed", "not_run"}
RISK_LEVELS = {"low", "medium", "high"}
REVIEW_READINESS_STATUSES = {"not_ready", "ready", "ready_with_risks", "blocked"}
INTENT_ALIGNMENT_BOOL_KEYS = {"problemResolved", "constraintsRespected", "nonGoalsAvoided"}
INTENT_ALIGNMENT_STRING_KEYS = {"rationaleValidated"}
RESIDUAL_RISK_PLACEHOLDER_MARKERS = (
    "initial skeleton",
    "replace this",
    "replace with actual residual risks",
)
DOCUMENTATION_ALIGNMENT_AREAS = (
    "plan",
    "contractSummaryEvidence",
    "documentationCommandsCapability",
    "multilingualSemantics",
    "limitationsUnknownsHistory",
)
DOCUMENTATION_ALIGNMENT_ROOT_FIELDS = {"schemaVersion", "status", "checkedAt", "checks"}
DOCUMENTATION_ALIGNMENT_CHECK_FIELDS = {"area", "status", "evidence", "reason"}
CODE_SUFFIXES = {
    ".c",
    ".cc",
    ".cpp",
    ".cs",
    ".go",
    ".java",
    ".js",
    ".jsx",
    ".kt",
    ".php",
    ".py",
    ".rb",
    ".rs",
    ".sh",
    ".swift",
    ".ts",
    ".tsx",
}
CONFIG_SUFFIXES = {".cfg", ".conf", ".ini", ".properties", ".toml", ".yaml", ".yml"}

APPROACH_STATUSES = {"complete", "incomplete", "not_applicable"}
CLAIM_STATUSES = {"verified", "unverified", "unknown"}
APPROACH_TYPES = {"implementation", "configuration"}
TOP_LEVEL_APPROACH_FIELDS = {
    "approachType",
    "status",
    "summary",
    "mechanism",
    "affectedComponents",
    "designDecisions",
    "technicalDetails",
    "evidence",
}
CLAIM_FIELDS = {"text", "status", "evidence"}
COMPONENT_FIELDS = {"component", "detail", "status", "evidence"}
DECISION_FIELDS = {"decision", "reason", "status", "evidence"}
DETAIL_FIELDS = {"topic", "detail", "status", "evidence"}
REFERENCE_FIELDS = {"source", "subject", "digest"}
EVIDENCE_FIELDS = {"claim", "status", "source", "subject", "digest"}
FORBIDDEN_APPROACH_KEYS = {
    "agentNotes",
    "chainOfThought",
    "chain_of_thought",
    "internalNotes",
    "operationLog",
    "operation_log",
    "reasoning",
    "thoughts",
    "verboseLog",
}
FORBIDDEN_APPROACH_KEY_PATTERN = re.compile(
    r"(?:thought|reasoning|operation.?log|agent.?notes)", re.IGNORECASE
)
HEX_DIGEST = re.compile(r"^[a-f0-9]{64}$")


def _approach_reference_is_real(value: Any, contract: dict[str, Any] | None = None) -> bool:
    """Return True only for an evidence reference bound to a repository file."""

    if not isinstance(value, dict) or not non_empty_string(value.get("source")):
        return False
    return _valid_repository_evidence_path(value["source"], contract) is None


def _validate_approach_reference(
    value: Any, path: str, contract: dict[str, Any] | None = None
) -> list[str]:
    if not isinstance(value, dict):
        return [f"{path} must be an evidence reference object"]
    issues = [f"{path}.{key} is not allowed" for key in value if key not in REFERENCE_FIELDS]
    for key in ("source", "subject"):
        if not non_empty_string(value.get(key)):
            issues.append(f"{path}.{key} must be a non-empty string")
    source = value.get("source")
    if isinstance(source, str) and source.strip():
        path_issue = _valid_repository_evidence_path(source, contract)
        if path_issue:
            issues.append(f"{path}.source {source!r} {path_issue}")
    digest = value.get("digest")
    if digest is not None and (not isinstance(digest, str) or not HEX_DIGEST.fullmatch(digest)):
        issues.append(f"{path}.digest must be a SHA-256 hex digest when provided")
    return issues


def _validate_approach_evidence(
    value: Any, path: str, contract: dict[str, Any] | None = None
) -> list[str]:
    if not isinstance(value, list):
        return [f"{path} must be a list"]
    issues: list[str] = []
    if len(value) > 20:
        issues.append(f"{path} must not contain verbose operation-log entries")
    for index, item in enumerate(value):
        issues.extend(_validate_approach_reference(item, f"{path}[{index}]", contract))
    return issues


def _has_real_approach_evidence(value: Any, contract: dict[str, Any] | None = None) -> bool:
    return isinstance(value, list) and any(
        _approach_reference_is_real(item, contract) for item in value
    )


def _validate_approach_claim(
    value: Any,
    path: str,
    fields: set[str],
    contract: dict[str, Any] | None = None,
) -> list[str]:
    if not isinstance(value, dict):
        return [f"{path} must be an object"]
    issues = [f"{path}.{key} is not allowed" for key in value if key not in fields]
    if "text" in fields and not non_empty_string(value.get("text")):
        issues.append(f"{path}.text must be a non-empty string")
    for key in ("component", "detail", "decision", "reason", "topic"):
        if key in fields and key in value and not non_empty_string(value.get(key)):
            issues.append(f"{path}.{key} must be a non-empty string")
    status = value.get("status")
    if status not in CLAIM_STATUSES:
        issues.append(f"{path}.status must be one of {sorted(CLAIM_STATUSES)}")
    evidence = value.get("evidence")
    issues.extend(_validate_approach_evidence(evidence, f"{path}.evidence", contract))
    if status == "verified" and not _has_real_approach_evidence(evidence, contract):
        issues.append(
            f"{path} verified claims require at least one existing repository evidence path"
        )
    for key, child in value.items():
        if key in FORBIDDEN_APPROACH_KEYS or FORBIDDEN_APPROACH_KEY_PATTERN.search(str(key)):
            issues.append(f"{path}.{key} is not allowed in an Implementation Approach")
        if isinstance(child, str) and len(child) > 2000:
            issues.append(f"{path}.{key} is too long for a concise knowledge record")
    return issues


def _validate_approach_global_evidence(
    value: Any, path: str, contract: dict[str, Any] | None = None
) -> list[str]:
    if not isinstance(value, list):
        return [f"{path} must be a list"]
    issues: list[str] = []
    if len(value) > 20:
        issues.append(f"{path} must not contain verbose operation-log entries")
    for index, item in enumerate(value):
        item_path = f"{path}[{index}]"
        if not isinstance(item, dict):
            issues.append(f"{item_path} must be an object")
            continue
        issues.extend(
            f"{item_path}.{key} is not allowed" for key in item if key not in EVIDENCE_FIELDS
        )
        for key in ("claim", "source", "subject"):
            if not non_empty_string(item.get(key)):
                issues.append(f"{item_path}.{key} must be a non-empty string")
        status = item.get("status")
        if status not in CLAIM_STATUSES:
            issues.append(f"{item_path}.status must be one of {sorted(CLAIM_STATUSES)}")
        path_issue = None
        if non_empty_string(item.get("source")):
            path_issue = _valid_repository_evidence_path(item["source"], contract)
            if path_issue:
                issues.append(f"{item_path}.source {item['source']!r} {path_issue}")
        digest = item.get("digest")
        if digest is not None and (not isinstance(digest, str) or not HEX_DIGEST.fullmatch(digest)):
            issues.append(f"{item_path}.digest must be a SHA-256 hex digest when provided")
        if status == "verified" and path_issue is not None:
            issues.append(
                f"{item_path} verified claims require an existing repository evidence path"
            )
        for key, child in item.items():
            if key in FORBIDDEN_APPROACH_KEYS or FORBIDDEN_APPROACH_KEY_PATTERN.search(str(key)):
                issues.append(f"{item_path}.{key} is not allowed in an Implementation Approach")
            if isinstance(child, str) and len(child) > 2000:
                issues.append(f"{item_path}.{key} is too long for a concise knowledge record")
    return issues


def validate_implementation_approach(
    value: Any, contract: dict[str, Any] | None = None
) -> list[str]:
    """Validate a bounded approach and require real repository evidence for verified claims."""

    if not isinstance(value, dict):
        return ["implementationApproach must be an object"]
    issues = [
        f"implementationApproach.{key} is not allowed"
        for key in value
        if key not in TOP_LEVEL_APPROACH_FIELDS
    ]
    approach_type = value.get("approachType")
    if approach_type not in APPROACH_TYPES:
        issues.append(
            f"implementationApproach.approachType must be one of {sorted(APPROACH_TYPES)}"
        )
    status = value.get("status")
    if status not in APPROACH_STATUSES:
        issues.append(f"implementationApproach.status must be one of {sorted(APPROACH_STATUSES)}")
    issues.extend(
        _validate_approach_claim(
            value.get("summary"), "implementationApproach.summary", CLAIM_FIELDS, contract
        )
    )
    issues.extend(
        _validate_approach_claim(
            value.get("mechanism"), "implementationApproach.mechanism", CLAIM_FIELDS, contract
        )
    )
    for index, item in enumerate(value.get("affectedComponents", [])):
        issues.extend(
            _validate_approach_claim(
                item,
                f"implementationApproach.affectedComponents[{index}]",
                COMPONENT_FIELDS,
                contract,
            )
        )
    if not isinstance(value.get("affectedComponents"), list):
        issues.append("implementationApproach.affectedComponents must be a list")
    for index, item in enumerate(value.get("designDecisions", [])):
        issues.extend(
            _validate_approach_claim(
                item, f"implementationApproach.designDecisions[{index}]", DECISION_FIELDS, contract
            )
        )
    if not isinstance(value.get("designDecisions"), list):
        issues.append("implementationApproach.designDecisions must be a list")
    for index, item in enumerate(value.get("technicalDetails", [])):
        issues.extend(
            _validate_approach_claim(
                item, f"implementationApproach.technicalDetails[{index}]", DETAIL_FIELDS, contract
            )
        )
    if not isinstance(value.get("technicalDetails"), list):
        issues.append("implementationApproach.technicalDetails must be a list")
    issues.extend(
        _validate_approach_global_evidence(
            value.get("evidence"), "implementationApproach.evidence", contract
        )
    )
    for key, child in value.items():
        if key in FORBIDDEN_APPROACH_KEYS or FORBIDDEN_APPROACH_KEY_PATTERN.search(str(key)):
            issues.append(
                f"implementationApproach.{key} is not allowed in an Implementation Approach"
            )
        if isinstance(child, str) and len(child) > 2000:
            issues.append(
                f"implementationApproach.{key} is too long for a concise knowledge record"
            )
    return issues


def _approach_scope_kinds(contract: dict[str, Any] | None) -> set[str]:
    # Installer adoption records a governance bootstrap, not a product code
    # change. Its dedicated bootstrap paths are copied runtime evidence and
    # must not create a false Implementation Approach requirement for the
    # first adopter finish.
    if (
        isinstance(contract, dict)
        and contract.get("workItemId") == "adopt_ai_cockpit"
        and isinstance(contract.get("adoptionBootstrapPaths"), list)
    ):
        return set()
    paths = contract.get("scope", []) if isinstance(contract, dict) else []
    kinds: set[str] = set()
    for raw_path in paths:
        if not isinstance(raw_path, str):
            continue
        path = raw_path.lower()
        if path.startswith((".ai/work-items/", "tests/", "docs/")) or path.endswith(
            (".md", ".rst", ".txt")
        ):
            continue
        suffix = Path(path).suffix
        if suffix in CODE_SUFFIXES or path.startswith(("scripts/", "src/", "lib/", "app/")):
            kinds.add("implementation")
        elif suffix in CONFIG_SUFFIXES or path.startswith(("config/", "settings/")):
            kinds.add("configuration")
    return kinds


def assess_implementation_approach(
    summary: dict[str, Any], contract: dict[str, Any] | None
) -> dict[str, Any]:
    """Return the completeness signal without turning a knowledge gap into a red gate."""

    kinds = _approach_scope_kinds(contract)
    if not kinds:
        return {
            "status": "not_applicable",
            "humanStatusColor": "unknown",
            "requiredField": None,
            "warnings": [],
            "issues": [],
        }
    fields = [
        "implementationApproach" if kind == "implementation" else "configurationApproach"
        for kind in ("implementation", "configuration")
        if kind in kinds
    ]
    issues: list[str] = []
    warnings: list[str] = []
    for field in fields:
        value = summary.get(field)
        if value is None:
            warnings.append(
                f"{field} is incomplete for the declared {field.removesuffix('Approach')} change"
            )
            continue
        issues.extend(validate_implementation_approach(value, contract))
        if isinstance(value, dict) and value.get("status") != "complete":
            warnings.append(f"{field} is marked {value.get('status', 'unknown')}")
    status = "complete" if not warnings and not issues else "incomplete"
    return {
        "status": status,
        "humanStatusColor": "green" if status == "complete" else "yellow",
        "requiredField": fields[0] if len(fields) == 1 else fields,
        "warnings": warnings,
        "issues": issues,
    }


def intent_alignment_is_compat_evidence_key(key: str) -> bool:
    """Return True for legacy archive evidence aliases.

    Older archived summaries used ``*Evidence`` field names for the same
    intent-alignment facts. Keep those readable without forcing archive rewrites.
    """
    return key.endswith("Evidence")


def changed_file_paths(summary: dict[str, Any]) -> set[str]:
    changed = summary.get("changedFiles")
    if not isinstance(changed, list):
        return set()
    return {
        str(item["path"])
        for item in changed
        if isinstance(item, dict) and non_empty_string(item.get("path"))
    }


def validate_required_evidence_claims(
    contract: dict[str, Any], summary: dict[str, Any]
) -> list[str]:
    """Require an explicit prohibited-claim statement for every derived evidence gap."""
    context = contract.get("requiredEvidenceContext")
    if not isinstance(context, dict):
        return []
    operation = contract.get("requestedOperation")
    operation = operation if isinstance(operation, dict) else {}
    risk = contract.get("riskAssessment")
    risk = risk if isinstance(risk, dict) else {}
    profile = contract.get("governanceProfile")
    profile = profile if isinstance(profile, dict) else {}
    result = derive_required_evidence(
        EvidenceContext(
            requested_operation=str(operation.get("action", "")),
            changed_paths=tuple(
                item for item in contract.get("scope", []) if isinstance(item, str)
            ),
            risk_types=tuple(item for item in risk.get("riskTypes", []) if isinstance(item, str)),
            capability_claims=tuple(
                item for item in contract.get("capabilityClaims", []) if isinstance(item, str)
            ),
            environment=str(operation.get("environment", "")),
            external_system=str(context.get("externalSystem", "")),
            destructive_level=str(context.get("destructiveLevel", "none")),
            governance_profile=str(profile.get("selected", "standard")),
            available_evidence=tuple(
                item for item in context.get("availableEvidence", []) if isinstance(item, str)
            ),
        )
    )
    if not result.missing_evidence:
        return []
    prevention = summary.get("overclaimPrevention")
    prevention = prevention if isinstance(prevention, str) else ""
    return [
        f"derived missing evidence requires forbidden claim: {claim}"
        for claim in result.forbidden_claims
        if claim not in prevention
    ]


def documentation_alignment_skeleton() -> dict[str, Any]:
    """Return the canonical not-yet-reviewed documentation alignment record."""
    return {
        "schemaVersion": 1,
        "status": "not_checked",
        "checkedAt": None,
        "checks": [
            {
                "area": area,
                "status": "not_checked",
                "evidence": [],
                "reason": "Complete this alignment check before finishing the Work Item.",
            }
            for area in DOCUMENTATION_ALIGNMENT_AREAS
        ],
    }


def complete_generated_documentation_alignment(
    changed_files: list[dict[str, Any]],
) -> dict[str, Any]:
    """Derive bounded installer alignment from its final declared write set."""
    paths = sorted(
        {
            str(item["path"])
            for item in changed_files
            if isinstance(item, dict) and non_empty_string(item.get("path"))
        }
    )
    usable = [
        path
        for path in paths
        if path != ".ai/cockpit/current_status.md" and not path.endswith(".summary.json")
    ]
    anchor = next(
        (path for path in usable if path.endswith(".contract.json")),
        usable[0] if usable else "",
    )
    documentation = [path for path in paths if _documentation_surface(path)]
    multilingual = [
        path
        for path in documentation
        if path.endswith((".ja.md", ".zh-CN.md")) or "/ja/" in path or "/zh-CN/" in path
    ]

    def check(
        area: str,
        evidence: list[str],
        reason: str,
        *,
        applicable: bool = True,
    ) -> dict[str, Any]:
        return {
            "area": area,
            "status": "aligned" if applicable else "not_applicable",
            "evidence": evidence if applicable else [],
            "reason": reason,
        }

    return {
        "schemaVersion": 1,
        "status": "aligned",
        "checkedAt": datetime.now(UTC).isoformat(),
        "checks": [
            check(
                "plan",
                [],
                "The bounded installer record has no remediation-plan scope.",
                applicable=False,
            ),
            check(
                "contractSummaryEvidence",
                [anchor] if anchor else [],
                "The generated Contract is declared in the final installer write set.",
                applicable=bool(anchor),
            ),
            check(
                "documentationCommandsCapability",
                documentation,
                "Every installer-written documentation and command surface is enumerated.",
                applicable=bool(documentation),
            ),
            check(
                "multilingualSemantics",
                multilingual,
                "Installer-written Japanese or Chinese surfaces remain explicit for review.",
                applicable=bool(multilingual),
            ),
            check(
                "limitationsUnknownsHistory",
                [anchor] if anchor else [],
                "The generated Contract retains adoption or upgrade boundaries and follow-ups.",
                applicable=bool(anchor),
            ),
        ],
    }


def summary_exempt_patterns() -> list[str]:
    policy_lists = simple_yaml_lists(SCOPE_POLICY)
    return policy_lists.get("allowAlways", [])


def _validate_summary_structure(
    summary: dict[str, Any],
    contract: dict[str, Any] | None,
    *,
    contract_path: str,
    summary_path: str,
    legacy_archive: bool,
) -> list[str]:
    issues: list[str] = []
    documentation_alignment_required = contract is None or contract.get("contractVersion") == 2
    for key in REQUIRED_FIELDS:
        if key == "documentationAlignment" and (
            legacy_archive or not documentation_alignment_required
        ):
            continue
        if key not in summary:
            issues.append(f"missing field: {key}")

    if summary.get("summaryVersion") != 2 and not legacy_archive:
        issues.append("summaryVersion must be 2")

    for key in summary:
        if key not in ALLOWED_FIELDS:
            issues.append(f"unknown field: {key}")

    if contract is not None and summary.get("workItemId") != contract.get("workItemId"):
        issues.append("workItemId does not match the Contract")

    if contract_path and not legacy_archive:
        expected_contract_path = Path(contract_path).as_posix()
        if summary.get("contractPath") != expected_contract_path:
            issues.append("contractPath does not match the Contract path")

    if summary_path:
        summary_file = Path(summary_path)
        stem = summary_file.name.removesuffix(".summary.json")
        if stem and summary.get("workItemId") != stem:
            issues.append("workItemId does not match the Summary filename")

    changed = summary.get("changedFiles")
    if not isinstance(changed, list) or not changed:
        issues.append("changedFiles must contain at least one item")
    elif any(
        not isinstance(item, dict)
        or not non_empty_string(item.get("path"))
        or not non_empty_string(item.get("reason"))
        for item in changed
    ):
        issues.append("changedFiles must be a list of objects with path and reason")

    archive_sequence = summary.get("archiveSequence")
    if archive_sequence is not None and (
        not isinstance(archive_sequence, int)
        or isinstance(archive_sequence, bool)
        or archive_sequence < 1
    ):
        issues.append("archiveSequence must be a positive integer when present")

    return issues


def _validate_verification_entries(
    summary: dict[str, Any],
    contract: dict[str, Any] | None,
    *,
    expected_contract_hash: str,
    contract_path: str,
    summary_path: str,
    legacy_archive: bool,
) -> list[str]:
    issues: list[str] = []
    verification = summary.get("verification")
    if not isinstance(verification, list) or not verification:
        issues.append("verification must contain at least one item")
    else:
        for index, item in enumerate(verification):
            if not isinstance(item, dict):
                issues.append(f"verification[{index}] must be an object")
                continue
            key = verification_key(item)
            if not key:
                issues.append(f"verification[{index}] requires check or command")
            if isinstance(contract, dict) and contract.get("contractVersion") == 2:
                if not non_empty_string(item.get("check")):
                    issues.append(f"verification[{index}].check is required for contractVersion 2")
                else:
                    try:
                        expected_command, _ = render_check_command(
                            item["check"],
                            contract_path=item.get("executionContractPath", contract_path),
                            summary_path=item.get("executionSummaryPath", summary_path),
                        )
                        if (
                            not registered_command_matches(
                                item["check"], item.get("command"), expected_command
                            )
                            and item.get("result") == "passed"
                        ):
                            issues.append(
                                f"verification[{index}].command does not match registered check"
                            )
                    except ValueError as exc:
                        issues.append(f"verification[{index}]: {exc}")
            if item.get("result") not in RESULTS:
                issues.append(f"verification[{index}].result must be one of {sorted(RESULTS)}")
            if item.get("result") == "passed" and (
                not isinstance(contract, dict) or contract.get("contractVersion") == 2
            ):
                if item.get("runner") != "ai_finish":
                    issues.append(f"verification[{index}] passed result requires runner ai_finish")
                if not non_empty_string(item.get("executedAt")):
                    issues.append(f"verification[{index}].executedAt is required for passed result")
                if not isinstance(item.get("exitCode"), int) or item.get("exitCode") != 0:
                    issues.append(f"verification[{index}].exitCode must be 0 for passed result")
                duration = item.get("durationMs")
                if not isinstance(duration, int) or duration < 0:
                    issues.append(
                        f"verification[{index}].durationMs must be a non-negative integer"
                    )
                digest = item.get("outputDigest")
                if (
                    not non_empty_string(digest)
                    or len(str(digest)) != 64
                    or any(ch not in "0123456789abcdef" for ch in str(digest))
                ):
                    issues.append(
                        f"verification[{index}].outputDigest must be a SHA-256 hex digest"
                    )
                if isinstance(contract, dict) and contract.get("contractVersion") == 2:
                    command = item.get("command", "")
                    command_hash = hashlib.sha256(
                        " ".join(command.split()).encode("utf-8")
                    ).hexdigest()
                    if item.get("commandHash") != command_hash:
                        issues.append(f"verification[{index}].commandHash does not match command")
                    if (
                        expected_contract_hash
                        and not legacy_archive
                        and item.get("contractHash") != expected_contract_hash
                    ):
                        issues.append(f"verification[{index}].contractHash does not match Contract")
                    for path_key in ("executionContractPath", "executionSummaryPath"):
                        if (
                            not non_empty_string(item.get(path_key))
                            or Path(item[path_key]).is_absolute()
                        ):
                            issues.append(
                                f"verification[{index}].{path_key} must be a repository-relative path"
                            )
                    commit_sha = item.get("commitSha")
                    if (
                        not non_empty_string(commit_sha)
                        or len(str(commit_sha)) not in {40, 64}
                        or any(ch not in "0123456789abcdef" for ch in str(commit_sha))
                    ):
                        issues.append(f"verification[{index}].commitSha must be a Git object id")
                    worktree_digest = item.get("worktreeDigest")
                    if worktree_digest is None:
                        if not legacy_archive:
                            issues.append(
                                f"verification[{index}].worktreeDigest is required for passed result"
                            )
                    elif (
                        not non_empty_string(worktree_digest)
                        or len(str(worktree_digest)) != 64
                        or any(ch not in "0123456789abcdef" for ch in str(worktree_digest))
                    ):
                        issues.append(
                            f"verification[{index}].worktreeDigest must be a SHA-256 hex digest"
                        )

    return issues


def registered_command_matches(check_id: str, command: object, expected_command: str) -> bool:
    """Accept the registered command plus the bounded Finish quality route."""
    if command == expected_command:
        return True
    if check_id != "quality" or not isinstance(command, str):
        return False
    return command in {
        f"{expected_command} GOVERNANCE_PROFILE=light",
        f"{expected_command} GOVERNANCE_PROFILE=standard",
        f"{expected_command} GOVERNANCE_PROFILE=strict",
    }


def _validate_non_risk_explanations(summary: dict[str, Any]) -> list[str]:
    """Validate Summary evidence that is informative but not an unresolved gap."""
    issues: list[str] = []
    non_risk_explanations = summary.get("nonRiskExplanations")
    if not isinstance(non_risk_explanations, list):
        return issues
    for index, explanation in enumerate(non_risk_explanations):
        prefix = f"nonRiskExplanations[{index}]"
        if not isinstance(explanation, dict):
            issues.append(f"{prefix} must be an object")
            continue
        allowed = {"sourceWarning", "reason", "evidence"}
        for key in explanation:
            if key not in allowed:
                issues.append(f"{prefix}.{key} is not allowed")
        for key in ("sourceWarning", "reason"):
            if not non_empty_string(explanation.get(key)):
                issues.append(f"{prefix}.{key} must be a non-empty string")
        evidence = explanation.get("evidence")
        if not isinstance(evidence, list):
            issues.append(f"{prefix}.evidence must be a list")
            continue
        if not evidence:
            issues.append(f"{prefix}.evidence must contain at least one evidence reference")
            continue
        for evidence_index, reference in enumerate(evidence):
            reference_prefix = f"{prefix}.evidence[{evidence_index}]"
            if not isinstance(reference, dict):
                issues.append(f"{reference_prefix} must be an object")
                continue
            if not non_empty_string(reference.get("source")):
                issues.append(f"{reference_prefix}.source must be a non-empty string")
            if not non_empty_string(reference.get("subject")):
                issues.append(f"{reference_prefix}.subject must be a non-empty string")
    return issues


def _validate_summary_metadata(summary: dict[str, Any]) -> list[str]:
    issues: list[str] = []
    risk = summary.get("risk")
    if not isinstance(risk, dict):
        issues.append("risk must be an object")
    else:
        if risk.get("level") not in RISK_LEVELS:
            issues.append(f"risk.level must be one of {sorted(RISK_LEVELS)}")
        if not non_empty_string(risk.get("detail")):
            issues.append("risk.detail is required")

    for key in (
        "sourcesUsed",
        "unknownsRemaining",
        "generatedFiles",
        "destructiveChanges",
        "observedIssues",
        "verificationHistory",
        "guidelinesCompliance",
        "followUps",
        "unverifiedScenarios",
    ):
        if key in summary and not isinstance(summary.get(key), list):
            issues.append(f"{key} must be a list")

    for key in (
        "userCorrectionsCaptured",
        "userCorrectionSolidification",
        "knownGaps",
        "nonRiskExplanations",
    ):
        if key in summary and not isinstance(summary.get(key), list):
            issues.append(f"{key} must be a list")

    issues.extend(_validate_non_risk_explanations(summary))

    known_gaps = summary.get("knownGaps")
    if (
        summary.get("archiveSequence") is None
        and isinstance(known_gaps, list)
        and any(
            isinstance(item, str)
            and re.search(r"\barchive[\s_-]+sequence\s+#?\d+\b", item, re.IGNORECASE)
            for item in known_gaps
        )
    ):
        issues.append(
            "knownGaps must not predict a numeric archive sequence before the generator allocates it"
        )

    checkpoints = summary.get("checkpointEvidence")
    if checkpoints is not None:
        if not isinstance(checkpoints, list):
            issues.append("checkpointEvidence must be a list")
        else:
            for index, item in enumerate(checkpoints):
                if not isinstance(item, dict):
                    issues.append(f"checkpointEvidence[{index}] must be an object")
                    continue
                if not non_empty_string(item.get("stage")):
                    issues.append(f"checkpointEvidence[{index}].stage is required")
                if "recorded" in item and not isinstance(item.get("recorded"), bool):
                    issues.append(f"checkpointEvidence[{index}].recorded must be boolean")
                if "detail" in item and not isinstance(item.get("detail"), str):
                    issues.append(f"checkpointEvidence[{index}].detail must be a string")
                if "contractHash" in item and not non_empty_string(item.get("contractHash")):
                    issues.append(
                        f"checkpointEvidence[{index}].contractHash must be a non-empty string"
                    )
                for metric in (
                    "acceptanceCount",
                    "unknownCount",
                    "requiredChecks",
                    "requiredChecksPassed",
                ):
                    if metric in item and not isinstance(item.get(metric), int):
                        issues.append(f"checkpointEvidence[{index}].{metric} must be integer")

    residual = summary.get("residualRisks")
    if residual is not None:
        if not isinstance(residual, list):
            issues.append("residualRisks must be a list")
        else:
            for index, item in enumerate(residual):
                if not isinstance(item, dict):
                    issues.append(f"residualRisks[{index}] must be an object")
                    continue
                if item.get("level") not in RISK_LEVELS:
                    issues.append(
                        f"residualRisks[{index}].level must be one of {sorted(RISK_LEVELS)}"
                    )
                if not non_empty_string(item.get("area")):
                    issues.append(f"residualRisks[{index}].area is required")
                if not non_empty_string(item.get("detail")):
                    issues.append(f"residualRisks[{index}].detail is required")

    readiness = summary.get("reviewReadiness")
    if readiness is not None:
        if not isinstance(readiness, dict):
            issues.append("reviewReadiness must be an object")
        else:
            if readiness.get("status") not in REVIEW_READINESS_STATUSES:
                issues.append(
                    f"reviewReadiness.status must be one of {sorted(REVIEW_READINESS_STATUSES)}"
                )
            if not non_empty_string(readiness.get("reason")):
                issues.append("reviewReadiness.reason is required")
            focus = readiness.get("expectedReviewFocus")
            if focus is not None and (
                not isinstance(focus, list) or any(not non_empty_string(item) for item in focus)
            ):
                issues.append(
                    "reviewReadiness.expectedReviewFocus must be a list of non-empty strings"
                )

    issues.extend(validate_intent_alignment(summary))

    boundary = summary.get("boundaryChecks")
    if boundary is not None:
        if not isinstance(boundary, dict):
            issues.append("boundaryChecks must be an object")
        else:
            for key, value in boundary.items():
                if not non_empty_string(key) or not non_empty_string(value):
                    issues.append(
                        "boundaryChecks must map non-empty names to non-empty status strings"
                    )
                    break

    if "overclaimPrevention" in summary and not non_empty_string(
        summary.get("overclaimPrevention")
    ):
        issues.append("overclaimPrevention must be a non-empty string")

    issues.extend(validate_scenario_coverage(summary.get("scenarioCoverage")))

    def scan_machine_paths(value: Any, location: str) -> None:
        if isinstance(value, str) and contains_machine_path(value):
            issues.append(f"{location} contains a machine-specific path")
        elif isinstance(value, dict):
            for key, child in value.items():
                scan_machine_paths(child, f"{location}.{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                scan_machine_paths(child, f"{location}[{index}]")

    scan_machine_paths(summary, "summary")

    return issues


def validate_hosted_performance_evidence(summary: dict[str, Any]) -> list[str]:
    """Validate the registered, structured hosted performance evidence shape."""
    value = summary.get("hostedPerformanceEvidence")
    if value is None:
        return []
    issues: list[str] = []
    if not isinstance(value, dict):
        return ["hostedPerformanceEvidence must be an object"]
    if value.get("schemaVersion") != 1:
        issues.append("hostedPerformanceEvidence.schemaVersion must be 1")
    if not non_empty_string(value.get("baselineWorkItem")):
        issues.append("hostedPerformanceEvidence.baselineWorkItem is required")
    if not non_empty_string(value.get("comparisonRule")):
        issues.append("hostedPerformanceEvidence.comparisonRule is required")
    if value.get("status") not in {"not_run", "partial", "complete"}:
        issues.append("hostedPerformanceEvidence.status must be not_run, partial, or complete")
    scenarios = value.get("scenarios")
    if not isinstance(scenarios, list) or not scenarios:
        issues.append("hostedPerformanceEvidence.scenarios must be a non-empty list")
        return issues
    for index, scenario in enumerate(scenarios):
        prefix = f"hostedPerformanceEvidence.scenarios[{index}]"
        if not isinstance(scenario, dict):
            issues.append(f"{prefix} must be an object")
            continue
        if not non_empty_string(scenario.get("scenario")):
            issues.append(f"{prefix}.scenario is required")
        if scenario.get("status") not in {"pass", "not_run", "fail"}:
            issues.append(f"{prefix}.status must be pass, not_run, or fail")
        if scenario.get("status") in {"not_run", "fail"} and not non_empty_string(
            scenario.get("reason")
        ):
            issues.append(f"{prefix}.reason is required")
        evidence = scenario.get("evidence")
        if not isinstance(evidence, list) or any(not non_empty_string(item) for item in evidence):
            issues.append(f"{prefix}.evidence must be a list of strings")
    return issues


def _offset_aware_iso_timestamp(value: Any) -> bool:
    if not non_empty_string(value):
        return False
    try:
        parsed = datetime.fromisoformat(str(value))
    except ValueError:
        return False
    return parsed.tzinfo is not None and parsed.utcoffset() is not None


def _documentation_surface(path: str) -> bool:
    if path == ".ai/cockpit/current_status.md" or path.startswith(".ai/work-items/"):
        return False
    name = Path(path).name
    return (
        path.endswith(".md")
        or name == "Makefile"
        or name.endswith(".mk")
        or path.startswith("templates/make/")
    )


def _active_contract_owns_untracked_evidence(path: str, contract: dict[str, Any] | None) -> bool:
    """Allow only current Work Item scope to supply pre-commit evidence."""
    if not isinstance(contract, dict) or contract.get("contractVersion") != 2:
        return False
    scope = contract.get("scope")
    return isinstance(scope, list) and included(
        path, [item for item in scope if isinstance(item, str)]
    )


def _valid_repository_evidence_path(
    path: str, contract: dict[str, Any] | None = None
) -> str | None:
    if "://" in path:
        return "must be a repository-relative path, not a URL"
    candidate = Path(path)
    if candidate.is_absolute() or path.startswith("~") or "\\" in path:
        return "must be repository-relative"
    if not path or any(part in {"", ".", ".."} for part in candidate.parts):
        return "must be a normalized repository-relative path"
    if not (PROJECT_ROOT / candidate).exists():
        return "does not exist"
    if not _is_git_tracked_repository_path(path) and not _active_contract_owns_untracked_evidence(
        path, contract
    ):
        return "must be a Git-tracked repository file or an active Contract-scoped file"
    return None


def _is_git_tracked_repository_path(path: str) -> bool:
    """Return whether a path can be present in a clean repository checkout."""
    result = subprocess.run(  # nosec B603 B607 - fixed list-form Git interrogation
        ["git", "ls-files", "--error-unmatch", "--", path],
        cwd=PROJECT_ROOT,
        text=True,
        capture_output=True,
        check=False,
    )
    return result.returncode == 0


def validate_documentation_alignment(
    summary: dict[str, Any],
    contract: dict[str, Any] | None = None,
    *,
    legacy_archive: bool = False,
    required: bool = True,
) -> list[str]:
    """Validate source-bound close-out alignment without rewriting old archives."""
    value = summary.get("documentationAlignment")
    if value is None:
        return [] if legacy_archive or not required else ["documentationAlignment is required"]
    if not isinstance(value, dict):
        return ["documentationAlignment must be an object"]

    issues: list[str] = []
    unknown_root = set(value) - DOCUMENTATION_ALIGNMENT_ROOT_FIELDS
    missing_root = DOCUMENTATION_ALIGNMENT_ROOT_FIELDS - set(value)
    for key in sorted(unknown_root):
        issues.append(f"documentationAlignment.{key} is not a recognized field")
    for key in sorted(missing_root):
        issues.append(f"documentationAlignment.{key} is required")
    if value.get("schemaVersion") != 1:
        issues.append("documentationAlignment.schemaVersion must be 1")
    if value.get("status") != "aligned":
        issues.append("documentationAlignment.status must be aligned before finish")
    if not _offset_aware_iso_timestamp(value.get("checkedAt")):
        issues.append("documentationAlignment.checkedAt must be an offset-aware ISO-8601 timestamp")

    checks = value.get("checks")
    if not isinstance(checks, list):
        issues.append("documentationAlignment.checks must be a list")
        return issues

    declared_paths = changed_file_paths(summary)
    sources = summary.get("sourcesUsed")
    if isinstance(sources, list):
        declared_paths.update(item for item in sources if non_empty_string(item))

    seen_areas: set[str] = set()
    aligned_evidence: set[str] = set()
    for index, check in enumerate(checks):
        prefix = f"documentationAlignment.checks[{index}]"
        if not isinstance(check, dict):
            issues.append(f"{prefix} must be an object")
            continue
        for key in sorted(set(check) - DOCUMENTATION_ALIGNMENT_CHECK_FIELDS):
            issues.append(f"{prefix}.{key} is not a recognized field")
        for key in sorted(DOCUMENTATION_ALIGNMENT_CHECK_FIELDS - set(check)):
            issues.append(f"{prefix}.{key} is required")

        area = check.get("area")
        if area not in DOCUMENTATION_ALIGNMENT_AREAS:
            issues.append(f"{prefix}.area must be one of {sorted(DOCUMENTATION_ALIGNMENT_AREAS)}")
        elif area in seen_areas:
            issues.append(f"{prefix}.area is a duplicate area: {area}")
        else:
            seen_areas.add(area)

        status = check.get("status")
        if status not in {"aligned", "not_applicable"}:
            issues.append(f"{prefix}.status must be aligned or not_applicable")
        if not non_empty_string(check.get("reason")):
            issues.append(f"{prefix}.reason is required")
        evidence = check.get("evidence")
        if not isinstance(evidence, list) or any(not non_empty_string(item) for item in evidence):
            issues.append(f"{prefix}.evidence must be a list of repository-relative paths")
            continue
        if len(evidence) != len(set(evidence)):
            issues.append(f"{prefix}.evidence must not contain duplicate paths")
        if status == "aligned" and not evidence:
            issues.append(f"{prefix}.evidence must not be empty when aligned")
        if status == "not_applicable" and evidence:
            issues.append(f"{prefix}.evidence must be empty when not_applicable")
        for path in evidence:
            aligned_evidence.add(path)
            path_issue = _valid_repository_evidence_path(path, contract)
            if path_issue:
                issues.append(f"{prefix}.evidence path {path!r} {path_issue}")
            elif path not in declared_paths:
                issues.append(
                    f"{prefix}.evidence path {path!r} is not declared in "
                    "changedFiles or sourcesUsed"
                )

    for area in DOCUMENTATION_ALIGNMENT_AREAS:
        if area not in seen_areas:
            issues.append(f"documentationAlignment.checks is missing required area: {area}")

    documentation_surfaces = {
        path for path in changed_file_paths(summary) if _documentation_surface(path)
    }
    for path in sorted(documentation_surfaces - aligned_evidence):
        issues.append(
            "documentationAlignment evidence is missing changed "
            f"documentation/command surface: {path}"
        )
    return issues


def _validate_required_verification(
    summary: dict[str, Any], contract: dict[str, Any] | None
) -> list[str]:
    issues: list[str] = []
    if contract is not None:
        required = [
            verification_key(item)
            for item in contract.get("verification", [])
            if isinstance(item, dict) and item.get("required") is True and verification_key(item)
        ]
        status = {
            verification_key(item): item.get("result")
            for item in summary.get("verification", [])
            if isinstance(item, dict)
        }
        # aiSummary is the check currently being executed. Requiring its prior
        # result would make a fresh Summary impossible to validate; ai_finish
        # records the passing evidence immediately after this validator returns.
        self_check = "aiSummary"
        missing = [
            command for command in required if command != self_check and command not in status
        ]
        non_passed = [
            command
            for command in required
            if command != self_check and status.get(command) != "passed"
        ]
        if missing:
            issues.append(f"Summary is missing required verification: {', '.join(missing)}")
        if non_passed:
            issues.append(f"required verification is not passed: {', '.join(non_passed)}")
    return issues


def validate_summary(
    summary: dict[str, Any],
    contract: dict[str, Any] | None,
    *,
    expected_contract_hash: str = "",
    contract_path: str = "",
    summary_path: str = "",
    legacy_archive: bool = False,
) -> list[str]:
    """Validate a Summary by composing focused schema and evidence checks."""
    issues: list[str] = []
    issues.extend(
        _validate_summary_structure(
            summary,
            contract,
            contract_path=contract_path,
            summary_path=summary_path,
            legacy_archive=legacy_archive,
        )
    )
    issues.extend(
        _validate_verification_entries(
            summary,
            contract,
            expected_contract_hash=expected_contract_hash,
            contract_path=contract_path,
            summary_path=summary_path,
            legacy_archive=legacy_archive,
        )
    )
    issues.extend(_validate_summary_metadata(summary))
    approach_assessment = assess_implementation_approach(summary, contract)
    issues.extend(approach_assessment["issues"])
    issues.extend(validate_hosted_performance_evidence(summary))
    issues.extend(
        validate_documentation_alignment(
            summary,
            contract,
            legacy_archive=legacy_archive,
            required=contract is None or contract.get("contractVersion") == 2,
        )
    )
    issues.extend(
        validate_residual_risk_semantics(
            summary, legacy_archive=legacy_archive, summary_path=summary_path
        )
    )
    issues.extend(_validate_required_verification(summary, contract))
    if isinstance(contract, dict):
        issues.extend(validate_required_evidence_claims(contract, summary))
        issues.extend(
            validate_acceptance_evidence(
                contract,
                summary,
                summary.get("verification", [])
                if isinstance(summary.get("verification"), list)
                else [],
            )
        )
    return issues


def validate_intent_alignment(summary: dict[str, Any]) -> list[str]:
    """Validate the optional Summary intentAlignment section.

    The section may be absent, null, empty, partially populated, or complete.
    Legacy archived summaries may also use ``*Evidence`` aliases for the same
    fields, and those remain accepted for backward compatibility.
    """
    issues: list[str] = []
    alignment = summary.get("intentAlignment")
    if alignment is None:
        return issues
    if not isinstance(alignment, dict):
        issues.append("intentAlignment must be an object")
        return issues

    for key in alignment:
        if (
            key not in INTENT_ALIGNMENT_BOOL_KEYS | INTENT_ALIGNMENT_STRING_KEYS
            and not intent_alignment_is_compat_evidence_key(key)
        ):
            issues.append(f"intentAlignment.{key} is not a recognized field")

    for key in INTENT_ALIGNMENT_BOOL_KEYS:
        value = alignment.get(key)
        if value is not None and not isinstance(value, bool):
            issues.append(f"intentAlignment.{key} must be boolean when provided")

    for key, value in alignment.items():
        if key not in INTENT_ALIGNMENT_STRING_KEYS and not intent_alignment_is_compat_evidence_key(
            key
        ):
            continue
        if value is not None and not non_empty_string(value):
            issues.append(f"intentAlignment.{key} must be a non-empty string when provided")

    return issues


def validate_residual_risk_semantics(
    summary: dict[str, Any], *, legacy_archive: bool = False, summary_path: str = ""
) -> list[str]:
    """Reject generated residual-risk skeleton text before a new Summary is archived.

    Historical archive evidence remains readable and immutable; the gate applies to
    active v2 Summaries so the generated ai-start placeholder cannot reach Finish.
    """
    if legacy_archive or "/archive/" in Path(summary_path).as_posix():
        return []
    residual = summary.get("residualRisks")
    if not isinstance(residual, list):
        return []
    issues: list[str] = []
    for index, item in enumerate(residual):
        if not isinstance(item, dict) or not isinstance(item.get("detail"), str):
            continue
        detail = item["detail"].casefold()
        if any(marker in detail for marker in RESIDUAL_RISK_PLACEHOLDER_MARKERS):
            issues.append(f"residualRisks[{index}].detail contains generated placeholder text")
    return issues


def validate_changed_files_cover_diff(
    summary: dict[str, Any], contract: dict[str, Any] | None = None
) -> list[str]:
    try:
        paths = changed_paths(contract)
    except RuntimeError as exc:
        return [f"failed to read changed paths: {exc}"]

    reported = changed_file_paths(summary)
    exempt = summary_exempt_patterns()
    binding = contract.get("startReceipt") if isinstance(contract, dict) else None
    receipt_path = binding.get("path") if isinstance(binding, dict) else None
    missing = [
        path
        for path in paths
        if path not in reported and path != receipt_path and not included(path, exempt)
    ]
    if not missing:
        return []
    return [f"changedFiles is missing actual changed path: {path}" for path in missing]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate AI Change Summary.")
    parser.add_argument("summary", nargs="?")
    parser.add_argument("--contract")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.summary:
        print("Skipping summary check (no active summary provided)")
        return 0
    start = time.time()
    try:
        summary = load_json(Path(args.summary))
        contract = load_json(Path(args.contract)) if args.contract else None
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"Failed to read Summary or Contract: {exc}", file=sys.stderr)
        return 1

    obs = create_observability(work_item_id=summary.get("workItemId", ""))
    expected_hash = (
        hashlib.sha256(Path(args.contract).read_bytes()).hexdigest() if args.contract else ""
    )
    issues = validate_summary(
        summary,
        contract,
        expected_contract_hash=expected_hash,
        contract_path=args.contract or "",
        summary_path=args.summary,
    )
    issues.extend(validate_changed_files_cover_diff(summary, contract))
    duration = elapsed_ms(start)
    if issues:
        for issue in issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        obs.check_failed(
            check_id="aiSummary", duration_ms=duration, detail=f"{len(issues)} issue(s)"
        )
        return 1
    print(f"ai summary check passed: {args.summary}")
    obs.check_passed(check_id="aiSummary", duration_ms=duration)
    return 0


if __name__ == "__main__":
    sys.exit(main())
