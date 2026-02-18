# Stream 3B: CI/CD + Polish -- Development Notes

**Status**: In Progress
**Started**: 2026-02-17

## Objective

Automated CI/CD pipeline with multi-platform builds, release packaging, and benchmarks.

## Decisions

### CI Workflow (ci.yml)

- 3 stages: check (fmt + clippy + check) -> test -> build matrix
- Build matrix: 5 native targets (linux x86/arm, macos intel/arm, windows)
- WASM build added separately later (Phase 3A dependency)
- Uses `cross` for Linux musl static builds
- Uses `Swatinem/rust-cache` for caching

### Release Workflow (release.yml)

- Triggered on `v*` tags
- Builds all targets in parallel
- Creates zip packages with binary + docs for each platform
- Generates SHA256 checksums
- Creates GitHub Release with all artifacts
- Builds and pushes Docker image to GHCR

### Docker Strategy

- `FROM scratch` with static musl binary
- No runtime dependencies needed
- Target image size: < 20 MB

### Benchmarks

- 4 benchmarks: startup time, memory RSS, CLI throughput, binary size
- Bash scripts in `scripts/bench/`
- Report at `.planning/development_notes/report_benchmarks.md`
- CI integration planned for release workflow

## Challenges

- Cross-compilation for ARM64 Linux requires `cross` tool
- macOS builds need separate Intel (macos-13) and ARM (macos-14) runners
- WASM build depends on Phase 3A completion
- Docker multi-arch images deferred to later iteration

## GitHub Actions Notes

- Workflows at repo root `.github/workflows/` (not in clawft/ subdir)
- `working-directory: clawft` set on all steps that use Cargo
- Artifact paths must include `clawft/` prefix when relative to repo root
- `rust-cache` uses `workspaces: "clawft -> target"` for proper caching
