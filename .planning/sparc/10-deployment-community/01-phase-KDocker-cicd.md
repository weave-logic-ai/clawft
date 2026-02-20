# Phase K-Docker: Multi-Arch Docker Images & CI/CD Pipelines

> **Element:** 10 -- Deployment & Community
> **Phase:** K-Docker (K2, K2-CI)
> **Timeline:** Week 8-9
> **Priority:** P1
> **Crates:** `clawft-cli` (binary target), `clawft-wasm` (size gate)
> **Dependencies IN:** None (standalone infrastructure phase)
> **Blocks:** K3 (sandbox needs container runtime), K4 (ClawHub needs GHCR publish), K5 (benchmark suite needs CI runner)
> **Status:** Planning
> **Orchestrator Ref:** `10-deployment-community/00-orchestrator.md` Section 2 (Phase K-Docker)
> **Dev Assignment Ref:** `02-improvements-overview/dev-assignment-10-deployment-community.md` Unit 1

---

## 1. Overview

This phase replaces the current `FROM scratch` Docker image with a production-grade multi-arch
container build, establishes CI/CD pipelines for quality gates, and provides one-click VPS
deployment scripts.

### Current State

The existing Dockerfile (`Dockerfile`, 12 lines) uses `FROM scratch` with a statically linked
musl binary. This approach cannot support wasmtime, which requires glibc for its JIT compiler
and signal handling. The current `scripts/build/docker-build.sh` targets only `x86_64-unknown-linux-musl`
and builds a single-arch image.

Existing CI workflows cover benchmarks (`benchmarks.yml`, 108 lines) and WASM build/size gate
(`wasm-build.yml`, 130 lines), but there are no PR gate workflows for clippy, workspace tests,
binary size, or Docker integration tests.

### Deliverables

| ID | Deliverable | Description |
|----|-------------|-------------|
| K2-D1 | Multi-stage Dockerfile | 4-stage build with `cargo-chef`, targeting `debian:bookworm-slim` runtime |
| K2-D2 | Multi-arch build script | `scripts/build/docker-multiarch.sh` supporting linux/amd64 + linux/arm64 via `docker buildx` |
| K2-D3 | VPS deployment scripts | `scripts/deploy/vps-setup.sh` and `scripts/deploy/vps-update.sh` for one-click deployment |
| K2-CI-D4 | PR gates workflow | `.github/workflows/pr-gates.yml` with clippy, test, WASM size, binary size checks |
| K2-CI-D5 | Release pipeline | `.github/workflows/release.yml` with multi-arch Docker build and GHCR push on semver tag |
| K2-CI-D6 | Integration smoke test | Docker-based health check in CI, verifiable locally |

---

## 2. Specification

### 2.1 K2: Multi-Stage Dockerfile

**Replace:** `Dockerfile` (project root, currently 12 lines)

The new Dockerfile uses four stages to maximize layer caching and minimize the final image size.
The key change from the current approach is switching from `FROM scratch` + musl to
`debian:bookworm-slim` + glibc, which is required for wasmtime support.

#### Build Stage Architecture

| Stage | Base Image | Purpose | Cache Layer |
|-------|-----------|---------|-------------|
| 1. `chef` | `rust:1.93-bookworm` | Install cargo-chef, prepare recipe | Toolchain |
| 2. `deps` | `rust:1.93-bookworm` | `cargo chef cook` -- build dependencies only | Dependencies (stable) |
| 3. `build` | `rust:1.93-bookworm` | `cargo build --release` -- build application code | Application (changes often) |
| 4. `runtime` | `debian:bookworm-slim` | Copy stripped binary + minimal runtime libs | Final image |

#### Size Budget

| Component | Target | Notes |
|-----------|--------|-------|
| Base image (bookworm-slim) | ~25MB compressed | Minimal Debian, glibc included |
| Release binary (stripped) | ~4.7MB | Current baseline from `scripts/bench/baseline.json` |
| Runtime libraries (libssl3, libgcc-s1, ca-certificates) | ~5MB | Required for TLS + wasmtime |
| **Total compressed** | **<50MB** | Enforced by CI assertion |

#### Dockerfile Requirements

| Requirement | Detail |
|-------------|--------|
| Base builder | `rust:1.93-bookworm` matching workspace `rust-version = "1.93"` from `Cargo.toml` |
| Runtime base | `debian:bookworm-slim` for glibc support (wasmtime requirement) |
| cargo-chef version | Latest stable (`cargo install cargo-chef`) |
| Strip binary | `strip --strip-all` in build stage (also configured in `[profile.release]` with `strip = true`) |
| Platform arg | `ARG TARGETPLATFORM` for `docker buildx` multi-arch |
| Non-root user | `useradd --system --no-create-home weft` in runtime stage |
| Config volume | `VOLUME ["/home/weft/.clawft"]` |
| Health check | `HEALTHCHECK --interval=30s --timeout=5s CMD ["/usr/local/bin/weft", "health"]` |
| Labels | OCI labels: `org.opencontainers.image.source`, `org.opencontainers.image.version` |
| Entrypoint | `ENTRYPOINT ["/usr/local/bin/weft"]`, `CMD ["gateway"]` |

