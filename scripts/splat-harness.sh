#!/usr/bin/env bash
# splat-harness — submit a video to clawft-splatd, wait for a splat, open the viewer.
#
# Usage:
#   scripts/splat-harness.sh /path/to/walkaround.mov
#   scripts/splat-harness.sh --view-only path/to/splat.sog
#   scripts/splat-harness.sh --job <id>          # open viewer for existing job
#   scripts/splat-harness.sh --serve-viewer      # static viewer only (no train)
#
# Env:
#   SPLATD_ADDR / SPLATD_URL   default http://127.0.0.1:7860
#   SPLATD_CONFIG              default config/splatd.toml
#   SPLAT_DATA_DIR             overrides data_dir for this run (absolute)
#   SPLAT_HARNESS_NO_OPEN      if set, print viewer URL only
#
# Requires: curl, cargo (to build/start splatd if not already up).
# Pipeline tools (ffmpeg, colmap, brush-cli, splat-transform) must be on PATH
# for a full train; missing tools fail at the relevant stage with logs under
# data/jobs/<id>/logs/.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SPLATD_URL="${SPLATD_URL:-http://${SPLATD_ADDR:-127.0.0.1:7860}}"
SPLATD_URL="${SPLATD_URL%/}"
CONFIG="${SPLATD_CONFIG:-$ROOT/config/splatd.toml}"
VIEWER_DIR="$ROOT/examples/splat-viewer"
VIEWER_PORT="${SPLAT_VIEWER_PORT:-8765}"

die() { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }

usage() {
  sed -n '2,20p' "$0" | sed 's/^# \?//'
  exit 0
}

MODE="train"
VIDEO=""
JOB_ID=""
VIEW_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -h|--help) usage ;;
    --view-only) MODE="view-only"; VIEW_FILE="${2:-}"; shift 2 || shift ;;
    --job) MODE="job"; JOB_ID="${2:-}"; shift 2 || shift ;;
    --serve-viewer) MODE="serve"; shift ;;
    --) shift; break ;;
    -*) die "unknown flag: $1" ;;
    *) VIDEO="$1"; shift; break ;;
  esac
done

