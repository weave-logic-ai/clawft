# WeftOS Kernel Manual Testing Guide

Version: K0-K5 Sprint Completion
Date: 2026-03-25
Branch: feature/weftos-kernel-sprint
Estimated Total Time: 25-30 minutes

---

## Prerequisites

- Rust 1.93+ installed (`rustup show` to verify)
- Git checkout of `feature/weftos-kernel-sprint` branch
- Docker installed (optional, for Pass 4 container tests)
- Terminal with at least 120 columns for table output

---

## Pass 1: Build Verification (5 min)

Validates that the codebase compiles, passes all tests, and is lint-clean across all feature combinations.

### 1.1 Compile Check (default features)

```bash
scripts/build.sh check
```

- [ ] **Expected**: "Finished" with exit code 0
- [ ] **Acceptance**: No compilation errors

### 1.2 Compile Check (all kernel features)

```bash
cargo check -p clawft-kernel --features "exochain,ecc,wasm-sandbox,containers,cluster"
```

- [ ] **Expected**: "Finished" with exit code 0
- [ ] **Acceptance**: No compilation errors with all features enabled

### 1.3 Run All Tests (kernel)

```bash
cargo test -p clawft-kernel --features "exochain,ecc,wasm-sandbox"
```

- [ ] **Expected**: "test result: ok. 600+ passed; 0 failed"
- [ ] **Acceptance**: Zero failures, zero panics
- [ ] **Note**: Exact count may vary slightly; 606 as of 2026-03-25

### 1.4 Run All Tests (weave CLI)

```bash
cargo test -p clawft-weave
```

- [ ] **Expected**: "test result: ok. 14+ passed; 0 failed"
- [ ] **Acceptance**: Zero failures

### 1.5 Clippy Lint Check

```bash
cargo clippy -p clawft-kernel --features "exochain,ecc,wasm-sandbox" -- -W clippy::all
```

- [ ] **Expected**: 3 or fewer warnings (collapsible if statements)
- [ ] **Acceptance**: No errors, no new warnings beyond the 3 known collapsible-if items
- [ ] **Note**: Warnings are auto-fixable; zero warnings in clippy::correctness category

### 1.6 Rustdoc Build

```bash
cargo doc -p clawft-kernel --features "exochain,ecc,wasm-sandbox" --no-deps 2>&1 | grep -c "warning"
```

- [ ] **Expected**: Low or zero warning count
- [ ] **Acceptance**: Docs build successfully (exit code 0)

### 1.7 WASM Target Check

```bash
cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser 2>&1 | tail -3
```

- [ ] **Expected**: "Finished" -- kernel crate excluded from browser build
- [ ] **Acceptance**: No compilation errors; kernel modules correctly gated

---

## Pass 2: Kernel Boot and Console (10 min)

Tests the daemon lifecycle, kernel status reporting, and interactive console. Requires building the `weaver` binary first.

### 2.0 Build the Binary

```bash
cargo build --bin weaver --features "exochain,ecc,wasm-sandbox"
```

- [ ] **Expected**: Binary builds successfully
- [ ] **Note**: Binary location: `target/debug/weaver`

### 2.1 Start Daemon

```bash
weaver kernel start
```

- [ ] **Expected**: "Kernel daemon started" or similar confirmation
- [ ] **Acceptance**: Process starts, PID file created, no error output
- [ ] **Fallback**: If `weaver` not in PATH, use `./target/debug/weaver kernel start`

### 2.2 Check Kernel Status

```bash
weaver kernel status
```

- [ ] **Expected**: Output showing kernel state "Running", uptime, process count, service count
- [ ] **Acceptance**: State is "Running", services count > 0
- [ ] **Look for**: Kernel state, uptime duration, active service names

### 2.3 List Services

```bash
weaver kernel services
```

- [ ] **Expected**: Table showing registered services with name, type, and health status
- [ ] **Acceptance**: At least one service listed (e.g., cron-service, health-monitor)
- [ ] **Look for**: Service names, types (Core/Plugin/Custom), health (Healthy/Unknown)

### 2.4 List Processes

```bash
weaver kernel ps
```

- [ ] **Expected**: Process table with PID, agent ID, state, resource usage columns
- [ ] **Acceptance**: Output renders as a table (may be empty if no agents spawned)
- [ ] **Look for**: Column headers (PID, AGENT, STATE, MEM, CPU)

### 2.5 Show Kernel Logs

```bash
weaver kernel logs
```

- [ ] **Expected**: Boot events and runtime log entries
- [ ] **Acceptance**: Chronologically ordered entries with timestamps
- [ ] **Look for**: [INIT], [SERVICES], [READY] phase markers from boot sequence

