#!/usr/bin/env python3
"""Validate the reader documentation surface and its repository facts."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
READER_ROOTS = (
    ROOT / "README.md",
    ROOT / "NORTH_STAR.md",
    ROOT / "ENGINEERING_PRINCIPLES.md",
    ROOT / "docs" / "README.md",
)
READER_DIRECTORIES = (
    "product",
    "architecture",
    "data",
    "domain",
    "scoring",
    "validation",
    "operations",
)
FORBIDDEN_READER_MARKERS = (
    "delivery status",
    "active work item",
    "next work item",
    "next wi",
    "in progress",
    "完成并归档",
    "下一项",
    "下一步",
)
LINK_RE = re.compile(r"\[[^]]+\]\(([^)]+)\)")


def reader_files() -> list[Path]:
    files = [path for path in READER_ROOTS if path.is_file()]
    for directory in READER_DIRECTORIES:
        files.extend(sorted((ROOT / "docs" / directory).glob("*.md")))
    return files


def check_headings(path: Path, text: str) -> list[str]:
    headings = [line for line in text.splitlines() if line.startswith("#")]
    if sum(line.startswith("# ") for line in headings) != 1:
        return [f"{path.relative_to(ROOT)}: expected exactly one level-one heading"]
    return []


def check_links(path: Path, text: str) -> list[str]:
    errors: list[str] = []
    for target in LINK_RE.findall(text):
        target = target.strip().split("#", 1)[0]
        if not target or "://" in target or target.startswith("mailto:"):
            continue
        resolved = (path.parent / target).resolve()
        if not resolved.exists():
            errors.append(f"{path.relative_to(ROOT)}: missing internal link target {target}")
    return errors


def check_reader_markers(path: Path, text: str) -> list[str]:
    lowered = text.casefold()
    return [
        f"{path.relative_to(ROOT)}: reader document contains process marker {marker!r}"
        for marker in FORBIDDEN_READER_MARKERS
        if marker.casefold() in lowered
    ]


def check_runtime_alignment() -> list[str]:
    errors: list[str] = []
    workflow = (ROOT / ".github" / "workflows" / "weekly-radar.yml").read_text(
        encoding="utf-8"
    )
    operations = (ROOT / "docs" / "operations" / "WEEKLY_RADAR.md").read_text(
        encoding="utf-8"
    )
    if "cron: '0 0 * * 1'" not in workflow or "0 0 * * 1" not in operations:
        errors.append("Weekly Radar schedule is not consistently documented as Monday 09:00 JST")
    if "actions/checkout@v5" not in workflow or "actions/checkout@v5" not in operations:
        errors.append("Weekly Radar checkout version is not consistently documented as v5")
    if "actions/checkout@v4" in workflow or "actions/checkout@v4" in operations:
        errors.append("reader or workflow documentation still mentions checkout@v4")
    for name in ("ORGX_SEC_USER_AGENT", "ORGX_TELEGRAM_BOT_TOKEN", "ORGX_TELEGRAM_CHAT_ID"):
        if name not in operations:
            errors.append(f"Weekly Radar operations documentation is missing {name}")
    if "365" not in operations or "data" not in operations:
        errors.append("Weekly Radar data branch retention is not documented")
    return errors


def main() -> int:
    errors: list[str] = []
    files = reader_files()
    if not files:
        print("documentation metadata check failed: no reader documents found", file=sys.stderr)
        return 1
    for path in files:
        text = path.read_text(encoding="utf-8")
        errors.extend(check_headings(path, text))
        errors.extend(check_links(path, text))
        errors.extend(check_reader_markers(path, text))
    errors.extend(check_runtime_alignment())
    if errors:
        print("documentation metadata check failed:", file=sys.stderr)
        print("\n".join(f"- {error}" for error in errors), file=sys.stderr)
        return 1
    print(f"documentation metadata check passed: {len(files)} reader documents")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
