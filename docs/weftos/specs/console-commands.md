# WeftOS Console Command Catalog

**Version**: 0.2.0
**Date**: 2026-03-27
**Source**: Sprint 11 Symposium Track 4, Session 1 (Console-to-Kernel Connection Model)
**Status**: Formal specification -- implementable

---

## Overview

The WeftOS Console is a first-class OS shell powered by the kernel's `ShellAdapter`. Every command maps to `ServiceApi::call(service, method, params)`. Commands are:

- **Governed**: Every command passes the dual-layer governance gate (CapabilityGate + TileZeroGate) before execution.
- **Witnessed**: Every command execution is logged to the ExoChain as a `ChainEvent` with kind `shell.exec`, `shell.deny`, or `shell.error`.
- **Tab-completable**: The `CompletionProvider` returns candidates based on live kernel state.

### Command Syntax

```
<namespace>.<method> [arguments...]
```

Arguments are positional or named with `--flag value` syntax. The `ShellAdapter` parser splits the input string at the first `.` to determine the namespace and method, then passes remaining tokens as params to `ServiceApi::call()`.

### Response Format

Every command returns a `ShellResult`:

```
ShellResult {
  text: String          -- Plain text output (always present)
  block: Option<JSON>   -- Optional block descriptor for rich rendering
  display: DisplayHint  -- Text | Table | Block | SpawnBlock
}
```

The console renders `text` by default. If `display` is `Block`, it renders the JSON descriptor inline. If `display` is `SpawnBlock`, it detaches the descriptor as a new Lego block pane.

### Governance Display

Each command shows its governance decision:

```
weftos> process.spawn coder
[PERMIT] governance: risk=0.1, security=0.2 (below threshold)
Spawned PID 42: coder-42

weftos> config.set max_agents 100
[WARN] governance: risk=0.4, security=0.3 (permit-with-warning)
! Increasing max_agents may impact system stability
Config set: max_agents = 100

weftos> mesh.resolve sensitive-data-service
[DENY] governance: privacy=0.9, risk=0.7 (above threshold)
Access denied: privacy threshold exceeded. Escalate with --force-human-review.
```

---

## Namespace: kernel

System-level kernel operations.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `kernel.status` | (none) | KernelStatus | read gate | No | Show kernel version, uptime, health score |
| `kernel.metrics` | (none) | KernelMetrics | read gate | No | Show CPU, memory, tick count, peer count |
| `kernel.shutdown` | `[--force]` | bool | **dual-layer** | Yes | Graceful kernel shutdown. `--force` skips drain. |

**Example session**:
```
weftos> kernel.status
[PERMIT]
WeftOS Kernel v0.2.0
Uptime:    14h 22m
Health:    100%
Services:  14 active
Processes: 4 running, 1 idle
Chain:     block #4,271

weftos> kernel.metrics
[PERMIT]
CPU:       12%
Memory:    342 MB / 2048 MB (17%)
Tick:      #847,291 (interval: 100ms)
Peers:     5 connected
```

---

## Namespace: process

Agent process lifecycle management.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `process.list` | (none) | Vec<ProcessEntry> | read gate | No | List all processes with PID, agent type, state |
| `process.spawn` | `<type> [--name <name>]` | SpawnResult | dual-layer | Yes | Spawn a new agent process |
| `process.stop` | `<pid>` | bool | dual-layer | Yes | Stop an agent by PID |
| `process.info` | `<pid>` | ProcessDetail | read gate | No | Detailed process info including capabilities |

**Arguments detail**:
- `<type>`: Agent type from the agent registry (e.g., `coder`, `reviewer`, `tester`, `planner`, `researcher`, `security-auditor`)
- `<pid>`: Process ID (integer)
- `--name`: Optional human-readable name for the agent

**Example session**:
```
weftos> process.list
[PERMIT]
PID   Agent        Type       State    CPU    Memory
001   weaver-0     weaver     running  12%    48 MB
002   coder-1      coder      running   8%    32 MB
003   reviewer-0   reviewer   idle      0%    16 MB
004   tester-1     tester     running   5%    28 MB

weftos> process.spawn coder --name coder-2
[PERMIT] governance: risk=0.1, security=0.2
Spawned PID 005: coder-2 (type: coder)

weftos> process.info 005
[PERMIT]
PID:          005
Agent ID:     coder-2
Type:         coder
State:        running
CPU:          3%
Memory:       24 MB
Uptime:       2m 14s
Capabilities: [tool.exec, ipc.send, file.read, file.write]
Parent:       supervisor-0
```

