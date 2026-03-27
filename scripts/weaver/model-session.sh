#!/usr/bin/env bash
# model-session.sh — Start, resume, or inspect a Weaver modeling session.
#
# This script sends messages to the WeaverEngine SystemService via the
# daemon Unix socket. The Weaver process runs inside the WeftOS kernel;
# this script is a thin CLI wrapper.
#
# Usage:
#   model-session.sh start --domain <name> [--git <path>] [--context <text>] [--goal <text>]
#   model-session.sh resume --domain <name>
#   model-session.sh stop --domain <name>
#   model-session.sh list
#   model-session.sh status --domain <name>

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Default socket path (matches kernel daemon config)
SOCKET="${WEAVER_SOCKET:-/tmp/clawft-daemon.sock}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") <command> [options]

Commands:
  start   Start a new modeling session
  resume  Resume an existing session
  stop    Stop a session (checkpoint and deactivate)
  list    List all active sessions
  status  Show session status and current confidence

Start options:
  --domain <name>      Domain identifier (required)
  --git <path>         Git repository path (adds git_log + file_tree sources)
  --context <text>     Domain context description
  --goal <text>        Target modeling goal (e.g., "0.8+ confidence on commit->test")
  --mode <mode>        Operating mode: analyze (default), act, generate

Examples:
  $(basename "$0") start --domain weftos --git . --goal "model module deps at 0.8+"
  $(basename "$0") resume --domain weftos
  $(basename "$0") status --domain weftos
  $(basename "$0") list
USAGE
    exit 1
}

send_message() {
    local command="$1"
    local payload="$2"

    if command -v weaver &>/dev/null; then
        # Use the weaver CLI if available
        weaver ecc "$command" "$payload"
    elif [ -S "$SOCKET" ]; then
        # Fall back to direct socket communication
        echo "{\"service\":\"ecc.weaver\",\"command\":\"$command\",\"payload\":$payload}" | \
            socat - UNIX-CONNECT:"$SOCKET" 2>/dev/null
    else
        echo "Error: Weaver daemon not running. Start the kernel first." >&2
        echo "  Expected socket at: $SOCKET" >&2
        exit 1
    fi
}

cmd="${1:-}"
shift || true

case "$cmd" in
    start)
        domain="" git_path="" context="" goal="" mode="analyze"
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --domain)  domain="$2"; shift 2 ;;
                --git)     git_path="$2"; shift 2 ;;
                --context) context="$2"; shift 2 ;;
                --goal)    goal="$2"; shift 2 ;;
                --mode)    mode="$2"; shift 2 ;;
                *) echo "Unknown option: $1" >&2; usage ;;
            esac
        done
        [[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

        # Resolve git path to absolute
        if [[ -n "$git_path" ]]; then
            git_path="$(cd "$git_path" && pwd)"
        fi

        payload=$(cat <<JSON
{
    "domain": "$domain",
    "git_path": "$git_path",
    "context": "$context",
    "goal": "$goal",
    "mode": "$mode"
}
JSON
)
        echo "Starting modeling session for domain: $domain"
        send_message "session.start" "$payload"
        ;;

    resume)
        domain=""
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --domain) domain="$2"; shift 2 ;;
                *) echo "Unknown option: $1" >&2; usage ;;
            esac
        done
        [[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

        echo "Resuming session: $domain"
        send_message "session.resume" "{\"domain\":\"$domain\"}"
        ;;

    stop)
        domain=""
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --domain) domain="$2"; shift 2 ;;
                *) echo "Unknown option: $1" >&2; usage ;;
            esac
        done
        [[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

        echo "Stopping session: $domain"
        send_message "session.stop" "{\"domain\":\"$domain\"}"
        ;;

    list)
        send_message "session.list" "{}"
        ;;

    status)
        domain=""
        while [[ $# -gt 0 ]]; do
            case "$1" in
                --domain) domain="$2"; shift 2 ;;
                *) echo "Unknown option: $1" >&2; usage ;;
            esac
        done
        [[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }

        send_message "session.status" "{\"domain\":\"$domain\"}"
        ;;

    *)
        usage
        ;;
esac
