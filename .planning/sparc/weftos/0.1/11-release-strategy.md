# WeftOS Release & Distribution Strategy

**Document ID**: 11
**Workstream**: W-DEPLOY
**Date**: 2026-03-27
**Priority**: P0
**Goal**: Define versioning, build outputs, distribution channels, and the path from "repo on a server" to "anyone can install WeftOS in 10 seconds"

---

## 1. Versioning Strategy

### Scheme: Workspace-Level Semver (Lockstep)

All 22 crates share a single version via `[workspace.package] version`:

| Version | Milestone | Sprint | Theme |
|---------|-----------|--------|-------|
| **0.1.0** | Kernel complete, CLI works, WASM builds | 10-11 | Foundation |
| **0.2.0** | K8 GUI (Tauri), operational hardening | 12-14 | Operational Maturity |
| **0.3.0** | Features TBD, Plugin marketplace, multi-tenant | 15-17 | Enterprise Readiness |
| **1.0.0** | Stable API, security audit, production users | 18+ | Stable Release |

**Cargo 0.x convention**: Breaking changes bump minor (0.1 → 0.2), non-breaking bump patch (0.1.0 → 0.1.1).

**1.0 criteria** (all must be true):
- Public API stable across 2+ minor releases without breaking changes
- Production users depending on API stability
- Security audit completed
- All public APIs documented
- Migration path from every 0.x documented

### Workspace Cargo.toml Change

```toml
[workspace.package]
version = "0.1.0"   # Single source of truth
```

Each crate inherits:
```toml
[package]
version.workspace = true
```

---

## 2. Product Naming: WeftOS vs clawft

### The Split

| Name | What It Is | Published As | Binary |
|------|-----------|-------------|--------|
| **WeftOS** | The product — kernel OS, the thing customers buy | `weftos` on crates.io | `weftos` |
| **weft** | The CLI tool — primary user interface | `weft` on crates.io | `weft` |
| **weaver** | The operator CLI — kernel management | Internal (not published) | `weaver` |
| **clawft** | The framework — internal crate prefix | Internal workspace name | — |

### Crate Naming for Publishing

Published crates use the `weftos-` prefix (product brand). Internal crates keep `clawft-` (no rename churn).

| Current Name | Published Name | Tier |
|---|---|---|
| `weftos` | `weftos` | 1 — Facade/meta-crate |
| `clawft-types` | `weftos-types` | 1 |
| `clawft-core` | `weftos-core` | 1 |
| `clawft-kernel` | `weftos-kernel` | 1 |
| `clawft-plugin` | `weftos-plugin` | 1 — Plugin SDK for third parties |
| `exo-resource-tree` | `exo-resource-tree` | 1 — Standalone identity |
| `clawft-llm` | `weftos-llm` | 2 — Publish when demand exists |
| `clawft-tools` | `weftos-tools` | 2 |
| `clawft-security` | `weftos-security` | 2 |
| `clawft-platform` | `weftos-platform` | 2 |
| `clawft-channels` | `weftos-channels` | 2 |
| `clawft-cli` | — | 3 — `publish = false`, distribute as binary |
| `clawft-weave` | — | 3 — `publish = false`, distribute as binary |
| `clawft-wasm` | — | 3 — Distribute via npm, not crates.io |
| All `clawft-plugin-*` | — | 3 — `publish = false`, first-party plugins |
| `clawft-services` | — | 3 — Internal glue |

**Action**: Reserve names on crates.io NOW (publish empty 0.0.1 placeholders for all `weftos-*` names). Bevy does this explicitly.

---

## 3. Build Outputs

### 3.1 Native Binaries

| Binary | Source Crate | Purpose | Distributed |
|--------|-------------|---------|-------------|
| `weft` | `clawft-cli` | CLI tool (primary user interface) | Yes — all channels |
| `weftos` | `weftos` | Kernel daemon | Yes — all channels |
| `weaver` | `clawft-weave` | Operator CLI (kernel management) | Yes — binary releases only |

### 3.2 Platform Matrix

