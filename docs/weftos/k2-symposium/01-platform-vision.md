# WeftOS: Platform Vision

**Presenter**: Platform Architecture Panel

---

## What Is WeftOS?

WeftOS is an agent operating system kernel -- a Rust-native runtime that
manages AI agents the way a traditional OS manages processes. It provides:

- **Process management**: PID allocation, state machines, supervision trees
- **Inter-process communication**: Typed message envelopes, pub/sub topics, capability-checked routing
- **Service registry**: Named services with health checks and lifecycle management
- **Cryptographic audit**: Append-only hash-linked event chains with Ed25519 signing
- **Constitutional governance**: Three-branch model with 5D effect algebra for every agent action
- **Resource namespace**: Merkle-hashed hierarchical tree with atomic mutations

## The Three-Branch Constitutional Model

WeftOS implements constitutional governance inspired by separation of powers:

```
Legislative Branch    Rules, SOPs, manifests define boundaries
Executive Branch      Agents act within those boundaries
Judicial Branch       Governance engine validates every action
```

Every agent action passes through the GovernanceGate, which evaluates a
5-dimensional effect vector (risk, fairness, privacy, novelty, security)
against configurable thresholds. Decisions are logged to the ExoChain
for full auditability.

## Architecture at a Glance

```
weaver (CLI + daemon)
  |
  +-- Kernel
  |     +-- ProcessTable (DashMap, lock-free)
  |     +-- AgentSupervisor (spawn/stop/restart)
  |     +-- ServiceRegistry (named lifecycle)
  |     +-- KernelIpc + A2ARouter (per-agent inboxes)
  |     +-- HealthSystem (aggregated checks)
  |     +-- CronService (interval scheduling)
  |     +-- ClusterMembership (peer tracking)
  |     |
  |     +-- [exochain]
  |     |     +-- ChainManager (SHAKE-256 hash chain)
  |     |     +-- TreeManager (Merkle resource tree)
  |     |     +-- GovernanceGate (5D effect algebra)
  |     |
  |     +-- [K3+] WasmToolRunner, ContainerManager,
  |           AppManager (types defined, runtimes pending)
  |
  +-- Unix Socket (JSON-RPC)
```

## What Has Been Built (K0-K2)

### K0: Foundation (Complete)
- `Kernel<P>` generic over platform trait (native/WASM/browser)
- Boot sequence with 7 phases (Init through Ready)
- Boot event log with ring buffer (1024 events)
- KernelConfig with environment-specific overrides

### K1: Process & Supervision (Complete)
- ProcessTable with DashMap for lock-free concurrent access
- ProcessState machine: Starting -> Running -> Suspended -> Exited
- AgentSupervisor with spawn_and_run, graceful stop, force stop, restart
- Watchdog sweep for stale processes
- AgentCapabilities with IpcScope, SandboxPolicy, ResourceLimits

### K2: IPC & Communication (Complete)
- KernelIpc with typed MessagePayload (Text, Json, ToolCall, ToolResult, Signal, Rvf)
- A2ARouter with per-agent inboxes (1024 capacity), capability-checked routing
- TopicRouter for pub/sub with subscriber filtering
- CronService for interval-based job scheduling
- kernel_agent_loop() with gate checks, suspend/resume, chain logging

### ExoChain Subsystem (Complete)
- ChainManager: append-only log with SHAKE-256 triple-hash scheme
- RVF persistence: segment-based binary format with Ed25519 signatures
- Witness chains and lineage tracking (DNA-style provenance)
- TreeManager: unified facade over ResourceTree + MutationLog + ChainManager
- GovernanceGate: rule-based, chain-logged, 4-way decisions
- TileZeroGate: CGR with cryptographic receipts (feature-gated)

## Where We're Headed (K3-K6)

### K3: WASM Sandbox
Isolated tool execution using Wasmtime with fuel metering, memory limits,
and host function binding. Tools declared in app manifests run sandboxed
with governance oversight and chain-logged results.

### K4: Container Management
Docker/Podman sidecars for services that need full OS environments.
Health checks integrated with HealthSystem, lifecycle events chain-logged,
services registered in ServiceRegistry.

### K5: Application Framework
`weftapp.toml` manifests declaring agents, tools, services, and capabilities.
AppManager orchestrates the full lifecycle: install, validate, start
(spawn agents + load tools + start services), stop, remove.

### K6: Cluster & Networking
Multi-node operation with Raft consensus (via ruvector-raft), cross-node
IPC routing, chain replication, and distributed resource tree
synchronization.

## The Vision

WeftOS enables a world where AI agents operate as first-class processes
under constitutional governance. Every action is auditable, every
capability is checked, every decision is logged to a tamper-evident chain.

Applications compose agents, tools, and services through typed manifests.
The governance engine ensures agents operate within defined boundaries,
with the ability to escalate to human review for high-risk decisions.

The kernel provides the trust infrastructure. The chain provides the proof.
The gate provides the guardrails. The tree provides the namespace.
Together they form the foundation for cryptographically contracted,
governable, auditable agent systems.

## How to Use It

```bash
# Build the kernel and CLI
scripts/build.sh native

# Start the daemon
weaver kernel start

# Check status
weaver kernel status

# Spawn an agent
weaver agent spawn -t worker --name my-agent

# Send a message
weaver agent send 1 '{"cmd":"ping"}'

# View the audit trail
weaver chain local -n 10

# Verify chain integrity
weaver chain verify

# View the resource tree
weaver resource tree

# Stop everything
weaver kernel stop
```

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `clawft-kernel` | Kernel modules (process, IPC, chain, gate, etc.) |
| `clawft-weave` | CLI binary + daemon + JSON-RPC |
| `clawft-core` | Agent loop, pipeline, message bus |
| `clawft-types` | Shared type definitions |
| `clawft-platform` | Platform abstraction traits |
| `clawft-llm` | LLM provider integration |
| `clawft-plugin` | Plugin system |
| `clawft-wasm` | WASM entry point |
| `exo-resource-tree` | Merkle-hashed resource namespace |

## Test Coverage

- **298 tests** passing across the kernel crate
- All major modules have 10-40 tests each
- Concurrent access patterns tested
- Error conditions and edge cases covered
- Integration tests for chain+tree, gate+loop combinations
