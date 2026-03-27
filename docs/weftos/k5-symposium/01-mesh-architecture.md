# K5 Panel 1: Transport-Agnostic Mesh Architecture

**Date**: 2026-03-25
**Panel**: Mesh Architecture
**Status**: COMPLETE

---

## 1. Design Philosophy

### The Mesh IS the Network

In the DeFi-era networking model, the mesh protocol does not know or care
about the underlying transport. A node on QUIC, a browser on WebSocket, and
an embedded device on BLE all participate in the same logical mesh. The
protocol handles:

- **Identity** -- who are you? (public key)
- **Encryption** -- how do we talk privately? (Noise handshake)
- **Discovery** -- how do I find you? (DHT, mDNS, bootstrap)
- **Routing** -- how does my message reach you? (overlay routing)
- **Application** -- what do we talk about? (WeftOS IPC, chain sync, tree sync)

The transport underneath (TCP, QUIC, WebSocket, WebRTC, BLE, LoRa) is a
pluggable implementation detail.

### Why This Works for WeftOS

WeftOS already has the right abstractions:

1. **`NodePlatform`** in `cluster.rs` distinguishes `CloudNative`, `Edge`,
   `Browser`, `Wasi`, `Custom(String)` -- these map directly to different
   transport capabilities
2. **`PeerNode`** already carries `address`, `capabilities`, and `labels` --
   sufficient for transport negotiation
3. **`MessageTarget`** dispatches on enum variants -- adding `RemoteNode` is
   a one-variant extension
4. **Governance gate** already enforces two-layer access control -- it will
   gate remote messages identically to local ones

---

## 2. Five-Layer Architecture

```
+------------------------------------------------------------------+
|                    APPLICATION LAYER                               |
|  WeftOS IPC (A2ARouter), Chain Sync, Tree Sync, Service Discovery |
+------------------------------------------------------------------+
|                    DISCOVERY LAYER                                 |
|  Kademlia DHT (libp2p-kad), mDNS (libp2p-mdns), Bootstrap Peers  |
+------------------------------------------------------------------+
|                    ENCRYPTION LAYER                                |
|  Noise Protocol (snow) -- XX handshake for first contact,         |
|  IK handshake for known peers, Ed25519 static keys                |
+------------------------------------------------------------------+
|                    TRANSPORT LAYER                                 |
|  quinn (QUIC) | tokio-tungstenite (WS) | webrtc-rs | raw TCP     |
+------------------------------------------------------------------+
|                    IDENTITY LAYER                                  |
|  Ed25519 keypair = node identity, governance.genesis = trust root |
+------------------------------------------------------------------+
```

### Layer Responsibilities

| Layer | Responsibility | Crate(s) |
|-------|---------------|----------|
| Identity | Generate/load Ed25519 keypair, derive NodeId from pubkey | ed25519-dalek |
| Transport | Byte-stream or datagram delivery over physical network | quinn, tokio-tungstenite, webrtc-rs |
| Encryption | Authenticated encryption of all inter-node traffic | snow (Noise) |
| Discovery | Find peers, maintain routing table, handle join/leave | libp2p-kad, libp2p-mdns |
| Application | WeftOS-specific protocols (IPC forward, chain sync, tree sync) | clawft-kernel |

---

## 3. Transport Comparison

### Full Framework: libp2p

| Aspect | Assessment |
|--------|-----------|
| Transport abstraction | Excellent -- pluggable `Transport` trait, multiaddr |
| Encryption | Built-in Noise via snow |
| Discovery | Kademlia DHT, mDNS, Rendezvous |
| Pub/sub | GossipSub |
| Adoption | Polkadot, Filecoin, IPFS, Ethereum 2.0 |
| Concern | Heavy dependency tree, opinionated runtime model |

### QUIC-First: iroh

| Aspect | Assessment |
|--------|-----------|
| Transport | QUIC-only via quinn |
| Encryption | Built-in via QUIC TLS |
| Discovery | Custom relay-based |
| Concern | Not transport-agnostic -- QUIC only, no WebSocket/BLE |

### Recommended: Selective Composition

| Component | Crate | Why |
|-----------|-------|-----|
| Primary transport | `quinn 0.11` | QUIC multiplexing, congestion control, connection migration |
| Encryption | `snow 0.9` | Transport-agnostic Noise Protocol, proven in WireGuard |
| DHT discovery | `libp2p-kad 0.46` | Kademlia routing, peer lookup without full libp2p |
| Local discovery | `libp2p-mdns 0.46` | Zero-config LAN peer finding |
| Key exchange | `x25519-dalek 2.0` | Noise DH operations |
| Framing | `rvf-wire` (existing) | Zero-copy segment serialization already in workspace |

