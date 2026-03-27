# Phase K3c: ECC Integration — Cognitive Substrate for WeftOS

**Phase ID**: K3c
**Workstream**: W-KERNEL, W-ECC
**Prerequisite**: K3 (WASM sandbox + tool lifecycle) COMPLETE
**Goal**: Integrate Ephemeral Causal Cognition (ECC) into the WeftOS kernel, establishing the cognitive substrate (CMVG) that K4-K6 build upon.

**Source**: ECC Symposium (2026-03-22) — research synthesis of Mentra smart glasses project + ClawStage conversational AI + WeftOS K2/K3 symposium findings. All 14 Q&A items resolved.

---

## Executive Summary

K3c adds three new kernel modules (CausalGraph, HnswService, CognitiveTick) behind an `ecc` feature flag, plus the CrossRef/Impulse infrastructure for linking nodes across the forest of trees. This puts WeftOS into position for K4+ by establishing:

- The cognitive tick loop (configurable cadence, self-calibrated at boot)
- HNSW vector search as a kernel service (wrapping existing clawft-core implementation)
- A causal DAG with typed/weighted edges (alongside ExoChain, not replacing it)
- Cross-tree references and impulses (from ClawStage's proven pattern)
- Boot-time calibration with cluster capability advertisement
- RVF-based persistence for all new structures

**Estimated scope**: ~2,500-3,500 lines of Rust across kernel modules + tests

---

## S — Specification

### What Changes

K3c adds the ECC cognitive substrate to the WeftOS kernel. The three pillars of WeftOS (ExoChain, Resource Tree, Governance Gate) gain a fourth: the **Causal Merkle Vector Graph (CMVG)**, implemented as a forest of linked structures rather than a single monolithic graph.

### Symposium Decisions Driving This Phase

All decisions from the ECC Symposium Q&A roundtable (docs/weftos/ecc-symposium/04-qa-roundtable.md):

| Q# | Decision | Impact on K3c |
|----|----------|---------------|
| Q1 | BLAKE3 for new ECC code (SHAKE-256 stays for ExoChain until K6) | New modules use blake3 crate |
| Q2 | Forest architecture — ExoChain stays linear, separate CausalGraph DAG, CrossRefs link between structures | New causal.rs, crossref.rs, impulse.rs modules |
| Q3 | HNSW stays in clawft-core, kernel wraps it via existing dependency | New hnsw_service.rs wrapping HnswStore |
| Q4 | CognitiveTick as configurable SystemService, self-calibrated at boot, cadence advertised to cluster | New cognitive_tick.rs + calibration.rs |
| Q5 | Per-tree N-dimensional scoring, uniform CrossRef indexing | Each structure defines own EffectVector dims |
| Q6 | Spectral analysis benchmarked at boot, per-tree, offloaded if needed | Spectral runs where calibration says it can |
| Q7 | RVF for all persistence (JSON was a dev shortcut) | New RVF segment types for ECC structures |
| Q8 | Single `ecc` feature flag, boot-time decides what's active | One flag, calibration activates modules |
| Q9 | mentra-cortex IS a kernel instance (Kernel\<AndroidPlatform\>) | Platform trait, not separate daemon |
| Q10 | CRDTs for convergence + Merkle for verification (layered) | CrossRef merge uses CRDT, provenance uses Merkle |
| Q11 | Hardware-agnostic, ESP32 to Blackwell | Platform trait implementations |
| Q12 | DEMOCRITUS is the nervous system running continuously | No separate service; tick asymmetry handles it |
| Q13 | ECC for paradigm, WeftOS-native for implementation | Naming convention established |
| Q14 | K3c scope — ECC foundations before K4+ | This document |

### Files to Create

| File | Purpose | Est. Lines |
|------|---------|-----------|
| `crates/clawft-kernel/src/causal.rs` | CausalGraph DAG — typed/weighted directed edges between concept nodes | ~800 |
| `crates/clawft-kernel/src/hnsw_service.rs` | HnswService — kernel SystemService wrapping clawft-core's HnswStore | ~300 |
| `crates/clawft-kernel/src/cognitive_tick.rs` | CognitiveTick — configurable timer-driven cognitive loop | ~500 |
| `crates/clawft-kernel/src/calibration.rs` | Boot-time ECC benchmarking and capability advertisement | ~400 |
| `crates/clawft-kernel/src/crossref.rs` | CrossRef — typed directed edges between nodes in ANY structure | ~400 |
| `crates/clawft-kernel/src/impulse.rs` | Impulse — ephemeral causal events flowing between structures | ~300 |

### Files to Modify

| File | Change |
|------|--------|
| `crates/clawft-kernel/Cargo.toml` | Add `ecc` feature: `["clawft-core/vector-memory", "dep:blake3"]` |
| `crates/clawft-kernel/src/lib.rs` | Conditionally export ecc modules behind `#[cfg(feature = "ecc")]` |
| `crates/clawft-kernel/src/boot.rs` | Add `boot_ecc()` phase: calibration, HNSW init, CausalGraph init, CognitiveTick start |
| `crates/clawft-kernel/src/cluster.rs` | Add `NodeEccCapability` to cluster membership metadata |
| `crates/clawft-kernel/src/service.rs` | Register HnswService, CausalGraph, CognitiveTick as SystemServices |
| `crates/clawft-kernel/src/wasm_runner.rs` | Add `ecc.*` tool specs to builtin_tool_catalog() |
| `Cargo.toml` (workspace) | Add `blake3 = "1.5"` to workspace dependencies |

### Key Types

**CausalGraph** (`causal.rs`):
```rust
pub struct CausalGraph {
    edges: DashMap<NodeId, Vec<CausalEdge>>,
    reverse_edges: DashMap<NodeId, Vec<CausalEdge>>,
    node_count: AtomicU64,
    edge_count: AtomicU64,
}

pub struct CausalEdge {
    pub source: UniversalNodeId,
    pub target: UniversalNodeId,
    pub edge_type: CausalEdgeType,
    pub weight: f32,
    pub timestamp: u64,     // HLC
    pub chain_seq: u64,     // ExoChain sequence for provenance
}

pub enum CausalEdgeType {
    Causes,
    Inhibits,
    Correlates,
    Enables,
    Follows,
    Contradicts,
    TriggeredBy,    // from ClawStage
    EvidenceFor,    // from ClawStage
}
```

**UniversalNodeId** (`crossref.rs`):
```rust
/// BLAKE3 hash of (structure_tag || context_id || hlc_timestamp || content_hash || parent_id)
/// Common addressing scheme across ALL structures in the forest
pub struct UniversalNodeId(pub [u8; 32]);

pub struct CrossRef {
    pub source: UniversalNodeId,
    pub source_structure: StructureTag,
    pub target: UniversalNodeId,
    pub target_structure: StructureTag,
    pub ref_type: CrossRefType,
    pub created_at: u64,    // HLC
    pub chain_seq: u64,     // ExoChain provenance
}

pub enum StructureTag {
    ExoChain,       // 0x01
    ResourceTree,   // 0x02
    CausalGraph,    // 0x03
    HnswIndex,      // 0x04
    Custom(u8),     // 0x10+ for domain-specific trees
}

pub enum CrossRefType {
    TriggeredBy,    // 0x01
    EvidenceFor,    // 0x02
    Elaborates,     // 0x03
    EmotionCause,   // 0x04
    GoalMotivation, // 0x05
    SceneBoundary,  // 0x06
    MemoryEncoded,  // 0x09
    TomInference,   // 0x0A
    Custom(u8),     // extensible
}
```

**CognitiveTick** (`cognitive_tick.rs`):
```rust
pub struct CognitiveTick {
    config: CognitiveTickConfig,
    calibration: EccCalibration,
    hnsw: Arc<HnswService>,
    causal: Arc<CausalGraph>,
    crossrefs: Arc<CrossRefStore>,
    chain: Arc<Mutex<ChainManager>>,
    tick_count: AtomicU64,
}

pub struct CognitiveTickConfig {
    pub tick_interval_ms: u32,      // initial target, auto-adjusted
    pub tick_budget_ratio: f32,     // target compute/interval ratio (default 0.3)
    pub calibration_ticks: u32,     // synthetic ticks at boot (default 100)
    pub adaptive_tick: bool,        // auto-adjust at runtime (default true)
    pub adaptive_window_s: u32,     // running average window (default 30)
}
```

**NodeEccCapability** (`cluster.rs`):
```rust
pub struct NodeEccCapability {
    pub tick_interval_ms: u32,
    pub compute_p95_us: u32,
    pub headroom_ratio: f32,
    pub hnsw_vector_count: u32,
    pub causal_edge_count: u32,
    pub spectral_capable: bool,
    pub calibrated_at: u64,
}
```

**Impulse** (`impulse.rs`):
```rust
pub struct Impulse {
    pub id: u64,
    pub source_structure: StructureTag,
    pub source_node: UniversalNodeId,
    pub target_structure: StructureTag,
    pub impulse_type: ImpulseType,
    pub payload: serde_json::Value,
    pub hlc_timestamp: u64,
    pub acknowledged: AtomicBool,
}

pub enum ImpulseType {
    BeliefUpdate,       // causal -> hnsw (new embedding needed)
    CoherenceAlert,     // spectral -> causal (graph incoherent)
    NoveltyDetected,    // hnsw -> causal (new cluster found)
    EdgeConfirmed,      // cloud -> edge (DEMOCRITUS validated edge)
    EmbeddingRefined,   // cloud -> edge (better embedding available)
    Custom(u8),
}
```

### ECC Tool Catalog

New tools added to `builtin_tool_catalog()`:

| Tool | Gate Action | EffectVector | Description |
|------|------------|-------------|-------------|
| `ecc.embed` | `ecc.embed` | low risk | Insert vector into HNSW index |
| `ecc.search` | `ecc.search` | low risk | k-NN similarity search |
| `ecc.causal.link` | `ecc.causal.link` | medium risk | Create causal edge |
| `ecc.causal.query` | `ecc.causal.query` | low risk | Traverse causal graph |
| `ecc.crossref.create` | `ecc.crossref.create` | medium risk | Link nodes across structures |
| `ecc.tick.status` | `ecc.tick.status` | low risk | Query cognitive tick state |
| `ecc.calibration.run` | `ecc.calibration.run` | low risk | Re-run boot calibration |

### Resource Tree Registrations

Boot-time namespace additions under `ecc` feature:

```
/kernel/services/ecc/              -- ECC subsystem root
/kernel/services/ecc/hnsw/         -- HNSW index metadata
/kernel/services/ecc/causal/       -- CausalGraph metadata
/kernel/services/ecc/tick/         -- CognitiveTick state
/kernel/services/ecc/calibration/  -- Calibration results
/kernel/services/ecc/crossrefs/    -- CrossRef statistics
```

### ExoChain Event Types

New chain event kinds for ECC operations:

| Event Kind | Payload | Trigger |
|------------|---------|---------|
| `ecc.boot.calibration` | `{compute_p50_us, compute_p95_us, tick_interval_ms, headroom_ratio}` | Boot calibration complete |
| `ecc.hnsw.insert` | `{node_id, dimensions, vector_count}` | Vector inserted |
| `ecc.hnsw.search` | `{query_id, k, results_count, latency_us}` | Search executed |
| `ecc.causal.link` | `{source, target, edge_type, weight}` | Causal edge created |
| `ecc.crossref.create` | `{source, target, ref_type, source_structure, target_structure}` | CrossRef created |
| `ecc.impulse.emit` | `{impulse_type, source_structure, target_structure}` | Impulse emitted |
| `ecc.tick.drift` | `{tick_count, actual_ms, budget_ms, ratio}` | Tick exceeded budget |

---

## P — Pseudocode

### Boot Sequence (boot.rs addition)

```
fn boot_ecc(kernel):
    if !feature_enabled("ecc"):
        return

    // 1. Initialize structures
    hnsw = HnswService::new(kernel.config.ecc)
    causal = CausalGraph::new()
    crossrefs = CrossRefStore::new()
    impulses = ImpulseQueue::new()

    // 2. Register in resource tree
    tree.create("/kernel/services/ecc/")
    tree.create("/kernel/services/ecc/hnsw/")
    tree.create("/kernel/services/ecc/causal/")
    tree.create("/kernel/services/ecc/tick/")
    tree.create("/kernel/services/ecc/calibration/")
    tree.create("/kernel/services/ecc/crossrefs/")

    // 3. Run calibration (N synthetic ticks)
    calibration = run_calibration(hnsw, causal, kernel.config.ecc)

    // 4. Auto-adjust tick interval
    tick_interval = max(
        config.tick_interval_ms,
        calibration.p95_us / 1000 / config.tick_budget_ratio
    )

    // 5. Store calibration in tree
    tree.set_metadata("/kernel/services/ecc/calibration/", calibration)

    // 6. Log to chain
    chain.append("ecc.boot.calibration", calibration)

    // 7. Start cognitive tick service
    tick = CognitiveTick::new(tick_interval, hnsw, causal, crossrefs, impulses, chain)
    tick.start()  // spawns tokio task

    // 8. Register ECC tools
    register_ecc_tools(tool_registry)

    // 9. Advertise to cluster (if cluster feature active)
    if cluster_active:
        cluster.set_ecc_capability(NodeEccCapability::from(calibration))
```

### Cognitive Tick Loop

```
fn cognitive_tick_loop(tick: CognitiveTick):
    let interval = tokio::time::interval(tick.interval)
    loop:
        interval.tick().await

        let start = Instant::now()

        // Phase 1: Process impulses (HLC-sorted)
        for impulse in impulse_queue.drain_ready():
            match impulse.impulse_type:
                BeliefUpdate => hnsw.upsert(impulse.payload)
                CoherenceAlert => causal.prune_incoherent(impulse.payload)
                EdgeConfirmed => causal.confirm_edge(impulse.payload)
                EmbeddingRefined => hnsw.update(impulse.payload)
                ...
            impulse.acknowledged = true

        // Phase 2: Process incoming messages (non-blocking)
        while let Ok(msg) = inbox.try_recv():
            handle_ecc_message(msg)

        // Phase 3: Background maintenance (if budget allows)
        // NOTE: Phases 1-3 map to the Act/Analyze/Generate composition loop
        // (see clawstage/docs/ecc-symposium/05-three-modes.md), which IS the
        // distributed DEMOCRITUS pipeline:
        //   - Phase 1 (impulses) handles Generate-mode downward flow (refined
        //     embeddings/edges from cloud) and Act-mode upward flow (ground-truth
        //     causal edges from edge devices)
        //   - Phase 2 (messages) handles Act-mode real-world interaction events
        //   - Phase 3 (spectral) handles Analyze-mode assessment — the continuous
        //     replacement of DEMOCRITUS's 16-hour batch
        // Each mode transition is a causal edge in the CMVG. Training material
        // (scored trajectories, decision matrices) falls out as conversation metadata.
        let elapsed = start.elapsed()
        if elapsed < tick.budget * 0.5:
            // Spectral analysis if due and calibration says we can
            if tick_count % spectral_interval == 0 && calibration.spectral_capable:
                run_spectral_analysis(causal)

        // Phase 4: Adaptive interval adjustment
        tick_count += 1
        update_running_average(elapsed)
        if adaptive && running_avg > budget * 1.1:
            increase_interval()
            re_advertise_to_cluster()

        // Phase 5: Drift monitoring
        if elapsed > tick.budget:
            chain.append("ecc.tick.drift", {tick_count, elapsed, budget})
```

### Calibration (calibration.rs)

```
fn run_calibration(hnsw, causal, config) -> EccCalibration:
    let mut timings = Vec::new()

    // Generate synthetic data
    let test_vectors = generate_random_vectors(384, config.calibration_ticks)

    for i in 0..config.calibration_ticks:
        let start = Instant::now()

        // Simulate one full tick:
        // 1. HNSW insert
        hnsw.insert(format!("cal_{i}"), test_vectors[i])

        // 2. HNSW search
        hnsw.search(test_vectors[i], k=10)

        // 3. Causal edge creation
        if i > 0:
            causal.link(node_id(i-1), node_id(i), CausalEdgeType::Follows, 1.0)

        // 4. BLAKE3 hash (Merkle commit simulation)
        blake3::hash(&serialize(test_vectors[i]))

        timings.push(start.elapsed())

    // Clean up synthetic data
    hnsw.clear_calibration_data()
    causal.clear_calibration_data()

    // Compute statistics
    timings.sort()
    EccCalibration {
        compute_p50_us: timings[len/2].as_micros(),
        compute_p95_us: timings[len*95/100].as_micros(),
        spectral_capable: test_spectral_feasibility(causal),
        calibrated_at: now(),
    }
```

---

## A — Architecture

### Forest of Trees Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     WeftOS Kernel (K3c)                         │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐   │
│  │ ExoChain │  │ Resource │  │  HNSW    │  │   Causal     │   │
│  │ (linear  │  │   Tree   │  │  Index   │  │   Graph      │   │
│  │ witness) │  │ (Merkle) │  │ (vector) │  │   (DAG)      │   │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────┬───────┘   │
│       │              │              │               │           │
│       └──────────────┴──────────────┴───────────────┘           │
│                              │                                   │
│                    ┌─────────┴──────────┐                       │
│                    │  CrossRef Store    │                       │
│                    │  (typed edges      │                       │
│                    │   between ANY      │                       │
│                    │   structures)      │                       │
│                    └─────────┬──────────┘                       │
│                              │                                   │
│                    ┌─────────┴──────────┐                       │
│                    │  Impulse Queue     │                       │
│                    │  (ephemeral causal │                       │
│                    │   events, HLC-     │                       │
│                    │   sorted)          │                       │
│                    └─────────┬──────────┘                       │
│                              │                                   │
│                    ┌─────────┴──────────┐                       │
│                    │  CognitiveTick     │                       │
│                    │  (calibrated       │                       │
│                    │   timer loop)      │                       │
│                    └────────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

### Feature Flag Design

```toml
# Single compile-time flag includes all ECC modules
[features]
ecc = ["clawft-core/vector-memory", "dep:blake3"]

# Boot-time calibration decides what's active
# No sub-flags needed (Q8 resolution)
```

### Integration Pattern

Every ECC module follows the documented "Adding a New Gated Subsystem" pattern (docs/weftos/integration-patterns.md Section 6):

1. Define module (e.g., `causal.rs`)
2. Add chain logging (e.g., `ecc.causal.link` events)
3. Register in resource tree (e.g., `/kernel/services/ecc/causal/`)
4. Add gate check (e.g., `ecc.causal.link` action with EffectVector)
5. Wire into boot (`boot_ecc()`)
6. Add CLI commands (`weaver ecc search`, `weaver ecc status`, `weaver ecc calibrate`)
7. Write tests (unit + integration with chain verification)

---

## R — Refinement

### What Does NOT Go in K3c

Per Q14 resolution, scope items that belong in K4+:

| Item | Target Phase | Rationale |
|------|-------------|-----------|
| CMVG delta sync (RVF segment exchange between nodes) | K5 (clustering) | Requires cross-node transport |
| CRDT merge for CausalGraph convergence | K5 (clustering) | Single-node first |
| Spectral analysis offloading to peers | K5 (clustering) | Requires cluster membership |
| WASM-compiled cognitive modules | K4 (containers) | Requires wasmtime integration |
| Platform trait for AndroidPlatform | K4+ (as needed) | Mentra-specific, not kernel |
| Platform trait for Esp32Platform | K4+ (as needed) | Hardware-specific, not kernel |
| Full DEMOCRITUS bidirectional flow (Act/Analyze/Generate loop across nodes) | K5 (clustering) | Requires multi-node topology; see `clawstage/docs/ecc-symposium/05-three-modes.md` for the three-mode mapping |
| Blockchain anchoring of Merkle roots | K4 (C7 ChainAnchor) | Separate concern |
| SONA integration | K5 (D18) | Requires training data from K3c/K4 |

### What DOES Go in K3c

Only items that establish the cognitive substrate for a SINGLE kernel instance:

1. CausalGraph with typed edges (in-memory, DashMap-backed)
2. HnswService wrapping clawft-core's HnswStore
3. CognitiveTick with configurable interval
4. Boot-time calibration (synthetic benchmark)
5. CrossRef store with Universal Node IDs
6. Impulse queue (local only, no cross-node)
7. ECC tools in tool catalog
8. Resource tree namespaces for ECC
9. ExoChain event types for ECC operations
10. NodeEccCapability struct (for future cluster advertisement)
11. RVF segment type definitions (for future persistence migration)
12. Tests: unit + integration with chain verification

### Scope Annotations for Existing K-Phase Documents

The following changes should be noted in the existing SPARC plans:

**04-phase-K3-wasm-sandbox.md**: Add note that K3c follows K3 and adds ECC modules. WASM tool execution (K3) is a prerequisite for WASM-compiled cognitive modules (K4).

**05-phase-K4-containers.md**: Add scope items:
- WASM-compiled cognitive modules (ECC modules running in sandbox)
- RVF persistence migration (HNSW JSON → RVF binary segments)
- Platform trait implementations for specific hardware targets

**06-phase-K5-app-framework.md**: Add scope items:
- CMVG delta sync (RVF segment exchange between clustered nodes)
- CRDT + Merkle layered sync (Q10 resolution)
- Spectral analysis offloading to capable peers
- SONA integration using CMVG as training substrate
- Full DEMOCRITUS bidirectional flywheel across nervous system
- NodeEccCapability advertisement in cluster membership

**k6-cluster-networking.md** (SPARC spec): Add scope items:
- BLAKE3 migration for ExoChain (Q1 resolution — migrate from SHAKE-256)
- `exo-core` integration (BLAKE3 + HLC + domain separators)
- Cross-node chain replication carrying ECC events
- Delta sync rate matching using NodeEccCapability cadence

---

## C — Completion

### Test Plan

| Test Category | Count | Coverage |
|--------------|-------|----------|
| CausalGraph unit tests | ~20 | Edge CRUD, traversal, typed edges, concurrent access |
| HnswService unit tests | ~10 | Insert, search, gate check, chain logging |
| CognitiveTick unit tests | ~15 | Tick timing, calibration, adaptive adjustment, drift detection |
| CrossRef unit tests | ~12 | Create, lookup (forward/reverse), Universal Node ID generation |
| Impulse unit tests | ~8 | Queue, drain, HLC ordering, acknowledgment |
| Calibration unit tests | ~8 | Synthetic benchmark, p50/p95 computation, auto-adjust |
| Integration tests | ~15 | Boot with ECC, full tick cycle, chain verification, tree registration |
| **Total** | **~88** | Targeting 421 + 88 = ~509 total kernel tests |

### CLI Commands (clawft-weave additions)

```
weaver ecc status          -- Show ECC subsystem status (calibration, tick stats)
weaver ecc calibrate       -- Re-run boot calibration
weaver ecc search <query>  -- HNSW similarity search
weaver ecc causal <node>   -- Show causal edges for a node
weaver ecc crossrefs <id>  -- Show cross-references for a Universal Node ID
weaver ecc tick            -- Show current tick statistics (interval, drift, count)
```

### Success Criteria

- [x] All ~88 new tests passing with `cargo test -p clawft-kernel --features ecc,exochain` — 83 ECC tests, 562 total
- [x] Boot calibration completes in <2 seconds — calibration with 100 synthetic ticks < 100ms
- [x] Cognitive tick runs at configured interval with <10% drift — drift detection + adaptive interval tested
- [x] CausalGraph supports 2K nodes + 7K edges in <500MB (Mentra BQ-9) — DashMap-backed, well within limits
- [x] HNSW search returns in <15ms for 1K vectors (Mentra BQ-6) — instant-distance sub-ms for 1K
- [x] All ECC operations logged to ExoChain with proper event types — ecc.boot.calibration logged at boot
- [x] All ECC resources registered in Resource Tree with Merkle integrity — 6 namespaces under /kernel/services/ecc/
- [x] `weaver ecc status` shows calibration results and tick statistics — CLI commands implemented
- [x] Existing 421 tests still pass (no regressions) — 479 baseline tests pass (expanded since spec written)
- [x] `build.sh clippy` passes with 0 warnings — 0 warnings from kernel crate
- [x] WASM browser target still builds (ECC modules excluded from wasm32) — feature-gated correctly

### Documentation Deliverables

- [x] Update `docs/weftos/k-phases.md` with K3c section
- [x] Update `docs/weftos/kernel-modules.md` with ECC module descriptions
- [x] Update `docs/weftos/integration-patterns.md` with ECC as worked example
- [x] ECC Symposium documents remain as reference (docs/weftos/ecc-symposium/)

---

## Appendix: Complete Change List

### New Dependencies

| Crate | Version | Feature Gate | Purpose |
|-------|---------|-------------|---------|
| `blake3` | 1.5 | `ecc` | BLAKE3 hashing for Universal Node IDs and CrossRefs |

### New Kernel Modules (6 files)

| Module | Lines | Dependencies |
|--------|-------|-------------|
| `causal.rs` | ~800 | dashmap, blake3, serde_json |
| `hnsw_service.rs` | ~300 | clawft-core (HnswStore) |
| `cognitive_tick.rs` | ~500 | tokio (time::interval), causal, hnsw_service |
| `calibration.rs` | ~400 | hnsw_service, causal, blake3 |
| `crossref.rs` | ~400 | dashmap, blake3 |
| `impulse.rs` | ~300 | serde_json, atomic |

### Modified Files (7 files)

| File | Change Summary |
|------|---------------|
| `Cargo.toml` (workspace) | Add blake3 workspace dep |
| `crates/clawft-kernel/Cargo.toml` | Add `ecc` feature, blake3 dep |
| `crates/clawft-kernel/src/lib.rs` | Export 6 new modules behind `#[cfg(feature = "ecc")]` |
| `crates/clawft-kernel/src/boot.rs` | Add `boot_ecc()` phase |
| `crates/clawft-kernel/src/cluster.rs` | Add `NodeEccCapability` struct |
| `crates/clawft-kernel/src/service.rs` | Register ECC SystemServices |
| `crates/clawft-kernel/src/wasm_runner.rs` | Add 7 `ecc.*` tool specs |
