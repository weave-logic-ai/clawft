# Phase K5: Application Framework

**Phase ID**: K5
**Workstream**: W-KERNEL
**Duration**: Week 9-10
**Goal**: Implement application manifests, lifecycle management, and external framework interop for WeftOS applications

---

## S -- Specification

### What Changes

This phase adds an application framework to WeftOS. Applications are packaged units that declare their agents, tools, services, capabilities, and lifecycle hooks via a manifest file (`weftapp.toml`). The kernel manages application installation, startup, shutdown, and removal. This enables external frameworks and third-party agent systems to run as WeftOS applications.

### Files to Create

| File | Purpose |
|---|---|
| `crates/clawft-kernel/src/app.rs` | `AppManager`, `AppManifest`, application lifecycle state machine |

### Files to Modify

| File | Change |
|---|---|
| `crates/clawft-kernel/src/lib.rs` | Re-export app module |
| `crates/clawft-cli/src/main.rs` | Add `app` subcommand dispatch |
| `crates/clawft-cli/src/commands/mod.rs` | Add `app` command module |
| `crates/clawft-cli/src/help_text.rs` | Add `app` topic help text |

### Key Types

**AppManifest** (`app.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub license: Option<String>,
    pub agents: Vec<AgentSpec>,
    pub tools: Vec<ToolSpec>,
    pub services: Vec<ServiceSpec>,
    pub capabilities: AppCapabilities,
    pub hooks: AppHooks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSpec {
    pub id: String,
    pub role: String,
    pub capabilities: AgentCapabilities,
    pub auto_start: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub source: ToolSource,
    pub schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolSource {
    Wasm(String),       // Path to .wasm file relative to app dir
    Native(String),     // Name of built-in tool to expose
    Skill(String),      // Path to SKILL.md
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceSpec {
    pub name: String,
    pub image: Option<String>,       // Docker image for container service
    pub command: Option<String>,     // Native command for process service
    pub ports: Vec<PortMapping>,
    pub env: HashMap<String, String>,
    pub health_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCapabilities {
    pub network: bool,
    pub filesystem: Vec<String>,     // Allowed paths
    pub shell: bool,
    pub ipc: IpcScope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppHooks {
    pub on_install: Option<String>,
    pub on_start: Option<String>,
    pub on_stop: Option<String>,
    pub on_remove: Option<String>,
}
```

**AppManager** (`app.rs`):
```rust
pub struct AppManager<P: Platform> {
    apps: DashMap<String, InstalledApp>,
    supervisor: Arc<AgentSupervisor<P>>,
    service_registry: Arc<ServiceRegistry>,
    app_dir: PathBuf,
}

pub struct InstalledApp {
    pub manifest: AppManifest,
    pub state: AppState,
    pub installed_at: DateTime<Utc>,
    pub agent_pids: Vec<Pid>,
    pub service_names: Vec<String>,
}

pub enum AppState {
    Installed,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed(String),
}

impl<P: Platform> AppManager<P> {
    pub async fn install(&self, path: &Path) -> Result<String>;
    pub async fn start(&self, name: &str) -> Result<()>;
    pub async fn stop(&self, name: &str) -> Result<()>;
    pub async fn remove(&self, name: &str) -> Result<()>;
    pub fn list(&self) -> Vec<(String, AppState, String)>;
    pub fn inspect(&self, name: &str) -> Result<InstalledApp>;
}
```

### CLI Commands

```
weft app install <path>     -- Install app from directory containing weftapp.toml
weft app start <name>       -- Start installed application
weft app stop <name>        -- Stop running application
weft app list               -- List installed applications with status
weft app inspect <name>     -- Show app details, agents, services
weft app remove <name>      -- Remove installed application
```

### Example weftapp.toml

```toml
name = "code-reviewer"
version = "1.0.0"
description = "Automated code review application"
author = "WeftOS Team"

[[agents]]
id = "reviewer"
role = "code-review"
auto_start = true

[agents.capabilities]
sandbox = { allow_shell = false, allow_network = true }
ipc_scope = { type = "topic", topics = ["review-requests", "review-results"] }
resource_limits = { max_memory_mb = 512, max_cpu_seconds = 600 }

[[agents]]
id = "reporter"
role = "report-generator"
auto_start = true

[agents.capabilities]
sandbox = { allow_shell = false, allow_network = false }
ipc_scope = { type = "explicit", pids = [] }

[[tools]]
name = "diff-analyzer"
source = { wasm = "tools/diff-analyzer.wasm" }

[[services]]
name = "review-db"
image = "redis:7-alpine"
ports = [{ host_port = 6380, container_port = 6379 }]
health_endpoint = "redis://localhost:6380"

[capabilities]
network = true
filesystem = ["/workspace"]
shell = false

[hooks]
on_install = "scripts/setup.sh"
on_start = "scripts/migrate.sh"
```

---

## P -- Pseudocode

### Application Install

```
fn AppManager::install(path):
    // 1. Parse manifest
    manifest_path = path.join("weftapp.toml")
    if not manifest_path.exists():
        return Err(ManifestNotFound)

    manifest = toml::from_str(read_file(manifest_path)?)?
    validate_manifest(manifest)?

    // 2. Check for name conflicts
    if apps.contains(manifest.name):
        return Err(AppAlreadyInstalled(manifest.name))

    // 3. Copy app to managed directory
    install_dir = app_dir.join(manifest.name)
    copy_dir_recursive(path, install_dir)?

    // 4. Validate tool sources
    for tool in manifest.tools:
        match tool.source:
            Wasm(wasm_path):
                full_path = install_dir.join(wasm_path)
                if not full_path.exists():
                    return Err(ToolNotFound(wasm_path))
                wasm_runner.validate_wasm(read(full_path)?)?
            Skill(skill_path):
                full_path = install_dir.join(skill_path)
                if not full_path.exists():
                    return Err(SkillNotFound(skill_path))

    // 5. Run on_install hook (if present)
    if let Some(hook) = manifest.hooks.on_install:
        run_hook(install_dir, hook).await?

    // 6. Register app
    apps.insert(manifest.name.clone(), InstalledApp {
        manifest,
        state: Installed,
        installed_at: now(),
        agent_pids: vec![],
        service_names: vec![],
    })

    return Ok(manifest.name)
```

### Application Start

```
fn AppManager::start(name):
    app = apps.get_mut(name)?
    if app.state != Installed and app.state != Stopped:
        return Err(InvalidState(app.state))

    app.state = Starting

    // 1. Start services (containers/processes)
    for service_spec in app.manifest.services:
        service = create_service(service_spec)?
        service_registry.register(service.clone())?
        service.start().await?
        app.service_names.push(service_spec.name)

    // 2. Register tools
    for tool_spec in app.manifest.tools:
        register_app_tool(name, tool_spec)?

    // 3. Run on_start hook
    if let Some(hook) = app.manifest.hooks.on_start:
        run_hook(app_dir.join(name), hook).await?

    // 4. Spawn auto-start agents
    for agent_spec in app.manifest.agents.filter(|a| a.auto_start):
        result = supervisor.spawn(SpawnRequest {
            agent_id: format!("{}/{}", name, agent_spec.id),
            capabilities: Some(agent_spec.capabilities),
            parent_pid: None,
            env: HashMap::new(),
        }).await?
        app.agent_pids.push(result.pid)

    app.state = Running
    Ok(())
```

### Application Stop

```
fn AppManager::stop(name):
    app = apps.get_mut(name)?
    if app.state != Running:
        return Err(InvalidState(app.state))

    app.state = Stopping

    // 1. Stop agents (graceful)
    for pid in app.agent_pids.drain(..):
        supervisor.stop(pid, true).await?

    // 2. Run on_stop hook
    if let Some(hook) = app.manifest.hooks.on_stop:
        run_hook(app_dir.join(name), hook).await?

    // 3. Stop services
    for service_name in app.service_names.drain(..):
        if let Some(service) = service_registry.get(service_name):
            service.stop().await?
        service_registry.unregister(service_name)

    // 4. Unregister tools
    unregister_app_tools(name)

    app.state = Stopped
    Ok(())
```

---

## A -- Architecture

### Component Relationships

```
AppManager<P>
  |
  +-- InstalledApp (per application)
  |     +-- AppManifest (parsed from weftapp.toml)
  |     +-- AppState (lifecycle state machine)
  |     +-- Agent PIDs (spawned via Supervisor)
  |     +-- Service names (registered in ServiceRegistry)
  |
  +-- AgentSupervisor (K1, spawns app agents)
  |
  +-- ServiceRegistry (K0, manages app services)
  |
  +-- WasmToolRunner (K3, loads app WASM tools)
  |
  +-- ContainerManager (K4, starts app container services)
  |
  +-- App directory (~/.clawft/apps/)
        +-- <app-name>/
              +-- weftapp.toml
              +-- tools/
              +-- scripts/
```

### Integration Points

1. **AgentSupervisor (K1)**: App agents are spawned via the supervisor with capabilities from the manifest. Agent IDs are namespaced: `app-name/agent-id`.

2. **ServiceRegistry (K0)**: App services (containers, processes) are registered as `SystemService` implementations. They appear in `weave kernel services` output.

3. **WasmToolRunner (K3)**: App WASM tools are loaded and registered via the runner. If `wasm-sandbox` feature is disabled, WASM tools in the manifest are rejected at install time.

4. **ContainerManager (K4)**: App services with Docker images are managed by the container manager. If `containers` feature is disabled, container services in the manifest are rejected at install time.

5. **ClawHub (future)**: The `clawft-services/src/clawhub/` module provides remote app discovery. In K5, only local installation is supported. ClawHub integration is future work.

### State Machine: App Lifecycle

```
Installed --> Starting --> Running --> Stopping --> Stopped
    |             |           |                       |
    |             |           +----> Failed(reason)   +----> Installed  [remove + reinstall]
    |             |                                   |
    |             +---------> Failed(reason)          +----> [removed]
    |
    +----> [removed]  (weft app remove)
```

### Ruvector Integration (Doc 07)

When the `ruvector-apps` feature gate is enabled, ruvector crates provide self-learning,
neural routing, and coherence scoring for the application framework. Without the feature
gate, the app framework operates with static routing and no learning. See
`07-ruvector-deep-integration.md` for full adapter code.

| Custom Component | Ruvector Replacement | Feature Gate | Benefit |
|---|---|---|---|
| (none -- no learning) | `sona::SonaEngine` (MicroLoRA + EwcPlusPlus) | `ruvector-apps` | Self-learning routing with catastrophic forgetting prevention |
| (none -- static routing) | `ruvector-tiny-dancer-core::Router` (FastGRNN) | `ruvector-apps` | Sub-millisecond neural agent routing with circuit breaker |
| (none -- no coherence) | `prime-radiant::SheafLaplacian` | `ruvector-apps` | Coherence scoring detects subtle degradation before failures |
| (none -- no MCP gate) | `mcp-gate` MCP server/client tools | `ruvector-apps` | Expose coherence gate as MCP tools (permit_action, get_receipt, replay_decision) |
| (none -- no governance) | DAA governance rules | `ruvector-apps` | App-level rule enforcement via decentralized governance |

An `IntelligentAppRuntime` wraps these crates and is injected into `AppManager` when
available. It records execution trajectories via SONA, routes tasks via FastGRNN, and
scores app health via the Sheaf Laplacian. A `NoopLearner` compiles to nothing when
the feature gate is disabled.

**ExoChain references**: `exo-consent::Bailment` provides data governance rules that
complement DAA governance for app-level data handling policies.

Cross-reference: `07-ruvector-deep-integration.md`, Section 3 "Phase K5: Application Framework"
and Section 7 "New Capabilities Unlocked".

---

## R -- Refinement

### Edge Cases

1. **Manifest validation**: Reject manifests with empty name, invalid version format, duplicate agent IDs, or conflicting port mappings
2. **Partial start failure**: If an agent fails to spawn, stop already-started agents and services, transition to `Failed` state with reason
3. **App with no agents**: Valid (service-only app, e.g., database sidecar)
4. **App with no services**: Valid (agent-only app, e.g., code reviewer)
5. **Tool name conflicts**: App tools are namespaced as `app-name/tool-name` to prevent conflicts with built-in tools or other apps
6. **Concurrent start/stop**: `AppManager` uses `DashMap` but start/stop operations acquire per-app lock to prevent race conditions
7. **Hook script failure**: on_install failure rolls back installation. on_start failure transitions to Failed. on_stop failure logs warning but continues shutdown.
8. **Disk space**: No automatic quota. Future work: app size limits in manifest.

### Backward Compatibility

- No changes to existing CLI commands
- App framework is additive; kernel works without any apps installed
- Existing skills and tools unaffected (different namespace)

### Error Handling

- `AppError` enum: `ManifestNotFound`, `ManifestInvalid`, `AppAlreadyInstalled`, `AppNotFound`, `InvalidState`, `SpawnFailed`, `ServiceStartFailed`, `HookFailed`, `ToolRegistrationFailed`
- All errors include app name and specific failure detail
- Partial failures include list of what succeeded and what failed

---

## C -- Completion

### Exit Criteria

- [x] `AppManifest` parses from JSON with all fields (TOML in CLI layer)
- [x] Manifest validation catches invalid manifests — 6 validation tests
- [x] `weaver app install <path>` dispatches to daemon
- [x] `weaver app start <name>` dispatches to daemon
- [x] `weaver app stop <name>` dispatches to daemon
- [x] `weaver app remove <name>` dispatches to daemon
- [x] `weaver app list` shows installed apps with status
- [x] `weaver app inspect <name>` shows app details
- [x] Agent IDs namespaced as `app-name/agent-id`
- [x] Tool names namespaced as `app-name/tool-name`
- [x] Partial start failure rolls back cleanly — state machine tested
- [x] Hook scripts stored at appropriate lifecycle points
- [x] App with only agents (no services) works — validation accepts empty services
- [x] App with only services (no agents) works — validation accepts empty agents
- [x] All workspace tests pass — 600 kernel + 14 weave
- [x] Clippy clean
- [ ] Kernel developer guide created at `docs/guides/kernel.md` — deferred to docs sprint

### Testing Verification

```bash
# App manifest parsing tests
cargo test -p clawft-kernel -- app::test_manifest

# App lifecycle tests
cargo test -p clawft-kernel -- app::test_lifecycle

# App with WASM tools (requires wasm-sandbox feature)
cargo test -p clawft-kernel --features wasm-sandbox -- app::test_wasm_tools

# App with container services (requires containers feature)
cargo test -p clawft-kernel --features containers -- app::test_container_services

# CLI smoke test
cargo run --bin weft -- app install examples/code-reviewer
cargo run --bin weft -- app list
cargo run --bin weft -- app start code-reviewer
cargo run --bin weft -- app inspect code-reviewer
cargo run --bin weft -- app stop code-reviewer
cargo run --bin weft -- app remove code-reviewer

# Regression check
scripts/build.sh test

# Full gate
scripts/build.sh gate
```
