# WEFT-84 result — memory reindex when MEMORY.md changes (MW-6)

**Ticket:** WEFT-84  
**Branch:** `wave0e/weft-84-memory-reindex`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef4-7621-9495-d5c4a6919b59`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-84 (wave-0e)

## Problem

`bootstrap_memory_index` built the vector index once and skipped whenever
`memory.rvf` / `memory.rvf.json` already existed. Edits to `MEMORY.md` were
invisible to semantic search until the index was deleted by hand.

## What shipped

### Core — `clawft-core::memory_bootstrap`

| Item | Detail |
|------|--------|
| mtime gate | If index exists and `MEMORY.md` mtime ≤ index mtime → skip (`Ok(0)`) |
| Stale rebuild | If `MEMORY.md` is **newer** than the index → remove index + sidecar, re-embed |
| `reindex_memory_index` | Public force-rebuild API (always reindexes when content present) |
| Helpers | `memory_is_newer_than_index`, `remove_memory_index`, `memory_index_sidecar_path` |

Real RVF backend also clears `path.sidecar.json` on rebuild (desync-safe).

### CLI — `weft memory reindex`

| Item | Detail |
|------|--------|
| Command | `weft memory reindex` (optional `--config`) |
| Paths | Resolves `MEMORY.md` via `MemoryStore`; index at sibling `memory.rvf` |
| Embedder | `ApiEmbedder::hash_only(384)` (offline, deterministic; same family as bootstrap tests) |

### Docs

- `docs/src/content/docs/clawft/memory.mdx` — vector index bootstrap table + CLI
- `docs/guides/rvf.md` §9 — mtime-aware bootstrap + `weft memory reindex`

## Acceptance

| Criterion | Status |
|-----------|--------|
| Check mtime in `bootstrap_memory_index` and re-index when MEMORY.md newer | Yes |
| `weft memory reindex` command | Yes |
| Reindex test: write MEMORY.md, reindex/bootstrap, search finds new content | Yes (`bootstrap_reindexes_when_memory_newer`, `force_reindex_rebuilds_current_index`) |
| Documented under existing memory docs | Yes (memory.mdx + rvf.md) |

## Tests

```bash
cargo test -p clawft-core --lib memory_bootstrap
# → 16 passed (includes WEFT-84 mtime + force reindex)

cargo test -p clawft-cli --bin weft memory
# → 12 passed (includes default_memory_index_path_beside_memory_md)

scripts/build.sh test clawft-core clawft-cli
# → memory_bootstrap WEFT-84 tests PASS
# → pre-existing FAIL: workspace::config::tests::load_merged_config_mcp_servers
#   (Json null MCPServerConfig — unrelated to WEFT-84; not touched)

scripts/build.sh check
# → ok (workspace compile)
```

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/memory_bootstrap.rs` | mtime + force reindex + tests |
| `crates/clawft-cli/src/commands/memory_cmd.rs` | `memory_reindex` + path helper |
| `crates/clawft-cli/src/main.rs` | `MemoryCmd::Reindex` + dispatch |
| `docs/src/content/docs/clawft/memory.mdx` | bootstrap / reindex section |
| `docs/guides/rvf.md` | current-state note for WEFT-84 |
| `docs/plans/wave-0e-WEFT-84-result.md` | this report |

## Commit

- **Branch:** `wave0e/weft-84-memory-reindex`
- **Message:** `fix(memory): WEFT-84 reindex memory.rvf when MEMORY.md changes`
- **SHA:** see `git rev-parse wave0e/weft-84-memory-reindex` on this branch

## Residual risks / notes

- Bootstrap is still only invoked from tests/callers of `bootstrap_memory_index`; the daemon does not auto-call it every turn. Operators use `weft memory reindex` or wire bootstrap at startup.
- Hash embedder is offline; production API embeddings would need a config path (out of scope).
- Pre-existing `load_merged_config_mcp_servers` failure remains on `release/0.8-staging` tree; not introduced by this ticket.
- Do not push (wave-0e policy); leave branch for lead merge.
