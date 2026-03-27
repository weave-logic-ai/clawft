# K6 Developer Readiness Review

**Date**: 2026-03-26
**Reviewer**: Code Review Agent (independent)
**Document**: `.planning/sparc/weftos/07-phase-K6-mesh-framework.md`
**Context**: `.planning/development_notes/k6-readiness-audit.md`

## Overall Assessment: NEEDS WORK

The plan is detailed and well-structured for a design document. A senior Rust
developer familiar with the codebase could implement K6.0 and K6.1 from this
plan alone. However, K6.2 through K6.5 have gaps that would force a developer
to consult symposium documents, guess at design decisions, or block on open
questions. Eight open questions (Q1-Q8) remain unresolved, and several of them
gate K6.4 implementation directly.

---

## Section-by-Section Review

### Specification: PASS (with caveats)

The specification is the strongest section. All new files are listed with
purpose, phase, and estimated line counts. All modified files are listed with
specific changes. Key types include Rust code with field definitions.

- **Missing types**:
  - `TransportListener` -- referenced by `MeshTransport::listen()` return type but never defined. A developer needs at minimum an async `accept()` method signature.
  - `MeshStream` -- referenced by `MeshTransport::connect()` and `NoiseChannel` but never defined. Is it `AsyncRead + AsyncWrite`? A trait? A concrete type?
  - `EncryptedPeer` -- used in pseudocode but no struct definition provided. Fields are inferred from usage but not specified.
  - `WeftHandshake` -- used in the dial pseudocode but no formal struct definition. Fields can be guessed from context but should be explicit.
  - `JoinRequest` / `JoinResponse` -- used in the cluster join pseudocode but no struct definitions.
  - `ChainSyncRequest` / `ChainSyncResponse` -- used in chain sync pseudocode but no struct definitions.
  - `TreeSyncRequest` / `TreeSyncResponse` -- referenced in K6.4 file table but no struct definitions.
  - `ServiceEndpoint` -- used in `ResolvedService` but never defined.
  - `ProcessAdvertisement` / `ServiceAdvertisement` -- referenced in K6.5 but no definitions.
  - `Frame` -- used in message routing pseudocode but no struct definition. The `msg_type` discriminant values are not fully enumerated.

- **Missing files**:
  - `mesh_adapter.rs` appears in K6.3 phase breakdown (line 1050) but is absent from the "Files to Create" table (lines 37-56). This is a discrepancy -- a developer would not know whether `MeshAdapter` goes in `mesh_adapter.rs` or `mesh_ipc.rs`.
  - `mesh/handshake.rs` appears in K6.4b (line 1129) using a subdirectory path (`mesh/handshake.rs`) while all other mesh files use flat naming (`mesh_*.rs`). Is there a `mesh/` subdirectory or not?

- **Incomplete definitions**:
  - `MeshTransport` trait is complete enough to implement.
  - `NoiseChannel` has fields but no method signatures. A developer needs `send()`, `recv()`, `recv_handshake()` at minimum.
  - `NodeIdentity` has fields but no methods. The pseudocode calls `identity.x25519_secret()`, `identity.node_id()`, `identity.node_id_bytes()` -- these should be listed.
  - `MeshConfig` is defined but its relationship to `ClusterConfig` is unclear. Are they separate configs? Does `MeshConfig` nest inside `ClusterConfig`?
  - The feature gate `mesh-full` depends on `mesh` and `mesh-discovery` (line 320), but the `mesh-discovery` feature depends on `mesh` (line 319). This circular structure needs clarification -- is `mesh-discovery` an additive gate on top of `mesh`?

- **Dependencies**:
  - `quinn` 0.11, `snow` 0.9, `x25519-dalek` 2.0 -- versions specified. Good.
  - `libp2p-kad` 0.46, `libp2p-mdns` 0.46 -- versions specified. Good.
  - `ruvector-dag` -- workspace dependency, version implied. Good.
  - `tokio-tungstenite` -- mentioned in the 5-layer diagram (line 31) for WebSocket transport but not listed in the dependencies table. Is it needed for K6, or deferred?
  - `webrtc-rs` -- mentioned in the 5-layer diagram but not in the dependencies table. Assumed deferred but not stated.

