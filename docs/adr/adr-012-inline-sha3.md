# ADR-012: Inline sha3 to Eliminate rvf-crypto Path Dependency

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Tracks 3 + 8 (Release Engineering + Codebase Report)

## Context

Track 3 identified that `exo-resource-tree` appeared to have a non-optional `rvf-crypto` dependency that breaks standalone builds (critical blocker B1). Track 8 clarified that the actual dependency chain is: `clawft-kernel`'s `exochain` feature gate pulls `rvf-crypto` via path dep to `../ruvector/`. The `weftos` crate enables `exochain` by default, which means standalone builds require the ruvector workspace to be present.

`exo-resource-tree` itself depends only on `sha3`, `serde`, and `chrono` -- not on `rvf-crypto`.

## Decision

Use `blake3` as the fallback hash when `rvf-crypto` is feature-gated off. blake3 is already a workspace dependency (used by the ECC cognitive substrate) and is faster than SHA-256. The fix scope is to make the `weftos` crate's default features not include `exochain`, or to ensure `rvf-crypto` path deps have a published fallback. For v0.1.0, use option 3 (feature-gate default set to exclude ruvector paths). For v0.2.0, publish ruvector crates to crates.io.

## Consequences

### Positive
- Unblocks standalone builds and crates.io publication
- blake3 is faster than SHA-256 and already in the dependency tree
- Fix scope is narrower than Track 3 originally estimated

### Negative
- Default builds lose ExoChain cryptographic signing until ruvector is published
- Two hash implementations in the codebase (sha3 for ExoChain, blake3 for fallback)

### Neutral
- This is a v0.1.0 critical blocker fix with estimated 2-hour effort
