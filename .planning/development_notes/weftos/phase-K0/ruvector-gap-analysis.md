# K0 Ruvector Ecosystem Gap Analysis

**Date**: 2026-03-01
**Scope**: All 17 ruvector topic areas vs K0 kernel implementation
**Conclusion**: K0 is correctly scoped. No missing ruvector integration blocks K0→K1.

---

## Integration Status Summary

| # | Topic | Ruvector Crate | K0 Status | Target Phase |
|---|-------|---------------|-----------|-------------|
| 1 | Service Discovery | `ruvector-cluster` | **Integrated** | K0 (done) |
| 2 | Audit Trail | `cognitum-gate-tilezero` (receipts) | Custom impl | K0 chain (done), tilezero receipts K1 |
| 3 | Health Monitoring | `ruvector-cluster`, `ruvector-coherence` | Custom impl | K0 basic (done), coherence K1+ |
| 4 | Kernel Boot | `rvf-kernel` / `KernelBuilder` | Custom impl | N/A (different concern) |
| 5 | Wire Format | `rvf-wire` | Hand-rolled JSON/Unix socket | K1 (improvement, not blocker) |
| 6 | Capability Bitmaps | `rvf-runtime` / `MembershipFilter` | Custom `AgentCapabilities` | K1 |
| 7 | RBAC / Permissions | `cognitum-gate-tilezero` | Custom `CapabilityChecker` | K1 |
| 8 | Agent Routing | `ruvector-tiny-dancer-core` | None | K1 |
| 9 | Coherence Scoring | `prime-radiant` | None | K1+ |
| 10 | Consensus | `ruvector-raft`, `ruvector-delta-consensus` | Deps added, not wired | K2/K6 |
| 11 | IPC / Messaging | `ruvector-delta-consensus`, `ruvector-nervous-system` | Custom `KernelIpc` + `MessageBus` | K2 |
| 12 | WASM Sandbox | `ruvector-cognitive-container`, `rvf` (WASM_SEG) | Stub `WasmToolRunner` | K3 |
| 13 | Container / Boot | `ruvector-cognitive-container` | Stub `ContainerManager` | K4 |
| 14 | Self-Learning | `sona` | None | K5 |
| 15 | eBPF Fast-Path | `rvf-ebpf` | None | Post-K6 |
| 16 | P2P Networking | QuDAG | None | K6 |
| 17 | Token Economy | `ruvector-economy-wasm` | None | Post-K6 |

---

## Detailed Analysis

### Already Done in K0

#### Service Discovery — `ruvector-cluster`
- `ClusterService` in `cluster.rs` wraps `ruvector_cluster::ClusterManager`
- Behind `#[cfg(feature = "cluster")]`, native-only
- Syncs discovered native nodes into kernel's `ClusterMembership`
- CLI: `weaver cluster {status|nodes|join|leave|health|shards}`

#### Audit Trail — Custom ChainManager
- K0 implements SHA-256 hash-linked chain (`chain.rs`) with `ChainManager`
- `TreeManager` facade unifies chain + tree + mutation log
- Boot-to-chain audit trail: all boot phases emit chain events
- `verify_integrity()` walks all events, recomputes hashes
- tilezero's `WitnessReceipt` (signed tokens, hash-chained receipts) is richer — K1 target

#### Health Monitoring — Custom HealthSystem
- `HealthSystem` in `health.rs` with `HealthStatus` enum (Healthy/Degraded/Unhealthy)
- Per-service health via `SystemService::health_check()`
- `ruvector-coherence` adds topological coherence scoring (Betti numbers, sheaf Laplacian) — K1+ agent health

#### Kernel Boot — Custom boot.rs
- `rvf-kernel` builds unikernels for QEMU/Firecracker. Different concern entirely.
- WeftOS kernel is an application-level OS abstraction, not a hypervisor image.

### K1 Priorities (Top 3 Ruvector Integrations)

#### 1. cognitum-gate-tilezero — Permission Backend
- Crate exists at `ruvector/crates/cognitum-gate-tilezero/`
- 256-tile coherence arbiter with three-filter decision: Permit/Defer/Deny
- `PermitToken` (signed capability tokens), `WitnessReceipt` (hash-chained audit)
- Maps to `tree.check()` permission backend and `GovernanceDecision` in governance.rs
- **Action**: Add workspace dep, wire as permission evaluator for resource tree

