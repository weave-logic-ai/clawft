# WeftOS Kernel: RuVector Deep Integration Architecture

**Document ID**: 07-ruvector-deep-integration
**Workstream**: W-KERNEL
**Date**: 2026-02-28
**Status**: Partially Implemented (cluster layer done)
**Supersedes**: Portions of K0-K5 where ruvector crates replace custom implementations
**Dependencies**: ruvector distributed crates from `https://github.com/weave-logic-ai/ruvector`

---

## 0. Reality Check: What Actually Exists in ruvector

> **This section was added 2026-03-01 after auditing the actual ruvector repository.**

The original version of this document (below) references **101 crates** and names many
specific crates. After cloning and inspecting the ruvector repo, only a subset exists.
The integration plan has been adjusted to work with what is real.

### Crates That Exist and Are Integrated

| Crate | Purpose | Status in WeftOS |
|-------|---------|-----------------|
| `ruvector-cluster` | `ClusterManager`, `ShardRouter`, `DagConsensus`, pluggable `Discovery` | **Integrated** behind `#[cfg(feature = "cluster")]` in `clawft-kernel` |
| `ruvector-raft` | `RaftNode` leader election and log replication | **Available** as optional dep, not yet wired as kernel service |
| `ruvector-replication` | `ReplicaSet`, `SyncManager` for state replication | **Available** as optional dep, not yet wired |
| `ruvector-core` | Vector store, HNSW index, quantization | **Optional** — feature-gated behind `vector-store` in all 3 distributed crates |

### Crates Referenced Below That Do NOT Exist

The following crate names appear in the original architecture below but **do not exist**
in the ruvector repository. They represent aspirational design, not available code:

- `cognitum-gate-tilezero` (coherence gate)
- `ruvector-nervous-system` (event bus)
- `ruvector-delta-consensus` (causal ordering)
- `prime-radiant` (routing optimizer)
- `rvf-wasm` (WASM runtime)
- `rvf-kernel` (kernel primitives)
- `rvf-crypto` (crypto operations)
- `rvf-wire` (wire format)
- `mcp-gate` (MCP integration)

### What Was Actually Implemented

1. **Universal ClusterMembership** (`crates/clawft-kernel/src/cluster.rs`):
   Compiles on all platforms including WASM. Tracks peers with `Option<String>`
   addresses and `NodePlatform` enum (CloudNative, Browser, Edge, Wasi). Always
   present on `Kernel<P>`.

2. **Native ClusterService** (`crates/clawft-kernel/src/cluster.rs`, behind `cluster` feature):
   Wraps `ruvector_cluster::ClusterManager` with `StaticDiscovery`. Implements
   `SystemService` trait. Syncs ruvector node state into kernel `ClusterMembership`.

3. **Daemon RPC** (`crates/clawft-weave/src/daemon.rs`):
   6 new dispatch methods: `cluster.status`, `cluster.nodes`, `cluster.join`,
   `cluster.leave`, `cluster.health`, `cluster.shards`.

4. **CLI** (`crates/clawft-weave/src/commands/cluster_cmd.rs`):
   `weaver cluster {status|nodes|join|leave|health|shards}` commands.

5. **ruvector fork fix**: Made `ruvector-core` optional in all 3 distributed crates
   (it was listed as dep but never imported).

### Future Integration Phases

- **Raft leader election**: Wire `RaftNode` as a kernel service for metadata consensus
- **Replication**: Wire `ReplicaSet` + `SyncManager` for agent state across nodes
- **WS cluster protocol**: Full bidirectional cluster protocol over WebSocket
- **Vector store**: Enable `ruvector-core` vector-store feature for distributed vectors

---

## 1. Integration Strategy

### Why Depend on RuVector Instead of Reimplementing

The original WeftOS plan (K0-K5) proposes building custom implementations of service
registry, capability checking, IPC messaging, WASM sandboxing, and container management.
The ruvector ecosystem already provides production-grade versions of nearly everything
WeftOS needs, and in many cases provides capabilities that would take months to build
from scratch.

**Arguments for deep integration:**

1. **Avoid reimplementation of solved problems.** The original K0 `ServiceRegistry` is
   a `DashMap<String, Arc<dyn SystemService>>`. ruvector-cluster provides `ClusterManager`
   with the same `DashMap` foundation plus health checks, consistent hash ring, and
   pluggable discovery -- all tested. Building our own means duplicating this work and
   maintaining it separately.

2. **Gain capabilities that are not in the current plan.** The K0-K5 plan does not include
   self-learning routing, causal ordering for IPC, coherence scoring, hash-chained audit
   trails, three-way capability decisions, or epoch-budgeted execution. These are all
   available in ruvector and would require significant research and engineering to build.

3. **Unified container format.** RVF (RuVector Format) is a segment-based binary format
   that can contain kernel, WASM, vectors, witness chains, and configuration in a single
   file. This is fundamentally different from Docker containers and offers properties
   (crash-safe append-only writes, 64-byte SIMD alignment, cryptographic attestation)
   that Docker does not.

4. **Wire format efficiency.** rvf-wire segments are 64-byte SIMD-aligned with
   type/size/checksum headers. This replaces JSON-RPC for IPC with a format that is
   both more efficient and crash-safe.

5. **Shared maintenance burden.** ruvector is actively maintained. By depending on it
   rather than reimplementing, WeftOS benefits from upstream improvements without
   additional engineering effort.

**Arguments against (and mitigations):**

| Concern | Mitigation |
|---------|------------|
| API instability (pre-1.0 crates) | Pin exact git commits; wrap behind clawft trait abstractions |
| Binary size bloat | Feature gates for every ruvector crate; native-only for heavy deps |
| WASM incompatibility | All ruvector deps behind `native` feature gate; browser builds exclude them |
| Coupling to external project | Adapter layer isolates ruvector types from clawft public API |
| Not published on crates.io | Use git dependencies with pinned revisions; vendor if needed |

### Integration Principle

**Depend, do not fork. Wrap, do not expose.**

clawft-kernel depends on ruvector crates but wraps them behind clawft's own trait
abstractions. No ruvector type appears in clawft-kernel's public API. This allows
swapping out ruvector for an alternative implementation without changing consumers.

---

## 2. Dependency Architecture

### Ruvector Crates as Dependencies of clawft-kernel

Each ruvector crate maps to a feature gate in clawft-kernel. The default build includes
none of them. Feature bundles provide convenience groupings.

```toml
# crates/clawft-kernel/Cargo.toml

[features]
default = []

# K0: Foundation -- cluster-based service registry
ruvector-cluster = ["dep:ruvector-cluster"]

# K1: Supervisor + RBAC -- cognitive container + coherence gate
ruvector-supervisor = [
    "dep:ruvector-cognitive-container",
    "dep:cognitum-gate-tilezero",
]

# K2: IPC -- delta consensus + nervous system event bus
ruvector-ipc = [
    "dep:ruvector-delta-consensus",
    "dep:ruvector-nervous-system",
]

# K3: WASM -- rvf-wasm ultra-minimal runtime
ruvector-wasm = ["dep:rvf-wasm"]
# Wasmtime for full tools (orthogonal to rvf-wasm)
wasm-sandbox = ["dep:wasmtime"]

# K4: Containers -- RVF format replaces Docker
ruvector-containers = [
    "dep:rvf-wire",
    "dep:rvf-kernel",
    "dep:rvf-crypto",
]
# Docker fallback (original plan)
containers-docker = ["dep:bollard"]

# K5: App framework -- self-learning + coherence scoring
ruvector-apps = [
    "dep:ruvector-tiny-dancer-core",
    "dep:sona",
    "dep:prime-radiant",
    "dep:mcp-gate",
]

# Cross-cutting: crypto audit trail
ruvector-crypto = ["dep:rvf-crypto"]

# Cross-cutting: wire format for IPC
ruvector-wire = ["dep:rvf-wire"]

# Convenience bundles
ruvector-core = [
    "ruvector-cluster",
    "ruvector-supervisor",
    "ruvector-ipc",
    "ruvector-crypto",
    "ruvector-wire",
]
ruvector-full = [
    "ruvector-core",
    "ruvector-wasm",
    "ruvector-containers",
    "ruvector-apps",
]

[dependencies]
# Always required (clawft ecosystem)
clawft-core = { path = "../clawft-core" }
clawft-types = { path = "../clawft-types" }
clawft-platform = { path = "../clawft-platform" }
clawft-plugin = { path = "../clawft-plugin" }
dashmap = "6.0"
tokio = { workspace = true }
chrono = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tracing = { workspace = true }
uuid = { workspace = true }

# ruvector dependencies (all optional, feature-gated)
ruvector-cluster = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
ruvector-cognitive-container = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
cognitum-gate-tilezero = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
ruvector-delta-consensus = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
ruvector-nervous-system = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
ruvector-tiny-dancer-core = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
sona = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
prime-radiant = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
mcp-gate = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
rvf-wire = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
rvf-wasm = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
rvf-kernel = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
rvf-crypto = { git = "https://github.com/ruvnet/ruvector", rev = "PIN_ME", optional = true }
wasmtime = { version = "27.0", optional = true }
bollard = { version = "0.17", optional = true }
```