#### Dockerfile

```dockerfile
# ============================================================================
# Stage 1: Chef -- install cargo-chef and prepare the dependency recipe
# ============================================================================
FROM rust:1.93-bookworm AS chef

RUN cargo install cargo-chef --locked
WORKDIR /app

# ============================================================================
# Stage 2: Deps -- cook dependencies from the recipe (cached layer)
# ============================================================================
FROM chef AS deps

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS cook

COPY --from=deps /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p clawft-cli

# ============================================================================
# Stage 3: Build -- compile the actual application
# ============================================================================
FROM cook AS build

COPY . .
RUN cargo build --release -p clawft-cli \
    && strip --strip-all target/release/weft

# ============================================================================
# Stage 4: Runtime -- minimal Debian with only the binary
# ============================================================================
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        libgcc-s1 \
    && rm -rf /var/lib/apt/lists/*

# Non-root user
RUN useradd --system --no-create-home --shell /usr/sbin/nologin weft

COPY --from=build /app/target/release/weft /usr/local/bin/weft

# OCI labels (overridden by build args in CI)
ARG VERSION=dev
LABEL org.opencontainers.image.source="https://github.com/clawft/clawft"
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.description="clawft gateway -- multi-channel AI agent runtime"

# Config directory
RUN mkdir -p /home/weft/.clawft && chown weft:weft /home/weft/.clawft
VOLUME ["/home/weft/.clawft"]

USER weft

# Health check (requires `weft health` subcommand -- see Section 2.6)
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/weft", "health"]

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/weft"]
CMD ["gateway"]
```

### 2.2 K2: Multi-Arch Build Script

**New file:** `scripts/build/docker-multiarch.sh`

This script wraps `docker buildx` to produce multi-platform images. It replaces the current
single-arch `docker-build.sh` for production use.

#### Requirements

| Requirement | Detail |
|-------------|--------|
| Platforms | `linux/amd64,linux/arm64` |
| Builder | Create/reuse `clawft-multiarch` buildx builder instance |
| Image name | `ghcr.io/clawft/clawft:<tag>` |
| Tag strategy | `--tag latest --tag <version>` (version from git tag or argument) |
| Load vs Push | `--load` for local testing (single-arch only), `--push` for registry |
| Size validation | After build, inspect and assert compressed size < 50MB |
| Cache | `--cache-from type=gha --cache-to type=gha,mode=max` in CI context |

#### Script Pseudocode

```bash
#!/usr/bin/env bash
# scripts/build/docker-multiarch.sh
# Usage: docker-multiarch.sh [--tag <tag>] [--push] [--load] [--platforms <p>]
set -euo pipefail

# 1. Parse arguments
#    --tag <tag>        Image tag (default: "latest")
#    --push             Push to registry (requires docker login)
#    --load             Load into local Docker (single-arch only)
#    --platforms <list> Comma-separated platforms (default: linux/amd64,linux/arm64)

# 2. Resolve workspace root via SCRIPT_DIR
WORKSPACE_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"

# 3. Ensure buildx builder exists
if ! docker buildx inspect clawft-multiarch >/dev/null 2>&1; then
    docker buildx create --name clawft-multiarch --driver docker-container --use
else
    docker buildx use clawft-multiarch
fi

# 4. Construct build command
#    docker buildx build \
#      --platform "$PLATFORMS" \
#      --tag "ghcr.io/clawft/clawft:${TAG}" \
#      --build-arg VERSION="${TAG}" \
#      --file "$WORKSPACE_ROOT/Dockerfile" \
#      ${PUSH:+--push} ${LOAD:+--load} \
#      "$WORKSPACE_ROOT"

# 5. If --load was used, validate image size
#    IMAGE_SIZE=$(docker image inspect ... --format='{{.Size}}')
#    Assert < 50MB compressed (52428800 bytes)

# 6. Print summary
```

### 2.3 K2: VPS Deployment Scripts

**New files:** `scripts/deploy/vps-setup.sh`, `scripts/deploy/vps-update.sh`

One-click scripts for deploying the clawft gateway on a fresh VPS (Ubuntu 22.04+ or Debian 12+).

#### vps-setup.sh -- Initial Setup

Performs first-time setup on a clean VPS:

```bash
#!/usr/bin/env bash
# scripts/deploy/vps-setup.sh
# One-click clawft gateway deployment on a fresh VPS.
# Usage: curl -sSL https://raw.githubusercontent.com/clawft/clawft/main/scripts/deploy/vps-setup.sh | bash
set -euo pipefail

# 1. Check prerequisites
#    - Running as root or with sudo
#    - systemd available
#    - x86_64 or aarch64 architecture

# 2. Install Docker (if not present)
#    - Use official Docker convenience script (get.docker.com)
#    - Enable and start dockerd
#    - Add current user to docker group

# 3. Pull latest image
#    docker pull ghcr.io/clawft/clawft:latest

# 4. Create data directories
#    mkdir -p /opt/clawft/config /opt/clawft/data

# 5. Generate default config
#    docker run --rm ghcr.io/clawft/clawft:latest config init > /opt/clawft/config/config.json

# 6. Create systemd service unit
#    /etc/systemd/system/clawft.service
#    [Unit]
#    Description=Clawft AI Agent Gateway
#    After=docker.service
#    Requires=docker.service
#    [Service]
#    Type=simple
#    Restart=always
#    RestartSec=10
#    ExecStartPre=-/usr/bin/docker stop clawft
#    ExecStartPre=-/usr/bin/docker rm clawft
#    ExecStart=/usr/bin/docker run --name clawft \
#      -v /opt/clawft/config:/home/weft/.clawft \
#      -p 8080:8080 \
#      --restart unless-stopped \
#      ghcr.io/clawft/clawft:latest
#    ExecStop=/usr/bin/docker stop clawft
#    [Install]
#    WantedBy=multi-user.target

# 7. Enable and start
#    systemctl daemon-reload
#    systemctl enable --now clawft

# 8. Verify health
#    Wait up to 30s for container health check to pass
#    curl -sf http://localhost:8080/health || warn "Health check not yet responding"

# 9. Print summary with access instructions
```