**Rationale**: Full libp2p brings ~80 transitive dependencies and forces its
runtime model. By composing snow + quinn + selective libp2p modules, WeftOS
gets the same capabilities with a fraction of the dependency weight and full
control over the event loop integration with the existing `tokio` runtime.

---

## 4. Transport Trait Design

```rust
/// Transport-agnostic byte stream for WeftOS mesh networking.
/// Each transport implementation (QUIC, WebSocket, TCP, WebRTC, BLE)
/// implements this trait.
#[async_trait]
pub trait MeshTransport: Send + Sync + 'static {
    /// Unique name for this transport (e.g., "quic", "websocket", "ble")
    fn name(&self) -> &str;

    /// Bind and start listening for incoming connections.
    async fn listen(&self, addr: &str) -> Result<TransportListener>;

    /// Connect to a remote peer at the given address.
    async fn connect(&self, addr: &str) -> Result<MeshStream>;

    /// Whether this transport supports the given address scheme.
    fn supports(&self, addr: &str) -> bool;
}

/// A bidirectional byte stream over the mesh.
/// The Noise layer wraps this for encryption.
#[async_trait]
pub trait MeshStream: Send + Sync + 'static {
    async fn send(&mut self, data: &[u8]) -> Result<()>;
    async fn recv(&mut self) -> Result<Vec<u8>>;
    fn remote_addr(&self) -> String;
    async fn close(&mut self) -> Result<()>;
}

/// Listener that accepts incoming MeshStream connections.
#[async_trait]
pub trait TransportListener: Send + Sync + 'static {
    async fn accept(&mut self) -> Result<MeshStream>;
    fn local_addr(&self) -> String;
}
```

### Transport Implementations

| Transport | Target Platforms | Address Format | Crate |
|-----------|-----------------|----------------|-------|
| QUIC | Cloud, Edge | `quic://host:port` | quinn |
| WebSocket | Browser, WASI, Edge | `ws://host:port/mesh` | tokio-tungstenite |
| WebRTC | Browser-to-Browser | `webrtc://sdp-signal-server` | webrtc-rs |
| TCP | Legacy, Edge | `tcp://host:port` | tokio::net |
| BLE | Embedded, IoT | `ble://device-id` | btleplug (future) |
| LoRa | Remote, IoT | `lora://freq/sf/bw` | custom (future) |

---

## 5. Connection Lifecycle

```
Node A (initiator)                     Node B (listener)
    |                                       |
    |--- Transport.connect() ------------->|
    |                                       |--- TransportListener.accept()
    |<-------------- MeshStream ---------->|
    |                                       |
    |--- Noise XX handshake (snow) ------->|
    |    -> e, s, payload                  |
    |<-- e, ee, se, s, es, payload --------|
    |--- payload (encrypted) ------------>|
    |                                       |
    |    [Encrypted MeshStream established] |
    |                                       |
    |--- WeftOS Handshake ----------------->|
    |    NodeId, capabilities, chain_head   |
    |<-- NodeId, capabilities, chain_head --|
    |                                       |
    |    [Peer registered in ClusterMembership]
    |                                       |
    |--- KernelMessage (framed, encrypted)->|
    |<-- KernelMessage (framed, encrypted)-|
```

### Message Framing

Messages are framed using length-prefix encoding over the encrypted stream:

```
+--------+--------+------------------+
| LEN(4) | TYPE(1)| PAYLOAD (LEN-1)  |
+--------+--------+------------------+

LEN:     u32 big-endian, total payload size including TYPE byte
TYPE:    0x01 = KernelMessage (JSON)
         0x02 = KernelMessage (RVF segment)
         0x03 = ChainSync request/response
         0x04 = TreeSync request/response
         0x05 = Heartbeat
         0x06 = Governance rule distribution
         0xFF = Protocol error / close
```

Maximum message size: 16 MiB (enforced at deserialization).

---

## 6. How Different Node Types Connect

```
                         +-----------+
                         | Bootstrap |
                         |  Peers    |
                         +-----+-----+
                               |
            Kademlia DHT / mDNS discovery
                               |
         +----------+----------+----------+
         |          |          |          |
    +----+----+ +---+----+ +--+-----+ +--+-----+
    |  Cloud  | |  Edge  | | Browser| |  WASI  |
    |  (QUIC) | | (QUIC) | |  (WS)  | |  (WS)  |
    +---------+ +--------+ +--------+ +--------+
         |          |          |          |
         +----+-----+----+----+-----+----+
              |          |         |
         Same mesh protocol (Noise-encrypted)
         Same KernelMessage format
         Same governance enforcement
         Same chain verification
```