| Target | OS | Arch | CI Runner | Archive | Phase |
|--------|-----|------|-----------|---------|-------|
| `x86_64-unknown-linux-gnu` | Linux | x86_64 | `ubuntu-latest` | `.tar.gz` | 1 |
| `aarch64-unknown-linux-gnu` | Linux | aarch64 | `ubuntu-latest` + cross | `.tar.gz` | 1 |
| `x86_64-apple-darwin` | macOS | Intel | `macos-13` | `.tar.gz` | 1 |
| `aarch64-apple-darwin` | macOS | Apple Silicon | `macos-14` | `.tar.gz` | 1 |
| `x86_64-pc-windows-msvc` | Windows | x86_64 | `windows-latest` | `.zip` | 1 |
| `x86_64-unknown-linux-musl` | Linux | x86_64 static | `ubuntu-latest` + cross | `.tar.gz` | 1 |
| `aarch64-unknown-linux-musl` | Linux | aarch64 static | `ubuntu-latest` + cross | `.tar.gz` | 1 |
| `aarch64-pc-windows-msvc` | Windows | aarch64 | `windows-latest` | `.zip` | 1 |

Each archive contains: binary, LICENSE, shell completions (bash/zsh/fish), SHA256 checksum.

### 3.3 WASM Outputs

| Target | Output | Size Budget | Distribution |
|--------|--------|-------------|-------------|
| `wasm32-wasip1` | `clawft_wasm.wasm` | <300KB raw, <120KB gzip | GitHub Releases |
| `wasm32-unknown-unknown` | `clawft_wasm.wasm` | <300KB raw, <120KB gzip | npm (`@weftos/core`) |

### 3.4 Docker Images

| Image | Registry | Platforms | Base |
|-------|----------|-----------|------|
| `ghcr.io/weave-logic-ai/weftos` | GHCR (primary) | linux/amd64, linux/arm64 | `distroless/cc-debian12` |
| `docker.io/weftos/weftos` | Docker Hub (mirror) | linux/amd64, linux/arm64 | same |

**Tagging strategy**:
| Tag | Meaning | Mutable? |
|-----|---------|----------|
| `v0.1.0` | Exact version | Immutable |
| `0.1` | Latest patch in 0.1.x | Mutable |
| `0` | Latest in 0.x | Mutable |
| `latest` | Latest release | Mutable |
| `edge` | Latest commit on main | Mutable |

### 3.5 GUI (Tauri Desktop App) — Phase 3

| OS | Format | Notes |
|----|--------|-------|
| macOS | `.dmg` | Drag-to-Applications. Notarization recommended ($99/yr). |
| Windows | `.msi` (WiX) + `.exe` (NSIS) | SmartScreen warning without EV cert ($200-400/yr). |
| Linux | `.deb` + `.AppImage` | AppImage is universal, no install needed. |

Auto-updater via `tauri-plugin-updater` using GitHub Releases as backend. Ed25519 signature verification.

---

## 4. Distribution Channels

### Priority Order

| Priority | Channel | Command | Time to Running | Phase |
|----------|---------|---------|----------------|-------|
| **1** | GitHub Releases | Download binary | ~30 seconds | 1 |
| **2** | Install script | `curl -fsSL https://install.weftos.dev \| sh` | ~5 seconds | 1 |
| **3** | cargo binstall | `cargo binstall weftos` | ~10 seconds | 1 |
| **4** | Docker | `docker run ghcr.io/weave-logic-ai/weftos` | ~15 seconds | 1 |
| **5** | Homebrew | `brew install weavelogic/tap/weftos` | ~10 seconds | 2 |
| **6** | cargo install | `cargo install weftos` | ~3 minutes | 2 |
| **7** | npm (WASM) | `npm install @weftos/core` | ~10 seconds | 2 |
| **8** | npm (binary wrapper) | `npx weftos@latest` | ~10 seconds | 3 |
| **9** | Nix flake | `nix run github:weave-logic-ai/weftos` | ~seconds | 3 |
| **10** | AUR | `yay -S weftos-bin` | ~10 seconds | 3 |
| **11** | Tauri desktop | Download .dmg/.msi/.AppImage | ~30 seconds | 3 |

### What cargo-dist Gives Us for Free

