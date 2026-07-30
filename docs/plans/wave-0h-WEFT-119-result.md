# WEFT-119 result — Make Mesh a SystemService with start/stop/health_check

**Branch:** `wave0h/weft-119-mesh-system-service`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bed-7c60-8226-842f859e5ffd`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

Mesh transport boots in phase 5d (listener + seed peers) but was **not** a
`SystemService`. Operators could not inspect or restart mesh via the standard
service registry surface (`start` / `stop` / `health_check` / `health_all`).

Audit: `.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` Task List #30  
Plane: WEFT-119 (ws02-kernel).

## What shipped

| Surface | Change |
|---------|--------|
| `MeshService` | New `SystemService` adapter wrapping `Arc<MeshRuntime>` (`name = "mesh"`, `ServiceType::Core`) |
| Lifecycle | `start` (idempotent live flag), `stop` (disconnect all peers + clear live), `health_check` (Unhealthy if not started; peer HB → Degraded/Unhealthy) |
| Probes | `os-patterns` liveness/readiness reflect started flag |
| `MeshRuntime` | `disconnect_all_peers()` for clean stop/shutdown |
| Boot (5d) | When `mesh.enabled`, register `MeshService` before phase-8 `start_all()`; boot log line for registration |

## Acceptance

| Criterion | Status |
|-----------|--------|
| MeshService implements SystemService | Done — `mesh_system_service::MeshService` |
| Start/stop/health_check exposed via weaver kernel services | Done — registered on `ServiceRegistry` as `"mesh"` when mesh enabled; participates in `start_all` / `stop_all` / `health_all` |
| Tests for lifecycle transitions | Done — unit + boot integration tests |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/mesh_system_service.rs` | **New** — MeshService + lifecycle/registry/HB health tests |
| `crates/clawft-kernel/src/mesh_runtime.rs` | `disconnect_all_peers()` |
| `crates/clawft-kernel/src/boot.rs` | Register MeshService in phase 5d; boot test with mesh.enabled |
| `crates/clawft-kernel/src/lib.rs` | `mod mesh_system_service`; re-export `MeshService` |
| `docs/plans/wave-0h-WEFT-119-result.md` | This report |

## Verification

```bash
cargo test -p clawft-kernel --lib mesh_system_service
# 5 passed:
#   endpoints_accessors
#   lifecycle_start_stop_health
#   registry_start_all_stop_all
#   health_unhealthy_when_all_discovery_peers_dead
#   boot_registers_mesh_system_service

scripts/build.sh check
# ok (clawft-kernel compiles)

scripts/build.sh test clawft-kernel
# 2110 passed, 1 failed (pre-existing golden config snapshot drift:
#   config_snapshots::default_config_snapshot — skills.autogen; unrelated)
```

**Commit:** tip of `wave0h/weft-119-mesh-system-service` (`git log -1 --oneline`)
```

### Operator surface

```text
# When kernel config has mesh.enabled = true:
#   services list includes "mesh" (Core)
#   health_all includes mesh → Healthy after boot start_all
#   shutdown stop_all → mesh Unhealthy("mesh service not started"), peers disconnected
```

## Notes

- Listen/accept loops remain spawned by boot (phase 5d). `MeshService` owns the
  **registry lifecycle**, not re-binding the socket on every start. A future
  ticket can move accept into `start()` with a cancel token if full restart is
  required without kernel reboot.
- Service name `"mesh"` is distinct from `mesh_service.rs` (service *resolution*
  / DHT lookup).
