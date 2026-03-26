#!/usr/bin/env bash
# weave-source-tree.sh -- Ingest file/module structure into causal graph nodes.
#
# Usage: weave-source-tree.sh --root <path> [--patterns "**/*.rs,**/*.ts"] [--output <file>]

set -euo pipefail

ROOT=""; PATTERNS="**/*.rs,**/*.ts,**/*.js,**/*.py,**/*.go"; OUTPUT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root) ROOT="$2"; shift 2 ;;
        --patterns) PATTERNS="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$ROOT" ]]; then
    echo "Usage: weave-source-tree.sh --root <path> [--patterns ...] [--output <file>]" >&2
    exit 1
fi

if [[ ! -d "$ROOT" ]]; then
    echo "Error: $ROOT is not a directory" >&2
    exit 1
fi

# Build find expression from comma-separated glob patterns
FIND_EXPR=""
IFS=',' read -ra PAT_ARR <<< "$PATTERNS"
for pat in "${PAT_ARR[@]}"; do
    ext=$(echo "$pat" | sed 's/.*\*\.//')
    if [[ -n "$FIND_EXPR" ]]; then FIND_EXPR="$FIND_EXPR -o"; fi
    FIND_EXPR="$FIND_EXPR -name \"*.$ext\""
done

FILES=$(eval find "\"$ROOT\"" \
    -not -path "'*/.git/*'" \
    -not -path "'*/target/*'" \
    -not -path "'*/node_modules/*'" \
    -type f \\\( $FIND_EXPR \\\) 2>/dev/null | sort)

if [[ -z "$FILES" ]]; then
    echo "Error: No files found matching patterns: $PATTERNS" >&2
    exit 1
fi

DIRS=$(echo "$FILES" | while read -r f; do dirname "$f"; done | sort -u)

NODES=""; EDGES=""
NODE_COUNT=0; EDGE_COUNT=0

# Namespace nodes (directories)
while IFS= read -r dir; do
    rel_dir=$(realpath --relative-to="$ROOT" "$dir" 2>/dev/null || echo "$dir")
    label="ns:$rel_dir"
    NODE="{\"label\":\"$label\",\"node_type\":\"namespace\",\"metadata\":{\"path\":\"$rel_dir\"}}"
    if [[ -n "$NODES" ]]; then NODES="$NODES,$NODE"; else NODES="$NODE"; fi
    NODE_COUNT=$((NODE_COUNT + 1))

    parent_dir=$(dirname "$dir")
    if [[ "$parent_dir" != "$dir" ]] && [[ "$parent_dir" != "." ]]; then
        parent_rel=$(realpath --relative-to="$ROOT" "$parent_dir" 2>/dev/null || echo "$parent_dir")
        EDGE="{\"source\":\"ns:$parent_rel\",\"target\":\"$label\",\"edge_type\":\"Enables\",\"weight\":1.0}"
        if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
        EDGE_COUNT=$((EDGE_COUNT + 1))
    fi
done <<< "$DIRS"

# File nodes
while IFS= read -r file; do
    rel_file=$(realpath --relative-to="$ROOT" "$file" 2>/dev/null || echo "$file")
    filename=$(basename "$file")
    dir_rel=$(realpath --relative-to="$ROOT" "$(dirname "$file")" 2>/dev/null || echo "$(dirname "$file")")
    file_size=$(stat -c%s "$file" 2>/dev/null || echo 0)

    ftype="source"
    case "$filename" in
        *_test.* | *_spec.* | test_*.* | *Test.* | *Spec.*) ftype="test" ;;
        *.toml | *.yaml | *.yml | *.json) ftype="config" ;;
        *.md | *.rst | *.txt) ftype="doc" ;;
    esac
    case "$dir_rel" in */tests/* | */test/* | */spec/*) ftype="test" ;; esac

    label="file:$rel_file"
    NODE="{\"label\":\"$label\",\"node_type\":\"$ftype\",\"metadata\":{\"path\":\"$rel_file\",\"size\":$file_size}}"
    if [[ -n "$NODES" ]]; then NODES="$NODES,$NODE"; else NODES="$NODE"; fi
    NODE_COUNT=$((NODE_COUNT + 1))

    EDGE="{\"source\":\"ns:$dir_rel\",\"target\":\"$label\",\"edge_type\":\"Enables\",\"weight\":0.9}"
    if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
    EDGE_COUNT=$((EDGE_COUNT + 1))

    # Test -> source EvidenceFor edges
    if [[ "$ftype" == "test" ]]; then
        source_name=$(echo "$filename" | sed 's/_test\././; s/_spec\././; s/^test_//; s/Test\././')
        potential=$(find "$ROOT" -name "$source_name" -not -path '*/test*' -type f 2>/dev/null | head -1)
        if [[ -n "$potential" ]]; then
            src_rel=$(realpath --relative-to="$ROOT" "$potential" 2>/dev/null || echo "$potential")
            EDGE="{\"source\":\"file:$rel_file\",\"target\":\"file:$src_rel\",\"edge_type\":\"EvidenceFor\",\"weight\":0.9}"
            if [[ -n "$EDGES" ]]; then EDGES="$EDGES,$EDGE"; else EDGES="$EDGE"; fi
            EDGE_COUNT=$((EDGE_COUNT + 1))
        fi
    fi
done <<< "$FILES"

JSON="{\"source_tree_weave\":{\"root\":\"$(realpath "$ROOT")\",\"patterns\":\"$PATTERNS\",\"nodes_created\":$NODE_COUNT,\"edges_created\":$EDGE_COUNT,\"nodes\":[$NODES],\"edges\":[$EDGES]}}"

if [[ -n "$OUTPUT" ]]; then
    echo "$JSON" > "$OUTPUT"
    echo "Source tree weave: $OUTPUT ($NODE_COUNT nodes, $EDGE_COUNT edges)"
else
    echo "$JSON"
fi
