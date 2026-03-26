# Sprint 09b: Decision Triage

**Date**: 2026-03-26
**Author**: governance-counsel
**Branch**: feature/weftos-kernel-sprint

---

## Triage Summary

| Resolution | Count | Decisions |
|------------|:-----:|-----------|
| **IMPLEMENTED** (already in code) | 4 | k3:D1, k2:D11, k2:C2/ServiceApi, k3:D8 |
| **IMPLEMENT** (this sprint, code changes) | 2 | k3:D2, k3:D12 |
| **IMPLEMENT** (Sprint 10 plan) | 6 | k3:D4, k3:D6, k3:D7, k3:D10, k2:D8, k2:D20 |
| **DEFER** | 8 | k3:D3, k3:D5, k3:D9, k3:D14, k2:D10, k2:D12, k2:D13, k2:D17 |
| **SUPERSEDE** | 1 | k2:D18 (by Weaver ECC) |
| **CONFIRM DEFERRED** | 2 | k3:D11, k3:D13 |
| **MAP TO SPRINT** | 1 | ecc:D5 (to 09c) |
| **Total** | **24** | |

---

## Tier 1: HIGH Priority (Resolve Fully)

### k3:D1 -- Hierarchical ToolRegistry with kernel base + per-agent overlays
- **Resolution**: IMPLEMENTED (already in code)
- **Evidence**: `wasm_runner.rs` line 1041+ has `ToolRegistry` with `parent: Option<Arc<ToolRegistry>>`, `with_parent()` constructor, and 4 hierarchical tests
- **Commitment**: k3:AC-1 status -> implemented

### k3:D2 -- Context-based gate actions: generic tool.exec with rich context
- **Resolution**: IMPLEMENT (this sprint)
- **Code change**: Added `GovernanceRequest::with_tool_context()` builder in `governance.rs`
- **What it does**: Sets `context["tool"]`, `context["gate_action"]`, `context["pid"]`, and `effect` from tool catalog data so governance rules can distinguish `fs.read_file` from `fs.remove`
- **Lines**: ~30 new in governance.rs
- **Commitment**: k3:AC-2 status -> implemented

### k3:D12 -- Multi-layer sandboxing: governance + environment + sudo override
- **Resolution**: IMPLEMENT (this sprint)
- **Code change**: Added `SandboxLayer` enum, `SandboxDecision` struct, `SandboxConfig::check_path_multilayer()` in `wasm_runner.rs`. Added `SandboxConfig::sudo_override` field. Added `EVENT_KIND_SANDBOX_SUDO_OVERRIDE` constant in `chain.rs`
- **Lines**: ~80 new in wasm_runner.rs, ~8 new in chain.rs
- **Commitment**: k3:AC-3 status -> implemented

### k2:D11 -- Enable post-quantum signing immediately
- **Resolution**: IMPLEMENTED (already in code)
- **Evidence**: `chain.rs` has `dual_sign()`, `verify_dual_signature()`, `set_ml_dsa_key()`, `has_dual_signing()`, plus `DualSignature` struct and `DualSignerConfig`. 8+ tests covering Ed25519-only, both algorithms, tampered data, checkpoint verification
- **Commitment**: k2:C6 status -> implemented

---

## Tier 2: MEDIUM Priority (Resolve or Defer)

### k3:D3 -- Implement all 25 remaining tools in K4
- **Resolution**: DEFER
- **Rationale**: 25 tools is a large work package requiring stable tool framework (D1, D4). Framework is now stable; implementation belongs in Sprint 10 (08c).
- **Defer-until**: Sprint 10 begins and tool framework passes integration tests
- **Commitment**: k3:AC-4 deferred to post-Sprint 10

### k3:D4 -- Wasmtime integration in K4 alongside container runtime
- **Resolution**: IMPLEMENT (Sprint 10 plan)
- **Scope**: Replace mock WASM execution with `wasmtime::Engine`, ~200 lines new
- **File**: `wasm_runner.rs`
- **Commitment**: k3:AC-5 scheduled for Sprint 10

### k3:D6 -- Configurable WASI with read-only sandbox default
- **Resolution**: IMPLEMENT (bundled with D4 in Sprint 10)
- **Scope**: `WasmSandboxConfig` gains `wasi_fs_scope: WasiFsScope` enum
- **Commitment**: No separate commitment; part of AC-5

### k3:D7 -- Tree metadata for version history
- **Resolution**: IMPLEMENT (Sprint 10 plan)
- **Scope**: `TreeNodeVersion` struct with timestamp, author_pid, prev_hash; ~100 lines
- **File**: `tree_manager.rs`
- **Commitment**: k3:AC-6 scheduled for Sprint 10

### k3:D8 -- Informational revocation; governance gate handles enforcement
- **Resolution**: IMPLEMENT (this sprint, minimal)
- **Code change**: Added `EVENT_KIND_CAPABILITY_REVOKED` and `EVENT_KIND_TOOL_VERSION_REVOKED` constants in `chain.rs`
- **Rationale**: Revocation is data; governance rules decide enforcement policy. No enforcement code needed in ToolRegistry.

