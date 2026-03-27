# K5 Symposium Results

**Date**: 2026-03-25
**Status**: COMPLETE -- 15 decisions rendered, 5 commitments made
**Branch**: feature/weftos-kernel-sprint
**Predecessor**: [ECC Symposium Results](../ecc-symposium/05-symposium-results-report.md)

---

## 1. Executive Summary

The K5 Symposium synthesized three research efforts -- a K6 readiness audit,
mesh networking technology survey, and ruvector crate inventory -- into a
cohesive architecture for WeftOS's transport-agnostic encrypted mesh network.

**The kernel is 75% ready for K6.** 41 items are wire-ready (GREEN), 22 need
minor changes (YELLOW), and 21 are missing (RED). The 6 critical gaps -- no
transport layer, no cluster-join auth, no encryption, no RemoteNode target,
no chain merge, no tree sync -- are all addressable in ~1,500 new lines +
~370 changed lines across 6 sub-phases.

### K6 Readiness Scorecard

| Dimension | Score | Evidence |
|-----------|-------|----------|
| Cluster readiness | 9/10 | Full peer lifecycle, DashMap concurrency, platform enum |
| IPC wire-readiness | 8/10 | KernelMessage serde, all payload variants, correlation IDs |
| Chain crypto | 9/10 | SHAKE-256 + Ed25519 + ML-DSA-65, integrity verification |
| Governance determinism | 8/10 | Pure evaluation, GateBackend extensibility |
| Security posture | 7/10 | Capability checks, resource limits, dual-layer gate |
| Tree Merkle | 7/10 | Root hash, atomic mutations, peer namespace |
| Ruvector reuse | 8/10 | Algorithms without I/O -- perfect composition boundary |
| **Overall** | **8.0/10** | **Ready for K6 sprint** |

---

## 2. Key Decisions

### D1: Selective Composition over Full Framework

**Decision**: Use snow (Noise Protocol) + quinn (QUIC) + selective libp2p
(kad, mdns) rather than the full libp2p stack or iroh.

**Rationale**: Full libp2p brings ~80 transitive dependencies and an
opinionated runtime model. iroh is QUIC-only and not transport-agnostic.
Selective composition gives the same capabilities with full control over
integration with the existing tokio runtime.

**Origin**: Mesh Architecture Panel

### D2: Ed25519 Public Key as Node Identity

**Decision**: Replace UUID-based NodeId with Ed25519 public key. The node ID
is `hex(SHAKE-256(pubkey)[0..16])`.

**Rationale**: Every node can authenticate itself by signing a challenge. No
certificate authority or shared secret distribution needed. Consistent with
the Ed25519 signing already in `chain.rs`.

**Origin**: Security Panel

### D3: Noise Protocol for All Inter-Node Encryption

**Decision**: All inter-node traffic uses Noise Protocol (via snow). XX
pattern for first contact, IK pattern for known peers.

**Rationale**: Transport-agnostic (works over TCP, QUIC, WebSocket, anything
with a byte stream). Forward secrecy. Mutual authentication. No X.509
certificates, no PKI, no CRL. Proven in WireGuard and libp2p.

**Origin**: Security Panel

### D4: Governance.genesis as Cluster Trust Root

**Decision**: The `governance.genesis` chain event is the cluster-wide trust
root. Its SHAKE-256 hash serves as the cluster ID. All joining nodes must
present a matching `genesis_hash`.

**Rationale**: The governance.genesis event already exists and contains the
founding authority. Using it as the trust root requires no new infrastructure.

**Origin**: Readiness Audit Panel

### D5: Feature-Gated Mesh Networking

**Decision**: All mesh networking code lives behind `mesh` / `mesh-discovery`
/ `mesh-full` feature gates. The default build has zero networking code.

**Rationale**: Preserves single-node zero-dependency builds for development,
testing, embedded, and WASI targets. Feature gates are the existing pattern
in the kernel (see `ecc`, `exochain`, `cluster`).

**Origin**: All Panels

### D6: QUIC as Primary Transport, WebSocket as Browser Fallback

**Decision**: Cloud and Edge nodes use QUIC (quinn). Browser and WASI nodes
use WebSocket. WebRTC, BLE, and LoRa are future transport implementations.

**Rationale**: QUIC provides multiplexing, congestion control, and connection
migration. Browsers cannot use QUIC directly but support WebSocket natively.

**Origin**: Mesh Architecture Panel

### D7: Ruvector Algorithms as Pure Computation

**Decision**: Reuse ruvector-cluster, ruvector-raft, ruvector-replication,
and ruvector-delta-consensus as computation-only components. The mesh layer
provides all I/O. No changes to ruvector crates needed.

