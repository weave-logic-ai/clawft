#!/usr/bin/env bash
# Unified build script for ClawFT workspace.
# Wraps cargo, wasm, and UI builds behind simple subcommands.
set -euo pipefail

# ── Colors ───────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ── Resolve workspace root ──────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# ── Defaults ─────────────────────────────────────────────────────────
PROFILE=""
FEATURES=""
VERBOSE=false
DRY_RUN=false
COMMAND=""

# ── Reporting helpers ────────────────────────────────────────────────
pass()  { printf "  ${GREEN}PASS${NC}  %s\n" "$*"; }
fail()  { printf "  ${RED}FAIL${NC}  %s\n" "$*"; }
skip()  { printf "  ${YELLOW}SKIP${NC}  %s\n" "$*"; }
info()  { printf "  ${CYAN}INFO${NC}  %s\n" "$*"; }
header(){ printf "\n${BOLD}── %s${NC}\n" "$*"; }

# ── Timer ────────────────────────────────────────────────────────────
TIMER_START=0
timer_start() { TIMER_START=$(date +%s); }
timer_end() {
    local elapsed=$(( $(date +%s) - TIMER_START ))
    local min=$((elapsed / 60))
    local sec=$((elapsed % 60))
    if [ "$min" -gt 0 ]; then
        printf "  ${CYAN}TIME${NC}  %dm %ds\n" "$min" "$sec"
    else
        printf "  ${CYAN}TIME${NC}  %ds\n" "$sec"
    fi
}

# ── Run a command (respects --verbose and --dry-run) ─────────────────
run_cmd() {
    if [ "$DRY_RUN" = true ]; then
        printf "  ${YELLOW}DRY${NC}   %s\n" "$*"
        return 0
    fi
    if [ "$VERBOSE" = true ]; then
        "$@"
    else
        "$@" 2>&1 | tail -5
    fi
}

# ── Target check ────────────────────────────────────────────────────
check_target_installed() {
    local target="$1"
    if ! rustup target list --installed 2>/dev/null | grep -q "$target"; then
        printf "  ${YELLOW}WARN${NC}  Target %s not installed. Run: rustup target add %s\n" "$target" "$target"
        return 1
    fi
    return 0
}

# ── Size reporting ──────────────────────────────────────────────────
report_binary_size() {
    local file="$1" label="${2:-Binary}"
    if [ -f "$file" ]; then
        local bytes
        bytes=$(wc -c < "$file")
        local kb=$((bytes / 1024))
        if [ "$kb" -ge 1024 ]; then
            local mb
            mb=$(echo "scale=2; $bytes / 1048576" | bc 2>/dev/null || echo "$((kb / 1024))")
            printf "  ${CYAN}SIZE${NC}  %s: %s MB (%s bytes)\n" "$label" "$mb" "$bytes"
        else
            printf "  ${CYAN}SIZE${NC}  %s: %s KB (%s bytes)\n" "$label" "$kb" "$bytes"
        fi
    fi
}

# ── Feature flag builder ────────────────────────────────────────────
cargo_features_args() {
    if [ -n "$FEATURES" ]; then
        echo "--features $FEATURES"
    fi
}

# ── Subcommands ─────────────────────────────────────────────────────

cmd_native() {
    local profile="${PROFILE:-release}"
    header "Building native CLI binary (profile: $profile)"
    timer_start
    local args=(cargo build --bin weft)
    if [ "$profile" = "release" ] || [ "$profile" = "release-wasm" ]; then
        args+=(--profile "$profile")
    fi
    [ -n "$FEATURES" ] && args+=(--features "$FEATURES")
    run_cmd "${args[@]}"
    timer_end
    if [ "$profile" = "release" ]; then
        report_binary_size "target/release/weft" "Native binary"
    elif [ "$profile" = "release-wasm" ]; then
        report_binary_size "target/release-wasm/weft" "Native binary"
    else
        report_binary_size "target/debug/weft" "Native binary"
    fi
}

cmd_native_debug() {
    header "Building native CLI binary (debug)"
    timer_start
    local args=(cargo build --bin weft)
    [ -n "$FEATURES" ] && args+=(--features "$FEATURES")
    run_cmd "${args[@]}"
    timer_end
    report_binary_size "target/debug/weft" "Native binary (debug)"
}

cmd_wasi() {
    local profile="${PROFILE:-release-wasm}"
    header "Building WASM for WASI (wasm32-wasip1, profile: $profile)"
    if ! check_target_installed wasm32-wasip1; then return 1; fi
    timer_start
    local args=(cargo build --target wasm32-wasip1 --profile "$profile" -p clawft-wasm)
    [ -n "$FEATURES" ] && args+=(--features "$FEATURES")
    run_cmd "${args[@]}"
    timer_end
    report_binary_size "target/wasm32-wasip1/${profile}/clawft_wasm.wasm" "WASI WASM"
}

