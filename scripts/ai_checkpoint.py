#!/usr/bin/env python3
"""Print a compact checkpoint for the active Work Item."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from ai_check_diff_ownership import format_preview, preview
from ai_common import (
    load_json,
    save_json,
    verification_key,
    verification_status_for_generation,
)
from ai_work_item_intelligence import record_fact_once


def required_verification(contract: dict[str, Any]) -> list[str]:
    return [
        verification_key(item)
        for item in contract.get("verification", [])
        if isinstance(item, dict) and item.get("required") is True and verification_key(item)
    ]


def contract_hash(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:16]


def verification_status(
    summary: dict[str, Any] | None, contract: dict[str, Any] | None = None
) -> dict[str, str]:
    return verification_status_for_generation(summary, contract or {})


def review_focus(summary: dict[str, Any] | None) -> list[str]:
    if not isinstance(summary, dict):
        return []
    readiness = summary.get("reviewReadiness")
    if not isinstance(readiness, dict):
        return []
    focus = readiness.get("expectedReviewFocus")
    if not isinstance(focus, list):
        return []
    return [item for item in focus if isinstance(item, str) and item.strip()]


def intent_context(contract: dict[str, Any]) -> list[str]:
    """intent セクションからチェックポイント表示用のコンテキスト行を抽出する。

    intent が欠落・null・空でもチェックポイントが止まらないよう、
    problem / constraint / rationale の3要素を常に表示する。
    """
    intent = contract.get("intent")
    lines: list[str] = []
    if not isinstance(intent, dict):
        return [
            "problem: not provided",
            "constraint: not provided",
            "rationale: not provided",
        ]
    problem = intent.get("problem")
    if isinstance(problem, str) and problem.strip():
        lines.append(f"problem: {problem.strip()}")
    else:
        lines.append("problem: not provided")
    constraints = intent.get("constraints")
    if isinstance(constraints, list) and constraints:
        appended = False
        for item in constraints:
            if isinstance(item, str) and item.strip():
                lines.append(f"constraint: {item.strip()}")
                appended = True
        if not appended:
            lines.append("constraint: not provided")
    else:
        lines.append("constraint: not provided")
    rationale = intent.get("rationale")
    if isinstance(rationale, str) and rationale.strip():
        lines.append(f"rationale: {rationale.strip()}")
    else:
        lines.append("rationale: not provided")
    return lines


def next_action(contract: dict[str, Any], summary: dict[str, Any] | None) -> str:
    if contract.get("notCodable") is True:
        return "Stop coding. Resolve notCodable or record blocker/unknowns."
    unknowns = contract.get("unknowns")
    if isinstance(unknowns, list) and unknowns:
        return "Stop coding. Resolve unknowns or switch executionDecision away from continue."
    missing = [
        command
        for command in required_verification(contract)
        if verification_status(summary, contract).get(command) != "passed"
    ]
    if missing:
        return f"Run or record required verification: {missing[0]}"
    return "Ready for final status generation and human review."


def print_list(title: str, values: list[Any]) -> None:
    print(f"\n## {title}")
    if not values:
        print("- none")
        return
    for value in values:
        print(f"- {value}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Print an AI Work Item checkpoint.")
    parser.add_argument("--contract", required=True)
    parser.add_argument("--summary")
    parser.add_argument(
        "--stage",
        default="manual",
        help="Checkpoint stage, for example before_edit or before_finish.",
    )
    parser.add_argument("--previous-contract-hash")
    parser.add_argument("--reason")
    return parser.parse_args()


def record_checkpoint(
    summary: dict[str, Any], contract: dict[str, Any], stage: str, path: Path, summary_path: Path
) -> None:
    evidence = summary.get("checkpointEvidence", [])
    if not isinstance(evidence, list):
        evidence = []
    if stage == "before_edit" and any(
        isinstance(item, dict)
        and item.get("stage") == "before_edit"
        and item.get("recorded") is True
        for item in evidence
    ):
        raise ValueError(
            "before_edit checkpoint already exists; use make ai-revalidate-contract-amendment"
        )
    statuses = verification_status(summary, contract)
    record = {
        "stage": stage,
        "recorded": True,
        "contractHash": contract_hash(path),
        "acceptanceCount": len(contract.get("acceptance", [])),
        "unknownCount": len(contract.get("unknowns", [])),
        "requiredChecks": len(required_verification(contract)),
        # A before_edit checkpoint is a phase boundary, not a progress
        # snapshot.  Scope corrections may require a fresh implementation
        # checkpoint after a failed attempt has already recorded verification;
        # carrying those results forward would falsely place this checkpoint
        # after verification and correctly trigger the agent-risk fail-closed
        # guard.  Later checkpoint stages retain their progress snapshot.
        "requiredChecksPassed": (
            0
            if stage == "before_edit"
            else len(
                [item for item in required_verification(contract) if statuses.get(item) == "passed"]
            )
        ),
    }
    # The first implementation boundary is immutable.  A Contract amendment
    # must append its own revalidation record; replacing before_edit erases the
    # exact state that makes that amendment auditable.
    if stage == "before_edit" and any(
        item.get("stage") == "before_edit" and item.get("recorded") is True
        for item in evidence
        if isinstance(item, dict)
    ):
        raise ValueError("duplicate before_edit prepare is refused; append contract revalidation")
    summary["checkpointEvidence"] = [
        item for item in evidence if isinstance(item, dict) and item.get("stage") != stage
    ]
    summary["checkpointEvidence"].append(record)
    save_json(summary_path, summary)


def record_contract_amendment_revalidation(
    summary: dict[str, Any],
    contract: dict[str, Any],
    path: Path,
    summary_path: Path,
    *,
    previous_contract_hash: str,
    reason: str,
) -> dict[str, Any]:
    """Append evidence that an amended Contract was revalidated, never rewriting before_edit."""
    evidence = summary.get("checkpointEvidence", [])
    if not isinstance(evidence, list):
        raise TypeError("checkpointEvidence must be a list before amendment revalidation")
    original = next(
        (
            item
            for item in evidence
            if isinstance(item, dict)
            and item.get("stage") == "before_edit"
            and item.get("recorded") is True
        ),
        None,
    )
    if not isinstance(original, dict):
        raise TypeError("before_edit checkpoint is required before contract amendment revalidation")
    original_hash = original.get("contractHash")
    if not isinstance(original_hash, str) or not original_hash:
        raise ValueError("before_edit checkpoint contractHash is required")
    if not isinstance(previous_contract_hash, str) or not previous_contract_hash:
        raise ValueError("previous Contract hash is required")
    preceding = [
        item
        for item in evidence
        if isinstance(item, dict)
        and item.get("stage") == "contract_amendment_revalidation"
        and item.get("recorded") is True
    ]
    preceding_hash = preceding[-1].get("contractHash") if preceding else original_hash
    if previous_contract_hash != preceding_hash:
        raise ValueError(
            "previous Contract hash must bind the immediately preceding checkpoint evidence"
        )
    if not isinstance(reason, str) or not reason.strip():
        raise ValueError("amendment reason is required")
    statuses = verification_status(summary, contract)
    required_checks = required_verification(contract)
    verification_started = any(
        statuses.get(check) in {"passed", "failed", "warning", "blocked"}
        for check in required_checks
    )
    record = {
        "stage": "contract_amendment_revalidation",
        "recorded": True,
        "originalBeforeEditContractHash": original_hash,
        "previousContractHash": previous_contract_hash,
        "contractHash": contract_hash(path),
        "acceptanceCount": len(contract.get("acceptance", [])),
        "unknownCount": len(contract.get("unknowns", [])),
        "requiredChecks": len(required_verification(contract)),
        "requiredChecksPassed": 0,
        "reason": reason.strip(),
        "verificationStarted": verification_started,
        # A post-verification amendment is stricter than an ordinary
        # pre-verification revalidation: every required result is explicitly
        # invalidated so Finish must establish the amended Contract's full
        # verification set again. The old records remain append-only evidence.
        "invalidatedRequiredChecks": required_checks if verification_started else [],
        "requiredChecksPassedAtAmendment": (
            len([check for check in required_checks if statuses.get(check) == "passed"])
            if verification_started
            else 0
        ),
        "recordedAt": datetime.now(UTC).isoformat(),
    }
    summary["checkpointEvidence"] = [*evidence, record]
    save_json(summary_path, summary)
    return record


def main() -> int:
    args = parse_args()
    try:
        contract = load_json(Path(args.contract))
        summary = (
            load_json(Path(args.summary)) if args.summary and Path(args.summary).exists() else None
        )
    except (OSError, json.JSONDecodeError, ValueError) as exc:
        print(f"Failed to load checkpoint inputs: {exc}", file=sys.stderr)
        return 1

    try:
        if args.summary and isinstance(summary, dict):
            if args.stage == "contract_amendment_revalidation":
                record_contract_amendment_revalidation(
                    summary,
                    contract,
                    Path(args.contract),
                    Path(args.summary),
                    previous_contract_hash=args.previous_contract_hash or "",
                    reason=args.reason or "",
                )
            else:
                record_checkpoint(
                    summary, contract, args.stage, Path(args.contract), Path(args.summary)
                )
    except ValueError as exc:
        print(f"Checkpoint blocked: {exc}", file=sys.stderr)
        return 2
    if args.stage == "before_edit":
        record_fact_once(
            str(contract.get("workItemId", "")),
            "implementation_started",
            {"checkpoint": "before_edit", "contractHash": contract_hash(Path(args.contract))},
        )

    print("# AI Work Item Checkpoint")
    print(f"- Stage: `{args.stage}`")
    print(f"- Work Item: `{contract.get('workItemId', '')}`")
    print(f"- Contract Hash: `{contract_hash(Path(args.contract))}`")
    print(f"- Mode: `{contract.get('mode', '')}`")
    print(f"- notCodable: `{contract.get('notCodable')}`")
    print(f"- Execution Decision: `{contract.get('executionDecision', {}).get('status', '')}`")
    acceptance = (
        contract.get("acceptance", []) if isinstance(contract.get("acceptance"), list) else []
    )
    unknowns = contract.get("unknowns", []) if isinstance(contract.get("unknowns"), list) else []
    required = required_verification(contract)
    status = verification_status(summary, contract)
    passed_required = [command for command in required if status.get(command) == "passed"]
    print(f"- Acceptance Count: `{len(acceptance)}`")
    print(f"- Unknown Count: `{len(unknowns)}`")
    print(f"- Required Checks: `{len(required)}`")
    print(f"- Required Checks Passed: `{len(passed_required)}`")

    print_list("Intent Context", intent_context(contract))
    print_list(
        "Scope", contract.get("scope", []) if isinstance(contract.get("scope"), list) else []
    )
    print_list(
        "Out Of Scope",
        contract.get("outOfScope", []) if isinstance(contract.get("outOfScope"), list) else [],
    )
    print_list("Unknowns", unknowns)
    print_list("Acceptance", acceptance)

    print("\n## Required Verification")
    if not required:
        print("- none")
    for command in required:
        print(f"- `{command}`: {status.get(command, 'not_recorded')}")

    print_list("Review Focus", review_focus(summary))
    print()
    print("\n".join(format_preview(preview(contract=contract))))
    print(f"\n## Next Action\n- {next_action(contract, summary)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
