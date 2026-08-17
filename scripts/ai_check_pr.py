#!/usr/bin/env python3
"""Validate all changed archived Work Items against the complete PR diff."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from itertools import pairwise
from pathlib import Path
from typing import Any

from ai_check_summary import changed_file_paths, validate_summary
from ai_check_work_item import validate_contract
from ai_common import (
    PROJECT_ROOT,
    changed_name_status,
    changed_paths,
    contains_machine_path,
    first_match,
    included,
    load_json,
    parse_simple_manifest,
    run_git,
    simple_yaml_lists,
)
from ai_lifecycle_truth import (
    archived_outcome_projection,
    is_valid_superseded_transition,
    superseded_summary_validation_exception,
)
from ai_post_archive_recovery import RECEIPT_DIRECTORY, validate_recovery_receipt
from ai_start_receipt import validate_receipt, validate_resume_history_structure

SCOPE_POLICY = PROJECT_ROOT / ".ai" / "guards" / "scope_policy.yaml"
OWNERSHIP_POLICY = PROJECT_ROOT / ".ai" / "guards" / "file_ownership.yaml"
HUMAN_REPORT_JSON = ".ai/cockpit/task_report.json"
HUMAN_REPORT_MARKDOWN = ".ai/cockpit/task_report.md"


ARCHIVE_PREFIX = ".ai/work-items/archive/"
ARCHIVE_SUFFIXES = (".contract.json", ".summary.json", ".review.json")
RECOVERY_RECEIPT_PREFIX = ".ai/work-items/recovery-receipts/"
# Worktree-bound verification evidence became mandatory at this migration point.
# Archives created before it remain immutable historical evidence.
WORKTREE_DIGEST_INTRODUCED_AT = "63ec6fcd3c8f945b379966d43457e44ccaeba258"
# New archive pairs use explicit ordering evidence. Older pairs remain readable
# through the timestamp fallback and are never rewritten in place.
ARCHIVE_SEQUENCE_INTRODUCED_AT = "f0b7caa9fdc8fa0bc25cf8c099fc2cef5f0c61b7"
NEW_WORK_ITEM_SEQUENCE = 74
ARCHIVE_BOUND_RELEASE_METADATA = frozenset(
    {
        ".ai/cockpit/release-digests.json",
        ".ai/cockpit/release-freeze.json",
        "docs/reference/capability-truth-matrix.json",
        "release-state.json",
        "release.json",
    }
)


def _git_blob_hash(revision: str, path: str) -> str:
    """Return the git object hash of *path* at *revision*, or empty string on error."""
    result = run_git(["rev-parse", f"{revision}:{path}"])
    return result.stdout.strip() if result.returncode == 0 else ""


def _worktree_blob_hash(path: str) -> str:
    """Return the git blob hash for the current worktree copy of *path*."""
    result = run_git(["hash-object", "--no-filters", path])
    return result.stdout.strip() if result.returncode == 0 else ""


def _git_records(output: str) -> list[str]:
    if "\0" in output:
        return [item for item in output.split("\0") if item]
    return [line for line in output.splitlines() if line]


def _is_no_op_restore(base: str, path: str) -> bool:
    """Return True if the worktree restores *path* to an allowed PR baseline blob.

    This handles the case where an archive file was accidentally modified in a
    previous commit and the current change restores it to the merge-base content
    or to the direct parent of that base. The latter is needed when the PR base
    already contains the accidental archive edit. The archive integrity is fully
    preserved so append-only policy should not flag it.
    """
    worktree_blob = _worktree_blob_hash(path)
    if not worktree_blob:
        return False
    for revision in (base, f"{base}^"):
        baseline_blob = _git_blob_hash(revision, path)
        if baseline_blob and baseline_blob == worktree_blob:
            return True
    return False


def archive_evidence_changes(base: str) -> dict[str, str]:
    result: dict[str, str] = {}
    diff = run_git(["diff", "--name-status", "-z", f"{base}...HEAD"])
    saw_archive_evidence = False
    if diff.returncode == 0:
        ordered_changes: list[tuple[str, str]] = []
        records = _git_records(diff.stdout)
        if records and "\t" in records[0]:
            for line in records:
                parts = line.split("\t")
                if len(parts) < 2:
                    continue
                status = parts[0]
                if status.startswith(("R", "C")) and len(parts) >= 3:
                    ordered_changes.extend([("D", parts[1]), (parts[0], parts[2])])
                else:
                    ordered_changes.append((status, parts[-1]))
        else:
            i = 0
            while i < len(records):
                status = records[i]
                i += 1
                if status.startswith(("R", "C")):
                    if i + 1 >= len(records):
                        break
                    ordered_changes.extend([("D", records[i]), (status, records[i + 1])])
                    i += 2
                    continue
                if i >= len(records):
                    break
                ordered_changes.append((status, records[i]))
                i += 1
    else:
        ordered_changes = changed_name_status(
            {"baseCommit": base, "baselineDirtyPaths": []}, ignore_baseline_dirty=True
        )
    for status, path in ordered_changes:
        if not (path.startswith(ARCHIVE_PREFIX) and path.endswith(ARCHIVE_SUFFIXES)):
            continue
        # A later commit may restore a historical archive file to the exact
        # parent blob. It is not new evidence and must not become a PR owner.
        if status == "M" and _is_no_op_restore(base, path):
            continue
        saw_archive_evidence = True
        result[path] = status
    if not saw_archive_evidence:
        for status, path in changed_name_status(
            {"baseCommit": base, "baselineDirtyPaths": []}, ignore_baseline_dirty=True
        ):
            if not (path.startswith(ARCHIVE_PREFIX) and path.endswith(ARCHIVE_SUFFIXES)):
                continue
            result[path] = status
    return result


def archive_stem(path: str) -> str:
    for suffix in ARCHIVE_SUFFIXES:
        if path.endswith(suffix):
            return path[: -len(suffix)]
    raise ValueError(f"not an archive evidence path: {path}")


def archived_contract_paths(base: str) -> list[Path]:
    stems = dict.fromkeys(archive_stem(path) for path in archive_evidence_changes(base))
    return [PROJECT_ROOT / f"{stem}.contract.json" for stem in stems]


def archive_pair_rank(contract_path: Path, summary_path: Path) -> tuple[int, str, str]:
    try:
        contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
        summary_rel = summary_path.relative_to(PROJECT_ROOT).as_posix()
    except ValueError:
        return 0, contract_path.as_posix(), summary_path.as_posix()
    try:
        summary = load_json(summary_path)
    except (OSError, ValueError, json.JSONDecodeError):
        summary = {}
    sequence = summary.get("archiveSequence") if isinstance(summary, dict) else None
    if isinstance(sequence, int) and not isinstance(sequence, bool) and sequence > 0:
        return sequence, contract_rel, summary_rel
    result = run_git(["log", "-1", "--format=%ct", "--", contract_rel, summary_rel])
    if result.returncode != 0:
        return 0, contract_rel, summary_rel
    try:
        timestamp = int(result.stdout.strip())
    except ValueError:
        timestamp = 0
    return timestamp, contract_rel, summary_rel


def archive_sequence_required(contract: dict[str, Any]) -> bool:
    base_commit = contract.get("baseCommit")
    if not isinstance(base_commit, str) or not base_commit:
        return False
    result = run_git(["merge-base", "--is-ancestor", ARCHIVE_SEQUENCE_INTRODUCED_AT, base_commit])
    return result.returncode == 0


def archive_sequence_issue(contract: dict[str, Any], summary: dict[str, Any]) -> str | None:
    if not archive_sequence_required(contract):
        return None
    sequence = summary.get("archiveSequence")
    if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence < 1:
        return "archiveSequence must be a positive integer for new archive evidence"
    return None


def is_legacy_archive(contract: dict[str, Any], summary: dict[str, Any]) -> bool:
    """Return whether an archive pair predates strict worktree evidence."""
    if summary.get("summaryVersion") != 2:
        return True
    base_commit = contract.get("baseCommit")
    if not isinstance(base_commit, str) or not base_commit:
        return False
    result = run_git(["merge-base", "--is-ancestor", WORKTREE_DIGEST_INTRODUCED_AT, base_commit])
    return result.returncode != 0


def archive_base_is_compatible(contract: dict[str, Any], pr_base: str) -> bool:
    """Accept a frozen archive's historical base after a safe sequential rebase."""
    archived_base = contract.get("baseCommit")
    if not isinstance(archived_base, str) or not archived_base:
        return False
    receipt = contract.get("startReceipt")
    if archived_base == pr_base and not isinstance(receipt, dict):
        return True
    if not isinstance(receipt, dict):
        return False
    if not isinstance(receipt.get("path"), str) or not receipt["path"]:
        return False
    receipt_base = receipt.get("baseCommit")
    if not isinstance(receipt_base, str) or not receipt_base:
        return False
    if receipt_base != archived_base and validate_resume_history_structure(contract, receipt_base):
        return False
    if (
        receipt_base == archived_base
        and contract.get("resumeHistory") is not None
        and validate_resume_history_structure(contract, receipt_base)
    ):
        return False
    if archived_base == pr_base:
        return True
    return run_git(["merge-base", "--is-ancestor", archived_base, pr_base]).returncode == 0


