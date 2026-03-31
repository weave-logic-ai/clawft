# Track 1: Code Pattern Extraction Workshop

**Chair**: code-analyzer
**Panelists**: code-analyzer, sparc-coder, reviewer, researcher, weaver
**Date**: 2026-03-27
**Branch**: `feature/weftos-kernel-sprint`
**Codebase**: 22 crates, 181,703 lines Rust, 5,040 tests

---

## Exercise 1: The "Registry" Pattern

### 1.1 Inventory of All Registry Implementations

We found **15 distinct Registry structs** across 6 crates. Every single one follows the same shape: a HashMap/DashMap keyed by String, with `new()`, `register()`, `get()`, and `list()` methods.

| # | Registry | Crate | File:Line | Backing Store | Thread-Safe |
|---|----------|-------|-----------|---------------|-------------|
| 1 | `ServiceRegistry` | clawft-kernel | `service.rs:150` | `DashMap<String, Arc<dyn SystemService>>` + `DashMap<String, ServiceEntry>` | Yes (DashMap) |
| 2 | `ToolRegistry` (kernel) | clawft-kernel | `wasm_runner.rs:1100` | `HashMap<String, Arc<dyn BuiltinTool>>` | No (requires `&mut self`) |
| 3 | `ToolRegistry` (core) | clawft-core | `tools/registry.rs:338` | `HashMap<String, Arc<dyn Tool>>` + metadata map | No (requires `&mut self`) |
| 4 | `MetricsRegistry` | clawft-kernel | `metrics.rs:201` | `DashMap<String, AtomicU64>` + gauges + histograms | Yes (DashMap + atomics) |
| 5 | `MonitorRegistry` | clawft-kernel | `monitor.rs:68` | `DashMap<Pid, Vec<Pid>>` + `DashMap<Pid, Vec<ProcessMonitor>>` | Yes (DashMap) |
| 6 | `NamedPipeRegistry` | clawft-kernel | `named_pipe.rs:98` | `DashMap<String, NamedPipe>` | Yes (DashMap) |
| 7 | `ClusterServiceRegistry` | clawft-kernel | `mesh_service_adv.rs:34` | `HashMap<String, Vec<ServiceAdvertisement>>` | No |
| 8 | `RegistryQueryService` | clawft-kernel | `mesh_service.rs:136` | `Vec<String>` (methods only) | No (stateless) |
| 9 | `ProcessTable` | clawft-kernel | `process.rs:108` | `DashMap<Pid, ProcessEntry>` + `AtomicU64` | Yes (DashMap + atomic) |
| 10 | `AgentRegistry` | clawft-core | `agent/agents.rs:243` | `HashMap<String, AgentDefinition>` | No |
| 11 | `SkillRegistry` | clawft-core | `agent/skills_v2.rs:312` | `HashMap<String, SkillDefinition>` | No |
| 12 | `PipelineRegistry` | clawft-core | `pipeline/traits.rs:397` | `HashMap<TaskType, Pipeline>` + default | No |
| 13 | `VoiceCommandRegistry` | clawft-plugin | `voice/commands.rs:33` | `Vec<VoiceCommand>` + `HashMap<String, usize>` index | No |
| 14 | `WorkspaceRegistry` | clawft-types | `workspace.rs:66` | `Vec<WorkspaceEntry>` (serializable) | No |
| 15 | `SlashCommandRegistry` | clawft-cli | `interactive/registry.rs:74` | `HashMap<String, Box<dyn SlashCommand>>` | No |

### 1.2 Common Interface Analysis

Every registry implements some subset of these operations:

| Operation | Registries That Implement It | Notes |
|-----------|------------------------------|-------|
| `new()` | 15/15 | Universal constructor |
| `register(&self/&mut self, item)` | 14/15 | Insert by name/key |
| `get(key) -> Option<V>` | 13/15 | Lookup by key |
| `list() -> Vec<...>` | 12/15 | Enumerate all entries |
| `has(key) -> bool` | 4/15 | Existence check (most use `get().is_some()`) |
| `unregister/remove(key)` | 5/15 | ServiceRegistry, NamedPipeRegistry, MonitorRegistry, WorkspaceRegistry, ProcessTable |
| `health() / health_all()` | 2/15 | ServiceRegistry, RegistryQueryService |
| `Default` impl | 10/15 | Delegates to `new()` |

