# Element 08: Memory & Workspace -- Execution Tracker

## Summary

- **Total items**: 11 (H1, H2.1-H2.8, H3)
- **Workstream**: H (Memory & Workspace)
- **Timeline**: Weeks 4-8
- **Status**: Planning -> Development
- **Dependencies**: 04/C1 (MemoryBackend plugin trait), 03/A2 (stable hash for HashEmbedder)
- **Blocks**: 10/K4 (ClawHub needs vector search from H2), 09/L2 (routing calls ensure_agent_workspace)

---

## Execution Schedule

Element 08 has 11 deliverables across 3 phases spanning Weeks 4-8.

### Week 4-6 (H1 + H3 -- independent, parallel)

- [ ] H1 -- Per-agent workspace isolation: `~/.clawft/agents/<agentId>/` with SOUL.md, AGENTS.md, USER.md, session store, skill overrides, cross-agent opt-in
- [ ] H3 -- Timestamp standardization: `DateTime<Utc>` throughout (replacing i64 ms, `Option<String>`)

### Week 5-7 (H2 Core -- foundational vector memory)

- [ ] H2.1 -- HNSW-backed VectorStore using `instant-distance` (replaces brute-force cosine scan)
- [ ] H2.2 -- Production Embedder trait + `HashEmbedder` / `ApiEmbedder` implementations (depends on A2 stable hash)

### Week 6-7 (H2 MVP -- persistence and export)

- [ ] H2.3 -- RVF segment I/O for vector data (depends on RVF 0.2 audit, H2.1)
- [ ] H2.4 -- `weft memory export/import` CLI commands
- [ ] H2.5 -- POLICY_KERNEL persistence (routing policy store)

### Week 7-8 (H2 Advanced -- post-MVP)

- [ ] H2.6 -- WITNESS hash chains (SHA-256 tamper-evident audit trail, depends on H2.3)
- [ ] H2.7 -- Temperature-based quantization (storage-layer only: hot=f32, warm=fp16, cold=PQ)
- [ ] H2.8 -- WASM micro-HNSW (separate `micro-hnsw-wasm` crate, 8KB budget)

---

## Per-Item Status Table

| Item | Description | Priority | Week | Crate(s) / Location | Status | Owner | Branch | Key Deliverable |
|------|-------------|----------|------|----------------------|--------|-------|--------|-----------------|
| H1 | Per-agent workspace isolation | P0 | 4-6 | clawft-core/workspace.rs | Pending | -- | -- | `~/.clawft/agents/<id>/` with SOUL.md, session store, skill overrides |
| H2.1 | HNSW-backed VectorStore (instant-distance) | P0 | 5-7 | clawft-core/src/embeddings/hnsw_store.rs (new) | Pending | -- | -- | HNSW search returns relevant results in <10ms for 100K vectors |
| H2.2 | Production Embedder trait | P0 | 5-7 | clawft-core/src/embeddings/mod.rs | Pending | -- | -- | `Embedder` trait with `HashEmbedder` + `ApiEmbedder` implementations |
| H2.3 | RVF segment I/O | P1 | 6-7 | clawft-core/src/embeddings/rvf_stub.rs (replace) | Pending | -- | -- | Vector data persisted via RVF segments (or local fallback) |
| H2.4 | `weft memory export/import` CLI | P1 | 6-7 | clawft-cli/src/commands/ (new subcommand) | Pending | -- | -- | Export memory to file, import with WITNESS chain validation |
| H2.5 | POLICY_KERNEL persistence | P1 | 6-7 | clawft-core (routing policy store) | Pending | -- | -- | Routing policy survives agent restart |
| H2.6 | WITNESS segments (SHA-256 chain) | P2 | 7-8 | clawft-core/src/embeddings/witness.rs (new) | Pending | -- | -- | Tamper-evident audit trail; sequential verification from root |
| H2.7 | Temperature quantization | P2 | 7-8 | clawft-core/src/embeddings/quantization.rs (new) | Pending | -- | -- | Cold vectors stored as PQ, warm as fp16; decompressed on access |
| H2.8 | WASM micro-HNSW (8KB budget) | P2 | 7-8 | micro-hnsw-wasm (new crate) | Pending | -- | -- | Separate WASM module <8KB; communicates via message passing |
| H3 | Timestamp standardization (DateTime<Utc>) | P1 | 4-5 | clawft-types/workspace.rs + all crates | Pending | -- | -- | Zero i64 ms or Option<String> timestamps remaining |

---

## Dependency Graph

### External Dependencies (must land before Element 08 work begins)

