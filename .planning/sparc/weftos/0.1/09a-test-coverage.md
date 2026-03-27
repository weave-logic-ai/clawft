# Sprint 09a: Test Coverage & Stability

**Document ID**: 09a
**Workstream**: W-KERNEL
**Duration**: 5 days
**Goal**: Close the test coverage inversion -- ensure the most-depended-on modules have the highest test coverage
**Depends on**: K0-K6 (all prior phases)
**Orchestrator**: `09-orchestrator.md`
**Priority**: P0 (Critical) -- foundation for all subsequent sprints

---

## S -- Specification

### Problem Statement

The ECC graph analysis identified a critical test coverage inversion: the 8
modules with the highest fan-out (most dependencies, most downstream impact)
have the fewest tests. Combined, these 8 modules represent 14,724 lines of
code with only 6 tests total.

**Source data**: `.weftos/analysis/gap-report.json` ranks 6-20,
`docs/weftos/09-symposium/01-graph-findings.md` section 2.

### High-Risk Module Inventory

| Module | Crate | Lines | Tests | Fan-Out | Risk Score |
|--------|-------|------:|------:|:-------:|:----------:|
| `wasm_runner.rs` | clawft-kernel | 4,423 | 0 | 12 | CRITICAL |
| `chain.rs` | clawft-kernel | 2,814 | 0 | 8 | CRITICAL |
| `boot.rs` | clawft-kernel | 1,953 | 2 | 19 | CRITICAL |
| `daemon.rs` | clawft-weave | 1,647 | 1 | 7 | HIGH |
| `a2a.rs` | clawft-kernel | 1,452 | 1 | 11 | HIGH |
| `agent_loop.rs` | clawft-kernel | 1,412 | 0 | 9 | CRITICAL |
| `host.rs` | clawft-channels | 561 | 0 | 4 | HIGH |
| `agent_bus.rs` | clawft-core | 462 | 1 | 5 | MEDIUM |

**File paths**:
- `crates/clawft-kernel/src/wasm_runner.rs`
- `crates/clawft-kernel/src/chain.rs`
- `crates/clawft-kernel/src/boot.rs`
- `crates/clawft-weave/src/daemon.rs`
- `crates/clawft-kernel/src/a2a.rs`
- `crates/clawft-kernel/src/agent_loop.rs`
- `crates/clawft-channels/src/host.rs`
- `crates/clawft-core/src/agent_bus.rs`

### Feature Gate

No new feature gates required. Tests use existing feature gates:
- `#[cfg(test)]` for all unit tests
- `#[cfg(feature = "exochain")]` for chain.rs tests
- `#[cfg(feature = "mesh")]` for mesh-related integration tests

---

## P -- Pseudocode

### Test Strategy Per Module

Each module gets a behavioral test suite focused on the public API and
failure paths, not internal implementation details.

```
For each high-risk module:
  1. Identify public API surface (pub fn, pub struct methods)
  2. Enumerate success paths (normal operation)
  3. Enumerate failure paths (invalid input, resource exhaustion, concurrency)
  4. Write one test per behavior, not per line
  5. Integration tests for cross-module interactions
```

### boot.rs Test Strategy (15+ tests)

```
// boot.rs has 12 distinct boot stages (from 08-orchestrator.md):
//   1. ProcessTable
//   2. KernelIpc + MessageBus
//   3. A2ARouter + TopicRouter
//   4. ServiceRegistry
//   5. HealthSystem
//   6. AgentSupervisor
//   7. WasmToolRunner
//   8. AppManager
//   9. CronService
//   10. GovernanceEngine
//   11. ChainManager + TreeManager
//   12. Mesh (if enabled)

test boot_stage_1_process_table_registered:
    kernel = TestKernel::boot_minimal()
    assert kernel.process_table() is not None
    assert kernel.process_table().count() >= 1  // kernel process

test boot_stage_2_ipc_operational:
    kernel = TestKernel::boot_minimal()
    msg = KernelMessage::test()
    result = kernel.ipc().send(msg)
    assert result is Ok

test boot_stage_4_service_registry_populated:
    kernel = TestKernel::boot_full()
    services = kernel.service_registry().list()
    assert services.len() >= 8  // minimum core services

test boot_stage_11_chain_manager_if_exochain:
    kernel = TestKernel::boot_with_features(["exochain"])
    chain = kernel.service_registry().lookup("chain_manager")
    assert chain is Some
    assert chain.health() == Healthy

test boot_all_stages_complete:
    kernel = TestKernel::boot_full()
    for service in kernel.service_registry().list():
        assert service.health() == Healthy

test boot_handles_missing_config:
    result = TestKernel::boot_with_config(None)
    assert result uses default config

test boot_handles_service_registration_failure:
    // Inject a service that fails to start
    kernel = TestKernel::boot_with_failing_service("mock_fail")
    assert kernel.health_system().has_degraded_service("mock_fail")
```

