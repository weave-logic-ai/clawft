# Sprint 11 Symposium -- Track 9: Extreme Software Optimization Plan

**Panel**: Performance Engineer (chair), System Performance Remediator,
FrankenSearch Specialist, AgentDB Optimizer, GDB Debugging Specialist,
Asupersync Analyst

**Date**: 2026-03-27
**Codebase**: WeftOS v0.1.0-pre (`feature/weftos-kernel-sprint`)
**Methodology**: Profile first, prove behavior unchanged, one change at a time,
Score >= 2.0 only

---

## 1. Codebase Profile Summary

### 1.1 Architecture Overview

The kernel (`clawft-kernel`, 60,618 LOC across 68 source files) is the
performance-critical core. Key subsystems and their hot paths:

| Subsystem | Files | LOC | Feature Gate | Hot Path? |
|-----------|-------|-----|-------------|-----------|
| DEMOCRITUS tick loop | `democritus.rs`, `cognitive_tick.rs` | 1,319 | `ecc` | **YES** -- runs every 50ms |
| HNSW vector search | `hnsw_store.rs`, `hnsw_service.rs` | 979 | `ecc` | **YES** -- called per impulse per tick |
| A2A message routing | `a2a.rs`, `ipc.rs`, `topic.rs` | 2,862 | always | **YES** -- every agent message |
| ExoChain append | `chain.rs` | 2,997 | `exochain` | **YES** -- every audited event |
| Kernel boot | `boot.rs` | 2,820 | always | Startup only |
| WASM sandbox | `wasm_runner.rs` | 5,039 | `wasm-sandbox` | Per tool call |
| Causal graph | `causal.rs` | 1,850 | `ecc` | Per tick (via DEMOCRITUS) |
| Embedding | `embedding.rs`, `embedding_onnx.rs` | 1,784 | `ecc` | Per tick (via DEMOCRITUS) |

### 1.2 Build Profile

From `Cargo.toml`:

```toml
[profile.release]
opt-level = "z"     # size-optimized, NOT speed-optimized
lto = true
strip = true
codegen-units = 1
panic = "abort"
```

**Finding**: `opt-level = "z"` trades speed for binary size. This is correct
for WASM targets but penalizes native performance. The workspace uses a single
release profile for everything.

### 1.3 Dependency Weight

Heavy optional dependencies:
- `wasmtime` (v29) -- massive, feature-gated behind `wasm-sandbox`
- `ort` (v2.0.0-rc.12) -- ONNX runtime, feature-gated behind `onnx-embeddings`
- `ed25519-dalek`, `rvf-crypto`, `rvf-wire` -- crypto stack for exochain
- `tokio` with `features = ["full"]` -- pulls in everything

### 1.4 Concurrency Model

- `std::sync::Mutex` used in 42 locations across 10 files
- `DashMap` used for concurrent structures (causal graph, crossref store, A2A inboxes)
- `HnswService` wraps `HnswStore` behind a `Mutex` -- search blocks insert and vice versa
- `CognitiveTick` uses `Mutex` for mutable state on every tick recording

---

## 2. Hotspot Analysis

### 2.1 DEMOCRITUS Tick Loop (50ms budget, 15ms compute budget)

**Path**: `DemocritusLoop::tick()` -- SENSE -> EMBED -> SEARCH -> UPDATE -> COMMIT

**Evidence from code** (`democritus.rs:130-189`):

1. **SENSE**: `impulse_queue.drain_ready()` -- drains all impulses, truncates to 64.
   The `ImpulseQueue` uses `Mutex<Vec<Impulse>>`. Every drain locks the queue.

2. **EMBED**: Calls `embedding_provider.embed_batch()`. The default trait impl
   is sequential (`for text in texts { embed(text).await }`). Even the
   `MockEmbeddingProvider` allocates a new `Vec<f32>` per embedding via SHA-256
   hashing.

3. **SEARCH**: Calls `hnsw.search()` per impulse in a serial loop
   (`democritus.rs:165-173`). Each search acquires the `Mutex` on `HnswStore`.
   With 64 impulses/tick, that is 64 mutex acquisitions.

4. **UPDATE**: For each impulse, calls `hnsw.insert()` (another mutex acquisition),
   `causal_graph.add_node()`, and `crossref_store.insert()`. With N impulses
   and K neighbors each, this is O(N*K) edge insertions.

