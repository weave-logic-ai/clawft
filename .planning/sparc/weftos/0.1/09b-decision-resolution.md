# Sprint 09b: Decision Resolution

**Document ID**: 09b
**Workstream**: W-KERNEL
**Duration**: 5 days
**Goal**: Resolve the 23 pending/blocked decisions creating decision debt and update commitment tracking
**Depends on**: K0-K6 (all prior phases)
**Orchestrator**: `09-orchestrator.md`
**Priority**: P0 (Critical) -- unblocks downstream implementation work

---

## S -- Specification

### Problem Statement

The ECC graph analysis identified 23 pending or blocked decisions across three
symposiums (K2, K3, ECC). These decisions create "decision debt" -- downstream
work packages that cannot begin because foundational choices have not been made.
Additionally, 14 commitments remain unfulfilled, and 1 decision (k2:D11) is
actively blocked by an API gap.

**Source data**: `docs/weftos/09-symposium/00-symposium-overview.md` section 3,
`docs/weftos/09-symposium/01-graph-findings.md` section 3,
`.weftos/analysis/gap-report.json` ranks 4-18.

### Decision Inventory

| Source | Total | Implemented | Pending | Blocked | Deferred |
|--------|:-----:|:-----------:|:-------:|:-------:|:--------:|
| K2 | 22 | 12 | 7 | 1 | 0 |
| K3 | 14 | 0 | 11 | 0 | 2 |
| ECC | 8 | 7 | 1 | 0 | 0 |
| **Total** | **44** | **19** | **19** | **1** | **2** |

### Target Outcome

- Pending decisions reduced from 23 to <5
- Blocked decisions reduced from 1 to 0
- All resolutions documented with rationale
- Commitment tracking updated to reflect current state

---

## P -- Pseudocode

### Decision Resolution Algorithm

```
For each pending decision:
  1. Re-read original symposium rationale
  2. Check if circumstances changed since symposium
  3. Classify: IMPLEMENT | DEFER | SUPERSEDE
  4. If IMPLEMENT:
     a. Write implementation plan (scope, files, estimated lines)
     b. Assign to appropriate sprint (09 or 10+)
     c. Update commitment if one exists
  5. If DEFER:
     a. Write rationale (why now is not the right time)
     b. Set defer-until condition (not a date, a trigger)
     c. Mark commitment as deferred with rationale
  6. If SUPERSEDE:
     a. Reference the superseding decision
     b. Close original and associated commitments
  7. Record resolution in symposium log
```

### Triage Priority Matrix

```
Priority HIGH (resolve fully with implementation plan):
  - Decisions that block commitments
  - Decisions that block high-risk modules
  - Decisions with downstream dependency chains

Priority MEDIUM (resolve with minimal implementation or defer):
  - Decisions with no blocking relationships
  - Decisions targeting future phases (K4+)

Priority LOW (batch defer with rationale):
  - Decisions explicitly marked DEFERRED in original symposium
  - Decisions superseded by subsequent work
```

---

## A -- Architecture

### Decision Resolution Format

Each decision resolution follows this structure:

```markdown
### [Decision ID]: [Title]

**Original rationale**: [1-sentence summary from symposium]
**Status change**: [pending|blocked] -> [implemented|deferred|superseded]
**Resolution**: [What we decided and why]
**Implementation**: [If implemented: scope, files, sprint assignment]
**Commitment impact**: [Which commitments are affected]
```

---

## R -- Refinement

### Resolution Plan: K3 Decisions (14 total)

#### Tier 1: HIGH Priority -- Full Resolution (4 decisions)

##### k3:D1 -- Hierarchical ToolRegistry with kernel base + per-agent overlays

**Original rationale**: Tools should be available at two levels: a shared kernel
registry (immutable base) and per-agent overlays (agent-specific tools).
**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c content ops). The hierarchical
registry is prerequisite for k3:AC-1 and affects `wasm_runner.rs`.
**Implementation plan**:
- File: `crates/clawft-kernel/src/wasm_runner.rs`
- Add `BaseToolRegistry` (Arc-shared, read-only after boot)
- Add `AgentToolOverlay` (per-agent, mutable)
- `WasmToolRunner::resolve_tool()` checks overlay first, then base
- Estimated: ~150 lines new, ~50 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k3:AC-1 (Hierarchical ToolRegistry with Arc-shared base)

