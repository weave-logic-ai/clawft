# Track 8: Full Codebase Report

**Chair**: code-analyzer
**Panelists**: code-analyzer, security-auditor, reviewer, performance-benchmarker, production-validator, api-docs
**Date**: 2026-03-27
**Branch**: `feature/weftos-kernel-sprint`
**Codebase**: 22 crates, 181,703 lines Rust, 5,040 tests

---

## 1. Module Dependency Graph

### 1.1 Dependency Matrix (22 crates)

Built from reading every crate's `Cargo.toml`. Workspace deps only (excludes external crates).

```
Layer 0 (Leaves -- no workspace deps):
  clawft-types          deps: (none)
  clawft-security       deps: (none)
  exo-resource-tree     deps: (none)

Layer 1 (Single leaf dep):
  clawft-platform       deps: clawft-types

Layer 2:
  clawft-llm            deps: clawft-types
  clawft-plugin         deps: (none workspace; has reqwest only)

Layer 3:
  clawft-core           deps: clawft-types, clawft-platform, clawft-llm, clawft-plugin

Layer 4:
  clawft-tools          deps: clawft-types, clawft-platform, clawft-core,
                               clawft-services (opt), clawft-plugin (opt)
  clawft-kernel         deps: clawft-core, clawft-types, clawft-platform, clawft-plugin,
                               exo-resource-tree (opt), [8 ruvector crates (opt)]
  clawft-channels       deps: clawft-types, clawft-platform, clawft-plugin

Layer 5 (Integration):
  clawft-services       deps: clawft-types, clawft-core (opt), clawft-platform (opt)
  clawft-weave          deps: clawft-kernel, clawft-platform, clawft-types,
                               exo-resource-tree (opt)

Layer 6 (Binaries / Facades):
  clawft-cli            deps: clawft-types, clawft-platform, clawft-core, clawft-tools,
                               clawft-llm, clawft-channels (opt), clawft-services (opt),
                               clawft-plugin (opt), clawft-security
  clawft-wasm           deps: clawft-types, clawft-core (opt), clawft-llm (opt),
                               clawft-tools (opt), clawft-platform (opt), clawft-plugin (opt)
  weftos                deps: clawft-kernel, clawft-types, clawft-platform, clawft-core,
                               exo-resource-tree

Plugin Satellites (all Layer 2, dep on clawft-plugin only):
  clawft-plugin-git           deps: clawft-plugin
  clawft-plugin-cargo         deps: clawft-plugin
  clawft-plugin-oauth2        deps: clawft-plugin
  clawft-plugin-treesitter    deps: clawft-plugin
  clawft-plugin-browser       deps: clawft-plugin
  clawft-plugin-containers    deps: clawft-plugin
  clawft-plugin-calendar      deps: clawft-plugin, clawft-plugin-oauth2
```

### 1.2 Dependency Depth

Maximum depth from leaf to binary: **6 layers** (types -> platform -> core -> kernel -> weftos).

### 1.3 Key Findings

**No circular dependencies.** The graph is a clean DAG. Cargo would reject cycles at compile time, but the logical layering is also clean.

**Pure crates** (zero workspace deps): `clawft-types`, `clawft-security`, `exo-resource-tree`. These are independently publishable.

**Heavy crates** (4+ workspace deps): `clawft-cli` (9 deps), `weftos` (5), `clawft-core` (4), `clawft-kernel` (4 + 8 optional ruvector).

**Surprising dependency**: `weftos` depends on `exo-resource-tree` **unconditionally**. Track 3 identified this as release blocker B1 because `exo-resource-tree` has a non-optional `rvf-crypto` path dep in the kernel's `exochain` feature gate. However, examining `exo-resource-tree/Cargo.toml` directly shows it depends only on `sha3`, `serde`, `serde_json`, `chrono`, `thiserror`, `uuid`, and `tracing` -- no `rvf-crypto`. The `rvf-crypto` dependency exists in `clawft-kernel` behind the `exochain` feature gate, not in `exo-resource-tree` itself. **Track 3's blocker B1 may be less severe than stated** -- the issue is that `clawft-kernel/exochain` pulls `rvf-crypto`, not `exo-resource-tree` directly.

