"""Pure, deterministic verification selection, caching, and escalation policies."""

from __future__ import annotations

import hashlib
import json
from typing import Any

from ai_impact_classifier import classify_path

POLICY_LEVELS = ("light", "standard", "strict")
VERIFICATION_SCOPES = ("focused", "full")
ESCALATION_DOMAINS = frozenset(
    {"release", "workflow", "trust", "installer", "dependency", "unknown"}
)
DOMAIN_LEVELS = {
    "docs": "light",
    "project_code": "standard",
    "tests": "standard",
    "unknown": "standard",
    "dependency": "strict",
    "workflow": "strict",
    "trust": "strict",
    "installer": "strict",
    "lifecycle": "strict",
    "release": "strict",
}


def finish_quality_route(
    changed_paths: list[str], *, requested: str | None = None
) -> dict[str, Any]:
    """Return the auditable Finish route without lowering a Contract profile."""

    policy = select_policy("task", changed_paths, requested=requested)
    return {
        "policy": policy,
        "command": f"make ai-cockpit-quality GOVERNANCE_PROFILE={policy['level']}",
    }


def finish_quality_route_for_contract(
    changed_paths: list[str], governance_profile: dict[str, Any] | None
) -> dict[str, Any]:
    """Route Finish from final scope without treating automatic defaults as overrides.

    An automatic profile is a prior classification, not a human instruction.  Finish
    must therefore reclassify it against the final Contract scope, while preserving
    a recorded higher automatic level.  A human override remains an explicit request
    and is validated fail-closed by ``select_policy``.
    """
    profile = governance_profile if isinstance(governance_profile, dict) else {}
    selected = profile.get("selected")
    source = profile.get("source")
    automatic_route = finish_quality_route(changed_paths)

    if source != "automatic":
        return finish_quality_route(changed_paths, requested=selected)
    if selected not in POLICY_LEVELS:
        return automatic_route

    automatic_level = str(automatic_route["policy"]["level"])
    if POLICY_LEVELS.index(str(selected)) > POLICY_LEVELS.index(automatic_level):
        return finish_quality_route(changed_paths, requested=str(selected))
    return automatic_route


def select_policy(
    stage: str, changed_paths: list[str], *, requested: str | None = None
) -> dict[str, Any]:
    """Select a policy without permitting a caller to downgrade risk."""
    if requested is not None and requested not in POLICY_LEVELS:
        raise ValueError(f"unsupported policy level: {requested}")
    domains = {classify_path(path) for path in changed_paths}
    levels = [DOMAIN_LEVELS.get(domain, "standard") for domain in domains]
    level = max(levels, key=POLICY_LEVELS.index) if levels else "standard"
    stage_floor = "strict" if stage == "release" else "standard" if stage == "pr" else "light"
    if POLICY_LEVELS.index(stage_floor) > POLICY_LEVELS.index(level):
        level = stage_floor
    if requested is not None:
        if POLICY_LEVELS.index(requested) < POLICY_LEVELS.index(level):
            raise ValueError(f"requested policy {requested} cannot lower selected policy {level}")
        level = requested
    scope = "focused" if level == "light" else "full"
    return {"level": level, "scope": scope, "stage": stage, "domains": sorted(domains)}


