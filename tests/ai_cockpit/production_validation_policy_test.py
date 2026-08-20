"""Regression tests for the bounded Weekly Radar production-validation gate."""

from __future__ import annotations

import unittest

from ai_critical_domain_guards import critical_domain_signals
from ai_trust_guards import intent_capability_signal


def bounded_contract() -> dict[str, object]:
    return {
        "requestedOperation": {
            "type": "external_operation",
            "action": "dispatch",
            "target": "weekly_radar_validation",
            "environment": "production",
            "effect": "publish_weekly_radar_validation",
            "authorityRequired": True,
        },
        "authorityEvidence": {
            "type": "human_authorization",
            "granted": True,
            "scope": "bounded Weekly Radar validation",
        },
        "intent": {
            "problem": "Validate the merged Weekly Radar runtime with real Providers.",
            "rationale": "The exact bounded operation is required for evidence.",
            "constraints": ["Never expose Secret values."],
        },
    }


class ProductionValidationPolicyTests(unittest.TestCase):
    def test_exact_bounded_operation_is_allowed_with_authority(self) -> None:
        contract = bounded_contract()
        critical, _, _, production = critical_domain_signals(contract)
        self.assertEqual(critical["value"], "Ready")
        self.assertEqual(production["value"], "Ready")
        self.assertEqual(intent_capability_signal(contract)["value"], "Ready")

    def test_unbounded_production_operation_remains_blocked(self) -> None:
        contract = bounded_contract()
        operation = contract["requestedOperation"]
        assert isinstance(operation, dict)
        operation["target"] = "arbitrary_production_system"
        operation["effect"] = "publish"
        critical, _, _, production = critical_domain_signals(contract)
        self.assertEqual(critical["value"], "Inconsistent")
        self.assertEqual(production["value"], "Ready")


if __name__ == "__main__":
    unittest.main()