### 1.3 Proposed Common Abstraction

```rust
/// Trait for registry-like structures with name-based lookup.
///
/// This does NOT replace domain-specific registries; it provides
/// a common interface for introspection, health monitoring, and
/// the GUI's registry browser view.
pub trait Registry {
    /// The key type (usually String or Pid).
    type Key: Clone + std::fmt::Display;
    /// The value type.
    type Value;
    /// Human-readable name for this registry kind (e.g. "services", "tools").
    fn registry_name(&self) -> &str;
    /// Number of entries.
    fn len(&self) -> usize;
    /// Lookup by key.
    fn get(&self, key: &Self::Key) -> Option<Self::Value>;
    /// List all keys.
    fn keys(&self) -> Vec<Self::Key>;
    /// Whether the registry is empty.
    fn is_empty(&self) -> bool { self.len() == 0 }
}
```

**Two key variants** should be provided:

1. **`MutableRegistry`** -- adds `register()` and `unregister()` for registries that accept runtime additions (tools, services, agents).
2. **`ConcurrentRegistry`** -- marker for DashMap-backed registries that are `Send + Sync` without external locking.

### 1.4 Recommendation

- **Extract to**: `clawft-types` (trait definition only, no DashMap dependency)
- **Action**: Define the trait; impl it on existing registries one at a time. This enables the GUI's "Registry Browser" view (Sprint 11 K8) to display all registries through a uniform interface.
- **Crates that benefit**: 6 (kernel, core, plugin, types, cli, services) -- all 15 registries.
- **Score**: 6/22 crates

---

## Exercise 2: The "Event -> Chain -> Log" Pattern

### 2.1 Event Creation Points

Events are created via `ChainManager::append(source, kind, payload)`. The `ChainManager` wraps a `Mutex<ExoChainInner>` (file: `chain.rs:575`).

Key event producers found in the codebase:

| Source | Kind | File | Description |
|--------|------|------|-------------|
| `"kernel"` | `"boot"` | `boot.rs` | Kernel boot event |
| service name | `"service.contract.register"` | `service.rs:383` | Service contract anchored on-chain |
| `"kernel"` | `"wasm.tool.*"` | `wasm_runner.rs:3431` | WASM tool execution logged |
| various | `"tree.*"` | `tree_manager.rs:164-951` | ExoResourceTree mutations (14 call sites) |
| `"gate.*"` | `"gate.permit/deny/defer"` | `gate.rs` (TileZeroGate) | Governance decisions |

### 2.2 Event Envelope: `ChainEvent`

The common event envelope is `ChainEvent` (file: `chain.rs:141`):

```rust
pub struct ChainEvent {
    pub sequence: u64,           // Monotonic, 0 = genesis
    pub chain_id: u32,           // 0 = local
    pub timestamp: DateTime<Utc>,
    pub prev_hash: [u8; 32],     // SHAKE-256 chain link
    pub hash: [u8; 32],          // SHAKE-256 of all fields
    pub payload_hash: [u8; 32],  // SHAKE-256 of payload bytes
    pub source: String,          // Who emitted (e.g. "kernel", service name)
    pub kind: String,            // What happened (e.g. "boot", "tool.exec")
    pub payload: Option<serde_json::Value>,
}
```

### 2.3 Flow: Event -> ExoChain -> Persistence

```
Producer (service, tool, gate, tree_manager)
  |
  v
ChainManager::append(source, kind, payload)   [chain.rs:575]
  |
  +---> ExoChainInner::append()                [chain.rs:343]
  |       - Computes payload_hash (SHAKE-256)
  |       - Computes event hash (SHAKE-256 of all fields)
  |       - Creates WitnessEntry
  |       - Auto-checkpoints every N events
  |
  +---> Returns ChainEvent (clone)
  |
  v
SQLite persistence (optional, via persistence.rs)
  +---> On shutdown: save chain to disk
  +---> On boot: reload from disk
```

