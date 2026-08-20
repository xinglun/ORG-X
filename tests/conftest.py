import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))


@pytest.fixture(autouse=True)
def isolate_ci_diff_baseline(monkeypatch):
    """Keep workflow-level baselines out of temporary Git repositories."""
    monkeypatch.delenv("AI_BASE_COMMIT", raising=False)
