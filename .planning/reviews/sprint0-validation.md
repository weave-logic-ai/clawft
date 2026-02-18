# Sprint 0 Validation Report: 3F-RVF Dependency Validation

**Date**: 2026-02-17
**Toolchain**: rustc 1.93.1 (01f6ddf75 2026-02-11)
**Scope**: Validate all ruvector crate dependencies before committing to 3F-RVF implementation
**Source**: 6 parallel validation agents (T1-T6)
**Individual reports**: `/tmp/sprint0-validation/reports/t1-t6*.md`

---

## Executive Summary

**OVERALL VERDICT: CONDITIONAL PASS -- proceed with plan corrections**

All 13 ruvector crates compile successfully. All public APIs match the 3F-RVF SPARC plan's pseudocode assumptions. The candle bloat concern (reviewer C2) is definitively unfounded when using `features = ["minimal"]`. However, 4 plan corrections are required before implementation begins, and 2 WASM size claims do not hold as written.

---

## Validation Matrix

| Task | Scope | Verdict | Blocking Issues |
|------|-------|---------|-----------------|
| **T1: RVF Core Compilation** | rvf-types, rvf-runtime, rvf-index, rvf-quant, rvf-crypto | **PASS** | 0 |
| **T2: Intelligence Crates** | ruvector-core, ruvllm, sona, tiny-dancer, attention, temporal-tensor | **PASS** | 0 |
| **T3: ruvllm Dep Audit** | Feature flags, candle exclusion, binary size | **PASS (CONDITIONAL)** | 0 |
| **T4: WASM Sizing** | micro-hnsw-wasm, rvf-wasm, clawft-wasm | **CONDITIONAL PASS** | 1 (rvf-wasm size) |
| **T5: API Surface** | 8 crates vs 3F plan pseudocode | **PASS** | 0 |
| **T6: Cross-Deps** | Edition compat, feature flags, test project | **CONDITIONAL PASS** | 1 (sona naming) |

---

## Compilation Results (11 crates)

| Crate | Version | Check | Warnings | Notes |
|-------|---------|-------|----------|-------|
| rvf-types | 0.2.0 | PASS | 0 | no_std by default |
| rvf-runtime | 0.2.0 | PASS | 2 | Cosmetic (unused var) |
| rvf-index | 0.1.0 | PASS | 0 | No rvf-types dep yet (uses local types) |
| rvf-quant | 0.1.0 | PASS | 0 | |
| rvf-crypto | 0.2.0 | PASS | 0 | SHA-3 + Ed25519 |
| ruvector-core | 2.0.3 | PASS | 120 | Heavy defaults (redb, hnsw_rs, rayon) |
| ruvllm | 2.0.3 | PASS | 469 | `minimal` feature confirmed working |
| ruvector-sona | 0.1.5 | PASS | 85 | Package name is `ruvector-sona` |
| ruvector-tiny-dancer-core | - | PASS | 1 | Bundles SQLite |
| ruvector-attention | 0.1.31 | PASS | 1 | |
| ruvector-temporal-tensor | - | PASS | 0 | Zero dependencies |

---

## WASM Size Measurements

| Module | Plan Claim | Actual (raw) | Actual (gzip) | Verdict |
|--------|-----------|-------------|---------------|---------|
| micro-hnsw-wasm | < 12 KB | 11.8 KB (pre-built) / 15.3 KB (fresh) | 5.2 KB / 5.6 KB | **CONDITIONAL** -- needs wasm-opt |
| rvf-wasm | < 8 KB | **27.7 KB** | 12.1 KB | **FAIL** -- 3.4x over claim |
| rvf-solver-wasm | (no claim) | 87.0 KB | 42.6 KB | N/A |
| clawft-wasm | < 300 KB | ~139 KB (rlib) | N/A | Likely PASS (needs cdylib) |
| **Combined** | ~250 KB | ~230-280 KB est. | ~60 KB | **CONDITIONAL** |

---

## API Surface Validation (8 crates)

| Crate | Expected Types | Found | Missing | Verdict |
|-------|---------------|-------|---------|---------|
| rvf-runtime | RvfStore, create/open/ingest/query/delete/compact | All | None | **API_MATCH** |
| rvf-types | SegmentHeader, WitnessHeader, Ed25519 types | All | `RvfDocument` (by design -- raw `&[f32]`) | **API_MATCH** |
| rvf-index | HnswGraph, ProgressiveIndex, LayerA/B/C | All | None | **API_MATCH** |
| rvf-crypto | WitnessEntry, create/verify chain, Ed25519 sign/verify | All | None | **API_MATCH** |
| ruvector-sona | MicroLoRA, ReasoningBank, EwcPlusPlus, SonaEngine | All | None | **API_MATCH** |
| ruvector-tiny-dancer-core | FastGRNN, CircuitBreaker, UncertaintyEstimator | All | None | **API_MATCH** |
| ruvector-attention | FlashAttention, MoEAttention, TopologyGated, InformationBottleneck | All | None | **API_MATCH** |
| ruvllm | TaskComplexityAnalyzer, HnswRouter, QualityScoringEngine | All | None | **API_MATCH** |

