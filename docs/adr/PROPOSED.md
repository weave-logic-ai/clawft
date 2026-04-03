# Proposed ADRs -- Undocumented Decisions Audit

**Date**: 2026-04-02
**Auditor**: ADR Architect Agent
**Scope**: Symposium transcripts, planning docs, crate sources, Cargo.toml, CHANGELOG
**Existing ADRs reviewed**: ADR-001 through ADR-023

Next available ADR number: **024**

---

## Criteria

Each proposal below represents an architectural decision that:
- Is already made (code exists, or a symposium decision was rendered)
- Affects multiple crates, security boundaries, or wire formats
- Is NOT covered by any existing ADR (001-023)
- Would be difficult or costly to reverse later

---

## Proposals

### ADR-024: Noise Protocol (snow) for All Inter-Node Encryption

- [ ] **Title**: Noise Protocol XX/IK via snow for mesh encryption
- **Source**: K5 Symposium D3 (`docs/weftos/k5-symposium/03-security-and-identity.md:79-148`), K5 results D3 (`docs/weftos/k5-symposium/05-symposium-results.md:63-69`)
- **Decision**: All inter-node traffic uses Noise Protocol via the `snow` crate. XX pattern for first contact, IK pattern for known peers. No X.509 certificates, no PKI.
- **Impact**: Locks the encryption layer for all mesh communication. Every transport adapter (QUIC, WebSocket, WebRTC, BLE) must frame through Noise. Affects `mesh_noise.rs`, all future transport implementations, and the key management story. Choosing Noise over TLS means no browser-native TLS integration -- browser nodes must use WebSocket with application-layer Noise.
- **Why it needs an ADR**: Irreversible wire-format commitment. Changing the encryption protocol later would break all existing peer relationships.

---

### ADR-025: Ed25519 Public Key as Node Identity

- [ ] **Title**: Ed25519 public key replaces UUID for node identity
- **Source**: K5 Symposium D2 (`docs/weftos/k5-symposium/05-symposium-results.md:55-61`), Security Panel (`docs/weftos/k5-symposium/03-security-and-identity.md:9-64`)
- **Decision**: Node ID is `hex(SHAKE-256(pubkey)[0..16])` derived from an Ed25519 keypair. No UUID, no CA, no shared secrets. The `governance.genesis` chain event is the cluster trust root (D4).
- **Impact**: Every module that references a node (cluster.rs, mesh_*.rs, ipc.rs GlobalPid) must use key-derived identity. Changes the cluster join protocol, peer discovery, and chain replication authentication. Cannot be changed without a cluster-wide re-keying event.
- **Why it needs an ADR**: Foundational identity model for all multi-node features. Hard to change once clusters are deployed.

---

### ADR-026: QUIC Primary Transport, WebSocket Browser Fallback

- [ ] **Title**: QUIC (quinn) as primary mesh transport, WebSocket for browsers
- **Source**: K5 Symposium D6 (`docs/weftos/k5-symposium/05-symposium-results.md:96-103`), Mesh Architecture Panel (`docs/weftos/k5-symposium/01-mesh-architecture.md:44-63`)
- **Decision**: Cloud and Edge nodes use QUIC via the `quinn` crate. Browser and WASI nodes use WebSocket via `tokio-tungstenite`. WebRTC, BLE, and LoRa are future transport implementations behind the `MeshTransport` trait.
- **Impact**: Determines the dependency footprint for networked builds. QUIC requires UDP, which some firewalls block -- affects deployment guidance. The `MeshTransport` trait shape (`listen`, `connect`, `supports`) is the contract all future transports must implement.
- **Why it needs an ADR**: Transport choice affects firewall traversal, dependency size, and platform compatibility for every deployment.

---

### ADR-027: Selective Composition over Full libp2p Framework