### WASM/Browser Exclusion

All ruvector dependencies are native-only. The existing `clawft-wasm` crate targets
`wasm32-unknown-unknown` and must not transitively pull any ruvector crate. This is
enforced by:

1. `clawft-kernel` is never a dependency of `clawft-wasm`
2. All ruvector features are behind optional feature gates
3. The workspace `Cargo.toml` does not enable any ruvector features for `clawft-wasm`
4. CI verifies: `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser`

---

## 3. Phase-by-Phase Changes

### Phase K0: Kernel Foundation

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| `ServiceRegistry` (DashMap + `SystemService` trait) | `ruvector-cluster::ClusterManager` | Same DashMap foundation, adds health checks, consistent hash ring, pluggable discovery |
| `HealthSystem` (periodic health checks) | `ruvector-cluster::ClusterManager::health_check()` + `prime-radiant::SheafLaplacian` | Cluster manager already does health; prime-radiant adds coherence scoring |
| Custom `KernelState` enum | `ruvector-cognitive-container::ContainerState` | CognitiveContainer has richer state machine with epoch tracking |

#### What stays the same

| Component | Reason |
|---|---|
| `ProcessTable` (PID tracking) | Clawft-specific; no ruvector equivalent for PID-based process tracking |
| `KernelConfig` types | Clawft-specific configuration schema |
| `Kernel<P: Platform>` boot sequence | Orchestration is clawft-specific; ruvector provides subsystems, not the boot orchestrator |
| CLI commands (`weave kernel status`, etc.) | Clawft CLI layer, not ruvector's concern |

#### New integration code needed

```rust
// service_adapter.rs -- bridges ClusterManager to clawft's ServiceRegistry interface

/// Adapter that wraps ruvector-cluster's ClusterManager
/// to implement clawft-kernel's ServiceRegistry semantics.
pub struct ClusterServiceRegistry {
    cluster: ClusterManager,
    #[cfg(feature = "ruvector-apps")]
    coherence: Option<SheafLaplacian>,
}

impl ClusterServiceRegistry {
    pub fn new() -> Self {
        let cluster = ClusterManager::new(ClusterConfig::default());
        Self { cluster, coherence: None }
    }

    /// Register a SystemService as a ClusterNode.
    pub fn register(&self, service: Arc<dyn SystemService>) -> Result<()> {
        let node = ClusterNode {
            id: service.name().to_string(),
            status: NodeStatus::Active,
            metadata: HashMap::new(),
        };
        self.cluster.register_node(node)?;
        // Store the service impl for lifecycle callbacks
        self.cluster.set_user_data(service.name(), service);
        Ok(())
    }

    /// Health check with optional coherence scoring.
    pub async fn health_all(&self) -> Vec<(String, HealthStatus)> {
        let mut results = self.cluster.health_check_all().await;

        #[cfg(feature = "ruvector-apps")]
        if let Some(ref coherence) = self.coherence {
            // Augment each health result with a coherence score
            for (name, status) in &mut results {
                let score = coherence.score_node(name);
                status.coherence_score = Some(score);
            }
        }

        results
    }
}
```

```rust
// boot.rs -- kernel boot now uses ClusterServiceRegistry

pub struct Kernel<P: Platform> {
    state: KernelState,
    app_context: Option<AppContext<P>>,
    process_table: Arc<ProcessTable>,
    #[cfg(feature = "ruvector-cluster")]
    service_registry: Arc<ClusterServiceRegistry>,
    #[cfg(not(feature = "ruvector-cluster"))]
    service_registry: Arc<ServiceRegistry>,  // fallback to original
    ipc: Arc<KernelIpc>,
    health: HealthSystem,
    #[cfg(feature = "ruvector-crypto")]
    witness: Arc<WitnessChain>,  // audit trail from boot
}
```

---

### Phase K1: Supervisor + RBAC

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| `AgentCapabilities` with binary Permit/Deny | `cognitum-gate-tilezero::TileZero` with three-way Permit/Defer/Deny | Defer allows escalation patterns; PermitTokens provide cryptographic proof of authorization |
| `CapabilityChecker` | `cognitum-gate-tilezero::TileZero::evaluate()` | 256-tile arbiter with three-filter decision pipeline is strictly more powerful |
| Resource tracking (manual `ResourceUsage`) | `ruvector-cognitive-container::EpochController` | Budget system with `try_budget()`, `consume()`, partial results on exhaustion |

#### What stays the same

| Component | Reason |
|---|---|
| `AgentSupervisor<P>` spawn/stop/restart | Lifecycle management is clawft-specific; ruvector does not manage agent processes |
| `ProcessEntry` struct | PID tracking is clawft-specific |
| Pre-tool-call hook in `loop_core.rs` | Hook point is clawft architecture; ruvector provides the check implementation |
| `filtered_tools()` on ToolRegistry | Tool filtering is clawft-specific |

#### New integration code needed

```rust
// capability_adapter.rs

/// Three-way capability decision, replacing binary Permit/Deny.
#[derive(Debug, Clone)]
pub enum CapabilityDecision {
    /// Access granted. PermitToken is cryptographic proof.
    Permit(PermitToken),
    /// Decision deferred -- escalate to supervisor or human.
    Defer { reason: String, context: ActionContext },
    /// Access denied.
    Deny { reason: String, receipt: WitnessReceipt },
}

/// Adapter wrapping TileZero for clawft's capability checking.
pub struct GateCapabilityChecker {
    gate: TileZero,
    process_table: Arc<ProcessTable>,
    #[cfg(feature = "ruvector-crypto")]
    witness: Arc<WitnessChain>,
}

impl GateCapabilityChecker {
    pub fn check_tool_access(&self, pid: Pid, tool_name: &str) -> Result<CapabilityDecision> {
        let entry = self.process_table.get(pid)?;
        let context = ActionContext::new()
            .with_actor(format!("pid:{}", pid))
            .with_resource(format!("tool:{}", tool_name))
            .with_metadata("agent_id", &entry.agent_id);

        let decision = self.gate.evaluate(&context)?;

        // Record every decision in the witness chain
        #[cfg(feature = "ruvector-crypto")]
        {
            let receipt = decision.receipt();
            self.witness.append(receipt)?;
        }

        match decision {
            GateDecision::Permit(token) => Ok(CapabilityDecision::Permit(token)),
            GateDecision::Defer(ctx) => Ok(CapabilityDecision::Defer {
                reason: ctx.reason().to_string(),
                context: ctx,
            }),
            GateDecision::Deny(receipt) => Ok(CapabilityDecision::Deny {
                reason: receipt.reason().to_string(),
                receipt,
            }),
        }
    }

    pub fn check_ipc_target(&self, from_pid: Pid, to_pid: Pid) -> Result<CapabilityDecision> {
        let context = ActionContext::new()
            .with_actor(format!("pid:{}", from_pid))
            .with_resource(format!("ipc:pid:{}", to_pid));
        // Same three-way evaluation
        self.evaluate_context(context)
    }
}

/// Epoch-budgeted agent execution wrapper.
pub struct BudgetedAgent {
    epoch: EpochController,
    component_mask: ComponentMask,
}

impl BudgetedAgent {
    pub fn try_tool_call(&mut self, cost: u64) -> Result<(), BudgetExhausted> {
        self.epoch.try_budget(cost)?;
        Ok(())
    }

    pub fn consume(&mut self, actual_cost: u64) {
        self.epoch.consume(actual_cost);
    }

    /// Returns partial results when budget is exhausted instead of failing.
    pub fn partial_result(&self) -> Option<ContainerSnapshot> {
        if self.epoch.is_exhausted() {
            Some(self.epoch.snapshot())
        } else {
            None
        }
    }
}
```

