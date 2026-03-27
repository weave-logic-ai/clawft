# K2 Symposium Results Report

**Date**: 2026-03-04
**Status**: Complete (22/22 questions answered, 22 decisions, 10 approved changes)
**Branch**: feature/weftos-kernel-sprint

---

## 1. Executive Summary

The K2 Symposium brought together five specialist panels to evaluate
WeftOS kernel readiness for K3+ development. The kernel is 90% ready
with 298 tests passing across 17,052 lines. This report captures the
design decisions, approved changes, and strategic direction emerging
from the symposium discussion.

### Platform Identity

WeftOS is an agent operating system kernel -- a Rust-native runtime that
manages AI agents as first-class processes under constitutional governance.
Every action is auditable via cryptographic hash chains, every capability
is checked at the gate, and every decision is logged to a tamper-evident
ExoChain.

### Where We Stand

- **K0-K2**: Complete. Foundation, process management, IPC, ExoChain,
  governance gate, resource tree -- all implemented and tested.
- **K3-K6**: Types defined, runtimes pending. WASM sandbox, container
  management, application framework, and cluster networking have
  complete type definitions but need runtime wiring.
- **RUV Ecosystem**: 4 of 143 crates fully used. Significant acceleration
  opportunity by adopting existing production-ready crates.

### What's on the Horizon

K3+ introduces agents-as-services: WASM-sandboxed tool execution,
Docker-managed sidecars, manifest-driven application lifecycle, and
eventually multi-node clustering. The governance gate extends to cover
all new action types. The ExoChain logs everything.

---

## 2. Design Decisions

Decisions made during the Q&A roundtable, with rationale.

### D1: Service Identity Model (Q1)

**Decision**: (c) Services are a separate concept from processes.

**Rationale**: Services may be processes, external systems, or anything
an agent is aware of and responsible for. A service is not necessarily
a PID -- it could be a Redis instance, an HTTP endpoint, or a shell
script. The agent that owns it has a PID; the service itself lives in
the ServiceRegistry as a distinct entity.

**Impact**: ServiceRegistry needs a `ServiceEntry` that can reference
an agent PID (owner), an external endpoint, or a container ID. The
A2ARouter resolves `MessageTarget::Service(name)` by looking up the
owning agent's PID in the registry, not by giving the service its own PID.

### D2: Agent Backend Selection (Q2)

**Decision**: (d) All of the above, built iteratively.

**Rationale**: Flexibility with progressive disclosure. Start with
explicit backend selection in SpawnRequest (K2/K3), add manifest-based
inference (K4), then policy-based environment rules (K5+). The fallback
chain is: explicit > manifest > policy > default (Native).

**Impact**: SpawnRequest gets `backend: Option<SpawnBackend>` immediately.
AgentManifest gets `backend` field when K4 lands. Environment config
gets backend selection rules when K5 lands.

### D3: Spawn Backend API Surface (Q3)

**Decision**: Bake the API now, implement incrementally.

**Rationale**: Crystallize the configuration hooks and handles in K2 so
K3+ phases are additive, not breaking. `SpawnBackend` enum starts with
all variants defined; only `Native` is implemented. Each K-phase fills
in the next variant.

**Impact**: See change C1 below.

### D4: Layered Protocol Architecture (Q4)

**Decision**: Layered -- kernel IPC → internal API surface → protocol adapters.

**Rationale**: The question isn't "which protocol" but "what shape is the
API that protocols bind to." K3 requires an internal API layer that runs
local to the kernel as a service. Protocol adapters (MCP, gRPC, Shell, HTTP)
bind to this API surface. Networking comes later to expose the API externally.
All protocols should work long-term; MCP is especially valuable for allowing
services to bind through to external tool ecosystems. gRPC belongs in the
adapter layer alongside the others.

**Impact**: K3 defines a `ServiceApi` trait (or similar) as the internal
surface. Each protocol adapter implements binding to this surface. The
agent loop dispatches through the API layer, not directly to protocols.
See change C2 below.

```
Kernel IPC (K0-K2)
  └─→ ServiceApi (K3 -- internal, local)
        ├─→ MCP adapter (K3/K4)
        ├─→ gRPC adapter (K4/K5)
        ├─→ Shell adapter (K3)
        └─→ HTTP/REST adapter (K5 -- when networking lands)
```

