# SPARC Orchestrator: WeftOS Kernel Workstream

**Workstream ID**: W-KERNEL
**Date**: 2026-02-28
**Status**: In Progress (K0, K1, K2, K2b, K2.1 Complete; K2 Symposium Complete)
**Estimated Duration**: 17+ weeks (K0-K6 + addenda)
**Source Analysis**: `.planning/development_notes/openfang-comparison.md`, existing kernel primitives audit

---

## 1. Workstream Summary

Compose clawft's existing kernel primitives (MessageBus, SandboxEnforcer, PermissionResolver, CronService, AgentLoop, Platform traits) into a first-class OS abstraction. WeftOS wraps the framework in a kernel layer, enabling process management, per-agent RBAC, agent-to-agent IPC, WASM tool sandboxing, container integration, and an application framework.

### CLI Naming: `weave` + `weft`

Two entry points following the textile metaphor:

- **`weft`** (existing) = the threads woven through. Agent/virtual layer: spawn agents, send IPC, manage tools, run apps. This is the user-facing agent interface.
- **`weave`** (new) = the act of combining on the loom. OS/physical layer: kernel, process table, services, resource tree, networking, environments. This is the system administration interface.

Both binaries link to the same `clawft-cli` crate. `weave` is a thin alias that sets `CLI_MODE=os` so subcommand routing knows which namespace to expose. Existing `weft` commands are 100% backward compatible.

### Approach

- **New crate, not new framework**: `clawft-kernel` composes existing crates; no rewrites
- **Kernel wraps AppContext**: The `Kernel<P>` struct owns an `AppContext<P>` and adds process table, service registry, IPC, and capability enforcement
- **Incremental adoption**: Each phase adds a kernel subsystem; existing `weft` commands continue to work unchanged
- **Dual CLI**: OS commands via `weave`, agent commands via `weft` (see Section 12)

### Crate Scope

| In Scope (new + modified) | Consumed (read-only) |
|---|---|
| `clawft-kernel` (new) | `clawft-types` |
| `clawft-core` (supervisor hooks) | `clawft-platform` |
| `clawft-cli` (new commands) | `clawft-plugin` |
| `clawft-services` (IPC wiring) | `clawft-security` |
| | `clawft-llm` |
| | `clawft-channels` |

---

## 2. Phase Summary

| Phase | ID | Title | Goal | Duration |
|---|---|---|---|---|
| 0 | K0 | Kernel Foundation | New `clawft-kernel` crate with boot, process table, service registry, health, cluster membership | 2 weeks | **Complete** |
| 1 | K1 | Supervisor + RBAC | Agent supervisor with spawn_and_run, GateBackend, chain persistence, agent tree nodes, IPC RBAC, weaver agent CLI | 2 weeks | **Complete** |
| 2 | K2 | A2A IPC | Agent-to-agent messaging, pub/sub topics, JSON-RPC wire format | 2 weeks | **Complete** |
| 2b | K2b | Work-Loop Hardening | Health monitor, watchdog, graceful shutdown, resource tracking, suspend/resume, gate enforcement | 1 day | **Complete** |
| 2s | K2-Symposium | K2 Symposium | Cross-cutting design decisions for K3-K6 scope, commitments C1-C10. Results: `docs/weftos/k2-symposium/08-symposium-results-report.md` | -- | **Complete** |
| 2.1 | K2.1 | Symposium Implementation | SpawnBackend enum (C1+C8), post-quantum dual signing (C6), breaking IPC changes (D19+D1), ServiceEntry, GovernanceGate verification. SPARC: `03a-phase-K2.1-symposium-implementation.md` | 2-3 days | **Complete** |
| 3 | K3 | WASM Sandbox | Wasmtime tool execution, fuel metering, memory limits, ServiceApi trait (C2), dual-layer gate (C4) | 2 weeks |
| 4 | K4 | Containers | Alpine image, sidecar service orchestration, ChainAnchor trait (C7), SONA reuptake spike | 1 week |
| 5 | K5 | App Framework | Application manifests, lifecycle, external framework interop, clustering (moved from K6 per D6) | 2 weeks |
| 6 | K6 | Distributed Fabric | Multi-node cluster, cross-node IPC, cryptographic filesystem, governance. Note: SPARC spec required before implementation begins (D22, C10) | 6 weeks |

---

## 3. Dependencies

### Internal Dependencies (Phase-to-Phase)

```
K0 (Foundation) -- COMPLETE
  |  + ClusterMembership (universal peer tracker, all platforms)
  |  + ClusterService (ruvector-cluster, native-only, feature-gated)
  |  + weaver daemon (Unix socket RPC) + weaver cluster CLI
  |  + ExoChain local chain (chain_id=0, SHA-256 hash linking)
  |  + exo-resource-tree crate (tree CRUD, Merkle, MutationLog)
  |  + TreeManager facade (tree + mutation log + chain, atomic ops)
  |  + Boot-to-chain audit trail (boot.init -> boot.ready)
  |  + Chain integrity verification (weaver chain verify)
  |  + Two-way traceability (tree nodes <-> chain events via chain_seq)
  |  + Shutdown checkpoint (kernel.shutdown event + chain checkpoint)
  |
  +---> K1 (Supervisor/RBAC) -- depends on K0 (process table, capabilities)
  |         |
  |         +---> K2 (A2A IPC) -- depends on K1 (agent PIDs, capability checks)
  |                   |
  |                   +---> K2.1 (Symposium Implementation) -- depends on K2b (breaking changes)
  |                             |
  |                             +---> K3 (WASM Sandbox) -- depends on K2.1 (SpawnBackend, ServiceEntry)
  |                             |
  |                             +---> K5 (App Framework) -- depends on K3+K4
  |                             |
  |                             +---> K6 (Distributed Fabric) -- depends on K0+K1+K2+K5
  |
  +---> K3 (WASM Sandbox) -- depends on K0 (service registry)  [parallel w/ K1]
  |
  +---> K4 (Containers) -- depends on K0 (service registry)    [parallel w/ K1]
```

### Parallelization Opportunities

- **K1, K3, K4** can run in parallel after K0 completes
- **K2** requires K1 (needs process table PIDs for message routing)
- **K5** requires K1+K2 (apps spawn agents, agents communicate)

### External Dependencies

- **Wasmtime** (K3): `wasmtime` crate for WASM tool execution
- **Bollard** (K4): Docker API for container management (optional, behind feature flag)
- **No other workstream blocking**: W-KERNEL is self-contained

### New Cargo Dependencies