---

## Namespace: service

Service registry operations.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `service.list` | (none) | Vec<ServiceInfo> | read gate | No | List all registered services |
| `service.health` | `[<name>]` | HealthStatus | read gate | No | Health of one or all services |
| `service.register` | `<name>` | bool | dual-layer | Yes | Register a new service |

**Example session**:
```
weftos> service.list
[PERMIT]
Service            Version   Health   Handlers
registry           0.2.0     OK       3
governance         0.2.0     OK       4
mesh-sync          0.2.0     OK       2
chain-manager      0.2.0     OK       3
hnsw-service       0.2.0     OK       2
process-table      0.2.0     OK       4
causal-graph       0.2.0     OK       2
config-service     0.2.0     OK       3
tool-registry      0.2.0     OK       2
weaver             0.2.0     OK       3

weftos> service.health governance
[PERMIT]
Service:    governance
Version:    0.2.0
Health:     OK
Uptime:     14h 22m
Requests:   4,271 total, 0 errors
Avg Latency: 2ms
```

---

## Namespace: chain

ExoChain query and integrity verification.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `chain.query` | `[--from <N>] [--limit <N>]` | Vec<ChainEvent> | read gate | No | Query ExoChain events |
| `chain.height` | (none) | u64 | read gate | No | Current chain height (latest sequence number) |
| `chain.verify` | `[<seq>]` | VerifyResult | read gate | No | Verify chain integrity from sequence |

**Arguments detail**:
- `--from`: Starting sequence number (default: latest - limit)
- `--limit`: Maximum events to return (default: 20)
- `<seq>`: Sequence number to verify from (default: 0, full verification)

**Example session**:
```
weftos> chain.height
[PERMIT]
Chain height: 4,271

weftos> chain.query --from 4268 --limit 5
[PERMIT]
Seq   Kind          Source        Summary                  Time
4268  shell.exec    gui.console   process.spawn coder      14:22:58
4269  governance    governance    permit: risk=0.1         14:22:58
4270  ipc.send      coder-1       tool.exec file_read      14:23:01
4271  shell.exec    gui.console   chain.query              14:23:15

weftos> chain.verify 4200
[PERMIT]
Verifying chain from #4200 to #4271...
All 72 events verified. No integrity violations.
Hash chain: consistent
Timestamps: monotonic
```

---

## Namespace: ecc

ECC cognitive substrate -- HNSW semantic search and causal graph queries.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `ecc.search` | `<query> [--top-k <N>]` | Vec<EccResult> | read gate | No | Semantic search via HNSW index |
| `ecc.graph` | `[--depth <N>] [--root <id>]` | GraphData | read gate | No | Dump causal graph subgraph |
| `ecc.tick-stats` | (none) | TickStats | read gate | No | DEMOCRITUS tick loop statistics |

**Arguments detail**:
- `<query>`: Free-text semantic query string
- `--top-k`: Number of results (default: 10)
- `--depth`: Graph traversal depth (default: 3)
- `--root`: Root node ID for subgraph extraction

**Example session**:
```
weftos> ecc.search "authentication failure patterns" --top-k 5
[PERMIT]
Score  Node ID                  Type        Summary
0.92   auth-validate-bug        Bug         Missing input validation in auth/validate.rs
0.87   auth-module-design       Design      Auth module architecture decision
0.83   pr-47-merged             Event       PR #47 merged: auth refactor
0.79   auth-timeout-incident    Incident    Auth timeout errors (Tue 02:14)
0.71   jwt-refresh-pattern      Pattern     JWT with refresh token pattern

weftos> ecc.graph --depth 2 --root auth-timeout-incident
[PERMIT]
auth-timeout-incident (Incident)
  <-Causes- pr-47-merged (Event)
    <-Causes- auth-validate-bug (Bug)
  -Causes-> auto-rollback-v093 (Event)
  -Causes-> hotfix-pr-52 (Event)
    -Causes-> auth-validate-fix (Fix)

weftos> ecc.tick-stats
[PERMIT]
DEMOCRITUS Tick Loop
Current tick:    #847,291
Interval:        100ms
Avg duration:    2.3ms
Max duration:    18ms (tick #847,102)
Nodes processed: 1,247
Edges updated:   3,891
HNSW index size: 12,471 vectors
```

---

## Namespace: governance

Governance engine operations.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `governance.check` | `<action>` | GovernanceDecision | No (it IS governance) | Yes | Run governance gate on hypothetical action |
| `governance.rules` | (none) | Vec<GovernanceRule> | read gate | No | List active governance rules |
| `governance.history` | `[--limit <N>]` | Vec<GovernanceDecision> | read gate | No | Recent governance decisions |