### D5: Standardization Strategy (Q5)

**Decision**: (c+d) Kernel-native first, then both A2A + MCP as adapters.

**Rationale**: The layered architecture from D4 means kernel-native
performance IS the foundation for external protocols. Build the internal
API surface first (c), then A2A and MCP bind to it as protocol adapters
(d). No false choice between performance and interop -- the API layer
gives us both.

**Impact**: K3 focuses on the internal API surface. MCP adapter is
a near-term follow-on (mcp-gate crate is ready). A2A bridge comes
when HTTP adapter is available.

### D6: Cross-Node Scope and K-Phase Assignment (Q6)

**Decision**: K3 = same-node only. Clustering = K5 (moved from K6).

**Rationale**: K3 handles agents talking to agents and services on the
same node. Cross-node service discovery is a clustering concern, but
clustering moves to K5 (from K6) because distributed apps need it before
the full network transport layer. K6 becomes the deep networking and
replication phase.

**Impact**: K3 scopes its ServiceRegistry, A2ARouter, and service API
to single-node operation. K5 extends these with discovery and remote
routing. K6 adds chain replication and Merkle sync. This also changes
the K-phase ordering (see D-pending for Q21).

### D7: Defense-in-Depth Governance (Q7)

**Decision**: (c) Both routing-time and handler-time gate checks.

