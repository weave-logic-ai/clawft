# K5 Panel 2: K6 Readiness Assessment

**Date**: 2026-03-25
**Panel**: K6 Readiness
**Status**: COMPLETE

---

## 1. Executive Summary

A source-level audit of all K0-K5 kernel code assessed readiness for K6
networking. The kernel is **75% ready** -- 41 items are GREEN (wire-ready),
22 need minor changes (YELLOW), and 21 are missing (RED). The 6 critical
RED gaps are all addressable in ~1,150 lines of new Rust code.

### Readiness Matrix

| Subsystem | GREEN | YELLOW | RED | File(s) |
|-----------|:-----:|:------:|:---:|---------|
| Cluster | 10 | 5 | 3 | `cluster.rs` |
| IPC/A2A | 6 | 5 | 3 | `ipc.rs`, `a2a.rs` |
| Chain | 8 | 3 | 4 | `chain.rs` |
| Governance | 5 | 3 | 2 | `governance.rs`, `gate.rs` |
| Security | 6 | 3 | 5 | capability, gate, chain |
| Tree | 6 | 3 | 4 | `tree_manager.rs`, exo-resource-tree |
| **Total** | **41** | **22** | **21** | |

---

## 2. GREEN -- Ready for K6

These items require no changes. They are wire-ready and will work across
nodes as-is.

### 2.1 Cluster Module (`crates/clawft-kernel/src/cluster.rs`)

| Item | Detail |
|------|--------|
| `ClusterMembership` | Dynamic peer join/leave via `add_peer()`/`remove_peer()`, `DashMap` backing |
| `NodeState` lifecycle | `Joining -> Active -> Suspect -> Unreachable -> Leaving -> Left` |
| `PeerNode` fields | `id`, `name`, `platform`, `state`, `address`, `capabilities`, `labels` |
| `NodePlatform` enum | `CloudNative`, `Edge`, `Browser`, `Wasi`, `Custom(String)` |
| `ClusterConfig` | Heartbeat interval, suspect/unreachable thresholds, max node count |
| `ClusterService` | Wraps `ruvector_cluster::ClusterManager`, syncs via `sync_to_membership()` |
| `NodeEccCapability` | Cognitive tick metrics, HNSW stats, spectral analysis (ECC feature) |
| Serde derives | All types derive `Serialize`/`Deserialize` |
| Test coverage | Serde roundtrips, peer lifecycle, error cases |
| State mapping | Bidirectional ruvector-to-WeftOS state mapping in `cluster_node_to_peer()` |

### 2.2 IPC/A2A (`crates/clawft-kernel/src/ipc.rs`, `a2a.rs`)

| Item | Detail |
|------|--------|
| `KernelMessage` serde | All fields wire-safe: `id`, `from`, `target`, `payload`, `timestamp`, `correlation_id` |
| `MessagePayload` variants | `Text`, `Json`, `ToolCall`, `ToolResult`, `Signal`, `Rvf` (binary) |
| `A2ARouter` separation | Capability checking, inbox, topic routing, service routing -- distinct layers |
| Correlation IDs | Fully supported for request-response patterns |
| Dual-layer governance | Routing-time gate + handler-time gate via `GateBackend` trait |
| `MessageTarget` extensibility | `Service` and `ServiceMethod` variants show extensibility pattern |

### 2.3 Chain (`crates/clawft-kernel/src/chain.rs`)

| Item | Detail |
|------|--------|
| `ChainEvent` serde | All fields serializable: `sequence`, `chain_id`, `timestamp`, `prev_hash`, `hash`, `payload_hash`, `source`, `kind`, `payload` |
| SHAKE-256 hashing | `compute_event_hash()` commits to all fields including payload |
| Integrity verification | `verify_integrity()` walks all events checking hash linkage |
| RVF segment persistence | `save_to_rvf()` / `load_from_rvf()` with signed segments |
| Ed25519 signing | `with_signing_key()` for RVF segment signatures |
| ML-DSA-65 dual signing | Post-quantum dual signing available |
| Witness chain | `WitnessEntry`, `create_witness_chain`, `verify_witness_chain` |
| Lineage records | `record_lineage()` / `verify_lineage()` for provenance |