### agent_loop.rs Test Strategy (20+ tests)

```
test agent_loop_starts_and_stops:
    loop = AgentLoop::new(mock_agent(), mock_inbox())
    handle = loop.start()
    handle.stop_graceful()
    assert handle.exit_code() == 0

test agent_loop_processes_messages:
    loop = AgentLoop::new(mock_agent(), mock_inbox())
    inbox.send(KernelMessage::test())
    handle = loop.start()
    wait_for(|| mock_agent.received_count() == 1)
    handle.stop_graceful()

test agent_loop_respects_cancellation_token:
    loop = AgentLoop::new(mock_agent(), mock_inbox())
    token = CancellationToken::new()
    handle = loop.start_with_token(token.clone())
    token.cancel()
    assert handle.await_exit().is_ok()

test agent_loop_reports_resource_usage:
    loop = AgentLoop::new(mock_agent(), mock_inbox())
    handle = loop.start()
    inbox.send_many(10)
    wait_for(|| mock_agent.received_count() == 10)
    usage = handle.resource_usage()
    assert usage.messages_received == 10

test agent_loop_handles_agent_panic:
    loop = AgentLoop::new(panicking_agent(), mock_inbox())
    handle = loop.start()
    result = handle.await_exit()
    assert result.is_err()  // JoinError from panic

test agent_loop_handles_inbox_full:
    loop = AgentLoop::new(slow_agent(), bounded_inbox(1))
    handle = loop.start()
    for _ in 0..100:
        inbox.try_send(KernelMessage::test())
    // Should not panic or deadlock
    handle.stop_graceful()
```

### chain.rs Test Strategy (15+ tests)

```
test chain_append_and_verify:
    chain = ChainManager::new_in_memory()
    chain.append("test", "agent.spawn", payload)
    assert chain.height() == 1
    assert chain.verify_all().is_ok()

test chain_hash_integrity:
    chain = ChainManager::new_in_memory()
    chain.append("test", "event1", payload1)
    chain.append("test", "event2", payload2)
    entries = chain.entries()
    assert entries[1].prev_hash == entries[0].hash

test chain_tamper_detection:
    chain = ChainManager::new_in_memory()
    chain.append("test", "event1", payload1)
    // Manually tamper with entry
    chain.raw_modify(0, |e| e.payload = tampered_payload)
    assert chain.verify_all().is_err()

test chain_segment_boundary:
    chain = ChainManager::new_in_memory_with_segment_size(10)
    for i in 0..25:
        chain.append("test", format!("event_{}", i), payload)
    assert chain.segment_count() == 3
    assert chain.verify_all().is_ok()

test chain_rvf_roundtrip:
    chain = ChainManager::new_in_memory()
    chain.append("test", "event1", payload)
    rvf_bytes = chain.export_rvf()
    imported = ChainManager::import_rvf(rvf_bytes)
    assert imported.height() == chain.height()
    assert imported.entries()[0].hash == chain.entries()[0].hash

test chain_merkle_root_consistency:
    chain = ChainManager::new_in_memory()
    for i in 0..8:
        chain.append("test", format!("event_{}", i), payload)
    root1 = chain.merkle_root()
    root2 = chain.merkle_root()  // idempotent
    assert root1 == root2
```

### wasm_runner.rs Test Strategy (20+ tests)

