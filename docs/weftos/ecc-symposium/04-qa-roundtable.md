# ECC Symposium: Q&A Roundtable

**All questions from the research panels requiring resolution before final documentation and implementation**

---

## Architecture Questions

### Q1: Hash Algorithm Unification (SHAKE-256 vs BLAKE3) -- RESOLVED

**Source**: Crate Analysis panel + development notes investigation

**Finding**: SHAKE-256 is a **temporary K0-K5 implementation choice**, not an architectural commitment. The original SPARC architecture documents (doc 08, 13, 15) specify BLAKE3 throughout via `exo-core`:
- Content-addressed filesystem: BLAKE3 (doc 08 line 96)
- ResourceId: `Blake3Hash` (doc 13 line 164)
- Merkle roots: `Blake3Hasher` (doc 13 line 542)
- ExoChain substrate: "BLAKE3 hashing, Ed25519 signing, HLC" (doc 13 line 68)
- DIDs: `blake3(pubkey)` (doc 08 line 174)

**Why SHAKE-256 was chosen (Decision 17, K0)**:
1. K0's original `sha2::Sha256` had a real vulnerability: payload was NOT included in `compute_event_hash()`, allowing payload swaps without breaking the chain
2. `rvf-crypto` was already a dependency and exposed `shake256_256` as its canonical hash
3. Decision 19 then unified Resource Tree to SHAKE-256 for "single-hash-family invariant"

**Migration path was always planned**: The gap analysis (ruvector-gap-analysis.md line 127) explicitly states: "K0 uses SHA-256 ChainManager with custom ChainEvent structs, not exochain's BLAKE3 + HLC + DAG. This is intentional for K0 simplicity. Migration path to `exo-dag::DagStore` exists for K6." The `exo-core` crate (BLAKE3 + HLC + domain separators) is scheduled for K6.1.

**Resolution**: Option (a) -- use BLAKE3 for new ECC code (aligning with the intended architecture), keep SHAKE-256 for existing ExoChain/tree until K6 migration brings everything to BLAKE3 via `exo-core`. This preserves the single-hash-family invariant within each subsystem while converging on the target.

**References**:
- `.planning/development_notes/weftos/phase-K0/decisions.md` (Decisions 17, 19)
- `.planning/development_notes/weftos/phase-K0/ruvector-gap-analysis.md` (line 127)
- `.planning/sparc/weftos/08-ephemeral-os-architecture.md` (lines 96, 174, 964, 989)
- `.planning/sparc/weftos/13-exo-resource-tree.md` (lines 68, 156-175, 539-547)

---

### Q2: ExoChain DAG Extension vs Separate CausalGraph -- RESOLVED

**Source**: WeftOS Core panel + K3 panel + ClawStage architecture investigation

**Original options**:
a) Extend ExoChain with `parent_hashes: Vec<[u8; 32]>` to support multiple parents (true DAG)
b) Keep ExoChain linear and build a separate CausalGraph module

**Finding from ClawStage**: The ClawStage conversational AI engine already implements the answer -- a **polyglot tree ensemble** (forest of 5 specialized trees) bound together by cross-engine references, impulses, a shared HNSW embedding space, and a linear witness chain. This is the origin of the CMVG concept.

ClawStage's architecture explicitly rejects a single monolithic DAG in favor of multiple domain-specific structures linked together. Rationale: separation of concerns (different domains need different data structures), independent evolution (engines can be replaced without touching others), targeted optimization (different access patterns), and composability (new engines added without modifying existing ones).

**Resolution: Option C (new) -- Forest Architecture**

ExoChain stays linear (it IS the witness chain -- temporal provenance). Multiple domain-specific graph/tree structures sit alongside it. A cross-reference system links nodes between ANY structures.

```
ExoChain (linear witness chain -- temporal provenance, append-only)
    |
    |-- references nodes in -->  CausalGraph (DAG -- typed causal edges)
    |-- references nodes in -->  ResourceTree (tree -- namespace/permissions)
    |-- references nodes in -->  HNSW Index (flat -- vector embeddings)
    |-- references nodes in -->  [future domain-specific trees]

CrossRefs (typed directed edges between nodes in ANY structure)
Impulses (ephemeral causal events flowing between structures)
```