---

### Phase K2: Agent-to-Agent IPC

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| `KernelMessage` with JSON-RPC wire format | `rvf-wire` segments with type/size/checksum headers | SIMD-aligned, crash-safe, more efficient than JSON-RPC |
| `A2AProtocol` request-response tracking | `ruvector-delta-consensus::DeltaConsensus` | CRDT-based sync with causal ordering via VectorClock |
| `TopicRouter` (DashMap subscriptions) | `ruvector-nervous-system::ShardedEventBus` | High-throughput sharded delivery with budget-aware routing |
| Manual per-agent rate limiting | `ruvector-nervous-system::BudgetGuardrail` | Resource-aware routing that avoids overloaded agents |
| No causal ordering | `ruvector-delta-consensus::VectorClock` | Causal ordering for concurrent messages; no reordering bugs |

#### What stays the same

| Component | Reason |
|---|---|
| `MessageTarget` enum (Pid, Topic, Broadcast, Service) | Clawft's addressing model is specific to its process table |
| `MessagePayload` variants (ToolCall, ToolResult, Signal) | Clawft-specific payload types ride inside rvf-wire segments |
| IPC scope enforcement logic | Delegates to `GateCapabilityChecker` (K1) for access checks |
| `DelegationEngine` wiring | Integration point is clawft-specific |
| MCP tool exposure (`ipc_send`, `ipc_subscribe`) | Clawft MCP layer |

#### New integration code needed

```rust
// ipc_adapter.rs

/// IPC subsystem backed by ruvector nervous system + delta consensus.
pub struct RuvectorIpc {
    #[cfg(feature = "ruvector-ipc")]
    event_bus: ShardedEventBus,
    #[cfg(feature = "ruvector-ipc")]
    consensus: DeltaConsensus,
    #[cfg(feature = "ruvector-ipc")]
    guardrail: BudgetGuardrail,
    process_table: Arc<ProcessTable>,
    capability_checker: Arc<GateCapabilityChecker>,
}

impl RuvectorIpc {
    /// Send a message with causal ordering.
    pub async fn send(&self, from: Pid, msg: KernelMessage) -> Result<DeliveryStatus> {
        // 1. Capability check (three-way)
        let decision = self.capability_checker.check_ipc_target(from, msg.target_pid())?;
        match decision {
            CapabilityDecision::Deny { reason, .. } => return Err(IpcError::Denied(reason)),
            CapabilityDecision::Defer { .. } => return Err(IpcError::Deferred),
            CapabilityDecision::Permit(_token) => { /* proceed */ }
        }

        // 2. Check budget guardrail (don't route to overloaded agent)
        if !self.guardrail.can_route_to(msg.target_pid()) {
            return Err(IpcError::TargetOverloaded);
        }

        // 3. Serialize to rvf-wire segment
        let segment = self.serialize_to_wire(&msg)?;

        // 4. Create causal delta with vector clock
        let delta = CausalDelta::new(
            segment,
            self.consensus.current_clock(),
        );

        // 5. Deliver via sharded event bus
        let status = self.event_bus.publish(
            msg.target_topic(),
            delta,
        ).await?;

        Ok(status)
    }

    /// Subscribe to topic with sharded delivery.
    pub fn subscribe(&self, pid: Pid, topic: &str) -> Result<mpsc::Receiver<KernelMessage>> {
        let (tx, rx) = mpsc::channel(1024);
        self.event_bus.subscribe(topic, move |delta: CausalDelta| {
            let msg = Self::deserialize_from_wire(delta.payload());
            let _ = tx.try_send(msg);
        })?;
        Ok(rx)
    }

    fn serialize_to_wire(&self, msg: &KernelMessage) -> Result<WireSegment> {
        // rvf-wire segment: 64-byte aligned, type+size+checksum header
        let payload_bytes = serde_json::to_vec(&msg.payload)?;
        Ok(WireSegment::new(
            SegmentType::IPC_MESSAGE,
            &payload_bytes,
        ))
    }

    fn deserialize_from_wire(segment: &[u8]) -> KernelMessage {
        let wire = WireSegment::parse(segment).expect("valid segment");
        let payload: MessagePayload = serde_json::from_slice(wire.data()).expect("valid payload");
        // Reconstruct KernelMessage from wire data + metadata
        KernelMessage::from_wire(wire, payload)
    }
}
```

---

### Phase K3: WASM Tool Sandboxing

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| Wasmtime-only approach | Dual runtime: rvf-wasm (micro-tools) + wasmtime (full tools) | rvf-wasm is 5.5KB with 14 C exports; perfect for micro-tools that need sub-ms execution |
| Custom fuel metering | `ruvector-cognitive-container::EpochController` for budget | Unified budget system across WASM and native execution |

#### What stays the same

| Component | Reason |
|---|---|
| `WasmToolRunner` with wasmtime | Wasmtime is still needed for full WASI tools; rvf-wasm handles micro-tools |
| `WasmSandboxConfig` | Configuration is clawft-specific |
| `ToolRegistry` integration | Registration interface is clawft-specific |
| Feature gate `wasm-sandbox` | wasmtime remains optional |

#### New integration code needed

```rust
// wasm_dual.rs -- dual WASM runtime strategy

/// Micro-tool runner using rvf-wasm (5.5KB, no allocator, 14 C exports).
/// For tools that need sub-millisecond execution: transforms, lookups, filters.
#[cfg(feature = "ruvector-wasm")]
pub struct MicroWasmRunner {
    // rvf-wasm operates on explicit memory layout, no allocator
}

#[cfg(feature = "ruvector-wasm")]
impl MicroWasmRunner {
    pub fn execute(&self, wasm_bytes: &[u8], input: &[u8]) -> Result<Vec<u8>> {
        // rvf-wasm C-ABI: rvf_init, rvf_load_query, rvf_distances, rvf_topk_merge, etc.
        // No wasmtime overhead, no JIT compilation
        // Budget enforcement via EpochController
        todo!("Wire rvf-wasm C exports")
    }

    pub fn estimated_size() -> usize {
        5_500 // 5.5KB runtime
    }
}

/// Full tool runner using wasmtime (for WASI-capable tools).
/// For tools that need filesystem, network, or complex computation.
#[cfg(feature = "wasm-sandbox")]
pub struct FullWasmRunner {
    engine: wasmtime::Engine,
    linker: wasmtime::Linker<ToolState>,
    config: WasmSandboxConfig,
}

/// Unified WASM dispatcher that routes to micro or full runner.
pub struct WasmDispatcher {
    #[cfg(feature = "ruvector-wasm")]
    micro: MicroWasmRunner,
    #[cfg(feature = "wasm-sandbox")]
    full: FullWasmRunner,
}

impl WasmDispatcher {
    pub async fn execute(&self, tool: &WasmTool, input: serde_json::Value) -> Result<WasmToolResult> {
        match tool.runtime_hint {
            RuntimeHint::Micro => {
                #[cfg(feature = "ruvector-wasm")]
                return self.micro.execute(&tool.wasm_bytes, &input_bytes);
                #[cfg(not(feature = "ruvector-wasm"))]
                return Err(WasmError::MicroRuntimeNotAvailable);
            }
            RuntimeHint::Full => {
                #[cfg(feature = "wasm-sandbox")]
                return self.full.execute(tool, input).await;
                #[cfg(not(feature = "wasm-sandbox"))]
                return Err(WasmError::FullRuntimeNotAvailable);
            }
            RuntimeHint::Auto => {
                // Heuristic: if module < 64KB and no WASI imports, use micro
                if tool.wasm_bytes.len() < 65536 && !tool.requires_wasi {
                    #[cfg(feature = "ruvector-wasm")]
                    return self.micro.execute(&tool.wasm_bytes, &input_bytes);
                }
                #[cfg(feature = "wasm-sandbox")]
                return self.full.execute(tool, input).await;
                Err(WasmError::NoRuntimeAvailable)
            }
        }
    }
}
```

