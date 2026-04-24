#!/bin/bash
# Docker E2E Validation for agentic-flow v1.5.10
# Tests ReasoningBank WASM and QUIC in clean container environment

set -e

echo "🐳 agentic-flow v1.5.10 - Docker E2E Validation"
echo "================================================"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test results
PASSED=0
FAILED=0
TOTAL=0

# Function to run test
run_test() {
    local test_name="$1"
    local test_cmd="$2"

    TOTAL=$((TOTAL + 1))
    echo -n "Testing: $test_name... "

    if eval "$test_cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}✓ PASS${NC}"
        PASSED=$((PASSED + 1))
        return 0
    else
        echo -e "${RED}✗ FAIL${NC}"
        FAILED=$((FAILED + 1))
        return 1
    fi
}

# Test 1: Package Installation
echo "📦 Step 1: Package Installation Test"
echo "======================================"
run_test "npm install" "npm install"
run_test "Build project" "npm run build"
echo ""

# Test 2: ReasoningBank WASM Tests
echo "🧠 Step 2: ReasoningBank WASM Tests"
echo "======================================"
run_test "WASM E2E suite" "npm run test:wasm:e2e"
run_test "WASM files exist" "test -f wasm/reasoningbank/reasoningbank_wasm_bg.wasm"
run_test "WASM size correct" "test $(stat -f%z wasm/reasoningbank/reasoningbank_wasm_bg.wasm 2>/dev/null || stat -c%s wasm/reasoningbank/reasoningbank_wasm_bg.wasm) -gt 200000"
echo ""

# Test 3: Performance Benchmarks
echo "⚡ Step 3: Performance Benchmarks"
echo "=================================="
echo "Running performance tests..."

# Create test file
cat > /tmp/perf-test.mjs << 'EOF'
import { createReasoningBank } from './dist/reasoningbank/wasm-adapter.js';

async function testPerformance() {
    const rb = await createReasoningBank('perf-test');
    const iterations = 10;
    const start = Date.now();

    for (let i = 0; i < iterations; i++) {
        await rb.storePattern({
            task_description: `Performance test ${i}`,
            task_category: 'benchmark',
            strategy: 'speed-test',
            success_score: 0.85
        });
    }

    const duration = Date.now() - start;
    const avgTime = duration / iterations;

    console.log(JSON.stringify({
        iterations,
        totalDuration: duration,
        avgTime,
        opsPerSec: Math.round(1000 / avgTime)
    }));

    // Verify performance targets
    if (avgTime > 100) {
        process.exit(1); // Fail if too slow
    }
}

testPerformance().catch(err => {
    console.error('Performance test failed:', err);
    process.exit(1);
});
EOF

if node /tmp/perf-test.mjs 2>&1 | grep -q '"avgTime"'; then
    PERF_RESULTS=$(node /tmp/perf-test.mjs 2>&1)
    AVG_TIME=$(echo "$PERF_RESULTS" | grep -o '"avgTime":[0-9.]*' | cut -d: -f2)

    if (( $(echo "$AVG_TIME < 100" | bc -l) )); then
        echo -e "  Average time per operation: ${GREEN}${AVG_TIME}ms${NC} ✓"
        PASSED=$((PASSED + 1))
    else
        echo -e "  Average time per operation: ${RED}${AVG_TIME}ms${NC} ✗ (target: <100ms)"
        FAILED=$((FAILED + 1))
    fi
    TOTAL=$((TOTAL + 1))
else
    echo -e "  ${RED}✗ Performance test failed${NC}"
    FAILED=$((FAILED + 1))
    TOTAL=$((TOTAL + 1))
fi

rm -f /tmp/perf-test.mjs
echo ""

# Test 4: Memory Leak Detection
echo "💾 Step 4: Memory Leak Detection"
echo "=================================="
cat > /tmp/memory-test.mjs << 'EOF'
import { createReasoningBank } from './dist/reasoningbank/wasm-adapter.js';

async function testMemory() {
    const initialMem = process.memoryUsage().heapUsed;
    const rb = await createReasoningBank('memory-test');

    for (let i = 0; i < 100; i++) {
        await rb.storePattern({
            task_description: `Memory test ${i}`,
            task_category: 'memory',
            strategy: 'stability',
            success_score: 0.9
        });
    }

    if (global.gc) global.gc();
    const finalMem = process.memoryUsage().heapUsed;
    const memDelta = (finalMem - initialMem) / 1024 / 1024;

    console.log(JSON.stringify({
        initialMB: Math.round(initialMem / 1024 / 1024),
        finalMB: Math.round(finalMem / 1024 / 1024),
        deltaMB: Math.round(memDelta * 100) / 100
    }));

    // Fail if memory leak > 50MB
    if (memDelta > 50) {
        process.exit(1);
    }
}

testMemory().catch(err => {
    console.error('Memory test failed:', err);
    process.exit(1);
});
EOF

if node --expose-gc /tmp/memory-test.mjs 2>&1 | grep -q '"deltaMB"'; then
    MEM_RESULTS=$(node --expose-gc /tmp/memory-test.mjs 2>&1)
    MEM_DELTA=$(echo "$MEM_RESULTS" | grep -o '"deltaMB":[0-9.-]*' | cut -d: -f2)

    if (( $(echo "$MEM_DELTA < 50" | bc -l) )); then
        echo -e "  Memory delta after 100 ops: ${GREEN}${MEM_DELTA}MB${NC} ✓"
        PASSED=$((PASSED + 1))
    else
        echo -e "  Memory delta: ${RED}${MEM_DELTA}MB${NC} ✗ (threshold: <50MB)"
        FAILED=$((FAILED + 1))
    fi
    TOTAL=$((TOTAL + 1))
else
    echo -e "  ${RED}✗ Memory test failed${NC}"
    FAILED=$((FAILED + 1))
    TOTAL=$((TOTAL + 1))
fi

rm -f /tmp/memory-test.mjs
echo ""

# Test 5: QUIC Implementation Status
echo "🚀 Step 5: QUIC Implementation Check"
echo "======================================"
run_test "QUIC transport compiled" "test -f dist/transport/quic.js"
run_test "QUIC config compiled" "test -f dist/config/quic.js"
run_test "QUIC proxy compiled" "test -f dist/proxy/quic-proxy.js"
echo ""

# Summary
echo "================================================"
echo "📊 Test Summary"
echo "================================================"
echo ""
echo "Total Tests: $TOTAL"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo ""

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL TESTS PASSED!${NC}"
    echo ""
    echo "agentic-flow v1.5.10 is READY FOR PUBLICATION"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo ""
    echo "Please fix failing tests before publishing"
    exit 1
fi
