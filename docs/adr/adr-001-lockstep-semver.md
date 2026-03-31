# ADR-001: Workspace-Level Lockstep Semver Versioning

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 3 (Release Engineering)

## Context

The WeftOS workspace contains 22 crates with tight inter-crate coupling. During pre-1.0 development, independently versioning each crate would impose significant coordination overhead on a small team. Every change that crosses a crate boundary would require manual version bump cascades. The Bevy, Zellij, and Dioxus projects all faced this problem and converged on lockstep versioning.

## Decision

All 22 crates share a single version via `[workspace.package] version` in the root `Cargo.toml`. Each crate inherits with `version.workspace = true`. Breaking changes bump the minor version (0.1 to 0.2), non-breaking changes bump the patch (0.1.0 to 0.1.1), following the Cargo 0.x convention.

## Consequences

### Positive
- Single source of truth for version -- eliminates version drift between crates
- Simplifies release automation (one version bump, one tag, one changelog entry)
- Matches the Bevy/Dioxus pattern familiar to the Rust ecosystem

### Negative
- A bug fix in one crate bumps the version of all 22 crates, even unchanged ones
- Cannot publish crates independently at different cadences
- After 1.0, may need to revisit if crates diverge in stability

### Neutral
- Tools like `cargo-workspaces` and `release-plz` are designed for this pattern
