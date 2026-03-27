# Phase K0: Kernel Foundation -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. KernelConfig split across crates
**Problem**: KernelConfig must be in the root Config (clawft-types) but AgentCapabilities is defined in clawft-kernel, creating a circular dependency.

**Decision**: Define a minimal `KernelConfig` in `clawft-types/src/config/kernel.rs` with only serializable fields (enabled, max_processes, health_check_interval_secs). The kernel crate defines `KernelConfigExt` that wraps the base config and adds `AgentCapabilities`.

**Rationale**: Avoids circular dependency while keeping serde-compatible config in clawft-types where it belongs.

### 2. DashMap for concurrent collections
**Decision**: Added `dashmap = "6"` as a workspace dependency. Used for ProcessTable and ServiceRegistry.

**Rationale**: Lock-free concurrent reads are essential for a kernel that multiple subsystems query simultaneously. DashMap is well-established and provides the HashMap-like API needed.

### 3. ProcessTable PID allocation
**Decision**: PIDs are monotonically increasing from AtomicU64, starting at 1 (PID 0 reserved for the kernel process). PIDs are never reused within a session.

**Rationale**: Simplicity and debuggability. PID reuse adds complexity (race conditions with stale references) for no benefit in a single-session kernel.

### 4. Boot sequence owns AppContext
**Decision**: Kernel::boot() creates AppContext internally and holds it as `Option<AppContext<P>>`. The `take_app_context()` method allows extracting it for agent loop consumption.

**Rationale**: The kernel needs to extract Arc references (bus, tools, etc.) from AppContext before it's consumed by into_agent_loop(). Holding it as Option allows this two-phase pattern.

### 5. Console REPL is stubbed
**Decision**: Only boot event types (BootEvent, BootPhase, LogLevel) and output formatting are implemented. The interactive REPL loop is not implemented in K0.

**Rationale**: Interactive stdin handling with async is complex and requires careful signal handling. The boot event types provide immediate value for `weft kernel boot` output. The REPL can be added later.

### 6. Pre-existing clippy fixes
**Decision**: Fixed all pre-existing clippy issues in clawft-types, clawft-core, clawft-cli, and clawft-services as part of K0. These were collapsible-if, derivable-impls, match-like-matches-macro, map_or->is_none_or, and manual-pattern-char-comparison lints.

**Rationale**: The project requires clippy-clean builds. Fixing pre-existing issues is necessary for the gate to pass.

### 7. Two-layer cluster architecture
**Problem**: WeftOS must connect all ephemeral instances — native, browser, edge, IoT — but ruvector's distributed crates use `std::net::SocketAddr` which doesn't compile to WASM.

**Decision**: Two-layer architecture. `ClusterMembership` is a universal peer tracker that compiles on all platforms (uses `Option<String>` addresses and `NodePlatform` enum). `ClusterService` (ruvector-powered, native-only) wraps `ruvector_cluster::ClusterManager` behind `#[cfg(feature = "cluster")]` and syncs state into `ClusterMembership`. Browser/edge nodes join via WebSocket to a coordinator and get full cluster visibility.

**Rationale**: Keeps the universal layer WASM-compatible while still leveraging ruvector's tested coordination primitives (consistent hashing, discovery, shard routing) on native coordinator nodes.

### 8. Ruvector-core feature-gating
**Problem**: All 3 ruvector distributed crates (`ruvector-cluster`, `ruvector-raft`, `ruvector-replication`) list `ruvector-core` as a dependency, but none of them import from it. `ruvector-core` pulls in heavy deps (hnsw, quantization) that bloat the build and are not needed for coordination primitives.

**Decision**: Feature-gate `ruvector-core` as optional behind a `vector-store` feature in all 3 crates. Pushed to `weave-logic-ai/ruvector` fork.

**Rationale**: Allows using the coordination primitives (cluster, consensus, replication) without pulling in the entire vector store stack. When vector operations are needed, enable `vector-store`.