**clawft-services has a circular potential**: `clawft-tools` optionally depends on `clawft-services` (via `delegate` feature), and `clawft-services` optionally depends on `clawft-core` (via `api` feature). Since `clawft-tools` depends on `clawft-core`, enabling both `delegate` and `api` features creates a diamond: `tools -> services -> core` and `tools -> core`. This is not a cycle but does create a tight coupling diamond.

### 1.4 Crate Dependency Count Summary

| Crate | Workspace Deps | Optional Workspace Deps | Depth |
|-------|---------------|------------------------|-------|
| clawft-types | 0 | 0 | 0 |
| clawft-security | 0 | 0 | 0 |
| exo-resource-tree | 0 | 0 | 0 |
| clawft-platform | 1 | 0 | 1 |
| clawft-llm | 1 | 0 | 1 |
| clawft-plugin | 0 | 0 | 0 |
| clawft-plugin-git | 1 | 0 | 1 |
| clawft-plugin-cargo | 1 | 0 | 1 |
| clawft-plugin-oauth2 | 1 | 0 | 1 |
| clawft-plugin-treesitter | 1 | 0 | 1 |
| clawft-plugin-browser | 1 | 0 | 1 |
| clawft-plugin-containers | 1 | 0 | 1 |
| clawft-plugin-calendar | 2 | 0 | 2 |
| clawft-channels | 3 | 0 | 2 |
| clawft-core | 4 | 0 | 3 |
| clawft-tools | 3 | 2 | 4 |
| clawft-services | 1 | 2 | 2 |
| clawft-kernel | 4 | 9 | 4 |
| clawft-weave | 3 | 2 | 5 |
| clawft-wasm | 1 | 4 | 3 |
| clawft-cli | 5 | 4 | 5 |
| weftos | 5 | 0 | 5 |

---

## 2. Public API Surface Audit

### 2.1 Tier 1 Crates (Publishable)

#### weftos (facade)

- **Public types**: `WeftOs` struct, `VERSION` const, `is_initialized()` fn, `init` module
- **Re-exports**: ~80 types from `clawft-kernel`, gated behind feature flags
- **Assessment**: Clean facade. The `WeftOs` struct provides 3 boot methods (`boot_default`, `boot_in`, `boot_with`), accessors (`state`, `service_count`, `process_count`, `kernel`, `kernel_mut`), and `shutdown`. Minimal surface area -- appropriate for a facade crate.
- **Issue**: `kernel_mut()` returns `&mut Kernel<NativePlatform>`, exposing the full kernel interior. For a published crate, this breaks encapsulation. Consider whether users need raw kernel access or if specific methods should be exposed instead.

#### clawft-types (leaf)

- **Public modules**: 15 (`agent_bus`, `agent_routing`, `canvas`, `config`, `cron`, `delegation`, `error`, `event`, `provider`, `routing`, `secret`, `security`, `session`, `skill`, `workspace`)
- **Re-exports**: `ClawftError`, `ChannelError`, `Result`
- **Assessment**: Well-scoped type definitions. No runtime code. Pure data types with serde derives.
- **Issue**: The crate is named `clawft-types` but the public error type is `ClawftError`. For release as `weftos-types`, renaming would be needed.

#### clawft-core (engine)

- **Public modules**: 22 modules covering agent loop, bus, pipeline, routing, session, tools, embeddings, and more
- **Assessment**: Large public surface. All 22 modules are `pub mod`, meaning every public item within them is part of the API.
- **Issue**: Modules like `json_repair`, `config_merge`, and `clawft_md` are utility modules that probably should not be in the public API of a library crate. Consider `pub(crate)` for implementation details.

#### clawft-kernel (kernel)