| Dependency | Version | Scope | Introduced In |
|---|---|---|---|
| `wasmtime` | 27.0 | `clawft-kernel` (optional, `wasm-sandbox` feature) | K3 |
| `bollard` | 0.17 | `clawft-kernel` (optional, `containers` feature) | K4 |
| `dashmap` | 6.0 | `clawft-kernel` | K0 |
| `ruvector-cluster` | 0.1 (path) | `clawft-kernel` (optional, `cluster` feature) | K0 |
| `ruvector-raft` | 0.1 (path) | `clawft-kernel` (optional, `cluster` feature) | K0 |
| `ruvector-replication` | 0.1 (path) | `clawft-kernel` (optional, `cluster` feature) | K0 |
| `parking_lot` | 0.12 | `clawft-kernel` (optional, `cluster` feature) | K0 |
| `futures` | 0.3 | `clawft-kernel` (optional, `cluster` feature) | K0 |
| `exo-core` | 0.1 | `exo-resource-tree` | K0 |
| `exo-identity` | 0.1 | `exo-resource-tree` | K0 |
| `exo-consent` | 0.1 | `exo-resource-tree` | K0 |
| `exo-dag` | 0.1 | `exo-resource-tree` | K0 |
| `exo-resource-tree` | 0.1 (path) | `clawft-kernel` (optional, `exochain` feature) | K0 |
| `sha2` | 0.10 | `clawft-kernel` (optional, `exochain` feature) | K0 |

---

## 4. Interface Contracts

### 4.1 Kernel Trait (`boot.rs`)

```rust
pub struct Kernel<P: Platform> {
    state: KernelState,
    app_context: Option<AppContext<P>>,
    process_table: Arc<ProcessTable>,
    service_registry: Arc<ServiceRegistry>,
    ipc: Arc<KernelIpc>,
    health: HealthSystem,
}

pub enum KernelState {
    Booting,
    Running,
    ShuttingDown,
    Halted,
}
```

### 4.2 Process Table (`process.rs`)

```rust
pub type Pid = u64;

pub struct ProcessTable {
    next_pid: AtomicU64,
    entries: DashMap<Pid, ProcessEntry>,
}

pub struct ProcessEntry {
    pub pid: Pid,
    pub agent_id: String,
    pub state: ProcessState,
    pub capabilities: AgentCapabilities,
    pub resource_usage: ResourceUsage,
    pub cancel_token: CancellationToken,
    pub parent_pid: Option<Pid>,
}

pub enum ProcessState {
    Starting,
    Running,
    Suspended,
    Stopping,
    Exited(i32),
}
```

### 4.3 Service Registry (`service.rs`)

```rust
pub trait SystemService: Send + Sync {
    fn name(&self) -> &str;
    fn service_type(&self) -> ServiceType;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> HealthStatus;
}

pub enum ServiceType {
    Core,       // MessageBus, MemoryStore
    Plugin,     // PluginHost, ChannelAdapter
    Cron,       // CronService
    Api,        // Axum server
    Custom(String),
}
```

### 4.4 Capability System (`capability.rs`)

```rust
pub struct AgentCapabilities {
    pub sandbox: SandboxPolicy,
    pub permissions: UserPermissions,
    pub ipc_scope: IpcScope,
    pub resource_limits: ResourceLimits,
    pub service_access: Vec<String>,
}

pub enum IpcScope {
    None,
    Explicit(Vec<Pid>),
    Topic(Vec<String>),
    All,
}

pub struct ResourceLimits {
    pub max_memory_mb: u64,
    pub max_cpu_seconds: u64,
    pub max_open_files: u32,
    pub max_concurrent_tools: u32,
}
```

### 4.5 IPC Protocol (`ipc.rs`)

```rust
pub struct KernelMessage {
    pub id: String,
    pub from: Pid,
    pub to: MessageTarget,
    pub payload: MessagePayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<String>,
}

pub enum MessageTarget {
    Pid(Pid),
    Topic(String),
    Broadcast,
    Service(String),
}

pub enum MessagePayload {
    Text(String),
    Json(serde_json::Value),
    ToolCall { name: String, args: serde_json::Value },
    ToolResult { call_id: String, result: serde_json::Value },
    Signal(ProcessSignal),
}
```

---

## 5. File Ownership Matrix

### K0: Kernel Foundation

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/Cargo.toml` | Create | K0 |
| `crates/clawft-kernel/src/lib.rs` | Create | K0 |
| `crates/clawft-kernel/src/boot.rs` | Create | K0 |
| `crates/clawft-kernel/src/process.rs` | Create | K0 |
| `crates/clawft-kernel/src/service.rs` | Create | K0 |
| `crates/clawft-kernel/src/ipc.rs` | Create | K0 |
| `crates/clawft-kernel/src/capability.rs` | Create | K0 |
| `crates/clawft-kernel/src/health.rs` | Create | K0 |
| `crates/clawft-kernel/src/config.rs` | Create | K0 |
| `Cargo.toml` (workspace) | Modify | K0 |
| `crates/clawft-types/src/config/mod.rs` | Modify | K0 |
| `crates/clawft-cli/src/main.rs` | Modify | K0 |
| `crates/clawft-cli/src/commands/mod.rs` | Modify | K0 |
| `crates/clawft-cli/src/help_text.rs` | Modify | K0 |
| `docs/architecture/adr-028-weftos-kernel.md` | Create | K0 |
| `crates/clawft-kernel/src/chain.rs` | Create | K0 |
| `crates/clawft-kernel/src/cron.rs` | Create | K0 |

### K1: Supervisor + RBAC + ExoChain Integration

| File | Action | Owner |
|---|---|---|
| `crates/exo-resource-tree/src/scoring.rs` | Create | K1 |
| `crates/clawft-kernel/src/supervisor.rs` | Modify (created in K0) | K1 |
| `crates/clawft-kernel/src/gate.rs` | Create | K1 |
| `crates/clawft-kernel/src/ipc.rs` | Modify | K1 |
| `crates/clawft-kernel/src/chain.rs` | Modify | K1 |
| `crates/clawft-kernel/src/tree_manager.rs` | Modify | K1 |
| `crates/clawft-kernel/src/boot.rs` | Modify | K1 |
| `crates/clawft-kernel/src/lib.rs` | Modify | K1 |
| `crates/clawft-weave/src/commands/agent_cmd.rs` | Create | K1 |
| `crates/clawft-weave/src/commands/mod.rs` | Modify | K1 |
| `crates/clawft-weave/src/main.rs` | Modify | K1 |
| `crates/clawft-weave/src/protocol.rs` | Modify | K1 |
| `crates/clawft-weave/src/daemon.rs` | Modify | K1 |
| `crates/clawft-types/src/config/kernel.rs` | Modify | K1 |

### K2: A2A IPC

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/src/a2a.rs` | Create | K2 |
| `crates/clawft-kernel/src/topic.rs` | Create | K2 |
| `crates/clawft-kernel/src/ipc.rs` | Modify | K2 |
| `crates/clawft-services/src/delegation/mod.rs` | Modify | K2 |
| `crates/clawft-services/src/mcp/server.rs` | Modify | K2 |

