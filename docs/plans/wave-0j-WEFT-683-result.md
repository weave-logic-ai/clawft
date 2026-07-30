# WEFT-683 result — ADR-031 drift: RVF deferred + JSON explicit default

**Ticket:** WEFT-683  
**Branch:** `wave0j/weft-683-rvf-encoding`  
**Wave:** 0j  
**Date:** 2026-07-30  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4cf-e146-7d52-a3e6-24bc30edf109`  
**Choice:** **Option A** (amend ADR + feature-gate RVF API) — per ticket notes (mesh throughput not a live problem)

## Problem

ADR-031 (Accepted) declared dual JSON/RVF encoding for `KernelMessage`, with RVF as the **production default**. Only the JSON half was built (`mesh_ipc.rs` → unconditional `serde_json::to_vec`). WEFT-110 correctly closed the JSON path; the tracker still implied both encodings existed. Living-ADR rule: plan and code must agree.

## Decision

**Option A** — document reality and reserve RVF behind a feature gate. Do **not** flip production to an unimplemented encoding.

| Layer | Shipped | Deferred |
|-------|---------|----------|
| `FrameType` (message kind) | Full enum in `mesh_framing.rs` | — |
| IPC payload encoding | JSON (`MeshIpcEncoding::Json`) | RVF (`MeshIpcEncoding::Rvf` under `mesh-rvf`) |

Original ADR type-byte table mixed **message kind** with **encoding**; code never matched. WEFT-683 separates the two layers in ADR + code comments.

## What shipped

### 1. ADR-031 amendment

- `Updated: 2026-07-30 (WEFT-683)`
- Implementation status table (JSON shipped; RVF not built)
- Framing vs encoding clarification (aligned with `FrameType`)
- Deferral rationale
- **Revisit triggers** for Option B (measured JSON cost ~50 µs p99, ≥10k msg/s hop target, zero-copy relay need, or cheap reuse of weave RVF codecs)
- Option B acceptance checklist retained for when triggers fire

### 2. Feature-gated RVF surface (no production flip)

| Item | Detail |
|------|--------|
| Cargo feature | `mesh-rvf = ["mesh"]` on `clawft-kernel` (off by default) |
| API | `MeshIpcEncoding::{Json, Rvf}`, `DEFAULT_MESH_IPC_ENCODING = Json` |
| Methods | `to_bytes_with_encoding` / `from_bytes_with_encoding` |
| RVF behavior today | `MeshIpcError::UnsupportedEncoding { encoding: "rvf" }` |
| `to_bytes` / `from_bytes` | Unchanged wire: still JSON via default encoding |

### 3. Docs / comments

- `mesh_framing.rs` + `mesh_ipc.rs` module docs: kind vs encoding
- `clawft-kernel` lib feature list documents `mesh-rvf`
- ADR README row notes JSON shipped / RVF deferred

## Acceptance mapping

| AC (Option A) | Status |
|---------------|--------|
| Update ADR-031 with Updated date + RVF deferred + reason | Done |
| State what would trigger building RVF (revisitable) | Done — four triggers in ADR |
| Feature-gate RVF path so ADR matches reality | Done — `mesh-rvf`, JSON default |

Option B items (type-byte dispatch on wire, real RVF codec, production flip, bench) intentionally **not** done — deferred until triggers fire.

## Files changed

| File | Change |
|------|--------|
| `docs/adr/adr-031-rvf-wire-mesh-format.md` | Implementation status, deferral, triggers, framing/encoding split |
| `docs/adr/README.md` | ADR-031 row note |
| `crates/clawft-kernel/Cargo.toml` | `mesh-rvf` feature |
| `crates/clawft-kernel/src/mesh_ipc.rs` | `MeshIpcEncoding`, encoding-aware APIs, tests, `UnsupportedEncoding` |
| `crates/clawft-kernel/src/mesh_framing.rs` | Kind-vs-encoding docs |
| `crates/clawft-kernel/src/lib.rs` | Feature flag docs |
| `docs/plans/wave-0j-WEFT-683-result.md` | This result |

## How to test

```bash
# Default (JSON only)
scripts/build.sh test clawft-kernel
# or focused:
cargo test -p clawft-kernel --lib mesh_ipc::
cargo test -p clawft-kernel --lib mesh_framing::

# Experimental RVF API surface (UnsupportedEncoding)
cargo test -p clawft-kernel --features mesh-rvf --lib mesh_ipc::

# ADR claims
grep -n 'Updated\|Implementation status\|mesh-rvf\|DEFAULT_MESH_IPC' \
  docs/adr/adr-031-rvf-wire-mesh-format.md \
  crates/clawft-kernel/src/mesh_ipc.rs \
  crates/clawft-kernel/Cargo.toml
```

### Results (this worktree)

| Check | Result |
|-------|--------|
| `scripts/build.sh test clawft-kernel` | **2157 passed**, 2 skipped |
| `cargo test -p clawft-kernel --features mesh-rvf --lib mesh_ipc::` | **19 passed** (includes `rvf_encoding_returns_unsupported`) |

## Follow-ups

1. **Option B** when ADR-031 revisit triggers fire (new ticket; do not reopen WEFT-110).
2. Plane: mark WEFT-683 Done with this commit SHA + result path.
3. Optional: encoding prefix byte on the wire when dual encoding is negotiated (peers of mixed versions).

## Commit

- **Implementation SHA:** `f31d505c02f6413405a65ddc60ceb81c1dac7177` (`f31d505c`)
- **Branch tip:** `git rev-parse wave0j/weft-683-rvf-encoding` (may include docs SHA note)
- **Branch:** `wave0j/weft-683-rvf-encoding`
- **Message:** `WEFT-683: ADR-031 match reality — JSON default, mesh-rvf gate`
- **No push** (per wave instructions)
