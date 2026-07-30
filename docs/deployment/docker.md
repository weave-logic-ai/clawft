# Docker Deployment

WeftOS ships a multi-arch Docker image based on `alpine:3.21`. The image is
**self-contained**: a multi-stage build compiles a static musl `weft` binary
from the repository tag (or local checkout), then copies it into a minimal
Alpine runtime. There is no download of cargo-dist / GitHub Release assets at
image-build time — the image tag is not coupled to whether platform binaries
published successfully (WEFT-594 decision; see also WEFT-593).

Compressed size is typically ~30–40 MB; supported architectures are
`linux/amd64` and `linux/arm64`.

## Strategy (WEFT-594)

| Option | Chosen? | Why |
|--------|---------|-----|
| Download release musl tarball into Alpine | No | Couples image tag to cargo-dist binary publish; broke when the plan matrix was empty (v0.6.21 / WEFT-593). |
| Self-contained multi-stage + QEMU multi-arch | No | arm64 Rust compile under QEMU is impractically slow in CI. |
| Self-contained multi-stage + **native** multi-arch | **Yes** | Compile from source on `ubuntu-latest` (amd64) and `ubuntu-24.04-arm` (arm64); merge digests into one OCI manifest. No QEMU. |
| Single-arch only | No | arm64 is first-class (Apple Silicon hosts, ARM cloud). |

CI: `.github/workflows/release-docker.yml` — per-arch digests, then
`docker buildx imagetools create` for `latest` and `vX.Y.Z`.