**Rationale**: All ruvector crates produce messages to send and consume
messages received. They have complete algorithms but no network I/O. The
mesh layer is the I/O bridge -- a clean composition boundary.

**Origin**: Ruvector Inventory Panel

### D8: rvf-wire as Mesh Wire Format

**Decision**: Use `rvf-wire` zero-copy segment serialization as the mesh
wire format for binary payloads. JSON framing remains available for
`KernelMessage` transport.

**Rationale**: rvf-wire is already in the workspace, already used for chain
persistence, and provides zero-copy deserialization. Avoids introducing a
new serialization dependency (protobuf, flatbuffers, etc.).

**Origin**: Mesh Architecture Panel

### D9: Dual Signing (Ed25519 + ML-DSA-65) for Cross-Node Chain Events

**Decision**: Chain events that cross node boundaries must carry both
Ed25519 and ML-DSA-65 signatures. Verification requires both to pass.

**Rationale**: ML-DSA-65 provides quantum resistance today. Ed25519 provides
backward compatibility and fast verification. Dual signing is already
implemented in `chain.rs` -- K6 just enforces it for cross-node events.

**Origin**: Security Panel

### D10: 6-Phase Implementation (K6.0-K6.5)

**Decision**: Implement K6 in 6 sub-phases, each independently testable:
K6.0 (prep), K6.1 (transport + encryption), K6.2 (discovery), K6.3 (IPC),
K6.4 (chain + tree sync), K6.5 (distributed services).

**Rationale**: Each phase delivers testable value. The prep phase (K6.0) can
be merged without any new dependencies. Later phases can be split across
multiple developers or sprints.

**Origin**: All Panels

---

## 3. Architecture Commitments

### C1: `MessageTarget::RemoteNode` Variant

Add `RemoteNode { node_id: String, target: Box<MessageTarget> }` to the
`MessageTarget` enum in `crates/clawft-kernel/src/ipc.rs`.

**Phase**: K6.0
**Impact**: All `match` arms on `MessageTarget` must handle the new variant.
Until K6.1, the new arm returns `Err(IpcError::RemoteNotAvailable)`.

### C2: `GlobalPid` Composite Identifier

Add `GlobalPid { node_id: String, pid: Pid }` to `crates/clawft-kernel/src/ipc.rs`.
All cross-node message routing uses `GlobalPid` instead of bare `Pid`.

**Phase**: K6.0
**Impact**: Local code continues to use `Pid`. `GlobalPid` is used only at
mesh boundaries.

### C3: `MeshTransport` Trait

Define a transport-agnostic `MeshTransport` trait with `listen()`, `connect()`,
and `supports()` methods. QUIC and WebSocket are the initial implementations.

**Phase**: K6.1
**Impact**: New code only. No changes to existing modules.

### C4: `mesh` Feature Gate

All mesh networking code lives behind `#[cfg(feature = "mesh")]`. The feature
gate structure:

```
mesh          = quinn + snow + x25519-dalek
mesh-discovery = mesh + libp2p-kad + libp2p-mdns
mesh-full     = mesh + mesh-discovery
```

**Phase**: K6.0 (gate definition), K6.1+ (code behind gate)
**Impact**: `Cargo.toml` only. Default builds unchanged.

### C5: Cluster-Join Authentication Protocol

All cluster joins require a signed `JoinRequest` verified against
`governance.genesis`. The GovernanceGate evaluates join requests using
the `cluster.join` action.

**Phase**: K6.0 (JoinRequest struct), K6.1 (verification in mesh handshake)
**Impact**: `add_peer()` gains a gating check. Existing tests that call
`add_peer()` directly are unaffected (they bypass the mesh layer).

---

## 4. Open Questions for K6

| # | Question | Impact | When to Resolve |
|---|----------|--------|----------------|
| Q1 | Should chain merge use leader-based consensus or DAG? | Architecture of K6.4 | Before K6.4 starts |
| Q2 | Should the mesh wire format be JSON or RVF for KernelMessage? | Performance vs debuggability | K6.1 design phase |
| Q3 | How do Browser nodes persist identity across sessions? | UX and security tradeoff | K6.1 browser transport |
| Q4 | Should discovery use full libp2p-kad or a lighter custom DHT? | Dependency weight | K6.2 design phase |
| Q5 | How to handle split-brain when cluster partitions? | Consistency vs availability | Before K6.4 |
| Q6 | Should K6 use BLAKE3 (per D6 from ECC symposium) or stay with SHAKE-256? | Hash migration | K6.0 design phase |
| Q7 | What is the maximum practical cluster size for WeftOS? | Config defaults, test scenarios | K6.2 testing |
| Q8 | Should tree sync use full snapshot or Merkle proof exchange? | Bandwidth vs complexity | K6.4 design phase |