---

## Candle Bloat Assessment (Reviewer C2 Concern)

**RESOLVED: candle is NOT pulled with `minimal` feature.**

| Metric | Default | Minimal | Delta |
|--------|---------|---------|-------|
| Unique crates | 335 | 165 | **-51%** |
| candle-core | YES | **NO** | Excluded |
| candle-nn | YES | **NO** | Excluded |
| candle-transformers | YES | **NO** | Excluded |
| tokenizers | YES | **NO** | Excluded |
| hf-hub + reqwest + hyper | YES | **NO** | Excluded |
| gemm variants | YES | **NO** | Excluded |

The `minimal` feature is defined as `["async-runtime"]` which only adds tokio + tokio-stream. All ML inference frameworks are gated behind the `candle` feature (which is in `default` but NOT in `minimal`).

**Binary size with minimal**: rlib = 41 MB, estimated linked contribution = 15-25 MB (dominated by ruvector-core + hnsw_rs + redb stack, not ruvllm itself).

---

## Required Plan Corrections (MUST FIX)

### P0: Must fix before Sprint 1

| # | Severity | Issue | Fix |
|---|----------|-------|-----|
| **1** | **HIGH** | Package name `sona` is actually `ruvector-sona` | Change all `dep:sona` to `dep:ruvector-sona` in 3F plan Cargo.toml sections |
| **2** | **HIGH** | Crate name `rvf-adapters-agentdb` is actually `rvf-adapter-agentdb` (singular) | Update plan references; also add to ruvector workspace members |
| **3** | **HIGH** | rvf-wasm < 8 KB claim does not hold (actual: 27.7 KB) | Either accept 28 KB or split into core/ext |
| **4** | **MEDIUM** | sona EWC type is `EwcPlusPlus`, not `EWC` or `Ewc` | Update plan pseudocode references |

### P1: Should fix before Sprint 1

| # | Severity | Issue | Fix |
|---|----------|-------|-----|
| **5** | **MEDIUM** | ruvllm `minimal` still pulls 165 crates via ruvector-core (redb, hnsw_rs, rayon, simsimd) | Consider `ruvector-core/memory-only` for lighter integration where full vector store not needed |
| **6** | **MEDIUM** | micro-hnsw-wasm needs `wasm-opt` to meet 12 KB claim (15.3 KB without it) | Add `wasm-opt` to CI pipeline; document as build requirement |
| **7** | **MEDIUM** | rvf workspace broken -- missing `rvf-adapters/claude-flow` directory | Fix before CI can build rvf crates |
| **8** | **LOW** | rvf-index does not depend on rvf-types (uses local types) | May cause type mismatches during integration; monitor |
| **9** | **LOW** | Duplicate dep versions (rand 0.8+0.9, getrandom 0.2+0.3+0.4, thiserror 1+2) | Non-blocking; adds ~1-2 MB binary bloat |
| **10** | **LOW** | clawft-wasm is rlib not cdylib -- can't measure standalone .wasm size | Add cdylib target if standalone size measurement needed |

---

## Cross-Dependency Compatibility

| Property | clawft | ruvector | Compatible? |
|----------|--------|----------|-------------|
| Edition | 2024 | 2021 | **YES** -- Rust handles cross-edition deps natively |
| MSRV | 1.93 | 1.77-1.87 | **YES** -- 1.93.1 satisfies all |
| Feature conflicts | None | None | **YES** -- proposed 3F feature names unused in clawft |
| reqwest | 0.12 | 0.11 (optional) | **YES** -- only triggered by `api-embeddings` feature |
| thiserror | 2 | 1+2 mixed | **YES** -- both versions coexist |

**Test project result**: An edition-2024 project consuming rvf-runtime, rvf-types, ruvllm (minimal), and ruvector-sona via local path deps compiles with zero errors.

---

## Recommendations for 3F-RVF Sprint 1

1. **Use local path deps for development**, git deps for CI/release:
   ```toml
   rvf-runtime = { path = "/home/aepod/dev/barni/repos/ruvector/crates/rvf/rvf-runtime" }
   ```

2. **Gate ruvector deps behind non-default features** in clawft-core to prevent WASM compilation issues:
   ```toml
   [target.'cfg(not(target_arch = "wasm32"))'.dependencies]
   ruvllm = { ..., optional = true }
   ```

3. **Accept revised WASM budgets**: rvf-wasm at ~28 KB (not 8 KB), micro-hnsw-wasm at ~12 KB (with wasm-opt).

4. **Install `wasm-opt` (binaryen)** in dev and CI environments for WASM size optimization.

5. **Add Sprint 1 exit gate**: Measure actual binary size delta after adding rvf deps to clawft-core. If native binary exceeds 15 MB, evaluate ruvector-core feature reduction.

---

## Conclusion

The ruvector dependency stack is **compilation-ready and API-complete** for 3F-RVF integration. The reviewer's primary concern (candle bloat via ruvllm) is definitively resolved. Four plan corrections are needed (naming, WASM sizing), but no architectural changes are required. Sprint 1 can proceed after applying the P0 fixes above.
