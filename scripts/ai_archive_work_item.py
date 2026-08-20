#!/usr/bin/env python3
"""Archive a Work Item from active/ to archive/YYYY/."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import shutil
import subprocess
import sys
import time
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from ai_check_summary import validate_summary
from ai_check_work_item import validate_contract
from ai_common import (
    PROJECT_ROOT,
    InvalidDataShapeError,
    changed_paths,
    clean_git_environment,
    discover_remote_default_candidates,
    load_json,
    non_empty_string,
    numeric_value,
    parse_yaml,
    path_fingerprint,
    redact_machine_paths_in_data,
    save_json,
    verification_key,
)
from ai_lifecycle_truth import (
    archived_outcome_projection,
    is_valid_superseded_transition,
    rewrite_exact_path_references,
    superseded_summary_validation_exception,
)
from ai_observability import AiEvent, AiEventLevel, AiEventType, create_observability
from ai_outcome_gate import validate_terminal_outcome
from ai_projection_lease import ProjectionLeaseError, requires_lease
from ai_projection_lease import acquire as acquire_projection_lease
from ai_work_item_intelligence import record_fact_once

ACTIVE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "active"
ARCHIVE_BASE_DIR = PROJECT_ROOT / ".ai" / "work-items" / "archive"
TRACEABILITY_MANIFEST = Path("docs/reference/remediation-instruction-traceability.json")


def _generate_knowledge_projection(contract_path: Path) -> Path | None:
    """Generate the projection only after archive paths are final."""
    work_item_id = contract_path.name.removesuffix(".contract.json")
    summary_path = contract_path.with_name(f"{work_item_id}.summary.json")
    outcome_path = contract_path.with_name(f"{work_item_id}.outcome.json")
    if not outcome_path.is_file():
        # Historic/superseded records without a terminal Outcome remain
        # discoverable through the authoritative archive index; no projection
        # is fabricated for them.
        return None
    from ai_check_knowledge_index import check_index, check_record
    from ai_generate_knowledge_record import (
        _write_if_changed,
        build_record,
        rebuild_existing_projections,
        rebuild_index,
    )

    record = build_record(
        contract_path,
        summary_path,
        outcome_path,
        repo_root=PROJECT_ROOT,
    )
    record_path = PROJECT_ROOT / ".ai" / "knowledge" / "work-items" / f"{work_item_id}.json"
    index_path = PROJECT_ROOT / ".ai" / "knowledge" / "index.json"
    _write_if_changed(record_path, record)
    rebuild_index(
        record_path.parent,
        index_path,
        record_updates={work_item_id: record},
    )
    rebuild_existing_projections(
        repo_root=PROJECT_ROOT,
        include_work_item_ids=[work_item_id],
    )
    issues = check_record(record_path, repo_root=PROJECT_ROOT)
    issues.extend(check_index(index_path, records_dir=record_path.parent, repo_root=PROJECT_ROOT))
    if issues:
        raise ValueError(
            "generated Implementation Knowledge projection is stale or invalid: "
            + "; ".join(issues)
        )
    return record_path


def owned_success_criteria_path(contract_path: Path) -> Path:
    """Return the task-owned Success Criteria sibling for an active Contract."""
    return contract_path.with_name(contract_path.name.replace(".contract.json", ".success.json"))


def outcome_artifact_paths(contract_path: Path) -> list[Path]:
    """Return optional active Outcome and event siblings for one Work Item."""
    stem = contract_path.name.replace(".contract.json", "")
    return [
        contract_path.with_name(f"{stem}.outcome.json"),
        contract_path.with_name(f"{stem}.outcome.md"),
        contract_path.with_name(f"{stem}.events.jsonl"),
        contract_path.with_name(f"{stem}.successor-receipt.json"),
    ]


def superseded_archive_validation_exception(
    *, contract_path: Path, work_item_id: str, summary_issues: list[str]
) -> bool:
    """Allow only a bound superseded blocked predecessor to archive its red evidence."""
    return superseded_summary_validation_exception(
        contract_path=contract_path,
        work_item_id=work_item_id,
        summary_issues=summary_issues,
    )


def _archive_index_path() -> Path:
    return ARCHIVE_BASE_DIR / "index.json"


def _is_ignored(path: Path) -> bool:
    """Identify local-only archive evidence excluded from repository checkouts."""
    try:
        relative_path = path.relative_to(PROJECT_ROOT)
    except ValueError:
        return False
    gitignore = PROJECT_ROOT / ".gitignore"
    if not gitignore.is_file():
        return False
    relative_text = relative_path.as_posix()
    ignored = False
    for line in gitignore.read_text(encoding="utf-8").splitlines():
        pattern = line.strip()
        if not pattern or pattern.startswith("#"):
            continue
        negated = pattern.startswith("!")
        if negated:
            pattern = pattern[1:]
        pattern = pattern.rstrip("/").lstrip("/")
        if fnmatch.fnmatch(relative_text, pattern) or fnmatch.fnmatch(path.name, pattern):
            ignored = not negated
    return ignored


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Archive a Work Item.")
    parser.add_argument("contract", nargs="?", help="Path to the active contract JSON.")
    parser.add_argument(
        "--rebuild-index",
        action="store_true",
        help="Rebuild the archive discovery index from authoritative Contract/Summary pairs.",
    )
    parser.add_argument(
        "--dry-run", action="store_true", help="Print actions without modifying files."
    )
    return parser.parse_args()


def _restore_files(files_to_move: list[tuple[Path, Path]]) -> None:
    for src, target in reversed(files_to_move):
        if target.exists():
            shutil.move(str(target), str(src))


def _rewrite_archived_path_references(value: Any, replacements: dict[str, str]) -> Any:
    """Rewrite exact active-artifact paths anywhere in archived evidence."""
    return rewrite_exact_path_references(value, replacements)


_SUMMARY_PATH_VALUE_FIELDS = frozenset({"path", "contractPath", "summaryPath", "location"})
_SUMMARY_PATH_COLLECTION_FIELDS = frozenset(
    {"evidence", "generatedFiles", "solidifiedIn", "sourcesUsed"}
)
_IMMUTABLE_SUMMARY_SUBTREES = frozenset({"verification"})


def _rewrite_archived_summary_paths(
    value: Any,
    replacements: dict[str, str],
    *,
    collection_values_are_paths: bool = False,
) -> Any:
    """Migrate schema-defined Summary paths without altering prose or provenance."""
    if isinstance(value, dict):
        rewritten: dict[Any, Any] = {}
        for key, item in value.items():
            if key in _IMMUTABLE_SUMMARY_SUBTREES:
                rewritten[key] = item
            elif key in _SUMMARY_PATH_VALUE_FIELDS and isinstance(item, str):
                rewritten[key] = replacements.get(item, item)
            else:
                rewritten[key] = _rewrite_archived_summary_paths(
                    item,
                    replacements,
                    collection_values_are_paths=(
                        collection_values_are_paths or key in _SUMMARY_PATH_COLLECTION_FIELDS
                    ),
                )
        return rewritten
    if isinstance(value, list):
        return [
            _rewrite_archived_summary_paths(
                item,
                replacements,
                collection_values_are_paths=collection_values_are_paths,
            )
            for item in value
        ]
    if collection_values_are_paths and isinstance(value, str):
        return replacements.get(value, value)
    return value


def _rewrite_exact_string(value: Any, source: str, target: str) -> tuple[Any, int]:
    """Recursively replace one exact string and return the replacement count."""
    if isinstance(value, dict):
        rewritten: dict[Any, Any] = {}
        replacements = 0
        for key, item in value.items():
            rewritten_item, count = _rewrite_exact_string(item, source, target)
            rewritten[key] = rewritten_item
            replacements += count
        return rewritten, replacements
    if isinstance(value, list):
        rewritten_list: list[Any] = []
        replacements = 0
        for item in value:
            rewritten_item, count = _rewrite_exact_string(item, source, target)
            rewritten_list.append(rewritten_item)
            replacements += count
        return rewritten_list, replacements
    if isinstance(value, str) and value == source:
        return target, 1
    return value, 0


def _rewrite_traceability_paths(
    payload: dict[str, Any], replacements: dict[str, str]
) -> tuple[dict[str, Any], int]:
    """Rewrite every exact active evidence path to its archive destination."""
    rewritten: Any = payload
    replacement_count = 0
    for source, target in replacements.items():
        rewritten, count = _rewrite_exact_string(rewritten, source, target)
        replacement_count += count
    if not isinstance(rewritten, dict):
        raise InvalidDataShapeError("rewritten traceability manifest must remain an object")
    return rewritten, replacement_count


def _load_registered_traceability() -> tuple[Path, bytes | None, dict[str, Any] | None]:
    """Read the mutable traceability manifest before any archive mutation."""
    path = PROJECT_ROOT / TRACEABILITY_MANIFEST
    if not path.is_file():
        return path, None, None
    original = path.read_bytes()
    try:
        payload = json.loads(original.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise ValueError(f"registered traceability manifest cannot be read: {exc}") from exc
    if not isinstance(payload, dict):
        raise InvalidDataShapeError("registered traceability manifest must contain a JSON object")
    return path, original, payload


def _atomic_save_json(path: Path, payload: dict[str, Any]) -> None:
    temporary = path.with_suffix(f"{path.suffix}.tmp")
    try:
        save_json(temporary, payload)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def _restore_original_bytes(path: Path, content: bytes) -> None:
    temporary = path.with_suffix(f"{path.suffix}.rollback.tmp")
    try:
        temporary.parent.mkdir(parents=True, exist_ok=True)
        temporary.write_bytes(content)
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def _generate_status(command: list[str]) -> None:
    subprocess.run(command, cwd=PROJECT_ROOT, env=clean_git_environment(), check=True)


def _worktree_digest(paths: list[str]) -> str:
    digest = hashlib.sha256()
    for path in sorted(set(paths)):
        digest.update(path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(path_fingerprint(path).encode("utf-8"))
        digest.update(b"\0")
    return digest.hexdigest()


def _summary_worktree_digest(summary: dict[str, object]) -> str:
    verification = summary.get("verification", [])
    if not isinstance(verification, list):
        return ""
    for item in reversed(verification):
        if not isinstance(item, dict):
            continue
        if verification_key(item) != "aiSummary" or item.get("result") != "passed":
            continue
        digest = item.get("worktreeDigest")
        if non_empty_string(digest):
            return str(digest)
    return ""


def _current_worktree_digest(contract: dict[str, object]) -> str:
    """Anchor owned source bytes while manifest roots derived lifecycle projections."""
    summary_path = str(contract.get("summaryPath", ""))
    work_item = contract.get("workItemId")
    derived_paths = (
        {
            f".ai/work-items/active/{work_item}.outcome.json",
            f".ai/work-items/active/{work_item}.outcome.md",
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
        }
        if isinstance(work_item, str) and work_item
        else {
            ".ai/cockpit/current_status.md",
            ".ai/cockpit/task_report.json",
            ".ai/cockpit/task_report.md",
        }
    )
    paths = [
        path
        for path in changed_paths(contract)
        if path != summary_path and path not in derived_paths
    ]
    return _worktree_digest(paths)


def _next_archive_sequence() -> int:
    """Return the next monotonic sequence, preferring the archive index."""
    highest = 0
    try:
        index = load_json(_archive_index_path())
    except (OSError, ValueError):
        index = None
    if isinstance(index, dict) and isinstance(index.get("entries"), list):
        for entry in index["entries"]:
            if isinstance(entry, dict) and isinstance(entry.get("archiveSequence"), int):
                highest = max(highest, int(entry["archiveSequence"]))
        if highest:
            return highest + 1
    for summary_path in ARCHIVE_BASE_DIR.rglob("*.summary.json"):
        try:
            summary = load_json(summary_path)
        except (OSError, ValueError):
            continue
        value = summary.get("archiveSequence")
        if isinstance(value, int) and not isinstance(value, bool):
            highest = max(highest, value)
    return highest + 1


def validate_archive_growth_reservation(
    contract: dict[str, Any], current_count: int, policy: dict[str, Any]
) -> list[str]:
    """Fail closed before archive mutation unless projected growth is reserved."""
    projected = current_count + 1
    limits = policy.get("max", {}) if isinstance(policy, dict) else {}
    limit = numeric_value(limits.get("archiveGrowth"))
    impact = contract.get("budgetImpact")
    expected_metrics = impact.get("expectedMetrics") if isinstance(impact, dict) else None
    expected = expected_metrics.get("archiveGrowth") if isinstance(expected_metrics, dict) else None
    future_metrics = impact.get("reservedFutureMetrics") if isinstance(impact, dict) else None
    future = future_metrics.get("archiveGrowth") if isinstance(future_metrics, dict) else None
    issues: list[str] = []
    warning_mode = archive_growth_enforcement(policy) == "warning"
    if not isinstance(expected, int) or isinstance(expected, bool):
        if not warning_mode:
            issues.append(
                "archiveGrowth reservation is required: budgetImpact.expectedMetrics.archiveGrowth "
                f"must equal projected archive count {projected}"
            )
    elif expected != projected and not warning_mode:
        issues.append(
            "archiveGrowth reservation is stale: "
            f"expected {expected}, projected archive count is {projected}"
        )
    if (
        future is not None
        and (not isinstance(future, int) or isinstance(future, bool) or future < projected)
        and not warning_mode
    ):
        issues.append(
            "reservedFutureMetrics.archiveGrowth must be an integer at least the current projected archive count"
        )
    if isinstance(limit, int) and projected > limit and not warning_mode:
        issues.append(f"projected archiveGrowth={projected} exceeds configured maximum {limit}")
    if (
        isinstance(expected, int)
        and isinstance(limit, int)
        and expected > limit
        and not warning_mode
    ):
        if not isinstance(impact, dict) or impact.get("approved") is not True:
            issues.append("archiveGrowth reservation requires budgetImpact.approved=true")
        if not isinstance(impact, dict) or not impact.get("repaymentWorkItem"):
            issues.append("archiveGrowth reservation requires repaymentWorkItem")
        if not isinstance(impact, dict) or not impact.get("repaymentRecords"):
            issues.append("archiveGrowth reservation requires repaymentRecords")
    return issues


def archive_growth_enforcement(policy: dict[str, Any]) -> str:
    enforcement = policy.get("enforcement", {}) if isinstance(policy, dict) else {}
    mode = enforcement.get("archiveGrowth") if isinstance(enforcement, dict) else None
    return mode if mode in {"error", "warning"} else "error"


def archive_growth_warnings(
    contract: dict[str, Any], current_count: int, policy: dict[str, Any]
) -> list[str]:
    if archive_growth_enforcement(policy) != "warning":
        return []
    limits = policy.get("max", {}) if isinstance(policy, dict) else {}
    limit = numeric_value(limits.get("archiveGrowth"))
    projected = current_count + 1
    if isinstance(limit, int) and projected > limit:
        return [f"projected archiveGrowth={projected} exceeds configured maximum {limit} (warning)"]
    return []


def _archive_growth_issues(contract: dict[str, Any]) -> list[str]:
    policy_path = PROJECT_ROOT / ".ai" / "guards" / "governance_complexity_policy.yaml"
    try:
        policy = parse_yaml(policy_path)
    except (OSError, ValueError) as exc:
        return [f"cannot read archiveGrowth policy before mutation: {exc}"]
    current_count = len(list(ARCHIVE_BASE_DIR.rglob("*.contract.json")))
    return validate_archive_growth_reservation(contract, current_count, policy)


def _archive_growth_warnings(contract: dict[str, Any]) -> list[str]:
    policy_path = PROJECT_ROOT / ".ai" / "guards" / "governance_complexity_policy.yaml"
    try:
        policy = parse_yaml(policy_path)
    except (OSError, ValueError):
        return []
    current_count = len(list(ARCHIVE_BASE_DIR.rglob("*.contract.json")))
    return archive_growth_warnings(contract, current_count, policy)


def _archive_entry(
    *,
    contract_path: Path,
    summary_path: Path | None,
    target_dir: Path,
    archive_sequence: int,
) -> dict[str, object]:
    """Build a portable discovery record for one archived Work Item."""
    contract_target = target_dir / contract_path.name
    contract = load_json(contract_target)
    entry: dict[str, object] = {
        "workItemId": contract.get("workItemId", contract_path.stem.replace(".contract", "")),
        "archiveSequence": archive_sequence,
        "archiveYear": target_dir.name,
        "contractPath": contract_target.relative_to(PROJECT_ROOT).as_posix(),
        "contractSha256": hashlib.sha256(contract_target.read_bytes()).hexdigest(),
        "archivedAt": datetime.now().astimezone().isoformat(),
    }
    if summary_path is not None:
        summary_target = target_dir / summary_path.name
        entry["summaryPath"] = summary_target.relative_to(PROJECT_ROOT).as_posix()
        entry["summarySha256"] = hashlib.sha256(summary_target.read_bytes()).hexdigest()
    manifest_target = target_dir / contract_path.name.replace(
        ".contract.json", ".archive-manifest.json"
    )
    if manifest_target.is_file():
        entry["manifestPath"] = manifest_target.relative_to(PROJECT_ROOT).as_posix()
        entry["manifestSha256"] = hashlib.sha256(manifest_target.read_bytes()).hexdigest()
    return entry


def _archive_manifest(
    *,
    contract_target: Path,
    summary_target: Path,
    archive_sequence: int,
    outcome_targets: list[Path] | None = None,
    pre_archive_candidate_coverage: dict[str, object] | None = None,
) -> dict[str, object]:
    """Build the immutable root after Contract and Summary are frozen."""
    manifest = {
        "format": "ai-cockpit-archive-manifest",
        "manifestVersion": 1,
        "workItemId": load_json(contract_target).get("workItemId"),
        "archiveSequence": archive_sequence,
        "contractPath": contract_target.relative_to(PROJECT_ROOT).as_posix(),
        "summaryPath": summary_target.relative_to(PROJECT_ROOT).as_posix(),
        "contractSha256": hashlib.sha256(contract_target.read_bytes()).hexdigest(),
        "summarySha256": hashlib.sha256(summary_target.read_bytes()).hexdigest(),
        "generatedStatusExcluded": True,
    }
    if outcome_targets:
        manifest["outcomeArtifacts"] = [
            {
                "path": target.relative_to(PROJECT_ROOT).as_posix(),
                "sha256": hashlib.sha256(target.read_bytes()).hexdigest(),
            }
            for target in outcome_targets
        ]
    if pre_archive_candidate_coverage is not None:
        manifest["preArchiveCandidateCoverage"] = pre_archive_candidate_coverage
    return manifest


def load_pre_archive_candidate_coverage(
    *, contract_path: Path, contract: dict[str, object]
) -> dict[str, object]:
    """Load and revalidate the exact candidate report that authorizes archive.

    The report remains local generated state, but its content-addressed binding
    must still describe the current candidate immediately before immutable
    archive mutation.  The returned report digest is the durable manifest root.
    """
    report_path = PROJECT_ROOT / "target" / "changed-critical-coverage.json"
    try:
        report_bytes = report_path.read_bytes()
        report = json.loads(report_bytes)
    except (OSError, json.JSONDecodeError) as exc:
        raise ValueError(f"missing readable pre-archive candidate coverage report: {exc}") from exc
    if not isinstance(report, dict) or not isinstance(report.get("binding"), dict):
        raise TypeError("pre-archive candidate coverage report is missing a binding")
    contract_base = contract.get("baseCommit")
    if not isinstance(contract_base, str) or not contract_base:
        raise ValueError("pre-archive candidate coverage requires Contract baseCommit")
    from check_changed_critical_coverage import candidate_snapshot

    binding = report["binding"]
    base = _pre_archive_candidate_base(
        contract_path=contract_path,
        contract=contract,
        binding=binding,
    )
    current = candidate_snapshot(base=base, project_root=PROJECT_ROOT, contract_path=contract_path)
    required = ("baseCommit", "candidateHead", "candidateTreeDigest", "candidateDiffDigest")
    if any(binding.get(key) != current.get(key) for key in required):
        raise ValueError(
            "pre-archive candidate coverage binding is stale or does not match the current candidate"
        )
    coverage: dict[str, object] = {
        "reportSha256": hashlib.sha256(report_bytes).hexdigest(),
        "binding": {key: binding[key] for key in required},
    }
    work_item = contract.get("workItemId")
    if not isinstance(work_item, str) or not work_item:
        raise ValueError("pre-archive candidate coverage requires Contract workItemId")
    outcome_path = ACTIVE_DIR / f"{work_item}.outcome.json"
    try:
        outcome = load_json(outcome_path)
    except (OSError, ValueError) as exc:
        raise ValueError(f"cannot read Outcome candidate coverage binding: {exc}") from exc
    bindings = outcome.get("bindings") if isinstance(outcome, dict) else None
    bound_coverage = (
        bindings.get("preArchiveCandidateCoverage") if isinstance(bindings, dict) else None
    )
    if bound_coverage == coverage:
        return coverage
    if bound_coverage is None and is_valid_superseded_transition(
        contract_path=contract_path,
        work_item_id=work_item,
    ):
        return coverage
    raise ValueError("Task Outcome does not bind the current pre-archive candidate coverage")


def _run_git_metadata(args: list[str]) -> subprocess.CompletedProcess[str]:
    """Run read-only Git metadata queries for archive evidence validation."""
    return subprocess.run(  # nosec B603 B607 - fixed Git metadata command
        ["git", *args],
        cwd=PROJECT_ROOT,
        env=clean_git_environment(),
        text=True,
        capture_output=True,
        check=False,
    )


def _unique_remote_default_tip() -> str:
    """Return the exact locally tracked tip of the unique remote default branch."""
    candidates = discover_remote_default_candidates(_run_git_metadata)
    if len(candidates) != 1:
        raise ValueError("superseded alternate coverage requires one unique remote default tip")
    remote, branch = candidates[0]
    result = _run_git_metadata(["rev-parse", "--verify", f"refs/remotes/{remote}/{branch}"])
    tip = result.stdout.strip()
    if result.returncode or len(tip) != 40:
        raise ValueError("superseded alternate coverage cannot resolve remote default tip")
    return tip


def _pre_archive_candidate_base(
    *, contract_path: Path, contract: dict[str, object], binding: dict[str, object]
) -> str:
    """Select the strict Contract base or the exact superseded transaction base."""
    contract_base = contract.get("baseCommit")
    report_base = binding.get("baseCommit")
    if not isinstance(contract_base, str) or not contract_base:
        raise ValueError("pre-archive candidate coverage requires Contract baseCommit")
    if report_base == contract_base:
        return contract_base
    work_item = contract.get("workItemId")
    if (
        not isinstance(report_base, str)
        or not isinstance(work_item, str)
        or not work_item
        or not is_valid_superseded_transition(
            contract_path=contract_path,
            work_item_id=work_item,
        )
    ):
        raise ValueError(
            "superseded alternate coverage base must match candidate HEAD and remote default tip"
        )
    candidate_head = binding.get("candidateHead")
    remote_tip = _unique_remote_default_tip()
    if report_base != candidate_head or report_base != remote_tip:
        raise ValueError(
            "superseded alternate coverage base must match candidate HEAD and remote default tip"
        )
    return report_base


def _archive_sequence_key(item: object) -> int:
    if isinstance(item, dict) and isinstance(item.get("archiveSequence"), int):
        return int(item["archiveSequence"])
    return 0


def _load_archive_index() -> dict[str, object]:
    """Load the index and add any authoritative archive pair it omits."""
    try:
        index = load_json(_archive_index_path())
    except (OSError, ValueError):
        index = None
    if isinstance(index, dict) and isinstance(index.get("entries"), list):
        entries = index["entries"]
    else:
        entries = []
        index = {
            "indexVersion": 1,
            "description": "Discovery index; archived Contract and Summary files remain authoritative.",
            "entries": entries,
        }

    deduplicated: list[dict[str, object]] = []
    positions: dict[tuple[object, object], int] = {}
    for existing_entry in entries:
        if not isinstance(existing_entry, dict):
            continue
        pair = (existing_entry.get("contractPath"), existing_entry.get("summaryPath"))
        position = positions.get(pair)
        if position is None:
            positions[pair] = len(deduplicated)
            deduplicated.append(existing_entry)
            continue
        current = deduplicated[position]
        current_is_strict = isinstance(current.get("contractSha256"), str) and isinstance(
            current.get("summarySha256"), str
        )
        candidate_is_strict = isinstance(existing_entry.get("contractSha256"), str) and isinstance(
            existing_entry.get("summarySha256"), str
        )
        if (not current_is_strict and candidate_is_strict) or (
            current.get("archivedAt") == "legacy" and existing_entry.get("archivedAt") != "legacy"
        ):
            deduplicated[position] = existing_entry
    entries = [
        entry
        for entry in deduplicated
        if isinstance(entry.get("contractPath"), str)
        and isinstance(entry.get("summaryPath"), str)
        and (PROJECT_ROOT / str(entry["contractPath"])).is_file()
        and (PROJECT_ROOT / str(entry["summaryPath"])).is_file()
        and not _is_ignored(PROJECT_ROOT / str(entry["contractPath"]))
        and not _is_ignored(PROJECT_ROOT / str(entry["summaryPath"]))
    ]
    index["entries"] = entries

    indexed_pairs = {
        (entry.get("contractPath"), entry.get("summaryPath"))
        for entry in entries
        if isinstance(entry, dict)
    }
    for summary_path in ARCHIVE_BASE_DIR.rglob("*.summary.json"):
        if _is_ignored(summary_path):
            continue
        try:
            summary = load_json(summary_path)
            contract_path = PROJECT_ROOT / str(summary["contractPath"])
            if not contract_path.exists():
                contract_path = summary_path.with_name(
                    summary_path.name.replace(".summary.json", ".contract.json")
                )
            sequence = summary.get("archiveSequence")
            if not contract_path.exists() or not isinstance(summary.get("workItemId"), str):
                continue
            if _is_ignored(contract_path):
                continue
            contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
            summary_rel = summary_path.relative_to(PROJECT_ROOT).as_posix()
            if (contract_rel, summary_rel) in indexed_pairs:
                continue
            entry: dict[str, object] = {
                "workItemId": summary["workItemId"],
                "archiveSequence": sequence if isinstance(sequence, int) else 0,
                "archiveYear": summary_path.parent.name,
                "contractPath": contract_rel,
                "summaryPath": summary_rel,
                "archivedAt": summary.get("archivedAt", "legacy"),
            }
            if isinstance(sequence, int) and sequence > 0:
                entry["contractSha256"] = hashlib.sha256(contract_path.read_bytes()).hexdigest()
                entry["summarySha256"] = hashlib.sha256(summary_path.read_bytes()).hexdigest()
            entries.append(entry)
            indexed_pairs.add((contract_rel, summary_rel))
        except (KeyError, OSError, ValueError):
            continue
    entries.sort(key=_archive_sequence_key)
    return index


def _write_archive_index(index: dict[str, object]) -> None:
    """Atomically persist the discovery index."""
    ARCHIVE_BASE_DIR.mkdir(parents=True, exist_ok=True)
    index_path = _archive_index_path()
    temporary = index_path.with_suffix(".json.tmp")
    save_json(temporary, index)
    temporary.replace(index_path)


def _validate_archive_inputs(
    contract_path: Path,
    contract: dict,
    summary_path: Path | None,
    summary: dict | None,
    *,
    require_outcome: bool = True,
) -> list[str]:
    issues = validate_contract(contract)
    if summary_path is None or summary is None:
        return issues
    contract_rel = contract_path.relative_to(PROJECT_ROOT).as_posix()
    summary_rel = summary_path.relative_to(PROJECT_ROOT).as_posix()
    contract_hash = hashlib.sha256(contract_path.read_bytes()).hexdigest()
    summary_issues = validate_summary(
        summary,
        contract,
        expected_contract_hash=contract_hash,
        contract_path=contract_rel,
        summary_path=summary_rel,
    )
    superseded = is_valid_superseded_transition(
        contract_path=contract_path,
        work_item_id=str(contract.get("workItemId", "")),
    )
    if not issues and superseded_archive_validation_exception(
        contract_path=contract_path,
        work_item_id=str(contract.get("workItemId", "")),
        summary_issues=summary_issues,
    ):
        return []
    issues.extend(summary_issues)
    if require_outcome and not superseded:
        outcome_path = contract_path.with_name(
            contract_path.name.replace(".contract.json", ".outcome.json")
        )
        markdown_path = outcome_path.with_suffix(".md")
        expected_head = None
        if (PROJECT_ROOT / ".git").exists():
            head_result = _run_git_metadata(["rev-parse", "HEAD"])
            if head_result.returncode == 0 and head_result.stdout.strip():
                expected_head = head_result.stdout.strip()
        gate = validate_terminal_outcome(
            outcome_path,
            markdown_path,
            expected_task_id=str(contract.get("workItemId", "")),
            contract_path=contract_path,
            summary_path=summary_path,
            expected_base_commit=contract.get("baseCommit"),
            expected_head_commit=expected_head,
        )
        issues.extend(f"Task Outcome gate: {issue}" for issue in gate.issues)
    return issues


def archive_text_whitespace_issues(paths: list[Path]) -> list[str]:
    """Reject whitespace that would make newly archived evidence uncommittable."""
    issues: list[str] = []
    for path in paths:
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except (OSError, UnicodeDecodeError) as exc:
            issues.append(f"{path.relative_to(PROJECT_ROOT)}: cannot read archive artifact: {exc}")
            continue
        for line_number, line in enumerate(lines, start=1):
            if line.endswith((" ", "\t")):
                issues.append(
                    f"{path.relative_to(PROJECT_ROOT)}:{line_number}: trailing whitespace"
                )
    return issues


def _execute_archive_transaction(
    *,
    contract_path: Path,
    summary_path: Path,
    review_path: Path,
    success_path: Path,
    outcome_paths: list[Path],
    files_to_move: list[tuple[Path, Path]],
    target_dir: Path,
    summary_tmp: Path | None,
    manifest_target: Path,
    has_summary: bool,
    has_review: bool,
    has_success: bool,
    archive_sequence: int,
    traceability_path: Path,
    traceability_backup: bytes | None,
    traceability_payload: dict[str, Any] | None,
    pre_archive_candidate_coverage: dict[str, object] | None = None,
    preserve_superseded_outcome: bool = False,
) -> None:
    """Execute and roll back one archive mutation as a single transaction."""
    work_item_id = contract_path.name.removesuffix(".contract.json")
    index_path = _archive_index_path()
    index_backup = index_path.read_bytes() if index_path.exists() else None
    status_path = PROJECT_ROOT / ".ai" / "cockpit" / "current_status.md"
    status_backup = status_path.read_bytes() if status_path.exists() else None
    report_paths = (
        PROJECT_ROOT / ".ai" / "cockpit" / "task_report.json",
        PROJECT_ROOT / ".ai" / "cockpit" / "task_report.md",
    )
    report_backups = {path: path.read_bytes() if path.exists() else None for path in report_paths}
    refreshed_report_paths = False
    active_file_backups = {source: source.read_bytes() for source, _ in files_to_move}
    traceability_changed = False
    try:
        for src, target in files_to_move:
            shutil.move(str(src), str(target))
            print(f"moved: {target.relative_to(PROJECT_ROOT)}")
        _generate_status([sys.executable, "scripts/ai_generate_status.py", "--no-active"])

        if has_summary:
            archived_contract = (
                (target_dir / contract_path.name).relative_to(PROJECT_ROOT).as_posix()
            )
            archived_summary = (target_dir / summary_path.name).relative_to(PROJECT_ROOT).as_posix()
            replacements = {
                contract_path.relative_to(PROJECT_ROOT).as_posix(): archived_contract,
                summary_path.relative_to(PROJECT_ROOT).as_posix(): archived_summary,
            }
            if has_review:
                replacements[review_path.relative_to(PROJECT_ROOT).as_posix()] = (
                    (target_dir / review_path.name).relative_to(PROJECT_ROOT).as_posix()
                )
            if has_success:
                replacements[success_path.relative_to(PROJECT_ROOT).as_posix()] = (
                    (target_dir / success_path.name).relative_to(PROJECT_ROOT).as_posix()
                )
            for path in outcome_paths:
                replacements[path.relative_to(PROJECT_ROOT).as_posix()] = (
                    (target_dir / path.name).relative_to(PROJECT_ROOT).as_posix()
                )
            for path in outcome_paths:
                if path.name.endswith(".outcome.json"):
                    outcome_target = target_dir / path.name
                    outcome = load_json(outcome_target)
                    rewritten_outcome = archived_outcome_projection(
                        outcome,
                        root=PROJECT_ROOT,
                        contract_path=target_dir / contract_path.name,
                        work_item_id=contract_path.name.removesuffix(".contract.json"),
                    )
                    if not preserve_superseded_outcome:
                        save_json(outcome_target, rewritten_outcome)
                    from ai_generate_human_report import (
                        generate_human_report,
                        render_human_report,
                    )

                    has_prior_report = any(
                        content is not None for content in report_backups.values()
                    )
                    if has_prior_report and not all(path.is_file() for path in report_paths):
                        raise ValueError(
                            "Human Benefit Report archive refresh requires both report files"
                        )
                    try:
                        report = generate_human_report(
                            rewritten_outcome,
                            phase="review",
                            contract=load_json(target_dir / contract_path.name),
                        )
                    except ValueError:
                        # A superseded predecessor may intentionally retain a
                        # historical or fixture-only Outcome whose bytes are
                        # bound by lifecycle evidence but cannot produce a new
                        # Human Benefit Report. If a report already exists,
                        # preserve the previous fail-closed behavior; when no
                        # report exists, leave it absent and keep the archive
                        # transaction focused on its canonical evidence.
                        if has_prior_report:
                            raise
                    else:
                        save_json(report_paths[0], report)
                        report_paths[1].write_text(render_human_report(report), encoding="utf-8")
                        refreshed_report_paths = True
            if traceability_payload is not None:
                rewritten_traceability, replacement_count = _rewrite_traceability_paths(
                    traceability_payload, replacements
                )
                if replacement_count:
                    _atomic_save_json(traceability_path, rewritten_traceability)
                    traceability_changed = True
            summary = redact_machine_paths_in_data(load_json(target_dir / summary_path.name))
            summary = _rewrite_archived_summary_paths(summary, replacements)
            summary["contractPath"] = archived_contract
            summary["archiveSequence"] = archive_sequence
            changed = summary.get("changedFiles", [])
            if isinstance(changed, list):
                if not any(
                    isinstance(item, dict) and item.get("path") == ".ai/cockpit/current_status.md"
                    for item in changed
                ):
                    changed.append(
                        {
                            "path": ".ai/cockpit/current_status.md",
                            "reason": "Generated no-active cockpit status after archival.",
                        }
                    )
                existing = {item.get("path") for item in changed if isinstance(item, dict)}
                if refreshed_report_paths:
                    for report_path in report_paths:
                        report_rel = report_path.relative_to(PROJECT_ROOT).as_posix()
                        if report_rel not in existing:
                            changed.append(
                                {
                                    "path": report_rel,
                                    "reason": (
                                        "Regenerated from the rewritten archived Task Outcome "
                                        "during this archive transaction."
                                    ),
                                }
                            )
                            existing.add(report_rel)
                    alignment = summary.get("documentationAlignment")
                    if isinstance(alignment, dict):
                        checks = alignment.get("checks")
                        if isinstance(checks, list):
                            for check in checks:
                                if (
                                    isinstance(check, dict)
                                    and check.get("area") == "documentationCommandsCapability"
                                ):
                                    evidence = check.get("evidence")
                                    report_markdown = (
                                        report_paths[1].relative_to(PROJECT_ROOT).as_posix()
                                    )
                                    if (
                                        isinstance(evidence, list)
                                        and report_markdown not in evidence
                                    ):
                                        evidence.append(report_markdown)
                                    if check.get("status") == "not_applicable":
                                        check["status"] = "aligned"
                                        check["reason"] = (
                                            "Archive transaction regenerated the Human Benefit "
                                            "Report from the rewritten archived Task Outcome."
                                        )
                                    break
                for archived_path in replacements.values():
                    if archived_path not in existing:
                        changed.append(
                            {"path": archived_path, "reason": "Archived Work Item audit evidence."}
                        )
                index_rel = _archive_index_path().relative_to(PROJECT_ROOT).as_posix()
                if index_rel not in existing:
                    changed.append(
                        {
                            "path": index_rel,
                            "reason": "Generated archive discovery index.",
                        }
                    )
                manifest_rel = manifest_target.relative_to(PROJECT_ROOT).as_posix()
                if manifest_rel not in existing:
                    changed.append(
                        {"path": manifest_rel, "reason": "Immutable archive evidence root."}
                    )
                traceability_rel = traceability_path.relative_to(PROJECT_ROOT).as_posix()
                if traceability_changed and traceability_rel not in existing:
                    changed.append(
                        {
                            "path": traceability_rel,
                            "reason": (
                                "Archive transaction migrated current Work Item evidence "
                                "references to durable traceability paths."
                            ),
                        }
                    )
                if (target_dir / f"{work_item_id}.outcome.json").is_file():
                    knowledge_paths = (
                        (
                            f".ai/knowledge/work-items/{work_item_id}.json",
                            "Generated evidence-bound Implementation Knowledge Record.",
                        ),
                        (
                            ".ai/knowledge/index.json",
                            "Rebuilt deterministic Implementation Knowledge index.",
                        ),
                    )
                    for knowledge_path, reason in knowledge_paths:
                        if knowledge_path not in existing:
                            changed.append({"path": knowledge_path, "reason": reason})
                            existing.add(knowledge_path)
            summary_target = target_dir / summary_path.name
            assert summary_tmp is not None
            save_json(summary_tmp, summary)
            summary_tmp.replace(summary_target)

            # Archive path projection changes the Summary bytes. Refresh the
            # current Outcome's content bindings only after the final archived
            # Summary exists, then regenerate its Markdown and Human Benefit
            # Report from that exact persisted state. Otherwise PR/close would
            # correctly discover a stale summaryDigest after archive.
            if not preserve_superseded_outcome:
                from ai_generate_human_report import generate_human_report, render_human_report
                from ai_render_task_outcome import render_task_outcome

                for path in outcome_paths:
                    if not path.name.endswith(".outcome.json"):
                        continue
                    outcome_target = target_dir / path.name
                    outcome = load_json(outcome_target)
                    bindings = outcome.get("bindings")
                    if not isinstance(bindings, dict):
                        raise TypeError("archived Outcome bindings are missing")
                    bindings["contractDigest"] = hashlib.sha256(
                        (target_dir / contract_path.name).read_bytes()
                    ).hexdigest()
                    bindings["summaryDigest"] = hashlib.sha256(
                        summary_target.read_bytes()
                    ).hexdigest()
                    verification = summary.get("verification", [])
                    bindings["verificationDigest"] = hashlib.sha256(
                        json.dumps(
                            verification,
                            ensure_ascii=False,
                            sort_keys=True,
                            separators=(",", ":"),
                        ).encode("utf-8")
                    ).hexdigest()
                    save_json(outcome_target, outcome)
                    outcome_markdown_target = outcome_target.with_suffix(".md")
                    outcome_markdown_target.write_text(
                        render_task_outcome(outcome), encoding="utf-8"
                    )
                    report = generate_human_report(
                        outcome,
                        phase="review",
                        contract=load_json(target_dir / contract_path.name),
                    )
                    save_json(report_paths[0], report)
                    report_paths[1].write_text(render_human_report(report), encoding="utf-8")
                    refreshed_report_paths = True

            save_json(
                manifest_target,
                _archive_manifest(
                    contract_target=target_dir / contract_path.name,
                    summary_target=summary_target,
                    archive_sequence=archive_sequence,
                    outcome_targets=[target_dir / path.name for path in outcome_paths],
                    pre_archive_candidate_coverage=pre_archive_candidate_coverage,
                ),
            )

        index = _load_archive_index()
        entries = index.get("entries")
        if not isinstance(entries, list):
            raise InvalidDataShapeError("archive index entries must be a list")
        new_entry = _archive_entry(
            contract_path=contract_path,
            summary_path=summary_path if has_summary else None,
            target_dir=target_dir,
            archive_sequence=archive_sequence,
        )
        new_pair = (new_entry.get("contractPath"), new_entry.get("summaryPath"))
        for entry_index, entry in enumerate(entries):
            if (
                isinstance(entry, dict)
                and (entry.get("contractPath"), entry.get("summaryPath")) == new_pair
            ):
                entries[entry_index] = new_entry
                break
        else:
            entries.append(new_entry)
        entries.sort(key=_archive_sequence_key)
        _write_archive_index(index)
    except Exception:
        if summary_tmp and summary_tmp.exists():
            summary_tmp.unlink()
        manifest_target.unlink(missing_ok=True)
        if index_backup is None:
            index_path.unlink(missing_ok=True)
        else:
            index_path.write_bytes(index_backup)
        try:
            _restore_files(files_to_move)
            for source, content in active_file_backups.items():
                _restore_original_bytes(source, content)
        except Exception as rollback_exc:  # noqa: BLE001 - rollback must retain every recovery failure
            print(f"ERROR: Failed to roll back archive files: {rollback_exc}", file=sys.stderr)
        if traceability_changed and traceability_backup is not None:
            try:
                _restore_original_bytes(traceability_path, traceability_backup)
            except Exception as rollback_exc:  # noqa: BLE001 - rollback must retain every recovery failure
                print(
                    f"ERROR: Failed to roll back traceability manifest: {rollback_exc}",
                    file=sys.stderr,
                )
        if status_backup is None:
            status_path.unlink(missing_ok=True)
        else:
            _restore_original_bytes(status_path, status_backup)
        for path, report_content in report_backups.items():
            if report_content is None:
                path.unlink(missing_ok=True)
            else:
                _restore_original_bytes(path, report_content)
        raise


def main() -> int:
    phase_start = time.time()
    args = parse_args()
    if getattr(args, "rebuild_index", False):
        try:
            index = _load_archive_index()
            _write_archive_index(index)
        except (OSError, ValueError, KeyError) as exc:
            print(f"ERROR: Failed to rebuild archive index: {exc}", file=sys.stderr)
            return 1
        entries = index.get("entries", [])
        count = len(entries) if isinstance(entries, list) else 0
        print(f"archive index rebuilt: {count} entries")
        return 0
    if not args.contract:
        print(
            "ERROR: an active Contract path is required unless --rebuild-index is used",
            file=sys.stderr,
        )
        return 1
    contract_path = Path(args.contract).resolve()

    try:
        contract_path.relative_to(ACTIVE_DIR)
    except ValueError:
        print(f"ERROR: Contract must be in {ACTIVE_DIR.relative_to(PROJECT_ROOT)}", file=sys.stderr)
        return 1

    if not contract_path.exists():
        print(
            f"ERROR: Contract not found: {contract_path.relative_to(PROJECT_ROOT)}", file=sys.stderr
        )
        return 1

    try:
        contract = load_json(contract_path)
    except Exception as exc:  # noqa: BLE001 - unreadable Contract must fail closed with its diagnostic
        print(f"ERROR: Failed to read contract: {exc}", file=sys.stderr)
        return 1

    work_item_id = contract.get("workItemId")
    if not work_item_id:
        print("ERROR: Contract missing workItemId", file=sys.stderr)
        return 1

    file_basename = contract_path.name.replace(".contract.json", "")
    mode = contract.get("mode")
    summary_path = ACTIVE_DIR / f"{file_basename}.summary.json"
    review_path = ACTIVE_DIR / f"{file_basename}.review.json"
    success_path = owned_success_criteria_path(contract_path)
    has_summary = summary_path.exists()
    has_review = review_path.exists()
    has_success = success_path.exists()
    outcome_paths = [path for path in outcome_artifact_paths(contract_path) if path.exists()]

    if mode == "code" and not has_summary:
        print(
            f"ERROR: mode code requires Summary: {summary_path.relative_to(PROJECT_ROOT)}",
            file=sys.stderr,
        )
        return 1

    summary = None
    if has_summary:
        try:
            summary = load_json(summary_path)
        except Exception as exc:  # noqa: BLE001 - unreadable Summary must fail closed with its diagnostic
            print(f"ERROR: Failed to read summary: {exc}", file=sys.stderr)
            return 1

    issues = _validate_archive_inputs(
        contract_path,
        contract,
        summary_path if has_summary else None,
        summary,
        require_outcome=not args.dry_run,
    )
    if issues:
        for issue in issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        return 1

    if has_summary:
        recorded_digest = _summary_worktree_digest(summary or {})
        digest_contract = dict(contract)
        digest_contract["summaryPath"] = summary_path.relative_to(PROJECT_ROOT).as_posix()
        current_digest = _current_worktree_digest(digest_contract)
        if recorded_digest and recorded_digest != current_digest:
            print(
                "ERROR: Summary worktreeDigest does not match current Work Item state; re-run ai-finish before archiving.",
                file=sys.stderr,
            )
            return 1

    archive_growth_issues = _archive_growth_issues(contract)
    for warning in _archive_growth_warnings(contract):
        print(f"[WARNING] {warning}", file=sys.stderr)
    if archive_growth_issues:
        for issue in archive_growth_issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        print(
            "ERROR: archive mutation blocked before budget reservation is valid.", file=sys.stderr
        )
        return 1

    target_dir = ARCHIVE_BASE_DIR / str(datetime.now(UTC).astimezone().year)
    files_to_move: list[tuple[Path, Path]] = [(contract_path, target_dir / contract_path.name)]
    if has_summary:
        files_to_move.append((summary_path, target_dir / summary_path.name))
    if has_review:
        files_to_move.append((review_path, target_dir / review_path.name))
    if has_success:
        files_to_move.append((success_path, target_dir / success_path.name))
    files_to_move.extend((path, target_dir / path.name) for path in outcome_paths)
    summary_tmp = target_dir / f"{summary_path.name}.tmp" if has_summary else None
    manifest_target = target_dir / contract_path.name.replace(
        ".contract.json", ".archive-manifest.json"
    )

    whitespace_issues = archive_text_whitespace_issues([source for source, _ in files_to_move])
    if whitespace_issues:
        for issue in whitespace_issues:
            print(f"[ERROR] {issue}", file=sys.stderr)
        print(
            "ERROR: archive mutation blocked until active evidence is whitespace-clean.",
            file=sys.stderr,
        )
        return 1

    for _, target in files_to_move:
        if target.exists():
            print(
                f"ERROR: Target already exists: {target.relative_to(PROJECT_ROOT)}", file=sys.stderr
            )
            return 1
    if manifest_target.exists():
        print(
            f"ERROR: Target already exists: {manifest_target.relative_to(PROJECT_ROOT)}",
            file=sys.stderr,
        )
        return 1

    try:
        traceability_path, traceability_backup, traceability_payload = (
            _load_registered_traceability()
        )
    except (OSError, ValueError) as exc:
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1

    preserve_superseded_outcome = has_summary and is_valid_superseded_transition(
        contract_path=contract_path,
        work_item_id=str(work_item_id),
    )

    if args.dry_run:
        print("Dry run: files that would be archived:")
        for src, target in files_to_move:
            print(f"  {src.relative_to(PROJECT_ROOT)} -> {target.relative_to(PROJECT_ROOT)}")
        return 0

    pre_archive_candidate_coverage: dict[str, object] | None = None
    # A non-Git fixture cannot model a post-archive commit or its coverage
    # candidate. Real repository archive mutation is always fail-closed.
    if (PROJECT_ROOT / ".git").exists():
        try:
            pre_archive_candidate_coverage = load_pre_archive_candidate_coverage(
                contract_path=contract_path, contract=contract
            )
        except (TypeError, ValueError) as exc:
            print(f"ERROR: archive mutation blocked: {exc}", file=sys.stderr)
            return 1

    # Unit fixtures without a Git checkout cannot create branch projections.
    # A real repository must hold the explicit lease before archive mutates the
    # shared archive index, status, or task-report projections.
    if (PROJECT_ROOT / ".git").exists() and requires_lease(contract):
        try:
            acquire_projection_lease(str(work_item_id), root=PROJECT_ROOT)
        except ProjectionLeaseError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 2

    archive_sequence = _next_archive_sequence()
    target_dir.mkdir(parents=True, exist_ok=True)
    try:
        _execute_archive_transaction(
            contract_path=contract_path,
            summary_path=summary_path,
            review_path=review_path,
            success_path=success_path,
            outcome_paths=outcome_paths,
            files_to_move=files_to_move,
            target_dir=target_dir,
            summary_tmp=summary_tmp,
            manifest_target=manifest_target,
            has_summary=has_summary,
            has_review=has_review,
            has_success=has_success,
            archive_sequence=archive_sequence,
            traceability_path=traceability_path,
            traceability_backup=traceability_backup,
            traceability_payload=traceability_payload,
            pre_archive_candidate_coverage=pre_archive_candidate_coverage,
            preserve_superseded_outcome=preserve_superseded_outcome,
        )
    except Exception as exc:  # noqa: BLE001 - archive mutation failures must fail closed with their diagnostic
        print(f"ERROR: Failed to archive Work Item: {exc}", file=sys.stderr)
        return 1

    try:
        knowledge_record_path = _generate_knowledge_projection(target_dir / contract_path.name)
    except (OSError, ValueError, TypeError, json.JSONDecodeError) as exc:
        print(f"ERROR: archived Work Item knowledge projection failed: {exc}", file=sys.stderr)
        return 1

    obs = create_observability(work_item_id=work_item_id)
    record_fact_once(
        str(work_item_id),
        "archived",
        {"archiveYear": target_dir.name, "archiveSequence": archive_sequence},
    )
    obs.record(
        AiEvent(
            AiEventType.CHECK_PASSED,
            AiEventLevel.INFO,
            f"Work Item archived to {target_dir.name}",
            check_id="aiArchive",
            fields={"year": target_dir.name, "files": len(files_to_move)},
        )
    )
    if knowledge_record_path is not None:
        print(f"Implementation Knowledge Record: {knowledge_record_path.relative_to(PROJECT_ROOT)}")
    getattr(obs, "lifecycle_phase_finished", lambda *_args, **_kwargs: None)(
        "archive", duration_ms=int((time.time() - phase_start) * 1000), cache_outcome="miss"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