5. **COMMIT**: Atomic counter increment + debug log. Negligible.

**Allocation hotspots in tick loop**:
- `embedding.to_vec()` at line 266 -- copies entire embedding vector per insert
- `embedding.clone()` at line 172 -- clones embedding per neighbor-search tuple
- `node_id.to_string()` at line 265 -- u64-to-String allocation per insert
- `format!("{type_str}:{payload_str}")` at line 213 -- String allocation per embed
- `imp.payload.to_string()` at line 212 -- JSON-to-String for embedding text
- `entry.id.clone()` and `entry.metadata.clone()` in HNSW query results

### 2.2 HNSW Search (`hnsw_store.rs`)

**Critical findings**:

1. **O(n) upsert via `retain`** (`hnsw_store.rs:149`): `self.entries.retain(|e| e.id != id)`
   scans all entries on every insert. At 10,000 entries this is 10,000 string
   comparisons per insert.

2. **Full index rebuild on every query after any insert** (`hnsw_store.rs:176-178`):
   When `dirty` is true, `rebuild_index()` clones all embedding vectors and
   rebuilds the entire HNSW graph. A single insert between queries triggers a
   full rebuild.

3. **Metadata cloned on every query result** (`hnsw_store.rs:199`):
   `metadata: entry.metadata.clone()` -- copies arbitrary JSON per result.

4. **Cosine similarity is naive scalar** (`hnsw_store.rs:370-384`):
   ```rust
   let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
   ```
   No SIMD intrinsics, no loop unrolling hints.

5. **`Mutex` serializes all access** (`hnsw_service.rs:84,91`):
   The `HnswService` wraps the store in `Mutex<HnswStore>`, meaning search()
   takes `&mut self` (because `query()` may call `rebuild_index()`). This
   prevents concurrent reads.

### 2.3 IPC Message Routing

**Finding**: Every kernel message creates a UUID v4 string (`ipc.rs:197`),
serializes the entire message to JSON (`ipc.rs:401-402`), and publishes through
the bus. UUID generation involves syscall overhead (entropy source). JSON
serialization is O(message_size).

17 `uuid::Uuid::new_v4()` calls scattered across the kernel for various
identifiers. On the IPC hot path, this is called per message.

### 2.4 ExoChain Append

**Finding**: Chain events use SHAKE-256 hashing for `prev_hash`, `payload_hash`,
and `event_hash` -- three hash operations per append. The chain is behind a
`Mutex`. Ed25519 signing (when enabled) adds ~50us per event on ARM64.

### 2.5 Build Configuration

- `tokio` with `features = ["full"]` -- pulls `io`, `net`, `signal`, `process`, `fs`.
  The CLI crate explicitly depends on `clawft-core` with `features = ["native", "full"]`.
  Most agents do not need `process` or `signal`.

- No separate `profile.release-native` for speed-optimized native builds.
  Both WASM and native share `opt-level = "z"`.

---

## 3. Opportunity Matrix

Scoring: **Score = Impact x Confidence / Effort** (minimum 2.0 to recommend)

### 3.1 v0.1.x Backport Candidates (Safe, Low-Risk)

