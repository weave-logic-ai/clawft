# K6 Mesh Networking -- Development Notes

**Date**: 2026-03-26
**Branch**: `feature/weftos-kernel-sprint`
**Status**: Implementation Complete (Phase 1 -- Types, Traits, and Logic)

---

## Summary

K6 delivers the transport-agnostic encrypted mesh networking layer for WeftOS.
It extends the kernel from a single-node system to a multi-node cluster with
encrypted peer-to-peer communication, distributed IPC, chain replication, tree
synchronization, and cluster-wide service and process discovery.

Phase 1 (this branch) implements all types, traits, protocol messages, and
in-process logic (3,543 lines across 14 new files, 133 tests). Actual network
I/O (QUIC via quinn, Noise via snow) is abstracted behind traits and deferred
to Phase 2 wiring.

The architecture follows the 5-layer model approved by the K5 Symposium
(Panel 1, Decision D1):

```
APPLICATION    WeftOS IPC (A2ARouter), Chain Sync, Tree Sync
DISCOVERY      Kademlia DHT, mDNS, Bootstrap Peers
ENCRYPTION     Noise Protocol (snow) -- XX/IK handshakes, Ed25519 keys
TRANSPORT      quinn (QUIC) | tokio-tungstenite (WS) | webrtc-rs
IDENTITY       Ed25519 keypair = node identity, governance.genesis = trust root
```

---

## Implementation Map

### New Files Created

| File | Phase | Lines | Tests | Purpose |
|------|-------|------:|------:|---------|
| `mesh.rs` | K6.1 | 255 | 12 | `MeshTransport` trait, `MeshStream`, `TransportListener`, `WeftHandshake`, `MeshPeer`, `MeshError` |
| `mesh_noise.rs` | K6.1 | 253 | 7 | `EncryptedChannel` trait, `NoiseConfig`, `NoisePattern` (XX/IK), `PassthroughChannel` for testing, `MockStream` pair |
| `mesh_framing.rs` | K6.1 | 240 | 8 | Length-prefix framing: `FrameType` (10 variants), `MeshFrame` encode/decode, `read_frame`/`write_frame` |
| `mesh_listener.rs` | K6.1 | 284 | 9 | `MeshConnectionPool` (DashMap-backed), `JoinRequest`/`JoinResponse`, `PeerInfo` |
| `mesh_discovery.rs` | K6.2 | 276 | 7 | `DiscoveryBackend` trait, `DiscoveryCoordinator` (multi-backend with dedup), `DiscoveredPeer`, `DiscoverySource` |
| `mesh_bootstrap.rs` | K6.2 | 223 | 7 | `BootstrapDiscovery` (static seed peers), `PeerExchangeDiscovery` (learned from handshake) |
| `mesh_ipc.rs` | K6.3 | 201 | 9 | `MeshIpcEnvelope` (routing metadata + hop count + dedup ID), `MeshIpcError`, `inner_target()` unwrap |
| `mesh_dedup.rs` | K6.3 | 163 | 8 | `DedupFilter` -- time-bounded HashMap with TTL eviction + capacity cap, default 60s / 10k entries |
| `mesh_service.rs` | K6.3 | 261 | 11 | `ServiceResolveRequest`/`Response`, `RemoteServiceEndpoint`, `ServiceResolutionCache` (positive + negative cache with TTL) |
| `mesh_chain.rs` | K6.4 | 239 | 8 | `ChainSyncRequest`/`Response`, `ChainBridgeEvent`, `ChainForkStatus` (4 variants), `detect_chain_fork()`, `SyncStateDigest` |
| `mesh_tree.rs` | K6.4 | 255 | 10 | `TreeSyncRequest`/`Response`, `TreeNodeDiff`, `TreeDiffType`, `MerkleProof` with `verify()`, `compare_tree_roots()` |
| `mesh_process.rs` | K6.5 | 292 | 8 | `DistributedProcessTable` (LWW CRDT merge), `ProcessAdvertisement`, `ProcessStatus`, `ResourceSummary`, `least_loaded_node()` |
| `mesh_service_adv.rs` | K6.5 | 244 | 10 | `ClusterServiceRegistry` (multi-node service ads), `ServiceAdvertisement`, `resolve_preferred()`, `remove_node()` |
| `mesh_heartbeat.rs` | K6.5 | 357 | 9 | `HeartbeatTracker` (SWIM protocol), `PeerHeartbeat` state machine (Alive -> Suspect -> Dead), `PingRequest`/`PingResponse` |
| **Total** | | **3,543** | **133** | |

