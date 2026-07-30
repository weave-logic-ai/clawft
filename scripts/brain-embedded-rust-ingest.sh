#!/usr/bin/env bash
# brain-embedded-rust-ingest.sh — validate / inspect / re-ingest the
# `embedded-rust` brain corpus (.planning/embedded-rust/brain/).
#
# See .planning/embedded-rust/brain/README.md for the trust model and the
# refresh procedure. chunks.jsonl is the reviewable ingest payload.
#
# ─────────────────────────────────────────────────────────────────────────
# ⚠ READ THIS BEFORE USING `ingest`
#
# This repo currently has TWO memory store files, and the CLI and the MCP
# server disagree about which one is authoritative:
#
#   .swarm/memory.db            188 entries, last written 2026-07-03.
#                               Schema has NO `provenance_type` column.
#                               This is what `ruflo memory *` defaults to
#                               (its documented default is cwd/.swarm/memory.db).
#
#   .swarm/agentdb-memory.db    The live store. Schema ADDS `provenance_type`
#                               (ADR-323). This is what the claude-flow MCP
#                               memory_* tools read and write.
#
# Consequences:
#   1. `ruflo memory store` WITHOUT `--path` writes to the ORPHANED store,
#      where the MCP tools (and therefore the agents) will never see it.
#   2. The MCP backend is `sql.js + HNSW` — in-memory SQLite serialized to
#      disk. An external writer touching the same file while the MCP server
#      holds it can be CLOBBERED by the server's next flush.
#
# Therefore the SAFE and VERIFIED ingest path is the MCP `memory_store`
# tool, from inside a Claude Code session — the same process that reads it.
# That is how the 40 chunks currently in the store were written.
#
# `ingest` here exists for headless/CI use. It requires an explicit
# --path and refuses to guess. Stop the MCP server first.
# ─────────────────────────────────────────────────────────────────────────

set -euo pipefail

BRAIN_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/.planning/embedded-rust/brain"
CHUNKS="$BRAIN_DIR/chunks.jsonl"
NAMESPACE="embedded-rust"

usage() {
    cat <<'EOF'
Usage: scripts/brain-embedded-rust-ingest.sh <command> [options]

Commands:
  validate            Validate chunks.jsonl: JSON well-formedness, required
                      fields, duplicate keys, trust-tier tally. (default)
  plan                Print what would be ingested — key, trust tier, byte
                      size — without writing anything.
  stores              Report both memory store files: path, size, mtime,
                      entry count, and whether the schema has provenance_type.
  ingest --path DB    Ingest into an EXPLICIT store file via the ruflo CLI.
                      Refuses to run without --path. Stop the MCP server first.

Options:
  --path DB           Target store file (required for `ingest`).
  --dry-run           With `ingest`, print the commands instead of running them.
  -h, --help          This message.

Notes:
  * Requires python3 for JSON handling; `ingest` additionally requires the
    GLOBAL `ruflo` binary (never `npx ruflo@latest`, which uses its own
    private copy and hides drift).
  * `memory store` is upsert-by-default, so re-ingest is idempotent per key.
  * On refresh, regenerate chunks.jsonl from notes/ first, then `validate`,
    then diff against `plan` output to see what actually changed.
EOF
}

require_python() {
    command -v python3 >/dev/null 2>&1 || { echo "error: python3 not found" >&2; exit 1; }
}

cmd_validate() {
    require_python
    [[ -f "$CHUNKS" ]] || { echo "error: not found: $CHUNKS" >&2; exit 1; }
    python3 - "$CHUNKS" <<'PY'
import json, sys, collections
path = sys.argv[1]
required = ("key", "content", "type", "trust", "source_file")
tiers = ("upstream-official", "ecosystem-canon", "in-repo-verified")
keys, errors, tally = [], [], collections.Counter()
for n, line in enumerate(open(path), 1):
    line = line.strip()
    if not line:
        continue
    try:
        obj = json.loads(line)
    except Exception as exc:
        errors.append(f"line {n}: invalid JSON — {exc}")
        continue
    for field in required:
        if field not in obj:
            errors.append(f"line {n} ({obj.get('key','?')}): missing '{field}'")
    if obj.get("trust") not in tiers:
        errors.append(f"line {n} ({obj.get('key','?')}): unknown trust tier {obj.get('trust')!r}")
    if obj.get("trust") != "in-repo-verified" and "source_url" not in obj:
        errors.append(f"line {n} ({obj.get('key','?')}): external tier requires source_url")
    keys.append(obj.get("key"))
    tally[obj.get("trust")] += 1
dupes = [k for k, c in collections.Counter(keys).items() if c > 1]
print(f"chunks:      {len(keys)}")
print(f"trust tiers: " + ", ".join(f"{t}={tally[t]}" for t in tiers))
print(f"duplicates:  {dupes or 'none'}")
if dupes:
    errors.append(f"duplicate keys: {dupes}")
if errors:
    print("\nFAILED:")
    for e in errors:
        print("  -", e)
    sys.exit(1)
print("\nOK — payload is well-formed.")
PY
}

cmd_plan() {
    require_python
    python3 - "$CHUNKS" "$NAMESPACE" <<'PY'
import json, sys
path, ns = sys.argv[1], sys.argv[2]
print(f"namespace: {ns}\n")
print(f"{'KEY':<45} {'TIER':<20} {'TYPE':<12} BYTES")
print("-" * 92)
total = 0
for line in open(path):
    line = line.strip()
    if not line:
        continue
    o = json.loads(line)
    size = len(o["content"].encode())
    total += size
    print(f"{o['key']:<45} {o['trust']:<20} {o.get('type',''):<12} {size}")
print("-" * 92)
print(f"{'TOTAL':<45} {'':<20} {'':<12} {total}")
PY
}

