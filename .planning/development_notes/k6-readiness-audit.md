# K6 Networking Readiness Audit

**Date**: 2026-03-25
**Scope**: WeftOS kernel (K0-K5) audit for K6 networking layer readiness
**Branch**: `feature/weftos-kernel-sprint`

---

## 1. Cluster Module (`cluster.rs`)

### GREEN -- Ready

- **ClusterMembership** supports dynamic peer join/leave via `add_peer()`, `remove_peer()`, and state transitions through `update_state()`. The `DashMap` backing store is already concurrent-safe.
- **NodeState** has a full lifecycle: `Joining -> Active -> Suspect -> Unreachable -> Leaving -> Left`. This covers the failure detection semantics K6 needs.
- **PeerNode** has all essential fields for mesh networking: `id`, `name`, `platform`, `state`, `address: Option<String>`, `first_seen`, `last_heartbeat`, `capabilities: Vec<String>`, `labels: HashMap<String, String>`.
- **NodePlatform** covers all relevant targets: `CloudNative`, `Edge`, `Browser`, `Wasi`, `Custom(String)`.
- **ClusterConfig** has heartbeat interval, suspect/unreachable thresholds, and max node count -- all configurable.
- **ClusterService** (behind `cluster` feature) wraps `ruvector_cluster::ClusterManager` and syncs discovered nodes into `ClusterMembership`. The `sync_to_membership()` method handles bidirectional state mapping.
- **NodeEccCapability** (behind `ecc` feature) carries cognitive tick metrics, HNSW stats, and spectral analysis capability -- sufficient for mesh-aware workload routing.
- All types derive `Serialize`/`Deserialize`, enabling wire transport.
- Existing test coverage includes serde roundtrips, peer lifecycle, and error cases.

### YELLOW -- Minor Changes Needed

- **`PeerNode.address`** is `Option<String>`. K6 should add a structured address type (or at least a parsed `SocketAddr` alternative) to avoid string-parsing bugs in the hot path.
- **`ClusterConfig`** lacks a `bind_address` / `listen_port` field for the local node. K6 will need this for the networking listener.
- **`ClusterConfig`** lacks `seed_peers: Vec<String>` for bootstrap discovery. Currently relies on ruvector's `StaticDiscovery`, which takes an empty list by default.
- **`ClusterService::sync_to_membership()`** is pull-based (called manually). K6 needs a background tick loop to drive periodic sync.
- **`cluster_node_to_peer()`** hardcodes `NodePlatform::CloudNative` for all ruvector nodes. Should read platform from node metadata.

### RED -- Missing

- **No gossip or membership protocol** beyond ruvector's `ClusterManager`. K6 needs a protocol for browser/WASI nodes that cannot use ruvector's TCP-based discovery.
- **No cluster-join authentication**. `add_peer()` accepts any `PeerNode` with no signature verification or shared-secret check. A malicious node can join by calling `add_peer()` with fabricated data.
- **No split-brain detection** or partition handling logic.

---

## 2. IPC Readiness for Remote Messaging (`ipc.rs`, `a2a.rs`)

### GREEN -- Ready

- **KernelMessage** derives `Serialize`/`Deserialize` and roundtrips cleanly through JSON. All fields are wire-safe: `id: String`, `from: Pid`, `target: MessageTarget`, `payload: MessagePayload`, `timestamp: DateTime<Utc>`, `correlation_id: Option<String>`.
- **MessagePayload** covers all needed variants: `Text`, `Json`, `ToolCall`, `ToolResult`, `Signal`, and `Rvf` (binary segment transport). The `Rvf` variant with `segment_type: u8` + `data: Vec<u8>` is directly suitable for cross-node binary exchange.
- **MessageTarget::Service** and **MessageTarget::ServiceMethod** are already extensible patterns. Adding `MessageTarget::RemoteNode { node_id, inner: Box<MessageTarget> }` is a natural extension.
- **A2ARouter** has clean separation: capability checking, inbox management, topic routing, and service routing are all distinct layers. The `send()` method dispatches on `MessageTarget` variants -- adding a `RemoteNode` arm is straightforward.
- **Correlation IDs** are fully supported for request-response patterns across nodes.
- **Dual-layer governance** (routing-time gate + handler-time gate) is in place via `GateBackend` trait. This will correctly gate remote messages.

### YELLOW -- Minor Changes Needed