```
test wasm_runner_executes_valid_module:
    runner = WasmToolRunner::new(default_config())
    module = compile_test_wasm("add.wasm")
    result = runner.execute(module, "add", &[Val::I32(2), Val::I32(3)])
    assert result == Ok([Val::I32(5)])

test wasm_runner_fuel_exhaustion:
    runner = WasmToolRunner::new(Config { fuel_limit: 100, .. })
    module = compile_test_wasm("infinite_loop.wasm")
    result = runner.execute(module, "run", &[])
    assert result.is_err()
    assert result.error_kind() == FuelExhausted

test wasm_runner_memory_limit:
    runner = WasmToolRunner::new(Config { memory_limit: 1024, .. })
    module = compile_test_wasm("memory_hog.wasm")
    result = runner.execute(module, "allocate_1mb", &[])
    assert result.is_err()

test wasm_runner_capability_denial:
    caps = CapabilitySet::empty()
    runner = WasmToolRunner::new_with_caps(default_config(), caps)
    module = compile_test_wasm("file_reader.wasm")
    result = runner.execute(module, "read_file", &[Val::String("/etc/passwd")])
    assert result.is_err()
    assert result.error_kind() == CapabilityDenied

test wasm_runner_capability_grant:
    caps = CapabilitySet::with_fs_read(&["/data/*"])
    runner = WasmToolRunner::new_with_caps(default_config(), caps)
    module = compile_test_wasm("file_reader.wasm")
    result = runner.execute(module, "read_file", &[Val::String("/data/test.txt")])
    assert result.is_ok()

test wasm_runner_invalid_module:
    runner = WasmToolRunner::new(default_config())
    result = runner.execute(b"not a wasm module", "run", &[])
    assert result.is_err()

test wasm_runner_missing_export:
    runner = WasmToolRunner::new(default_config())
    module = compile_test_wasm("add.wasm")
    result = runner.execute(module, "nonexistent_fn", &[])
    assert result.is_err()

test wasm_runner_concurrent_execution:
    runner = Arc::new(WasmToolRunner::new(default_config()))
    handles = (0..10).map(|_| {
        let r = runner.clone()
        spawn(async { r.execute(add_module, "add", &[1, 2]) })
    })
    results = join_all(handles)
    assert results.iter().all(|r| r.is_ok())
```

### a2a.rs Test Strategy (10+ tests)

```
test a2a_route_to_pid:
    router = A2ARouter::new(process_table)
    process_table.spawn_test_agent("agent-1", pid=42)
    msg = KernelMessage::new(0, Process(42), payload)
    result = router.route(msg)
    assert result.is_ok()

test a2a_route_to_nonexistent_pid:
    router = A2ARouter::new(process_table)
    msg = KernelMessage::new(0, Process(999), payload)
    result = router.route(msg)
    assert result.is_err()  // or routes to DLQ when available

test a2a_topic_broadcast:
    router = A2ARouter::new(process_table)
    router.subscribe(42, "events.test")
    router.subscribe(43, "events.test")
    msg = KernelMessage::new(0, Topic("events.test"), payload)
    result = router.route(msg)
    assert agent_42.received_count() == 1
    assert agent_43.received_count() == 1

test a2a_route_to_service:
    router = A2ARouter::new(process_table)
    service_registry.register("my_service", pid=42)
    msg = KernelMessage::new(0, Service("my_service"), payload)
    result = router.route(msg)
    assert result.is_ok()

test a2a_backpressure:
    router = A2ARouter::new(process_table)
    // Fill inbox to capacity
    for _ in 0..1000:
        router.route(msg_to_slow_agent)
    // Router should handle backpressure without panic
    assert router.stats().dropped_or_backed > 0
```

---

## A -- Architecture

### Test Infrastructure

All tests share a `TestKernel` helper that boots a minimal kernel in-process:

```rust
// crates/clawft-kernel/src/test_helpers.rs (or tests/common/mod.rs)
pub struct TestKernel {
    handle: KernelHandle,
    cancel: CancellationToken,
}

impl TestKernel {
    /// Boot with default features (native only)
    pub async fn boot_minimal() -> Self { ... }

    /// Boot with all features enabled
    pub async fn boot_full() -> Self { ... }

    /// Boot with specific features
    pub async fn boot_with_features(features: &[&str]) -> Self { ... }

    /// Access kernel subsystems
    pub fn process_table(&self) -> &ProcessTable { ... }
    pub fn ipc(&self) -> &KernelIpc { ... }
    pub fn service_registry(&self) -> &ServiceRegistry { ... }
}
```

### Test File Organization

```
crates/clawft-kernel/src/
  boot.rs                  -- inline #[cfg(test)] mod tests { ... }
  agent_loop.rs            -- inline #[cfg(test)] mod tests { ... }
  chain.rs                 -- inline #[cfg(test)] mod tests { ... }
  wasm_runner.rs           -- inline #[cfg(test)] mod tests { ... }
  a2a.rs                   -- inline #[cfg(test)] mod tests { ... }

crates/clawft-weave/src/
  daemon.rs                -- inline #[cfg(test)] mod tests { ... }

crates/clawft-channels/src/
  host.rs                  -- inline #[cfg(test)] mod tests { ... }

crates/clawft-core/src/
  agent_bus.rs             -- inline #[cfg(test)] mod tests { ... }

crates/clawft-kernel/tests/
  feature_composition.rs   -- cross-feature integration tests
```