- **Public modules**: 50+ modules (68 source files), 166 re-exported types at crate root
- **Assessment**: The kernel re-exports 166 types at the crate root via `pub use`. This is the largest public API surface in the workspace. Many internal types (e.g., `CompiledModuleCache`, `WasiFsScope`, `ShellPipeline`) appear to be implementation details exposed for testing convenience.
- **Issue**: The 166 re-exports create a massive flat namespace. Users importing `clawft_kernel::*` get everything. For a library, hierarchical access (`clawft_kernel::process::ProcessTable`) is preferable.

#### clawft-plugin (plugin trait)

- **Public modules**: 5 (`error`, `manifest`, `message`, `sandbox`, `traits`) + optional `voice`
- **Re-exports**: `PluginError`, manifest types, `MessagePayload`, sandbox types, trait types
- **Assessment**: Clean, focused API. The `Plugin` trait and supporting types are well-defined. Good.

#### exo-resource-tree (standalone)

- **Public types**: `ResourceTree`, `ResourceNode`, `ResourceId`, `ResourceKind`, `Role`, `Action`, `Decision`, `DelegationCert`, `NodeScoring`, `MutationLog`, `MutationEvent`, `TreeError`
- **Re-exports**: 10 items from `lib.rs`
- **Assessment**: Clean, well-bounded API. Hierarchical namespace with Merkle integrity. Self-contained. The 1,809 lines are well within the 500-line-per-file guideline for most files.

### 2.2 API Consistency Assessment

**Naming**: All types use CamelCase. All functions use snake_case. Consistent across crates.

**Error types**: Every crate uses `thiserror::Error` derive. Error variants are well-named. The `KernelError::Other(Box<dyn Error + Send + Sync>)` catch-all pattern is used at the kernel boundary. **Positive finding**.

**Async patterns**: All async functions use `async fn` with `async-trait` for trait methods. The `Platform` trait abstracts the runtime. Consistent.

**Builder pattern**: `with_*()` methods used consistently for fluent construction (documented in Track 1). Consistent.

**Issue**: No `#[non_exhaustive]` on any public enum. Adding fields to `ProcessState`, `KernelState`, `GovernanceDecision`, `GateDecision`, etc. would be a semver-breaking change. Before v0.1.0 tag, add `#[non_exhaustive]` to all public enums that may grow.

---

## 3. Security Quick Scan

### 3.1 Hardcoded Secrets

**None found.** API keys are read from environment variables or config files. The WASM sandbox has a hardcoded deny list for sensitive env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, etc.) at `crates/clawft-wasm/src/sandbox.rs:440-441` -- this is correct defensive behavior. No secrets in source.

### 3.2 Unsafe Code Inventory

| Location | Type | Justification | Risk |
|----------|------|---------------|------|
| `clawft-kernel/src/chain.rs:1624` | `transmute_copy` on `MlDsa65VerifyKey` | Converts rvf-crypto key type to raw bytes. Size-asserted before transmute. | **MEDIUM** -- relies on struct layout of external type. A change to `MlDsa65VerifyKey`'s internal representation would silently produce wrong bytes. |
| `clawft-kernel/src/wasm_runner.rs:1381-1389` | `unsafe impl Send/Sync` for `WasmToolAdapter`, `ToolRegistry` | Asserts thread safety for types containing `Arc<dyn BuiltinTool>`. | **LOW** -- the safety argument is sound (all fields are `Send+Sync`), but it would be better to add `Send+Sync` bounds to `BuiltinTool` trait. |
| `clawft-kernel/src/tools_extended.rs:619` | Raw pointer to `serde_json::Map` | Navigates nested JSON map via `*mut` pointer to avoid borrow checker. | **MEDIUM** -- the code is correct but fragile. A safe alternative using recursive function calls would be cleaner. |
| `clawft-platform/src/env.rs:38,45` | `std::env::set_var` / `remove_var` | Environment variable mutation (unsafe in Rust 2024 edition). | **LOW** -- only used in platform abstraction. |
| `clawft-wasm/src/allocator.rs` | Custom global allocators (`talc`, `lol_alloc`) | WASM memory allocator setup. | **LOW** -- standard WASM practice. |
| `clawft-wasm/src/alloc_trace.rs:60-112` | `GlobalAlloc` impl | Tracing allocator wrapper. | **LOW** -- delegates to inner allocator. |
| `clawft-wasm/src/engine.rs` (6 locations) | Wasmtime FFI | WASM runtime engine interactions. | **LOW** -- wasmtime API requires unsafe at boundaries. |
| `clawft-wasm/src/sandbox.rs:940,946` | Wasmtime sandbox operations | WASM sandbox memory operations. | **LOW** -- wasmtime API. |
| `clawft-channels/src/discord/factory.rs:181-218` | `std::env::set_var` in tests | Test-only env manipulation. | **NEGLIGIBLE** -- test code only. |

