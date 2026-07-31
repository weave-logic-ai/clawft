# WEFT-592 — BVH spatial-temporal index: plan review & phase decomposition

**Status**: Complete (decomposition)  
**Ticket**: WEFT-592  
**Date**: 2026-07-31  
**Branch**: `release/0.8-staging` (docs commit)  
**Cycle**: 0.8.x (phase children) / 0.9.x (consumer-heavy D–E if capacity slips)  
**Labels**: `ws02-kernel`, `ws17-research`, `gap`  
**Sources**:
- ADR-056 (Accepted) — `docs/adr/adr-056-bvh-spatial-index.md`
- Companion plan — `.planning/bvh-spatial-index/PLAN.md`
- Placeholder body — `.planning/bvh-spatial-index/plane-ticket-body.md`
- Tree state — `crates/clawft-bvh/` (partial Phase A land)

---

## 1. Purpose of this ticket

WEFT-592 is the **re-entry / decomposition gate** for BVH-on-RVF work, not the
implementation itself. Done means:

1. ADR-056 + PLAN.md re-validated against the 0.8.x tree.
2. Phases A–E filed as separate Plane work items with acceptance criteria.
3. Phase children carry the same workstream labels as the parent.

This document is the durable review artifact under `docs/plans/`.

---

## 2. Plan review (2026-07-31)

### 2.1 Decisions that still hold (no re-open)

| ADR-056 decision | Status |
|------------------|--------|
| New crate `clawft-bvh`, no kernel dep | Holds — crate exists in workspace |
| AABB as broad-phase primitive | Holds |
| Tagged-union leaf registry | Holds (tags currently provisional in-crate) |
| `Object` vs `Event` identity kinds | Holds — `IdentityKind` in tree |
| ChainAnchor / ExoChain audit from v1 (no off-chain write path) | Holds — still future work (Phase C+) |
| `SpatialBackend` trait mirroring `VectorBackend` | Holds — not implemented yet |
| Feature gate: kernel spatial behind existing `ecc` | Holds |
| Phase F (BVH × HNSW fingerprinting) deferred | Holds — still no consumer pin |

### 2.2 What the tree already has (Phase A partial)

`crates/clawft-bvh/` is present (~694 LOC) and described as **ADR-056 Phase A**:

| Planned module / capability | In tree? |
|-----------------------------|----------|
| `aabb.rs` — `Vec3`, `Aabb`, `Ray` | Yes |
| `Frustum` | **No** |
| `leaf.rs` — `Leaf`, `LeafId`, `IdentityKind` | Yes |
| Provisional splat/world-model tag constants | Yes (in `leaf::tags`, not `weftos-leaf-types`) |
| `tree.rs` — top-down median-split `BvhTree` | Yes |
| Queries: point, AABB, sphere, ray | Yes |
| Queries: frustum, kNN | **No** |
| `registry.rs` — narrow-phase tag dispatch | Yes (stub interpreters) |
| `store.rs` / `BvhStore` / `BvhStoreConfig` | **No** |
| `chain.rs` / `ChainSink` | **No** (correctly deferred to C) |
| `branch.rs` / COW | **No** (Phase D) |
| `determinism.rs` phase seal | **No** (Phase D) |
| `tests/` brute-force differential ≥10k scenarios | **No** dedicated suite yet |
| Kernel `spatial_*.rs` | **No** |
| `weftos-leaf-types::spatial` | **No** (crate/module not present as planned) |

**Implication**: Phase A is **not Done**. Scaffold + core broad-phase are in;
PLAN.md Phase A acceptance criteria are only partially met. Child ticket A
must finish remaining A surface, not re-create the crate.

### 2.3 Phase shape validation

| Phase | PLAN.md intent | Still right? | Notes |
|-------|----------------|--------------|-------|
| **A** | Standalone `clawft-bvh` broad-phase, no chain | **Yes** (complete remaining) | Scope = gaps above; do not invent kernel deps |
| **B** | Canonical tags in `weftos-leaf-types::spatial` | **Yes** | Move provisional tags out of `clawft-bvh`; freeze `#[repr(u32)]` |
| **C** | Kernel `SpatialBackend` + service + `ChainSink` | **Yes** | Depends on A+B; `ecc` feature only |
| **D** | Determinism phase + COW branches + branch_diff | **Yes** | Depends on C for real chain events |
| **E** | `weaver ecc spatial` CLI + first consumer path | **Yes** | Depends on D; document next to `ecc search` / `ecc causal` |
| **F** | BVH × HNSW fingerprinting | **Still deferred** | Do **not** file as 0.8.x implementation; optional backlog stub only |

No phase merge or drop is warranted. Cadence remains **one PR per phase**.

### 2.4 Open questions (from PLAN.md) — placement

