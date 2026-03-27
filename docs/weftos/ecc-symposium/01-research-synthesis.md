# ECC Symposium: Research Synthesis

**Understanding the Two Projects Together**

---

## 1. The ECC Paradigm (from Mentra)

### What It Is

Ephemeral Causal Cognition (ECC) is a computational paradigm where intelligence arises from real-time geometric and causal operations on growing structured data, not from LLM token generation.

| Property | LLM | ECC |
|----------|-----|-----|
| Cost per "thought" | Billions of FLOPs | Thousands of FLOPs |
| Hardware | GPU, 7B+ params | Any CPU, Cortex-A53 |
| Latency | 100ms-10s | 1-15ms |
| Memory behavior | Degrades (context fills) | Improves (graph grows, search stays O(log n)) |
| Explainability | Opaque | Walk causal edges + show neighbors |
| Verifiability | None | Merkle chain proves reasoning history |

**Key Inversion**: In LLMs, cognition is permanent (frozen weights) and memory is ephemeral (context window). In ECC, cognition is ephemeral (each tick is fresh, 3-10ms) and memory is permanent (Merkle DAG grows monotonically).

### The CMVG Data Structure

The Causal Merkle Vector Graph has three layers:

1. **Semantic Layer**: 384-dim vector embeddings in HNSW index for O(log n) similarity search
2. **Causal Layer**: Typed, weighted, directed edges expressing causation, analyzed via normalized Laplacian spectral methods
3. **Provenance Layer**: BLAKE3 Merkle hash chains with Ed25519 signatures and Hybrid Logical Clock timestamps

### The Cognitive Tick

A 50ms loop that executes in 3.6ms (76% headroom on ARM Cortex-A53):
1. Read sensors / receive input
2. Generate embeddings (vectorize the moment)
3. HNSW search (find similar past moments)
4. Update causal graph (link cause to effect)
5. Merkle commit (cryptographically seal the tick)

Background every ~5 seconds: spectral analysis (Lambda_2 Fiedler value) for metacognition about coherence.

### Proven on Hardware

All 9 go/no-go business questions passed on the Mentra Live glasses (MT6761, 2GB RAM, Android 11):
- Combined tick: 3.6ms (target <15ms)
- Zero thermal throttling over 60s sustained
- Scalar quantization: 0.99 recall with 4x compression
- 2K nodes + 7K edges + 2K vectors fit in <500MB

---

## 2. WeftOS Architecture (from ClawFT)

### What It Is

WeftOS is an ephemeral operating system kernel for AI agent orchestration, implemented as the `clawft-kernel` Rust crate (25 modules, 17,052+ lines, 421 tests). It provides:

- **ExoChain**: Append-only hash-linked event log (SHAKE-256 + Ed25519)
- **Resource Tree**: Hierarchical Merkle-hashed namespace with 6D scoring
- **Governance Gate**: Constitutional three-branch model with 5D EffectVector
- **A2A Router**: Per-agent inboxes with capability-checked routing
- **WASM Sandbox**: Wasmtime-based isolation with fuel metering
- **Tool Registry**: 27 built-in tools with lifecycle management

### K-Phase Status

| Phase | Scope | Status | Tests |
|-------|-------|--------|-------|
| K0 | Foundation (boot, config, errors) | COMPLETE | 45+ |
| K1 | Process table, supervisor, RBAC | COMPLETE | 80+ |
| K2 | IPC, A2A routing, pub/sub, agent loop | COMPLETE | 130+ |
| K2b | Hardening, chain-logged lifecycle | COMPLETE | 30+ |
| K3 | Tool lifecycle, WASM runner, registry | COMPLETE | 50+ |
| K4 | Containers, remaining tools | PLANNED | -- |
| K5 | Apps + clustering | PLANNED | -- |
| K6 | Deep networking + replication | SPEC ONLY | -- |

### Key Design Decisions (K2 Symposium, D1-D22)

