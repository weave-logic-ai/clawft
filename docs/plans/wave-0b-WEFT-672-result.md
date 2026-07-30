# WEFT-672 result — ungated strip_think for browser/WASM

**Ticket:** WEFT-672  
**Branch:** `wave0b/weft-672-strip-think-wasm`  
**Wave:** 0b  
**Date:** 2026-07-30  
**Base:** `release/0.8-staging` (WEFT-663 already integrated)

## Problem

`crates/clawft-core/src/pipeline/transport.rs` called
`clawft_llm::hermes::strip_think` unconditionally, but
`clawft-llm::hermes` was `#[cfg(feature = "native")]`-only. Under
`--no-default-features --features browser --target wasm32-unknown-unknown`
the symbol did not exist (E0433), breaking the browser-WASM gate after
WEFT-663 cleared the Send-future class of errors.

Introduced by Hermes serving-provider work (`cf1a77c1`); independent of
WEFT-663's root cause.

## Fix

Expose the pure Hermes dialect module under all feature combinations.
`hermes` has no I/O and only depends on always-on crates (`serde_json`,
`types`). Making it available under browser keeps the transport call site
unchanged and preserves `<think>` stripping for browser LLM responses.

```rust
// crates/clawft-llm/src/lib.rs
// Pure Hermes dialect helpers — available under browser/WASM as well as
// native so shared pipeline code can strip `<think>` without a native-only
// cfg (WEFT-672).
pub mod hermes;
pub use hermes::ReasoningMode;
```

Native-only modules (`local_provider`, `openai_compat`, `router`, …) stay
gated. No change to `transport.rs` call site.

## Verification

### Browser WASM (this ticket's error class)

```bash
cargo check -p clawft-core --no-default-features --features browser --target wasm32-unknown-unknown
# Finished `dev` profile … (exit 0) — no hermes E0433

cargo check -p clawft-wasm --no-default-features --features browser --target wasm32-unknown-unknown
# Finished `dev` profile … (exit 0) — PR-gates matrix command
```

| Metric | Before (post-663) | After |
|--------|-------------------|-------|
| E0433 `hermes` unresolved in transport | **1** | **0** |
| Cascading E0282 from that path | 4 | 0 |
| clawft-core browser wasm check | fail | **pass** |
| clawft-wasm browser wasm check | fail (same class) | **pass** |

### Native

```bash
scripts/build.sh check   # cargo check --workspace — OK (~40s)
cargo test -p clawft-llm --lib hermes::   # 19 passed
```

## Files changed

- `crates/clawft-llm/src/lib.rs` — ungate `pub mod hermes` + `ReasoningMode` re-export
- `docs/plans/wave-0b-WEFT-672-result.md` (this file)

## Follow-ups

- Promote browser-WASM matrix from soft to hard gate if still soft (WEFT-447 / ticket notes) so native-only symbols called from shared code cannot land silently again.
- Broader audit: other shared pipeline call sites into `#[cfg(feature = "native")]` clawft-llm modules.

## Commit

**SHA:** `c4d921491c9fa40a75b99dc246b9520a9e44b4be`  
Branch: `wave0b/weft-672-strip-think-wasm`
