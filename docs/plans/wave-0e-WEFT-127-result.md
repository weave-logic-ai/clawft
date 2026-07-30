# WEFT-127 result — Persist HNSW tombstones across save/load

**Branch:** `wave0e/weft-127-hnsw-tombstones`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef4-7621-9495-d5e8e79eed25`  
**Date:** 2026-07-30

## Summary

`HnswBackend` soft-delete tombstones were process-local. A save/load cycle
through the HNSW JSON path restored soft-deleted vectors. WEFT-127 extends
the on-disk format so tombstone state (and id map / epoch / capacity)
survives restart, with empty defaults for legacy snapshots.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Save/load format persists tombstone state | Done — `HnswBackendSnapshot` includes `tombstones: Vec<(u64, Tombstone)>` |
| Backward-compatible read for existing snapshots | Done — missing `tombstones` / `id_map` / `epoch` / `max_vectors` → empty / 0 / None |
| Test: delete → save → load → deleted stay gone | Done — `soft_delete_save_load_keeps_tombstones` |
| Compaction story documented | Done — module docs on `vector_hnsw.rs` + this note + `vector-hardening.md` |

## Compaction story

1. **`soft_delete(id)`** — marks id tombstoned at the current epoch; vector
   remains in the inner `HnswService` so mesh/sync peers can observe the
   soft-delete; search / contains / len exclude it.
2. **`compact(older_than_epoch)`** — for each tombstone with
   `deleted_at_epoch < older_than_epoch`, drop the id-map entry, remove the
   tombstone, and **hard-delete** the string key from the inner store.
3. **Durability** — call `HnswBackend::save_to_file` after soft-deletes (to
   keep them tombstoned after restart) and after compact (to persist the
   purged store). `flush()` remains a no-op; this path is explicit save.

## On-disk format

JSON superset of core `HnswStore` snapshot:

```json
{
  "entries": [ { "id": "a", "embedding": [1.0], "metadata": {} } ],
  "ef_search": 100,
  "ef_construction": 200,
  "id_map": [ [1, "a"] ],
  "tombstones": [ [1, { "deleted_at_epoch": 3 }] ],
  "epoch": 3,
  "max_vectors": null
}
```

- `HnswStore::load` still accepts these files (unknown/extra keys ignored;
  it only needs entries + ef params).
- Plain legacy store files load into `HnswBackend` with empty tombstones.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/src/embeddings/hnsw_store.rs` | `entries()`, `ef_construction()`, `from_entries()`; load docs |
| `crates/clawft-kernel/src/hnsw_service.rs` | `from_store`, `snapshot_entries`, `ef_params`, `delete`; load uses store params |
| `crates/clawft-kernel/src/vector_hnsw.rs` | save/load, compaction hard-delete, module docs, WEFT-127 tests |
| `.planning/development_notes/sprint-16/vector-hardening.md` | Tombstones note updated |
| `docs/plans/wave-0e-WEFT-127-result.md` | This report |

## Tests

```bash
cargo test -p clawft-kernel --lib vector_hnsw
# → 26 passed (incl. soft_delete_save_load_keeps_tombstones,
#   load_legacy_store_snapshot_has_empty_tombstones,
#   compact_physically_removes_and_survives_save)

cargo test -p clawft-kernel --lib hnsw_service
# → 24 passed

cargo test -p clawft-core --lib embeddings::hnsw_store
# → 17 passed

cargo test -p clawft-kernel --lib vector_hybrid
# → 19 passed
```

## Residual

- Profile store still creates empty `HnswBackend` on load and does not yet
  call `HnswBackend::save_to_file` for per-profile indexes (out of scope).
- Mesh vector sync (WS4 / Gap #11) can consume the same tombstone section
  once peers exchange backend snapshots.
