# ADR-043: BLAKE3 Forward, SHAKE-256 Present (Hash Migration Plan)

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: ECC Symposium (D6, Q1 Resolution)

## Context

The WeftOS codebase uses two hash algorithms in different subsystems:

**SHAKE-256** (via `rvf-crypto`): Used by ExoChain (`chain.rs`), Resource Tree (`tree_manager.rs`), and node identity (`cluster.rs` -- `node_id()` computes `hex(SHAKE-256(pubkey)[0..16])`). Introduced as Decision 17 in K0 when the original `sha2::Sha256` was found to have a vulnerability (payload was not included in `compute_event_hash()`, allowing payload swaps). Decision 19 then unified Resource Tree to SHAKE-256 to maintain a "single-hash-family invariant."

**BLAKE3**: The original SPARC architecture documents specify BLAKE3 throughout via `exo-core`: content-addressed filesystem (doc 08), ResourceId as `Blake3Hash` (doc 13), Merkle roots via `Blake3Hasher` (doc 13), ExoChain substrate (doc 13), and DIDs as `blake3(pubkey)` (doc 08). The `exo-core` crate (BLAKE3 + HLC + domain separators) is scheduled for K6.1. BLAKE3 is already a workspace dependency used by the ECC cognitive substrate.

The ECC Symposium Q1 investigation confirmed that SHAKE-256 was a pragmatic K0 choice, not an architectural commitment. The gap analysis (`ruvector-gap-analysis.md`) explicitly documents the migration path: "K0 uses SHA-256 ChainManager with custom ChainEvent structs, not exochain's BLAKE3 + HLC + DAG. Migration path to `exo-dag::DagStore` exists for K6."

## Decision

New ECC code (CrossRefs via `crossref.rs`, `UniversalNodeId`, `CausalGraph`, `ImpulseQueue`) uses BLAKE3, aligning with the intended architecture in `exo-core`. Existing ExoChain and Resource Tree keep SHAKE-256 until the K6 migration brings everything to BLAKE3 via `exo-core`.

The single-hash-family invariant is maintained within each subsystem:
- ECC subsystem (K3c+): BLAKE3 exclusively
- ExoChain subsystem (K0-K5): SHAKE-256 exclusively
- Resource Tree subsystem (K0-K5): SHAKE-256 exclusively

Cross-references between ECC structures (BLAKE3 `UniversalNodeId`) and ExoChain events (SHAKE-256 hashes) carry both hash representations at the boundary, with the `StructureTag` enum (ExoChain=0x01, ResourceTree=0x02, CausalGraph=0x03, HnswIndex=0x04) disambiguating which hash family applies.

The K6 migration is the hard dependency for hash unification. It will replace the SHAKE-256 ChainManager with `exo-dag::DagStore` (BLAKE3 + HLC) and migrate Resource Tree hashes. This is a one-way migration requiring chain re-hashing.

## Consequences

### Positive
- New ECC code aligns with the target architecture from day one, avoiding a second migration
- BLAKE3 is faster than SHAKE-256 on all target platforms (especially significant for the cognitive tick budget)
- The single-hash-family invariant within each subsystem prevents mixed-hash confusion during development
- The `StructureTag` discriminant provides a clean mechanism for handling cross-subsystem references during the coexistence period

### Negative
- Two hash algorithms coexist in the codebase until K6, increasing cognitive load for developers
- Cross-references between ECC (BLAKE3) and ExoChain (SHAKE-256) must carry dual hashes at the boundary, adding storage overhead
- The K6 migration is a hard dependency: hash unification cannot happen incrementally, it requires a coordinated cutover
- Chain re-hashing during K6 migration breaks all existing `AnchorReceipt` references (ADR-041) that reference SHAKE-256 hashes

### Neutral
- Both `blake3` and `sha3` (SHAKE-256 via rvf-crypto) remain workspace dependencies until K6 completes
- The migration path is explicitly documented in planning artifacts, reducing the risk of it being forgotten or deprioritized
- The coexistence period is bounded: K6 is a defined milestone, not an indefinite deferral
