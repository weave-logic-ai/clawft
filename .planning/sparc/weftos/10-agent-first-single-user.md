# WeftOS Kernel: Agent-First Architecture & Single-User Boot

```
ID:          W-KERNEL-10
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Agent-First Architecture & Single-User Boot
Status:      Proposed
Date:        2026-02-28
Depends-On:  00-orchestrator.md, 08-ephemeral-os-architecture.md, 09-environments-and-learning.md
Supersedes:  Modifies build order and agent model for K0-K5
```

---

## 1. Core Insight: Agents ARE the OS

Every feature of WeftOS is delivered by an agent. The kernel is a thin
orchestration layer that boots a root agent, which has **agency** -- the
ability to spawn, manage, and communicate with other agents.

### Previous Model (Process-Centric)
```
Kernel → boots services → services run agents → agents do work
```

### New Model (Agent-First)
```
Kernel → boots root agent → root agent spawns service agents → agents ARE services
```

**Key principles:**

1. **Every node has a root agent** -- the first agent spawned during boot, with
   superuser/root capabilities. This is "user 1" in single-user mode.

2. **Services are agents** -- each service (cron, message bus, memory, API, etc.)
   is defined by an agent file and runs as an agent process. Communication to
   services goes through their agent interface.

3. **Agency = ability to spawn agents** -- any agent with the `spawn` capability
   can bring in other agents. The root agent has unlimited agency. Service agents
   may have scoped agency (e.g., the cron agent can spawn scheduled task agents).

4. **Agent files define services** -- a `.agent.toml` manifest describes what an
   agent does, what capabilities it needs, and how to communicate with it. This
   is analogous to systemd unit files but for AI agents.

5. **Single-user mode first** -- boot with user 1 (root). RBAC is about agent
   permissions, not user permissions. Multi-user comes later.

6. **Modular by design** -- every kernel feature is an agent that can be
   independently tested, replaced, or upgraded.

---

## 2. Agent File Manifest

Every service agent is defined by an `.agent.toml` file:

```toml
# .agents/memory-service.agent.toml

[agent]
name = "memory-service"
version = "0.1.0"
description = "Persistent memory storage and retrieval service"
role = "service"           # service | worker | supervisor | user

[capabilities]
# What this agent is allowed to do
tools = ["memory_store", "memory_retrieve", "memory_search", "memory_list"]
ipc_scope = "topic"        # none | explicit | topic | all
topics_publish = ["memory.stored", "memory.deleted"]
topics_subscribe = ["memory.request"]
can_spawn = false           # this service doesn't spawn sub-agents
filesystem_access = ["data/memory/"]  # scoped filesystem access

[resources]
max_memory_mb = 256
max_concurrent_requests = 100
priority = "high"           # low | normal | high | critical

[interface]
# How other agents communicate with this service
protocol = "ipc"            # ipc | rest | grpc | mcp
request_topic = "memory.request"
response_mode = "direct"    # direct (reply to sender) | broadcast | topic

[health]
check_interval = "30s"
timeout = "5s"
restart_policy = "always"   # never | on-failure | always
max_restarts = 5

[dependencies]
# Other agents that must be running before this one starts
requires = []               # no deps -- this is a core service
after = ["kernel-init"]     # start after kernel initialization
```

### Agent Roles

| Role | Description | Agency | Example |
|------|-------------|--------|---------|
| `root` | Superuser agent, user 1, unlimited capabilities | Unlimited | The boot agent |
| `supervisor` | Manages lifecycle of other agents | Can spawn/stop/restart managed agents | Agent runner |
| `service` | Provides a capability to other agents | Scoped (may spawn workers) | Memory, Cron, API |
| `worker` | Performs specific tasks, spawned by others | None (leaf agent) | Scheduled task, tool executor |
| `user` | Represents a human or external system | Per-user RBAC (future) | Human operator |

---

## 3. Single-User Mode

### 3.1 Boot Sequence