| # | Hotspot | Fix | Impact | Conf | Effort | Score | Skill |
|---|---------|-----|--------|------|--------|-------|-------|
| B1 | HNSW O(n) upsert | Replace `retain()` + `push()` with `HashMap<String, usize>` ID index | 5 | 5 | 2 | **12.5** | `agentdb-optimization` |
| B2 | HNSW full rebuild on every dirty query | Use incremental insert into existing HNSW graph (instant-distance supports `insert` if we track the builder, or batch rebuilds only when count changes by >10%) | 5 | 5 | 3 | **8.3** | `agentdb-optimization` |
| B3 | DEMOCRITUS embedding.clone() in search loop | Borrow `&[f32]` instead of cloning; restructure the neighbor tuple to use references | 3 | 5 | 1 | **15.0** | `extreme-software-optimization` |
| B4 | HNSW metadata.clone() on query | Return `&serde_json::Value` or use `Arc<serde_json::Value>` for metadata | 3 | 4 | 2 | **6.0** | `extreme-software-optimization` |
| B5 | IPC JSON serialization on hot path | Use `serde_json::to_vec()` (avoids UTF-8 validation) or switch to bincode/CBOR for internal messages | 4 | 4 | 2 | **8.0** | `extreme-software-optimization` |
| B6 | UUID v4 on every IPC message | Use atomic u64 counter for internal message IDs; reserve UUID for external-facing IDs | 3 | 4 | 1 | **12.0** | `extreme-software-optimization` |
| B7 | Release profile `opt-level = "z"` for native | Add `[profile.release-native]` with `opt-level = 3` for native builds | 4 | 5 | 1 | **20.0** | `system-performance-remediation` |
| B8 | `CognitiveTick` timing window `Vec` growth | Pre-allocate `recent_timings_us` with `Vec::with_capacity(window_size)` | 2 | 4 | 1 | **8.0** | `extreme-software-optimization` |
| B9 | `tokio` full features in CLI | Reduce to `["rt-multi-thread", "macros", "io-util", "time"]` | 2 | 3 | 1 | **6.0** | `system-performance-remediation` |
| B10 | DEMOCRITUS format! allocations in embed() | Use a reusable `String` buffer or `Cow<str>` | 2 | 4 | 1 | **8.0** | `extreme-software-optimization` |

### 3.2 v0.2 Candidates (With K8 GUI)

| # | Hotspot | Fix | Impact | Conf | Effort | Score | Skill |
|---|---------|-----|--------|------|--------|-------|-------|
| O1 | HNSW Mutex contention | Replace `Mutex<HnswStore>` with `RwLock<HnswStore>` + separate dirty flag. Reads can proceed concurrently; only rebuild needs write lock | 5 | 4 | 3 | **6.7** | `agentdb-optimization` |
| O2 | Cosine similarity scalar loop | Add `#[cfg(target_arch)]` SIMD paths using `std::simd` (nightly) or `packed_simd2`; fallback to auto-vectorizable loop with `chunks_exact(4)` | 4 | 4 | 3 | **5.3** | `extreme-software-optimization` |
| O3 | DEMOCRITUS serial search loop | Batch search: collect all embeddings, do a single lock acquisition, run all queries within one lock scope | 5 | 4 | 2 | **10.0** | `agentdb-optimization` |
| O4 | Adaptive tick budget waste | Implement "batch when idle, single when busy" -- accumulate impulses for batch embedding instead of per-tick drain | 4 | 3 | 3 | **4.0** | `extreme-software-optimization` |
| O5 | FrankenSearch two-tier search | Evaluate adding keyword BM25 index alongside HNSW for hybrid search with RRF fusion. Most beneficial at >10K entries when pure semantic search misses exact matches | 4 | 3 | 4 | **3.0** | `frankensearch-integration-for-rust-projects` |
| O6 | Binary size: split release profiles | `[profile.release-wasm]` with `opt-level = "z"`, `[profile.release-native]` with `opt-level = 3`. Gate script already has `cmd_native` / `cmd_wasi` distinction | 4 | 5 | 1 | **20.0** | `system-performance-remediation` |
| O7 | HNSW persistence JSON format | Switch to bincode or MessagePack for save/load; JSON serialization of float vectors is 3-5x larger than binary | 3 | 4 | 2 | **6.0** | `agentdb-optimization` |
| O8 | Embedding batch size tuning | Current `embed_batch` default impl is sequential; ONNX backend should batch up to 16 texts per inference call (config exists: `batch_size: 16`) | 4 | 4 | 2 | **8.0** | `extreme-software-optimization` |
| O9 | Chain append Mutex contention | Use `parking_lot::Mutex` for shorter critical sections (no poison checking overhead) or ring buffer for append-only chain | 3 | 3 | 2 | **4.5** | `extreme-software-optimization` |
| O10 | HNSW quantization for large stores | Implement int8 quantization for stored embeddings (4x memory reduction for 384-dim vectors: 384*4 = 1536B -> 384B per vector) | 4 | 3 | 4 | **3.0** | `agentdb-optimization` |

### 3.3 v0.3 Candidates (Enterprise Readiness)