### 2.4 What Gets Logged vs What Doesn't

**Logged on-chain**:
- Service contract registrations
- WASM tool executions
- ExoResourceTree mutations (all 14 operations)
- Gate decisions (TileZero feature)
- Boot events
- Mesh chain replication events

**NOT logged on-chain (gap)**:
- Process spawns/exits (only in ProcessTable, not chain-logged)
- IPC message delivery/failure (goes to DLQ, not chain)
- DEMOCRITUS tick results (logged via tracing, not chain)
- Governance evaluations from GovernanceEngine (only TileZeroGate chain-logs)
- Metric changes
- Supervisor restart events

### 2.5 Recommendation

The `tree_manager.rs` has 14 `chain.append()` call sites -- the heaviest chain user by far. The pattern is always:

```rust
self.chain.append("source", "kind.subkind", Some(serde_json::json!({...})));
```

**Proposed**: Create a `ChainLoggable` trait:

```rust
pub trait ChainLoggable {
    fn chain_source(&self) -> &str;
    fn chain_kind(&self) -> &str;
    fn chain_payload(&self) -> Option<serde_json::Value>;
}
```

Then a single `chain.log(event: &impl ChainLoggable)` method replaces the raw append calls. This would:
- Reduce the 14 call sites in tree_manager.rs to structured event types
- Enable typed event catalogs for the GUI Chain Viewer
- Score: 3/22 crates (kernel, weftos, services)

---

## Exercise 3: The "Supervisor -> Restart -> Budget" Pattern

### 3.1 Component Map

All self-healing infrastructure lives in `clawft-kernel` behind the `os-patterns` feature gate.

| Component | File:Line | Purpose |
|-----------|-----------|---------|
| `RestartStrategy` | `supervisor.rs:33` | Enum: OneForOne, OneForAll, RestForOne, Permanent, Transient |
| `RestartBudget` | `supervisor.rs:52` | Struct: `max_restarts: u32`, `within_secs: u64` (default 5/60) |
| `RestartTracker` | `supervisor.rs:72` | Per-agent state: count, window_start, last_restart, backoff_ms |
| `RestartTracker::next_backoff_ms()` | `supervisor.rs:98` | Exponential: 100ms * 2^(n-1), capped at 30s |
| `RestartTracker::record_restart()` | `supervisor.rs:116` | Increments count, checks window, returns within-budget bool |
| `RestartTracker::should_restart()` | `supervisor.rs:150` | Static: Permanent=never, Transient=on-nonzero, others=always |
| `ResourceCheckResult` | `supervisor.rs:172` | Enum: Ok, Warning (80-99%), Exceeded (>=100%) |
| `check_resource_usage()` | `supervisor.rs:191` | Checks memory, cpu_time, messages against limits |
| `AgentSupervisor<P>` | `supervisor.rs` (later) | Orchestrates spawns, stops, restarts via ProcessTable |
| `ProcessLink` | `monitor.rs:20` | Bidirectional PID link (Erlang-style) |
| `ProcessMonitor` | `monitor.rs:42` | Unidirectional watcher with ref_id |
| `MonitorRegistry` | `monitor.rs:68` | DashMap-backed link/monitor registry |
| `MonitorRegistry::process_exited()` | `monitor.rs:144` | Delivers link signals + ProcessDown notifications |
| `ProcessTable` | `process.rs:108` | PID allocation + DashMap of ProcessEntry |
| `ProcessState` | `process.rs:21` | FSM: Starting -> Running -> Suspended -> Stopping -> Exited |
| `ProcessState::can_transition_to()` | `process.rs:43` | Validates state transitions |

### 3.2 Restart Flow

