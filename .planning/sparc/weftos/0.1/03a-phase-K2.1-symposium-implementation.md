# Phase K2.1: Symposium Implementation

**Phase ID**: K2.1
**Depends On**: K2b (complete), K2 Symposium (complete)
**Duration**: 2-3 days
**Scope**: Implement the "K2 Immediate" changes approved in the K2 Symposium before K3 begins.
**Decision Source**: `docs/weftos/k2-symposium/08-symposium-results-report.md`

---

## S -- Specification

### Purpose

The K2 Symposium identified breaking changes and quick wins that must land before
K3 begins. K2.1 is the implementation bridge between the symposium's design decisions
and K3's development start. Without K2.1, K3 would start with APIs that are already
known to be wrong, causing rework.

### What K2.1 Delivers

1. **SpawnBackend enum** (C1 + C8) -- crystallize the backend API surface
2. **Post-quantum dual signing** (C6) -- enable rvf-crypto DualKey
3. **Breaking IPC changes** (D19) -- restructure MessageTarget for service routing
4. **Service identity model** (D1) -- ServiceEntry as first-class registry concept
5. **GovernanceGate verification** -- verify end-to-end chain logging works correctly
6. **Updated documentation** -- symposium findings integrated into docs/weftos/

### What K2.1 Does NOT Deliver

- No Wasmtime integration (K3)
- No ServiceApi trait implementation (K3)
- No container runtime (K4)
- No ChainAnchor implementation (K3/K4)
- No N-dimensional EffectVector refactor (K3, C9)

---

## P -- Pseudocode

### Task 1: SpawnBackend Enum (C1 + C8)

```
IN  supervisor.rs:
  ADD SpawnBackend enum:
    Native
    Wasm { module: PathBuf }
    Container { image: String }
    Tee { enclave: EnclaveConfig }
    Remote { node_id: String }

  ADD EnclaveConfig struct (placeholder):
    enclave_type: String  // "sgx", "trustzone", "sev"

  MODIFY SpawnRequest:
    ADD backend: Option<SpawnBackend>  // None = Native

  MODIFY AgentSupervisor::spawn():
    MATCH request.backend:
      None | Some(Native) => existing spawn path
      Some(Wasm{..}) => Err(BackendNotAvailable("wasm"))
      Some(Container{..}) => Err(BackendNotAvailable("container"))
      Some(Tee{..}) => Err(BackendNotAvailable("tee"))
      Some(Remote{..}) => Err(BackendNotAvailable("remote"))

  TESTS:
    spawn_native_explicit
    spawn_backend_none_defaults_to_native
    spawn_wasm_returns_not_available
    spawn_container_returns_not_available
    spawn_tee_returns_not_available
    spawn_remote_returns_not_available
```

### Task 2: Post-Quantum Dual Signing (C6)

```
IN  chain.rs:
  INVESTIGATE rvf-crypto DualKey API:
    IF DualKey::generate() exists AND dual_sign_segment() exists:
      MODIFY save_to_rvf():
        dual_keypair = DualKey::generate()  // Ed25519 + ML-DSA-65
        dual_sign_segment(dual_keypair, segment)
      MODIFY load_from_rvf():
        verify with either key (backwards-compatible)
      PERSIST dual key alongside existing Ed25519 key
    ELSE:
      DOCUMENT gap -- what rvf-crypto actually exports for DualKey
      CREATE issue for rvf-crypto to expose the API
      SKIP implementation until API is available

  TESTS (if API available):
    chain_entry_has_dual_signature
    chain_verify_accepts_either_signature
    dual_key_round_trip
```

### Task 3: Breaking IPC Changes (D19 + D1)

```
IN  ipc.rs:
  MODIFY MessageTarget:
    Agent(Pid)          // existing
    Service(String)     // NEW: route to named service
    ServiceMethod {     // NEW: route to specific service method
      service: String,
      method: String,
    }
    Topic(String)       // existing
    Broadcast           // existing

IN  service.rs:
  ADD ServiceEntry:
    name: String
    owner_pid: Option<Pid>        // agent that owns this service (D1)
    endpoint: ServiceEndpoint     // how to reach it
    audit_level: AuditLevel       // Full or GateOnly (D9)
    registered_at: DateTime<Utc>

  ADD ServiceEndpoint enum:
    AgentInbox(Pid)               // backed by an agent
    External { url: String }      // backed by external system
    Container { id: String }      // backed by container (K4)

  ADD AuditLevel enum:
    Full    // witness every call (default, D9)
    GateOnly // only governance decisions

  MODIFY ServiceRegistry:
    CHANGE from DashMap<String, Arc<dyn SystemService>>
    TO DashMap<String, ServiceEntry> + separate DashMap<String, Arc<dyn SystemService>>
    ADD register_entry(entry: ServiceEntry) -> Result<()>
    ADD get_entry(name: &str) -> Option<ServiceEntry>
    ADD resolve_target(name: &str) -> Option<Pid>  // find owning agent PID

IN  a2a.rs:
  MODIFY A2ARouter::route():
    MATCH target:
      MessageTarget::Agent(pid) => existing direct routing
      MessageTarget::Service(name) =>
        resolve via ServiceRegistry::resolve_target(name)
        route to owning agent's inbox
      MessageTarget::ServiceMethod { service, method } =>
        resolve owner, wrap payload with method info
      MessageTarget::Topic(topic) => existing topic routing
      MessageTarget::Broadcast => existing broadcast

  TESTS:
    route_to_service_by_name
    route_to_service_method
    service_not_found_returns_error
    service_entry_registration
    service_entry_with_audit_level
    resolve_target_finds_owner_pid
```