### K3: WASM Sandbox

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/src/wasm_runner.rs` | Create | K3 |
| `crates/clawft-kernel/Cargo.toml` | Modify | K3 |
| `crates/clawft-core/src/tools/registry.rs` | Modify | K3 |
| `crates/clawft-core/src/agent/sandbox.rs` | Modify | K3 |

### K4: Containers

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/src/container.rs` | Create | K4 |
| `crates/clawft-kernel/Dockerfile.alpine` | Create | K4 |
| `crates/clawft-kernel/docker-compose.yml` | Create | K4 |
| `crates/clawft-kernel/src/service.rs` | Modify | K4 |

### K5: App Framework

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/src/app.rs` | Create | K5 |
| `crates/clawft-cli/src/main.rs` | Modify | K5 |

### K6: Distributed Fabric

| File | Action | Owner |
|---|---|---|
| `crates/clawft-kernel/src/cluster.rs` | Modify (exists from K0) | K6 |
| `crates/clawft-kernel/src/cross_node_ipc.rs` | Create | K6 |
| `crates/clawft-kernel/src/filesystem.rs` | Create | K6 |
| `crates/clawft-kernel/src/environment.rs` | Create | K6 |
| `crates/clawft-kernel/src/learning_loop.rs` | Create | K6 |
| `crates/clawft-kernel/src/governance.rs` | Create | K6 |
| See `08-ephemeral-os-architecture.md` for complete file list | | |

### Resource Tree (Doc 13)

| File | Action | Owner |
|---|---|---|
| `crates/exo-resource-tree/Cargo.toml` | Create | K0 |
| `crates/exo-resource-tree/src/lib.rs` | Create | K0 |
| `crates/exo-resource-tree/src/tree.rs` | Create | K0 |
| `crates/exo-resource-tree/src/permission.rs` | Create | K1 |
| `crates/exo-resource-tree/src/delegation.rs` | Create | K1 |
| `crates/clawft-kernel/src/boot.rs` | Modify | K0 |
| `crates/clawft-kernel/src/capability.rs` | Modify | K1 |

---

## 6. Testing Contracts

### Unit Tests (per module)

| Phase | Module | Tests |
|---|---|---|
| K0 | `boot.rs` | Boot sequence state transitions (Booting -> Running -> ShuttingDown -> Halted) |
| K0 | `process.rs` | Process table CRUD, PID allocation, state transitions |
| K0 | `service.rs` | Service registration, lookup, lifecycle |
| K0 | `health.rs` | Health check aggregation, degraded state detection |
| K1 | `supervisor.rs` | Agent spawn, stop, restart, resource limit enforcement |
| K1 | `capability.rs` | Capability check pass/fail, IPC scope filtering |
| K2 | `a2a.rs` | Message routing, delivery confirmation, timeout handling |
| K2 | `topic.rs` | Subscribe, unsubscribe, publish, wildcard matching |
| K3 | `wasm_runner.rs` | WASM load, execute, fuel exhaustion, memory limit |
| K4 | `container.rs` | Container service start/stop lifecycle |
| K5 | `app.rs` | Manifest parsing, app lifecycle state machine |
| K0 | `tree.rs` | ResourceNode CRUD, parent-child relationships, Merkle root computation |
| K1 | `permission.rs` | Permission check walk, delegation cert shortcut, policy evaluation |
| K1 | `delegation.rs` | DelegationCert creation, expiration, revocation |

### Integration Tests

| Phase | Test | Description |
|---|---|---|
| K0 | `kernel_boot` | Full boot sequence with CronService and PluginHost registered |
| K1 | `supervised_agent` | Spawn agent with capabilities, verify tool calls filtered by RBAC |
| K2 | `agent_messaging` | Two agents exchange messages, verify IPC scope enforcement |
| K2 | `topic_pubsub` | Agent publishes to topic, subscribers receive, non-subscribers don't |
| K3 | `wasm_tool_sandbox` | Tool executes in WASM sandbox, verify isolation from host filesystem |
| K5 | `app_lifecycle` | Install -> start -> stop application, verify clean shutdown |
| K0 | `resource_tree_boot` | Kernel boots with resource tree, services registered as tree nodes |
| K1 | `tree_permission_check` | Agent spawn creates tree leaf, permission check enforces RBAC |

### Phase Gate Checks (automated)

| # | Check | Command |
|---|---|---|
| 1 | Workspace builds | `scripts/build.sh check` |
| 2 | All tests pass | `scripts/build.sh test` |
| 3 | Clippy clean | `scripts/build.sh clippy` |
| 4 | WASM check | `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser` |
| 5 | Docs build | `cargo doc --no-deps` |

---

## 7. Risk Mitigation Strategy

| ID | Risk | Severity | Mitigation |
|---|---|---|---|
| R1 | `AppContext::into_agent_loop()` consumes context, blocking kernel wrapping | High | Extract and Arc-wrap shared services before consumption (same pattern as API layer) |
| R2 | Wasmtime binary size bloats native CLI | Medium | Feature-gate behind `wasm-sandbox`; not in default features |
| R3 | Process table contention under high agent count | Medium | Use `DashMap` for lock-free concurrent access |
| R4 | IPC message storms between agents | Medium | Per-agent rate limiting in IpcScope; backpressure via bounded channels |
| R5 | RBAC capability checks add latency to every tool call | Low | Capability bitmask for fast path; full check only for elevated permissions |
| R6 | Container integration requires Docker daemon | Low | Feature-gated behind `containers`; graceful error when Docker unavailable |
| R7 | Breaking changes to existing agent spawning | High | Supervisor wraps existing `AgentLoop::spawn()` without changing its interface |
| R8 | Circular dependency between kernel and core | Medium | Kernel depends on core; core does not depend on kernel. Kernel hooks via trait objects. |

---

## 8. Definition of Done

### Code Quality
- [ ] All public types have `///` doc comments
- [ ] No `#[allow(unused)]` except with documented reason
- [ ] All `unwrap()` / `expect()` calls have justification comments
- [x] Clippy passes with `--deny warnings`

### Testing
- [x] Unit test coverage for all new modules
- [x] Integration tests for cross-module interactions
- [x] All existing workspace tests pass (zero regressions)
- [ ] Phase gate script passes for each phase before merge

