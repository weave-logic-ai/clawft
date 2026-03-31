# Sprint 11 Symposium -- Track 3: Release Engineering

**Chair**: release-manager
**Panelists**: release-manager, cicd-engineer, repo-architect, pr-manager, sync-coordinator
**Date**: 2026-03-27
**Baseline**: 00-opening-plenary.md
**Strategy ref**: .planning/sparc/weftos/0.1/11-release-strategy.md

---

## 1. Infrastructure Audit

### 1.1 `.github/workflows/release.yml` -- EXISTS, INCOMPLETE

**What it does**:
- Triggers on `v*.*.*` tag push
- Builds multi-arch Docker image (linux/amd64, linux/arm64) via Buildx + QEMU
- Pushes to GHCR with semver tags (v0.1.0, 0.1, 0, latest)
- Runs a post-publish smoke test (docker run + 5s wait)

**What is missing**:
- No native binary builds (no cross-platform matrix for macOS, Windows, Linux standalone)
- No GitHub Release creation (no artifacts uploaded to Releases page)
- No install script generation
- No Homebrew formula publishing
- No shell completions bundled
- No SHA256 checksums or attestations
- No changelog injection into release notes
- No cargo publish step
- No WASM artifact upload
- The smoke test is fragile (5-second sleep, no real health check)

**Verdict**: Docker-only release. Unusable as primary distribution. cargo-dist would replace this entirely.

### 1.2 `.github/workflows/pr-gates.yml` -- EXISTS, SOLID

**What it does** (7 jobs):
1. Clippy lint (warnings-as-errors)
2. Full workspace test suite
3. WASM size gate (<300KB raw, <120KB gzip) -- targets `wasm32-wasip2`
4. Binary size check (<10MB for `weft`)
5. Browser WASM check (`wasm32-unknown-unknown`) -- soft fail
6. Voice feature check -- soft fail
7. UI lint + type-check (pnpm, Node 20)
8. Integration smoke test (Docker build + gateway start)

**What is missing**:
- No `cargo-semver-checks` gate (breaking change detection)
- No `cargo deny` / license check
- No security audit (`cargo audit`)
- PR trigger targets `main` and `master` -- should be just `master` (actual default branch)
- WASM target is `wasm32-wasip2` in CI but `wasm32-wasip1` in `scripts/build.sh` -- mismatch

**Verdict**: Good foundation. Add semver-checks and audit gates before v0.1.0 tagging.

### 1.3 `.github/workflows/benchmarks.yml` -- EXISTS

Runs on PR to main and push to main. Builds release binary, runs benchmark scripts, saves results, checks for regressions against baseline. Posts PR comment on regression. Good.

### 1.4 `.github/workflows/wasm-build.yml` -- EXISTS

Detailed WASM build pipeline: build, wasm-opt, size gate, twiggy profiling, artifact upload. Targets `wasm32-wasip2`. Well structured.

### 1.5 `Dockerfile` -- EXISTS, GOOD

- Multi-stage: cargo-chef planner, builder, runtime
- Runtime: `debian:bookworm-slim` (strategy doc recommends distroless -- upgrade later)
- Builds `weft` binary only (not `weaver` or `weftos`)
- Has HEALTHCHECK via `weft status`
- Non-root user
- Target: <50MB compressed

**Issues**:
- Only builds `weft`, not `weaver` or `weftos` daemon binary
- Uses `debian:bookworm-slim`, not distroless (Phase 2 improvement per strategy)
- VOLUME path is `/home/weft/.clawft` (old name, should be `.weftos` for branding)
- Rust version pinned to `1.93-bookworm` -- correct

### 1.6 `docker-compose.yml` -- EXISTS, MINIMAL

Two-node mesh config. Uses build context (no pre-built image ref). Environment vars `WEFTOS_DATA_DIR`, `WEFTOS_SEED_PEERS`. Functional for local dev/testing.

### 1.7 `scripts/build.sh` -- EXISTS, COMPREHENSIVE