### 2.6 Console Mode (Interactive)

```bash
weaver console --attach
```

Once in the console (the `weave>` prompt):

```
status
```
- [ ] **Expected**: Kernel status summary (same data as `weaver kernel status`)

```
ps
```
- [ ] **Expected**: Process table listing

```
services
```
- [ ] **Expected**: Service listing

```
boot-log
```
- [ ] **Expected**: Replay of boot events

```
chain status
```
- [ ] **Expected**: Chain info (chain_id, sequence number, latest hash)

```
ecc status
```
- [ ] **Expected**: ECC subsystem status (calibration results, tick stats) if ecc feature enabled; error message if not

```
exit
```
- [ ] **Expected**: Console exits cleanly, returns to shell
- [ ] **Acceptance**: No hang, no panic, prompt returns

### 2.7 Stop Daemon

```bash
weaver kernel stop
```

- [ ] **Expected**: "Kernel daemon stopped" or similar confirmation
- [ ] **Acceptance**: Process terminates, PID file removed
- [ ] **Look for**: Graceful shutdown message (agents stopped, chain checkpointed)

---

## Pass 3: Chain, Resource Tree, and ECC (5 min)

Tests the ExoChain audit trail, resource tree, and ECC cognitive substrate. Requires daemon running.

### 3.0 Start Daemon (if not already running)

```bash
weaver kernel start
```

- [ ] **Expected**: Daemon starts successfully

### 3.1 Chain Status

```bash
weaver chain status
```

- [ ] **Expected**: Chain ID, current sequence number, latest hash, event count
- [ ] **Acceptance**: Chain ID is present, sequence > 0 (boot events logged)

### 3.2 Chain Local Events

```bash
weaver chain local
```

- [ ] **Expected**: List of chain events with sequence, kind, timestamp
- [ ] **Acceptance**: Boot events present (boot.init, service.register, boot.ready)
- [ ] **Look for**: Chronological ordering, valid timestamps

### 3.3 Chain Verify

```bash
weaver chain verify
```

- [ ] **Expected**: "Chain verified: N events, integrity OK" or similar
- [ ] **Acceptance**: Verification passes with no integrity errors
- [ ] **Look for**: Hash chain continuity confirmed

### 3.4 Resource Stats

```bash
weaver resource stats
```

- [ ] **Expected**: Resource tree statistics (node count, depth, namespaces)
- [ ] **Acceptance**: Node count > 0, namespaces listed

### 3.5 ECC Status

```bash
weaver ecc status
```

- [ ] **Expected**: Calibration results (compute p50/p95, tick interval), tick count
- [ ] **Acceptance**: If ecc feature enabled: calibration data present. If not: clear "feature not enabled" error

### 3.6 ECC Tick

```bash
weaver ecc tick
```

- [ ] **Expected**: Current tick statistics (interval_ms, tick_count, drift metrics)
- [ ] **Acceptance**: Tick count incrementing if cognitive tick service is running

### 3.7 Stop Daemon

```bash
weaver kernel stop
```

- [ ] **Expected**: Clean shutdown with chain checkpoint saved

---

## Pass 4: Application Lifecycle (5 min)

Tests the K5 application framework. Requires daemon running and a test manifest.

### 4.0 Start Daemon

```bash
weaver kernel start
```

- [ ] **Expected**: Daemon starts successfully

### 4.1 List Apps (empty)

```bash
weaver app list
```

- [ ] **Expected**: Empty list or "No applications installed"
- [ ] **Acceptance**: No error, clean empty state

### 4.2 Create Test Manifest

Create a minimal test app manifest:

```bash
mkdir -p /tmp/test-weftapp
cat > /tmp/test-weftapp/weftapp.json << 'EOF'
{
  "name": "test-app",
  "version": "0.1.0",
  "description": "Manual testing application",
  "agents": [
    {
      "id": "worker",
      "role": "test-worker",
      "auto_start": true,
      "capabilities": {
        "can_exec_tools": false,
        "can_read_context": true,
        "sandbox": { "allow_shell": false, "allow_network": false },
        "ipc_scope": "all"
      }
    }
  ],
  "tools": [],
  "services": [],
  "capabilities": {
    "network": false,
    "filesystem": [],
    "shell": false
  },
  "hooks": {}
}
EOF
```

- [ ] **Expected**: File created at /tmp/test-weftapp/weftapp.json

### 4.3 Install App

```bash
weaver app install /tmp/test-weftapp
```

- [ ] **Expected**: "Application 'test-app' installed" or similar
- [ ] **Acceptance**: No error, app name echoed back

### 4.4 List Apps (installed)