#### vps-update.sh -- Rolling Update

```bash
#!/usr/bin/env bash
# scripts/deploy/vps-update.sh
# Pull latest image and restart the clawft service.
# Usage: bash scripts/deploy/vps-update.sh [--tag <tag>]
set -euo pipefail

# 1. Parse --tag argument (default: latest)
# 2. Pull new image:   docker pull ghcr.io/clawft/clawft:${TAG}
# 3. Restart service:  systemctl restart clawft
# 4. Wait for health:  poll /health for up to 30s
# 5. Print old vs new image digest
```

### 2.4 K2-CI: PR Gates Workflow

**New file:** `.github/workflows/pr-gates.yml`

Consolidates quality checks that run on every pull request. This workflow is additive to the
existing `benchmarks.yml` and `wasm-build.yml` workflows.

#### Workflow Matrix

| Job | Runs On | Steps | Fail = PR Blocked |
|-----|---------|-------|-------------------|
| `clippy` | `ubuntu-latest` | `cargo clippy --workspace -- -D warnings` | Yes |
| `test` | `ubuntu-latest` | `cargo test --workspace` | Yes |
| `wasm-size` | `ubuntu-latest` | Build WASM, assert <300KB raw / <120KB gzip | Yes |
| `binary-size` | `ubuntu-latest` | Build release, assert <10MB | Yes |
| `docker-smoke` | `ubuntu-latest` | Build image, start container, check `/health`, stop | Yes |

#### Workflow Definition

```yaml
# .github/workflows/pr-gates.yml
name: PR Gates

on:
  pull_request:
    branches: [main, master]

concurrency:
  group: pr-gates-${{ github.head_ref || github.run_id }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # ---------------------------------------------------------------
  # Job 1: Clippy lint
  # ---------------------------------------------------------------
  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-clippy-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-clippy-

      - name: Run clippy
        run: cargo clippy --workspace -- -D warnings

  # ---------------------------------------------------------------
  # Job 2: Workspace tests
  # ---------------------------------------------------------------
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-test-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-test-

      - name: Run tests
        run: cargo test --workspace

  # ---------------------------------------------------------------
  # Job 3: WASM size gate
  # ---------------------------------------------------------------
  wasm-size:
    name: WASM Size Gate
    runs-on: ubuntu-latest
    env:
      WASM_TARGET: wasm32-wasip2
      WASM_BINARY: target/wasm32-wasip2/release-wasm/clawft_wasm.wasm
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip2

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-wasm-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-wasm-

      - name: Install binaryen
        run: sudo apt-get update && sudo apt-get install -y binaryen bc

      - name: Build WASM
        run: cargo build --target $WASM_TARGET --profile release-wasm -p clawft-wasm

      - name: Size gate
        run: |
          chmod +x scripts/bench/wasm-size-gate.sh
          bash scripts/bench/wasm-size-gate.sh "$WASM_BINARY" 300 120

  # ---------------------------------------------------------------
  # Job 4: Binary size regression
  # ---------------------------------------------------------------
  binary-size:
    name: Binary Size Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-release-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-release-

      - name: Build release binary
        run: cargo build --release -p clawft-cli

      - name: Check binary size
        run: |
          chmod +x scripts/build/size-check.sh
          bash scripts/build/size-check.sh target/release/weft 10

  # ---------------------------------------------------------------
  # Job 5: Docker integration smoke test
  # ---------------------------------------------------------------
  docker-smoke:
    name: Docker Smoke Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-docker-${{ hashFiles('**/Cargo.lock') }}
          restore-keys: |
            ${{ runner.os }}-cargo-docker-

      - name: Build Docker image
        run: docker build -t clawft:smoke-test .

      - name: Validate image size
        run: |
          SIZE_BYTES=$(docker image inspect clawft:smoke-test --format='{{.Size}}')
          SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)
          echo "Image size: ${SIZE_MB} MB"
          MAX_MB=50
          if (( $(echo "$SIZE_MB > $MAX_MB" | bc -l) )); then
            echo "FAIL: Image exceeds ${MAX_MB} MB (actual: ${SIZE_MB} MB)"
            exit 1
          fi
          echo "PASS: Image size within ${MAX_MB} MB limit"

      - name: Start container
        run: |
          docker run --rm -d \
            --name clawft-smoke \
            -p 8080:8080 \
            clawft:smoke-test gateway
          echo "Container started, waiting for health..."

      - name: Wait for health
        run: |
          RETRIES=30
          for i in $(seq 1 $RETRIES); do
            if docker inspect --format='{{.State.Health.Status}}' clawft-smoke 2>/dev/null | grep -q healthy; then
              echo "PASS: Container is healthy after ${i}s"
              exit 0
            fi
            # Fallback: try HTTP health check directly
            if curl -sf http://localhost:8080/health >/dev/null 2>&1; then
              echo "PASS: Health endpoint responding after ${i}s"
              exit 0
            fi
            sleep 1
          done
          echo "FAIL: Container did not become healthy within ${RETRIES}s"
          docker logs clawft-smoke
          exit 1

      - name: Stop container
        if: always()
        run: docker stop clawft-smoke 2>/dev/null || true
```