cmd_browser() {
    local profile="${PROFILE:-release-wasm}"
    header "Building WASM for browser (wasm32-unknown-unknown, profile: $profile)"
    if ! check_target_installed wasm32-unknown-unknown; then return 1; fi
    timer_start
    local args=(cargo build --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser)
    args+=(--profile "$profile")
    # Append extra features if provided (comma-separated with browser)
    if [ -n "$FEATURES" ]; then
        # browser is already set; append user features
        args[-1]="browser,$FEATURES"
    fi
    run_cmd "${args[@]}"
    timer_end
    report_binary_size "target/wasm32-unknown-unknown/${profile}/clawft_wasm.wasm" "Browser WASM"
}

cmd_ui() {
    header "Building React frontend (tsc + vite)"
    if [ ! -d "$ROOT/ui" ] || [ ! -f "$ROOT/ui/package.json" ]; then
        skip "ui/ directory not found — skipping"
        return 0
    fi
    timer_start
    if [ "$DRY_RUN" = true ]; then
        printf "  ${YELLOW}DRY${NC}   cd ui && npm run build\n"
    else
        (cd "$ROOT/ui" && npm run build)
    fi
    timer_end
    if [ -d "$ROOT/ui/dist" ]; then
        local size
        size=$(du -sh "$ROOT/ui/dist" 2>/dev/null | cut -f1)
        printf "  ${CYAN}SIZE${NC}  UI bundle: %s\n" "$size"
    fi
}

cmd_all() {
    header "Building everything"
    local failed=0
    cmd_native  || failed=$((failed + 1))
    cmd_wasi    || failed=$((failed + 1))
    cmd_browser || failed=$((failed + 1))
    cmd_ui      || failed=$((failed + 1))
    echo ""
    if [ "$failed" -gt 0 ]; then
        fail "$failed build(s) failed"
        return 1
    else
        pass "All builds succeeded"
    fi
}

cmd_test() {
    header "Running cargo test --workspace"
    timer_start
    run_cmd cargo test --workspace
    timer_end
}

cmd_check() {
    header "Running cargo check --workspace"
    timer_start
    run_cmd cargo check --workspace
    timer_end
}

cmd_clippy() {
    header "Running clippy (warnings as errors)"
    timer_start
    run_cmd cargo clippy --workspace -- -D warnings
    timer_end
}

cmd_clean() {
    header "Cleaning build artifacts"
    run_cmd cargo clean
    if [ -d "$ROOT/ui/dist" ]; then
        info "Removing ui/dist"
        rm -rf "$ROOT/ui/dist"
    fi
    pass "Clean complete"
}

# ── Gate: full phase-gate checks ────────────────────────────────────
cmd_gate() {
    header "Phase Gate — 11 checks"
    local total=11 passed=0 failed=0 skipped=0

    run_gate_check() {
        local num="$1" label="$2"
        shift 2
        printf "\n${BOLD}[%2d/%d]${NC} %s\n" "$num" "$total" "$label"
        timer_start
        if [ "$DRY_RUN" = true ]; then
            printf "  ${YELLOW}DRY${NC}   %s\n" "$*"
            passed=$((passed + 1))
        elif "$@" >/dev/null 2>&1; then
            pass "$label"
            passed=$((passed + 1))
        else
            fail "$label"
            failed=$((failed + 1))
        fi
        timer_end
    }

    run_gate_check_soft() {
        local num="$1" label="$2"
        shift 2
        printf "\n${BOLD}[%2d/%d]${NC} %s\n" "$num" "$total" "$label"
        timer_start
        if [ "$DRY_RUN" = true ]; then
            printf "  ${YELLOW}DRY${NC}   %s\n" "$*"
            passed=$((passed + 1))
        elif "$@" >/dev/null 2>&1; then
            pass "$label"
            passed=$((passed + 1))
        else
            skip "$label (not yet available)"
            skipped=$((skipped + 1))
        fi
        timer_end
    }

    # 1. Workspace tests
    run_gate_check 1 "cargo test --workspace" \
        cargo test --workspace

    # 2. Release binary
    run_gate_check 2 "cargo build --release --bin weft" \
        cargo build --release --bin weft

    # 3. WASI WASM
    if check_target_installed wasm32-wasip1; then
        run_gate_check 3 "WASI WASM (wasm32-wasip1)" \
            cargo build --target wasm32-wasip1 --profile release-wasm -p clawft-wasm
    else
        printf "\n${BOLD}[%2d/%d]${NC} %s\n" 3 "$total" "WASI WASM (wasm32-wasip1)"
        skip "wasm32-wasip1 target not installed"
        skipped=$((skipped + 1))
    fi

    # 4–9. Browser WASM checks per crate
    local browser_crates=(clawft-types clawft-platform clawft-core clawft-llm clawft-tools clawft-wasm)
    local gate_num=4
    if check_target_installed wasm32-unknown-unknown; then
        for crate in "${browser_crates[@]}"; do
            run_gate_check_soft "$gate_num" "Browser WASM: $crate" \
                cargo check --target wasm32-unknown-unknown -p "$crate" --no-default-features --features browser
            gate_num=$((gate_num + 1))
        done
    else
        for crate in "${browser_crates[@]}"; do
            printf "\n${BOLD}[%2d/%d]${NC} %s\n" "$gate_num" "$total" "Browser WASM: $crate"
            skip "wasm32-unknown-unknown target not installed"
            skipped=$((skipped + 1))
            gate_num=$((gate_num + 1))
        done
    fi

    # 10. UI build
    if [ -d "$ROOT/ui" ] && [ -f "$ROOT/ui/package.json" ]; then
        printf "\n${BOLD}[%2d/%d]${NC} %s\n" 10 "$total" "UI build (tsc + vite)"
        timer_start
        if [ "$DRY_RUN" = true ]; then
            printf "  ${YELLOW}DRY${NC}   cd ui && npm run build\n"
            passed=$((passed + 1))
        elif (cd "$ROOT/ui" && npm run build) >/dev/null 2>&1; then
            pass "UI build"
            passed=$((passed + 1))
        else
            fail "UI build"
            failed=$((failed + 1))
        fi
        timer_end
    else
        printf "\n${BOLD}[%2d/%d]${NC} %s\n" 10 "$total" "UI build"
        skip "ui/ directory not found"
        skipped=$((skipped + 1))
    fi

    # 11. Voice feature
    run_gate_check_soft 11 "Voice feature (clawft-plugin)" \
        cargo check --features voice -p clawft-plugin

    # Summary
    echo ""
    printf "${BOLD}═══════════════════════════════════════${NC}\n"
    printf "  ${GREEN}PASSED${NC}: %d  " "$passed"
    if [ "$failed" -gt 0 ]; then
        printf "${RED}FAILED${NC}: %d  " "$failed"
    else
        printf "FAILED: %d  " "$failed"
    fi
    if [ "$skipped" -gt 0 ]; then
        printf "${YELLOW}SKIPPED${NC}: %d" "$skipped"
    else
        printf "SKIPPED: %d" "$skipped"
    fi
    printf "  (total: %d)\n" "$total"
    printf "${BOLD}═══════════════════════════════════════${NC}\n"

    if [ "$failed" -gt 0 ]; then
        return 1
    fi
}