| # | Hotspot | Fix | Impact | Conf | Effort | Score | Skill |
|---|---------|-----|--------|------|--------|-------|-------|
| E1 | Async runtime cancel-correctness | Evaluate Tokio's cancellation safety across all `.await` points in DEMOCRITUS, ExoChain, and A2A. Consider `tokio::select!` audit for cancel-safe branches | 5 | 3 | 4 | **3.8** | `asupersync-mega-skill` |
| E2 | Memory architecture: arena allocation | Replace per-impulse `Vec<f32>` embedding allocation with arena allocator (bumpalo) for the tick scope. Embeddings are short-lived within a single tick | 4 | 3 | 3 | **4.0** | `extreme-software-optimization` |
| E3 | Zero-copy IPC | Replace JSON serialization with zero-copy message passing (rkyv or flatbuffers) for in-process A2A messages. JSON only for mesh/external transport | 5 | 4 | 4 | **5.0** | `extreme-software-optimization` |
| E4 | HNSW -> dedicated vector DB | Evaluate replacing in-process `instant-distance` with a dedicated vector store (e.g., qdrant-client, or the rvf-runtime vector layer already in deps) | 4 | 3 | 5 | **2.4** | `agentdb-optimization` |
| E5 | Multi-node mesh throughput | Profile mesh framing (`mesh_framing.rs`) + noise protocol handshake overhead. Batched message sending, connection pooling per peer | 4 | 3 | 4 | **3.0** | `gdb-for-debugging` |
| E6 | Deadlock detection in mesh | Add `parking_lot`'s deadlock detection feature or implement lock ordering protocol across `HnswService`, `ChainManager`, and `ImpulseQueue` mutexes | 5 | 3 | 3 | **5.0** | `gdb-for-debugging` |
| E7 | Adaptive HNSW ef_search | Dynamically adjust ef_search based on dataset size and query latency percentiles. Small stores (< 1K) use low ef; large stores (> 100K) use higher ef | 3 | 3 | 3 | **3.0** | `agentdb-optimization` |

### 3.4 v1.0+ Candidates (Production)

| # | Hotspot | Fix | Impact | Conf | Effort | Score | Skill |
|---|---------|-----|--------|------|--------|-------|-------|
| P1 | SIMD cosine similarity (hardware-specific) | Use `std::arch::aarch64::vfmaq_f32` intrinsics for ARM64, `_mm256_fmadd_ps` for x86-64 AVX2 | 4 | 4 | 4 | **4.0** | `extreme-software-optimization` |
| P2 | GPU-accelerated embedding | Move ONNX inference to GPU backend when available (ort already supports CUDA/CoreML) | 5 | 3 | 5 | **3.0** | `extreme-software-optimization` |
| P3 | Product quantization for HNSW | Implement PQ for 10-100x memory reduction at scale (>100K vectors) | 4 | 3 | 5 | **2.4** | `agentdb-optimization` |
| P4 | Compile-time feature analysis | Measure compile times per feature combination; identify features that pull in disproportionate dependency trees | 3 | 4 | 2 | **6.0** | `system-performance-remediation` |

---

## 4. Version Plan

### 4.1 v0.1.x Backport/Fixes (Pre-tag, This Sprint)

**Theme**: Zero behavior change, pure performance wins.

**Priority order** (by Score):

1. **B7** (Score 20.0): Add `profile.release-native` with `opt-level = 3`
   - File: `Cargo.toml` (workspace root)
   - Verification: `testing-golden-artifacts` -- binary output unchanged for same inputs
   - Risk: None. Existing `release` profile stays for WASM.

2. **B3** (Score 15.0): Eliminate `embedding.clone()` in DEMOCRITUS search loop
   - File: `crates/clawft-kernel/src/democritus.rs:172`
   - Change: Restructure `neighbors_per_event` to borrow `&[f32]` from `embedded` vec
   - Verification: Existing tests in `democritus.rs::tests` cover all tick phases

3. **B1** (Score 12.5): HNSW O(n) upsert -> O(1) with HashMap index
   - File: `crates/clawft-core/src/embeddings/hnsw_store.rs:148-155`
   - Change: Add `id_index: HashMap<String, usize>` to `HnswStore`; on insert, check index first, swap_remove + update index
   - Verification: `testing-golden-artifacts` for search result ordering; existing unit tests