##### k3:D2 -- Context-based gate actions: generic tool.exec with rich context

**Original rationale**: Gate evaluation should receive full context (tool name,
effect vector, agent capabilities) not just a boolean allow/deny signal.
**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). This is a refinement of the
existing `GovernanceGate::evaluate()` API.
**Implementation plan**:
- File: `crates/clawft-kernel/src/governance.rs`
- Enrich `GateContext` struct with `tool_name: String`, `effect_vector: EffectVector`
- `GovernanceGate::evaluate()` takes `&GateContext` instead of action string
- Estimated: ~80 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k3:AC-2 (Enriched gate context)

##### k3:D4 -- Wasmtime integration in K4 alongside container runtime

**Original rationale**: The kernel should use Wasmtime (not wasmer or another
runtime) for WASM execution, with disk-persisted compiled module cache.
**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). Wasmtime is already a workspace
dependency. The integration requires wiring it into `WasmToolRunner`.
**Implementation plan**:
- File: `crates/clawft-kernel/src/wasm_runner.rs`
- Replace mock WASM execution with actual `wasmtime::Engine`
- Add `ModuleCache` with disk persistence behind `os-patterns` feature
- Estimated: ~200 lines new, ~100 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k3:AC-5 (Wasmtime activation + disk cache + WASI scope)

##### k3:D12 -- Multi-layer sandboxing: governance + environment + sudo override

**Original rationale**: WASM sandboxing should have three enforcement layers:
(1) governance rules, (2) environment-scoped limits, (3) sudo override for
emergency bypass.
**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). The governance layer exists;
environment scoping and sudo override need implementation.
**Implementation plan**:
- File: `crates/clawft-kernel/src/wasm_runner.rs`, `governance.rs`
- Add `SandboxLayer` enum: `Governance | Environment | SudoOverride`
- `WasmToolRunner::pre_execute()` checks all three layers in order
- Sudo override requires `AgentCapabilities::sudo` flag + chain event
- Estimated: ~120 lines new, ~60 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k3:AC-3 (Multi-layer FsReadFileTool sandboxing)

#### Tier 2: MEDIUM Priority -- Resolve or Defer (6 decisions)

##### k3:D3 -- Implement all 25 remaining tools in K4

**Status change**: pending -> deferred
**Resolution**: DEFER to post-Sprint 10. Implementing 25 tools is a large work
package that belongs in its own sprint. The tool framework (from D1, D4) must
be stable first.
**Defer-until**: Sprint 10 completes and tool framework is tested.
**Commitment impact**: k3:AC-4 deferred to post-Sprint 10.

##### k3:D6 -- Configurable WASI with read-only sandbox default

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT as part of D4 Wasmtime integration. WASI configuration
is part of the Wasmtime setup.
**Implementation**: Bundled with k3:D4 in Sprint 10. Default WASI grants
read-only `/sandbox/` directory. Configurable via `WasiConfig` struct.
**Commitment impact**: No separate commitment.

##### k3:D7 -- Tree metadata for version history

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). Tree version history is needed
for the artifact store.
**Implementation plan**:
- File: `crates/clawft-kernel/src/tree_manager.rs`
- Add `TreeNodeVersion` struct with timestamp, author_pid, prev_hash
- `TreeManager::insert()` creates version entry
- `TreeManager::history()` returns version chain for a path
- Estimated: ~100 lines new
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k3:AC-6 (Tree version history persistence)

##### k3:D8 -- Informational revocation; governance gate handles enforcement

**Status change**: pending -> implemented (minimal)
**Resolution**: IMPLEMENT as documentation + type. Revocation is informational
(logged to chain), not enforcement (governance gate handles that separately).
**Implementation**: Add `ChainEventType::CapabilityRevoked` variant to chain.rs.
No enforcement code needed -- governance gate already denies revoked capabilities.
**Commitment impact**: No separate commitment.

##### k3:D9 -- Central authority + CA chain for tool signing