**Rationale**: Defense in depth. A2ARouter gets a routing-time gate check
that prevents unauthorized messages from reaching services. The handler-
time GovernanceGate remains as the second layer before command execution.
The A2ARouter should also be considered for a tiny-dancer rewrite (per
RUV ecosystem expert's analysis of ruvector-tiny-dancer-core) to add
learned semantic routing alongside capability enforcement.

**Impact**: A2ARouter gains a `GateBackend` check before inbox delivery.
GovernanceGate continues to check at agent_loop dispatch. Two gate
evaluations per service call: one at routing, one at execution. Chain
logs both. Consider tiny-dancer integration for the routing layer in
K3/K4 timeframe.

### D8: Immutable API Contracts (Q8)

**Decision**: (a) ExoChain-stored, enforced immutable API contracts.

**Rationale**: Service API schemas are critical artifacts -- they relate
to scores, bonds, and performance evaluation. Immutable chain-anchored
schemas are the source of truth. A service's contract is as important
as its governance rules; both must be verifiable and tamper-evident.

**Impact**: When a service registers, its `ServiceSchema` is serialized
and appended to the ExoChain as a `service.contract.register` event.
Schema changes produce new chain entries (versioned, not mutated).
Callers can verify they're invoking the correct contract version. See
change C3 below.

### D9: Universal Witness with Configurable Override (Q9)

**Decision**: (a+c) Every call witnessed by default, configurable per service.

**Rationale**: RVF witness generation is sub-microsecond (SHAKE-256
hashing). The "overhead" concern is storage growth, not compute. With
every call witnessed by default, the system maintains a complete audit
trail. Services that generate high-frequency low-value calls can opt
out via their manifest, but the default is full witnessing.

**Impact**: `ServiceApi::call()` wraps every invocation with a witness
chain entry. The `ServiceEntry` (from D1) gains an `audit_level` field:
`Full` (default) or `GateOnly` (only governance decisions logged). This
is a runtime configuration, not a compile-time choice.

### D10: WASM-Compiled Shell with Container Sandbox (Q10)

**Decision**: (a+d) WASM compilation + container isolation for shell commands.

**Rationale**: Shell commands must be enforced and immutable. Compile
shell scripts to WASM modules, tie them to the cryptochain for
provenance verification. The WASM module runs inside a container
sandbox for runtime isolation. Governance gate checks at both layers
(per D7). This creates a verifiable, reproducible, sandboxed execution
path for shell-based service operations.

**Impact**: Shell service pipeline becomes:
1. Shell script → WASM compilation (ahead-of-time, cached)
2. WASM module hash → chain entry (`shell.wasm.compile`)
3. Execution: container sandbox runs WASM module
4. Governance: gate check before compilation AND before execution
5. Witness: full RVF witness for each invocation (per D9)

This is a K3/K4 deliverable -- K3 provides the WASM compilation path,
K4 provides the container sandbox. Until K4, WASM-only sandboxing is
sufficient.

### D11: Post-Quantum Now (Q11)

**Decision**: (a) Enable post-quantum signatures immediately.

**Rationale**: rvf-crypto already imports ML-DSA-65 (Dilithium) dual-key
support -- it's just not called. Enabling it is a quick win that future-
proofs the entire signing infrastructure. Agent lifetimes may be short,
but chain entries are permanent. Signing chains with post-quantum
algorithms protects the audit trail against future quantum attacks.

**Impact**: Enable `DualKey` signing in rvf-crypto usage. Chain entries
get both Ed25519 and ML-DSA-65 signatures. Verification checks either
(backwards-compatible). See change C6 below.

### D12: Chain-Agnostic Blockchain Anchoring (Q12)

**Decision**: Provide the anchoring trait surface, chain-agnostic by design.

**Rationale**: Not deferring (the capability is needed) but also not
picking one blockchain. The pattern is the same as D2/D3/D4: define
the trait surface now, implement the easiest binding first to prove
the interface, then other chains bind to the same trait. Users choose
their anchoring backend. The surface matters more than the first
implementation.

**Impact**: Define a `ChainAnchor` trait with `anchor()`, `verify()`,
and `status()` methods. Implement the simplest binding first (likely
OpenTimestamps or a local mock) to validate the trait shape. Ethereum,
Solana, and consortium chain backends implement the same trait later.
See change C7 below.

```rust
pub trait ChainAnchor: Send + Sync {
    fn anchor(&self, hash: &[u8; 32], metadata: &AnchorMeta)
        -> Result<AnchorReceipt, AnchorError>;
    fn verify(&self, receipt: &AnchorReceipt)
        -> Result<AnchorStatus, AnchorError>;
}
```

### D13: Zero-Knowledge as Foundational Capability (Q13)

**Decision**: (a) Yes -- ZK is critical infrastructure, not optional.

**Rationale**: Zero-knowledge proofs are essential for the overall stack.
Rollups alone justify the integration, but ZK also enables privacy-
preserving governance (prove compliance without revealing decision
details), confidential service invocations, and verifiable computation
receipts. This is a foundational capability that touches governance,
audit, and service execution.

**Impact**: SNARK prover integration becomes a K3/K4 work item.
GovernanceGate gains the ability to produce ZK proofs alongside chain
entries for sensitive decisions. The `GateDecision` can carry an optional
`ZkProof` field. Research the right prover library (likely arkworks or
halo2) during K3.

### D14: TEE Backend -- Build Compatible, Test Later (Q14)

**Decision**: (a) Yes, as another execution backend in the layered architecture.

**Rationale**: TEE (SGX, TrustZone) uses the same WASM compilation +
container sandbox pattern from D10. TEE becomes another `SpawnBackend`
variant. No hardware available for testing currently, but the trait
surface can be defined so TEE backends plug in when hardware is
available. Same philosophy as D2/D12 -- crystallize the interface,
implement when ready.

**Impact**: Add `SpawnBackend::Tee { enclave: EnclaveConfig }` to the
SpawnBackend enum (C1). The variant returns `BackendNotAvailable` until
TEE runtime is implemented. `EnclaveConfig` captures SGX/TrustZone
parameters. No runtime implementation until hardware is available for
testing.

### D15: Maximize Adoption, Not Just RUV (Q15)

**Decision**: (a) Maximize adoption of production-ready code, ecosystem-wide.

**Rationale**: Not limited to ruv crates -- any existing technology that
is as good or better than what we need should be adopted rather than
rewritten. Rust is critical for implementation, but inspiration and
patterns can come from any language or ecosystem. The 143 ruv crates
are the primary acceleration vector, but the principle is broader:
don't build what already exists in production-ready form.

**Impact**: Each K-phase starts with an integration audit: what's
available, what fits, what gaps remain. Custom code only where no
production-ready alternative exists or where domain-specific logic
requires it. The K2 symposium's RUV ecosystem audit (panel 5) becomes
a living document updated each phase.

### D16: CRDT Consensus + Chain-Native Ordering (Q16)

**Decision**: (b) ruvector-delta-consensus for CRDT-based convergence.

**Rationale**: The ExoChain already provides causal ordering through
chain events -- anything that flows through the chain has a natural
HLC. CRDTs via delta-consensus add conflict-free convergence for state
that lives outside the chain (e.g., service registry state, routing
tables, resource tree mutations during concurrent updates). The two
systems complement each other: chain for ordered events, CRDTs for
convergent state.

**Impact**: K3 adopts ruvector-delta-consensus for pub/sub and state
convergence. IPC messages that transit the chain already have ordering.
State that doesn't touch the chain (routing hints, health scores,
capability caches) uses CRDTs for eventual consistency.

