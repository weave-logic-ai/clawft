#!/usr/bin/env bash
# DEPRECATED (WEFT-409 / WEFT-564). Use: scripts/build.sh gate
#
# History: added 2026-02-25 (6a1416c6) as a 5-gate feature-flag check.
# Superseded by scripts/build.sh gate (full phase gate: native + WASI +
# browser + clippy + bundle-size + audit + more). Kept as a thin shim so
# muscle-memory and stale docs still land on the canonical entrypoint.
#
# This standalone body was retired in WEFT-564; do not re-expand it.
set -euo pipefail

echo "DEPRECATED (WEFT-409/WEFT-564): scripts/check-features.sh is retired." >&2
echo "Redirecting to: scripts/build.sh gate" >&2
echo "Update callers to invoke scripts/build.sh gate directly." >&2

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec "$SCRIPT_DIR/build.sh" gate "$@"
