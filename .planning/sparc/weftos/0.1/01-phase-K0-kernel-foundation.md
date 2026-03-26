# Phase K0: Kernel Foundation

**Phase ID**: K0
**Workstream**: W-KERNEL
**Duration**: Week 1-2
**Goal**: Create the `clawft-kernel` crate with boot sequence, process table, service registry, and health subsystem

---

## S -- Specification

### What Changes

This phase creates the `clawft-kernel` crate and its foundational modules. The kernel wraps `AppContext<P>` in a boot sequence, introduces a process table for PID-based agent tracking, a service registry for lifecycle management of system services, a health subsystem, and an interactive kernel console. The console provides a TTY-like terminal that streams boot output in real-time and drops into a REPL for running both `weave` (OS) and `weft` (agent) commands -- making it the primary interactive interface for the kernel. CLI commands for kernel introspection (`weave kernel status|services|ps`) and console access (`weave console`, `weave boot`) are added.

No existing behavior changes. The kernel is opt-in: existing `weft` commands continue to work without kernel involvement.

### Files to Create

| File | Purpose |
|---|---|
| `crates/clawft-kernel/Cargo.toml` | Crate manifest with deps on `clawft-core`, `clawft-types`, `clawft-platform` |
| `crates/clawft-kernel/src/lib.rs` | Crate root with re-exports |
| `crates/clawft-kernel/src/boot.rs` | `Kernel<P>` struct, boot sequence, state machine |
| `crates/clawft-kernel/src/process.rs` | `ProcessTable`, `ProcessEntry`, PID allocation |
| `crates/clawft-kernel/src/service.rs` | `SystemService` trait, `ServiceRegistry` |
| `crates/clawft-kernel/src/ipc.rs` | `KernelIpc` (MessageBus wrapper), `KernelMessage` types |
| `crates/clawft-kernel/src/capability.rs` | `AgentCapabilities`, `IpcScope`, `ResourceLimits` |
| `crates/clawft-kernel/src/health.rs` | `HealthSystem`, `HealthStatus`, aggregated checks |
| `crates/clawft-kernel/src/config.rs` | `KernelConfig` (embedded in `ClawftConfig`) |
| `crates/clawft-kernel/src/console.rs` | `KernelConsole` -- interactive terminal with boot output and command REPL |

### Files to Modify

| File | Change |
|---|---|
| `Cargo.toml` (workspace root) | Add `clawft-kernel` to workspace members; add `dashmap` workspace dep |
| `crates/clawft-types/src/config/mod.rs` | Add `KernelConfig` field to `ClawftConfig` |
| `crates/clawft-cli/src/main.rs` | Add `kernel` subcommand dispatch |
| `crates/clawft-cli/src/commands/mod.rs` | Add `kernel` command module |
| `crates/clawft-cli/src/help_text.rs` | Add `kernel` topic help text |

### Key Types

**Kernel** (`boot.rs`):
```rust
pub struct Kernel<P: Platform> {
    state: KernelState,
    app_context: Option<AppContext<P>>,
    process_table: Arc<ProcessTable>,
    service_registry: Arc<ServiceRegistry>,
    ipc: Arc<KernelIpc>,
    health: HealthSystem,
}

pub enum KernelState {
    Booting,
    Running,
    ShuttingDown,
    Halted,
}

impl<P: Platform> Kernel<P> {
    pub async fn boot(config: ClawftConfig, platform: P) -> Result<Self>;
    pub async fn shutdown(&mut self) -> Result<()>;
    pub fn state(&self) -> &KernelState;
    pub fn process_table(&self) -> &Arc<ProcessTable>;
    pub fn services(&self) -> &Arc<ServiceRegistry>;
    pub fn ipc(&self) -> &Arc<KernelIpc>;
    pub fn uptime(&self) -> Duration;
}
```

