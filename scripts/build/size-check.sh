#!/bin/bash
# Check binary sizes against targets
set -euo pipefail

BINARY="${1:-target/release/weft}"
MAX_SIZE_MB="${2:-15}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found: $BINARY"
    exit 1
fi

SIZE_BYTES=$(stat -f%z "$BINARY" 2>/dev/null || stat -c%s "$BINARY" 2>/dev/null)
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)

echo "Binary: $BINARY"
echo "Size: ${SIZE_MB} MB (${SIZE_BYTES} bytes)"

if (( $(echo "$SIZE_MB > $MAX_SIZE_MB" | bc -l) )); then
    echo "FAIL: Binary exceeds ${MAX_SIZE_MB} MB limit"
    exit 1
fi

echo "PASS: Binary within ${MAX_SIZE_MB} MB limit"
