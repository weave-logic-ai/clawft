#!/usr/bin/env bash
# Docker smoke test against the browser harness pkg (WEFT-408 / P6.7).
#
# Serves `crates/clawft-wasm/www/` (including the post-bindgen `pkg/`)
# from an nginx:alpine container and verifies the critical assets respond
# with HTTP 200. Does not execute WASM in-browser — that is the job of
# `scripts/build.sh test-browser`. This gate catches packaging / path /
# MIME regressions that would break the playground harness deploy.
#
# Usage:
#   scripts/ci/browser-harness-docker-smoke.sh [--build] [--port N]
#
# Environment:
#   BROWSER_SMOKE_SOFT=1   skip (exit 0) when docker is unavailable
#   BROWSER_SMOKE_PORT=N   host port (default: 18080)
#
# Exit codes:
#   0 — all probes pass (or soft-skip)
#   1 — smoke failure
#   2 — usage / missing pkg without --build

set -euo pipefail

RED=$'\033[0;31m'
GREEN=$'\033[0;32m'
YELLOW=$'\033[1;33m'
CYAN=$'\033[0;36m'
NC=$'\033[0m'

info() { printf "${CYAN}[INFO]${NC}  %s\n" "$*"; }
ok()   { printf "${GREEN}[PASS]${NC}  %s\n" "$*"; }
warn() { printf "${YELLOW}[WARN]${NC}  %s\n" "$*"; }
fail() { printf "${RED}[FAIL]${NC}  %s\n" "$*" >&2; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WWW_DIR="$WORKSPACE_ROOT/crates/clawft-wasm/www"
PKG_DIR="$WWW_DIR/pkg"

DO_BUILD=false
PORT="${BROWSER_SMOKE_PORT:-18080}"
IMAGE="${BROWSER_SMOKE_IMAGE:-nginx:alpine}"
CONTAINER_NAME="weft-browser-harness-smoke-$$"
NGINX_CONF_HOST=""
CLEANED=false

usage() {
    cat <<'EOF'
Usage: browser-harness-docker-smoke.sh [--build] [--port N] [--image NAME]

  --build   Run `scripts/build.sh browser` if www/pkg/ is missing
  --port N  Host port to publish (default 18080)
  --image   Docker image (default nginx:alpine)
EOF
    exit 2
}

while [ $# -gt 0 ]; do
    case "$1" in
        --build) DO_BUILD=true; shift ;;
        --port)  PORT="${2:?}"; shift 2 ;;
        --image) IMAGE="${2:?}"; shift 2 ;;
        -h|--help) usage ;;
        *) fail "unknown arg: $1"; usage ;;
    esac
done

cleanup() {
    if [ "$CLEANED" = true ]; then
        return
    fi
    CLEANED=true
    if command -v docker >/dev/null 2>&1; then
        docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
    fi
    if [ -n "$NGINX_CONF_HOST" ] && [ -f "$NGINX_CONF_HOST" ]; then
        rm -f "$NGINX_CONF_HOST"
    fi
}
trap cleanup EXIT INT TERM

echo "=== Browser Harness Docker Smoke (WEFT-408 / P6.7) ==="
echo "www:   $WWW_DIR"
echo "image: $IMAGE"
echo "port:  $PORT"
echo ""

if ! command -v docker >/dev/null 2>&1; then
    if [ "${BROWSER_SMOKE_SOFT:-0}" = "1" ]; then
        warn "docker not installed — soft skip (BROWSER_SMOKE_SOFT=1)"
        exit 0
    fi
    fail "docker not installed"
    fail "Install Docker Engine, or set BROWSER_SMOKE_SOFT=1 to skip locally"
    exit 1
fi

if ! docker info >/dev/null 2>&1; then
    if [ "${BROWSER_SMOKE_SOFT:-0}" = "1" ]; then
        warn "docker daemon not reachable — soft skip"
        exit 0
    fi
    fail "docker daemon not reachable"
    exit 1
fi

if [ ! -d "$PKG_DIR" ] || [ ! -f "$PKG_DIR/clawft_wasm_bg.wasm" ]; then
    if [ "$DO_BUILD" = true ]; then
        info "www/pkg/ missing — running scripts/build.sh browser"
        bash "$WORKSPACE_ROOT/scripts/build.sh" browser
    else
        fail "www/pkg/clawft_wasm_bg.wasm not found"
        fail "Build first: scripts/build.sh browser"
        fail "Or re-run with --build"
        exit 2
    fi
fi

if [ ! -f "$PKG_DIR/clawft_wasm_bg.wasm" ]; then
    fail "browser build did not produce pkg/clawft_wasm_bg.wasm"
    exit 1