| OQ | Resolve in | Still open? |
|----|------------|-------------|
| OQ1 narrow-phase recursion ban | **A** | Yes — encode registry-time or type-level ban |
| OQ2 supersession schedule for high-churn leaves | **D** | Yes |
| OQ3 branch retention vs RVF compaction | **D** (spike) | Yes |
| OQ4 multi-tenant membership filters | Before **E** | Yes — trait gap vs ADR text |
| OQ5 BVH × HNSW fingerprinting | After **E**, separate ADR | Yes |

### 2.5 Related consumers / demand signals (do not expand A–E)

- ADR-069 panopticon primary index expects future BVH leaves keyed by `chain_seq`.
- Splat / world-model docs already name BVH as the spatial home for volumes.
- Agent context fusion (ADR-058) marks multi-index fusion including BVH as **v2**.

These reinforce shipping A→E; they do not change phase order.

---

## 3. Decomposed phase work items

Filed in Plane under cycle **0.8.x** with labels
`ws02-kernel`, `ws17-research`, `gap`. Parent: **WEFT-592**.

Priority: **low** (matches parent; not a 0.7 gate). If 0.8.x capacity is
exhausted after B/C, D–E may be deferred to **0.9.x** with an explicit Plane
comment — do not silently stall.

### Phase A — Finish `clawft-bvh` standalone broad-phase

**Plane**: **WEFT-716**  
**Plane title**: `ws02: BVH Phase A — finish clawft-bvh broad-phase (no chain)`

**Blocked by**: none  
**Blocks**: Phase B (soft — tags can land in parallel after A public surface stabilizes), Phase C

**Scope**:
1. Complete public primitives: `Frustum`; expose kNN + frustum query APIs.
2. Add `BvhStore` (in-memory) as the integrated store surface **without**
   chain coupling (`ChainSink` trait stub OK; no kernel/tokio).
3. Documented surface (`#![warn(missing_docs)]` remains).
4. Resolve **OQ1** (narrow-phase recursion ban).
5. Differential tests vs brute force for every query type (≥10k random scenarios).
6. Do **not** add `clawft-kernel` dependency.

**Acceptance** (from PLAN.md, adjusted for partial land):
- [ ] `scripts/build.sh test` green for `-p clawft-bvh` / workspace tests covering the crate
- [ ] `scripts/build.sh check` clean for the touched packages
- [ ] point / AABB / sphere / ray / frustum / knn implemented and differential-tested
- [ ] `BvhStore` insert/remove/get + query path works without chain
- [ ] OQ1 documented + enforced in registry

**Refs**: ADR-056 §1–3; `.planning/bvh-spatial-index/PLAN.md` Phase A

---

### Phase B — `weftos-leaf-types::spatial` canonical tags

**Plane**: **WEFT-717**  
**Plane title**: `ws02: BVH Phase B — weftos-leaf-types spatial tag registry`

**Blocked by**: Phase A public leaf/tag shape stable enough to re-export  
**Blocks**: Phase C (consumers need frozen discriminants)

**Scope**:
1. Add `weftos-leaf-types/src/spatial/` (`tags.rs`, `primitives.rs`, `mod.rs`).
2. Freeze initial `SpatialLeafTag` `#[repr(u32)]` set (Sphere, Aabb, Obb,
   Capsule, SweptAabb, Frustum, RadialSphereEvent, BeamTrace, SensorRead4D,
   plus splat/world-model tags currently provisional in `clawft-bvh`).
3. CBOR payload structs + round-trip tests.
4. `clawft-bvh` re-exports / consumes the same discriminants (remove drift).

**Acceptance**:
- [ ] Tag discriminants stable (snapshot / const test)
- [ ] CBOR round-trip for every primitive struct
- [ ] No duplicate conflicting tag constants between crates
- [ ] ADR-031 deprecation rules noted for future renames

**Refs**: ADR-056 §3; ADR-031; PLAN.md Phase B

---

### Phase C — Kernel `spatial_*` adapter + service + ChainSink

**Plane**: **WEFT-718**  
**Plane title**: `ws02: BVH Phase C — SpatialBackend + SpatialService + ChainSink`

**Blocked by**: Phase A, Phase B  
**Blocks**: Phase D

**Scope**:
1. `clawft-kernel/src/spatial_backend.rs` — trait per ADR-056 §8.
2. `spatial_bvh.rs` — `BvhStore` adapter (`Arc`/`Mutex` patterns from HNSW).
3. `spatial_service.rs` — `SystemService` registration (ADR-035).
4. Kernel boot wiring behind `ecc`; `SpatialConfig` in types/config.
5. Kernel implements `ChainSink` → chain events for insert/remove/derive/rebalance_seal
   (CBOR, dual-sign per ADR-028 when chain path is live).

**Acceptance**:
- [ ] `scripts/build.sh test` / `check` green with `ecc` features as applicable
- [ ] Integration test: insert leaves → restart/replay → BVH state matches
- [ ] Service health/status path exists (CLI may still be stub until Phase E)

**Refs**: ADR-056 §6–9; ADR-022, ADR-028, ADR-035, ADR-041; PLAN.md Phase C