### 2.5 K2-CI: Release Pipeline

**New file:** `.github/workflows/release.yml`

Triggered on semver tags (`v*.*.*`). Builds multi-arch Docker images and pushes to GHCR.

#### Trigger and Permissions

```yaml
on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
      - 'v[0-9]+.[0-9]+.[0-9]+-*'  # pre-releases: v1.0.0-rc.1

permissions:
  contents: read
  packages: write
```

#### Workflow Definition

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'
      - 'v[0-9]+.[0-9]+.[0-9]+-*'

permissions:
  contents: read
  packages: write

env:
  CARGO_TERM_COLOR: always
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  # ---------------------------------------------------------------
  # Job 1: Run all quality gates before release
  # ---------------------------------------------------------------
  quality-gates:
    name: Quality Gates
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
          targets: wasm32-wasip2

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-release-${{ hashFiles('**/Cargo.lock') }}

      - name: Clippy
        run: cargo clippy --workspace -- -D warnings

      - name: Test
        run: cargo test --workspace

      - name: Binary size check
        run: |
          cargo build --release -p clawft-cli
          chmod +x scripts/build/size-check.sh
          bash scripts/build/size-check.sh target/release/weft 10

  # ---------------------------------------------------------------
  # Job 2: Multi-arch Docker build and GHCR push
  # ---------------------------------------------------------------
  docker-publish:
    name: Docker Publish
    needs: quality-gates
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Extract version from tag
        id: version
        run: |
          VERSION="${GITHUB_REF_NAME#v}"
          echo "version=${VERSION}" >> "$GITHUB_OUTPUT"
          echo "Building version: ${VERSION}"

      - name: Set up QEMU
        uses: docker/setup-qemu-action@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract Docker metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=semver,pattern={{major}}
            type=raw,value=latest

      - name: Build and push
        uses: docker/build-push-action@v6
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          build-args: |
            VERSION=${{ steps.version.outputs.version }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Verify published image
        run: |
          docker pull ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ steps.version.outputs.version }}
          docker image inspect ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ steps.version.outputs.version }}

      - name: Post image size summary
        run: |
          SIZE_BYTES=$(docker image inspect \
            ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ steps.version.outputs.version }} \
            --format='{{.Size}}')
          SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)
          echo "## Docker Image Published" >> "$GITHUB_STEP_SUMMARY"
          echo "" >> "$GITHUB_STEP_SUMMARY"
          echo "| Field | Value |" >> "$GITHUB_STEP_SUMMARY"
          echo "|-------|-------|" >> "$GITHUB_STEP_SUMMARY"
          echo "| Version | ${{ steps.version.outputs.version }} |" >> "$GITHUB_STEP_SUMMARY"
          echo "| Platforms | linux/amd64, linux/arm64 |" >> "$GITHUB_STEP_SUMMARY"
          echo "| Image Size | ${SIZE_MB} MB |" >> "$GITHUB_STEP_SUMMARY"
          echo "| Registry | ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }} |" >> "$GITHUB_STEP_SUMMARY"
```

### 2.6 Health Check Subcommand Prerequisite

The Dockerfile specifies `HEALTHCHECK CMD ["/usr/local/bin/weft", "health"]`. This requires a
minimal `weft health` CLI subcommand that does not currently exist.

**Implementation scope** (minimal, just for Docker health check):

```rust
// crates/clawft-cli/src/commands/health.rs

use clap::Args;

/// Arguments for the `weft health` subcommand.
#[derive(Args)]
pub struct HealthArgs;

impl HealthArgs {
    /// Run the health check. Exits 0 if healthy, 1 otherwise.
    ///
    /// For the Docker HEALTHCHECK, this only needs to verify that the
    /// process can start and the binary is not corrupted. A future
    /// iteration may check gateway port binding.
    pub async fn run(&self) -> anyhow::Result<()> {
        // Minimal: if we get here, the binary is functional
        println!("ok");
        Ok(())
    }
}
```

Register in the CLI command enum:

```rust
// In crates/clawft-cli/src/commands/mod.rs, add:
#[derive(Subcommand)]
pub enum Commands {
    // ... existing variants ...