All files reside in `crates/clawft-kernel/src/` and are gated behind `#[cfg(feature = "mesh")]`.

### Existing Files Modified

| File | Changes | Phase |
|------|---------|-------|
| `crates/clawft-kernel/src/lib.rs` | 14 mesh module declarations + re-exports behind `#[cfg(feature = "mesh")]` | K6.1-K6.5 |
| `crates/clawft-kernel/src/ipc.rs` | Added `MessageTarget::RemoteNode { node_id, target }` variant; added `GlobalPid` struct with `Display` impl | K6.0 |
| `crates/clawft-kernel/src/cluster.rs` | Added `bind_address`, `seed_peers`, `identity_key_path` to `ClusterConfig`; added `NodeIdentity` struct (Ed25519 keypair, generate/sign/verify) | K6.0 |
| `crates/clawft-kernel/src/chain.rs` | Added `tail_from(after: u64) -> Vec<ChainEvent>` for incremental replication | K6.0/K6.4 |
| `crates/clawft-kernel/src/tree_manager.rs` | Added `snapshot()` -> `TreeSnapshot` and `apply_remote_mutation()` with signature verification | K6.0/K6.4 |
| `crates/clawft-kernel/Cargo.toml` | Added `mesh` feature gate with `ed25519-dalek` and `rand` dependencies | K6.0 |

---

## Architecture

### 5-Layer Model (as implemented)

```
+-------------------------------------------------------------------+
| APPLICATION LAYER                                                  |
|  mesh_ipc.rs       -- MeshIpcEnvelope, hop tracking, dedup        |
|  mesh_service.rs   -- ServiceResolveRequest/Response, cache       |
|  mesh_chain.rs     -- ChainSyncRequest/Response, fork detection   |
|  mesh_tree.rs      -- TreeSyncRequest/Response, Merkle proofs     |
+-------------------------------------------------------------------+
| DISCOVERY LAYER                                                    |
|  mesh_discovery.rs  -- DiscoveryBackend trait, coordinator         |
|  mesh_bootstrap.rs  -- Seed peers, peer exchange                   |
|  mesh_service_adv.rs -- ClusterServiceRegistry gossip              |
|  mesh_process.rs    -- DistributedProcessTable CRDT gossip         |
|  mesh_heartbeat.rs  -- SWIM heartbeat / failure detection          |
+-------------------------------------------------------------------+
| FRAMING LAYER                                                      |
|  mesh_framing.rs   -- [4-byte len][1-byte type][payload]           |
|                       10 frame types (0x01-0x0A)                   |
+-------------------------------------------------------------------+
| ENCRYPTION LAYER                                                   |
|  mesh_noise.rs     -- EncryptedChannel trait, NoiseConfig          |
|                       PassthroughChannel (test), XX/IK patterns    |
|  (Phase 2: snow-backed SnowChannel)                                |
+-------------------------------------------------------------------+
| TRANSPORT LAYER                                                    |
|  mesh.rs           -- MeshTransport trait, MeshStream trait        |
|  mesh_listener.rs  -- MeshConnectionPool, JoinRequest/Response     |
|  (Phase 2: QuicTransport via quinn)                                |
+-------------------------------------------------------------------+
| IDENTITY LAYER                                                     |
|  cluster.rs        -- NodeIdentity (Ed25519 keypair)               |
|  ipc.rs            -- GlobalPid (node_id, pid)                     |
+-------------------------------------------------------------------+
```

### Module Dependency Graph