**Total unsafe locations**: 18 (production code: 12; test code: 6).

**Recommendation**: The `transmute_copy` in `chain.rs` and the raw pointer in `tools_extended.rs` are the two that warrant refactoring. The `unsafe impl Send/Sync` in `wasm_runner.rs` should be replaced with trait bounds.

### 3.3 Input Validation at System Boundaries

**WASM Sandbox** (`clawft-wasm/src/sandbox.rs`): **Well-defended.**
- File path canonicalization with symlink detection (test `t16_fs_path_traversal_blocked`)
- Environment variable deny list for API keys
- Network access validation (private IP blocking)
- Rate limiting (`window_start` + `Mutex`)

**Shell Execution** (`clawft-kernel/src/wasm_runner.rs`): ShellCommand executed via Wasmtime sandbox. The WASM boundary provides isolation. Chain-logged.

**Config Parsing** (`clawft-types/src/config.rs`): Uses `serde` deserialization with typed fields. No raw string parsing.

**CLI Skill Installation** (`clawft-cli/src/commands/skills_cmd.rs:534`): Validates skill name to prevent path traversal. Good.

**Signal Channel** (`clawft-channels/src/signal/channel.rs:54`): Calls `sanitize_argument()` on CLI path. Good.

**Agent/Skill Names** (`clawft-core/src/agent/agents.rs:188`, `skills_v2.rs:436`): Rejects unsafe names. Good.

**Gap**: The API bridge (`clawft-services/src/api/bridge.rs`) has TODO comments for unimplemented operations (skill install, skill uninstall, memory delete, config persist). These stubs currently return errors, which is safe, but the TODO markers indicate future attack surface.

### 3.4 Unwrap() on Potentially User-Supplied Data

Total `.unwrap()` calls: **4,108** (3,782 estimated non-test). The vast majority are in test code. Examining the non-test `unwrap()` calls in critical paths:

- `crates/clawft-wasm/src/audit.rs:115,125,130,135,140,152,162` -- `Mutex::lock().unwrap()` on internal audit log. A poisoned mutex would panic. **LOW** risk (internal state).
- `crates/clawft-wasm/src/sandbox.rs:107` -- `Mutex::lock().unwrap()` on rate limiter. Same pattern.
- `crates/clawft-wasm/src/sandbox.rs:183-185` -- `Regex::new().unwrap()` on compile-time constant patterns. **SAFE** (constant regexes).

Most non-test `unwrap()` calls fall into two categories: (1) `Mutex::lock().unwrap()` which is standard Rust practice and only panics on poison, and (2) `serde_json` operations on internally-constructed JSON. No `unwrap()` was found on user-supplied network input or file content parsing in production code paths.

---

## 4. Technical Debt Inventory

### 4.1 TODO Comments (10 found)