### 9. ClusterMembership always present
**Decision**: `ClusterMembership` is a field on `Kernel<P>` and is always created during boot, even without the `cluster` feature. It tracks the local node at minimum.

**Rationale**: All kernel subsystems and CLI commands can query cluster state uniformly. Without `cluster` feature, the membership contains only the local node. With `cluster`, the `ClusterService` syncs discovered native nodes into it. Browser nodes that join via WS also get registered here.

## What Was Skipped

1. ~~**Ruvector integration**~~ -- **Done**: `ClusterMembership` (universal) + `ClusterService` (native, feature-gated) integrated. `weaver cluster` CLI added. See decisions 7-9 above.
2. ~~**Exo-resource-tree**~~ -- **Done**: `exo-resource-tree` crate created with core types, tree CRUD, Merkle recomputation, and namespace bootstrap. Integrated into kernel boot behind `exochain` feature gate. CLI commands `weaver resource {tree, inspect, stats}` added. See decisions 13 below.
3. **Interactive console REPL** -- only event types and formatting implemented
4. ~~**CronService wrapper**~~ -- **Done**: `CronService` registered at boot as a `SystemService`. See `crates/clawft-kernel/src/cron.rs` and boot.rs step 5b.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/Cargo.toml` | 46 | Crate manifest |
| `crates/clawft-kernel/src/lib.rs` | 47 | Crate root with re-exports |
| `crates/clawft-kernel/src/boot.rs` | ~330 | Kernel struct, boot/shutdown, state machine |
| `crates/clawft-kernel/src/process.rs` | ~280 | ProcessTable, ProcessEntry, PID allocation |
| `crates/clawft-kernel/src/service.rs` | ~200 | SystemService trait, ServiceRegistry |
| `crates/clawft-kernel/src/ipc.rs` | ~190 | KernelIpc, KernelMessage, MessageTarget |
| `crates/clawft-kernel/src/capability.rs` | ~200 | AgentCapabilities, IpcScope, ResourceLimits |
| `crates/clawft-kernel/src/health.rs` | ~200 | HealthSystem, HealthStatus, OverallHealth |
| `crates/clawft-kernel/src/console.rs` | ~200 | BootEvent, BootPhase, LogLevel, BootLog |
| `crates/clawft-kernel/src/config.rs` | ~60 | KernelConfigExt (wraps types-level config) |
| `crates/clawft-kernel/src/error.rs` | ~55 | KernelError, KernelResult |
| `crates/clawft-types/src/config/kernel.rs` | ~75 | KernelConfig (base, in types crate) |
| `crates/clawft-cli/src/commands/kernel_cmd.rs` | ~240 | CLI kernel subcommand |

## Files Modified

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Added clawft-kernel member, dashmap workspace dep |
| `crates/clawft-types/src/config/mod.rs` | Added kernel module, KernelConfig field to Config |
| `crates/clawft-types/src/config/voice.rs` | Fixed derivable-impls clippy lints |
| `crates/clawft-core/src/agent/loop_core.rs` | Fixed collapsible-if clippy lints |
| `crates/clawft-core/src/agent/verification.rs` | Fixed collapsible-if and char comparison lints |
| `crates/clawft-services/src/api/delegation.rs` | Fixed map_or->is_none_or lint |
| `crates/clawft-cli/src/main.rs` | Added Kernel subcommand |
| `crates/clawft-cli/src/commands/mod.rs` | Added kernel_cmd module |
| `crates/clawft-cli/src/commands/agent.rs` | Fixed collapsible-if lint |
| `crates/clawft-cli/src/commands/gateway.rs` | Fixed collapsible-if lints |
| `crates/clawft-cli/src/help_text.rs` | Added kernel help topic |
| `crates/clawft-cli/Cargo.toml` | Added clawft-kernel dependency |

### Decision 10: Exochain Fork Strategy

**Context**: ExoChain upstream uses `serde_cbor` (unmaintained) and has
native-only deps (libp2p, graphql) that block WASM compilation.

**Decision**: Fork to weave-logic-ai/exochain. Replace serde_cbor with
ciborium. Feature-gate native-only crates.

**Rationale**: Minimal diff from upstream, ciborium is the maintained
successor, feature gates preserve functionality for native builds.

### Decision 11: Local Chain First, Global Chain Deferred

**Context**: Full exochain includes local + global root chain with
cross-node consensus.

**Decision**: K0 implements local chain only (chain_id=0). Global root
chain (chain_id=1) deferred to K6 when ruvector-raft consensus is
integrated.

**Rationale**: Local chain provides immediate value (audit trail) without
distributed consensus complexity. K6 timeline aligns with networking
maturity.

### Decision 12: RVF ExoChain Segment Types (0x40-0x42)

**Context**: ExoChain events need persistent storage in RVF format.

**Decision**: Reserve segment types 0x40 (ExochainEvent), 0x41
(ExochainCheckpoint), 0x42 (ExochainProof) in rvf-types. 64-byte
ExoChainHeader prefix with CBOR payload.

**Rationale**: Consistent with existing RVF segment architecture.
64-byte header provides efficient fixed-offset parsing. CBOR payload
preserves schema flexibility.

### Decision 13: ERT K0 Scope — Tree CRUD and Merkle Only

**Context**: The exo-resource-tree spec (Doc 13) covers tree CRUD,
permission engine, delegation certificates, and CLI integration.

**Decision**: K0 implements tree CRUD, Merkle recomputation, bootstrap,
and mutation log. Permission engine (check()) stubs to Allow. Delegation
is type-only. CLI covers tree/inspect/stats.

**Rationale**: Tree structure is prerequisite for all permission checks.
Permission engine requires capability model (K1). Stubs allow K1 to
enable gradually without breaking K0 code.

### Decision 14: TreeManager Facade

**Context**: Three subsystems (ChainManager, ResourceTree, MutationLog) existed as
islands — ChainManager appended hash-linked events, ResourceTree stored nodes with
Merkle hashes, MutationLog recorded DAG-backed mutations, but none communicated.
Boot created both chain and tree but didn't log boot phases to chain. Service
registration had `register_with_tree()` but it didn't create chain events.

**Decision**: Create `TreeManager` facade (`crates/clawft-kernel/src/tree_manager.rs`)
that holds all three subsystems behind a unified API. Every tree mutation atomically:
modifies tree → appends chain event → stores `chain_seq` metadata on the node →
appends MutationEvent to log. This ensures two-way traceability: tree nodes know
their chain event sequence number, chain events reference tree paths in their payload.

**Rationale**: Guarantees that the tree and chain can never drift out of sync. The
facade pattern keeps the individual subsystems simple while enforcing the invariant
that every mutation is auditable. Enables `weaver chain verify` to prove integrity
and `weaver resource inspect` to show chain provenance for each node.

### Decision 15: Boot-to-Chain Audit Trail

**Context**: Kernel boot executed multiple phases (init, config, services, cluster,
tree bootstrap) but none of these were recorded in the hash chain. The chain only
contained events appended after boot by explicit user actions.

**Decision**: Log each boot phase as a chain event during startup:
- `boot.init` — kernel version
- `boot.config` — max_processes, health_interval
- `boot.services` — registered service count
- `tree.bootstrap` — node count, root hash (via TreeManager)
- `boot.cluster` — node_id
- `boot.ready` — elapsed_ms, tree_root_hash, process/service counts

Additionally, shutdown emits `kernel.shutdown` with tree_root_hash, chain_seq,
and tree_nodes, then creates a chain checkpoint.

**Rationale**: Makes the entire kernel lifecycle auditable. `weaver chain local`
now shows a complete boot trace from genesis through ready state. Shutdown
checkpointing enables future K1 work on persistent state recovery.

## Test Summary

- 66 new tests in clawft-kernel (process table, service registry, health, IPC, boot, console, config, capability)
- 8 new tests in tree_manager (bootstrap, insert, remove, update_meta, register_service, stats, checkpoint, chain integrity)
- 1 new test in chain (verify_integrity)
- 2 new tests in kernel_cmd (format_bytes, args parsing)
- All pre-existing tests continue to pass

## K0 Completion

**Gate passed**: 2026-03-01
- `scripts/build.sh check` -- PASS
- `scripts/build.sh test` -- PASS (all workspace tests)
- `scripts/build.sh clippy` -- PASS (zero warnings)
- Manual verification of exochain boot/chain/tree/checkpoint -- PASS

K1 (Supervisor + RBAC + ExoChain Integration) started immediately after K0 gate.

## K1 Decisions (continued)

### Decision 16: cognitum-gate-tilezero Integration (K1, not deferred)

**Context**: K1 plan originally deferred TileZero to K2. User explicitly pulled it into K1:
"we do not want to defer this."

**Decision**: Implement `TileZeroGate` in `gate.rs` behind `#[cfg(feature = "tilezero")]`.
The adapter wraps `Arc<TileZero>` + optional `Arc<ChainManager>`, maps our gate parameters
to `ActionContext`, bridges async `TileZero::decide()` to sync `GateBackend::check()` via
`tokio::task::block_in_place`, and logs gate.permit/gate.defer/gate.deny chain events.