    /// Check gateway health (used by Docker HEALTHCHECK).
    Health(health::HealthArgs),
}
```

This is deliberately minimal. The health check can be extended in a later phase to probe the
gateway's HTTP listener or channel connections.

---

## 3. Pseudocode

### 3.1 Dockerfile Build Flow

```
STAGE 1: chef
  FROM rust:1.93-bookworm
  RUN cargo install cargo-chef --locked
  WORKDIR /app

STAGE 2a: deps (prepare recipe)
  FROM chef
  COPY full source tree
  RUN cargo chef prepare --recipe-path recipe.json
  # Output: recipe.json (dependency manifest without source code)

STAGE 2b: cook (build dependencies only)
  FROM chef
  COPY recipe.json from deps stage
  RUN cargo chef cook --release --recipe-path recipe.json -p clawft-cli
  # Output: compiled dependency artifacts in target/
  # This layer is cached until Cargo.toml/Cargo.lock change

STAGE 3: build (compile application)
  FROM cook
  COPY full source tree (invalidates layer on any src change)
  RUN cargo build --release -p clawft-cli
  RUN strip --strip-all target/release/weft
  # Output: stripped release binary

STAGE 4: runtime
  FROM debian:bookworm-slim
  INSTALL ca-certificates, libssl3, libgcc-s1
  CREATE non-root user "weft"
  COPY binary from build stage
  SET volume, healthcheck, expose, entrypoint, cmd
```

### 3.2 CI PR Gates Flow

```
ON: pull_request to main/master

PARALLEL JOBS:
  clippy:
    checkout -> install rust+clippy -> cache restore -> cargo clippy --workspace -- -D warnings
    FAIL IF: any clippy warning

  test:
    checkout -> install rust -> cache restore -> cargo test --workspace
    FAIL IF: any test failure

  wasm-size:
    checkout -> install rust+wasm target -> install binaryen -> build wasm -> run size gate
    FAIL IF: raw > 300KB or gzip > 120KB

  binary-size:
    checkout -> install rust -> cache restore -> cargo build --release -> size-check.sh 10
    FAIL IF: binary > 10MB

  docker-smoke:
    checkout -> install rust -> docker build -> validate size < 50MB
    -> docker run -d -> poll health for 30s -> docker stop
    FAIL IF: image > 50MB or health check timeout
```

### 3.3 Release Pipeline Flow

```
ON: push tag matching v*.*.*

SEQUENTIAL:
  quality-gates:
    clippy + test + binary-size (all must pass)

  docker-publish (needs: quality-gates):
    extract version from tag
    setup QEMU (for arm64 emulation)
    setup Docker Buildx
    login to GHCR
    extract metadata (semver tags: X.Y.Z, X.Y, X, latest)
    build + push multi-arch (linux/amd64, linux/arm64)
    verify: pull published image
    post: image size summary to workflow output
```

### 3.4 VPS Setup Flow

```
FUNCTION vps_setup():
  CHECK: root/sudo, systemd, architecture (x86_64 or aarch64)
  IF docker not installed:
    curl -fsSL get.docker.com | sh
    systemctl enable --now docker
  PULL: ghcr.io/clawft/clawft:latest
  MKDIR: /opt/clawft/{config,data}
  GENERATE: default config.json
  WRITE: /etc/systemd/system/clawft.service
  systemctl daemon-reload
  systemctl enable --now clawft
  WAIT: poll health for 30s
  PRINT: summary with access URL
```

### 3.5 VPS Update Flow

```
FUNCTION vps_update(tag="latest"):
  docker pull ghcr.io/clawft/clawft:${tag}
  OLD_DIGEST = docker inspect clawft --format='{{.Image}}'
  systemctl restart clawft
  WAIT: poll health for 30s
  NEW_DIGEST = docker inspect clawft --format='{{.Image}}'
  PRINT: "Updated ${OLD_DIGEST} -> ${NEW_DIGEST}"
```

---

## 4. Architecture

### 4.1 Docker Image Layer Diagram

```
+------------------------------------------------------------------+
|  FINAL IMAGE: debian:bookworm-slim                               |
|                                                                  |
|  Layer 1: Base OS (debian:bookworm-slim)           ~25MB         |
|  Layer 2: Runtime libs (ca-certs, libssl3, libgcc)  ~5MB         |
|  Layer 3: User setup (weft user, config dir)         <1MB        |
|  Layer 4: /usr/local/bin/weft (stripped binary)     ~4.7MB       |
|                                                                  |
|  TOTAL COMPRESSED: <50MB target                                  |
+------------------------------------------------------------------+
```

### 4.2 Build Pipeline Flow

```
Developer pushes PR:
  +----------------+     +------------------+     +------------------+
  | PR Gates       |     |  Benchmarks      |     |  WASM Build      |
  | (pr-gates.yml) |     | (benchmarks.yml) |     | (wasm-build.yml) |
  +-------+--------+     +--------+---------+     +--------+---------+
          |                       |                         |
          v                       v                         v
    [clippy]              [bench suite]              [wasm-opt + gate]
    [test]                [regression check]
    [wasm-size]
    [binary-size]
    [docker-smoke]
          |
          v
    All green -> PR mergeable

