# Mesh Transport Boot Integration

**Date**: 2026-04-17
**Version**: v0.6.13

## Problem

The K6 mesh transport layer was fully implemented (3,543 lines, 14 files,
133 tests) but never instantiated during kernel boot. `MeshRuntime::new()`
was only called in test modules. The `[mesh] enabled = true` config field
was parsed but ignored — a dead field.

## Solution

Added boot phase 5d between A2A router setup (5a) and cluster membership (6):

1. Read `kernel.mesh` config (MeshConfig struct)
2. Create MeshRuntime (with optional Kademlia discovery)
3. Wire to A2A router via `set_local_router()`
4. Spawn transport listener (TCP default, WebSocket available)
5. Connect to seed peers in background tasks

## Design Decisions

### Transport default: TCP

TCP was chosen as the default because:
- Already fully implemented and tested
- Works everywhere without additional dependencies
- QUIC (via quinn) planned as future upgrade but adds ~5MB to binary
- WebSocket available for browser-to-node communication

### Boot ordering: after A2A, before cluster

Mesh must start before cluster because:
- Cluster service needs mesh to discover and communicate with peers
- A2A router must exist before mesh can inject incoming messages
- Mesh listener should be accepting connections before service_registry.start_all()

### Config location: `[kernel.mesh]` not `[mesh]`

Moved from top-level `[mesh]` to `[kernel.mesh]` because:
- Mesh is a kernel subsystem, not a standalone feature
- Consistent with `[kernel.chain]`, `[kernel.vector]`, etc.
- Old `[mesh]` config was never actually consumed

### Feature gate

The boot integration code is gated on `#[cfg(all(feature = "native", feature = "mesh"))]`.
The `mesh` feature was added to kernel defaults so it compiles normally.
The mesh listener only starts when `config.mesh.enabled = true` — the
feature gate controls compilation, the config controls runtime activation.

## Testing

- Boot with `mesh.enabled = false`: mesh phase logs "Mesh transport disabled", no listener
- Boot with `mesh.enabled = true`: logs "Mesh transport started (tcp on 0.0.0.0:9470, 0 seed peers)"
- Boot with seed peers: connects to each in background, logs success/failure per peer
- Existing 133 mesh tests continue to pass (unit tests, not integration)

## Future Work

- QUIC transport (quinn + snow for Noise Protocol encryption)
- Mesh as a SystemService (proper start/stop/health_check lifecycle)
- Cluster service wired to mesh peer discovery
- Mesh health metrics in observability subsystem
