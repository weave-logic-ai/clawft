# Planning Documents 07-15 Review and Discrepancy List

**Date**: 2026-03-25
**Reviewer**: Code Review Agent
**Compared against**: K5 Symposium results, K6 readiness audit, implemented kernel code
**Branch**: `feature/weftos-kernel-sprint`

---

## Summary

Documents 07-15 represent the visionary layer of WeftOS design, written on
2026-02-28 before significant kernel implementation work. The K5 symposium
(2026-03-25) refined several positions from these documents, particularly
doc 12 (networking). Many crates referenced in doc 07 do not exist. The
exo-resource-tree (doc 13/15) and exochain substrate (doc 14) have been
partially implemented in K0. Documents 09-11 remain largely aspirational.

---

## Document 07: RuVector Deep Integration

**File**: `.planning/sparc/weftos/07-ruvector-deep-integration.md`
**Status**: PARTIALLY OUTDATED

### Key Discrepancies

1. **Non-existent crates** (Section 0, lines 29-42): The document itself was
   corrected on 2026-03-01 with a reality-check section. The following crates
   referenced in the original architecture (Sections 2-5) do NOT exist:
   `cognitum-gate-tilezero`, `ruvector-nervous-system`,
   `ruvector-delta-consensus`, `prime-radiant`, `rvf-wasm`, `rvf-kernel`,
   `rvf-crypto`, `rvf-wire`, `mcp-gate`, `sona`,
   `ruvector-tiny-dancer-core`, `ruvector-cognitive-container`.

2. **Feature gate Cargo.toml** (Section 2, lines 138-199): Lists dependency
   blocks for all non-existent crates. Only `ruvector-cluster`, `ruvector-raft`,
   `ruvector-replication`, and `wasmtime`/`bollard` are real.

3. **ClusterServiceRegistry adapter** (Section 3, lines 270-316): Proposes
   replacing `ServiceRegistry` with `ClusterServiceRegistry` wrapping
   `ClusterManager`. In practice, `ClusterService` was implemented as a
   separate `SystemService` that syncs into `ClusterMembership`, not as a
   replacement for `ServiceRegistry`. The two coexist.

4. **Phase K1 replacements** (Section 3, lines 340-399): Proposes replacing
   `CapabilityChecker` with `cognitum-gate-tilezero::TileZero`. This crate
   does not exist. Instead, K2.1 implemented `GovernanceGate` and `GateBackend`
   trait as the three-way decision mechanism.

5. **ruvector-delta-consensus for IPC** (Section 3, K2): Proposes using
   `ruvector-delta-consensus` for CRDT-based causal ordering. This crate does
   not exist. K6.5 plans to use CRDT gossip for distributed process tables
   but the algorithm implementation remains TBD.

### Items Now Implemented

- `ClusterService` wrapping `ruvector_cluster::ClusterManager` (Section 0, lines 46-58)
- Daemon RPC for cluster commands (Section 0, lines 56-58)
- CLI cluster commands (Section 0, lines 59-60)
- ruvector-core made optional in distributed crates (Section 0, lines 62-63)

### Items Still Valid for Future Phases

- Raft leader election as kernel service (line 67)
- Replication via `ReplicaSet` + `SyncManager` (line 68)
- Vector store feature for distributed vectors (line 70)

### Items Superseded by Symposium

- **Transport layer**: Doc 07 proposes ruvector-based transports. Symposium
  D1 chose snow + quinn + selective libp2p instead.
- **Wire format**: Doc 07 proposes `rvf-wire` from ruvector. Symposium D8
  confirmed rvf-wire but notes it is already in the workspace, not from
  ruvector (the ruvector `rvf-wire` does not exist; a local workspace
  version does).

### Recommendation

**Archive the aspirational sections (2-5) but keep Section 0** as the
authoritative record of what exists. Update the document header to clarify
that Sections 2-5 describe a design that assumed crates which were never
built.

---

## Document 08: Ephemeral OS Architecture

**File**: `.planning/sparc/weftos/08-ephemeral-os-architecture.md`
**Status**: PARTIALLY OUTDATED

### Key Discrepancies

1. **Layer 0 references QuDAG libp2p** (Section 2, line 82-83): States that
   "QuDAG libp2p" provides P2P networking. QuDAG is part of the exochain
   project but the libp2p component is feature-gated and not used. Symposium
   D1 chose snow + quinn + selective libp2p-kad/mdns instead of full libp2p.