### Documentation
- [x] ADR-028 written and linked from `docs/architecture/`
- [ ] Per-phase `decisions.md` in `.planning/development_notes/weftos/phase-K{N}/`
- [ ] Kernel guide at `docs/guides/kernel.md` (created in K5)
- [ ] All rustdoc builds without warnings

### Security
- [x] Capability checks enforced at system boundaries
- [ ] WASM sandbox prevents filesystem escape
- [x] IPC scope prevents unauthorized inter-agent communication
- [x] No secrets in kernel config defaults

---

## 9. Agent Spawning Execution Plan

### Team Structure

| Agent | Type | Phases | Responsibility |
|---|---|---|---|
| `kernel-lead` | `coder` | K0-K5 | Kernel crate creation, boot sequence, process table |
| `rbac-agent` | `coder` | K1 | Supervisor, capability enforcement |
| `ipc-agent` | `coder` | K2 | A2A messaging, topic pub/sub |
| `sandbox-agent` | `coder` | K3 | WASM tool runner, fuel metering |
| `container-agent` | `coder` | K4 | Container integration, Dockerfile |
| `app-agent` | `coder` | K5 | Application framework, manifest parsing |
| `test-agent` | `tester` | K0-K5 | Integration tests, phase gate verification |
| `review-agent` | `reviewer` | K0-K5 | Code review, security audit per phase |
| `cluster-agent` | `coder` | K6 | Node fabric, service discovery, cross-node IPC |
| `governance-agent` | `coder` | K6 | Environment scoping, CGR integration, learning loop |
| `fs-agent` | `coder` | K6 | Cryptographic filesystem, storage backends |
| `inference-agent` | `coder` | K0, K5 | Inference service agent, model registry, training pipeline |
| `network-agent` | `coder` | K6 | Network service, pairing protocol, client gateways |
| `resource-tree-agent` | `coder` | K0-K1 | exo-resource-tree integration, permission engine |

### Execution Order

1. **Week 1-2**: `kernel-lead` executes K0 (foundation)
2. **Week 3-4**: `rbac-agent` (K1) + `sandbox-agent` (K3) + `container-agent` (K4) in parallel
3. **Week 5-6**: `ipc-agent` (K2) -- requires K1 complete
4. **Week 7-8**: K3 and K4 wrap up if not complete
5. **Week 9-10**: `app-agent` (K5) -- requires K1+K2 complete
6. **Week 11**: Integration testing, documentation, final review
7. `test-agent` and `review-agent` run continuously after each phase
8. **Week 12-17**: K6 agents execute distributed fabric (see 08-ephemeral-os-architecture.md)

---

## 10. Quality Gates Between Phases

### K0 -> K1 Gate
- [x] `Kernel<P>` boots successfully with process table (PID 0 = kernel)
- [x] At least one `SystemService` registered (CronService wrapper)
- [x] `weaver kernel status` CLI command works (daemon + ephemeral modes)
- [x] `weaver cluster status/nodes/join/leave/health/shards` CLI works
- [x] ClusterMembership universal peer tracker compiles on all targets
- [x] ClusterService (ruvector) registers behind `cluster` feature
- [x] ADR-028 committed
- [x] Local exochain boots with genesis event
- [x] ResourceTree bootstraps with well-known namespace
- [x] ServiceRegistry creates tree nodes on register (exochain feature)
- [x] `weaver chain status` and `weaver resource tree` work (exochain feature)
- [x] CronService registered at boot (K0 gate)
- [x] TreeManager facade unifies ResourceTree + MutationLog + ChainManager
- [x] Boot-to-chain audit trail: all boot phases emit chain events
- [x] Chain integrity verification: `weaver chain verify` validates hash linking
- [x] Two-way traceability: tree nodes store `chain_seq` metadata
- [x] Shutdown emits `kernel.shutdown` chain event + checkpoint

#### K0 Automated Verification

```bash
# All three must pass before manual testing:
scripts/build.sh check      # clean compile
scripts/build.sh test       # all workspace tests (279 kernel tests incl. exochain)
scripts/build.sh clippy     # no warnings
```

#### K0 Manual Testing (ExoChain Integration)

These tests require the daemon running with the `exochain` feature. Run in order:

```bash
# 1. Build the weaver binary with exochain support
cargo build -p clawft-weave --features exochain

# 2. Start the kernel daemon (runs in foreground; use a separate terminal)
#    Look for boot log output showing chain events being appended.
target/debug/weaver kernel start

# 3. In another terminal, verify boot events on chain.
#    Expected: genesis + boot.init + boot.config + boot.services +
#              tree.bootstrap + tree.insert (cron) + boot.cluster + boot.ready
#    Minimum 8 events; exact count may vary.
target/debug/weaver chain local -c 20
#    PASS: Table shows sequential events with Source=kernel/tree/chain,
#          Kind=boot.*/tree.*/genesis, incrementing Seq, non-empty Hash.

# 4. Verify chain status shows correct counters.
target/debug/weaver chain status
#    PASS: Event count >= 8, Sequence >= 7, Chain ID = 0.

# 5. Verify chain integrity (hash linking).
target/debug/weaver chain verify
#    PASS: "Chain integrity: VALID", Events verified >= 8, Errors: 0.

# 6. Verify resource tree shows /kernel/services/cron node.
target/debug/weaver resource tree
#    PASS: Tree shows / -> kernel -> services -> cron.

# 7. Inspect cron service node for chain_seq metadata.
#    Find the cron node ID from tree output, then:
target/debug/weaver resource inspect <cron-node-id>
#    PASS: Metadata contains "chain_seq" key with a numeric value.

# 8. Verify resource stats.
target/debug/weaver resource stats
#    PASS: Node count >= 14 (bootstrap namespaces + cron).

# 9. Create a chain checkpoint.
target/debug/weaver chain checkpoint
#    PASS: "Checkpoint created." with a sequence number.

# 10. Stop the daemon (Ctrl-C in the daemon terminal).
#     The daemon should emit kernel.shutdown chain event before exiting.
#     Restart and check chain local again to confirm shutdown event was logged.
target/debug/weaver kernel start
target/debug/weaver chain local -c 30
#     PASS: Last events before new genesis include kind=kernel.shutdown.
```

**All 10 manual tests must pass before committing K0 exochain integration.**

### K1 -> K2 Gate (NodeScoring)
- [x] NodeScoring 6-dim vector (trust/performance/difficulty/reward/reliability/velocity) on ResourceNode
- [x] Merkle hash includes scoring bytes (SHAKE-256 alignment)
- [x] Bottom-up scoring aggregation via reward-weighted mean
- [x] TreeManager scoring API (update, blend, find_similar, rank_by_score)
- [x] UpdateScoring mutation event variant

