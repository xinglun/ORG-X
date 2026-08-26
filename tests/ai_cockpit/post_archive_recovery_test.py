"""Regression tests for the documented post-archive recovery classifier."""

from __future__ import annotations

import ai_post_archive_recovery


def test_stale_knowledge_projection_failure_is_archive_evidence_recovery() -> None:
    output = "\n".join(
        [
            "[ERROR] wi-weekly-radar-content-quality.json: evidence docs/operations/WEEKLY_RADAR.md: digest is stale",
            "[ERROR] wi-weekly-radar-content-quality.json: verified record is not currently valid",
        ]
    )

    assert ai_post_archive_recovery.classify_failure(output) == "archiveEvidence"