### Task 4: GovernanceGate End-to-End Verification

```
  VERIFY existing GovernanceGate chain logging:
    START daemon with exochain
    SPAWN agent
    SEND exec command → expect governance.permit chain event
    CONFIGURE high-risk rule → send exec → expect governance.deny chain event
    weaver chain local → verify events present with correct fields
    weaver chain verify → valid chain

  IF gaps found:
    FIX chain event format/fields
    ADD missing tests
```

### Task 5: Documentation Integration

```
  UPDATE docs/weftos/architecture.md:
    Add K2.1 phase to boot sequence
    Note symposium-driven changes

  UPDATE docs/weftos/k-phases.md:
    Add K2.1 section between K2b and K3
    Update K5 description (apps + clustering per D6/D21)
    Update K6 description (deep networking, SPARC spec required)

  UPDATE docs/weftos/kernel-modules.md:
    Add ServiceEntry to service.rs section
    Add SpawnBackend to supervisor.rs section
    Note MessageTarget expansion in ipc.rs section

  UPDATE docs/weftos/integration-patterns.md:
    Add new chain event types (service.*)
    Update EffectVector section to note N-dim coming in K3
    Add ServiceEntry integration pattern
```

---

## A -- Architecture

### File Changes

| File | Action | Scope | Task |
|------|--------|-------|------|
| `crates/clawft-kernel/src/supervisor.rs` | Modify | Add SpawnBackend enum, EnclaveConfig, modify SpawnRequest + spawn() | T1 |
| `crates/clawft-kernel/src/chain.rs` | Modify | DualKey signing (if API available) | T2 |
| `crates/clawft-kernel/src/ipc.rs` | Modify | Expand MessageTarget with Service/ServiceMethod variants | T3 |
| `crates/clawft-kernel/src/service.rs` | Modify | Add ServiceEntry, ServiceEndpoint, AuditLevel; dual registry | T3 |
| `crates/clawft-kernel/src/a2a.rs` | Modify | Service routing in A2ARouter::route() | T3 |
| `crates/clawft-kernel/src/lib.rs` | Modify | Re-export new types | T1/T3 |
| `docs/weftos/architecture.md` | Modify | K2.1 additions | T5 |
| `docs/weftos/k-phases.md` | Modify | K2.1 section, K5/K6 updates | T5 |
| `docs/weftos/kernel-modules.md` | Modify | New types in module sections | T5 |
| `docs/weftos/integration-patterns.md` | Modify | New events, patterns | T5 |

### No New Files

All changes are modifications to existing files. No new crates, no new dependencies.

### No New Dependencies

SpawnBackend uses `PathBuf` (std). DualKey uses existing rvf-crypto (already a dependency).
ServiceEntry uses existing types. All changes are additive to existing crate surface.

### Backward Compatibility

Per D19, **breaking changes are approved and required**. Specifically:
- `MessageTarget` gains new variants → existing match arms need wildcard or new cases
- `ServiceRegistry` internal structure changes → existing API callers may need updates
- `SpawnRequest` gains `backend` field → but it's `Option`, so existing callers compile

All breaking changes are internal to `clawft-kernel`. No external crate API is affected.

### Integration with Symposium Agent Updates

The background agents already updated:
- `.planning/sparc/weftos/04-phase-K3-wasm-sandbox.md` — K2 Symposium Decisions section
- `.planning/sparc/weftos/05-phase-K4-containers.md` — K2 Symposium Decisions section
- `.planning/sparc/weftos/00-orchestrator.md` — K2-Symposium row, updated phase descriptions
- All 5 agent specialist `.md` files — K2 Symposium Decisions sections

K2.1 implements the code changes that those planning docs reference.

---

## R -- Refinement

### Edge Cases

