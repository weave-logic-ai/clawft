#!/usr/bin/env bash
# add-source.sh — Add a new data source to a running Weaver modeling session.
#
# When a new source is added, the Weaver re-evaluates its entire model,
# not just the new data. The confidence report updates accordingly.
#
# Usage:
#   add-source.sh --domain <name> --type <source_type> [source-specific options]

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOCKET="${WEAVER_SOCKET:-/tmp/clawft-daemon.sock}"

usage() {
    cat <<USAGE
Usage: $(basename "$0") --domain <name> --type <type> [options]

Required:
  --domain <name>    Target domain session (required)
  --type <type>      Source type (required)

Source types and their options:
  git_log:
    --path <path>      Repository path (default: .)
    --branch <name>    Branch to track (default: main)
    --watch            Enable live tracking via file watcher

  file_tree:
    --root <path>      Root directory to scan
    --patterns <glob>  File patterns, comma-separated (e.g., "**/*.rs,**/*.ts")
    --watch            Enable live tracking

  ci_pipeline:
    --webhook-url <url>  Webhook endpoint for CI events

  issue_tracker:
    --api-url <url>      Issue tracker API URL
    --auth-env <var>     Environment variable holding auth token

  documentation:
    --root <path>      Documentation root directory

  sparc_plan:
    --path <path>      SPARC planning directory

  custom_stream:
    --name <name>      Stream identifier
    --format <fmt>     Format: json_lines, csv, custom

Examples:
  $(basename "$0") --domain weftos --type git_log --path . --branch main --watch
  $(basename "$0") --domain weftos --type ci_pipeline --webhook-url https://ci.example.com/hooks
  $(basename "$0") --domain weftos --type file_tree --root src/ --patterns "**/*.rs" --watch
  $(basename "$0") --domain weftos --type issue_tracker --api-url https://api.github.com/repos/org/repo
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

domain="" source_type=""
# Source-specific options
path="" branch="main" root="" patterns="" webhook_url="" api_url="" auth_env=""
stream_name="" stream_format="json_lines" watch=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --domain)       domain="$2"; shift 2 ;;
        --type)         source_type="$2"; shift 2 ;;
        --path)         path="$2"; shift 2 ;;
        --branch)       branch="$2"; shift 2 ;;
        --root)         root="$2"; shift 2 ;;
        --patterns)     patterns="$2"; shift 2 ;;
        --webhook-url)  webhook_url="$2"; shift 2 ;;
        --api-url)      api_url="$2"; shift 2 ;;
        --auth-env)     auth_env="$2"; shift 2 ;;
        --name)         stream_name="$2"; shift 2 ;;
        --format)       stream_format="$2"; shift 2 ;;
        --watch)        watch=true; shift ;;
        -h|--help)      usage ;;
        *)              echo "Unknown option: $1" >&2; usage ;;
    esac
done

[[ -z "$domain" ]] && { echo "Error: --domain is required" >&2; usage; }
[[ -z "$source_type" ]] && { echo "Error: --type is required" >&2; usage; }

# Resolve relative paths to absolute
[[ -n "$path" ]] && path="$(cd "$path" 2>/dev/null && pwd || echo "$path")"
[[ -n "$root" ]] && root="$(cd "$root" 2>/dev/null && pwd || echo "$root")"

# Build source-specific payload
case "$source_type" in
    git_log)
        source_payload="{\"path\":\"${path:-.}\",\"branch\":\"$branch\",\"watch\":$watch}"
        ;;
    file_tree)
        [[ -z "$root" ]] && { echo "Error: --root is required for file_tree" >&2; usage; }
        # Convert comma-separated patterns to JSON array
        patterns_json=$(echo "$patterns" | tr ',' '\n' | sed 's/^/"/;s/$/"/' | paste -sd ',' -)
        source_payload="{\"root\":\"$root\",\"patterns\":[$patterns_json],\"watch\":$watch}"
        ;;
    ci_pipeline)
        [[ -z "$webhook_url" ]] && { echo "Error: --webhook-url is required for ci_pipeline" >&2; usage; }
        source_payload="{\"webhook_url\":\"$webhook_url\"}"
        ;;
    issue_tracker)
        [[ -z "$api_url" ]] && { echo "Error: --api-url is required for issue_tracker" >&2; usage; }
        source_payload="{\"api_url\":\"$api_url\",\"auth_env_var\":\"$auth_env\"}"
        ;;
    documentation)
        [[ -z "$root" ]] && { echo "Error: --root is required for documentation" >&2; usage; }
        source_payload="{\"root\":\"$root\"}"
        ;;
    sparc_plan)
        [[ -z "$path" ]] && { echo "Error: --path is required for sparc_plan" >&2; usage; }
        source_payload="{\"path\":\"$path\"}"
        ;;
    custom_stream)
        [[ -z "$stream_name" ]] && { echo "Error: --name is required for custom_stream" >&2; usage; }
        source_payload="{\"name\":\"$stream_name\",\"format\":\"$stream_format\"}"
        ;;
    *)
        echo "Error: Unknown source type: $source_type" >&2
        echo "Valid types: git_log, file_tree, ci_pipeline, issue_tracker, documentation, sparc_plan, custom_stream" >&2
        exit 1
        ;;
esac

payload=$(cat <<JSON
{
    "domain": "$domain",
    "source_type": "$source_type",
    "source_config": $source_payload
}
JSON
)

echo "Adding $source_type source to domain: $domain"
send_message "source.add" "$payload"
echo "Source added. The Weaver will re-evaluate the model on the next cognitive tick."