- [ ] **Title**: Selective libp2p components (kad, mdns) over full framework or iroh
- **Source**: K5 Symposium D1 (`docs/weftos/k5-symposium/05-symposium-results.md:39-49`), Mesh Architecture Panel (`docs/weftos/k5-symposium/01-mesh-architecture.md:78-80`)
- **Decision**: Use `snow` (Noise) + `quinn` (QUIC) + selective libp2p components (`libp2p-kad` for DHT, `libp2p-mdns` for LAN discovery) rather than the full libp2p stack or iroh. Full libp2p brings ~80 transitive dependencies and an opinionated runtime model. iroh is QUIC-only.
- **Impact**: Controls the dependency budget for mesh networking. Prevents libp2p from dictating the runtime model. Requires maintaining the composition glue between snow, quinn, and libp2p components. Affects every crate that touches peer discovery.
- **Why it needs an ADR**: Framework-level dependency decision that constrains how discovery, routing, and NAT traversal are implemented.

---

### ADR-028: Post-Quantum Dual Signing (Ed25519 + ML-DSA-65)

- [ ] **Title**: Mandatory dual signing for ExoChain events
- **Source**: K2 Symposium D11 (`docs/weftos/k2-symposium/08-symposium-results-report.md:211-224`), K5 Symposium D9 (`docs/weftos/k5-symposium/05-symposium-results.md:127-138`), Cargo.toml line 127 (`weftos-rvf-crypto`)
- **Decision**: Chain events carry both Ed25519 and ML-DSA-65 (Dilithium) signatures. Cross-node events (K6) MUST have both. Local events default to both but can fall back to Ed25519-only. The project maintains a fork `weftos-rvf-crypto` to add ML-DSA-65 support not present in upstream `rvf-crypto`.
- **Impact**: Doubles signature size on chain events (~32+2420 bytes vs ~64 bytes). The fork of rvf-crypto creates a maintenance burden and divergence from upstream. Verification requires both signatures to pass for cross-node events. Affects chain storage, replication bandwidth, and verification latency.
- **Why it needs an ADR**: Cryptographic commitment that affects wire format, storage, and the upstream fork maintenance obligation.

---

### ADR-029: weftos-rvf-crypto Fork Strategy

- [ ] **Title**: Maintain weftos-rvf-crypto as a published fork of rvf-crypto
- **Source**: Cargo.toml lines 124-129 (`weftos-rvf-crypto = "0.3"`, `weftos-rvf-wire = "0.2"`), ADR-012 context
- **Decision**: Rather than use upstream `rvf-crypto` (which lacks ML-DSA-65) or vendoring the code, the project publishes `weftos-rvf-crypto` and `weftos-rvf-wire` as independent crates on crates.io. These are forks, not wrappers.
- **Impact**: Two crates must be maintained in sync with upstream rvf changes. Version conflicts possible if other workspace crates depend on upstream rvf-crypto. The fork naming convention (`weftos-` prefix) must be consistent. Affects publishing strategy and CI.
- **Why it needs an ADR**: Dependency governance decision -- forking a core crypto crate creates long-term maintenance obligations.

---

### ADR-030: CBOR (ciborium) as ExoChain Payload Codec

- [ ] **Title**: CBOR via ciborium for ExoChain event payloads
- **Source**: Cargo.toml line 131 (`ciborium = "0.2"`), `docs/weftos/architecture.md:61` (feature flags table), `chain.rs` usage
- **Decision**: ExoChain event payloads are serialized as CBOR (RFC 8949) using the `ciborium` crate, not JSON or bincode. CBOR was chosen for compact binary representation, self-describing format, and deterministic encoding for hash stability.
- **Impact**: All chain event consumers must understand CBOR. Cross-language tooling (JavaScript chain viewers, Python analysis scripts) needs CBOR libraries. The deterministic encoding requirement means CBOR canonical mode must be enforced for hash-chain integrity. This is a wire format -- changing it later requires a chain migration.
- **Why it needs an ADR**: Wire format for the tamper-evident audit trail. Irreversible once chains exist in production.