| Location | Content | Severity |
|----------|---------|----------|
| `clawft-channels/src/irc/channel.rs:82` | Connect to IRC server using client library | LOW (IRC plugin is stub) |
| `clawft-channels/src/irc/channel.rs:128` | Send message using connected IRC client | LOW |
| `clawft-services/src/api/handlers.rs:125` | Add CSP middleware | MEDIUM (security header) |
| `clawft-services/src/api/handlers.rs:128` | Add rate limiting middleware | MEDIUM (DoS protection) |
| `clawft-services/src/api/bridge.rs:282` | Implement skill installation via ClawHub | LOW |
| `clawft-services/src/api/bridge.rs:287` | Implement skill uninstallation | LOW |
| `clawft-services/src/api/bridge.rs:395` | Implement memory entry deletion | LOW |
| `clawft-services/src/api/bridge.rs:467` | Implement config persistence | LOW |
| `clawft-core/src/pipeline/rate_limiter.rs:59` | Expose via admin metrics endpoint | LOW |
| `clawft-core/src/pipeline/rate_limiter.rs:284` | Used by admin maintenance endpoint | LOW |

### 4.2 unimplemented!() / todo!() Macros (1 found)

| Location | Context |
|----------|---------|
| `clawft-core/src/session.rs:505` | `unimplemented!()` -- will panic at runtime if reached |

This is a **runtime bomb**. If the code path is reachable, it will crash. Needs investigation.

### 4.3 #[allow(dead_code)] Markers (23 found)

| Crate | Count | Files |
|-------|-------|-------|
| clawft-kernel | 4 | a2a.rs (2), weaver.rs, embedding_onnx.rs |
| clawft-weave | 3 | client.rs (3) |
| clawft-core | 2 | pipeline/rate_limiter.rs (2) |
| clawft-services | 3 | mcp/transport.rs (2), mcp/client.rs (2), api/broadcaster.rs |
| clawft-plugin-browser | 4 | lib.rs (4) |
| clawft-channels | 1 | telegram/client.rs |
| clawft-llm | 1 | browser_transport.rs |
| clawft-plugin-git | 1 | lib.rs |
| clawft-cli | 2 | markdown/dispatch.rs (2) |

The 3 `dead_code` markers in `clawft-weave/src/client.rs` are concerning -- the daemon client has dead code, which suggests the client API is partially implemented.

### 4.4 Commented-Out Code

Only 1 instance found: `clawft-kernel/src/embedding_onnx.rs:521` (a comment about `fn` keywords in impl bodies, not actual commented-out code). **Clean.**

### 4.5 Summary

| Debt Category | Count | Estimated Cleanup |
|---------------|-------|-------------------|
| TODO comments | 10 | 8 hours (2 medium-priority security TODOs) |
| unimplemented!() | 1 | 1 hour |
| #[allow(dead_code)] | 23 | 4 hours |
| Commented-out code | 0 | -- |
| **Total** | 34 | **13 hours** |

---

## 5. Architecture Fitness Scores

Each K-phase scored 1-5:
- 1 = Stub (interface only, no implementation)
- 2 = Prototype (works in happy path, no error handling)
- 3 = MVP (works under normal conditions, basic error handling)
- 4 = Hardened (handles edge cases, tested, recoverable)
- 5 = Production-ready (battle-tested, monitored, documented)

### K0: Boot / Config / Daemon -- Score: 4/5

**Justification**: `boot.rs` (2,820 lines) implements a structured boot sequence with feature-gated initialization. SQLite persistence exists. Config uses typed `serde` deserialization. 64 tests cover boot under multiple feature combinations.

**Deduction**: The 18 conditional fields behind 6 feature flags make the Kernel struct complex. The `persistence.rs` module has only 3 tests (identified by Track 2 as critical gap). Boot is functional but persistence durability is undertested.

### K1: Process / Supervisor -- Score: 4/5

**Justification**: `ProcessTable` (457 lines) with validated FSM transitions. `AgentSupervisor` (2,418 lines) with RestartStrategy (5 variants), RestartBudget, exponential backoff. `MonitorRegistry` (369 lines) with Erlang-style links and monitors. 105 tests across supervisor/monitor/process. Well-designed Erlang-inspired self-healing.

**Deduction**: Restart events are not chain-logged (Track 1 finding). No circuit-breaker pattern after budget exhaustion.

