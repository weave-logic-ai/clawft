#!/usr/bin/env python3
"""WEFT-670: restore tags dropped by memory_import during the store-split migration.

The MCP memory_import path (pre-fix) re-stored key/namespace/value and
re-embedded, but left the tags column empty. The fidelity manifest captured
every tag set + content_sha256 so recovery is lossless for tags.

Usage:
  # dry-run against default paths
  scripts/memory-restore-legacy-tags.py --dry-run

  # apply (stop ruflo daemon first so sql.js does not clobber)
  ruflo daemon stop
  scripts/memory-restore-legacy-tags.py \\
    --manifest ~/.claude/backups/weftos-swarm-20260729-205855/legacy-fidelity-manifest.json \\
    --db .swarm/agentdb-memory.db

  # verify only
  scripts/memory-restore-legacy-tags.py --verify

Safety:
  - Only UPDATEs the tags column; never rewrites content/embeddings.
  - Skips rows whose content_sha256 does not match the manifest.
  - Skips rows that already have a non-empty tags array.
  - Default paths prefer CLAUDE_FLOW_DB_PATH / weftos .swarm/agentdb-memory.db.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import sys
from pathlib import Path
from typing import Any


DEFAULT_MANIFEST = Path.home() / (
    ".claude/backups/weftos-swarm-20260729-205855/legacy-fidelity-manifest.json"
)


def resolve_db(explicit: str | None) -> Path:
    if explicit:
        return Path(explicit).expanduser().resolve()
    env = os.environ.get("CLAUDE_FLOW_DB_PATH") or os.environ.get("CLAUDE_FLOW_MEMORY_PATH")
    if env:
        p = Path(env).expanduser()
        if p.is_dir():
            # Prefer agentdb-memory.db (schema-bearing live store) then memory.db
            for name in ("agentdb-memory.db", "memory.db"):
                cand = p / name
                if cand.exists():
                    return cand.resolve()
        return p.resolve()
    # Prefer main clone if worktree has no .swarm store
    candidates = [
        Path.cwd() / ".swarm" / "agentdb-memory.db",
        Path.home() / "weftos" / ".swarm" / "agentdb-memory.db",
        Path.cwd() / ".swarm" / "memory.db",
    ]
    for c in candidates:
        if c.exists():
            return c.resolve()
    raise SystemExit(
        "No memory DB found. Pass --db path/to/agentdb-memory.db "
        "(or set CLAUDE_FLOW_DB_PATH)."
    )


def parse_tags(raw: Any) -> list[str]:
    if raw is None or raw == "":
        return []
    if isinstance(raw, list):
        return [t for t in raw if isinstance(t, str) and t]
    if isinstance(raw, str):
        try:
            parsed = json.loads(raw)
            if isinstance(parsed, list):
                return [t for t in parsed if isinstance(t, str) and t]
        except json.JSONDecodeError:
            pass
        return [t.strip() for t in raw.split(",") if t.strip()]
    return []


def content_sha256(content: str | bytes | None) -> str:
    if content is None:
        data = b""
    elif isinstance(content, bytes):
        data = content
    else:
        data = content.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def load_manifest(path: Path) -> list[dict[str, Any]]:
    with path.open() as f:
        doc = json.load(f)
    entries = doc.get("entries") if isinstance(doc, dict) else doc
    if not isinstance(entries, list):
        raise SystemExit(f"Manifest has no entries list: {path}")
    return entries


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST, help="legacy-fidelity-manifest.json")
    ap.add_argument("--db", type=str, default=None, help="path to agentdb-memory.db / memory.db")
    ap.add_argument("--dry-run", action="store_true", help="report what would change; no writes")
    ap.add_argument("--verify", action="store_true", help="check restored state only; no writes")
    ap.add_argument("--force", action="store_true", help="overwrite non-empty tags that disagree")
    args = ap.parse_args()

    manifest_path = args.manifest.expanduser().resolve()
    if not manifest_path.exists():
        raise SystemExit(f"Manifest not found: {manifest_path}")

    db_path = resolve_db(args.db)
    entries = load_manifest(manifest_path)
    tagged = [e for e in entries if parse_tags(e.get("tags"))]
    print(f"manifest: {manifest_path}")
    print(f"db:       {db_path}")
    print(f"entries:  {len(entries)} total, {len(tagged)} with tags")

    conn = sqlite3.connect(str(db_path))
    conn.row_factory = sqlite3.Row

    # Detect table
    tables = {r[0] for r in conn.execute("SELECT name FROM sqlite_master WHERE type='table'")}
    if "memory_entries" not in tables:
        raise SystemExit(f"No memory_entries table in {db_path}")

    would_restore = 0
    already_ok = 0
    missing = 0
    sha_mismatch = 0
    disagree = 0
    restored = 0
    errors: list[str] = []

    for e in tagged:
        key = e["key"]
        ns = e["namespace"]
        want = parse_tags(e.get("tags"))
        expected_sha = e.get("content_sha256")
        row = conn.execute(
            """
            SELECT key, namespace, content, tags, status
            FROM memory_entries
            WHERE key = ? AND namespace = ?
              AND (status IS NULL OR status = 'active')
            LIMIT 1
            """,
            (key, ns),
        ).fetchone()
        if row is None:
            missing += 1
            errors.append(f"missing: {ns}/{key}")
            continue
        sha = content_sha256(row["content"])
        if expected_sha and sha != expected_sha:
            sha_mismatch += 1
            errors.append(f"sha mismatch: {ns}/{key}")
            continue
        have = parse_tags(row["tags"])
        if have == want:
            already_ok += 1
            continue
        if have and not args.force:
            disagree += 1
            errors.append(f"tags disagree (use --force): {ns}/{key} have={have} want={want}")
            continue
        would_restore += 1
        if args.verify or args.dry_run:
            continue
        conn.execute(
            """
            UPDATE memory_entries
            SET tags = ?
            WHERE key = ? AND namespace = ?
              AND (status IS NULL OR status = 'active')
            """,
            (json.dumps(want), key, ns),
        )
        restored += 1

    if not args.verify and not args.dry_run and restored:
        conn.commit()
    conn.close()

    mode = "verify" if args.verify else ("dry-run" if args.dry_run else "apply")
    print(f"mode:     {mode}")
    print(f"already_ok:    {already_ok}")
    if mode == "apply":
        print(f"restored:      {restored}")
    else:
        print(f"would_restore: {would_restore}")
    print(f"missing:       {missing}")
    print(f"sha_mismatch:  {sha_mismatch}")
    print(f"disagree:      {disagree}")
    if errors and (missing or sha_mismatch or disagree):
        print("--- first issues ---")
        for line in errors[:20]:
            print(f"  {line}")
        if len(errors) > 20:
            print(f"  ... +{len(errors) - 20} more")

    # Exit non-zero if verification finds unrepaired tagged entries that
    # still need work (would_restore) or hard failures (missing/disagree).
    # sha_mismatch alone is a skip, not a restore failure, on apply/dry-run.
    if args.verify and (would_restore or missing or sha_mismatch or disagree):
        return 1
    if not args.verify and not args.dry_run and missing:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