### Feature Composition Test Matrix

```rust
// crates/clawft-kernel/tests/feature_composition.rs

#[cfg(all(feature = "native", feature = "ecc", feature = "mesh"))]
#[tokio::test]
async fn test_native_ecc_mesh_composition() {
    let kernel = TestKernel::boot_with_features(&["native", "ecc", "mesh"]).await;
    // Verify all services register without conflict
    let services = kernel.service_registry().list();
    assert!(services.iter().any(|s| s.name() == "causal_graph"));
    assert!(services.iter().any(|s| s.name() == "hnsw_service"));
    // Mesh services only if mesh feature
    assert!(services.iter().any(|s| s.name() == "mesh_listener"));
}

#[cfg(all(feature = "native", feature = "os-patterns"))]
#[tokio::test]
async fn test_native_os_patterns_composition() {
    let kernel = TestKernel::boot_with_features(&["native", "os-patterns"]).await;
    // os-patterns implies exochain
    let chain = kernel.service_registry().lookup("chain_manager");
    assert!(chain.is_some());
}

#[cfg(all(feature = "native", feature = "exochain", feature = "ecc"))]
#[tokio::test]
async fn test_exochain_ecc_no_conflict() {
    // Both use blake3 and ed25519-dalek -- verify no version conflict
    let kernel = TestKernel::boot_with_features(&["native", "exochain", "ecc"]).await;
    assert!(kernel.service_registry().lookup("chain_manager").is_some());
    assert!(kernel.service_registry().lookup("causal_graph").is_some());
}
```

### Orphan Wiring Plan

Wire the top 20 orphan modules (by size) into the Weaver's causal graph by
adding `Uses`, `EvidenceFor`, and `GatedBy` edges in
`.weftos/graph/module-deps.json`:

| Orphan Module | Edge Type | Target | Rationale |
|--------------|-----------|--------|-----------|
| boot.rs | Uses | process.rs, service.rs, a2a.rs, ... (19 deps) | Boot sequence dependencies |
| tree_manager.rs | Uses | chain.rs | ExoChain tree storage |
| app.rs | Uses | supervisor.rs, service.rs | App lifecycle |
| engine.rs (wasm) | Uses | wasm_runner.rs | WASM execution engine |
| daemon.rs | Uses | boot.rs, ipc.rs | Weave daemon wraps kernel |
| routing_validation.rs | EvidenceFor | a2a.rs | Validates routing |
| a2a.rs | Uses | process.rs, ipc.rs | Agent-to-agent routing |
| agent_loop.rs | Uses | process.rs, supervisor.rs | Agent execution |
| weaver.rs | Uses | causal.rs, hnsw_service.rs | Weaver engine |
| process.rs | Uses | ipc.rs | Process management |

---

## R -- Refinement

### Early Validation Step

Before writing any tests, verify feature composition compiles:

```bash
# Day 1 morning: verify all compositions build
cargo check -p clawft-kernel --features native
cargo check -p clawft-kernel --features native,ecc
cargo check -p clawft-kernel --features native,mesh
cargo check -p clawft-kernel --features native,ecc,mesh
cargo check -p clawft-kernel --features native,os-patterns
cargo check -p clawft-kernel --features native,ecc,mesh,os-patterns
```

If any fail, fix the compilation issue before proceeding with tests.

### Performance Considerations

- Tests should complete in <30 seconds total for the kernel crate
- `TestKernel::boot_minimal()` should take <100ms (no I/O, no network)
- WASM tests need pre-compiled test modules (not compiled at test time)
- Chain tests use in-memory storage (no disk I/O)

### Testing Restart Strategies Without Real Crashes

Per process-supervisor's guidance:

```rust
#[tokio::test]
async fn test_restart_on_panic() {
    let supervisor = TestSupervisor::new(RestartStrategy::OneForOne);
    let agent = supervisor.spawn_mock_agent("test-agent");

    // Trigger controlled panic
    agent.inject_panic("test panic");

    // Verify supervisor handles the JoinError
    tokio::time::sleep(Duration::from_millis(200)).await;
    let status = supervisor.agent_status("test-agent");
    assert_eq!(status, AgentStatus::Running);  // restarted
    assert_eq!(supervisor.restart_count("test-agent"), 1);
}
```

