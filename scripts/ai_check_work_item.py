#!/usr/bin/env python3
"""Validate the minimum Work Item Contract structure."""

from __future__ import annotations

import json
import sys
import time
from pathlib import Path
from typing import Any

from ai_calibration_corrective import (
    calibration_corrective_binding_issue,
    validate_calibration_corrective_shape,
)
from ai_check_guards import detect as detect_guard_items
from ai_common import (
    PROJECT_ROOT,
    contains_machine_path,
    load_check_registry,
    load_json,
    matches,
    non_empty_string,
    validate_scenario_coverage,
)
from ai_evidence_dependencies import (
    SOURCE_BOUND_GENERATED_DOCUMENTATION_PATHS,
    SOURCE_BOUND_GENERATED_EVIDENCE_MODE,
)
from ai_external_identity import high_risk_approval_issues
from ai_observability import create_observability, elapsed_ms
from ai_projection_lease import BRANCH_INTEGRATED_GENERATED_PATHS
from ai_start_receipt import receipt_path, validate_receipt

REQUIRED_FIELDS = (
    "contractVersion",
    "workItemId",
    "mode",
    "title",
    "scope",
    "outOfScope",
    "sources",
    "unknowns",
    "notCodable",
    "acceptance",
    "verification",
    "rollbackNote",
)
ALLOWED_FIELDS = set(REQUIRED_FIELDS) | {
    "agentCapability",
    "checkpointPolicy",
    "destructiveChangePolicy",
    "executionDecision",
    "preReviewWarnings",
    "riskAssessment",
    "baseCommit",
    "baseRemote",
    "baseBranch",
    "sourceReleaseTag",
    "sourceRepository",
    "baselineDirtyPaths",
    "adoptionBootstrapPaths",
    "restrictedWriteApproval",
    "guidelines",
    "intent",
    "rawUserRequest",
    "rawRequestSource",
    "rawRequestExemption",
    "declaredIntent",
    "requestedOperation",
    "authorityEvidence",
    "problemStatement",
    "scenarioCoverage",
    "archiveIndexRepair",
    "startReceipt",
    "predecessorWorkItem",
    "resumeHistory",
    "synchronizationHistory",
    "synchronizationCheckpoint",
    "budgetImpact",
    "governanceProfile",
    "operationClasses",
    "verificationEscalations",
    "capabilityClaims",
    "governanceMetadataVersion",
    "requiredEvidence",
    "humanDecisionPoints",
    "documentationImpact",
    "performanceImpact",
    "residualRiskExpectation",
    "predecessorClosureEvidence",
    "rollbackPlan",
    "requiredEvidenceContext",
    "sourceBoundGeneratedEvidence",
    "concurrencyBoundary",
    "calibrationCorrective",
    "implementationSurface",
}
MODES = {"investigate", "author_todo", "code", "review", "cleanup"}
RISK_LEVELS = {"low", "medium", "high"}
EXECUTION_STATUSES = {"continue", "defer", "needs_human_decision", "block"}
GOVERNANCE_PROFILES = {"light", "standard", "strict"}
GOVERNANCE_PROFILE_SOURCES = {"automatic", "human_override"}
INTENT_STRING_KEYS = {"businessGoal", "userGoal", "problem", "rationale"}
INTENT_LIST_KEYS = {"constraints", "nonGoals"}


def validate_concurrency_boundary(data: dict[str, Any]) -> list[str]:
    """Validate explicit parallel ownership including every shared projection."""
    boundary = data.get("concurrencyBoundary")
    if boundary is None:
        return []
    if not isinstance(boundary, dict):
        return ["concurrencyBoundary must be an object"]
    issues: list[str] = []
    if boundary.get("schemaVersion") != 1:
        issues.append("concurrencyBoundary.schemaVersion must be 1")
    task = data.get("workItemId")
    for key in ("implementationPaths", "generatedEvidencePaths", "verificationOutputPaths"):
        values = boundary.get(key)
        if (
            not isinstance(values, list)
            or not values
            or any(not non_empty_string(item) for item in values)
        ):
            issues.append(f"concurrencyBoundary.{key} must be a non-empty list of paths")
            continue
        for index, value in enumerate(values):
            if not isinstance(value, str):
                continue
            if value.startswith("/") or ".." in Path(value).parts:
                issues.append(
                    f"concurrencyBoundary.{key}[{index}] must be a repository-relative path"
                )
            if "*" in value and (
                key != "verificationOutputPaths" or value != f"target/quality/{task}/**"
            ):
                issues.append(
                    f"concurrencyBoundary.{key}[{index}] may use ** only as target/quality/{task}/**"
                )
    serialized = boundary.get("serializedProjectionPaths")
    if not isinstance(serialized, list) or any(not non_empty_string(item) for item in serialized):
        issues.append("concurrencyBoundary.serializedProjectionPaths must be a string list")
    elif set(serialized) != BRANCH_INTEGRATED_GENERATED_PATHS or len(serialized) != len(
        set(serialized)
    ):
        issues.append(
            "concurrencyBoundary.serializedProjectionPaths must exactly declare the closed branch-integrated projection inventory"
        )
    if not non_empty_string(boundary.get("reason")):
        issues.append("concurrencyBoundary.reason must be a non-empty string")
    return issues