- **`MessageTarget`** needs a `RemoteNode` variant: `RemoteNode { node_id: String, target: Box<MessageTarget> }`. Without this, there is no way to address a remote inbox.
- **`KernelIpc::send()`** publishes all messages to the local `MessageBus` as `InboundMessage` on the "kernel-ipc" channel. K6 must add a transport fork: if the target is remote, serialize and send over the network transport instead of the local bus.
- **`A2ARouter`** resolves services only through the local `ServiceRegistry`. K6 needs a cluster-aware service resolution path that can query peer registries.
- **`Pid` is a `u64`** -- locally unique but not globally unique. K6 needs either a `(NodeId, Pid)` composite identifier or a globally unique PID allocation scheme.
- **Inbox channel (`mpsc`)** is in-process only. Remote delivery requires a network transport bridge that serializes messages, sends them to the target node, and enqueues into the remote inbox.

### RED -- Missing

- **No network transport layer**. There is no TCP/WebSocket/QUIC listener or client. This is the primary K6 deliverable.
- **No message framing protocol** for the wire. Messages are JSON-serialized but there is no length-prefix framing or protocol negotiation.
- **No message deduplication**. If a message is retransmitted after a network hiccup, the inbox will receive duplicates.

---

## 3. Chain Replication Readiness (`chain.rs`)

### GREEN -- Ready

- **ChainEvent** is fully serializable (derives `Serialize`/`Deserialize`) with all fields: `sequence`, `chain_id`, `timestamp`, `prev_hash`, `hash`, `payload_hash`, `source`, `kind`, `payload`.
- **Hash scheme** uses SHAKE-256 with a well-defined canonical form (`compute_event_hash`) that commits to all fields including payload. This is suitable for cross-node verification -- any node can independently recompute and verify.
- **Integrity verification** (`verify_integrity()`) walks all events checking prev_hash linkage, payload_hash, and event hash. This works identically on a remote copy.
- **RVF segment persistence** is implemented: `save_to_rvf()` writes chain events as signed RVF segments with ExoChainHeader + CBOR payload, and `load_from_rvf()` reads them back with validation.
- **Ed25519 signing** of RVF segments is supported via `with_signing_key()`. ML-DSA-65 post-quantum dual signing is also available.
- **Witness chain** (`WitnessEntry`, `create_witness_chain`, `verify_witness_chain`) provides a secondary audit trail suitable for cross-node attestation.
- **Lineage records** (`record_lineage()`, `verify_lineage()`) provide provenance tracking that could anchor cross-node chain relationships.
- **Scorecard aggregation** (`aggregate_scorecard()`) summarizes chain metrics -- useful for remote health/audit queries.

### YELLOW -- Minor Changes Needed

- **`chain_id` is `u32`** -- sufficient for node-local chains, but K6 should define a convention for globally unique chain IDs (e.g., hash of `node_id + chain_purpose`).
- **`LocalChain`** stores all events in a `Vec<ChainEvent>` in memory. For long-running nodes, K6 will need event pruning or a streaming/paged export API for replication.
- **`save_to_file()` / `load_from_file()`** use newline-delimited JSON. For cross-node transfer, the RVF segment format (`save_to_rvf` / `load_from_rvf`) should be the canonical wire format.

### RED -- Missing

- **No chain merge or conflict resolution**. If two nodes independently extend a chain (fork), there is no protocol to reconcile them. K6 needs either a consensus protocol for chain extension or a DAG-based structure.
- **No cross-node chain anchoring**. There is no mechanism to embed a remote chain's head hash into the local chain (anchor/bridge events).
- **No incremental replication**. `save_to_rvf()` exports the full chain. K6 needs a delta-sync API: "give me events after sequence N".
- **No chain subscription / push notification**. Remote nodes cannot subscribe to chain events from a peer.

---

## 4. Governance for Multi-Node (`governance.rs`, `gate.rs`)

### GREEN -- Ready

- **GovernanceRule** is serializable and carries `id`, `description`, `branch`, `severity`, `active`, `reference_url`, and `sop_category`. These can be distributed to peer nodes as-is.
- **GovernanceEngine** evaluates requests against rules using the effect algebra (5-dimensional `EffectVector` with L2 norm magnitude). The evaluation logic is pure and deterministic -- given the same rules and request, any node produces the same decision.
- **GovernanceDecision** maps cleanly to RVF `PolicyCheck` variants (`Allowed`, `Confirmed`, `Denied`). The RVF bridge (`to_rvf_policy_check()`, `to_rvf_mode()`, `to_rvf_policy()`) is already implemented.
- **GateBackend trait** is extensible: `CapabilityGate` for binary decisions, `TileZeroGate` for three-way decisions with cryptographic receipts. K6 can add a `RemoteGate` implementation that delegates to a remote governance authority.
- **GateDecision** includes an optional `token: Vec<u8>` (permit) and `receipt: Vec<u8>` (deny) -- these can carry cryptographic attestations for cross-node verification.