Covers: native (release/debug), wasi, browser, ui, test, check, clippy, clean, serve, gate (11 checks). Feature flag support, dry-run, verbose, force rebuild. Well engineered.

**Issues**:
- WASI target is `wasm32-wasip1` but CI uses `wasm32-wasip2` -- **mismatch** [HP-8]
- No `dist` or `release` subcommand for running cargo-dist locally
- No `changelog` subcommand for git-cliff
- Gate check 4-9 uses `wasm32-unknown-unknown` browser checks per-crate -- good

### 1.8 `CHANGELOG.md` -- EXISTS, STALE

Contains an entry for `[0.1.0] - 2026-02-17` that documents the initial workspace from Sprint 1-5 era. Does NOT include Sprint 6-10 work (self-healing, persistence, mesh, DEMOCRITUS, tool signing, WASM shell, kernel hardening, 983 new tests, K8 GUI prototype). The changelog is approximately 40% of what v0.1.0 actually contains.

**Verdict**: Must be regenerated. git-cliff from conventional commits, or manual curation of Sprint 6-10 additions.

### 1.9 Missing Release Tooling Config Files

| File | Status | Action |
|------|--------|--------|
| `cliff.toml` | MISSING | Create (git-cliff config) |
| `release-plz.toml` | MISSING | Create (Phase 2) |
| `.github/release.yml` (PR categorization) | MISSING | Create |
| `ROADMAP.md` | MISSING | Create |
| `MIGRATION.md` | MISSING | Create (empty, for 0.2.0) |
| `dist-workspace.toml` or `[dist]` in Cargo.toml | MISSING | Created by `cargo dist init` |

---

## 2. Version State -- Per-Crate Analysis

### 2.1 Workspace Version

The `[workspace.package]` section in the root `Cargo.toml` does **NOT** define `version`. It defines:
- `edition = "2024"`
- `rust-version = "1.93"`
- `license = "MIT OR Apache-2.0"`
- `repository` and `homepage`

**Each crate has `version = "0.1.0"` hardcoded.** None use `version.workspace = true`.

This contradicts the release strategy document which assumes `version.workspace = true` is the target state. The migration has not been done.

### 2.2 Per-Crate Version Audit

All 22 crates have `version = "0.1.0"` (hardcoded). All use `edition.workspace = true` and `rust-version.workspace = true`. Versions are consistent.

| Crate | Version | `publish = false` | Binary | Tier (per strategy) |
|-------|---------|-------------------|--------|---------------------|
| clawft-types | 0.1.0 | NO | -- | 1 (publish as weftos-types) |
| clawft-platform | 0.1.0 | NO | -- | 2 |
| clawft-core | 0.1.0 | NO | -- | 1 (publish as weftos-core) |
| clawft-kernel | 0.1.0 | NO | -- | 1 (publish as weftos-kernel) |
| clawft-llm | 0.1.0 | NO | -- | 2 |
| clawft-tools | 0.1.0 | NO | -- | 2 |
| clawft-channels | 0.1.0 | NO | -- | 3 |
| clawft-services | 0.1.0 | NO | -- | 3 |
| clawft-cli | 0.1.0 | NO | `weft` | 3 (binary only) |
| clawft-weave | 0.1.0 | NO | `weaver` | 3 (binary only) |
| clawft-wasm | 0.1.0 | NO | -- | 3 (npm only) |
| clawft-plugin | 0.1.0 | NO | -- | 1 (publish as weftos-plugin) |
| clawft-plugin-git | 0.1.0 | NO | -- | 3 |
| clawft-plugin-cargo | 0.1.0 | NO | -- | 3 |
| clawft-plugin-oauth2 | 0.1.0 | NO | -- | 3 |
| clawft-plugin-treesitter | 0.1.0 | NO | -- | 3 |
| clawft-plugin-browser | 0.1.0 | NO | -- | 3 |
| clawft-plugin-calendar | 0.1.0 | NO | -- | 3 |
| clawft-plugin-containers | 0.1.0 | NO | -- | 3 |
| clawft-security | 0.1.0 | NO | -- | 2 |
| exo-resource-tree | 0.1.0 | NO | -- | 1 (standalone brand) |
| weftos | 0.1.0 | NO | `weftos` | 1 (facade) |