2. **Layer 0 references rvf-kernel** (line 83): Claims "rvf-kernel (125ms boot,
   single-file containers)". This crate does not exist in ruvector.

3. **Layer 1 references BLAKE3 hashes** (line 96): States data is
   "content-addressed using BLAKE3 hashes". Current chain.rs uses SHAKE-256.
   Symposium Q6 asks whether to migrate. This is a valid open question, not
   necessarily outdated.

4. **Layer 2 references ruvector-delta-consensus and ruvector-nervous-system**
   (lines 66-67): Neither crate exists.

5. **Cluster topology** (Section 3.2, lines 138-164): Describes libp2p-based
   topology. Symposium replaced this with the 5-layer mesh model.

6. **AgentUser model** (Section 3.3, lines 169-194): Proposes `AgentUser`
   with `Did`, `AgentRole`, `capabilities`, `budget`, `key_state`. The
   implemented kernel uses simpler `ProcessEntry` with `agent_id`, `state`,
   `spawn_config`. DID-based identity is not yet implemented.

### Items Now Implemented

- `NodePlatform` enum (CloudNative, Edge, Browser, Wasi) in cluster.rs
- `ClusterMembership` with `PeerNode` tracking
- Platform trait for native/browser/WASI builds

### Items Still Valid for Future Phases

- Multi-tenant fabric vision (post-K6)
- Agent-as-user model with DID identity (post-K6)
- Cryptographic filesystem concept (relies on exochain DAG)
- Environment-scoped governance (doc 09 elaborates)

### Items Superseded by Symposium

- P2P networking via libp2p/QuDAG replaced by snow + quinn (D1)
- rvf-kernel boot model (no implementation exists)

### Recommendation

**Keep as architectural vision document** but add a header note stating that
the networking specifics (Layers 0, 2) have been superseded by the K5
symposium mesh architecture. The multi-tenant and agent-as-user concepts
remain valid long-term goals.

---

## Document 09: Environments and Learning

**File**: `.planning/sparc/weftos/09-environments-and-learning.md`
**Status**: CURRENT (aspirational, not yet due)

### Key Discrepancies

1. **Implementation Strategy table** (Section 6.1, lines 617-624): References
   phase assignments that may not match reality:
   - "K1: `EnvironmentId` in `AgentCapabilities`" -- not implemented in K1.
   - "K2: `EnvironmentId` in `KernelMessage`" -- not implemented in K2.
   - "K5: `environments` in `weftapp.toml`" -- K5 app framework does not
     include environment declarations yet.
   - "K6.4: `GovernanceScope` enforcement" -- symposium K6 plan does not
     include per-environment governance; it focuses on mesh transport.

2. **SONA engine references** (Section 4, lines 287-319): References `SonaEngine`,
   `MicroLoRA`, `TrajectoryBuilder`, `EWC++`. None of these exist as
   implemented code. The SONA concept comes from the ruvector `sona` crate
   which does not exist.

3. **Cross-environment learning propagation** (Section 4.5, lines 434-495):
   Describes a sophisticated pattern propagation system. No implementation
   exists or is planned for K6.

### Items Still Valid for Future Phases

- Environment model (Dev/Staging/Production) with risk thresholds
- GovernanceScope per environment
- Constitutional invariants concept
- Learning loop concept (record -> evaluate -> learn)
- All new files listed in Section 7 (environment.rs, learning_loop.rs, etc.)

### Items Superseded by Symposium

- None directly. This document operates at a higher level than K6 networking.

### Recommendation

**Keep as-is.** This is a post-K6 vision document. Update the phase assignments
in Section 6.1 to reflect that these are K7+ items, not K1-K5 extensions.

---

## Document 10: Agent-First Architecture

**File**: `.planning/sparc/weftos/10-agent-first-single-user.md`
**Status**: PARTIALLY OUTDATED

### Key Discrepancies

1. **Boot sequence** (Section 3.1, lines 117-150): Describes PID 0 as root agent,
   PID 1 as message-bus, etc. The actual kernel boot (boot.rs) does not spawn
   agents in this exact sequence. Services are registered via `ServiceRegistry`
   and `service.rs`, not via `.agent.toml` file parsing.

2. **`.agent.toml` manifest** (Section 2, lines 57-99): Proposes agent manifest
   files. These are not implemented. The kernel uses `SpawnBackend` and
   `SpawnConfig` structs for agent configuration (implemented in K2.1).