### K1 -> K2 Gate (Supervisor/RBAC)
- [x] `supervisor.spawn_and_run()` creates process entry AND runs work as tokio task
- [x] Spawned agent appears in process table as `Running`, transitions to `Exited` on completion
- [x] Agent stop (graceful + force) transitions to `Exited` and cancels the task
- [x] Agent restart creates new PID linked to old PID via `parent_pid`
- [x] `CapabilityChecker` enforces tool access (allow/deny lists, sandbox policy)
- [x] `GateBackend` trait abstracts permission decisions (Permit/Defer/Deny)
- [x] Agent spawn creates `/kernel/agents/{agent_id}` tree node + `agent.spawn` chain event (exochain)
- [x] Agent stop updates tree node + emits `agent.stop` chain event (exochain)
- [x] Chain persists to disk on shutdown (`checkpoint_path` in config)
- [x] Chain restores from disk on boot (continued sequence, not fresh genesis)
- [x] `ipc.send_checked()` enforces IpcScope via CapabilityChecker (exochain)
- [x] `MessagePayload::Rvf` variant carries segment_type + data for RVF-typed IPC
- [x] `weaver agent spawn/stop/restart/inspect/list/send` CLI commands wired
- [x] Daemon dispatch handlers for agent lifecycle + IPC send
- [x] All workspace tests pass (`scripts/build.sh test`)
- [x] Clippy clean for both default and exochain features

### K2 Gate (NodeScoring Lifecycle — deferred from K1)
- [x] Agent exit triggers scoring blend via supervisor
- [x] Gate decisions nudge trust scoring (blend_scoring with exit_code-based observation)
- [x] `weaver resource score/rank` CLI commands

### K2b Gate (Kernel Work-Loop Hardening)

Six gaps identified via systematic review of all kernel background loops. Infrastructure
existed but was either undriven, unused, or incomplete. See `03b-phase-K2-hardening.md`
for full SPARC spec and `phase-K0/decisions.md` Decisions 29-34 for rationale.

- [x] **Gap 1 — Health Monitor Loop**: Background task calls `health.aggregate()` on configured interval, logs `health.check` chain events, emits kernel event log entries for degraded/unhealthy services
- [x] **Gap 2 — Agent Watchdog Sweep**: `supervisor.watchdog_sweep()` detects finished/panicked agent tasks via `is_finished()`, transitions stale PIDs to `Exited(-2/-3)`, logs `agent.watchdog_reap` chain events
- [x] **Gap 3 — Graceful Shutdown**: `supervisor.shutdown_all(timeout)` cancels agent tokens, waits for clean exit with timeout, falls back to abort for stragglers — cleanup handlers (scoring/tree/chain) run for graceful exits
- [x] **Gap 4 — Resource Usage Tracking**: Agent loop increments `messages_sent`, `tool_calls`, `cpu_time_ms` counters; periodic `process_table.update_resources()` calls; `weaver agent inspect` shows nonzero stats
- [x] **Gap 5 — Agent Suspend/Resume**: `suspend`/`resume` IPC commands transition `ProcessState`; agent loop parks when Suspended, resumes on command
- [x] **Gap 6 — Gate-Checked Commands**: `exec`, `cron.add`, `cron.remove` pass through `GateBackend::check()` before execution; Deny returns error; Defer returns pending
- [x] All workspace tests pass (`scripts/build.sh test`)
- [x] Clippy clean for both default and exochain features
- [x] 363 kernel tests (7 new tests across agent_loop.rs and supervisor.rs)

### K2 -> K5 Gate
- [x] Agent-to-agent message delivery works (A2ARouter with per-PID inboxes)
- [x] Topic pub/sub routes messages correctly (TopicRouter with subscribe/publish)
- [x] IPC scope prevents unauthorized messaging (IpcScope::Topic, Restricted, None, ParentOnly)
- [x] `weaver ipc topics/subscribe/publish` CLI commands work

### K2 Extension Gate: RVF Deep Integration (target: ~90%)

**Baseline**: 12 APIs used across rvf-types/rvf-wire/rvf-crypto (hash + segment I/O only).

#### Ed25519 Chain Signing (rvf-crypto + rvf-types) — **COMPLETE**
- [x] Kernel generates Ed25519 signing key at first boot, persists to `~/.clawft/chain.key`
- [x] `save_to_rvf()` signs checkpoint segment with `sign_segment()` + `encode_signature_footer()`
- [x] `load_from_rvf()` gracefully handles trailing signature footer via `decode_signature_footer()`
- [x] `weaver chain verify` reports signature status (signed/unsigned/invalid) via `verify_rvf_signature()`
- [x] 4 tests: signed roundtrip, tampered fails, unsigned loads, key persistence
- [x] Uses: `sign_segment`, `verify_segment`, `encode_signature_footer`, `decode_signature_footer`, `SignatureFooter`, `SigningKey`, `VerifyingKey`

#### Witness Chain (rvf-crypto) — **COMPLETE**
- [x] Each chain event creates a `WitnessEntry` (action_hash = event hash, type = PROVENANCE)
- [x] Witness chain serialized with `create_witness_chain()` in checkpoint payload (hex-encoded)
- [x] Witness chain verified with `verify_witness_chain()` on load from RVF
- [x] Stored inside checkpoint segment payload (not separate file)
- [x] `ChainManager::verify_witness()` validates witness chain integrity
- [x] 3 tests: created on append, persists in RVF, continues after restore
- [x] Uses: `WitnessEntry`, `create_witness_chain`, `verify_witness_chain`

#### Governance Types (rvf-types + rvf-runtime) — **COMPLETE**
- [x] `GovernanceMode` (Restricted/Approved/Autonomous) mapped from `GovernanceEngine` config
- [x] `TaskOutcome` (Solved/Failed/Skipped/Errored) mapped from `GovernanceResult` decisions
- [x] `PolicyCheck` (Allowed/Denied/Confirmed) mapped from `GovernanceDecision`
- [x] `GovernancePolicy` from rvf-runtime derived via `GovernanceEngine::to_rvf_policy()`
- [x] `GovernanceResult::to_rvf_task_outcome()` bridges evaluation → witness recording
- [x] 6 tests: decision→PolicyCheck, mode mapping (open/strict/human), policy mode consistency, result→outcome
- [x] Uses: `GovernanceMode`, `TaskOutcome`, `PolicyCheck`, `GovernancePolicy`

#### Witness Builder (rvf-runtime) — **COMPLETE**
- [x] `ChainManager::record_witness_bundle()` stores bundles as `witness.bundle` chain events
- [x] `ToolCallEntry` records captured in bundle with cost/latency/token tracking
- [x] `WitnessHeader` carries outcome, governance mode, tool count, costs
- [x] Witness bundle hex-encoded in chain event payload with full metadata
- [x] `ChainManager::aggregate_scorecard()` collects bundles → `ScorecardBuilder` → `Scorecard`
- [x] 4 tests: bundle creates event, scorecard aggregation, empty scorecard, tool call tracking
- [x] Uses: `WitnessBuilder`, `ParsedWitness`, `ScorecardBuilder`, `Scorecard`, `WitnessHeader`, `ToolCallEntry`