### D17: Layered Routing -- Learned + Enforced (Q17)

**Decision**: (b) Layer tiny-dancer on top of governance enforcement.

**Rationale**: Tiny-dancer provides sub-ms semantic routing hints via
FastGRNN-based learned dispatch. Governance and capability enforcement
remain at a deeper authoritative layer. The routing stack becomes:
tiny-dancer suggests → capabilities verify → governance gate checks →
message delivered. Learned routing is advisory; enforcement is
non-negotiable.

**Impact**: A2ARouter gains a `RoutingHint` layer powered by tiny-dancer.
The existing capability check remains the authority. This aligns with
D7 (defense in depth) -- tiny-dancer optimizes routing, governance
controls it. Integration target: K3/K4 timeframe.

### D18: SONA at K5, Training Data from K3 Forward (Q18)

**Decision**: (b+a) K5 for SONA integration; K3/K4 build the training data pipeline.

**Rationale**: SONA needs real data to train on. K3 and K4 are single-
kernel development phases that must build training data into all
endpoints: chain events, governance decisions, routing outcomes, service
call performance, resource usage patterns. Every system we build
produces data points for SONA. Consider pulling the SONA reuptake
process from K5 into late K4 so it's complete and ready when K5 begins.
Learning remains in sight throughout -- it's not deferred, it's
accumulated.

**Impact**: K3/K4 work includes a "training data" dimension:
- Chain events include structured fields for ML consumption
- Governance decisions log effect vectors + outcomes (not just pass/fail)
- Service calls log latency, fuel, memory alongside results
- Resource tree mutations log scoring deltas
- Late K4: SONA reuptake spike -- pull forward, validate against
  accumulated data, confirm K5 integration path

### D19: Breaking Changes Required (Q19)

**Decision**: (a) Breaking changes are mandatory, pre-1.0.

**Rationale**: The purpose of this symposium is to identify where current
APIs and design decisions must break to create a solid core. Services
are a prime example -- they are abstract types (Redis, RabbitMQ, database
backends, APIs, external systems) that require the IPC API to evolve.
`MessageTarget` needs `Service(name)` routing. `ServiceEntry` needs to
exist as a first-class registry concept (D1). The current IPC API was
designed for agent-to-agent only; service routing is a fundamentally
different pattern that requires restructuring, not extending.

**Impact**: K3 can freely restructure `MessageTarget`, `A2ARouter`, and
IPC dispatch. The symposium decisions (D1, D4, D7, D8) all assume this
freedom. No backwards-compatibility shims. Clean break, clean core.

### D20: Configurable N-Dimensional Effect Algebra (Q20)

**Decision**: (c) N-dimensional with named dimensions, configurable per environment.

**Rationale**: Edge devices may need truncated effect vectors with only
a few compound dimensions. Full-featured nodes can use 10D or more.
Governance and environment configuration should drive dimensionality,
not a compile-time constant. This also supports D18 (SONA training) --
as empirical data accumulates, dimensions can be added or tuned without
code changes.

**Impact**: `EffectVector` becomes `EffectVector<N>` or uses a dynamic
`Vec<NamedDimension>` with a configuration-driven schema. Default
environments ship with the current 5D. Production environments can
expand to 10D. Edge environments can compact to 3D (compound risk,
compound privacy, compound security). See change C9 below.

### D21: K3 → K4 → K5 → K6, Iterative Cycle (Q21)

**Decision**: (b) K3 → K4 → K5 → K6, with iteration.

**Rationale**: K3-K6 are a development cycle, not a strict waterfall.
Later phases may drive changes that loop back. The symposium's purpose
is to minimize K0-K2 rework by making the right API breaks now (D19).
Within K3-K6, iteration is expected and healthy.

The phases tell a coherent story:
- **K3**: WASM signed containers (RVF-anchored tool execution)
- **K4**: Container runtime (Docker/Podman for sidecars + services)
- **K5**: Apps -- crypto-signed bundles (config + executables + manifest)
  that integrate weft, openclaw, claudecode, etc. as on-chain-verified
  applications. Clustering enables multi-node app distribution.