| Prerequisite | Source Element | Description | Blocks |
|-------------|---------------|-------------|--------|
| C1 Plugin Traits | 04 | `MemoryBackend` plugin trait in `clawft-plugin` crate | H2 (all vector memory items need plugin trait) |
| A2 Stable Hash | 03 | Deterministic `compute_embedding()` across platforms | H2.1, H2.2 (HashEmbedder must produce stable results) |

### Internal Dependencies (within Element 08)

```
H1 (Workspace Isolation) -- independent, no internal deps
H3 (Timestamps) -- independent, no internal deps

H2.1 (HNSW VectorStore)
  └──> H2.3 (RVF Segment I/O needs HNSW for vector segment read/write)

RVF 0.2 audit (Week 4 pre-work)
  └──> H2.3 (determines implementation path: rvf-runtime vs local)

H2.3 (RVF Segment I/O)
  └──> H2.6 (WITNESS integrates with RVF I/O for hash chains)
```

### Cross-Element Dependencies (Element 08 unblocks other elements)

| Source (Element 08) | Target (Other Element) | Type | Impact |
|---------------------|------------------------|------|--------|
| H2 (vector search) | K4 (Element 10, ClawHub) | **CRITICAL** | ClawHub needs vector search for semantic skill/plugin discovery |
| H1 (workspace) | L2 (Element 09, routing) | Blocks | Agent routing calls `ensure_agent_workspace()` on first message |

### Cross-Element Integration Tests

| Test | Elements | Week | Priority |
|------|----------|------|----------|
| Workspace creation via routing | 08, 09 | 6 | P0 |
| Vector search for ClawHub discovery | 08, 10 | 8 | P1 |

---

## Exit Criteria

### Core Exit Criteria

- [ ] Each agent has isolated workspace under `~/.clawft/agents/<id>/` (H1)
- [ ] HNSW-backed vector search returns relevant results (H2.1)
- [ ] Production embedder produces real embeddings (not hash-based) (H2.2)
- [ ] `weft memory export` and `weft memory import` work (H2.4)
- [ ] All timestamps use `DateTime<Utc>` (H3)
- [ ] All existing tests pass (regression gate)

### Embedder Trait Exit Criteria

- [ ] `Embedder` trait defined in `clawft-core/src/embeddings/mod.rs` (H2.2)
- [ ] `HashEmbedder` and `ApiEmbedder` both implement `Embedder` trait (H2.2)
- [ ] `MemoryBackend` accepts `Arc<dyn Embedder>` at construction (H2.2)

### HNSW Exit Criteria

- [ ] `instant-distance` integrated as the HNSW backend for H2.1
- [ ] H2.8 WASM micro-HNSW is a separate module with <8KB compiled size
- [ ] Periodic re-index works correctly for incremental updates (H2.1)

### Cross-Agent Memory Exit Criteria

- [ ] Shared namespaces configurable via `shared_namespaces` and `import_namespaces` (H1)
- [ ] Symlink-based references work for read-only cross-agent access (H1)
- [ ] Write access requires explicit `read_write = true` flag (H1)

### WITNESS Exit Criteria

- [ ] WITNESS segments use SHA-256 hash chaining (H2.6)
- [ ] Sequential verification from root detects tampering (H2.6)
- [ ] `weft memory export` includes WITNESS chain; `weft memory import` validates it (H2.4 + H2.6)

### Async Embedding Exit Criteria

- [ ] `store()` returns immediately; embedding runs in background (H2.2)
- [ ] `pending_embeddings` queue tracks in-flight items (H2.2)
- [ ] Fallback to keyword-only search when embedding API unavailable (H2.2)

### RVF 0.2 Exit Criteria

- [ ] RVF 0.2 API audit completed in Week 4 (H2.3 pre-work)
- [ ] Segment I/O path confirmed (either via `rvf-runtime` or local implementation) (H2.3)

---

## Security Checklist

| Check | Items Affected | Requirement |
|-------|---------------|-------------|
| Workspace filesystem permissions | H1 | Agent dirs created with 0700; cross-agent symlinks respect read-only default |
| Secret isolation | H1 | Per-agent workspace does not expose other agents' secrets via shared namespaces |
| WITNESS integrity | H2.6 | SHA-256 chain cannot be forged; import validates full chain before accepting data |
| Embedding API key handling | H2.2 | API keys via `SecretRef` (env var name, not plaintext); no keys in memory exports |
| RVF file validation | H2.3 | Imported RVF files validated for segment integrity before loading into memory |