```
Agent crash detected
  |
  v
MonitorRegistry::process_exited(pid, reason)
  |
  +---> Delivers link signals to linked PIDs
  +---> Delivers ProcessDown to monitors
  |
  v
AgentSupervisor checks RestartStrategy
  |
  +--- Permanent: do nothing
  +--- Transient: restart only if exit_code != 0
  +--- OneForOne: restart failed child only
  +--- OneForAll: restart all children
  +--- RestForOne: restart failed + later children
  |
  v
RestartTracker::record_restart(budget)
  |
  +--- Within budget: compute backoff, schedule restart
  |     backoff = 100ms * 2^(n-1), max 30s
  |
  +--- Budget exceeded: escalate to parent supervisor
```

### 3.3 Observations

- The Erlang-style supervision tree is well-implemented with proper budget tracking and exponential backoff.
- ProcessState has a validated FSM (`can_transition_to()`), preventing invalid transitions.
- The MonitorRegistry correctly handles both bidirectional links and unidirectional monitors.
- **Gap**: Restart events are NOT chain-logged (see Exercise 2). This means the audit trail has no record of self-healing actions.
- **Gap**: No circuit-breaker pattern. After budget exhaustion, escalation happens but there is no way to re-enable a permanently-failed agent without restarting the supervisor.

### 3.4 Recommendation

- Chain-log all restart events (`supervisor.restart`, `supervisor.budget_exceeded`, `supervisor.escalate`)
- Consider extracting `RestartBudget` + `RestartTracker` to `clawft-types` so the GUI can display restart state.
- Score: 2/22 crates (kernel + types for GUI display)

---

## Exercise 4: The "Governance Gate" Pattern

### 4.1 Two-Layer Governance Architecture

WeftOS implements governance at **two distinct layers**:

**Layer 1: GateBackend (capability-based, per-action)**
- File: `gate.rs`
- Trait: `GateBackend::check(agent_id, action, context) -> GateDecision`
- `GateDecision` enum: `Permit { token }`, `Defer { reason }`, `Deny { reason, receipt }`
- Default impl: `CapabilityGate` wraps `CapabilityChecker` (binary Permit/Deny)
- Advanced impl: `TileZeroGate` (behind `tilezero` feature) adds three-way decisions with Ed25519 receipts

**Layer 2: GovernanceEngine (effect-vector-based, policy-level)**
- File: `governance.rs`
- Struct: `GovernanceEngine` with rules, risk_threshold, human_approval flag
- Input: `GovernanceRequest` containing `EffectVector` (5-dimensional: risk, fairness, privacy, novelty, security)
- Output: `GovernanceResult` with `GovernanceDecision` (Permit / PermitWithWarning / EscalateToHuman / Deny)
- Evaluation: magnitude of EffectVector vs threshold, with rule severity escalation

### 4.2 EffectVector Structure

```rust
pub struct EffectVector {
    pub risk: f64,      // 0.0-1.0
    pub fairness: f64,
    pub privacy: f64,
    pub novelty: f64,
    pub security: f64,
}
```

Key methods:
- `magnitude()` -- L2 norm of all 5 dimensions
- `any_exceeds(threshold)` -- checks if any single dimension is above threshold
- `max_dimension()` -- returns highest individual score

### 4.3 Gate Decision Flow

```
Agent requests action
  |
  v
GateBackend::check(agent_id, action, context)    [gate.rs]
  |
  +--- CapabilityGate: checks ProcessTable capabilities
  |    Routes by action prefix: "tool." / "ipc." / "service."
  |
  +--- TileZeroGate (optional): three-way with receipts
  |    Chain-logs: "gate.permit", "gate.defer", "gate.deny"
  |
  v
GovernanceEngine::evaluate(request)               [governance.rs]
  |
  +--- Compute EffectVector magnitude
  +--- Check against risk_threshold
  +--- Evaluate rules by severity (Advisory/Warning/Blocking/Critical)
  +--- Environment-aware thresholds (Dev=lenient, Prod=strict)
  |
  v
GovernanceDecision
```

### 4.4 Where Gates Are Checked