### 2.4 Governance (`crates/clawft-kernel/src/governance.rs`, `gate.rs`)

| Item | Detail |
|------|--------|
| `GovernanceRule` serde | `id`, `description`, `branch`, `severity`, `active`, `reference_url`, `sop_category` |
| Deterministic evaluation | Same rules + same request = same decision on any node |
| RVF bridge | `to_rvf_policy_check()`, `to_rvf_mode()`, `to_rvf_policy()` |
| `GateBackend` trait | Extensible: `CapabilityGate`, `TileZeroGate` -- can add `RemoteGate` |
| `GateDecision` tokens | `token: Vec<u8>` (permit) and `receipt: Vec<u8>` (deny) for cryptographic attestation |

### 2.5 Security

| Item | Detail |
|------|--------|
| Capability-based access | `CapabilityChecker` validates tool, IPC scope, and service access |
| `IpcScope` granularity | `All`, `ParentOnly`, `Restricted(Vec<Pid>)`, `Topic(Vec<String>)`, `None` |
| `ResourceLimits` | Memory, CPU time, tool call count, message count budgets per agent |
| Dual-layer gate | Routing-time + handler-time independent checks |
| Chain integrity | SHAKE-256 hashes + optional Ed25519 + ML-DSA-65 signatures |
| RVF validation | `validate_segment` / `verify_segment` checks structure before parsing |

### 2.6 Resource Tree (`crates/clawft-kernel/src/tree_manager.rs`, `crates/exo-resource-tree`)

| Item | Detail |
|------|--------|
| Merkle root hash | `root_hash()` via `recompute_all()` -- 32-byte compact consistency check |
| Atomic mutations | Every tree mutation generates both `MutationEvent` and `ChainEvent` |
| `chain_seq` metadata | Links tree nodes to causative chain events |
| `TreeStats` serde | Node count, mutation count, root hash -- exchangeable between nodes |
| `/network/peers/` namespace | Already bootstrapped in tree |
| `add_peer_with_tree()` | Creates tree node under `/network/peers/{name}` (exochain feature) |

---

## 3. YELLOW -- Minor Changes Needed

These items work but need small modifications for K6.

### 3.1 Cluster

| Item | Change Needed | Effort |
|------|--------------|--------|
| `PeerNode.address` is `Option<String>` | Add structured `SocketAddr` alternative or `MeshAddress` type | ~15 lines |
| `ClusterConfig` lacks `bind_address` | Add `bind_address: Option<String>` and `listen_port: Option<u16>` | ~5 lines |
| `ClusterConfig` lacks `seed_peers` | Add `seed_peers: Vec<String>` for bootstrap discovery | ~3 lines |
| `sync_to_membership()` is pull-based | Add background tick loop in K6 mesh module | ~30 lines |
| `cluster_node_to_peer()` hardcodes platform | Read platform from node metadata labels | ~10 lines |

### 3.2 IPC/A2A

| Item | Change Needed | Effort |
|------|--------------|--------|
| No `RemoteNode` target | Add `MessageTarget::RemoteNode { node_id: String, target: Box<MessageTarget> }` | ~10 lines |
| `KernelIpc::send()` is local-only | Add transport fork: if remote, serialize and send over mesh | ~40 lines |
| `A2ARouter` resolves locally | Add cluster-aware service resolution path | ~50 lines |
| `Pid` is `u64` (locally unique) | Add `GlobalPid { node_id: String, pid: Pid }` | ~20 lines |
| Inbox is `mpsc` (in-process) | Add network transport bridge for remote delivery | ~60 lines |

### 3.3 Chain

