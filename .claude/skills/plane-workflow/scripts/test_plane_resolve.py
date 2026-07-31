#!/usr/bin/env python3
"""Offline unit tests for WEFT-639 plane.py helpers (no network).

Run:
  python3 .claude/skills/plane-workflow/scripts/test_plane_resolve.py
"""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "plane.py"


def load_plane():
    spec = importlib.util.spec_from_file_location("plane_mod", SCRIPT)
    mod = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(mod)
    return mod


plane = load_plane()


class TestParseWeft(unittest.TestCase):
    def test_weft_prefix(self):
        self.assertEqual(plane.parse_weft_sequence("WEFT-630"), 630)
        self.assertEqual(plane.parse_weft_sequence("weft-42"), 42)

    def test_bare_number(self):
        self.assertEqual(plane.parse_weft_sequence("630"), 630)

    def test_uuid_not_sequence(self):
        self.assertIsNone(
            plane.parse_weft_sequence("1c3f125d-abdd-4651-9cfc-9d8cb599cbc3")
        )
        self.assertTrue(
            plane.is_uuid("1c3f125d-abdd-4651-9cfc-9d8cb599cbc3")
        )


class TestResolveIssueId(unittest.TestCase):
    def test_uuid_passthrough(self):
        uid = "1c3f125d-abdd-4651-9cfc-9d8cb599cbc3"
        self.assertEqual(plane.resolve_issue_id(uid), uid)

    def test_weft_uses_cache(self):
        plane._ISSUE_ID_CACHE[99901] = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        self.assertEqual(
            plane.resolve_issue_id("WEFT-99901"),
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        )
        plane._ISSUE_ID_CACHE.pop(99901, None)

    def test_bad_arg(self):
        with self.assertRaises(plane.PlaneError):
            plane.resolve_issue_id("not-an-id")


class TestCycleMembership(unittest.TestCase):
    def test_cycle_id_field(self):
        issue = {"cycle_id": "76a2e899-a3fd-4fdd-ab88-5310d458bb22"}
        self.assertTrue(
            plane.issue_in_cycle(
                issue, "76a2e899-a3fd-4fdd-ab88-5310d458bb22", "0.8.x"
            )
        )

    def test_cycles_list_name(self):
        issue = {"cycles": ["0.8.x"]}
        self.assertTrue(
            plane.issue_in_cycle(
                issue, "76a2e899-a3fd-4fdd-ab88-5310d458bb22", "0.8.x"
            )
        )

    def test_cycles_list_objects(self):
        issue = {
            "cycles": [
                {"id": "76a2e899-a3fd-4fdd-ab88-5310d458bb22", "name": "0.8.x"}
            ]
        }
        self.assertTrue(
            plane.issue_in_cycle(
                issue, "76a2e899-a3fd-4fdd-ab88-5310d458bb22", "0.8.x"
            )
        )

    def test_not_member(self):
        issue = {"cycle_id": "other"}
        self.assertFalse(
            plane.issue_in_cycle(
                issue, "76a2e899-a3fd-4fdd-ab88-5310d458bb22", "0.8.x"
            )
        )


class TestResolveUser(unittest.TestCase):
    def test_explicit_uuid(self):
        ids = {"workspace_slug": "weftos"}
        uid = "0d63f76f-0231-49e8-b81a-b2471bb7b91a"
        self.assertEqual(plane._resolve_user(ids, uid), uid)

    def test_me_from_ids_json_field(self):
        ids = {
            "workspace_slug": "weftos",
            "me_user_id": "0d63f76f-0231-49e8-b81a-b2471bb7b91a",
        }
        self.assertEqual(
            plane._resolve_user(ids, "me"),
            "0d63f76f-0231-49e8-b81a-b2471bb7b91a",
        )

    def test_bot_filter(self):
        self.assertTrue(
            plane._looks_like_bot(
                {"email": "github-actions[bot]@users.noreply.github.com"}
            )
        )
        self.assertFalse(
            plane._looks_like_bot(
                {"email": "human@example.com", "display_name": "aepod23"}
            )
        )


if __name__ == "__main__":
    # Avoid network: me_user_id path must not call API.
    suite = unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    sys.exit(0 if result.wasSuccessful() else 1)