**Rationale**: TileZero is the coherence arbiter for the ruvector ecosystem. Having it wired
as an alternative `GateBackend` means permission decisions can be based on TileZero's
256-tile coherence model (three stacked filters: Structural/Shift/Evidence) rather than just
binary capability checks. Ed25519-signed PermitTokens and blake3-chained WitnessReceipts
provide cryptographic auditability.

### Decision 17: SHAKE-256 Chain Hashing (rvf-crypto)

**Context**: Chain.rs used raw SHA-256 (`sha2::Sha256`) and critically did NOT include
the payload in `compute_event_hash()`. This meant payloads could be swapped without breaking
the hash chain — a real integrity vulnerability. User requested proper hashing using the
ruvector ecosystem's crypto primitives.

**Decision**: Replace `sha2` with `rvf-crypto` (SHAKE-256). Add `payload_hash` field to
`ChainEvent`. The hash scheme is now:

```
payload_hash = SHAKE-256(canonical_json_bytes(payload)) | [0; 32] if None
hash = SHAKE-256(sequence ‖ chain_id ‖ prev_hash ‖ source ‖ 0x00 ‖ kind ‖ 0x00 ‖ timestamp ‖ payload_hash)
```

Null-byte separators between source/kind prevent domain collisions.

**Rationale**:
1. **Payload integrity**: Every chain event now commits to its payload content
2. **2-way verification**: Given an event, verify chain link (prev_hash) AND content (payload_hash) independently
3. **Ecosystem alignment**: SHAKE-256 is the canonical hash for RVF witness chains
4. **Cross-service verification**: Two services producing the same event get the same payload_hash
5. **Domain separation**: Null-byte separators prevent "foo" + "bar.baz" colliding with "foo.bar" + "baz"