Developer tags release (v1.2.3):
  +------------------+     +--------------------+
  | Quality Gates    |---->| Docker Publish     |
  | (clippy+test+    |     | (buildx multi-arch)|
  |  binary size)    |     | (GHCR push)        |
  +------------------+     +--------+-----------+
                                    |
                                    v
                          ghcr.io/clawft/clawft:1.2.3
                          ghcr.io/clawft/clawft:1.2
                          ghcr.io/clawft/clawft:1
                          ghcr.io/clawft/clawft:latest
```

### 4.3 VPS Deployment Architecture

```
VPS (Ubuntu/Debian)
+----------------------------------------------------------+
|  systemd                                                  |
|  +-----------------------+                                |
|  | clawft.service        |                                |
|  |  ExecStart: docker    |                                |
|  |    run clawft:latest  |                                |
|  +-----------+-----------+                                |
|              |                                            |
|  Docker      v                                            |
|  +-----------------------+     +------------------------+ |
|  | clawft container      |     | /opt/clawft/config/    | |
|  | debian:bookworm-slim  |<--->| config.json (bind vol) | |
|  | user: weft            |     +------------------------+ |
|  | port: 8080->8080      |                                |
|  +-----------------------+                                |
+----------------------------------------------------------+
```

### 4.4 Cross-Arch Build Matrix

```
docker buildx build --platform linux/amd64,linux/arm64

  +--------------------+          +--------------------+
  | linux/amd64        |          | linux/arm64        |
  |                    |          |                    |
  | rust:1.93-bookworm |          | rust:1.93-bookworm |
  | (native compile)   |          | (QEMU emulation)   |
  |                    |          |                    |
  | cargo build        |          | cargo build        |
  | --release          |          | --release          |
  | -p clawft-cli      |          | -p clawft-cli      |
  +--------+-----------+          +--------+-----------+
           |                               |
           v                               v
  +--------+-----------+          +--------+-----------+
  | debian:bookworm-   |          | debian:bookworm-   |
  | slim (amd64)       |          | slim (arm64)       |
  | + weft binary      |          | + weft binary      |
  +--------+-----------+          +--------+-----------+
           |                               |
           +----------- MANIFEST ----------+
           |
           v
  ghcr.io/clawft/clawft:v1.2.3  (multi-arch manifest)
```

---

## 5. Refinement

### 5.1 Cross-Architecture Gotchas

| Issue | Impact | Mitigation |
|-------|--------|------------|
| QEMU arm64 emulation is slow (~10x slower than native) | CI release builds take 30-60 min | Use GHA cache (`type=gha,mode=max`) to skip dependency rebuild; cargo-chef isolates deps layer |
| Some Rust crates use C dependencies with arch-specific builds | Compile failure on arm64 | `reqwest` uses `rustls-tls` (no native OpenSSL dependency); verify all workspace crates compile on arm64 early |
| `strip` binary path differs across architectures | Build failure in multi-arch context | Use generic `strip --strip-all` (available in rust base image); also set `strip = true` in `[profile.release]` as a fallback |
| `debian:bookworm-slim` arm64 variant has slightly different package versions | Inconsistent runtime behavior | Pin package versions in `apt-get install` if issues arise; test both arches in CI |

### 5.2 Image Size Optimization Strategies

| Strategy | Expected Savings | Status |
|----------|-----------------|--------|
| Multi-stage build (don't ship compiler) | ~1.5GB (removes entire toolchain from final image) | Required |
| `cargo-chef` (cache deps separately) | Build time only (no image size impact) | Required |
| `strip --strip-all` on release binary | ~30-50% of binary size | Already in `[profile.release]` |
| `opt-level = "z"` (optimize for size) | ~10-20% binary reduction | Already in `[profile.release]` |
| `lto = true` (link-time optimization) | ~10-15% binary reduction | Already in `[profile.release]` |
| `codegen-units = 1` | Better optimization, slower build | Already in `[profile.release]` |
| `panic = "abort"` (no unwinding) | ~5-10% binary reduction | Already in `[profile.release]` |
| `--no-install-recommends` in apt-get | ~10-30MB saved | Required |
| `rm -rf /var/lib/apt/lists/*` | ~20-40MB saved | Required |
| Only install required runtime libs | Variable | Only `ca-certificates`, `libssl3`, `libgcc-s1` |

### 5.3 CI Caching Strategies

| Cache Type | Key | What It Caches | TTL |
|-----------|-----|----------------|-----|
| Cargo registry + git | `${{ runner.os }}-cargo-*-${{ hashFiles('**/Cargo.lock') }}` | Downloaded crates | Until Cargo.lock changes |
| Cargo build artifacts | Same key as above (in `target/` path) | Compiled dependencies | Until Cargo.lock changes |
| Docker layer cache | `type=gha` (GitHub Actions cache) | Docker build layers | 10GB per repo limit |
| cargo-chef recipe | Docker layer cache (layer 2b) | Pre-built deps | Until Cargo.toml/Cargo.lock change |

### 5.4 Consolidation with Existing Workflows

The new `pr-gates.yml` incorporates WASM size gating that currently runs in `wasm-build.yml`.
The consolidation strategy:

1. **Keep `wasm-build.yml` as-is** for now -- it includes twiggy profiling, artifact uploads,
   and step summary that are useful for WASM-focused development.