def validate_calibration_corrective(data: dict[str, Any]) -> list[str]:
    """Validate an optional controlled-calibration declaration in a Contract."""
    value = data.get("calibrationCorrective")
    if value is None:
        return []
    issue = validate_calibration_corrective_shape(value)
    if issue:
        return [issue]
    if not isinstance(value, dict):
        return ["calibrationCorrective must be a JSON object"]
    repair_paths = value["repairPaths"]
    if not isinstance(repair_paths, list):
        return ["calibrationCorrective.repairPaths must be a list"]
    scope = data.get("scope", [])
    out_of_scope = data.get("outOfScope", [])
    issues: list[str] = []
    for index, path in enumerate(repair_paths):
        if not isinstance(path, str):
            issues.append(f"calibrationCorrective.repairPaths[{index}] must be a string")
            continue
        if not any(isinstance(pattern, str) and matches(pattern, path) for pattern in scope):
            issues.append(f"calibrationCorrective.repairPaths[{index}] is not covered by scope")
        if any(isinstance(pattern, str) and matches(pattern, path) for pattern in out_of_scope):
            issues.append(f"calibrationCorrective.repairPaths[{index}] is covered by outOfScope")
    return issues


def validate_string_list(data: dict[str, Any], key: str, *, allow_empty: bool) -> list[str]:
    issues: list[str] = []
    value = data.get(key)
    if not isinstance(value, list):
        return [f"{key} must be a list"]
    if not allow_empty and not value:
        issues.append(f"{key} must contain at least one item")
    for index, item in enumerate(value):
        if not non_empty_string(item):
            issues.append(f"{key}[{index}] must be a non-empty string")
    return issues


def validate_governance_profile(data: dict[str, Any]) -> list[str]:
    """Validate Contract evidence for automatic selection or a bounded override."""
    profile = data.get("governanceProfile")
    if profile is None:
        return []
    if not isinstance(profile, dict):
        return ["governanceProfile must be an object"]

    issues: list[str] = []
    selected = profile.get("selected")
    source = profile.get("source")
    reasons = profile.get("reasons")
    override = profile.get("override")
    if selected not in GOVERNANCE_PROFILES:
        issues.append(f"governanceProfile.selected must be one of {sorted(GOVERNANCE_PROFILES)}")
    if source not in GOVERNANCE_PROFILE_SOURCES:
        issues.append(
            f"governanceProfile.source must be one of {sorted(GOVERNANCE_PROFILE_SOURCES)}"
        )
    if (
        not isinstance(reasons, list)
        or not reasons
        or any(not non_empty_string(item) for item in reasons)
    ):
        issues.append("governanceProfile.reasons must contain at least one non-empty string")

    if source == "automatic":
        if override is not None:
            issues.append("governanceProfile.override must be null when source is automatic")
        return issues
    if source != "human_override":
        return issues
    if not isinstance(override, dict):
        issues.append("governanceProfile.override must be an object when source is human_override")
        return issues

    for key in ("approvalEvidence", "reason"):
        if not non_empty_string(override.get(key)):
            issues.append(f"governanceProfile.override.{key} must be a non-empty string")
    for key in ("risks", "notRunChecks"):
        values = override.get(key)
        if (
            not isinstance(values, list)
            or not values
            or any(not non_empty_string(item) for item in values)
        ):
            issues.append(
                f"governanceProfile.override.{key} must contain at least one non-empty string"
            )
    expires_at = override.get("expiresAt")
    work_item_only = override.get("workItemOnly")
    if not non_empty_string(expires_at) and work_item_only is not True:
        issues.append("governanceProfile.override requires expiresAt or workItemOnly true")
    if "workItemId" in override and not non_empty_string(override.get("workItemId")):
        issues.append("governanceProfile.override.workItemId must be a non-empty string")
    return issues