```
1. Kernel init
   - Load KernelConfig
   - Initialize process table (empty)
   - Initialize IPC bus

2. Spawn root agent (user 1)
   - PID 0
   - Role: root
   - Capabilities: ALL (superuser)
   - Agency: unlimited
   - Identity: did:weft:root (local DID, not exochain -- single node)

3. Root agent reads .agents/ directory
   - Discovers service agent files
   - Sorts by dependency order (topological sort on `requires` + `after`)
   - Spawns service agents in order

4. Core services come up
   - PID 1: message-bus.agent.toml     (IPC backbone)
   - PID 2: memory-service.agent.toml  (persistent storage)
   - PID 3: tool-registry.agent.toml   (tool discovery)
   - PID 4: cron-service.agent.toml    (scheduled tasks)
   - PID 5: health-monitor.agent.toml  (watchdog)

5. Optional services (if configured)
   - PID 6: api-server.agent.toml      (HTTP/WS API)
   - PID 7: plugin-host.agent.toml     (channel plugins)
   - PID 8+: user-defined agents

6. Kernel reports ready
   - All required services healthy
   - Root agent enters interactive mode (or runs configured task)
```

### 3.2 Root Agent

```rust
pub struct RootAgent {
    /// Always PID 0
    pub pid: Pid,

    /// User 1 -- the superuser
    pub user_id: UserId, // UserId(1)

    /// Root has ALL capabilities
    pub capabilities: AgentCapabilities, // AgentCapabilities::root()

    /// Unlimited agency -- can spawn any agent
    pub agency: Agency,

    /// Manages the service agent lifecycle
    pub service_manager: ServiceManager,
}

pub struct Agency {
    /// Maximum number of agents this agent can spawn
    pub max_children: Option<usize>, // None = unlimited

    /// What roles this agent can spawn
    pub allowed_roles: Vec<AgentRole>, // root can spawn any role

    /// What capabilities spawned agents can have (ceiling)
    pub capability_ceiling: AgentCapabilities,

    /// Current children
    pub children: Vec<Pid>,
}

impl Agency {
    /// Root agent has unlimited agency
    pub fn root() -> Self {
        Agency {
            max_children: None,
            allowed_roles: vec![
                AgentRole::Supervisor,
                AgentRole::Service,
                AgentRole::Worker,
                AgentRole::User,
            ],
            capability_ceiling: AgentCapabilities::root(),
            children: vec![],
        }
    }

    /// Service agent has scoped agency
    pub fn service(max_workers: usize) -> Self {
        Agency {
            max_children: Some(max_workers),
            allowed_roles: vec![AgentRole::Worker],
            capability_ceiling: AgentCapabilities::default(),
            children: vec![],
        }
    }

    /// Worker agent has no agency
    pub fn none() -> Self {
        Agency {
            max_children: Some(0),
            allowed_roles: vec![],
            capability_ceiling: AgentCapabilities::default(),
            children: vec![],
        }
    }
}
```

### 3.3 User Model (Single-User to Multi-User)

```rust
pub struct UserId(pub u64);

pub struct User {
    pub id: UserId,
    pub name: String,
    pub role: UserRole,
    pub capabilities: AgentCapabilities,
    pub agency: Agency,
}

pub enum UserRole {
    /// User 1: full system access, all capabilities
    Root,
    /// Future: regular user with scoped permissions
    Regular,
    /// Future: service account (non-interactive)
    Service,
    /// Future: guest with minimal permissions
    Guest,
}

impl User {
    /// Single-user mode: create user 1 with root privileges
    pub fn root() -> Self {
        User {
            id: UserId(1),
            name: "root".to_string(),
            role: UserRole::Root,
            capabilities: AgentCapabilities::root(),
            agency: Agency::root(),
        }
    }
}
```

**Single-user mode**: Only user 1 exists. All agents run as user 1. RBAC is
about what individual agents can do (their capabilities from `.agent.toml`),
not about which user they belong to.