- **K6**: Deep networking -- chain replication, Merkle sync, full
  distributed orchestration

**Impact**: K5 is redefined as the "apps + clustering" phase (combines
original K5 app framework with clustering from original K6). K6 becomes
purely deep networking and replication. This is consistent with D6.

### D22: K6 SPARC Spec Required (Q22)

**Decision**: (a) Write a K6 SPARC spec with gates.

**Rationale**: Some items currently assigned to K5 belong in K6, and
vice versa. A clear SPARC spec for K6 establishes the boundary between
K5 (apps + clustering) and K6 (deep networking + replication). The spec
can be a draft, but it needs architecture, interfaces, and gate
criteria to prevent scope confusion during K5 development.

**Impact**: Write `docs/weftos/sparc/k6-cluster-networking.md` with:
- Architecture: network transport, chain replication, tree sync
- Interfaces: RemoteRouter, ChainReplicator, TreeSync traits
- Gates: what must pass before K6 is considered complete
- Boundary: explicit list of what is K5 vs K6
See change C10 below.

---

## 3. Approved Changes

Changes approved during the symposium, ready for implementation.

### C1: Add SpawnBackend to SpawnRequest

**Files**: `crates/clawft-kernel/src/supervisor.rs`
**Scope**: ~15 lines + tests
**Phase**: K2 (now)

Add `SpawnBackend` enum and `backend: Option<SpawnBackend>` field to
`SpawnRequest`. `None` defaults to `Native`. Other variants are defined
but return `Err(KernelError::BackendNotAvailable)` until their K-phase
implements them.

```rust
pub enum SpawnBackend {
    Native,                          // K0-K2: tokio task with agent_loop
    Wasm { module: PathBuf },        // K3: Wasmtime sandbox
    Container { image: String },     // K4: Docker/Podman
    Remote { node_id: String },      // K6: cluster delegation
}

pub struct SpawnRequest {
    pub name: String,
    pub role: AgentRole,
    pub capabilities: AgentCapabilities,
    pub backend: Option<SpawnBackend>,  // None = Native
}
```

**Tests**:
- `spawn_native_explicit` -- SpawnBackend::Native works
- `spawn_backend_none_defaults_to_native` -- None behaves like Native
- `spawn_wasm_returns_not_available` -- Wasm variant errors cleanly
- `spawn_container_returns_not_available` -- Container variant errors cleanly

### C2: Define ServiceApi Internal Surface

**Files**: `crates/clawft-kernel/src/service.rs` (or new `api.rs`)
**Scope**: ~40 lines trait + types
**Phase**: K3 (next)

Define the internal API surface that protocol adapters bind to. This is
the contract between the kernel's service runtime and external protocol
adapters (MCP, gRPC, Shell, HTTP).

```rust
/// The internal API surface that protocol adapters bind to.
/// K3 implements this for local dispatch; K5 extends for remote.
pub trait ServiceApi: Send + Sync {
    fn call(&self, request: ServiceRequest) -> Result<ServiceResponse, ServiceError>;
    fn list_methods(&self) -> Vec<ServiceMethod>;
    fn schema(&self, method: &str) -> Option<ServiceSchema>;
}

pub struct ServiceRequest {
    pub method: String,
    pub params: serde_json::Value,
    pub caller: AgentId,           // who is calling
    pub trace_id: Option<String>,  // for chain correlation
}
```

Protocol adapters implement the translation layer:
- **Shell adapter**: maps CLI args → ServiceRequest, stdout → ServiceResponse
- **MCP adapter**: maps JSON-RPC tool calls → ServiceRequest (via mcp-gate)
- **gRPC adapter**: maps protobuf → ServiceRequest (K4/K5)
- **HTTP adapter**: maps REST → ServiceRequest (K5+, when networking lands)

**Tests**: Deferred to K3 implementation sprint.

### C3: Chain-Anchored Service Contracts

**Files**: `crates/clawft-kernel/src/chain.rs`, `crates/clawft-kernel/src/service.rs`
**Scope**: ~30 lines
**Phase**: K3

When a service registers its API schema via `ServiceApi::schema()`, the
schema is serialized and appended to the ExoChain as a
`service.contract.register` event. Schema changes produce new versioned
entries -- contracts are never mutated, only superseded.