```
mesh_heartbeat --.
mesh_process ----.
mesh_service_adv --> ipc (GlobalPid)
                     |
mesh_ipc ----------> ipc (KernelMessage, MessageTarget)
mesh_service ------> (standalone types)
mesh_dedup --------> (standalone, std only)
                     |
mesh_chain --------> (standalone types, chrono)
mesh_tree ---------> (standalone types)
                     |
mesh_discovery ----> (standalone trait)
mesh_bootstrap ----> mesh_discovery (DiscoveryBackend)
                     |
mesh_framing ------> mesh (MeshError, MeshStream, MAX_MESSAGE_SIZE)
mesh_noise --------> mesh (MeshError, MeshStream)
mesh_listener -----> mesh (MeshPeer, WeftHandshake)
                     |
mesh --------------> (root: traits, errors, handshake types)
                     |
cluster -----------> (NodeIdentity, ClusterConfig extensions)
ipc ---------------> (GlobalPid, MessageTarget::RemoteNode)
chain -------------> (tail_from for replication)
tree_manager ------> (snapshot, apply_remote_mutation)
```

---

## Phase-by-Phase Notes

### K6.0: Prep Changes

**What changed**: Modified four existing K0-K5 files to accept mesh extensions
without breaking single-node behavior.

- **`ipc.rs`**: Added `MessageTarget::RemoteNode { node_id, target }` variant.
  The inner `target` is `Box<MessageTarget>`, resolved on the destination node.
  `A2ARouter::send()` gains a new match arm returning
  `Err(IpcError::RemoteNotAvailable)` until the mesh transport is wired.
  Added `GlobalPid { node_id, pid }` with `Display` impl (`"node:pid"` format).
  Per symposium commitment C2.

- **`cluster.rs`**: Extended `ClusterConfig` with `bind_address: Option<String>`,
  `seed_peers: Vec<String>`, `identity_key_path: Option<PathBuf>`. All default
  to `None`/empty so existing configs remain valid. Added `NodeIdentity` behind
  `#[cfg(any(feature = "mesh", feature = "exochain"))]` -- Ed25519 keypair with
  `generate()`, `sign()`, `verify()`, and `node_id()` (hex-encoded SHAKE-256
  hash of public key). Per symposium decision D2.

- **`chain.rs`**: Added `tail_from(after: u64) -> Vec<ChainEvent>` for
  incremental chain replication. Returns events with sequence > `after`.

- **`tree_manager.rs`**: Added `snapshot() -> Result<TreeSnapshot>` for full
  tree state export and `apply_remote_mutation()` with Ed25519 signature
  verification for receiving mutations from remote nodes.

- **`Cargo.toml`**: Added `mesh = ["dep:ed25519-dalek", "dep:rand"]` feature.

**Deviation**: The spec called for `quinn`, `snow`, and `x25519-dalek` in the
mesh feature. These are deferred to Phase 2 -- the mesh feature currently only
pulls `ed25519-dalek` and `rand` for node identity. Transport crypto is behind
trait abstractions.

### K6.1: Transport Layer

**Design**: Three core traits define the transport abstraction (per symposium C3):

- `MeshTransport` -- connect/listen/supports for any transport backend
- `MeshStream` -- bidirectional send/recv/close with optional `remote_addr()`
- `TransportListener` -- accept loop returning `(Box<dyn MeshStream>, SocketAddr)`

**`MeshError`** has 9 variants covering transport, handshake, peer, size, timeout,
encryption, genesis mismatch, connection closed, and I/O errors.

**`WeftHandshake`** carries: node_id, governance_genesis_hash, governance_version,
capabilities bitmap, kem_supported flag, chain_seq, and supported_sync_streams.
Per symposium D3 and D11.

**`MeshConnectionPool`** uses `DashMap<String, MeshPeerConnection>` for lock-free
concurrent access from accept loops, heartbeat tickers, and message dispatch.

**`mesh_framing.rs`**: Wire format is `[4-byte BE length][1-byte type][payload]`.
10 frame types defined (0x01 Handshake through 0x0A SyncDigest). Max frame size
enforced at 16 MiB per symposium D8.

**`mesh_noise.rs`**: Defines `EncryptedChannel` trait (send_encrypted,
recv_encrypted, remote_static_key, close). `NoisePattern::XX` for first contact,
`NoisePattern::IK` for known peers (1-RTT). `PassthroughChannel` wraps a raw
stream for zero-crypto testing. `MockStream::pair()` creates connected in-memory
stream pairs.

**Key decision**: `MeshStream` is an async trait (via `async_trait`) rather than
requiring `AsyncRead + AsyncWrite`. This simplifies the abstraction at the cost
of one vtable indirection, which is negligible relative to network latency.