Gates are checked at:
1. **Tool execution**: `GovernanceRequest::with_tool_context()` enriches the request with tool name, gate action, effect vector, and PID (governance.rs:255)
2. **TileZeroGate integration**: Wraps `cognitum_gate_tilezero::TileZero` for external tile reports (gate.rs:154)
3. **Dead letter routing**: `DeadLetterReason::GovernanceDenied` captures denial reasons (dead_letter.rs:37)
4. **Environment-scoped evaluation**: `evaluate_in_environment()` adjusts thresholds per environment class (governance.rs:429)

### 4.5 Recommendation

- The two-layer design (capability gate + governance engine) is sound. The `GateBackend` trait is clean and extensible.
- **Gap**: GovernanceEngine decisions are NOT chain-logged (only TileZeroGate chain-logs). The non-TileZero path has no audit trail for governance evaluations.
- **Proposed**: Add `ChainLoggable` impl for `GovernanceResult` so all governance decisions are auditable.
- Score: 2/22 crates (kernel, potentially weftos)

---

## Exercise 5: Cross-Cutting Concerns

### 5.1 Error Handling Pattern

**Pattern**: Per-domain `thiserror` enums with typed variants.

Found **15 error enums** across 4 crates:

| Error Type | Crate | File |
|------------|-------|------|
| `KernelError` | kernel | `error.rs:11` |
| `WasmError` | kernel | `wasm_runner.rs:232` |
| `ToolError` | kernel | `wasm_runner.rs:281` |
| `MeshError` | kernel | `mesh.rs:13` |
| `MeshIpcError` | kernel | `mesh_ipc.rs:105` |
| `DiscoveryError` | kernel | `mesh_discovery.rs:55` |
| `ClusterError` | kernel | `cluster.rs:275` |
| `AppError` | kernel | `app.rs:245` |
| `ContainerError` | kernel | `container.rs:214` |
| `EnvironmentError` | kernel | `environment.rs:201` |
| `EmbeddingError` | kernel | `embedding.rs:19` |
| `WeaverError` | kernel | `weaver.rs:459` |
| `ClawHubError` | services | `clawhub/registry.rs:19` |
| `UrlSafetyError` | tools | `url_safety.rs:42` |
| `PolicyError` | tools | `security_policy.rs:25` |
| `InitError` | weftos | `init.rs:138` |

**Observation**: All use `thiserror::Error` derive. Most convert to `KernelError` via `Box<dyn Error + Send + Sync>` through the `KernelError::Other` variant. This is the correct pattern. No action needed -- this is a **positive finding**.

However, there is no unified `WeftOsError` that composes all domain errors. Code at system boundaries (boot, CLI) often uses `Box<dyn Error>` or `anyhow::Result` instead of a typed error. For a library, this makes downstream error handling imprecise.

### 5.2 DashMap Concurrency Pattern

**Pattern**: `DashMap<K, V>` for concurrent registries; `Mutex` for single-writer structures.

DashMap usage: **134 occurrences across 26 files** (all in clawft-kernel).

Key observation: The kernel crate uses DashMap for all runtime registries (ServiceRegistry, ProcessTable, MonitorRegistry, MetricsRegistry, NamedPipeRegistry, A2ARouter, TopicRouter, etc.). The non-kernel crates use plain `HashMap` because they operate in single-threaded contexts or behind `Arc<Mutex<...>>`.

**Consistency**: Good. The boundary is clear -- kernel = concurrent, core/types/cli = single-threaded ownership.

### 5.3 Feature Gate Pattern

**Pattern**: Heavy use of `#[cfg(feature = "...")]` for conditional compilation.

Stats: **242 `#[cfg(feature` occurrences** in `clawft-kernel/src/` alone across 10 files. The `boot.rs` file has **77 feature-gated blocks** -- the highest density.