---

### Phase K4: Containers

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| Docker/bollard container management | `rvf-kernel` + `rvf-wire` RVF container format | Single binary containing kernel, WASM, vectors, witness chains; 125ms boot |
| `Dockerfile.alpine` | `rvf-kernel` bzImage + initramfs builder | No Docker daemon dependency; bare metal or VM deployment |
| `docker-compose.yml` | RVF manifest within the container file | Self-describing format |
| Docker health checks | `rvf-crypto` witness-attested health | Cryptographic proof of health state |

**This is the most significant architectural decision.** RVF is a fundamentally different
container format from Docker. It produces a single binary file containing:

- A Linux microkernel (bzImage + initramfs, 125ms boot)
- WASM modules (tools, plugins)
- Vector indexes (HNSW, embeddings)
- Witness chains (audit trail)
- Configuration segments
- Cryptographic signatures (ML-DSA-65/Ed25519 dual-signing)

This replaces Docker's layered filesystem approach with a segment-based archive
that is crash-safe (append-only), SIMD-aligned, and cryptographically attested.

#### What stays the same

| Component | Reason |
|---|---|
| `ContainerService` implementing `SystemService` | Service abstraction unchanged; implementation switches from Docker to RVF |
| Container lifecycle state machine | States remain the same; backing implementation changes |
| Feature gating | `ruvector-containers` replaces `containers` for RVF; `containers-docker` retains Docker fallback |

#### New integration code needed

```rust
// container_rvf.rs -- RVF container format integration

/// RVF container manager -- replaces Docker for WeftOS packaging.
#[cfg(feature = "ruvector-containers")]
pub struct RvfContainerManager {
    // rvf-kernel builds bzImage + initramfs
    // rvf-wire handles segment read/write
    // rvf-crypto provides attestation
}

#[cfg(feature = "ruvector-containers")]
impl RvfContainerManager {
    /// Build an RVF container from a manifest.
    pub async fn build(&self, manifest: &ContainerManifest) -> Result<RvfContainer> {
        let mut builder = rvf_kernel::Builder::new();

        // Add kernel
        builder.set_kernel(rvf_kernel::minimal_bzimage()?);

        // Add WASM modules
        for tool in &manifest.tools {
            let segment = WireSegment::new(SegmentType::WASM_MODULE, &tool.wasm_bytes);
            builder.add_segment(segment);
        }

        // Add vector indexes
        for index in &manifest.indexes {
            let segment = WireSegment::new(SegmentType::HNSW_INDEX, &index.data);
            builder.add_segment(segment);
        }

        // Add configuration
        let config_segment = WireSegment::new(
            SegmentType::CONFIG,
            &serde_json::to_vec(&manifest.config)?,
        );
        builder.add_segment(config_segment);

        // Sign the container
        let container = builder.build()?;
        let signed = rvf_crypto::sign_container(&container, &self.signing_key)?;

        Ok(signed)
    }

    /// Boot an RVF container (125ms target).
    pub async fn start(&self, container: &RvfContainer) -> Result<RunningContainer> {
        // Verify signature chain
        rvf_crypto::verify_container(container)?;

        // Extract and boot kernel
        let kernel = container.kernel_segment()?;
        let initramfs = container.initramfs_segment()?;

        // Boot via rvf-kernel (bare metal or VM)
        let instance = rvf_kernel::boot(kernel, initramfs).await?;

        Ok(RunningContainer { instance, container: container.clone() })
    }

    /// Health check with cryptographic attestation.
    pub async fn health_check(&self, container: &RunningContainer) -> Result<AttestedHealth> {
        let health = container.instance.health().await?;
        let attestation = rvf_crypto::attest_health(&health, &self.signing_key)?;
        Ok(AttestedHealth { health, attestation })
    }
}

/// Fallback: Docker container manager (original K4 plan).
/// Available when `containers-docker` feature is enabled.
#[cfg(feature = "containers-docker")]
pub struct DockerContainerManager {
    docker: bollard::Docker,
    managed_containers: DashMap<String, ManagedContainer>,
    config: ContainerConfig,
}

/// Unified container interface.
pub enum ContainerBackend {
    #[cfg(feature = "ruvector-containers")]
    Rvf(RvfContainerManager),
    #[cfg(feature = "containers-docker")]
    Docker(DockerContainerManager),
}
```

**Decision: Docker as fallback, RVF as primary.**

RVF containers are the primary path because they:
- Eliminate the Docker daemon dependency
- Provide cryptographic attestation
- Boot in 125ms vs Docker's multi-second startup
- Include everything in a single file (no layer resolution)
- Are crash-safe by construction

Docker remains available behind `containers-docker` for:
- Third-party images that are only available as Docker images
- Development environments where Docker is already present
- Migration path from existing Docker-based deployments

---

### Phase K5: Application Framework

#### What ruvector replaces

| Original (custom) | Replaced by (ruvector) | Why |
|---|---|---|
| No learning capability | `sona::SonaEngine` (MicroLoRA + EwcPlusPlus) | Self-learning routing, pattern extraction, trajectory tracking |
| No coherence scoring | `prime-radiant::SheafLaplacian` | Health/quality scoring for application subsystems |
| No MCP gate integration | `mcp-gate` tools | Expose coherence gate as MCP tools (permit_action, get_receipt, replay_decision) |
| Static routing decisions | `ruvector-tiny-dancer-core::Router` (FastGRNN) | Sub-millisecond self-learning agent routing |

#### What stays the same

| Component | Reason |
|---|---|
| `AppManifest` (weftapp.toml) | Clawft-specific manifest format |
| `AppManager<P>` lifecycle | Clawft-specific application lifecycle |
| CLI commands (`weft app install/start/stop/list/inspect`) | Clawft CLI layer |
| Tool/service namespacing | Clawft convention |

#### New integration code needed

