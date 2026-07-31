# WEFT-356 result — KG-017 knowledge distillation for edge EML (SevenNet-Nano)

**Branch:** `feat/weft-356-distill-eml`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb84a-8c66-7331-9873-ac3d4a63a197`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-31

## Problem

Audit gap KG-017: no end-to-end path to distill depth-4 EML → depth-2 for
WASM/ESP32. A basic `EmlModel::distill` helper existed without SevenNet-Nano
naming, fidelity reporting, edge load path, or design note.

## What shipped

| Surface | Behavior |
|---------|----------|
| `distill(teacher, depth, n)` | Free offline function (docs-aligned) |
| `distill_with_report` + `DistillConfig` | Synthetic teacher→student + `DistillReport` |
| `SevenNetNano` | Depth-2 edge package; `from_teacher` / `from_json` / freeze inference |
| `EdgeEmlRuntime` | Teacher vs Nano selection (`select`, `select_with_flag`, `edge`, `host`) |
| Design note | `docs/design/eml-sevennet-nano-distillation.md` |

`EmlModel::distill` now delegates to the free function (single implementation).

## Files changed

| File | Change |
|------|--------|
| `crates/eml-core/src/distill.rs` | **new** — pipeline, Nano, runtime, tests |
| `crates/eml-core/src/lib.rs` | module + re-exports |
| `crates/eml-core/src/model.rs` | `distill` delegates to free API |
| `docs/design/eml-sevennet-nano-distillation.md` | design note |
| `docs/src/content/docs/weftos/eml.mdx` | KG-017 / SevenNet-Nano docs |
| `docs/plans/wave-0-WEFT-356-result.md` | this report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Distillation pipeline (offline) producing depth-2 weights | Done |
| Runtime path that loads depth-2 weights when on edge target | Done |
| Recall delta vs depth-4 documented | Done (`DistillReport::recall_delta` + design note) |
| WASM target tested | Done (`cargo check -p eml-core --target wasm32-unknown-unknown`) |
| Pure Rust synthetic train/eval + freeze-step tests | Done |
| SevenNet-Nano naming/interface | Done |

## Deferred

- Dual-weight load in `EmlCoherenceModel` / HNSW-EML wrappers
- CAS/OPFS packaging for browser (WS16)
- Real graph-feature / embedding batch sampler (beyond synthetic LCG)
- Hard wasm latency microbench vs depth-4

## Verification

```bash
scripts/build.sh test eml-core
cargo check -p eml-core --target wasm32-unknown-unknown
```