Running `cargo dist init` generates:
- GitHub Actions release workflow (all 5+ platform targets)
- Shell installer script (`install.sh`)
- PowerShell installer script (`install.ps1`)
- Homebrew formula (auto-published to tap repo)
- npm binary wrapper packages (optional)
- SHA256 checksums + GitHub Attestations
- `cargo binstall` metadata

This is the single highest-leverage action for distribution.

---

## 5. Registry Publishing

### 5.1 crates.io

**Tooling**: `cargo-workspaces` for lockstep multi-crate publishing in dependency order.

```bash
cargo install cargo-workspaces
cargo ws publish --from-git --publish-interval 10
```

The `--publish-interval 10` avoids crates.io rate limits when publishing 10+ crates.

**Blocker**: 8 path dependencies to `../ruvector/` must be resolved:
- `rvf-runtime`, `rvf-types`, `rvf-crypto`, `rvf-wire`
- `ruvector-cluster`, `ruvector-raft`, `ruvector-replication`
- `cognitum-gate-tilezero`

**Resolution options** (pick one):
1. Publish ruvector crates to crates.io first (cleanest)
2. Vendor ruvector into the workspace (self-contained)
3. Feature-gate so default published crate never pulls ruvector (fastest — already partially done)

**Recommendation**: Option 3 for 0.1.0 (ship fast), then option 1 for 0.2.0 (clean up).

### 5.2 npm

| Package | Contents | Source |
|---------|----------|--------|
| `@weftos/core` | WASM module for browser | `clawft-wasm` + `browser` feature via `wasm-pack` |
| `@weftos/wasi` | WASI module for Node/Deno | `clawft-wasm` without browser feature |
| `weftos` (Phase 3) | Binary wrapper (esbuild pattern) | Platform-specific optional deps |

Phase 3 binary wrapper packages: `@weftos/cli-linux-x64`, `@weftos/cli-linux-arm64`, `@weftos/cli-darwin-x64`, `@weftos/cli-darwin-arm64`, `@weftos/cli-win32-x64`.

### 5.3 Docker

Already working. Improvements:
- Switch runtime base from `debian:bookworm-slim` to `gcr.io/distroless/cc-debian12` (smaller, more secure)
- Add Docker Hub mirror push alongside GHCR
- Add `edge` tag for main branch builds

### 5.4 Homebrew

Create `weavelogic/homebrew-tap` repo. cargo-dist auto-publishes formula on each release.

---

## 6. Release Automation Toolchain

### Core Tools

| Tool | Role | Replaces |
|------|------|----------|
| **cargo-dist** | Build artifacts, installers, GitHub Releases | Hand-rolled CI matrix |
| **git-cliff** | Generate CHANGELOG.md from conventional commits | Manual changelog |
| **release-plz** | Automate version bumps + release PRs + crates.io publish | Manual tagging + cargo publish |
| **cargo-semver-checks** | CI gate: catch accidental breaking changes | Manual review |
| **cargo-workspaces** | Lockstep workspace publishing | Per-crate manual publish |
| **cross** | Linux cross-compilation (aarch64, musl) | QEMU in Docker |

### Release Flow

```
Developer pushes conventional commits to main
        │
        ▼
release-plz detects changes, creates Release PR
  (bumps version in Cargo.toml, updates CHANGELOG.md via git-cliff)
        │
        ▼
CI runs on Release PR:
  ├── cargo-semver-checks (breaking change guard)
  ├── scripts/build.sh gate (11 checks)
  └── PR review
        │
        ▼
Merge Release PR → release-plz creates git tag (v0.1.0)
        │
        ▼
Tag triggers cargo-dist workflow:
  ├── Build 5 platform binaries (cross for Linux ARM)
  ├── Build WASM targets
  ├── Build Docker images (multi-arch)
  ├── Generate install scripts
  ├── Create GitHub Release with artifacts + notes
  ├── Publish Homebrew formula to tap repo
  └── (Optional) Publish npm packages
        │
        ▼
release-plz publishes crates to crates.io (dependency order, 10s intervals)
```

---

## 7. Version Documentation

### Files to Maintain

