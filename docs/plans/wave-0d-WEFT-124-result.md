# WEFT-124 result — wire VectorBackend into DemocritusLoop

**Branch:** `wave0d/weft-124-vector-backend`  
**Date:** 2026-07-30  
**Status:** Shipped (code + democritus/vector unit tests green)  
**Worktree:** this agent worktree (base `release/0.8-staging`)

## Ticket

ws02: vector — wire VectorBackend into DemocritusLoop.

DEMOCRITUS still consumed a raw `HnswService`. The `VectorBackend` trait +
Hybrid backend existed but was unused on the cognitive path. Switching
unlocks DiskANN / Hybrid swap-in.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| DemocritusLoop consumes `Arc<dyn VectorBackend>` | **Done** |
| Default binding is the existing HNSW backend (no behavior change) | **Done** — tests use `HnswBackend` |
| Tests confirm backend swap is wiring-only | **Done** — `backend_swap_is_wiring_only` + `default_binding_is_hnsw_backend` |

## What shipped

### `crates/clawft-kernel/src/democritus.rs`

1. **Field change:** `hnsw: Arc<HnswService>` → `vector: Arc<dyn VectorBackend>`.
2. **`DemocritusLoop::new`** takes `Arc<dyn VectorBackend>` (typically
   `Arc::new(HnswBackend::new(...))` for the production default).
3. **`vector_backend()`** accessor for diagnostics / tests.
4. **SEARCH:** calls `VectorBackend::search`. Converts backend distance to
   cosine similarity (`score = 1.0 - distance`) so `correlation_threshold`
   keeps its pre-existing similarity semantics (matches `HnswBackend`'s
   distance encoding).
5. **UPDATE insert:** `vector.insert(node_id, &node_id.to_string(), emb, meta)`
   — numeric id + string key are both the causal node id so neighbor →
   `link` parsing still works. Insert errors are logged, not panics.
6. **Tests** wire `HnswBackend` as the default; added a `CountingBackend`
   mock to prove a non-HNSW implementor runs the full loop.

### Tests (28 democritus pass; was 26)

| Test | Asserts |
|------|---------|
| `default_binding_is_hnsw_backend` | make_loop / accessor report `"hnsw"` |
| `backend_swap_is_wiring_only` | CountingBackend receives insert+search; edges form |
| Existing suite | Adapted to `Arc<dyn VectorBackend>` / `.len()` / `.search()` |

## Out of scope (intentionally)

- Wiring `DemocritusLoop` into boot / `run_democritus_loop` (still uses
  raw `HnswService` for the separate two-tier path in `cognitive_tick`).
- Adding `search_batch` to the `VectorBackend` trait (sequential
  `search` is sufficient; batch remains an HnswService-only optim).
- DiskANN / Hybrid production selection (already in boot; this unlocks
  democritus to accept those backends when a caller wires them).

## Verification

```bash
scripts/build.sh check
cargo test -p clawft-kernel democritus
# → 28 passed
cargo test -p clawft-kernel vector_
# → 94 passed (vector_* + related filters)
```

## Follow-ups

- Point boot / cognitive path at the same `Arc<dyn VectorBackend>` that
  democritus uses when the loop is activated in production.
- Optional: default `search_batch` on `VectorBackend` for backends that
  can amortize locking (HnswBackend can forward to `HnswService::search_batch`).