**Key design elements (from ClawStage)**:
- **Universal Node ID**: BLAKE3 hash of `(structure_tag || context_id || hlc_timestamp || content_hash || parent_id)` -- common addressing scheme across all structures
- **CrossRef entries**: 72 bytes, typed (10+ types: "triggered by", "evidence for", "elaborates", "emotion cause", etc.), bidirectionally indexed
- **Impulses**: Ephemeral causal events processed in HLC order. Processing one may produce new cross-refs and new impulses for the next tick
- **Shared HNSW**: All structures embed into the same vector space with structure-tag filtering for cross-structure similarity search

**Why this is better than A or B**:
- ExoChain's proven linear integrity model is preserved (no breaking change)
- Domain-specific structures use data structures appropriate to their domain (trees, DAGs, graphs, flat indexes)
- New structure types can be added without modifying existing ones
- Cross-references provide typed, auditable links between any pair of nodes in any structures
- The witness chain (ExoChain) provides the total ordering that binds everything together

**References**:
- `clawstage/docs/engines/01-overview.md` -- Polyglot tree ensemble architecture
- `clawstage/docs/engines/06-RVF-implementation.md` -- CrossRef and Impulse data structures, shared HNSW, Universal Node ID
- `clawstage/docs/engines/02-DCTE.md` -- Per-actor private trees, merge engine
- `mentra/docs/CAUSAL_MERKLE_VECTOR_GRAPHS.md` -- CMVG as general primitive extracted from ClawStage

---

### Q3: HNSW Location -- clawft-core vs clawft-kernel -- RESOLVED

**Source**: Crate Analysis panel (corrected after code verification)

**Panel error corrected**: The panel stated "clawft-kernel currently does NOT depend on clawft-core." This is wrong. Verified in `clawft-kernel/Cargo.toml`: the kernel already depends on clawft-core and imports `AppContext`, `MessageBus` from it (`boot.rs` lines 15-16, `ipc.rs` line 13).

**Resolution: Option B -- keep HNSW in clawft-core, wrap in kernel service**

Since the kernel already depends on clawft-core, there is no new coupling cost. A separate `clawft-hnsw` crate would be premature extraction -- added maintenance overhead (Cargo.toml, feature flags, versioning) for no architectural benefit.

The kernel adds an `ecc` feature that activates clawft-core's `vector-memory` feature:
```toml
# clawft-kernel/Cargo.toml
[features]
ecc = ["clawft-core/vector-memory"]
```

The kernel's HNSW service module (`hnsw.rs`) wraps `clawft_core::embeddings::HnswStore` with chain logging, tree registration, and gate checks per the "Adding a New Gated Subsystem" pattern.

**Future consideration**: If a standalone edge daemon (e.g., mentra-cortex) needs HNSW without pulling in all of clawft-core, extraction to a shared crate would be justified then. Not now.

---

### Q4: Cognitive Tick Relationship to Agent Loop -- RESOLVED

**Source**: WeftOS Core panel + K3 panel + user input on tick interval

**Resolution: Option C -- CognitiveTick as a configurable SystemService running alongside the agent loop**

The tick runs its own loop and sends `KernelMessage` events to the agent loop when cognitive state changes require action. The agent loop remains event-driven for agent orchestration; the cognitive tick is clock-driven for continuous cognition.

**Critical design point: the tick interval is NOT fixed at 50ms.** 50ms was the Mentra-specific budget (Cortex-A53, 60Hz display). The actual compute is 3.6ms -- the interval is about *how often to sample/respond*, not how long compute takes. The interval must be configurable per platform:

| Platform | Suggested Default | Rationale |
|----------|------------------|-----------|
| Server (x86_64) | 10-20ms | Abundant compute, low-latency responses |
| Desktop/Laptop | 20-50ms | Good compute, balanced with other workloads |
| Phone (S25 Ultra) | 30-50ms | Strong NPU, battery-conscious |
| Smart glasses (MT6761) | 50ms | Budget SoC, proven at 3.6ms compute |
| Browser (WASM) | 50-100ms | Limited to `requestAnimationFrame` or `setInterval` |
| ESP32 | 100-200ms | Minimal compute |

