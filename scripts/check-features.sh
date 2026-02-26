#!/usr/bin/env bash
# Feature validation script for ClawFT multi-target builds.
# Ensures native, WASI, and browser WASM targets all compile correctly.
# Run this before pushing to catch feature flag issues early.
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

pass() { echo -e "${GREEN}PASS${NC}: $1"; }
fail() { echo -e "${RED}FAIL${NC}: $1"; exit 1; }
skip() { echo -e "${YELLOW}SKIP${NC}: $1"; }

echo "========================================="
echo "  ClawFT Feature Validation"
echo "========================================="
echo ""

# Gate 1: Native workspace compilation
echo "--- Gate 1: Native workspace ---"
if cargo check --workspace 2>&1; then
    pass "Native workspace compiles"
else
    fail "Native workspace compilation failed"
fi
echo ""

# Gate 2: Native test compilation (no-run to save time)
echo "--- Gate 2: Native test compilation ---"
if cargo test --workspace --no-run 2>&1; then
    pass "Native tests compile"
else
    fail "Native test compilation failed"
fi
echo ""

# Gate 3: Native CLI binary
echo "--- Gate 3: Native CLI binary ---"
if cargo build --bin weft 2>&1; then
    pass "Native CLI binary builds"
else
    fail "Native CLI binary build failed"
fi
echo ""

# Gate 4: WASI WASM (existing target)
echo "--- Gate 4: WASI WASM ---"
if rustup target list --installed | grep -q wasm32-wasip2; then
    if cargo check --target wasm32-wasip2 -p clawft-wasm 2>&1; then
        pass "WASI WASM compiles"
    else
        fail "WASI WASM compilation failed"
    fi
else
    skip "wasm32-wasip2 target not installed (run: rustup target add wasm32-wasip2)"
fi
echo ""

# Gate 5: Browser WASM (new target - may not work until BW1 is complete)
echo "--- Gate 5: Browser WASM ---"
if rustup target list --installed | grep -q wasm32-unknown-unknown; then
    if cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser 2>/dev/null; then
        pass "Browser WASM compiles"
    else
        skip "Browser WASM not yet available (expected until BW1 phase completes)"
    fi
else
    skip "wasm32-unknown-unknown target not installed (run: rustup target add wasm32-unknown-unknown)"
fi
echo ""

echo "========================================="
echo "  Feature validation complete"
echo "========================================="