def source_references_archive_pair(contract: dict[str, Any], contract_path: Path) -> bool:
    """Return whether a Contract sources the exact predecessor Contract or paired Summary."""
    try:
        contract_source = contract_path.relative_to(PROJECT_ROOT).as_posix()
    except ValueError:
        return False
    summary_source = contract_source.replace(".contract.json", ".summary.json")
    expected = {contract_source, summary_source}
    return any(
        isinstance(source, dict) and source.get("path") in expected
        for source in contract.get("sources", [])
    )


def is_documented_pr_recovery_pair(
    predecessor: tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]],
    recovery: tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]],
    pr_base: str,
    *,
    require_pr_base_compatibility: bool = True,
) -> bool:
    """Accept only one auditable, immediately sequential recovery relationship."""
    predecessor_path, predecessor_contract, predecessor_summary, _ = predecessor
    _, recovery_contract, recovery_summary, _ = recovery
    predecessor_base = predecessor_contract.get("baseCommit")
    recovery_base = recovery_contract.get("baseCommit")
    approval = recovery_contract.get("restrictedWriteApproval")
    receipt = recovery_contract.get("startReceipt")
    request_source = recovery_contract.get("rawRequestSource")
    return (
        isinstance(predecessor_summary.get("archiveSequence"), int)
        and isinstance(recovery_summary.get("archiveSequence"), int)
        and recovery_summary["archiveSequence"] == predecessor_summary["archiveSequence"] + 1
        and (
            not require_pr_base_compatibility
            or archive_base_is_compatible(predecessor_contract, pr_base)
        )
        and isinstance(predecessor_base, str)
        and isinstance(recovery_base, str)
        and run_git(["merge-base", "--is-ancestor", predecessor_base, recovery_base]).returncode
        == 0
        and source_references_archive_pair(recovery_contract, predecessor_path)
        and isinstance(approval, dict)
        and approval.get("approved") is True
        and isinstance(approval.get("approvedBy"), str)
        and bool(approval["approvedBy"].strip())
        and isinstance(receipt, dict)
        and receipt.get("baseCommit") == recovery_base
        and isinstance(receipt.get("path"), str)
        and bool(receipt["path"])
        and isinstance(request_source, dict)
        and request_source.get("type") == "human"
    )