**Future multi-user mode** (post-K5):
- Users 2+ can be created with scoped capabilities
- Agents belong to a user and inherit the user's capability ceiling
- An agent can never exceed its user's capabilities
- User 1 remains root/superuser

---

## 4. Services as Agents

### 4.1 Core Service Agents

These are the kernel's built-in services, each defined by an agent file:

| Agent | PID | Purpose | Agency | Key Tools |
|-------|-----|---------|--------|-----------|
| `root` | 0 | Superuser, boot orchestrator | Unlimited | all |
| `message-bus` | 1 | IPC backbone, topic routing | None | `ipc_send`, `ipc_subscribe` |
| `memory-service` | 2 | Persistent memory storage | None | `memory_*` |
| `tool-registry` | 3 | Tool discovery and routing | None | `tool_register`, `tool_lookup` |
| `cron-service` | 4 | Scheduled task execution | Workers (spawn task agents) | `cron_schedule`, `cron_list` |
| `health-monitor` | 5 | Watchdog, health checks | None | `health_check`, `health_report` |

### 4.2 Optional Service Agents

| Agent | Purpose | Agency | Enabled By |
|-------|---------|--------|------------|
| `api-server` | HTTP/WebSocket API | None | `[services.api]` in config |
| `plugin-host` | Channel plugin management | Workers (plugin agents) | `[services.plugins]` |
| `voice-service` | Voice/TTS/STT processing | Workers (voice sessions) | `[services.voice]` |
| `delegation-service` | Multi-agent task delegation | Workers (delegated agents) | `[services.delegation]` |
| `sandbox-service` | WASM tool execution | Workers (sandboxed executors) | `[services.sandbox]` |

### 4.3 Communication Through Agents

All inter-service communication goes through agent IPC, not direct function calls:

```
# Old way (direct function call):
memory_store.store("key", value).await?;

# New way (agent IPC):
ipc.send(
    MessageTarget::Service("memory-service"),
    MessagePayload::ToolCall {
        name: "memory_store",
        args: json!({"key": "key", "value": value}),
    },
).await?;

# Or via topic:
ipc.publish(
    "memory.request",
    MemoryRequest::Store { key: "key", value },
).await?;
```

**Why?**
- Services can be replaced without changing callers
- Services can be remote (on another node) transparently
- Every interaction is auditable (IPC messages are logged)
- Services can be tested in isolation
- Agency boundaries are enforced (only agents with IPC access can call services)

### 4.4 Agent Lifecycle

```
             spawn
    ┌─────────────────┐
    │                 v
    │          ┌──────────┐
    │          │ Starting │
    │          └────┬─────┘
    │               │ init complete
    │               v
    │          ┌──────────┐
    │    ┌─────│ Running  │──────┐
    │    │     └──────────┘      │
    │    │ suspend          │ error (if restart_policy)
    │    v                  │
    │ ┌──────────┐          │
    │ │Suspended │          │
    │ └────┬─────┘          │
    │      │ resume         │
    │      └────────────────┤
    │                       │
    │               stop    │
    │               ┌───────┘
    │               v
    │          ┌──────────┐
    │          │ Stopping │
    │          └────┬─────┘
    │               │ cleanup complete
    │               v
    │          ┌──────────┐
    └──────────│  Exited  │ (restart if policy says so)
               └──────────┘
```

---

## 5. Revised Build Order

The agent-first model changes the implementation sequence:

### Phase K0: Single-User Kernel Boot (2 weeks)

**Goal**: Kernel boots, spawns root agent (user 1), root agent spawns core
service agents from `.agent.toml` files.

**What gets built**:
1. `KernelConfig` loading
2. Process table with PID allocation
3. Basic IPC (in-process message passing)
4. `.agent.toml` parser
5. Root agent (PID 0, user 1, superuser)
6. Service agent spawner (reads `.agents/` directory, topological sort)
7. `message-bus` service agent (PID 1)
8. `health-monitor` service agent (PID 5)
9. CLI: `weave kernel boot`, `weave kernel status`, `weave kernel ps`