### k3:D9 -- Central authority + CA chain for tool signing
- **Resolution**: DEFER
- **Rationale**: PKI infrastructure out of scope for Sprint 09/10. Existing Ed25519 signing on chain entries provides integrity. CA chains are post-1.0.
- **Defer-until**: Post-1.0 release, multi-operator deployment
- **Commitment**: k3:AC-7 deferred to post-1.0

### k3:D10 -- Separate ServiceApi and BuiltinTool concepts
- **Resolution**: IMPLEMENT (Sprint 10 plan)
- **Note**: `ServiceApi` trait already exists in `service.rs` (K3 C2). `BuiltinTool` trait already exists in `wasm_runner.rs`. The separation is already in code. Sprint 10 formalizes the distinction with clear documentation.
- **Commitment**: No separate commitment; concepts already separated

---

## Tier 3: DEFERRED/LOW (Batch Resolution)

### k3:D5 -- Disk-persisted compiled module cache
- **Resolution**: DEFER (bundled with D4)
- **Rationale**: Sub-feature of Wasmtime integration. Not standalone.
- **Defer-until**: Implemented as part of D4 in Sprint 10

### k3:D11 -- Routing-time gate deferred to K5 cluster build
- **Resolution**: CONFIRM DEFERRED
- **Rationale**: Already deferred in original symposium. Single-node handler-time gate is sufficient for K4. No action needed.

### k3:D13 -- WASM snapshots unnecessary now; noted for K6
- **Resolution**: CONFIRM DEFERRED
- **Rationale**: Already deferred. WASM tools are stateless; snapshots irrelevant until K5+ long-running services.

### k3:D14 -- tiny-dancer scoring for native vs WASM routing
- **Resolution**: DEFER
- **Rationale**: Depends on governance trajectory learning (k2:D17), which is also deferred. Coupled dependencies.
- **Defer-until**: Governance trajectory learning implemented

### k2:D8 -- ExoChain-stored immutable API contracts
- **Resolution**: IMPLEMENT (Sprint 10 plan)
- **Code change (this sprint)**: Added `EVENT_KIND_API_CONTRACT_REGISTERED` constant in `chain.rs`
- **Scope for Sprint 10**: `ServiceRegistry::register()` optionally accepts `ApiContract`, appends chain event
- **Commitment**: k2:C3 scheduled for Sprint 10

### k2:D10 -- WASM-compiled shell with container sandbox
- **Resolution**: DEFER
- **Rationale**: Requires both Wasmtime (D4) and container runtime. Sprint 10 delivers Wasmtime; shell can be built on top afterward.
- **Defer-until**: Sprint 10 completes Wasmtime integration
- **Commitment**: k2:C5 deferred to post-Sprint 10

### k2:D12 -- Chain-agnostic blockchain anchoring trait
- **Resolution**: DEFER
- **Rationale**: Blockchain anchoring is a deployment-time feature requiring external chain SDKs. Not a kernel concern for Sprint 09/10.
- **Defer-until**: Multi-chain deployment use case is prioritized
- **Commitment**: k2:C7 deferred to post-1.0

### k2:D13 -- Zero-knowledge proofs as foundational capability
- **Resolution**: DEFER
- **Rationale**: Requires significant cryptographic infrastructure (circuit definition, proving key management). Out of scope.
- **Defer-until**: Post-1.0, when privacy-preserving computation is a user requirement

### k2:D17 -- Layered routing: tiny-dancer learned + governance enforced
- **Resolution**: DEFER
- **Rationale**: Learned routing requires trajectory data from governance decisions. Governance engine logs decisions but does not yet support trajectory learning.
- **Defer-until**: Governance trajectory learning implemented in Sprint 10

### k2:D18 -- SONA at K5, training data pipeline from K3 forward
- **Resolution**: SUPERSEDE
- **Rationale**: SONA (Self-Organizing Neural Architecture) is superseded by the Weaver ECC model, which provides the practical implementation of continuous cognitive operation. SONA as a separate system is not needed.
- **Superseded by**: Weaver ECC (Sprint 09c)

### k2:D20 -- Configurable N-dimensional effect algebra
- **Resolution**: IMPLEMENT (Sprint 10 plan)
- **Scope**: Replace fixed 5-field `EffectVector` with `HashMap<String, f64>` dimensions; ~60 lines changed in `governance.rs`
- **Commitment**: k2:C9 scheduled for Sprint 10

### ecc:D5 -- DEMOCRITUS as continuous nervous system operation
- **Resolution**: MAP TO SPRINT 09c
- **Rationale**: DEMOCRITUS is realized through the CognitiveTick integration in the Weaver Runtime (Sprint 09c). The Weaver runs on every tick, processes new data, updates the causal model, and emits impulses when confidence changes.
- **Commitment**: Covered by Weaver TODO items in Sprint 09c