---

## 5. Dependencies to Add

### Cargo.toml Changes

```toml
[dependencies]
# K6 mesh transport (behind feature gate)
quinn = { version = "0.11", optional = true }
snow = { version = "0.9", optional = true }
x25519-dalek = { version = "2.0", optional = true, features = ["static_secrets"] }

# K6 discovery (behind feature gate, optional)
libp2p-kad = { version = "0.46", optional = true }
libp2p-mdns = { version = "0.46", optional = true }

[features]
mesh = ["quinn", "snow", "x25519-dalek"]
mesh-discovery = ["mesh", "libp2p-kad", "libp2p-mdns"]
mesh-full = ["mesh", "mesh-discovery"]
```

### Already Available (No New Deps)

| Crate | K6 Use |
|-------|--------|
| ed25519-dalek | Node identity signing |
| rvf-wire | Wire format |
| tokio | Async runtime |
| dashmap | Concurrent membership map |
| serde, serde_json | Message serialization |

---

## 6. Prep Work Before K6 Sprint

These changes can be made immediately, before K6 formally starts, with zero
new dependencies:

| # | Change | File | Lines | Risk |
|---|--------|------|:-----:|------|
| P1 | Add `RemoteNode` to `MessageTarget` | `ipc.rs` | ~10 | Low |
| P2 | Add `GlobalPid` struct | `ipc.rs` | ~20 | Low |
| P3 | Add `bind_address`, `seed_peers` to `ClusterConfig` | `cluster.rs` | ~10 | Low |
| P4 | Add `NodeIdentity` struct | `cluster.rs` or `identity.rs` | ~40 | Low |
| P5 | Add `tail_from()` to `LocalChain` | `chain.rs` | ~10 | Low |
| P6 | Add `mesh` feature gate to `Cargo.toml` | `Cargo.toml` | ~5 | None |
| P7 | Sign `MutationEvent.signature` with node key | `tree_manager.rs` | ~15 | Low |
| **Total** | | | **~110** | **Low** |

All prep changes maintain backward compatibility. Existing tests pass
without modification.

---

## 7. Relationship to Previous Symposiums

### From K2 Symposium

| K2 Commitment | K5/K6 Relevance |
|---------------|----------------|
| C10: K6 SPARC spec | This symposium defines K6 architecture |

### From K3 Symposium

| K3 Item | K5/K6 Relevance |
|---------|----------------|
| 25 unimplemented tools | K6 enables remote tool execution across nodes |
| ServiceApi trait (C2) | K6.5 service discovery builds on this |

### From ECC Symposium

| ECC Decision | K5/K6 Relevance |
|-------------|----------------|
| D6: BLAKE3 forward | K6 Q6 asks whether to migrate hashing now |
| D1: Nervous system model | K6 mesh IS the nervous system transport |
| D3: Self-calibrating tick | K6 mesh carries tick coordination |
| D4: CRDTs for convergence | K6.5 uses ruvector-delta-consensus CRDTs |

---

## 8. Panel Verdicts

| Panel | Verdict | Confidence |
|-------|---------|-----------|
| K6 Readiness Audit | READY with 6 critical gaps identified | High |
| Mesh Architecture | snow + quinn + selective libp2p APPROVED | High |
| Ruvector Inventory | Algorithms reusable, no I/O -- clean boundary | High |
| Security Model | Noise + Ed25519 + GovernanceGate APPROVED | High |
| Implementation Plan | 6-phase plan APPROVED, ~1,870 total lines | Medium-High |

### D11: Hybrid Noise + ML-KEM-768 Post-Quantum Key Exchange

**Decision**: Add a hybrid post-quantum key exchange to K6.4b. After the
Noise XX handshake establishes a classical X25519 channel, perform an
ML-KEM-768 key encapsulation upgrade inside the encrypted channel. The final
session key combines both secrets via HKDF.

**Rationale**: Protects mesh transport against store-now-decrypt-later quantum
attacks. Leverages existing `ruvector-dag` ML-KEM-768 implementation (behind
`production-crypto` feature). Negotiated via `kem_supported: bool` in the
handshake payload -- graceful degradation when unsupported. Cost: ~2.4KB extra
per handshake, ~1ms latency, zero per-message overhead.

**Origin**: Security Panel (moved from K7+ to K6.4b)

---

### D12: Layered Service Resolution with Genesis-Hash DHT Keys

