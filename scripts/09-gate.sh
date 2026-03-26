#!/usr/bin/env bash
set -euo pipefail

# Sprint 09 WeftOS Gaps -- Gate Script
# Validates all Sprint 09 exit criteria before merge.
# Run from the repository root: scripts/09-gate.sh

cd "$(git rev-parse --show-toplevel)"

echo "=== 09 WeftOS Gaps Sprint Gate ==="
echo ""

PASS=0
FAIL=0
FAILURES=()

run_gate() {
    local name="$1"; local cmd="$2"
    printf "  %-60s " "$name"
    if eval "$cmd" > /dev/null 2>&1; then
        echo "PASS"; PASS=$((PASS + 1))
    else
        echo "FAIL"; FAIL=$((FAIL + 1))
        FAILURES+=("$name")
    fi
}

# ---------------------------------------------------------------------------
echo "--- 09a: Test Coverage ---"
# ---------------------------------------------------------------------------

run_gate "boot.rs has 5+ tests" \
    "test \$(grep -cE '#\[test\]|#\[tokio::test\]' crates/clawft-kernel/src/boot.rs) -ge 5"

run_gate "agent_loop.rs has 10+ tests" \
    "test \$(grep -c -E '#\[test\]|#\[tokio::test\]' crates/clawft-kernel/src/agent_loop.rs) -ge 10"

run_gate "a2a.rs has 1+ tests" \
    "test \$(grep -c '#\[test\]' crates/clawft-kernel/src/a2a.rs) -ge 1"

run_gate "governance.rs has 15+ tests" \
    "test \$(grep -c '#\[test\]' crates/clawft-kernel/src/governance.rs) -ge 15"

run_gate "chain.rs has 15+ tests" \
    "test \$(grep -c '#\[test\]' crates/clawft-kernel/src/chain.rs) -ge 15"

run_gate "wasm_runner.rs has 20+ tests" \
    "test \$(grep -c '#\[test\]' crates/clawft-kernel/src/wasm_runner.rs) -ge 20"

run_gate "supervisor.rs has 10+ tests" \
    "test \$(grep -c '#\[test\]' crates/clawft-kernel/src/supervisor.rs) -ge 10"

run_gate "Feature composition tests pass" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh,ecc' --test feature_composition"

run_gate "Full feature build compiles" \
    "cargo check -p clawft-kernel --features 'native,exochain,cluster,mesh,ecc,os-patterns,wasm-sandbox'"

echo ""
# ---------------------------------------------------------------------------
echo "--- 09b: Decision Resolution ---"
# ---------------------------------------------------------------------------

run_gate "09b decision resolution plan exists" \
    "test -f .planning/sparc/weftos/0.1/09b-decision-resolution.md"

run_gate "Per-phase decisions exist (K0-K6)" \
    "test -f .planning/development_notes/weftos/phase-K0/decisions.md && \
     test -f .planning/development_notes/weftos/phase-K3/decisions.md && \
     test -f .planning/development_notes/weftos/phase-K6/decisions.md"

echo ""
# ---------------------------------------------------------------------------
echo "--- 09c: Weaver Runtime ---"
# ---------------------------------------------------------------------------

run_gate "Weaver module exists" \
    "test -f crates/clawft-kernel/src/weaver.rs"

run_gate "Embedding provider exists" \
    "grep -q 'EmbeddingProvider' crates/clawft-kernel/src/weaver.rs 2>/dev/null || \
     grep -rq 'EmbeddingProvider' crates/clawft-kernel/src/ 2>/dev/null"

echo ""
# ---------------------------------------------------------------------------
echo "--- 09d: Integration & Feature Composition ---"
# ---------------------------------------------------------------------------

run_gate "mesh + os-patterns compiles" \
    "cargo check -p clawft-kernel --features 'native,mesh,os-patterns'"

run_gate "exochain + mesh + os-patterns compiles" \
    "cargo check -p clawft-kernel --features 'native,exochain,mesh,os-patterns'"

run_gate "All features (no wasm-sandbox) compile" \
    "cargo check -p clawft-kernel --features 'native,exochain,cluster,mesh,ecc,os-patterns'"

run_gate "All features (incl wasm-sandbox) compile" \
    "cargo check -p clawft-kernel --features 'native,exochain,cluster,mesh,ecc,os-patterns,wasm-sandbox'"

run_gate "1200+ kernel tests pass" \
    "test \$(cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh,ecc,os-patterns' 2>&1 \
        | grep 'test result' | head -1 | grep -oP '[0-9]+ passed' | grep -oP '[0-9]+') -ge 1200"

run_gate "Kernel guide documentation exists" \
    "test -f docs/src/content/docs/weftos/kernel-guide.mdx"

run_gate "09-gate.sh is executable" \
    "test -x scripts/09-gate.sh"

echo ""
echo "================================"
echo "  PASS: $PASS / $((PASS + FAIL))"
echo "  FAIL: $FAIL / $((PASS + FAIL))"
echo "================================"

if [ ${#FAILURES[@]} -gt 0 ]; then
    echo ""
    echo "Failed checks:"
    for f in "${FAILURES[@]}"; do
        echo "  - $f"
    done
    echo ""
    exit 1
fi

echo "All gates passed!"