```rust
// app_intelligence.rs -- cross-cutting learning and coherence for apps

/// Self-learning application runtime.
#[cfg(feature = "ruvector-apps")]
pub struct IntelligentAppRuntime {
    /// Sub-ms agent routing via FastGRNN.
    router: ruvector_tiny_dancer_core::Router,
    /// Self-learning with MicroLoRA rank-1 (<1ms) + BaseLoRA rank-8.
    learner: sona::SonaEngine,
    /// Coherence scoring for application health.
    coherence: prime_radiant::SheafLaplacian,
    /// Circuit breaker for graceful degradation.
    circuit_breaker: ruvector_tiny_dancer_core::CircuitBreaker,
}

#[cfg(feature = "ruvector-apps")]
impl IntelligentAppRuntime {
    /// Route a task to the best agent within an application.
    pub async fn route_task(&self, task: &AppTask) -> Result<RoutingDecision> {
        // FastGRNN neural inference for agent selection
        let decision = self.router.route(task.embedding()).await?;

        // Check circuit breaker
        if self.circuit_breaker.is_open(&decision.target_agent) {
            return self.router.fallback_route(task).await;
        }

        // Record trajectory for learning
        self.learner.trajectory_start(task.id());

        Ok(decision)
    }

    /// Record task outcome for self-learning.
    pub async fn record_outcome(&self, task_id: &str, outcome: &TaskOutcome) -> Result<()> {
        // MicroLoRA instant adaptation (<1ms)
        self.learner.trajectory_step(task_id, outcome)?;
        self.learner.micro_adapt(outcome)?;

        // Update circuit breaker state
        if outcome.is_failure() {
            self.circuit_breaker.record_failure(&outcome.agent_id);
        } else {
            self.circuit_breaker.record_success(&outcome.agent_id);
        }

        Ok(())
    }

    /// Get coherence score for an application.
    pub fn coherence_score(&self, app_name: &str) -> f64 {
        self.coherence.score_subsystem(app_name)
    }
}
```

---

## 4. Type Mapping

### WeftOS Types to RuVector Types

| WeftOS Type (original K0-K5) | RuVector Type | Crate | Notes |
|---|---|---|---|
| `ServiceRegistry` | `ClusterManager` | ruvector-cluster | DashMap-based, adds health checks + consistent hash ring |
| `HealthStatus` | `NodeStatus` | ruvector-cluster | Maps: Healthy->Active, Degraded->Degraded, Unhealthy->Inactive, Unknown->Unknown |
| `SystemService` trait | `DiscoveryService` trait | ruvector-cluster | clawft `SystemService` wraps `DiscoveryService`; adds lifecycle (start/stop) |
| `AgentCapabilities` | `ActionContext` + `TileZero` config | cognitum-gate-tilezero | Capabilities expressed as gate filter rules, not a struct |
| `CapabilityChecker` | `TileZero` | cognitum-gate-tilezero | 256-tile arbiter; three-way decisions |
| Binary Permit/Deny | `GateDecision::Permit/Defer/Deny` | cognitum-gate-tilezero | Defer is new; enables escalation patterns |
| `ResourceLimits` | `EpochController` budget | ruvector-cognitive-container | `try_budget()` / `consume()` with partial results on exhaustion |
| `ResourceUsage` | `ComponentMask` + epoch telemetry | ruvector-cognitive-container | Bitmask for tracking completed phases |
| `KernelMessage` (JSON) | `WireSegment` (binary) | rvf-wire | 64-byte SIMD-aligned, type/size/checksum header |
| `A2AProtocol` pending requests | `DeltaConsensus` + `VectorClock` | ruvector-delta-consensus | Causal ordering replaces manual correlation tracking |
| `TopicRouter` | `ShardedEventBus` | ruvector-nervous-system | High-throughput sharded delivery |
| Per-agent rate limiter | `BudgetGuardrail` | ruvector-nervous-system | Resource-aware routing |
| `WasmToolRunner` (wasmtime only) | `rvf-wasm` (5.5KB) + wasmtime | rvf-wasm | Dual runtime: micro + full |
| `ContainerManager` (Docker/bollard) | RVF container builder | rvf-kernel + rvf-wire | Single-file containers; 125ms boot |
| Docker health checks | `rvf-crypto` witness attestation | rvf-crypto | Cryptographic proof of health |
| No audit trail | `WitnessChain` (SHAKE-256) | ruvector-cognitive-container + rvf-crypto | Hash-linked audit trail for every decision |
| No learning | `SonaEngine` (MicroLoRA + EwcPlusPlus) | sona | Self-learning with catastrophic forgetting prevention |
| No neural routing | `Router` (FastGRNN) | ruvector-tiny-dancer-core | Sub-ms learned routing with circuit breaker |
| No coherence scoring | `SheafLaplacian` | prime-radiant | Quality/health scoring engine |
| No MCP gate | `mcp-gate` tools | mcp-gate | Expose coherence gate as MCP-callable tools |
| `ContainerSnapshot` | `ContainerSnapshot` | ruvector-cognitive-container | State serialization/checkpoint (same type) |
| No conflict resolution | `ConflictStrategy` (LWW/Merge/Custom) | ruvector-delta-consensus | Configurable CRDT conflict resolution |
| No gossip protocol | `DeltaGossip` | ruvector-delta-consensus | Gossip-based pub/sub dissemination |

---

## 5. Wire Format Migration

### JSON-RPC to rvf-wire Segments

The original K2 plan uses JSON-RPC for IPC between agents. This is replaced by
rvf-wire segments.

#### Why migrate

| Property | JSON-RPC | rvf-wire |
|---|---|---|
| Alignment | None | 64-byte SIMD |
| Crash safety | No (incomplete JSON is unrecoverable) | Yes (append-only, each segment self-contained) |
| Overhead | Variable (JSON text) | Fixed 64-byte header |
| Checksum | None | Per-segment CRC |
| Zero-copy parsing | No (requires full deserialization) | Yes (memory-mapped access to segment data) |
| Type safety | Schema via convention | Type field in header |

#### Migration path

1. **Phase 1**: `KernelMessage` payload is serialized to JSON, then wrapped in a
   `WireSegment`. Deserialization still goes through JSON. This is the compatibility layer.

2. **Phase 2**: High-frequency message types (ToolCall, ToolResult) get dedicated
   segment types with binary serialization. JSON remains for Text and custom payloads.

3. **Phase 3**: All message types have binary segment representations. JSON fallback
   retained for debugging and external interop.

#### Segment format for IPC messages

```
Offset  Size  Field
0       2     segment_type (u16): IPC_DIRECT=0x0100, IPC_TOPIC=0x0101, IPC_BROADCAST=0x0102
2       2     flags (u16): HAS_CORRELATION=0x01, REQUIRES_ACK=0x02
4       4     payload_length (u32)
8       8     from_pid (u64)
16      8     to_pid_or_topic_hash (u64)
24      8     timestamp (u64, nanos since epoch)
32      16    correlation_id (u128, optional, zeros if none)
48      4     vector_clock_len (u32)
52      4     checksum (u32, CRC32 of payload)
56      8     reserved (padding to 64 bytes)
--      var   payload (payload_length bytes, 64-byte aligned)
--      var   vector_clock (vector_clock_len bytes)
```

#### Backward compatibility

When `ruvector-wire` feature is disabled, IPC uses the original JSON-based
`KernelMessage` format. The `KernelIpc` trait abstracts over both:

```rust
pub trait IpcTransport: Send + Sync {
    async fn send(&self, msg: KernelMessage) -> Result<DeliveryStatus>;
    async fn recv(&self, pid: Pid) -> Result<KernelMessage>;
}

#[cfg(feature = "ruvector-wire")]
pub struct WireIpcTransport { /* rvf-wire based */ }

#[cfg(not(feature = "ruvector-wire"))]
pub struct JsonIpcTransport { /* JSON-RPC based, original plan */ }
```

---

## 6. Container Format

### RVF Replaces Docker/Alpine for Agent Containers

#### RVF container structure

An RVF container is a single file with the following segment layout:

```
[Header: magic, version, segment count, total size]
[Segment 0: KERNEL -- bzImage, ~2MB compressed]
[Segment 1: INITRAMFS -- minimal initrd, ~500KB]
[Segment 2: WASM_MODULE -- agent tool 1, variable]
[Segment 3: WASM_MODULE -- agent tool 2, variable]
[Segment 4: HNSW_INDEX -- vector index, variable]
[Segment 5: WITNESS_CHAIN -- audit trail, append-only]
[Segment 6: CONFIG -- agent configuration, JSON]
[Segment 7: SIGNATURE -- ML-DSA-65 + Ed25519 dual signature]
```

#### Comparison with Docker