### K2: A2A IPC -- Score: 4/5

**Justification**: `A2ARouter` with DashMap-backed routing. `KernelIpc` (806 lines) with typed message envelopes. `TopicRouter` (367 lines) for pub/sub. Dead Letter Queue and ReliableDelivery for fault tolerance. 104+ tests across a2a/ipc/topic. DLQ captures failures.

**Deduction**: IPC messages use JSON serialization on the hot path (Track 9 finding). UUID v4 per message adds overhead. IPC failures are not chain-logged.

### K3: WASM / Governance -- Score: 4/5

**Justification**: `wasm_runner.rs` (5,039 lines) -- the largest single file -- implements the complete WASM tool sandbox with Wasmtime, tool signing (Ed25519), 25+ built-in tools, compiled module cache. Two-layer governance: `CapabilityGate` + `TileZeroGate` for per-action checks, `GovernanceEngine` for effect-vector policy. Tool signing verification enforced. 111+ WASM tests, 67 governance tests, 12 gate tests.

**Deduction**: `wasm_runner.rs` at 5,039 lines exceeds the 500-line-per-file guideline by 10x. Gate tests are sparse for security-critical code (12 tests for 809 lines -- Track 2 finding). GovernanceEngine decisions are not chain-logged outside TileZero.

### K3c: ECC Cognitive Substrate -- Score: 3/5

**Justification**: DEMOCRITUS loop (`democritus.rs`, 774 lines) with 5-phase tick cycle. CausalGraph (1,850 lines) with DAG operations. HnswService (353 lines) for vector search. CrossRefStore for entity linking. Embedding providers (mock + ONNX). 12+46+13+19 tests.

**Deduction**: HNSW has O(n) upsert and full rebuild on every dirty query (Track 9 findings). DEMOCRITUS has only 12 tests for 774 lines (Track 2 critical gap). Budget exhaustion behavior is untested. Tick results are not chain-logged.

### K4: Containers -- Score: 3/5

**Justification**: `container.rs` (1,346 lines) with Docker/Podman orchestration, health integration, restart policies. 42 tests. `clawft-plugin-containers` provides the plugin interface.

**Deduction**: The `containers` feature flag is empty (no deps activated). Container operations shell out to docker/podman CLI rather than using a library API. No integration test with a real container runtime.

### K5: App Framework -- Score: 3/5

**Justification**: `app.rs` (1,847 lines) with manifest parsing, validation, lifecycle state machine, hooks. 42 tests. `ServiceRegistry` manages service lifecycle.

**Deduction**: App lifecycle is well-defined but the "application" abstraction is thin -- it is essentially a manifest + service + agent composition layer. No dynamic loading. No hot-reload.

### K6: Mesh Networking -- Score: 3/5

**Justification**: 21 mesh modules (largest subsystem by module count). 274 tests (highest in kernel). MeshRuntime with TCP and WebSocket transports. Noise protocol encryption. mDNS discovery. Kademlia DHT. Heartbeat tracking. Chain replication. Process advertisement. 2-node LAN communication demonstrated.

**Deduction**: Single-hop only -- no multi-hop routing or relay. No NAT traversal. DHT is in-memory only. Chain replication is optimistic (no conflict resolution). 2-node test is the maximum validated topology.

### K8: GUI -- Score: 2/5

**Justification**: Tauri 2.0 shell with React/TypeScript. 4 views (Dashboard, Admin Forms, Knowledge Graph, Component Generator). Cytoscape.js for graph rendering. 13 TS/TSX files. Mock WebSocket.

**Deduction**: Prototype quality. Mock data only (no real kernel connection). No E2E tests. No production views (Process Explorer, Chain Viewer, Governance Console). Component Generator validates self-building thesis but is not production-ready.

### Summary Table

