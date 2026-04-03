# ADR-026: QUIC as Primary Mesh Transport, WebSocket as Browser Fallback

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K5 Symposium Mesh Architecture Panel (D6, M4), Security Panel
**Depends-On**: ADR-024 (Noise Protocol Encryption)

## Context

WeftOS K6 requires a transport layer for its mesh network that supports Cloud, Edge, Browser, and WASI node types. The transport must integrate with the Noise encryption layer (ADR-024) and the `MeshTransport` trait abstraction. Three approaches were evaluated: TCP with custom framing, QUIC via `quinn`, and WebSocket via `tokio-tungstenite`. Each platform type has different network capabilities -- Cloud and Edge nodes have unrestricted UDP/TCP access, Browser nodes are limited to WebSocket and WebRTC, and WASI nodes typically only support WebSocket via host-provided networking.

## Decision

Cloud and Edge nodes use QUIC via the `quinn 0.11` crate as the primary mesh transport. Browser and WASI nodes use WebSocket via `tokio-tungstenite` as the fallback transport. Both transports implement the `MeshTransport` trait:

```rust
#[async_trait]
pub trait MeshTransport: Send + Sync + 'static {
    fn name(&self) -> &str;
    async fn listen(&self, addr: &str) -> Result<TransportListener>;
    async fn connect(&self, addr: &str) -> Result<MeshStream>;
    fn supports(&self, addr: &str) -> bool;
}
```

### Platform-Transport Matrix

| Platform | Primary Transport | Fallback | Address Format |
|----------|------------------|----------|----------------|
| CloudNative | QUIC (`quinn`) | TCP | `quic://host:port` |
| Edge | QUIC (`quinn`) | TCP, BLE | `quic://host:port` |
| Browser | WebSocket (`tokio-tungstenite`) | WebRTC | `ws://host:port/mesh` |
| WASI | WebSocket (`tokio-tungstenite`) | -- | `ws://host:port/mesh` |
| Embedded | BLE, LoRa | TCP | `ble://device-id` (future) |

WebRTC (browser-to-browser), BLE (embedded/IoT), and LoRa (remote/IoT) are future transport implementations behind the same `MeshTransport` trait. They are not part of K6.

QUIC provides native stream multiplexing (used by D14 for CMVG cognitive sync over the same connection), built-in congestion control, and connection migration (allowing mobile/edge nodes to switch networks without re-handshaking). The Noise encryption layer (ADR-024) wraps the QUIC/WebSocket byte streams; QUIC's built-in TLS is not used because WeftOS manages its own key identity and trust model.

### Message Framing

Messages over the encrypted stream use length-prefix encoding:

```
+--------+--------+------------------+
| LEN(4) | TYPE(1)| PAYLOAD (LEN-1)  |
+--------+--------+------------------+
```

Type bytes: `0x01` KernelMessage (JSON), `0x02` KernelMessage (RVF segment), `0x03` ChainSync, `0x04` TreeSync, `0x05` Heartbeat, `0x06` Governance rule distribution, `0xFF` Protocol error/close. Maximum message size: 16 MiB.

All transports live behind the `mesh` feature gate in `Cargo.toml`:

```toml
[features]
mesh = ["quinn", "snow", "x25519-dalek"]
mesh-discovery = ["mesh", "libp2p-kad", "libp2p-mdns"]
mesh-full = ["mesh", "mesh-discovery"]
```

Default builds include zero networking code, preserving single-node zero-dependency builds for development, testing, embedded, and WASI targets.

## Consequences

### Positive
- QUIC provides multiplexed streams over a single connection, enabling D14 (CMVG cognitive sync) without separate connections per data type
- Built-in congestion control eliminates the need for custom flow control implementation
- Connection migration lets Edge nodes switch networks without dropping mesh membership
- Feature-gated networking means single-node builds carry zero transport dependencies
- The `MeshTransport` trait is transport-agnostic: adding WebRTC, BLE, or LoRa later requires only a new trait implementation, no changes to the mesh protocol layer

### Negative
- QUIC requires UDP, which some corporate firewalls and restrictive network environments block; deployment guidance must document UDP port requirements and TCP fallback configuration
- `quinn 0.11` adds a significant dependency subtree (rustls, ring, etc.) to networked builds
- QUIC's built-in TLS is unused (Noise handles encryption), meaning WeftOS pays the dependency cost of `quinn`'s TLS stack without using it; future optimization could use `quinn` with a custom crypto provider to eliminate this
- Browser nodes cannot use QUIC directly (no browser API for raw QUIC), creating a two-tier transport architecture where Browser/WASI nodes have different performance characteristics than Cloud/Edge nodes

### Neutral
- QUIC stream prioritization (Chain > Tree > IPC > Cognitive > Impulse) is configured at the application layer via `SyncStreamType` discriminators in RVF wire frames (D15)
- The same Noise-encrypted connection carries control messages, chain replication, tree sync, IPC forwarding, and (in K7) cognitive structure sync
- WebSocket transport inherits the same Noise encryption, message framing, and protocol semantics as QUIC -- the mesh protocol is transport-agnostic by design