**Critical finding**: No crate has `publish = false`. The strategy document requires Tier 3 crates to have it. Without this, `cargo publish` or `cargo ws publish` will attempt to publish all 22 crates, including internal ones.

### 2.3 Ruvector Path Dependencies

8 external path dependencies point to `../ruvector/`:

| Ruvector Crate | Depended On By | Optional? | Feature Gate |
|----------------|---------------|-----------|-------------|
| `rvf-runtime` | clawft-core, clawft-kernel | YES | `rvf` (core), `cluster` (kernel) |
| `rvf-types` | clawft-core, clawft-kernel, clawft-weave | YES | `rvf` (core), `cluster` (kernel), `rvf-rpc` (weave) |
| `rvf-crypto` | **exo-resource-tree**, clawft-kernel | **NO** (exo), YES (kernel) | Non-optional in exo-resource-tree |
| `rvf-wire` | clawft-kernel, clawft-weave | YES | `cluster` (kernel), `rvf-rpc` (weave) |
| `ruvector-cluster` | clawft-kernel | YES | `cluster` |
| `ruvector-raft` | clawft-kernel | YES | `cluster` |
| `ruvector-replication` | clawft-kernel | YES | `cluster` |
| `cognitum-gate-tilezero` | clawft-kernel | YES | `cluster` |

**BLOCKER**: `exo-resource-tree` has a **non-optional** dependency on `rvf-crypto`. Since `weftos` depends on `exo-resource-tree` unconditionally, every build of the `weftos` binary (and anything that depends on `exo-resource-tree`) requires the `../ruvector/` sibling checkout to exist. This breaks:
- `cargo dist` in CI (no sibling repo)
- `cargo install weftos` from crates.io
- Any clone-and-build from the public repo alone

All other ruvector deps are properly feature-gated behind optional features.

### 2.4 Binary Targets

| Binary | Source Crate | Purpose |
|--------|-------------|---------|
| `weft` | clawft-cli | CLI tool |
| `weaver` | clawft-weave | Operator/daemon CLI |
| `weftos` | weftos | Kernel daemon binary |

The `weftos` crate workspace exclude includes `gui/src-tauri` which is correct (Tauri builds separately).

---

## 3. Blocker List (Ordered by Severity)

### SEVERITY: CRITICAL (must fix before v0.1.0 tag)

**B1. exo-resource-tree non-optional rvf-crypto dependency**
- `exo-resource-tree` requires `rvf-crypto` unconditionally
- `weftos` depends on `exo-resource-tree` unconditionally
- This makes standalone builds impossible
- **Fix**: Feature-gate `rvf-crypto` behind an optional `signing` or `merkle-crypto` feature in exo-resource-tree. Provide a fallback (e.g., SHA256 via `sha2` crate, already in workspace deps). [HP-8: Which hash function should exo-resource-tree use when rvf-crypto is not available?]

**B2. No `publish = false` on Tier 3 crates**
- 13 crates that should never be published to crates.io lack `publish = false`
- Risk: accidental publication of internal crates
- **Fix**: Add `publish = false` to: clawft-cli, clawft-weave, clawft-wasm, clawft-channels, clawft-services, clawft-plugin-git, clawft-plugin-cargo, clawft-plugin-oauth2, clawft-plugin-treesitter, clawft-plugin-browser, clawft-plugin-calendar, clawft-plugin-containers

**B3. No `version` in `[workspace.package]` -- version not centralized**
- Each crate hardcodes `version = "0.1.0"` instead of inheriting `version.workspace = true`
- Makes lockstep versioning manual and error-prone
- **Fix**: Add `version = "0.1.0"` to `[workspace.package]`, change all 22 crates to `version.workspace = true`

### SEVERITY: HIGH (should fix before v0.1.0 tag)