```bash
weaver app list
```

- [ ] **Expected**: "test-app" appears in the list with state "Installed"
- [ ] **Acceptance**: At least one app listed

### 4.5 Inspect App

```bash
weaver app inspect test-app
```

- [ ] **Expected**: Detailed app info: name, version, description, agents, tools, services
- [ ] **Acceptance**: All manifest fields rendered correctly

### 4.6 Start App

```bash
weaver app start test-app
```

- [ ] **Expected**: "Application 'test-app' started" or similar
- [ ] **Acceptance**: State transitions to Running

### 4.7 Verify App Running

```bash
weaver app list
```

- [ ] **Expected**: "test-app" shows state "Running"

```bash
weaver kernel ps
```

- [ ] **Expected**: Agent "test-app/worker" appears in process table (if auto_start worked)

### 4.8 Stop App

```bash
weaver app stop test-app
```

- [ ] **Expected**: "Application 'test-app' stopped" or similar
- [ ] **Acceptance**: State transitions to Stopped

### 4.9 Remove App

```bash
weaver app remove test-app
```

- [ ] **Expected**: "Application 'test-app' removed" or similar
- [ ] **Acceptance**: App no longer appears in `weaver app list`

### 4.10 Verify Removal

```bash
weaver app list
```

- [ ] **Expected**: Empty list or "No applications installed"

### 4.11 Stop Daemon

```bash
weaver kernel stop
```

- [ ] **Expected**: Clean shutdown

### 4.12 Cleanup

```bash
rm -rf /tmp/test-weftapp
```

- [ ] **Expected**: Test files removed

---

## Pass 5: Agent Lifecycle (5 min)

Tests agent spawn, inspect, messaging, and stop via the CLI.

### 5.0 Start Daemon

```bash
weaver kernel start
```

- [ ] **Expected**: Daemon starts

### 5.1 Spawn Agent

```bash
weaver agent spawn test-agent
```

- [ ] **Expected**: Agent spawned with PID echoed back
- [ ] **Acceptance**: PID > 0 returned

### 5.2 List Agents

```bash
weaver agent list
```

- [ ] **Expected**: test-agent appears in the list with state Running
- [ ] **Acceptance**: At least one agent listed

### 5.3 Inspect Agent

```bash
weaver agent inspect 1
```

(Use the PID returned from spawn)

- [ ] **Expected**: Agent details: PID, agent_id, state, capabilities, resource usage
- [ ] **Acceptance**: State shows Running, resource counters present

### 5.4 Send Message

```bash
weaver agent send 1 "hello from test"
```

- [ ] **Expected**: Message sent confirmation or response from agent
- [ ] **Acceptance**: No error

### 5.5 Stop Agent

```bash
weaver agent stop 1
```

- [ ] **Expected**: Agent stopped confirmation
- [ ] **Acceptance**: Agent transitions to Exited state

### 5.6 Verify Chain Events

```bash
weaver chain local
```

- [ ] **Expected**: agent.spawn and agent.stop events visible in chain
- [ ] **Acceptance**: Both lifecycle events present with correct PIDs

### 5.7 Stop Daemon

```bash
weaver kernel stop
```

- [ ] **Expected**: Clean shutdown

---

## Acceptance Summary

### Pass Results

- [ ] Pass 1: Build Verification -- all checks green
- [ ] Pass 2: Kernel Boot and Console -- all commands work
- [ ] Pass 3: Chain, Tree, and ECC -- audit trail intact
- [ ] Pass 4: Application Lifecycle -- full install-to-remove cycle
- [ ] Pass 5: Agent Lifecycle -- spawn, interact, stop, verify chain

### Overall

- [ ] All passes complete
- [ ] No unexpected errors or panics
- [ ] All services reported healthy
- [ ] Chain integrity verified
- [ ] 600+ kernel tests passing
- [ ] 14+ weave CLI tests passing
- [ ] Clippy warnings <= 3 (known, auto-fixable)

### Known Limitations

1. Container tests (K4) require Docker daemon -- ContainerManager returns DockerNotAvailable without it
2. App state is in-memory only -- apps are lost on daemon restart
3. ECC features require `--features ecc` at build time
4. WASM sandbox requires `--features wasm-sandbox` at build time
5. `weaver agent attach <pid>` is stubbed (prints not-yet-implemented)
6. Console commands may differ slightly from documented format depending on daemon state

### Test Environment Notes

- If running on a CI server without a display, skip Pass 2.6 (interactive console)
- If Docker is not available, skip Pass 4 container service tests (agent-only apps still work)
- All Pass 1 checks can run without a daemon
- Passes 2-5 require a running daemon (`weaver kernel start`)