**Self-calibration at boot**: The tick interval should NOT be purely config-driven. At kernel boot, the CognitiveTick service runs a calibration phase:

1. **Benchmark phase** (boot time, ~1-2 seconds): Execute N synthetic ticks (HNSW insert + search + causal edge + Merkle commit) and measure p50/p95 compute time. This is the automated version of Mentra's BQ-1 through BQ-9 benchmarks.
2. **Derive interval**: Set tick_interval = `max(config.tick_interval_ms, calibrated_p95 * (1.0 / config.tick_budget_ratio))`. If the hardware can't meet the configured interval, the system auto-adjusts upward and logs a warning.
3. **Record as node capability**: Store the calibrated metrics in the Resource Tree at `/kernel/services/ecc/calibration` with metadata: `compute_p50_us`, `compute_p95_us`, `tick_interval_ms`, `headroom_ratio`, `calibrated_at`.
4. **Advertise to cluster**: When joining a cluster, the node's ECC calibration data is included in `ClusterMembership` peer metadata. This allows:
   - **Delta sync rate matching**: A fast node (10ms tick) won't flood a slow node (200ms tick) with deltas faster than it can process them. The sender throttles to the receiver's advertised cadence.
   - **Task routing**: The cluster can route latency-sensitive cognitive tasks to nodes with lower tick intervals.
   - **Heterogeneous swarms**: A glasses node (50ms), phone node (30ms), and server node (10ms) can all participate in the same swarm with the cluster coordinator understanding each node's cognitive throughput.

**Adaptive tick (runtime)**: Beyond boot calibration, the service monitors actual tick durations. If sustained drift exceeds 10% of the calibrated budget (e.g., HNSW index grew and searches got slower), the interval auto-adjusts and re-advertises to peers. This is the `adaptive_tick` config option.

Configuration via `KernelConfig`:
```toml
[ecc]
tick_interval_ms = 50        # Initial target (auto-adjusted upward if hardware can't meet it)
tick_budget_ratio = 0.3      # Target: use 30% of interval for compute, 70% headroom
calibration_ticks = 100      # Number of synthetic ticks at boot for benchmarking
adaptive_tick = true         # Auto-adjust interval based on observed performance
adaptive_window_s = 30       # Window for computing running tick duration average
```

**Cluster integration**: The `ClusterMembership` node metadata (already defined in `cluster.rs`) gains ECC fields:
```rust
pub struct NodeEccCapability {
    pub tick_interval_ms: u32,       // Calibrated tick interval
    pub compute_p95_us: u32,         // p95 compute time in microseconds
    pub headroom_ratio: f32,         // Actual headroom (e.g., 0.76 = 76%)
    pub hnsw_vector_count: u32,      // Current index size
    pub causal_edge_count: u32,      // Current graph size
    pub calibrated_at: u64,          // Timestamp of last calibration
}
```

This makes ECC capability a first-class property of cluster membership, enabling intelligent delta sync and task routing across heterogeneous swarms.

The `CognitiveTick` service uses `tokio::time::interval()` on native, `setInterval`/`requestAnimationFrame` on WASM, respecting the `Platform` trait abstraction that `Kernel<P: Platform>` already provides.

**Message protocol**: CognitiveTick sends standard `KernelMessage` with `MessagePayload::Json` containing tick results (new causal edges, HNSW matches, coherence score). The agent loop handles these like any other message -- gate check, dispatch, chain log.

---

### Q5: EffectVector Dimensionality for ECC Operations -- RESOLVED

**Source**: K2 panel (D20: N-dimensional EffectVector) + user input on per-tree topology

**Resolution: Each tree defines its own scoring dimensions. Cross-tree indexing is what matters.**

The EffectVector does not need to be uniform across all structures. Each tree/structure defines N-dimensional scoring appropriate to its domain:

