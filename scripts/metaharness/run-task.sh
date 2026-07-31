#!/usr/bin/env bash
# Run a named MetaHarness task from .metaharness/tasks/<id>.md (WEFT-725).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
TASK="${1:-}"
if [[ -z "$TASK" ]]; then
  echo "usage: $0 <gate|plane-dag|fusion-view|task-id>" >&2
  echo "available:" >&2
  ls -1 "$ROOT/.metaharness/tasks" 2>/dev/null | sed 's/\.md$//' | sed 's/^/  /' || true
  exit 2
fi
TASK_FILE="$ROOT/.metaharness/tasks/${TASK}.md"
if [[ ! -f "$TASK_FILE" ]]; then
  echo "unknown task: $TASK ($TASK_FILE)" >&2
  exit 2
fi
# Extract command: from frontmatter
CMD=$(awk '
  BEGIN { in_fm=0 }
  /^---$/ { if (in_fm) exit; in_fm=1; next }
  in_fm && /^command:/ {
    sub(/^command:[[:space:]]*/, "")
    print
    exit
  }
' "$TASK_FILE")
if [[ -z "${CMD:-}" ]]; then
  echo "no command: in frontmatter of $TASK_FILE" >&2
  exit 2
fi
echo "== MetaHarness task: $TASK =="
echo "command: $CMD"
echo "doc: $TASK_FILE"
# shellcheck disable=SC2086
exec bash -lc "$CMD"
