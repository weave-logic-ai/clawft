# WEFT-120 result — Wire ClusterService to mesh peer discovery

**Branch:** `wave0h/weft-120-cluster-mesh`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bed-7c60-8226-843dca21db18`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

`ClusterService` was pull-based: `sync_to_membership()` ran once at service
start. Mesh peer discovery / connections lived separately in
`MeshRuntime` (`add_peer` / `disconnect_peer` / heartbeat). Cluster
membership did not reflect live mesh state without manual sync.

Audit: `.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` row #31  
Prior: `mesh-boot-integration.md` Future Work  
Plane: WEFT-120 (ws02-kernel).

## What shipped

| Surface | Change |
|---------|--------|
| `MeshPeerEvent` | Join / Left / Alive / Suspect / Unreachable / Recovered |
| `MeshPeerEventBus` | Tokio broadcast bus (capacity 256) |
| `MeshRuntime` | Emits events on add / disconnect / heartbeat / remove_dead_peers; `subscribe_peer_events()` |
| `ClusterMembership` | `apply_mesh_peer_event` (atomic upsert/state/remove, mesh-trusted, no rate limit); `spawn_mesh_peer_listener` |
| `ClusterService` | `apply_mesh_peer_event` / `_async` (membership + ruvector manager); `spawn_mesh_peer_listener` |
| Boot | When mesh live + cluster feature: ClusterService subscribes to mesh bus; mesh-only fallback wires membership |

## Acceptance

| Criterion | Status |
|-----------|--------|
| ClusterService subscribes to mesh peer-event stream | Done — boot + `spawn_mesh_peer_listener` |
| Membership updates atomically | Done — single-event `apply_mesh_peer_event` + one persist |
| Tests for join / leave / partition recovery | Done — unit + async bus + ClusterService stream tests |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/mesh_discovery.rs` | `MeshPeerEvent`, `MeshPeerEventBus`, tests |
| `crates/clawft-kernel/src/mesh_runtime.rs` | Emit bus events; subscribe API; e2e membership test |
| `crates/clawft-kernel/src/cluster.rs` | Membership apply + ClusterService subscribe/apply |
| `crates/clawft-kernel/src/boot.rs` | Wire ClusterService ↔ mesh bus at phase 7 |
| `crates/clawft-kernel/src/lib.rs` | Re-export `MeshPeerEvent`, `MeshPeerEventBus` |
| `docs/plans/wave-0h-WEFT-120-result.md` | This report |

## Verification

```bash
cargo test -p clawft-kernel --lib mesh_peer_events
# 8 passed (join, leave, partition recovery, suspect/alive,
#   upsert, membership listener, ClusterService async apply,
#   ClusterService stream subscribe)

cargo test -p clawft-kernel --lib -- \
  add_peer_emits_joined disconnect_peer_emits_left \
  re_add_peer_emits_recovered remove_dead_peers_emits_unreachable \
  mesh_runtime_events_drive_cluster peer_event_bus mesh_peer_event
# 16 passed

scripts/build.sh check
# ok
```

## Design notes

- **Push over pull:** mesh is the authority for connect/health; ClusterService
  still has `sync_to_membership()` for bootstrap/fallback only.
- **Atomic apply:** one event → one consistent membership mutation (upsert
  Active, state transition, or remove). Mesh path bypasses operator
  `add_peer` rate limit (session already authenticated).
- **Partition recovery:** `Unreachable` marks membership; reconnect
  re-registers the same `node_id` → `Recovered` → Active; manager
  `add_node` / `remove_node` stay best-effort in the async path.
- **Lag:** broadcast bus; lagged consumers skip old events and log a warning.

## Follow-ups

- Optionally emit `Suspect` from mesh heartbeat tick (today Alive/Recovered
  on `record_heartbeat`, Unreachable on `remove_dead_peers`).
- Consolidate seed-peer address into `Joined.address` when discovery map is set
  before `add_peer` (boot seed path registers address after connect).
- WEFT-119 (Mesh as SystemService) can own stop-time bus teardown.