---

### ADR-031: rvf-wire as Mesh Wire Format

- [ ] **Title**: rvf-wire zero-copy segments for mesh binary payloads
- **Source**: K5 Symposium D8 (`docs/weftos/k5-symposium/05-symposium-results.md:118-126`)
- **Decision**: Use `rvf-wire` zero-copy segment serialization as the mesh wire format for binary payloads. JSON framing remains available for `KernelMessage` transport. Avoids introducing protobuf, flatbuffers, or other serialization dependencies.
- **Impact**: All mesh protocol messages must be expressible as rvf-wire segments. Constrains the message format to what rvf-wire supports. Cross-language clients need an rvf-wire implementation (currently Rust-only). Affects interoperability with non-Rust mesh participants.
- **Why it needs an ADR**: Wire format for all inter-node communication. Changing it breaks mesh protocol compatibility.

---

### ADR-032: DashMap as Concurrency Primitive for Kernel State

- [ ] **Title**: DashMap for lock-free concurrent kernel data structures
- **Source**: `docs/weftos/architecture.md:104` (Concurrency Model section), Sprint 11 Track 1 (DashMap deadlock fix in a2a.rs), `docs/weftos/sprint11-symposium/10-symposium-synthesis.md:148` (W39: document DashMap convention)
- **Decision**: All kernel registries (ProcessTable, ServiceRegistry, A2ARouter inboxes, TopicRouter, ToolRegistry) use `DashMap` for concurrent access. `tokio::sync::Mutex` is reserved for ChainManager and TreeManager internals where ordering matters.
- **Impact**: DashMap's sharded lock model enables high read concurrency but has known deadlock risks with nested access (already caused a bug in a2a.rs). Every new kernel registry must follow this convention. Affects testing strategy (concurrent access tests needed). The K2b deadlock fix demonstrates the pattern is not zero-risk.
- **Why it needs an ADR**: Cross-cutting concurrency pattern that every kernel module must follow. The deadlock history makes the tradeoffs worth documenting.

---

### ADR-033: Three-Branch Constitutional Governance Model

- [ ] **Title**: Legislative/Executive/Judicial separation in GovernanceEngine
- **Source**: `docs/weftos/kernel-governance.md:1-100`, `governance.rs` (GovernanceBranch enum), K-phases.md (22 genesis rules)
- **Decision**: Governance uses a three-branch constitutional model (Legislative, Executive, Judicial) where no branch can modify another's constraints. 22 genesis rules are chain-anchored at boot as immutable `governance.genesis` and `governance.rule` events. The only mechanism for change is `governance.root.supersede`.
- **Impact**: Defines how all agent permissions, tool access, and service invocations are authorized. The immutability guarantee means governance bugs cannot be patched without a supersede event (which creates a new constitutional root). Affects cluster formation (governance.genesis hash = cluster ID per K5 D4).
- **Why it needs an ADR**: The governance model is the security foundation of the entire system. Its immutability properties and branch separation guarantees are architectural invariants that all other decisions depend on.

---

### ADR-034: Five-Dimensional Effect Algebra for Governance Scoring

- [ ] **Title**: L2-norm effect vector scoring (risk, fairness, privacy, novelty, security)
- **Source**: `docs/weftos/kernel-governance.md:27-29`, K3 Symposium D2 (context-based gate actions), ECC Symposium D7 (per-tree N-dimensional scoring), K2 Symposium C9 (N-dimensional EffectVector refactor)
- **Decision**: Actions are scored along 5 fixed dimensions (risk, fairness, privacy, novelty, security) using L2-norm magnitude. The magnitude is compared against a configurable threshold. K2 Symposium C9 calls for refactoring to N-dimensional configurable named dimensions, but the current 5D model is the accepted baseline.
- **Impact**: Every governance rule, every gate check, and every tool in the 27-tool catalog has an effect vector. Changing dimensions is a breaking change to all governance rules. The C9 refactor (N-dimensional) would be a superseding ADR. Affects how security policies are authored and evaluated.
- **Why it needs an ADR**: The scoring dimensions are embedded in genesis rules, tool catalogs, and governance evaluation logic. Adding or removing a dimension cascades across the system.

