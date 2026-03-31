# ADR-019: Registry Trait in clawft-types

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 1 (Code Pattern Extraction)

## Context

Track 1 identified 15 registry structs across 6 crates all following the same `HashMap`-keyed pattern: insert, get, remove, list, count. Each registry implements these operations independently. The GUI (Track 4) needs to browse registries generically via the Registry Browser block. Without a shared trait, each registry requires a separate Tauri command and a separate frontend binding. Additionally, moving registries to a shared trait in `clawft-types` means the GUI does not need a heavyweight dependency on `clawft-kernel`.

## Decision

Extract a `Registry` trait to `clawft-types` that provides a uniform interface for all HashMap-keyed registries: `get`, `insert`, `remove`, `list`, `count`. All 15 existing registries will implement this trait. The GUI Registry Browser block can then introspect any registry through a single Tauri command that dispatches by registry name.

Also extract `EffectVector` and `GovernanceDecision` types to `clawft-types` so the GUI's Governance Console does not depend on the kernel crate.

## Consequences

### Positive
- Uniform GUI introspection of all registries through one interface
- GUI does not depend on kernel crate -- only on types
- Reduces boilerplate across 15 registry implementations
- Enables generic registry browser, search, and monitoring in the GUI

### Negative
- 4-hour refactoring effort to extract and implement across 6 crates
- Trait may not perfectly fit all registries if some have non-standard operations
- Breaking change for any external code depending on current registry APIs

### Neutral
- This is a v0.2 deliverable (W17, W19 in the work items)
- Related to ADR-020 (ChainLoggable trait) -- both are trait extractions from Track 1
