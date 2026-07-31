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

# Gate 3b: WEFT-398 — native wasmtime plugin host crate
echo "--- Gate 3b: clawft-wasm-host (WEFT-398) ---"
if cargo test -p clawft-wasm-host --lib 2>&1; then
    pass "clawft-wasm-host tests pass"
else
    fail "clawft-wasm-host tests failed"
fi
# Compat re-export: clawft-wasm --features wasm-plugins must still resolve.
if cargo check -p clawft-wasm --features wasm-plugins 2>&1; then
    pass "clawft-wasm wasm-plugins re-export compiles"
else
    fail "clawft-wasm wasm-plugins re-export failed"
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

# Gate 6: WEFT-397 — native ⊥ browser feature mutex
# Enabling both must fail with compile_error! in each portable crate.
# Default features include `native`, so `--features browser` enables both.
echo "--- Gate 6: native ⊥ browser mutex (WEFT-397) ---"
MUTEX_CRATES=(clawft-types clawft-platform clawft-llm clawft-core clawft-tools)
for pkg in "${MUTEX_CRATES[@]}"; do
    log="/tmp/weft397-${pkg}.log"
    set +e
    cargo check -p "$pkg" --features browser >"$log" 2>&1
    status=$?
    set -e
    if [ "$status" -eq 0 ]; then
        fail "$pkg: cargo check succeeded with native+browser — mutex missing"
    fi
    if grep -Eq 'mutually exclusive' "$log"; then
        pass "$pkg: both features produce compile_error!"
    else
        echo "---- cargo output (truncated) ----"
        tail -40 "$log" || true
        fail "$pkg: expected compile_error! naming mutual exclusion of native and browser"
    fi
done
echo ""

echo "========================================="
echo "  Feature validation complete"
echo "========================================="