**B4. CHANGELOG.md is stale**
- Documents Sprint 1-5 only. Missing Sprint 6-10 work (self-healing, persistence, mesh, DEMOCRITUS, tool signing, WASM shell, K8 GUI, 983 new tests)
- **Fix**: Regenerate via git-cliff or manual curation

**B5. No cargo-dist configuration**
- No `[dist]` section in workspace Cargo.toml, no `dist-workspace.toml`
- Cannot produce cross-platform binaries or install scripts
- **Fix**: Run `cargo dist init`

**B6. release.yml only builds Docker**
- No native binary artifacts on GitHub Releases
- **Fix**: cargo-dist will generate a replacement workflow

### SEVERITY: MEDIUM (acceptable to fix in 0.1.1)

**B7. WASM target mismatch: wasip1 vs wasip2**
- `scripts/build.sh` targets `wasm32-wasip1`
- `.github/workflows/pr-gates.yml` and `wasm-build.yml` target `wasm32-wasip2`
- **Fix**: Align on one target. wasip2 is the forward-looking choice. [HP-9: Should we standardize on wasip1 or wasip2?]

**B8. Dockerfile only builds `weft`, not `weaver` or `weftos`**
- Strategy doc specifies 3 binaries in Docker
- **Fix**: Add `weaver` and `weftos` to builder stage, or create separate Dockerfiles

**B9. No `cargo audit` or `cargo deny` in CI**
- No supply-chain security checks
- **Fix**: Add to pr-gates.yml

**B10. PR gates target `main` but default branch is `master`**
- `pr-gates.yml` triggers on `branches: [main, master]` -- works but messy
- `benchmarks.yml` and `wasm-build.yml` trigger on `main` only -- may not fire
- **Fix**: Standardize on `master` across all workflows

### SEVERITY: LOW (post-v0.1.0)

**B11. Dockerfile runtime base should be distroless** (Phase 2 per strategy)
**B12. No `.github/release.yml` for PR categorization in release notes**
**B13. No `ROADMAP.md` or `MIGRATION.md`**
**B14. Volume path `.clawft` should be `.weftos` for branding**

---

## 4. v0.1.0 Release Checklist

### Phase 0: Fix Blockers (estimated: 2-3 hours)

```bash
# B1: Feature-gate rvf-crypto in exo-resource-tree
# Edit crates/exo-resource-tree/Cargo.toml:
#   [features]
#   default = ["rvf-signing"]
#   rvf-signing = ["dep:rvf-crypto"]
#   [dependencies]
#   rvf-crypto = { workspace = true, optional = true }
# Then provide fallback hash impl when feature is off.
# Ensure weftos crate enables exo-resource-tree/rvf-signing when cluster feature is on.

# B2: Add publish = false to Tier 3 crates (13 crates)
for crate in clawft-cli clawft-weave clawft-wasm clawft-channels clawft-services \
  clawft-plugin-git clawft-plugin-cargo clawft-plugin-oauth2 \
  clawft-plugin-treesitter clawft-plugin-browser clawft-plugin-calendar \
  clawft-plugin-containers; do
  # Add 'publish = false' after version line in each Cargo.toml
  echo "Add publish = false to crates/$crate/Cargo.toml"
done

# B3: Centralize version in workspace
# Add to root Cargo.toml [workspace.package]:
#   version = "0.1.0"
# Then in each crate, change:
#   version = "0.1.0"  -->  version.workspace = true
```

### Phase 1: Prepare Release Artifacts (estimated: 1-2 hours)

```bash
# Step 1: Install release tooling
cargo install cargo-dist git-cliff cargo-workspaces

# Step 2: Initialize cargo-dist
cd /claw/root/weavelogic/projects/clawft
cargo dist init
# Configure: 5 platform targets, install scripts, Homebrew formula
# This generates .github/workflows/release.yml (replaces existing)
# and adds [dist] to workspace Cargo.toml

# Step 3: Create cliff.toml for changelog generation
# (See strategy doc section 7 for full config)

# Step 4: Generate changelog covering Sprint 1-10
git cliff --output CHANGELOG.md

# Step 5: Verify build
scripts/build.sh gate
```

### Phase 2: Validate (estimated: 30 min)