**Test**: Boot kernel, verify root agent spawns, message-bus and health-monitor
come up, `weave kernel ps` shows 3 agents (root, message-bus, health-monitor).

### Phase K0.5: Core Service Agents (1 week)

**Goal**: Remaining core services come up as agents.

**What gets built**:
1. `memory-service` agent (wraps existing MemoryStore)
2. `tool-registry` agent (wraps existing ToolRegistry)
3. `cron-service` agent (wraps existing CronService)
4. Service health checks via health-monitor
5. CLI: `weave kernel services`

**Test**: All 6 agents running. Memory store/retrieve via IPC. Cron schedules
via IPC. Tool lookup via IPC. Health check reports all green.

### Phase K1: Agent Runner & Permissions (2 weeks)

**Goal**: Agent supervisor that manages lifecycle. Agent permissions enforced
via capabilities from `.agent.toml`.

**What gets built**:
1. `Agency` struct -- spawn limits, role limits, capability ceiling
2. `AgentCapabilities::root()` for user 1
3. Capability enforcement on tool calls
4. IPC scope enforcement (agents can only IPC with allowed targets)
5. Agent spawn/stop/restart via supervisor
6. Resource limit enforcement (memory, CPU, concurrent requests)
7. CLI: `weft agent spawn <agent.toml>`, `weft agent stop <pid>`,
   `weft agent restart <pid>`, `weft agent inspect <pid>`

**RBAC scope**: This is about agent permissions, not user permissions. Every
agent runs as user 1 (root) but has its own capability set from `.agent.toml`.
The root agent can do anything; service agents are scoped.

**Test**: Spawn agent with restricted capabilities. Verify tool calls outside
capabilities are denied. Verify IPC outside scope is denied. Verify resource
limits are enforced.

### Phase K2: Agent-to-Agent Communication (2 weeks)

**Goal**: Full A2A protocol. Agents communicate via IPC topics, direct
messages, and request/response patterns.

**What gets built**:
1. Topic-based pub/sub (agents subscribe to topics, publish messages)
2. Direct messaging (agent-to-agent by PID or service name)
3. Request/response pattern (send request, get correlated response)
4. Service discovery (find agent by name or role)
5. Message routing (root agent can route between agents)
6. Wire format for messages (JSON initially, rvf-wire later)
7. CLI: `weft ipc send <target> <message>`, `weft ipc topics`,
   `weft ipc subscribe <topic>`

**Test**: Two agents exchange messages. Topic pub/sub works. Request/response
returns correlated result. IPC scope enforcement prevents unauthorized messaging.

### Phase K3: WASM Tool Sandbox (2 weeks)

**Goal**: Tools can run in WASM sandboxes. The `sandbox-service` agent manages
WASM execution.

**What gets built**:
1. `sandbox-service` agent (manages Wasmtime instances)
2. WASM tool loading and execution
3. Fuel metering (epoch-based budget)
4. Memory limits
5. Host function interface (WASM tools call back to kernel)
6. Agent can request sandboxed tool execution via IPC to sandbox-service

**Test**: Tool executes in WASM sandbox. Fuel exhaustion terminates cleanly.
Memory limit prevents allocation bomb. Host filesystem not accessible.

### Phase K4: Container/Service Orchestration (1 week)

**Goal**: External services (databases, APIs, etc.) can be managed as agents
via container sidecar pattern.

**What gets built**:
1. Container agent type (wraps Docker/Podman container as agent interface)
2. Sidecar service management (start/stop external services with kernel)
3. Container health checks feed into health-monitor
4. CLI: `weft service start <name>`, `weft service stop <name>`

**Test**: Container service starts/stops with kernel. Health checks propagate.
Agent IPC can communicate with container service.

### Phase K5: Application Framework (2 weeks)