**ProcessTable** (`process.rs`):
```rust
pub type Pid = u64;

pub struct ProcessTable {
    next_pid: AtomicU64,
    entries: DashMap<Pid, ProcessEntry>,
}

pub struct ProcessEntry {
    pub pid: Pid,
    pub agent_id: String,
    pub state: ProcessState,
    pub capabilities: AgentCapabilities,
    pub resource_usage: ResourceUsage,
    pub cancel_token: CancellationToken,
    pub parent_pid: Option<Pid>,
}

pub enum ProcessState {
    Starting,
    Running,
    Suspended,
    Stopping,
    Exited(i32),
}

pub struct ResourceUsage {
    pub memory_bytes: u64,
    pub cpu_time_ms: u64,
    pub tool_calls: u64,
    pub messages_sent: u64,
}

impl ProcessTable {
    pub fn new() -> Self;
    pub fn allocate_pid(&self) -> Pid;
    pub fn insert(&self, entry: ProcessEntry) -> Pid;
    pub fn get(&self, pid: Pid) -> Option<ProcessEntry>;
    pub fn remove(&self, pid: Pid) -> Option<ProcessEntry>;
    pub fn list(&self) -> Vec<ProcessEntry>;
    pub fn update_state(&self, pid: Pid, state: ProcessState) -> Result<()>;
}
```

**ServiceRegistry** (`service.rs`):
```rust
pub trait SystemService: Send + Sync {
    fn name(&self) -> &str;
    fn service_type(&self) -> ServiceType;
    async fn start(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn health_check(&self) -> HealthStatus;
}

pub enum ServiceType {
    Core,
    Plugin,
    Cron,
    Api,
    Custom(String),
}

pub struct ServiceRegistry {
    services: DashMap<String, Arc<dyn SystemService>>,
}

impl ServiceRegistry {
    pub fn new() -> Self;
    pub fn register(&self, service: Arc<dyn SystemService>) -> Result<()>;
    pub fn unregister(&self, name: &str) -> Option<Arc<dyn SystemService>>;
    pub fn get(&self, name: &str) -> Option<Arc<dyn SystemService>>;
    pub fn list(&self) -> Vec<(String, ServiceType)>;
    pub async fn start_all(&self) -> Result<()>;
    pub async fn stop_all(&self) -> Result<()>;
    pub async fn health_all(&self) -> Vec<(String, HealthStatus)>;
}
```