**Decision**: Service resolution uses a 9-step layered flow with genesis-hash-namespaced
DHT keys, resolution cache (30s TTL), negative cache, connection pool, and
circuit breaker. Replicated services use round-robin in K6.3, graduating to
lowest-latency and load-aware selection. DHT keys prefixed with governance
genesis hash (first 16 hex chars) for cross-cluster isolation (defense-in-depth).

**Rationale**: The join protocol already gates on governance genesis, but DHT key
namespacing prevents a node that accidentally connects to a different cluster's DHT
from resolving or polluting service records. Layered caching (resolution + negative +
connection pool) prevents DHT storms and redundant network calls. Circuit breaker
prevents cascade failures from slow or erroring remote nodes. Selection strategy
evolves from simple round-robin (K6.3) through affinity routing (K6.5) to load-aware
selection using `NodeEccCapability.headroom_ratio` from ECC gossip (K7).

**Origin**: Mesh Architecture Panel (M9), K6 SPARC refinement

**Cross-references**: M9 in `01-mesh-architecture.md`, K6 phase breakdown in
`07-phase-K6-mesh-framework.md`

### D13: Mesh RPCs Reuse ServiceApi Adapter Pattern

**Decision**: Mesh RPCs reuse the existing `ServiceApi` adapter pattern. No dedicated
mesh protocol types. The `MeshAdapter` feeds incoming mesh messages into the
local `A2ARouter`, which dispatches through `ServiceApi` -- making every kernel
service (registry, chain, ecc, kernel) automatically remotely callable. Only
~160 lines of new code: `RegistryQueryService` (50), `MeshAdapter` (80),
`mesh.request()` (30). Reuses K2's correlation-based request-response.

**Rationale**: ServiceApi is already the universal dispatch interface used by
Shell, MCP, and daemon RPC adapters. Adding a mesh adapter to this pattern
eliminates the need for a dedicated mesh RPC protocol and ensures governance
gates, chain witnessing, and all existing middleware apply uniformly to remote
calls.

**Origin**: Mesh Architecture Panel (K6.3 refinement)

### D14: CMVG Cognitive Sync via Multiplexed QUIC Streams

**Decision**: CMVG cognitive substrate syncs via multiplexed QUIC streams over
the same mesh connection as IPC traffic. No separate tree sync protocol.
Chain replication (K6.4) implicitly carries ECC mutations via `ecc.*` events,
providing ~80% CMVG sync. Dedicated cognitive streams (causal CRDT merge,
vector batch, impulse flood) added in K7 for real-time coordination.
CmvgSyncService registered as SystemService, queryable via ServiceApi (D13).

**Rationale**: QUIC provides native stream multiplexing. Using separate protocols
would duplicate connection management, authentication, governance gates, and
chain witnessing. One Noise-encrypted connection per node pair covers control,
chain replication, tree sync, IPC, and (in K7) all cognitive structure sync.
Chain events already include `ecc.hnsw.insert`, `ecc.causal.link`,
`ecc.crossref.create`, and `ecc.impulse.emit`, so replaying the chain
reconstructs most CMVG state without dedicated streams.

**Origin**: CMVG Architecture (K3c + K6 mesh synthesis)

**Cross-references**: M11 in `01-mesh-architecture.md`, CMVG Cognitive Sync
Architecture in `07-phase-K6-mesh-framework.md`

### D15: Sync Framing, Stream Prioritization, Delta Computation, and Observability

**Decision**: Sync frames use RVF wire segments with `SyncStreamType` discriminator.
QUIC stream prioritization: Chain > Tree > IPC > Cognitive > Impulse.
`SyncStateDigest` (~140 bytes) exchanged on stream open for delta computation.
`PeerMetrics` tracks 8 observability dimensions feeding affinity scoring
and circuit breaker decisions. Hybrid KEM upgrade runs once before streams open.

**Rationale**: Stream prioritization ensures foundation state (chain, tree)
propagates before cognitive and ephemeral data. `SyncStateDigest` enables
efficient delta computation without full state exchange. `PeerMetrics` provides
the observability backbone for progressive resolution strategies (round-robin
in K6.3 → affinity in K6.5 → load-aware in K7). KEM timing before stream open
ensures all sync traffic inherits hybrid-encrypted transport.

**Origin**: K6 SPARC refinement (D14 elaboration)

**Cross-references**: CMVG Cognitive Sync Architecture, Stream Prioritization,
Delta Computation, Observability Scoring in `07-phase-K6-mesh-framework.md`

---

**Overall Symposium Verdict**: K6 sprint is approved to proceed. Begin with
K6.0 prep changes (P1-P7) immediately.
