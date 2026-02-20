# SPARC Feature Element 08: Memory & Workspace

**Workstream**: H (Memory & Workspace)
**Timeline**: Weeks 4-8
**Status**: Planning
**Dependencies**: 04/C1 (MemoryBackend trait), 03/A2 (stable hash for vector memory)
**Blocks**: 10/K4 (ClawHub needs vector search from H2)

---

## 1. Summary

Per-agent workspace isolation, completion of RVF Phase 3 vector memory, and timestamp standardization.

---

## 2. Phases

### Phase H1: Per-Agent Workspace Isolation (Week 4-6)

| Deliverable | Description |
|-------------|-------------|
| Agent workspace dirs | `~/.clawft/agents/<agentId>/` with dedicated SOUL.md, AGENTS.md, USER.md |
| Session isolation | Per-agent session store |
| Skill overrides | Per-agent skill directory |
| Cross-agent access | Explicit opt-in for shared memory namespace (see Section 4) |

### Phase H2: RVF Phase 3 Vector Memory (Week 5-8)

| Item | Description | Current State |
|------|-------------|---------------|
| H2.1 | HNSW-backed VectorStore (using `instant-distance`, see Section 3) | Brute-force stub |
| H2.2 | Production embedder via `Embedder` trait (see Section 5) | HashEmbedder only |
| H2.3 | RVF file I/O (requires RVF 0.2 audit, see Section 7) | Plain JSON |
| H2.4 | `weft memory export/import` | Missing |
| H2.5 | POLICY_KERNEL storage | Not persisted |
| H2.6 | WITNESS segments (SHA-256 hash chain, see Section 8) | Not implemented |
| H2.7 | Temperature-based quantization (storage-layer only, see Section 6) | Not implemented |
| H2.8 | WASM micro-HNSW (separate module, 8KB budget, see Section 3) | Not integrated |

### Phase H3: Timestamp Standardization (Week 4-5)

| Deliverable | Description |
|-------------|-------------|
| Unified timestamps | `DateTime<Utc>` throughout (replacing i64 ms, Option<String>) |

---

## 3. HNSW Crate Selection: `instant-distance`

**Selected crate**: `instant-distance` (pure Rust, no unsafe, WASM-compatible).

**Rationale**: Covers both native and WASM targets from a single codebase. The `hnsw`/`hnswlib-rs` alternatives use C++ bindings and are not WASM-compatible.

**Incremental insert limitation**: `instant-distance` builds an immutable index. This is mitigated by periodic full re-index, which is acceptable for <100K vectors at the stated performance target of <10ms search.

**H2.8 WASM micro-HNSW**: Implemented as a **separate WASM module** (`micro-hnsw-wasm`) with its own **8KB size budget**. It is NOT bundled in the main `clawft-wasm` crate (which has a 300KB budget). The micro-HNSW module communicates with the main WASM agent via message passing, keeping vector search optional and the base agent WASM small.

---

## 4. Cross-Agent Shared Memory Protocol

Cross-agent memory sharing is **read-only by default** with config-driven opt-in.

**Sharing protocol**:
1. Agent A exports a namespace via config: `shared_namespaces = ["project-context"]`
2. Agent B imports it via config: `import_namespaces = [{ agent = "agent-a", namespace = "project-context" }]`
3. Implementation uses **symlink-based references** to Agent A's memory directory
4. Write access requires explicit `read_write = true` flag with filesystem-level locking

**Consistency model**: Read-your-writes for the owning agent; eventual consistency for importing agents (symlink target updates are atomic at the filesystem level).

---

## 5. Embedder Trait Specification

Define an `Embedder` trait in `clawft-core/src/embeddings/mod.rs`:

```rust
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Returns the dimensionality of the embedding vectors produced.
    fn dimensions(&self) -> usize;

    /// Embeds a batch of text strings into vectors.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;

    /// Returns the name/identifier of this embedder (e.g., "openai-text-embedding-3-small").
    fn name(&self) -> &str;
}
```

Implemented by:
- `HashEmbedder` -- for testing and offline use (deterministic, no API calls)
- `ApiEmbedder` -- for production (OpenAI `text-embedding-3-small` = 1536 dims, or local ONNX models)

The `MemoryBackend` plugin trait accepts an `Arc<dyn Embedder>` at construction time. Embedder selection is configured globally in `clawft.toml` with per-agent override support.

---

## 6. Temperature-Based Quantization

Uses a **single HNSW index with full-precision vectors** for search. Quantization applies only to the **storage layer**:

- **Hot vectors**: Stored as full-precision `Vec<f32>` in memory and on disk
- **Warm vectors**: Stored as fp16 on disk, decompressed to f32 on access
- **Cold vectors**: Stored as product-quantized (PQ) on disk, decompressed on access

The HNSW index itself always uses full-precision pointers. No index rebuild is required when vectors move between temperature tiers. Tier transitions are driven by access frequency and recency.

---