### Decision 18: bootstrap_fresh adds /kernel/agents

**Context**: K1 added `register_agent()` which inserts nodes under `/kernel/agents/`,
but `bootstrap_fresh()` in exo-resource-tree only created 7 namespaces (missing /kernel/agents).
This caused ParentNotFound errors in tree_manager tests.

**Decision**: Add `/kernel/agents` to `bootstrap_fresh()` (now 8 namespaces + root = 9 nodes).

**Rationale**: Agent nodes are first-class kernel resources. The namespace must exist at
bootstrap time, consistent with `/kernel/services` and `/kernel/processes`.

### Decision 19: Unified SHAKE-256 Merkle Hashing (exo-resource-tree)

**Context**: After migrating chain.rs from SHA-256 to SHAKE-256 (Decision 17), the
exo-resource-tree crate still used `sha2::Sha256` for Merkle hash computation. This created
a mixed-hash ecosystem where tree root hashes (SHA-256) flowed into chain event payloads
(SHAKE-256), breaking the single-hash-family invariant.

**Decision**: Replace `sha2` with `rvf-crypto` in exo-resource-tree. `recompute_merkle()`
now uses `rvf_crypto::hash::shake256_256` — the same primitive used by chain.rs and the
RVF witness chain.

**Hash scheme**:
```
node_hash = SHAKE-256(sorted_child_hashes || sorted_metadata_kv_bytes)
```

