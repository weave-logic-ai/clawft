# ECC Symposium: WeftOS Gap Analysis

**What WeftOS needs to become an ECC-capable cognitive platform**

---

## Summary Scorecard

| ECC Component | WeftOS Implementation % | Primary Location | Gap Severity |
|---|---|---|---|
| HNSW Vector Index | **95%** | clawft-core/embeddings/hnsw_store.rs | LOW -- needs kernel wiring |
| ExoChain Provenance | **85%** | clawft-kernel/chain.rs + tree_manager.rs | LOW -- needs DAG extension |
| Resource Tree | **80%** | exo-resource-tree/ | LOW -- permission stub |
| WASM Sandbox | **80%** | clawft-wasm/sandbox.rs, engine.rs | LOW -- needs tick-budget tuning |
| Governance/Gate | **70%** | clawft-kernel/governance.rs, gate.rs | LOW -- enrich context |
| CMVG Wire Protocol | **60%** | clawft-weave/rvf_codec.rs | MEDIUM -- missing zstd, delta |
| Causal Graph (DAG) | **15%** | exo-resource-tree (tree only) | CRITICAL -- no DAG structure |
| Cognitive Tick Loop | **0%** | N/A | CRITICAL -- entirely new |
| Spectral Analysis | **0%** | N/A | MEDIUM -- background service |

**Overall: ~55% of ECC infrastructure exists. Two critical gaps, two medium, five low.**

---

## Critical Gaps

### GAP-1: Causal Graph with Typed Edges (15% -> needs ~85%)

**What exists**: The ResourceTree is a tree (single parent per node). ExoChain has lineage records (Clone/Mutate/Combine/Derive/Fork). Neither supports arbitrary directed edges between concept nodes.

**What ECC requires**: A directed acyclic graph where nodes represent cognitive moments and edges represent causal relationships with:
- Edge types: Causes, Inhibits, Correlates, Enables, Follows, Contradicts
- Edge weights: f32 strength values
- Multi-parent support (a moment can have multiple causes)
- Fast adjacency lookup (DashMap-backed for concurrency)
- Integration with ExoChain (each edge mutation logged as a chain event)
- Integration with Resource Tree (graph registered under `/kernel/services/causal-graph`)

**Recommended implementation**:
- New module `causal.rs` in clawft-kernel behind an `ecc` feature flag
- In-memory adjacency structure using `DashMap<NodeId, Vec<CausalEdge>>`
- `CausalEdge` struct: `{source, target, edge_type, weight, timestamp, chain_seq}`
- Follows the "Adding a New Gated Subsystem" pattern from integration-patterns.md
- Consider adopting `exo-dag` crate from the ruv ecosystem (DAG audit trail with MMR proofs)

**Effort estimate**: ~800-1200 lines of Rust

---

### GAP-2: Cognitive Tick Loop (0% -> needs 100%)

**What exists**: The `kernel_agent_loop` is event-driven (blocks on `inbox.recv()`). The `CronService` is seconds-granularity. Neither supports fixed-interval millisecond ticks.

**What ECC requires**: A 50ms fixed-rate loop that executes in 3.6ms:
1. **Sense**: Read sensor data / receive input events
2. **Embed**: Generate vector embedding of current moment
3. **Search**: HNSW k-NN search for similar past moments
4. **Update**: Add causal edges linking current moment to causes
5. **Commit**: Merkle-hash the tick's results to ExoChain

Background every ~100 ticks (~5s): spectral analysis (Fiedler value).

**Recommended implementation**:
- New `CognitiveTick` service implementing `SystemService` trait
- Uses `tokio::time::interval(Duration::from_millis(50))` for the tick timer
- Wraps the HNSW service and CausalGraph service via kernel handles
- Tick counter with drift monitoring (alert if tick exceeds budget)
- Sends `KernelMessage` to the agent loop when cognitive state changes require action
- Separate from the agent loop (runs its own tight loop), but communicates via the existing A2A/IPC infrastructure

**Effort estimate**: ~500-800 lines of Rust

---

## Medium Gaps

### GAP-3: Spectral Analysis (0% -> needs 100%)

**What exists**: No linear algebra, no eigenvalue computation, no graph Laplacian.

**What ECC requires**:
- Normalized Laplacian matrix from the causal graph adjacency matrix
- Eigenvalue decomposition (power iteration or Lanczos)
- Lambda_2 (Fiedler value) extraction for metacognition ("how coherent is the current knowledge?")
- Runs in background every ~5 seconds (not per-tick)

**Recommended implementation**:
- New module or crate (e.g., `mentra-spectral` or inline in clawft-kernel)
- Lightweight: only needs top-5 eigenvalues, not full decomposition
- Power iteration sufficient for graphs <2000 nodes (proven on ARM64 at <200ms)
- Consider `prime-radiant` crate from ruv ecosystem (sheaf Laplacian, Betti numbers, topological coherence)
- Add `nalgebra` or custom sparse matrix ops to Cargo.toml

**Effort estimate**: ~400-600 lines of Rust

---

### GAP-4: CMVG Wire Protocol Extensions (60% -> needs ~40%)

**What exists**: RVF frame codec with length-prefixed framing, content hash verification, Ed25519 segment signing, `rvf_wire::validate_segment`.