2. **`pr-gates.yml` WASM job** runs the size gate only (no twiggy, no artifact upload) for
   fast PR feedback.
3. **Future consolidation** (post-K2): merge `wasm-build.yml` into `pr-gates.yml` with
   conditional twiggy profiling.

Similarly, `benchmarks.yml` is kept separate because it has different trigger semantics
(push to main) and a regression-comment bot that doesn't belong in PR gates.

### 5.5 Docker Health Check Strategy

The `HEALTHCHECK` directive in the Dockerfile uses `weft health` (Section 2.6). The implementation
follows a two-phase approach:

1. **Phase 1 (this document)**: Minimal binary-alive check. The `weft health` command simply
   prints "ok" and exits 0. This confirms the binary is not corrupted and can execute.

2. **Phase 2 (future)**: Once the gateway exposes an HTTP listener, `weft health` should
   attempt a TCP connect or HTTP GET to `localhost:8080/health`. This requires the gateway
   to bind a health endpoint, which is tracked separately.

For the Docker smoke test in CI, we use both approaches: the `HEALTHCHECK` directive for
Docker-native health and a fallback `curl` to `localhost:8080/health` if the gateway exposes it.

### 5.6 Security Considerations

| Concern | Mitigation |
|---------|------------|
| Running as root in container | `USER weft` directive; non-root by default |
| Secrets in build args | Only `VERSION` passed as build arg; no secrets in Dockerfile |
| Base image CVEs | `debian:bookworm-slim` is Debian's long-term supported minimal image; pin and update regularly |
| GHCR token scope | `packages: write` only; `contents: read` only; least-privilege |
| VPS setup script runs as root | Required for Docker install and systemd; script is auditable |
| Supply chain (GitHub Actions) | Pin action versions by SHA in production (use tags in this doc for readability) |

---

## 6. Completion

### 6.1 Acceptance Criteria

| # | Criterion | Verification Method |
|---|-----------|-------------------|
| 1 | Multi-stage Dockerfile builds successfully | `docker build -t clawft:test .` exits 0 |
| 2 | Final image uses `debian:bookworm-slim` base | `docker inspect --format='{{.Config.Image}}'` or check layer history |
| 3 | Binary runs as non-root user `weft` | `docker run --rm clawft:test whoami` outputs `weft` |
| 4 | Image size < 50MB compressed | `docker image inspect --format='{{.Size}}'` < 52428800 |
| 5 | `docker buildx build --platform linux/amd64,linux/arm64` succeeds | CI release pipeline green |
| 6 | `HEALTHCHECK` passes within 30s of container start | `docker inspect --format='{{.State.Health.Status}}'` returns `healthy` |
| 7 | `weft health` subcommand exists and exits 0 | `weft health` prints "ok" |
| 8 | PR gates workflow blocks on clippy warnings | Introduce deliberate warning, verify PR is blocked |
| 9 | PR gates workflow blocks on test failure | Introduce deliberate test failure, verify PR is blocked |
| 10 | PR gates workflow checks WASM size (<300KB/<120KB) | Verify against current baseline (57.9KB raw, 24.3KB gzip) |
| 11 | PR gates workflow checks binary size (<10MB) | Verify against current baseline (4.7MB) |
| 12 | Docker smoke test runs in PR gates | Verify `docker-smoke` job passes |
| 13 | Release pipeline triggers on semver tag | Push `v0.0.1-test` tag, verify workflow runs |
| 14 | Release pipeline pushes to GHCR | `docker pull ghcr.io/clawft/clawft:0.0.1-test` succeeds |
| 15 | Multi-arch manifest contains both platforms | `docker manifest inspect` shows amd64 + arm64 |
| 16 | VPS setup script installs Docker and starts service | Manual test on fresh VPS |
| 17 | VPS update script pulls new image and restarts | Manual test after publishing new tag |
| 18 | Existing CI workflows (`benchmarks.yml`, `wasm-build.yml`) still pass | Verify no regressions after PR gates addition |

### 6.2 Test Plan

#### Automated Tests (CI)

| Test | Location | Type |
|------|----------|------|
| `weft health` exits 0 | `cargo test --workspace` | Unit (added to clawft-cli tests) |
| Docker image builds | `pr-gates.yml` / `docker-smoke` job | Integration |
| Image size < 50MB | `pr-gates.yml` / `docker-smoke` job | Assertion |
| Container starts and becomes healthy | `pr-gates.yml` / `docker-smoke` job | Smoke |
| Clippy clean | `pr-gates.yml` / `clippy` job | Lint |
| All workspace tests pass | `pr-gates.yml` / `test` job | Unit + Integration |
| WASM size within budget | `pr-gates.yml` / `wasm-size` job | Assertion |
| Binary size < 10MB | `pr-gates.yml` / `binary-size` job | Assertion |
| Multi-arch build + GHCR push | `release.yml` / `docker-publish` job | Integration |

#### Manual Tests