**Goal**: Applications are collections of agents defined by `weftapp.toml`.
Install, start, stop applications as units.

**What gets built**:
1. `weftapp.toml` parser (app manifest)
2. Application as agent group (multiple agents working together)
3. App lifecycle (install -> start -> stop -> uninstall)
4. App-level capabilities (all agents in an app share a capability ceiling)
5. App-level IPC namespace (agents within an app can freely communicate)
6. CLI: `weft app install <path>`, `weft app start <name>`,
   `weft app stop <name>`, `weft app list`

**Test**: Install app. App agents spawn with correct capabilities. Agents
communicate within app namespace. Stop app cleanly shuts down all agents.

---

## 6. Agent File Directory Structure

```
.agents/                              # System service agents
  root.agent.toml                     # Root agent (auto-spawned by kernel)
  message-bus.agent.toml              # IPC backbone
  memory-service.agent.toml           # Persistent storage
  tool-registry.agent.toml            # Tool discovery
  cron-service.agent.toml             # Scheduled tasks
  health-monitor.agent.toml           # Watchdog
  api-server.agent.toml               # HTTP/WS API (optional)
  plugin-host.agent.toml              # Channel plugins (optional)
  sandbox-service.agent.toml          # WASM sandbox (optional)
  delegation-service.agent.toml       # Task delegation (optional)
  voice-service.agent.toml            # Voice processing (optional)

apps/                                 # Installed applications
  my-app/
    weftapp.toml                      # Application manifest
    agents/
      coordinator.agent.toml          # App's coordinator agent
      worker.agent.toml               # App's worker agent(s)
```

---

## 7. Key Types

### 7.1 Agent Process Entry (Updated)

```rust
pub struct ProcessEntry {
    pub pid: Pid,
    pub agent_id: String,
    pub state: ProcessState,

    /// User this agent belongs to (user 1 in single-user mode)
    pub user: UserId,

    /// Capabilities from .agent.toml
    pub capabilities: AgentCapabilities,

    /// Agency -- ability to spawn other agents
    pub agency: Agency,

    /// Role (root, supervisor, service, worker, user)
    pub role: AgentRole,

    /// Parent agent that spawned this one
    pub parent_pid: Option<Pid>,

    /// Resource usage tracking
    pub resource_usage: ResourceUsage,

    /// Cancellation token for clean shutdown
    pub cancel_token: CancellationToken,

    /// Source manifest file
    pub manifest: Option<PathBuf>,

    /// Health check configuration
    pub health: HealthConfig,

    /// Dependencies (other agents that must be running)
    pub requires: Vec<String>,
}
```

### 7.2 AgentCapabilities (Updated)

```rust
pub struct AgentCapabilities {
    /// Tools this agent is allowed to invoke
    pub tools: ToolScope,

    /// IPC communication scope
    pub ipc_scope: IpcScope,

    /// Filesystem access scope
    pub filesystem: FilesystemScope,

    /// Resource limits
    pub resource_limits: ResourceLimits,

    /// Whether this agent can spawn sub-agents
    pub agency: Agency,

    /// Sandbox policy for tool execution
    pub sandbox: SandboxPolicy,
}

pub enum ToolScope {
    /// Can use any tool (root only)
    All,
    /// Can use specific tools
    Explicit(Vec<String>),
    /// Can use tools matching patterns
    Pattern(Vec<String>), // e.g., "memory_*", "cron_*"
    /// No tools
    None,
}

pub enum FilesystemScope {
    /// Full filesystem access (root only)
    Full,
    /// Access to specific directories
    Scoped(Vec<PathBuf>),
    /// No filesystem access
    None,
}

impl AgentCapabilities {
    /// Root agent has all capabilities
    pub fn root() -> Self {
        AgentCapabilities {
            tools: ToolScope::All,
            ipc_scope: IpcScope::All,
            filesystem: FilesystemScope::Full,
            resource_limits: ResourceLimits::unlimited(),
            agency: Agency::root(),
            sandbox: SandboxPolicy::None, // root is not sandboxed
        }
    }

    /// Default service agent capabilities (restrictive)
    pub fn service() -> Self {
        AgentCapabilities {
            tools: ToolScope::None,       // must be explicitly granted
            ipc_scope: IpcScope::None,    // must be explicitly granted
            filesystem: FilesystemScope::None,
            resource_limits: ResourceLimits::default(),
            agency: Agency::none(),
            sandbox: SandboxPolicy::Strict,
        }
    }
}
```