4. **B6** (Score 12.0): Replace UUID v4 with atomic counter for internal IPC message IDs
   - File: `crates/clawft-kernel/src/ipc.rs:197`
   - Change: `static NEXT_MSG_ID: AtomicU64`; use `format!("kmsg-{}", id)` or just u64
   - Verification: `testing-metamorphic` -- message delivery unchanged regardless of ID format

5. **B5** (Score 8.0): IPC serialization: `to_string` -> `to_vec`
   - File: `crates/clawft-kernel/src/ipc.rs:401`
   - Minimal change: `serde_json::to_vec(msg)` avoids one UTF-8 validation pass

6. **B2** (Score 8.3): Deferred HNSW rebuild
   - File: `crates/clawft-core/src/embeddings/hnsw_store.rs:176-178`
   - Change: Track insert count since last rebuild; only rebuild when count exceeds 10% of current size or on explicit flush
   - Verification: `testing-metamorphic` -- recall@k must stay within 5% of brute-force for same data

7. **B8** (Score 8.0): Pre-allocate CognitiveTick timing window
8. **B10** (Score 8.0): Reuse String buffer in DEMOCRITUS embed phase
9. **B4** (Score 6.0): Arc metadata in HNSW query results
10. **B9** (Score 6.0): Trim tokio features

### 4.2 v0.2 (With K8 GUI)

**Theme**: Search optimization, tick loop efficiency, build speed.

**Critical path items**:

1. **O6** (Score 20.0): Split release profiles (native vs WASM)
   - Add to `Cargo.toml`:
     ```toml
     [profile.release-native]
     inherits = "release"
     opt-level = 3
     ```
   - Update `scripts/build.sh` `cmd_native()` to use `--profile release-native`

2. **O3** (Score 10.0): Batch HNSW searches within single lock scope
   - DEMOCRITUS acquires the HNSW mutex once per tick instead of N times
   - Requires adding `search_batch(&[&[f32]], top_k) -> Vec<Vec<Result>>` to HnswStore

3. **O8** (Score 8.0): Wire ONNX batch embedding properly
   - The `LlmEmbeddingConfig` already has `batch_size: 16`
   - OnnxEmbeddingProvider needs to implement true batch inference

4. **O1** (Score 6.7): RwLock for HnswService
   - Separate read path (search) from write path (insert + rebuild)
   - Search returns immutable references; rebuild takes exclusive write lock

5. **O7** (Score 6.0): Binary persistence for HNSW
   - Replace JSON with bincode for 3-5x size reduction and faster load

6. **O2** (Score 5.3): Auto-vectorizable cosine similarity
   - Rewrite with `chunks_exact(4)` to hint auto-vectorization
   - Measure before/after with criterion benchmarks

7. **O5** (Score 3.0): FrankenSearch evaluation
   - Only if search quality issues emerge at scale
   - Hybrid BM25 + HNSW with Reciprocal Rank Fusion
   - Decision point: defer to 0.3 unless user testing shows missed matches

### 4.3 v0.3 (Enterprise Readiness)

**Theme**: Async safety, memory architecture, multi-node performance.

1. **E6** (Score 5.0): Lock ordering protocol + deadlock detection
   - Define global lock ordering: ImpulseQueue < HnswService < ChainManager < CausalGraph
   - Use `parking_lot` with deadlock detection in debug builds

2. **E3** (Score 5.0): Zero-copy in-process IPC
   - JSON stays for mesh/external; rkyv for in-kernel A2A
   - Biggest win for tool-call-heavy workloads

3. **E2** (Score 4.0): Arena allocation for tick-scoped data
   - `bumpalo::Bump` allocated per tick; embeddings, labels, impulse clones all go in arena
   - Freed in one shot at tick end

4. **E1** (Score 3.8): Tokio cancellation safety audit
   - Map every `select!` branch for cancel-safety
   - Evaluate whether Asupersync's cancel-correct model is worth the migration cost
   - **Panel consensus**: Asupersync is architecturally interesting but the migration
     cost to replace Tokio would be extreme (every async dependency). Recommend
     auditing cancel-safety within Tokio rather than switching runtimes.

5. **E4** (Score 2.4): Vector DB evaluation
   - Only if HNSW in-process becomes a scaling bottleneck (>500K vectors)
   - The `rvf-runtime` crate is already a dependency -- investigate its vector capabilities first

### 4.4 v1.0+ (Production)

