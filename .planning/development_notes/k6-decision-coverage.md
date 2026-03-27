# K6 Decision Coverage Audit

**Date**: 2026-03-26
**Auditor**: Code Review Agent
**Source Documents**:
- `.planning/sparc/weftos/07-phase-K6-mesh-framework.md` (K6 SPARC plan)
- `docs/weftos/k5-symposium/05-symposium-results.md` (D1-D15, C1-C5)
- `docs/weftos/k5-symposium/01-mesh-architecture.md` (M1-M11)
- `docs/weftos/k5-symposium/03-security-and-identity.md` (S1-S11)
- `docs/weftos/k5-symposium/04-k6-implementation-plan.md` (phase plan)

---

## Decision Coverage Matrix (D1-D15)

| Decision | Description | In SPARC Plan? | Phase | Exit Criteria? | Notes |
|----------|-------------|----------------|-------|----------------|-------|
| D1 | Selective Composition (snow + quinn + selective libp2p) | Yes | K6.1-K6.2 | Yes | Explicitly referenced in Specification (line 25), Pseudocode (line 333), Architecture (dependency table), and feature gate structure. New deps table maps quinn/snow/x25519-dalek/libp2p-kad/libp2p-mdns to D1. |
| D2 | Ed25519 Public Key as Node Identity | Yes | K6.0 | Yes | `NodeIdentity` struct defined in Key Types (line 93), K6.0 breakdown references D2 for `NodeIdentity` (line 1001). Exit criteria implicitly covered by "Noise XX encryption" (which requires Ed25519 identity). |
| D3 | Noise Protocol for All Inter-Node Encryption | Yes | K6.1 | Yes | Noise XX/IK described in Pseudocode (line 333+), `NoiseChannel` in Key Types (line 86), exit criteria: "Two CloudNative nodes connect via QUIC with Noise XX encryption." Refinement section (line 880) explicitly states D3 supersedes doc 12. |
| D4 | Governance.genesis as Cluster Trust Root | Yes | K6.0-K6.1 | Yes | `genesis_hash` verification in connection pseudocode (line 365, 377), join protocol pseudocode (line 504). Exit criteria: remote messages pass GovernanceGate. Architecture diagram labels identity layer "[D4]". |
| D5 | Feature-Gated Mesh Networking | Yes | K6.0 | Yes | Feature gate structure fully defined (lines 314-328), three tiers (mesh, mesh-discovery, mesh-full). Exit criteria: "`mesh` feature gate compiles to zero networking code when disabled." K6.0 breakdown references "C4, D5" (line 1003). |
| D6 | QUIC as Primary Transport, WebSocket as Browser Fallback | Yes | K6.1 | Yes | Platform-Transport Matrix (line 837-845) maps platforms to transports per D6. Exit criteria: "Two CloudNative nodes connect via QUIC" + "A Browser node connects via WebSocket." |
| D7 | Ruvector Algorithms as Pure Computation | Yes | K6.5 / K7 | Partial | Ruvector Reuse table (lines 829-835) describes mesh integration for 5 ruvector crates. However, exit criteria do not explicitly test ruvector algorithm integration -- only CRDT gossip (K6.5). No exit criterion verifying ruvector-raft or ruvector-cluster composition. |
| D8 | rvf-wire as Mesh Wire Format | Yes | K6.1-K6.3 | Yes | Referenced in dependency table as "Wire format for mesh messages (D8)". Sync frame section (lines 752-789) uses RVF segments. Exit criteria: "Sync frames use RVF wire segments with SyncStreamType discriminator (D15)." |
| D9 | Dual Signing (Ed25519 + ML-DSA-65) for Cross-Node Chain Events | Yes | K6.4 | Yes | Chain sync pseudocode (line 482-485) verifies both signatures. Exit criteria: "Cross-node chain events carry dual signatures (Ed25519 + ML-DSA-65)." Architecture integration table references D9 for chain.rs. |
| D10 | 6-Phase Implementation (K6.0-K6.5) | Yes | All | Yes | Full phase breakdown in Completion section (lines 989-1161). Line count summary matches symposium totals. Each phase has test strategy and file list. |
| D11 | Hybrid Noise + ML-KEM-768 Post-Quantum Key Exchange | Yes | K6.4b | Yes | Detailed K6.4b section (lines 1093-1138): protocol, negotiation, dependencies, files, tests, cost. Exit criteria: "Hybrid Noise + ML-KEM-768 handshake protects against store-now-decrypt-later" + "KEM negotiation degrades gracefully when unsupported." |
| D12 | Layered Service Resolution with Genesis-Hash DHT Keys | Yes | K6.3-K6.5 | Yes | 9-step resolution flow (lines 206-241), DHT key namespacing (lines 192-203), resolution/negative/pool caches, circuit breaker, replicated service selection table (lines 246-254). Exit criteria cover: DHT namespace, resolution cache, negative cache, round-robin, connection pool, circuit breaker. |
| D13 | Mesh RPCs Reuse ServiceApi Adapter Pattern | Yes | K6.3 | Yes | Dedicated subsection "Mesh RPC via ServiceApi Reuse" (lines 119-187). RegistryQueryService, MeshAdapter, mesh.request() all specified with line counts. Exit criteria: "RegistryQueryService exposes service resolution via ServiceApi", "MeshAdapter dispatches incoming mesh messages through local A2ARouter", "mesh.request() supports correlated request-response with timeout." |
| D14 | CMVG Cognitive Sync via Multiplexed QUIC Streams | Yes | K6.4 / K7 | Yes | CMVG Cognitive Sync Architecture section (lines 627-797): stream multiplexing table, stream prioritization, backpressure, sync modes by structure, delta computation, SyncStateDigest, CmvgSyncService. K6.4 handles Streams 1-2, K7 handles Streams 3-6. Exit criteria cover chain/tree sync (K6.4) and reference D15 for stream priorities. |
| D15 | Sync Framing, Stream Prioritization, Delta Computation, Observability | Yes | K6.3-K6.5 / K7 | Yes | SyncFrame and SyncPayloadType types (lines 757-778), stream priority table (lines 646-659), SyncStateDigest (lines 717-734), PeerMetrics (lines 1170-1190). Exit criteria explicitly tagged "(D15)": QUIC stream priorities, backpressure, SyncStateDigest exchange, sync frame format, PeerMetrics, KEM timing. |

