# ADR-002: cargo-dist for Release Artifact Generation

**Date**: 2026-03-28
**Status**: Accepted (amended 2026-07-31 — WEFT-471)
**Deciders**: Sprint 11 Symposium Track 3 (Release Engineering);
amendment: 0.8.x governance (WEFT-471)
**Closes**: WEFT-471 (governance gap: ADR vs actual version/changelog flow)
**Runbook**: [`docs/deployment/release.md`](../deployment/release.md)

## Context

WeftOS needs cross-platform binary releases for 5+ targets (Linux x86/ARM,
macOS Intel/Apple Silicon, Windows), shell/PowerShell install scripts,
Homebrew formula, and GitHub Release artifacts. Hand-rolling a CI matrix
for this is weeks of debugging work. The Ruff project proved that
cargo-dist scales to large Rust workspaces.

The original decision also named `release-plz` (version bumps / release
PRs) and `git-cliff` (changelog from conventional commits) as complements
to cargo-dist. Neither was wired into CI or the shipping runbook. Reality
diverged: releases are **manual version bump + annotated tag + push**,
with a home-rolled conventional-commit grouper for draft release notes.
The 0.7.0 release-gate audit
(`.planning/reviews/0.7.0-release-gate/14-deployment-release.md`) flagged
this as deferred rows "No release-plz" and "No git-cliff" (Task #19 /
WEFT-471).

Adopting release-plz now would mean new workflow config, crates.io token
policy, PR bot semantics across a large lockstep workspace (ADR-001), and
operator training — out of proportion for a pure documentation gap.
Amending this ADR is the chosen path.

## Decision

### Artifact generation (unchanged, shipped)

Use **`cargo-dist`** for release artifact generation. Workspace metadata
lives under `[workspace.metadata.dist]` in the root `Cargo.toml`
(`cargo-dist-version = "0.31.0"`). Tag push runs `.github/workflows/release.yml`
and produces platform archives, install scripts, Homebrew tap updates,
`cargo binstall` metadata, SHA256 checksums, and sigstore attestations.

Parallel non-cargo-dist legs (WASI, browser WASM, KB, crates.io, Docker,
release gate) are documented in the runbook; they do not replace
cargo-dist for native binary distribution.

### Version bumps and release PRs — **not** release-plz (amendment)

**Do not adopt `release-plz` for the current shipping path.**

Canonical version flow:

1. Bump the workspace lockstep version with
   `cargo workspaces version {patch|minor|major}` (ADR-001).
2. Curate `CHANGELOG.md` and regenerate docs MDX
   (`scripts/build.sh releases-mdx`) in the same commit as the bump.
3. Create an annotated SemVer tag (`vX.Y.Z`) on that commit.
4. `git push origin vX.Y.Z` — workflows fan out from the tag.

No automated release PR bot, no `release-plz.toml`, and no release-plz
GitHub Action. Revisit only if release cadence or multi-maintainer
coordination makes manual bumps a bottleneck (file a new WEFT / ADR
amendment then).

### Changelog generation — home-rolled path is canonical (amendment)

**Canonical changelog tooling is manual Keep-a-Changelog curation of
`CHANGELOG.md`, optionally seeded by
`scripts/release/generate-changelog.sh`.**

| Tool | Role today |
|------|------------|
| `CHANGELOG.md` | Source of truth for release notes and docs-site MDX |
| `scripts/release/generate-changelog.sh` | Optional draft helper: groups conventional commits between two refs into markdown sections |
| `scripts/build.sh releases-mdx` | Regenerates the docs-site Release Notes page from `CHANGELOG.md` |
| `cliff.toml` | **Dormant scaffold only** — present from Sprint 11 planning; **not** invoked by CI, `scripts/build.sh`, or the release runbook |
| `git-cliff` / `release-plz` | **Not adopted** for shipping |

Operators may still install git-cliff locally and point it at `cliff.toml`
for experiments; that path is not blessed and must not be assumed by
automation.

## Consequences

### Positive
- Eliminates weeks of CI matrix debugging (cargo-dist)
- Provides install scripts, Homebrew, and binstall metadata for free
- Well-maintained by the Axo team; used by Ruff, Zellij, and others
- ADR and runbook now match the path operators actually run (no
  phantom release-plz / git-cliff dependency)

### Negative
- Adds a build-time dependency on cargo-dist (external tool)
- Configuration is opinionated — custom archive formats require workarounds
- Manual version bumps and changelog curation do not scale as well as a
  release bot; acceptable at current cadence

### Neutral
- release-plz remains a reasonable *future* option (Rust-native, works
  with lockstep workspaces) but is **explicitly out of scope** until a
  new decision reopens it
- `cliff.toml` may be deleted or wired up in a later change; either way
  requires an explicit runbook update

## References

- Runbook: [`docs/deployment/release.md`](../deployment/release.md)
- Lockstep versioning: [ADR-001](adr-001-lockstep-semver.md)
- Changelog draft helper: `scripts/release/generate-changelog.sh`
- Plane: WEFT-471 (ws14 governance)
- Audit source: `.planning/reviews/0.7.0-release-gate/14-deployment-release.md`