1. **P4** (Score 6.0): Feature compilation analysis
2. **P1** (Score 4.0): Platform-specific SIMD intrinsics
3. **P2** (Score 3.0): GPU-accelerated embedding
4. **P3** (Score 2.4): Product quantization

---

## 5. Cross-Skill Tool/Methodology Mapping

| Version | Item | Primary Skill | Supporting Skills |
|---------|------|---------------|-------------------|
| 0.1.x | B1-B10 | `extreme-software-optimization` | `testing-golden-artifacts`, `testing-metamorphic` |
| 0.1.x | B1, B2 | `agentdb-optimization` | `testing-golden-artifacts` |
| 0.1.x | B7 | `system-performance-remediation` | -- |
| 0.2 | O1-O3, O7, O10 | `agentdb-optimization` | `testing-metamorphic` |
| 0.2 | O2, O4, O8, O9 | `extreme-software-optimization` | `testing-golden-artifacts` |
| 0.2 | O5 | `frankensearch-integration-for-rust-projects` | `testing-metamorphic` |
| 0.2 | O6 | `system-performance-remediation` | -- |
| 0.3 | E1 | `asupersync-mega-skill` | `gdb-for-debugging` |
| 0.3 | E5, E6 | `gdb-for-debugging` | `testing-metamorphic` |
| 0.3 | E2, E3 | `extreme-software-optimization` | `testing-golden-artifacts` |
| 0.3 | E4, E7 | `agentdb-optimization` | `testing-metamorphic` |
| 1.0+ | P1-P4 | `extreme-software-optimization` | `system-performance-remediation` |

### Verification Requirements Per Change

Every optimization MUST follow this protocol:

1. **Baseline benchmark** before the change (criterion or `std::time::Instant` microbench)
2. **Golden output test**: capture output for a known input set before the change
3. **Apply one change only**
4. **Run existing test suite**: `scripts/build.sh test`
5. **Run golden output comparison**: outputs must be byte-identical (or within
   defined tolerance for float operations)