- **ExoChain (witness)**: sequence, recency, verification_cost
- **CausalGraph**: coherence (Lambda_2), causal_depth, edge_density
- **HNSW Index**: novelty (distance to nearest neighbor), cluster_membership, embedding_confidence
- **ResourceTree**: trust, performance, difficulty, reward, reliability, velocity (existing 6D)
- **Domain-specific trees**: whatever dimensions that domain needs (e.g., EMOT uses VAD -- valence/arousal/dominance)

**What must be uniform**: The **CrossRef indexing system** that enables:
- **Grafting**: Linking nodes between trees (typed cross-references with Universal Node IDs). A CrossRef from an HNSW node to a CausalGraph edge to an ExoChain event works regardless of each structure's internal scoring dimensions.
- **Shaking**: Pruning/detaching subgraphs when they become incoherent or irrelevant. Triggered by scoring thresholds within a tree (e.g., coherence drops below threshold), propagated to linked trees via impulses.

The governance gate evaluates operations using the scoring dimensions of the **structure being modified**, not a global vector. A `causal.link` operation is scored on the CausalGraph's dimensions (coherence, depth); an `hnsw.insert` is scored on the HNSW's dimensions (novelty, confidence). The gate's threshold logic is dimension-count-agnostic -- it computes magnitude against threshold regardless of N.

This matches ClawStage's proven pattern: each engine (DCTE, DSTE, RSTE, EMOT, SCEN) has its own scoring model, but the CrossRef and Impulse systems provide typed edges that span across trees regardless of internal dimensionality.

---

## Implementation Questions

### Q6: Spectral Analysis -- Inline vs Offload -- RESOLVED

**Source**: K3 panel + Mentra research + user input on per-tree calibration

**Resolution: Same pattern as Q4 -- benchmarked at boot, configured per tree, auto-adjusted at runtime.**

Spectral analysis is not a single global operation. Each tree that needs coherence measurement defines its own spectral analysis requirements and runs it where the calibration says it can:

- Each tree registers its spectral analysis budget during boot calibration (alongside the tick calibration from Q4)
- If the calibration shows the local hardware can run eigenvalue decomposition within the tree's background budget, it runs in-process on a background Tokio task
- If not (e.g., browser WASM, ESP32), it offloads via A2A message to a peer node that advertised sufficient capability (using the `NodeEccCapability` from Q4)
- The tree's spectral interval is also self-calibrated: a 1K-node graph might run every 5 seconds; a 100-node graph every 500ms

This is per-tree, not global. The CausalGraph might need Lambda_2 for coherence. The HNSW index might need cluster analysis. A domain-specific tree might not need spectral analysis at all. Each tree decides for itself based on its own calibration data.

No separate architecture decision is needed -- this falls out naturally from the boot-calibration + cluster-advertisement pattern established in Q4.

---

### Q7: CMVG State Persistence Model -- RESOLVED

**Source**: Crate Analysis panel + user input + code verification

**Resolution: RVF for everything. The JSON persistence is a development shortcut, not a design choice.**

Verified in code: `rvf_io.rs` (line 7-10) explicitly states the JSON format is a "local fallback" because "`rvf-runtime` 0.2 API provides full segment I/O, but is tightly coupled to its binary format." The HNSW store's `save()`/`load()` via `serde_json::to_string_pretty` is the same shortcut.

RVF is literally built for this -- it provides:
- 64-byte SIMD-aligned segments with CRC32C/XXH3/SHAKE-256 checksums
- Compression and forward-compatible segment skipping
- Ed25519 signature footers
- Content hash verification via `rvf_wire::validate_segment`
- The wire codec in `clawft-weave/src/rvf_codec.rs` already frames RVF over transport

Each tree/structure defines its own RVF segment types (ClawStage already does this with 22 segment types across 5 engines). New segment types for ECC:
- `VEC_SEG`: HNSW vector entries (already conceptually defined in rvf_io.rs)
- `CAUSAL_EDGE_SEG`: Causal graph edges
- `CROSSREF_SEG`: Cross-tree references (from ClawStage pattern)
- `IMPULSE_SEG`: Ephemeral causal events (from ClawStage pattern)
- `SPECTRAL_SEG`: Spectral analysis checkpoints (Lambda_2, eigenvalues)

