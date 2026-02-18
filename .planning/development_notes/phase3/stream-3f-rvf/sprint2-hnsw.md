# Sprint 2: Progressive HNSW Search Stub

## Summary

Implemented `ProgressiveSearch` in `crates/clawft-core/src/embeddings/progressive.rs`,
gated behind the `rvf` feature flag. This provides the interface for a three-tier
progressive search strategy:

- **Layer A** -- Coarse routing (~70% recall, microseconds)
- **Layer B** -- Hot-region refinement (~85% recall)
- **Layer C** -- Full graph traversal (~95% recall)

## Current Implementation

The current implementation is a **brute-force cosine similarity** stub. This is
intentional -- the public API is stable and will remain unchanged when the real
HNSW implementation from `rvf-index` is swapped in.

## Public API

```rust
pub struct ProgressiveSearch { ... }

impl ProgressiveSearch {
    pub fn new() -> Self;
    pub fn insert(&mut self, id: &str, embedding: Vec<f32>, metadata: serde_json::Value);
    pub fn search(&self, query: &[f32], k: usize) -> Vec<SearchResult>;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn save(&self, path: &Path) -> io::Result<()>;
    pub fn load(path: &Path) -> io::Result<Self>;
}

pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}
```

## Persistence

Save/load uses JSON serialization via `serde`. When HNSW is added, the
serialization format will change but the method signatures remain the same.

## Tests (14 total)

- Insert + search returns correct top-k ordering
- Cosine similarity: identical, orthogonal, opposite, zero, mismatched lengths
- Save/load round-trip preserves data and search results
- Empty index, k=0, k > store size edge cases
- Metadata preservation
- Default trait

## Files Changed

- `crates/clawft-core/Cargo.toml` -- Added `rvf` feature (implies `vector-memory`)
- `crates/clawft-core/src/embeddings/mod.rs` -- Added `progressive` module under `rvf` gate
- `crates/clawft-core/src/embeddings/progressive.rs` -- New file

## Future Work

- Replace brute-force search with HNSW from `rvf-index` crate
- Add tier-specific search methods (layer_a, layer_b, layer_c)
- Add incremental index updates without full rebuild