```rust
// Chain event when service contract is registered
chain.append("service", "service.contract.register", json!({
    "service": service_name,
    "version": schema_version,
    "methods": method_list,
    "schema_hash": shake256(&schema_bytes),
}));
```

Callers can verify contract integrity before invocation by comparing
the method schema hash against the chain entry.

### C4: Dual-Layer Gate in A2ARouter

**Files**: `crates/clawft-kernel/src/a2a.rs`
**Scope**: ~20 lines
**Phase**: K3

Add a `GateBackend` check to `A2ARouter::route()` before message
delivery to the target inbox. This is the routing-time governance
check (D7). The agent_loop's GovernanceGate remains as the handler-time
check.

```rust
// In A2ARouter::route()
if let Some(gate) = &self.gate {
    match gate.check(&sender_id, "ipc.route", &route_context) {
        GateDecision::Deny { reason, .. } => return Err(RoutingError::Denied(reason)),
        GateDecision::Defer { reason } => return Err(RoutingError::Deferred(reason)),
        GateDecision::Permit { .. } => { /* continue routing */ }
    }
}
```

### C5: WASM-Compiled Shell Pipeline

**Files**: `crates/clawft-kernel/src/wasm_runner.rs`
**Scope**: ~50 lines (types + pipeline definition)
**Phase**: K3 (WASM path) / K4 (container sandbox)

Define the shell-to-WASM compilation pipeline that produces verifiable,
chain-anchored, container-sandboxed shell execution.

```rust
pub struct CompiledShellTool {
    pub source_hash: [u8; 32],     // SHAKE-256 of original script
    pub wasm_module: Vec<u8>,      // compiled WASM bytes
    pub chain_ref: ChainEntryRef,  // link to shell.wasm.compile event
}
```

K3 delivers WASM-only sandboxing. K4 wraps execution in a container.

**Tests**: Deferred to K3 implementation sprint.

### C6: Enable Post-Quantum Dual Signing

**Files**: `crates/clawft-kernel/src/chain.rs`, exochain signing calls
**Scope**: ~20 lines (enable existing rvf-crypto DualKey path)
**Phase**: K2 (now -- quick win)

rvf-crypto already provides `DualKey` with ML-DSA-65 (Dilithium)
alongside Ed25519. Currently only Ed25519 signing is called. Enable
dual signing for chain entries so the audit trail is post-quantum
resilient from the start.

```rust
// Current: sign_segment(&keypair, &segment)
// Change to: dual_sign_segment(&dual_keypair, &segment)
// Verification: verify with either key (backwards-compatible)
```

**Tests**:
- `chain_entry_has_dual_signature` -- both Ed25519 and ML-DSA-65
- `chain_verify_accepts_either_signature` -- backwards compatibility
- `dual_key_round_trip` -- sign + verify cycle

### C7: ChainAnchor Trait for Blockchain Anchoring

**Files**: `crates/clawft-kernel/src/chain.rs` (or new `anchor.rs`)
**Scope**: ~40 lines (trait + types + mock implementation)
**Phase**: K3/K4

Define the chain-agnostic anchoring trait. First implementation is a
local mock or OpenTimestamps (simplest to validate the interface).
Ethereum, Solana, and consortium backends implement later.

```rust
pub trait ChainAnchor: Send + Sync {
    fn anchor(&self, hash: &[u8; 32], metadata: &AnchorMeta)
        -> Result<AnchorReceipt, AnchorError>;
    fn verify(&self, receipt: &AnchorReceipt)
        -> Result<AnchorStatus, AnchorError>;
}

pub struct AnchorReceipt {
    pub chain_name: String,        // "ethereum", "solana", "local", etc.
    pub tx_ref: String,            // chain-specific transaction reference
    pub timestamp: u64,
    pub proof: Vec<u8>,            // chain-specific inclusion proof
}
```

**Tests**:
- `mock_anchor_round_trip` -- anchor + verify with mock backend
- `anchor_receipt_serialization` -- receipt survives round-trip

### C8: Add SpawnBackend::Tee Variant

**Files**: `crates/clawft-kernel/src/supervisor.rs` (extends C1)
**Scope**: ~5 lines
**Phase**: K2 (define), K6+ (implement)