---

### ADR-035: Layered Protocol Architecture (ServiceApi Trait)

- [ ] **Title**: Kernel IPC -> ServiceApi -> Protocol Adapters (MCP, gRPC, Shell, HTTP)
- **Source**: K2 Symposium D4 (`docs/weftos/k2-symposium/08-symposium-results-report.md:88-111`), K3 Symposium (C2 status: NOT STARTED as of K3)
- **Decision**: The kernel exposes an internal `ServiceApi` trait that protocol adapters bind to. MCP, gRPC, Shell, and HTTP are all adapters over the same API surface. The agent loop dispatches through the API layer, not directly to protocols.
- **Impact**: Every external integration (MCP tools, HTTP API endpoints, gRPC services) must go through ServiceApi. This is the contract between the kernel and the outside world. Shapes how third-party integrations are built. Currently NOT STARTED per K3 results -- the decision is made but implementation is pending.
- **Why it needs an ADR**: Defines the integration contract for all external protocol bindings. Foundational for K4+ development.

---

### ADR-036: Hierarchical ToolRegistry with Kernel Base + Per-Agent Overlays

- [ ] **Title**: Hierarchical tool registry with shared base and per-agent overlays
- **Source**: K3 Symposium D1 (`docs/weftos/k3-symposium/07-symposium-results-report.md:108-126`), Sprint 09b resolution (`docs/weftos/09-symposium/02-decision-resolutions.md:37-42`)
- **Decision**: ToolRegistry has `parent: Option<Arc<ToolRegistry>>`. Kernel boot creates one shared base registry (27 built-in tools). Per-agent overlays can add, remove, or reconfigure tools. Lookup walks overlay -> base. Governance can inject deny-lists at the agent layer.
- **Impact**: Affects how tools are discovered, how governance restricts tool access per-agent, and how the GUI tool browser displays available tools. The `Arc`-shared base resolves the CF-3 wasteful allocation finding. All future tool registration must follow this pattern.
- **Why it needs an ADR**: Defines the tool access model for all agents. Security-relevant because it controls what tools each agent can execute.

---

### ADR-037: Rust Edition 2024 and MSRV 1.93

- [ ] **Title**: Rust Edition 2024 with minimum supported Rust version 1.93
- **Source**: `Cargo.toml:35-36` (`edition = "2024"`, `rust-version = "1.93"`)
- **Decision**: The workspace uses Rust Edition 2024 (the latest edition) with MSRV 1.93. This is an aggressive choice -- Edition 2024 was stabilized recently and MSRV 1.93 excludes users on older toolchains.
- **Impact**: Enables Edition 2024 language features (e.g., `gen` blocks if stabilized, new borrow checker rules). Excludes contributors and CI environments running older Rust versions. Affects downstream consumers who depend on weftos crates via crates.io. All 22 workspace crates inherit this setting.
- **Why it needs an ADR**: MSRV is an API contract with downstream users. Edition choice affects which language features are available across the entire workspace.

---

### ADR-038: Tauri 2.0 as Desktop Application Shell

- [ ] **Title**: Tauri 2.0 for WeftOS desktop GUI (not Electron)
- **Source**: CHANGELOG.md line 108 (K8 GUI prototype: Tauri 2.0 wrapper), ADR-007 (Zustand + Tauri events), `gui/src-tauri/` in workspace exclude
- **Decision**: The desktop GUI uses Tauri 2.0 as the native shell, with React rendering the frontend. Communication uses Tauri invoke (request/response) and Tauri events (state push). The GUI is excluded from the Cargo workspace to avoid build coupling.
- **Impact**: Locks the desktop platform to Tauri's capabilities (WebView2 on Windows, WebKitGTK on Linux, WebKit on macOS). The invoke/event dual-channel model (ADR-007) is Tauri-specific. Excluding from workspace means separate build tooling. Affects all GUI development.
- **Why it needs an ADR**: Platform choice for the desktop application. Affects build pipeline, cross-platform compatibility, and the GUI-kernel communication model.

