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


if __name__ == "__main__":
    unittest.main()
