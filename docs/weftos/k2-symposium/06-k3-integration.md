# K3+ Integration Architecture

**Presenter**: Integration Architect
**Scope**: Integration paths from K3-K6 back through K0-K2

---

## Executive Summary

All K3+ subsystems have **complete type definitions** but **lack runtime
implementations**. This is sophisticated planning code that defines
interfaces but doesn't execute. The architecture is excellent; what's
needed is runtime wiring.

## Integration Readiness Matrix

| Subsystem | Types | K0-K2 Integration | Runtime | Overall |
|-----------|-------|-------------------|---------|---------|
| K3 WASM | 100% | 30% | 0% | **25%** |
| K4 Containers | 100% | 20% | 0% | **15%** |
| K5 Apps | 100% | 10% | 0% | **10%** |
| K6 Cluster | 90% | 40% | 5% | **20%** |

## K3: WASM Tool Execution

### Current State

**WasmToolRunner** (wasm_runner.rs, ~530 lines):
- WasmSandboxConfig: fuel_limit, memory_limit_bytes, timeout_ms
- WasmTool: name, module bytes, permissions
- WasmToolResult: output, fuel_consumed, memory_used
- WasmValidation: valid, errors

### Missing Integration Points

1. **ToolRegistry**: No WasmToolRunner -> ToolRegistry binding
2. **Agent Loop**: No "exec" dispatch to WASM backend
3. **Capability Check**: No gate.check() before WASM execution
4. **Chain Logging**: No tool.wasm.exec / tool.wasm.error events
5. **ResourceUsage**: No fuel tracking in process ResourceUsage
6. **Cron Integration**: No scheduled WASM tool runs

### Data Flow (Proposed)

```
Agent receives {cmd: "exec", tool: "my-tool", args: {...}}
  |
  v
agent_loop extracts tool name
  |
  v
GovernanceGate.check("tool.exec", {tool, fuel_budget})
  |-- Deny --> error reply
  |-- Permit -->
  v
ToolRegistry.get("my-tool")
  |
  v
WasmToolRunner.execute(module, args, sandbox_config)
  |-- Wasmtime instantiate module
  |-- Set fuel limit
  |-- Call entry point
  |-- Collect output + fuel_consumed
  |
  v
chain.append("wasm_runner", "tool.wasm.exec", {tool, fuel, memory})
  |
  v
Reply to agent with WasmToolResult
```

### Risk: MEDIUM

Core WASM execution is isolated and safe (Wasmtime provides sandboxing).
Missing integration means WASM tools run WITHOUT governance oversight
and WITHOUT chain audit -- both fixable.

## K4: Container Management

### Current State

**ContainerManager** (container.rs, ~600 lines):
- ContainerConfig: image, ports, volumes, env, restart policy
- ManagedContainer: id, config, state
- ContainerState: Created, Running, Stopped, Failed
- Health check integration point defined

### Missing Integration Points

1. **Docker Runtime**: start_container() returns DockerNotAvailable
2. **ServiceRegistry**: No ContainerService wrapper
3. **HealthSystem**: Container health not aggregated
4. **ChainManager**: No lifecycle events (container.start/stop/failed)
5. **ProcessTable**: No tracking of container-spawned agents
6. **NetworkConfig**: Containers use hardcoded network, not env-scoped

### Data Flow (Proposed)

```
AppManager::start("my-app")
  |-- reads AppManifest
  |-- finds ServiceSpec entries with .image
  |
  v
ContainerManager::start_container(spec)
  |-- [MISSING] bollard: pull image
  |-- [MISSING] bollard: create container
  |-- [MISSING] bollard: map ports, volumes, env
  |-- [MISSING] bollard: start container
  |
  v
ServiceRegistry::register("my-app/redis", ContainerService)
  |-- [MISSING] ContainerService wrapper
  |
  v
chain.append("container", "container.start", {image, ports})
  |
  v
HealthSystem tracks container health
```

### Risk: HIGH

Docker runtime NOT implemented (critical gap). Container health not
monitored (blind spot). Port conflicts not detected.

## K5: Application Framework

### Current State

**AppManager** (app.rs, ~980 lines):
- AppManifest: agents, tools, services, capabilities, hooks
- InstalledApp: manifest, state, agent_pids, service_names
- AppState: Installed -> Starting -> Running -> Stopping -> Stopped
- validate_manifest(): name uniqueness, format checks

### Missing Integration Points

1. **Supervisor**: AppManager doesn't call spawn_and_run() for agents
2. **WasmToolRunner**: AppManager doesn't load WASM tools
3. **ContainerManager**: AppManager doesn't start service containers
4. **ServiceRegistry**: App services not registered
5. **TreeManager**: No /apps/{name} tree nodes
6. **GovernanceGate**: App capabilities not validated before start

### Data Flow (Proposed)

```
AppManager::install("/path/to/app")
  |-- read weftapp.toml
  |-- validate_manifest()
  |-- copy to install directory
  |-- state = Installed
  |
  v
AppManager::start("my-app")
  |-- [MISSING] GovernanceGate.check("app.start", capabilities)
  |
  |-- for each AgentSpec:
  |     [MISSING] Supervisor.spawn_and_run(agent_spec)
  |     record PID in app.agent_pids
  |
  |-- for each ToolSpec (WASM):
  |     [MISSING] WasmToolRunner.load(tool_path)
  |     register in ToolRegistry
  |
  |-- for each ServiceSpec (container):
  |     [MISSING] ContainerManager.start_container(service_spec)
  |     register in ServiceRegistry
  |
  |-- [MISSING] TreeManager.insert("/apps/my-app", metadata)
  |-- state = Running
```