The ruvector ecosystem has the tools: `rvf-runtime` for full binary I/O, `rvf-wire` for segment read/write/validate, `rvf-types` for header definitions. The "tightly coupled" API concern from early development may have been resolved in newer versions of rvf-runtime.

**Action**: Migrate HNSW persistence from JSON to RVF binary segments. Build new ECC structures on RVF from the start. This gives uniform persistence, cross-node transfer (RVF segments ARE the delta sync payload), and cryptographic verification for free.

---

### Q8: ECC Feature Flag Scope -- RESOLVED

**Source**: All panels + user input on boot-time activation

**Resolution: One compile-time flag (`ecc`), boot-time calibration decides what's active.**

Compile-time `--features ecc` includes ALL ECC modules (HNSW kernel service, CausalGraph, CognitiveTick, spectral analysis, wire protocol extensions). This is the build-time question: "does this binary have ECC capability?"

Boot-time calibration decides what actually runs. A resource-constrained device might:
- Activate HNSW + CausalGraph + CognitiveTick (calibration passes)
- Skip spectral analysis (calibration shows eigenvalue decomposition exceeds budget, offload to peer)
- Skip wire protocol compression (no zstd needed for single-node operation)

This avoids combinatorial feature flag complexity while still supporting heterogeneous deployments. The boot calibration (Q4) already determines per-module viability. The `NodeEccCapability` struct advertised to the cluster tells peers exactly which modules are active.

If a dependency is truly problematic at compile time (e.g., `nalgebra` adds too much binary size for a tiny WASM target), a sub-flag like `ecc-no-spectral` can exclude it. But this is the exception, not the design principle. Keep one flag, let boot decide.

---

### Q9: Edge Device Integration Path -- RESOLVED

**Source**: Mentra research (hardware analysis) + user architectural direction

**Resolution: Option A -- mentra-cortex IS a WeftOS kernel instance, not a separate daemon.**

`mentra-cortex` is `Kernel<AndroidPlatform>` cross-compiled to `aarch64-linux-android` with `--features ecc`. The phone runs `Kernel<NativePlatform>`. The server runs `Kernel<NativePlatform>`. The browser runs `Kernel<BrowserPlatform>`. They are all peers in the same nervous system, differentiated only by:

1. **Platform trait implementation** — how they access the filesystem, network, timers (already designed for this via `Kernel<P: Platform>`)
2. **Boot-time calibration** — what tick interval, which modules are active (Q4)
3. **Cluster membership capabilities** — advertised via `NodeEccCapability` (Q4)

Each instance:
- Boots the same kernel code
- Runs its own cognitive tick at its calibrated interval
- Maintains its own local forest of trees (HNSW, CausalGraph, ExoChain)
- Connects to peers via the K6 cluster networking (QUIC/gRPC)
- Exchanges CMVG deltas as RVF segments (Q7)
- Respects peer cadence for delta sync rate matching (Q4)

This means there is no "mentra-cortex" as a separate project. There is a WeftOS build target for Mentra glasses:

```bash
# Build for glasses
cargo build --target aarch64-linux-android --features ecc -p clawft-weave

# Build for server
cargo build --features ecc,cluster -p clawft-weave

# Build for browser
cargo build --target wasm32-unknown-unknown --features ecc -p clawft-wasm
```

The Mentra-specific code (BES2700 UART, WiFi Lock, AudioBridge) lives in a `mentra-platform` crate that implements the `Platform` trait for the glasses hardware. This is the only Mentra-specific code — everything else is WeftOS.

**Why this is better than a separate daemon**: One codebase, one set of tests, one upgrade path. When WeftOS gains a new tree type or improves spectral analysis, every node in the nervous system — glasses, phone, server, browser — gets the improvement. No protocol translation, no API versioning between mentra-cortex and WeftOS.

---

### Q10: CMVG Delta Sync vs ruvector-delta-consensus -- RESOLVED

**Source**: K2 panel (D16) + user architectural direction