def documented_recovery_paths(
    entries: list[tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]]], pr_base: str
) -> set[Path]:
    """Return recovery Contracts only when every adjacent chain link is auditable."""
    new_entries = sorted(
        [
            entry
            for entry in entries
            if isinstance(entry[2].get("archiveSequence"), int)
            and entry[2].get("archiveSequence", 0) >= NEW_WORK_ITEM_SEQUENCE
            and entry[1].get("workItemId")
        ],
        key=lambda entry: entry[3],
    )
    root_is_compatible = bool(
        new_entries and archive_base_is_compatible(new_entries[0][1], pr_base)
    )
    if (
        len(new_entries) >= 2
        and root_is_compatible
        and all(
            is_documented_pr_recovery_pair(
                predecessor,
                recovery,
                pr_base,
                require_pr_base_compatibility=False,
            )
            for predecessor, recovery in pairwise(new_entries)
        )
    ):
        return {entry[0] for entry in new_entries[1:]}
    return set()


def recovery_receipt_entry(
    entry: tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]],
) -> dict[str, Any]:
    """Return the immutable archive identity recorded by a recovery receipt."""
    contract_path, contract, summary, _rank = entry
    summary_path = Path(str(contract_path).replace(".contract.json", ".summary.json"))
    return {
        "workItemId": contract.get("workItemId"),
        "contractPath": contract_path.relative_to(PROJECT_ROOT).as_posix(),
        "summaryPath": summary_path.relative_to(PROJECT_ROOT).as_posix(),
        "baseCommit": contract.get("baseCommit"),
        "archiveSequence": summary.get("archiveSequence"),
        "contractDigest": hashlib.sha256(contract_path.read_bytes()).hexdigest(),
        "summaryDigest": hashlib.sha256(summary_path.read_bytes()).hexdigest(),
    }


