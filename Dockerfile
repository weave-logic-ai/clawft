# Multi-stage Dockerfile for weft (clawft CLI)
#
# Uses cargo-chef for dependency caching and debian:bookworm-slim as the
# runtime base (glibc required for wasmtime).
#
# Build:
#   docker buildx build --platform linux/amd64,linux/arm64 -t weft .
#
# Target: <50MB compressed image.

# ---------------------------------------------------------------------------
# Stage 1: Chef -- prepare dependency recipe
# ---------------------------------------------------------------------------
FROM rust:1.93-bookworm AS chef

RUN cargo install cargo-chef --locked
WORKDIR /app

# ---------------------------------------------------------------------------
# Stage 2: Planner -- extract the dependency recipe
# ---------------------------------------------------------------------------
FROM chef AS planner

COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---------------------------------------------------------------------------
# Stage 3: Builder -- build dependencies (cached), then build the application
# ---------------------------------------------------------------------------
FROM chef AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies (cached layer -- only rebuilds when Cargo.toml/lock change)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
COPY . .
RUN cargo build --release --bin weft \
    && strip target/release/weft

# ---------------------------------------------------------------------------
# Stage 4: Runtime -- minimal image with only the binary
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libgcc-s1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /bin/bash weft

COPY --from=builder /app/target/release/weft /usr/local/bin/weft

USER weft
WORKDIR /home/weft

# Default config directory
VOLUME ["/home/weft/.clawft"]

# Health check for gateway mode
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD ["weft", "status"] || exit 1

EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/weft"]
CMD ["gateway"]