**Resolution: Option C -- Layered. CRDTs for convergence, Merkle causal graphs for verification.**

They operate at different layers and combine:

**ruvector-delta-consensus (CRDT)** handles **convergence**: automatic, conflict-free merging in highly concurrent, unreliable networks. Lightweight, great for real-time behavior. This is the "how do two nodes agree on state?" layer.
- G-Counters for tick counts, vector counts, edge counts
- OR-Sets for active node memberships, peer lists
- LWW-Registers for mutable metadata (scoring, calibration data)
- Merge semantics are mathematically guaranteed to converge

**CMVG / Merkle-provable causal graphs** handle **verification**: cryptographic proofs + clear causal history. This is the "can I prove this state is legitimate?" layer.
- Merkle proofs that a specific causal fragment is part of the canonical graph
- Provenance chains showing how a conclusion was reached
- Blockchain bridging (anchor Merkle roots on-chain without exposing data)
- Zero-knowledge proofs (prove "this state fragment is legit" without trusting anyone)
- Audit trails for regulatory compliance

**How they compose**: CRDT operations produce the convergent state. Each CRDT merge operation is logged as a causal edge in the Merkle graph with the merge inputs as parents. This means:
1. Two nodes exchange CRDT deltas for fast, conflict-free convergence (real-time, sub-ms)
2. The resulting merged state gets a Merkle commitment linking it to both source states (provenance)
3. Any third party can verify the merge was legitimate by walking the Merkle proof (verification)

The CRDT ensures the nodes agree. The Merkle graph proves they agreed honestly.

---

## Cross-Project Questions

### Q11: Mentra-Specific vs General ECC -- RESOLVED

**Source**: All panels + user architectural direction

**Resolution: WeftOS is fully hardware-agnostic. Mentra, ClawStage, NetworkNav are all spike prototypes that informed the architecture.**

The Mentra project, ClawStage, and NetworkNav are all experiments in this architecture — spike prototypes that helped discover and validate the ECC paradigm. Mentra hit paydirt with the ECC realization and the hardware benchmarks that proved feasibility. But none of them define WeftOS's scope.

WeftOS targets the full spectrum via `Kernel<P: Platform>`:

| Target | Role in Nervous System | Platform Trait |
|--------|----------------------|----------------|
| ESP32 | Sensor node (WiFi CSI, environmental) | `Esp32Platform` (no_std, minimal) |
| Smart glasses (MT6761) | Edge cognitive node | `AndroidPlatform` (aarch64) |
| Phone (S25/S26) | Near-edge coordinator, NPU inference | `AndroidPlatform` (aarch64) |
| Laptop/Desktop | Development, local inference | `NativePlatform` (x86_64/aarch64) |
| Browser | Client-side cognition | `BrowserPlatform` (wasm32) |
| Cloud (Blackwell GPU) | Full LLM inference, DEMOCRITUS, training | `NativePlatform` (x86_64) |

All are peers in the same nervous system. Each boots, calibrates, advertises its capabilities, and participates at its calibrated cadence. An ESP32 sensor node might only run a tiny HNSW (micro_hnsw, 1024 vectors) and emit impulses. A Blackwell node runs full spectral analysis, LLM inference, and DEMOCRITUS causal extraction. They're the same kernel — different calibrations.

Hardware-specific code lives in Platform trait implementations, never in WeftOS core. Mentra's BES2700 UART, WiFi Lock, AudioBridge → `mentra-platform` crate. ESP32 GPIO, WiFi CSI → `esp32-platform` crate. These are deployment crates, not architecture.

---

### Q12: DEMOCRITUS Integration -- RESOLVED

**Source**: Architecture panel (Mentra research) + user architectural direction

**Resolution: DEMOCRITUS is not a separate batch job — it is the bidirectional flywheel running continuously across the nervous system.**

The academic DEMOCRITUS takes 16 hours as a batch. Our implementation distributes this across the network in real-time, processing as data arrives. The 16-hour batch becomes 30-second micro-batches because causal relations are created at runtime as they happen.

**The bidirectional flow**:

