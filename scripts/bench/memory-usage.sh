#!/bin/bash
# Benchmark: Memory usage (RSS) measurement
# Measures peak RSS of the weft binary during startup
set -euo pipefail

BINARY="${1:-clawft/target/release/weft}"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

echo "=== Memory Usage Benchmark ==="
echo "Binary: $BINARY"
echo ""

# Use /usr/bin/time for RSS measurement (GNU time)
TIME_CMD="/usr/bin/time"
if [ ! -x "$TIME_CMD" ]; then
    TIME_CMD=$(which time 2>/dev/null || echo "time")
fi

# Run with --version to get startup memory
RSS_OUTPUT=$($TIME_CMD -v "$BINARY" --version 2>&1 || true)

# Extract maximum RSS (in KB)
RSS_KB=$(echo "$RSS_OUTPUT" | grep -i "maximum resident" | awk '{print $NF}')

if [ -z "$RSS_KB" ] || [ "$RSS_KB" = "0" ]; then
    # Fallback: use /proc on Linux
    "$BINARY" --version &
    PID=$!
    sleep 0.1
    if [ -f "/proc/$PID/status" ]; then
        RSS_KB=$(grep VmRSS /proc/$PID/status 2>/dev/null | awk '{print $2}')
    fi
    wait $PID 2>/dev/null || true
fi

if [ -n "$RSS_KB" ] && [ "$RSS_KB" != "0" ]; then
    RSS_MB=$(echo "scale=2; $RSS_KB / 1024" | bc)
    echo "Peak RSS: ${RSS_MB} MB (${RSS_KB} KB)"
    echo ""
    echo "{\"benchmark\": \"memory_rss\", \"peak_rss_kb\": $RSS_KB, \"peak_rss_mb\": $RSS_MB}"
else
    echo "WARNING: Could not measure RSS. Install GNU time or run on Linux."
    echo "{\"benchmark\": \"memory_rss\", \"peak_rss_kb\": null, \"note\": \"measurement failed\"}"
fi