| Test | Steps | Expected Result |
|------|-------|-----------------|
| Local Docker build | `docker build -t clawft:local .` | Builds successfully, < 50MB |
| Local multi-arch build | `scripts/build/docker-multiarch.sh --load` | Builds for current arch |
| Container gateway start | `docker run -d -p 8080:8080 clawft:local` | Container starts, logs show gateway init |
| Non-root verification | `docker exec <id> whoami` | Outputs `weft` |
| VPS fresh setup | Run `vps-setup.sh` on fresh Debian 12 VM | Service running, health check passing |
| VPS rolling update | Run `vps-update.sh --tag v0.1.0` | New image pulled, service restarted |
| ARM64 local test | Build on Apple Silicon or ARM64 host | Image builds and runs |

### 6.3 Exit Criteria

This phase is complete when:

1. The `Dockerfile` in the project root is replaced with the multi-stage build defined in
   Section 2.1, and the old `FROM scratch` approach is removed.

2. The `scripts/build/docker-multiarch.sh` script successfully builds images for both
   `linux/amd64` and `linux/arm64`.

3. `.github/workflows/pr-gates.yml` is live and all five jobs (clippy, test, wasm-size,
   binary-size, docker-smoke) pass on the branch that introduces these changes.

4. `.github/workflows/release.yml` is live and a test tag (e.g., `v0.0.1-rc.1`) successfully
   triggers a multi-arch build and GHCR push.

5. The `weft health` subcommand is implemented and registered in the CLI.

6. VPS deployment scripts (`scripts/deploy/vps-setup.sh`, `scripts/deploy/vps-update.sh`)
   are written and tested on at least one VPS instance.

7. The existing `scripts/build/docker-build.sh` is updated or deprecated with a note pointing
   to `docker-multiarch.sh`.

8. All existing CI workflows (`benchmarks.yml`, `wasm-build.yml`) continue to pass.

### 6.4 File Manifest

| File | Action | Description |
|------|--------|-------------|
| `Dockerfile` | **Replace** | Multi-stage build with cargo-chef (Section 2.1) |
| `scripts/build/docker-multiarch.sh` | **Create** | Multi-arch buildx wrapper (Section 2.2) |
| `scripts/deploy/vps-setup.sh` | **Create** | First-time VPS deployment (Section 2.3) |
| `scripts/deploy/vps-update.sh` | **Create** | Rolling update script (Section 2.3) |
| `.github/workflows/pr-gates.yml` | **Create** | PR quality gate workflow (Section 2.4) |
| `.github/workflows/release.yml` | **Create** | Release pipeline workflow (Section 2.5) |
| `crates/clawft-cli/src/commands/health.rs` | **Create** | Health check subcommand (Section 2.6) |
| `crates/clawft-cli/src/commands/mod.rs` | **Edit** | Register `Health` variant in `Commands` enum |
| `scripts/build/docker-build.sh` | **Edit** | Add deprecation notice pointing to `docker-multiarch.sh` |

### 6.5 Estimated Effort

| Task | Effort | Notes |
|------|--------|-------|
| Multi-stage Dockerfile | 2h | Straightforward; cargo-chef is well-documented |
| docker-multiarch.sh script | 1h | Thin wrapper over `docker buildx` |
| VPS deployment scripts | 2h | systemd unit + Docker run |
| PR gates workflow | 3h | Five parallel jobs, caching config |
| Release pipeline | 2h | buildx + GHCR auth + metadata action |
| Health subcommand | 1h | Minimal implementation |
| Testing + iteration | 3h | Cross-arch builds are slow; expect iteration on CI caching |
| **Total** | **14h** | Spread across 2 weeks with other K-phase work |

---

## Appendix A: Current Baseline Metrics

From `scripts/bench/baseline.json` (as of 2026-02-17):

| Metric | Value | CI Gate Threshold |
|--------|-------|-------------------|
| Binary size | 4,710 KB (4.6 MB) | < 10 MB |
| WASM size (raw) | 57.9 KB | < 300 KB |
| WASM size (gzip) | 24.3 KB | < 120 KB |
| Startup time | 3.5 ms | N/A (tracked in benchmarks.yml) |
| Throughput | 418 inv/sec | N/A (tracked in benchmarks.yml) |

These baselines provide substantial headroom under the CI gate thresholds, which are set
conservatively to catch unexpected regressions without triggering false positives.

## Appendix B: Cargo Profile Reference

The workspace `Cargo.toml` already configures aggressive size optimization for release builds:

```toml
[profile.release]
opt-level = "z"    # Optimize for size
lto = true         # Link-time optimization
strip = true       # Strip symbols
codegen-units = 1  # Single codegen unit for better optimization
panic = "abort"    # No unwinding overhead
```

The `strip = true` in the Cargo profile makes the `strip --strip-all` in the Dockerfile
technically redundant, but both are kept for defense-in-depth (the Dockerfile strip acts
as a safety net if the profile is modified).

## Appendix C: Related Orchestrator Items

From `10-deployment-community/00-orchestrator.md`:

- **K3** (Per-agent sandbox): Depends on K2 for container runtime base image
- **K2-CI** (CI/CD pipeline): Cross-cutting; benefits all subsequent K-phase work
- **K4** (ClawHub): GHCR publish infrastructure from K2-CI is reused for skill package hosting
- **K5** (Benchmark suite): Uses the same CI runner configuration established here
