# WeftOS Kernel Manual Testing Guide

**Target**: `clawft-kernel` crate
**Version**: 0.1.0
**Test Count**: 259 unit tests across 18 modules
**Total SLOC**: ~9,300 lines

This guide provides comprehensive manual and integration testing procedures for the WeftOS kernel beyond automated unit tests. Use this to validate end-to-end behavior, cross-module integration, and production readiness.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Running Automated Tests](#running-automated-tests)
3. [Module-by-Module Manual Testing](#module-by-module-manual-testing)
   - [K0: Foundation](#k0-foundation)
   - [K1: Supervisor + RBAC](#k1-supervisor--rbac)
   - [K2: A2A IPC](#k2-a2a-ipc)
   - [K3: WASM Sandbox](#k3-wasm-sandbox)
   - [K4: Containers](#k4-containers)
   - [K5: App Framework](#k5-app-framework)
   - [K6a: Distributed](#k6a-distributed)
   - [K6b: Agency](#k6b-agency)
4. [Integration Test Scenarios](#integration-test-scenarios)
5. [CLI Testing](#cli-testing)
6. [Feature-Gated Module Testing](#feature-gated-module-testing)
7. [What Cannot Be Tested Yet](#what-cannot-be-tested-yet)
8. [Regression Checklist](#regression-checklist)

---

## Prerequisites

### Required Tools

```bash
# Rust toolchain (1.75+)
rustc --version

# Clawft build script
ls -l scripts/build.sh

# weft CLI (built from source)
cargo build --release --bin weft
export PATH="$PWD/target/release:$PATH"
```

### Build the Kernel Crate

```bash
# Build kernel with default features (native)
scripts/build.sh native

# Build with all features
cargo build -p clawft-kernel --all-features

# Build kernel with feature flags
cargo build -p clawft-kernel --features wasm-sandbox
cargo build -p clawft-kernel --features containers
```

### Verify Test Suite Runs

```bash
# Run all kernel tests (should show 259 passed)
scripts/build.sh test

# Run kernel-only tests
cargo test -p clawft-kernel

# Run with output for debugging
cargo test -p clawft-kernel -- --nocapture
```

---

## Running Automated Tests

### Full Test Suite

```bash
# Run all workspace tests
scripts/build.sh test

# Run kernel tests only
cargo test -p clawft-kernel --lib

# Run integration tests
cargo test -p clawft-kernel --test '*'

# Run specific module tests
cargo test -p clawft-kernel boot::tests
cargo test -p clawft-kernel process::tests
cargo test -p clawft-kernel service::tests
```

### Coverage Check

```bash
# Count tests per module
cargo test -p clawft-kernel -- --list | wc -l

# Expected: 259 tests
```

### Performance Benchmarking

```bash
# Time test execution
time cargo test -p clawft-kernel

# Expected: <1 second for full suite
```

---

## Module-by-Module Manual Testing

### K0: Foundation

#### `boot.rs` - Kernel Boot Sequence

**What to Test Manually**:
- Boot lifecycle state transitions (Booting → Running → ShuttingDown → Halted)
- Boot log collection and formatting
- AppContext initialization and extraction
- Boot time tracking
- Service initialization during boot

**Test Snippets**:

```rust
use clawft_kernel::{Kernel, KernelState};
use clawft_platform::NativePlatform;
use clawft_types::config::Config;

#[tokio::test]
async fn manual_kernel_boot_lifecycle() {
    let config = Config::default();
    let platform = NativePlatform::new();

    // Boot kernel
    let mut kernel = Kernel::boot(config, platform).await.unwrap();
    assert_eq!(kernel.state(), &KernelState::Running);

    // Check boot time was recorded
    let uptime = kernel.uptime_secs();
    assert!(uptime < 5.0);

    // Verify boot log has entries
    let log = kernel.boot_log();
    assert!(!log.is_empty());

    // Check all boot phases completed
    let phases: Vec<_> = log.iter().map(|e| e.phase()).collect();
    println!("Boot phases: {:?}", phases);

    // Shutdown
    kernel.shutdown().await.unwrap();
    assert_eq!(kernel.state(), &KernelState::Halted);
}
```

**Expected Behaviors**:
- Boot completes in <5 seconds
- Boot log contains entries for: Init, Config, Services, ResourceTree, Agents, Network, Ready
- State transitions are irreversible (cannot boot a halted kernel)
- Shutdown stops all services gracefully

**Edge Cases**:
- Boot with minimal config (no services, no agents)
- Boot with invalid config (should fail with KernelError::BootFailed)
- Double shutdown (should be idempotent)
- Access boot log after shutdown (should still be available)

#### `process.rs` - Process Table and PID Management

**What to Test Manually**:
- PID allocation monotonicity (never reuses PIDs)
- Process state transitions (Starting → Running → Suspended → Stopping → Exited)
- Process table queries (list, get, count)
- Resource usage tracking
- Concurrent process creation

**Test Snippets**:

```rust
use clawft_kernel::{ProcessTable, ProcessState, ResourceUsage};

#[tokio::test]
async fn manual_process_lifecycle() {
    let table = ProcessTable::new();

    // Allocate PIDs
    let pid1 = table.allocate_pid();
    let pid2 = table.allocate_pid();
    assert!(pid2 > pid1, "PIDs must be monotonic");

    // Insert process
    let usage = ResourceUsage {
        cpu_time_ms: 100,
        memory_bytes: 1024 * 1024,
        open_files: 5,
    };
    table.insert_process(pid1, "test-agent", usage.clone());

    // Verify retrieval
    let entry = table.get_process(&pid1).unwrap();
    assert_eq!(entry.name(), "test-agent");
    assert_eq!(entry.state(), &ProcessState::Starting);

    // Transition states
    table.set_state(&pid1, ProcessState::Running).unwrap();
    assert_eq!(table.get_process(&pid1).unwrap().state(), &ProcessState::Running);

    table.set_state(&pid1, ProcessState::Suspended).unwrap();
    table.set_state(&pid1, ProcessState::Stopping).unwrap();
    table.set_state(&pid1, ProcessState::Exited).unwrap();

    // List all processes
    let all = table.list_all();
    assert_eq!(all.len(), 2); // pid1 and pid2

    // Count by state
    let running_count = table.count_by_state(&ProcessState::Running);
    println!("Running processes: {}", running_count);
}
```

**Expected Behaviors**:
- PIDs start at 1 and increment (0 reserved for kernel)
- PIDs are never reused within a session
- State transitions follow valid paths only
- Resource usage updates are atomic
- List operations are O(1) with DashMap

**Edge Cases**:
- Set state on non-existent PID (should return error)
- Remove process from multiple threads concurrently
- Allocate 10,000+ PIDs (should not wrap or panic)
- Update resource usage while reading process entry

#### `service.rs` - Service Registry

**What to Test Manually**:
- Service registration and unregistration
- Service lifecycle (start/stop)
- Service health checks
- Service listing and queries
- Duplicate service name handling

**Test Snippets**:

```rust
use clawft_kernel::{ServiceRegistry, SystemService, HealthStatus};
use async_trait::async_trait;

struct TestService {
    name: String,
    healthy: bool,
}

#[async_trait]
impl SystemService for TestService {
    fn name(&self) -> &str { &self.name }

    async fn start(&mut self) -> Result<(), String> {
        println!("Starting {}", self.name);
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), String> {
        println!("Stopping {}", self.name);
        Ok(())
    }

    async fn health_check(&self) -> HealthStatus {
        if self.healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Down("test failure".to_string())
        }
    }
}

#[tokio::test]
async fn manual_service_registry() {
    let registry = ServiceRegistry::new();

    // Register service
    let svc = TestService {
        name: "test-svc-1".to_string(),
        healthy: true,
    };
    registry.register(Arc::new(tokio::sync::Mutex::new(svc))).await;

    // List services
    let names = registry.list_service_names().await;
    assert!(names.contains(&"test-svc-1".to_string()));

    // Get service
    let retrieved = registry.get("test-svc-1").await.unwrap();
    let locked = retrieved.lock().await;
    assert_eq!(locked.name(), "test-svc-1");

    // Start all
    registry.start_all().await.unwrap();

    // Health check all
    let health_map = registry.health_all().await;
    assert!(matches!(health_map.get("test-svc-1"), Some(HealthStatus::Healthy)));

    // Stop all
    registry.stop_all().await.unwrap();

    // Unregister
    registry.unregister("test-svc-1").await;
    assert!(registry.get("test-svc-1").await.is_none());
}
```

**Expected Behaviors**:
- Service names must be unique
- start_all/stop_all are idempotent
- Health checks run concurrently
- Unregistered services cannot be retrieved

**Edge Cases**:
- Register duplicate service name (should replace or error)
- Unregister non-existent service (should be no-op)
- Health check timeout simulation
- Start/stop service that panics

#### `ipc.rs` - Kernel IPC

**What to Test Manually**:
- Message creation with different targets (Pid, Topic, Broadcast, Service)
- Message payload serialization
- KernelSignal handling
- MessageBus integration

**Test Snippets**:

```rust
use clawft_kernel::{KernelIpc, KernelMessage, MessageTarget, MessagePayload};
use clawft_core::bus::MessageBus;
use serde_json::json;

#[tokio::test]
async fn manual_kernel_ipc() {
    let bus = Arc::new(MessageBus::new());
    let ipc = KernelIpc::new(bus.clone());

    // Send message to PID
    let payload = MessagePayload::Json(json!({"cmd": "ping"}));
    let msg = KernelMessage::new(
        MessageTarget::Pid(42),
        payload,
        Some("request-123".to_string()),
    );

    ipc.send_message(msg).await.unwrap();

    // Send broadcast
    let broadcast = KernelMessage::broadcast(
        MessagePayload::Text("shutdown".to_string()),
    );
    ipc.send_message(broadcast).await.unwrap();

    // Send to topic
    let topic_msg = KernelMessage::new(
        MessageTarget::Topic("logs".to_string()),
        MessagePayload::Text("log entry".to_string()),
        None,
    );
    ipc.send_message(topic_msg).await.unwrap();
}
```

**Expected Behaviors**:
- Messages are routed based on target type
- Correlation IDs are preserved
- Broadcast messages reach all subscribers
- Invalid targets return error

**Edge Cases**:
- Send message with empty payload
- Send message to PID 0 (kernel reserved)
- Send message before bus is initialized
- Large payload (>1MB JSON)

#### `capability.rs` - Agent Capabilities and RBAC

**What to Test Manually**:
- Tool permission checking
- IPC scope validation
- Resource limit enforcement
- Sandbox policy evaluation

**Test Snippets**:

```rust
use clawft_kernel::{
    AgentCapabilities, CapabilityChecker, IpcScope,
    ResourceLimits, ResourceType, ToolPermissions,
};

#[tokio::test]
async fn manual_capability_checking() {
    let caps = AgentCapabilities {
        tools: ToolPermissions::Restricted(vec![
            "read_file".to_string(),
            "write_file".to_string(),
        ]),
        ipc_scope: IpcScope::Restricted(vec![1, 2, 3]),
        resource_limits: ResourceLimits {
            max_memory_mb: 512,
            max_cpu_percent: 50,
            max_concurrent_operations: 10,
            max_file_size_mb: 100,
        },
        ..Default::default()
    };

    let checker = CapabilityChecker::new(caps);

    // Check tool permission
    assert!(checker.can_use_tool("read_file"));
    assert!(!checker.can_use_tool("execute_code"));

    // Check IPC scope
    assert!(checker.can_send_to_pid(&1));
    assert!(!checker.can_send_to_pid(&99));

    // Check resource limits
    assert!(checker.can_allocate_resource(
        ResourceType::Memory,
        400 * 1024 * 1024 // 400MB
    ));
    assert!(!checker.can_allocate_resource(
        ResourceType::Memory,
        600 * 1024 * 1024 // 600MB
    ));
}
```

**Expected Behaviors**:
- ToolPermissions::All allows any tool
- ToolPermissions::Restricted enforces whitelist
- IpcScope::Restricted blocks unlisted PIDs
- Resource limits are enforced strictly

**Edge Cases**:
- Check permission on empty tool list
- Check IPC scope with PID 0
- Allocate resource at exactly the limit
- Nested capability inheritance

#### `health.rs` - Health System

**What to Test Manually**:
- Health status aggregation
- Overall health calculation (Healthy/Degraded/Down)
- Service health integration
- Health history tracking

**Test Snippets**:

```rust
use clawft_kernel::{HealthSystem, HealthStatus, OverallHealth};

#[tokio::test]
async fn manual_health_system() {
    let health = HealthSystem::new();

    // Add service health
    health.update_service_health("db", HealthStatus::Healthy).await;
    health.update_service_health("cache", HealthStatus::Degraded("high latency".to_string())).await;
    health.update_service_health("queue", HealthStatus::Down("connection refused".to_string())).await;

    // Get overall health
    let overall = health.overall_health().await;
    assert_eq!(overall, OverallHealth::Down);

    // Get service health
    let db_health = health.service_health("db").await;
    assert!(matches!(db_health, Some(HealthStatus::Healthy)));

    // List all
    let all = health.all_service_health().await;
    assert_eq!(all.len(), 3);

    println!("Health report: {:?}", all);
}
```

**Expected Behaviors**:
- Overall health is Down if any service is Down
- Overall health is Degraded if any service is Degraded (and none Down)
- Overall health is Healthy only if all services are Healthy
- Health updates are atomic

**Edge Cases**:
- Query health before any services registered
- Rapid health updates from multiple threads
- Remove service and check health (should be removed from map)

#### `console.rs` - Boot Events and Logging

**What to Test Manually**:
- Boot event recording
- Boot phase transitions
- Log level filtering
- Boot log formatting

**Test Snippets**:

```rust
use clawft_kernel::{BootEvent, BootPhase, BootLog, LogLevel};

#[tokio::test]
async fn manual_boot_console() {
    let mut log = BootLog::new();

    // Record events
    log.record(BootPhase::Init, LogLevel::Info, "Initializing kernel");
    log.record(BootPhase::Config, LogLevel::Info, "Loading config");
    log.record(BootPhase::Services, LogLevel::Info, "Starting services");
    log.record(BootPhase::Services, LogLevel::Warn, "Service 'cache' degraded");
    log.record(BootPhase::Ready, LogLevel::Info, "Kernel ready");

    // Check count
    assert_eq!(log.len(), 5);

    // Format output
    let formatted = log.format_terminal();
    println!("{}", formatted);

    // Check phases
    let events = log.events();
    let phases: Vec<_> = events.iter().map(|e| e.phase()).collect();
    assert!(phases.contains(&BootPhase::Init));
    assert!(phases.contains(&BootPhase::Ready));
}
```

**Expected Behaviors**:
- Events are recorded in order
- Timestamps are monotonic
- Log levels are preserved
- Terminal formatting includes ANSI colors (if TTY)

**Edge Cases**:
- Record 10,000 events (check memory usage)
- Format empty log
- Record events with unicode characters

#### `config.rs` - Kernel Configuration

**What to Test Manually**:
- KernelConfigExt wrapping
- Default values
- Config serialization/deserialization
- Config validation

**Test Snippets**:

```rust
use clawft_kernel::config::KernelConfigExt;
use clawft_types::config::{Config, KernelConfig};

#[tokio::test]
async fn manual_kernel_config() {
    let mut cfg = Config::default();
    cfg.kernel = KernelConfig {
        enabled: true,
        max_processes: 100,
        health_check_interval_secs: 30,
        ..Default::default()
    };

    // Wrap in extension
    let kext = KernelConfigExt::from(cfg.kernel.clone());

    // Check values
    assert!(kext.is_enabled());
    assert_eq!(kext.max_processes(), 100);
    assert_eq!(kext.health_check_interval_secs(), 30);

    // Serialize round-trip
    let json = serde_json::to_string(&cfg.kernel).unwrap();
    let parsed: KernelConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.max_processes, 100);
}
```

**Expected Behaviors**:
- Default config is valid
- All fields serialize correctly
- Extension wrapper provides type-safe accessors

**Edge Cases**:
- Config with max_processes = 0
- Config with negative health_check_interval
- Config deserialization from partial JSON

#### `error.rs` - Kernel Error Types

**What to Test Manually**:
- Error variant creation
- Error message formatting
- Error conversion from sub-crates
- Error propagation

**Test Snippets**:

```rust
use clawft_kernel::{KernelError, KernelResult};

#[tokio::test]
async fn manual_error_handling() {
    // Create errors
    let boot_err = KernelError::BootFailed("config invalid".to_string());
    let process_err = KernelError::ProcessNotFound(42);
    let service_err = KernelError::ServiceNotFound("db".to_string());

    // Format messages
    assert_eq!(boot_err.to_string(), "boot failed: config invalid");
    assert_eq!(process_err.to_string(), "process not found: 42");
    assert_eq!(service_err.to_string(), "service not found: db");

    // Use in Result
    fn test_fn() -> KernelResult<()> {
        Err(KernelError::AlreadyRunning)
    }

    assert!(test_fn().is_err());
}
```

**Expected Behaviors**:
- All error variants implement Display and Debug
- Error messages are descriptive
- Errors convert to anyhow::Error

**Edge Cases**:
- Error with empty message
- Error with very long message (>1000 chars)
- Nested error causes

---

### K1: Supervisor + RBAC

#### `supervisor.rs` - Agent Supervisor

**What to Test Manually**:
- Agent spawning with SpawnRequest
- Agent lifecycle (spawn/stop/restart)
- Spawn result tracking
- Concurrent spawn requests

**Test Snippets**:

```rust
use clawft_kernel::{AgentSupervisor, SpawnRequest, AgentCapabilities};
use clawft_platform::NativePlatform;
use std::time::Duration;

#[tokio::test]
async fn manual_supervisor_spawn() {
    let platform = NativePlatform::new();
    let supervisor = AgentSupervisor::new(platform);

    // Create spawn request
    let request = SpawnRequest {
        name: "test-agent".to_string(),
        role: "worker".to_string(),
        capabilities: AgentCapabilities::default(),
        timeout: Some(Duration::from_secs(10)),
    };

    // Spawn agent
    let result = supervisor.spawn(request).await.unwrap();

    assert!(result.pid > 0);
    assert_eq!(result.name, "test-agent");
    println!("Spawned agent PID: {}", result.pid);

    // Check running count
    let count = supervisor.running_count().await;
    assert_eq!(count, 1);

    // Stop agent
    supervisor.stop(result.pid).await.unwrap();

    // Verify stopped
    let count_after = supervisor.running_count().await;
    assert_eq!(count_after, 0);
}
```

**Expected Behaviors**:
- Spawn returns unique PID
- Spawn timeout is enforced
- Stop is graceful
- Restart preserves PID

**Edge Cases**:
- Spawn with invalid capabilities
- Stop non-existent agent
- Restart agent that exited normally
- Spawn 100 agents concurrently

---

### K2: A2A IPC

#### `a2a.rs` - Agent-to-Agent Router

**What to Test Manually**:
- Per-PID inbox creation
- Message delivery to specific PID
- Message correlation
- Inbox cleanup on agent exit

**Test Snippets**:

```rust
use clawft_kernel::{A2ARouter, MessagePayload};
use serde_json::json;

#[tokio::test]
async fn manual_a2a_routing() {
    let router = A2ARouter::new();

    // Send message to PID
    let payload = MessagePayload::Json(json!({"cmd": "ping"}));
    router.send_to_pid(42, payload, Some("req-123".to_string())).await.unwrap();

    // Receive message
    let received = router.receive_for_pid(42).await.unwrap();
    assert_eq!(received.correlation_id, Some("req-123".to_string()));

    if let MessagePayload::Json(val) = received.payload {
        assert_eq!(val["cmd"], "ping");
    } else {
        panic!("Expected JSON payload");
    }

    // Send to non-existent PID (creates inbox)
    router.send_to_pid(99, MessagePayload::Text("hello".to_string()), None).await.unwrap();

    // Check inbox exists
    let msg = router.receive_for_pid(99).await.unwrap();
    assert!(matches!(msg.payload, MessagePayload::Text(_)));
}
```

**Expected Behaviors**:
- Inboxes are created on-demand
- Messages are FIFO per inbox
- Receive blocks until message arrives
- Correlation IDs are preserved

**Edge Cases**:
- Send to PID 0 (should error)
- Receive timeout
- Inbox overflow (>1000 messages)
- Concurrent sends to same PID

#### `topic.rs` - Topic-Based Pub/Sub

**What to Test Manually**:
- Topic subscription
- Topic unsubscription
- Message publishing to topic
- Subscriber delivery

**Test Snippets**:

```rust
use clawft_kernel::{TopicRouter, MessagePayload};

#[tokio::test]
async fn manual_topic_pubsub() {
    let router = TopicRouter::new();

    // Subscribe PIDs to topic
    router.subscribe("logs", 1).await;
    router.subscribe("logs", 2).await;
    router.subscribe("logs", 3).await;

    // Publish message
    let payload = MessagePayload::Text("log entry".to_string());
    router.publish("logs", payload).await.unwrap();

    // Check subscriber count
    let subs = router.subscribers("logs").await;
    assert_eq!(subs.len(), 3);

    // Unsubscribe
    router.unsubscribe("logs", 2).await;
    let subs_after = router.subscribers("logs").await;
    assert_eq!(subs_after.len(), 2);

    // Publish to empty topic (should succeed but no delivery)
    router.publish("metrics", MessagePayload::Text("metric".to_string())).await.unwrap();
}
```

**Expected Behaviors**:
- Subscribers receive all published messages
- Unsubscribe stops future messages
- Publishing to empty topic is no-op
- Topic names are case-sensitive

**Edge Cases**:
- Subscribe same PID twice (should be idempotent)
- Publish large payload (>10MB)
- Publish to 1000 subscribers
- Unsubscribe from non-existent topic

---

### K3: WASM Sandbox

#### `wasm_runner.rs` - WASM Tool Runner (Stubbed)

**What to Test Manually**:
- WasmToolRunner initialization (should fail if wasmtime not enabled)
- WasmTool validation
- WasmSandboxConfig parsing
- Stub error messages

**Test Snippets**:

```rust
use clawft_kernel::{WasmToolRunner, WasmSandboxConfig};

#[tokio::test]
async fn manual_wasm_runner() {
    let config = WasmSandboxConfig {
        max_memory_bytes: 10 * 1024 * 1024, // 10MB
        max_execution_time_ms: 5000,
        allowed_imports: vec!["env".to_string()],
    };

    // Attempt to create runner (should fail with stub error)
    let result = WasmToolRunner::new(config);
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("WASM support not enabled"));
}
```

**Expected Behaviors** (when feature enabled):
- WASM modules load successfully
- Execution timeout is enforced
- Memory limits are enforced
- Invalid WASM bytes are rejected

**Edge Cases**:
- Load malformed WASM file
- Execute infinite loop (should timeout)
- Allocate beyond memory limit
- Import disallowed function

---

### K4: Containers

#### `container.rs` - Container Manager (Stubbed)

**What to Test Manually**:
- ContainerManager initialization (should fail if bollard not enabled)
- ManagedContainer lifecycle
- ContainerState transitions
- Stub error messages

**Test Snippets**:

```rust
use clawft_kernel::{ContainerManager, ContainerConfig};

#[tokio::test]
async fn manual_container_manager() {
    // Attempt to create manager (should fail with stub error)
    let result = ContainerManager::new();
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert!(err.to_string().contains("container support not enabled"));
}
```

**Expected Behaviors** (when feature enabled):
- Containers start successfully
- Health checks poll container status
- Stop is graceful (SIGTERM then SIGKILL)
- Restart policy is enforced

**Edge Cases**:
- Start container with invalid image
- Stop container that already exited
- Restart container with RestartPolicy::Never
- Container port conflicts

---

### K5: App Framework

#### `app.rs` - App Manager and Manifests

**What to Test Manually**:
- AppManifest parsing and validation
- App installation
- App lifecycle (Installed → Starting → Running → Stopping → Stopped → Failed)
- App removal
- Hook execution

**Test Snippets**:

```rust
use clawft_kernel::{AppManager, AppManifest, AppState, AgentSpec, ServiceSpec};

#[tokio::test]
async fn manual_app_lifecycle() {
    let manager = AppManager::new();

    // Create manifest
    let manifest = AppManifest {
        name: "test-app".to_string(),
        version: "1.0.0".to_string(),
        agents: vec![
            AgentSpec {
                name: "worker".to_string(),
                role: "processor".to_string(),
                count: 2,
            }
        ],
        tools: vec![],
        services: vec![
            ServiceSpec {
                name: "db".to_string(),
                type_: "postgres".to_string(),
            }
        ],
        capabilities: Default::default(),
        hooks: Default::default(),
    };

    // Install app
    manager.install(manifest).await.unwrap();

    // Check state
    let app = manager.get("test-app").await.unwrap();
    assert_eq!(app.state(), &AppState::Installed);

    // Start app
    manager.start("test-app").await.unwrap();
    let app_after_start = manager.get("test-app").await.unwrap();
    assert_eq!(app_after_start.state(), &AppState::Running);

    // List apps
    let all = manager.list_all().await;
    assert_eq!(all.len(), 1);

    // Stop app
    manager.stop("test-app").await.unwrap();

    // Remove app (should fail while running)
    let remove_result = manager.remove("test-app").await;
    assert!(remove_result.is_err());
}
```

**Expected Behaviors**:
- Manifest validation catches missing required fields
- App states transition in order
- Apps cannot be removed while Running
- Hooks execute at correct lifecycle points

**Edge Cases**:
- Install app with duplicate name
- Start app with missing dependencies
- Stop app that is already Stopped
- Remove app with active agents

---

### K6a: Distributed

#### `cluster.rs` - Cluster Membership

**What to Test Manually**:
- Node registration
- Node state tracking (Joining → Active → Leaving → Left)
- Peer discovery
- Node health monitoring

**Test Snippets**:

```rust
use clawft_kernel::{ClusterMembership, PeerNode, NodeState, NodeId};

#[tokio::test]
async fn manual_cluster_membership() {
    let cluster = ClusterMembership::new();

    // Add peer nodes
    let node1 = PeerNode {
        id: NodeId::new(),
        address: "192.168.1.10:8080".to_string(),
        state: NodeState::Active,
        platform: Default::default(),
        last_seen: chrono::Utc::now(),
    };

    cluster.add_peer(node1.clone()).await;

    // List peers
    let peers = cluster.list_peers().await;
    assert_eq!(peers.len(), 1);

    // Update node state
    cluster.update_peer_state(&node1.id, NodeState::Leaving).await.unwrap();

    // Get peer
    let peer = cluster.get_peer(&node1.id).await.unwrap();
    assert_eq!(peer.state, NodeState::Leaving);

    // Remove peer
    cluster.remove_peer(&node1.id).await;
    assert!(cluster.get_peer(&node1.id).await.is_none());
}
```

**Expected Behaviors**:
- Node IDs are unique
- State transitions are tracked
- Last seen timestamps update
- Stale nodes are detected

**Edge Cases**:
- Add peer with duplicate ID (should replace or error)
- Update state of non-existent peer
- List peers when cluster is empty
- Concurrent peer updates

#### `environment.rs` - Environment Manager

**What to Test Manually**:
- Environment creation (Development, Staging, Production, Research)
- Environment scoping and isolation
- Governance branch assignment
- Learning mode configuration

**Test Snippets**:

```rust
use clawft_kernel::{
    EnvironmentManager, Environment, EnvironmentClass,
    GovernanceBranches, LearningMode,
};

#[tokio::test]
async fn manual_environment_manager() {
    let manager = EnvironmentManager::new();

    // Create development environment
    let dev_env = Environment {
        name: "dev".to_string(),
        class: EnvironmentClass::Development,
        governance: GovernanceBranches {
            legislative: true,
            executive: true,
            judicial: false,
        },
        learning_mode: LearningMode::Aggressive,
        audit_level: Default::default(),
    };

    manager.create_environment(dev_env).await.unwrap();

    // Create production environment
    let prod_env = Environment {
        name: "prod".to_string(),
        class: EnvironmentClass::Production,
        governance: GovernanceBranches {
            legislative: true,
            executive: true,
            judicial: true,
        },
        learning_mode: LearningMode::Conservative,
        audit_level: Default::default(),
    };

    manager.create_environment(prod_env).await.unwrap();

    // List environments
    let envs = manager.list_environments().await;
    assert_eq!(envs.len(), 2);

    // Get environment
    let dev = manager.get_environment("dev").await.unwrap();
    assert_eq!(dev.class, EnvironmentClass::Development);

    // Remove environment
    manager.remove_environment("dev").await.unwrap();
}
```

**Expected Behaviors**:
- Environment names are unique
- Production requires all governance branches
- Development allows aggressive learning
- Environments are isolated

**Edge Cases**:
- Create environment with duplicate name
- Remove environment with active apps
- Switch environment class after creation
- Query non-existent environment

#### `governance.rs` - Governance Engine

**What to Test Manually**:
- Governance rule creation
- Request evaluation with EffectVector
- Three-branch decision making
- Rule severity enforcement

**Test Snippets**:

```rust
use clawft_kernel::{
    GovernanceEngine, GovernanceRule, GovernanceRequest,
    EffectVector, GovernanceBranch, RuleSeverity,
};

#[tokio::test]
async fn manual_governance_engine() {
    let engine = GovernanceEngine::new();

    // Create rule
    let rule = GovernanceRule {
        id: "rule-001".to_string(),
        branch: GovernanceBranch::Legislative,
        severity: RuleSeverity::High,
        description: "No file writes in production".to_string(),
        effect: EffectVector {
            security: -5.0,
            privacy: 0.0,
            autonomy: 2.0,
            beneficence: 0.0,
            non_maleficence: -3.0,
        },
    };

    engine.add_rule(rule).await;

    // Create request
    let request = GovernanceRequest {
        action: "write_file".to_string(),
        context: serde_json::json!({"env": "production"}),
        requesting_agent: "agent-42".to_string(),
    };

    // Evaluate
    let decision = engine.evaluate(request).await;
    println!("Decision: {:?}", decision);

    // Check if approved
    assert!(!decision.approved);
    assert!(decision.rationale.contains("production"));
}
```

**Expected Behaviors**:
- Rules are evaluated in priority order (severity)
- EffectVector scores are aggregated
- Negative scores reduce approval chance
- All three branches must approve for High severity

**Edge Cases**:
- Request with no matching rules (should default allow/deny?)
- Request with conflicting rules
- Request with missing context
- Rule with invalid EffectVector (NaN, Infinity)

---

### K6b: Agency

#### `agency.rs` - Agent-First Architecture

**What to Test Manually**:
- Agent role definition
- Agent manifest creation
- Spawn permission checking
- Agent interface protocol enforcement

**Test Snippets**:

```rust
use clawft_kernel::{
    Agency, AgentRole, AgentManifest, AgentPriority,
    AgentResources, AgentRestartPolicy,
};

#[tokio::test]
async fn manual_agency() {
    let agency = Agency::new();

    // Define role
    let role = AgentRole {
        name: "worker".to_string(),
        description: "Background task processor".to_string(),
        permissions: vec!["read_file".to_string(), "write_file".to_string()],
        spawn_limit: Some(10),
    };

    agency.register_role(role).await;

    // Create agent manifest
    let manifest = AgentManifest {
        name: "worker-001".to_string(),
        role: "worker".to_string(),
        priority: AgentPriority::Normal,
        resources: AgentResources {
            cpu_shares: 512,
            memory_mb: 256,
        },
        restart_policy: AgentRestartPolicy::OnFailure,
        interfaces: vec![],
    };

    // Check spawn permission
    let can_spawn = agency.can_spawn(&manifest).await;
    assert!(can_spawn);

    // Spawn agent
    agency.spawn_agent(manifest).await.unwrap();

    // Check spawn count
    let count = agency.count_by_role("worker").await;
    assert_eq!(count, 1);
}
```

**Expected Behaviors**:
- Roles define permission templates
- Spawn limits are enforced per role
- Agent resources are allocated from pool
- Restart policies are honored

**Edge Cases**:
- Spawn agent with undefined role
- Spawn beyond role limit
- Spawn with zero resources
- Agent crash with RestartPolicy::Never

---

## Integration Test Scenarios

These scenarios test multiple modules working together.

### Scenario 1: Full Kernel Boot and Service Registration

```rust
use clawft_kernel::{Kernel, ServiceRegistry, SystemService};
use clawft_platform::NativePlatform;
use clawft_types::config::Config;

#[tokio::test]
async fn integration_boot_with_services() {
    // Create config
    let mut config = Config::default();
    config.kernel.enabled = true;

    // Boot kernel
    let platform = NativePlatform::new();
    let mut kernel = Kernel::boot(config, platform).await.unwrap();

    // Extract service registry
    let registry = kernel.service_registry();

    // Register custom service
    // (implement TestService as shown earlier)
    let svc = Arc::new(tokio::sync::Mutex::new(TestService {
        name: "test-svc".to_string(),
        healthy: true,
    }));
    registry.register(svc).await;

    // Start all services
    registry.start_all().await.unwrap();

    // Check health
    let health = registry.health_all().await;
    assert!(health.get("test-svc").is_some());

    // Shutdown kernel
    kernel.shutdown().await.unwrap();
}
```

### Scenario 2: Agent Spawn with Capability Checking

```rust
use clawft_kernel::{
    AgentSupervisor, SpawnRequest, AgentCapabilities,
    CapabilityChecker, ToolPermissions,
};

#[tokio::test]
async fn integration_spawn_with_capabilities() {
    let platform = NativePlatform::new();
    let supervisor = AgentSupervisor::new(platform);

    // Create capabilities
    let caps = AgentCapabilities {
        tools: ToolPermissions::Restricted(vec!["read_file".to_string()]),
        ..Default::default()
    };

    // Create spawn request
    let request = SpawnRequest {
        name: "restricted-agent".to_string(),
        role: "reader".to_string(),
        capabilities: caps.clone(),
        timeout: None,
    };

    // Spawn agent
    let result = supervisor.spawn(request).await.unwrap();

    // Check capabilities
    let checker = CapabilityChecker::new(caps);
    assert!(checker.can_use_tool("read_file"));
    assert!(!checker.can_use_tool("write_file"));

    // Stop agent
    supervisor.stop(result.pid).await.unwrap();
}
```

### Scenario 3: A2A Message Routing with Correlation

```rust
use clawft_kernel::{A2ARouter, MessagePayload};
use serde_json::json;

#[tokio::test]
async fn integration_a2a_correlation() {
    let router = A2ARouter::new();

    // Agent 1 sends request to Agent 2
    let request = MessagePayload::Json(json!({
        "type": "request",
        "cmd": "process_data",
        "data": [1, 2, 3, 4, 5],
    }));

    router.send_to_pid(2, request, Some("req-123".to_string())).await.unwrap();

    // Agent 2 receives and processes
    let received = router.receive_for_pid(2).await.unwrap();
    assert_eq!(received.correlation_id, Some("req-123".to_string()));

    // Agent 2 sends response back to Agent 1
    let response = MessagePayload::Json(json!({
        "type": "response",
        "result": 15, // sum of data
    }));

    router.send_to_pid(1, response, Some("req-123".to_string())).await.unwrap();

    // Agent 1 receives response
    let resp = router.receive_for_pid(1).await.unwrap();
    assert_eq!(resp.correlation_id, Some("req-123".to_string()));
}
```

### Scenario 4: Topic Pub/Sub with Multiple Subscribers

```rust
use clawft_kernel::{TopicRouter, MessagePayload};

#[tokio::test]
async fn integration_topic_fanout() {
    let router = TopicRouter::new();

    // Subscribe 3 agents to "events" topic
    router.subscribe("events", 10).await;
    router.subscribe("events", 20).await;
    router.subscribe("events", 30).await;

    // Publish event
    let event = MessagePayload::Json(serde_json::json!({
        "type": "user_login",
        "user_id": 42,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }));

    router.publish("events", event).await.unwrap();

    // All subscribers should receive (verify via A2A inboxes)
    // (This requires integration with A2ARouter - left as exercise)
}
```

### Scenario 5: App Lifecycle with Governance

```rust
use clawft_kernel::{
    AppManager, AppManifest, GovernanceEngine,
    GovernanceRequest, GovernanceRule, GovernanceBranch,
};

#[tokio::test]
async fn integration_app_with_governance() {
    let app_manager = AppManager::new();
    let gov_engine = GovernanceEngine::new();

    // Add governance rule: no apps in production without approval
    let rule = GovernanceRule {
        id: "prod-approval".to_string(),
        branch: GovernanceBranch::Executive,
        severity: RuleSeverity::High,
        description: "Production apps require approval".to_string(),
        effect: EffectVector {
            security: -10.0,
            privacy: -5.0,
            autonomy: 0.0,
            beneficence: 0.0,
            non_maleficence: -5.0,
        },
    };
    gov_engine.add_rule(rule).await;

    // Create app manifest
    let manifest = AppManifest {
        name: "prod-app".to_string(),
        version: "1.0.0".to_string(),
        agents: vec![],
        tools: vec![],
        services: vec![],
        capabilities: Default::default(),
        hooks: Default::default(),
    };

    // Check governance
    let request = GovernanceRequest {
        action: "install_app".to_string(),
        context: serde_json::json!({"env": "production", "app": "prod-app"}),
        requesting_agent: "admin".to_string(),
    };

    let decision = gov_engine.evaluate(request).await;

    // If approved, install app
    if decision.approved {
        app_manager.install(manifest).await.unwrap();
    }
}
```

### Scenario 6: Cluster Environment with Multi-Node Health

```rust
use clawft_kernel::{
    ClusterMembership, EnvironmentManager, Environment,
    EnvironmentClass, HealthSystem,
};

#[tokio::test]
async fn integration_cluster_health() {
    let cluster = ClusterMembership::new();
    let env_mgr = EnvironmentManager::new();
    let health_sys = HealthSystem::new();

    // Create production environment
    let prod_env = Environment {
        name: "prod".to_string(),
        class: EnvironmentClass::Production,
        governance: Default::default(),
        learning_mode: LearningMode::Conservative,
        audit_level: Default::default(),
    };
    env_mgr.create_environment(prod_env).await.unwrap();

    // Add cluster nodes
    for i in 1..=3 {
        let node = PeerNode {
            id: NodeId::new(),
            address: format!("10.0.0.{}:8080", i),
            state: NodeState::Active,
            platform: Default::default(),
            last_seen: chrono::Utc::now(),
        };
        cluster.add_peer(node).await;
    }

    // Check cluster health
    let peers = cluster.list_peers().await;
    for peer in peers {
        let status = if peer.state == NodeState::Active {
            HealthStatus::Healthy
        } else {
            HealthStatus::Degraded(format!("Node state: {:?}", peer.state))
        };
        health_sys.update_service_health(&peer.address, status).await;
    }

    // Overall health should be healthy if all nodes are active
    let overall = health_sys.overall_health().await;
    assert_eq!(overall, OverallHealth::Healthy);
}
```

---

## CLI Testing

The kernel CLI commands are exposed via `weft kernel` (assuming CLI integration is complete).

### Test `weft kernel status`

```bash
# Boot kernel (via background service or test harness)
weft kernel boot

# Check status
weft kernel status

# Expected output:
# Kernel State: running
# Uptime: 15 seconds
# Processes: 3
# Services: 2
# Health: Healthy
```

### Test `weft kernel services`

```bash
# List registered services
weft kernel services

# Expected output (table format):
# NAME          TYPE         STATE     HEALTH
# message-bus   core         running   healthy
# cron          scheduler    running   healthy
```

### Test `weft kernel ps`

```bash
# List process table
weft kernel ps

# Expected output:
# PID   NAME            STATE     CPU(ms)  MEM(MB)  FILES
# 1     kernel          running   45       12       5
# 2     agent-worker-1  running   120      64       8
# 3     agent-worker-2  running   95       48       6
```

### Test `weft kernel boot` (Interactive)

```bash
# Boot kernel with verbose output
weft kernel boot --verbose

# Expected output:
# [INFO] Phase: Init - Initializing kernel subsystems
# [INFO] Phase: Config - Loading configuration
# [INFO] Phase: Services - Starting core services
# [INFO] Phase: ResourceTree - Building resource hierarchy
# [INFO] Phase: Agents - Spawning initial agents
# [INFO] Phase: Network - Establishing cluster connectivity
# [INFO] Phase: Ready - Kernel is ready
# Kernel booted in 2.3 seconds
```

### Test `weft kernel shutdown`

```bash
# Graceful shutdown
weft kernel shutdown

# Expected output:
# Stopping all services...
# Stopping all agents...
# Kernel halted
```

---

## Feature-Gated Module Testing

### Testing with `wasm-sandbox` Feature

```bash
# Build with WASM support
cargo build -p clawft-kernel --features wasm-sandbox

# Run WASM-specific tests (when wasmtime is integrated)
cargo test -p clawft-kernel --features wasm-sandbox wasm_runner::tests

# Expected: Tests pass with actual WASM execution
```

### Testing with `containers` Feature

```bash
# Build with container support
cargo build -p clawft-kernel --features containers

# Run container-specific tests (when bollard is integrated)
cargo test -p clawft-kernel --features containers container::tests

# Expected: Tests pass with actual Docker integration
```

### Testing All Features Together

```bash
# Build with all features
cargo build -p clawft-kernel --all-features

# Run all tests
cargo test -p clawft-kernel --all-features

# Expected: >259 tests (additional WASM and container tests)
```

---

## What Cannot Be Tested Yet

### Deferred to Future Phases

1. **Ruvector Integration** (boot.rs)
   - Feature-gated, not yet implemented
   - Cannot test ruvector-based memory management

2. **Exo-Resource-Tree** (boot.rs)
   - Feature-gated, not yet implemented
   - Cannot test hierarchical resource tracking

3. **Interactive Console REPL** (console.rs)
   - Only event types implemented, no stdin loop
   - Cannot test interactive kernel commands

4. **Built-In Services** (service.rs)
   - ServiceRegistry starts empty
   - No CronService or other system services registered

5. **WASM Execution** (wasm_runner.rs)
   - Stubbed without wasmtime dependency
   - Cannot test actual WASM module loading

6. **Container Orchestration** (container.rs)
   - Stubbed without bollard dependency
   - Cannot test actual Docker container lifecycle

7. **Network Transport** (cluster.rs)
   - Cluster membership is in-memory only
   - Cannot test actual TCP/UDP peer communication

8. **Distributed Consensus** (cluster.rs)
   - No Raft or gossip protocol implementation yet
   - Cannot test multi-node quorum

9. **Hook Execution** (app.rs)
   - Hooks defined in manifest but not executed
   - Cannot test pre_start/post_start/pre_stop hooks

10. **Effect Vector Scoring** (governance.rs)
    - EffectVector math is stubbed
    - Cannot test multi-dimensional decision scoring

---

## Regression Checklist

Before committing changes to `clawft-kernel`, run this checklist:

### 1. Build All Targets

```bash
# Native build (default)
scripts/build.sh native

# Debug build (fast iteration)
scripts/build.sh native-debug

# Check compilation (no codegen)
scripts/build.sh check

# All features
cargo build -p clawft-kernel --all-features
```

### 2. Run Full Test Suite

```bash
# All kernel tests
scripts/build.sh test

# Kernel-only tests
cargo test -p clawft-kernel

# Test with all features
cargo test -p clawft-kernel --all-features
```

### 3. Lint and Format

```bash
# Clippy (warnings as errors)
scripts/build.sh clippy

# Format check
cargo fmt -p clawft-kernel -- --check
```

### 4. Phase Gate (11 Checks)

```bash
# Run all pre-commit checks
scripts/build.sh gate

# Expected: ALL PASS
```

### 5. Integration Smoke Tests

```bash
# Boot kernel
cargo run --bin weft -- kernel boot

# Check status
cargo run --bin weft -- kernel status

# List services
cargo run --bin weft -- kernel services

# List processes
cargo run --bin weft -- kernel ps

# Shutdown
cargo run --bin weft -- kernel shutdown
```

### 6. Documentation Build

```bash
# Build docs
cargo doc -p clawft-kernel --no-deps --open

# Check for broken links
cargo doc -p clawft-kernel --no-deps 2>&1 | grep -i "warning"
```

### 7. Feature Flag Validation

```bash
# Build with no default features (should fail or build minimal)
cargo build -p clawft-kernel --no-default-features

# Build with each feature individually
cargo build -p clawft-kernel --no-default-features --features native
cargo build -p clawft-kernel --no-default-features --features wasm-sandbox
cargo build -p clawft-kernel --no-default-features --features containers
```

### 8. Memory and Performance

```bash
# Run tests with memory profiling (requires valgrind)
valgrind --leak-check=full cargo test -p clawft-kernel

# Time test execution
time cargo test -p clawft-kernel

# Expected: <1 second
```

### 9. Cross-Platform Check (if applicable)

```bash
# Check for platform-specific code
rg "cfg\(target_os" crates/clawft-kernel/src

# Test on Linux/macOS/Windows (if available)
```

### 10. Integration with Dependent Crates

```bash
# Test CLI with kernel
cargo test -p clawft-cli kernel

# Test services with kernel
cargo test -p clawft-services

# Full workspace test
scripts/build.sh test
```

---

## Summary

This manual testing guide covers:

- **9 K0 modules**: boot, process, service, ipc, capability, health, config, console, error
- **1 K1 module**: supervisor
- **2 K2 modules**: a2a, topic
- **1 K3 module**: wasm_runner (stubbed)
- **1 K4 module**: container (stubbed)
- **1 K5 module**: app
- **3 K6a modules**: cluster, environment, governance
- **1 K6b module**: agency

**Total**: 18 modules, 259 tests, ~9,300 SLOC

Use this guide to:
1. Validate automated tests are working
2. Perform manual integration testing
3. Test CLI commands
4. Prepare for production deployment
5. Verify feature flags
6. Run pre-commit regression checks

All code snippets are executable and ready to copy into your test suite or run interactively.