# leftover positional
if [[ -z "${VIDEO:-}" && $# -gt 0 ]]; then VIDEO="$1"; fi

splatd_up() {
  curl -sf "$SPLATD_URL/healthz" >/dev/null 2>&1
}

start_splatd_if_needed() {
  if splatd_up; then
    info "splatd already up at $SPLATD_URL"
    return
  fi
  info "building splatd (release)…"
  cargo build -p clawft-splatd --release
  mkdir -p "$ROOT/data/splat/jobs"
  local data_dir="${SPLAT_DATA_DIR:-$ROOT/data/splat}"
  # Absolute data dir so job cwd cannot poison relative paths.
  case "$data_dir" in
    /*) ;;
    *) data_dir="$ROOT/$data_dir" ;;
  esac
  info "starting splatd → $SPLATD_URL (data_dir=$data_dir)"
  # Inject data_dir via temp config if needed
  local cfg="$CONFIG"
  if [[ -f "$CONFIG" ]]; then
    local tmp
    tmp="$(mktemp)"
    # shellcheck disable=SC2016
    awk -v d="$data_dir" '
      BEGIN { done=0 }
      /^data_dir/ { print "data_dir = \"" d "\""; done=1; next }
      { print }
      END { if (!done) print "data_dir = \"" d "\"" }
    ' "$CONFIG" >"$tmp"
    cfg="$tmp"
  fi
  SPLATD_CONFIG="$cfg" \
    RUST_LOG="${RUST_LOG:-info}" \
    nohup "$ROOT/target/release/splatd" >"$ROOT/data/splat/splatd.log" 2>&1 &
  echo $! >"$ROOT/data/splat/splatd.pid"
  for _ in $(seq 1 50); do
    if splatd_up; then
      info "splatd ready (pid $(cat "$ROOT/data/splat/splatd.pid"))"
      return
    fi
    sleep 0.1
  done
  die "splatd did not become healthy — see data/splat/splatd.log"
}

serve_viewer() {
  info "viewer at http://127.0.0.1:${VIEWER_PORT}/ (ctrl-c to stop)"
  if command -v python3 >/dev/null; then
    (cd "$VIEWER_DIR" && python3 -m http.server "$VIEWER_PORT")
  else
    die "python3 required to serve viewer"
  fi
}

open_viewer_url() {
  # $1 is either a full http URL or a query string starting with "?"
  local q="$1"
  mkdir -p "$ROOT/data/splat"
  if [[ -n "${SPLAT_HARNESS_NO_OPEN:-}" ]]; then
    if [[ "$q" == http* ]]; then
      info "viewer URL: $q"
    else
      info "viewer URL: http://127.0.0.1:${VIEWER_PORT}/index.html${q}"
    fi
    return
  fi
  if ! curl -sf "http://127.0.0.1:${VIEWER_PORT}/" >/dev/null 2>&1; then
    (cd "$VIEWER_DIR" && python3 -m http.server "$VIEWER_PORT") >/dev/null 2>&1 &
    echo $! >"$ROOT/data/splat/viewer.pid"
    sleep 0.3
  fi
  local full
  if [[ "$q" == http* ]]; then
    full="$q"
  else
    full="http://127.0.0.1:${VIEWER_PORT}/index.html${q}"
  fi
  info "opening $full"
  if command -v open >/dev/null; then
    open "$full" || true
  elif command -v xdg-open >/dev/null; then
    xdg-open "$full" || true
  fi
}

case "$MODE" in
  serve)
    serve_viewer
    ;;
  view-only)
    [[ -n "$VIEW_FILE" && -f "$VIEW_FILE" ]] || die "--view-only requires a splat file"
    # Serve file via file URL through python is awkward; copy into viewer tmp
    mkdir -p "$VIEWER_DIR/.harness"
    local_name="$(basename "$VIEW_FILE")"
    cp -f "$VIEW_FILE" "$VIEWER_DIR/.harness/$local_name"
    open_viewer_url "?url=.harness/${local_name}"
    info "press enter to stop viewer server (if we started one)…"
    if [[ -f "$ROOT/data/splat/viewer.pid" ]]; then
      read -r _
      kill "$(cat "$ROOT/data/splat/viewer.pid")" 2>/dev/null || true
    fi
    ;;
  job)
    [[ -n "$JOB_ID" ]] || die "--job requires an id"
    # Prefer same-origin copy under the viewer (avoids CORS / large cross-origin
    # fetch flakiness). Fall back to ?job=&splatd= if artifact not on disk yet.
    art_dir="$ROOT/data/splat/jobs/${JOB_ID}/artifacts"
    sog="$art_dir/splat.sog"
    ply="$art_dir/splat.ply"
    if [[ -f "$sog" ]]; then
      mkdir -p "$VIEWER_DIR/.harness"
      cp -f "$sog" "$VIEWER_DIR/.harness/${JOB_ID}.sog"
      info "serving local SOG same-origin (job $JOB_ID)"
      open_viewer_url "?url=.harness/${JOB_ID}.sog"
    elif [[ -f "$ply" ]]; then
      mkdir -p "$VIEWER_DIR/.harness"
      cp -f "$ply" "$VIEWER_DIR/.harness/${JOB_ID}.ply"
      info "serving local PLY same-origin (job $JOB_ID) — large file, SOG preferred"
      open_viewer_url "?url=.harness/${JOB_ID}.ply"
    else
      start_splatd_if_needed
      info "no local artifact; viewer will fetch via splatd (needs CORS)"
      open_viewer_url "?job=${JOB_ID}&splatd=${SPLATD_URL}"
    fi
    ;;
  train)
    [[ -n "${VIDEO:-}" && -f "$VIDEO" ]] || die "usage: $0 /path/to/video.mov"
    start_splatd_if_needed
    info "submitting $VIDEO"
    RESP=$(curl -sf -X POST "$SPLATD_URL/v1/jobs" -F "video=@${VIDEO}")
    JOB_ID=$(echo "$RESP" | python3 -c 'import sys,json; print(json.load(sys.stdin)["job_id"])')
    info "job_id=$JOB_ID"
    info "polling (logs: data/splat/jobs/$JOB_ID/logs/)…"
    while true; do
      REC=$(curl -sf "$SPLATD_URL/v1/jobs/$JOB_ID")
      STATUS=$(echo "$REC" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("status",""))')
      STAGE=$(echo "$REC" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("stage") or "")')
      ERR=$(echo "$REC" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("error") or "")')
      printf '\r    status=%-10s stage=%-10s' "$STATUS" "$STAGE"
      if [[ "$STATUS" == "done" ]]; then
        echo
        info "done — artifacts: $(echo "$REC" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("artifacts",{}))')"
        open_viewer_url "?job=${JOB_ID}&splatd=${SPLATD_URL}"
        exit 0
      fi
      if [[ "$STATUS" == "failed" ]]; then
        echo
        die "job failed at stage=$STAGE: $ERR"
      fi
      sleep 2
    done
    ;;
esac