## 7. RVF 0.2 Audit

**Week 4 pre-work**: Audit `rvf-runtime` 0.2 API for segment I/O capabilities. Specifically verify:
- Segment read/write operations exist in the public API
- Segment types support the fields needed for H2.3 (vector data, metadata, timestamps)
- WITNESS metadata can be attached to segments

If `rvf-runtime` 0.2 only provides stub segment types and lacks I/O, plan a local implementation that serializes to the RVF format directly using `rvf-types`. This must be determined before H2.3 implementation begins.

---

## 8. WITNESS Segment Structure

WITNESS segments provide a tamper-evident audit trail using **SHA-256 hash chaining**.

**Structure**:
- Each WITNESS segment contains: `segment_id`, `timestamp`, `operation` (store/update/delete), `data_hash` (SHA-256 of the stored data), `previous_hash` (SHA-256 of the previous WITNESS segment)
- The chain starts from a root segment with `previous_hash = [0u8; 32]`
- Verification is **sequential scan from root**: recompute each segment's hash and verify it matches the `previous_hash` of the next segment

**Dependency**: The `sha2` crate is already a workspace dependency.

**Integration**: All memory write paths (H2.1-H2.5) optionally include WITNESS metadata. The `weft memory export` command includes the full WITNESS chain. `weft memory import` validates the chain before importing.

---

## 9. Async Embedding Pipeline

Embedding computation is **asynchronous** to avoid blocking the agent loop:

1. `store()` writes raw data immediately (available for keyword search)
2. `store()` spawns a background embedding task via `tokio::spawn`
3. The background task calls `Embedder.embed()` and inserts the result into the HNSW index
4. A `pending_embeddings` queue tracks items awaiting embedding
5. The vector index is **eventually consistent**: a newly stored item may not appear in vector search results until its embedding completes

**Fallback behavior**: If the embedding API is unavailable, items remain in the `pending_embeddings` queue with exponential backoff retry (max 3 attempts). After exhausting retries, the item is logged as a warning and remains keyword-searchable only.

---

## 10. Exit Criteria

### Core Exit Criteria

- [ ] Each agent has isolated workspace under `~/.clawft/agents/<id>/`
- [ ] HNSW-backed vector search returns relevant results
- [ ] Production embedder produces real embeddings (not hash-based)
- [ ] `weft memory export` and `weft memory import` work
- [ ] All timestamps use `DateTime<Utc>`
- [ ] All existing tests pass

### Embedder Trait Exit Criteria

- [ ] `Embedder` trait defined in `clawft-core/src/embeddings/mod.rs`
- [ ] `HashEmbedder` and `ApiEmbedder` both implement `Embedder` trait
- [ ] `MemoryBackend` accepts `Arc<dyn Embedder>` at construction

### HNSW Exit Criteria

- [ ] `instant-distance` integrated as the HNSW backend for H2.1
- [ ] H2.8 WASM micro-HNSW is a separate module with <8KB compiled size
- [ ] Periodic re-index works correctly for incremental updates

### Cross-Agent Memory Exit Criteria

- [ ] Shared namespaces configurable via `shared_namespaces` and `import_namespaces`
- [ ] Symlink-based references work for read-only cross-agent access
- [ ] Write access requires explicit `read_write = true` flag

### WITNESS Exit Criteria

- [ ] WITNESS segments use SHA-256 hash chaining
- [ ] Sequential verification from root detects tampering
- [ ] `weft memory export` includes WITNESS chain; `weft memory import` validates it

### Async Embedding Exit Criteria

- [ ] `store()` returns immediately; embedding runs in background
- [ ] `pending_embeddings` queue tracks in-flight items
- [ ] Fallback to keyword-only search when embedding API unavailable

### RVF 0.2 Exit Criteria

- [ ] RVF 0.2 API audit completed in Week 4
- [ ] Segment I/O path confirmed (either via `rvf-runtime` or local implementation)

---

## 11. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| `instant-distance` crate maturity (smaller project, limited maintainers) | Medium | Medium | **4** | Pure Rust with no unsafe; verify correctness via property-based tests; vendoring as fallback |
| Embedding API availability/latency affects store() path | Medium | Medium | **4** | Async pipeline with pending_embeddings queue; fallback to keyword-only search; retry with backoff |
| Cross-agent shared memory consistency under concurrent writes | Low | High | **4** | Read-only by default; write access requires explicit opt-in with fs-level locking; eventual consistency model |
| RVF 0.2 lacks segment I/O, forcing local implementation | Medium | Medium | **4** | Week 4 audit determines path; local implementation using `rvf-types` as fallback |
| HNSW index memory consumption (100K vectors x 1536 dims ~ 600MB) | Medium | High | **6** | Support dimensionality reduction (256-dim via Matryoshka); `max_vectors` config with cold eviction |
| Periodic re-index latency spike under load | Low | Medium | **3** | Background re-index with read-continue semantics; schedule during low-activity windows |