fi

# Minimal nginx config: serve static files + correct WASM MIME type.
NGINX_CONF_HOST="$(mktemp -t weft-browser-smoke-XXXXXX.conf)"
cat > "$NGINX_CONF_HOST" <<'NGINX'
server {
    listen 80;
    server_name localhost;
    root /usr/share/nginx/html;
    index index.html;

    location / {
        try_files $uri $uri/ =404;
        add_header Cross-Origin-Opener-Policy same-origin always;
        add_header Cross-Origin-Embedder-Policy require-corp always;
    }

    types {
        application/javascript js mjs;
        application/wasm      wasm;
        text/html             html;
        text/css              css;
        application/json      json;
        text/plain            txt map;
    }
    default_type application/octet-stream;
}
NGINX

info "Starting $IMAGE as $CONTAINER_NAME"
docker run -d --rm \
    --name "$CONTAINER_NAME" \
    -p "127.0.0.1:${PORT}:80" \
    -v "$WWW_DIR:/usr/share/nginx/html:ro" \
    -v "$NGINX_CONF_HOST:/etc/nginx/conf.d/default.conf:ro" \
    "$IMAGE" >/dev/null

# Wait for nginx to accept connections
READY=false
for i in $(seq 1 30); do
    if curl -fsS -o /dev/null "http://127.0.0.1:${PORT}/" 2>/dev/null; then
        READY=true
        break
    fi
    sleep 0.2
done

if [ "$READY" != true ]; then
    fail "nginx did not become ready on port $PORT within ~6s"
    docker logs "$CONTAINER_NAME" 2>&1 | tail -20 || true
    exit 1
fi
ok "nginx ready on 127.0.0.1:${PORT}"

probe() {
    local path="$1"
    local expect_ctype="${2:-}"
    local url="http://127.0.0.1:${PORT}${path}"
    local tmp
    tmp="$(mktemp)"
    local code
    code=$(curl -sS -o "$tmp" -w "%{http_code}" \
        -D "${tmp}.hdr" \
        "$url" || echo "000")
    if [ "$code" != "200" ]; then
        fail "GET $path → HTTP $code (expected 200)"
        rm -f "$tmp" "${tmp}.hdr"
        return 1
    fi
    local bytes
    bytes=$(wc -c < "$tmp" | tr -d ' ')
    if [ "${bytes:-0}" -lt 1 ]; then
        fail "GET $path → empty body"
        rm -f "$tmp" "${tmp}.hdr"
        return 1
    fi
    if [ -n "$expect_ctype" ]; then
        local ctype
        ctype=$(grep -i '^content-type:' "${tmp}.hdr" | head -1 | tr -d '\r' || true)
        if ! echo "$ctype" | grep -qiE "$expect_ctype"; then
            fail "GET $path → Content-Type '$ctype' (expected to match /$expect_ctype/)"
            rm -f "$tmp" "${tmp}.hdr"
            return 1
        fi
    fi
    ok "GET $path → 200 (${bytes} bytes${expect_ctype:+, $expect_ctype})"
    rm -f "$tmp" "${tmp}.hdr"
    return 0
}

FAILED=0

# Core harness shell
probe "/" "text/html" || FAILED=$((FAILED + 1))
probe "/index.html" "text/html" || FAILED=$((FAILED + 1))
probe "/main.js" || FAILED=$((FAILED + 1))

# Optional worker harness (WEFT-400) — warn if absent, do not fail
if [ -f "$WWW_DIR/index-worker.html" ]; then
    probe "/index-worker.html" "text/html" || FAILED=$((FAILED + 1))
fi
if [ -f "$WWW_DIR/worker.js" ]; then
    probe "/worker.js" || FAILED=$((FAILED + 1))
fi

# Post-bindgen package (names from wasm-bindgen --target web)
probe "/pkg/clawft_wasm.js" || FAILED=$((FAILED + 1))
probe "/pkg/clawft_wasm_bg.wasm" "wasm\|application/octet-stream" || FAILED=$((FAILED + 1))

# GitHub Actions summary
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
    {
        echo "## Browser Harness Docker Smoke"
        echo ""
        if [ "$FAILED" -eq 0 ]; then
            echo "All probes passed against \`$IMAGE\` serving \`crates/clawft-wasm/www/\`."
        else
            echo "**$FAILED probe(s) failed.**"
        fi
        echo ""
    } >> "$GITHUB_STEP_SUMMARY"
fi

if [ "$FAILED" -gt 0 ]; then
    fail "$FAILED probe(s) failed"
    exit 1
fi

ok "harness pkg docker smoke clean"
exit 0