| File | Purpose | When to Update |
|------|---------|---------------|
| `CHANGELOG.md` | Technical change log (Keep a Changelog format) | Auto-generated by git-cliff |
| `MIGRATION.md` | Upgrade guides between versions | Every minor bump with breaking changes |
| `ROADMAP.md` | Version → sprint → theme mapping, 1.0 criteria | Each sprint planning session |
| `.github/release.yml` | PR categorization for auto-generated release notes | Once, then rarely |
| `cliff.toml` | git-cliff configuration | Once, then rarely |

### Changelog Automation (git-cliff)

```toml
# cliff.toml
[changelog]
header = """
# Changelog\n
All notable changes to WeftOS will be documented in this file.\n
"""
body = """
{% if version %}\
    ## [{{ version | trim_start_matches(pat="v") }}] - {{ timestamp | date(format="%Y-%m-%d") }}
{% else %}\
    ## [Unreleased]
{% endif %}\
{% for group, commits in commits | group_by(attribute="group") %}
    ### {{ group | upper_first }}
    {% for commit in commits %}
        - {{ commit.message | upper_first }}\
    {% endfor %}
{% endfor %}\n
"""

[git]
conventional_commits = true
commit_parsers = [
    { message = "^feat", group = "Added" },
    { message = "^fix", group = "Fixed" },
    { message = "^doc", group = "Documentation" },
    { message = "^perf", group = "Performance" },
    { message = "^refactor", group = "Changed" },
    { message = "^security", group = "Security" },
    { message = "^chore", group = "Miscellaneous" },
    { message = "^plan", skip = true },
    { message = "^data", skip = true },
    { message = "^review", skip = true },
]
tag_pattern = "v[0-9].*"
```

### Docs Versioning

**Do NOT version docs at 0.x.** Single "latest" Fumadocs site tracking main. Add a banner: "These docs are for the development version (0.x). APIs may change." Start versioning docs at 1.0 or when users need to pin to specific versions.

### Stability Notice (add to README)

```
## Stability

WeftOS is in active development (v0.x). The API may change between minor
versions. We follow Cargo's semver conventions for 0.x: breaking changes
bump the minor version (0.1 → 0.2), non-breaking changes bump the patch
(0.1.0 → 0.1.1). See MIGRATION.md for upgrade guides.
```

---

## 8. Implementation Phases

### Phase 1: "Ship 0.1.0" (1-2 days)

| # | Action | Effort |
|---|--------|--------|
| 1 | Add `version.workspace = true` to all crate Cargo.tomls | 30 min |
| 2 | Add `publish = false` to Tier 3 crates | 15 min |
| 3 | Reserve `weftos-*` names on crates.io (empty 0.0.1 placeholders) | 30 min |
| 4 | Run `cargo dist init` — configure 8 native targets + 2 WASM + installers | 1 hour |
| 5 | Add `[package.metadata.binstall]` to CLI crate | 5 min |
| 6 | Create `cliff.toml` | 15 min |
| 7 | Create `ROADMAP.md` with version-sprint mapping | 30 min |
| 8 | Add stability notice to README | 5 min |
| 9 | Build + verify musl static binaries (`ldd` = "not dynamic") | 30 min |
| 10 | Build + verify WASI binary (`wasmtime run weftos.wasm`) | 30 min |
| 11 | Tag `v0.1.0` and push — cargo-dist builds everything | 5 min |
| 12 | Write narrative 0.1.0 release notes on GitHub Release | 30 min |

### Phase 2: "Broaden Reach" (1 week, during Sprint 11)

| # | Action | Effort |
|---|--------|--------|
| 11 | Set up `release-plz` for automated release PRs | 2 hours |
| 12 | Add `cargo-semver-checks` to CI | 30 min |
| 13 | Create `weavelogic/homebrew-tap` repo (cargo-dist auto-publishes) | 1 hour |
| 14 | Publish `@weftos/core` WASM to npm via wasm-pack | 2 hours |
| 15 | Resolve ruvector path deps (feature-gate default set) | 4 hours |
| 16 | Publish Tier 1 crates to crates.io | 2 hours |
| 17 | Switch Docker base to distroless, add Docker Hub mirror | 1 hour |
| 18 | Create `MIGRATION.md` (empty, structured for 0.2.0) | 15 min |
| 19 | Create GitHub Milestones (v0.1.0 closed, v0.2.0 open) | 15 min |
| 20 | Add Nix flake (`flake.nix`) | 1 hour |
| 21 | Create AUR `PKGBUILD` for `weftos-bin` | 30 min |