### Risk: CRITICAL

App framework is a skeleton with NO execution runtime. All integration
points are stubs. This is K5 planning code, not K5 implementation.

## K6: Cluster & Networking

### Current State

**ClusterMembership** (cluster.rs, ~460 lines):
- PeerNode with platform labels (CloudNative, Edge, Browser, Wasi)
- NodeState lifecycle (Joining -> Active -> Suspect -> Left)
- ClusterService wraps ruvector ClusterManager (feature-gated)

### Missing Integration Points

1. **Network Transport**: No TCP/WebSocket listener
2. **Remote PID Routing**: A2ARouter is local-only
3. **Chain Replication**: ChainManager is local-only
4. **Tree Sync**: TreeManager has no Merkle replication
5. **Environment Sync**: Each node has independent state
6. **Message Serialization**: No network wire format

### Architecture Gap

```
Current:
  Kernel (Node A)
    A2ARouter (local inboxes)
    ChainManager (local file)
    TreeManager (local Merkle)
    ClusterMembership (peer tracker, no networking)

Needed:
  Kernel (Node A) <--network transport--> Kernel (Node B)
    RemoteRouter     <--serialize/send-->   RemoteRouter
    ChainReplicator  <--consensus-->        ChainReplicator
    TreeSync         <--Merkle diff-->      TreeSync
```

### Risk: VERY HIGH

Universal ClusterMembership tracks peers but can't route messages.
Chain and tree are local-only. No Byzantine fault tolerance.

## Daemon as Integration Hub

### Current Boot Sequence (daemon.rs)

```
1.  ProcessTable
2.  ServiceRegistry
3.  AppContext (MessageBus)
4.  KernelIpc
5.  HealthSystem
6.  Register kernel process (PID 0)
7.  A2ARouter + TopicRouter
8.  CronService
9.  ClusterMembership
10. [cluster] ClusterService (ruvector)
11. [exochain] ChainManager + TreeManager
12. Start all services
13. Transition to Running
```

### Proposed K3+ Boot Additions

```
6.5.  Create ToolRegistry
      - Register built-in tools
      - [wasm-sandbox] Create WasmToolRunner
      - Load WASM tools

7.5.  [containers] Create ContainerManager
      - Connect to Docker socket
      - Scan for managed containers

9.5.  Create AppManager
      - Scan app directory for weftapp.toml
      - Parse manifests
      - Register installed apps

11.5. Start K3+ subsystems
      - AppManager::start_all_auto_start()
      - ContainerManager health checks
```

### Missing RPC Methods

Current RPC dispatch handles kernel, cluster, chain, resource, agent,
cron, and ipc commands. K3+ needs:

```
wasm.load, wasm.list, wasm.execute, wasm.validate
container.start, container.stop, container.list, container.logs
app.install, app.start, app.stop, app.list, app.inspect, app.remove
```

## Cross-Cutting: Governance Extension

### New Action Types for K3+

| Action | Branch | Severity | Risk Profile |
|--------|--------|----------|--------------|
| tool.wasm.load | Judicial | Warning | risk: 0.3, novelty: 0.6 |
| tool.wasm.exec | Judicial | Blocking | risk: fuel/10M, security: 0.2 |
| container.start | Judicial | Blocking | risk: 0.5 (0.9 if privileged) |
| app.install | Executive | Warning | risk: 0.4, novelty: 0.7 |
| app.start | Judicial | Blocking | risk: based on capabilities |
| cluster.join | Judicial | Blocking | risk: 0.6, novelty: 0.8 |

### Environment Scoping for K3+

```
Development:
  wasm.max_fuel = 10M (permissive)
  containers.privileged = allowed
  apps.require_signature = false

Production:
  wasm.max_fuel = 100K (strict)
  containers.allowed_images = ["redis:7", "postgres:14"]
  apps.auto_start = false (manual review)
```

## Recommended Implementation Order

1. **K3 WASM** (easiest, fewest dependencies)
   - Wasmtime integration
   - ToolRegistry wiring
   - Gate checks + chain logging

2. **K4 Containers** (enables K5 sidecars)
   - bollard Docker API
   - ContainerService wrapper
   - Health integration

3. **K6 Cluster** (foundational for distributed apps)
   - Network transport layer
   - Remote PID routing
   - Chain replication

4. **K5 Apps** (depends on K3 + K4 + optionally K6)
   - Supervisor wiring
   - Tool loading
   - Service orchestration
   - Governance validation

## SPARC Planning vs Implementation

| Phase | SPARC Spec | Implementation | Gap |
|-------|-----------|----------------|-----|
| K3 | Exists | 80% types, 0% runtime | Wasmtime stub |
| K4 | Exists | 40% types, 0% runtime | Docker stub |
| K5 | Exists | 30% types, 0% runtime | No orchestration |
| K6 | Not found | 25% types, 5% runtime | No SPARC spec |

**Observation**: K6 has implementation but no SPARC planning doc.
Consider writing a SPARC spec for cluster/networking before
proceeding with K6 implementation.
