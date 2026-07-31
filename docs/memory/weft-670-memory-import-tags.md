# WEFT-670 — `memory_import` drops the tags column

**Status:** fixed (forward path) + restored (legacy 128 tag sets)  
**Cycle:** 0.8.x · **Labels:** ws06-memory, gap

## Problem

During the store-split migration (legacy `.swarm/memory.db` → live
`.swarm/agentdb-memory.db`), entries were moved with the MCP
`memory_import` tool. A two-entry probe showed:

| Field | After import |
|-------|----------------|
| key / namespace / value | preserved |
| embeddings | re-computed |
| **tags** | **empty** |
| created_at | reset to import time |
| access_count | reset to 0 |
| provenance_type | `unknown` |

Of 182 migrated entries, **128** had tags (306 distinct tag labels).
Nothing was permanently lost: every tag set + original timestamp +
access count was captured in:

`~/.claude/backups/weftos-swarm-20260729-205855/legacy-fidelity-manifest.json`

keyed by `(namespace, key)` with `content_sha256` for byte verification.

## Root cause (upstream ruflo / `@claude-flow/cli`)

In `v3/@claude-flow/cli/src/mcp-tools/memory-tools.ts`:

1. **`memory_export`** mapped only key/namespace/value/timestamps — **no `tags`**.
2. **`memory_import`** called `storeEntry({ key, value, namespace, upsert })` — **no `tags`**.
3. **`listEntries` / `bridgeListEntries`** `SELECT` omitted the `tags` column, so even a fixed exporter had nothing to emit.
4. **`memory_list`** description claimed “optionally filtered by namespace/tags” but the input schema had **no `tags` parameter** and responses omitted tags.

`memory_store` and `memory_retrieve` already carried tags end-to-end; the
backend column and write path were fine.

## Fix (forward-looking, lossless migrations)

Upstream changes (linked local ruflo `3.32.38` build / weave-logic-ai fork):

| Surface | Change |
|---------|--------|
| `listEntries` / `bridgeListEntries` | SELECT + return `tags`; optional `tags: string[]` AND-filter |
| `memory_list` | schema gains `tags`; response includes `tags` |
| `memory_export` | schema `ruflo-memory-export/v1.1`; each entry includes `tags: string[]` |
| `memory_import` | passes `tags` into `storeEntry`; reports `imported.withTags` |

v1 export files remain importable (missing tags → `[]`). v1.1 exports
round-trip tags.

**Pin note (WEFT-684):** WeftOS still declares `"ruflo": "3.32.38"`. Machines
using the published npm artifact do not get this fix until a deliberate pin
bump after the change is published. Local link
`~/.nvm/.../ruflo → ~/dev/ruflo/ruflo` with a rebuilt
`v3/@claude-flow/cli` dist is the development path.

## Legacy restore (this repo)

Script: [`scripts/memory-restore-legacy-tags.py`](../../scripts/memory-restore-legacy-tags.py)

- Reads the fidelity manifest.
- `UPDATE memory_entries SET tags = ?` only when `content_sha256` matches.
- Never rewrites content or embeddings (avoids re-embed cost and sql.js
  clobber races when used with the daemon stopped).

```bash
# stop MCP/daemon writers first
ruflo daemon stop

scripts/memory-restore-legacy-tags.py --dry-run
scripts/memory-restore-legacy-tags.py \
  --manifest ~/.claude/backups/weftos-swarm-20260729-205855/legacy-fidelity-manifest.json \
  --db ~/weftos/.swarm/agentdb-memory.db

scripts/memory-restore-legacy-tags.py --verify
```

### Acceptance criteria checklist

| Criterion | Outcome |
|-----------|---------|
| Determine whether a tag-filtered read path exists | **Yes** after fix: `memory_list.tags`, backend `listEntries({ tags })`, plus `memory_retrieve` always returned tags. Schema/description discrepancy resolved. |
| If queryable: restore 128 tag sets from fidelity manifest | **Done** via `scripts/memory-restore-legacy-tags.py` (tags-only SQL UPDATE, sha-gated). |
| Record fidelity limits for next migration | **This doc** + export format v1.1. `created_at` / `access_count` still intentionally unrestored (lower value; still in the manifest). |

## Not restored (by design)

- Original `created_at` / `updated_at`
- Original `access_count`
- Original `provenance_type` (legacy store had no column)

Those remain in the fidelity manifest if a future pass wants them.

## Related

- Parent: store-split migration / two-file AgentDB divergence
- WEFT-684 — ruflo pin / schema ownership of `.swarm/agentdb-memory.db`
- Handoff: `docs/handoff-tracker-ci-memory.md` (manifest path, clobber warning)