def historical_recovery_receipt_paths(
    entries: list[tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]]],
    pr_base: str,
    receipt: Any,
) -> set[Path]:
    """Return only receipt-bound historical recovery entries.

    An archive cannot be rewritten merely to add a predecessor source.  This
    narrow path instead requires an append-only receipt that exactly reproduces
    a consecutive prefix of the current PR's archive identities and ancestry.
    """
    if not isinstance(receipt, dict) or receipt.get("receiptVersion") != 1:
        return set()
    if receipt.get("prBaseCommit") != pr_base:
        return set()
    authorization = receipt.get("humanAuthorization")
    if not (
        isinstance(authorization, dict)
        and authorization.get("type") == "human"
        and isinstance(authorization.get("reference"), str)
        and authorization["reference"].strip()
    ):
        return set()
    recorded = receipt.get("archives")
    if not isinstance(recorded, list) or len(recorded) < 2:
        return set()
    new_entries = sorted(
        [
            entry
            for entry in entries
            if isinstance(entry[2].get("archiveSequence"), int)
            and entry[2].get("archiveSequence", 0) >= NEW_WORK_ITEM_SEQUENCE
            and entry[1].get("workItemId")
        ],
        key=lambda entry: entry[3],
    )
    prefix = new_entries[: len(recorded)]
    if len(prefix) != len(recorded) or recorded != [
        recovery_receipt_entry(entry) for entry in prefix
    ]:
        return set()
    if not archive_base_is_compatible(prefix[0][1], pr_base):
        return set()
    for predecessor, recovery in pairwise(prefix):
        predecessor_contract, predecessor_summary = predecessor[1], predecessor[2]
        recovery_contract, recovery_summary = recovery[1], recovery[2]
        predecessor_sequence = predecessor_summary.get("archiveSequence")
        recovery_sequence = recovery_summary.get("archiveSequence")
        if not (
            isinstance(predecessor_sequence, int)
            and not isinstance(predecessor_sequence, bool)
            and isinstance(recovery_sequence, int)
            and not isinstance(recovery_sequence, bool)
            and recovery_sequence == predecessor_sequence + 1
        ):
            return set()
        predecessor_base = predecessor_contract.get("baseCommit")
        recovery_base = recovery_contract.get("baseCommit")
        if not (
            isinstance(predecessor_base, str)
            and isinstance(recovery_base, str)
            and run_git(["merge-base", "--is-ancestor", predecessor_base, recovery_base]).returncode
            == 0
        ):
            return set()
    return {entry[0] for entry in prefix[1:]}


def extend_documented_recovery_paths(
    entries: list[tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]]],
    pr_base: str,
    trusted_paths: set[Path],
) -> set[Path]:
    """Extend a verified historical prefix only through normal adjacent links."""
    ordered = sorted(entries, key=lambda entry: entry[3])
    trusted = set(trusted_paths)
    for predecessor, recovery in pairwise(ordered):
        if predecessor[0] in trusted and is_documented_pr_recovery_pair(
            predecessor, recovery, pr_base, require_pr_base_compatibility=False
        ):
            trusted.add(recovery[0])
    return trusted


def historical_recovery_receipts() -> list[tuple[str, Any]]:
    """Load append-only receipts; invalid content is returned for fail-closed validation."""
    directory = PROJECT_ROOT / RECOVERY_RECEIPT_PREFIX
    if not directory.is_dir():
        return []
    receipts: list[tuple[str, Any]] = []
    for path in sorted(directory.glob("*.json")):
        relative = path.relative_to(PROJECT_ROOT).as_posix()
        try:
            receipts.append((relative, load_json(path)))
        except (OSError, json.JSONDecodeError, ValueError) as exc:
            receipts.append((relative, {"_loadError": str(exc)}))
    return receipts


def same_work_item_recovery_paths(
    base: str,
    entries: list[tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]]],
) -> tuple[dict[str, set[str]], set[str], list[str]]:
    """Return only receipt-bound repair paths for archives in this exact PR."""
    known_tasks = {
        contract.get("workItemId")
        for _path, contract, _summary, _rank in entries
        if isinstance(contract.get("workItemId"), str)
    }
    permitted: dict[str, set[str]] = {}
    receipts: set[str] = set()
    blockers: list[str] = []
    directory = PROJECT_ROOT / RECEIPT_DIRECTORY
    if not directory.is_dir():
        return permitted, receipts, blockers
    for path in sorted(directory.glob("*.json")):
        try:
            receipt = load_json(path)
        except (OSError, json.JSONDecodeError, ValueError):
            continue
        if not isinstance(receipt, dict) or receipt.get("workItemId") not in known_tasks:
            continue
        receipt_issues = validate_recovery_receipt(PROJECT_ROOT, receipt, pr_base=base)
        if receipt_issues:
            blockers.append(
                "BLOCKED: provider-bound recovery receipt cannot be verified: "
                f"{'; '.join(receipt_issues)}. Recovery: regenerate the recovery receipt "
                "from the failed hosted job."
            )
            continue
        values = receipt.get("recoveryPaths")
        if not isinstance(values, list) or not all(isinstance(value, str) for value in values):
            continue
        task = receipt["workItemId"]
        permitted.setdefault(task, set()).update(values)
        receipts.add(path.relative_to(PROJECT_ROOT).as_posix())
    return permitted, receipts, blockers


def archive_pair_addition_commits(contract_path: Path) -> list[str]:
    """Return commits that added this immutable Contract/Summary pair together."""
    try:
        contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
    except ValueError:
        return []
    summary_rel = contract_rel.replace(".contract.json", ".summary.json")
    candidates = _git_records(
        run_git(
            [
                "log",
                "--format=%H",
                "--diff-filter=A",
                "HEAD",
                "--",
                contract_rel,
                summary_rel,
            ]
        ).stdout
    )
    additions: list[str] = []
    for commit in candidates:
        changes = _git_records(
            run_git(["diff-tree", "--no-commit-id", "--name-status", "-r", commit]).stdout
        )
        added_paths = {
            record.split("\t", 1)[1]
            for record in changes
            if record.startswith("A\t") and "\t" in record
        }
        if {contract_rel, summary_rel}.issubset(added_paths):
            additions.append(commit)
    return additions