**Rationale**:
1. **Ecosystem coherence**: One hash function (SHAKE-256) across chain events, Merkle tree,
   and RVF witness chains
2. **Cross-verification**: Tree root hash in a chain event can be independently recomputed
   using the same primitive — no hash-family translation needed
3. **rvf-crypto canonical**: SHAKE-256 is the ecosystem standard per rvf-crypto's public API
4. **No backward compat needed**: K0 chain data is ephemeral (no persistent Merkle hashes
   survived across sessions pre-K1 chain persistence)

### Decision 20: NodeScoring — Trust/Performance Vectors at Base Node Level

**Context**: WeftOS needs comparable trust/performance data across all entities (agents,
services, namespaces, devices) for optimal path selection, concurrent task comparison
("gamification"), and training data generation.

**Decision**: Embed a 6-dimensional `NodeScoring` vector (24 bytes) directly on every
`ResourceNode`: trust, performance, difficulty, reward, reliability, velocity. All
dimensions are f32 in [0.0, 1.0], defaulting to 0.5 (neutral). The scoring bytes flow
into the SHAKE-256 Merkle hash (`child_hashes || scoring_bytes || metadata_kv`), making
scores tamper-evident. `recompute_all()` aggregates children's scoring into parents via
reward-weighted mean. TreeManager provides `update_scoring()`, `blend_scoring()` (EMA),
`find_similar()` (cosine similarity ranking), and `rank_by_score()` (weighted linear).
`MutationEvent::UpdateScoring` records old/new scoring for full audit trail.

**Rationale**:
1. **Universality**: Every node type inherits scoring — no per-type special cases
2. **Tamper evidence**: Scoring flows into Merkle hash, so modifying scores without
   updating the tree is detectable
3. **Aggregation**: Parent nodes automatically reflect aggregate child performance
4. **Compact**: 24 bytes per node — minimal memory and serialization overhead
5. **No new deps**: Uses only std f32 ops and existing serde; no external crates added
6. **Training-ready**: Scores are on-chain via MutationEvent + ChainEvent for ML pipelines

**Deferred to K2**: Supervisor exit-triggered scoring blend, gate decision trust nudges,
`weaver resource score/rank` CLI commands and daemon RPC handlers.

## K2 Decisions (continued)

### Decision 21: IpcScope::Topic for Topic-Only Agents

**Context**: Agents that only communicate via pub/sub topics (not direct PID messaging)
needed a way to restrict IPC scope to specific topics.

**Decision**: Add `IpcScope::Topic(Vec<String>)` variant. `can_message()` returns false
for Topic scope. `can_topic(topic)` checks the allowed topics list. CapabilityChecker
gets `check_ipc_topic()` for topic-scoped enforcement.

**Rationale**: Completes the IpcScope matrix. Agents with Topic scope can subscribe
and publish to named topics but cannot send direct PID-addressed messages, enforcing
a communication pattern suitable for event-driven microservices.

### Decision 22: Scoring Blend on Agent Exit

**Context**: NodeScoring vectors needed a feedback mechanism tied to agent lifecycle.

**Decision**: When an agent exits via `spawn_and_run()` cleanup, compute a NodeScoring
observation based on exit_code (success → high trust/reliability, failure → low) and
call `blend_scoring(rid, &observation, 0.3)`. The 30% blend weight ensures new
observations nudge but don't dominate accumulated history. A `scoring.blend` chain
event is logged.