### Security Considerations

- WASM tests must not execute untrusted modules -- use only pre-built test WATs
- Chain tests must verify tamper detection (hash integrity)
- Capability tests must verify denial before grant

---

## C -- Completion

### Work Packages

#### WP-1: Test Infrastructure (Day 1)

**Owner**: test-sentinel
**Reviewer**: kernel-architect

- Create `TestKernel` boot helper
- Create mock agent, inbox, and service helpers
- Verify all 6 feature compositions compile
- Estimated: ~300 lines of test infrastructure

#### WP-2: boot.rs Tests (Day 1-2)

**Owner**: test-sentinel
**Reviewer**: process-supervisor

- 15+ tests covering 12 boot stages
- Test: each stage's service is registered and healthy
- Test: boot handles missing config (uses defaults)
- Test: boot handles service registration failure (degraded not crashed)
- File: `crates/clawft-kernel/src/boot.rs` (inline test module)
- Estimated: ~400 lines of test code

#### WP-3: agent_loop.rs Tests (Day 2)

**Owner**: test-sentinel
**Reviewer**: process-supervisor

- 20+ tests covering agent execution lifecycle
- Test: start/stop, message processing, cancellation
- Test: resource usage tracking, panic handling
- Test: inbox full handling, concurrent message delivery
- File: `crates/clawft-kernel/src/agent_loop.rs` (inline test module)
- Estimated: ~500 lines of test code

#### WP-4: chain.rs Tests (Day 2-3)

**Owner**: test-sentinel
**Reviewer**: chain-guardian

- 15+ tests covering chain integrity
- Mandatory categories (per chain-guardian review):
  - Hash chain integrity (append-then-verify, tamper detection, segment boundary)
  - RVF serialization roundtrip (export, import, verify)
  - Merkle root consistency
  - Event type serialization
  - Concurrent append safety
- File: `crates/clawft-kernel/src/chain.rs` (inline test module, `#[cfg(feature = "exochain")]`)
- Estimated: ~400 lines of test code

#### WP-5: wasm_runner.rs Tests (Day 3)

**Owner**: test-sentinel
**Reviewer**: sandbox-warden

- 20+ tests covering three sandbox layers (per sandbox-warden):
  - Layer 1: Fuel metering (exhaustion, budget tracking)
  - Layer 2: Memory limits (allocation cap, OOM handling)
  - Layer 3: Capability enforcement (deny, grant, scope validation)
- Also: module loading (valid, invalid, missing export), concurrent execution
- File: `crates/clawft-kernel/src/wasm_runner.rs` (inline test module)
- Pre-build test WASM modules as WAT files in `crates/clawft-kernel/tests/wasm/`
- Estimated: ~500 lines of test code

#### WP-6: a2a.rs + daemon.rs + host.rs + agent_bus.rs Tests (Day 4)

**Owner**: test-sentinel
**Reviewer**: kernel-architect, mesh-engineer

- a2a.rs: 10+ tests (routing, topic broadcast, service dispatch, backpressure)
- daemon.rs: 10+ tests (socket IPC lifecycle, command dispatch)
- host.rs: 8+ tests (channel host operations, lifecycle)
- agent_bus.rs: 5+ tests (bus send/recv, subscription)
- Files: respective source files (inline test modules)
- Estimated: ~600 lines of test code

#### WP-7: Feature Composition Tests (Day 4-5)

**Owner**: test-sentinel
**Reviewer**: mesh-engineer

- Create `crates/clawft-kernel/tests/feature_composition.rs`
- Test: `native + ecc + mesh` -- all services register without conflict
- Test: `native + os-patterns` -- exochain implied, chain service available
- Test: `native + exochain + ecc` -- no version conflicts
- Estimated: ~200 lines of test code

#### WP-8: Orphan Module Wiring (Day 5)

**Owner**: ecc-analyst
**Reviewer**: kernel-architect

- Update `.weftos/graph/module-deps.json` with edges for top 20 orphan modules
- Each edge needs: source node, target node, edge type, confidence score
- Reduce orphan count from 116 to <96
- Estimated: ~200 lines of JSON additions

### Exit Criteria