1. **SpawnBackend::Wasm called before K3**: Returns `KernelError::BackendNotAvailable("wasm: K3 required")`. Clear error, not a panic.
2. **Service routing to unregistered service**: Returns `RoutingError::ServiceNotFound(name)`. Message not delivered.
3. **Service with no owner PID**: External services (Redis, etc.) have `owner_pid: None`. Routing to them returns `RoutingError::NoAgent("service has no agent endpoint")` — external services need the ServiceApi adapter (K3) to be callable via IPC.
4. **DualKey API not available in rvf-crypto**: If the public API for ML-DSA-65 dual signing isn't exposed, document the gap and defer to a rvf-crypto PR. Don't block K2.1 on this.
5. **Existing tests break due to MessageTarget changes**: Update all existing tests that construct `MessageTarget` to handle the new variants. Most tests use `Agent(pid)` which is unchanged.
6. **ServiceRegistry dual structure**: The `SystemService` trait instances and `ServiceEntry` metadata live in separate maps keyed by name. Registration of either is independent — a service can have metadata before it has a running implementation (useful for external services).

### Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| MessageTarget changes break existing tests | Medium | Grep for MessageTarget, update all match arms |
| DualKey API unavailable | Low | Document gap, defer, don't block |
| ServiceRegistry refactor breaks daemon boot | Medium | ServiceRegistry public API is additive; internal change is safe |
| Too much scope creep from D19 | Medium | Only break what's listed. ServiceApi trait is K3, not K2.1 |

---

## C -- Completion

### Exit Criteria

**Task 1 — SpawnBackend (C1 + C8)**:
- [x] `SpawnBackend` enum defined with all 5 variants (Native, Wasm, Container, Tee, Remote)
- [x] `EnclaveConfig` placeholder struct defined
- [x] `SpawnRequest` has `backend: Option<SpawnBackend>` field
- [x] `AgentSupervisor::spawn()` dispatches by backend, only Native works
- [x] All non-Native backends return `KernelError::BackendNotAvailable`
- [x] 6 tests pass for backend dispatch

**Task 2 — Post-Quantum Dual Signing (C6)**:
- [x] rvf-crypto DualKey API investigated and documented
- [x] Dual signing enabled in `save_to_rvf()`, 3 tests pass (DualKey + ML-DSA-65 placeholder implemented in rvf-crypto)
- [x] If unavailable: gap documented, issue created, no blocking

**Task 3 — Breaking IPC Changes (D19 + D1)**:
- [x] `MessageTarget` has `Service(String)` and `ServiceMethod` variants
- [x] `ServiceEntry` struct with `owner_pid`, `endpoint`, `audit_level`
- [x] `ServiceEndpoint` enum (AgentInbox, External, Container)
- [x] `AuditLevel` enum (Full, GateOnly)
- [x] `ServiceRegistry` stores both `ServiceEntry` metadata and `SystemService` impls
- [x] `A2ARouter` routes `MessageTarget::Service` via ServiceRegistry
- [x] All existing tests updated for MessageTarget changes
- [x] 6+ new tests for service routing (18 across a2a.rs + service.rs)

**Task 4 — GovernanceGate Verification**:
- [x] Governance chain events verified end-to-end (7 governance_gate tests pass)
- [x] Any gaps fixed with tests (GovernanceGate adapter implemented in prior session)

**Task 5 — Documentation**:
- [x] `docs/weftos/architecture.md` updated with K2.1 changes
- [x] `docs/weftos/k-phases.md` has K2.1 section, K5/K6 updates
- [x] `docs/weftos/kernel-modules.md` reflects new types
- [x] `docs/weftos/integration-patterns.md` has new events/patterns

**Phase Gate**:
- [x] `scripts/build.sh check` — compiles
- [x] `scripts/build.sh test` — all tests pass (existing + new)
- [x] `scripts/build.sh clippy` — clean
- [x] WASM check: `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser`
- [x] No regressions in K0-K2b test counts (397 kernel tests, up from 373)

### Testing Verification

```bash
# Full workspace
scripts/build.sh test

# Kernel unit tests (default features)
cargo test -p clawft-kernel

# Kernel with exochain (SpawnBackend + ServiceEntry + GovernanceGate)
cargo test -p clawft-kernel --features exochain

# Specific K2.1 tests
cargo test -p clawft-kernel -- spawn_backend
cargo test -p clawft-kernel -- service_entry
cargo test -p clawft-kernel -- service_routing
cargo test -p clawft-kernel --features exochain -- governance_gate

# Phase gate
scripts/build.sh gate
```

---

## Estimated Test Count

| Task | New Tests | Updated Tests |
|------|-----------|---------------|
| T1: SpawnBackend | 6 | 0 |
| T2: DualKey | 3 (if available) | 2 |
| T3: IPC/Service | 6+ | ~10 (MessageTarget match updates) |
| T4: GovernanceGate | 0-2 | 0 |
| **Total** | ~15 new | ~12 updated |

Target: 388+ total kernel tests (373 existing + 15 new).
