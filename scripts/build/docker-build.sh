#!/usr/bin/env bash
# Build a minimal Docker image for weft.
# Usage: ./scripts/build/docker-build.sh [--tag <tag>] [--push]
#
# Examples:
#   ./scripts/build/docker-build.sh
#   ./scripts/build/docker-build.sh --tag v0.1.0
#   ./scripts/build/docker-build.sh --tag latest --push

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# --- Helpers ---
info()  { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()    { printf "${GREEN}[OK]${NC}    %s\n" "$*"; }
warn()  { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
err()   { printf "${RED}[ERROR]${NC} %s\n" "$*" >&2; }

usage() {
    cat <<'EOF'
Build a minimal Docker image for weft (clawft CLI).

Usage: docker-build.sh [--tag <tag>] [--push]

Options:
  --tag <tag>    Docker image tag (default: latest).
                 The image is always named "clawft:<tag>".
  --push         Push the image to the registry after building.
                 Requires prior `docker login`.
  --help         Show this help message.

Examples:
  docker-build.sh
  docker-build.sh --tag v0.1.0
  docker-build.sh --tag latest --push
EOF
    exit 0
}

# --- Parse arguments ---
IMAGE_TAG="latest"
PUSH=false

while [ $# -gt 0 ]; do
    case "$1" in
        --tag)
            if [ $# -lt 2 ]; then
                err "--tag requires a value"
                exit 1
            fi
            IMAGE_TAG="$2"
            shift 2
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --help|-h)
            usage
            ;;
        *)
            err "Unknown option: $1"
            printf "\n"
            usage
            ;;
    esac
done

IMAGE_NAME="clawft:${IMAGE_TAG}"

# --- Resolve workspace root ---
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

info "Workspace root: $WORKSPACE_ROOT"
info "Image:          $IMAGE_NAME"

# --- Check prerequisites ---
if ! command -v docker >/dev/null 2>&1; then
    err "'docker' is not installed or not in PATH."
    exit 1
fi

# --- Determine if cross is available, fall back to cargo ---
MUSL_TARGET="x86_64-unknown-linux-musl"
BUILD_CMD="cargo"
if command -v cross >/dev/null 2>&1; then
    BUILD_CMD="cross"
    info "Using 'cross' for static musl build."
else
    warn "'cross' not found, falling back to 'cargo'."
    warn "Ensure the $MUSL_TARGET target and musl toolchain are installed."
fi

# --- Build static binary ---
info "Building static binary for $MUSL_TARGET..."

(cd "$WORKSPACE_ROOT" && "$BUILD_CMD" build --release --target "$MUSL_TARGET" -p clawft-cli)

BINARY_PATH="$WORKSPACE_ROOT/target/$MUSL_TARGET/release/weft"

if [ ! -f "$BINARY_PATH" ]; then
    err "Build succeeded but binary not found at: $BINARY_PATH"
    exit 1
fi

ok "Static binary built: $BINARY_PATH"

# --- Prepare Docker build context ---
DOCKER_BUILD_DIR="$WORKSPACE_ROOT/docker-build"
mkdir -p "$DOCKER_BUILD_DIR"

cp "$BINARY_PATH" "$DOCKER_BUILD_DIR/weft-linux-x86_64"
chmod +x "$DOCKER_BUILD_DIR/weft-linux-x86_64"

info "Binary staged to docker-build/weft-linux-x86_64"

# --- Build Docker image ---
info "Building Docker image: $IMAGE_NAME"

docker build -t "$IMAGE_NAME" -f "$WORKSPACE_ROOT/Dockerfile" "$WORKSPACE_ROOT"

ok "Docker image built: $IMAGE_NAME"

# --- Validate image size ---
MAX_IMAGE_SIZE_MB=20
IMAGE_SIZE_BYTES=$(docker image inspect "$IMAGE_NAME" --format='{{.Size}}' 2>/dev/null)
IMAGE_SIZE_MB=$(echo "scale=2; $IMAGE_SIZE_BYTES / 1048576" | bc)

info "Image size: ${IMAGE_SIZE_MB} MB"

OVER_LIMIT=$(echo "$IMAGE_SIZE_MB > $MAX_IMAGE_SIZE_MB" | bc -l)
if [ "$OVER_LIMIT" -eq 1 ]; then
    err "Image exceeds ${MAX_IMAGE_SIZE_MB} MB limit (${IMAGE_SIZE_MB} MB)."
    err "Check binary size and Dockerfile for unnecessary layers."
    exit 1
fi

ok "Image size within ${MAX_IMAGE_SIZE_MB} MB limit."

# --- Print details ---
printf "\n"
info "Image details:"
docker image ls "$IMAGE_NAME"

# --- Push if requested ---
if [ "$PUSH" = true ]; then
    printf "\n"
    info "Pushing $IMAGE_NAME to registry..."
    docker push "$IMAGE_NAME"
    ok "Image pushed: $IMAGE_NAME"
fi

printf "\n"
ok "Docker build complete: $IMAGE_NAME"