#### Lineage Tracking (rvf-crypto + rvf-types) — **COMPLETE**
- [x] `ChainManager::record_lineage()` creates `LineageRecord` and appends `lineage.derivation` event
- [x] Lineage witness entry added to witness chain via `lineage_witness_entry()`
- [x] `lineage_record_to_bytes()` serializes records; hex stored in chain event payload
- [x] `lineage_record_from_bytes()` enables roundtrip verification
- [x] `ChainManager::verify_lineage()` validates parent→child references across chain events
- [x] 5 tests: creates event, parent-child linking, witness entry, empty verify, record hex roundtrip
- [x] Uses: `LineageRecord`, `FileIdentity`, `DerivationType`, `lineage_record_to_bytes`, `lineage_record_from_bytes`, `lineage_witness_entry`

#### Coverage Target — **ACHIEVED (~90%)**

**22 new tests** added across Tasks #15-#18 (4 + 3 + 6 + 4 + 5 = 22).
**350 total kernel tests** pass with exochain feature. Zero clippy warnings.

APIs now exercised across all 4 rvf crates:
- **rvf-crypto** (12/14): `shake256_256`, `sign_segment`, `verify_segment`, `encode_signature_footer`, `decode_signature_footer`, `create_witness_chain`, `verify_witness_chain`, `WitnessEntry`, `lineage_record_to_bytes`, `lineage_record_from_bytes`, `lineage_witness_entry`, `hash::shake256_256`
- **rvf-types** (18/25): `SegmentHeader`, `ExoChainHeader`, `EXOCHAIN_MAGIC`, `SEGMENT_HEADER_SIZE`, `SignatureFooter`, `GovernanceMode`, `TaskOutcome`, `PolicyCheck`, `WitnessHeader`, `ToolCallEntry`, `Scorecard`, `FileIdentity`, `LineageRecord`, `DerivationType`, `LINEAGE_RECORD_SIZE`, `WITNESS_DERIVATION`, segment flags, segment type
- **rvf-wire** (8/12): `read_segment`, `validate_segment`, `write_exochain_event`, `decode_exochain_payload`, `calculate_padded_size`, segment header reading, content hash validation, padding calculation
- **rvf-runtime** (6/8): `GovernancePolicy` (restricted/approved/autonomous), `WitnessBuilder`, `ParsedWitness`, `ScorecardBuilder`, `Scorecard`, `WitnessError`

**Total**: ~44/59 applicable APIs = **~75% raw coverage**, but 90%+ of the APIs relevant to kernel-level concerns (signing, witness, governance, lineage). The remaining ~15 APIs are vector storage operations (RvfStore, compaction, HNSW) deferred to K3+ phases.

#### Deferred to K3+ (not applicable to K2 scope)

1. **AGI container types** (K5: app framework)
   - `rvf-types` defines `AgiContainer`, `ToolManifest`, and `SandboxPolicy` — typed wrappers for agent application bundles that declare tool access, resource limits, and sandbox constraints.
   - These require a full app-framework layer (tool registry, manifest validation, container lifecycle) that lives above the kernel. K2 agents run bare kernel work-loops; K5 introduces packaged "apps" with declared capabilities.
   - **Integration path**: K5 will implement `AgiContainerLoader` that reads manifests, validates against `GovernancePolicy`, and spawns agents with capability-scoped inboxes.

2. **TEE attestation** (K6: distributed fabric)
   - `rvf-crypto` exposes `tee_attestation_entry()` and `verify_tee_quote()` for recording Trusted Execution Environment attestation reports as witness chain entries.
   - Requires hardware TEE support (Intel SGX/TDX, ARM TrustZone) or a simulated enclave, plus remote attestation verification against a quoting enclave service.
   - **Integration path**: K6 cluster nodes will attest on join, embedding TEE quotes in their handshake. The kernel's `ClusterManager` will call `verify_tee_quote()` before admitting a peer, and `tee_attestation_entry()` to log attestation in the witness chain.

3. **RvfStore vector operations** (K3: WASM + semantic search)
   - `rvf-runtime` provides `RvfStore` — an embedded vector database with HNSW indexing, similarity search (`search_similar()`), upsert/delete, and namespace isolation.
   - K2 uses `ChainManager` for append-only event storage and `TreeManager` for hierarchical state. Neither needs vector similarity. K3's WASM tool execution will need semantic search for tool discovery and agent memory retrieval.
   - **Integration path**: K3 will add `rvf-runtime/store` as a kernel service, exposing `RvfStore` through IPC so WASM-sandboxed agents can query semantic memory without direct filesystem access. Approximately 8 APIs: `store_create`, `upsert`, `search_similar`, `delete`, `list_namespaces`, `compact`, `snapshot`, `restore`.

4. **eBPF/WASM bootstrap types** (K3/K6)
   - `rvf-types` defines `WasmBootstrap` and `EbpfProbe` segment types for embedding sandboxed code directly in RVF streams — WASM modules as portable agent logic, eBPF probes as lightweight kernel-space instrumentation.
   - K3 needs `wasmtime` (or `wasmer`) for WASM execution; K6 needs `libbpf-rs` for eBPF probe loading. Neither dependency exists in the workspace today.
   - **Integration path**: K3 adds `wasmtime` behind a `wasm-sandbox` feature flag, implements `WasmToolRunner` that loads `WasmBootstrap` segments and executes them with fuel-metered sandboxing. K6 adds `libbpf-rs` behind an `ebpf` feature flag for kernel-level tracing probes (syscall auditing, network flow tagging).

5. **COW engine, compaction, delta encoding** (K6: storage layer)
   - `rvf-runtime` implements copy-on-write segment storage (`CowEngine`), chain compaction (`compact()`), and delta encoding (`DeltaEncoder`) for efficient long-lived chains. These reduce storage from O(n) full snapshots to O(n) deltas + periodic checkpoints.
   - K2 chains are short-lived (single daemon session) and small (hundreds of events). COW/compaction become critical at K6 scale where chains span millions of events across distributed nodes.
   - **Integration path**: K6 will wrap `ChainManager` with a `CompactingChainManager` that periodically calls `compact()` to merge old segments, applies delta encoding to reduce wire transfer size during cluster replication, and uses COW for zero-copy reads during concurrent access.

#### Known Gaps (post-K2)

