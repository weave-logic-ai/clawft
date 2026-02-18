# Stream 3F: RVF Integration -- Sprint 1 Foundation

## Date: 2026-02-17

## Summary

Sprint 1 establishes the RVF (RuVector Format) foundation for clawft-core,
adding API-based embeddings, an RVF-compatible stub vector store, and
memory bootstrap functionality. All new code lives behind the `rvf` feature
flag and compiles cleanly with and without it.

## Task Completion Status

| Task | Status | Notes |
|------|--------|-------|
| 1. Check RVF crate availability | Done | rvf-runtime 0.2.0 and rvf-types 0.2.0 are published |
| 2. Create RVF feature flags | Done | `rvf` feature in clawft-core, depends on vector-memory |
| 3. Create ApiEmbedder | Done | `embeddings/api_embedder.rs` |
| 4. Memory bootstrap | Done | `memory_bootstrap.rs` |
| 5. Stub VectorStore (RVF interface) | Done | `embeddings/rvf_stub.rs` |
| 6. Tests | Done | 43 new tests, all passing |
| 7. Verify | Done | cargo test + clippy clean |
| 8. Dev notes | Done | This file |

## Architecture Decisions

### RVF Crates Available but Stub Used Initially

Although `rvf-runtime` 0.2.0 and `rvf-types` 0.2.0 are available on
crates.io, the stub store was implemented first because:

1. The RVF crate APIs need evaluation for clawft's specific use case
2. The stub provides the same interface with simpler JSON persistence
3. The stub enables full testing without external dependencies
4. Migration to real RVF is straightforward (same method signatures)

The `rvf-runtime` and `rvf-types` crates are declared as optional deps
behind the `rvf` feature flag, ready for integration in Sprint 2.

### ApiEmbedder with Hash Fallback

The `ApiEmbedder` calls OpenAI-compatible `/embeddings` endpoints when an
API key is available. When no key is set (or the call fails), it falls back
to SHA-256-based pseudo-embeddings. This design:

- Works in CI/testing environments without API keys
- Degrades gracefully in air-gapped environments
- Produces deterministic, reproducible results for testing
- Can be swapped to real embeddings with zero code changes

### Memory Bootstrap

The `bootstrap_memory_index` function implements a one-shot indexing pass
over `MEMORY.md`. It:

- Skips if the index file already exists (idempotent)
- Splits by `## ` headers first, falls back to paragraph splitting
- Embeds each section and stores in an RvfStore
- Persists via JSON compaction

## Files Created

- `crates/clawft-core/src/embeddings/api_embedder.rs` (233 lines)
- `crates/clawft-core/src/embeddings/rvf_stub.rs` (302 lines)
- `crates/clawft-core/src/memory_bootstrap.rs` (273 lines)

## Files Modified

- `Cargo.toml` (workspace) -- added rvf-runtime, rvf-types workspace deps
- `crates/clawft-core/Cargo.toml` -- added rvf feature flag with deps
- `crates/clawft-core/src/embeddings/mod.rs` -- added api_embedder, rvf_stub modules
- `crates/clawft-core/src/lib.rs` -- added memory_bootstrap module

## Test Coverage

- **api_embedder**: 13 tests (hash fallback, determinism, dimensions, batch, trait impl)
- **rvf_stub**: 17 tests (CRUD, query ordering, persistence roundtrip, cosine similarity)
- **memory_bootstrap**: 13 tests (header/paragraph splitting, bootstrap lifecycle, searchability)
- **Total new tests**: 43
- All pre-existing tests continue to pass (499 without rvf, 602 with rvf)

## Next Steps (Sprint 2)

1. Integrate real `rvf-runtime::RvfStore` behind a compile-time switch
2. Wire `ApiEmbedder` into `AppContext::new()` for automatic bootstrap
3. Add semantic search to `MemoryStore` using the vector index
4. Implement incremental index updates (append to index on memory writes)
5. Add HNSW parameters tuning for production workloads