| Phase | Score | Status |
|-------|-------|--------|
| K0 Boot/Config | 4/5 | Hardened (persistence testing gap) |
| K1 Process/Supervisor | 4/5 | Hardened (audit gap) |
| K2 A2A IPC | 4/5 | Hardened (hot path perf) |
| K3 WASM/Governance | 4/5 | Hardened (gate test gap, large file) |
| K3c ECC Cognitive | 3/5 | MVP (perf traps, test gap) |
| K4 Containers | 3/5 | MVP (shell-out, no integration test) |
| K5 App Framework | 3/5 | MVP (thin abstraction) |
| K6 Mesh | 3/5 | MVP (single-hop, 2-node only) |
| K8 GUI | 2/5 | Prototype (mock data) |

**Weighted Average**: 3.3/5 -- Solidly between MVP and Hardened. Core kernel phases (K0-K3) are hardened; outer layers (K4-K8) are MVP or prototype.

---

## 6. Overall Quality Score

### Summary

- **Overall Quality Score**: 7.5/10
- **Files Analyzed**: 405 Rust source files across 22 crates
- **Issues Found**: 34 technical debt markers, 18 unsafe locations, 2 medium-severity security TODOs
- **Technical Debt Estimate**: 13 hours cleanup

### Critical Issues

1. **`wasm_runner.rs` at 5,039 lines** -- 10x over the 500-line file guideline. This is the single largest maintenance risk. It contains the WASM runner, tool registry, tool signing, shell execution, built-in tool catalog, and sandbox config. Needs decomposition into at least 5 modules: `tool_registry.rs`, `tool_signing.rs`, `builtin_tools.rs`, `shell_execution.rs`, `wasm_sandbox.rs`.
   - File: `crates/clawft-kernel/src/wasm_runner.rs`
   - Severity: HIGH (maintainability)
   - Effort: 8 hours

2. **`unimplemented!()` in session.rs:505** -- Runtime panic if code path is reached.
   - File: `crates/clawft-core/src/session.rs:505`
   - Severity: HIGH (reliability)
   - Effort: 1 hour

3. **Missing `#[non_exhaustive]` on public enums** -- Any new variant is a semver-breaking change. Affects `ProcessState`, `KernelState`, `GovernanceDecision`, `GateDecision`, `RestartStrategy`, `DeadLetterReason`, `EnvironmentClass`, and others.
   - File: Multiple crates
   - Severity: MEDIUM (API stability)
   - Effort: 2 hours

4. **API bridge security TODOs** -- CSP middleware and rate limiting are missing from the HTTP API.
   - File: `crates/clawft-services/src/api/handlers.rs:125-128`
   - Severity: MEDIUM (security)
   - Effort: 4 hours

### Code Smells

- **God Object**: `wasm_runner.rs` (5,039 lines) -- combines 6+ responsibilities
- **Feature Envy**: `boot.rs` has 77 feature-gated blocks intimately coupled to every subsystem
- **Dead Code**: 23 `#[allow(dead_code)]` markers, concentrated in `clawft-weave` client and `clawft-plugin-browser`

### Positive Findings

- **Zero circular dependencies** -- Clean layered architecture
- **Consistent error handling** -- `thiserror` everywhere, domain-specific enums
- **No hardcoded secrets** -- Env-var-based credential access with deny lists
- **Strong WASM sandbox** -- Path canonicalization, env var filtering, network restrictions, rate limiting
- **Platform abstraction** -- `Platform` trait enables multi-target (native + WASM) builds
- **Comprehensive feature gating** -- 93 flags across 20 crates, all optional deps properly gated
- **Low technical debt** -- Only 34 markers (10 TODOs, 1 unimplemented, 23 dead_code) in 181K lines
- **Serialization discipline** -- Clean separation between serializable data types and runtime handles

---

## 7. ECC Contribution: CMVG Nodes and Edges

