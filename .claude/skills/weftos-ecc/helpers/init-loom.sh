#!/usr/bin/env bash
# init-loom.sh -- Initialize a new ECC Loom from a weave.toml configuration.
#
# Usage: init-loom.sh --config <weave.toml> [--data-dir <path>]
#
# Creates the directory structure and initial state files for a Loom.

set -euo pipefail

CONFIG=""
DATA_DIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --config) CONFIG="$2"; shift 2 ;;
        --data-dir) DATA_DIR="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$CONFIG" ]]; then
    echo "Usage: init-loom.sh --config <weave.toml> [--data-dir <path>]" >&2
    exit 1
fi

if [[ ! -f "$CONFIG" ]]; then
    echo "Error: Config file not found: $CONFIG" >&2
    exit 1
fi

DOMAIN=$(grep -E '^name\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*"\(.*\)"/\1/')
if [[ -z "$DOMAIN" ]]; then
    echo "Error: Could not parse domain name from $CONFIG" >&2
    exit 1
fi

if [[ -z "$DATA_DIR" ]]; then
    DATA_DIR=".weaver/looms/$DOMAIN"
fi

echo "Initializing loom: $DOMAIN"
echo "  Config: $CONFIG"
echo "  Data dir: $DATA_DIR"

mkdir -p "$DATA_DIR"/{causal,hnsw,crossrefs,chain,tree,snapshots,archives}
cp "$CONFIG" "$DATA_DIR/weave.toml"

DIMENSIONS=$(grep -E '^dimensions\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*//' || echo "384")
EF_SEARCH=$(grep -E '^ef_search\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*//' || echo "100")
EF_CONSTRUCTION=$(grep -E '^ef_construction\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*//' || echo "200")
TICK_INTERVAL=$(grep -E '^interval_ms\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*//' || echo "50")
BUDGET_RATIO=$(grep -E '^budget_ratio\s*=' "$CONFIG" | head -1 | sed 's/.*=\s*//' || echo "0.3")

TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
cat > "$DATA_DIR/init-manifest.json" <<ENDJSON
{
  "domain": "$DOMAIN",
  "initialized_at": "$TIMESTAMP",
  "config_path": "$(realpath "$CONFIG")",
  "data_dir": "$(realpath "$DATA_DIR")",
  "parameters": {
    "hnsw_dimensions": $DIMENSIONS,
    "hnsw_ef_search": $EF_SEARCH,
    "hnsw_ef_construction": $EF_CONSTRUCTION,
    "tick_interval_ms": $TICK_INTERVAL,
    "tick_budget_ratio": $BUDGET_RATIO
  },
  "structures": {
    "causal_graph": "$DATA_DIR/causal/",
    "hnsw_index": "$DATA_DIR/hnsw/",
    "crossref_store": "$DATA_DIR/crossrefs/",
    "exo_chain": "$DATA_DIR/chain/",
    "resource_tree": "$DATA_DIR/tree/",
    "snapshots": "$DATA_DIR/snapshots/",
    "archives": "$DATA_DIR/archives/"
  },
  "status": "initialized",
  "needs_calibration": true,
  "needs_ingestion": true
}
ENDJSON

echo ""
echo "Loom initialized: $DOMAIN"
echo "  Data directory: $DATA_DIR"
echo "  Manifest: $DATA_DIR/init-manifest.json"
echo ""
echo "Next steps:"
echo "  1. Run calibration:  weaver ecc calibrate --loom $DOMAIN"
echo "  2. Weave sources:    weaver ecc weave all --config $DATA_DIR/weave.toml"
echo "  3. Check health:     weaver ecc analyze --loom $DOMAIN --health"