### K6.2: Discovery

**`DiscoveryBackend`** trait: `start()`, `poll()`, `stop()` -- async, name-returning.
Three concrete implementations planned; two implemented in Phase 1:

1. **`BootstrapDiscovery`**: Reads `ClusterConfig.seed_peers` and emits
   `DiscoveredPeer` entries with `DiscoverySource::SeedPeer`. Idempotent start
   (double-start produces no duplicates). Always available, zero dependencies.

2. **`PeerExchangeDiscovery`**: Learns peers from remote `JoinResponse.peer_list`
   during handshake. Source is rewritten to `PeerExchange` on insert.

3. **Kademlia / mDNS**: Planned as `mesh_kad.rs` and `mesh_mdns.rs` behind the
   `mesh-discovery` feature. Not implemented in Phase 1 (the `mesh-discovery`
   feature gate is not yet in `Cargo.toml`).

**`DiscoveryCoordinator`**: Manages multiple backends. `poll_all()` deduplicates
across backends using a `HashSet<String>` of known node IDs -- a peer discovered
by both bootstrap and peer exchange is returned only once.

### K6.3: Cross-Node IPC

**`MeshIpcEnvelope`**: Wraps `KernelMessage` with routing metadata:
- `source_node` / `dest_node` -- for routing
- `hop_count` -- incremented at each relay, max 8 hops (prevents routing loops)
- `envelope_id` -- UUID for deduplication
- `inner_target()` -- unwraps `RemoteNode` wrapper to get the local target

**`DedupFilter`**: Time-bounded `HashMap<String, Instant>` with:
- Configurable TTL (default 60s) and max capacity (default 10,000)
- `check_and_insert()` returns true for new messages, false for duplicates
- Automatic TTL eviction on each check + oldest-first eviction at capacity
- Not a bloom filter as originally spec'd -- HashMap is simpler and provides
  exact dedup. Can be swapped for a bloom filter if memory pressure requires it.

**`ServiceResolutionCache`**: Dual-cache for remote service resolution:
- Positive cache: `HashMap<name, ResolvedService>` with per-entry TTL
- Negative cache: `HashMap<name, Instant>` with 30s TTL
- `evict_expired()` cleans both caches
- Positive insert clears negative entry (service came back online)

Per symposium D13: all remote service resolution reuses the `ServiceApi` pattern.
`mesh_service.rs` defines the wire types; the `MeshAdapter` and
`RegistryQueryService` wiring is deferred to Phase 2.

### K6.4: Chain Replication + Tree Sync

**Chain sync** (`mesh_chain.rs`):
- `ChainSyncRequest` carries `chain_id`, `after_sequence`, `after_hash`, `max_events`
- `ChainSyncResponse` returns JSON-serialized events with `has_more` pagination
- `ChainBridgeEvent` anchors a remote chain's head hash in the local chain
  (cross-chain accountability per symposium discussion)
- `detect_chain_fork()` compares (seq, hash) pairs and returns one of 4 states:
  `InSync`, `LocalBehind`, `RemoteBehind`, `Forked` (same seq, different hash)
- `SyncStateDigest` provides a compact summary for sync stream initialization

**Tree sync** (`mesh_tree.rs`):
- `TreeSyncRequest` carries local Merkle root hash + node count + full_snapshot flag
- `TreeSyncResponse` returns diff nodes and deleted paths
- `TreeNodeDiff` includes path, kind, hash, metadata, chain_seq, and diff type
- `MerkleProof` with `verify()` -- currently structural validation only; full
  hash recomputation requires wiring to the tree's hash function
- `compare_tree_roots()` returns `InSync`, `FullPull`, `FullPush`, or `IncrementalSync`

**`chain.rs` changes**: `tail_from(after)` enables incremental pull -- returns
all events with `sequence > after`. Tests verify full export (from 0), incremental
(from N), and empty result (from head).

**`tree_manager.rs` changes**: `snapshot()` serializes full tree state.
`apply_remote_mutation()` verifies Ed25519 signature before applying.

### K6.5: Distributed Process Table + Service Discovery

**`DistributedProcessTable`** (`mesh_process.rs`):
- LWW CRDT semantics: `merge()` compares `last_updated` timestamps, newer wins
- `ProcessAdvertisement` carries: global_pid, agent_type, capabilities, services,
  status, last_updated, resource_summary