---

### ADR-039: SWIM Protocol for Mesh Failure Detection

- [ ] **Title**: SWIM protocol (via HeartbeatTracker) for peer failure detection
- **Source**: `docs/weftos/k6-development-notes.md:53` (mesh_heartbeat.rs: SWIM protocol), K6 implementation map
- **Decision**: Peer liveness detection uses the SWIM protocol with Alive -> Suspect -> Dead state machine. PingRequest/PingResponse messages drive the heartbeat. Default TTL is 60 seconds with 10K entry dedup capacity.
- **Impact**: SWIM's probabilistic failure detection means false positives are possible under network partitions. The Alive -> Suspect -> Dead state machine affects how the cluster handles node departures, which cascades to service migration, chain replication, and distributed process table cleanup.
- **Why it needs an ADR**: Failure detection semantics affect cluster stability and data consistency guarantees.

---

### ADR-040: LWW-CRDT for Distributed Process Table

- [ ] **Title**: Last-Writer-Wins CRDT for cross-node process table merging
- **Source**: `docs/weftos/k6-development-notes.md:51` (mesh_process.rs: DistributedProcessTable LWW CRDT merge), ECC Symposium D4 (CRDTs for convergence)
- **Decision**: The distributed process table uses LWW (Last-Writer-Wins) CRDT semantics for conflict-free merging across nodes. Each node advertises its processes; conflicts are resolved by timestamp.
- **Impact**: LWW means concurrent process state changes on different nodes resolve by wall-clock time, which can be surprising (clock skew matters). Affects process migration, load balancing (`least_loaded_node()`), and the consistency model for the distributed supervisor. The choice of LWW over other CRDT types (ORSet, MVRegister) is a tradeoff favoring simplicity.
- **Why it needs an ADR**: Defines the consistency model for distributed process management. Clock skew sensitivity is a known tradeoff.

---

### ADR-041: ChainAnchor Trait for Blockchain Anchoring

- [ ] **Title**: Chain-agnostic blockchain anchoring via ChainAnchor trait
- **Source**: K2 Symposium D12 (`docs/weftos/k2-symposium/08-symposium-results-report.md:225-249`), K-phases.md K4 section (C7)
- **Decision**: Define a `ChainAnchor` trait with `anchor()`, `verify()`, and `status()` methods. First implementation is a local mock or OpenTimestamps. Ethereum, Solana, and consortium chains implement the same trait later. The trait is chain-agnostic by design.
- **Impact**: Defines the extension point for external blockchain integration. The trait shape constrains what blockchain operations are possible. Affects how audit trail integrity is proven to external parties. Currently K4 scope, not yet implemented.
- **Why it needs an ADR**: API contract for all future blockchain integrations. The trait shape is hard to change once multiple backends implement it.

---

### ADR-042: Three Operating Modes (Act, Analyze, Generate)

- [ ] **Title**: WeftOS cognitive engine operates in three composable modes
- **Source**: `docs/weftos/ecc-symposium/06-three-modes.md` (complete document), ECC Symposium D5 (DEMOCRITUS as nervous system operation)
- **Decision**: The cognitive engine (CMVG/ECC) operates in three modes sharing the same data structures: Act (real-time conversation), Analyze (post-hoc corpus understanding), and Generate (goal-directed agent conversation). The modes compose into a continuous loop linked by ExoChain provenance.
- **Impact**: This is the product architecture -- it defines what WeftOS IS as a cognitive platform. All three modes share the same ExoChain, causal graph, HNSW index, and scoring system. The Generate mode is the foundation for the assessment product (ADR-023). Affects how the product is marketed, documented, and extended.
- **Why it needs an ADR**: Defines the cognitive architecture that differentiates WeftOS from other agent orchestrators. Product-defining decision.

