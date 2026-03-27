# Q&A Roundtable: Design Decisions for K3+

**Format**: Questions collected from all five symposium panels.
**Purpose**: These design decisions shape K3+ implementation direction.

---

## Architecture & Identity

### Q1: Service Identity Model
**From**: A2A Services Architect

Should services have their own PIDs, or are they always backed by an
agent process? This affects how ServiceTarget routing works in A2ARouter.

Options:
- (a) Services always map to an agent PID (simpler, current model)
- (b) Services get their own PID namespace (more flexible)
- (c) Services are a separate concept from processes (dual registry)

C - Services may be processes or even other systems providing the service not even running in the agent, but the agent is aware and responsible for it.

### Q2: Agent Backend Selection
**From**: Integration Architect

When spawning an agent, how should the kernel decide between native,
WASM, and container backends?

Options:
- (a) Explicit in SpawnRequest (caller decides)
- (b) Inferred from AgentManifest (manifest declares backend)
- (c) Policy-based (environment rules select backend)
- (d) All of the above (explicit > manifest > policy fallback)
D - This should be flexible, it can be created iteratively allowing explicit initially and adding manifest and policy based in a later revision.

### Q3: Spawn Backend for K3
**From**: K0-K2 Auditor

Should we add `backend: Option<SpawnBackend>` to SpawnRequest now
(trivial change) or wait until K3 implementation is further along?

---

## Protocol & Integration

### Q4: Protocol Priority
**From**: A2A Services Architect

Which protocol is highest priority for K3+ service binding?

Options:
- (a) HTTP APIs (RESTful endpoints)
- (b) Shell scripts/commands
- (c) gRPC services
- (d) MCP tools
- (e) Kernel IPC only (defer multi-protocol)

Layered architecture: kernel IPC → internal API surface → protocol adapters (MCP, gRPC, Shell, HTTP). K3 requires the internal API layer (local to the kernel, running as a service). Networking comes later to expose the API externally. All protocols should work long-term. MCP is especially valuable for allowing services to bind through. gRPC was under-discussed but belongs in the adapter layer alongside the others.

### Q5: Standardization Priority
**From**: Industry Research Analyst

Should we prioritize A2A/MCP compatibility or focus on kernel-native
performance?

Options:
- (a) A2A bridge first (interop with external agents)
- (b) MCP server first (tool ecosystem integration)
- (c) Kernel-native performance first (defer external protocols)
- (d) Both A2A + MCP in parallel (most ambitious)

C+D — Kernel-native performance first (the internal API layer from Q4), with both A2A + MCP as protocol adapters that bind to it. The layered approach means kernel-native IS the foundation for external protocols.

### Q6: Cross-Node Services
**From**: A2A Services Architect

Is cluster mode a K3+ requirement, or can we defer service discovery
across nodes to K6?

Options:
- (a) Required for K3+ (services must be discoverable across nodes)
- (b) Defer to K6 (single-node services first)

K3 handles same-node agent-to-agent and agent-to-service communication. Clustering is K5 (not K6). K3 focuses on local inter-agent and service interaction.

---

## Governance & Security

### Q7: Governance Enforcement Point
**From**: A2A Services Architect

Should gate checks happen at A2A routing time (before message sent)
or at service handler time (before execution)?

Options:
- (a) Routing time (prevent unauthorized messages from reaching services)
- (b) Handler time (service decides based on full context)
- (c) Both (defense in depth)