### 7.3 Service Manager

```rust
/// Managed by the root agent to orchestrate service agents
pub struct ServiceManager {
    /// Agent manifest directory
    agent_dir: PathBuf,

    /// Boot order (topologically sorted)
    boot_order: Vec<AgentManifest>,

    /// Running service agents
    running: DashMap<String, Pid>,

    /// Reference to kernel process table
    process_table: Arc<ProcessTable>,
}

impl ServiceManager {
    /// Discover and sort agent manifests
    pub fn discover(&mut self) -> Result<Vec<AgentManifest>> {
        // 1. Read all .agent.toml files from agent_dir
        // 2. Parse into AgentManifest structs
        // 3. Build dependency graph from `requires` and `after` fields
        // 4. Topological sort
        // 5. Return sorted list
    }

    /// Boot all discovered service agents in order
    pub async fn boot_services(&mut self) -> Result<()> {
        for manifest in &self.boot_order {
            let pid = self.spawn_service(manifest).await?;
            self.running.insert(manifest.name.clone(), pid);

            // Wait for health check before proceeding to next
            self.wait_healthy(pid, manifest.health.timeout).await?;
        }
        Ok(())
    }

    /// Spawn a single service agent
    async fn spawn_service(&self, manifest: &AgentManifest) -> Result<Pid> {
        let entry = ProcessEntry {
            pid: self.process_table.allocate_pid(),
            agent_id: manifest.name.clone(),
            state: ProcessState::Starting,
            user: UserId(1), // single-user mode: always user 1
            capabilities: manifest.to_capabilities(),
            agency: manifest.to_agency(),
            role: manifest.role.clone(),
            parent_pid: Some(Pid(0)), // root agent is parent
            manifest: Some(manifest.path.clone()),
            health: manifest.health.clone(),
            requires: manifest.requires.clone(),
            ..Default::default()
        };

        self.process_table.insert(entry)
    }
}
```

---

## 8. Impact on Existing K0-K5 Plans

### Changes to K0 (Kernel Foundation)

| Original | Agent-First |
|----------|-------------|
| Boot sequence initializes services directly | Boot spawns root agent; root agent spawns services |
| `ServiceRegistry` holds service handles | Process table holds agent entries; services ARE agents |
| Services accessed by direct reference | Services accessed via IPC to service agent |
| No user concept | User 1 (root) created at boot |
| Process table is flat | Process table has parent-child (agency tree) |

### Changes to K1 (Supervisor + RBAC)

| Original | Agent-First |
|----------|-------------|
| Supervisor manages agents | Supervisor IS an agent (the root agent, or a delegated supervisor agent) |
| RBAC = per-agent capabilities | RBAC = agent capabilities from `.agent.toml` + user ceiling |
| Capabilities are runtime-configured | Capabilities are declared in agent manifest |
| No user concept | User 1 runs all agents; per-agent caps still enforced |

### Changes to K2 (A2A IPC)

| Original | Agent-First |
|----------|-------------|
| IPC between agent processes | IPC between agent processes (unchanged, but now ALL communication is IPC) |
| Some services called directly | ALL services called via IPC (no direct function calls) |
| Message routing by PID | Message routing by PID, service name, or topic |

### Changes to K3 (WASM Sandbox)

| Original | Agent-First |
|----------|-------------|
| WASM runner is a kernel module | WASM runner IS the `sandbox-service` agent |
| Tools execute in WASM directly | Agents request sandboxed execution via IPC to sandbox-service |

