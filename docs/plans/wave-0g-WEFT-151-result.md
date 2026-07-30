# WEFT-151 result — Audit mesh_log / mesh_dedup / mesh_listener / mesh_bootstrap callers

**Branch:** `wave0g/weft-151-mesh-audit`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb489-7de1-7a40-b2a1-d6c4f399ff91`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Plane id:** `bb9aeb52-5115-44cd-8bfe-e06207169b30`

## Problem

Audit gap (`.planning/reviews/0.7.0-release-gate/02-kernel-governance.md` row
#62 / plane inventory): four K6 mesh modules are implemented and re-exported
but appear to have no daemon RPC, CLI, or observability surface. Either they
have a hidden caller, or they are orphans.

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| End-to-end caller traced for each module | **Done** — full graph below |
| If wired: documented and tested | N/A — none of the four are production-wired |
| If orphaned: scheduled for wiring or marked deprecated | **Done** — all four **orphaned**; docs-scheduled (no rustc `#[deprecated]`) |

## Method

Workspace-wide search (Rust + docs) for:

- Modules: `mesh_log`, `mesh_dedup`, `mesh_listener`, `mesh_bootstrap`
- Types: `LogAggregator`, `RemoteLogEntry`, `MeshLogQuery`, `DedupFilter`,
  `MeshConnectionPool`, `JoinRequest`, `JoinResponse` (mesh_listener),
  `BootstrapDiscovery`, `PeerExchangeDiscovery`, `PeerInfo` (mesh_listener)

Sources inspected: `crates/**/*.rs`, `boot.rs` mesh path, `mesh_runtime.rs`,
`a2a.rs`, CLI/weave/services, agent mesh skill, ADRs, K6 notes.

## Caller graph (production vs library)

### Shared pattern

All four modules:

1. Live under `#[cfg(feature = "mesh")]` in `clawft-kernel`
2. Are `pub mod` + `pub use` from `lib.rs` (and several types from `weftos`)
3. Have **in-module unit tests only** as executable callers
4. Are **not** referenced from `boot.rs`, `MeshRuntime`, daemon RPC, or CLI

**Live mesh path today** (for contrast):

```
boot.rs (mesh enabled)
  ├─ MeshRuntime::new / with_discovery
  ├─ a2a_router.set_mesh_runtime
  ├─ TcpTransport | WsTransport listen → accept loop (inline)
  │     NoiseChannel / PassthroughChannel
  │     MeshRuntime::handle_incoming_from / send_to_peer
  └─ seed_peers: for addr in seed_peers { transport.connect; runtime.add_peer }
```

That path never constructs the four audited types.

---

### 1. `mesh_log` — **ORPHAN**

| Surface | Present? |
|---------|----------|
| Module | `crates/clawft-kernel/src/mesh_log.rs` (~325 LOC) |
| Public API | `RemoteLogEntry`, `LogQuery`, `LogAggregator` |
| Re-export | `lib.rs` → `LogAggregator`, `MeshLogQuery`, `RemoteLogEntry` |
| Unit tests | 11 (local/remote add, query filters, serde) |
| Production callers | **None** |
| CLI / RPC / observability | **None** |

**Intended role (K6-G2):** cross-node log aggregation for mesh ops.

**Disposition:** Keep library. **Schedule** wire-up to mesh admin /
observability RPC when peer mesh-status exists (related: WEFT-117). Not deleted.

---

### 2. `mesh_dedup` — **ORPHAN**

| Surface | Present? |
|---------|----------|
| Module | `crates/clawft-kernel/src/mesh_dedup.rs` (~165 LOC) |
| Public API | `DedupFilter` (`check_and_insert`, `is_duplicate`, `default_mesh`) |
| Re-export | `lib.rs`, `weftos` |
| Unit tests | 8 (new/dup, TTL, capacity, defaults) |
| Production callers | **None** |
| Docs that *claim* use | ADR-039 (SWIM), K6 notes (MeshAdapter plan) — aspirational |

**Intended role (K6.3):** drop duplicate mesh envelopes (multi-path).

**Disposition:** Keep library. **Schedule** `DedupFilter::default_mesh()` on
`MeshRuntime` inbound path (envelope / message id) before A2A inject. ADR-039
remains design intent until that lands.

---

### 3. `mesh_listener` — **ORPHAN** (name vs reality)

| Surface | Present? |
|---------|----------|
| Module | `crates/clawft-kernel/src/mesh_listener.rs` (~389 LOC) |
| Public API | `MeshConnectionPool`, `MeshPeerConnection`, `JoinRequest`,
  `JoinResponse`, `PeerInfo` |