**Upward (edge → cloud)**: Edge devices generate causal relations at runtime from real observations. Each cognitive tick on a glasses node or phone creates causal edges in its local graph. These edges — with their Merkle provenance — flow upward via CMVG delta sync to nodes with more compute. Some causal relations are clear to the edge device that generated them (e.g., "I said X, then they reacted with Y"). These are committed locally.

**Assessment (cloud/GPU)**: Higher-tier nodes (server, Blackwell GPU) receive these upward-flowing causal fragments and assess relations that were NOT clear to the edge device. An edge node sees "A happened, then B happened" but doesn't have the compute to determine whether A *caused* B or they were coincidental. The GPU node runs the heavier DEMOCRITUS analysis — spectral decomposition across large graph regions, cross-domain causal inference, LLM-assisted relation typing — and produces refined causal edges and new embeddings.

**Downward (cloud → edge)**: The refined embeddings and validated causal relations push back down to edge devices. The edge node's HNSW index receives new vectors that encode knowledge it couldn't derive locally. Its causal graph receives confirmed edges. Its next cognitive tick is smarter because the embedding space now has richer geometry.

**Implementation in WeftOS**: This is not a special service — it's the normal operation of heterogeneous kernel instances in the nervous system:

1. Edge node's cognitive tick creates local causal edges (cheap, per-tick)
2. CMVG delta sync sends new edges upward as RVF segments (Q7, Q10)
3. GPU node's cognitive tick runs at the same cadence but with a much larger compute budget — its calibrated tick (Q4) can afford heavy spectral analysis and LLM calls
4. GPU node creates refined edges and embeddings, which flow back down via the same delta sync
5. Edge node's HNSW receives the refined vectors, causal graph receives confirmed edges

No special DEMOCRITUS service needed. The asymmetry is handled by boot-time calibration (Q4) — an edge node's tick does lightweight ops, a GPU node's tick does heavyweight ops, and the delta sync connects them. The nervous system IS the distributed DEMOCRITUS pipeline.

**How the three composition modes map to DEMOCRITUS** (see `clawstage/docs/ecc-symposium/05-three-modes.md`):

The Act/Analyze/Generate composition loop from ClawStage provides the concrete implementation mechanism for this distributed pipeline:

- **Generate mode = DEMOCRITUS "downward flow"**: Expert agents produce causal relations at runtime through structured conversation. LLM-derived knowledge flows into the CMVG as causal edges and embeddings. This is the GPU/cloud node's cognitive tick creating refined edges and richer embedding geometry that push down to edge devices.

- **Analyze mode = DEMOCRITUS "assessment"**: Higher-tier nodes assess relations that weren't clear to the edge device. The academic 16-hour batch becomes a continuous analysis pass over the conversation's causal graph. Each cognitive tick on a capable node runs spectral decomposition, cross-domain causal inference, and LLM-assisted relation typing — the same work DEMOCRITUS does, but incrementally.

- **Act mode = DEMOCRITUS "upward flow"**: Real-world interactions on edge devices produce ground-truth causal data. Each cognitive tick on a glasses node or phone creates causal edges from direct observation ("I said X, then they reacted with Y"). These flow upward via CMVG delta sync to improve the models.

The loop composes continuously: **Generate -> Analyze -> Act -> Generate**. Each mode transition is itself a causal edge in the CMVG. Training material — scored trajectories, decision matrices, results metrics — falls out naturally as conversation metadata attached to these edges. The three modes are not phases that run sequentially; they run concurrently across the nervous system, with each node's calibrated capability determining which modes dominate its cognitive tick.

---

### Q13: Naming Convention for Cognitive Primitives -- RESOLVED

**Source**: Documentation review + user direction

**Resolution: Option C -- ECC for the paradigm, WeftOS-native for the implementation. "ClawFT" can almost always be replaced by "WeftOS."**

