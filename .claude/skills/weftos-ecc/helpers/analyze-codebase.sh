#!/usr/bin/env bash
# analyze-codebase.sh -- Analyze a directory and produce a JSON ECC initialization plan.
#
# Usage: analyze-codebase.sh <directory> [--output <file>]
#
# Walks the directory tree, classifies files, identifies modules and dependencies,
# maps test-to-source relationships, and outputs a JSON AnalysisPlan suitable for
# feeding into `weaver ecc init`.

set -euo pipefail

DIR="${1:?Usage: analyze-codebase.sh <directory> [--output <file>]}"
OUTPUT=""

shift
while [[ $# -gt 0 ]]; do
    case "$1" in
        --output) OUTPUT="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ ! -d "$DIR" ]]; then
    echo "Error: $DIR is not a directory" >&2
    exit 1
fi

# Collect file inventory
SOURCE_COUNT=0
TEST_COUNT=0
DOC_COUNT=0
CONFIG_COUNT=0
OTHER_COUNT=0

while IFS= read -r file; do
    name=$(basename "$file")
    case "$file" in
        *_test.* | *_spec.* | */tests/* | */test/* | *test_*.*)
            TEST_COUNT=$((TEST_COUNT + 1)) ;;
        *.md | *.rst | *.txt | *.adoc)
            DOC_COUNT=$((DOC_COUNT + 1)) ;;
        *.toml | *.yaml | *.yml | *.json | *.ini | *.cfg | *.conf)
            CONFIG_COUNT=$((CONFIG_COUNT + 1)) ;;
        *.rs | *.ts | *.js | *.py | *.go | *.java | *.c | *.cpp | *.h)
            SOURCE_COUNT=$((SOURCE_COUNT + 1)) ;;
        *)
            OTHER_COUNT=$((OTHER_COUNT + 1)) ;;
    esac
done < <(find "$DIR" \
    -not -path '*/.git/*' \
    -not -path '*/target/*' \
    -not -path '*/node_modules/*' \
    -not -path '*/__pycache__/*' \
    -type f 2>/dev/null | head -10000)

TOTAL=$((SOURCE_COUNT + TEST_COUNT + DOC_COUNT + CONFIG_COUNT + OTHER_COUNT))
EST_VECTORS=$((SOURCE_COUNT + DOC_COUNT))
EST_EDGES=$((SOURCE_COUNT * 2 + TEST_COUNT))

if [[ $EST_VECTORS -lt 1000 ]]; then
    REC_DIMS=128
elif [[ $EST_VECTORS -lt 10000 ]]; then
    REC_DIMS=256
else
    REC_DIMS=384
fi

# Detect language and build system
LANG="unknown"; BUILD_SYSTEM="unknown"
if [[ -f "$DIR/Cargo.toml" ]]; then LANG="rust"; BUILD_SYSTEM="cargo"
elif [[ -f "$DIR/package.json" ]]; then LANG="typescript"; BUILD_SYSTEM="npm"
elif [[ -f "$DIR/go.mod" ]]; then LANG="go"; BUILD_SYSTEM="go"
elif [[ -f "$DIR/pyproject.toml" ]] || [[ -f "$DIR/setup.py" ]]; then LANG="python"; BUILD_SYSTEM="pip"
fi

HAS_GIT="false"; COMMIT_COUNT=0
if [[ -d "$DIR/.git" ]]; then
    HAS_GIT="true"
    COMMIT_COUNT=$(git -C "$DIR" rev-list --count HEAD 2>/dev/null || echo 0)
fi

JSON=$(cat <<ENDJSON
{
  "analysis_plan": {
    "directory": "$(realpath "$DIR")",
    "language": "$LANG",
    "build_system": "$BUILD_SYSTEM",
    "has_git": $HAS_GIT,
    "commit_count": $COMMIT_COUNT,
    "file_counts": {
      "total": $TOTAL,
      "source": $SOURCE_COUNT,
      "test": $TEST_COUNT,
      "doc": $DOC_COUNT,
      "config": $CONFIG_COUNT,
      "other": $OTHER_COUNT
    },
    "estimates": {
      "vectors": $EST_VECTORS,
      "edges": $EST_EDGES,
      "recommended_dimensions": $REC_DIMS
    },
    "suggested_patterns": [
      {"name":"ci-cd","description":"Build, test, deploy lifecycle","edge_sequence":["Causes","EvidenceFor","TriggeredBy"],"recurrence":"on_commit"},
      {"name":"feature-dev","description":"Plan, implement, review cycle","edge_sequence":["Causes","Follows","Enables","EvidenceFor"],"recurrence":"on_event:pr_opened"}
    ],
    "meta_conversations": [
      {"name":"testing-strategy","description":"How testing is organized","relates_to":["implementation"],"pattern":"plan-implement-review-reflect"},
      {"name":"build-pipeline","description":"How code is built and deployed","relates_to":["ci-cd"],"pattern":"configure-test-deploy-monitor"}
    ]
  }
}
ENDJSON
)

if [[ -n "$OUTPUT" ]]; then
    echo "$JSON" > "$OUTPUT"
    echo "Analysis written to $OUTPUT"
else
    echo "$JSON"
fi