def validate_operation_escalations(data: dict[str, Any]) -> list[str]:
    """Validate optional operation inputs without trusting them as the sole derivation source."""
    issues: list[str] = []
    for key in ("operationClasses", "verificationEscalations", "capabilityClaims"):
        if key in data:
            issues.extend(validate_string_list(data, key, allow_empty=True))
    return issues


def validate_sources(data: dict[str, Any]) -> list[str]:
    issues: list[str] = []
    sources = data.get("sources")
    if not isinstance(sources, list) or not sources:
        return ["sources must contain at least one item"]
    for index, item in enumerate(sources):
        if non_empty_string(item):
            continue
        if isinstance(item, dict):
            if not non_empty_string(item.get("path")):
                issues.append(f"sources[{index}].path is required")
            if not non_empty_string(item.get("reason")):
                issues.append(f"sources[{index}].reason is required")
            continue
        issues.append(f"sources[{index}] must be a string or a path/reason object")
    return issues


def validate_verification(data: dict[str, Any]) -> list[str]:
    issues: list[str] = []
    values = data.get("verification")
    if not isinstance(values, list) or not values:
        return ["verification must contain at least one item"]
    version = data.get("contractVersion")
    registry = load_check_registry()
    seen: set[str] = set()
    for index, item in enumerate(values):
        if not isinstance(item, dict):
            issues.append(f"verification[{index}] must be an object")
            continue
        if version == 2:
            check_id = item.get("check")
            if not non_empty_string(check_id):
                issues.append(f"verification[{index}].check is required")
            elif check_id not in registry:
                issues.append(f"verification[{index}].check is not registered: {check_id}")
            elif check_id in seen:
                issues.append(f"verification[{index}].check is duplicated: {check_id}")
            else:
                seen.add(check_id)
            if "command" in item:
                issues.append(f"verification[{index}].command is forbidden in contractVersion 2")
        elif not non_empty_string(item.get("command")):
            issues.append(f"verification[{index}].command is required")
        if not isinstance(item.get("required"), bool):
            issues.append(f"verification[{index}].required must be boolean")
    return issues