Key feature gates:
- `exochain` -- ExoChain, ChainManager, chain-logging
- `ecc` -- DEMOCRITUS loop, HNSW, CausalGraph, CrossRefStore
- `os-patterns` -- supervisor restart, metrics, DLQ, log service, timer
- `tilezero` -- TileZeroGate
- `governance` / `ruvector-apps` -- GovernanceEngine evaluation

**Problem**: The `Kernel` struct in `boot.rs` has **18 conditional fields** behind 6 different feature flags. This creates a combinatorial testing burden (the plenary notes 6 feature-gate combinations tested). Consider grouping related features into fewer, coarser-grained gates.

### 5.4 Serialization Pattern

**Pattern**: `#[derive(Debug, Clone, Serialize, Deserialize)]` on all data types.

This is applied consistently on: ChainEvent, ProcessState, RestartStrategy, RestartBudget, EffectVector, GovernanceRule, GovernanceRequest, GovernanceResult, DeadLetter, ChainCheckpoint, and all configuration types.

**Positive finding**: The separation between serializable data types and runtime handles (which contain `Arc`, `DashMap`, channels) is clean. No serialization leaks into infrastructure types.

### 5.5 Async/Spawn Pattern

**Pattern**: `tokio::spawn` behind `Platform` abstraction.

The `clawft-platform` crate provides a `Platform` trait that abstracts runtime operations. The kernel is generic over `P: Platform`, enabling WASM targets (which cannot use tokio) to provide alternative implementations.

`AgentSupervisor<P: Platform>` and `Kernel<P: Platform>` thread the platform through the entire stack. This is a strong architectural pattern that enables multi-target builds.

### 5.6 Builder / With-Method Pattern

**Pattern**: `with_*()` methods for fluent construction.

Found extensively in:
- `GovernanceRequest::with_effect()`, `with_node_id()`, `with_tool_context()`, `with_context_entry()`
- `ToolRegistry::with_parent()`
- Various config builders

This is consistent and well-applied.

---

## Pattern Catalog Summary Table

| # | Pattern Name | Crates | Instances | Abstracted? | Recommendation |
|---|-------------|--------|-----------|-------------|----------------|
| P1 | Registry (name-keyed lookup) | 6 | 15 structs | No | Extract `Registry` trait to clawft-types |
| P2 | Event -> Chain -> Log | 3 | 20+ call sites | Partial (ChainManager) | Add `ChainLoggable` trait |
| P3 | Supervisor -> Restart -> Budget | 1 | 6 types | No (kernel-internal) | Chain-log restart events; expose to GUI |
| P4 | Governance Gate (two-layer) | 1 | 2 impls | Yes (GateBackend trait) | Chain-log GovernanceEngine decisions |
| P5 | EffectVector scoring | 1 | 1 struct | Yes | Consider moving to clawft-types for GUI |
| P6 | Per-domain thiserror enums | 4 | 15 enums | Yes (pattern, not trait) | Good as-is; consider unified WeftOsError |
| P7 | DashMap concurrent registry | 1 | 134 uses in 26 files | Convention | Document as architectural decision |
| P8 | Feature gate grouping | 1 | 242 cfg blocks | N/A | Coarsen gates; reduce Kernel struct complexity |
| P9 | Serde-clean data types | 6+ | All data types | Convention | Good as-is |
| P10 | Platform-generic async | 3 | Kernel + Supervisor + services | Yes (Platform trait) | Good as-is; strong multi-target enabler |

---

## Top 5 Recommendations for Sprint 11

### 1. Extract `Registry` Trait to `clawft-types` (P1)

**Impact**: 6 crates, 15 registries
**Effort**: 4 hours
**Why**: The GUI's Registry Browser (K8) needs a uniform way to enumerate all registries. Currently, the Kernel struct manually exposes each registry through bespoke accessors. A common trait enables generic introspection. Start with `ServiceRegistry` and `ProcessTable` as proof of concept.

### 2. Add `ChainLoggable` Trait and Close Audit Gaps (P2, P3, P4)