### Platform-Transport Matrix

| Platform | Primary Transport | Fallback | Discovery |
|----------|------------------|----------|-----------|
| CloudNative | QUIC (quinn) | TCP | Kademlia DHT + bootstrap |
| Edge | QUIC (quinn) | TCP, BLE | Kademlia DHT + mDNS |
| Browser | WebSocket | WebRTC | Bootstrap peers via WS |
| Wasi | WebSocket | -- | Bootstrap peers via WS |
| Embedded | BLE, LoRa | TCP | mDNS, static config |

---

## 7. Governance.genesis as Trust Root

The `governance.genesis` chain event is the cluster-wide trust root. All
nodes in a cluster share the same genesis event, which contains:

1. **Cluster ID** -- SHAKE-256 hash of genesis payload
2. **Founding node's Ed25519 public key** -- the initial authority
3. **Initial governance rules** -- the constitutional rule set
4. **Bootstrap peers** -- seed addresses for discovery

### Cluster Join Protocol

```
Joining Node                          Existing Peer
    |                                      |
    |-- JoinRequest ---------------------->|
    |   { pubkey, capabilities,            |
    |     genesis_hash, platform }         |
    |                                      |--- verify genesis_hash matches
    |                                      |--- check GovernanceGate
    |<- JoinResponse ---------------------|
    |   { accepted: true,                  |
    |     peer_list, governance_rules,     |
    |     chain_head }                     |
    |                                      |
    |-- [Start chain sync] -------------->|
    |-- [Start tree sync] --------------->|
    |                                      |
    |   [Node enters Joining -> Active]    |
```

A node that presents a different `genesis_hash` is rejected -- it belongs
to a different cluster. The GovernanceGate evaluates the join request against
the cluster's governance rules before admission.

---

## 8. Integration with Existing Kernel

### New Module: `mesh.rs` (in clawft-kernel)

The mesh module lives alongside the existing kernel modules and integrates
through the same patterns:

| Existing Module | Integration Point |
|----------------|-------------------|
| `cluster.rs` | `ClusterMembership` receives peer updates from mesh discovery |
| `ipc.rs` | `KernelIpc::send()` forks to mesh transport for `RemoteNode` targets |
| `a2a.rs` | `A2ARouter` gains cluster-aware service resolution |
| `chain.rs` | `LocalChain` gains `tail_from(seq)` for incremental replication |
| `tree_manager.rs` | `TreeManager` gains snapshot/diff for tree sync |
| `governance.rs` | `GovernanceEngine` distributes rules via mesh |
| `boot.rs` | Boot sequence gains mesh listener startup + peer discovery |
| `service.rs` | `ServiceRegistry` gains cross-node service advertisement |

### Feature Gate

All mesh networking code lives behind the `mesh` feature gate:

```toml
[features]
mesh = ["quinn", "snow", "x25519-dalek", "libp2p-kad", "libp2p-mdns"]
```

This preserves the ability to run WeftOS as a single-node kernel with zero
networking dependencies.

---

## 9. Service Resolution Across the Mesh

The A2ARouter's `route_to_service()` method extends from local-only to mesh-wide
resolution through a 9-step flow with multiple caching layers.

### DHT Key Namespacing

All DHT keys include the governance genesis hash as a prefix for cluster isolation:

```
svc:<genesis_hex[0..16]>:cache     → service advertisement
node:<genesis_hex[0..16]>:abc123   → node presence with transport addresses
```

This prevents cross-cluster pollution even if nodes from different governance
roots share the same DHT overlay. Defense-in-depth alongside the join protocol's
governance verification.

### Resolution Caching

Three cache layers prevent redundant network calls:

| Cache | Purpose | TTL | Invalidation |
|-------|---------|-----|--------------|
| Resolution cache | Avoids re-resolving known services | 30s | TTL, gossip event, connection failure |
| Negative cache | Avoids DHT lookup for missing services | 30s | TTL only |
| Connection pool | Reuses Noise+QUIC channels | Idle 60s | Idle timeout, node departure gossip |

### Replicated Service Selection

When multiple nodes host the same service, the DHT returns all advertisers.
Selection evolves with implementation maturity:

- **K6.3**: Round-robin (fair distribution) or lowest-latency (prefer closest node)
- **K6.5**: Affinity (sticky to one node for connection reuse) + circuit breaker
- **K7**: Load-aware using `NodeEccCapability.headroom_ratio` from ECC gossip

### Circuit Breaker

Prevents cascade failures when a remote node is slow or erroring:

