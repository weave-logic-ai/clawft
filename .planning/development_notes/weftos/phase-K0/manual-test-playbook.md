# WeftOS K0-K2b Manual Test Playbook

**Purpose**: Comprehensive manual verification of all kernel features built through K2b.
**Time estimate**: ~20 minutes for full run-through.
**Prerequisite**: `cargo build -p clawft-weave --features exochain` succeeds.

---

## Quick Reference

```bash
BIN=target/debug/weaver
```

All commands below assume `$BIN` points to the debug build. Replace with release path
for release testing. The daemon uses `~/.clawft/` for state (chain, tree, keys).

---

## Section 1: Kernel Boot + Chain Persistence (K0) — ~3 min

### 1.1 Fresh Boot
```bash
rm -rf ~/.clawft/chain.* ~/.clawft/tree.* ~/.clawft/chain.key  # clean slate
$BIN kernel start
```
**Expect**: Boots to "Running". Shows boot phases in output. No errors.

### 1.2 Kernel Status
```bash
$BIN kernel status
```
**Expect**: `state=running`, `processes >= 1` (kernel PID 0), `services >= 1` (cron).

### 1.3 Chain Genesis
```bash
$BIN chain local -c 10
```
**Expect**: Chain events visible starting from `boot.init` through `boot.ready`. Each event has a sequence number, hash, and timestamp.

### 1.4 Chain Integrity
```bash
$BIN chain verify
```
**Expect**: "Chain integrity: VALID". If Ed25519 key exists, also "Signature: valid".

### 1.5 Shutdown + Persistence
```bash
# Ctrl+C the daemon (or: $BIN kernel stop)
```
**Expect**: Clean shutdown message. `~/.clawft/chain.rvf` (or `chain.json`) created.

### 1.6 Restart + Chain Restore
```bash
$BIN kernel start
$BIN chain local -c 20
```
**Expect**: Previous chain events visible (not fresh genesis). New boot events appended.

```bash
$BIN chain verify
```
**Expect**: "Chain integrity: VALID" across the full restored + new chain.

---

## Section 2: Resource Tree + Merkle (K0) — ~2 min

### 2.1 Tree Structure
```bash
$BIN resource tree
```
**Expect**: Tree with root `/` and namespaces: `/kernel`, `/kernel/agents`, `/kernel/services`, `/kernel/processes`, `/data`, `/network`, `/cluster`.

### 2.2 Node Inspection
```bash
$BIN resource inspect /kernel
```
**Expect**: Shows node metadata, child count, Merkle hash. Metadata includes `chain_seq` linking to chain.

### 2.3 Tree Stats
```bash
$BIN resource stats
```
**Expect**: `node_count >= 9` (bootstrap creates 9 nodes). Shows root hash.

---

## Section 3: Agent Lifecycle (K1) — ~4 min

### 3.1 Spawn Agent
```bash
$BIN agent spawn worker-1
```
**Expect**: Returns PID (e.g., `pid: 2`). Agent is "Running".

### 3.2 Agent in Process Table
```bash
$BIN kernel ps
```
**Expect**: Shows kernel (PID 0, running) + worker-1 (PID 2, running).

### 3.3 Agent in Resource Tree
```bash
$BIN resource tree
```
**Expect**: `/kernel/agents/worker-1` node present.

### 3.4 Agent Inspect
```bash
$BIN agent inspect <pid>
```
**Expect**: Shows agent_id, state=running, capabilities, resource_usage.

### 3.5 Ping Agent
```bash
$BIN agent send <pid> '{"cmd":"ping"}'
```
**Expect**: Returns `{"status":"ok","pid":<N>,"uptime_ms":<M>}`.

### 3.6 Echo Command
```bash
$BIN agent send <pid> '{"cmd":"echo","text":"hello world"}'
```
**Expect**: Returns `{"echo":"hello world","pid":<N>}`.

### 3.7 Exec Command
```bash
$BIN agent send <pid> '{"cmd":"exec","text":"test input"}'
```
**Expect**: Returns `{"status":"ok","echo":"test input","pid":<N>}`.

