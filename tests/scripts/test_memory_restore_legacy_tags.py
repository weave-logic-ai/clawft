#!/usr/bin/env python3
"""Unit tests for scripts/memory-restore-legacy-tags.py (WEFT-670)."""

from __future__ import annotations

import json
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SCRIPT = REPO / "scripts" / "memory-restore-legacy-tags.py"


def _run(args: list[str], check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=check,
    )


class RestoreLegacyTagsTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.db = self.root / "agentdb-memory.db"
        self.manifest = self.root / "manifest.json"

        conn = sqlite3.connect(self.db)
        conn.execute(
            """
            CREATE TABLE memory_entries (
              id TEXT PRIMARY KEY,
              key TEXT NOT NULL,
              namespace TEXT NOT NULL,
              content TEXT,
              tags TEXT,
              status TEXT,
              access_count INTEGER DEFAULT 0
            )
            """
        )
        # content sha256 of "hello-tags"
        content = "hello-tags"
        import hashlib

        sha = hashlib.sha256(content.encode()).hexdigest()
        conn.execute(
            "INSERT INTO memory_entries VALUES (?,?,?,?,?,?,?)",
            ("1", "k1", "ns1", content, "[]", "active", 0),
        )
        conn.execute(
            "INSERT INTO memory_entries VALUES (?,?,?,?,?,?,?)",
            ("2", "k2", "ns1", "other", "[]", "active", 0),
        )
        conn.commit()
        conn.close()

        self.manifest.write_text(
            json.dumps(
                {
                    "source": "test",
                    "active_entries": 2,
                    "entries": [
                        {
                            "key": "k1",
                            "namespace": "ns1",
                            "tags": ["a", "b"],
                            "content_sha256": sha,
                        },
                        {
                            "key": "k2",
                            "namespace": "ns1",
                            "tags": ["x"],
                            "content_sha256": hashlib.sha256(b"wrong").hexdigest(),
                        },
                    ],
                }
            )
        )

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def test_dry_run_and_apply_restore_matching_sha(self) -> None:
        out = _run(
            [
                "--manifest",
                str(self.manifest),
                "--db",
                str(self.db),
                "--dry-run",
            ]
        )
        self.assertIn("would_restore: 1", out.stdout)
        self.assertIn("sha_mismatch:  1", out.stdout)

        _run(["--manifest", str(self.manifest), "--db", str(self.db)])
        conn = sqlite3.connect(self.db)
        tags = json.loads(
            conn.execute(
                "SELECT tags FROM memory_entries WHERE key='k1'"
            ).fetchone()[0]
        )
        conn.close()
        self.assertEqual(tags, ["a", "b"])

        verify = _run(
            ["--manifest", str(self.manifest), "--db", str(self.db), "--verify"],
            check=False,
        )
        # k2 still mismatches sha → exit 1, but k1 already_ok
        self.assertIn("already_ok:    1", verify.stdout)
        self.assertEqual(verify.returncode, 1)


if __name__ == "__main__":
    unittest.main()
