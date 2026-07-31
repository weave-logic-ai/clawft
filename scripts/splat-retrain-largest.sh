#!/usr/bin/env bash
# Re-train an existing splat job using only the largest COLMAP model.
# Use when the splat looks "exploded" because Brush saw sparse/0 + sparse/1 + …
#
# Usage:
#   scripts/splat-retrain-largest.sh <job_id>
#   scripts/splat-retrain-largest.sh 4030edd8-3a70-434c-99c3-cb0c8f0194fa
#
# Requires: brush-cli, splat-transform (optional), colmap (for model pick)

set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JOB_ID="${1:-}"
[[ -n "$JOB_ID" ]] || { echo "usage: $0 <job_id>" >&2; exit 1; }

J="$ROOT/data/splat/jobs/$JOB_ID"
[[ -d "$J/dataset/images" ]] || { echo "error: no dataset at $J" >&2; exit 1; }
SPARSE="$J/dataset/sparse"
ART="$J/artifacts"
mkdir -p "$ART"

echo "==> picking largest COLMAP model under $SPARSE"
best="" best_n=-1
for d in "$SPARSE"/*/; do
  [[ -d "$d" ]] || continue
  base=$(basename "$d")
  [[ "$base" =~ ^[0-9]+$ ]] || continue
  n=0
  if command -v colmap >/dev/null; then
    n=$(colmap model_analyzer --path "$d" 2>&1 | sed -n 's/.*Registered images: *//p' | head -1 | tr -d '[:space:]')
    n=${n:-0}
  fi
  if [[ -z "$n" || "$n" -eq 0 ]]; then
    # fallback: points3D.bin size
    n=$(stat -f%z "$d/points3D.bin" 2>/dev/null || stat -c%s "$d/points3D.bin" 2>/dev/null || echo 0)
  fi
  echo "    model $base: score=$n"
  if [[ "$n" -gt "$best_n" ]]; then best_n=$n; best=$base; fi
done
[[ -n "$best" ]] || { echo "error: no numeric sparse models" >&2; exit 1; }
echo "==> keeping sparse/$best (score=$best_n)"

TMP="$SPARSE/_keep"
rm -rf "$TMP"
cp -a "$SPARSE/$best" "$TMP"
# wipe other models
find "$SPARSE" -mindepth 1 -maxdepth 1 ! -name '_keep' -exec rm -rf {} +
mv "$TMP" "$SPARSE/0"
echo "==> sparse/ now:"
ls -la "$SPARSE"

ITERS="${SPLAT_ITERS:-30000}"
MAX_SPLATS="${SPLAT_MAX_SPLATS:-2000000}"
echo "==> brush train (iters=$ITERS max_splats=$MAX_SPLATS)…"
(
  cd "$J"
  brush-cli dataset \
    --total-steps "$ITERS" \
    --max-resolution 1600 \
    --max-splats "$MAX_SPLATS" \
    --sh-degree 3 \
    --export-every "$ITERS" \
    --export-path artifacts \
    --export-name splat.ply \
    --eval-every "$ITERS"
)

if command -v splat-transform >/dev/null; then
  echo "==> compress → splat.sog"
  splat-transform "$ART/splat.ply" "$ART/splat.sog"
else
  echo "==> skip SOG (splat-transform not on PATH); view PLY or install @playcanvas/splat-transform"
fi

echo "==> open viewer"
exec "$ROOT/scripts/splat-harness.sh" --view-only "${ART}/splat.sog"