**Impact**: 3 crates, 20+ chain-append call sites
**Effort**: 6 hours
**Why**: Three categories of significant kernel events are NOT chain-logged: process restarts, governance decisions (non-TileZero path), and IPC failures. This is an audit trail gap. A `ChainLoggable` trait standardizes the pattern and makes it easy to add chain-logging to new subsystems. The tree_manager's 14 raw `chain.append()` calls become typed events.

### 3. Coarsen Feature Gates in `boot.rs` (P8)

**Impact**: 1 crate (kernel), but reduces CI matrix
**Effort**: 3 hours
**Why**: The Kernel struct has 18 conditional fields behind 6 flags, and boot.rs has 77 cfg blocks. Group `ecc` + `exochain` into a single `cognitive` meta-feature, and `os-patterns` + `governance` into an `observability` meta-feature. This halves the combinatorial testing surface while preserving fine-grained control for embedders.

### 4. Move `EffectVector` and `GovernanceDecision` to `clawft-types` (P5)

**Impact**: 2 crates (kernel -> types)
**Effort**: 2 hours
**Why**: The GUI's Governance Console (Sprint 11 K8) needs these types for rendering. Currently they are deep inside `clawft-kernel::governance`. Moving them to `clawft-types` avoids the GUI depending on the kernel crate.

### 5. Document DashMap Convention and Thread-Safety Boundary (P7)

**Impact**: All crates (developer onboarding)
**Effort**: 1 hour
**Why**: The kernel-uses-DashMap / other-crates-use-HashMap split is consistent but undocumented. A short architecture note in the crate-level docs prevents future contributors from accidentally mixing the patterns. This is low-effort, high-clarity.

---

## ECC Contribution: CMVG Nodes and Edges

The following causal nodes and edges should be added to the Sprint 11 CMVG from Track 1's findings:

```
NODES:
  [N7] Registry Pattern Extracted
       status: PROPOSED
       evidence: 15 registries across 6 crates, common interface identified
       artifact: Registry trait in clawft-types

  [N8] Audit Trail Gaps Identified
       status: IDENTIFIED
       evidence: Process restarts, governance decisions, IPC failures not chain-logged
       risk: Compliance gap for production deployments

  [N9] ChainLoggable Trait Defined
       status: PROPOSED
       depends_on: N8
       artifact: ChainLoggable trait + typed event catalog

  [N10] Feature Gate Consolidation Plan
       status: PROPOSED
       evidence: 242 cfg blocks, 18 conditional Kernel fields, 6 feature combos
       artifact: Meta-features reducing combinatorial surface

  [N11] GUI-Ready Type Extractions
       status: PROPOSED
       evidence: EffectVector, GovernanceDecision, RestartBudget needed by K8
       depends_on: N2 (K8 GUI Prototype)

EDGES:
  N1  --[Reveals]--> N8     Kernel completion reveals audit gaps
  N8  --[Motivates]--> N9   Audit gaps motivate ChainLoggable trait
  N7  --[Enables]--> N2     Registry trait enables GUI Registry Browser
  N11 --[Enables]--> N2     Type extractions enable GUI Governance Console
  N10 --[Simplifies]--> N1  Gate consolidation simplifies kernel maintenance
  N9  --[Closes]--> N8      ChainLoggable closes audit trail gaps

CAUSAL CHAIN:
  N1 (achieved) --> N8 (identified) --> N9 (proposed) --> [audit compliance]
  N1 (achieved) --> N7 (proposed) --> N2 (achieved) --> [K8 production views]
  N1 (achieved) --> N10 (proposed) --> [reduced CI burden]
  N2 (achieved) --> N11 (proposed) --> [K8 Governance Console]
```

---

## Session Notes

- All findings are based on direct code reading of the `feature/weftos-kernel-sprint` branch as of 2026-03-27.
- Line numbers reference the current state of the codebase and may shift with further development.
- The 15-registry inventory is believed to be complete; no additional registry-like structures were found outside this set.
- The `ProcessTable` is included in the registry inventory despite using Pid (u64) keys rather than String keys, because it follows the same structural pattern and would benefit from the same GUI introspection capability.