### 3.8 Unknown Command
```bash
$BIN agent send <pid> '{"cmd":"nosuchcmd"}'
```
**Expect**: Returns `{"error":"unknown command: nosuchcmd","pid":<N>}`.

### 3.9 Spawn Multiple Agents
```bash
$BIN agent spawn worker-2
$BIN agent spawn worker-3
$BIN agent list
```
**Expect**: 3 agents listed, each with unique PID, all Running.

### 3.10 Stop Agent (Graceful)
```bash
$BIN agent stop <pid>
$BIN agent inspect <pid>
```
**Expect**: State transitions to "exited(0)".

### 3.11 Agent Exit in Chain
```bash
$BIN chain local -c 5
```
**Expect**: `agent.spawn` and `agent.exit` (or `agent.stop`) events with agent_id and pid.

### 3.12 Scoring After Exit
```bash
$BIN resource inspect /kernel/agents/worker-1
```
**Expect**: NodeScoring visible with non-default values (trust, reliability boosted for clean exit).

---

## Section 4: A2A IPC + Topics (K2) — ~3 min

### 4.1 Agent-to-Agent Message
```bash
# With two agents running:
$BIN agent send <pid-A> '{"cmd":"ping"}'
```
**Expect**: Reply routed back to the sender (kernel PID 0).

### 4.2 IPC Chain Logging
```bash
$BIN chain local -c 10
```
**Expect**: `ipc.send` events showing from/to PIDs, payload_type, msg_id.

### 4.3 Topic Subscription
```bash
$BIN ipc subscribe <pid> build
$BIN ipc topics
```
**Expect**: Topic "build" listed with 1 subscriber.

### 4.4 Topic Publish
```bash
$BIN ipc publish build "build complete"
```
**Expect**: Published to subscriber(s). Returns delivery count.

---

## Section 5: Cron Scheduling (K1) — ~3 min

### 5.1 Add Cron Job via CLI
```bash
$BIN cron add --name "heartbeat" --interval 10 --command "ping" --target <pid>
```
**Expect**: Returns job ID, name, interval.

### 5.2 List Cron Jobs
```bash
$BIN cron list
```
**Expect**: Shows heartbeat job with interval=10s, enabled=true.

### 5.3 Add Cron Job via Agent
```bash
$BIN agent send <pid> '{"cmd":"cron.add","name":"check","interval_secs":30,"command":"health"}'
```
**Expect**: Returns job info with ID.

### 5.4 List via Agent
```bash
$BIN agent send <pid> '{"cmd":"cron.list"}'
```
**Expect**: Returns array of all cron jobs.

### 5.5 Remove Cron Job
```bash
$BIN cron remove <job-id>
```
**Expect**: `{"removed": true}`.

### 5.6 Cron in Chain
```bash
$BIN chain local -c 10
```
**Expect**: `cron.add` and `cron.remove` events.

---

## Section 6: K2b Hardening Features — ~4 min

### 6.1 Health Monitoring
```bash
# Let daemon run for 30+ seconds (default health interval)
$BIN kernel logs -c 10
```
**Expect**: Health check entries in kernel event log. If any service degraded, warning entries visible.

### 6.2 Resource Usage Tracking
```bash
# Send several messages to an agent
$BIN agent send <pid> '{"cmd":"ping"}'
$BIN agent send <pid> '{"cmd":"ping"}'
$BIN agent send <pid> '{"cmd":"ping"}'
$BIN agent send <pid> '{"cmd":"exec","text":"test"}'
$BIN agent inspect <pid>
```
**Expect**: `resource_usage.messages_sent >= 4`, `resource_usage.tool_calls >= 1`, `resource_usage.cpu_time_ms > 0`.

### 6.3 Agent Suspend/Resume
```bash
$BIN agent send <pid> '{"cmd":"suspend"}'
$BIN agent inspect <pid>
```
**Expect**: State = "suspended".

```bash
# Messages while suspended should get error response
$BIN agent send <pid> '{"cmd":"ping"}'
```
**Expect**: Returns `{"error":"agent suspended","pid":<N>}`.

