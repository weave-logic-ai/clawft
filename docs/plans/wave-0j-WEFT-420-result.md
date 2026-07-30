# WEFT-420 result — clawft-substrate network/bluetooth Linux-only + feature gates

**Ticket:** WEFT-420  
**Branch:** `wave0j/weft-420-net-bt-platform`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-420 (wave-0j)

## Problem

`network` / `bluetooth` adapters read Linux `/sys/class/*`. On non-Linux
they previously returned plain `absent` / `present: false` with no way
to tell "no radio" from "this OS cannot probe sysfs." macOS / Windows
variants were unscheduled. Audit: document Linux-only **or** port.

## Decision

**Honest docs + OS feature gate** (not full CoreWLAN / WinRT ports).

macOS / Windows ports remain unscheduled; non-Linux hosts keep a uniform
open/subscribe surface with an explicit `sysfs-unavailable` cause.

## What shipped

### 1. Shared platform probe — `clawft-substrate::sysfs`

| Symbol | Role |
|--------|------|
| `linux_sysfs_native()` | `cfg!(target_os = "linux")` |
| `CAUSE_SYSFS_UNAVAILABLE` | Wire-stable `"sysfs-unavailable"` |
| `PLATFORM_LINUX_SYSFS` / `PLATFORM_UNSUPPORTED` | Payload `platform` tags |
| `sysfs_unavailable_health_delta(...)` | `AdapterHealthEvent::Error` on `substrate/meta/adapter/<id>/health` |
| `unsupported_payload_fields()` | Shared `platform` + `cause` map |

Gate is **target OS**, not a Cargo feature (Cargo cannot express
"Linux only").

### 2. Network adapter

- `NetworkAdapter::new()` sets `platform_supported` from `linux_sysfs_native()`.
- `with_roots` → supported (tests inject fake trees on any host).
- `with_roots_and_platform(..., false)` → force unsupported path in tests.
- Non-Linux / forced-unsupported:
  1. First delta: health `error` with sysfs reason.
  2. Topic ticks: `state: "absent"` / `present: false` + `cause` + `platform`.
- Linux missing hardware: plain `absent` **without** `cause` (unchanged).

### 3. Bluetooth adapter

Same pattern as network for `substrate/bluetooth`
(`present`/`enabled` false + cause; health error).

### 4. Docs

- Module rustdoc tables on `network` / `bluetooth` / `sysfs` / `lib.rs`.
- `docs/weftos/FEATURE_GATES.md` — new **Platform constraints** section.
- `native_live.rs` comments note Linux-only + WEFT-420.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Implement macOS / Windows **or** document Linux-only | **Done** — document + OS gate (ports unscheduled) |
| Docs clearly state Linux-only + behaviour on other OSes | **Done** — rustdoc + FEATURE_GATES |
| Surface non-Linux fallback in adapter-health (sysfs cause) | **Done** — `event: error`, reason contains `sysfs-unavailable` |
| Tests | **Done** — unit + async open/poll paths |

## Tests / build

```bash
scripts/build.sh test clawft-substrate
# 177 passed (incl. sysfs + network/bluetooth unsupported paths)
```

Key tests:

- `sysfs::tests::*`
- `network::tests::unsupported_platform_emits_health_error_then_absent_payload`
- `network::tests::sample_unsupported_*`
- `bluetooth::tests::unsupported_platform_emits_health_error_then_disabled_payload`
- `bluetooth::tests::sample_unsupported_carries_sysfs_cause`
- `*_new_respects_compile_time_platform_gate`

## Files

| Path | Change |
|------|--------|
| `crates/clawft-substrate/src/sysfs.rs` | **new** shared probe |
| `crates/clawft-substrate/src/network.rs` | platform gate + health + tests |
| `crates/clawft-substrate/src/bluetooth.rs` | platform gate + health + tests |
| `crates/clawft-substrate/src/lib.rs` | `sysfs` mod + Linux-only docs |
| `crates/clawft-gui-egui/src/live/native_live.rs` | comment only |
| `docs/weftos/FEATURE_GATES.md` | platform constraints section |
| `docs/plans/wave-0j-WEFT-420-result.md` | this file |

## Follow-ups (out of scope)

- CoreWLAN / IOBluetooth (macOS) and WinRT radio APIs.
- Same `sysfs` gate on `rfkill` adapter (related; not in ticket AC).
- Plane work item close (In Progress → Done) with commit SHA after merge.
