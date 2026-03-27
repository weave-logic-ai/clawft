#!/usr/bin/env bash
set -euo pipefail

echo "=== WeftOS K3-K6 Phase Gate ==="
echo ""

PASS=0
FAIL=0

run_gate() {
    local name="$1"
    local cmd="$2"
    printf "  %-60s " "$name"
    if eval "$cmd" > /dev/null 2>&1; then
        echo "PASS"
        PASS=$((PASS + 1))
    else
        echo "FAIL"
        FAIL=$((FAIL + 1))
    fi
}

echo "--- K3: WASM Sandbox ---"
run_gate "WASM tool loads and executes" \
    "cargo test -p clawft-kernel --features 'native,wasm-sandbox' -- k3_wasm_tool_loads_and_executes"
run_gate "Fuel exhaustion terminates cleanly" \
    "cargo test -p clawft-kernel --features 'native,wasm-sandbox' -- k3_fuel_exhaustion_terminates_cleanly"
run_gate "Memory limit prevents allocation bomb" \
    "cargo test -p clawft-kernel --features 'native,wasm-sandbox' -- k3_memory_limit_prevents_allocation_bomb"
run_gate "Host filesystem not accessible from sandbox" \
    "cargo test -p clawft-kernel --features 'native,wasm-sandbox' -- k3_host_filesystem_not_accessible_from_sandbox"

echo ""
echo "--- K4: Containers ---"
run_gate "Container config validates" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- container_config_validates"
run_gate "Container lifecycle start/stop" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- container_lifecycle_configure_start_stop"
run_gate "Container health propagates to kernel" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- container_health_propagates_to_kernel_health_system"

echo ""
echo "--- K5: Application Framework ---"
run_gate "App manifest parsed and validated" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- k5_manifest_parsed_and_validated"
run_gate "App install/start/stop lifecycle" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- k5_app_install_start_stop_lifecycle"
run_gate "App agents spawn with correct capabilities" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- k5_app_agents_spawn_with_correct_capabilities"
run_gate "App list shows installed applications" \
    "cargo test -p clawft-kernel --features 'native,exochain' -- k5_app_list_shows_installed"

echo ""
echo "--- K6: Distributed Fabric ---"
run_gate "Two-node cluster forms and discovers peers" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh' -- two_node_cluster_forms"
run_gate "Cross-node IPC delivers messages" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh' -- cross_node_ipc_delivers"
run_gate "WebSocket transport send/recv" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh' -- ws_transport_connect_send_recv"
run_gate "WebSocket transport scheme detection" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh' -- ws_transport_supports"
run_gate "Browser node connects via WebSocket" \
    "cargo test -p clawft-kernel --features 'native,exochain,cluster,mesh' -- browser_node_connects_via_websocket"

echo ""
echo "================================"
echo "  PASS: $PASS / $((PASS + FAIL))"
echo "  FAIL: $FAIL / $((PASS + FAIL))"
echo "================================"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
echo "All gates passed!"
