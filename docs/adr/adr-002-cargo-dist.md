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
- Generated `release.yml` is easy to drift from stock (`dist generate`
  overwrites); hand-patches (WEFT-593) require a documented re-apply path

### Neutral
- release-plz chosen over release-please because it is Rust-native and integrates with git-cliff and cargo publish
- As of 2026-07-31, release-plz / git-cliff are **not** adopted; version
  bumps + changelog remain manual (`scripts/release/generate-changelog.sh`
  + runbook). That drift is tracked separately from the cargo-dist pin.

## Amendment (2026-07-31) — version pin and regenerate cadence (WEFT-462)

### Pin policy

- The authoritative pin is
  `[workspace.metadata.dist].cargo-dist-version` in the root `Cargo.toml`.
- CI installs that exact cargo-dist version; developer machines should
  match when running `dist plan` / `dist generate`.
- Bump to **current upstream stable** when ready — not to a speculative
  major. As of 2026-07-31 upstream latest stable is **0.32.0**; **v1.0
  has not shipped**. Ticket wording that assumed v1.0 is outdated.

### Regenerate cadence (quarterly sweep)

- **At least quarterly**, or sooner for security / Actions-related
  cargo-dist releases, check
  <https://github.com/axodotdev/cargo-dist/releases> and open a chore PR
  if the pin is more than one minor behind **or** we need a verified
  feature (SBOM, wasip2-in-matrix, etc.).
- **Never** combine a cargo-dist bump with a product release tag cut.
- Procedure lives in `docs/deployment/release.md` ("How to bump
  cargo-dist") and the migration plan
  `docs/plans/weft-462-cargo-dist.md`.

### Coupling to WEFT-593

`.github/workflows/release.yml` is **not** pure stock output: it carries
a hand-patch so GHA secret-scanning cannot drop the plan job's matrix
output (v0.6.21 binary-less release). While that patch exists:

1. `allow-dirty = ["ci"]` stays set.
2. Every `dist generate` **must** re-apply the WEFT-593 checklist (slim
   job outputs, dedicated matrix output, refuse binary-less announce).
3. Full bump PRs prove `dist plan` still yields the expected
   `artifacts_matrix.include` length before merge.

When upstream stock workflow is proven safe against the secret-scan
failure mode, the hand-patch (and then `allow-dirty`) may be retired in
the same bump PR with explicit evidence.