- **D1**: Services are abstract (not tied to PIDs) -- can be anything an agent owns
- **D4**: Layered protocol -- internal ServiceApi first, protocol adapters bind to it
- **D8**: Service API schemas are chain-anchored, immutable, versioned
- **D9**: Every service call gets an RVF witness by default (sub-us overhead)
- **D13**: Zero-knowledge proofs are foundational, not optional
- **D16**: CRDT (ruvector-delta-consensus) for off-chain convergence; ExoChain for on-chain ordering
- **D20**: N-dimensional EffectVector (configurable per environment)

---

## 3. Where They Converge

### 3.1 Merkle Provenance (Strongest Alignment)

| ECC Concept | WeftOS Implementation | Alignment |
|-------------|----------------------|-----------|
| Merkle DAG grows monotonically | ExoChain is append-only | Direct match |
| Every moment has provenance | Every ChainEvent has hash + prev_hash + seq | Direct match |
| Hash links prove temporal ordering | `verify_integrity()` chain walk | Direct match |
| Witness verification | `verify_witness_chain()` | Direct match |
| Ed25519 signing | `load_or_create_key()` in chain.rs | Direct match |
| RVF segment persistence | `write_exochain_event()` | Direct match |

**Key difference**: ExoChain is a linear chain (single prev_hash). CMVG is a DAG (multiple parents). This is the primary structural gap in the provenance layer.

### 3.2 Vector Search (Surprising Discovery)

The crate analysis revealed that **HNSW is already fully implemented in clawft-core**:

- `hnsw_store.rs`: Full `instant-distance` integration with configurable ef_search (100) and ef_construction (200)
- `micro_hnsw.rs`: Compact single-layer HNSW for WASM (8KB budget, max 1024 vectors)
- Cosine similarity, upsert semantics, JSON persistence, auto-fallback to brute-force below 32 entries

This was NOT connected to the kernel. The HNSW lives in `clawft-core` (the agent/pipeline layer) but not in `clawft-kernel` (the OS layer). ECC integration means wiring the existing HNSW into the kernel as a `SystemService`.

### 3.3 Governance as Cognitive Gating

WeftOS GovernanceGate evaluates a 5D EffectVector (risk, performance, difficulty, reward, reliability) for every action. ECC's insight: these effect vectors should live in the same HNSW index as concept embeddings. Governance decisions become searchable: "find the most similar past situation and apply its outcome." This turns rule-evaluation into similarity-search.

The Resource Tree's 6D NodeScoring (trust, performance, difficulty, reward, reliability, velocity) with EMA blending, cosine similarity, and Pareto dominance is an even richer vector space that could directly serve as cognitive metadata.

### 3.4 Agent Loop as Cognitive Loop Foundation

The `kernel_agent_loop` (1100 lines) provides the structural skeleton:
- Receives messages from mpsc inbox
- Dispatches to handlers with gate checks
- Logs events to ExoChain
- Handles lifecycle (suspend/resume)

ECC needs this same pattern but clock-driven (50ms cadence) rather than event-driven, with the sense-embed-search-update-commit cycle replacing the message-dispatch-respond cycle.

### 3.5 A2A Routing as CMVG Delta Sync

K2 decision D16 chose ruvector-delta-consensus for off-chain CRDT convergence while ExoChain provides on-chain ordering. CMVG delta sync is a specialized form of this: sending only changed portions of the vector graph between nodes as Merkle-provable causal graph fragments. The A2A router's topic pub/sub and cross-node IPC (K6 spec) map directly to CMVG delta broadcast channels.

### 3.6 RUV Ecosystem Alignment