```bash
# Verify workspace builds with no ruvector features
cargo check --workspace
cargo test --workspace

# Verify release binary builds
cargo build --release --bin weft --bin weaver --bin weftos

# Verify Docker still builds
docker build -t weftos:pre-release .

# Dry-run cargo-dist
cargo dist build
cargo dist plan
```

### Phase 3: Tag and Release (estimated: 15 min)

```bash
# Create release commit
git add -A
git commit -m "release: prepare v0.1.0 -- version centralization, publish gates, changelog"

# Create annotated tag
git tag -a v0.1.0 -m "WeftOS v0.1.0: Foundation Release

22-crate Rust workspace. Kernel layers K0-K6 complete.
5,040 tests. 181,703 lines of Rust.
Self-healing, persistence, mesh networking, DEMOCRITUS cognitive loop,
tool signing, WASM sandbox, K8 GUI prototype."

# Push tag (triggers cargo-dist release workflow)
git push origin v0.1.0
```

### Phase 4: Post-Release Verification (estimated: 30 min)

```bash
# Verify GitHub Release was created with artifacts
gh release view v0.1.0

# Verify Docker image was pushed
docker pull ghcr.io/clawft/clawft:0.1.0

# Verify install script works
curl -fsSL https://install.weftos.dev | sh  # (after DNS setup)

# Verify cargo binstall works
cargo binstall weftos  # (after crates.io publish)
```

---

## 5. Post-v0.1.0 Roadmap (v0.2 prep)

### Sprint 11 (immediately after v0.1.0 tag)

| # | Action | Effort | Owner |
|---|--------|--------|-------|
| 1 | Set up `release-plz` for automated release PRs | 2h | cicd-engineer |
| 2 | Add `cargo-semver-checks` to pr-gates.yml | 30m | cicd-engineer |
| 3 | Add `cargo audit` to pr-gates.yml | 30m | cicd-engineer |
| 4 | Fix WASM target mismatch (wasip1 vs wasip2) | 1h | repo-architect |
| 5 | Reserve `weftos-*` crate names on crates.io | 30m | release-manager |
| 6 | Create `weavelogic/homebrew-tap` repo | 1h | release-manager |
| 7 | Standardize CI branch targets to `master` | 15m | cicd-engineer |
| 8 | Resolve ruvector deps for crates.io publish (feature-gate default set) | 4h | repo-architect |
| 9 | Publish Tier 1 crates to crates.io | 2h | release-manager |
| 10 | Switch Docker base to distroless | 1h | cicd-engineer |
| 11 | Add `weaver` and `weftos` to Dockerfile | 30m | cicd-engineer |
| 12 | Create `MIGRATION.md` (empty, structured) | 15m | pr-manager |
| 13 | Create GitHub Milestones (v0.1.0 closed, v0.2.0 open) | 15m | pr-manager |

### Sprint 12-14 (v0.2 cycle)

- Tauri desktop app builds (tauri-action in CI)
- npm WASM package publishing (@weftos/core)
- npm binary wrapper packages
- Nix flake
- AUR PKGBUILD
- macOS notarization
- release-plz fully operational (automated release PRs)

---

## 6. ECC Contribution -- Causal Nodes and Edges for CMVG

