#!/usr/bin/env bash
# export-model.sh — Export a Weaver session's learned model to weave-model.json.
#
# The exported file contains the learned schema (node types, edge types,
# patterns, tick config) but NOT the data. It is the deployable configuration
# for edge devices running the cognitive tick.
#
# Usage:
#   export-model.sh --domain <name> [--output <path>] [--min-confidence <0.0-1.0>] [--include-history]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCKET="${WEAVER_SOCKET:-/tmp/clawft-daemon.sock}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") --domain <name> [options]

Options:
  --domain <name>           Domain to export (required)
  --output <path>           Output file path (default: weave-model.json)
  --min-confidence <float>  Minimum confidence threshold; abort if below (default: 0.0)
  --include-history         Include full evolution history in the export
  --force                   Export even if below min-confidence

Examples:
  $(basename "$0") --domain weftos --output weave-model.json
  $(basename "$0") --domain weftos --min-confidence 0.75 --output deploy/model.json
  $(basename "$0") --domain weftos --include-history
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

domain="" output="weave-model.json" min_confidence="0.0" include_history=false force=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --domain)           domain="$2"; shift 2 ;;
        --output)           output="$2"; shift 2 ;;
        --min-confidence)   min_confidence="$2"; shift 2 ;;
        --include-history)  include_history=true; shift ;;
        --force)            force=true; shift ;;
        -h|--help)          usage ;;
        *)                  echo "Unknown option: $1" >&2; usage ;;
    esac
done

[[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

# Ensure output directory exists
output_dir="$(dirname "$output")"
if [[ "$output_dir" != "." ]] && [[ ! -d "$output_dir" ]]; then
    mkdir -p "$output_dir"
fi

payload=$(cat <<JSON
{
    "domain": "$domain",
    "output": "$output",
    "min_confidence": $min_confidence,
    "include_history": $include_history,
    "force": $force
}
JSON
)

echo "Exporting model for domain: $domain"
result=$(send_message "export" "$payload")

if [[ $? -eq 0 ]]; then
    echo "Model exported to: $output"
    # If the daemon returned JSON directly (future: write to file)
    if [[ -n "$result" ]]; then
        echo "$result" > "$output"
        echo "  File size: $(wc -c < "$output") bytes"
    fi
else
    echo "Export failed." >&2
    exit 1
fi