Add `Tee { enclave: EnclaveConfig }` variant to `SpawnBackend` enum.
Returns `BackendNotAvailable` until TEE runtime is implemented on
actual hardware.

```rust
pub enum SpawnBackend {
    Native,
    Wasm { module: PathBuf },
    Container { image: String },
    Tee { enclave: EnclaveConfig },   // NEW
    Remote { node_id: String },
}
```

**Tests**:
- `spawn_tee_returns_not_available` -- clean error until implemented

### C9: Configurable N-Dimensional EffectVector

**Files**: `crates/clawft-kernel/src/governance.rs`
**Scope**: ~60 lines (refactor + config)
**Phase**: K3 (refactor from fixed 5D to configurable)

Refactor `EffectVector` from a fixed 5-field struct to a configurable
N-dimensional system with named dimensions. The default schema ships
with the current 5D (risk, fairness, privacy, novelty, security).
Environments configure their dimension set.

```rust
pub struct EffectVector {
    dimensions: Vec<NamedDimension>,
}

pub struct NamedDimension {
    pub name: String,        // "risk", "fairness", "velocity", etc.
    pub value: f64,          // [0.0, 1.0]
    pub weight: f64,         // environment-configured weight
}

impl EffectVector {
    pub fn magnitude(&self) -> f64 {
        // weighted euclidean magnitude
        self.dimensions.iter()
            .map(|d| (d.value * d.weight).powi(2))
            .sum::<f64>()
            .sqrt()
    }
}
```

Environment configs:
- **Full (default)**: 5D → expandable to 10D
- **Edge**: 3D compound (risk, privacy, security)
- **Custom**: user-defined dimensions via config

**Tests**:
- `effect_vector_5d_backwards_compatible` -- default matches current behavior
- `effect_vector_3d_edge` -- compact edge profile works
- `effect_vector_10d_expanded` -- additional dimensions function
- `effect_vector_magnitude_weighted` -- weights affect threshold checks

### C10: K6 SPARC Spec (Draft)

**Files**: `docs/weftos/sparc/k6-cluster-networking.md` (new)
**Scope**: ~200 lines (architecture + interfaces + gates)
**Phase**: Pre-K6 planning

Write a SPARC specification for K6 that covers:
- Architecture: network transport, chain replication, tree sync
- Interfaces: RemoteRouter, ChainReplicator, TreeSync traits
- Gates: completion criteria for K6
- K5/K6 boundary: explicit scope separation

This separates K5 concerns (apps + clustering) from K6 concerns
(deep networking + replication) and prevents scope confusion.

---

## 4. Decision Index

All 22 questions answered. See `07-qa-roundtable.md` for full Q&A text.

| Q# | Topic | Category | Decision | Changes |
|----|-------|----------|----------|---------|
| Q1 | Service Identity | Architecture | D1 | -- |
| Q2 | Backend Selection | Architecture | D2 | -- |
| Q3 | Backend API Surface | Architecture | D3 | C1 |
| Q4 | Protocol Priority | Protocol | D4 | C2 |
| Q5 | Standardization | Protocol | D5 | -- |
| Q6 | Cross-Node Scope | Protocol | D6 | -- |
| Q7 | Governance Enforcement | Governance | D7 | C4 |
| Q8 | Contract Storage | Governance | D8 | C3 |
| Q9 | RVF Witness Depth | Governance | D9 | -- |
| Q10 | Shell Security | Governance | D10 | C5 |
| Q11 | Post-Quantum | Crypto | D11 | C6 |
| Q12 | Blockchain Anchoring | Crypto | D12 | C7 |
| Q13 | SNARK/ZK | Crypto | D13 | -- |
| Q14 | Hardware Enclaves | Crypto | D14 | C8 |
| Q15 | Build vs Integrate | Strategy | D15 | -- |
| Q16 | Causal Ordering | Strategy | D16 | -- |
| Q17 | Intelligent Routing | Strategy | D17 | -- |
| Q18 | Learning Infrastructure | Strategy | D18 | -- |
| Q19 | Backward Compatibility | IPC | D19 | -- |
| Q20 | Effect Algebra | IPC | D20 | C9 |
| Q21 | K3+ Ordering | Implementation | D21 | -- |
| Q22 | K6 SPARC Spec | Implementation | D22 | C10 |

---

## 5. Strategic Direction

