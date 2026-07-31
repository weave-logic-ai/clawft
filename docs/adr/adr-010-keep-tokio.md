# ADR-010: Keep Tokio (Do Not Adopt Asupersync)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 9 (Optimization Plan)

## Context

The Asupersync runtime offers a cancel-correct execution model that eliminates a class of bugs that Tokio's cancellation semantics can produce (particularly in `select!` branches). However, every async function in the WeftOS codebase, plus all dependencies that use Tokio types (channels, timers, I/O), would need to be rewritten. The migration cost is extreme.

Cancel-correctness concerns are real but localized: the primary risk area is `select!` branches within the mesh networking code (originally cited as `mesh_runtime.rs` / `mesh_heartbeat.rs`; the live production `select!` lives in the mesh peer loop in `boot.rs`, with framing on `MeshStream` / `mesh_tcp.rs`).

## Decision

Keep Tokio as the async runtime. Address cancel-correctness by auditing `select!` branches within the mesh networking code at v0.3, rather than replacing the runtime. Do not adopt Asupersync.

## Audit status (WEFT-18, 2026-07-30)

v0.3 cancel-correctness audit **completed**. Findings and regression convention:
[`docs/research/adr-010-mesh-select-cancel-audit-2026-07-30.md`](../research/adr-010-mesh-select-cancel-audit-2026-07-30.md).

Material fix: `TcpMeshStream::recv` is now cancel-safe (partial length/body progress retained on `self`). `MeshStream::recv` / `EncryptedChannel::recv_encrypted` document the cancel-safety contract for future transports.

## Consequences

### Positive
- Zero migration cost -- all existing async code remains unchanged
- Tokio has the largest ecosystem of compatible libraries
- Cancel-safety issues are addressable through targeted audit, not wholesale replacement
- The Asupersync Analyst panelist explicitly recommended against replacement
- Mesh `select!` cancel risks are catalogued, fixed where clear, and guarded by contract + tests (WEFT-18)

### Negative
- ~~Latent cancel-safety bugs remain until the v0.3 audit~~ — audit complete; residual notes in the memo only
- Cannot leverage Asupersync's structurally correct cancellation model

### Neutral
- 42 Mutex instances remain a latent deadlock risk -- addressed separately via lock ordering protocol at v0.3
- The focused audit approach is standard practice for mature async Rust codebases