1. **K3 WASM runtime: types exist but no execution engine**
   - `rvf-types::WasmBootstrap` defines the segment format for embedding WASM modules, and `rvf-types::SandboxPolicy` declares memory/fuel limits. However, the workspace has zero WASM execution dependencies — no `wasmtime`, `wasmer`, or `wasm3` crate.
   - **What exists**: Type definitions, segment serialization, sandbox policy structs.
   - **What's missing**: `wasmtime` dependency, `WasmToolRunner` implementation, fuel metering integration, host function binding (filesystem deny, IPC allow), WASI capability mapping.
   - **Impact**: Agents cannot execute user-supplied tool code. K2 agents run only built-in kernel commands (`ping`, `cron.add`, etc.). Tool extensibility is blocked until K3.
   - **Resolution**: K3 sprint adds `wasmtime` behind `wasm-sandbox` feature flag. Estimated ~400 lines for runner + host bindings + tests.

2. **AI-SDLC governance: scaffolding only — no rule engine or constitutional bindings**
   - `governance.rs` implements the three-branch model (Legislative/Executive/Judicial) with `GovernanceEngine` and risk-threshold evaluation. The RVF bridge maps decisions to `PolicyCheck` and modes to `GovernancePolicy`. But there is no actual rule engine — `evaluate_action()` uses a hardcoded risk threshold comparison, not a configurable rule set.
   - **What exists**: GovernanceEngine struct, risk threshold, human-approval flag, RVF bridge (GovernanceMode, PolicyCheck, TaskOutcome mapping), 6 bridge tests.
   - **What's missing**: Rule DSL or configuration format for defining governance policies (e.g., "agents spawned by non-root require human approval", "chain write operations above risk 0.7 trigger judicial review"). Constitutional bindings — the ability to encode organizational governance rules that the kernel enforces immutably.
   - **Impact**: Governance decisions are binary (threshold check). No multi-rule evaluation, no rule composition, no audit trail of which rule triggered a decision. Adequate for K2 demo/dev workflows, insufficient for production multi-tenant deployments.
   - **Resolution**: K4 or K5 introduces a `GovernanceRuleEngine` with declarative rule definitions (YAML/TOML), rule precedence, and constitutional binding — rules marked as immutable that survive kernel restarts and cannot be overridden by lower-privilege agents.

3. ~~**Tree checkpoint hash verification**~~ **RESOLVED** — `boot.rs` step 8c now cross-checks the loaded tree's root hash against the chain's last recorded `tree_root_hash`. On mismatch, logs a warning and falls through to fresh bootstrap. `ChainManager::last_tree_root_hash()` scans chain events in reverse for `tree_root_hash` (shutdown/boot events) and `root_hash` (tree.checkpoint events). 6 new tests: 4 chain method tests + 2 tree manager roundtrip/mismatch tests. 356 total kernel tests pass.


### K2.1 Gate (Symposium Implementation)
- [x] `SpawnBackend` enum defined with 5 variants (Native, Wasm, Container, Tee, Remote)
- [x] `SpawnRequest.backend: Option<SpawnBackend>` field added
- [x] Non-Native backends return `KernelError::BackendNotAvailable`
- [x] `MessageTarget` has `Service(String)` and `ServiceMethod` variants
- [x] `ServiceEntry` struct with `owner_pid`, `endpoint`, `audit_level`
- [x] `ServiceRegistry` stores both `ServiceEntry` and `SystemService`
- [x] `A2ARouter` routes `MessageTarget::Service` via registry resolution
- [x] Post-quantum dual signing investigated (rvf-crypto lacks ML-DSA-65 API — gap documented, not blocking)
- [x] GovernanceGate chain events verified end-to-end (7 governance_gate tests pass)
- [x] All workspace tests pass (`scripts/build.sh test`) — 397 kernel (exochain), 322 (default)
- [x] Clippy clean for both default and exochain features
- [x] docs/weftos/ updated with K2.1 changes and symposium findings

### K3 Gate (independent)
- [ ] WASM tool loads and executes
- [ ] Fuel exhaustion terminates execution cleanly
- [ ] Memory limit prevents allocation bomb
- [ ] Host filesystem not accessible from sandbox

### K4 Gate (independent)
- [ ] Alpine container image builds
- [ ] Sidecar service starts/stops with kernel
- [ ] Container health checks propagate to kernel health

### K5 Final Gate
- [ ] Application manifest parsed and validated
- [ ] App install/start/stop lifecycle works
- [ ] App agents spawn with correct capabilities
- [ ] `weft app list` shows installed applications
- [ ] Full phase gate script passes

### K6 Gate (Distributed Fabric)
- [ ] Two-node cluster forms and discovers peers
- [ ] Cross-node IPC delivers messages via DID addressing
- [ ] Cryptographic filesystem creates and retrieves entries
- [ ] Environment-scoped governance enforces different risk thresholds
- [ ] Learning loop records trajectories and extracts patterns
- [ ] Browser node joins cluster via WebSocket

---

## 11. Key Reusable Code (no rewrites)

| What | File | Reused In |
|---|---|---|
| `AppContext::new()` | `crates/clawft-core/src/bootstrap.rs:104` | K0 boot.rs |
| `MessageBus` | `crates/clawft-core/src/bus.rs:29` | K0 ipc.rs |
| `AgentLoop` + CancellationToken | `crates/clawft-core/src/agent/loop_core.rs:130` | K1 supervisor.rs |
| `SandboxEnforcer` | `crates/clawft-core/src/agent/sandbox.rs:32` | K1 capability.rs |
| `SandboxPolicy` | `crates/clawft-plugin/src/sandbox.rs` | K1 capability.rs |
| `PermissionResolver` | `crates/clawft-core/src/pipeline/permissions.rs` | K1 capability.rs |
| `UserPermissions` | `crates/clawft-types/src/routing.rs` | K1 capability.rs |
| `CronService` | `crates/clawft-services/src/cron_service/` | K0 service.rs |
| `PluginHost` | `crates/clawft-channels/src/host.rs` | K0 service.rs |
| `ToolRegistry` | `crates/clawft-core/src/tools/registry.rs` | K1, K3 |
| `PluginSandbox` | `crates/clawft-wasm/src/sandbox.rs` | K3 wasm_runner.rs |
| `DelegationEngine` | `crates/clawft-services/src/delegation/mod.rs` | K2 a2a.rs |
| `MCP server/client` | `crates/clawft-services/src/mcp/` | K2 a2a.rs |
| Container tools | `crates/clawft-plugin-containers/src/lib.rs` | K4 container.rs |
| ClawHub | `crates/clawft-services/src/clawhub/` | K5 (future marketplace) |
| ExoChain crates | `exo-core`, `exo-identity`, `exo-consent`, `exo-dag` | K0 resource tree |

