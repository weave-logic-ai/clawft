# Development Notes: Element 08 - Memory & Workspace

**Workstream**: H
**Weeks**: 4-8
**Status**: All phases complete (H3, H1, H2.1-H2.8)
**Completed**: 2026-02-20 (Core + MVP + Advanced)
**Agent**: Agent-08 (af05f4a), Agent-08b (continuation)

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

### H2.5: POLICY_KERNEL Persistence -- DONE
- `crates/clawft-core/src/policy_kernel.rs` (new, ~515 lines)
- `PolicyKernel` struct with load/save/add_policy/record_cost/clear/set_state
- Persists to `~/.clawft/agents/<id>/policy_kernel.json` or `~/.clawft/memory/policy_kernel.json`
- `PolicyKernelState` with version field for forward compatibility
- `PolicyEntry` and `PersistedCostRecord` serializable types
- `load_policy_kernel(agent_id)` convenience with agent -> global -> in-memory fallback
- Gated behind `vector-memory` feature
- 12 tests covering persistence, roundtrip, clear, set_state, paths, errors

### H2.6: WITNESS Segments (SHA-256 Chain) -- DONE
- `crates/clawft-core/src/embeddings/witness.rs` (new, ~490 lines)
- SHA-256 hash-chained audit trail for memory operations
- `WitnessSegment` with segment_id (Uuid), timestamp, operation, data_hash, previous_hash, segment_hash
- `WitnessChain` with append/verify/verify_detailed/save/load/load_verified/to_json/from_json
- Chain integrity from GENESIS_HASH = [0u8; 32]
- `WitnessOperation` enum: Store, Update, Delete
- `segments_mut()` for test/import access
- Gated behind `rvf` feature (uses `sha2` crate)
- 17 tests covering chain integrity, tamper detection, serialization, persistence

### H2.3: RVF Segment I/O -- DONE
- `crates/clawft-core/src/embeddings/rvf_io.rs` (new, ~540 lines)
- **RVF 0.2 Audit Result**: rvf-runtime 0.2 has full segment I/O but is tightly coupled to binary format with internal indexing. Decision: local JSON-based serialization using rvf-types as conceptual model.
- `SegmentFile` with header, segments vec, optional WitnessChain
- `MemorySegment` with id, segment_type, namespace, embedding, text, metadata, tags, timestamps
- `MemorySegmentType` enum: Vector, Text, Policy
- `SegmentFileBuilder` for fluent file construction
- `write_segment_file()`, `read_segment_file()`, `read_verified_segment_file()` I/O functions
- `segment_file_path()` resolves `~/.clawft/agents/<id>/memory/<namespace>.rvf.json`
- Gated behind `rvf` feature
- 10 tests including WITNESS integration and tamper rejection

### H2.4: Memory Export/Import CLI -- DONE
- Modified `crates/clawft-cli/src/main.rs` -- added Export/Import variants to MemoryCmd enum
- Modified `crates/clawft-cli/src/commands/memory_cmd.rs` -- added `memory_export()` and `memory_import()` async functions
- `MemoryExport` struct with version, agent_id, format, exported_at, memory_content, history_content
- Export writes JSON to file; Import reads/validates
- Tests for serialization roundtrip

### H2.7: Temperature-Based Quantization -- DONE
- `crates/clawft-core/src/embeddings/quantization.rs` (new, ~490 lines)
- Hot (f32, full precision), Warm (fp16, half precision), Cold (PQ, product quantized)
- `f32_to_fp16()` and `fp16_to_f32()` IEEE 754 half-precision conversion
- `PqCodebook` with build/encode/decode (uniform quantization, 8 subvectors, 256 centroids)
- `QuantizedVector` enum with compress/decompress/temperature methods
- `AccessTracker` with access_count, last_access, temperature, recommended_tier()
- Tier thresholds: Hot (<1hr or >10 accesses), Warm (<1day or >3 accesses), Cold (else)
- Gated behind `rvf` feature
- 18 tests covering fp16 roundtrip, PQ roundtrip, tier transitions

### H2.8: WASM Micro-HNSW -- DONE
- `crates/clawft-core/src/embeddings/micro_hnsw.rs` (new, ~544 lines)
- Minimal HNSW for WASM with 8KB size budget
- Single-layer graph, max 1024 vectors, max 16 neighbors per node
- `MicroHnswRequest` and `MicroHnswResponse` enums for message passing protocol
- `MicroHnsw` with process/insert/query/delete methods
- Brute-force fallback for <= 64 nodes, greedy graph search above
- Upsert semantics, dimension validation
- Cosine similarity distance metric
- Gated behind `vector-memory` feature
- 12 tests covering insert/query/delete/upsert/ordering/serialization

### Ancillary Fixes (Agent-08b)
- Fixed `collapsible_if` clippy warning in `intelligent_router.rs`
- Fixed missing `sandbox` module stub in `clawft-core/src/agent/`
- Fixed borrow checker error in `sandbox.rs` drain pattern
- Fixed 3 raw string literal errors in `clawft-security/src/checks/patterns.rs` (pre-existing)
- Fixed `collapsible_if` in `policy_kernel.rs` and `rvf_io.rs` using let-chains

### Test Results
- All 1016 tests pass with `cargo test -p clawft-core --features rvf --lib`
- Workspace clippy passes with `-D warnings`
- `cargo check -p clawft-cli` compiles cleanly

### Feature Gating Summary
- `vector-memory` feature: hnsw_store, policy_kernel, micro_hnsw
- `rvf` feature (superset of vector-memory): witness, rvf_io, quantization, api_embedder
