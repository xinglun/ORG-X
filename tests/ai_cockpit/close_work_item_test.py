"""Regression tests for the AI Cockpit close lifecycle boundary."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import ai_close_work_item


class CloseWorkItemBranchIdentityTests(unittest.TestCase):
    def test_adoption_start_receipt_preserves_installer_branch_identity(self) -> None:
        """The adoption close path must accept the installer-created branch name."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = root / ".ai" / "work-items" / "starts" / "adopt_ai_cockpit.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(
                json.dumps({"baseBranch": "adopt/ai-cockpit"}), encoding="utf-8"
            )

            previous_root = ai_close_work_item.PROJECT_ROOT
            ai_close_work_item.PROJECT_ROOT = root
            try:
                self.assertEqual(
                    ai_close_work_item._recorded_start_branch("adopt_ai_cockpit"),
                    "adopt/ai-cockpit",
                )
            finally:
                ai_close_work_item.PROJECT_ROOT = previous_root

    def test_recorded_start_receipt_preserves_noncanonical_branch_identity(self) -> None:
        """A historical Work Item may retain the exact branch recorded at start."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = root / ".ai" / "work-items" / "starts" / "wi-legacy.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(
                json.dumps({"baseBranch": "wi/legacy"}), encoding="utf-8"
            )

            previous_root = ai_close_work_item.PROJECT_ROOT
            ai_close_work_item.PROJECT_ROOT = root
            try:
                self.assertEqual(
                    ai_close_work_item._recorded_start_branch("wi-legacy"),
                    "wi/legacy",
                )
            finally:
                ai_close_work_item.PROJECT_ROOT = previous_root

    def test_branch_identity_accepts_canonical_or_exact_recorded_branch(self) -> None:
        """Branch compatibility must remain exact and preserve the canonical default."""
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = root / ".ai" / "work-items" / "starts" / "wi-legacy.json"
            receipt.parent.mkdir(parents=True)
            receipt.write_text(
                json.dumps({"baseBranch": "wi/legacy"}), encoding="utf-8"
            )

            previous_root = ai_close_work_item.PROJECT_ROOT
            ai_close_work_item.PROJECT_ROOT = root
            try:
                self.assertTrue(
                    ai_close_work_item._work_item_branch_matches(
                        "wi-legacy", "codex/wi-legacy"
                    )
                )
                self.assertTrue(
                    ai_close_work_item._work_item_branch_matches(
                        "wi-legacy", "wi/legacy"
                    )
                )
                self.assertFalse(
                    ai_close_work_item._work_item_branch_matches(
                        "wi-legacy", "wi/other"
                    )
                )
            finally:
                ai_close_work_item.PROJECT_ROOT = previous_root

    def test_canonical_branch_is_accepted_without_start_receipt(self) -> None:
        """Canonical Work Items do not depend on a compatibility receipt."""
        with tempfile.TemporaryDirectory() as directory:
            previous_root = ai_close_work_item.PROJECT_ROOT
            ai_close_work_item.PROJECT_ROOT = Path(directory)
            try:
                self.assertTrue(
                    ai_close_work_item._work_item_branch_matches(
                        "wi-canonical", "codex/wi-canonical"
                    )
                )
            finally:
                ai_close_work_item.PROJECT_ROOT = previous_root


if __name__ == "__main__":
    unittest.main()