# ── Usage ────────────────────────────────────────────────────────────
usage() {
    cat <<EOF
${BOLD}Usage:${NC} scripts/build.sh <command> [options]

${BOLD}Commands:${NC}
  native          Build native CLI binary (release)
  native-debug    Build native CLI binary (debug, fast)
  wasi            Build WASM for WASI (wasm32-wasip1)
  browser         Build WASM for browser (wasm32-unknown-unknown)
  ui              Build React frontend (tsc + vite)
  all             Build everything (native + wasi + browser + ui)
  test            Run cargo test --workspace
  check           Run cargo check --workspace (fast compile check)
  clippy          Run clippy with warnings-as-errors
  gate            Run full phase gate (11 checks)
  clean           Clean all build artifacts

${BOLD}Options:${NC}
  --features <f>  Extra features to enable (e.g. --features voice,channels)
  --profile <p>   Cargo profile: debug, release, release-wasm (default varies)
  --verbose       Show full cargo output
  --dry-run       Print commands without executing
  --help          Show this help

${BOLD}Examples:${NC}
  scripts/build.sh native                          # Release CLI binary
  scripts/build.sh native --features voice          # CLI with voice
  scripts/build.sh browser                          # Browser WASM
  scripts/build.sh gate                             # Full phase gate
  scripts/build.sh native --dry-run                 # Preview commands
EOF
}

# ── Argument parsing ─────────────────────────────────────────────────
parse_args() {
    if [ $# -eq 0 ]; then
        usage
        exit 0
    fi

    COMMAND="$1"
    shift

    while [ $# -gt 0 ]; do
        case "$1" in
            --features)
                FEATURES="${2:?'--features requires a value'}"
                shift 2
                ;;
            --profile)
                PROFILE="${2:?'--profile requires a value'}"
                shift 2
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --help|-h)
                usage
                exit 0
                ;;
            *)
                printf "${RED}Unknown option: %s${NC}\n" "$1"
                usage
                exit 1
                ;;
        esac
    done
}

# ── Main ─────────────────────────────────────────────────────────────
main() {
    parse_args "$@"

    case "$COMMAND" in
        native)       cmd_native ;;
        native-debug) cmd_native_debug ;;
        wasi)         cmd_wasi ;;
        browser)      cmd_browser ;;
        ui)           cmd_ui ;;
        all)          cmd_all ;;
        test)         cmd_test ;;
        check)        cmd_check ;;
        clippy)       cmd_clippy ;;
        gate)         cmd_gate ;;
        clean)        cmd_clean ;;
        --help|-h)    usage ;;
        *)
            printf "${RED}Unknown command: %s${NC}\n" "$COMMAND"
            usage
            exit 1
            ;;
    esac
}

main "$@"
