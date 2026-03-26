#!/usr/bin/env bash
# generate-weave-toml.sh -- Generate weave.toml from codebase inspection.
#
# Usage: generate-weave-toml.sh --dir <path> [--domain <name>] [--mode <act|analyze|generate>] [--output <file>]

set -euo pipefail

DIR=""; DOMAIN=""; MODE="analyze"; OUTPUT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) DIR="$2"; shift 2 ;;
        --domain) DOMAIN="$2"; shift 2 ;;
        --mode) MODE="$2"; shift 2 ;;
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$DIR" ]]; then
    echo "Usage: generate-weave-toml.sh --dir <path> [--domain <name>] [--mode ...] [--output <file>]" >&2
    exit 1
fi

[[ ! -d "$DIR" ]] && echo "Error: $DIR is not a directory" >&2 && exit 1
[[ -z "$DOMAIN" ]] && DOMAIN=$(basename "$(realpath "$DIR")")

# Detect language
LANG="unknown"; SRC_PAT=""; SRC_ROOT="src/"
if [[ -f "$DIR/Cargo.toml" ]]; then
    LANG="rust"; SRC_PAT='"**/*.rs"'; SRC_ROOT="src/"
    grep -q '\[workspace\]' "$DIR/Cargo.toml" 2>/dev/null && SRC_ROOT="crates/"
elif [[ -f "$DIR/package.json" ]]; then
    LANG="typescript"; SRC_PAT='"**/*.ts", "**/*.tsx", "**/*.js"'; SRC_ROOT="src/"
elif [[ -f "$DIR/go.mod" ]]; then
    LANG="go"; SRC_PAT='"**/*.go"'; SRC_ROOT="./"
elif [[ -f "$DIR/pyproject.toml" ]] || [[ -f "$DIR/setup.py" ]]; then
    LANG="python"; SRC_PAT='"**/*.py"'; SRC_ROOT="src/"
else
    SRC_PAT='"**/*.rs", "**/*.ts", "**/*.py", "**/*.go"'
fi

FC=$(find "$DIR" -type f -not -path '*/.git/*' -not -path '*/target/*' -not -path '*/node_modules/*' 2>/dev/null | wc -l)
if [[ $FC -lt 1000 ]]; then DIM=128; MX=10000
elif [[ $FC -lt 10000 ]]; then DIM=256; MX=50000
else DIM=384; MX=100000; fi

GIT="false"; GB="main"
[[ -d "$DIR/.git" ]] && GIT="true" && GB=$(git -C "$DIR" branch --show-current 2>/dev/null || echo "main")

DR=""; for d in docs doc documentation; do [[ -d "$DIR/$d" ]] && DR="$d/" && break; done
SP=""; [[ -d "$DIR/.planning/sparc" ]] && SP=".planning/sparc/"

{
echo "# weave.toml -- ECC Loom for $DOMAIN ($LANG, $FC files)"
echo ""
echo "[domain]"
echo "name = \"$DOMAIN\""
echo "description = \"ECC cognitive workspace for $DOMAIN ($LANG)\""
echo "mode = \"$MODE\""
echo ""
echo "[tick]"
echo "interval_ms = 50"
echo "budget_ratio = 0.3"
echo "calibration_ticks = 100"
echo "adaptive = true"
echo "adaptive_window_s = 30"
echo ""
echo "[causal]"
echo "edge_types = [\"Causes\",\"Inhibits\",\"Correlates\",\"Enables\",\"Follows\",\"Contradicts\",\"TriggeredBy\",\"EvidenceFor\"]"
echo "decay_rate = 0.01"
echo "decay_threshold = 0.05"
echo "max_edges = $((MX * 3))"
echo "max_nodes = $MX"
echo ""
echo "[hnsw]"
echo "dimensions = $DIM"
echo "ef_search = 100"
echo "ef_construction = 200"
echo "max_entries = $MX"
echo ""
echo "[impulse]"
echo "ttl_ticks = 100"
echo "max_queue_depth = 1000"
echo ""
echo "[[patterns]]"
echo "name = \"ci-cd\""
echo "description = \"Build, test, deploy\""
echo "edge_sequence = [\"Causes\",\"EvidenceFor\",\"TriggeredBy\"]"
echo "recurrence = \"on_commit\""
echo ""
echo "[[patterns]]"
echo "name = \"feature-dev\""
echo "description = \"Plan, implement, review\""
echo "edge_sequence = [\"Causes\",\"Follows\",\"Enables\",\"EvidenceFor\"]"
echo "recurrence = \"on_event:pr_opened\""
echo ""
echo "[[patterns]]"
echo "name = \"bug-lifecycle\""
echo "description = \"Report, triage, fix, verify\""
echo "edge_sequence = [\"Causes\",\"Enables\",\"Follows\",\"EvidenceFor\"]"
echo "recurrence = \"on_event:issue_opened\""
echo ""
echo "[[patterns]]"
echo "name = \"architecture-decision\""
echo "description = \"Propose, discuss, decide, document\""
echo "edge_sequence = [\"Causes\",\"Correlates\",\"Enables\",\"EvidenceFor\"]"
echo "recurrence = \"periodic:weekly\""

[[ "$GIT" == "true" ]] && echo "" && echo "[[sources]]" && echo "type = \"git_log\"" && echo "path = \".\"" && echo "branch = \"$GB\""

echo ""
echo "[[sources]]"
echo "type = \"file_tree\""
echo "root = \"$SRC_ROOT\""
echo "patterns = [$SRC_PAT]"

[[ -n "$DR" ]] && echo "" && echo "[[sources]]" && echo "type = \"documentation\"" && echo "root = \"$DR\""
[[ -n "$SP" ]] && echo "" && echo "[[sources]]" && echo "type = \"sparc_plan\"" && echo "path = \"$SP\""

echo ""
echo "[meta]"
echo "enabled = true"
echo ""
echo "[[meta.conversations]]"
echo "name = \"testing-strategy\""
echo "relates_to = [\"implementation\"]"
echo "pattern = \"plan-implement-review-reflect\""
echo ""
echo "[[meta.conversations]]"
echo "name = \"build-pipeline\""
echo "relates_to = [\"ci-cd\"]"
echo "pattern = \"configure-test-deploy-monitor\""
echo ""
echo "[[meta.conversations]]"
echo "name = \"architecture\""
echo "relates_to = [\"feature-dev\"]"
echo "pattern = \"propose-discuss-decide-document\""
} | if [[ -n "$OUTPUT" ]]; then
    cat > "$OUTPUT"
    echo "Generated weave.toml for '$DOMAIN' at $OUTPUT (lang=$LANG files=$FC dims=$DIM)" >&2
else
    cat
fi