### K2 Immediate (Breaking Changes + Quick Wins)

- **C1**: Add SpawnBackend enum to SpawnRequest (incl. Tee variant, C8)
- **C6**: Enable post-quantum dual signing (rvf-crypto DualKey)
- **D19**: Break IPC API -- restructure MessageTarget for service routing
- Verify GovernanceGate chain logging end-to-end

### K3: WASM Signed Containers + ServiceApi

K3 is about creating WASM signed containers (RVF-anchored) and the
internal API surface that protocols bind to.

- **C2**: Define ServiceApi trait (internal surface)
- **C3**: Chain-anchored service contracts
- **C4**: Dual-layer gate in A2ARouter
- **C5**: WASM-compiled shell pipeline
- **C9**: Configurable N-dimensional EffectVector
- Complete WASM sandbox runtime (Wasmtime integration)
- Wire exec dispatch in agent_loop
- Shell adapter + MCP adapter for ServiceApi
- Same-node agent-to-agent and agent-to-service communication
- Define K3 governance action types (tool.wasm.*)
- tiny-dancer routing hints layer (D17)
- ruvector-delta-consensus for off-chain state convergence (D16)
- Build training data into all endpoints (D18)

### K4: Container Runtime + Service Orchestration

- Docker/Podman container runtime (bollard)
- Container sandbox for WASM-compiled shell tools (D10)
- gRPC adapter for ServiceApi
- Application manifest → supervisor → services pipeline
- **C7**: ChainAnchor trait + first backend implementation
- SNARK prover research spike (D13 -- arkworks or halo2)
- Continue training data accumulation
- Late K4: SONA reuptake spike -- pull from K5, validate data (D18)

### K5: Apps + Clustering

K5 integrates crypto-signed app bundles (weft, openclaw, claudecode,
etc.) with clustering for multi-node distribution. Apps are RVF-signed
bundles of config + executables that live on-chain.

- App framework wiring (manifest → supervisor → services)
- Crypto-signed app bundles with chain verification
- Clustering and cross-node service discovery
- HTTP/REST adapter for ServiceApi (network exposure)
- A2A protocol bridge
- Remote agent routing through A2ARouter
- SONA integration with accumulated K3/K4 training data
- Service-level governance rules

### K6: Deep Networking + Replication

Requires SPARC spec first (C10).

- Network transport layer (TCP/WebSocket)
- Chain replication with Raft consensus
- Distributed resource tree (Merkle sync)
- Full multi-node orchestration
- TEE backend implementation (when hardware available, D14)
- **C10**: K6 SPARC spec (draft, pre-implementation)

### K-Phase Cycle

K3-K6 are iterative, not waterfall (D21). Discoveries in later phases
may drive changes that loop back. The symposium's breaking changes
(D19) minimize K0-K2 rework, but K3-K6 iteration is expected.

### Ecosystem Integration Targets

Maximize adoption of production-ready code, not just ruv (D15):

| Phase | Crate / Technology | Purpose |
|-------|-------------------|---------|
| K2 | rvf-crypto DualKey | Post-quantum signing (C6) |
| K2 | cognitum-gate-kernel | Audit trail verification |
| K3 | mcp-gate | MCP adapter for ServiceApi |
| K3 | ruvector-tiny-dancer-core | Semantic routing hints (D17) |
| K3 | ruvector-delta-consensus | CRDT state convergence (D16) |
| K3 | ruvector-snapshot | WASM state snapshots |
| K4 | arkworks / halo2 | SNARK prover (D13) |
| K4/5 | sona | Self-optimizing agents (D18) |
| K6 | exo-dag | DAG-based audit with Merkle proofs |
| K6 | exo-identity | W3C DIDs for agent identity |

---

## 6. Symposium Panel Reports

Full reports from each panel are available in this directory:

- [00 Overview](./00-symposium-overview.md)
- [01 Platform Vision](./01-platform-vision.md)
- [02 K0-K2 Completeness Audit](./02-k0-k2-audit.md)
- [03 A2A & Services Architecture](./03-a2a-services.md)
- [04 Industry Landscape](./04-industry-landscape.md)
- [05 RUV Ecosystem Audit](./05-ruv-ecosystem.md)
- [06 K3+ Integration Architecture](./06-k3-integration.md)
- [07 Q&A Roundtable](./07-qa-roundtable.md)