C — Defense in depth. A2ARouter gets a routing-time gate check (consider tiny-dancer rewrite per RUV ecosystem expert's findings), and handler-time check remains via GovernanceGate. Two layers.

### Q8: Contract Storage
**From**: A2A Services Architect

Should service API schemas be stored in the exochain (immutable) or
in-memory manifest only?

Options:
- (a) ExoChain (immutable, auditable, verifiable)
- (b) In-memory manifest (fast, mutable, no persistence overhead)
- (c) Both (manifest for runtime, chain for audit snapshot)

A — Enforced immutable API contracts and schemas are critical. Related to scores, bonds, performance. Chain-anchored schemas are the source of truth.

### Q9: RVF Integration Depth
**From**: A2A Services Architect

Should every service call generate an RVF witness, or only
governance-critical actions?

Options:
- (a) Every call (complete audit trail, higher overhead)
- (b) Governance-critical only (risk > threshold)
- (c) Configurable per service (service manifest declares audit level)

A+C — Every call by default (witness generation is sub-ms via SHAKE-256, negligible overhead). Configurable per service to turn off if needed, but on by default. Complete audit trail is the standard.

### Q10: Shell Service Security
**From**: A2A Services Architect

How should we sandbox shell commands executed by agent services?

Options:
- (a) Docker exec (container isolation)
- (b) Seccomp + namespaces (Linux sandboxing)
- (c) Governance rules only (trust the gate)
- (d) WASM compilation (compile shell to WASM)

A+D — WASM compilation + container isolation. Shell commands must be enforced and immutable. Compile to WASM, tie to cryptochain for provenance. Container provides the runtime sandbox. Governance gate still checks at both layers (per D7).

---

## Cryptographic Architecture

### Q11: Post-Quantum Timeline
**From**: Industry Research Analyst

When should we add SPHINCS+/Dilithium signatures alongside Ed25519?

Options:
- (a) Now (future-proof from the start)
- (b) K6 timeframe (when cluster needs long-lived keys)
- (c) Wait for hardware acceleration (performance concern)
- (d) Never (Ed25519 sufficient for agent lifetimes)

A — Now. rvf-crypto already has ML-DSA-65 dual-key support imported but not called. Enable it.

### Q12: Blockchain Anchoring
**From**: Industry Research Analyst

Should we add optional blockchain anchoring for external verifiability?

Options:
- (a) Ethereum (widest adoption, highest gas costs)
- (b) Solana (fast finality, lower costs)
- (c) Private consortium chain (Hyperledger)
- (d) Timestamping service (e.g., OpenTimestamps)
- (e) Defer (local chains sufficient for now)

Not E (defer) — but also not picking one chain. Provide the anchoring surface/trait so users choose their chain. Implement the easiest one first (for the trait surface), then others bind to the same interface. Chain-agnostic by design.

### Q13: SNARK Integration
**From**: Industry Research Analyst

Should we add SNARK prover integration for privacy-preserving
governance decisions?

Options:
- (a) Yes, for sensitive governance decisions
- (b) No, current chain logging is sufficient
- (c) Research spike first, then decide

A — Yes. Zero-knowledge is critical for the overall stack. Useful for rollups, governance privacy, and many other applications. ZK is a foundational capability, not optional.

### Q14: Hardware Enclaves
**From**: Industry Research Analyst

Should the kernel support hardware enclaves (SGX, TrustZone) for
confidential agent workloads?

Options:
- (a) Yes, add TEE backend for K3+ agents
- (b) Defer to post-K6 (not critical path)
- (c) Use WASM sandbox as sufficient isolation

A — Yes. Uses the same WASM compilation + container sandbox pattern from Q10/D10. TEE becomes another execution backend in the same layered architecture. No hardware to test on currently, but build to be compatible — define the trait surface so TEE backends can be plugged in when hardware is available.

---

## RUV Ecosystem Strategy

### Q15: Build vs Integrate
**From**: RUV Ecosystem Expert

The ruv ecosystem provides production-ready implementations of nearly
everything K3-K6 needs. How should we approach integration?

Options:
- (a) Maximize ruv crate adoption (fastest development)
- (b) Build custom, adopt ruv selectively (more control)
- (c) Use ruv for infrastructure, custom for domain logic

A — Not just ruv, but any existing technology. If it can be used rather than written and is as good or better than what we need, it's an easy decision. Rust is critical for implementation, but we can look outside Rust for inspiration and patterns. Maximize adoption of production-ready code across the entire ecosystem.

### Q16: Causal Ordering for IPC
**From**: RUV Ecosystem Expert

Should K2 IPC adopt exo-core's Hybrid Logical Clocks for message
ordering?

Options:
- (a) Yes, adopt HLC now (O(1) causal ordering)
- (b) Use ruvector-delta-consensus instead (CRDTs)
- (c) Defer ordering (current timestamps sufficient)

B — ruvector-delta-consensus plus other complementary crates. Combined with the ExoChain, we already have solid HLC through chain events — the chain itself provides causal ordering for anything that flows through it. CRDTs add conflict-free convergence for state that lives outside the chain.

### Q17: Intelligent Routing
**From**: RUV Ecosystem Expert

Should we integrate ruvector-tiny-dancer-core for sub-ms semantic
agent routing?

Options:
- (a) Yes, replace capability-based routing with learned routing
- (b) Layer on top (tiny-dancer for routing hints, capabilities for enforcement)
- (c) Defer to K5 (current routing sufficient for K3)

B — Tiny-dancer provides routing hints and learned semantic dispatch. Governance and enforcement remain at a deeper layer managing routing systematically. Learned routing is advisory; capability/governance enforcement is authoritative.

### Q18: Learning Infrastructure
**From**: RUV Ecosystem Expert

When should we integrate sona (self-optimizing neural architecture)
for agent learning?

Options:
- (a) K3 (start learning early)
- (b) K5 (as planned in roadmap)
- (c) Post-K6 (focus on runtime first)

B with some A — K5 for SONA integration, but K3 and K4 must build training data into all endpoints. Chain events, governance decisions, routing outcomes, service performance — all become data points for SONA. K3/K4 are single-kernel testing phases that vastly improve stats collection. Consider pulling SONA reuptake from K5 into late K4 so it's ready. Keep learning in sight throughout, ensuring real data accumulates so SONA has substance to train on.

---

## IPC & Compatibility

### Q19: Backward Compatibility
**From**: A2A Services Architect

Can we break the current IPC API to add service routing, or must we
maintain compatibility?

Options:
- (a) Breaking changes OK (pre-1.0, move fast)
- (b) Additive only (extend MessageTarget, don't change existing)
- (c) Deprecation cycle (add new, mark old deprecated)

A — MUST break it. That is the whole point of this symposium: find where we need to break current APIs and design decisions to create a solid core that agentic systems, services, and business logic tie to. Services are a strong example — they could be Redis, RabbitMQ, a database backend, APIs, or other abstract types. This requires breaking changes now to implement the pattern correctly. Pre-1.0, this is the time.

### Q20: Effect Algebra Expansion
**From**: Industry Research Analyst

Should we expand EffectVector from 5D to 10D now, or wait for
empirical data?

Options:
- (a) Expand now (velocity, scope, reversibility, uncertainty, cascading)
- (b) Wait for empirical data on current 5D performance
- (c) Make it configurable (N-dimensional with named dimensions)

C — Configurable N-dimensional with named dimensions. Edge devices may need truncated versions with only a few compound vectors. Governance and environment configuration should drive dimensionality. Flexibility is more important than a fixed expansion.

---

## Implementation Priority

### Q21: K3+ Ordering
**From**: Integration Architect

Recommended order is K3 -> K4 -> K6 -> K5 (based on dependency
analysis). Do you agree?

Options:
- (a) K3 -> K4 -> K6 -> K5 (dependency-optimal)
- (b) K3 -> K4 -> K5 -> K6 (feature-complete apps before cluster)
- (c) K3 -> K5 -> K4 -> K6 (apps drive container requirements)
- (d) Different order entirely

B — K3 → K4 → K5 → K6, but K3-K6 are a cycle. May need to loop back to earlier phases from discoveries in later ones. The symposium's purpose is to minimize K0-K2 rework by making the right breaks now. K3-K5 are about creating WASM signed containers (RVF), running them, and integrating "apps" (weft, openclaw, claudecode, etc. — crypto-signed bundles of config + executables that live on-chain). K6 is deep networking and replication.

### Q22: K6 SPARC Spec
**From**: Integration Architect

K6 (cluster/networking) has implementation but no SPARC planning doc.
Should we write one before proceeding?

Options:
- (a) Yes, write SPARC spec first (consistent with other phases)
- (b) No, implementation-first is fine for K6
- (c) Lightweight spec only (architecture + interfaces)

A — Write a K6 SPARC spec with gates. Can be a draft, but it helps because some items from K5 belong in K6. The boundary between K5 (apps/clustering) and K6 (deep networking/replication) needs a clear spec to separate concerns properly.

---

## Summary

22 design questions across 6 categories. The most impactful decisions
are:

1. **Q4 (Protocol Priority)** -- determines the shape of K3+ agent loop
2. **Q15 (Build vs Integrate)** -- determines development velocity
3. **Q21 (K3+ Ordering)** -- determines the implementation sequence
4. **Q9 (RVF Depth)** -- determines audit overhead vs completeness
5. **Q5 (Standardization)** -- determines external interoperability
