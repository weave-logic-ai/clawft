# RuVector Ecosystem Deep Analysis: WeftOS Alignment

**Date**: 2026-02-28
**Purpose**: Cross-reference the ruvector/agentic-flow/ruflo ecosystem against WeftOS SPARC plans (W-KERNEL K0-K5) to identify reusable patterns, integration points, and architecture alignment opportunities.

---

## 1. Ecosystem Overview

The ruv ecosystem spans 5+ repos and 100+ Rust crates:

| Repo | Purpose | Key Relevance to WeftOS |
|------|---------|------------------------|
| [ruvector](https://github.com/ruvnet/ruvector) | Vector DB + self-learning engine (101 Rust crates) | Cognitive containers, Raft consensus, CRDT delta sync, cluster management, witness chains, WASM sandbox, service discovery, coherence gate |
| [agentic-flow](https://github.com/ruvnet/agentic-flow) | Agent orchestration (66 agents, 213 MCP tools) | Agent routing, swarm coordination, AgentDB memory, QUIC transport, attention-based consensus |
| [ruflo](https://github.com/ruvnet/ruflo) | Claude agent platform (swarms, hive-mind) | Queen/worker patterns, ReasoningBank learning, anti-drift, gossip protocol |
| [QuDAG](https://github.com/ruvnet/QuDAG) | Quantum-resistant DAG communication | P2P messaging via LibP2P/Kademlia, onion routing, QR-Avalanche consensus, .dark domain naming, MCP server |
| [DAA](https://github.com/ruvnet/daa) | Decentralized Autonomous Applications | MRAP autonomy loop, governance rule engine, token economy, federated learning |

---

## 2. Phase-by-Phase Alignment

### K0: Kernel Foundation

#### Boot Sequence -- RVF Cognitive Container Pattern

**ruvector source**: `crates/ruvector-cognitive-container/src/container.rs`

The `CognitiveContainer` implements an epoch-based tick loop with phased execution under budget constraints. This is directly applicable to our kernel boot sequence:

```
ruvector pattern:                    WeftOS adaptation:
CognitiveContainer::new()      -->   Kernel::boot()
  MemorySlab::new()            -->   ProcessTable::new()
  EpochController::new()       -->   ServiceRegistry::new()
  WitnessChain::new()          -->   KernelIpc::new()
  tick() phases:               -->   Boot phases:
    Phase::Ingest              -->     Phase::InitSubsystems
    Phase::MinCut              -->     Phase::RegisterServices
    Phase::Spectral            -->     Phase::StartServices
    Phase::Evidence            -->     Phase::HealthCheck
    Phase::Witness             -->     Phase::Ready
```

**Adopt**: The `ComponentMask` bitmask pattern for tracking which boot phases completed. Clean, zero-allocation, testable.

```rust
// From ruvector - adopt directly for WeftOS boot tracking
pub struct BootPhaseMask(pub u8);
impl BootPhaseMask {
    pub const SUBSYSTEMS: Self = Self(0b0001);
    pub const SERVICES: Self  = Self(0b0010);
    pub const HEALTH: Self    = Self(0b0100);
    pub const READY: Self     = Self(0b1000);
    pub const ALL: Self       = Self(0b1111);
}
```

**Adopt**: The `ContainerSnapshot` serialization pattern for kernel state checkpointing.

**Annotate K0 plan**: Add `BootPhaseMask` to `boot.rs` types. Add `KernelSnapshot` for state serialization following the `ContainerSnapshot` pattern.

#### Service Registry -- Cluster Manager Pattern

**ruvector source**: `crates/ruvector-cluster/src/lib.rs`

The `ClusterManager` demonstrates exactly the pattern WeftOS needs for service registration:

| ClusterManager | WeftOS ServiceRegistry |
|---|---|
| `DashMap<String, ClusterNode>` | `DashMap<String, Arc<dyn SystemService>>` |
| `add_node()` / `remove_node()` | `register()` / `unregister()` |
| `healthy_nodes()` | `health_all()` |
| `run_health_checks()` | `HealthSystem::aggregate()` |
| `ConsistentHashRing` | N/A (future: shard work across agents) |
| `ShardRouter` | N/A (future: route tool calls across agents) |
| `DiscoveryService` trait | **Adopt**: trait for service auto-discovery |

**Adopt**: The `ClusterConfig` default pattern with heartbeat/timeout/quorum settings maps directly to `KernelConfig`.

**Adopt**: The `NodeStatus` enum (`Leader/Follower/Candidate/Offline`) as inspiration for `ProcessState`. Add `Leader` state for supervisor agents.

**Annotate K0 plan**: Add `DiscoveryService` trait stub to `service.rs` for future K2/K5 use. Add `ClusterStats`-style `KernelStats` struct.

#### Health System -- Coherence Gate Pattern

**ruvector source**: `crates/cognitum-gate-tilezero/src/lib.rs`

The `TileZero` coherence gate is a sophisticated health/decision system that we can simplify for WeftOS:

```
TileZero pattern:                    WeftOS adaptation:
  collect_reports(tiles)       -->   health.collect(services)
  three-filter decision:       -->   health aggregation:
    structural_ok (min-cut)    -->     all_services_healthy
    shift_ok (pressure)        -->     resource_usage_ok
    evidence_decision          -->     recent_errors_check
  GateDecision::Permit/Defer/Deny    OverallHealth::Healthy/Degraded/Down
  PermitToken (signed)         -->   HealthReport (timestamped)
  WitnessReceipt (hash-chain)  -->   HealthHistory (for debugging)
```

**Adopt**: The `PermitToken` concept as a lightweight "capability token" issued by the kernel to agents. An agent presents its token to prove it has permission for an operation. This enhances K1 RBAC significantly.

**Adopt**: The `WitnessReceipt` hash-chain pattern for audit logging. Every kernel operation gets a chained receipt. Tamper-evident kernel audit trail.

**Annotate K0 plan**: Add `KernelAuditLog` with hash-chained receipts to `health.rs`.

---

### K1: Supervisor + RBAC

#### Capability Enforcement -- MCP Gate Pattern

**ruvector source**: `crates/mcp-gate/src/lib.rs`, `crates/cognitum-gate-tilezero/`

The MCP Gate exposes three tools (`permit_action`, `get_receipt`, `replay_decision`) that are a direct model for how our capability checker should work:

```
MCP Gate pattern:                    WeftOS adaptation:
  permit_action(action_ctx)    -->   CapabilityChecker::check_tool_access(pid, tool)
  ActionContext {              -->   ToolCallContext {
    action_id                  -->     call_id
    action_type                -->     tool_name
    target.device              -->     target_path
    context.agent_id           -->     pid (from ProcessTable)
    context.prior_actions      -->     call_history
  }
  GateDecision::Permit         -->   Ok(())
  GateDecision::Deny           -->   Err(ToolDenied)
  GateDecision::Defer          -->   Err(NeedsEscalation) [NEW - adopt from ruvector]
```

**Key insight**: ruvector adds a `Defer` decision alongside `Permit`/`Deny`. This is valuable for WeftOS: an agent can be told "this action requires escalation to the supervisor" rather than hard-denied. This enables hierarchical capability delegation.

**Adopt**: Three-way capability decision: `Permit`, `Deny`, `Escalate(EscalationInfo)`

**Adopt**: The `EscalationInfo` struct for escalation targets:
```rust
pub struct EscalationInfo {
    pub to: Pid,                    // Supervisor PID
    pub context_url: Option<String>, // Context for the human
    pub timeout: Duration,          // Auto-deny after timeout
    pub default_on_timeout: CapabilityDecision, // Deny or Permit
}
```

**Annotate K1 plan**: Add `CapabilityDecision::Escalate(EscalationInfo)` to capability.rs. This enables human-in-the-loop approval for sensitive operations without blocking the agent.

#### Agent Lifecycle -- DAA MRAP Pattern

**daa SDK source**: Documented in repo

The DAA SDK's MRAP (Monitor -> Reason -> Act -> Reflect -> Adapt) loop is a more sophisticated version of our agent lifecycle:

```
DAA MRAP:                           WeftOS Agent Lifecycle:
  Monitor (env data)           -->   AgentLoop polls inbox + tools
  Reason (AI decision)         -->   Pipeline: classify -> route -> assemble -> LLM
  Act (blockchain actions)     -->   Tool execution
  Reflect (performance)        -->   SONA learning signal (future)
  Adapt (strategy update)      -->   Capability adjustment (future)
```

**Adopt**: The `Reflect` and `Adapt` phases as optional hooks in the supervisor. After each agent task completes, the supervisor can trigger a reflection step that feeds into SONA learning.

**Adopt**: DAA's `TokenManager` budget allocation concept for per-agent resource budgets in the supervisor. Agents "spend" capability tokens on operations.

**Annotate K1 plan**: Add `PostTaskHook` to supervisor for optional Reflect/Adapt phases. Add `ResourceBudget` type alongside `ResourceLimits` for soft limits (budget) vs hard limits (caps).

---

### K2: Agent-to-Agent IPC

#### Message Protocol -- Delta Consensus + CRDT Pattern

**ruvector source**: `crates/ruvector-delta-consensus/src/lib.rs`

This is the most directly reusable code for K2. The `DeltaConsensus` implements exactly what WeftOS A2A IPC needs:

| DeltaConsensus | WeftOS A2A IPC |
|---|---|
| `CausalDelta` | `KernelMessage` with causal metadata |
| `VectorClock` | Causal ordering of messages between agents |
| `DeltaConsensus::receive()` | `A2AProtocol::receive()` |
| `DeliveryStatus::Delivered/Pending/AlreadyApplied` | Message delivery tracking |
| `DeltaGossip` | Topic-based pub/sub dissemination |
| `ConflictStrategy::LastWriteWins/Merge` | Concurrent message conflict resolution |
| `dependencies_satisfied()` | Causal message ordering |

**Adopt**: `VectorClock` for causal ordering of inter-agent messages. This prevents message reordering issues that simple timestamps can't handle.

**Adopt**: `CausalDelta` pattern for `KernelMessage`. Add `vector_clock` and `dependencies` fields:

```rust
pub struct KernelMessage {
    pub id: String,
    pub from: Pid,
    pub to: MessageTarget,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
    // NEW from ruvector:
    pub vector_clock: VectorClock,   // Causal ordering
    pub dependencies: Vec<String>,    // Message dependencies
}
```

**Adopt**: `DeltaGossip` outbox/inbox pattern for topic pub/sub:
```rust
// From ruvector gossip protocol
pub struct TopicGossip {
    subscriptions: DashMap<String, Vec<Pid>>,
    outbox: RwLock<Vec<KernelMessage>>,  // Messages to deliver
}
```

**Adopt**: The `DeliveryStatus` enum (Delivered/Pending/AlreadyApplied/Rejected) for message acknowledgment tracking.

**Annotate K2 plan**: Replace simple `mpsc` inbox with causal-ordered delivery using VectorClock. Add `GossipResult` stats to topic publish return value. Add message deduplication via `AlreadyApplied` tracking.

#### P2P Transport -- QuDAG Pattern

**QuDAG source**: Documented architecture

For future K2 extensions (remote agents, distributed WeftOS):

| QuDAG | WeftOS Future |
|---|---|
| LibP2P + Kademlia DHT | Remote agent discovery |
| Onion routing | Secure inter-node agent communication |
| .dark domain system | Agent naming without central authority |
| QR-Avalanche consensus | Byzantine-tolerant multi-node agreement |
| MCP server over stdio/HTTP/WS | Already have (MCP server in clawft-services) |

**Annotate K2 plan**: Add "Future: P2P transport" section referencing QuDAG's LibP2P pattern for distributed WeftOS deployments.

#### Routing -- Nervous System Pattern

**ruvector source**: `crates/ruvector-nervous-system/src/routing.rs`

The nervous system's `OscillatoryRouter` and `CoherenceGatedSystem` provide biologically-inspired routing:

| NervousSystem | WeftOS IPC |
|---|---|
| `OscillatoryRouter` | Route messages based on phase-locked agent activity |
| `GlobalWorkspace` | Shared attention space for agent coordination |
| `CircadianController` | Time-based routing policies (business hours, maintenance windows) |
| `BudgetGuardrail` | Resource-aware routing (don't send to overloaded agent) |
| `ShardedEventBus` | High-throughput event distribution |

**Adopt**: `BudgetGuardrail` for IPC routing. Don't deliver messages to agents that have exhausted their resource budgets.

**Adopt**: `ShardedEventBus` pattern for high-throughput topic distribution. Instead of a single subscriber list, shard by topic prefix for parallel delivery.

**Annotate K2 plan**: Add `BudgetGuardrail` to A2A protocol routing. Add `ShardedEventBus` reference for topic scaling.

---

### K3: WASM Sandbox

#### WASM Runtime -- RVF WASM Segment + OSpipe Pattern

**ruvector source**: RVF format spec + `examples/OSpipe/src/`

The RVF format's three-tier execution model (WASM -> eBPF -> Kernel) directly informs K3:

| RVF Tier | WeftOS WASM Sandbox |
|---|---|
| WASM_SEG (5.5KB) | Tool WASM modules |
| Fuel metering (EpochController budget) | `WasmSandboxConfig.max_fuel` |
| Memory slab allocation (MemorySlab) | `WasmSandboxConfig.max_memory_bytes` |
| Witness chain on execution | Audit trail for sandboxed tool calls |
| Copy-on-Write branching | Snapshot/restore for deterministic re-execution |

**Key insight from ruvector**: The `EpochController` budget system in cognitive containers is conceptually identical to WASM fuel metering. Both enforce "you can do N units of work, then you're done."

```
ruvector EpochController:           WeftOS WASM fuel:
  epoch_budget.total           -->   max_fuel
  try_budget(Phase)            -->   store.consume_fuel(amount)
  consume(work_units)          -->   fuel decremented per instruction
  partial result on exhaustion -->   partial tool result on exhaustion
```

**Adopt**: The `EpochController` pattern for compound tool execution. When a WASM tool calls multiple sub-operations, each sub-operation checks and consumes budget. The tool gets a `partial` result if budget runs out mid-execution.

**Adopt**: Copy-on-Write state branching for tool undo. Before executing a WASM tool, create a COW snapshot. If the tool fails or exceeds limits, roll back to the snapshot.

**Adopt**: Witness chain for sandboxed execution audit. Every WASM tool call gets a `WitnessReceipt` with input hash, output hash, fuel consumed, and memory peak.

**Annotate K3 plan**: Add `WasmWitnessReceipt` to `wasm_runner.rs` types. Add COW snapshot/restore for tool rollback. Add `partial` field to `WasmToolResult` (already in plan, reinforce with epoch pattern).

#### OSpipe Pipeline Pattern

**ruvector source**: `examples/OSpipe/src/lib.rs`

OSpipe's pipeline architecture maps to how WASM tools should be chained:

```
OSpipe:                              WeftOS WASM Tools:
Capture -> Safety -> Dedup ->        Input -> Validate -> Execute ->
  Embed -> VectorStore               Record -> Return
```

The `safety.rs` module (PII detection, content redaction) is directly useful for K3 sandboxing: detect and redact sensitive data before passing it to WASM tools.

**Annotate K3 plan**: Reference OSpipe's safety pipeline for pre-execution input sanitization.

---

### K4: Containers

#### Container Orchestration -- Cluster Discovery Pattern

**ruvector source**: `crates/ruvector-cluster/src/discovery.rs`

The cluster's `DiscoveryService` trait provides the pattern for container service discovery:

```rust
// From ruvector - adopt for container discovery
pub trait DiscoveryService: Send + Sync {
    async fn discover_nodes(&self) -> Result<Vec<ClusterNode>>;
}

pub struct StaticDiscovery { nodes: Vec<ClusterNode> }
pub struct GossipDiscovery { /* ... */ }
```

**Adapt for containers**:
```rust
pub trait ContainerDiscovery: Send + Sync {
    async fn discover_services(&self) -> Result<Vec<ContainerSpec>>;
}
pub struct DockerDiscovery { /* bollard */ }
pub struct ComposeDiscovery { /* parse compose file */ }
pub struct StaticDiscovery { services: Vec<ContainerSpec> }
```

**Adopt**: Multiple discovery backends (static, Docker, compose) behind a single trait.

**Annotate K4 plan**: Add `ContainerDiscovery` trait with `DockerDiscovery` and `ComposeDiscovery` implementations.

#### Replication & Failover

**ruvector source**: `crates/ruvector-replication/src/`

The replication crate provides patterns for container service resilience:

| Replication | Container Sidecar |
|---|---|
| `replica.rs` - replica lifecycle | Container lifecycle |
| `failover.rs` - automatic failover | Container restart policy |
| `sync.rs` - state synchronization | Volume mount sync |
| `conflict.rs` - conflict resolution | Port conflict resolution |
| `stream.rs` - change streaming | Container log streaming |

**Annotate K4 plan**: Reference replication failover patterns for container restart and health recovery.

---

### K5: Application Framework

#### Application Manifests -- DAA Orchestrator + RVF Manifest Pattern

**daa source**: `daa-orchestrator` crate
**ruvector source**: RVF MANIFEST_SEG

DAA's builder pattern for agent configuration:
```rust
// DAA pattern
agent.with_role("reviewer")
     .with_rules(governance_rules)
     .with_ai_advisor(claude_config)
     .with_economy(token_budget)
```

This maps to WeftOS `weftapp.toml` agent specs:
```toml
[[agents]]
id = "reviewer"
role = "code-review"
[agents.capabilities]
sandbox = { allow_shell = false }
resource_limits = { max_memory_mb = 512 }
```

**Adopt**: DAA's rule engine concept for app-level governance rules. Apps can define rules that constrain their agents:

```toml
[rules]
max_spend_per_hour = 100        # Token budget
operating_hours = "09:00-17:00"  # CircadianController-inspired
require_approval_above = 0.8     # Escalation threshold
```

**Annotate K5 plan**: Add `[rules]` section to `weftapp.toml` spec for app-level governance constraints.

#### Self-Learning -- SONA + ReasoningBank Pattern

**ruvector source**: `crates/sona/src/lib.rs`

SONA provides self-optimization that WeftOS applications can leverage:

| SONA | WeftOS App Learning |
|---|---|
| `MicroLoRA` (instant learning, rank 1-2) | Per-task routing optimization |
| `BaseLoRA` (background learning) | Agent behavior improvement over time |
| `EWC++` (forgetting prevention) | Preserve good patterns across updates |
| `ReasoningBank` (pattern storage/search) | Application-level pattern library |
| `TrajectoryBuilder` | Track agent task execution for learning |
| Three loops: Instant/Background/Coordination | App-level learning loops |

**Adopt**: The `TrajectoryBuilder` pattern for app-level learning:

```rust
// From SONA - adapt for WeftOS apps
let mut trajectory = app.begin_trajectory("code-review-task");
trajectory.add_step(agent_action, tool_calls, quality_score);
app.end_trajectory(trajectory, overall_score);
// -> ReasoningBank stores pattern for future similar tasks
```

**Annotate K5 plan**: Add optional `[learning]` section to `weftapp.toml` for apps that want SONA-style self-optimization. Add `TrajectoryHook` to app lifecycle hooks.

---

## 3. Hidden Features & Easter Eggs

### Cognitum Gate (256-tile WASM fabric)

**Source**: `crates/cognitum-gate-tilezero/`, `crates/cognitum-gate-kernel/`

A 256-tile WASM fabric where each tile evaluates coherence independently, and TileZero merges results into a global decision. This is an **underappreciated pattern** for WeftOS:

- **Apply to K1 RBAC**: Instead of a single capability checker, use a tile-based pattern where each subsystem (filesystem, network, IPC, tools) runs its own coherence check, and the supervisor merges results.
- **Apply to K2 consensus**: Multi-agent decisions can use the tile pattern -- each agent "votes" via a tile report, TileZero (the supervisor) merges.

### Agentic-Jujutsu (quantum-resistant VCS for agents)

Lock-free concurrent version control where agents commit simultaneously without conflicts. The key insight: agents work on separate branches that are automatically merged using ReasoningBank to resolve conflicts.

**Apply to K2 IPC**: When two agents modify shared state concurrently, use Jujutsu-style CRDT merging instead of locking.

### RVF Kernel Segment (bootable microkernel in a file)

The RVF format can embed a Linux microkernel that self-boots in 125ms. This is the most extreme version of what WeftOS is building -- a single file that IS an operating system.

**Future aspiration**: WeftOS applications could be packaged as `.rvf` files that contain the application manifest, WASM tools, agent configs, and a WeftOS kernel bootstrap -- all in one deployable artifact.

### OSpipe (personal AI memory)

Screenpipe integration that captures screen content, processes through safety/PII filters, and stores as searchable vectors. Hidden gem: the `quantum` module in OSpipe implements quantum-resistant encryption for personal data.

### Neural Trader, Spiking Networks, Verified Applications

40+ example applications demonstrating ruvector capabilities including financial trading, spiking neural networks, DNA analysis, and formally verified applications with proof-gated mutations.

---

## 4. Concrete Integration Recommendations

### Immediate (K0-K1)

| # | What | From | Into | Effort |
|---|------|------|------|--------|
| 1 | `ComponentMask` bitmask for boot phases | `ruvector-cognitive-container` | `boot.rs` | Small |
| 2 | `ClusterManager` DashMap service pattern | `ruvector-cluster` | `service.rs` | Small |
| 3 | `WitnessChain` hash-chained audit log | `ruvector-cognitive-container` | `health.rs` | Medium |
| 4 | Three-way `Permit/Deny/Escalate` decisions | `cognitum-gate-tilezero` | `capability.rs` | Small |
| 5 | `PermitToken` signed capability tokens | `cognitum-gate-tilezero` | `capability.rs` | Medium |
| 6 | `NodeStatus` enum enrichment | `ruvector-cluster` | `process.rs` | Small |

### Short-term (K2-K3)

| # | What | From | Into | Effort |
|---|------|------|------|--------|
| 7 | `VectorClock` causal ordering | `ruvector-delta-consensus` | `ipc.rs` | Medium |
| 8 | `CausalDelta` message dependencies | `ruvector-delta-consensus` | `a2a.rs` | Medium |
| 9 | `DeltaGossip` pub/sub dissemination | `ruvector-delta-consensus` | `topic.rs` | Medium |
| 10 | `DeliveryStatus` ack tracking | `ruvector-delta-consensus` | `a2a.rs` | Small |
| 11 | `BudgetGuardrail` routing | `ruvector-nervous-system` | `ipc.rs` | Small |
| 12 | `EpochController` fuel budgeting | `ruvector-cognitive-container` | `wasm_runner.rs` | Medium |
| 13 | `WitnessReceipt` for WASM execution | `ruvector-cognitive-container` | `wasm_runner.rs` | Small |

### Medium-term (K4-K5)

| # | What | From | Into | Effort |
|---|------|------|------|--------|
| 14 | `DiscoveryService` trait | `ruvector-cluster` | `container.rs` | Small |
| 15 | Failover patterns | `ruvector-replication` | `container.rs` | Medium |
| 16 | DAA governance rule engine | `daa` SDK | `app.rs` | Medium |
| 17 | SONA `TrajectoryBuilder` | `sona` crate | `app.rs` (hook) | Medium |
| 18 | Jujutsu CRDT merge for shared state | agentic-jujutsu | `a2a.rs` | Large |

### Aspirational (post-K5)

| # | What | From | Into | Effort |
|---|------|------|------|--------|
| 19 | RVF packaging for WeftOS apps | `rvf` crate | App packaging | Large |
| 20 | QuDAG P2P for distributed WeftOS | `QuDAG` | Remote agents | Large |
| 21 | Tile-based multi-subsystem coherence | `cognitum-gate` | Kernel RBAC | Large |
| 22 | OSpipe safety pipeline for tools | `OSpipe` | `wasm_runner.rs` | Medium |

---

## 5. Architecture Pattern Summary

### Patterns We Should Directly Adopt

1. **Bitmask phase tracking** (`ComponentMask`) -- zero-allocation, testable
2. **DashMap service registry** -- lock-free concurrent access
3. **Hash-chained audit trail** (`WitnessChain`) -- tamper-evident
4. **Three-way capability decisions** (Permit/Deny/Escalate) -- human-in-the-loop
5. **Vector clock causal ordering** -- correct message ordering
6. **CRDT-based conflict resolution** -- concurrent state updates
7. **Gossip dissemination** -- scalable pub/sub
8. **Budget guardrails** -- resource-aware routing
9. **Epoch-based fuel budgeting** -- compound operation limits
10. **Discovery service trait** -- pluggable service discovery

### Patterns We Should Reference But Not Copy

1. **256-tile WASM fabric** -- too complex for initial WeftOS, but valuable mental model
2. **RVF binary format** -- purpose-built for vectors, not general kernel state
3. **Post-quantum crypto** -- overkill for local IPC, valuable for distributed future
4. **Spiking neural networks** -- fascinating but not kernel-level
5. **P2P onion routing** -- QuDAG is for distributed darknet, not local agents

### Patterns That Diverge From WeftOS

1. **DAG consensus** (ruvector) vs **Raft** (WeftOS K2) -- DAG is leaderless, good for distributed; Raft is simpler for local supervisor
2. **Token economy** (DAA) vs **Capability RBAC** (WeftOS K1) -- token trading adds complexity; RBAC is sufficient for local agents
3. **Vector embeddings** everywhere vs **typed messages** (WeftOS K2) -- WeftOS IPC doesn't need vector similarity search

---

## 6. Dependency Considerations

### Can We Take Crates Directly?

| Crate | Usable? | Notes |
|---|---|---|
| `ruvector-delta-consensus` | Yes (via dep) | Depends on `ruvector-delta-core` for `VectorDelta` type. We'd need to either depend on it or extract the `VectorClock` + `CausalDelta` pattern |
| `cognitum-gate-tilezero` | Concepts only | Tightly coupled to ruvector's graph model; extract the decision pattern |
| `sona` | Yes (optional dep) | Self-contained learning engine, works standalone |
| `ruvector-raft` | Yes (via dep) | Clean Raft implementation, no external deps beyond bincode |
| `ruvector-cluster` | Concepts only | Coupled to vector DB sharding; extract DashMap + discovery patterns |

### Recommended Approach

**Don't add ruvector as a dependency.** Instead:
1. Extract patterns and implement them in `clawft-kernel`
2. Add SONA as an optional dependency behind a `learning` feature flag for K5
3. Consider `ruvector-raft` as a dep behind a `consensus` feature flag for distributed WeftOS (post-K5)
4. Reference the RVF format spec when designing future app packaging

---

## 7. Updated Phase Plans with Annotations

See individual phase plan files for inline annotations. Key additions per phase:

- **K0**: `BootPhaseMask`, `KernelSnapshot`, `KernelAuditLog` with hash-chain, `DiscoveryService` trait stub
- **K1**: `CapabilityDecision::Escalate`, `EscalationInfo`, `PostTaskHook` for MRAP Reflect/Adapt, `ResourceBudget`
- **K2**: `VectorClock` in `KernelMessage`, `CausalDelta`-style dependencies, `DeltaGossip`-style topic pub/sub, `DeliveryStatus` tracking, `BudgetGuardrail` routing
- **K3**: `WasmWitnessReceipt`, COW snapshot/restore, `EpochController`-style compound budget, OSpipe safety pre-filter
- **K4**: `ContainerDiscovery` trait with multiple backends, replication failover patterns
- **K5**: App governance `[rules]` in manifest, optional SONA `TrajectoryBuilder` for learning apps

---

## 8. Sources

- [ruvector](https://github.com/ruvnet/ruvector) -- 101 Rust crates, RVF format, cognitive containers
- [agentic-flow](https://github.com/ruvnet/agentic-flow) -- Agent orchestration, AgentDB, 213 MCP tools
- [ruflo](https://github.com/ruvnet/ruflo) -- Claude agent platform, swarm coordination
- [QuDAG](https://github.com/ruvnet/QuDAG) -- Quantum-resistant DAG communication
- [DAA](https://github.com/ruvnet/daa) -- Decentralized Autonomous Applications
- [AgentDB](https://agentdb.ruv.io/) -- Vector database for AI agents