### YELLOW -- Minor Changes Needed

- **GovernanceEngine** stores rules in a local `Vec<GovernanceRule>`. K6 needs a mechanism to synchronize rules across cluster nodes -- either a rules-distribution protocol or a shared governance chain.
- **`GovernanceRequest.agent_id`** is a `String` but does not include a `node_id` field. K6 governance decisions should reference the originating node.
- **The genesis chain event** (`chain.genesis`) is per-node. A cluster-wide governance root (a "constitutional genesis") that all nodes share is not yet defined.

### RED -- Missing

- **No distributed governance consensus**. If two nodes have different rule sets, governance decisions will diverge. K6 needs either a leader-based rule authority or a consensus mechanism for rule changes.
- **No cross-node escalation**. `GovernanceDecision::EscalateToHuman` is local-only. In a cluster, escalation may need to route to a designated governance node.

---

## 5. Security Posture

### GREEN -- Ready

- **Capability-based access control** is enforced at every IPC boundary: `CapabilityChecker` validates tool access, IPC scope, and service access before message delivery.
- **IpcScope** provides fine-grained control: `All`, `ParentOnly`, `Restricted(Vec<Pid>)`, `Topic(Vec<String>)`, `None`.
- **ResourceLimits** enforce memory, CPU time, tool call count, and message count budgets per agent.
- **Dual-layer gate** (routing-time + handler-time) means a malicious message must pass two independent checks.
- **Chain integrity** is cryptographically enforced with SHAKE-256 hashes and optional Ed25519 + ML-DSA-65 signatures.
- **RVF segment validation** (`validate_segment`, `verify_segment`) checks segment structure before parsing payloads.

### YELLOW -- Minor Changes Needed

- **Deserialization of `KernelMessage`** from the wire should use `serde_json::from_str` with size limits. Currently there is no maximum message size check, which could allow a memory exhaustion attack.
- **`PeerNode.capabilities`** is `Vec<String>` -- there is no validation that capability strings are from a known set. A malicious node could advertise arbitrary capabilities.
- **`ClusterConfig.max_nodes`** protects against unbounded cluster growth, but there is no rate-limiting on `add_peer()` calls.

### RED -- Missing

- **No TLS/mTLS for cluster communication**. All transport is assumed in-process. K6 must enforce encrypted channels and mutual authentication for all inter-node traffic.
- **No cluster-join authentication**. There is no shared secret, certificate, or challenge-response protocol for verifying that a joining node is authorized.
- **No message authentication on the wire**. `KernelMessage` is serialized as plain JSON. K6 must add per-message signatures or use an authenticated transport (TLS).
- **Capability checks are node-local only**. `CapabilityChecker` uses the local `ProcessTable`. If a remote node sends a message claiming `from: pid-5`, there is no way to verify that PID 5 on the remote node actually has the claimed capabilities.
- **No DoS protection for governance**. A compromised or malicious node could flood the governance engine with evaluation requests.

---

## 6. Resource Tree for Multi-Node (`tree_manager.rs`, `exo-resource-tree`)

### GREEN -- Ready

- **ResourceTree** has a Merkle hash root (`root_hash()`) computed via `recompute_all()`. This 32-byte hash is a compact representation of the full tree state, suitable for cross-node consistency checks.
- **TreeManager** atomically couples tree mutations to chain events, producing a verifiable audit trail. Every insert, remove, and metadata update generates both a `MutationEvent` and a `ChainEvent`.
- **`chain_seq` metadata** on each tree node links back to the chain event that caused the mutation, enabling two-way traceability.
- **`TreeStats`** provides a serializable snapshot (node count, mutation count, root hash) that can be exchanged between nodes for quick consistency checks.
- **`/network/peers/`** namespace is already bootstrapped in the tree, ready for peer node entries.
- **`add_peer_with_tree()`** (behind `exochain` feature) creates a tree node for each cluster peer under `/network/peers/{name}`.

### YELLOW -- Minor Changes Needed