---

### Phase D — Determinism phase + COW branches

**Plane**: **WEFT-719**  
**Plane title**: `ws02: BVH Phase D — determinism phase seal + COW branch_diff`

**Blocked by**: Phase C  
**Blocks**: Phase E

**Scope**:
1. `BvhStore::derive` / `derive_branch` with COW node sharing.
2. Determinism-phase buffer + `seal_phase` sort by `(priority_tier, exochain_seq)`.
3. `branch_diff(a, b, region)` exactness tests.
4. Address OQ2 / spike OQ3 (document outcomes; file follow-up ADR only if
   compaction trait emerges).

**Acceptance**:
- [x] Deterministic dual-store replay: identical trees/branches/diffs
      (`BvhStore` dual-store test in `crates/clawft-bvh/src/store.rs`)
- [x] Volume branch-diff returns exactly the leaves that differ in region
      (`branch_diff_volume_exactness`)
- [x] OQ2/OQ3 outcomes written into plan or phase close comment
      (see `.planning/bvh-spatial-index/PLAN.md` OQ2/OQ3 Resolved)

**Implementation (WEFT-719)**: `clawft-bvh` modules `store`, `branch`,
`determinism`, `chain` — COW `derive_branch`, `seal_phase` sort key
`(priority_tier, exochain_seq)`, `branch_diff(a,b,region)`. Kernel
`SpatialBackend` adapter remains Phase C/E.

**Refs**: ADR-056 §5, §7; PLAN.md Phase D

---

### Phase E — Consumer integration (`weaver ecc spatial`)

**Plane**: **WEFT-720**  
**Plane title**: `ws02: BVH Phase E — weaver ecc spatial CLI + E2E`

**Blocked by**: Phase D; resolve OQ4 (membership filter) before query API freeze  
**Blocks**: none (unblocks agent router spatial use as a follow-on)

**Scope**:
1. CLI: insert / query / branch / diff / status under `weaver ecc spatial`.
2. E2E tests in weave crate.
3. Agent-guide docs next to existing `ecc search` / `ecc causal`.
4. Optional thin consumer hook for context router (not required for phase close
   if CLI E2E is solid — note in close comment).

**Acceptance**:
- [ ] CLI round-trip insert → query → branch → diff
- [ ] Chain events emitted and replayable
- [ ] Documented in `docs/clawft-agent-guide.md` (or successor ECC guide path)
- [ ] OQ4 resolved (filter arg or documented wrapper)

**Refs**: ADR-056 implementation pointer; PLAN.md Phase E

---

### Phase F — deferred (not filed as implementation)

**Title (backlog note only)**: BVH × HNSW neighborhood fingerprinting  
**Do not implement** until a real consumer pins requirements (concept paper
§12.3 Q7; ADR-056 Consequences). Track via a future ADR number when drafted —
ADR-057 is already substrate read ACLs.

---

## 4. Dependency graph

```
WEFT-592 (this gate — Done when A–E exist)
    │
    ├─► WEFT-716 Phase A  (finish broad-phase)
    │       │
    │       ├─► WEFT-717 Phase B  (canonical tags)
    │       │       │
    │       └───────┴─► WEFT-718 Phase C  (kernel + chain)
    │                       │
    │                       └─► WEFT-719 Phase D  (determinism + COW)
    │                               │
    │                               └─► WEFT-720 Phase E  (CLI + E2E)
    │
    └─► Phase F  (deferred; no 0.8.x implementation ticket)
```

Suggested Plane dependency edges (when API supports / via description):

- B blocked-by A (soft)
- C blocked-by A, B
- D blocked-by C
- E blocked-by D

---

## 5. Plane filing status

| Phase | WEFT | Cycle | State (at filing) | Labels |
|-------|------|-------|-------------------|--------|
| A | **WEFT-716** | 0.8.x | Todo | ws02-kernel, ws17-research, gap |
| B | **WEFT-717** | 0.8.x | Todo | same |
| C | **WEFT-718** | 0.8.x | Todo | same |
| D | **WEFT-719** | 0.8.x | Todo | same |
| E | **WEFT-720** | 0.8.x | Todo | same |

Filed 2026-07-31. Parent WEFT-592 closes once this doc is committed and
Plane close comment lists the five children + commit SHA.

---

## 6. Out of scope for WEFT-592

- Implementing any of Phases A–E code in this ticket.
- Physics engine, TSDB, FrankenSearch, or CVMG fork (ADR-056 non-goals).
- Replacing HNSW.
- Filing Phase F as a must-ship 0.8.x feature.

---

## 7. References

- `docs/adr/adr-056-bvh-spatial-index.md`
- `.planning/bvh-spatial-index/PLAN.md`
- `crates/clawft-bvh/`
- ADR-058 (context tier; BVH fusion v2)
- ADR-069 (panopticon; chain_seq cross-key)
- WEFT-621 is **unrelated** licensing work (AgentBBS FSL) — do not couple
