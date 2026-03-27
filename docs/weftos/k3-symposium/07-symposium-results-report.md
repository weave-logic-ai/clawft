# K3 Symposium Results Report

**Date**: 2026-03-04
**Status**: Complete (14 questions posed, 14 decisions rendered)
**Branch**: feature/weftos-kernel-sprint
**Predecessor**: [K2 Symposium Results](../k2-symposium/08-symposium-results-report.md)

---

## 1. Executive Summary

The K3 Symposium reconvened the five specialist panels from the K2 Symposium
to review the K3 Tool Lifecycle implementation. The kernel has grown from
397 tests (post-K2.1) to 421 tests (post-K3, with exochain). The implementation
delivers a 27-tool catalog, 2 reference implementations, and a complete
Build -> Deploy -> Execute -> Version -> Revoke lifecycle with ExoChain
audit logging.

### K3 Scorecard

| Dimension | Score | Evidence |
|-----------|-------|----------|
| Catalog completeness | 10/10 | 27 tools, all valid schemas, unique names |
| Reference implementations | 8/10 | 2 of 27 implemented, both fully tested |
| Lifecycle integrity | 9/10 | All 5 phases with chain events |
| Test coverage | 8/10 | 42 K3-specific tests, some edges untested |
| Security posture | 7/10 | Good defaults, gate granularity gap |
| Integration quality | 8/10 | Clean wiring, per-agent registry |
| **Overall** | **8.1/10** | **Ready for K4** |

### What Shipped

| Component | Lines | Tests | Verdict |
|-----------|-------|-------|---------|
| wasm_runner.rs (catalog, registry, tools) | 1,639 | 31 | DELIVERED |
| tree_manager.rs (lifecycle methods) | +208 | 6 | DELIVERED |
| agent_loop.rs (exec dispatch) | +63 | 12 (total) | DELIVERED |
| boot.rs (tool registration) | +46 | N/A (boot test) | DELIVERED |
| daemon.rs (ToolRegistry wiring) | +20 | N/A (integration) | DELIVERED |
| model.rs (ResourceKind::Tool) | +3 | 1 | DELIVERED |
| lib.rs (exports) | +4 | N/A | DELIVERED |

---

## 2. K2 Symposium Commitment Status

How K3 advances the K2 Symposium's commitments:

| K2 Commitment | K3 Status | Notes |
|---------------|-----------|-------|
| **C1** SpawnBackend enum | Shipped in K2.1 | K3 builds on it |
| **C2** ServiceApi trait | **NOT STARTED** | Q10 posed for K4 |
| **C3** Chain-anchored contracts | **NOT STARTED** | Depends on C2 |
| **C4** Dual-layer gate | **PARTIAL** | Handler-time done, routing-time deferred (Q11) |
| **C5** WASM-compiled shell | **NOT STARTED** | Deferred to K4/K5 |
| **C6** Post-quantum dual signing | **BLOCKED** | rvf-crypto DualKey API gap |
| **C7** ChainAnchor trait | **NOT STARTED** | K4 scope |
| **C8** SpawnBackend::Tee | Shipped in K2.1 | Returns BackendNotAvailable |
| **C9** N-dimensional EffectVector | **NOT STARTED** | K3 uses 5D fixed |
| **C10** K6 SPARC spec | **NOT STARTED** | Pre-K5 scope |

---

## 3. Critical Findings

### CF-1: Gate Action Granularity (Medium)
**Panels**: Security (Panel 3), Integration (Panel 4)

The exec handler passes `"tool.exec"` to GovernanceGate for all tools,
ignoring the per-tool `gate_action` strings in the catalog. This means
`fs.read_file` (risk 0.1) and `fs.remove` (risk 0.7) are gated identically.

**Impact**: Governance cannot distinguish between low-risk reads and high-risk
deletes until this is fixed.

**Recommendation**: K4 must use tool-specific `gate_action` from the catalog
in the exec handler's gate check.

### CF-2: FsReadFileTool Path Access (Medium)
**Panels**: Security (Panel 3), Testing (Panel 5)

FsReadFileTool reads any path the kernel process can access. No sandboxing.

**Impact**: An agent with exec capability can read any file on the system.

**Recommendation**: K4 should add `SandboxEnforcer.check_file_read()` or
tool-specific `allowed_paths` configuration.

