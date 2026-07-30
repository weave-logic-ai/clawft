# WEFT-117 result — Wire AssessmentTransport into daemon mesh + mesh-status CLI

**Branch:** `wave0g/weft-117-assessment-transport`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb489-7de2-72f2-89c6-2b84fefe2dfc`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30

## Problem

`AssessmentTransport` was fully unit-tested but never attached to the daemon
mesh event loop. Assessment results were not propagated over the mesh at
runtime; there was no CLI for mesh-assess peer state. Diff propagation of
**only changed findings** was not implemented on the wire.

Audit: `.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` row #28  
Plane: WEFT-117 (ws02-kernel).

## What shipped

| Surface | Change |
|---------|--------|
| Protocol | `AssessmentMessage::FindingDiff` — new/resolved findings + deltas only |
| `MeshCoordinator` | `build_finding_diff`; handles inbound `FindingDiff` peer state |
| `AssessmentService` | Shared `Arc<MeshCoordinator>`; `queue_mesh_propagation` prefers FindingDiff when a previous report exists |
| `AssessmentTransport` | `build_diff_frame`, `try_extract_payload` / `try_handle_raw`, `publish_report`, RequestReport → FullReport when published |
| `MeshRuntime` | Registers transport; demuxes AssessmentSync in `handle_incoming` / `handle_incoming_from`; `push_pending_assessment` / `assessment_gossip_tick` / `broadcast_raw` |
| Boot (5b½/5d) | When mesh enabled: shared node_id + coordinator; `AssessmentService::with_mesh_coordinator`; transport on runtime; 15s drain/gossip tick |
| CLI | `weft assess mesh-status` → daemon `assess.mesh.status` (table or `--json`) |
| Daemon RPC | Existing `assess.mesh.status` / `assess.mesh.gossip` now see a live coordinator when mesh is on |

## Acceptance

| Criterion | Status |
|-----------|--------|
| AssessmentTransport registered in daemon mesh event loop | Done — boot sets transport on `MeshRuntime`; accept loop demuxes via `handle_incoming_from` |
| `weft assess mesh-status` reports peer state | Done — CLI + offline fallback |
| Assessment diff propagation pushes only changed findings | Done — `FindingDiff` + `queue_mesh_propagation` |
| Integration test covers two-node propagation | Done — mock peers in `mesh_runtime::tests::assessment_transport_demux_and_diff_push_two_nodes` (+ TCP unit test retained) |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/assessment/mesh.rs` | FindingDiff message + handlers + builder |
| `crates/clawft-kernel/src/assessment/mod.rs` | Arc coordinator; `with_mesh_coordinator`; queue_mesh_propagation |
| `crates/clawft-kernel/src/mesh_assess.rs` | Diff frames, demux helpers, publish_report, tests |
| `crates/clawft-kernel/src/mesh_runtime.rs` | Transport slot, demux, broadcast helpers, integration test |
| `crates/clawft-kernel/src/boot.rs` | Mesh-aware assessment + transport wiring + tick loop |
| `crates/clawft-cli/src/commands/assess_cmd.rs` | `mesh-status` subcommand |
| `docs/plans/wave-0g-WEFT-117-result.md` | This report |

## Verification

```bash
cargo test -p clawft-kernel --lib mesh_assess
# 20 passed (incl. build_diff_frame_only_changed_findings,
#   queue_mesh_propagation_prefers_finding_diff,
#   tcp_assessment_sync_between_two_nodes)

cargo test -p clawft-kernel --lib assessment_transport_demux
# 1 passed — assessment_transport_demux_and_diff_push_two_nodes

scripts/build.sh test clawft-kernel
# 2114 passed, 1 failed (pre-existing golden config snapshot drift:
#   config_snapshots::default_config_snapshot — unrelated to WEFT-117)

cargo check -p clawft-cli -p clawft-weave
# ok
```

### How to exercise CLI

```bash
# Offline (no daemon)
weft assess mesh-status
weft assess mesh-status --json

# With daemon + mesh enabled in kernel config
weaver kernel start   # mesh.enabled = true
weft assess mesh-status
weft assess run --scope commit   # queues FindingDiff/gossip for tick push
```

## Follow-ups

- Seed-peer connection path is still send-only (pre-existing); bidirectional seed read would improve assessment inbound on outbound-initiated links.
- Optional: drain pending assessment immediately after `assess.run` RPC instead of waiting for the 15s tick.
- Golden snapshot `default_config` needs a separate refresh (unrelated config surface growth).