| Property | Docker (original K4) | RVF (ruvector) |
|---|---|---|
| Format | Layered filesystem (OCI image) | Segment-based binary file |
| Boot time | 2-10 seconds | 125ms |
| Daemon required | Yes (dockerd) | No |
| Image size | 50MB+ (Alpine base) | 3-5MB (bzImage + initramfs) |
| Crash safety | Filesystem-dependent | Append-only segments, inherently crash-safe |
| Attestation | None (content trust is opt-in) | Built-in ML-DSA-65/Ed25519 dual-signing |
| Updates | Layer pull + rebuild | Segment append (add new WASM, update config) |
| Composability | docker-compose | RVF manifest within container file |
| Debugging | `docker exec`, `docker logs` | Segment inspection, witness chain replay |
| Distribution | Docker registry | File copy, HTTP, IPFS (any content-addressable store) |

#### When to use each

| Use case | Format | Reason |
|---|---|---|
| WeftOS agent containers | RVF | Native format, fastest boot, smallest size, cryptographic attestation |
| Third-party sidecars (Redis, Postgres) | Docker | Already packaged as Docker images |
| CI/CD pipeline | Docker | Ecosystem tooling (GitHub Actions, etc.) |
| Edge deployment | RVF | No Docker daemon required; single binary |
| Air-gapped deployment | RVF | Single file, dual-signed, verifiable |

#### Container manifest (replaces docker-compose.yml for RVF)

```toml
# weftapp.toml container section (extended for RVF)

[container]
format = "rvf"  # or "docker" for fallback
boot_timeout_ms = 200
signing_algorithm = "ml-dsa-65+ed25519"

[[container.wasm_modules]]
name = "diff-analyzer"
path = "tools/diff-analyzer.wasm"
runtime = "micro"  # or "full" for wasmtime

[[container.wasm_modules]]
name = "code-formatter"
path = "tools/code-formatter.wasm"
runtime = "full"

[container.kernel]
memory_mb = 64
cpus = 1

# Docker sidecars (not in RVF container; run alongside)
[[sidecars]]
name = "review-db"
format = "docker"
image = "redis:7-alpine"
ports = [{ host = 6380, container = 6379 }]
```

---

## 7. New Capabilities Unlocked

Capabilities that are not in the original K0-K5 plan and become available through
ruvector integration:

### 7.1 Self-Learning Routing (sona + tiny-dancer)

Every routing decision (which agent handles a task, which model processes a query)
is recorded as a trajectory. SONA's MicroLoRA performs rank-1 adaptation in <1ms
per request. Over time, routing becomes increasingly accurate without manual tuning.

**Impact**: Agents get smarter about delegation. A code-review app learns which
reviewer agent handles which language best. A multi-model pipeline learns which
model tier produces the best results for which query type.

### 7.2 Coherence Scoring (prime-radiant)

The Sheaf Laplacian engine provides a mathematical measure of system health that
goes beyond binary healthy/unhealthy. It computes coherence across subsystem
boundaries, detecting subtle degradation patterns before they become failures.

**Impact**: `weave kernel status` shows not just "running" but a coherence score.
Applications with low coherence get flagged before users notice problems.

### 7.3 Causal Ordering (ruvector-delta-consensus)

Vector clocks on IPC messages guarantee causal ordering. If agent A sends a message
to agent B, and B sends a response, the response is always delivered after the
original message, even under concurrent load. CRDT-based delta synchronization
handles the common case without coordination overhead.

**Impact**: No more message reordering bugs in multi-agent workflows. Tool call
results always arrive after the corresponding tool call request.

### 7.4 Hash-Chained Audit Trail (rvf-crypto + WitnessChain)

Every capability decision, IPC message, and container lifecycle event is recorded
in a SHAKE-256 hash-linked witness chain. Each entry references the previous,
forming a tamper-evident log.

**Impact**: Full audit trail of what every agent did, which tools it called, which
permissions it was granted, and which messages it sent. Tamper detection is automatic.

### 7.5 Three-Way Capability Decisions (cognitum-gate)

Binary Permit/Deny becomes Permit/Defer/Deny. Defer enables:
- Escalation to supervisor agent when a tool call is ambiguous
- Human-in-the-loop approval for sensitive operations
- Graduated trust models where new agents start with Defer-by-default

**Impact**: More nuanced security model. An agent that has never used `shell_exec`
gets Defer (asking the human) instead of a hard Deny.

### 7.6 Budget-Aware Routing (nervous system)

The BudgetGuardrail prevents routing messages to overloaded agents. The
OscillatoryRouter uses phase-locked patterns for predictable load distribution.

**Impact**: No more message storms to a single agent. Load balancing is automatic
and resource-aware.

### 7.7 Epoch-Budgeted Execution (cognitive container)

Instead of hard-killing agents that exceed resource limits, the EpochController
provides partial results on budget exhaustion. An agent that runs out of CPU
budget mid-computation returns what it has computed so far.

**Impact**: Graceful degradation instead of lost work. A code analysis that runs
out of budget returns partial results (first 50 files analyzed) instead of nothing.

### 7.8 Sub-Millisecond WASM Tools (rvf-wasm)

The 5.5KB no-allocator WASM runtime enables micro-tools that execute in <1ms.
This is 1000x faster than wasmtime cold-start for simple operations like
string transforms, JSON filters, or hash computations.

**Impact**: Simple tools (capitalize, format, filter) run at native speed.
Complex tools (code analysis, compilation) still use wasmtime.

### 7.9 Single-File Containers (rvf-kernel)

RVF containers are single files containing everything needed to run an agent:
kernel, WASM tools, vector indexes, witness chains, and configuration. 125ms boot.

**Impact**: Deploy an agent by copying a single file. No Docker registry, no
layer resolution, no daemon. Air-gapped deployment is trivial.

### 7.10 Pattern Memory (sona ReasoningBank)

SONA's ReasoningBank extracts patterns from execution trajectories and stores
them for similarity search. When a new task arrives, the bank finds similar
past tasks and their outcomes.

**Impact**: Agents learn from past mistakes. A deployment agent that failed
on a specific error pattern recognizes it next time and adjusts its approach.

---

## 8. Dependency Tree Impact

### Binary Size Impact (estimated)

| Feature Configuration | Added Size | Total Binary | Use Case |
|---|---|---|---|
| Base (no ruvector) | -- | ~5 MB | Minimal WeftOS kernel |
| + ruvector-cluster | ~200 KB | ~5.2 MB | Service registry + health |
| + ruvector-supervisor | ~350 KB | ~5.5 MB | + cognitive container + gate |
| + ruvector-ipc | ~400 KB | ~5.9 MB | + delta consensus + event bus |
| + ruvector-crypto | ~100 KB | ~6.0 MB | + witness chains + signing |
| + ruvector-wire | ~80 KB | ~6.1 MB | + wire format |
| **ruvector-core bundle** | **~1.1 MB** | **~6.1 MB** | **All core subsystems** |
| + ruvector-wasm | ~30 KB | ~6.1 MB | + micro WASM runtime |
| + ruvector-containers | ~300 KB | ~6.4 MB | + RVF kernel builder |
| + ruvector-apps | ~900 KB | ~7.3 MB | + tiny-dancer + sona + prime-radiant + mcp-gate |
| **ruvector-full bundle** | **~2.3 MB** | **~7.3 MB** | **Everything** |

**Comparison**: Original plan with wasmtime (~20 MB) + bollard (~2 MB) = ~27 MB.
ruvector-full at ~7.3 MB is significantly smaller because rvf-wasm (5.5KB) replaces
wasmtime for micro-tools and rvf-kernel replaces Docker for containers.

### Compile Time Impact (estimated)