**Status change**: pending -> deferred
**Resolution**: DEFER. Tool signing requires a PKI infrastructure that is
out of scope for Sprint 09 and 10. The existing Ed25519 signing on chain
entries provides integrity. CA chains are a post-1.0 feature.
**Defer-until**: Post-1.0 release, when multi-operator deployment is prioritized.
**Commitment impact**: k3:AC-7 deferred to post-1.0.

##### k3:D10 -- Separate ServiceApi and BuiltinTool concepts

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10. ServiceApi is for kernel-internal
services (IPC-based); BuiltinTool is for user-facing tool invocations
(request/response based).
**Implementation plan**:
- File: `crates/clawft-kernel/src/service.rs`
- Rename existing `ServiceApi` trait methods to clarify internal vs external
- Add `BuiltinTool` trait with `fn execute(&self, args: ToolArgs) -> ToolResult`
- Estimated: ~80 lines new, ~40 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: No separate commitment.

#### Tier 3: DEFERRED/LOW -- Batch Resolution (4 decisions)

##### k3:D5 -- Disk-persisted compiled module cache

**Status change**: pending -> deferred
**Resolution**: DEFER. Bundled with D4 Wasmtime integration as a sub-feature.
Not a standalone decision.
**Defer-until**: Implemented as part of D4 in Sprint 10.

##### k3:D11 -- Routing-time gate deferred to K5 cluster build

**Status change**: deferred -> confirmed deferred
**Resolution**: Already deferred in original symposium. Routing-time gate
evaluation is a K5+ feature. Confirmed: no Sprint 09/10 action needed.

##### k3:D13 -- WASM snapshots unnecessary now; noted for K6

**Status change**: deferred -> confirmed deferred
**Resolution**: Already deferred. WASM snapshots (save/restore execution state)
are not needed for single-node operation. Confirmed deferred to post-mesh-Phase-2.

##### k3:D14 -- tiny-dancer scoring for native vs WASM routing

**Status change**: pending -> deferred
**Resolution**: DEFER. tiny-dancer is a learned routing system that requires
trajectory data from governance. The governance trajectory learning (k2:D17)
is also deferred. These are coupled.
**Defer-until**: Governance trajectory learning (k2:D17) is implemented.

### Resolution Plan: K2 Decisions (8 pending + 1 blocked)

#### k2:D8 -- ExoChain-stored immutable API contracts

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). API contracts stored as chain
entries enable service versioning and contract verification.
**Implementation plan**:
- File: `crates/clawft-kernel/src/chain.rs`, `service.rs`
- Add `ChainEventType::ApiContractRegistered` with schema hash
- `ServiceRegistry::register()` optionally accepts `ApiContract`
- Contract verified on service lookup (hash match)
- Estimated: ~100 lines new
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k2:C3 (Chain-anchored service contracts)

#### k2:D10 -- WASM-compiled shell with container sandbox

**Status change**: pending -> deferred
**Resolution**: DEFER. The WASM-compiled shell requires both Wasmtime (D4) and
container runtime integration. Sprint 10 delivers Wasmtime; the shell can be
built on top in a subsequent sprint.
**Defer-until**: Sprint 10 completes Wasmtime integration.
**Commitment impact**: k2:C5 deferred to post-Sprint 10.

#### k2:D11 -- Enable post-quantum signing immediately

**Status change**: blocked -> unblocked (implementation plan)
**Resolution**: IMPLEMENT the DualSigner call path in Sprint 09b. The blocker
is that `chain.rs` only calls `Ed25519Signer` even though `rvf-crypto` exposes
`DualKey::sign()` with ML-DSA-65 support.

**Technical analysis** (chain-guardian):
- `rvf-crypto` crate is already in workspace under `exochain` feature
- `DualKey` struct has `sign(&self, data: &[u8]) -> DualSignature`
- `DualSignature` contains both `ed25519_sig` and `ml_dsa_sig`
- The kernel's `ChainEntry` already has fields for both signatures
- Gap: `ChainManager::append()` only calls `self.signer.sign()` (Ed25519)

**Implementation plan**:
- File: `crates/clawft-kernel/src/chain.rs`
- Add `DualSigner` trait: `fn sign(&self, data: &[u8]) -> Result<DualSignature>`
- Implement `DualSigner for DualKey` (delegates to rvf-crypto)
- `ChainManager::new()` accepts `Box<dyn DualSigner>` instead of `Ed25519Signer`
- `ChainManager::append()` calls `dual_signer.sign()` and stores both sigs
- Fallback: if ML-DSA key not available, fill `ml_dsa_sig` with empty vec
- Estimated: ~80 lines new, ~30 lines changed
- Sprint: 09b (this sprint)