| Item | Change Needed | Effort |
|------|--------------|--------|
| `chain_id` is `u32` | Define globally unique convention: `hash(node_id + chain_purpose)` | ~10 lines |
| All events in `Vec<ChainEvent>` | Add event pruning or paged export API | ~40 lines |
| JSON for file I/O | Use RVF segments as canonical wire format (already supported) | ~5 lines |

### 3.4 Governance

| Item | Change Needed | Effort |
|------|--------------|--------|
| Rules in local `Vec<GovernanceRule>` | Add rule synchronization mechanism | ~40 lines |
| `GovernanceRequest.agent_id` no node_id | Add `node_id: Option<String>` field | ~5 lines |
| No cluster-wide governance root | Define shared "constitutional genesis" event | ~20 lines |

### 3.5 Security

| Item | Change Needed | Effort |
|------|--------------|--------|
| No max message size on deser | Add 16 MiB limit to `serde_json::from_str` | ~5 lines |
| `PeerNode.capabilities` unvalidated | Validate against known capability set | ~15 lines |
| No rate-limiting on `add_peer()` | Add rate limiter (token bucket) | ~20 lines |

### 3.6 Resource Tree

| Item | Change Needed | Effort |
|------|--------------|--------|
| `ResourceTree` uses `Mutex` (non-async) | Use `tokio::sync::RwLock` or message-passing | ~30 lines |
| `recompute_all()` full recalc | Add incremental dirty-flag hash propagation | ~50 lines |
| `MutationEvent.signature` always `None` | Sign mutations with node Ed25519 key | ~15 lines |

---

## 4. RED -- Critical Gaps

These are entirely missing and must be built for K6.

### 4.1 No Transport Layer (Critical -- K6.1)

There is no TCP/WebSocket/QUIC listener or client anywhere in the kernel.
This is the primary K6 deliverable.

**Files affected**: New module `mesh.rs` or new crate `clawft-net`
**Estimated effort**: ~400 lines (transport trait + QUIC impl + Noise wrapper)

### 4.2 No Cluster-Join Authentication (Critical -- K6.0)

`add_peer()` accepts any `PeerNode` with no signature verification. A
malicious node can join by calling `add_peer()` with fabricated data.

**Files affected**: `cluster.rs`, new `auth.rs`
**Estimated effort**: ~80 lines (signed JoinRequest + genesis hash verification)

### 4.3 No TLS/Noise Encryption (Critical -- K6.1)

All inter-process communication is in-process. No encrypted channel exists.

**Files affected**: New in mesh module
**Estimated effort**: Included in K6.1 transport layer (~400 lines)

### 4.4 No `RemoteNode` Message Target (Critical -- K6.0)

There is no way to address a message to a remote node's inbox.

**Files affected**: `ipc.rs` (add variant), `a2a.rs` (add routing arm)
**Estimated effort**: ~30 lines

### 4.5 No Chain Merge / Conflict Resolution (Critical -- K6.4)

If two nodes independently extend a chain (fork), there is no reconciliation
protocol. No cross-node chain anchoring. No incremental replication. No
chain subscription.

**Files affected**: `chain.rs`, new `chain_sync.rs`
**Estimated effort**: ~250 lines

### 4.6 No Tree Sync (Critical -- K6.4)

No mechanism to transfer tree snapshots, synchronize incremental changes,
compute tree diffs, or generate Merkle proofs.

**Files affected**: `tree_manager.rs`, exo-resource-tree `model.rs`
**Estimated effort**: ~150 lines

---

## 5. Ruvector Crate Inventory

### Available Crates and Reuse Potential

| Crate | Purpose | Has Algorithm | Has I/O | K6 Reuse |
|-------|---------|:---:|:---:|----------|
| ruvector-cluster | SWIM membership | Yes | No | Drive with mesh heartbeats |
| ruvector-raft | Raft consensus | Yes | No | Use for leader election |
| ruvector-replication | Log replication | Yes | No | Replicate chain events |
| ruvector-delta-consensus | CRDT + gossip | Yes | No | Gossip CRDT deltas |
| ruvector-dag | DAG + PQ crypto | Yes | No | Cross-node chain anchoring |
| rvf-wire | Zero-copy segments | Yes | No | Wire format for mesh messages |