| Feature | Added Compile Time | Notes |
|---|---|---|
| ruvector-cluster | ~5s | DashMap already in workspace |
| ruvector-cognitive-container | ~8s | Pure Rust, no C deps |
| cognitum-gate-tilezero | ~6s | Pure Rust |
| ruvector-delta-consensus | ~10s | CRDT implementation |
| ruvector-nervous-system | ~7s | Event bus + routing |
| rvf-wire | ~3s | Serialization only |
| rvf-wasm | ~2s | Minimal, no allocator |
| rvf-kernel | ~15s | Kernel builder (only needed for container builds) |
| rvf-crypto | ~8s | SHAKE-256 + Ed25519 |
| sona | ~12s | MicroLoRA + EwcPlusPlus |
| ruvector-tiny-dancer-core | ~10s | FastGRNN |
| prime-radiant | ~6s | Sheaf Laplacian |
| mcp-gate | ~4s | MCP server wrapper |
| **Total (ruvector-full)** | **~96s** | **Clean build only; incremental is fast** |

### Feature Gate Strategy

```
clawft-kernel feature tree:

[default]  -- no ruvector deps, original K0-K5 implementations
    |
    +-- [ruvector-cluster]  -- K0: cluster-based service registry
    |
    +-- [ruvector-supervisor]  -- K1: cognitive container + gate
    |       |
    |       +-- [ruvector-crypto]  -- cross-cutting: witness audit trail
    |
    +-- [ruvector-ipc]  -- K2: delta consensus + event bus
    |       |
    |       +-- [ruvector-wire]  -- K2: binary wire format (replaces JSON-RPC)
    |
    +-- [ruvector-wasm]  -- K3: micro WASM runtime (alongside wasmtime)
    |
    +-- [ruvector-containers]  -- K4: RVF container format
    |
    +-- [ruvector-apps]  -- K5: self-learning + coherence
    |
    +-- [ruvector-core]  -- bundle: cluster + supervisor + ipc + crypto + wire
    |
    +-- [ruvector-full]  -- bundle: core + wasm + containers + apps
```

---

## 9. Risk Assessment

| ID | Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|---|
| R1 | ruvector crates not published on crates.io | Certain | -- | Use git deps with pinned revisions (`rev = "abc123"`). If ruvector repo disappears, vendor the crates into `vendor/` directory |
| R2 | ruvector API changes break clawft | High | High (pre-1.0) | Pin exact git commits. All ruvector types hidden behind clawft adapter layer. Clawft's public API never exposes ruvector types |
| R3 | ruvector crate does not compile on Rust 1.93 | High | Medium | Sprint 0 validation gate: compile all 13 ruvector crates before any integration work. Block on this |
| R4 | rvf-kernel requires kernel build toolchain (cross-compile) | Medium | High | Feature-gate `ruvector-containers` separately. Most users use `ruvector-core` without container building. Document toolchain requirements |
| R5 | cognitum-gate Defer decision complicates existing binary logic | Medium | Medium | Provide `into_binary()` helper that converts Defer to Deny for backward compatibility. Defer handling is opt-in per agent |
| R6 | Delta consensus overhead for simple PID-to-PID messages | Low | Medium | Fast path: direct delivery for PID-to-PID without vector clock. Consensus only for topic/broadcast messages |
| R7 | rvf-wasm C-ABI surface area too limited for clawft's needs | Medium | Medium | rvf-wasm has 14 exports covering vector ops. For non-vector operations, fall back to wasmtime. Document the boundary clearly |
| R8 | SONA learning produces bad routing decisions | Medium | Low | EwcPlusPlus prevents catastrophic forgetting. Circuit breaker catches cascading failures. Manual override always available |
| R9 | RVF container format adoption barrier (unfamiliar to users) | Medium | High | Docker remains available as fallback. Provide `weft container convert --from docker --to rvf` migration tool. Document extensively |
| R10 | Binary size exceeds 10 MB with ruvector-full | Low | Low | Estimated 7.3 MB. Monitor with `twiggy top` per sprint. Feature gates allow selective inclusion |
| R11 | Witness chain grows unbounded | Medium | Medium | Implement segment compaction in rvf-crypto. Configurable retention policy (default 30 days). Compact on boot |
| R12 | WASM browser target breaks | High | Low | No ruvector crate is a dependency of `clawft-wasm`. CI gate: `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser` |

---

## 10. Updated Crate Structure

### clawft-kernel/Cargo.toml (with ruvector deps)

See Section 2 for the full Cargo.toml. Summary of dependency groups:

| Feature Gate | Dependencies | Phase |
|---|---|---|
| `ruvector-cluster` | ruvector-cluster | K0 |
| `ruvector-supervisor` | ruvector-cognitive-container, cognitum-gate-tilezero | K1 |
| `ruvector-ipc` | ruvector-delta-consensus, ruvector-nervous-system | K2 |
| `ruvector-wire` | rvf-wire | K2 |
| `ruvector-wasm` | rvf-wasm | K3 |
| `wasm-sandbox` | wasmtime | K3 |
| `ruvector-containers` | rvf-wire, rvf-kernel, rvf-crypto | K4 |
| `containers-docker` | bollard | K4 (fallback) |
| `ruvector-crypto` | rvf-crypto | Cross-cutting |
| `ruvector-apps` | ruvector-tiny-dancer-core, sona, prime-radiant, mcp-gate | K5 |

### Updated file structure

```
crates/clawft-kernel/
  Cargo.toml
  src/
    lib.rs                     -- crate root, conditional re-exports
    boot.rs                    -- Kernel boot sequence (modified for ruvector)
    process.rs                 -- Process table (unchanged)
    config.rs                  -- KernelConfig types (unchanged)

    -- Service layer (K0) --
    service.rs                 -- SystemService trait (unchanged interface)
    service_adapter.rs         -- NEW: ClusterServiceRegistry adapter (ruvector-cluster)

    -- Capability layer (K1) --
    capability.rs              -- AgentCapabilities (unchanged interface)
    capability_adapter.rs      -- NEW: GateCapabilityChecker adapter (cognitum-gate)
    budget.rs                  -- NEW: BudgetedAgent wrapper (cognitive-container)
    supervisor.rs              -- AgentSupervisor (modified to use gate + budget)

    -- IPC layer (K2) --
    ipc.rs                     -- KernelIpc trait (unchanged interface)
    ipc_adapter.rs             -- NEW: RuvectorIpc adapter (delta-consensus + nervous-system)
    wire.rs                    -- NEW: WireSegment serialization (rvf-wire)
    a2a.rs                     -- A2A protocol (modified to use RuvectorIpc)
    topic.rs                   -- TopicRouter (replaced by ShardedEventBus adapter)

    -- WASM layer (K3) --
    wasm_runner.rs             -- WasmToolRunner with wasmtime (unchanged)
    wasm_micro.rs              -- NEW: MicroWasmRunner (rvf-wasm)
    wasm_dispatch.rs           -- NEW: WasmDispatcher (routes to micro or full)

    -- Container layer (K4) --
    container.rs               -- ContainerManager trait (unchanged interface)
    container_rvf.rs           -- NEW: RvfContainerManager (rvf-kernel + rvf-wire)
    container_docker.rs        -- DockerContainerManager (original bollard, renamed)

    -- App framework (K5) --
    app.rs                     -- AppManager, AppManifest (unchanged interface)
    app_intelligence.rs        -- NEW: IntelligentAppRuntime (sona + tiny-dancer + prime-radiant)

    -- Cross-cutting --
    witness.rs                 -- NEW: WitnessChain adapter (rvf-crypto)
    health.rs                  -- HealthSystem (modified for coherence scoring)
```

### Module visibility with feature gates

