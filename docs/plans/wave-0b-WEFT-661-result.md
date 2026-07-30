# WEFT-661 result — HybridBackend RRF merge

**Branch:** `wave0b/weft-661-hybrid-merge`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb432-7bcf-7812-bb58-6dd008068455`  
**Date:** 2026-07-30  
**Policy:** Reciprocal Rank Fusion (RRF), `k = 60`

## Summary

`HybridBackend::merge_results` no longer sorts hot (cosine) and cold (L2²
under `--features diskann`) hits by raw distance. Ranking uses Reciprocal
Rank Fusion so metric scale cannot let cosine hits dominate. Dedup remains
by numeric `SearchResult::id` (unlocked by WEFT-660).

## Why RRF (not normalize / unify metric)

| Candidate | Why not / why |
|-----------|----------------|
| **Unify metric** | Real DiskANN path is L2²; changing HNSW or DiskANN metrics is a larger API/perf change than this ticket. |
| **Normalize scales** | Needs known ranges or per-list min/max; brittle under short lists and empty tiers. |
| **RRF (chosen)** | Metric-agnostic; standard multi-list fusion (Cormack et al.); works with empty lists and partial overlap; minimal surface change. |

Formula (0-based ranks, documented on `merge_results`):

```text
score(id) = Σ_lists  1 / (RRF_K + rank_list(id))
```

Higher score wins. Tie-break: native distance ascending, then id.

## Dedup / payload policy

- Key: numeric `id` (WEFT-660 fixed cold ids ≠ 0).
- Id in both lists: RRF mass from both ranks; **hot** payload retained
  (key/metadata); native `distance` kept (not rewritten to RRF).
- Callers must not assume a single metric on `distance` after merge.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/vector_hybrid.rs` | RRF merge + unit tests |
| `crates/clawft-kernel/benches/vector_backend_bench.rs` | Stale merge note → RRF |
| `docs/plans/wave-0b-WEFT-661-result.md` | This report |

## Tests

```bash
cargo test -p clawft-kernel --lib vector_hybrid
# → 19 passed (includes merge_results_rrf_not_raw_distance,
#   merge_results_dedup_prefers_hot_payload, cold-only + empty cases)

scripts/build.sh check
# → ok
```

New regression coverage:

- **`merge_results_rrf_not_raw_distance`**: tiny cosine + large L2² → top-2
  includes rank-0 from **both** tiers (raw sort would return only hot).
- **`merge_results_dedup_prefers_hot_payload`**: same id both tiers → one hit,
  hot key/metadata, distance preserved.
- **`merge_results_cold_only_ids_survive`**: cold-only ids not dropped.
- **`merge_results_empty_and_k_zero`**: empty inputs / k=0.

## Commit

- **Branch:** `wave0b/weft-661-hybrid-merge`
- **Message:** `fix(vector): WEFT-661 RRF merge for hybrid hot/cold ranks`
- **SHA:** this commit (branch tip: `git rev-parse wave0b/weft-661-hybrid-merge`)

## Residual risks

- **No full DiskANN recall re-bench** in this ticket (WEFT-366 harness not
  re-run with `--features diskann`). Expect hybrid recall to leave the
  ~0.113 floor; exact number still needs a bench pass.
- **Hybrid recall still scored vs cosine GT** in the bench — interpret
  carefully when cold is L2².
- **RRF does not re-score true geometric proximity** across metrics; it
  only fuses ranks. A future unify-metric path could improve absolute
  quality further.
- **Promotion / access counts** still driven by the fused top-k ids only.