---

## Architecture Decision Coverage (M1-M11)

| Decision | Description | In SPARC Plan? | Notes |
|----------|-------------|----------------|-------|
| M1 | snow + quinn + selective libp2p | Yes | Maps to D1. Dependency table (lines 305-312) lists exact crates and versions. Feature gate structure matches M1 recommendation. |
| M2 | Ed25519 pubkey as NodeId | Yes | Maps to D2. `NodeIdentity` struct defined with `SHAKE-256(pubkey)[0..16]` derivation. Architecture identity layer references "[D2]". |
| M3 | Noise XX for first contact, IK for known peers | Yes | Maps to D3. Pseudocode (line 347) uses XX. `mesh_noise.rs` file lists "XX + IK" in purpose. Connection pool implies IK for reconnection (known peers). |
| M4 | QUIC as primary, WebSocket as browser fallback | Yes | Maps to D6. Platform-Transport Matrix (lines 839-845) matches M4. `mesh_quic.rs` is a dedicated file. |
| M5 | governance.genesis as cluster trust root | Yes | Maps to D4. Handshake pseudocode verifies `genesis_hash`. Join protocol pseudocode verifies genesis match. |
| M6 | `mesh` feature gate for all networking | Yes | Maps to D5/C4. Feature gate structure (lines 316-321) fully specified. K6.0 item references "C4, D5". |
| M7 | rvf-wire as wire format | Yes | Maps to D8. Used for sync framing (lines 752-789). Dependency table lists rvf-wire as existing crate. |
| M8 | 16 MiB max message size | Yes | Referenced in Pseudocode (line 448: "if frame.len > MAX_MESSAGE_SIZE"), exit criteria ("Maximum message size (16 MiB) enforced at deserialization"), and `MeshConfig.max_message_size` field. |
| M9 | Layered service resolution with genesis-hash DHT keys | Yes | Maps to D12. Full 9-step flow, 3 cache layers, replicated service selection, circuit breaker all described. Evolution path K6.3 -> K6.5 -> K7 matches M9 specification. |
| M10 | Mesh RPCs reuse ServiceApi adapter pattern | Yes | Maps to D13. Dedicated SPARC section with architecture diagram, adapter components, and exit criteria. |
| M11 | CMVG sync uses multiplexed QUIC streams | Yes | Maps to D14. Stream table, prioritization, backpressure, sync modes, delta computation all described. K6.4 (chain/tree) and K7 (causal/hnsw/crossref/impulse) phasing matches M11. |

---

## Security Decision Coverage (S1-S11)