def has_valid_start_receipt(contract: dict[str, Any]) -> bool:
    """Return whether a Contract's canonical Start Receipt remains valid."""
    binding = contract.get("startReceipt")
    if not isinstance(binding, dict) or not isinstance(binding.get("path"), str):
        return False
    receipt_path = PROJECT_ROOT / binding["path"]
    try:
        receipt = load_json(receipt_path)
    except (OSError, json.JSONDecodeError, ValueError):
        return False
    return not validate_receipt(contract, receipt, project_root=PROJECT_ROOT, require_tracked=False)


def is_verified_merged_child_archive(
    entry: tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]], pr_base: str
) -> bool:
    """Accept a stacked child only after immutable pair and merge-parent proof.

    A child archive must have been added as one Contract/Summary pair and must
    enter the checked parent history through the second parent of a two-parent
    merge after the parent PR base.  HEAD reachability alone is insufficient:
    direct additions to the parent branch remain ordinary Work Items.
    """
    contract_path, contract, _summary, _rank = entry
    archived_base = contract.get("baseCommit")
    if not isinstance(archived_base, str) or not archived_base:
        return False
    if not has_valid_start_receipt(contract):
        return False
    if run_git(["merge-base", "--is-ancestor", archived_base, "HEAD"]).returncode != 0:
        return False
    merges = _git_records(
        run_git(["rev-list", "--merges", "--ancestry-path", f"{pr_base}..HEAD"]).stdout
    )
    if not merges:
        return False
    for addition in archive_pair_addition_commits(contract_path):
        for merge in merges:
            parents = _git_records(run_git(["show", "-s", "--format=%P", merge]).stdout)
            parent_ids = parents[0].split() if parents else []
            if len(parent_ids) != 2:
                continue
            first_parent, second_parent = parent_ids
            in_child = (
                run_git(["merge-base", "--is-ancestor", addition, second_parent]).returncode == 0
            )
            already_in_parent = (
                run_git(["merge-base", "--is-ancestor", addition, first_parent]).returncode == 0
            )
            if in_child and not already_in_parent:
                return True
    return False


def machine_path_issues(value: Any, location: str = "root") -> list[str]:
    issues: list[str] = []
    if isinstance(value, str) and contains_machine_path(value):
        issues.append(f"{location} contains a machine-specific path")
    elif isinstance(value, dict):
        for key, child in value.items():
            issues.extend(machine_path_issues(child, f"{location}.{key}"))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            issues.extend(machine_path_issues(child, f"{location}[{index}]"))
    return issues


def human_benefit_report_issues(contract_path: Path) -> list[str]:
    """Validate the committed Review Report against this archive's Outcome."""

    from ai_generate_human_report import validate_human_report

    outcome_path = contract_path.with_name(
        contract_path.name.replace(".contract.json", ".outcome.json")
    )
    report_path = PROJECT_ROOT / HUMAN_REPORT_JSON
    markdown_path = PROJECT_ROOT / HUMAN_REPORT_MARKDOWN
    missing = [
        path.relative_to(PROJECT_ROOT).as_posix()
        for path in (outcome_path, report_path, markdown_path)
        if not path.is_file()
    ]
    if missing:
        return ["Human Benefit Review Report evidence is missing: " + ", ".join(missing)]
    try:
        outcome = load_json(outcome_path)
        report = load_json(report_path)
        markdown = markdown_path.read_text(encoding="utf-8")
    except (OSError, ValueError, json.JSONDecodeError) as exc:
        return [f"Human Benefit Review Report cannot be loaded: {exc}"]
    work_item_id = outcome.get("workItemId") if isinstance(outcome, dict) else None
    if isinstance(work_item_id, str) and is_valid_superseded_transition(
        contract_path=contract_path,
        work_item_id=work_item_id,
    ):
        outcome = archived_outcome_projection(
            outcome,
            root=PROJECT_ROOT,
            contract_path=contract_path,
            work_item_id=work_item_id,
        )
    return validate_human_report(report, outcome, phase="review", markdown=markdown)