---

### ADR-043: BLAKE3 Forward, SHAKE-256 Present (Hash Migration Plan)

- [ ] **Title**: New ECC code uses BLAKE3; ExoChain keeps SHAKE-256 until K6 migration
- **Source**: ECC Symposium D6 (`docs/weftos/ecc-symposium/05-symposium-results-report.md:49-53`)
- **Decision**: New ECC code (CrossRefs, UniversalNodeId) uses BLAKE3. Existing ExoChain and Resource Tree keep SHAKE-256 until the K6 migration brings everything to BLAKE3 via exo-core. The single-hash-family invariant is maintained within each subsystem.
- **Impact**: Two hash algorithms coexist in the codebase. Cross-references between ECC (BLAKE3) and ExoChain (SHAKE-256) must handle the mismatch. The K6 migration is a hard dependency for hash unification. Affects all code that computes or verifies hashes across subsystem boundaries.
- **Why it needs an ADR**: Two coexisting hash algorithms create a migration obligation. The boundary between BLAKE3 and SHAKE-256 subsystems must be maintained until K6.

---

### ADR-044: wasm32-wasip1 as WASI Build Target (Not wasip2)

- [ ] **Title**: Build for wasm32-wasip1 in CI/release, wasip2 as future target
- **Source**: `scripts/build.sh:145-155` (wasip1 target), Sprint 11 TD-13 (`docs/weftos/sprint11-symposium/10-symposium-synthesis.md:191`), W49 (standardize on wasip2)
- **Decision**: The current CI/release pipeline builds for `wasm32-wasip1`. Sprint 11 TD-13 decided to standardize on wasip2 (component model) going forward, but the build script still targets wasip1. W49 (standardize on wasip2) is listed as P2/Sprint 12 work.
- **Impact**: wasip1 vs wasip2 affects the WASM component model, how WASM tools interact with host capabilities, and forward compatibility. The transition from wasip1 to wasip2 will change the wasmtime-wasi dependency configuration and may affect existing WASM modules.
- **Why it needs an ADR**: WASM target platform choice affects all WASM tool authors and the WASM sandbox (K3). The wasip1-to-wasip2 migration needs a documented plan.

---

### ADR-045: Tiered Router with Permission-Based Model Selection

- [ ] **Title**: TieredRouter replacing StaticRouter for cost-aware model dispatch
- **Source**: `.planning/08-tiered-router.md` (complete design), CHANGELOG.md line 116 (tiered router with multi-provider dispatch)
- **Decision**: Replace `StaticRouter` (always returns the same provider/model) with `TieredRouter` that combines task complexity scoring, user permission levels (zero_trust/user/admin), and cost budgets to select models. Three permission levels map to progressively broader model tier access.
- **Impact**: Defines the LLM cost control model for all deployments. The permission system (zero_trust, user, admin) affects every channel adapter (Telegram, Slack, Discord, IRC). Budget tracking creates state that must persist. Affects how agents are provisioned and how multi-tenant deployments work.
- **Why it needs an ADR**: Cost control and permission model for LLM usage. Affects every deployment and every channel integration.

---

### ADR-046: Forest of Trees Architecture (Polyglot Tree Ensemble)

