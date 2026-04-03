# ADR-027: Selective libp2p Components over Full Framework

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K5 Symposium Mesh Architecture Panel (D1, M1)
**Depends-On**: ADR-024 (Noise Protocol), ADR-026 (QUIC Primary Transport)

## Context

WeftOS K6 needs peer discovery (finding nodes on the mesh) and distributed hash table (DHT) capabilities for service resolution and node lookup. Three approaches were evaluated:

1. **Full libp2p stack**: Provides transport, encryption, discovery, pub/sub, and more as an integrated framework. Used by Polkadot, Filecoin, IPFS, and Ethereum 2.0. However, it brings approximately 80 transitive dependencies and imposes an opinionated runtime model that conflicts with WeftOS's existing tokio-based architecture.

2. **iroh**: A QUIC-first networking library. Clean API, but QUIC-only -- it cannot support WebSocket, WebRTC, BLE, or other transports that WeftOS requires for Browser, WASI, and Embedded node types.

3. **Selective composition**: Use individual crates (`snow` for Noise, `quinn` for QUIC, `libp2p-kad` for Kademlia DHT, `libp2p-mdns` for LAN discovery) without importing the full libp2p framework or its runtime model.

WeftOS already uses `snow` for Noise encryption (ADR-024) and `quinn` for QUIC transport (ADR-026). The remaining gap is peer discovery: Kademlia DHT for WAN discovery and mDNS for zero-configuration LAN discovery.

## Decision

Use `snow 0.9` + `quinn 0.11` + `libp2p-kad 0.46` + `libp2p-mdns 0.46` as selectively composed components. Do not depend on the `libp2p` meta-crate or adopt the libp2p `Swarm` runtime model.

| Component | Crate | Role |
|-----------|-------|------|
| Encryption | `snow 0.9` | Transport-agnostic Noise Protocol handshakes |
| Primary transport | `quinn 0.11` | QUIC multiplexed streams for Cloud/Edge |
| Key exchange | `x25519-dalek 2.0` | Noise DH operations |
| DHT discovery | `libp2p-kad 0.46` | Kademlia routing table, peer lookup, service resolution |
| LAN discovery | `libp2p-mdns 0.46` | Zero-config peer finding on local networks |
| Wire format | `rvf-wire` (existing) | Zero-copy segment serialization |

The composition glue between these components is maintained in the `clawft-kernel` mesh modules. The `MeshTransport` trait (ADR-026) provides the byte-stream abstraction. The Noise layer (ADR-024) provides encryption. The discovery components feed into `ClusterMembership` in `cluster.rs`.

DHT keys are namespaced with the governance genesis hash (first 16 hex characters) for cross-cluster isolation:

```
svc:<genesis_hex[0..16]>:cache     -> service advertisement
node:<genesis_hex[0..16]>:abc123   -> node presence with transport addresses
```

Feature gates control the dependency tiers:

```toml
[features]
mesh = ["quinn", "snow", "x25519-dalek"]
mesh-discovery = ["mesh", "libp2p-kad", "libp2p-mdns"]
mesh-full = ["mesh", "mesh-discovery"]
```

A node can be built with `mesh` only (manual peer configuration, no DHT) or `mesh-full` (automatic discovery). The `mesh-discovery` feature is optional, not default.

## Consequences

### Positive
- Dependency budget is controlled: selective composition avoids the ~80 transitive dependencies of full libp2p while retaining the specific capabilities needed (Kademlia DHT, mDNS)
- Full control over the event loop: the existing tokio runtime drives all I/O, rather than adapting to libp2p's `Swarm` model
- Transport agnosticism is maintained: unlike iroh (QUIC-only), this composition supports any transport behind the `MeshTransport` trait
- Discovery is feature-gated and optional: embedded or single-node deployments carry zero discovery dependencies
- Kademlia and mDNS are battle-tested components from the libp2p ecosystem without the framework overhead

### Negative
- The composition glue between `snow`, `quinn`, `libp2p-kad`, and `libp2p-mdns` is custom code that must be maintained by the WeftOS team; full libp2p would handle this integration automatically
- `libp2p-kad` and `libp2p-mdns` may still pull in some libp2p-core types as transitive dependencies, requiring careful version pinning
- Upgrades to individual components (e.g., `quinn` 0.12, `snow` 1.0) may require updating the composition glue simultaneously
- NAT traversal (hole-punching, relay) is not provided by the selected components; if needed, it must be implemented separately or `libp2p-relay` must be added

### Neutral
- The full libp2p framework remains available as a future option if the selective composition becomes too costly to maintain; migration would involve replacing the custom glue with libp2p's `Swarm`
- iroh could be reconsidered if WeftOS drops non-QUIC transports in a future major version
- The `libp2p-kad` and `libp2p-mdns` versions (0.46) should track libp2p releases to benefit from upstream bug fixes and Kademlia protocol improvements