- `ProcessStatus`: Running, Suspended, Stopping, Unreachable
- Query methods: `find_by_type()`, `find_by_capability()`, `least_loaded_node()`
- Failure handling: `mark_node_unreachable()` flips status, `remove_node()` purges

**`ClusterServiceRegistry`** (`mesh_service_adv.rs`):
- `HashMap<name, Vec<ServiceAdvertisement>>` -- one service can have multiple
  node replicas
- `merge()` uses LWW per (service_name, node_id) pair
- `resolve()` returns all hosting nodes; `resolve_preferred()` tries a specific
  node first, falls back to any replica
- `remove_node()` cleans up all advertisements from a departed node

**`HeartbeatTracker`** (`mesh_heartbeat.rs`):
- Simplified SWIM protocol: Alive -> Suspect (missed direct ping) -> Dead
  (suspect timeout exceeded)
- `HeartbeatConfig` with tunable intervals: probe (1s), ping timeout (500ms),
  indirect timeout (1s), 3 indirect witnesses, suspect timeout (5s)
- `PingRequest`/`PingResponse` carry sequence numbers for correlation and
  indirect ping support (`on_behalf_of` field)
- `HeartbeatTracker` manages per-peer state and exposes `suspect_peers()` /
  `dead_peers()` queries

---

## Deviations from SPARC Spec

| Deviation | Rationale |
|-----------|-----------|
| `quinn` / `snow` / `x25519-dalek` not yet as Cargo dependencies | Trait abstractions are in place. Adding concrete implementations is Phase 2 wiring. Keeps compile times low and the mesh feature gate lightweight. |
| `DedupFilter` uses `HashMap` instead of bloom filter | Exact dedup with automatic eviction. Bloom filter would save memory but add false-positive complexity. Can be swapped later if 10k entries becomes a bottleneck. |
| `mesh_kad.rs` and `mesh_mdns.rs` not created | Kademlia and mDNS discovery are planned behind the `mesh-discovery` feature gate. Bootstrap + peer exchange cover the common deployment case. |
| `mesh_quic.rs` not created | QuicTransport implementing `MeshTransport` trait is Phase 2. The trait is ready. |
| `mesh_adapter.rs` not created | MeshAdapter (incoming dispatch through A2ARouter) and RegistryQueryService are Phase 2 wiring. The wire types (`mesh_ipc.rs`, `mesh_service.rs`) are ready. |
| Post-quantum KEM (K6.4b) deferred | `WeftHandshake.kem_supported` flag is present. Actual ML-KEM-768 handshake upgrade step requires `ruvector-dag` with `production-crypto` feature. Deferred to a dedicated milestone. |
| `MerkleProof.verify()` is structural only | Full hash-chain verification requires access to the tree's hash function. Current verify checks non-empty fields. Full verification will be wired when `mesh_tree.rs` connects to `TreeManager`. |
| `MeshStream` uses async trait instead of `AsyncRead + AsyncWrite` | Simplifies the abstraction. One vtable indirection is negligible vs. network latency. |

---

## Test Coverage

