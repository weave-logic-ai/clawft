#!/usr/bin/env bash
# Flywheel status (ADR-096 / WEFT-730) — real measure via @metaharness/flywheel adapter.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
exec node "$ROOT/scripts/metaharness/flywheel-measure.mjs" status