**Arguments detail**:
- `<action>`: Action string to evaluate (e.g., `tool.exec code_search`, `ipc.send agent-5`)
- `--limit`: Max history entries (default: 20)

**Example session**:
```
weftos> governance.check "config.set max_agents 100"
EffectVector:
  risk:      0.4
  fairness:  0.0
  privacy:   0.0
  novelty:   0.1
  security:  0.3
Decision: PermitWithWarning
Reason: Increasing max_agents beyond 50 may impact system stability

weftos> governance.rules
[PERMIT]
Rule                     Threshold  Scope              Status
max-risk                 0.7        all                active
privacy-gate             0.5        mesh.*             active
security-audit           0.6        tool.exec          active
human-escalation         0.8        config.set         active
rate-limit-spawn         10/min     process.spawn      active

weftos> governance.history --limit 3
[PERMIT]
Time          Action                Decision         Risk
14:23:15      chain.query           Permit           0.0
14:22:58      process.spawn coder   Permit           0.1
14:20:01      config.set tick_ms    PermitWithWarn   0.3
```

---

## Namespace: mesh

Mesh networking and peer discovery.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `mesh.peers` | (none) | Vec<PeerInfo> | read gate | No | List connected mesh peers |
| `mesh.topology` | (none) | TopologyData | read gate | No | Show mesh topology graph |
| `mesh.resolve` | `<service>` | Vec<ServiceLocation> | read gate | No | Resolve a service across the mesh |

**Example session**:
```
weftos> mesh.peers
[PERMIT]
Peer      Type    Status  Latency   Services
node-01   cloud   OK      8ms       14
node-02   edge    OK      12ms      6
node-03   edge    WARN    45ms      4
node-04   cloud   OK      10ms      12
node-05   edge    DOWN    --        --

weftos> mesh.topology
[PERMIT]
          [self]
         /  |  \
       /    |    \
  [node-01] | [node-03]
      |   [node-02]   |
      |     |         |
  [node-04]-+-[node-05]

weftos> mesh.resolve governance
[PERMIT]
Service: governance
Locations:
  self        (local, 0ms)
  node-01     (cloud, 8ms)
  node-04     (cloud, 10ms)
Recommended: self (lowest latency)
```

---

## Namespace: config

Configuration management.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `config.get` | `<key>` | ConfigValue | read gate | No | Get a configuration value |
| `config.set` | `<key> <value>` | bool | dual-layer | Yes | Set a configuration value |
| `config.list` | `[<namespace>]` | Vec<ConfigEntry> | read gate | No | List config entries |

**Arguments detail**:
- `<key>`: Dot-separated config key (e.g., `tick_interval_ms`, `max_agents`, `governance.risk_threshold`)
- `<value>`: New value (string, number, or boolean)
- `<namespace>`: Optional namespace filter

**Example session**:
```
weftos> config.list
[PERMIT]
Key                        Value      Type
tick_interval_ms           100        number
max_agents                 50         number
governance.risk_threshold  0.7        number
governance.enabled         true       boolean
mesh.discovery_interval    5000       number
chain.max_payload_bytes    65536      number
hnsw.ef_construction       200        number
hnsw.m_connections         16         number

weftos> config.get max_agents
[PERMIT]
max_agents = 50

weftos> config.set max_agents 75
[WARN] governance: risk=0.3
Config set: max_agents = 75
```

---

## Namespace: weaver

Weaver AI assistant operations.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `weaver.generate` | `<description>` | BlockDescriptor | dual-layer | Yes | Generate a block descriptor from natural language |
| `weaver.session` | (none) | SessionInfo | read gate | No | Current Weaver session info |

**Example session**:
```
weftos> weaver.generate "dashboard showing CPU and memory metrics"
[PERMIT] governance: risk=0.2, novelty=0.4
Generated block descriptor:
{
  "version": "0.2.0",
  "root": "dash",
  "elements": {
    "dash": { "type": "Row", "children": ["cpu", "mem"] },
    "cpu": { "type": "Metric", "props": { "label": "CPU", "value": { "$state": "/kernel/metrics/cpu_percent" }, "unit": "%" } },
    "mem": { "type": "Metric", "props": { "label": "Memory", "value": { "$state": "/kernel/metrics/mem_percent" }, "unit": "%" } }
  }
}
Descriptor validated against catalog schema: PASS

weftos> weaver.session
[PERMIT]
Session ID:    sess-001
Started:       2026-03-27T10:00:00Z
Descriptors:   3 generated, 3 valid
Journeys:      1 generated
Context:       clawft (Rust kernel, 22 crates)
```