**Rationale**: Links resource tree trust scores to actual agent performance. Over time,
reliable agents accumulate higher trust scores, enabling trust-weighted task routing.

### Decision 23: IPC RPC Handlers + CLI

**Context**: Topic pub/sub existed in kernel (TopicRouter) but had no CLI or daemon access.

**Decision**: Add `ipc.topics`, `ipc.subscribe`, `ipc.publish` daemon RPC handlers.
Create `weaver ipc topics/subscribe/publish` CLI commands. `ipc.publish` routes through
`send_checked()` for capability enforcement and chain logging.

**Rationale**: Makes IPC first-class in the operator CLI. Topic subscriptions and
publications are chain-logged like all other IPC activity.

### Decision 24: Ed25519 Chain Segment Signing (K2 Extension)

**Context**: Chain events are hash-linked (SHAKE-256) but not cryptographically signed.
An adversary who can modify the chain file could reconstruct valid hash links. rvf-crypto
provides `sign_segment()`/`verify_segment()` with Ed25519.

**Decision**: Generate an Ed25519 signing key at first boot, persist to
`~/.clawft/chain.key`. Sign the checkpoint segment in `save_to_rvf()` using
`sign_segment()` + `encode_signature_footer()`. Verify on `load_from_rvf()` with
`verify_segment()` + `decode_signature_footer()`. This binds the entire chain to the
kernel's identity — the checkpoint records `last_hash` which covers all preceding events.

**Rationale**: Moves from tamper-evident (hash linking) to tamper-proof (Ed25519 signing).
Uses rvf-crypto's existing segment signing API exactly as designed. The checkpoint-level
signing strategy avoids per-event overhead while still covering the entire chain through
the hash link invariant.

### Decision 25: Witness Chain for Kernel Events

**Context**: rvf-crypto provides `WitnessEntry` / `create_witness_chain()` /
`verify_witness_chain()` — a SHAKE-256-linked audit trail parallel to the event chain.
The event chain records *what happened*; the witness chain records *who observed it*.

**Decision**: Each significant chain event (agent spawn, agent exit, cron fire, scoring
blend, ipc.send) creates a `WitnessEntry` with `action_hash = event.hash`. The witness
chain is serialized alongside the event chain as `~/.clawft/chain.witness`. Verification
in `weaver chain verify` includes witness chain integrity check.

**Rationale**: Two independent chains covering the same events: the event chain proves
the sequence of events, the witness chain proves the sequence was observed in order.
Both must agree for full integrity.

### Decision 26: Governance Type Mapping (rvf-types → kernel)

**Context**: rvf-types defines `GovernanceMode`, `TaskOutcome`, `PolicyCheck` for
agent governance. Our kernel has analogous concepts in `IpcScope`, `ProcessState`,
and `GateBackend`.

**Decision**: Map governance types into agent capabilities:
- `GovernanceMode::Restricted` → IpcScope::None + read-only tools
- `GovernanceMode::Approved` → IpcScope::Restricted + gated tools
- `GovernanceMode::Autonomous` → IpcScope::All + full tools
- `TaskOutcome` classifies agent exit: exit_code 0 → Solved, non-zero → Failed
- `PolicyCheck` used in gate decisions alongside existing GateBackend

**Rationale**: Aligns WeftOS agent governance with the ruvector ecosystem's
standardized governance model. Enables future cross-system interoperability
where governance policies are portable between ruvector and WeftOS nodes.

### Decision 27: WitnessBuilder for Agent Task Completion

**Context**: rvf-runtime provides `WitnessBuilder` for constructing signed witness
bundles that capture task execution evidence (spec, plan, tool trace, diff, test log).

**Decision**: When an agent exits, create a `WitnessBuilder` with:
- task_id derived from agent UUID
- GovernancePolicy based on agent's IpcScope/capabilities
- TaskOutcome from exit code classification
Build the witness bundle and store as chain event (kind=`witness.bundle`).
Use `ScorecardBuilder` to aggregate witness bundles into per-agent capability
scorecards stored on tree nodes.

