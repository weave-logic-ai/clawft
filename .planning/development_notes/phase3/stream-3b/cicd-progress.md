# Stream 3B: CI/CD Progress Tracker

**Status**: In Progress (Round 4)
**Started**: 2026-02-17
**Depends On**: cicd-setup.md (architecture decisions)
**Last Updated**: 2026-02-17 (Round 4)

## Round 1 Deliverables (Complete)

| Artifact | Path | Description |
|----------|------|-------------|
| CI workflow | `.github/workflows/ci.yml` | Check + Test + Build matrix (5 targets) |
| Release workflow | `.github/workflows/release.yml` | Tag-triggered multi-platform release |
| Dockerfile | `clawft/Dockerfile` | `FROM scratch` static musl binary |
| Startup benchmark | `clawft/scripts/bench/startup.sh` | Average of 10 cold starts |
| Memory benchmark | `clawft/scripts/bench/memory.sh` | Peak RSS via `/usr/bin/time -v` |
| Throughput benchmark | `clawft/scripts/bench/throughput.sh` | CLI invocation rate over 100 iterations |
| Size check | `clawft/scripts/bench/size-check.sh` | Binary size vs target limits |
| Bench runner | `clawft/scripts/bench/run-all.sh` | Orchestrates all benchmark scripts |

## Round 2 Deliverables (Complete)

| Artifact | Path | Description |
|----------|------|-------------|
| Benchmarks CI | `.github/workflows/benchmarks.yml` | Automated benchmark runs on push/PR |
| WASM build CI | `.github/workflows/wasm-build.yml` | WASM compilation + size check |
| Cross-compile script | `clawft/scripts/cross-compile.sh` | Local multi-target build script |
| Docker build script | `clawft/scripts/docker-build.sh` | Local Docker image build + tag |

## Architecture Decisions

### CI Pipeline Structure

```
Push/PR to main
  |
  +-- ci.yml
  |     +-- Stage 1: check (fmt, clippy, cargo check)
  |     +-- Stage 2: test (cargo test --workspace)
  |     +-- Stage 3: build matrix
  |           +-- linux-x86_64-musl (cross)
  |           +-- linux-aarch64-musl (cross)
  |           +-- macos-x86_64 (macos-13 runner)
  |           +-- macos-aarch64 (macos-14 runner)
  |           +-- windows-x86_64 (windows-latest)
  |
  +-- benchmarks.yml
  |     +-- Build release binary
  |     +-- Run startup, memory, throughput, size benchmarks
  |     +-- Post results as PR comment (if PR)
  |
  +-- wasm-build.yml
        +-- Install wasm32-wasip1 target (changed from wasip2 in R4)
        +-- cargo check --target wasm32-wasip1
        +-- Size check (< 300 KB)

Tag push (v*)
  |
  +-- release.yml
        +-- Build all 5 native targets + WASM
        +-- Package zips with binary + docs
        +-- SHA256 checksums
        +-- GitHub Release with artifacts
        +-- Docker image to GHCR
```

### Docker Strategy

- **Base**: `FROM scratch` (no OS layer)
- **Binary**: Static musl-linked `weft` binary
- **Size target**: < 20 MB
- **Multi-arch**: Deferred to later iteration (requires buildx + manifest lists)

### Benchmark CI Strategy

- Run on every push to main and on PRs
- Uses `ubuntu-latest` runner for consistency
- Builds release binary first, then runs all 4 benchmarks
- Results posted as PR comment for visibility
- Historical tracking via GitHub Actions artifacts

### Cross-Compile Script

- Wraps `cross` for Linux musl targets (x86_64, aarch64)
- Falls back to native `cargo build` for host architecture
- Used for local development and testing before pushing

## Round 3 Deliverables (Complete)

| Artifact | Path | Description |
|----------|------|-------------|
| Release packaging script | `clawft/scripts/release-package.sh` | Per-platform zip with binary + docs + SHA256 checksums |
| CHANGELOG | `clawft/CHANGELOG.md` | Keep a Changelog format, covers Phase 1-3 |
| Benchmark report (populated) | `.planning/development_notes/report_benchmarks.md` | Real benchmark data from local runs |
| CLI integration tests | `clawft/tests/cli_integration.rs` | End-to-end binary tests via `assert_cmd` |
| Deployment docs | `clawft/docs/deployment/*.md` | Docker, WASM, and release process guides |
| Security docs | `clawft/docs/security.md` | Command allowlist, SSRF protection reference |
| Crate metadata | `*/Cargo.toml` (9 crates) | Keywords, categories, descriptions, repository |

## Round 4 Deliverables (In Progress)

| Artifact | Path | Description |
|----------|------|-------------|
| WASM CI fix | `.github/workflows/wasm-build.yml` | Update to wasip1 target, YAML validation |
| Benchmark regression scripts | `clawft/scripts/bench/regression-check.sh` (est.) | Threshold-based regression detection |
| Benchmark regression CI | `.github/workflows/benchmarks.yml` update | Integrate regression detection into CI |
| Release validation | `clawft/scripts/release-package.sh` end-to-end test | Full build-package-checksum-verify cycle |
| CLI test enhancements | `clawft/tests/cli_integration.rs` | Additional coverage for edge cases |

### Release Packaging Strategy

- **Format**: Per-platform zip archives: `weft-{version}-{target}.zip`
- **Contents**: Binary + README.md + LICENSE + CHANGELOG.md
- **Checksums**: `checksums.sha256` with SHA256 hashes for all archives
- **Script**: `release-package.sh` wraps the build + package + checksum generation

### CHANGELOG Format

- Uses [Keep a Changelog](https://keepachangelog.com/) format
- Sections: Added, Changed, Fixed, Security
- Covers all phases (1, 2, 3) in the initial `[0.1.0]` entry
- Will be included in release zip packages

### Benchmark Results

- Benchmarks executed locally via `scripts/bench/run-all.sh`
- Results populate `report_benchmarks.md` (previously all "pending")
- CI workflow (`benchmarks.yml`) posts results as PR comments
- Historical tracking via GitHub Actions artifacts

## Remaining Work

| Item | Priority | Notes |
|------|----------|-------|
| WASM target in release.yml | Medium | Blocked on Stream 3A completion (wasip1 compilation) |
| WASM CI workflow fix (wasip1) | High | R4 in progress -- update target in wasm-build.yml |
| Benchmark regression detection | Medium | R4 in progress -- scripts + CI integration |
| Multi-arch Docker images | Low | Requires buildx setup |
| Caching optimization | Low | Tune `rust-cache` settings for faster CI |
| macOS code signing | Low | Required for distribution, not for CI |