**What ECC requires additionally**:
- **zstd compression**: ~5:1 on vector data. Add `zstd` crate, implement as a segment flag or wrapper
- **Delta encoding**: Send only changed graph nodes/edges, not full state
- **BLAKE3 option**: Already available via cognitum-gate-tilezero; surface as alternative to SHAKE-256

**Recommended implementation**:
- Add `zstd` to workspace Cargo.toml
- New segment type or flag in RVF codec for compressed segments
- Delta encoder struct tracking last-sent state hash per peer
- BLAKE3 as configurable hash algorithm in ChainManager (alongside SHAKE-256)

**Effort estimate**: ~300-500 lines of Rust

---

## Low Gaps (Minor Extensions to Existing Code)

### GAP-5: HNSW Kernel Integration (95% -> needs ~5%)

**What exists**: Full HNSW in `clawft-core/embeddings/hnsw_store.rs` with instant-distance, plus `micro_hnsw.rs` for WASM. NOT wired into the kernel.

**What's needed**:
- Register HNSW as a `SystemService` in kernel boot
- Add resource tree node at `/kernel/services/hnsw`
- Log index mutations to ExoChain (`hnsw.insert`, `hnsw.search`, `hnsw.rebuild`)
- Gate checks on which agents can query which indexes
- Feature flag: `ecc` or `hnsw-kernel`

**Effort estimate**: ~200-400 lines of Rust (wiring, not reimplementation)

### GAP-6: ExoChain DAG Extension (85% -> needs ~15%)

**What exists**: Linear hash chain with single `prev_hash` per event.

**What's needed for CMVG**: Support for multiple parent hashes (DAG structure). Options:
- Extend `ChainEvent` with `parent_hashes: Vec<[u8; 32]>` alongside existing `prev_hash`
- Or maintain the linear chain as-is and build the DAG overlay in the CausalGraph module, using chain sequence numbers as node identifiers

**Recommendation**: Keep ExoChain linear (it's the audit log), build the DAG in the CausalGraph module. The chain provides temporal ordering; the graph provides causal structure. Simpler and preserves ExoChain's proven integrity model.

**Effort estimate**: ~100-200 lines (DAG lives in GAP-1's CausalGraph module)

### GAP-7: Governance Context Enrichment (70% -> needs ~30%)

**What exists**: GovernanceGate with 5D EffectVector, generic `"tool.exec"` gate action.

**What's needed**: Per-tool gate actions (CF-1 from K3 symposium, addressed by D2). For ECC: cognitive operations need their own gate actions (`ecc.embed`, `ecc.search`, `ecc.causal.link`, `ecc.spectral.analyze`) with appropriate EffectVector scoring.

**Effort estimate**: ~100-200 lines

### GAP-8: Permission Engine (80% -> needs ~20%)

**What exists**: `check()` always returns `Allow`. DelegationCert type defined but no lifecycle.

**What's needed**: Real ACL-based permission checks. ECC agents need scoped access to specific HNSW indexes and causal graph regions. The deferred K1 work.

**Effort estimate**: ~300-500 lines

---

## Dependency Additions Required

| Crate | Purpose | Priority |
|-------|---------|----------|
| `nalgebra` or custom | Sparse matrix ops for spectral analysis | Medium (GAP-3) |
| `zstd` | Compression for CMVG wire protocol | Medium (GAP-4) |
| `blake3` | Already transitive dep (tilezero); surface for direct use | Low |
| `exo-dag` (ruv ecosystem) | DAG audit trail with MMR proofs | Consider for GAP-1 |
| `prime-radiant` (ruv ecosystem) | Topological coherence, Betti numbers | Consider for GAP-3 |

---

## Implementation Priority & Phasing

### Phase ECC-1 (K4 timeframe): Core Cognitive Substrate
1. **GAP-1**: CausalGraph module (CRITICAL)
2. **GAP-5**: HNSW kernel integration (LOW effort, HIGH value)
3. **GAP-2**: CognitiveTick service (CRITICAL)

This delivers a working cognitive tick: embed -> search -> link -> commit.

### Phase ECC-2 (K4-K5 timeframe): Intelligence Layer
4. **GAP-3**: Spectral analysis (background metacognition)
5. **GAP-7**: Governance context enrichment (per-operation gate actions)
6. **GAP-8**: Permission engine (scoped cognitive access)

This adds metacognition and security to the cognitive substrate.

### Phase ECC-3 (K5-K6 timeframe): Distribution
7. **GAP-4**: Wire protocol extensions (zstd, delta encoding)
8. **GAP-6**: DAG overlay for distributed provenance

This enables multi-node cognitive swarms with CMVG delta sync.

---

## What WeftOS Got Right for ECC (No Changes Needed)

1. **ExoChain's append-only model** -- perfectly matches CMVG's monotonically-growing provenance
2. **Universal witnessing (D9)** -- every interaction becomes a potential CMVG edge
3. **Resource Tree Merkle hashing** -- tamper-evident namespace for cognitive state
4. **6D NodeScoring with EMA** -- ready-made cognitive metadata with cosine similarity
5. **Feature-gated modularity** -- ECC can be added behind `ecc` feature flag without bloating the default build
6. **"Adding a New Gated Subsystem" integration pattern** -- documented recipe for exactly this kind of extension
7. **Platform-generic kernel (`Kernel<P: Platform>`)** -- ECC works on native, browser, WASM, and edge targets
8. **DashMap concurrency** -- lock-free maps for high-frequency cognitive operations
