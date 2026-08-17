#!/usr/bin/env python3
"""Verify changed files are covered by Work Item scope."""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Any

from ai_common import changed_paths, included, load_json, matches, simple_yaml_lists
from ai_evidence_dependencies import (
    EvidenceDependencyError,
    changed_path_dependency_issues,
    load_capability_evidence_dependencies,
    source_bound_generated_evidence_change_issues,
)
from ai_observability import create_observability, elapsed_ms

PROJECT_ROOT = Path(__file__).resolve().parents[1]
SCOPE_POLICY = PROJECT_ROOT / ".ai" / "guards" / "scope_policy.yaml"


def string_list(data: dict[str, Any], key: str) -> list[str]:
    value = data.get(key, [])
    return [item for item in value if isinstance(item, str)]


def dependency_scope_issues(
    contract: dict[str, Any], paths: list[str], policy_lists: dict[str, list[str]]
) -> list[str]:
    bootstrap_patterns = string_list(contract, "adoptionBootstrapPaths")
    dependency_paths = [path for path in paths if not included(path, bootstrap_patterns)]
    issues = []
    dependency_rules = {
        key.removeprefix("dependencyScopeRules."): values
        for key, values in policy_lists.items()
        if key.startswith("dependencyScopeRules.")
    }
    for trigger, required_patterns in dependency_rules.items():
        if any(included(path, [trigger]) for path in dependency_paths):
            for required_pattern in required_patterns:
                if not any(included(path, [required_pattern]) for path in paths):
                    issues.append(
                        f"dependency scope rule requires {required_pattern} when {trigger} changes"
                    )
    return issues


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate Work Item scope against the current diff."
    )
    parser.add_argument("contract", nargs="?")
    parser.add_argument("--verbose", action="store_true", help="Print debug paths detail.")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if not args.contract:
        print("Skipping scope check (no active contract provided)")
        return 0
    start = time.time()
    try:
        contract = load_json(Path(args.contract))
        paths = changed_paths(contract)
    except (OSError, json.JSONDecodeError, ValueError, RuntimeError) as exc:
        print(f"Failed to run scope guard: {exc}", file=sys.stderr)
        return 1

    obs = create_observability(work_item_id=contract.get("workItemId", ""))
    scope = string_list(contract, "scope")
    receipt_binding = contract.get("startReceipt")
    if isinstance(receipt_binding, dict) and isinstance(receipt_binding.get("path"), str):
        scope.append(receipt_binding["path"])
    work_item_id = contract.get("workItemId")
    if isinstance(work_item_id, str) and work_item_id:
        scope.extend(
            [
                f".ai/work-items/active/{work_item_id}.contract.json",
                f".ai/work-items/active/{work_item_id}.summary.json",
                f".ai/work-items/active/{work_item_id}.outcome.json",
                f".ai/work-items/active/{work_item_id}.outcome.md",
            ]
        )
    out_of_scope = string_list(contract, "outOfScope")
    policy_lists = simple_yaml_lists(SCOPE_POLICY)
    allow_patterns = policy_lists.get("allowAlways", [])
    destructive = contract.get("destructiveChangePolicy")
    if isinstance(destructive, dict) and destructive.get("allowed") is True:
        evidence = destructive.get("approvalEvidence")
        approved = destructive.get("requiresHumanApproval") is False or (
            isinstance(evidence, dict) and evidence.get("approved") is True
        )
        if approved:
            allow_patterns.extend(
                item for item in destructive.get("allowPatterns", []) if isinstance(item, str)
            )

    issues: list[str] = []
    for path in paths:
        matched_allow = [pat for pat in allow_patterns if matches(pat, path)]
        if matched_allow:
            if args.verbose:
                print(f"[DEBUG] {path} matches allowAlways pattern: '{matched_allow[0]}'")
            continue
        matched_out = [pat for pat in out_of_scope if matches(pat, path)]
        if matched_out:
            issues.append(f"path matches outOfScope pattern '{matched_out[0]}': {path}")
            obs.guard_violation(
                check_id="aiScope",
                severity="error",
                path=path,
                detail=f"matches_out_of_scope: {matched_out[0]}",
            )
        matched_scope = [pat for pat in scope if matches(pat, path)]
        if not matched_scope:
            issues.append(f"path is not covered by any pattern in scope: {path}")
            obs.guard_violation(
                check_id="aiScope", severity="error", path=path, detail="not_covered_by_scope"
            )
        else:
            if args.verbose:
                print(f"[DEBUG] {path} is covered by scope pattern: '{matched_scope[0]}'")

    try:
        dependencies = load_capability_evidence_dependencies(PROJECT_ROOT)
    except EvidenceDependencyError as exc:
        issue = str(exc)
        issues.append(issue)
        obs.guard_violation(
            check_id="aiScope",
            severity="error",
            path="capabilityEvidenceDependencies",
            detail=issue,
        )
    else:
        if dependencies:
            for issue in changed_path_dependency_issues(paths, dependencies):
                issues.append(issue)
                obs.guard_violation(
                    check_id="aiScope",
                    severity="error",
                    path="capabilityEvidenceDependencies",
                    detail=issue,
                )
            issues.extend(
                source_bound_generated_evidence_change_issues(contract, paths, dependencies)
            )

    for issue in dependency_scope_issues(contract, paths, policy_lists):
        issues.append(issue)
        obs.guard_violation(
            check_id="aiScope", severity="error", path="adoptionBootstrapPaths", detail=issue
        )

    duration = elapsed_ms(start)
    if issues:
        for issue in issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        obs.check_failed(check_id="aiScope", duration_ms=duration, detail=f"{len(issues)} issue(s)")
        return 1
    print(f"scope guard passed: {len(paths)} changed path(s) covered")
    obs.check_passed(check_id="aiScope", duration_ms=duration, fields={"changedPaths": len(paths)})
    return 0


if __name__ == "__main__":
    sys.exit(main())