### Changes to K4 (Containers)

| Original | Agent-First |
|----------|-------------|
| Container management module | Container management IS an agent (`container-service`) |
| Sidecar services are separate | Sidecars wrapped as agent interfaces |

### Changes to K5 (App Framework)

| Original | Agent-First |
|----------|-------------|
| Apps are manifests + lifecycle | Apps are manifests + agent groups (unchanged but more natural) |
| App agents are spawned by kernel | App agents are spawned by root agent via app manifest |

---

## 9. CLI Commands (Updated)

### Boot & Status
```
weave kernel boot                      -- boot kernel in single-user mode
weave kernel status                    -- kernel state, user, uptime
weave kernel ps                        -- process table (all agents)
weave kernel ps --tree                 -- process tree (parent-child agency)
weave kernel services                  -- list service agents with health
weave kernel shutdown                  -- graceful shutdown (stop all agents)
```

### Agent Management
```
weft agent spawn <agent.toml>         -- spawn agent from manifest
weft agent spawn --inline <name>      -- spawn ephemeral agent (no manifest)
weft agent stop <pid|name>            -- stop agent
weft agent restart <pid|name>         -- restart agent
weft agent inspect <pid|name>         -- show agent details (capabilities, agency, health)
weft agent logs <pid|name>            -- show agent IPC log
weft agent list --role service        -- list agents by role
```

### IPC
```
weft ipc send <target> <message>      -- send message to agent (by pid or name)
weft ipc call <service> <tool> <args> -- call service agent tool (request/response)
weft ipc topics                       -- list active topics
weft ipc subscribe <topic>            -- subscribe to topic (stream output)
weft ipc publish <topic> <message>    -- publish to topic
```

### Services
```
weft service list                     -- list all service agents
weft service start <name>             -- start a stopped service agent
weft service stop <name>              -- stop a service agent
weft service restart <name>           -- restart a service agent
weft service health                   -- health check all services
```

---

## 10. Agent-to-Agent Interaction Patterns

### 10.1 Request/Response (Tool Call)

```
[Engineering Agent] ──ToolCall("memory_store", {...})──> [Memory Service Agent]
                    <──ToolResult({ok: true})──────────
```

### 10.2 Pub/Sub (Events)

```
[Cron Service] ──publish("cron.fired", {job: "backup"})──> [Topic Router]
                                                              │
                                    ┌─────────────────────────┤
                                    v                         v
                            [Backup Worker]           [Monitor Agent]
                            (subscribed to             (subscribed to
                             "cron.fired")              "cron.*")
```

### 10.3 Agency Chain (Spawn Delegation)

```
[Root Agent (PID 0)]
    │
    ├── spawns → [Cron Service (PID 4)]
    │                 │
    │                 ├── spawns → [Backup Worker (PID 8)]   (agency: workers only)
    │                 └── spawns → [Cleanup Worker (PID 9)]
    │
    ├── spawns → [Delegation Service (PID 10)]
    │                 │
    │                 └── spawns → [Research Agent (PID 11)] (agency: workers only)
    │
    └── spawns → [App: my-app coordinator (PID 12)]
                      │
                      └── spawns → [App: my-app worker (PID 13)]
```

### 10.4 Health Monitoring

```
[Health Monitor (PID 5)]
    │
    ├── checks → [Message Bus (PID 1)]     ✓ healthy
    ├── checks → [Memory Service (PID 2)]  ✓ healthy
    ├── checks → [Tool Registry (PID 3)]   ✓ healthy
    ├── checks → [Cron Service (PID 4)]    ✗ unhealthy
    │               │
    │               └── notify → [Root Agent (PID 0)]
    │                               │
    │                               └── restart → [Cron Service (PID 4')]  (new PID)
    │
    └── publish("health.report", {...}) → [Topic: health.report]
```

---

## 11. Testing Strategy

