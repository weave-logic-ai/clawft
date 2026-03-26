#!/usr/bin/env bash
# confidence-report.sh — Evaluate and display the current confidence report
# for a Weaver modeling session.
#
# Sends a message to the WeaverEngine SystemService via the daemon socket.
#
# Usage:
#   confidence-report.sh --domain <name> [--edge <edge>] [--verbose] [--watch] [--json]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCKET="${WEAVER_SOCKET:-/tmp/clawft-daemon.sock}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") --domain <name> [options]

Options:
  --domain <name>   Domain to report on (required)
  --edge <edge>     Filter to a specific edge (e.g., "commit->test")
  --verbose         Include suggestion details and meta-loom summary
  --watch           Stream confidence updates in real-time
  --json            Output raw JSON instead of formatted text

Examples:
  $(basename "$0") --domain weftos
  $(basename "$0") --domain weftos --edge "commit->test" --verbose
  $(basename "$0") --domain weftos --watch
USAGE
    exit 1
}

send_message() {
    local command="$1"
    local payload="$2"

    if command -v weaver &>/dev/null; then
        weaver ecc "$command" "$payload"
    elif [ -S "$SOCKET" ]; then
        echo "{\"service\":\"ecc.weaver\",\"command\":\"$command\",\"payload\":$payload}" | \
            socat - UNIX-CONNECT:"$SOCKET" 2>/dev/null
    else
        echo "Error: Weaver daemon not running." >&2
        exit 1
    fi
}

domain="" edge="" verbose=false watch=false json_out=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --domain)   domain="$2"; shift 2 ;;
        --edge)     edge="$2"; shift 2 ;;
        --verbose)  verbose=true; shift ;;
        --watch)    watch=true; shift ;;
        --json)     json_out=true; shift ;;
        -h|--help)  usage ;;
        *)          echo "Unknown option: $1" >&2; usage ;;
    esac
done

[[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

payload=$(cat <<JSON
{
    "domain": "$domain",
    "edge": "$edge",
    "verbose": $verbose,
    "watch": $watch,
    "json": $json_out
}
JSON
)

if [[ "$watch" == "true" ]]; then
    echo "Watching confidence for domain: $domain (Ctrl+C to stop)"
    echo "---"
    # In watch mode, the daemon streams updates
    send_message "confidence.watch" "$payload"
else
    send_message "confidence" "$payload"
fi