def validate_optional_readiness(data: dict[str, Any]) -> list[str]:
    issues: list[str] = []

    if data.get("contractVersion") == 2:
        for key in (
            "riskAssessment",
            "agentCapability",
            "executionDecision",
            "checkpointPolicy",
        ):
            if key not in data:
                issues.append(f"contractVersion 2 requires field: {key}")
    if data.get("governanceMetadataVersion") == 1:
        for key in (
            "requiredEvidence",
            "humanDecisionPoints",
            "documentationImpact",
            "performanceImpact",
            "residualRiskExpectation",
            "predecessorClosureEvidence",
            "rollbackPlan",
        ):
            if key not in data:
                issues.append(f"governanceMetadataVersion 1 requires field: {key}")
    elif "governanceMetadataVersion" in data:
        issues.append("governanceMetadataVersion must be 1")

    risk = data.get("riskAssessment")
    if risk is not None:
        if not isinstance(risk, dict):
            issues.append("riskAssessment must be an object")
        else:
            if risk.get("level") not in RISK_LEVELS:
                issues.append(f"riskAssessment.level must be one of {sorted(RISK_LEVELS)}")
            risk_types = risk.get("riskTypes")
            if not isinstance(risk_types, list) or any(
                not non_empty_string(item) for item in risk_types
            ):
                issues.append("riskAssessment.riskTypes must be a list of non-empty strings")
            if not non_empty_string(risk.get("reason")):
                issues.append("riskAssessment.reason is required")

    capability = data.get("agentCapability")
    if capability is not None:
        if not isinstance(capability, dict):
            issues.append("agentCapability must be an object")
        else:
            for key in ("canImplement", "canVerify", "needsHumanDecision"):
                if not isinstance(capability.get(key), bool):
                    issues.append(f"agentCapability.{key} must be boolean")
            if "blockedReason" in capability and not isinstance(
                capability.get("blockedReason"), str
            ):
                issues.append("agentCapability.blockedReason must be a string")

    decision = data.get("executionDecision")
    if decision is not None:
        if not isinstance(decision, dict):
            issues.append("executionDecision must be an object")
        else:
            if decision.get("status") not in EXECUTION_STATUSES:
                issues.append(
                    f"executionDecision.status must be one of {sorted(EXECUTION_STATUSES)}"
                )
            if not non_empty_string(decision.get("reason")):
                issues.append("executionDecision.reason is required")

    if "archiveIndexRepair" in data and not isinstance(data.get("archiveIndexRepair"), bool):
        issues.append("archiveIndexRepair must be boolean")

    warnings = data.get("preReviewWarnings")
    if warnings is not None and (
        not isinstance(warnings, list) or any(not non_empty_string(item) for item in warnings)
    ):
        issues.append("preReviewWarnings must be a list of non-empty strings")

    checkpoint = data.get("checkpointPolicy")
    if checkpoint is not None:
        if not isinstance(checkpoint, dict):
            issues.append("checkpointPolicy must be an object")
        else:
            if "requiredBeforeFinish" in checkpoint and not isinstance(
                checkpoint.get("requiredBeforeFinish"), bool
            ):
                issues.append("checkpointPolicy.requiredBeforeFinish must be boolean")
            stages = checkpoint.get("requiredStages")
            if stages is not None and (
                not isinstance(stages, list) or any(not non_empty_string(item) for item in stages)
            ):
                issues.append("checkpointPolicy.requiredStages must be a list of non-empty strings")
            if "reason" in checkpoint and not non_empty_string(checkpoint.get("reason")):
                issues.append("checkpointPolicy.reason must be a non-empty string")

    synchronization_checkpoint = data.get("synchronizationCheckpoint")
    if synchronization_checkpoint is not None:
        if not isinstance(synchronization_checkpoint, dict):
            issues.append("synchronizationCheckpoint must be an object")
        else:
            if synchronization_checkpoint.get("authorized") is not True:
                issues.append("synchronizationCheckpoint.authorized must be true")
            if not non_empty_string(synchronization_checkpoint.get("reason")):
                issues.append("synchronizationCheckpoint.reason is required")

    issues.extend(validate_scenario_coverage(data.get("scenarioCoverage")))

    for key in ("requiredEvidence", "humanDecisionPoints"):
        if key in data:
            issues.extend(validate_string_list(data, key, allow_empty=False))
    for key in (
        "documentationImpact",
        "performanceImpact",
        "residualRiskExpectation",
        "predecessorClosureEvidence",
        "rollbackPlan",
    ):
        if key in data and not non_empty_string(data.get(key)):
            issues.append(f"{key} must be a non-empty string")

    return issues