3. **ServiceManager** (Section 7.3, lines 635-691): Proposes a `ServiceManager`
   that reads `.agents/` directory, does topological sort, and spawns in order.
   Not implemented. Services are registered programmatically in boot.rs.

4. **Agency struct** (Section 3.2, lines 172-221): Proposes `Agency` with
   `max_children`, `allowed_roles`, `capability_ceiling`. Not implemented
   as a standalone struct. Agent spawning limits are handled via
   `GovernanceGate` and capability checks.

5. **Revised build order** (Section 5, lines 367-491): Proposes a complete
   K0-K5 resequencing with agent-first approach. The actual implementation
   did not follow this revised order -- it followed the original K0-K5
   sequence with selective adoption of agent concepts.

### Items Now Implemented

- Process table with PID allocation
- IPC bus (KernelIpc)
- Agent lifecycle (Starting -> Running -> Suspended -> Stopping -> Exited)
- Service health checks
- `SpawnBackend` for agent spawning (K2.1, partial implementation of this vision)

### Items Still Valid for Future Phases

- `.agent.toml` manifest format (could be implemented as a K7 item)
- ServiceManager with topological sort
- Agency struct for spawn delegation control
- Multi-user mode (Section 3.3, post-K5)

### Recommendation

**Keep as vision document** but add a header note that the actual
implementation adopted a hybrid approach: the kernel retains programmatic
service registration rather than manifest-driven boot. The `.agent.toml`
concept remains a valid future direction.

---

## Document 11: Local Inference Agents

**File**: `.planning/sparc/weftos/11-local-inference-agents.md`
**Status**: CURRENT (aspirational, not yet due)

### Key Discrepancies

1. **ruvllm crate reference** (Section 1, line 34): References `ruvllm` crate
   from ruvector for GGUF inference. This crate may or may not exist; it is
   not in the ruvector crates audited in doc 07 Section 0.

2. **ruvltra-claude-code model** (Section 1, line 36): References a specific
   HuggingFace model. Existence and suitability unverified.

3. **ruvector-tiny-dancer-core** (Section 1, line 45): Referenced for
   FastGRNN routing. This crate does not exist per doc 07 audit.

4. **Inference service agent** (Section 2, lines 63-110): Described as
   `.agent.toml` manifest. The `.agent.toml` format is not implemented
   (see doc 10 review).

### Items Still Valid for Future Phases

- Tiered inference routing (local GGUF -> cloud fallback)
- Inference service as kernel service agent
- Model-per-agent pattern for specialized agents
- Air-gapped inference independence
- SONA integration for continuous model improvement

### Recommendation

**Keep as-is.** This is a post-K6 vision document. No symposium decisions
conflict with it. When inference integration begins, verify that referenced
crates/models actually exist.

---

## Document 12: Networking and Pairing

**File**: `.planning/sparc/weftos/12-networking-and-pairing.md`
**Status**: PARTIALLY OUTDATED (superseded by K5 symposium in several areas)

### Detailed Comparison with Symposium

#### 1. Architecture Alignment with 5-Layer Model

Doc 12 proposes a 4-layer transport stack (Section 2, lines 64-84):
```
Layer 4: Agent IPC Protocol
Layer 3: Secure Channel (Noise/TLS/PostQuantum)
Layer 2: Peer Discovery (mDNS, Kademlia, Bootstrap)
Layer 1: Base Transport (TCP/QUIC/WebSocket)
```

The symposium defines a 5-layer model (01-mesh-architecture.md, Section 2):
```
APPLICATION    (WeftOS IPC, Chain Sync, Tree Sync)
DISCOVERY      (Kademlia, mDNS, Bootstrap)
ENCRYPTION     (Noise Protocol)
TRANSPORT      (QUIC, WebSocket, TCP)
IDENTITY       (Ed25519 keypair, governance.genesis)
```

**Alignment**: Partial. Doc 12's layers 1-3 map to the symposium's Transport,
Encryption, and Discovery layers. However:
- Doc 12 lacks an explicit IDENTITY layer. Identity is implicit in the
  pairing handshake (Section 3.2) using DIDs.
- Doc 12 lacks an APPLICATION layer; agent IPC is Layer 4 but the symposium
  separates chain sync and tree sync as distinct application-layer protocols.
- The symposium's IDENTITY layer uses Ed25519 pubkey-derived NodeId (D2).
  Doc 12 uses exochain DIDs which are more complex
  (`did:exo:<base58(blake3(pubkey)[0..20])>`). The symposium simplifies
  to `hex(SHAKE-256(pubkey)[0..16])`.