### Phase 3: "Desktop + Full Ecosystem" (Sprint 12-14, with K8 GUI)

| # | Action | Effort |
|---|--------|--------|
| 22 | Wrap gui/ with Tauri 2.0 (`tauri init`) | 1 day |
| 23 | Add `tauri-action` to CI (.dmg, .msi, .deb, .AppImage) | 1 day |
| 24 | Configure `tauri-plugin-updater` with GitHub Releases | 4 hours |
| 25 | npm binary wrapper packages (esbuild pattern) | 2 days |
| 26 | macOS notarization ($99/yr) | 2 hours |
| 27 | Windows code signing (optional, $200-400/yr) | 2 hours |
| 28 | Publish Tier 2 crates to crates.io | 2 hours |
| 29 | Start versioned docs (if user base warrants) | 1 day |

---

## 9. CI/CD Architecture

### Workflows

| Workflow | Trigger | What It Does |
|----------|---------|-------------|
| `pr-gates.yml` | PR to main | Clippy, tests, WASM size, binary size, smoke test (already exists) |
| `release.yml` | Tag `v*.*.*` | cargo-dist: binaries, Docker, installers, GitHub Release, Homebrew |
| `nightly.yml` | Cron (daily) | Build `edge` Docker tag, optional binary snapshots |
| `semver-check.yml` | PR to main | cargo-semver-checks on published crates |

### Size Budgets (already enforced)

| Artifact | Budget |
|----------|--------|
| `weft` native binary | <10 MB |
| WASM (raw) | <300 KB |
| WASM (gzipped) | <120 KB |
| Docker image (compressed) | <50 MB |

---

## 10. Key Decisions

| Decision | Resolution | Rationale |
|----------|-----------|-----------|
| Lockstep vs independent versioning | **Lockstep** | Small team, all crates tightly coupled. Bevy/Dioxus pattern. |
| cargo-dist vs hand-rolled CI | **cargo-dist** | Eliminates weeks of CI debugging. Ruff proves it scales. |
| release-please vs release-plz | **release-plz** | Rust-native, integrates with git-cliff, handles cargo publish. |
| Rename crates for publishing | **Yes, weftos-* prefix** | Product brand > repo name for public API surface. |
| Reserve crate names | **Yes, immediately** | crates.io has no namespaces. First-come-first-served. |
| Version docs at 0.x | **No** | Too much overhead. Single "latest" site until 1.0. |
| Docker base image | **distroless/cc-debian12** | Smallest, most secure. No shell = smaller attack surface. |
| When to go 1.0 | **When API stable for 2+ minors + security audit** | Don't promise stability you can't maintain. |

---

## References

### Tools
- [cargo-dist](https://github.com/axodotdev/cargo-dist) — Release artifact generation
- [git-cliff](https://git-cliff.org/) — Changelog generation
- [release-plz](https://crates.io/crates/release-plz) — Automated release PRs + publish
- [cargo-semver-checks](https://github.com/obi1kenobi/cargo-semver-checks) — Breaking change detection
- [cargo-workspaces](https://crates.io/crates/cargo-workspaces) — Workspace publishing
- [cross](https://github.com/cross-rs/cross) — Cross-compilation
- [wasm-pack](https://rustwasm.github.io/wasm-pack/) — WASM → npm publishing
- [tauri-action](https://github.com/tauri-apps/tauri-action) — Tauri CI builds

### Patterns Studied
- **Zellij**: Workspace-level lockstep versioning
- **Helix**: CalVer, hand-curated changelogs
- **Nushell**: cargo binstall metadata, nightly builds, broad platform matrix
- **Starship**: release-please + cross, install script gold standard
- **Ruff**: cargo-dist at scale, npm binary wrappers, PyPI distribution
- **Tauri**: Covector for mixed Rust+JS, desktop app bundling + auto-update
- **Bevy**: Facade crate pattern, crate name reservation
