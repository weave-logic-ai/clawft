---
name: weftos-mesh
description: WeftOS mesh networking skill — transport traits, 5-layer architecture, frame types, discovery, and testing patterns for K6 mesh modules
version: 0.1.0
category: development
tags:
  - mesh
  - networking
  - transport
  - noise
  - discovery
  - framing
  - k6
author: WeftOS Kernel Team
---

# WeftOS Mesh Networking Skill

This skill teaches how to work on the mesh networking subsystem (K6) in
`clawft-kernel`. All mesh code is behind `#[cfg(feature = "mesh")]`.

## 5-Layer Architecture

The mesh subsystem follows a layered design, bottom to top:

```
Layer 5: Distributed State  — mesh_process, mesh_service_adv, mesh_kad
Layer 4: Sync Protocols     — mesh_chain, mesh_tree, mesh_heartbeat
Layer 3: Service Resolution — mesh_service, mesh_discovery, mesh_bootstrap, mesh_mdns
Layer 2: Framing & IPC      — mesh_framing, mesh_ipc, mesh_dedup
Layer 1: Transport & Crypto — mesh (traits), mesh_noise, mesh_tcp, mesh_ws, mesh_listener
```

### Layer 1: Transport & Crypto

**Files**: `mesh.rs`, `mesh_noise.rs`, `mesh_tcp.rs`, `mesh_ws.rs`, `mesh_listener.rs`

Core traits that all transports implement:

```rust
/// Bidirectional byte stream.
#[async_trait]
pub trait MeshStream: Send + Sync + 'static {
    async fn send(&mut self, data: &[u8]) -> Result<(), MeshError>;
    async fn recv(&mut self) -> Result<Vec<u8>, MeshError>;
    async fn close(&mut self) -> Result<(), MeshError>;
    fn remote_addr(&self) -> Option<SocketAddr>;
}

/// Listens for incoming connections.
#[async_trait]
pub trait TransportListener: Send + Sync + 'static {
    async fn accept(&mut self) -> Result<(Box<dyn MeshStream>, SocketAddr), MeshError>;
    fn local_addr(&self) -> Result<SocketAddr, MeshError>;
}

/// Transport-agnostic mesh transport.
#[async_trait]
pub trait MeshTransport: Send + Sync + 'static {
    fn name(&self) -> &str;
    async fn listen(&self, addr: &str) -> Result<Box<dyn TransportListener>, MeshError>;
    async fn connect(&self, addr: &str) -> Result<Box<dyn MeshStream>, MeshError>;
    fn supports(&self, addr: &str) -> bool;
}
```

**Existing implementations**:
- `TcpTransport` (`mesh_tcp.rs`) — TCP with length-prefix framing
- `WsTransport` (`mesh_ws.rs`) — WebSocket transport

**Noise encryption** (`mesh_noise.rs`):
- Noise XX handshake pattern for mutual authentication
- `EncryptedChannel` wraps a `MeshStream` with encrypt/decrypt
- `NoiseConfig` holds keypair and handshake state

**Connection pooling** (`mesh_listener.rs`):
- `MeshConnectionPool` manages active peer connections
- `JoinRequest`/`JoinResponse` for cluster joining
- `PeerInfo` tracks connected peers

### Layer 2: Framing & IPC

**Files**: `mesh_framing.rs`, `mesh_ipc.rs`, `mesh_dedup.rs`

Wire protocol: `[4-byte big-endian length][1-byte frame type][payload]`

**Frame types** (`FrameType` enum in `mesh_framing.rs`):

| Discriminant | Name | Purpose |
|-------------|------|---------|
| `0x01` | Handshake | WeftOS handshake payload |
| `0x02` | IpcMessage | KernelMessage (agent IPC) |
| `0x03` | ChainSync | Chain sync request/response |
| `0x04` | TreeSync | Tree sync request/response |
| `0x05` | ServiceAdvert | Service advertisement |
| `0x06` | ProcessAdvert | Process advertisement |
| `0x07` | Heartbeat | Ping/pong |
| `0x08` | JoinRequest | Cluster join request |
| `0x09` | JoinResponse | Cluster join response |
| `0x0A` | SyncDigest | Sync state digest |

**Maximum message size**: 16 MiB (`MAX_MESSAGE_SIZE`)

**Dedup** (`mesh_dedup.rs`): `DedupFilter` prevents processing duplicate messages (bloom filter based).

### Layer 3: Service Resolution

**Files**: `mesh_service.rs`, `mesh_discovery.rs`, `mesh_bootstrap.rs`, `mesh_mdns.rs`

- `DiscoveryCoordinator` manages multiple discovery backends
- `DiscoveryBackend` trait for pluggable discovery (mDNS, bootstrap, Kademlia, PEX)
- `ServiceResolutionCache` caches remote service endpoints
- `ServiceResolveRequest`/`ServiceResolveResponse` for mesh-wide service lookup