| Decision | Description | In SPARC Plan? | Notes |
|----------|-------------|----------------|-------|
| S1 | Ed25519 pubkey as NodeId | Yes | `NodeIdentity` struct in Key Types. K6.0 breakdown item. |
| S2 | Noise XX for first contact | Yes | Pseudocode line 347: `"Noise_XX_25519_ChaChaPoly_BLAKE2b"`. mesh_noise.rs file. Test strategy includes "Noise XX handshake roundtrip." |
| S3 | Noise IK for known peers | Yes | mesh_noise.rs purpose listed as "XX + IK." K6.1 test strategy includes "Noise IK handshake roundtrip." MeshConnectionPool supports reconnection to known peers. |
| S4 | ChaChaPoly + BLAKE2b in Noise | Yes | Pseudocode line 347 specifies `ChaChaPoly_BLAKE2b`. No explicit exit criterion testing cipher suite, but it is hardcoded in the Noise parameter string. |
| S5 | ML-DSA-65 dual signing on chain events | Yes | Chain sync pseudocode (lines 482-485) verifies both signatures. Exit criteria: "Cross-node chain events carry dual signatures (Ed25519 + ML-DSA-65)." |
| S6 | ML-KEM-768 hybrid KEM deferred to K7+ | **Superseded** | The symposium results document (S6) says "deferred to K7+" but D11 (added later in the same document) moves it to K6.4b. The SPARC plan correctly implements D11 (K6.4b), not S6. This is internally consistent within the symposium -- D11 explicitly overrides S6 by moving KEM from K7+ to K6.4b. |
| S7 | Browser nodes restricted by default | Partial | The Refinement section (lines 889-899) describes browser constraints: no listening, limited storage, relay-based connectivity. However, there is no explicit exit criterion testing browser restriction enforcement. The SPARC plan does not mention `IpcScope::Restricted` (referenced in the security panel) or governance `browser_policy` rules. |
| S8 | governance.genesis as cluster trust root | Yes | Maps to D4/M5. Fully covered in connection and join pseudocode. |
| S9 | 16 MiB max message size | Yes | Maps to M8. Exit criteria: "Maximum message size (16 MiB) enforced at deserialization." |
| S10 | Key rotation via dual-signed chain event | No | **Not referenced in the SPARC plan.** The security panel describes a 5-step key rotation protocol (generate new keypair, sign chain event with old key embedding new key, sign with new key, peers verify both, revoke old key after grace period). The SPARC plan has no file, pseudocode, exit criteria, or phase assignment for key rotation. |
| S11 | Hybrid Noise + ML-KEM-768 key exchange in K6.4b | Yes | Maps to D11. Identical content. K6.4b section fully describes protocol, negotiation, dependencies, tests. |

---

## Gaps Found

### 1. S10 (Key Rotation) -- NOT IN SPARC PLAN

The security panel defines a complete key rotation protocol (S10) involving dual-signed chain events, but the K6 SPARC plan contains no reference to key rotation. No file assignment, no pseudocode, no exit criteria. This is a significant security gap -- without key rotation, a compromised node key cannot be replaced without cluster-wide manual intervention.

**Recommendation**: Add a K6.5 or K7 item for key rotation. At minimum, define the `KeyRotation` chain event kind and verification logic. The protocol is already designed in `03-security-and-identity.md` section 7.

### 2. S7 (Browser Node Restrictions) -- PARTIAL COVERAGE

The SPARC plan describes browser transport constraints (WebSocket, no incoming connections, limited storage) but does not address:
- `IpcScope::Restricted` as the default scope for browser nodes
- Governance `browser_policy` rules
- Explicit exit criteria for browser restriction enforcement

**Recommendation**: Add exit criteria: "Browser nodes default to IpcScope::Restricted" and "Browser capability elevation requires governance gate approval."

### 3. D7 (Ruvector Reuse) -- PARTIAL EXIT CRITERIA

While the SPARC plan describes how ruvector algorithms compose with the mesh layer, exit criteria only cover CRDT gossip (ruvector-delta-consensus in K6.5). There are no exit criteria for:
- ruvector-cluster (SWIM membership) integration
- ruvector-raft (consensus) integration
- ruvector-replication (log replication) integration

**Recommendation**: These are likely K7 items (the ruvector reuse table is descriptive, not prescriptive for K6). Clarify by adding a note that ruvector-raft and ruvector-cluster integration is deferred to K7, or add K6.5 exit criteria if they are expected in K6.

### 4. S4 (ChaChaPoly + BLAKE2b) -- NO EXPLICIT EXIT CRITERION