def verification_cache_key(inputs: dict[str, Any]) -> str:
    """Return a content address over every input that can affect verification."""
    required = ("base", "diff", "command", "tool", "dependency", "environment", "config")
    missing = [name for name in required if name not in inputs]
    if missing:
        raise ValueError(f"cache key inputs missing: {', '.join(missing)}")
    canonical = json.dumps(inputs, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()


def order_checks(graph: dict[str, list[str]]) -> list[str]:
    """Topologically order a check DAG and reject unknown/cyclic dependencies."""
    nodes = set(graph)
    unknown = sorted({dependency for deps in graph.values() for dependency in deps} - nodes)
    if unknown:
        raise ValueError(f"unknown check dependencies: {', '.join(unknown)}")
    ordered: list[str] = []
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(node: str) -> None:
        if node in visiting:
            raise ValueError("verification check DAG contains a cycle")
        if node in visited:
            return
        visiting.add(node)
        for dependency in sorted(graph[node]):
            visit(dependency)
        visiting.remove(node)
        visited.add(node)
        ordered.append(node)

    for node in sorted(nodes):
        visit(node)
    return ordered


RECEIPT_BINDINGS = (
    "baseCommit",
    "headCommit",
    "changedPaths",
    "command",
    "environment",
    "toolchain",
    "policy",
)


def evaluate_impact_graph(
    graph: dict[str, Any], *, profile: str, receipt_bindings: dict[str, str]
) -> dict[str, Any]:
    """Describe a verification DAG without executing checks or scheduling work."""
    if profile not in (*POLICY_LEVELS, "release"):
        raise ValueError(f"unsupported graph profile: {profile}")
    raw_nodes = graph.get("nodes", {})
    nodes = raw_nodes if isinstance(raw_nodes, dict) else {}
    errors: list[str] = []
    required = sorted(name for name, node in nodes.items() if node.get("required") is True)
    final_proofs = [
        name
        for name, node in nodes.items()
        if node.get("required") is True and node.get("finalProof") is True
    ]
    if not final_proofs:
        errors.append("required final proof node is missing")
    dependencies = {name: list(node.get("dependsOn", [])) for name, node in nodes.items()}
    try:
        ordered = order_checks(dependencies)
    except ValueError as error:
        errors.append(str(error))
        ordered = []
    cached: list[str] = []
    invalidated: list[str] = []
    for name in sorted(nodes):
        expected = nodes[name].get("receiptBindings")
        if not isinstance(expected, dict):
            continue
        matches = all(expected.get(key) == receipt_bindings.get(key) for key in RECEIPT_BINDINGS)
        (cached if matches else invalidated).append(name)
    layers = {
        layer: sorted(name for name, node in nodes.items() if node.get("layer") == layer)
        for layer in ("Fast", "Finish", "Hosted")
    }
    parallelizable = sorted(name for name, dependencies in dependencies.items() if not dependencies)
    return {
        "valid": not errors,
        "errors": errors,
        "profile": profile,
        "requiredNodes": required,
        "orderedNodes": ordered,
        "dependencies": dependencies,
        "parallelizableGroups": [parallelizable] if parallelizable else [],
        "cachedNodes": cached,
        "invalidatedNodes": invalidated,
        "proofLayers": layers,
    }


def evaluate_current_impact_graph(
    *, profile: str, receipt_bindings: dict[str, str]
) -> dict[str, Any]:
    """Evaluate the repository's declared proof layers without running them."""
    return evaluate_impact_graph(
        {
            "nodes": {
                "fast": {"layer": "Fast", "required": True, "dependsOn": []},
                "finish": {
                    "layer": "Finish",
                    "required": True,
                    "dependsOn": ["fast"],
                },
                "hosted": {
                    "layer": "Hosted",
                    "required": True,
                    "finalProof": True,
                    "dependsOn": ["finish"],
                },
            }
        },
        profile=profile,
        receipt_bindings=receipt_bindings,
    )


def escalation_reasons(
    changed_paths: list[str],
    *,
    unknown: bool = False,
    injection: bool = False,
    prior_failure: bool = False,
) -> list[str]:
    """Return stable reasons; an empty result never lowers an already strict policy."""
    reasons = sorted({classify_path(path) for path in changed_paths} & ESCALATION_DOMAINS)
    if unknown:
        reasons.append("unknown_input")
    if injection:
        reasons.append("injection_signal")
    if prior_failure:
        reasons.append("test_changed_after_failure")
    return sorted(set(reasons))


def verification_signal(required: list[str], index: dict[str, str]) -> dict[str, Any]:
    missing = [x for x in required if x not in index]
    failed = [x for x in required if index.get(x) == "failed"]
    not_run = [x for x in required if index.get(x) == "not_run"]
    passed = [x for x in required if index.get(x) == "passed"]
    if failed:
        value, evidence = "failed", [f"required verification failed: {', '.join(failed)}"]
    elif missing or not_run:
        detail = []
        if missing:
            detail.append(f"missing: {', '.join(missing)}")
        if not_run:
            detail.append(f"not_run: {', '.join(not_run)}")
        value, evidence = "incomplete", [f"required verification incomplete ({'; '.join(detail)})"]
    else:
        value, evidence = "passed", [f"required verification passed: {len(passed)}/{len(required)}"]
    return {
        "value": value,
        "evidence": evidence,
        "sources": ["contract.verification", "summary.verification"],
        "required": required,
        "passed": passed,
        "failed": failed,
        "missing": missing,
        "not_run": not_run,
    }