#### 2. Transport Approach: snow+quinn Recommendation

Doc 12's position (Section 2, lines 88-106):
- TCP is the default transport. "A `telnet` to the discovery port should
  show gossip traffic."
- Encryption is opt-in per connection. Discovery gossip is plain TCP.
- QUIC is optional for multiplexed connections.

Symposium position (D1, D3, D6):
- QUIC (quinn) is the PRIMARY transport for Cloud and Edge nodes (D6).
- ALL inter-node traffic uses Noise encryption (D3). No plaintext.
- snow + quinn + selective libp2p is the chosen composition (D1).

**Conflict**: Doc 12's "TCP default, encryption opt-in" is directly
contradicted by symposium D3 (Noise mandatory). The symposium explicitly
rejects plaintext inter-node traffic. Doc 12's position that "discovery
traffic contains no sensitive data" (line 129) is overridden -- the
symposium encrypts everything to prevent traffic analysis.

#### 3. Identity Model: Ed25519 NodeId

Doc 12's position (Section 3.2, lines 260-398):
- Uses exochain DIDs for identity (`did:exo:...`).
- 5-step pairing handshake (HELLO/CHALLENGE/PROVE/ACCEPT/BOND).
- Identity proven via Ed25519 signature of a challenge nonce.

Symposium position (D2):
- Node identity is `hex(SHAKE-256(pubkey)[0..16])`, derived directly from
  Ed25519 public key.
- No DID layer needed for node identity (DIDs remain for agent identity).
- 2-phase handshake: Noise XX handshake (mutual auth) + WeftOS handshake
  (capabilities + chain head exchange).

**Conflict**: Doc 12's 5-step handshake is more complex than needed. The
Noise XX pattern already provides mutual authentication (both sides prove
they hold their static keys). The symposium simplifies to: Noise XX proves
identity, then a single WeftOS handshake message exchanges metadata. Doc
12's BOND step is deferred entirely.

#### 4. Discovery: Kademlia+mDNS

Doc 12's position (Section 3.1, lines 200-257):
- mDNS for LAN discovery.
- Kademlia DHT for WAN discovery.
- Bootstrap nodes as fallback.
- Gossip protocol for capability advertisement.
- Capability-based search via DHT.

Symposium position (D1, 01-mesh-architecture.md Section 6):
- Kademlia DHT via libp2p-kad (optional, behind mesh-discovery feature).
- mDNS via libp2p-mdns (optional, behind mesh-discovery feature).
- Bootstrap/seed peers as the always-available baseline.
- Static seed peers work without libp2p dependencies.

**Alignment**: Good. Both agree on mDNS + Kademlia + bootstrap. The
symposium makes DHT/mDNS optional (feature-gated) while doc 12 treats them
as core. The symposium does not include doc 12's gossip-based capability
advertisement as a discovery method; service discovery is handled at the
application layer (K6.5) rather than the discovery layer.

#### 5. Items in Doc 12 NOT Covered by Symposium

- **Trust levels** (Section 3.4, lines 505-570): Unknown -> Paired ->
  Trusted -> Bonded progression with reputation scoring. The symposium
  does not define trust levels; it uses governance.genesis hash match as
  a binary trust decision (D4).

- **Bond mechanism** (Section 3.5, lines 647-744): DeFi-style bonds with
  staking, slashing, and economic guarantees. Entirely absent from the
  symposium. This is a doc 12 original that would be a post-K6 feature.

- **Trust engine** (Section 3.4, lines 580-644): `TrustEngine` with
  `InteractionRecord`, `Violation`, `TrustConfig`. Not in symposium scope.

- **Network service agent manifest** (Section 4, lines 749-799): Proposes
  `network-service.agent.toml` with peer discovery, pairing, trust, and
  bond tools. The symposium does not use .agent.toml manifests.

- **NodeCapability enum** (Section 3.3, lines 436-502): Rich capability
  advertisement including `Inference`, `Storage`, `Tools`, `Relay`,
  `ExternalLlm`, `TrainingCompute`. The symposium uses simpler string-based
  capabilities inherited from `PeerNode.capabilities: Vec<String>`.

- **ChannelMetrics** (Section 2, lines 172-185): Per-channel throughput
  and latency tracking. Not in symposium scope.

- **Post-quantum ML-KEM-768 per channel** (Section 2, lines 135-138):
  Doc 12 proposes per-channel post-quantum encryption for bonded channels.
  Symposium D9 provides post-quantum via dual signing (Ed25519 + ML-DSA-65)
  on chain events, not per-channel encryption.