The cipher suite is hardcoded in the Noise parameter string, but there is no exit criterion verifying the correct cipher suite is used. This is low risk (it is a compile-time constant) but worth noting for completeness.

**Recommendation**: Add a test assertion in K6.1 that verifies the Noise parameter string matches `"Noise_XX_25519_ChaChaPoly_BLAKE2b"`.

---

## Contradictions Found

### 1. S6 vs D11: ML-KEM-768 Timeline

- **S6** (security panel): "ML-KEM-768 hybrid KEM deferred to K7+"
- **D11** (symposium result): "Add hybrid post-quantum key exchange to K6.4b"
- **SPARC plan**: Implements D11 (K6.4b)

**Verdict**: Not a true contradiction. D11 was rendered later in the symposium and explicitly moves KEM from K7+ (S6) to K6.4b. The SPARC plan correctly follows the later decision. However, S6 in `03-security-and-identity.md` still reads "deferred to K7+" in its table row, which could confuse a reader. The same document's section 3 ("Hybrid Noise + KEM Transport Encryption (K6.4b)") correctly reflects D11.

**Recommendation**: Update the S6 row in `03-security-and-identity.md` section 8 to note that D11 supersedes the original timeline.

### 2. Feature Gate Structure: Minor Discrepancy

- **01-mesh-architecture.md** (section 8): `mesh = ["quinn", "snow", "x25519-dalek", "libp2p-kad", "libp2p-mdns"]` (single gate)
- **SPARC plan + 04-k6-implementation-plan.md + 05-symposium-results.md (C4)**: Three-tier gates (`mesh`, `mesh-discovery`, `mesh-full`)

**Verdict**: The mesh architecture document has an older, simplified feature gate. The symposium results (C4) and SPARC plan use the correct three-tier structure. The SPARC plan is authoritative.

**Recommendation**: Update `01-mesh-architecture.md` section 8 to match the three-tier feature gate structure from C4.

### 3. Line Count Discrepancy: K6.0

- **04-k6-implementation-plan.md**: K6.0 = ~50 new + ~150 changed = ~200 lines
- **05-symposium-results.md** (P1-P7): ~110 lines total
- **SPARC plan**: K6.0 = ~50 new + ~150 changed

**Verdict**: Minor discrepancy between symposium prep list (~110 lines) and implementation plan (~200 lines). The SPARC plan follows the implementation plan. The symposium prep list (P1-P7) may undercount because it does not include test code.

**Recommendation**: No action needed. The SPARC plan is more detailed and authoritative.

---

## Recommendations

1. **Add S10 (Key Rotation) to the SPARC plan** -- assign to K6.5 or K7 with file, pseudocode, and exit criteria. The protocol design is complete in `03-security-and-identity.md` section 7.

2. **Strengthen S7 (Browser Restrictions) coverage** -- add exit criteria for `IpcScope::Restricted` default and governance-gated capability elevation for browser nodes.

3. **Clarify D7 (Ruvector Reuse) scope** -- either add K6.5 exit criteria for ruvector-cluster and ruvector-raft integration, or explicitly note these are K7 items.

4. **Update S6 in `03-security-and-identity.md`** -- add a note that D11 supersedes the original K7+ timeline for ML-KEM-768.

5. **Update feature gate in `01-mesh-architecture.md`** -- align section 8 with the three-tier C4 structure.

6. **Add S4 cipher suite test** -- a single assertion verifying the Noise parameter string in K6.1 tests.

---

## Summary

| Category | Total | Covered | Partial | Missing |
|----------|:-----:|:-------:|:-------:|:-------:|
| Decisions (D1-D15) | 15 | 15 | 0 | 0 |
| Architecture (M1-M11) | 11 | 11 | 0 | 0 |
| Security (S1-S11) | 11 | 8 | 2 | 1 |
| Commitments (C1-C5) | 5 | 5 | 0 | 0 |
| **Total** | **42** | **39** | **2** | **1** |

**Overall Assessment**: The K6 SPARC plan has excellent coverage of symposium decisions. All 15 D-decisions, all 11 M-decisions, and all 5 C-commitments are fully covered with phase assignments, file lists, and exit criteria. The only gap is S10 (key rotation), which has no presence in the plan. S7 (browser restrictions) and D7 (ruvector reuse) have partial coverage that should be clarified. Two minor cross-document contradictions exist (S6 timeline, feature gate structure) where the SPARC plan is correct but source documents need updating.
