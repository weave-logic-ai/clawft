# ADR-002: cargo-dist for Release Artifact Generation

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 3 (Release Engineering)

## Context

WeftOS needs cross-platform binary releases for 5+ targets (Linux x86/ARM, macOS Intel/Apple Silicon, Windows), shell/PowerShell install scripts, Homebrew formula, and GitHub Release artifacts. Hand-rolling a CI matrix for this is weeks of debugging work. The Ruff project proved that cargo-dist scales to large Rust workspaces.

## Decision

Use `cargo-dist` for release artifact generation. Running `cargo dist init` generates a GitHub Actions release workflow covering all platform targets, install scripts, Homebrew formula, `cargo binstall` metadata, and SHA256 checksums. The existing hand-rolled `release.yml` workflow will be replaced.

Complement with `release-plz` (over release-please) for automated version bumps and release PRs, and `git-cliff` for changelog generation from conventional commits.

## Consequences

### Positive
- Eliminates weeks of CI matrix debugging
- Provides install scripts, Homebrew, and binstall metadata for free
- Well-maintained by the Axo team; used by Ruff, Zellij, and others

### Negative
- Adds a build-time dependency on an external tool
- Configuration is opinionated -- custom archive formats require workarounds

### Neutral
- release-plz chosen over release-please because it is Rust-native and integrates with git-cliff and cargo publish