| Crate | WeftOS Status | ECC Relevance |
|-------|--------------|---------------|
| rvf-crypto (SHAKE-256, Ed25519) | Fully used | Core provenance |
| rvf-types | Fully used | Type compatibility |
| rvf-wire | Fully used | Wire format for delta sync |
| exo-resource-tree | Fully used (local fork) | Merkle namespace |
| instant-distance | In Cargo.toml, implemented | CMVG vector layer |
| cognitum-gate-tilezero | Partially used | CGR + blake3 receipts |
| ruvector-cluster | Types only | Distributed CMVG |
| micro-hnsw-wasm | **In clawft-core, not kernel** | Edge CMVG vector layer |
| ruvector-delta-consensus | **Not wired** | CMVG delta sync |
| prime-radiant | **Not wired** | Spectral analysis |
| exo-dag | **Not wired** | CMVG causal DAG layer |
| sona | **Not wired** | Adaptive learning |
| exo-identity | **Not wired** | W3C DIDs for CMVG nodes |

---

## 4. The Integration Thesis

### WeftOS Manages Agents That Think. ECC Is the Thinking Itself.

WeftOS was designed to orchestrate LLM-powered agents -- managing their processes, routing their messages, gating their actions, and auditing their behavior. ECC provides a complementary cognitive substrate that runs below the LLM layer: 80-90% of "intelligence" comes from vector geometry and causal graph traversal, with LLMs invoked only for genuinely novel situations.

The integration path:

1. **HNSW moves from clawft-core into the kernel** as a `SystemService` behind a feature flag. The existing implementation is nearly complete -- it just needs kernel integration (chain logging, tree registration, gate checks).

2. **A CausalGraph module extends ExoChain** with DAG structure. ExoChain remains the linear audit log; the causal graph adds typed, weighted edges between concept nodes. Each edge mutation is logged to ExoChain.

3. **The cognitive tick becomes an alternative agent loop mode**. `kernel_cognitive_loop` wraps `kernel_agent_loop` with a 50ms timer and the sense-embed-search-update-commit cycle. It sends `KernelMessage` events to the agent loop when action is needed.

4. **NodeScoring becomes the cognitive metadata layer**. The existing 6D scoring with EMA blending and cosine similarity already supports the vector geometry that ECC relies on. Enriching it with causal edge weights creates the CMVG scoring substrate.

5. **The K6 cluster networking spec absorbs CMVG delta sync**. The planned QUIC + gRPC transport, cross-node IPC, and chain synchronization are exactly what CMVG delta sync needs.

### 4.1 Bidirectional Knowledge Flywheel via Act/Analyze/Generate

The integration path above describes the *structural* convergence. The *operational* convergence — how knowledge actually flows through the nervous system — maps directly to ClawStage's Act/Analyze/Generate composition loop (see `clawstage/docs/ecc-symposium/05-three-modes.md`).

This three-mode loop IS the distributed DEMOCRITUS pipeline running continuously across heterogeneous kernel instances:

- **Generate** (downward flow): Cloud/GPU nodes produce refined causal relations and embeddings via LLM-assisted analysis. These flow downward as CMVG deltas, enriching edge devices' HNSW geometry and causal graphs.
- **Analyze** (assessment): Higher-tier nodes continuously assess causal relations that edge devices couldn't resolve locally — replacing DEMOCRITUS's 16-hour batch with incremental analysis passes over the growing causal graph.
- **Act** (upward flow): Edge devices generate ground-truth causal data from real-world interaction. Each cognitive tick on a glasses or phone node creates causal edges from direct observation, flowing upward via delta sync.

The loop composes continuously (**Generate -> Analyze -> Act -> Generate**), with each transition recorded as a causal edge in the CMVG. Training material — scored trajectories, decision matrices, results metrics — emerges naturally as conversation metadata. This makes the bidirectional flywheel concrete: it is not an abstract data flow diagram but a specific composition of three operational modes running across the kernel instances described in the integration path above.

### What This Means for ClawFT

ClawFT gains a concrete cognitive primitive -- not another wrapper around LLM API calls, but a self-contained intelligence engine that:
- Runs on any hardware (including $30 ARM SoCs)
- Gets smarter with more data without retraining
- Provides explainable, verifiable reasoning
- Operates at 3-10ms latency (3-5 orders of magnitude cheaper than LLM calls)
- Maintains cryptographic provenance of its entire reasoning history

This transforms WeftOS from an agent orchestrator into a cognitive platform.