### K0 Unit Tests

| Test | Description |
|------|-------------|
| `agent_manifest_parse` | Parse `.agent.toml` into `AgentManifest` struct |
| `root_agent_capabilities` | Root agent has `AgentCapabilities::root()` |
| `agency_root_unlimited` | Root agency can spawn any role |
| `agency_service_scoped` | Service agency can only spawn workers |
| `agency_worker_none` | Worker agency cannot spawn |
| `service_discovery` | Discover all `.agent.toml` files in directory |
| `service_sort_dependencies` | Topological sort respects `requires` and `after` |
| `boot_sequence_order` | Boot spawns agents in dependency order |
| `user_root_creation` | User 1 (root) created at boot with all privileges |

### K0 Integration Tests

| Test | Description |
|------|-------------|
| `single_user_boot` | Kernel boots, root agent spawns, services come up |
| `service_ipc` | Service agents communicate via IPC |
| `health_monitoring` | Health monitor detects unhealthy service, root restarts |
| `agency_enforcement` | Worker agent cannot spawn sub-agents |
| `capability_enforcement` | Agent cannot use tools outside its manifest |

---

## 12. Mapping to Existing Code

| Existing Code | Agent-First Usage |
|---------------|-------------------|
| `AppContext::new()` in `bootstrap.rs` | Becomes kernel init (pre-agent) |
| `MessageBus` in `bus.rs` | Becomes `message-bus.agent.toml` service |
| `AgentLoop` in `loop_core.rs` | Becomes the core of every agent process |
| `SandboxEnforcer` in `sandbox.rs` | Becomes `sandbox-service.agent.toml` |
| `ToolRegistry` in `tools/registry.rs` | Becomes `tool-registry.agent.toml` |
| `CronService` in `cron_service/` | Becomes `cron-service.agent.toml` |
| `PluginHost` in `channels/host.rs` | Becomes `plugin-host.agent.toml` |
| `MemoryStore` | Becomes `memory-service.agent.toml` |
| `PermissionResolver` in `permissions.rs` | Used inside capability enforcement |
| `UserPermissions` in `routing.rs` | Informs `AgentCapabilities` |

---

## 13. Risks and Mitigations

| ID | Risk | Severity | Mitigation |
|----|------|----------|------------|
| R1 | IPC overhead for service calls vs direct function calls | Medium | In-process IPC uses channels, not serialization. Benchmark early. |
| R2 | Agent manifest format churn during early development | Low | Start with `.agent.toml`; keep parser behind trait for future formats |
| R3 | Root agent becomes bottleneck as spawn orchestrator | Medium | Root delegates to supervisor agents for app-level spawning |
| R4 | Circular dependencies in agent `requires` graph | Low | Topological sort detects cycles; boot fails with clear error |
| R5 | Single-user mode has no isolation between agents | Low | By design -- capabilities enforce boundaries. Multi-user adds user isolation later. |
| R6 | Agent health check storms under high agent count | Low | Staggered health checks via health-monitor scheduling |

---

## 14. Cross-References

| Document | Relationship |
|----------|-------------|
| `00-orchestrator.md` | Build order updated by this document |
| `01-phase-K0-kernel-foundation.md` | K0 reframed: boot spawns root agent, not services directly |
| `02-phase-K1-supervisor-rbac.md` | K1 reframed: supervisor is an agent, RBAC is agent capabilities |
| `03-phase-K2-a2a-ipc.md` | K2 unchanged but now ALL communication is IPC |
| `04-phase-K3-wasm-sandbox.md` | K3 reframed: sandbox is a service agent |
| `05-phase-K4-containers.md` | K4 reframed: containers wrapped as agents |
| `06-phase-K5-app-framework.md` | K5 natural fit: apps are agent groups |
| `08-ephemeral-os-architecture.md` | Multi-node: each node has its own root agent |
| `09-environments-and-learning.md` | Environments scope agent capabilities per environment |