Local: `Dockerfile` at the repo root (same self-contained multi-stage).
Kernel-only variant: `crates/clawft-kernel/Dockerfile.alpine` (builds
`weaver`, not the full CLI).

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) 20.10 or later (with
  `buildx` for multi-arch builds), **or** a compatible OCI runtime (see
  [Local runtimes on macOS](#local-runtimes-on-macos) below).

## Quick Start

Pull the published image and run the gateway:

```bash
docker pull ghcr.io/weave-logic-ai/weftos:latest
docker run --rm -it ghcr.io/weave-logic-ai/weftos:latest --version
```

By default the entrypoint is `weft` and the default command is `gateway`,
so a bare `docker run ... weftos:latest` starts the gateway on port 8080.

## Image Tags

| Tag         | Contents                                            |
|-------------|-----------------------------------------------------|
| `latest`    | Most recent release (rolls forward on every release)|
| `vX.Y.Z`    | Pinned release (e.g. `v0.6.19`, `v0.7.0`)           |

Pin to a version in production. The `latest` tag is safe for local
exploration but moves under you on every release.

## Configuration

The container runs as the unprivileged `weft` user with `$HOME=/home/weft`.
The canonical config + state directory inside the container is
`/home/weft/.clawft` -- mount your host config there.

The image ships a default `config.json` that enables the auth-gated REST/WS
API on port 8080 so `weft gateway` starts out of the box. Only `/api/health`
and the token-bootstrap path are public; other routes require a Bearer token.

### Mounting Config

```bash
docker run --rm -it \
  -v "$HOME/.clawft:/home/weft/.clawft:ro" \
  ghcr.io/weave-logic-ai/weftos:latest gateway
```

### Environment Variables

Pass API keys and runtime settings via environment variables. Common ones:

```bash
docker run --rm -it \
  -e OPENAI_API_KEY="sk-..." \
  -e CLAWFT_CONFIG="/home/weft/.clawft/config.json" \
  -e RUST_LOG=info \
  -v "$HOME/.clawft:/home/weft/.clawft:ro" \
  ghcr.io/weave-logic-ai/weftos:latest gateway
```

### Persisting Workspace State

To persist sessions, memory, and skills across restarts, mount a writable
volume on the same path:

```bash
docker run --rm -it \
  -v weftos-data:/home/weft/.clawft \
  ghcr.io/weave-logic-ai/weftos:latest gateway
```

## Docker Compose

A working compose definition ships under `scripts/deploy/docker-compose.yml`.
Run it directly:

```bash
docker compose -f scripts/deploy/docker-compose.yml up -d
docker compose -f scripts/deploy/docker-compose.yml logs -f
```

The shipped file uses a named volume (`weft-config`) for state and exposes
port 8080. Override the version with the `WEFT_VERSION` environment
variable:

```bash
WEFT_VERSION=0.6.19 docker compose -f scripts/deploy/docker-compose.yml up -d
```

Inline equivalent:

```yaml
services:
  weft:
    image: ghcr.io/weave-logic-ai/weftos:${WEFT_VERSION:-latest}
    command: ["gateway"]
    restart: unless-stopped
    ports:
      - "8080:8080"
    volumes:
      - weft-config:/home/weft/.clawft
    environment:
      - RUST_LOG=info
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    healthcheck:
      test: ["CMD", "weft", "status"]
      interval: 30s
      timeout: 5s
      start_period: 10s
      retries: 3

volumes:
  weft-config:
```

## VPS One-Click Deploy

`scripts/deploy/vps-deploy.sh` wraps `docker run` with sensible defaults
for a single-host VPS install:

```bash
./scripts/deploy/vps-deploy.sh --pull
```

Flags: `--image`, `--port`, `--name`, `--config`, `--restart`, `--pull`.
The script idempotently stops any existing container with the same name
before starting the new one.

## Health Checks

The image declares a `HEALTHCHECK` that runs `weft status` every 30
seconds. The same probe is shown in the compose example above. The release
pipeline additionally probes `GET /api/health` (WEFT-550) after publish.

To check health from the host:

```bash
docker inspect --format '{{.State.Health.Status}}' weft
docker exec weft weft status --detailed
curl -sS http://127.0.0.1:8080/api/health
```

## Building Locally

Build from the repo root (compiles inside Docker — no pre-built binary
required):

```bash
# Host arch only (fast iteration)
./scripts/build/docker-build.sh --tag dev --version dev

# Or plain Docker / buildx
docker build -t weft:dev --build-arg VERSION=dev .
```

Multi-arch locally requires a builder that can reach both platforms (on
Apple Silicon, buildx can produce `linux/arm64` natively and `linux/amd64`
via emulation — fine for one-off publishes, not what CI uses):

```bash
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --build-arg VERSION=0.8.0 \
  -t weftos:local \
  --load .
```

For a kernel-only image (`weaver`):

```bash
docker build -f crates/clawft-kernel/Dockerfile.alpine -t weft-kernel:dev .
```

## CI / Release Pipeline

The image is built and published by `.github/workflows/release-docker.yml`,
triggered when the `Release` workflow (cargo-dist) succeeds on a tag push,
or via `workflow_dispatch`. The workflow:

1. Gates on Release success (orchestration only — binaries are **not**
   downloaded into the image).
2. Checks out the release tag.
3. Builds **natively** on `ubuntu-latest` (`linux/amd64`) and
   `ubuntu-24.04-arm` (`linux/arm64`) — no QEMU for Rust compilation.
4. Pushes per-platform digests, then creates a multi-arch manifest tagged
   `latest` and `vX.Y.Z` on GHCR.
5. Runs the post-publish `/api/health` smoke (WEFT-550).

See `docs/deployment/release.md` for how Docker fits into the full release
graph.

## Local runtimes on macOS

The published artifact is a standard multi-arch OCI image. Any runtime that
speaks OCI/GHCR can consume the same tags:

| Runtime | Notes |
|---------|--------|
| **Docker Desktop** | Pulls the matching `linux/arm64` layer on Apple Silicon; `linux/amd64` via Rosetta/QEMU if forced with `--platform`. |
| **OrbStack** | Drop-in Docker-compatible engine; uses the same `docker pull/run` commands against `ghcr.io/weave-logic-ai/weftos:*`. Prefer for lighter local resource use. |
| **Apple container CLI** | Apple's OCI-oriented tooling (`container pull` / `container run`) can pull the same GHCR multi-arch image and run the `linux/arm64` manifest entry on Apple Silicon — same image digest family as Docker/OrbStack, not a separate build. |

Example with Docker or OrbStack (identical CLI):

```bash
docker pull ghcr.io/weave-logic-ai/weftos:latest
docker run --rm -p 8080:8080 ghcr.io/weave-logic-ai/weftos:latest gateway
```

There is no separate "Apple Silicon image" — multi-arch is the product.

## Security Considerations

The image is small but not minimal -- it includes Alpine + musl libc +
`ca-certificates` + `tzdata`. Hardening recommendations:

- **Run as the built-in unprivileged user.** The image already declares
  `USER weft`. Don't override with `--user 0:0` unless you know why.
- **Mount config read-only.** Use `:ro` on the config bind mount; only
  the workspace volume needs to be writable.
- **Restrict egress.** For network-isolated deployments use a dedicated
  Docker network or `--network=none` for offline workflows.
- **Never bake secrets into the image.** Pass keys via environment
  variables, Docker secrets, or a sidecar secrets manager.
- **Pin tags in production.** `latest` is convenient for local dev; pin
  to `vX.Y.Z` in any deployment you don't want to silently roll forward.

## Troubleshooting

**Container exits immediately.** Inspect with:

```bash
docker run --rm -it \
  -v "$HOME/.clawft:/home/weft/.clawft:ro" \
  ghcr.io/weave-logic-ai/weftos:latest status --detailed
```

Common causes: missing config, invalid JSON, an LLM provider env var
referenced by config but not set in the container's environment.

**Permission denied on config file.** The container runs as a non-root
`weft` user. Make sure the host config is readable by anyone:

```bash
chmod 644 ~/.clawft/config.json
```

**Cannot reach LLM API.** Verify DNS and outbound network access from
inside the container:

```bash
docker run --rm ghcr.io/weave-logic-ai/weftos:latest agent -m "ping"
```

If running behind a corporate proxy, propagate the proxy env vars:

```bash
docker run --rm \
  -e HTTPS_PROXY="http://proxy:8080" \
  -e HTTP_PROXY="http://proxy:8080" \
  -e NO_PROXY="localhost,127.0.0.1" \
  ghcr.io/weave-logic-ai/weftos:latest agent -m "ping"
```

---

## Appendix: Legacy Build Approaches (Historical)

Earlier releases used different image layouts. Kept only so operators
upgrading from old documentation can map history.

### Download-based Alpine (pre-self-contained, ~0.4.3–0.6.x)

The root Dockerfile downloaded a cargo-dist musl tarball for `VERSION`
from GitHub Releases and installed it into Alpine. Build was ~2 minutes
and produced a ~15 MB image, but the image tag was coupled to binary
publish success — empty cargo-dist matrices (WEFT-593) produced 404s at
image build. Replaced by the self-contained multi-stage Dockerfile
(WEFT-594).

### `FROM scratch` (pre-0.4.2, deprecated)

Sprint 11 / 12 images were built `FROM scratch`, copying a single
statically-linked musl binary into the empty filesystem (~5 MB compressed).
Dropped for lack of `ca-certificates`, `tzdata`, and a shell for ops.

### `cargo-chef` multi-stage on Debian (proposed, never shipped)

A K2-era proposal used `debian:bookworm-slim` + `cargo-chef`. Never
adopted: longer cold builds, larger runtime (~50 MB), and QEMU multi-arch
cost. The current Alpine self-contained layout is the shipping path.