**Commitment impact**: Unblocks k2:C6 (Post-quantum dual signing)

**Fallback**: If rvf-crypto API proves insufficient (e.g., DualKey not
constructible from kernel context), defer k2:C6 with rationale:
"ML-DSA-65 signing requires rvf-crypto API extension; deferred until rvf-crypto
v0.3 exposes `DualKey::from_keypair(ed25519, ml_dsa)`."

#### k2:D12 -- Chain-agnostic blockchain anchoring trait

**Status change**: pending -> deferred
**Resolution**: DEFER. Blockchain anchoring (publishing merkle roots to external
chains like Ethereum or Solana) is a deployment-time feature, not a kernel
feature. The `ChainAnchor` trait can be defined but implementation requires
external chain SDKs.
**Defer-until**: Multi-chain deployment use case is prioritized.
**Commitment impact**: k2:C7 deferred to post-1.0.

#### k2:D13 -- Zero-knowledge proofs as foundational capability

**Status change**: pending -> deferred
**Resolution**: DEFER. ZK proofs are a powerful capability but require
significant cryptographic infrastructure (circuit definition, proving key
management, verifier integration). Out of scope for Sprint 09/10.
**Defer-until**: Post-1.0, when privacy-preserving computation is a user requirement.

#### k2:D17 -- Layered routing: tiny-dancer learned + governance enforced

**Status change**: pending -> deferred
**Resolution**: DEFER. Learned routing requires trajectory data from governance
decisions. The governance engine currently logs decisions but does not yet
support trajectory learning (which is itself a pending feature in 08c).
**Defer-until**: Governance trajectory learning is implemented in Sprint 10.

#### k2:D18 -- SONA at K5, training data pipeline from K3 forward

**Status change**: pending -> deferred
**Resolution**: DEFER. SONA (Self-Organizing Neural Architecture) is a research
concept from the Three-Mode Engine. The Weaver's ECC model is the practical
implementation of this concept. SONA as a separate system is not needed.
**Defer-until**: Superseded by Weaver ECC. Close this decision.

#### k2:D20 -- Configurable N-dimensional effect algebra

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT in Sprint 10 (08c). The current `EffectVector` has
fixed dimensions (cpu, memory, network, storage, trust_delta). Making it
configurable enables domain-specific governance.
**Implementation plan**:
- File: `crates/clawft-kernel/src/governance.rs`
- Replace fixed `EffectVector` fields with `dimensions: HashMap<String, f32>`
- Provide `EffectVector::default_kernel()` with the 5 standard dimensions
- `EnvironmentThreshold` validates dimension names match
- Estimated: ~60 lines changed
- Sprint: 10 (08c scope)
**Commitment impact**: Unblocks k2:C9 (Configurable N-dimensional EffectVector)

### Resolution Plan: ECC Decisions (1 pending)

#### ecc:D5 -- DEMOCRITUS as continuous nervous system operation

**Status change**: pending -> implemented (plan)
**Resolution**: IMPLEMENT via Sprint 09c Weaver Runtime. DEMOCRITUS (the
continuous cognitive operation model) is realized through the CognitiveTick
integration. The Weaver runs on every tick, processes new data, updates the
causal model, and emits impulses when confidence changes.
**Implementation**: Covered by Sprint 09c work packages (CognitiveTick
integration, confidence scoring, Meta-Loom persistence).
**Commitment impact**: No separate commitment; covered by Weaver TODO items.

### Commitment Tracking Update

After Sprint 09b, the commitment tracking table becomes:

| Commitment | Status | Resolution |
|-----------|--------|------------|
| k2:C1 | **implemented** | SpawnBackend enum (K2.1) |
| k2:C2 | pending -> Sprint 10 | ServiceApi internal surface trait |
| k2:C3 | pending -> Sprint 10 | Chain-anchored service contracts (via D8) |
| k2:C4 | partial -> Sprint 10 | Dual-layer gate in A2ARouter (via D2) |
| k2:C5 | pending -> deferred | WASM-compiled shell pipeline (via D10) |
| k2:C6 | **blocked -> unblocked** | Post-quantum dual signing (via D11, Sprint 09b) |
| k2:C7 | pending -> deferred | ChainAnchor trait (via D12, post-1.0) |
| k2:C8 | **implemented** | SpawnBackend::Tee variant (K2.1) |
| k2:C9 | pending -> Sprint 10 | Configurable N-dimensional EffectVector (via D20) |
| k2:C10 | **implemented** | K6 SPARC spec |
| k3:AC-1 | pending -> Sprint 10 | Hierarchical ToolRegistry (via D1) |
| k3:AC-2 | pending -> Sprint 10 | Enriched gate context (via D2) |
| k3:AC-3 | pending -> Sprint 10 | Multi-layer sandboxing (via D12) |
| k3:AC-4 | pending -> deferred | 25 tool implementations (via D3, post-Sprint 10) |
| k3:AC-5 | pending -> Sprint 10 | Wasmtime + disk cache + WASI (via D4) |
| k3:AC-6 | pending -> Sprint 10 | Tree version history (via D7) |
| k3:AC-7 | pending -> deferred | CA chain signing (via D9, post-1.0) |
| k5:C1-C5 | **implemented** | All K5 commitments done (K6) |

**Summary**: 9 implemented, 7 scheduled for Sprint 10, 4 deferred, 2 resolved.

---

## C -- Completion

### Work Packages

#### WP-1: K3 Tier 1 Decision Resolution (Day 1-2)

**Owner**: governance-counsel
**Reviewers**: sandbox-warden (D1, D4, D12), kernel-architect (all)

- Resolve k3:D1, D2, D4, D12 with full implementation plans
- Write implementation scope, affected files, estimated lines
- Assign each to Sprint 10 (08c scope)
- Document rationale for each

#### WP-2: K3 Tier 2+3 Decision Resolution (Day 2)

**Owner**: governance-counsel
**Reviewer**: kernel-architect

- Resolve k3:D3, D6, D7, D8, D9, D10 (implement-minimal or defer)
- Confirm k3:D5, D11, D13, D14 (defer with rationale)
- Document all resolutions in batch

#### WP-3: K2 Decision Resolution (Day 3)

**Owner**: governance-counsel
**Reviewers**: chain-guardian (D8, D11, D12), sandbox-warden (D10)

- Resolve k2:D8, D10, D12, D13, D17, D18, D20
- Implement or defer each with documented rationale

#### WP-4: D11 Post-Quantum Unblocking (Day 3-4)

**Owner**: chain-guardian
**Reviewer**: kernel-architect

- Implement `DualSigner` trait in `chain.rs`
- Wire `DualKey::sign()` from rvf-crypto
- Update `ChainManager::append()` to use dual signing
- Add 3+ tests for dual signature creation and verification
- File: `crates/clawft-kernel/src/chain.rs`
- Estimated: ~80 lines new, ~30 lines changed
- If rvf-crypto API insufficient, document blocker and defer with rationale

#### WP-5: ECC Decision Resolution (Day 4)

**Owner**: ecc-analyst
**Reviewer**: weaver

- Resolve ecc:D5 (DEMOCRITUS) -- map to Sprint 09c CognitiveTick integration
- Document how the Weaver's tick-based operation satisfies DEMOCRITUS

#### WP-6: Commitment Tracking Update (Day 5)

**Owner**: governance-counsel
**Reviewer**: kernel-architect

- Update commitment tracking table in symposium documentation
- Update `.weftos/graph/decisions.json` with resolution status
- Verify all decision->commitment chains are consistent
- Create summary document of all 23+ resolutions

### Exit Criteria

