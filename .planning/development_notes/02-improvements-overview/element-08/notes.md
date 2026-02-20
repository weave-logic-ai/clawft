# Development Notes: Element 08 - Memory & Workspace

**Workstream**: H
**Weeks**: 4-8
**Status**: Core complete (H3, H1, H2.1, H2.2), MVP/Advanced pending
**Completed**: 2026-02-20 (Core)
**Agent**: Agent-08 (af05f4a)

---

## Implementation Log

### H3: Timestamp Standardization -- DONE
- Unified timestamps to `DateTime<Utc>` throughout
- Replaced i64 ms and Option<String> timestamp patterns

### H1: Per-Agent Workspace Isolation -- DONE
- `workspace.rs` split: 1157 lines -> workspace/mod.rs (456) + agent.rs (511) + config.rs (261)
- `~/.clawft/agents/<agentId>/` directory structure with SOUL.md, AGENTS.md, USER.md
- Per-agent session store and skill overrides
- 3-level config merge (global -> agent -> workspace)
- Cross-agent shared namespace support (symlink-based, read-only default)

### H2.1: HNSW-backed VectorStore -- DONE
- `instant-distance` 0.6 added to workspace deps (feature-gated: `vector-memory`)
- `crates/clawft-core/src/embeddings/hnsw_store.rs` (625 lines)
- HnswStore with automatic brute-force fallback for <32 entries
- EmbeddingPoint wrapper using `1.0 - cosine_similarity` as distance metric
- Configurable ef_search (default 100), ef_construction (default 200)
- Upsert semantics, lazy index rebuild, delete, get, save/load JSON persistence
- 18 tests

### H2.2: Embedder Trait Enhancements -- DONE
- `HashEmbedder::name()` returns "hash"
- `ApiEmbedder::name()` returns model name (e.g. "text-embedding-3-small")
- Module registration in `embeddings/mod.rs`

### Ancillary Fix
- Fixed `collapsible_if` clippy warning in `intelligent_router.rs`

### Remaining
- H2.3 (RVF I/O), H2.4 (export/import CLI), H2.5 (POLICY_KERNEL) -- MVP
- H2.6 (WITNESS), H2.7 (quantization), H2.8 (WASM micro-HNSW) -- Advanced