- [ ] `boot.rs` has 15+ tests covering all 12 boot stages
- [ ] `agent_loop.rs` has 20+ tests covering agent execution lifecycle
- [ ] `chain.rs` has 15+ tests covering append, verify, segment, merkle
- [ ] `wasm_runner.rs` has 20+ tests covering fuel, memory, capability enforcement
- [ ] `a2a.rs` has 10+ tests covering routing, topic dispatch, error paths
- [ ] `daemon.rs` (clawft-weave) has 10+ tests covering socket IPC, lifecycle
- [ ] `host.rs` (clawft-channels) has 8+ tests covering channel host operations
- [ ] `agent_bus.rs` (clawft-core) has 5+ tests covering bus send/recv
- [ ] Feature gate composition: `native + ecc + mesh` builds and tests clean
- [ ] Feature gate composition: `native + os-patterns` builds and tests clean
- [ ] Feature gate composition: `native + exochain + ecc` builds and tests clean
- [ ] Orphan module count reduced by 20+ (from 116 to <96)
- [ ] All existing 1,197 tests still pass (no regressions)
- [ ] Total kernel test count reaches 1,320+ (120+ new)
- [ ] `scripts/build.sh clippy` clean for all modified files

### Agent Assignment

| Agent | Role | Work Packages |
|-------|------|---------------|
| **test-sentinel** | Primary implementer | WP-1 through WP-7 |
| **kernel-architect** | Reviewer | WP-1 (infra), WP-6 (a2a), WP-7 (composition), WP-8 (wiring) |
| **process-supervisor** | Reviewer + domain expert | WP-2 (boot), WP-3 (agent_loop) |
| **chain-guardian** | Reviewer + domain expert | WP-4 (chain) |
| **sandbox-warden** | Reviewer + domain expert | WP-5 (wasm_runner) |
| **mesh-engineer** | Reviewer + domain expert | WP-6 (host.rs), WP-7 (composition) |
| **ecc-analyst** | Primary implementer | WP-8 (orphan wiring) |
| **app-deployer** | Reviewer | WP-6 (agent_bus.rs) |

### Expert Review Notes

**test-sentinel**: "Focus on behavioral coverage, not line coverage. boot.rs
has 1,953 lines but only 12 meaningful behaviors (boot stages). Testing each
stage's registration and health is sufficient. For wasm_runner.rs, the three
sandbox layers (fuel, memory, capabilities) are the security boundaries -- test
those, not internal parsing."

**kernel-architect**: "TestKernel boot helper is critical infrastructure. It
must support selective feature enablement and be usable by all subsequent
sprints. Design it as a workspace-level test utility, not kernel-specific."

**process-supervisor**: "Agent restart tests use controlled panics via
`JoinHandle::abort()` or injected panic messages. The `tokio::task::JoinError`
from a panicked task triggers the same restart path as a real crash. No actual
process crashes needed."

**chain-guardian**: "chain.rs tests MUST include: (1) append-then-verify, (2)
tampered entry detection, (3) segment boundary verification, (4) RVF
serialization roundtrip, (5) merkle root consistency. These are non-negotiable
for chain integrity assurance."

**sandbox-warden**: "wasm_runner.rs tests must cover all three layers: fuel
(instruction budget), memory (allocation cap), and capability enforcement
(fs_read, net_connect, ipc_send denial and grant). A test that only covers
fuel without capabilities is incomplete."

**mesh-engineer**: "Test `mesh + os-patterns` compilation first, before writing
tests. Both features bring in `ed25519-dalek`. If there is a version conflict,
fix it in Cargo.toml before proceeding."

### Testing Verification Commands

```bash
# Build with all features to verify composition
scripts/build.sh check

# Run all tests
scripts/build.sh test

# Run kernel tests specifically
cargo test -p clawft-kernel --features native,ecc,mesh,exochain

# Run chain tests (requires exochain feature)
cargo test -p clawft-kernel --features exochain -- chain

# Verify test count
cargo test --workspace 2>&1 | grep "test result"

# Clippy
scripts/build.sh clippy
```

### Implementation Order

```
Day 1:
  WP-1: TestKernel infrastructure + feature composition validation
  WP-2: boot.rs tests (first batch)

Day 2:
  WP-2: boot.rs tests (complete)
  WP-3: agent_loop.rs tests

Day 3:
  WP-4: chain.rs tests
  WP-5: wasm_runner.rs tests (first batch)

Day 4:
  WP-5: wasm_runner.rs tests (complete)
  WP-6: a2a.rs, daemon.rs, host.rs, agent_bus.rs tests

Day 5:
  WP-7: Feature composition integration tests
  WP-8: Orphan module wiring
  Final regression check + clippy
```