- [ ] k3:D1 (Hierarchical ToolRegistry) resolved with implementation plan
- [ ] k3:D2 (Context-based gate actions) resolved with implementation plan
- [ ] k3:D4 (Wasmtime integration) resolved with implementation plan
- [ ] k3:D12 (Multi-layer sandboxing) resolved with implementation plan
- [ ] k3:D3 (25 remaining tools) resolved: defer with rationale
- [ ] k3:D5 (Disk-persisted module cache) formally deferred with rationale
- [ ] k3:D6 (Configurable WASI) resolved: bundled with D4
- [ ] k3:D7 (Tree metadata) resolved with implementation plan
- [ ] k3:D8 (Informational revocation) resolved: minimal implementation
- [ ] k3:D9 (CA chain for tool signing) resolved: deferred post-1.0
- [ ] k3:D10 (Separate ServiceApi/BuiltinTool) resolved with implementation plan
- [ ] k3:D11 (Routing-time gate deferred) confirmed deferred
- [ ] k3:D13 (WASM snapshots) confirmed deferred
- [ ] k3:D14 (tiny-dancer scoring) resolved: deferred
- [ ] k2:D8 (ExoChain API contracts) resolved with implementation plan
- [ ] k2:D10 (WASM shell + sandbox) resolved: deferred
- [ ] k2:D11 (Post-quantum signing) unblocked or formally deferred
- [ ] k2:D12 (Chain-agnostic anchoring) resolved: deferred post-1.0
- [ ] k2:D13 (Zero-knowledge proofs) resolved: deferred post-1.0
- [ ] k2:D17 (Layered routing) resolved: deferred
- [ ] k2:D18 (SONA training pipeline) resolved: superseded by Weaver ECC
- [ ] k2:D20 (N-dimensional effect algebra) resolved with implementation plan
- [ ] ecc:D5 (DEMOCRITUS continuous operation) resolved: maps to Sprint 09c
- [ ] Pending decisions reduced from 23 to <5
- [ ] Blocked decisions reduced from 1 to 0
- [ ] Commitment tracking document updated
- [ ] All decision resolutions recorded in symposium log

### Agent Assignment

| Agent | Role | Work Packages |
|-------|------|---------------|
| **governance-counsel** | Primary owner | WP-1, WP-2, WP-3, WP-6 |
| **chain-guardian** | Implementer | WP-4 (D11 unblocking) |
| **sandbox-warden** | Reviewer | WP-1 (D1, D4, D12), WP-3 (D10) |
| **kernel-architect** | Reviewer | All WPs (architectural consistency) |
| **ecc-analyst** | Owner | WP-5 (ecc:D5) |
| **weaver** | Reviewer | WP-5 (DEMOCRITUS mapping) |

### Expert Review Notes

**governance-counsel**: "Batching the K3 decisions into Tier 1 (4 HIGH, full
resolution) and Tier 2+3 (10 remaining, batch triage) is the right approach.
The key insight is that most K3 decisions target K4 implementation that has not
started. They are not stale -- they are pre-positioned. We resolve the blocking
ones now and schedule the rest for Sprint 10."

**chain-guardian**: "The D11 unblocking is straightforward if rvf-crypto's
DualKey is constructible. I have verified that `DualKey` has a `sign()` method
returning `DualSignature`. The remaining question is key generation: does the
kernel need to generate ML-DSA keys, or can it use pre-existing ones from
config? Recommendation: accept pre-generated keys from `KernelConfig` for now,
defer key generation to post-Sprint 10."

**sandbox-warden**: "D1 (hierarchical ToolRegistry) and D12 (multi-layer
sandboxing) are tightly coupled. The overlay registry (D1) defines what tools
an agent CAN access; the sandbox layers (D12) define HOW those tools are
constrained. Implement them together in Sprint 10."

**kernel-architect**: "The decision to defer k2:D12 (blockchain anchoring),
k2:D13 (ZK proofs), and k3:D9 (CA chain signing) to post-1.0 is correct.
These are deployment-scale features that require external infrastructure.
Including them in Sprint 09 or 10 would be scope creep."

### Implementation Order

```
Day 1:
  WP-1: K3 Tier 1 decisions (D1, D2, D4, D12) -- full resolution

Day 2:
  WP-1: Complete Tier 1 documentation
  WP-2: K3 Tier 2+3 decisions (batch triage)

Day 3:
  WP-3: K2 decisions (D8, D10, D12, D13, D17, D18, D20)
  WP-4: Start D11 unblocking (DualSigner implementation)

Day 4:
  WP-4: Complete D11 implementation + tests
  WP-5: ECC decisions (ecc:D5)

Day 5:
  WP-6: Commitment tracking update
  Final review: all 23+ resolutions documented and consistent
```