def validate_baseline_and_approvals(data: dict[str, Any]) -> list[str]:
    issues: list[str] = []
    base = data.get("baseCommit")
    requires_baseline = data.get("contractVersion") == 2
    if requires_baseline and (not non_empty_string(base) or len(str(base).strip()) < 7):
        issues.append("baseCommit must be a non-empty Git commit identifier")
    dirty = data.get("baselineDirtyPaths")
    if requires_baseline and not isinstance(dirty, list):
        issues.append("baselineDirtyPaths must be a list")
    elif isinstance(dirty, list):
        for index, item in enumerate(dirty):
            if not isinstance(item, dict):
                issues.append(f"baselineDirtyPaths[{index}] must be an object")
                continue
            for key in ("path", "status", "fingerprint"):
                if not non_empty_string(item.get(key)):
                    issues.append(f"baselineDirtyPaths[{index}].{key} is required")

    bootstrap = data.get("adoptionBootstrapPaths")
    if bootstrap is not None:
        if data.get("workItemId") != "adopt_ai_cockpit":
            issues.append("adoptionBootstrapPaths is only allowed for workItemId adopt_ai_cockpit")
        if (
            not isinstance(bootstrap, list)
            or not bootstrap
            or any(not non_empty_string(item) for item in bootstrap)
        ):
            issues.append("adoptionBootstrapPaths must be a non-empty list of path patterns")

    destructive = data.get("destructiveChangePolicy")
    if not isinstance(destructive, dict):
        issues.append("destructiveChangePolicy must be an object")
    else:
        for key in ("allowed", "requiresHumanApproval"):
            if not isinstance(destructive.get(key), bool):
                issues.append(f"destructiveChangePolicy.{key} must be boolean")
        patterns = destructive.get("allowPatterns")
        if not isinstance(patterns, list) or any(not non_empty_string(item) for item in patterns):
            issues.append(
                "destructiveChangePolicy.allowPatterns must be a list of non-empty strings"
            )
        if patterns and destructive.get("allowed") is not True:
            issues.append("destructiveChangePolicy.allowPatterns require allowed true")
        evidence = destructive.get("approvalEvidence")
        if destructive.get("allowed") is True and destructive.get("requiresHumanApproval") is True:
            if not isinstance(evidence, dict) or evidence.get("approved") is not True:
                issues.append("destructive changes require approvalEvidence.approved true")
            elif not non_empty_string(evidence.get("approvedBy")) or not non_empty_string(
                evidence.get("reason")
            ):
                issues.append("destructive approvalEvidence requires approvedBy and reason")
            if isinstance(evidence, dict):
                identity_evidence = evidence.get("identityEvidence")
                identity_issues = high_risk_approval_issues(
                    identity_evidence or evidence, required_scope=patterns
                )
                issues.extend(
                    f"destructive approval identity: {issue}" for issue in identity_issues
                )

    approval = data.get("restrictedWriteApproval")
    if approval is not None:
        if not isinstance(approval, dict):
            issues.append("restrictedWriteApproval must be an object")
        else:
            if not isinstance(approval.get("approved"), bool):
                issues.append("restrictedWriteApproval.approved must be boolean")
            if approval.get("approved") is True and (
                not non_empty_string(approval.get("approvedBy"))
                or not non_empty_string(approval.get("reason"))
            ):
                issues.append("approved restrictedWriteApproval requires approvedBy and reason")
    return issues


def validate_intent(data: dict[str, Any]) -> list[str]:
    """intent フィールドのオプション構造を検証する。

    intent セクション全体が省略されている場合は何も検証しない。
    存在する場合は、許可されたキーと各フィールドの型のみを検査する。
    全フィールドは任意であり、存在しないこと・空であることを許容する。
    """
    issues: list[str] = []
    intent = data.get("intent")
    if intent is None:
        return issues
    if not isinstance(intent, dict):
        issues.append("intent must be an object")
        return issues
    allowed_keys = INTENT_STRING_KEYS | INTENT_LIST_KEYS
    for key in intent:
        if key not in allowed_keys:
            issues.append(f"intent.{key} is not a recognized field")
    for key in INTENT_STRING_KEYS:
        value = intent.get(key)
        if value is not None and not non_empty_string(value):
            issues.append(f"intent.{key} must be a non-empty string when provided")
    for key in INTENT_LIST_KEYS:
        value = intent.get(key)
        if value is not None and (
            not isinstance(value, list) or any(not non_empty_string(item) for item in value)
        ):
            issues.append(f"intent.{key} must be a list of non-empty strings when provided")
    return issues


PLACEHOLDER_MARKERS = (
    "initial skeleton",
    "replace this",
    "replace with",
    "not verified",
    "new feature",
    "user documentation",
)