cmd_stores() {
    require_python
    local repo_root
    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    for db in "$repo_root/.swarm/memory.db" "$repo_root/.swarm/agentdb-memory.db"; do
        echo "── $db"
        if [[ ! -f "$db" ]]; then
            echo "   (absent)"
            continue
        fi
        # Copy aside so we never touch a live WAL.
        tmp="$(mktemp -d)"
        cp "$db" "$tmp/db" 2>/dev/null || true
        for ext in -wal -shm; do
            [[ -f "${db}${ext}" ]] && cp "${db}${ext}" "$tmp/db${ext}" 2>/dev/null || true
        done
        echo "   size:  $(wc -c < "$db" | tr -d ' ') bytes"
        echo "   mtime: $(date -r "$db" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || stat -c %y "$db")"
        if command -v sqlite3 >/dev/null 2>&1; then
            # Count ACTIVE rows only. Soft-deleted rows (status='deleted') stay
            # in the table but are excluded from memory_search / memory_stats, so
            # counting all rows makes this tool disagree with the MCP tools —
            # which is the exact confusion it exists to prevent.
            act=$(sqlite3 "$tmp/db" "SELECT COUNT(*) FROM memory_entries WHERE status='active';" 2>/dev/null || echo '?')
            del=$(sqlite3 "$tmp/db" "SELECT COUNT(*) FROM memory_entries WHERE status<>'active';" 2>/dev/null || echo '?')
            echo "   entries: $act active${del:+ (+$del soft-deleted, excluded from search)}"
            echo "   namespaces (active only):"
            # NB: single-quoted SQL string literals — SQLite reads "..." as an
            # identifier, so double quotes here silently yield no rows.
            sqlite3 "$tmp/db" "SELECT '     ' || namespace || ' = ' || COUNT(*) FROM memory_entries WHERE status='active' GROUP BY namespace ORDER BY COUNT(*) DESC;" 2>/dev/null || echo "     ?"
            if sqlite3 "$tmp/db" 'PRAGMA table_info(memory_entries);' 2>/dev/null | grep -q provenance_type; then
                echo "   schema: HAS provenance_type  →  this is the LIVE (MCP) store"
            else
                echo "   schema: no provenance_type   →  this is the LEGACY store"
            fi
        else
            echo "   (sqlite3 not available — install it for entry counts)"
        fi
        rm -rf "$tmp"
        echo
    done
}

cmd_ingest() {
    require_python
    local db="" dry=0
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --path) db="${2:-}"; shift 2 ;;
            --dry-run) dry=1; shift ;;
            *) echo "error: unknown option for ingest: $1" >&2; exit 1 ;;
        esac
    done
    if [[ -z "$db" ]]; then
        cat >&2 <<'EOF'
error: `ingest` requires an explicit --path.

This is deliberate. The ruflo CLI defaults to cwd/.swarm/memory.db, which in
this repo is the ORPHANED legacy store — writing there would silently put the
brain somewhere the agents cannot read.

The live store is .swarm/agentdb-memory.db. STOP THE MCP SERVER before
writing to it: its sql.js backend keeps state in memory and can clobber an
external writer on its next flush.

Prefer the MCP `memory_store` tool from inside a Claude Code session. Run
`scripts/brain-embedded-rust-ingest.sh stores` to see the current state.
EOF
        exit 1
    fi
    command -v ruflo >/dev/null 2>&1 || { echo "error: global \`ruflo\` not found on PATH" >&2; exit 1; }

    echo "⚠ target store: $db"
    echo "⚠ ensure the claude-flow MCP server is STOPPED before continuing."
    echo

    python3 - "$CHUNKS" "$NAMESPACE" <<'PY' > /tmp/er-ingest-args.txt
import json, sys
path, ns = sys.argv[1], sys.argv[2]
for line in open(path):
    line = line.strip()
    if not line:
        continue
    o = json.loads(line)
    tags = ",".join([o.get("trust", ""), o.get("type", ""), "embedded-rust"])
    # value carries content + metadata so provenance survives retrieval
    value = json.dumps({k: v for k, v in o.items() if k != "key"}, separators=(",", ":"))
    print("\x1f".join([o["key"], ns, tags, value]))
PY

    local count=0
    while IFS=$'\x1f' read -r key ns tags value; do
        count=$((count + 1))
        if [[ $dry -eq 1 ]]; then
            printf 'ruflo memory store --key %q --namespace %q --tags %q --path %q --value <%d bytes>\n' \
                "$key" "$ns" "$tags" "$db" "${#value}"
        else
            ruflo memory store \
                --key "$key" \
                --namespace "$ns" \
                --tags "$tags" \
                --path "$db" \
                --value "$value" >/dev/null
            echo "  stored $key"
        fi
    done < /tmp/er-ingest-args.txt
    rm -f /tmp/er-ingest-args.txt

    if [[ $dry -eq 1 ]]; then
        echo
        echo "dry run — $count chunks would be stored."
    else
        echo
        echo "$count chunks stored into $db"
        echo "VERIFY before trusting: re-run \`stores\`, and query via the MCP"
        echo "memory_search tool scoped to --namespace $NAMESPACE."
    fi
}

case "${1:-validate}" in
    validate)      cmd_validate ;;
    plan)          cmd_plan ;;
    stores)        cmd_stores ;;
    ingest)        shift; cmd_ingest "$@" ;;
    -h|--help)     usage ;;
    *)             echo "error: unknown command '${1}'" >&2; echo >&2; usage >&2; exit 1 ;;
esac