```bash
$BIN agent send <pid> '{"cmd":"resume"}'
$BIN agent inspect <pid>
```
**Expect**: State = "running".

```bash
# Normal commands work again
$BIN agent send <pid> '{"cmd":"ping"}'
```
**Expect**: Returns `{"status":"ok",...}`.

### 6.4 Gate-Checked Commands
```bash
# This tests that exec/cron commands check capabilities.
# Default agents have full capabilities, so these should succeed.
$BIN agent send <pid> '{"cmd":"exec","text":"gated"}'
```
**Expect**: Returns success (default agent has can_exec_tools=true).

### 6.5 Graceful Shutdown with Running Agents
```bash
# With 2+ agents running:
$BIN agent list
# Ctrl+C the daemon
```
**Expect**: Shutdown message shows agents exiting cleanly. Chain events for each agent exit.

```bash
# Restart and verify
$BIN kernel start
$BIN chain local -c 20
```
**Expect**: `agent.exit` events for each agent that was running at shutdown. `kernel.shutdown` event with tree/chain metadata.

---

## Section 7: Cross-Cutting Verification — ~2 min

### 7.1 Full Chain Integrity After All Operations
```bash
$BIN chain verify
```
**Expect**: "Chain integrity: VALID". Covers all events from all sections.

### 7.2 Tree Checkpoint Persistence
```bash
# After shutdown+restart:
$BIN resource stats
```
**Expect**: `node_count >= 9`. Root hash matches chain's last recorded tree_root_hash.

### 7.3 RVF Payload Delivery
```bash
# If RVF-framed connection is available:
# Send a binary RVF segment to an agent — it should acknowledge receipt.
# (This requires the RVF client tooling from weaver's rvf_rpc module.)
```
**Expect**: Agent returns `{"status":"ok","cmd":"rvf.recv",...}`.

---

## Quick Smoke Test (~5 min subset)

For time-constrained verification, run these core scenarios:

1. **Boot**: `kernel start` + `kernel status` (1.1, 1.2)
2. **Agent lifecycle**: `agent spawn` + `agent send ping` + `agent stop` (3.1, 3.5, 3.10)
3. **Cron**: `cron add` + `cron list` + `cron remove` (5.1, 5.2, 5.5)
4. **Chain**: `chain local` + `chain verify` (1.3, 1.4)
5. **Persistence**: Ctrl+C + `kernel start` + `chain local` (1.5, 1.6)

---

## Test Result Template

| Section | Test | Pass/Fail | Notes |
|---------|------|-----------|-------|
| 1.1 | Fresh boot | | |
| 1.2 | Kernel status | | |
| 1.3 | Chain genesis | | |
| 1.4 | Chain integrity | | |
| 1.5 | Shutdown persistence | | |
| 1.6 | Restart + restore | | |
| 2.1 | Tree structure | | |
| 2.2 | Node inspection | | |
| 2.3 | Tree stats | | |
| 3.1 | Spawn agent | | |
| 3.2 | Process table | | |
| 3.3 | Agent in tree | | |
| 3.4 | Agent inspect | | |
| 3.5 | Ping command | | |
| 3.6 | Echo command | | |
| 3.7 | Exec command | | |
| 3.8 | Unknown command | | |
| 3.9 | Multiple agents | | |
| 3.10 | Stop agent | | |
| 3.11 | Agent exit chain | | |
| 3.12 | Scoring after exit | | |
| 4.1 | Agent message | | |
| 4.2 | IPC chain logging | | |
| 4.3 | Topic subscribe | | |
| 4.4 | Topic publish | | |
| 5.1 | Cron add CLI | | |
| 5.2 | Cron list | | |
| 5.3 | Cron add via agent | | |
| 5.4 | Cron list via agent | | |
| 5.5 | Cron remove | | |
| 5.6 | Cron chain events | | |
| 6.1 | Health monitoring | | |
| 6.2 | Resource tracking | | |
| 6.3 | Suspend/resume | | |
| 6.4 | Gate-checked cmds | | |
| 6.5 | Graceful shutdown | | |
| 7.1 | Full chain verify | | |
| 7.2 | Tree persistence | | |