| Re-export | `lib.rs`, `weftos` (`MeshConnectionPool`) |
| Unit tests | 12 (pool register/cap, serde join types) |
| Production callers | **None** |
| Related (not callers) | `mesh_framing::FrameType::{JoinRequest,JoinResponse}` discriminants
  only; `assessment::PeerInfo` is a **different** type; live accept loop is
  **inline in `boot.rs`**; live peers are `MeshRuntime::peers: DashMap<…, PeerConnection>` |

**Intended role (K6.1):** shared connection pool + cluster join negotiation.

**Disposition:** Keep library; do not delete (would lose join serde + pool
caps). **Schedule** consolidate with `MeshRuntime` peer map **or** wrap pool
around runtime peers; serialize join frames on first connect.

---

### 4. `mesh_bootstrap` — **ORPHAN**

| Surface | Present? |
|---------|----------|
| Module | `crates/clawft-kernel/src/mesh_bootstrap.rs` (~223 LOC) |
| Public API | `BootstrapDiscovery`, `PeerExchangeDiscovery` (`DiscoveryBackend`) |
| Re-export | `lib.rs`, `weftos` |
| Unit tests | 7 (start/poll/stop, seeds, PEX) |
| Production callers | **None** |
| Related (not callers) | `boot.rs` uses `mesh_config.seed_peers` with direct
  `transport.connect` — **parallel reimplementation**, not this module.
  `DiscoveryCoordinator` is also only unit-tested (no daemon registration). |

**Intended role (K6.2):** static seed + peer-exchange discovery backends.

**Disposition:** Keep library. **Schedule** boot mesh init:

```text
DiscoveryCoordinator::new()
  .add_backend(Box::new(BootstrapDiscovery::new(seed_peers)))
  // later: PeerExchangeDiscovery after handshake
  // feed discovered peers into connect loop / MeshRuntime
```

---

## Summary table

| Module | LOC (approx) | Unit tests | Wired? | Disposition |
|--------|--------------|------------|--------|-------------|
| `mesh_log` | 325 | 11 | No | Schedule observability RPC (0.8.x+) |
| `mesh_dedup` | 165 | 8 | No | Schedule `MeshRuntime` inbound dedup (0.8.x) |
| `mesh_listener` | 389 | 12 | No | Schedule pool/join ↔ runtime consolidation (0.8.x) |
| `mesh_bootstrap` | 223 | 7 | No | Schedule `DiscoveryCoordinator` in boot (0.8.x) |

**Hard deprecation:** Not applied. Precedent WEFT-671: rustc `#[deprecated]`
fires under clippy deny-warnings; disposition is **documentation-enforced**
until wiring or deliberate delete.

## What shipped

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/mesh_log.rs` | Module rustdoc: WEFT-151 orphan + wiring schedule |
| `crates/clawft-kernel/src/mesh_dedup.rs` | Same |
| `crates/clawft-kernel/src/mesh_listener.rs` | Same |
| `crates/clawft-kernel/src/mesh_bootstrap.rs` | Same |
| `crates/clawft-kernel/src/lib.rs` | Mesh section comment pointing at WEFT-151 |
| `agents/weftos-mesh/MESH.md` | Layer diagram + orphan callouts |
| `docs/weftos/k6-development-notes.md` | Dependency graph + audit table |
| `docs/plans/wave-0g-WEFT-151-result.md` | This report |

No runtime behavior change. No API removals.

## Verification

```bash
# Module unit tests (mesh feature)
scripts/build.sh test clawft-kernel -- mesh_log mesh_dedup mesh_listener mesh_bootstrap
# or:
cargo test -p clawft-kernel --features mesh --lib \
  mesh_log:: mesh_dedup:: mesh_listener:: mesh_bootstrap::
```

## Follow-ups (suggested Plane items — not created here)

1. **Wire `DedupFilter` into `MeshRuntime` inbound** (small; safety win).
2. **Wire `BootstrapDiscovery` via `DiscoveryCoordinator` in `boot.rs`**
   (replace raw seed loop or feed it).
3. **Consolidate `MeshConnectionPool` with `MeshRuntime` peers** + join frames.
4. **`LogAggregator` admin RPC** once mesh-status CLI exists (WEFT-117).

## How to re-audit

```bash
# Should only hit module defs, tests, re-exports, and docs after this ticket:
rg -n 'LogAggregator|DedupFilter|MeshConnectionPool|BootstrapDiscovery' crates agents docs
```
