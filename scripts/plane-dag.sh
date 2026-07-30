#!/usr/bin/env bash
# plane-dag — dependency DAG helper for WeftOS Plane work items.
# Works with docs/plans/plane-dag.json and the plane-workflow HTTP wrapper.
#
# Usage:
#   scripts/plane-dag.sh refresh              # re-pull Plane → rebuild DAG + inventory + waves
#   scripts/plane-dag.sh ready [flags]        # list ready (Wave 0) tickets
#   scripts/plane-dag.sh waves                # print wave summary
#   scripts/plane-dag.sh show WEFT-N          # node detail + edges + wave index
#   scripts/plane-dag.sh path WEFT-N          # downstream unlock chain
#   scripts/plane-dag.sh claim WEFT-N         # transition In Progress + assignee me + comment
#   scripts/plane-dag.sh note WEFT-N "..."    # comment linking work to DAG
#   scripts/plane-dag.sh done WEFT-N --shipped S --commits C [--tests T] [--build B]
#   scripts/plane-dag.sh mermaid [--cycle C] [--priority P]
#
# Flags for ready:
#   --cycle 0.8.x|0.9.x|0.7.x|1.0.x
#   --priority high|urgent|medium|low|none
#   --lane A|B|...|J
#   --limit N
#
# Env: PLANE_API_KEY (or loaded from ~/.zshrc / ~/.claude.json via plane.sh)

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DAG_JSON="$ROOT/docs/plans/plane-dag.json"
INV_MD="$ROOT/docs/plans/plane-board-inventory.md"
WAVE_MD="$ROOT/docs/plans/plane-wave-plan.md"
RAW_JSON="$ROOT/docs/plans/plane-issues-raw.json"
PLANE_SH="$ROOT/.claude/skills/plane-workflow/scripts/plane.sh"
PLANE_PY="$ROOT/.claude/skills/plane-workflow/scripts/plane.py"
BUILDER="$ROOT/scripts/plane-dag-build.py"

# Load PLANE_API_KEY if missing
if [[ -z "${PLANE_API_KEY:-}" ]]; then
  if [[ -f "$HOME/.zshrc" ]]; then
    # shellcheck disable=SC1090
    PLANE_API_KEY="$(zsh -c 'source ~/.zshrc >/dev/null 2>&1; print -r -- ${PLANE_API_KEY:-}' 2>/dev/null || true)"
  fi
fi
if [[ -z "${PLANE_API_KEY:-}" && -x "$PLANE_SH" ]]; then
  # plane.sh will try ~/.claude.json
  :
fi
export PLANE_API_KEY="${PLANE_API_KEY:-}"

die() { echo "error: $*" >&2; exit 1; }

need_dag() {
  [[ -f "$DAG_JSON" ]] || die "missing $DAG_JSON — run: scripts/plane-dag.sh refresh"
}

cmd="${1:-}"
shift || true

case "$cmd" in
  refresh)
    [[ -n "${PLANE_API_KEY:-}" ]] || die "PLANE_API_KEY not set (export it or put it in ~/.zshrc)"
    [[ -f "$BUILDER" ]] || die "missing builder $BUILDER"
    python3 "$BUILDER" --root "$ROOT"
    echo "refreshed:"
    echo "  $DAG_JSON"
    echo "  $INV_MD"
    echo "  $WAVE_MD"
    echo "  $RAW_JSON"
    ;;
  ready|waves|show|path|mermaid|stats)
    need_dag
    python3 "$BUILDER" --root "$ROOT" --cmd "$cmd" -- "$@"
    ;;
  claim)
    need_dag
    weft="${1:-}"; shift || true
    [[ -n "$weft" ]] || die "usage: plane-dag.sh claim WEFT-N"
    [[ -n "${PLANE_API_KEY:-}" ]] || die "PLANE_API_KEY not set"
    uuid="$(python3 "$BUILDER" --root "$ROOT" --cmd resolve -- "$weft")"
    "$PLANE_SH" transition "$uuid" "In Progress" --assignee me
    note="Claimed via plane-dag. See docs/plans/plane-wave-plan.md and docs/plans/plane-board-inventory.md for graph position."
    "$PLANE_SH" comment "$uuid" --body "$note"
    echo "claimed $weft ($uuid)"
    # refresh local dag state lightly
    python3 "$BUILDER" --root "$ROOT" --cmd patch-state -- "$weft" "In Progress" || true
    ;;
  note)
    need_dag
    weft="${1:-}"; shift || true
    body="${1:-}"; shift || true
    [[ -n "$weft" && -n "$body" ]] || die "usage: plane-dag.sh note WEFT-N \"message\""
    [[ -n "${PLANE_API_KEY:-}" ]] || die "PLANE_API_KEY not set"
    uuid="$(python3 "$BUILDER" --root "$ROOT" --cmd resolve -- "$weft")"
    "$PLANE_SH" comment "$uuid" --body "$body"
    ;;
  done)
    need_dag
    weft="${1:-}"; shift || true
    [[ -n "$weft" ]] || die "usage: plane-dag.sh done WEFT-N --shipped S --commits C ..."
    [[ -n "${PLANE_API_KEY:-}" ]] || die "PLANE_API_KEY not set"
    uuid="$(python3 "$BUILDER" --root "$ROOT" --cmd resolve -- "$weft")"
    "$PLANE_SH" close "$uuid" "$@"
    echo "closed $weft — run: scripts/plane-dag.sh refresh"
    ;;
  help|-h|--help|"")
    sed -n '2,30p' "$0" | sed 's/^# \?//'
    ;;
  *)
    die "unknown command: $cmd (try help)"
    ;;
esac