def validate_semantic_placeholders(data: dict[str, Any]) -> list[str]:
    """Reject generic starter text in active v2 code Contracts."""
    if data.get("contractVersion") != 2 or data.get("mode") != "code":
        return []
    issues: list[str] = []

    def scan(value: Any, location: str) -> None:
        if isinstance(value, str):
            lowered = value.casefold()
            if any(marker in lowered for marker in PLACEHOLDER_MARKERS):
                issues.append(f"placeholder content remains in {location}")
        elif isinstance(value, dict):
            for key, child in value.items():
                scan(child, f"{location}.{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                scan(child, f"{location}[{index}]")

    for field in ("intent", "sources", "acceptance", "scenarioCoverage"):
        scan(data.get(field), field)
    return issues


RAW_REQUEST_EXEMPTIONS = {
    "system_maintenance",
    "dependency_upgrade",
    "release_metadata",
    "internal_governance",
}
RAW_REQUEST_EXEMPTION_FIELDS = {
    "exemption",
    "policyRef",
    "triggerRef",
    "applicability",
    "approvedBy",
}
RAW_REQUEST_TRIGGER_REFS = {
    "scheduled-maintenance",
    "automated-dependency-update",
    "release-automation",
    "internal-governance",
}
RAW_REQUEST_APPLICABILITY = {"repository", "sandbox", "test"}
RAW_REQUEST_SOURCE_TYPES = {"human", "issue", "pr_comment", "system"}
REQUESTED_OPERATION_FIELDS = ("target", "action", "environment", "effect")


def validate_raw_request_requirement(data: dict[str, Any]) -> list[str]:
    """Require portable raw-request provenance for full v2 code Work Items."""
    scope = data.get("scope")
    required = (
        data.get("contractVersion") == 2
        and data.get("mode") == "code"
        and isinstance(scope, list)
        and any(isinstance(item, str) and ".ai/work-items/active/" in item for item in scope)
    )
    raw = data.get("rawUserRequest")
    exemption = data.get("rawRequestExemption")
    if required and raw is None:
        if (
            not isinstance(exemption, dict)
            or exemption.get("exemption") not in RAW_REQUEST_EXEMPTIONS
        ):
            return [
                "rawUserRequest is required unless rawRequestExemption is a registered structured record"
            ]
        missing = RAW_REQUEST_EXEMPTION_FIELDS - set(exemption)
        if (
            missing
            or set(exemption) != RAW_REQUEST_EXEMPTION_FIELDS
            or exemption.get("policyRef") != "raw-request-exemptions.v1"
            or not isinstance(exemption.get("triggerRef"), str)
            or exemption["triggerRef"] not in RAW_REQUEST_TRIGGER_REFS
            or not isinstance(exemption.get("applicability"), list)
            or not exemption["applicability"]
            or not set(exemption["applicability"]).issubset(RAW_REQUEST_APPLICABILITY)
            or data.get("riskAssessment", {}).get("level") == "high"
        ):
            return [
                "rawRequestExemption must include approved policy, trigger, applicability, approver, and cannot exempt high-risk work"
            ]
        return []
    if raw is None:
        return []
    issues: list[str] = []
    if not isinstance(raw, str) or not raw.strip():
        issues.append("rawUserRequest must be a non-empty string")
    source = data.get("rawRequestSource")
    if not isinstance(source, dict):
        issues.append("rawRequestSource must be declared when rawUserRequest is present")
        return issues
    if source.get("type") not in RAW_REQUEST_SOURCE_TYPES:
        issues.append("rawRequestSource.type must be human, issue, pr_comment, or system")
    for field in ("reference", "capturedAt", "digest"):
        if not isinstance(source.get(field), str) or not source[field].strip():
            issues.append(f"rawRequestSource.{field} must be a non-empty string")
    return issues


def validate_requested_operation(data: dict[str, Any]) -> list[str]:
    required = (
        data.get("contractVersion") == 2
        and data.get("mode") == "code"
        and isinstance(data.get("scope"), list)
        and any(
            isinstance(item, str) and ".ai/work-items/active/" in item for item in data["scope"]
        )
    )
    operation = data.get("requestedOperation")
    if not required and operation is None:
        return []
    if not isinstance(operation, dict):
        return ["requestedOperation is required as an object for active MODE=code Work Items"]
    issues = [
        f"requestedOperation.{field} must be a non-empty string"
        for field in REQUESTED_OPERATION_FIELDS
        if not non_empty_string(operation.get(field))
    ]
    if not isinstance(operation.get("authorityRequired"), bool):
        issues.append("requestedOperation.authorityRequired must be boolean")
    return issues


def validate_required_evidence_context(data: dict[str, Any]) -> list[str]:
    """Validate optional structured inputs used by the rule engine."""
    context = data.get("requiredEvidenceContext")
    if context is None:
        return []
    if not isinstance(context, dict):
        return ["requiredEvidenceContext must be an object"]
    issues: list[str] = []
    allowed = {"destructiveLevel", "availableEvidence", "externalSystem"}
    for key in context:
        if key not in allowed:
            issues.append(f"requiredEvidenceContext.{key} is not allowed")
    level = context.get("destructiveLevel", "none")
    if level not in {"none", "delete"}:
        issues.append("requiredEvidenceContext.destructiveLevel must be none or delete")
    available = context.get("availableEvidence", [])
    if not isinstance(available, list) or any(not non_empty_string(item) for item in available):
        issues.append(
            "requiredEvidenceContext.availableEvidence must be a list of non-empty strings"
        )
    external = context.get("externalSystem", "")
    if not isinstance(external, str):
        issues.append("requiredEvidenceContext.externalSystem must be a string")
    return issues


IMPLEMENTATION_SURFACE_KEYS = ("production", "tests", "generated", "documentation")


def validate_implementation_surface(data: dict[str, Any]) -> list[str]:
    """Validate planned edit paths before the immutable implementation checkpoint."""
    surface = data.get("implementationSurface")
    if surface is None:
        return []
    if not isinstance(surface, dict):
        return ["implementationSurface must be an object"]

    issues: list[str] = []
    for key in surface:
        if key not in IMPLEMENTATION_SURFACE_KEYS:
            issues.append(f"implementationSurface.{key} is not allowed")
    paths_by_kind: dict[str, list[str]] = {}
    for kind in IMPLEMENTATION_SURFACE_KEYS:
        paths = surface.get(kind)
        if not isinstance(paths, list) or any(not non_empty_string(path) for path in paths):
            issues.append(f"implementationSurface.{kind} must be a list of non-empty paths")
            continue
        normalized = [path.strip() for path in paths]
        if len(normalized) != len(set(normalized)):
            issues.append(f"implementationSurface.{kind} must not contain duplicate paths")
        for path in normalized:
            if (
                path.startswith("/")
                or ".." in Path(path).parts
                or any(token in path for token in "*?[")
            ):
                issues.append(
                    f"implementationSurface.{kind} path must be a concrete repository-relative path: {path}"
                )
        paths_by_kind[kind] = normalized

    production = paths_by_kind.get("production", [])
    tests = paths_by_kind.get("tests", [])
    if production and not tests:
        issues.append("implementationSurface.production requires at least one tests path")

    scope = [path for path in data.get("scope", []) if isinstance(path, str)]
    out_of_scope = [path for path in data.get("outOfScope", []) if isinstance(path, str)]
    declared_paths = [path for paths in paths_by_kind.values() for path in paths]
    for kind, paths in paths_by_kind.items():
        for path in paths:
            if any(matches(pattern, path) for pattern in out_of_scope):
                issues.append(f"implementationSurface.{kind} path is covered by outOfScope: {path}")
            if not any(matches(pattern, path) for pattern in scope):
                issues.append(f"implementationSurface.{kind} path is not covered by scope: {path}")

    approval = data.get("restrictedWriteApproval")
    approved = isinstance(approval, dict) and approval.get("approved") is True
    guard_items = detect_guard_items(declared_paths, restricted_approved=approved)
    restricted = [item.path for item in guard_items if item.kind == "restricted_write"]
    if restricted and not approved:
        issues.append(
            "implementationSurface restricted paths require approved restrictedWriteApproval: "
            + ", ".join(restricted)
        )
    forbidden = [
        item.path for item in guard_items if item.kind in {"forbidden_write", "forbidden_boundary"}
    ]
    if forbidden:
        issues.append(
            "implementationSurface cannot declare forbidden paths: " + ", ".join(forbidden)
        )
    return issues


def validate_contract(data: dict[str, Any], contract_path: str = "") -> list[str]:
    issues: list[str] = []
    for key in REQUIRED_FIELDS:
        if key not in data:
            issues.append(f"missing field: {key}")
    for key in data:
        if key not in ALLOWED_FIELDS:
            issues.append(f"unknown field: {key}")

    if data.get("contractVersion") not in {1, 2}:
        issues.append("contractVersion must be 1 or 2")
    if data.get("mode") not in MODES:
        issues.append(f"mode must be one of {sorted(MODES)}")
    if contract_path:
        stem = Path(contract_path).name.removesuffix(".contract.json")
        if stem and data.get("workItemId") != stem:
            issues.append("workItemId does not match the Contract filename")
    for key in ("workItemId", "title", "rollbackNote"):
        if key in data and not non_empty_string(data.get(key)):
            issues.append(f"{key} must be a non-empty string")

    issues.extend(validate_string_list(data, "scope", allow_empty=False))
    issues.extend(validate_string_list(data, "outOfScope", allow_empty=True))
    issues.extend(validate_string_list(data, "unknowns", allow_empty=True))
    issues.extend(validate_string_list(data, "acceptance", allow_empty=False))
    if "guidelines" in data:
        issues.extend(validate_string_list(data, "guidelines", allow_empty=True))
    issues.extend(validate_sources(data))
    issues.extend(validate_verification(data))
    issues.extend(validate_optional_readiness(data))
    issues.extend(validate_baseline_and_approvals(data))
    issues.extend(validate_intent(data))
    issues.extend(validate_semantic_placeholders(data))
    issues.extend(validate_raw_request_requirement(data))
    issues.extend(validate_requested_operation(data))
    issues.extend(validate_required_evidence_context(data))
    issues.extend(validate_implementation_surface(data))
    policy = data.get("sourceBoundGeneratedEvidence")
    if policy is not None:
        if not isinstance(policy, dict) or set(policy) != {"mode", "generatedPaths"}:
            issues.append("sourceBoundGeneratedEvidence must contain only mode and generatedPaths")
        elif policy.get("mode") != SOURCE_BOUND_GENERATED_EVIDENCE_MODE or set(
            policy.get("generatedPaths", [])
        ) != set(SOURCE_BOUND_GENERATED_DOCUMENTATION_PATHS):
            issues.append(
                "sourceBoundGeneratedEvidence.generatedPaths must declare exactly the canonical generated paths"
            )
    issues.extend(validate_governance_profile(data))
    issues.extend(validate_operation_escalations(data))
    issues.extend(validate_concurrency_boundary(data))
    issues.extend(validate_calibration_corrective(data))
    if "problemStatement" in data and not non_empty_string(data.get("problemStatement")):
        issues.append("problemStatement must be a non-empty string")

    if not isinstance(data.get("notCodable"), bool):
        issues.append("notCodable must be boolean")
    if data.get("mode") == "code" and data.get("notCodable"):
        issues.append("mode code cannot run with notCodable true")
    if data.get("mode") == "code" and data.get("unknowns"):
        issues.append("mode code cannot run while unknowns remain")
    if data.get("notCodable") or data.get("unknowns"):
        decision = data.get("executionDecision")
        status = decision.get("status") if isinstance(decision, dict) else ""
        if status == "continue":
            issues.append(
                "unknowns or notCodable require executionDecision.status other than continue"
            )

    def scan_machine_paths(value: Any, location: str) -> None:
        if isinstance(value, str) and contains_machine_path(value):
            issues.append(f"{location} contains a machine-specific path")
        elif isinstance(value, dict):
            for key, child in value.items():
                scan_machine_paths(child, f"{location}.{key}")
        elif isinstance(value, list):
            for index, child in enumerate(value):
                scan_machine_paths(child, f"{location}[{index}]")

    scan_machine_paths(data, "contract")
    if contract_path and ".ai/work-items/active/" in Path(contract_path).as_posix():
        corrective = data.get("calibrationCorrective")
        if corrective is not None:
            binding_issue = calibration_corrective_binding_issue(corrective, root=PROJECT_ROOT)
            if binding_issue:
                issues.append(binding_issue.removeprefix("ERROR: "))
        receipt_file = receipt_path(str(data.get("workItemId", "")))
        try:
            receipt = load_json(receipt_file)
        except (OSError, json.JSONDecodeError, ValueError):
            receipt = None
        issues.extend(validate_receipt(data, receipt, require_tracked=False))
    return issues


def main() -> int:
    if len(sys.argv) < 2 or not sys.argv[1]:
        print("Skipping work item check (no active contract provided)")
        return 0
    path = Path(sys.argv[1])
    start = time.time()
    try:
        data = load_json(path)
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"Failed to read Work Item Contract: {exc}", file=sys.stderr)
        return 1

    obs = create_observability(work_item_id=data.get("workItemId", ""))
    issues = validate_contract(data, contract_path=path.as_posix())
    duration = elapsed_ms(start)
    if issues:
        for issue in issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        obs.check_failed(
            check_id="aiWorkItem", duration_ms=duration, detail=f"{len(issues)} issue(s)"
        )
        return 1
    print(f"work item contract check passed: {path}")
    obs.check_passed(check_id="aiWorkItem", duration_ms=duration)
    return 0


if __name__ == "__main__":
    sys.exit(main())
