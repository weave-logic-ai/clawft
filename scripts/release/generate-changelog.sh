#!/usr/bin/env bash
set -euo pipefail

# generate-changelog.sh — Generate release notes from git commit history.
#
# Reads conventional commits (feat:, fix:, docs:, etc.) between two refs
# and outputs grouped, formatted markdown suitable for release notes.
#
# Usage:
#   ./scripts/release/generate-changelog.sh [from-tag] [to-ref]
#
# Defaults:
#   from-tag  — most recent git tag (or initial commit if no tags exist)
#   to-ref    — HEAD
#
# Examples:
#   ./scripts/release/generate-changelog.sh                  # last tag..HEAD
#   ./scripts/release/generate-changelog.sh v0.1.0           # v0.1.0..HEAD
#   ./scripts/release/generate-changelog.sh v0.1.0 v0.2.0   # v0.1.0..v0.2.0

# ---------------------------------------------------------------------------
# Resolve range
# ---------------------------------------------------------------------------

FROM_REF="${1:-}"
TO_REF="${2:-HEAD}"

if [[ -z "$FROM_REF" ]]; then
    FROM_REF=$(git describe --tags --abbrev=0 2>/dev/null || true)
    if [[ -z "$FROM_REF" ]]; then
        # No tags at all — use the root commit
        FROM_REF=$(git rev-list --max-parents=0 HEAD 2>/dev/null | head -1)
        if [[ -z "$FROM_REF" ]]; then
            echo "Error: could not determine a starting ref (no tags, no commits)." >&2
            exit 1
        fi
        RANGE="${FROM_REF}..${TO_REF}"
    else
        RANGE="${FROM_REF}..${TO_REF}"
    fi
else
    RANGE="${FROM_REF}..${TO_REF}"
fi

echo "# Release Notes"
echo ""
echo "Changes from \`${FROM_REF}\` to \`${TO_REF}\`"
echo ""

# ---------------------------------------------------------------------------
# Collect commits
# ---------------------------------------------------------------------------

COMMITS=$(git log --pretty=format:"%s|%h|%an" "$RANGE" 2>/dev/null || true)

if [[ -z "$COMMITS" ]]; then
    echo "_No commits found in range ${RANGE}._"
    exit 0
fi

# ---------------------------------------------------------------------------
# Conventional-commit type mapping
# ---------------------------------------------------------------------------

declare -A TYPE_LABELS=(
    [feat]="Features"
    [fix]="Bug Fixes"
    [docs]="Documentation"
    [refactor]="Refactoring"
    [perf]="Performance"
    [test]="Tests"
    [ci]="CI/CD"
    [build]="Build"
    [chore]="Chores"
    [style]="Style"
    [revert]="Reverts"
)

# Ordered list so the output is deterministic and logical
TYPE_ORDER=(feat fix perf refactor docs test ci build style chore revert)

# Associative arrays to hold grouped lines
declare -A TYPE_LINES
for t in "${TYPE_ORDER[@]}"; do
    TYPE_LINES[$t]=""
done

UNCATEGORIZED=""
HAS_CONVENTIONAL=false

while IFS= read -r line; do
    [[ -z "$line" ]] && continue

    SUBJECT="${line%%|*}"
    REST="${line#*|}"
    HASH="${REST%%|*}"
    AUTHOR="${REST#*|}"

    MATCHED=false
    for t in "${TYPE_ORDER[@]}"; do
        # Match "type:" or "type(scope):" at the start of the subject
        if [[ "$SUBJECT" =~ ^${t}(\(.+\))?:\ (.+)$ ]]; then
            SCOPE="${BASH_REMATCH[1]}"
            MSG="${BASH_REMATCH[2]}"
            # Strip surrounding parens from scope
            SCOPE="${SCOPE#\(}"
            SCOPE="${SCOPE%\)}"

            if [[ -n "$SCOPE" ]]; then
                TYPE_LINES[$t]+="- **${SCOPE}**: ${MSG} (\`${HASH}\`)"$'\n'
            else
                TYPE_LINES[$t]+="- ${MSG} (\`${HASH}\`)"$'\n'
            fi
            MATCHED=true
            HAS_CONVENTIONAL=true
            break
        fi
    done

    if [[ "$MATCHED" == "false" ]]; then
        UNCATEGORIZED+="- ${SUBJECT} (\`${HASH}\`)"$'\n'
    fi
done <<< "$COMMITS"

# ---------------------------------------------------------------------------
# Output
# ---------------------------------------------------------------------------

if [[ "$HAS_CONVENTIONAL" == "true" ]]; then
    for t in "${TYPE_ORDER[@]}"; do
        if [[ -n "${TYPE_LINES[$t]}" ]]; then
            echo "## ${TYPE_LABELS[$t]}"
            echo ""
            echo -n "${TYPE_LINES[$t]}"
            echo ""
        fi
    done

    if [[ -n "$UNCATEGORIZED" ]]; then
        echo "## Other"
        echo ""
        echo -n "$UNCATEGORIZED"
        echo ""
    fi
else
    # No conventional commits found — fall back to a flat list
    echo "## Commits"
    echo ""
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        SUBJECT="${line%%|*}"
        REST="${line#*|}"
        HASH="${REST%%|*}"
        echo "- ${SUBJECT} (\`${HASH}\`)"
    done <<< "$COMMITS"
    echo ""
fi

# ---------------------------------------------------------------------------
# Stats
# ---------------------------------------------------------------------------

COMMIT_COUNT=$(echo "$COMMITS" | wc -l | tr -d ' ')
AUTHOR_COUNT=$(echo "$COMMITS" | awk -F'|' '{print $3}' | sort -u | wc -l | tr -d ' ')

echo "---"
echo ""
echo "_${COMMIT_COUNT} commit(s) by ${AUTHOR_COUNT} contributor(s)_"