| Module | Test Count | Coverage Areas |
|--------|:----------:|----------------|
| `mesh.rs` | 12 | All 9 error variant displays, MAX_MESSAGE_SIZE constant, WeftHandshake serde roundtrip, MeshPeer field access |
| `mesh_noise.rs` | 7 | NoisePattern variants + serde, NoiseConfig fields, PassthroughChannel send/recv/bidirectional/close/no-remote-key |
| `mesh_framing.rs` | 8 | FrameType from_byte (all 10 + unknown), encode/decode roundtrip (all types), empty payload, oversized frame rejection |
| `mesh_listener.rs` | 9 | Pool register/get/remove/overwrite, connected_peers list, default constructor, JoinRequest/Response/PeerInfo serde |
| `mesh_discovery.rs` | 7 | Coordinator add/start, poll dedup across backends, second poll empty, stop, DiscoveredPeer serde, DiscoverySource serde, error display |
| `mesh_bootstrap.rs` | 7 | Start produces peers, poll drains, double-start no dup, add_seed dedup, stop clears, PeerExchange add+poll, PeerExchange stop |
| `mesh_ipc.rs` | 9 | Envelope defaults, to/from bytes roundtrip, oversized rejection, invalid JSON rejection, hop increment + max exceeded, inner_target unwrap + passthrough, correlation_id preservation |
| `mesh_dedup.rs` | 8 | New returns true, duplicate returns false, different IDs, is_duplicate without insert, expired eviction, capacity eviction (oldest), empty filter, default settings |
| `mesh_service.rs` | 11 | Cache insert/get, expired returns None, missing key, negative cache, positive clears negative, evict expired, resolve request/response serde (found + not found), resolved expiry, empty cache |
| `mesh_chain.rs` | 8 | ChainSyncRequest/Response serde, BridgeEvent serde, detect_fork (in-sync, local-behind, remote-behind, forked), SyncStateDigest serde |
| `mesh_tree.rs` | 10 | TreeSyncRequest/Response serde, TreeNodeDiff serde, MerkleProof verify (valid + empty fields), compare_tree_roots (4 actions), TreeDiffType variant serde |
| `mesh_process.rs` | 8 | ProcessAdvertisement serde, LWW merge (newer wins, older ignored), find_by_type, find_by_capability, least_loaded_node, remove_node, mark_unreachable |
| `mesh_service_adv.rs` | 10 | ServiceAdvertisement serde, merge + resolve, resolve_preferred (found + fallback + missing), remove_node, multiple replicas, service_names, merge update + ignore older |
| `mesh_heartbeat.rs` | 9 | PingRequest/Response serde, config defaults, Alive->Suspect transition, Suspect->Dead transition, record_alive reset, tracker add/remove, suspect/dead peer queries, sequence increment |
| **Total** | **133** | |

---

## Feature Gate Structure

### Current (`Cargo.toml`)

```toml
[features]
mesh = ["dep:ed25519-dalek", "dep:rand"]
```

All 14 mesh modules are gated behind `#[cfg(feature = "mesh")]` in `lib.rs`.
When `mesh` is disabled, zero mesh code is compiled -- the kernel remains a
pure single-node system.

### Planned (Phase 2)

```toml
mesh = ["dep:ed25519-dalek", "dep:rand", "dep:quinn", "dep:snow", "dep:x25519-dalek"]
mesh-discovery = ["mesh", "dep:libp2p-kad", "dep:libp2p-mdns"]
mesh-full = ["mesh", "mesh-discovery"]
```

### Build Configurations

| Build | Features | Use Case |
|-------|----------|----------|
| Single-node | `default` (no mesh) | Development, testing, embedded |
| Static cluster | `mesh` | Known peers, seed-peer bootstrap |
| Dynamic cluster | `mesh-full` | DHT + mDNS auto-discovery |

---

## Next Steps (Phase 2: Wiring)

Phase 2 connects the type/trait layer to live networking. Approximate order:

1. **Add transport dependencies to workspace `Cargo.toml`**
   - `quinn = { version = "0.11", optional = true }`
   - `snow = { version = "0.9", optional = true }`
   - `x25519-dalek = { version = "2.0", optional = true }`

2. **Implement `QuicTransport`** (`mesh_quic.rs`, ~120 lines)
   - Implements `MeshTransport` trait
   - `listen()` -> quinn endpoint server
   - `connect()` -> quinn endpoint client
   - `QuicStream` wraps `(SendStream, RecvStream)` as `MeshStream`

3. **Implement `SnowChannel`** (extend `mesh_noise.rs`, ~100 lines)
   - Implements `EncryptedChannel` trait with real snow sessions
   - XX handshake for first contact, IK for known peers
   - WeftHandshake payload exchange inside Noise step 3

4. **Wire `MeshListener` into `boot.rs`**
   - Start QUIC listener on `ClusterConfig.bind_address`
   - Accept loop: handshake -> register in `MeshConnectionPool`
   - Check `governance_genesis_hash` match (reject mismatches)

5. **Implement `MeshAdapter`** (`mesh_adapter.rs`, ~80 lines)
   - Receives `MeshIpcEnvelope` from incoming mesh streams
   - Checks `DedupFilter`, increments hop count
   - Unwraps `RemoteNode` target, dispatches through local `A2ARouter::send()`
   - Returns response via correlation_id