### Layer 4: Sync Protocols

**Files**: `mesh_chain.rs`, `mesh_tree.rs`, `mesh_heartbeat.rs`

- Chain sync: `ChainSyncRequest`/`ChainSyncResponse`, `SyncStateDigest`
- Tree sync: `TreeSyncRequest`/`TreeSyncResponse`, `MerkleProof`, `TreeNodeDiff`
- Heartbeat: `HeartbeatTracker`, `PingRequest`/`PingResponse`, failure detection

### Layer 5: Distributed State

**Files**: `mesh_process.rs`, `mesh_service_adv.rs`, `mesh_kad.rs`

- `DistributedProcessTable` with CRDT gossip state
- `ConsistentHashRing` for process placement
- `ClusterServiceRegistry` for service advertisements
- `KademliaTable` (DHT) for distributed key-value storage

## How to Add a New Mesh Module

### 1. Create the file

```bash
touch crates/clawft-kernel/src/mesh_myfeature.rs
```

### 2. Add to lib.rs

In the mesh section (after the existing `#[cfg(feature = "mesh")]` block):

```rust
#[cfg(feature = "mesh")]
pub mod mesh_myfeature;
```

### 3. Add re-exports

```rust
#[cfg(feature = "mesh")]
pub use mesh_myfeature::{MyType, MyOtherType};
```

### 4. If adding a new frame type

In `mesh_framing.rs`:

1. Add a variant to `FrameType`:
   ```rust
   MyNewFrame = 0x0B,
   ```

2. Add the match arm in `FrameType::from_byte()`:
   ```rust
   0x0B => Some(Self::MyNewFrame),
   ```

3. Update `WeftHandshake.supported_sync_streams` to include the new discriminant.

### 5. If adding a new transport

Implement all three traits: `MeshTransport`, `MeshStream`, `TransportListener`.

Follow the pattern in `mesh_tcp.rs`:
- `struct MyTransport;`
- `struct MyStream { ... }`
- `struct MyListener { ... }`

## Testing Patterns

### Serde roundtrip (required for all Serialize types)

```rust
#[test]
fn my_type_serde_roundtrip() {
    let original = MyType { field: "value".into() };
    let json = serde_json::to_string(&original).unwrap();
    let restored: MyType = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.field, original.field);
}
```

### Error display tests

```rust
#[test]
fn mesh_error_display_my_variant() {
    let err = MeshError::MyVariant("details".into());
    assert_eq!(err.to_string(), "expected: details");
}
```

### In-process transport testing

For testing without real network connections, create in-memory streams:

```rust
/// In-memory stream pair for testing (no TCP/WS needed).
fn in_memory_stream_pair() -> (impl MeshStream, impl MeshStream) {
    // Use tokio::io::duplex or channel-based implementation
}
```

For TCP/WS integration tests, bind to `127.0.0.1:0` (OS assigns port):

```rust
#[tokio::test]
async fn tcp_roundtrip() {
    let transport = TcpTransport;
    let mut listener = transport.listen("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let connect_handle = tokio::spawn(async move {
        transport.connect(&addr.to_string()).await.unwrap()
    });

    let (mut server_stream, _) = listener.accept().await.unwrap();
    let mut client_stream = connect_handle.await.unwrap();

    client_stream.send(b"hello").await.unwrap();
    let received = server_stream.recv().await.unwrap();
    assert_eq!(received, b"hello");
}
```

### Frame encode/decode roundtrip

```rust
#[test]
fn frame_roundtrip() {
    let frame = MeshFrame {
        frame_type: FrameType::MyNewFrame,
        payload: b"test payload".to_vec(),
    };
    let encoded = frame.encode().unwrap();
    let decoded = MeshFrame::decode_from_bytes(&encoded).unwrap();
    assert_eq!(decoded.frame_type, frame.frame_type);
    assert_eq!(decoded.payload, frame.payload);
}
```

## Build Commands

```bash
# Check with mesh feature
scripts/build.sh check  # mesh is included in default feature set

# Test mesh modules
scripts/build.sh test

# Clippy lint
scripts/build.sh clippy
```

## Related Files

- **Transport traits**: `crates/clawft-kernel/src/mesh.rs`
- **All mesh modules**: `crates/clawft-kernel/src/mesh_*.rs`
- **K6 SPARC plan**: `.planning/sparc/weftos/00-orchestrator.md` (K6 section)
- **Gap-filling K6**: `.planning/sparc/weftos/08-os-gap-filling.md` (K6 Gaps section)
- **Kernel lib.rs**: `crates/clawft-kernel/src/lib.rs` (mesh re-exports)