#### 2. rvf-wire — Daemon RPC Framing
- Current: line-delimited JSON over Unix socket (`protocol.rs`, `client.rs`)
- rvf-wire provides: 64-byte SIMD-aligned segments, CRC32C/XXH3/SHAKE-256 checksums, compression, forward-compat segment skipping
- Same `Request`/`Response` types, just framed in RVF segments instead of JSON lines
- **Action**: Replace `protocol.rs` wire format with rvf-wire segment framing

#### 3. ruvector-tiny-dancer-core — Agent Dispatch Routing
- FastGRNN sub-millisecond routing: which agent handles this request?
- `Router`, `Candidate`, `CircuitBreaker`, learned routing via AgentDB
- Maps to K1 supervisor dispatch (currently no routing, just spawn/stop)
- **Action**: Add workspace dep, integrate into `AgentSupervisor` dispatch path

### K2 Targets

#### ruvector-delta-consensus — IPC Message Ordering
- CRDT-based delta sync with vector clocks and causal ordering
- `DeltaConsensus`, `CausalDelta`, `VectorClock`, `ConflictStrategy`
- Maps to K2 IPC when agents need causal message ordering

#### ruvector-nervous-system — High-Throughput EventBus
- `ShardedEventBus` for high-throughput pub/sub
- Bio-inspired routing with `OscillatoryRouter`, `BudgetGuardrail`
- Replace `clawft-core::MessageBus` for K2 IPC layer

### K3+ Targets

- **K3**: `ruvector-cognitive-container` for WASM epoch budgeting + witness chains
- **K4**: Container lifecycle (cognitive-container's `EpochController`)
- **K5**: `sona` for agent self-learning (LoRA + EWC++ + ReasoningBank)
- **K6**: `ruvector-raft` for global root chain consensus, QuDAG for P2P
- **Post-K6**: `rvf-ebpf` for kernel-bypass networking, `ruvector-economy-wasm` for token economics

### Not Applicable to WeftOS Kernel

- `rvf-kernel` / `KernelBuilder` — unikernel boot for VMs, not our architecture
- `rvf-runtime` / `MembershipFilter` — pulls in full vector DB stack; too heavy for capability checks alone. Keep custom `AgentCapabilities`.

---

## Workspace Dependency Status

| Crate | In workspace Cargo.toml | In clawft-kernel Cargo.toml | Actually imported |
|-------|------------------------|---------------------------|-------------------|
| `ruvector-cluster` | Yes (path dep) | Yes (optional, `cluster`) | Yes |
| `ruvector-raft` | Yes (path dep) | Yes (optional, `cluster`) | No (dep only) |
| `ruvector-replication` | Yes (path dep) | Yes (optional, `cluster`) | No (dep only) |
| `rvf-runtime` | Yes (crates.io v0.2) | No | No |
| `rvf-types` | Yes (crates.io v0.2) | No | No |
| `cognitum-gate-tilezero` | No | No | No |
| `ruvector-nervous-system` | No | No | No |
| `ruvector-tiny-dancer-core` | No | No | No |
| `prime-radiant` | No | No | No |
| `sona` | No | No | No |

---

## Key Architectural Notes

1. **Custom chain vs exochain DAG**: K0 uses SHA-256 `ChainManager` with custom
   `ChainEvent` structs, not exochain's BLAKE3 + HLC + DAG. This is intentional
   for K0 simplicity. Migration path to `exo-dag::DagStore` exists for K6 when
   cross-node consensus requires it.

2. **cognitum-gate-tilezero exists**: Located at `ruvector/crates/cognitum-gate-tilezero/`
   with full implementation (decision.rs, evidence.rs, permit.rs, receipt.rs,
   supergraph.rs). Not published to crates.io — needs path dep like ruvector-cluster.

3. **rvf-wire for daemon RPC**: The current JSON-over-Unix-socket protocol is
   functional but doesn't benefit from checksums, compression, or forward-compat.
   Switching to rvf-wire is a quality improvement, not a functional gap.
