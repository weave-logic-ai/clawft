# Docker Deployment

clawft ships as a minimal `FROM scratch` Docker image containing only the
statically-linked `weft` binary. The resulting image is typically around 5 MB.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) 20.10 or later

## Quick Start

Pull the pre-built image and run the gateway:

```bash
docker pull ghcr.io/clawft/clawft:latest
docker run --rm -it ghcr.io/clawft/clawft:latest --version
```

By default the container starts in gateway mode (`weft gateway`).

## Building from Source

Clone the repository and build a static musl binary, then build the Docker
image:

```bash
git clone https://github.com/clawft/clawft.git
cd clawft

# Build the static binary (requires musl target)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl

# Prepare the Docker build context
mkdir -p docker-build
cp target/x86_64-unknown-linux-musl/release/weft docker-build/weft-linux-x86_64

# Build the image
docker build -t clawft:local .
```

## Configuration

### Mounting a Config File

The container expects configuration at `/root/.clawft/config.json`. Mount your
local config into the container:

```bash
docker run --rm -it \
  -v "$HOME/.clawft:/root/.clawft:ro" \
  ghcr.io/clawft/clawft:latest gateway
```

### Environment Variables

Pass API keys and settings via environment variables:

```bash
docker run --rm -it \
  -e OPENAI_API_KEY="sk-..." \
  -e CLAWFT_CONFIG="/root/.clawft/config.json" \
  -v "$HOME/.clawft:/root/.clawft:ro" \
  ghcr.io/clawft/clawft:latest gateway
```

### Workspace Persistence

To persist agent session data across container restarts, mount the workspace
directory:

```bash
docker run --rm -it \
  -v "$HOME/.clawft:/root/.clawft" \
  ghcr.io/clawft/clawft:latest gateway
```

## Docker Compose

A Docker Compose configuration for running the gateway with persistent storage:

```yaml
services:
  clawft:
    image: ghcr.io/clawft/clawft:latest
    command: ["gateway"]
    restart: unless-stopped
    volumes:
      - ./config.json:/root/.clawft/config.json:ro
      - clawft-data:/root/.clawft/workspace
    environment:
      - OPENAI_API_KEY=${OPENAI_API_KEY}
    healthcheck:
      test: ["CMD", "/usr/local/bin/weft", "status"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  clawft-data:
```

Start the service:

```bash
docker compose up -d
docker compose logs -f clawft
```

## Health Checks

The `weft status` command returns a non-zero exit code if the agent cannot
initialise. Use it as a Docker health check (shown in the Compose example
above) or probe it manually:

```bash
docker exec <container> /usr/local/bin/weft status
```

## Security Considerations

The `FROM scratch` image has no shell, no package manager, and no OS libraries
-- the attack surface is minimal. For additional hardening:

- **Read-only filesystem**: mount the config volume as `:ro` and use a named
  volume for workspace data.
- **Non-root execution**: the `scratch` image has no users, so the process runs
  as UID 0 inside a minimal namespace. For stricter isolation, run with
  `--user 1000:1000` and adjust volume permissions accordingly.
- **Network restrictions**: use `--network=none` for offline testing, or create
  a dedicated Docker network to restrict egress.
- **Secrets**: never bake API keys into the image. Pass them via environment
  variables or Docker secrets.

## Troubleshooting

**Container exits immediately**

Check that the config file is valid JSON and the volume mount path is correct:

```bash
docker run --rm -it \
  -v "$HOME/.clawft:/root/.clawft:ro" \
  ghcr.io/clawft/clawft:latest status --detailed
```

**Permission denied on config file**

Ensure the config file is readable by the container user. On Linux:

```bash
chmod 644 ~/.clawft/config.json
```

**Cannot reach LLM API**

Verify DNS and network access from the container:

```bash
docker run --rm ghcr.io/clawft/clawft:latest agent -m "ping"
```

If running behind a corporate proxy, pass proxy environment variables:

```bash
docker run --rm -e HTTPS_PROXY="http://proxy:8080" ...
```