6. **Run benchmark**: must show measurable improvement
7. **Metamorphic test** (for search/graph changes): verify invariant properties
   (e.g., "if A is nearest neighbor of Q, adding unrelated point B does not
   change A's rank for Q")

---

## 6. Anti-Patterns to Avoid

### 6.1 Premature Optimization Without Profiling

The DEMOCRITUS tick loop has a 15ms budget with 50ms interval. Before optimizing
anything, run the full tick loop under `cargo flamegraph` with a realistic
workload (500+ impulses, 10K+ entries in HNSW). The hotspot might not be where
we think it is.

### 6.2 Changing Data Structures Without Benchmarks

Replacing `Vec<HnswEntry>` with `HashMap<String, HnswEntry>` for the HNSW store
changes iteration order and may affect HNSW graph quality. Always measure recall@k
before and after.

### 6.3 Over-Parallelizing the Tick Loop

The DEMOCRITUS tick is inherently sequential: SENSE -> EMBED -> SEARCH -> UPDATE -> COMMIT.
Attempting to pipeline stages adds complexity without benefit because each stage
depends on the previous. The parallelism opportunity is *within* stages (batch
search, batch embed), not *across* stages.

### 6.4 Replacing Tokio Without Measuring

The Asupersync panel member recommends against replacing Tokio. The cancel-
correctness concerns are real but addressable through audit. Replacing the async
runtime would require rewriting every `async fn` in the codebase and all
dependencies that use Tokio types (channels, timers, I/O). The cost/benefit
ratio is extremely unfavorable.

### 6.5 SIMD Without Fallback

Any SIMD optimization must have a scalar fallback. The codebase targets ARM64
(current server is `aarch64`) and x86-64 (CI). WASM targets need WASM SIMD
or scalar. Never ship a binary that panics on unsupported SIMD.

### 6.6 Optimizing Cold Paths

The boot sequence (`boot.rs`, 2820 LOC) runs once. Do not optimize it unless
startup time exceeds 2 seconds. Similarly, `save_to_file`/`load_from_file` for
HNSW are cold paths (checkpoint only).

### 6.7 Breaking the Feature Gate Contract

Many optimizations only apply when `ecc` is enabled. Never move `ecc`-gated
code out of its feature gate. If an optimization requires a dependency (e.g.,
`bumpalo` for arena allocation), it must be behind the same feature gate.

---

## 7. Benchmark Infrastructure Requirements

Before any optimization work begins, we need:

1. **Criterion benchmark suite** in `benches/`:
   - `bench_cosine_similarity` -- scalar vs auto-vectorized vs SIMD
   - `bench_hnsw_insert_10k` -- insert 10K vectors with upsert
   - `bench_hnsw_search_10k` -- search in 10K-entry store
   - `bench_democritus_tick` -- full tick with N impulses and M existing entries
   - `bench_ipc_send` -- message creation + serialization + publish
   - `bench_chain_append` -- exochain event append with signing

2. **Golden output fixtures** in `tests/fixtures/`:
   - Known HNSW search results for fixed embeddings
   - DEMOCRITUS tick results for fixed impulse sequences
   - Chain checkpoint for known event sequence

3. **CI integration**: Add benchmark step to `scripts/build.sh gate` that
   compares against stored baselines (warn-only, not blocking)

---

## 8. Estimated Impact Summary

| Version | Total Items | Estimated Speedup (Hot Path) | Memory Impact | Risk |
|---------|-------------|------------------------------|---------------|------|
| 0.1.x | 10 | 2-4x tick throughput | -10-20% (fewer clones) | Very Low |
| 0.2 | 10 | 5-10x search, 2-3x tick | -20-30% (binary persistence) | Low |
| 0.3 | 7 | 2-5x IPC, safe concurrency | -30-50% (arena, zero-copy) | Medium |
| 1.0+ | 4 | 4-8x cosine sim (SIMD) | -50-75% (PQ) | Medium |

### Key Metrics to Track

| Metric | Current (Estimated) | 0.1.x Target | 0.2 Target | 0.3 Target |
|--------|-------------------|--------------|------------|------------|
| HNSW insert (10K entries) | O(n) per insert | O(1) per insert | O(1) + deferred rebuild | O(1) + incremental |
| HNSW search (10K, top-5) | ~2ms + rebuild | ~2ms (no rebuild) | <1ms (batched, RwLock) | <0.5ms (SIMD) |
| Tick latency (64 impulses) | ~5-15ms (est) | ~3-8ms | ~1-3ms | <1ms |
| IPC message send | ~50-100us (UUID + JSON) | ~10-30us | ~10us (binary) | <5us (zero-copy) |
| Native binary opt-level | "z" (size) | 3 (speed) | 3 (speed) | 3 (speed) |

---

## 9. Panel Consensus Notes

**Performance Engineer** (chair): The biggest wins are B7 (release profile), B1
(HNSW upsert), and O3 (batch search). These three alone could double tick
throughput. Everything else is incremental.

**System Performance Remediator**: The build system is in good shape with
`scripts/build.sh`. The missing piece is split release profiles (trivial fix)
and proper benchmark infrastructure. CI compile times should be tracked.

**FrankenSearch Specialist**: Hybrid search (BM25 + HNSW) is not needed at v0.1
scale. Revisit at v0.2 if users report "I know the exact filename but search
misses it." The keyword index would cost ~2x memory but solve exact-match
problems that pure semantic search cannot.

**AgentDB Optimizer**: The HNSW implementation is sound but has three critical
O(n) traps: retain-based upsert, full rebuild on dirty, and metadata cloning.
Fix all three before tagging v0.1. Quantization and product quantization are
v0.3+ concerns.

**GDB Debugging Specialist**: The 42 `Mutex` instances are a latent deadlock
risk. Define a lock ordering document before v0.3. For v0.1, the single-lock-
per-service pattern is safe because no hot path holds two locks simultaneously.
But the DEMOCRITUS tick loop (ImpulseQueue lock -> HnswService lock -> CausalGraph
DashMap) is the riskiest path.

**Asupersync Analyst**: Tokio is the right choice for this project. The cancel-
correctness risk is in `select!` branches within the mesh networking code.
Recommend a focused audit of `mesh_runtime.rs` and `mesh_heartbeat.rs` for v0.3.
Do not replace the runtime.

---

*Document generated by Sprint 11 Symposium Track 9 panel.*
*Methodology: extreme-software-optimization (profile first, prove unchanged, one change at a time, Score >= 2.0)*
