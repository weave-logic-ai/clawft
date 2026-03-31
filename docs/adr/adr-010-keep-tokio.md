# ADR-010: Keep Tokio (Do Not Adopt Asupersync)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 9 (Optimization Plan)

## Context

The Asupersync runtime offers a cancel-correct execution model that eliminates a class of bugs that Tokio's cancellation semantics can produce (particularly in `select!` branches). However, every async function in the WeftOS codebase, plus all dependencies that use Tokio types (channels, timers, I/O), would need to be rewritten. The migration cost is extreme.

Cancel-correctness concerns are real but localized: the primary risk area is `select!` branches within the mesh networking code (`mesh_runtime.rs`, `mesh_heartbeat.rs`).

## Decision

Keep Tokio as the async runtime. Address cancel-correctness by auditing `select!` branches within the mesh networking code at v0.3, rather than replacing the runtime. Do not adopt Asupersync.

## Consequences

### Positive
- Zero migration cost -- all existing async code remains unchanged
- Tokio has the largest ecosystem of compatible libraries
- Cancel-safety issues are addressable through targeted audit, not wholesale replacement
- The Asupersync Analyst panelist explicitly recommended against replacement

### Negative
- Latent cancel-safety bugs remain until the v0.3 audit
- Cannot leverage Asupersync's structurally correct cancellation model

### Neutral
- 42 Mutex instances remain a latent deadlock risk -- addressed separately via lock ordering protocol at v0.3
- The focused audit approach is standard practice for mature async Rust codebases