```
CLOSED ──[>50% errors over 10 calls]──→ OPEN
OPEN ────[30s cooldown]────────────────→ HALF-OPEN
HALF-OPEN ─[test succeeds]────────────→ CLOSED
HALF-OPEN ─[test fails]───────────────→ OPEN
```

### Architecture Decision

**M9**: Service resolution uses layered caching (resolution + negative + pool)
with genesis-hash-namespaced DHT keys. Replicated services start with round-robin
in K6.3, graduating to load-aware selection in K7.

### Mesh as ServiceApi Adapter (D13)

The mesh transport is architecturally equivalent to Shell, MCP, and daemon RPC --
it is a protocol adapter that dispatches through `ServiceApi`. This means:

1. No dedicated mesh RPC protocol needed
2. Every kernel service is remotely callable without modification
3. Governance gates apply uniformly to local and remote calls
4. Chain witnessing covers remote calls via the existing `ipc.send` event kind
5. `weaver console --attach` could work across the mesh

The only new K6.3 components are:
- `RegistryQueryService`: wraps ServiceRegistry as a boot-time SystemService
- `MeshAdapter`: incoming mesh -> local A2ARouter dispatch
- `mesh.request()`: correlated KernelMessage send with timeout

---

## 10. Ruvector Reuse Strategy

The ruvector crates provide algorithms without I/O. The mesh layer provides
I/O without algorithms. They compose cleanly:

| Ruvector Crate | Algorithm | Mesh Integration |
|---------------|-----------|-----------------|
| ruvector-cluster | SWIM membership | Drive with mesh heartbeats instead of TCP |
| ruvector-raft | Raft consensus | Use mesh transport for AppendEntries/RequestVote |
| ruvector-replication | Log replication | Replicate chain events over mesh streams |
| ruvector-delta-consensus | CRDT gossip | Gossip CRDT deltas over mesh pub/sub |
| ruvector-dag | DAG + post-quantum | Anchor cross-node chain relationships |
| rvf-wire | Zero-copy segments | Use as wire format for mesh messages |

**Key insight**: ruvector algorithms are pure computation. They produce
messages to send and consume messages received. The mesh layer is the I/O
bridge that carries those messages between nodes. No changes to ruvector
crates are needed.

---

## 11. Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| M1 | snow + quinn + selective libp2p | Full libp2p too heavy, iroh not transport-agnostic |
| M2 | Ed25519 pubkey as NodeId | Consistent with chain signing, enables auth |
| M3 | Noise XX for first contact, IK for known peers | Standard Noise patterns |
| M4 | QUIC as primary, WebSocket as browser fallback | Best performance + broadest reach |
| M5 | governance.genesis as cluster trust root | Already exists, natural authority anchor |
| M6 | `mesh` feature gate for all networking | Preserves single-node zero-dep builds |
| M7 | rvf-wire as wire format | Already in workspace, zero-copy |
| M8 | 16 MiB max message size | Prevents memory exhaustion attacks |
| M9 | Layered service resolution with genesis-hash DHT keys | Defense-in-depth cluster isolation, prevents DHT storms, enables replicated service selection |
| M10 | Mesh RPCs reuse ServiceApi adapter pattern (D13) | No dedicated mesh protocol; all services remotely callable via existing dispatch |
| M11 | CMVG sync uses multiplexed QUIC streams over same mesh connection (D14) | No separate tree sync protocol; chain replication provides implicit CMVG sync in K6.4, dedicated cognitive streams in K7 |

---

### CMVG Cognitive Sync (D14)

The ECC cognitive substrate (K3c) syncs across the mesh using multiplexed
QUIC streams. Rather than a separate tree sync protocol, all CMVG structures
share the same Noise-encrypted connection as IPC traffic.

**Design principle**: chain replication implicitly carries CMVG mutations via
`ecc.*` events. Dedicated cognitive streams (causal merge, vector batch,
impulse flood) are optimization for real-time coordination in K7.

| Structure | Sync Mode | Conflict Resolution |
|-----------|-----------|-------------------|
| ExoChain | Log replication | Linear, no conflicts |
| ResourceTree | Merkle diff | Hash comparison, subtree transfer |
| CausalGraph | CRDT G-Set merge | Add-only, HLC ordering |
| HNSW | Entry batch transfer | Insert replay, graph rebuilds locally |
| CrossRefs | Edge gossip | Add-only, HLC dedup |
| Impulses | Ephemeral flood | TTL expiry, (id, hlc) dedup |

**M11**: CMVG sync uses multiplexed QUIC streams over the same mesh
connection, not a separate protocol. Chain replication in K6.4 provides
implicit CMVG sync. Dedicated cognitive streams in K7.