- **`ResourceTree` uses `Mutex`** (non-async). K6's async networking layer will need to avoid holding the tree lock across await points. Consider `tokio::sync::RwLock` or a message-passing pattern.
- **`recompute_all()`** recalculates the entire Merkle tree on every mutation. For large trees, K6 should use incremental hash updates (dirty-flag propagation up the tree).
- **`MutationEvent.signature`** is `Option<Vec<u8>>` but always set to `None`. K6 should sign mutations with the node's Ed25519 key for cross-node verification.

### RED -- Missing

- **No tree state sync protocol**. There is no mechanism to transfer a tree snapshot to a new node or to synchronize incremental changes.
- **No conflict resolution for concurrent tree mutations** from different nodes. If two nodes insert under the same parent simultaneously, there is no merge strategy.
- **No tree diff API**. K6 needs a way to compute `TreeDiff` between two root hashes and transmit only the changed nodes.
- **No Merkle proof generation**. While `root_hash()` exists, there is no function to generate a Merkle inclusion proof for a specific node -- needed for lightweight cross-node verification without transferring the full tree.

---

## Summary Matrix

| Subsystem | GREEN (Ready) | YELLOW (Minor) | RED (Missing) |
|-----------|:---:|:---:|:---:|
| Cluster   | 10  | 5   | 3   |
| IPC/A2A   | 6   | 5   | 3   |
| Chain     | 8   | 3   | 4   |
| Governance| 5   | 3   | 2   |
| Security  | 6   | 3   | 5   |
| Tree      | 6   | 3   | 4   |
| **Total** | **41** | **22** | **21** |

---

## Recommended Architecture for K6 Networking

### Transport Layer (new crate: `clawft-net` or module in kernel)

```
clawft-net/
  transport/
    tcp.rs          -- TCP + TLS listener/connector (native nodes)
    websocket.rs    -- WebSocket transport (browser/WASI nodes)
    framing.rs      -- Length-prefix framing + protocol version negotiation
  auth/
    tls.rs          -- mTLS certificate management
    challenge.rs    -- Challenge-response for cluster join
  protocol/
    handshake.rs    -- Node introduction protocol (exchange NodeId, capabilities, chain head)
    heartbeat.rs    -- Gossip-style failure detection
    membership.rs   -- SWIM-style membership protocol overlay
```

### Message Routing Extensions

1. Add `MessageTarget::RemoteNode { node_id: NodeId, target: Box<MessageTarget> }` to `ipc.rs`.
2. Add `GlobalPid { node_id: NodeId, pid: Pid }` composite identifier.
3. In `A2ARouter::send()`, intercept `RemoteNode` targets and hand off to the network transport instead of local inbox delivery.

### Chain Replication

1. Add `ChainManager::tail_from(sequence: u64) -> Vec<ChainEvent>` for incremental replication.
2. Define `BridgeEvent` chain event kind that anchors a remote chain's head hash into the local chain.
3. Implement pull-based replication: new nodes request events from sequence 0 (or last known checkpoint) from a peer.
4. Consider a leader-based chain extension for shared chains (governance chain, cluster membership chain).

### Tree Synchronization

1. Add `TreeManager::snapshot() -> TreeSnapshot` (serializable tree state).
2. Add `TreeManager::apply_remote_mutation(event: MutationEvent, signature: Vec<u8>)` with signature verification.
3. Implement Merkle proof generation for lightweight verification.

### Security Requirements

1. mTLS for all inter-node TCP connections.
2. WebSocket connections authenticated via signed challenge-response.
3. Per-message HMAC or use authenticated encryption at the transport level.
4. `add_peer()` gated behind a signed `JoinRequest` verified against a cluster CA or shared secret.
5. Maximum message size enforcement on deserialization.
6. Rate limiting on cluster join requests and governance evaluation requests.
7. Remote capability claims verified against the source node's signed capability advertisement.

### Phasing Suggestion

| Phase | Scope | Deliverables |
|-------|-------|-------------|
| K6.0  | Transport foundation | TCP+TLS listener, framing, handshake, `RemoteNode` target |
| K6.1  | Membership           | SWIM-style gossip, authenticated join, split-brain detection |
| K6.2  | Remote IPC           | Cross-node message routing, GlobalPid, service resolution |
| K6.3  | Chain replication     | Incremental sync, bridge events, chain subscription |
| K6.4  | Tree sync             | Snapshot transfer, incremental mutation sync, Merkle proofs |
| K6.5  | Distributed governance| Shared governance chain, cross-node escalation, rule distribution |