### CF-3: Per-Agent ToolRegistry (Low)
**Panel**: Integration (Panel 4)

The daemon creates a new ToolRegistry per agent spawn, including 27
BuiltinToolSpec allocations each time.

**Impact**: Wasteful allocation. Not a correctness issue.

**Recommendation**: Move ToolRegistry to Kernel struct, share via Arc.

---

## 4. Design Decisions (D1-D14)

Decisions rendered by the project lead on the Q&A roundtable questions.
These decisions are binding for K4 implementation.

### D1: Hierarchical ToolRegistry (Q1)

**Decision**: (c) Hierarchical -- kernel has base registry, agents can add
tool overrides. Governance, configuration, and agents can all modify the
registry at their respective layers.

**Rationale**: A pure singleton is too rigid -- governance needs to restrict
tools per-agent, configuration needs to customize tool availability per
environment, and agents themselves may need to register custom tools.
A hierarchical model provides a shared base (27 built-in tools from
`Kernel::boot()`) with per-agent overlay registries that can add, remove,
or reconfigure tools. The base layer is `Arc`-shared (addressing CF-3),
while overlay layers are per-agent.

**Impact**: `ToolRegistry` gains a `parent: Option<Arc<ToolRegistry>>`
field. Lookup walks the chain: agent overlay -> kernel base. Governance
can inject deny-lists at the agent layer. Configuration can set
environment-specific tool availability at boot. The daemon creates one
base registry at boot (shared via `Arc`), then creates per-agent overlays
as needed. This resolves CF-3 (wasteful per-agent allocation) while
enabling fine-grained control.

### D2: Context-Based Gate Actions (Q2)

**Decision**: (c) Keep generic `"tool.exec"` but pass tool name + effect
vector as context. Governance handles the context.

**Rationale**: Moving gate action resolution into the exec handler adds
complexity to the hot path. The governance gate is already designed to
evaluate context -- effect vectors, risk scores, agent identity. Passing
the tool name and its effect vector as context to the generic `"tool.exec"`
action lets governance make fine-grained decisions without changing the
gate action string. This keeps the exec handler simple and pushes
decision complexity where it belongs: into governance rules.

**Impact**: The exec handler's gate check becomes:
`gate.check(agent_id, "tool.exec", {tool: name, effect: vector, pid: N})`.
GovernanceGate rules can match on `context.tool` and `context.effect`
to distinguish `fs.read_file` from `fs.remove`. No change to the
`gate_action` field in BuiltinToolSpec -- it remains available for
documentation and rule authoring, but the exec handler doesn't resolve it.
CF-1 is addressed by enriching context, not changing the action string.

### D3: Implement All 25 Tools (Q3)

**Decision**: (a) Implement all 25 remaining tools in K4, with the option
to defer the hardest ones. Priority on general-use tools.