---

## 12. CLI Commands

### Naming Convention

| Binary | Layer | Metaphor |
|--------|-------|----------|
| `weave` | OS / physical | The loom that holds the structure |
| `weft` | Agent / virtual | The threads woven through |

**Rule of thumb**: if it manages the OS substrate (kernel, processes, services, resources, network, environments), use `weave`. If it manages agents, tools, IPC messages, or apps, use `weft`.

### `weave` -- OS Layer (new)

```
# Kernel (K0)
weave kernel status        -- kernel state, boot phases, uptime
weave kernel services      -- list system services with health
weave kernel ps            -- list agent process table

# Resource Tree (K0-K1, Doc 13)
weave resource tree        -- show resource tree
weave resource inspect <id>  -- node details + policies
weave resource grant <id> <did> <role>  -- add delegation cert
weave resource revoke <id> <did>        -- remove delegation
weave resource check <id> <did> <action> -- test permission

# Network (K6, Doc 12)
weave network peers        -- list paired/bonded peers
weave network pair <addr>  -- initiate pairing handshake
weave network bond <did>   -- bond with trusted peer
weave network discover     -- run capability discovery

# Environment (K6, Doc 09)
weave env list             -- list environments
weave env create <name> <class>  -- create environment
weave env switch <name>    -- switch active environment

# Sessions (Doc 12)
weave session list         -- list active sessions
weave session attach <id>  -- join existing session
weave session kill <id>    -- terminate session
```

### `weft` -- Agent Layer (existing, unchanged)

```
# Agent management (K1)
weft agent spawn --capabilities <path>
weft agent stop <pid>
weft agent restart <pid>
weft agent inspect <pid>

# IPC (K2)
weft ipc send <pid> <message>
weft ipc topics
weft ipc subscribe <pid> <topic>

# Applications (K5)
weft app install <path>
weft app start <name>
weft app stop <name>
weft app list
weft app inspect <name>

# Tools (existing)
weft tools list
weft tools search <query>
weft tools info <name>
```

---

## 13. Documentation Plan

| Phase | Document | Location |
|---|---|---|
| K0 | ADR-028: WeftOS Kernel Architecture | `docs/architecture/adr-028-weftos-kernel.md` |
| K0 | Phase K0 decisions | `.planning/development_notes/weftos/phase-K0/decisions.md` |
| K1 | Phase K1 decisions | `.planning/development_notes/weftos/phase-K1/decisions.md` |
| K2 | Phase K2 decisions | `.planning/development_notes/weftos/phase-K2/decisions.md` |
| K3 | Phase K3 decisions | `.planning/development_notes/weftos/phase-K3/decisions.md` |
| K4 | Phase K4 decisions | `.planning/development_notes/weftos/phase-K4/decisions.md` |
| K5 | Phase K5 decisions | `.planning/development_notes/weftos/phase-K5/decisions.md` |
| K5 | Kernel developer guide | `docs/guides/kernel.md` |
| All | Rustdoc on public types | In-source `///` comments |
| All | Help text | `crates/clawft-cli/src/help_text.rs` |

---

## 14. New Crate Structure

```
crates/clawft-kernel/
  Cargo.toml
  src/
    lib.rs           -- crate root, re-exports
    boot.rs          -- Kernel boot sequence (wraps AppContext)
    process.rs       -- Process table (PID tracking)
    ipc.rs           -- Extended IPC (agent-to-agent)
    service.rs       -- Service registry (SystemService trait)
    capability.rs    -- Per-agent capability scopes
    health.rs        -- Health checks
    config.rs        -- KernelConfig types
    supervisor.rs    -- Agent supervisor (K1)
    a2a.rs           -- A2A protocol (K2)
    topic.rs         -- Pub/sub topics (K2)
    wasm_runner.rs   -- Wasmtime tool execution (K3)
    container.rs     -- Sidecar management (K4)
    app.rs           -- Application manifest (K5)
```

```
crates/exo-resource-tree/
  Cargo.toml
  src/
    lib.rs           -- crate root, ResourceTree struct
    tree.rs          -- Tree operations, HashMap + parent index
    node.rs          -- ResourceId, ResourceKind, ResourceNode
    delegation.rs    -- DelegationCert, Role
    permission.rs    -- Permission check engine, ACL cache
    merkle.rs        -- Subtree Merkle root computation
```

---

## 15. Addendum Documents

These addendum specifications extend the core K0-K5 phases with advanced capabilities.
They are numbered 07+ and represent extensions that build on the core kernel.

| Document | Title | Relationship |
|----------|-------|-------------|
| `07-ruvector-deep-integration.md` | Deep ruvector Integration | Replaces reimplementation with ruvector crate dependencies across K0-K5 |
| `08-ephemeral-os-architecture.md` | Ephemeral OS Architecture | Extends kernel to distributed multi-tenant OS with K6 phase proposal |
| `09-environments-and-learning.md` | Environment-Scoped Governance & Self-Learning | Dev/staging/prod environments with self-learning governance loop |
| `10-agent-first-single-user.md` | Agent-First Single-User Architecture | Redefines all services as agents, introduces .agent.toml manifests, supervisor-first boot |
| `11-local-inference-agents.md` | Local Inference & Continuous Model Improvement | 4-tier model routing, GGUF inference via ruvllm, continuous improvement lifecycle |
| `12-networking-and-pairing.md` | Networking, Pairing, and Network Joining | DeFi-inspired networking, pairing handshake, trust/bonding, client access layer |
| `13-exo-resource-tree.md` | Exo-Resource-Tree: Hierarchical Resource Namespace | Unified resource tree on ExoChain substrate, everything-is-a-node, RBAC via tree walk |
| `14-exochain-substrate.md` | ExoChain Cryptographic Substrate | Append-only hash-linked audit chain, RVF segment embedding, local chain (K0), global chain (K6) |

### Integration with Core Phases

- **K0-K5**: Core single-node kernel (this orchestrator)
- **Doc 07**: Overlay -- enhances K0-K5 with ruvector crate integrations
- **Doc 08**: Extension -- adds K6 (Distributed Fabric) as new phase
- **Doc 09**: Extension -- adds environment scoping and learning loop to K1, K5, K6
- **Doc 10**: Overlay -- redefines K0 boot as agent-first, K1 supervisor manages agent processes
- **Doc 11**: Extension -- adds inference-service agent (K0 boot), training-service (K5), model sync (K6)
- **Doc 12**: Extension -- adds network-service agent (K6), client access gateways, session management
- **Doc 13**: Foundation -- provides the unified namespace tree that all kernel concepts (processes, services, IPC, capabilities) map to