### Key Observations

1. **All ruvector crates have complete algorithms but NO network I/O.** They
   produce messages to send and consume messages received, but the actual
   network transport is not implemented. This is exactly the right boundary
   for WeftOS -- the mesh layer provides the I/O.

2. **ruvector-delta-consensus has CRDT + gossip data model** that maps directly
   to the state sync needed for distributed cluster membership, service
   registries, and governance rule distribution.

3. **ruvector-dag has post-quantum crypto** (pqcrypto-dilithium for ML-DSA-65,
   pqcrypto-kyber for ML-KEM-768) already compiled and available. This aligns
   with the dual-signing already in `chain.rs`.

4. **rvf-wire provides zero-copy segment serialization** that is already used
   for chain event persistence. Using it as the mesh wire format avoids
   introducing a new serialization dependency.

5. **NO existing Noise/QUIC/libp2p dependencies** anywhere in the ruvector
   workspace. These are net-new dependencies for K6.

### Dependency Impact

New dependencies required for K6:

| Crate | Version | Feature | Size Impact |
|-------|---------|---------|-------------|
| quinn | 0.11 | QUIC transport | ~2.5 MB compile |
| snow | 0.9 | Noise Protocol | ~500 KB compile |
| x25519-dalek | 2.0 | Noise DH | Already via ed25519-dalek |
| libp2p-kad | 0.46 | Kademlia DHT | ~1 MB compile (optional) |
| libp2p-mdns | 0.46 | mDNS discovery | ~500 KB compile (optional) |

All behind the `mesh` feature gate -- zero impact on non-networked builds.

---

## 6. Strongest Assets for K6

The following existing code is the strongest foundation for K6:

1. **Cluster module** (`cluster.rs`) -- Complete peer lifecycle with
   `DashMap`-backed concurrent membership, configurable failure detection,
   and platform-aware node representation. K6 needs only to wire mesh
   discovery events into `ClusterMembership`.

2. **Chain crypto** (`chain.rs`) -- SHAKE-256 hash chain with Ed25519 + ML-DSA-65
   signing, integrity verification, and RVF segment persistence. Cross-node
   chain verification requires zero changes to the hash/sign/verify code.

3. **IPC message types** (`ipc.rs`) -- `KernelMessage` and `MessagePayload`
   are fully serializable with all wire-needed fields. The `Rvf` payload
   variant provides native binary transport.

4. **Governance determinism** (`governance.rs`) -- The governance engine is
   pure and deterministic: same rules + same request = same decision on any
   node. This means governance decisions are verifiable across the mesh
   without additional consensus.

5. **Resource tree Merkle** (`tree_manager.rs`) -- The Merkle root hash
   provides a 32-byte consistency fingerprint. Two nodes can compare
   `root_hash()` to detect divergence before initiating expensive sync.

---

## 7. Estimated K6 Effort

| Phase | Scope | New Lines | Changed Lines |
|-------|-------|:---------:|:------------:|
| K6.0 | Prep (RemoteNode, GlobalPid, ClusterConfig) | ~50 | ~150 |
| K6.1 | Transport + Noise encryption | ~400 | ~20 |
| K6.2 | Discovery (Kademlia + mDNS) | ~300 | ~30 |
| K6.3 | Cross-node IPC (A2A over mesh) | ~300 | ~80 |
| K6.4 | Chain replication + tree sync | ~250 | ~50 |
| K6.5 | Distributed process table + service discovery | ~200 | ~40 |
| **Total** | | **~1,500** | **~370** |

Note: The original ~1,150 line estimate covered new code only. Including
necessary modifications to existing files brings the total to ~1,870 lines
of changes across the K6 sprint.