### Pseudocode: PASS

The pseudocode covers all five critical flows and is the most implementation-ready
section. Error paths are partially covered.

- **Missing flows**:
  - No pseudocode for `MeshAdapter` dispatching incoming messages through local `A2ARouter`. The text describes it but there is no stepwise flow.
  - No pseudocode for `mesh.request()` correlated request-response. The text says "~30 lines" but does not show the timeout/correlation logic.
  - No pseudocode for SWIM-style heartbeat / failure detection (K6.5). This is a non-trivial protocol.
  - No pseudocode for CRDT-based distributed process table gossip (K6.5).
  - No pseudocode for the tree sync Merkle diff exchange (K6.4). Only chain sync has pseudocode.
  - No pseudocode for service advertisement publishing or DHT put operations.
  - No pseudocode for connection pool management (idle eviction, reconnection with backoff).

- **Error paths missing**:
  - The `dial()` pseudocode does not handle transport-level connection failure (e.g., QUIC connection refused, DNS resolution failure).
  - The `dial()` pseudocode does not handle Noise handshake failure (invalid static key, protocol mismatch).
  - Chain sync does not handle hash mismatch at the same sequence number (fork detection). The plan mentions this in the Refinement section but the pseudocode assumes linear chains.
  - Chain sync does not handle partial delivery (peer disconnects mid-sync).
  - No error handling for `channel.send()` failures in message routing.
  - No timeout on the `peer.channel.request()` call in chain sync.
  - Discovery loops have no backoff on repeated dial failures to the same peer.

### Architecture: PASS

The 5-layer diagram is clear. The component relationship diagram shows data flow.
The integration table maps existing modules to mesh extension points.

- **Unclear integration points**:
  - `boot.rs` integration says "add mesh listener startup + peer discovery to boot sequence" but does not specify where in the boot sequence. The boot sequence has ordering constraints (service registry must exist before mesh can advertise services). A developer would need to read `boot.rs` to determine insertion point.
  - `a2a.rs` integration says "cluster-aware service resolution; remote inbox delivery bridge" but does not specify which function(s) change. Is it `A2ARouter::send()`, `A2ARouter::resolve_target()`, or a new method?
  - `service.rs` integration says "cross-node service advertisement" but does not specify how. Is there a new trait method on `SystemService`? A registration hook? A background task?
  - The CMVG cognitive sync section (Streams 3-6) is detailed but marked as K7 scope. However, the `SyncStateDigest` struct includes fields for all streams (causal, HNSW, crossref). It is unclear whether K6.4 should implement `SyncStateDigest` with only chain/tree fields populated, or the full struct.
  - The `CmvgSyncService` struct references `CausalGraph`, `HnswService`, `CrossRefStore`, `ImpulseQueue` -- none of these types exist in the current codebase. They are K3c/K7 deliverables. The plan should state explicitly that this struct is K7-only.
  - `ruvector-delta-consensus` is referenced for chain replication (line 689) but not listed in the dependencies table. Is it a workspace crate? Does it exist yet?

### Refinement: PASS (marginal)

Edge cases are listed for NAT, split brain, partition recovery, and browser nodes.
Security boundaries are clear.

- **Missing edge cases**:
  - No discussion of clock skew between nodes. HLC is mentioned (line 865) but no specification of HLC implementation or maximum allowed clock drift.
  - No discussion of what happens when a node's Ed25519 keypair is compromised. Key revocation protocol is absent.
  - No discussion of node identity collision (two nodes generating the same `SHAKE-256(pubkey)[0..16]` -- 128-bit ID has negligible collision probability but should be stated).
  - No discussion of maximum cluster size behavior. `ClusterConfig.max_nodes` exists but the plan does not specify what happens when the limit is reached and a new node tries to join.
  - No discussion of message ordering guarantees for cross-node IPC. Are messages between two nodes delivered in order? QUIC streams are ordered but the plan uses multiple streams.
  - No discussion of graceful shutdown. When a node leaves, does it announce departure? How long before peers mark it unreachable?
  - No discussion of storage requirements. Chain events accumulate without bound; what is the expected growth rate and when does pruning become necessary?
  - Rate limiting is mentioned for cluster join and governance (line 887) but no specific limits or algorithms are given.