**KernelConfig** (`config.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_processes")]
    pub max_processes: u32,
    #[serde(default)]
    pub health_check_interval_secs: u64,
    #[serde(default)]
    pub default_capabilities: Option<AgentCapabilities>,
}
```

**KernelConsole** (`console.rs`):
```rust
pub struct KernelConsole {
    kernel: Arc<Kernel<NativePlatform>>,
    boot_log: Arc<RwLock<Vec<BootEvent>>>,
    prompt: String,
    history: Vec<String>,
}

pub struct BootEvent {
    pub timestamp: DateTime<Utc>,
    pub phase: BootPhase,
    pub message: String,
    pub level: LogLevel,
}

pub enum BootPhase {
    Init,            // Pre-boot initialization
    Config,          // Loading configuration
    Services,        // Registering system services
    ResourceTree,    // Loading resource tree from DAG
    Agents,          // Spawning service agents
    Network,         // Network service discovery
    Ready,           // Boot complete, ready for commands
}

pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl KernelConsole {
    pub async fn boot_interactive(config: ClawftConfig, platform: NativePlatform) -> Result<Self>;
    pub async fn attach(kernel: Arc<Kernel<NativePlatform>>) -> Result<Self>;
    pub async fn run_repl(&mut self) -> Result<()>;
    pub fn replay_boot_log(&self);
}
```

### CLI Commands

```
weave kernel status     -- Shows kernel state, uptime, process count, service count
weave kernel services   -- Lists registered services with name, type, health status
weave kernel ps         -- Lists process table entries with PID, agent_id, state, resource usage

weave console                -- Boot kernel and open interactive terminal
weave console --attach       -- Attach to already-running kernel
weave console --replay-boot  -- Show boot log of running kernel
weave boot                   -- Boot kernel as daemon (non-interactive)
weave boot --foreground      -- Boot in foreground with log output (no REPL)
```

---

## P -- Pseudocode

### Boot Sequence

```
fn Kernel::boot(config, platform):
    set state = Booting

    // 1. Create subsystems
    process_table = ProcessTable::new()
    service_registry = ServiceRegistry::new()
    ipc = KernelIpc::new(MessageBus from config)
    health = HealthSystem::new(config.health_check_interval)

    // 2. Create AppContext (reuse existing bootstrap)
    app_context = AppContext::new(config, platform).await

    // 3. Register built-in services
    if app_context has cron_service:
        service_registry.register(CronServiceWrapper(cron_service))

    // 4. Start services
    service_registry.start_all().await

    // 5. Start health monitor
    health.start(service_registry.clone())

    set state = Running
    return Kernel { state, app_context, process_table, service_registry, ipc, health }
```

### Process Table Operations

```
fn ProcessTable::insert(entry):
    pid = next_pid.fetch_add(1)
    entry.pid = pid
    entries.insert(pid, entry)
    return pid

fn ProcessTable::update_state(pid, new_state):
    if entry = entries.get_mut(pid):
        validate_transition(entry.state, new_state)
        entry.state = new_state
    else:
        return Err(ProcessNotFound)
```

### Health Aggregation

```
fn HealthSystem::aggregate():
    results = []
    for service in service_registry.list():
        status = service.health_check().await
        results.push((service.name(), status))

    if all results are Healthy:
        return OverallHealth::Healthy
    else if any result is Unhealthy:
        return OverallHealth::Degraded(unhealthy_services)
    else:
        return OverallHealth::Down
```

### Console Boot Sequence

```
fn KernelConsole::boot_interactive(config, platform):
    boot_log = Vec::new()

    // Phase: Init
    emit(BootEvent { phase: Init, message: "WeftOS v0.1.0 booting..." })
    emit(BootEvent { phase: Init, message: format!("PID 0 (kernel)") })

    // Phase: Config
    emit(BootEvent { phase: Config, message: format!("Loaded config from {}", config.path) })
    emit(BootEvent { phase: Config, message: format!("Max processes: {}", config.kernel.max_processes) })

    // Phase: Services
    kernel = Kernel::boot(config, platform).await
    for service in kernel.services().list():
        emit(BootEvent { phase: Services, message: format!("[OK] {}", service.name()) })

    // Phase: ResourceTree (if exo-resource-tree enabled)
    emit(BootEvent { phase: ResourceTree, message: "Loading resource tree from checkpoint..." })
    emit(BootEvent { phase: ResourceTree, message: format!("{} nodes loaded", tree.node_count()) })

    // Phase: Agents (if agent-first boot per doc 10)
    emit(BootEvent { phase: Agents, message: "Spawning root agent..." })
    emit(BootEvent { phase: Agents, message: "Spawning service agents: message-bus, health-monitor, cron" })

    // Phase: Network (if networking enabled per doc 12)
    emit(BootEvent { phase: Network, message: "Network service starting..." })
    emit(BootEvent { phase: Network, message: format!("Listening on {}", bind_addr) })

    // Phase: Ready
    emit(BootEvent { phase: Ready, message: "Boot complete. Type 'help' for commands." })

    return KernelConsole { kernel, boot_log, prompt: "weave> " }

fn KernelConsole::run_repl():
    loop:
        line = readline(self.prompt)
        match parse_command(line):
            WeaveCommand(cmd) => self.kernel.execute_weave(cmd)
            WeftCommand(cmd)  => self.kernel.execute_weft(cmd)
            "help"            => print_console_help()
            "exit" | "quit"   => break
            "boot-log"        => self.replay_boot_log()
            ""                => continue
            other             => println!("Unknown command: {}", other)
```

### Console Output Format

The following shows what the boot output and interactive session looks like:

```
$ weave console

  WeftOS v0.1.0
  ─────────────────────────────────────────────

  [INIT]      PID 0 (kernel)
  [CONFIG]    Loaded config from ~/.clawft/config.json
  [CONFIG]    Max processes: 64
  [SERVICES]  [OK] message-bus
  [SERVICES]  [OK] health-monitor
  [SERVICES]  [OK] cron-service
  [TREE]      Loading resource tree from checkpoint...
  [TREE]      247 nodes loaded (12ms)
  [AGENTS]    Spawning root agent (pid:1)
  [AGENTS]    Spawning service agents: message-bus, health, cron
  [AGENTS]    [OK] message-bus (pid:2)
  [AGENTS]    [OK] health-monitor (pid:3)
  [AGENTS]    [OK] cron-service (pid:4)
  [NETWORK]   Network service starting on 0.0.0.0:9100
  [NETWORK]   mDNS discovery active
  [READY]     Boot complete in 1.2s (4 agents, 3 services, 247 resources)

  Type 'help' for commands, 'exit' to shutdown.

weave> kernel ps
  PID  AGENT            STATE    MEM     CPU    PARENT
  1    root             running  12MB    0.1s   -
  2    message-bus      running  8MB     0.0s   1
  3    health-monitor   running  4MB     0.0s   1
  4    cron-service     running  6MB     0.0s   1

weave> weft agent spawn --name my-worker
  [SPAWN] my-worker (pid:5) -- capabilities: default

weave>
```

---

## A -- Architecture

### Component Relationships

```
Kernel<P: Platform>
  |
  +-- AppContext<P> (owned, created during boot)
  |     |
  |     +-- MessageBus (Arc, shared with KernelIpc)
  |     +-- MemoryStore<P>
  |     +-- ToolRegistry
  |     +-- SkillsLoader<P>
  |     +-- SessionManager<P>
  |
  +-- ProcessTable (Arc, shared)
  |     +-- DashMap<Pid, ProcessEntry>
  |
  +-- ServiceRegistry (Arc, shared)
  |     +-- DashMap<String, Arc<dyn SystemService>>
  |
  +-- KernelIpc (Arc, wraps MessageBus)
  |     +-- message routing
  |     +-- topic subscriptions (K2)
  |
  +-- HealthSystem
        +-- periodic health checks
        +-- aggregated status

KernelConsole
  |
  +-- Kernel<P> (Arc, shared)
  +-- BootLog (recorded boot events)
  +-- CommandParser (dispatches to weave/weft handlers)
```

### Integration Points

1. **AppContext ownership**: Kernel owns `AppContext<P>`. Before calling `into_agent_loop()`, kernel extracts Arc references to shared services
2. **MessageBus reuse**: `KernelIpc` wraps the existing `MessageBus` (no new message bus). Adds typed `KernelMessage` envelope on top
3. **CronService adapter**: `CronServiceWrapper` implements `SystemService` trait, delegates to existing `CronService`
4. **CLI dispatch**: New `kernel` subcommand in `clawft-cli` creates kernel instance and calls status/services/ps

### Crate Dependency Graph

```
clawft-kernel
  +-- clawft-core (for AppContext, MessageBus, AgentLoop)
  +-- clawft-types (for ClawftConfig, UserPermissions)
  +-- clawft-platform (for Platform trait)
  +-- clawft-plugin (for SandboxPolicy)
  +-- dashmap
  +-- tokio
  +-- chrono
  +-- serde, serde_json
  +-- tracing
```

### Ruvector Integration (Doc 07)

When the `ruvector-*` feature gates are enabled, ruvector crates replace or enhance
several custom K0 components. The custom implementations remain as fallbacks when
the feature gates are disabled. See `07-ruvector-deep-integration.md` for full
adapter code.

| Custom Component | Ruvector Replacement | Feature Gate | Benefit |
|---|---|---|---|
| `ServiceRegistry` (DashMap + `SystemService` trait) | `ruvector-cluster::ClusterManager` | `ruvector-cluster` | Same DashMap foundation plus health checks, consistent hash ring, and pluggable discovery |
| `HealthSystem` (periodic checks) | `ruvector-cluster::ClusterManager::health_check()` | `ruvector-cluster` | Health checking built into cluster manager; composable with coherence scoring |
| `KernelState` enum | `ruvector-cognitive-container::ContainerState` | `ruvector-cluster` | Richer state machine with epoch tracking |
| (none -- new capability) | `rvf-crypto` witness chain | `ruvector-crypto` | Hash-chained boot attestation and tamper-evident audit trail from first boot event |

**ExoChain references**: `exo-dag::Mmr` can provide a Merkle Mountain Range audit
trail at boot, complementing the `rvf-crypto` witness chain.

Cross-reference: `07-ruvector-deep-integration.md`, Section 3 "Phase K0: Kernel Foundation".

---

## R -- Refinement

### Edge Cases

1. **Double boot**: `Kernel::boot()` called twice -- return error if state is not `Halted`
2. **Service registration during boot**: Services registered before `start_all()` are started; services registered after must be started individually
3. **PID overflow**: `AtomicU64` wraps at `u64::MAX` -- practically impossible but document behavior
4. **Empty process table**: `weave kernel ps` shows "No agents running" rather than empty table
5. **Health check timeout**: Individual service health checks timeout after 5 seconds; mark as `Unknown` status

### Backward Compatibility

- **No breaking changes**: All existing `weft` commands work exactly as before
- **Kernel is opt-in**: `kernel.enabled = false` by default in `KernelConfig`; kernel subsystems only activate when explicitly enabled or when `weave kernel` commands are used
- **Config backward compat**: `KernelConfig` uses `#[serde(default)]` everywhere; existing config files parse without errors

### Error Handling

- Boot failures are fatal: kernel transitions to `Halted` state
- Individual service failures are non-fatal: marked as `Unhealthy`, kernel continues
- Process table operations return `Result<T>` with typed `KernelError` enum
- All errors logged via `tracing` before propagation

---

## C -- Completion

### Exit Criteria

- [x] `clawft-kernel` crate compiles with `cargo check`
- [x] `Kernel<NativePlatform>` boots successfully in unit test
- [x] Process table supports insert, get, remove, list, update_state
- [x] At least one `SystemService` implementation exists (CronService wrapper or mock)
- [x] Service registry start/stop lifecycle works
- [x] Health system aggregates service health correctly
- [x] `KernelConfig` parses from JSON with all defaults
- [x] `weave kernel status` CLI command returns kernel state
- [x] `weave kernel services` CLI command lists registered services
- [x] `weave kernel ps` CLI command lists process table
- [x] `weave console` boots kernel and opens interactive REPL
- [x] Boot events display in real-time during boot
- [x] `weave console --attach` connects to running kernel
- [x] REPL accepts both `weave` and `weft` commands
- [x] `boot-log` command replays boot events
- [x] All existing workspace tests pass (`scripts/build.sh test`)
- [x] Clippy clean (`scripts/build.sh clippy`)
- [x] WASM check passes (kernel crate excluded from browser build)
- [x] ADR-028 committed to `docs/architecture/`
- [x] Rustdoc builds without warnings for `clawft-kernel`

### Testing Verification

```bash
# Unit tests for kernel crate
cargo test -p clawft-kernel

# Full workspace regression check
scripts/build.sh test

# Clippy
scripts/build.sh clippy

# Verify WASM isn't broken
cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser

# CLI smoke test
cargo run --bin weft -- kernel status
cargo run --bin weft -- kernel services
cargo run --bin weft -- kernel ps
```