- [ ] **Title**: CMVG as polyglot tree ensemble linked by CrossRefs and Impulses
- **Source**: ECC Symposium D2 (`docs/weftos/ecc-symposium/05-symposium-results-report.md:23-29`), ECC Symposium D7 (per-tree scoring with uniform CrossRef indexing)
- **Decision**: The cognitive model is NOT one unified graph but a forest of domain-specific structures (ExoChain, Resource Tree, HNSW Index, Causal Graph) linked by CrossRefs (BLAKE3 Universal Node IDs) and Impulses (HLC-sorted ephemeral events). Each tree uses data structures appropriate to its domain. ExoChain stays linear as the witness chain.
- **Impact**: Affects how queries span multiple structures, how synchronization works (each tree syncs independently), and how new domain trees are added. The CrossRef indexing system is the integration point -- adding a new tree means implementing CrossRef linkage. Shapes the entire cognitive substrate architecture.
- **Why it needs an ADR**: The fundamental data architecture of the cognitive platform. Determines how knowledge is stored, linked, and queried.

---

### ADR-047: Self-Calibrating Cognitive Tick

- [ ] **Title**: Boot-time calibrated, runtime-adjusted cognitive tick interval
- **Source**: ECC Symposium D3 (`docs/weftos/ecc-symposium/05-symposium-results-report.md:30-35`), K-phases.md K3c section (calibration.rs: boot-time benchmarking)
- **Decision**: The cognitive tick interval is self-calibrated at boot via synthetic benchmarks, auto-adjusted at runtime based on load, and advertised to peers as a cluster membership property. A glasses node (50ms tick) and a server node (10ms tick) can participate in the same cluster.
- **Impact**: Heterogeneous tick rates affect delta synchronization timing, causal ordering across nodes, and the DEMOCRITUS pipeline budget. The calibration benchmark runs at every boot, adding startup latency. Affects cluster coordination protocols and ECC performance guarantees.
- **Why it needs an ADR**: Defines the timing model for the entire cognitive substrate. Heterogeneous tick rates in clusters create coordination complexity.

---

## Summary

| # | ADR | Category | Reversibility | Priority |
|---|-----|----------|---------------|----------|
| 024 | Noise Protocol encryption | Security | Hard | High |
| 025 | Ed25519 node identity | Security | Hard | High |
| 026 | QUIC + WebSocket transport | Architecture | Medium | High |
| 027 | Selective libp2p composition | Architecture | Medium | Medium |
| 028 | Post-quantum dual signing | Security | Hard | High |
| 029 | weftos-rvf-crypto fork strategy | Governance | Medium | Medium |
| 030 | CBOR for ExoChain payloads | Wire Format | Hard | High |
| 031 | rvf-wire mesh wire format | Wire Format | Hard | High |
| 032 | DashMap concurrency model | Architecture | Medium | Medium |
| 033 | Three-branch governance | Security | Hard | High |
| 034 | 5D effect algebra scoring | Security | Hard | High |
| 035 | ServiceApi layered protocol | Architecture | Medium | High |
| 036 | Hierarchical ToolRegistry | Security | Medium | Medium |
| 037 | Rust Edition 2024 / MSRV 1.93 | Compatibility | Easy | Low |
| 038 | Tauri 2.0 desktop shell | Architecture | Medium | Medium |
| 039 | SWIM failure detection | Architecture | Medium | Medium |
| 040 | LWW-CRDT process table | Architecture | Medium | Medium |
| 041 | ChainAnchor trait | Architecture | Medium | Medium |
| 042 | Three operating modes | Product | Hard | High |
| 043 | BLAKE3/SHAKE-256 coexistence | Architecture | Medium | Medium |
| 044 | wasm32-wasip1 target | Compatibility | Easy | Low |
| 045 | Tiered router permissions | Architecture | Medium | Medium |
| 046 | Forest of trees architecture | Architecture | Hard | High |
| 047 | Self-calibrating cognitive tick | Architecture | Medium | Medium |

### Suggested Prioritization

**Write first** (hard to reverse, security-critical, or product-defining):
024, 025, 028, 030, 033, 034, 042, 046

**Write second** (cross-cutting architecture):
026, 027, 031, 035, 038, 039, 040, 041, 043, 045

**Write last** (lower impact or easy to change):
029, 032, 036, 037, 044, 047