```
NODES (new, extending 00-opening-plenary.md baseline):

  [N7] Release Infrastructure Audited
       status: ACHIEVED
       evidence: 4 workflows, Dockerfile, build.sh, CHANGELOG exist
       gaps: 14 blockers identified (3 critical, 4 high, 4 medium, 3 low)

  [N8] Ruvector Path Dependency Resolved
       status: BLOCKED
       blocker: exo-resource-tree non-optional rvf-crypto (B1)
       resolution: feature-gate rvf-crypto behind optional feature

  [N9] Workspace Version Centralized
       status: PENDING
       action: Add version to [workspace.package], migrate 22 crates

  [N10] cargo-dist Configured
       status: PENDING
       depends_on: N8, N9
       action: cargo dist init, replace release.yml

  [N11] CHANGELOG Regenerated
       status: PENDING
       action: git-cliff or manual Sprint 6-10 curation

  [N12] Crate Names Reserved (crates.io)
       status: PENDING
       action: Publish 0.0.1 placeholders for weftos-* names

  [N13] Tier 3 Publish Gates Set
       status: PENDING
       action: Add publish = false to 13 crates

EDGES (new):

  N1 --[Enables]-->  N7    Kernel complete allows release audit
  N7 --[Reveals]-->  N8    Audit found rvf-crypto blocker
  N7 --[Reveals]-->  N9    Audit found version centralization gap
  N8 --[Enables]-->  N10   Path dep fix unblocks cargo-dist
  N9 --[Enables]-->  N10   Centralized version needed for cargo-dist
  N13 --[Enables]--> N10   Publish gates prevent accidental publish
  N10 --[Enables]--> N5    cargo-dist config enables v0.1.0 tag
  N11 --[Enables]--> N5    Changelog must exist before tag
  N12 --[Parallel]   N10   Name reservation is independent, do in parallel

CAUSAL CHAIN (release critical path):
  N1 (achieved) --> N7 (achieved) --> N8 (blocked) --> N10 (pending) --> N5 (blocked)
                                  --> N9 (pending) -/
                                  --> N11 (pending) --> N5
                                  --> N13 (pending) --> N10

MINIMUM UNBLOCK SEQUENCE:
  1. Fix B1 (exo-resource-tree rvf-crypto)     --> N8 resolved
  2. Fix B3 (centralize version)                --> N9 resolved
  3. Fix B2 (publish = false on Tier 3)         --> N13 resolved
  4. Run cargo dist init                        --> N10 resolved
  5. Regenerate CHANGELOG                       --> N11 resolved
  6. Tag v0.1.0                                 --> N5 resolved
```

---

## 7. High-Priority Questions

**[HP-8]** (NEW) `exo-resource-tree` uses `rvf-crypto` for Merkle hash integrity. When `rvf-crypto` is feature-gated off, what hash function should be the fallback? Options:
- (a) `sha2::Sha256` (already in workspace deps)
- (b) `blake3` (already in workspace deps, faster)
- (c) Compile error if neither `rvf-crypto` nor fallback enabled (safest but less ergonomic)

Recommendation: (b) blake3 -- it is already used by the ECC cognitive substrate, is faster than SHA256, and is already a workspace dependency.

**[HP-9]** (NEW) WASM target inconsistency: `scripts/build.sh` uses `wasm32-wasip1`, CI uses `wasm32-wasip2`. Which should be canonical?
- wasip1: More widely supported today, simpler
- wasip2: Component model, forward-looking, but tooling is newer

Recommendation: wasip2 for CI and release artifacts (forward-looking), wasip1 support retained in build.sh as a secondary target for compatibility.

**[HP-10]** (NEW) The GitHub repository URL in workspace Cargo.toml is `https://github.com/clawft/clawft`. Is this the correct org/repo for the public release, or should it be under `weavelogic`?

**[HP-11]** (NEW) crates.io name reservation: Should we reserve names now on the current crates.io account, or wait until the GitHub org and branding are finalized? Reservation is first-come-first-served and names cannot be transferred between accounts easily.

---

## 8. Summary

The release infrastructure is approximately 60% built. The CI gates are strong (7-job PR validation, benchmarks, WASM size gates). Docker build works. The build script is comprehensive. What is missing is the last-mile distribution: cargo-dist for cross-platform binaries, centralized versioning, publish gates, and one critical path dependency blocker (exo-resource-tree's unconditional rvf-crypto dep).

The minimum path to v0.1.0 tag requires fixing 3 critical blockers (B1, B2, B3), running `cargo dist init`, regenerating the changelog, and pushing a tag. Estimated total effort: 4-5 hours of focused work.

The release strategy document (.planning/sparc/weftos/0.1/11-release-strategy.md) is thorough and well-researched. This panel's findings are consistent with that strategy. The gap is execution: none of the Phase 1 actions from that document have been implemented yet.