- **Backward compatibility**: Well addressed. Feature gating is clear. The `RemoteNode` variant returning `Err(RemoteNotAvailable)` when mesh is disabled is a good pattern.

- **Security boundaries**: Mostly clear.
  - The plan does not specify the Noise protocol pattern variant string. Line 347 shows `"Noise_XX_25519_ChaChaPoly_BLAKE2b"` -- good, but the IK pattern for known peers is mentioned without the corresponding pattern string.
  - Remote capability verification is mentioned (line 886) but no protocol is described. How does a node verify a remote node's capability claims? Signed capability advertisement is stated but not specified.

### Completion: PASS (marginal)

Exit criteria are numerous (35+ items) and mostly testable. Test commands are
provided. The phase breakdown is complete with line estimates.

- **Untestable criteria**:
  - "All existing single-node tests pass unchanged" -- testable, but the command `scripts/build.sh test` does not distinguish mesh vs non-mesh tests.
  - "Remote capability claims verified against source node's signed advertisement" -- no test command or scenario described. How would a test forge a capability claim?
  - "Stopped nodes detected as Unreachable via SWIM-style heartbeats" -- testable in principle but requires a multi-node test harness that the plan does not describe.
  - "Hybrid Noise + ML-KEM-768 handshake protects against store-now-decrypt-later" -- this is a security property, not a functional test. What is the test?

- **Missing test commands**:
  - No command for running a two-node integration test. The mesh tests require at least two nodes. Is there a test harness? Docker compose? In-process test with loopback?
  - `scripts/build.sh test -- --features mesh mesh_` -- the trailing `mesh_` appears to be a test name filter but the syntax is unclear. Is this `cargo test --features mesh mesh_`? The `--` separator is ambiguous.
  - No benchmark commands for measuring handshake latency, message throughput, or connection pool performance.

- **Phase deliverables**: All phases have clear deliverables with file lists and line counts. The line count summary (1,500 new + 370 changed) is reasonable.

---

## Phase Readiness

| Phase | Ready to Implement? | Blockers |
|-------|---------------------|----------|
| K6.0 | Yes | None. All changes are additive to existing files with clear locations. |
| K6.1 | Yes (mostly) | `MeshStream` and `TransportListener` type definitions missing. Developer can infer from `MeshTransport` trait but should not have to. |
| K6.2 | Yes (mostly) | Q4 (full libp2p-kad or lighter DHT?) unresolved. Developer would need to know API surface of libp2p-kad 0.46 to implement `mesh_kad.rs`. |
| K6.3 | Needs work | `mesh_adapter.rs` vs `mesh_ipc.rs` file discrepancy. No pseudocode for `MeshAdapter` dispatch or `mesh.request()` correlation. |
| K6.4 | Blocked | Q1 (chain merge: leader-based or DAG?) and Q5 (split-brain handling) must be resolved before implementation. No pseudocode for tree Merkle diff. `ruvector-delta-consensus` dependency existence unconfirmed. |
| K6.4b | Yes | Protocol is specific, dependencies identified, files named. |
| K6.5 | Needs work | No pseudocode for SWIM heartbeat or CRDT gossip. `ProcessAdvertisement` and `ServiceAdvertisement` types undefined. |

---

## Developer Questions That Would Arise

1. **What is `MeshStream`?** Is it a trait (`AsyncRead + AsyncWrite + Send + Unpin`)? A concrete wrapper? The trait method `connect()` returns `Result<MeshStream>` but the type is never defined.

2. **Where does `MeshAdapter` live?** The "Files to Create" table does not list `mesh_adapter.rs`, but the K6.3 phase breakdown does. Which is correct?