**Rationale**: Provides deterministic replay evidence for every agent task execution.
Integrates rvf-runtime's witness system with our chain and tree, enabling
100% playback of agent activity.

### Decision 28: Lineage Tracking for Resource Derivation

**Context**: rvf-crypto provides `LineageRecord` / `FileIdentity` / `DerivationType`
for tracking parent-child derivation chains.

**Decision**: When an agent spawns, create a `LineageRecord`:
- parent = kernel FileIdentity
- child = agent FileIdentity
- derivation_type = Projection (agent is a projection of kernel)
Similarly for cron jobs. Lineage records are serialized via
`lineage_record_to_bytes()` and linked to the witness chain via
`lineage_witness_entry()`. `verify_lineage_chain()` validates during
chain verification.

**Rationale**: Tracks resource provenance — which kernel boot created which
agent, which agent created which cron job. Combined with the event chain
and witness chain, this provides a complete audit trail of *what* happened,
*who* observed it, and *where it came from*.

## K2b Gap Analysis and Decisions (Kernel Work-Loop Hardening)

### Gap Analysis Summary

Systematic review of all kernel background loops (agent_loop, cron tick, RPC accept,
supervisor lifecycle, A2ARouter) identified 6 gaps where infrastructure existed but
was either unused, undriven, or incomplete. These represent silent failures in the
kernel substrate — the kind of bugs that don't cause test failures but erode
production reliability.

**Method**: Read every kernel module (agent_loop.rs, supervisor.rs, cron.rs, a2a.rs,
health.rs, process.rs, gate.rs, boot.rs) and daemon.rs. For each background loop,
asked: "What drives this? What detects failure? What records activity?"

**Findings**:

| # | Gap | Severity | Root Cause |
|---|-----|----------|-----------|
| 1 | Health monitor loop never fires | Critical | HealthSystem.aggregate() exists but no timer calls it |
| 2 | Agent task panics undetected | Critical | JoinHandle tracked but is_finished() never polled |
| 3 | Shutdown aborts cleanup handlers | Critical | abort_all() cancels futures before scoring/tree/chain code runs |
| 4 | Resource counters always zero | Critical | ResourceUsage struct populated with defaults, never incremented |
| 5 | ProcessState::Suspended unused | Important | State defined but no mechanism to enter/exit it |
| 6 | Agent commands bypass capabilities | Important | GateBackend exists but agent_loop doesn't call it |

### Decision 29: Health Monitor Background Loop

**Context**: `HealthSystem` (health.rs) has `aggregate()` that queries all registered
services and produces `OverallHealth`. The `health_check_interval_secs` config field is
set at boot and stored on the HealthSystem struct. Daemon has a cron tick loop (1s) but
no health tick loop.

**Decision**: Add a health monitor background tokio task in daemon.rs alongside the
cron tick loop. Uses `health_check_interval_secs` as the interval (default 30s). Logs
a `health.check` chain event each cycle. Emits kernel event log warnings for
degraded/unhealthy services. Exits cleanly on shutdown signal.

**Rationale**: Health monitoring is the most fundamental kernel loop. Without it,
service failures (cron stopped, cluster disconnected) go undetected. The infrastructure
is already built — this is pure wiring.

### Decision 30: Agent Watchdog Sweep

**Context**: `AgentSupervisor` stores `JoinHandle<()>` per agent in a DashMap.
The `spawn_and_run` task completion handler runs cleanup (scoring, tree, chain) and
removes the DashMap entry. But if the task panics before cleanup, or the handle is
joined after abort, the PID stays "Running" in the process table.

**Decision**: Add `watchdog_sweep()` to `AgentSupervisor`. Iterates `running_agents`,
calls `is_finished()` on each handle. For finished handles: `join()` to get the
JoinError, classify as panic (-3) or cancelled (-4), transition PID to `Exited`,
log `agent.watchdog_reap` chain event. Run from a 5-second daemon background task.