def validate_pr_bundle(base: str, contract_paths: list[Path]) -> list[str]:
    issues: list[str] = []
    evidence_changes = archive_evidence_changes(base)
    changed_stems = dict.fromkeys(
        archive_stem(path) for path, status in evidence_changes.items() if status == "A"
    )
    discovered_contracts = [
        PROJECT_ROOT / f"{archive_stem(path)}.contract.json"
        for path in evidence_changes
        if path.startswith(ARCHIVE_PREFIX) and path.endswith(ARCHIVE_SUFFIXES)
    ]
    contract_paths = list(dict.fromkeys([*contract_paths, *discovered_contracts]))

    # Collect no-op restore paths so they are exempt from the ownership check below.
    all_archive_changes = changed_name_status(
        {"baseCommit": base, "baselineDirtyPaths": []}, ignore_baseline_dirty=True
    )
    no_op_restore_paths: set[str] = {
        path
        for status, path in all_archive_changes
        if path.startswith(ARCHIVE_PREFIX)
        and path.endswith(ARCHIVE_SUFFIXES)
        and status == "M"
        and _is_no_op_restore(base, path)
    }

    for path, status in sorted(evidence_changes.items()):
        if status != "A":
            issues.append(
                f"archive PR policy is append-only; existing evidence path has status {status}: {path}"
            )
    for stem in sorted(changed_stems):
        contract_rel = f"{stem}.contract.json"
        summary_rel = f"{stem}.summary.json"
        if evidence_changes.get(contract_rel) != "A" or evidence_changes.get(summary_rel) != "A":
            issues.append(
                "new archive evidence must add its Contract and Summary together: "
                f"{contract_rel}, {summary_rel}"
            )

    if not contract_paths:
        return ["PR diff must contain at least one archived Work Item Contract"]

    archive_entries: list[tuple[Path, dict[str, Any], dict[str, Any], tuple[int, str, str]]] = []
    audit_paths: set[str] = set()
    for contract_path in list(dict.fromkeys(contract_paths)):
        summary_path = Path(str(contract_path).replace(".contract.json", ".summary.json"))
        if not contract_path.exists():
            issues.append(
                f"archived Contract is missing or deleted: {contract_path.relative_to(PROJECT_ROOT)}"
            )
            continue
        if not summary_path.exists():
            issues.append(
                f"archived Contract is missing Summary: {summary_path.relative_to(PROJECT_ROOT)}"
            )
            continue
        try:
            contract = load_json(contract_path)
            summary = load_json(summary_path)
        except (OSError, json.JSONDecodeError, ValueError) as exc:
            issues.append(f"failed to load archive pair {contract_path}: {exc}")
            continue
        archive_entries.append(
            (contract_path, contract, summary, archive_pair_rank(contract_path, summary_path))
        )
        contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
        summary_rel = summary_path.relative_to(PROJECT_ROOT).as_posix()
        audit_paths.update({contract_rel, summary_rel})
        receipt_binding = contract.get("startReceipt")
        if isinstance(receipt_binding, dict) and isinstance(receipt_binding.get("path"), str):
            receipt_rel = receipt_binding["path"]
            receipt_file = PROJECT_ROOT / receipt_rel
            audit_paths.add(receipt_rel)
            if not receipt_file.exists():
                issues.append(f"{contract_rel}: Start Receipt is missing: {receipt_rel}")
            else:
                try:
                    receipt = load_json(receipt_file)
                except (OSError, json.JSONDecodeError, ValueError) as exc:
                    receipt = None
                    issues.append(f"{contract_rel}: failed to load Start Receipt: {exc}")
                issues.extend(
                    f"{contract_rel}: {issue}"
                    for issue in validate_receipt(
                        contract,
                        receipt,
                        project_root=PROJECT_ROOT,
                        require_tracked=False,
                    )
                )
                base_blob = _git_blob_hash(base, receipt_rel)
                current_blob = _worktree_blob_hash(receipt_rel)
                if base_blob and base_blob != current_blob:
                    issues.append(f"{contract_rel}: Start Receipt was modified after its base")
        if contract.get("contractVersion") != 2:
            issues.append(f"{contract_rel}: PR archive evidence requires contractVersion 2")
        issues.extend(f"{contract_rel}: {issue}" for issue in validate_contract(contract))
        legacy_archive = is_legacy_archive(contract, summary)
        summary_issues = validate_summary(
            summary,
            contract,
            expected_contract_hash=hashlib.sha256(contract_path.read_bytes()).hexdigest(),
            contract_path=contract_rel,
            summary_path=summary_rel,
            legacy_archive=legacy_archive,
        )
        if not superseded_summary_validation_exception(
            contract_path=contract_path,
            work_item_id=str(contract.get("workItemId", "")),
            summary_issues=summary_issues,
        ):
            issues.extend(f"{summary_rel}: {issue}" for issue in summary_issues)
        sequence_issue = archive_sequence_issue(contract, summary)
        if sequence_issue:
            issues.append(f"{summary_rel}: {sequence_issue}")
        issues.extend(f"{contract_rel}: {issue}" for issue in machine_path_issues(contract))
        issues.extend(f"{summary_rel}: {issue}" for issue in machine_path_issues(summary))

    archive_entries.sort(key=lambda entry: entry[3])
    recovery_paths = documented_recovery_paths(archive_entries, base)
    historical_paths: set[Path] = set()
    for receipt_path, receipt in historical_recovery_receipts():
        # Receipts are immutable historical evidence.  They apply only to the
        # exact PR base and archive set they name; an old recovery must not
        # affect unrelated future PRs.
        if not isinstance(receipt, dict) or receipt.get("prBaseCommit") != base:
            continue
        recorded = receipt.get("archives")
        recorded_ids = (
            {entry.get("workItemId") for entry in recorded if isinstance(entry, dict)}
            if isinstance(recorded, list)
            else set()
        )
        entry_ids = {entry[1].get("workItemId") for entry in archive_entries}
        if not recorded_ids or not recorded_ids.issubset(entry_ids):
            continue
        candidate = historical_recovery_receipt_paths(archive_entries, base, receipt)
        if not candidate:
            issues.append(
                f"{receipt_path}: historical recovery receipt does not exactly bind a consecutive compatible archive prefix"
            )
            continue
        historical_paths.update(candidate)
    recovery_paths.update(extend_documented_recovery_paths(archive_entries, base, historical_paths))
    recovery_paths.update(
        entry[0]
        for entry in archive_entries
        if not archive_base_is_compatible(entry[1], base)
        and is_verified_merged_child_archive(entry, base)
    )
    same_item_recovery_paths, same_item_receipts, recovery_blockers = same_work_item_recovery_paths(
        base, archive_entries
    )
    audit_paths.update(same_item_receipts)
    issues.extend(recovery_blockers)

    for contract_path, contract, summary, _rank in archive_entries:
        if (
            isinstance(summary.get("archiveSequence"), int)
            and summary.get("archiveSequence", 0) >= NEW_WORK_ITEM_SEQUENCE
            and not archive_base_is_compatible(contract, base)
            and contract_path not in recovery_paths
        ):
            contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
            issues.append(
                f"{contract_rel}: Contract baseCommit is not compatible with the PR merge-base {base}; "
                "require exact base or a verified ancestor base with matching Start Receipt"
            )

    untrusted_new_work_items = {
        summary.get("workItemId")
        for path, contract, summary, _rank in archive_entries
        if isinstance(summary.get("archiveSequence"), int)
        and summary.get("archiveSequence", 0) >= NEW_WORK_ITEM_SEQUENCE
        and contract.get("workItemId")
        and path not in recovery_paths
    }
    if len(untrusted_new_work_items) > 1:
        issues.append(
            "PR must contain exactly one newly maintained Work Item; "
            "found "
            f"{len(untrusted_new_work_items)}: "
            f"{', '.join(sorted(str(item) for item in untrusted_new_work_items))}"
        )

    sequences: dict[int, str] = {}
    for contract_path, contract, summary, _rank in archive_entries:
        if not archive_sequence_required(contract):
            continue
        sequence = summary.get("archiveSequence")
        if not isinstance(sequence, int) or isinstance(sequence, bool) or sequence < 1:
            continue
        contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
        previous = sequences.get(sequence)
        if previous is not None:
            issues.append(
                f"archiveSequence {sequence} is duplicated by {previous} and {contract_rel}"
            )
        else:
            sequences[sequence] = contract_rel

    all_paths = changed_paths(
        {"baseCommit": base, "baselineDirtyPaths": []}, ignore_baseline_dirty=True
    )
    report_required = (
        HUMAN_REPORT_JSON in all_paths
        or HUMAN_REPORT_MARKDOWN in all_paths
        or bool(_git_blob_hash(base, HUMAN_REPORT_JSON))
        or any(
            HUMAN_REPORT_JSON in entry[1].get("scope", [])
            for entry in archive_entries
            if isinstance(entry[1].get("scope"), list)
        )
    )
    if report_required and archive_entries:
        issues.extend(human_benefit_report_issues(archive_entries[-1][0]))
    policy = simple_yaml_lists(SCOPE_POLICY)
    ownership = parse_simple_manifest(OWNERSHIP_POLICY)
    exempt = policy.get("allowAlways", [])

    generated_archive_index = f"{ARCHIVE_PREFIX}index.json"
    report_paths = {HUMAN_REPORT_JSON, HUMAN_REPORT_MARKDOWN}

    def current_archive_generated_paths() -> set[str]:
        """Return generated archive artifacts named by the current Summary.

        Archive artifacts are immutable and intentionally outside a Work Item's
        implementation scope.  Their ownership is the frozen Summary's exact
        changed-file projection, not the Contract's historical archive scope.
        """
        generated: set[str] = set()
        for contract_path, _contract, summary, _rank in archive_entries:
            stem = contract_path.name.replace(".contract.json", "")
            archive_dir = contract_path.parent.relative_to(PROJECT_ROOT).as_posix()
            candidates = {
                f"{archive_dir}/{stem}.outcome.json",
                f"{archive_dir}/{stem}.outcome.md",
                f"{archive_dir}/{stem}.archive-manifest.json",
            }
            summary_paths = set(changed_file_paths(summary))
            if candidates.issubset(summary_paths) and candidates.issubset(all_paths):
                generated.update(candidates)
        return generated

    def current_archive_owns_report_pair() -> bool:
        """Accept only a complete current archive transaction's exact report pair.

        The archive transaction regenerates these reports after moving active
        evidence, so their durable ownership is the archived Summary rather
        than a broad Contract path.  Both reports and the corresponding
        archived Outcome must be part of this PR and validate together.
        """
        if not report_paths.issubset(all_paths):
            return False
        for contract_path, _contract, summary, _rank in reversed(archive_entries):
            if not report_paths.issubset(changed_file_paths(summary)):
                continue
            outcome_path = (
                contract_path.with_name(
                    contract_path.name.replace(".contract.json", ".outcome.json")
                )
                .relative_to(PROJECT_ROOT)
                .as_posix()
            )
            if outcome_path not in all_paths:
                continue
            if not human_benefit_report_issues(contract_path):
                return True
        return False

    def is_archived_generated_evidence(path: str) -> bool:
        """Accept generated archive metadata only when archived evidence names it."""
        if path in report_paths:
            return current_archive_owns_report_pair()
        if path in current_archive_generated_paths():
            return True
        return path == generated_archive_index and any(
            path in changed_file_paths(summary)
            for _contract_path, _contract, summary, _rank in archive_entries
        )

    def is_archive_bound_release_metadata(path: str) -> bool:
        """Accept post-archive release metadata only for an explicit pre-merge freeze."""
        if path not in ARCHIVE_BOUND_RELEASE_METADATA:
            return False
        freeze_path = PROJECT_ROOT / ".ai" / "cockpit" / "release-freeze.json"
        if not freeze_path.is_file():
            return False
        freeze = load_json(freeze_path)
        if (
            not isinstance(freeze, dict)
            or freeze.get("lifecycle", {}).get("state") != "premerge_finalized"
        ):
            return False
        return any(
            included(
                path, [pattern for pattern in contract.get("scope", []) if isinstance(pattern, str)]
            )
            and not included(
                path,
                [pattern for pattern in contract.get("outOfScope", []) if isinstance(pattern, str)],
            )
            for _contract_path, contract, _summary, _rank in archive_entries
        )

    for path in all_paths:
        if path in audit_paths or included(path, exempt) or path in no_op_restore_paths:
            continue
        owners = [
            entry
            for entry in archive_entries
            if included(
                path, [pattern for pattern in entry[1].get("scope", []) if isinstance(pattern, str)]
            )
            and not included(
                path,
                [pattern for pattern in entry[1].get("outOfScope", []) if isinstance(pattern, str)],
            )
            and path in changed_file_paths(entry[2])
        ]
        same_item_recovery_owner = False
        if not owners:
            owners = [
                entry
                for entry in archive_entries
                if path in same_item_recovery_paths.get(str(entry[1].get("workItemId", "")), set())
            ]
            same_item_recovery_owner = bool(owners)
        if not owners:
            if is_archived_generated_evidence(path) or is_archive_bound_release_metadata(path):
                continue
            issues.append(
                f"complete PR diff path lacks paired ownership (same Contract scope and Summary changedFiles): {path}"
            )
            continue
        # The PR audit resolves overlapping archive claims by the stable archive rank.
        _, effective_contract, _, _ = owners[-1]
        owner_match = first_match(path, ownership)
        if owner_match:
            _, owner = owner_match
            if owner.get("aiWrite") == "forbidden":
                issues.append(f"complete PR diff contains forbidden write: {path}")
            has_restricted_approval = (
                isinstance(effective_contract.get("restrictedWriteApproval"), dict)
                and effective_contract["restrictedWriteApproval"].get("approved") is True
            )
            if owner.get("aiWrite") == "restricted" and not (
                has_restricted_approval or same_item_recovery_owner
            ):
                issues.append(
                    f"complete PR diff restricted path lacks approval in a covering Contract: {path}"
                )
    return issues


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=os.environ.get("AI_BASE_COMMIT", ""))
    parser.add_argument("contracts", nargs="*")
    return parser.parse_args()


def validate_pr_boundary() -> list[str]:
    """Require the PR candidate to be fully committed before aggregate validation."""
    status = run_git(["status", "--porcelain", "--untracked-files=all"])
    if status.returncode != 0:
        return ["PR boundary cannot determine worktree cleanliness"]
    if status.stdout.strip():
        return [
            "PR boundary requires a clean committed worktree; commit generated release evidence before creating the PR"
        ]
    return []


def main() -> int:
    args = parse_args()
    if not args.base:
        print("ERROR: --base or AI_BASE_COMMIT is required", file=sys.stderr)
        return 2
    contract_paths = [Path(path).resolve() for path in args.contracts] or archived_contract_paths(
        args.base
    )
    issues = validate_pr_boundary() + validate_pr_bundle(args.base, contract_paths)
    if issues:
        for issue in issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        return 1
    print(f"aggregate PR check passed: {len(contract_paths)} Work Item(s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