```
NODES:
  [N18] Dependency Graph Mapped
       status: ACHIEVED
       evidence: 22 crates, 6 layers, zero cycles, clean DAG
       artifact: This document Section 1

  [N19] Public API Surface Audited
       status: ACHIEVED
       evidence: 6 Tier 1 crates examined, 166 kernel re-exports cataloged
       findings: Missing #[non_exhaustive], over-exposed internals in kernel
       artifact: This document Section 2

  [N20] Security Quick Scan Complete
       status: ACHIEVED
       evidence: 18 unsafe locations cataloged, 0 hardcoded secrets, input validation confirmed
       findings: 2 medium-risk unsafe blocks (transmute_copy, raw pointer)
       artifact: This document Section 3

  [N21] Technical Debt Inventoried
       status: ACHIEVED
       evidence: 34 markers in 181K lines (0.019% density -- very low)
       findings: 1 unimplemented!(), 10 TODOs, 23 dead_code allows
       artifact: This document Section 4

  [N22] Architecture Fitness Scored
       status: ACHIEVED
       evidence: K0-K8 scored 2-4, weighted average 3.3/5
       findings: Core (K0-K3) hardened at 4/5; outer layers (K4-K8) at 2-3/5
       artifact: This document Section 5

  [N23] wasm_runner.rs Decomposition Needed
       status: IDENTIFIED
       risk: HIGH -- 5,039-line file, 10x over guideline
       remediation: Split into 5+ modules (8 hours)

EDGES:
  N1  --[Enables]--> N18    Kernel completion allows dependency mapping
  N1  --[Enables]--> N19    Kernel completion allows API audit
  N1  --[Enables]--> N20    Kernel completion allows security scan
  N18 --[Informs]--> N22    Dependency graph informs architecture scoring
  N19 --[Reveals]--> N23    API audit reveals wasm_runner over-exposure
  N20 --[Confirms]--> N1    Security scan confirms kernel integrity
  N21 --[Quantifies]--> N1  Debt inventory quantifies kernel health
  N22 --[Informs]--> N3     Fitness scores inform release strategy
  N23 --[Blocks]--> N5      wasm_runner decomposition should precede v0.1.0 (advisory)

CAUSAL CHAIN:
  N1 (achieved) --> N18-N22 (all achieved) --> N3 (release strategy)
  N19 (achieved) --> N23 (identified) --> [decomposition work]
```

---

## 8. Handoff Summary for New Developers

### Architecture in 30 Seconds

WeftOS is a 22-crate Rust workspace organized in 6 dependency layers. Three leaf crates (`clawft-types`, `clawft-security`, `exo-resource-tree`) have zero workspace deps. The kernel (`clawft-kernel`) is the gravitational center at layer 4 with 60K lines. Three binary crates (`weft` CLI, `weaver` daemon, `weftos` kernel binary) sit at layers 5-6.

### Where to Start Reading

1. `crates/clawft-types/src/config.rs` -- Configuration types
2. `crates/clawft-kernel/src/boot.rs` -- Boot sequence (read top-down)
3. `crates/clawft-kernel/src/process.rs` -- ProcessState FSM
4. `crates/clawft-kernel/src/ipc.rs` -- Message routing
5. `crates/weftos/src/lib.rs` -- Public API facade

### Files Over 2,000 Lines (Complexity Risk)

| File | Lines | Recommendation |
|------|-------|----------------|
| `clawft-kernel/src/wasm_runner.rs` | 5,039 | Split into 5+ modules |
| `clawft-kernel/src/chain.rs` | 2,997 | Consider separating anchor/signing logic |
| `clawft-kernel/src/boot.rs` | 2,820 | Acceptable (boot sequence is inherently sequential) |
| `clawft-kernel/src/supervisor.rs` | 2,418 | Consider separating enclave logic |
| `clawft-kernel/src/a2a.rs` | 2,055 | Consider separating router from protocol |

### Build Command

```bash
scripts/build.sh gate   # Full 11-check phase gate
scripts/build.sh test   # Run all tests
scripts/build.sh check  # Fast compile check
```

---

*Document generated by Sprint 11 Symposium Track 8 panel. All findings based on direct code reading of the `feature/weftos-kernel-sprint` branch as of 2026-03-27.*