| Layer | Convention | Examples |
|-------|-----------|----------|
| **Paradigm** (conceptual) | ECC terminology | ECC, CMVG, cognitive tick, Fiedler value, inverted persistence, bidirectional flywheel |
| **Implementation** (code) | WeftOS-native | `CognitiveTick` service, `CausalGraph` module, `HnswService`, `CrossRef`, `Impulse` |
| **Crate naming** | `weftos-*` preferred over `clawft-*` for new crates | `weftos-platform-mentra`, `weftos-platform-esp32` |
| **Ecosystem libraries** | Keep existing names | `ruvector-*`, `rvf-*`, `exo-*`, `cognitum-*` |
| **Existing crates** | Keep `clawft-*` for now, migrate to `weftos-*` when appropriate | No rename churn during ECC integration |

In documentation: "WeftOS implements the ECC paradigm using CMVG as its cognitive substrate" — not "ClawFT has an ECC feature." The OS is WeftOS. ClawFT is the project/repo name but WeftOS is the product identity.

---

### Q14: Timeline and Prioritization -- RESOLVED

**Source**: Gap analysis synthesis + user architectural direction

**Resolution: K3c scope — ECC foundations inserted between K3 and K4.**

K1-K3 created the running kernel and all base services. K4+ implements containers, then app framework + clustering, then networking. K3c inserts the ECC cognitive substrate before K4 begins, putting WeftOS into position for cognitive operations throughout K4-K6.

**Phase mapping**:
- **K3c**: Single-node ECC substrate (CausalGraph, HnswService, CognitiveTick, CrossRefs, Impulses, calibration). ~2,500-3,500 lines. Full SPARC plan at `.planning/sparc/weftos/03c-ecc-integration.md`.
- **K4**: Containers + WASM cognitive modules + RVF persistence migration
- **K5**: Clustering + CMVG delta sync + CRDT+Merkle layered sync + SONA + DEMOCRITUS flywheel
- **K6**: Networking + BLAKE3 migration + cross-node delta sync rate matching

Clustering runs in containers or local processes BEFORE networking (by design — networking is the most complex part). K3c ensures the cognitive substrate is ready before containers need to host cognitive modules.

**SPARC plan**: `.planning/sparc/weftos/03c-ecc-integration.md`

---

## Summary: All Questions Resolved

| # | Question | Resolution | Key Decision |
|---|----------|-----------|-------------|
| Q1 | SHAKE-256 vs BLAKE3 | RESOLVED | BLAKE3 for new ECC code; SHAKE-256 stays until K6 migration |
| Q2 | ExoChain DAG vs CausalGraph | RESOLVED | Forest architecture — ExoChain linear + separate DAG + CrossRefs |
| Q3 | HNSW crate location | RESOLVED | Keep in clawft-core, wrap in kernel service (dep already exists) |
| Q4 | CognitiveTick relationship | RESOLVED | Configurable SystemService, self-calibrated at boot, cadence advertised |
| Q5 | EffectVector dimensionality | RESOLVED | Per-tree N-dim scoring, uniform CrossRef indexing for grafting/shaking |
| Q6 | Spectral analysis model | RESOLVED | Per-tree, benchmarked at boot, offloaded if calibration says so |
| Q7 | Persistence model | RESOLVED | RVF for everything (JSON was a dev shortcut) |
| Q8 | Feature flag scope | RESOLVED | One `ecc` flag at compile-time, boot calibration decides what's active |
| Q9 | Edge device integration | RESOLVED | mentra-cortex IS Kernel\<AndroidPlatform\> with --features ecc |
| Q10 | Delta sync vs CRDTs | RESOLVED | Layered — CRDTs for convergence, Merkle for verification |
| Q11 | Mentra-specific vs general | RESOLVED | Hardware-agnostic; ESP32 to Blackwell; Platform trait for specifics |
| Q12 | DEMOCRITUS integration | RESOLVED | Nervous system IS the distributed pipeline; tick asymmetry handles it |
| Q13 | Naming convention | RESOLVED | ECC for paradigm, WeftOS-native for implementation |
| Q14 | Timeline | RESOLVED | K3c scope — ECC substrate before K4, SPARC plan created |

**All 14 questions resolved. SPARC implementation plan at `.planning/sparc/weftos/03c-ecc-integration.md`. Ready for implementation.**
