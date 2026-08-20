"""Regression coverage for the AI Cockpit installer feature checklist."""

from __future__ import annotations

import json
import hashlib
import re
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class UpgradeFeatureChecklistTests(unittest.TestCase):
    def test_catalog_and_runtime_surface_are_complete(self) -> None:
        checklist = json.loads(
            (ROOT / ".ai/evidence/ai-cockpit-upgrade-feature-checklist.json").read_text(
                encoding="utf-8"
            )
        )
        catalog = json.loads((ROOT / "scripts/ai_installer_catalog.json").read_text(encoding="utf-8"))
        inventory = checklist["installerCatalog"]["inventory"]
        installed = checklist["installerCatalog"]["installed"]
        self.assertEqual(installed["missingFromMain"], [])
        self.assertEqual(installed["extraAgainstMain"], [])
        self.assertEqual(installed["missingOnDisk"], [])
        self.assertEqual(sorted(catalog["stacks"]), inventory["stacks"])
        self.assertEqual(sorted(catalog["scripts"]), inventory["scripts"])

        self.assertEqual(len(catalog["stacks"]), 13)
        self.assertEqual(len(catalog["scripts"]), 115)
        self.assertTrue(all((ROOT / "scripts" / name).is_file() for name in catalog["scripts"]))

        items = [
            {
                "name": name,
                "sha256": hashlib.sha256((ROOT / "scripts" / name).read_bytes()).hexdigest(),
            }
            for name in sorted(catalog["scripts"])
        ]
        installed_digest = hashlib.sha256(
            json.dumps(items, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()
        ).hexdigest()
        self.assertEqual(installed["scriptInventoryDigest"], installed_digest)
        self.assertEqual(
            checklist["installerCatalog"]["main"]["scriptInventoryDigest"],
            "3bc836bf109748301e99c0faf07b6f4950fbe46a67f0ae4f71445643fba7e9a3",
        )
        self.assertEqual(
            checklist["installerCatalog"]["publishedRelease"]["scriptInventoryDigest"],
            "3bc836bf109748301e99c0faf07b6f4950fbe46a67f0ae4f71445643fba7e9a3",
        )
        differences = sorted(installed["scriptContentDifferencesFromPublishedRelease"])
        preserved = {item["path"]: item for item in installed["preservedAdopterCustomizations"]}
        self.assertEqual(differences, sorted(preserved))
        for path, item in preserved.items():
            self.assertEqual(item["installedSha256"], hashlib.sha256((ROOT / path).read_bytes()).hexdigest())
            self.assertTrue(item["publishedSha256"])

    def test_declared_runtime_targets_are_present(self) -> None:
        checklist = json.loads(
            (ROOT / ".ai/evidence/ai-cockpit-upgrade-feature-checklist.json").read_text(
                encoding="utf-8"
            )
        )
        makefile = (ROOT / "Makefile.ai").read_text(encoding="utf-8")
        declared = {
            match.group(1)
            for line in makefile.splitlines()
            if (match := re.match(r"^([^:=\s]+):(?!=)", line))
        }
        expected = set(checklist["runtimeSurface"]["targets"])
        self.assertEqual(checklist["runtimeSurface"]["missingTargets"], [])
        self.assertEqual(checklist["runtimeSurface"]["requiredTargetCount"], len(expected))
        self.assertEqual(len(expected), 19)
        self.assertTrue(expected <= declared)


if __name__ == "__main__":
    unittest.main()
