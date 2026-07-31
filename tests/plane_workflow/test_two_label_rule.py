"""Offline unit tests for WEFT-682 two-label enforcement in plane.py.

Run:
    python3 -m pytest tests/plane_workflow/test_two_label_rule.py -q
or:
    python3 tests/plane_workflow/test_two_label_rule.py
"""

from __future__ import annotations

import sys
from pathlib import Path

SCRIPTS = (
    Path(__file__).resolve().parents[2]
    / ".claude"
    / "skills"
    / "plane-workflow"
    / "scripts"
)
sys.path.insert(0, str(SCRIPTS))

import plane  # noqa: E402


def test_accepts_workstream_plus_finding_type() -> None:
    plane.validate_two_label_rule(["ws15-mcp", "tech-debt"])
    plane.validate_two_label_rule(["ws18-firmware", "bug", "audit-finding"])
    plane.validate_two_label_rule(["ws02-kernel", "security"])  # handoff FP fix


def test_rejects_empty() -> None:
    try:
        plane.validate_two_label_rule([])
    except plane.PlaneError as e:
        assert "two-label" in str(e).lower() or "required" in str(e).lower()
        return
    raise AssertionError("expected PlaneError for empty labels")


def test_rejects_workstream_only() -> None:
    try:
        plane.validate_two_label_rule(["ws05-channels"])
    except plane.PlaneError as e:
        assert "finding-type" in str(e)
        return
    raise AssertionError("expected PlaneError for workstream-only")


def test_rejects_finding_type_only() -> None:
    try:
        plane.validate_two_label_rule(["gap"])
    except plane.PlaneError as e:
        assert "workstream" in str(e)
        return
    raise AssertionError("expected PlaneError for finding-type-only")


def test_classify_labels() -> None:
    ws, ft = plane.classify_labels(["ws18-firmware", "performance", "noise"])
    assert ws == ["ws18-firmware"]
    assert ft == ["performance"]


def test_create_issue_cli_requires_labels() -> None:
    # argparse exits 2 when --labels is omitted
    try:
        plane.main(["create-issue", "--name", "no-labels-item"])
    except SystemExit as e:
        assert e.code not in (0, None)
        return
    # If main returns instead of raising SystemExit
    # (depends on argparse setup), a non-zero rc is also fine.
    raise AssertionError("expected non-zero exit when --labels missing")


def test_create_issue_cli_rejects_one_sided_labels() -> None:
    rc = plane.main(
        [
            "create-issue",
            "--name",
            "ws-only",
            "--labels",
            "ws15-mcp",
        ]
    )
    assert rc == 1


if __name__ == "__main__":
    tests = [
        test_accepts_workstream_plus_finding_type,
        test_rejects_empty,
        test_rejects_workstream_only,
        test_rejects_finding_type_only,
        test_classify_labels,
        test_create_issue_cli_requires_labels,
        test_create_issue_cli_rejects_one_sided_labels,
    ]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"ok  {t.__name__}")
        except Exception as e:  # noqa: BLE001 — test runner
            failed += 1
            print(f"FAIL {t.__name__}: {e}")
    if failed:
        sys.exit(1)
    print(f"all {len(tests)} passed")
