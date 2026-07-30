# WEFT-660 result — DiskAnnBackend search numeric ids

**Branch:** `wave0a/weft-660-diskann-ids`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb418-df93-7cc3-bbbb-a512885bb6a1`  
**Date:** 2026-07-30

## Summary

Real `DiskAnnBackend::search` (feature `diskann`) no longer hardcodes `SearchResult.id = 0`. A reverse map `key → u64` is maintained beside `id_map` on insert/remove; search resolves DiskANN string keys to numeric ids and skips tombstones. Stub path unchanged for correctness (still returns real ids).

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/vector_diskann.rs` | Reverse map `key_to_id`; search/insert/remove/len/soft_delete/compact updates; feature-gated + stub distinct-id tests |
| `docs/plans/wave-0a-WEFT-660-result.md` | This report |

## Fix detail

1. **`key_to_id: Mutex<HashMap<String, u64>>`** on the real backend, kept in sync with `id_map` on insert (handles key/id rebinding) and remove.
2. **`search`**: `filter_map` over DiskANN hits → `key_to_id.get(&r.id)` → skip if missing or tombstoned → `SearchResult::new(numeric_id, r.id, r.distance, Null)`.
3. **Tombstones**: real path now implements `soft_delete` / `compact` (parity with stub); search excludes tombstoned ids.
4. **Capacity / len**: real insert checks capacity against live map size; `len()` uses `id_map` − tombstones (so soft-deleted rows still in the index don't inflate live count incorrectly).
5. **Metadata**: still `Null` on real path (DiskANN index does not store caller metadata) — documented in search comment.
6. **Metric note**: real DiskANN distances remain L2² (WEFT-661).

## Tests

```bash
# Default features (stub)
cargo test -p clawft-kernel --lib vector_diskann
# → 19 passed (includes stub_search_returns_distinct_numeric_ids)

# Real DiskANN
cargo test -p clawft-kernel --lib vector_diskann --features diskann
# → 17 passed (includes search_returns_distinct_numeric_ids)

scripts/build.sh check
# → ok

cargo check -p clawft-kernel --features diskann --tests
# → ok
```

Note: `scripts/build.sh test` does not currently forward `--features` into `workspace_test`; diskann coverage used `cargo test -p … --features diskann` for the feature matrix. Full `scripts/build.sh test clawft-kernel` was started (2000+ tests largely green) but was cut short on long-running `hnsw_eml` benchmarks unrelated to this change.

## Commit

- **Branch:** `wave0a/weft-660-diskann-ids`
- **Message:** `fix(vector): WEFT-660 resolve DiskANN SearchResult numeric ids`
- **SHA:** run `git rev-parse wave0a/weft-660-diskann-ids` (this result file ships in the same commit).

## Residual risks

- **Hybrid merge still broken by metric scale** (WEFT-661): cold L2² vs hot cosine; correct ids unlock dedup but do not fix ranking.
- **Real DiskANN requires `flush()`/`build()`** before search returns hits; callers that only insert will still see empty results (`Err(_) => Vec::new()`).
- **Upsert key change**: old key is dropped from reverse map; DiskANN index may still hold the old string key until delete — insert uses new key string; orphan keys in the index could still surface in search and are skipped (unknown key → filtered).
- **`scripts/build.sh test --features diskann`** does not pass features to cargo/nextest today — CI/gate diskann path relies on the explicit `cargo check -p clawft-kernel --features diskann --tests` step in gate.
- **No bench re-run** in this ticket; WEFT-366 harness not re-measured (out of minimal scope).