6. **Wire `A2ARouter` for remote dispatch** (modify `a2a.rs`)
   - On `MessageTarget::RemoteNode`: look up node in `MeshConnectionPool`
   - Wrap in `MeshIpcEnvelope`, serialize, send via encrypted stream
   - Add `ServiceApi`-based remote resolution (per symposium D13)

7. **Wire chain/tree sync into boot sequence**
   - `SyncStateDigest` exchange on peer connect
   - `detect_chain_fork()` -> pull/push as needed via `tail_from()`
   - `compare_tree_roots()` -> full snapshot or incremental diff

8. **Start heartbeat/gossip tasks**
   - Periodic `HeartbeatTracker` tick for SWIM failure detection
   - Periodic gossip of `ProcessAdvertisement` and `ServiceAdvertisement`
   - On Dead detection: `mark_node_unreachable()` + `remove_node()`

---

## K0-K5 Integration Points

Changes made to prepare existing kernel modules for K6:

| Module | Addition | Purpose |
|--------|----------|---------|
| `ipc.rs` | `MessageTarget::RemoteNode { node_id, target }` | Route messages to specific processes on remote nodes |
| `ipc.rs` | `GlobalPid { node_id, pid }` | Cross-node process addressing |
| `cluster.rs` | `ClusterConfig.bind_address` | Mesh listener bind address |
| `cluster.rs` | `ClusterConfig.seed_peers` | Bootstrap peer discovery |
| `cluster.rs` | `ClusterConfig.identity_key_path` | Persistent node identity |
| `cluster.rs` | `NodeIdentity` | Ed25519 keypair for signing/verification |
| `chain.rs` | `tail_from(after: u64)` | Incremental chain replication (pull delta) |
| `tree_manager.rs` | `snapshot()` | Full tree state export for sync |
| `tree_manager.rs` | `apply_remote_mutation()` | Receive verified mutations from remote nodes |

All additions are backward-compatible. Existing single-node tests pass unchanged.
`RemoteNode` is never constructed by K0-K5 code. `GlobalPid` is only used by
mesh modules. `tail_from()`, `snapshot()`, and `apply_remote_mutation()` are
additive methods with no impact on existing call paths.

---

## Symposium Decision Traceability

| Decision | Description | Implementation |
|----------|-------------|----------------|
| D1 | 5-layer architecture | Reflected in module layout (see Architecture section) |
| D2 | Ed25519 node identity | `NodeIdentity` in `cluster.rs` |
| D3 | Noise XX/IK handshakes | `NoisePattern`, `EncryptedChannel` trait in `mesh_noise.rs` |
| D8 | 16 MiB max message size | `MAX_MESSAGE_SIZE` constant, enforced in framing + IPC |
| D11 | Post-quantum KEM negotiation | `WeftHandshake.kem_supported` field present; implementation deferred |
| D13 | ServiceApi-based mesh RPC | `ServiceResolveRequest/Response` types ready; MeshAdapter deferred to Phase 2 |
| D15 | Stream priority + sync framing | `FrameType` enum with 10 frame types, `SyncStateDigest` |
| C2 | GlobalPid for cross-node addressing | `GlobalPid` in `ipc.rs` |
| C3 | Transport-agnostic trait | `MeshTransport` trait in `mesh.rs` |

---

## ruvector Crate Integration Status

The following ruvector workspace crates are available for Phase 2 integration:

| Crate | Relevance | Status |
|-------|-----------|--------|
| `ruvector-cluster` | Cluster membership + failure detection | Ready -- could back `HeartbeatTracker` with production SWIM |
| `ruvector-raft` | Consensus for leader election | Ready -- useful if mesh needs a coordinator node |
| `ruvector-delta-consensus` | CRDT delta replication | Ready -- could back `DistributedProcessTable` and `ClusterServiceRegistry` gossip |
| `ruvector-dag` | DAG operations + `production-crypto` feature | Ready -- provides ML-KEM-768 for post-quantum KEM (K6.4b) |
| `rvf-wire` | Zero-copy wire format | Already in workspace -- `MeshIpcEnvelope` can use RVF instead of JSON for efficiency |
| `rvf-crypto` | Cryptographic primitives | Already in exochain feature -- Ed25519 signing used by `NodeIdentity` |