#### 6. Symposium Decisions that Contradict Doc 12

| Symposium Decision | Doc 12 Position | Contradiction |
|-------------------|-----------------|---------------|
| D3: All traffic Noise-encrypted | Discovery gossip is plain TCP (line 129) | **Direct conflict** |
| D2: NodeId = hex(SHAKE-256(pubkey)) | DID-based identity (line 150) | **Simplification** |
| D1: snow+quinn primary | TCP default, QUIC optional (lines 88-106) | **Transport priority reversed** |
| D4: governance.genesis as trust root | Trust earned through reputation (lines 505-570) | **Different trust model** |
| D6: QUIC primary, WS fallback | TCP primary, QUIC optional | **Transport priority reversed** |

### Recommendation

**Keep as vision document** with a prominent header note: "Networking
architecture was refined by the K5 Symposium (2026-03-25). See
`07-phase-K6-mesh-framework.md` for the authoritative K6 implementation
plan. Key superseded areas: transport priority, encryption policy, identity
model, handshake protocol. Items NOT in symposium scope (trust levels, bonds,
capability-based discovery, network service agent) remain valid for post-K6
phases."

---

## Document 13: Exo-Resource-Tree

**File**: `.planning/sparc/weftos/13-exo-resource-tree.md`
**Status**: PARTIALLY IMPLEMENTED

### Key Discrepancies

1. **Implementation status** (Section 0, lines 19-49): The document itself
   tracks implementation status. K0 items are implemented (ResourceTree, CRUD,
   Merkle, MutationEvent, TreeManager facade, chain logging, integrity
   verification). K1 items (permission engine, delegation certs) are stubs only.

2. **exo-core, exo-identity, exo-consent dependencies** (Section 2.1, lines
   108-113): The Cargo.toml references `exo-core`, `exo-identity`,
   `exo-consent`, `exo-dag`. The actual `exo-resource-tree` crate uses a
   simplified model.rs that does not pull in all exochain subcrates. The
   ResourceId uses a simple `[u8; 32]` hash, not `exo_core::Blake3Hash`.

3. **Cedar policy integration** (Section 2.1, line 118): Listed as optional
   feature. Not implemented and not planned for K6.

4. **Permission engine check algorithm** (Section 5, referenced in K1): The
   walk-up-tree permission check with delegation cert shortcut is not
   implemented. Current `CapabilityChecker` uses a flat capability check,
   not tree-based resolution.

### Items Now Implemented

- ResourceId, ResourceKind, ResourceNode core types (model.rs)
- ResourceTree with CRUD, namespace bootstrap, Merkle recomputation
- TreeManager facade coupling tree + chain + mutation log
- Boot-time tree construction
- CLI: weaver resource {tree, inspect, stats}
- `/network/peers/` namespace populated during cluster join

### Items Still Valid for Future Phases

- Permission engine with tree-walk algorithm (K1 deferred)
- DelegationCert lifecycle (K1 deferred)
- IPC topic tree nodes (K2+)
- App subtrees (K5+)
- Environment subtrees (K7+)

### Recommendation

**Keep and update Section 0** to reflect current implementation state after
each sprint. The document serves as both specification and implementation
tracker.

---

## Document 14: ExoChain Cryptographic Substrate

**File**: `.planning/sparc/weftos/14-exochain-substrate.md`
**Status**: PARTIALLY IMPLEMENTED

### Key Discrepancies

1. **Chain hierarchy** (Section 2.1, lines 34-38): Defines Local (chain_id=0),
   Global Root (chain_id=1), and App Chains (chain_id=2+). Only Local
   (chain_id=0) is implemented. Global Root and App Chains are K6+ items.

2. **RVF Segment types** (Section 3, lines 66-70): Defines ExochainEvent
   (0x40), ExochainCheckpoint (0x41), ExochainProof (0x42). ExochainEvent
   and ExochainCheckpoint are implemented. ExochainProof (Merkle inclusion
   proof) is not implemented -- this is the "Merkle proof generation" RED
   item from the readiness audit.

3. **Boot integration** (Section 4, lines 91-101): All boot integration items
   are implemented. Genesis event, boot phase logging, TreeManager creation,
   CronService registration, cluster events, shutdown checkpoint.

4. **DAA Rules Engine** (Section 8, lines 173-179): References DAA (Dynamic
   Agent Allocation) rules engine as the policy backend for chain event
   validation. DAA is not implemented. The GovernanceGate and GateBackend
   trait serve a similar purpose.