```rust
// lib.rs

pub mod boot;
pub mod process;
pub mod config;
pub mod service;
pub mod capability;
pub mod health;
pub mod ipc;

// K1: Supervisor
pub mod supervisor;

// K2: IPC
pub mod a2a;
pub mod topic;

// ruvector adapters (feature-gated)
#[cfg(feature = "ruvector-cluster")]
pub mod service_adapter;

#[cfg(feature = "ruvector-supervisor")]
pub mod capability_adapter;
#[cfg(feature = "ruvector-supervisor")]
pub mod budget;

#[cfg(feature = "ruvector-ipc")]
pub mod ipc_adapter;
#[cfg(feature = "ruvector-wire")]
pub mod wire;

#[cfg(feature = "ruvector-wasm")]
pub mod wasm_micro;
pub mod wasm_dispatch;

#[cfg(feature = "wasm-sandbox")]
pub mod wasm_runner;

#[cfg(feature = "ruvector-containers")]
pub mod container_rvf;
#[cfg(feature = "containers-docker")]
pub mod container_docker;
pub mod container;

pub mod app;
#[cfg(feature = "ruvector-apps")]
pub mod app_intelligence;

#[cfg(feature = "ruvector-crypto")]
pub mod witness;
```

---

## 11. Learning as a Cross-Cutting Concern

### Why wire learning at K0, not K5

The original plan mentions SONA only for K5. But learning applies everywhere:

| Subsystem | What learns | Benefit |
|---|---|---|
| K0 Service Registry | Which services degrade under load | Predictive health warnings |
| K1 Capability Gate | Which Defer decisions get approved | Gradually reduces human-in-the-loop |
| K2 IPC Router | Which message patterns cause congestion | Adaptive routing around bottlenecks |
| K3 WASM Dispatch | Which tools are micro vs full | Auto-classifies new tools |
| K4 Container Boot | Which segments to preload | Faster cold starts |
| K5 App Routing | Which agent handles which task best | Improves over time without configuration |

### Architecture for cross-cutting learning

```rust
/// Trait for components that can learn from outcomes.
pub trait Learnable: Send + Sync {
    /// Start tracking an execution trajectory.
    fn trajectory_start(&self, id: &str);

    /// Record a step in the trajectory.
    fn trajectory_step(&self, id: &str, outcome: &dyn std::any::Any);

    /// Complete the trajectory and trigger learning.
    fn trajectory_end(&self, id: &str, success: bool);
}

/// SONA-backed learning implementation.
#[cfg(feature = "ruvector-apps")]
pub struct SonaLearner {
    engine: sona::SonaEngine,
    reasoning_bank: sona::ReasoningBank,
}

#[cfg(feature = "ruvector-apps")]
impl Learnable for SonaLearner {
    fn trajectory_start(&self, id: &str) {
        self.engine.trajectory_start(id);
    }

    fn trajectory_step(&self, id: &str, outcome: &dyn std::any::Any) {
        // MicroLoRA rank-1 instant adaptation
        self.engine.micro_lora_step(id, outcome);
    }

    fn trajectory_end(&self, id: &str, success: bool) {
        self.engine.trajectory_end(id);
        if success {
            // Extract pattern and store in reasoning bank
            self.reasoning_bank.store_pattern(id, self.engine.last_trajectory(id));
        }
    }
}

/// No-op learner when ruvector-apps is not enabled.
#[cfg(not(feature = "ruvector-apps"))]
pub struct NoopLearner;

#[cfg(not(feature = "ruvector-apps"))]
impl Learnable for NoopLearner {
    fn trajectory_start(&self, _id: &str) {}
    fn trajectory_step(&self, _id: &str, _outcome: &dyn std::any::Any) {}
    fn trajectory_end(&self, _id: &str, _success: bool) {}
}
```

The `Kernel<P>` struct holds an `Arc<dyn Learnable>` that is injected into every
subsystem. When `ruvector-apps` is enabled, this is a `SonaLearner`. Otherwise,
it is a `NoopLearner` that compiles to nothing.

---

## 12. Implementation Order

### Revised Phase Dependencies

```
K0 (Foundation + ruvector-cluster)  -- 2 weeks
  |
  +---> K1 (Supervisor + cognitum-gate + cognitive-container)  -- 2 weeks
  |         |
  |         +---> K2 (IPC + delta-consensus + nervous-system + rvf-wire)  -- 2 weeks
  |                   |
  |                   +---> K5 (App Framework + sona + tiny-dancer + prime-radiant)  -- 2 weeks
  |
  +---> K3 (WASM + rvf-wasm + wasmtime)  -- 2 weeks [parallel with K1]
  |
  +---> K4 (Containers + rvf-kernel)  -- 1 week [parallel with K1]
  |
  +---> Witness Chain (rvf-crypto)  -- wired at K0, used by K1-K5 [cross-cutting]
  |
  +---> Learning (sona)  -- wired at K0 as NoopLearner, activated at K5 [cross-cutting]
```

### Sprint 0: Dependency Validation Gate

Before any integration work begins:

1. Compile all 13 ruvector crates against Rust 1.93
2. Validate public API surfaces match this document
3. Measure binary size delta for each crate
4. Confirm WASM build is unaffected
5. Document any API mismatches requiring plan corrections

**This is a hard go/no-go gate. If any crate does not compile, block and resolve.**

---

## 13. Testing Contracts

### Feature Flag Test Matrix

| Build Configuration | Command | Must Pass |
|---|---|---|
| No ruvector (default) | `scripts/build.sh test` | All existing tests |
| ruvector-cluster only | `cargo test -p clawft-kernel --features ruvector-cluster` | Service registry adapter tests |
| ruvector-supervisor only | `cargo test -p clawft-kernel --features ruvector-supervisor` | Gate + budget tests |
| ruvector-ipc only | `cargo test -p clawft-kernel --features ruvector-ipc` | Delta consensus + event bus tests |
| ruvector-core bundle | `cargo test -p clawft-kernel --features ruvector-core` | All core subsystem tests |
| ruvector-full bundle | `cargo test -p clawft-kernel --features ruvector-full` | All tests including apps + containers |
| WASM (browser) | `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser` | Must not pull any ruvector dep |

### Adapter Tests

Each adapter module has tests that verify:

1. The adapter correctly wraps the ruvector API
2. Errors from ruvector are mapped to clawft error types
3. The adapter is API-compatible with the original (non-ruvector) implementation
4. Feature-gating compiles cleanly in both enabled and disabled states

### Integration Tests

| Test | Phases | Description |
|---|---|---|
| `kernel_boot_ruvector` | K0 | Boot with ClusterServiceRegistry, verify coherence scoring |
| `gate_three_way` | K1 | Capability check returns Permit, Defer, and Deny correctly |
| `budget_exhaustion` | K1 | Agent returns partial results when epoch budget exhausted |
| `causal_ordering` | K2 | Messages delivered in causal order under concurrent load |
| `wire_format_roundtrip` | K2 | KernelMessage -> WireSegment -> KernelMessage lossless |
| `dual_wasm_dispatch` | K3 | Micro tool uses rvf-wasm, full tool uses wasmtime |
| `witness_chain_integrity` | Cross | Tampered witness entry detected by chain verification |
| `learning_trajectory` | K5 | SONA records trajectory and improves routing over 100 iterations |

---

## 14. Definition of Done

### For this architecture document

- [x] Integration strategy justified with trade-offs
- [x] Every ruvector crate mapped to a feature gate
- [x] Phase-by-phase delta from original plan documented
- [x] Type mapping table complete
- [x] Wire format migration path defined
- [x] Container format decision made with comparison
- [x] New capabilities enumerated
- [x] Binary size and compile time estimated
- [x] Risk assessment with mitigations
- [x] Updated crate structure defined
- [x] Feature gate tree documented
- [x] Cross-cutting concerns (learning, audit) addressed
- [x] Test contracts defined

### For implementation (future phases)

- [ ] Sprint 0 validation gate passes (all 13 ruvector crates compile)
- [ ] Each adapter module has unit tests
- [ ] Feature flag test matrix passes for all configurations
- [ ] Binary size within estimates (ruvector-full < 8 MB)
- [ ] WASM browser build unaffected
- [ ] No ruvector type appears in clawft-kernel public API
- [ ] Existing tests pass with no ruvector features enabled