**Rationale**: The 2 reference implementations prove the dispatch pattern.
The remaining 25 are straightforward implementations of the same pattern.
Implementing all of them makes the kernel immediately useful for real
workloads. Tools that require external dependencies not yet available
(e.g., tools needing container runtime from K4's primary goal) can be
deferred, but the default posture is "implement everything."

**Impact**: K4 implementation plan includes all 25 tools ordered by
utility: filesystem tools first (most general), then agent tools
(needed for orchestration), then system tools (observability). Each
tool follows the FsReadFileTool/AgentSpawnTool pattern: implement
`BuiltinTool` trait, register in catalog, add tests. Estimated scope:
~50-80 lines per tool, ~1,500 lines total.

### D4: Wasmtime in K4 (Q4)

**Decision**: (a) Implement Wasmtime integration in K4 alongside the
container runtime.

**Rationale**: K4's primary goal is the container runtime. Wasmtime is
the WASM container -- it provides the same isolation guarantees as
Docker/Podman but for WASM modules. Implementing both in K4 creates a
complete execution backend story: native tools run directly, WASM tools
run in Wasmtime, container services run in Docker/Podman. The `wasm-sandbox`
feature gate is already plumbed; K4 fills in the real Wasmtime code.

**Impact**: K4 activates the `wasm-sandbox` feature path in
`WasmToolRunner`. The existing `WasmSandboxConfig` defaults (fuel: 1M,
memory: 16 MiB, timeout: 30s) become live. The 6 Wasmtime-specific
tests from the K3 plan (fuel exhaustion, memory limit, timeout, etc.)
are implemented and gated behind `wasm-sandbox`. Module validation
upgrades from header-only to full `wasmtime::Module::validate()`.

### D5: Disk-Persisted Module Cache (Q5)

**Decision**: (b) Disk-persisted compiled modules that survive restarts.

**Rationale**: Wasmtime module compilation is expensive (~50-200ms per
module). Disk persistence means compiled modules survive kernel restarts,
which is critical for production deployments. The cache key is the
module's SHAKE-256 hash (already computed during build), so cache
invalidation is automatic -- a new module version has a new hash.
In-memory LRU adds complexity without significant benefit over disk
caching with OS page cache.

**Impact**: Add a `CompiledModuleCache` that stores Wasmtime serialized
modules at `{data_dir}/cache/wasm/{module_hash}.cwasm`. Cache lookup
is O(1) by hash. Cache entries are validated against the module hash
on load. Cache eviction is LRU by access time with a configurable
size limit (default: 256 MiB). The OS page cache handles hot modules
in memory automatically.

### D6: Configurable WASI with Read-Only Default (Q6)

**Decision**: (d) Configurable per-tool via `ToolSpec.permissions`, with
(b) read-only sandbox directory as the default.

**Rationale**: Different tools need different filesystem access. A
code-analysis tool needs read-only access to source files. A build
tool needs read-write access to output directories. A pure computation
tool needs no filesystem at all. Making this configurable per-tool via
the existing `ToolSpec` gives maximum flexibility with safe defaults.
The read-only sandbox default (`/tmp/weft-tools/{tool-name}/`) ensures
no tool can write to the filesystem without explicit permission.

**Impact**: `WasmSandboxConfig` gains `wasi_fs_scope: WasiFsScope`
with variants: `None`, `ReadOnly(PathBuf)`, `ReadWrite(PathBuf)`,
`Custom(Vec<WasiMount>)`. Default: `ReadOnly("/tmp/weft-tools/{name}/")`.
`BuiltinToolSpec` gains `wasi_scope: Option<WasiFsScope>` that overrides
the default per-tool. Governance can further restrict WASI scope via
context (per D2).

### D7: Tree Metadata for Version History (Q7)

**Decision**: (b) Store version history in tree metadata.

**Rationale**: The resource tree is the kernel's primary state store and
already holds tool metadata. Storing version history in tree metadata
makes it queryable without chain traversal. The chain remains the
authoritative audit log, but tree metadata provides fast access for
common operations like "what versions exist" and "is version N revoked."
The `DeployedTool.versions: Vec<ToolVersion>` already exists in the
type system -- this decision says to persist it in the tree.

**Impact**: Tree metadata for tool nodes gains a `versions` field
containing the serialized `Vec<ToolVersion>`. `TreeManager::deploy_tool()`
writes the full version history. `TreeManager::update_tool_version()`
appends to the version array. This is redundant with chain data but
enables O(1) version queries. Tree Merkle hashes incorporate the
version history, so version tampering is detectable.

### D8: Informational Revocation (Q8)

**Decision**: (b) Revocation is informational; the governance gate handles
enforcement.

**Rationale**: Revocation marking in tree metadata (per D7) provides the
data. Governance rules decide what to do with it. Some environments may
want hard-block on revoked tools; others may want warning-only. Making
the ToolRegistry itself enforce revocation is inflexible -- it removes
the governance layer's ability to make context-sensitive decisions.
The governance gate already checks before every execution (per D2),
so it can inspect revocation status as part of its context evaluation.

**Impact**: `TreeManager::revoke_tool_version()` continues to mark
metadata only (current behavior). No revocation check added to
`ToolRegistry::execute()`. Governance rules can match on
`context.tool_version.revoked == true` to block, warn, or allow
execution of revoked tools. The chain event `tool.version.revoke`
provides the audit trail. Enforcement policy is configuration, not code.

### D9: Central Authority with CA Chain (Q9)

**Decision**: (a+b) Central authority for built-in tools, developer
keypairs with kernel-signed CA chain for third-party tools.

**Rationale**: Built-in tools are kernel code -- the kernel keypair
signs them directly. Third-party WASM tools need their own identity
but must be traceable to a trust root. A CA chain model lets developers
sign their tools with their own keypairs, with the kernel's CA signing
the developer's public key to establish trust. This is the same model
used by code signing in every major OS: Apple signs developer certs,
developers sign their apps.

**Impact**: K4 defines a `ToolSigningAuthority` with two modes:
- `Kernel`: tools signed directly by kernel Ed25519 key (built-in tools)
- `Developer(CertChain)`: developer key signed by kernel CA, tool signed
  by developer key. Verification checks: developer sig valid, CA sig
  on developer pubkey valid, CA pubkey matches kernel.
The `ToolVersion.signature` field remains `[u8; 64]` (Ed25519). A new
`cert_chain: Option<Vec<Certificate>>` field is added for CA chains.

### D10: Separate ServiceApi and BuiltinTool (Q10)

**Decision**: (b) Separate concepts -- ServiceApi for long-running services,
BuiltinTool for one-shot tools.

**Rationale**: Tools and services have fundamentally different lifecycles.
A tool executes once and returns a result. A service runs continuously
and responds to requests over time. BuiltinTool has `execute(args) -> Result`
semantics. ServiceApi needs `start()`, `stop()`, `health()`, `call()`
semantics. Conflating them creates an awkward API where tools must
implement lifecycle methods they don't need, or services must pretend
to be one-shot. K2 Symposium D4 already established the layered
architecture; ServiceApi is the service layer, BuiltinTool is the
tool layer.

**Impact**: K4 or K5 implements `ServiceApi` as a separate trait:
```rust
pub trait ServiceApi: Send + Sync {
    fn start(&self) -> Result<()>;
    fn stop(&self) -> Result<()>;
    fn health(&self) -> HealthStatus;
    fn call(&self, method: &str, args: Value) -> Result<Value>;
}
```
`ToolRegistry` manages tools. `ServiceRegistry` manages services.
Both are accessible through the agent loop, with governance gating
on both paths. The `ServiceEntry` from K2 D1 wraps `ServiceApi`
implementations.

### D11: Routing-Time Gate at Cluster Build (Q11)

**Decision**: (b) Add routing-time gate layer in K5 when cross-service
routing needs governance for cluster builds.

**Rationale**: K4 is single-node. The handler-time gate check (already
working) is sufficient for single-node tool dispatch. Routing-time
gate checks become critical when messages cross node boundaries in
K5 clustering -- you want to reject unauthorized cross-node messages
at the router before they traverse the network. Adding routing-time
gates in K4 would be premature optimization for a pattern that doesn't
exist yet.

**Impact**: K4 defers routing-time gate integration. K5 adds
`GateBackend` to `A2ARouter.route()` when implementing cross-node
message routing. The K2 commitment C4 (dual-layer gate) is partially
fulfilled in K3 (handler-time) and completes in K5 (routing-time).
CapabilityChecker remains in A2ARouter for K4.

### D12: Configurable Sandboxing with Governance (Q12)

**Decision**: (d) Governance gate + environment overrides + user
overrides/sudo. Multi-layer sandboxing.

**Rationale**: File access sandboxing should not be a single binary
decision. The governance gate (per D2) already receives tool context
including the effect vector. Environment configuration determines
baseline permissions (dev = permissive, prod = strict). Users with
elevated privileges can override restrictions when needed (like `sudo`).
This three-layer model provides defense in depth while remaining
practical for development workflows.

**Impact**: FsReadFileTool sandboxing stack:
1. **Governance**: gate check with `{tool: "fs.read_file", path: "/etc/passwd"}`
   -- governance rules can deny specific paths
2. **Environment**: `SandboxConfig.allowed_paths` per environment
   (dev: `["/"]`, prod: `["/data/", "/tmp/"]`)
3. **User override**: `ToolExecRequest.sudo: bool` flag that bypasses
   environment restrictions (logged to chain, requires elevated agent
   capability)

Tool-level defaults from BuiltinToolSpec + environment config +
governance rules + user override = final access decision. CF-2 is
resolved by this layered approach.

### D13: Snapshots Unnecessary Now (Q13)

**Decision**: (c) Unnecessary now. WASM tools are stateless. The
snapshot-to-chain pattern is interesting for future exploration.

**Rationale**: K3's WASM tools create fresh `Store<ToolState>` per
execution with no shared state between invocations. There's nothing
to snapshot. The ruvector-snapshot crate's checkpoint capability is
relevant for long-running WASM services (K5+), not one-shot tools.
However, the idea of snapshotting WASM linear memory and anchoring it
to the chain is architecturally interesting for debugging and migration
in K6 clustering.

**Impact**: No snapshot integration in K4. The pattern is noted for
K6 investigation: snapshot WASM linear memory -> hash -> chain event ->
restore on another node. This would enable tool migration in a cluster
without re-execution. Low priority until clustering (K5/K6) creates
the use case.

### D14: tiny-dancer for Optimal Routing (Q14)

**Decision**: (a) Use tiny-dancer scoring to find the best route between
native and WASM execution. Governance finds the *right* route;
tiny-dancer finds the *best* route.

**Rationale**: This follows K2 D17 (layered routing -- learned + enforced).
tiny-dancer's FastGRNN-based scoring can evaluate whether a tool invocation
is better served by native execution (fast, no isolation) or WASM execution
(isolated, metered). The governance gate determines what's *allowed*;
tiny-dancer determines what's *optimal*. For example, a low-risk
`fs.stat` call might skip WASM overhead entirely, while a high-risk
`fs.remove` call always goes through WASM sandbox.

**Impact**: K4 plumbs tiny-dancer scoring into the exec handler's
backend selection. The routing stack for tool execution becomes:
1. Governance gate: is this tool allowed? (per D2)
2. tiny-dancer: native or WASM? (scoring based on risk, load, tool profile)
3. ToolRegistry dispatch: execute via selected backend
4. Chain log: record backend selection + metrics

Integration depends on ruvector-tiny-dancer-core API stability. If
the crate API isn't ready for K4, the scoring logic can be a simple
heuristic (risk > 0.3 -> WASM, else native) until tiny-dancer is
integrated.

---

## 5. Approved Changes

Changes approved based on D1-D14 decisions:

### AC-1: Hierarchical ToolRegistry (from D1)

Move base ToolRegistry creation to `Kernel::boot()`. Store as
`Arc<ToolRegistry>` on the Kernel struct. Add `parent` field for
hierarchical lookup. Per-agent overlays created at spawn time as needed.

**Files**: `wasm_runner.rs`, `boot.rs`, `daemon.rs`
**Scope**: ~50 lines changed
**Phase**: K4

### AC-2: Enriched Gate Context (from D2)

Pass tool name + effect vector as context in the exec handler's gate
check. Keep `"tool.exec"` as the action string.

**Files**: `agent_loop.rs`
**Scope**: ~15 lines changed
**Phase**: K4

### AC-3: Multi-Layer FsReadFileTool Sandboxing (from D12)

Implement governance + environment + user override sandboxing stack
for file access tools.

**Files**: `wasm_runner.rs`, `agent_loop.rs`
**Scope**: ~60 lines
**Phase**: K4

### AC-4: All 25 Tool Implementations (from D3)

Implement remaining 25 built-in tools following the FsReadFileTool/
AgentSpawnTool pattern.

**Files**: `wasm_runner.rs` (or split into `tools/*.rs` modules)
**Scope**: ~1,500 lines
**Phase**: K4

### AC-5: Wasmtime Activation (from D4)

Fill in the `wasm-sandbox` feature path with real Wasmtime code.
Add disk-persisted module cache (D5). Add configurable WASI scope (D6).

**Files**: `wasm_runner.rs`, new `cache.rs`
**Scope**: ~400 lines
**Phase**: K4

### AC-6: Tree Version History (from D7)

Persist `Vec<ToolVersion>` in tree metadata for O(1) version queries.

**Files**: `tree_manager.rs`
**Scope**: ~30 lines changed
**Phase**: K4

### AC-7: CA Chain Signing (from D9)

Add `ToolSigningAuthority` with Kernel and Developer(CertChain) modes.

**Files**: `wasm_runner.rs`, new signing types
**Scope**: ~100 lines
**Phase**: K4

---

## 6. Test Coverage Summary

### K3 Test Inventory (42 tests)

**Catalog Tests (5)**:
- `builtin_catalog_has_27_tools`
- `all_tools_have_valid_schema`
- `all_tools_have_gate_action`
- `tool_names_are_unique`
- `tool_categories_correct`

**FsReadFileTool Tests (4)**:
- `fs_read_file_reads_content`
- `fs_read_file_with_offset_limit`
- `fs_read_file_not_found`
- `fs_read_file_returns_metadata`

**AgentSpawnTool Tests (3)**:
- `agent_spawn_creates_process`
- `agent_spawn_with_wasm_backend_fails`
- `agent_spawn_returns_pid`

**ToolRegistry Tests (2)**:
- `registry_register_and_execute`
- `registry_not_found`

**Module Hash Tests (2)**:
- `module_hash_deterministic`
- `module_hash_differs_for_different_input`

**Sandbox Config Tests (4)**:
- `default_config`
- `config_serde_roundtrip`
- `execution_timeout_duration`
- `tool_state_default`

**WASM Validation Tests (6)**:
- `validate_wasm_rejects_invalid_magic`
- `validate_wasm_rejects_too_large`
- `validate_wasm_rejects_too_short`
- `validate_wasm_accepts_valid_header`
- `validate_wasm_warns_on_wrong_version`
- `load_tool_without_feature_rejects`

**Serialization Tests (5)**:
- `wasm_error_display`
- `wasm_tool_result_serde_roundtrip`
- `wasm_validation_serde_roundtrip`
- `tool_version_serde_roundtrip`
- `builtin_tool_spec_serde_roundtrip`

**Lifecycle Tests (6, exochain)**:
- `tool_build_computes_hash_and_signs`
- `tool_deploy_creates_tree_node`
- `tool_deploy_emits_chain_event`
- `tool_version_update_chain_links`
- `tool_version_revoke_marks_revoked`
- `tool_revoke_emits_chain_event`

**Agent Loop Gate Tests (2, exochain)**:
- `gate_deny_blocks_exec`
- `gate_permit_allows_exec`

**Agent Loop Chain Tests (2, exochain)**:
- `chain_logs_ipc_recv_ack`
- `chain_logs_suspend_resume`

**Resource Tree Test (1)**:
- `resource_kind_tool_serde_roundtrip`

---

## 7. Strategic Direction

### K3 Delivered Scope

K3 was scoped as "tool lifecycle foundations" -- the catalog, registry,
reference implementations, lifecycle management, and chain integration.
This scope is fully delivered.

### K4 Confirmed Scope (from D1-D14)

1. **Container Runtime** (primary K4 goal from K2 symposium)
2. **Hierarchical ToolRegistry** (AC-1, from D1)
3. **Enriched gate context** (AC-2, from D2 -- addresses CF-1)
4. **All 25 tool implementations** (AC-4, from D3)
5. **Wasmtime integration** (AC-5, from D4)
6. **Disk-persisted module cache** (from D5)
7. **Configurable WASI with read-only default** (from D6)
8. **Tree version history** (AC-6, from D7)
9. **Multi-layer sandboxing** (AC-3, from D12 -- addresses CF-2)
10. **CA chain signing** (AC-7, from D9)
11. **tiny-dancer routing heuristic** (from D14)

### K5 Scope (from decisions)

- ServiceApi trait as separate concept (D10)
- Routing-time gate layer for clustering (D11)
- N-dimensional EffectVector (K2 C9)
- WASM state snapshots exploration (D13 -- if clustering creates need)

### Deferred / Blocked

- Post-quantum signing (K2 C6) -- blocked on rvf-crypto upstream
- K6 SPARC spec (K2 C10) -- pre-K5

---

## 8. Panel Signatures

| Panel | Lead | Verdict |
|-------|------|---------|
| Tool Catalog & Registry | Kernel Auditor | **PASS** -- 31/31 tests |
| Lifecycle & Chain Integrity | Services Architect | **PASS** -- 6/6 tests |
| Sandbox & Security | Research Analyst | **PASS with findings** -- CF-1, CF-2 |
| Agent Loop & Dispatch | Integration Architect | **PASS** -- 12/12 tests |
| Live Testing & Validation | RUV Expert | **PASS** -- 421/421 tests |

**Overall K3 Symposium Verdict**: **APPROVED for K4 transition**

The K3 Tool Lifecycle implementation is complete for its declared scope.
The catalog is comprehensive, the lifecycle chain is cryptographically
sound, and the reference implementations prove the dispatch pattern.
Three critical findings (CF-1 through CF-3) are addressed by decisions
D1, D2, and D12. All 14 design questions (Q1-Q14) have been decided
(D1-D14). K4 implementation may proceed immediately.