3. **What wire format for `KernelMessage`?** Q2 asks "JSON or RVF?" and is listed as unresolved (resolve by K6.1 design). The pseudocode uses `serialize_rvf()` but the question mark means the developer cannot implement framing until this is decided.

4. **How does `ruvector-delta-consensus` integrate?** It is referenced for chain replication LWW but is not in the dependency table and may not exist as a crate yet.

5. **Does `mesh/handshake.rs` mean a directory restructure?** All other mesh files are `mesh_*.rs` (flat). K6.4b introduces `mesh/handshake.rs` (subdirectory). Which layout is canonical?

6. **What is the Noise IK pattern string?** The XX pattern is `"Noise_XX_25519_ChaChaPoly_BLAKE2b"`. What is the IK pattern for known peers? When does a peer transition from "first contact" (XX) to "known" (IK)?

7. **How do I test two nodes?** There is no test harness for multi-node integration tests. Do I use in-process loopback? Spawn two kernel instances? The test commands only show single-process `cargo test`.

8. **What happens when chain sync finds a fork?** The pseudocode assumes linear chains. The Refinement section says "events from both sides are merged into a DAG structure" but Q1 (leader-based or DAG?) is unresolved.

9. **How does `MeshConfig` relate to `ClusterConfig`?** Both contain cluster-related configuration. Is `MeshConfig` a nested field in `ClusterConfig`? A sibling in the kernel config? A separate config file?

10. **What are the `msg_type` discriminant values for framing?** The pseudocode shows `0x02` for `KernelMessage` but does not enumerate other types (handshake, chain sync, tree sync, heartbeat, etc.).

11. **How does the connection pool handle concurrent access?** `MeshConnectionPool` is mentioned (K6.1) with idle timeout and backoff, but no type definition or locking strategy is provided.

12. **What is `ServiceEndpoint`?** Used in `ResolvedService.endpoint` but never defined.

---

## Recommendations

Prioritized by impact on developer ability to implement:

1. **Define `MeshStream` and `TransportListener` types** (blocks K6.1). Add trait or struct definitions with method signatures to the Specification section.

2. **Resolve Q1 and Q5 before K6.4** (blocks K6.4). Chain merge strategy and split-brain handling are architectural decisions that affect `mesh_chain.rs` design. Document the decision inline, not as an open question.

3. **Resolve Q2 before K6.1** (blocks K6.1 framing). The wire format decision affects `mesh_framing.rs` implementation. Pick RVF (the pseudocode already assumes it) and close the question.

4. **Fix the `mesh_adapter.rs` discrepancy** (blocks K6.3). Add it to the "Files to Create" table or merge it into `mesh_ipc.rs` and update the phase breakdown.

5. **Add struct definitions for protocol messages** -- `WeftHandshake`, `JoinRequest`, `JoinResponse`, `ChainSyncRequest`, `ChainSyncResponse`, `TreeSyncRequest`, `TreeSyncResponse`, `ProcessAdvertisement`, `ServiceAdvertisement`. These are all used in pseudocode but never formally defined.

6. **Add pseudocode for tree Merkle diff** (K6.4). Chain sync has detailed pseudocode; tree sync does not. The Merkle diff walk is non-trivial.

7. **Add pseudocode for SWIM heartbeat** (K6.5). This is a well-known protocol but the plan gives no details on probe interval, indirect probe count, suspicion timeout, or protocol messages.

8. **Clarify file layout** -- flat `mesh_*.rs` vs subdirectory `mesh/`. Pick one and be consistent.

9. **Add a multi-node test harness specification**. Even a brief description of how to run two nodes in-process for integration tests would unblock test writing for K6.1 onward.

10. **Enumerate `msg_type` frame discriminants**. The framing layer needs a complete list of message types it dispatches.

11. **Confirm `ruvector-delta-consensus` crate existence** and add it to the dependency table if it exists, or mark it as "to be created" with scope.

12. **Add HLC specification**. The plan uses Hybrid Logical Clocks for event ordering but does not specify the implementation (Kulkarni et al.?) or maximum drift tolerance.
