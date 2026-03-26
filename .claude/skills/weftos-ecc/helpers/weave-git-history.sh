#!/usr/bin/env bash
# weave-git-history.sh -- Ingest git log into a causal graph structure.
#
# Usage: weave-git-history.sh --repo <path> [--branch <name>] [--depth <n>] [--output <file>]
#
# Each commit becomes a node. Edges: sequential (Follows), merges (Enables),
# issue refs (TriggeredBy).

set -euo pipefail

REPO=""; BRANCH="main"; DEPTH=500; OUTPUT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo) REPO="$2"; shift 2 ;;
        --branch) BRANCH="$2"; shift 2 ;;
        --depth) DEPTH="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$REPO" ]]; then
    echo "Usage: weave-git-history.sh --repo <path> [--branch <name>] [--depth <n>] [--output <file>]" >&2
    exit 1
fi

if [[ ! -d "$REPO/.git" ]]; then
    echo "Error: $REPO is not a git repository" >&2
    exit 1
fi

GIT_LOG=$(git -C "$REPO" log "$BRANCH" --max-count="$DEPTH" --format='%H|%P|%an|%aI|%s' 2>/dev/null)

if [[ -z "$GIT_LOG" ]]; then
    echo "Error: No commits found on branch $BRANCH" >&2
    exit 1
fi

NODES=""; EDGES=""
NODE_COUNT=0; EDGE_COUNT=0; ISSUE_NODES=""

while IFS='|' read -r hash parents author date subject; do
    short_hash="${hash:0:7}"
    safe_subject=$(echo "$subject" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\t/\\t/g')

    NODE="{\"label\":\"commit:$short_hash\",\"node_type\":\"commit\",\"metadata\":{\"oid\":\"$hash\",\"author\":\"$author\",\"date\":\"$date\",\"message\":\"$safe_subject\"}}"
    if [[ -n "$NODES" ]]; then NODES="$NODES,$NODE"; else NODES="$NODE"; fi
    NODE_COUNT=$((NODE_COUNT + 1))

    for parent in $parents; do
        parent_short="${parent:0:7}"
        EDGE="{\"source\":\"commit:$parent_short\",\"target\":\"commit:$short_hash\",\"edge_type\":\"Follows\",\"weight\":1.0}"
        if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
        EDGE_COUNT=$((EDGE_COUNT + 1))
    done

    parent_count=$(echo "$parents" | wc -w)
    if [[ $parent_count -gt 1 ]]; then
        for parent in $parents; do
            parent_short="${parent:0:7}"
            EDGE="{\"source\":\"commit:$parent_short\",\"target\":\"commit:$short_hash\",\"edge_type\":\"Enables\",\"weight\":0.9}"
            if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
            EDGE_COUNT=$((EDGE_COUNT + 1))
        done
    fi

    issue_refs=$(echo "$subject" | grep -oE '#[0-9]+|GH-[0-9]+' || true)
    for ref in $issue_refs; do
        issue_id=$(echo "$ref" | sed 's/[^0-9]//g')
        issue_label="issue:$issue_id"
        if ! echo "$ISSUE_NODES" | grep -q "$issue_label"; then
            ISSUE_NODE="{\"label\":\"$issue_label\",\"node_type\":\"issue\",\"metadata\":{\"issue_number\":$issue_id}}"
            if [[ -n "$NODES" ]]; then NODES="$NODES,$ISSUE_NODE"; else NODES="$ISSUE_NODE"; fi
            ISSUE_NODES="$ISSUE_NODES $issue_label"
            NODE_COUNT=$((NODE_COUNT + 1))
        fi
        EDGE="{\"source\":\"$issue_label\",\"target\":\"commit:$short_hash\",\"edge_type\":\"TriggeredBy\",\"weight\":0.8}"
        if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
        EDGE_COUNT=$((EDGE_COUNT + 1))
    done
done <<< "$GIT_LOG"

JSON=$(cat <<ENDJSON
{
  "git_weave": {
    "repository": "$(realpath "$REPO")",
    "branch": "$BRANCH",
    "depth": $DEPTH,
    "nodes_created": $NODE_COUNT,
    "edges_created": $EDGE_COUNT,
    "nodes": [$NODES],
    "edges": [$EDGES]
  }
}
ENDJSON
)

if [[ -n "$OUTPUT" ]]; then
    echo "$JSON" > "$OUTPUT"
    echo "Git history weave written to $OUTPUT ($NODE_COUNT nodes, $EDGE_COUNT edges)"
else
    echo "$JSON"
fi
