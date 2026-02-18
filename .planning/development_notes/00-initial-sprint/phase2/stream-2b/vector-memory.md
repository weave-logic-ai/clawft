# Stream 2B: Vector Memory System -- Development Notes

**Agent**: vector-engineer (coder)
**Date**: 2026-02-17
**Phase**: 2 / Stream 2B
**Crate**: `clawft-core`
**Modules**: `embeddings/`, `vector_store`, `intelligent_router`, `session_indexer`
**Feature flag**: `vector-memory`

## Summary

Implemented the vector memory subsystem in `clawft-core` behind the `vector-memory` feature flag. This includes the `Embedder` trait, a SimHash-based `HashEmbedder`, a brute-force `VectorStore` with cosine similarity, the ADR-026 three-tier `IntelligentRouter`, and a `SessionIndexer` for cross-session semantic search. All modules compile both with and without the feature flag.

## Files Written

| File | Lines | Purpose |
|------|-------|---------|
| `embeddings/mod.rs` | 81 | `Embedder` trait, `EmbeddingError`, `embed_batch` default impl |
| `embeddings/hash_embedder.rs` | 241 | `HashEmbedder` -- SimHash, dimension 384, deterministic |
| `vector_store.rs` | 436 | `VectorStore` -- brute-force cosine similarity search |
| `intelligent_router.rs` | 550 | `IntelligentRouter` -- ADR-026 3-tier routing with cost tracking |
| `session_indexer.rs` | 434 | `SessionIndexer` -- conversation turn indexing, cross-session search |

**Total new lines**: ~1,742

## Architecture Decisions

### Simple in-memory implementations, not RVF crates
The SPARC plan referenced external RVF crates (`rvf-runtime`, `rvf-types`, etc.)
that do not yet exist. Rather than block on those, all vector operations use
simple in-memory data structures:
- `VectorStore` uses brute-force cosine similarity (O(n) scan)
- `HashEmbedder` uses SimHash for local deterministic embeddings
These preserve the trait-based API surface so that future RVF/HNSW
implementations can be swapped in without changing callers.

### SimHash embedder (dimension 384)
`HashEmbedder` uses a SimHash algorithm: each word's hash contributes +1/-1
to each dimension, and the final vector is normalized. Dimension 384 was
chosen as a reasonable balance between discriminative power and memory use.
The embedder is fully deterministic (no randomness), requires no API calls,
and runs in O(n) where n is the number of words.

### Feature flag gating
All 4 modules are behind `#[cfg(feature = "vector-memory")]` in `lib.rs`.
The `vector-memory` feature enables the optional `rand` dependency (used
only in `session_indexer` tests for generating random IDs). The crate
compiles cleanly with `--features vector-memory` and without.

### ADR-026 three-tier routing
The `IntelligentRouter` implements the 3-tier model from ADR-026:
- **Tier 1** (complexity < 0.15): Agent Booster / WASM transforms, skip LLM
- **Tier 2** (complexity 0.15-0.30): Haiku, ~500ms latency
- **Tier 3** (complexity > 0.30): Sonnet/Opus, 2-5s latency

`compute_complexity()` uses a keyword-density heuristic: count of domain-specific
keywords (architecture, security, implement, refactor, etc.) divided by word count.
The router also tracks cumulative cost per tier for observability.

### Policy caching
`IntelligentRouter` caches the most recent routing policy to avoid recomputing
identical routes. The cache is invalidated on any configuration change.

### VectorStore cosine similarity
Cosine similarity is computed as `dot(a, b) / (||a|| * ||b||)`. The store
supports `insert`, `search` (top-k), and `remove` operations. All vectors
are stored in a `Vec<(String, Vec<f32>)>` -- adequate for thousands of entries.
For production scale (millions), swap to HNSW via a future RVF backend.

### SessionIndexer
Indexes `ConversationTurn` structs (session_id, role, content, timestamp) and
supports cross-session semantic search. Each turn is embedded and stored in
the VectorStore with a composite key. Search results include the session_id
and original content for context reconstruction.

## Dependencies Added

- `rand` (workspace, optional): Only enabled by `vector-memory` feature, used in tests

## Test Coverage (63 tests)

- **embeddings/mod.rs** (3 tests): Error display, error trait impl
- **embeddings/hash_embedder.rs** (11 tests): Dimension, determinism, normalization, empty input, batch, Unicode, long text
- **vector_store.rs** (16 tests): Insert, search, remove, cosine similarity, empty store, top-k, duplicate keys, dimension mismatch
- **intelligent_router.rs** (19 tests): All 3 tiers, complexity computation, cost tracking, policy caching, config changes, edge cases
- **session_indexer.rs** (10 tests): Index turn, search, cross-session, empty index, multiple turns, role filtering, timestamp ordering
- **lib.rs** (4 tests): Module availability with/without feature flag

**Without feature flag**: clawft-core compiles with 200 tests (all pre-existing)
**With feature flag**: clawft-core compiles with 263 tests (200 + 63 new)

## Quality Gates

| Check | Result |
|-------|--------|
| `cargo build -p clawft-core` | PASS |
| `cargo build -p clawft-core --features vector-memory` | PASS |
| `cargo test -p clawft-core` | PASS (200 tests) |
| `cargo test -p clawft-core --features vector-memory` | PASS (263 tests) |
| `cargo clippy -p clawft-core -- -D warnings` | PASS (0 warnings) |

## Integration Points

- **clawft-core::pipeline::traits**: `IntelligentRouter` can replace `StaticRouter` in the pipeline for complexity-aware model selection
- **clawft-core::agent::memory**: `SessionIndexer` can augment `MemoryStore` with semantic retrieval
- **clawft-core::bootstrap**: `AppContext` can optionally wire vector-memory components when feature is enabled
- **Future RVF crates**: The `Embedder` and `VectorStore` trait surfaces are designed for drop-in replacement

## Next Steps

1. Implement HNSW-backed `VectorStore` when RVF crates are available
2. Add API-based embedder (OpenAI, Cohere) as alternative to `HashEmbedder`
3. Wire `IntelligentRouter` into `PipelineRegistry` as a replacement for `StaticRouter`
4. Add persistence layer for `VectorStore` (mmap or SQLite backing)
5. Benchmark SimHash quality against real embedding models