**Rationale**: Prevents phantom processes. Every agent that exits — cleanly, by panic,
or by cancellation — is detected within 5 seconds and properly recorded. Essential for
K3+ phases where agents run user-supplied WASM code that may panic.

### Decision 31: Graceful Shutdown Replaces Abort

**Context**: `supervisor.abort_all()` calls `handle.abort()` which cancels tokio
futures mid-execution. The cleanup handler in `spawn_and_run` (scoring blend, tree
unregister, chain exit event) never runs for aborted tasks. On daemon Ctrl+C, all
running agents lose their exit audit trail.

**Decision**: Add `shutdown_all(timeout: Duration) -> Vec<(Pid, i32)>`. Cancels
all agent cancellation tokens, then waits for all tasks to complete within `timeout`.
Tasks that don't exit in time are aborted as fallback. Daemon shutdown calls
`shutdown_all(5s)` instead of `abort_all()`. Keep `abort_all()` for emergency/force
shutdown.

**Rationale**: Cancellation tokens trigger the `cancel.cancelled()` branch in
`kernel_agent_loop`, which returns exit code 0 — allowing the `spawn_and_run` cleanup
handler to run normally (scoring, tree, chain). This preserves the audit trail for
every agent shutdown.

### Decision 32: Resource Usage Instrumentation

**Context**: `ResourceUsage` (process.rs) has 4 counters: `memory_bytes`,
`cpu_time_ms`, `tool_calls`, `messages_sent`. `ProcessTable::update_resources()` exists
but is never called. All counters are permanently zero.

**Decision**: Add resource tracking to `kernel_agent_loop`:
- Track `messages_sent` (incremented on each reply)
- Track `tool_calls` (incremented on `exec` command)
- Track `cpu_time_ms` (wall-clock time since loop started)
- Call `process_table.update_resources()` every 10 messages and on exit
- Pass `Arc<ProcessTable>` into the agent loop as a new parameter

**Rationale**: Resource counters are the foundation for limit enforcement (K3),
anomaly detection (K5), and training data generation (K6). Without instrumentation,
all downstream consumers get zeros. The per-10-message batching avoids per-message
DashMap write contention.

### Decision 33: Agent Suspend/Resume Protocol

**Context**: `ProcessState::Suspended` exists with valid transitions
`Running -> Suspended -> Running`. `ProcessSignal::Suspend` and `Resume` exist.
But nothing triggers or handles suspension.

**Decision**: Add `suspend` and `resume` as built-in agent commands. When the agent
loop receives `{"cmd": "suspend"}`, it transitions the process to `Suspended` and
enters a parking loop that only processes `resume` or cancellation. All other
messages receive an `{"error": "agent suspended"}` response. On `resume`, transitions
back to `Running` and returns to normal processing.

**Rationale**: Suspend/resume enables: debugging (pause agent, inspect state, resume),
resource conservation (park idle agents), migration (suspend on source, restore on
target in K6). Making it a command means it's accessible from both IPC and CLI.

### Decision 34: Gate Enforcement in Agent Loop

**Context**: `GateBackend` trait has two implementations: `CapabilityGate` (binary
Permit/Deny from CapabilityChecker) and `TileZeroGate` (three-way with crypto
receipts). The agent loop executes all commands without consulting either.

**Decision**: Pass `Arc<dyn GateBackend>` into `kernel_agent_loop`. Before executing
`exec`, `cron.add`, or `cron.remove`, call `gate.check(agent_id, action, context)`.
Map action strings: `exec` -> `"tool.exec"`, `cron.add` -> `"service.cron.add"`,
`cron.remove` -> `"service.cron.remove"`. On `Deny`, return error response with
reason. On `Defer`, return `{"deferred": true, "reason": ...}`.

**Rationale**: Closes the enforcement gap between declared capabilities and actual
execution. An agent with `can_exec_tools: false` will now actually be blocked from
running `exec`. When TileZero is enabled, gate decisions are cryptographically
logged to the chain, providing a full audit trail of permission checks.