---

## Namespace: wasm

WASM tool registry and execution.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `wasm.tools` | (none) | Vec<ToolInfo> | read gate | No | List registered WASM tools |
| `wasm.exec` | `<tool> <input>` | ToolResult | dual-layer | Yes | Execute a WASM tool with JSON input |

**Arguments detail**:
- `<tool>`: Tool name from the ToolRegistry
- `<input>`: JSON string input for the tool

**Example session**:
```
weftos> wasm.tools
[PERMIT]
Tool           Version  Signed   Fuel Limit  Memory
code_search    0.2.0    Ed25519  1000000     4 MB
file_read      0.2.0    Ed25519  500000      2 MB
file_write     0.2.0    Ed25519  500000      2 MB
browser_nav    0.2.0    Ed25519  2000000     8 MB
shell_exec     0.2.0    Ed25519  1000000     4 MB

weftos> wasm.exec shell_exec '{"command": "echo hello"}'
[PERMIT] governance: risk=0.3, security=0.4
Tool: shell_exec
Fuel used: 12,450 / 1,000,000
Output: {"stdout": "hello\n", "stderr": "", "exit_code": 0}
Duration: 3ms
```

---

## Namespace: system (reserved)

System-level commands that are not service-routed. These are handled directly by the ShellAdapter.

| Command | Arguments | Return Type | Governance | ECC Witnessed | Description |
|---------|-----------|-------------|------------|---------------|-------------|
| `help` | `[<namespace>]` | String | No | No | Show available commands or namespace help |
| `clear` | (none) | void | No | No | Clear console output |
| `history` | `[--limit <N>]` | Vec<String> | No | No | Show command history |
| `alias` | `<name> <command>` | bool | No | Yes | Create a command alias |
| `echo` | `<text>` | String | No | No | Echo text to console |

**Example session**:
```
weftos> help
Available namespaces:
  kernel       System-level kernel operations
  process      Agent process lifecycle
  service      Service registry
  chain        ExoChain queries
  ecc          Cognitive substrate (HNSW + causal graph)
  governance   Governance engine
  mesh         Mesh networking
  config       Configuration management
  weaver       Weaver AI assistant
  wasm         WASM tool execution

Type "help <namespace>" for commands in a namespace.

weftos> help process
process.list                List all processes with PID, state
process.spawn <type> [name] Spawn new agent process
process.stop <pid>          Stop agent by PID
process.info <pid>          Detailed process info + capabilities

weftos> alias status kernel.status
Alias created: status -> kernel.status
```

---

## Tab Completion

The console supports context-aware tab completion via the `complete_command` Tauri command. The `CompletionProvider` on the Rust side returns a `Vec<String>` of candidates.

### Completion Sources

| Context | Data Source | Example |
|---------|-------------|---------|
| First token (empty) | Hard-coded namespace list | `kernel`, `process`, `service`, `chain`, `ecc`, `governance`, `mesh`, `config`, `weaver`, `wasm` |
| After namespace `.` | Per-namespace method list | `process.` -> `list`, `spawn`, `stop`, `info` |
| After `process.stop ` | `ProcessTable::list()` PIDs | `1`, `2`, `3`, `4`, `5` |
| After `process.spawn ` | Agent type registry | `coder`, `reviewer`, `tester`, `planner`, `researcher`, `security-auditor`, `security-architect` |
| After `service.health ` | `ServiceRegistry::list()` names | `registry`, `governance`, `mesh-sync`, `chain-manager` |
| After `ecc.search ` | No completion (free text) | -- |
| After `governance.check ` | Action prefix catalog | `tool.exec`, `ipc.send`, `service.register`, `config.set`, `process.spawn` |
| After `config.get ` / `config.set ` | `ConfigService::list()` keys | `tick_interval_ms`, `max_agents`, `governance.risk_threshold` |
| After `wasm.exec ` | `ToolRegistry::list()` names | `code_search`, `file_read`, `file_write`, `browser_nav`, `shell_exec` |

### Completion Protocol

```
Frontend: complete_command("proc") -> ["process."]
Frontend: complete_command("process.") -> ["process.list", "process.spawn", "process.stop", "process.info"]
Frontend: complete_command("process.spawn ") -> ["coder", "reviewer", "tester", "planner", ...]
```

The frontend sends the partial input string; Rust returns candidates. The frontend handles display (dropdown, inline cycling).