---

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| H1 Workspace Review | H1 | Agent isolation verified; symlink-based sharing tested; 0700 permissions confirmed |
| H2 Core Review | H2.1, H2.2 | Embedder trait compliance; HNSW search quality verified; async embedding pipeline tested |
| H2 MVP Review | H2.3, H2.4, H2.5 | RVF segment I/O works; export/import round-trip verified; POLICY_KERNEL persistence tested |
| H2 Advanced Review | H2.6, H2.7, H2.8 | WITNESS chain validated; quantization tiers correct; WASM module <8KB |
| H3 Timestamp Review | H3 | Zero i64 ms or Option<String> remaining; all crates compile with DateTime<Utc> |
| Cross-Element Review | H1, H2 | Workspace creation via routing (L2); vector search for ClawHub (K4) |

---

## MVP vs Post-MVP

### MVP (H1, H2.1, H2.2, H2.3, H2.4, H2.5, H3)

Core functionality required for the agent system to operate with proper workspace isolation, vector memory, persistence, and consistent timestamps. These 7 items (plus H3) are required before Element 09 (multi-agent routing) and Element 10 (ClawHub) can proceed.

### Post-MVP (H2.6, H2.7, H2.8)

Advanced features that enhance security (WITNESS), storage efficiency (quantization), and WASM support (micro-HNSW). These can be deferred without blocking other elements. If schedule pressure mounts in Week 7-8, these items defer to a follow-up sprint.

---

## Risk Register

Scoring: Likelihood (Low=1, Medium=2, High=3) x Impact (Low=1, Medium=2, High=3, Critical=4)

| # | Risk | Likelihood | Impact | Score | Mitigation |
|---|------|-----------|--------|-------|------------|
| R1 | `instant-distance` crate maturity (smaller project, limited maintainers) | Medium | Medium | **4** | Pure Rust with no unsafe; verify correctness via property-based tests; vendoring as fallback |
| R2 | Embedding API availability/latency affects store() path | Medium | Medium | **4** | Async pipeline with pending_embeddings queue; fallback to keyword-only search; retry with backoff |
| R3 | Cross-agent shared memory consistency under concurrent writes | Low | High | **4** | Read-only by default; write access requires explicit opt-in with fs-level locking; eventual consistency |
| R4 | RVF 0.2 lacks segment I/O, forcing local implementation | Medium | Medium | **4** | Week 4 audit determines path; local implementation using `rvf-types` as fallback |
| R5 | HNSW index memory consumption (100K vectors x 1536 dims ~ 600MB) | Medium | High | **6** | Support dimensionality reduction (256-dim via Matryoshka); `max_vectors` config with cold eviction |
| R6 | Periodic re-index latency spike under load | Low | Medium | **3** | Background re-index with read-continue semantics; schedule during low-activity windows |

---

## Merge Coordination

### Merge Order

1. **H3** (Timestamps) -- foundational type change, merge first to avoid conflicts
2. **H1** (Workspace Isolation) -- independent, unblocks L2 routing
3. **H2.1 + H2.2** (HNSW + Embedder) -- core vector memory
4. **H2.3 + H2.5** (RVF I/O + POLICY_KERNEL) -- persistence layer
5. **H2.4** (Export/Import CLI) -- depends on H2.3
6. **H2.6 + H2.7 + H2.8** (Post-MVP, independent of each other)

### File Ownership

Within `clawft-core/src/embeddings/`:
- H2.1 owns `hnsw_store.rs` (new)
- H2.2 owns `mod.rs` (Embedder trait) + `hash_embedder.rs` + `api_embedder.rs`
- H2.3 owns `rvf_stub.rs` (replace with full implementation)
- H2.6 owns `witness.rs` (new)
- H2.7 owns `quantization.rs` (new)

### Conflict Zones

- `clawft-core/src/embeddings/mod.rs` is touched by H2.1, H2.2, and H2.3 -- coordinate merges
- `clawft-types/` timestamp changes (H3) affect all crates -- merge H3 first
- Workspace config types in `clawft-types/workspace.rs` touched by both H1 and H3

---

## Progress Summary

| Phase | Items | Pending | In Progress | Completed | % Done |
|-------|-------|---------|-------------|-----------|--------|
| H1 + H3 (Workspace + Timestamps) | 2 | 2 | 0 | 0 | 0% |
| H2 Core (H2.1, H2.2) | 2 | 2 | 0 | 0 | 0% |
| H2 MVP (H2.3, H2.4, H2.5) | 3 | 3 | 0 | 0 | 0% |
| H2 Advanced (H2.6, H2.7, H2.8) | 3 | 3 | 0 | 0 | 0% |
| **Total** | **10** | **10** | **0** | **0** | **0%** |