5. **Future phases** (Section 6, lines 149-162): K1 permission chain events
   are not implemented. K6 global root chain and multi-node sync are the
   subject of the current K6 plan.

### Items Now Implemented

- ChainManager with genesis, append, checkpoint, verify_integrity
- ChainEvent with SHAKE-256 linking
- ExoChainHeader (64-byte repr(C))
- RVF segment persistence (save_to_rvf, load_from_rvf)
- Ed25519 + ML-DSA-65 dual signing
- Witness chain
- Boot-to-chain integration
- Shutdown checkpoint

### Items Still Valid for Future Phases

- Global Root Chain (K6.4)
- App Chains (K6+)
- ExochainProof Merkle inclusion proofs (K6.4)
- K1 permission chain events (deferred)

### Recommendation

**Keep as-is.** This is well-maintained and the implementation status
accurately reflects what exists. Update after K6 implementation.

---

## Document 15: Example RVF-ExoChain-Resource-Tree

**File**: `.planning/sparc/weftos/15-example-rfv-exochain-resource-tree.md`
**Status**: CURRENT (reference example)

### Key Discrepancies

1. **References exo-core types** (lines 28-30): Uses `Blake3Hash`, `Signature`,
   `HLC` from `exo_core`. The actual implementation uses simplified types
   (e.g., `[u8; 32]` for hashes) rather than exo-core wrappers.

2. **References exo-identity, exo-consent** (lines 29-30): Uses `Did`,
   `BailmentPolicy`, `ConsentProof`. These exochain subcrate types are not
   directly used in the clawft-kernel implementation. The kernel has its own
   simpler equivalents.

3. **Cedar policy mention** (line 142): References `cedar-policy = "4"` as
   optional. Not implemented.

4. **"We're literally one crate away from a production-grade agentic IAM
   substrate"** (line 146): Optimistic. The actual implementation required
   significant adaptation from the exochain model to the clawft kernel model.

### Items Now Implemented

- Core data model (ResourceId, ResourceKind, ResourceNode) adapted into
  `crates/exo-resource-tree/src/model.rs`
- ResourceTree with permission check concept (check() method exists as stub)
- Integration with clawft-kernel via TreeManager

### Recommendation

**Keep as reference.** This is a design example, not a specification. It
correctly illustrates the target architecture even if the implementation
diverged in type details.

---

## Cross-Document Observations

### Pattern: Aspirational Crate References

Documents 07, 08, 09, 11 all reference ruvector crates that do not exist.
This is a systemic issue: the vision documents were written assuming a
ruvector ecosystem that is much smaller than described. The reality check
in doc 07 Section 0 partially addresses this, but documents 08-11 were not
similarly updated.

**Action**: All documents should note which ruvector crates they depend on
and whether those crates actually exist.

### Pattern: .agent.toml Not Implemented

Documents 10, 11, and 12 all propose `.agent.toml` manifests as the standard
way to define service agents. This format is not implemented. The kernel uses
programmatic service registration via `boot.rs` and `ServiceRegistry`. The
`.agent.toml` concept remains a valid future direction but should not be
assumed in current K6 planning.

### Pattern: DID vs Ed25519 Identity

Documents 08, 09, 12 use exochain DIDs (`did:exo:...`) for agent/node identity.
The symposium (D2) simplified node identity to `hex(SHAKE-256(pubkey)[0..16])`.
DIDs remain valid for agent-level identity but are not used for node-level
identity in the K6 mesh.

### Symposium Impact Summary

| Decision | Documents Affected | Impact |
|----------|-------------------|--------|
| D1 (snow+quinn) | 07, 08, 12 | Replaces ruvector/libp2p transport assumptions |
| D2 (Ed25519 NodeId) | 08, 12 | Simplifies identity from DID to pubkey-derived |
| D3 (Noise mandatory) | 12 | Contradicts "encryption opt-in" |
| D4 (genesis trust root) | 12 | Replaces reputation-based trust |
| D5 (feature gates) | All | Confirms existing pattern |
| D6 (QUIC primary) | 12 | Reverses TCP-default position |
| D7 (ruvector pure computation) | 07 | Confirms adapter approach |
| D8 (rvf-wire) | 07, 14 | Confirms existing choice |
| D9 (dual signing) | 14 | Extends existing capability to cross-node |
| D10 (6-phase K6) | 12 | Replaces doc 12's implicit phasing |
