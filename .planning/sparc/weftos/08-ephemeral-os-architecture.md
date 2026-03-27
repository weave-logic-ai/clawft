# WeftOS Kernel: Ephemeral OS Architecture -- Multi-Tenant Distributed Kernel

```
ID:          W-KERNEL-08
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Ephemeral OS Architecture -- Multi-Tenant Distributed Kernel
Status:      Proposed
Date:        2026-02-28
Depends-On:  07-ruvector-deep-integration.md, all K0-K5 phases
```

---

## 1. Vision Statement

WeftOS is not a traditional OS running on a single machine. It is an **ephemeral
distributed operating system** where:

- **The kernel is the governance** -- agents cannot modify their own rules
- **Identity follows the agent**, not the machine (exochain DIDs)
- **Any node can join** the OS fabric (cloud server, edge device, browser tab)
- **Agent roles are OS-level users** (devops, monitoring, business analytics, etc.)
- **The filesystem is a cryptographic DAG** with cloud/edge/browser storage backends
- **The same OS runs across multiple servers** with agents functioning as specialized users

This creates an OS where cloud, edge, and browser are unified into a single ephemeral
fabric. Agents migrate between nodes. Data is content-addressed. Governance is immutable.

### The Kernel-as-Governance Insight

Traditional OS kernels enforce security through hardware privilege rings (ring 0 vs
ring 3). WeftOS enforces governance through mathematical proof. The CGR engine
(from AI-SDLC) makes governance violations impossible states at the type level, not
merely audited events. Combined with exochain's cryptographic substrate (DIDs, HLC,
MMR audit trails) and ruvector's cognitive infrastructure (cognitum-gate three-way
decisions, SONA learning, delta-consensus CRDTs), this produces an OS where:

- **Agents are the Executive branch** -- they act within defined boundaries
- **The CGR engine is the Judicial branch** -- it validates every action
- **SOPs, genesis protocol YAML, and weftapp.toml rules are the Legislative branch** -- they define the boundaries
- **No branch can modify another's constraints**

---

## 2. Architecture Layers

```
+-----------------------------------------------------+
| Layer 5: Applications (weftapp.toml manifests)       |
|   Apps with lifecycle rules, governance policies     |
+-----------------------------------------------------+
| Layer 4: Agent Roles (OS "Users")                    |
|   DevOps, Monitoring, Analytics, Security, etc.      |
|   Each agent has a DID, capabilities, resource       |
|   budget. Agents ARE users on the OS.                |
+-----------------------------------------------------+
| Layer 3: Governance Kernel (AI-SDLC + CGR Engine)    |
|   Constitutional invariants (immutable)              |
|   Effect algebra (risk, fairness, privacy, etc.)     |
|   Three-branch separation (legislative/exec/judic.)  |
|   Agents CANNOT modify this layer                    |
+-----------------------------------------------------+
| Layer 2: Identity + IPC Fabric                       |
|   Exochain DIDs (agent identity, survives migration) |
|   ruvector-delta-consensus (cross-node CRDTs)        |
|   Exochain HLC (causal ordering across nodes)        |
|   ruvector-nervous-system (routing, budget guards)   |
+-----------------------------------------------------+
| Layer 1: Cryptographic Filesystem                    |
|   Exochain DAG = metadata layer (hashes, consent)    |
|   Storage backends per node type:                    |
|     Cloud: S3/GCS/Azure Blob                         |
|     Edge: Local SSD/NVMe                             |
|     Browser: IndexedDB/OPFS                          |
|     Air-gapped: RVF container segments               |
|   Gatekeeper trait enforces access policies          |
|   Bailment model: data never on ledger               |
+-----------------------------------------------------+
| Layer 0: Node Fabric                                 |
|   Any platform: cloud VM, edge, browser tab          |
|   ruvector-cluster (service discovery, health)       |
|   QuDAG libp2p (P2P networking between nodes)        |
|   rvf-kernel (125ms boot, single-file containers)    |
|   Platform traits (native/browser/WASI)              |
+-----------------------------------------------------+
```

### Layer Descriptions

**Layer 0 (Node Fabric)**: The physical substrate. Each node is a process running
the WeftOS kernel -- whether that process is a native binary on a cloud VM, a native
binary on an edge device, or a WASM module in a browser tab. ruvector-cluster handles
service discovery and health monitoring across nodes. libp2p (from exochain's QuDAG)
provides P2P networking. rvf-kernel enables 125ms boot for containerized deployments.

**Layer 1 (Cryptographic Filesystem)**: Data is content-addressed using BLAKE3 hashes.
The exochain DAG stores metadata (hashes, ownership, consent policies) but never the
data itself (bailment model). Actual data lives in storage backends appropriate to
the node type. The Gatekeeper trait (from exochain) enforces access policies before
any data is read or written.

**Layer 2 (Identity + IPC Fabric)**: Agent identity is based on exochain DIDs, which
are derived from cryptographic keys and survive node migration, restarts, and key
rotation. IPC between agents on any node uses ruvector-delta-consensus for CRDT-based
state synchronization and exochain HLC for causal ordering. Messages are addressed
by DID, not by PID or node address.

**Layer 3 (Governance Kernel)**: The immutable governance layer. Constitutional
invariants from the AI-SDLC framework are enforced by the CGR engine. Every
capability request is scored by the effect algebra (risk, fairness, privacy, novelty,
security). Agents cannot modify this layer -- violations are type errors, not runtime
exceptions.

**Layer 4 (Agent Roles)**: Agents are OS-level users with persistent identity (DID),
role-based capabilities, resource budgets (epoch controller), and governance policies.
Roles include DevOps, Monitoring, Analytics, Security, Engineering, Assistant,
Compliance, and Custom.

**Layer 5 (Applications)**: Applications are defined by weftapp.toml manifests that
specify lifecycle rules, governance policies, agent role requirements, and deployment
gates. Applications span multiple agents and potentially multiple nodes.

---

## 3. Multi-Tenancy Model

### 3.1 Node Types

| Node Type | Runtime | Storage Backend | Network | Bootstrap |
|-----------|---------|-----------------|---------|-----------|
| **Cloud Server** | Native binary (tokio) | S3/GCS + local SSD | TCP/QUIC direct | systemd service or container |
| **Edge Device** | Native binary (tokio) | Local filesystem | TCP/QUIC, possibly NAT-punched | rvf-kernel (125ms boot) |
| **Browser Tab** | WASM (wasm-bindgen) | IndexedDB/OPFS | WebSocket/WebRTC | Page load |
| **Air-gapped** | Native binary | RVF self-contained segments | None (sneakernet) | rvf-kernel |
| **Ephemeral Worker** | Native/WASM | None (stateless) | Via parent node | On-demand spawn |

### 3.2 Cluster Topology

Nodes form a cluster using ruvector-cluster with consistent hashing. Service discovery
allows any node to find any other node. Health checks detect node failures. The cluster
is self-healing -- when a node dies, its agents can be respawned on other nodes because
identity (DID) is independent of the node.

```
Cloud Region A          Cloud Region B          Edge
+----------+           +----------+           +----------+
| Node A1  |<--------->| Node B1  |<--------->| Edge E1  |
| (DevOps) |  libp2p   | (Monitor)|  libp2p   | (Sensor) |
| S3 store |           | GCS store|           | SSD store|
+----+-----+           +----+-----+           +----+-----+
     | WebSocket             |                      |
     v                       |                      |
+----------+                 |                      |
| Browser  |<----------------+                      |
| Tab B1   |  WebSocket                             |
| IDB store|<--------------------------------------+
+----------+

All nodes share:
- Exochain DAG (cryptographic filesystem metadata)
- ruvector-delta-consensus (CRDT-synced state)
- Governance kernel (identical rules everywhere)
- Agent directory (DID -> current node mapping)
```

### 3.3 Agent-as-User Model

Agents are not just processes -- they are **users on the OS** with persistent identity
and roles:

```rust
pub struct AgentUser {
    /// Persistent identity (survives node migration, key rotation).
    /// Format: "did:exo:<base58(blake3(pubkey)[0..20])>"
    pub did: Did,

    /// OS-level role (determines default capabilities)
    pub role: AgentRole,

    /// Current node assignment (changes on migration)
    pub node: Option<NodeId>,

    /// Capabilities (follow the agent, not the node)
    pub capabilities: AgentCapabilities,

    /// Resource budget (cross-node accounting via EpochController)
    pub budget: ResourceBudget,

    /// Governance policies that apply to this agent
    pub governance: Vec<GovernancePolicy>,

    /// Key material for signing/encryption
    pub key_state: KeyState,
}

pub enum AgentRole {
    /// Infrastructure management, deployment, scaling
    DevOps,
    /// System health, metrics, alerting
    Monitoring,
    /// Data analysis, reporting, insights
    Analytics,
    /// Vulnerability scanning, access auditing
    Security,
    /// Code review, testing, CI/CD
    Engineering,
    /// User-facing assistant, chat, voice
    Assistant,
    /// Governance oversight, compliance checking
    Compliance,
    /// Custom role with specified capabilities
    Custom(String),
}
```

### 3.4 Agent Migration

Agents can migrate between nodes. Because identity is DID-based (not PID-based), an
agent that migrates retains:
- Its identity (DID)
- Its capabilities (travel with the DID)
- Its reputation/trust score (stored in exochain via risk attestation)
- Its learning state (SONA patterns synced via delta-consensus)

Migration protocol:

```
1. Source node serializes agent state
   - SONA weights (MicroLoRA rank-1 + BaseLoRA rank-8)
   - In-flight messages (queued in ShardedEventBus)
   - Working context (current task, partial results from EpochController)

2. State is signed with agent's key, stored in exochain DAG
   - BLAKE3 content hash of serialized state
   - Ed25519 signature by agent's current key
   - HLC timestamp for causal ordering

3. Destination node spawns agent with DID
   - Loads state from exochain DAG
   - Verifies signature chain (key must match DID)
   - Restores SONA learning state
   - Resumes in-flight messages

4. Agent directory updated
   - DID -> new NodeId mapping via delta-consensus CRDT
   - Eventually consistent across all nodes

5. IPC messages rerouted
   - Messages addressed to DID are transparently rerouted
   - Causal ordering preserved via HLC
   - No message loss (delta-consensus handles in-flight)
```

---

## 4. Cryptographic Filesystem

### 4.1 Architecture

The "filesystem" is not a traditional filesystem. It is an exochain DAG where each
entry is a content-addressed reference to data stored in a backing store.

```rust
pub struct FileEntry {
    /// Content hash (BLAKE3, 32 bytes)
    pub content_hash: Hash,

    /// Exochain DID of the owner
    pub owner: Did,

    /// Access policy (who can read/write/delete)
    pub policy: Policy,

    /// Storage backend reference
    pub location: StorageLocation,

    /// Hybrid logical clock timestamp
    pub timestamp: HybridLogicalClock,

    /// Parent entry in the DAG (for versioning)
    pub parent: Option<Hash>,

    /// Metadata (MIME type, size, custom tags)
    pub metadata: HashMap<String, String>,
}

pub enum StorageLocation {
    /// AWS S3 bucket + key
    S3 { bucket: String, key: String, region: String },
    /// Google Cloud Storage
    Gcs { bucket: String, object: String },
    /// Azure Blob Storage
    AzureBlob { container: String, blob: String },
    /// Local filesystem path
    Local { path: PathBuf },
    /// Browser IndexedDB
    IndexedDb { database: String, store: String, key: String },
    /// Browser Origin Private File System
    Opfs { path: String },
    /// RVF container segment
    RvfSegment { container_hash: Hash, segment_id: u8 },
    /// IPFS content-addressed
    Ipfs { cid: String },
    /// Inline (small data embedded in the entry itself)
    Inline { data: Vec<u8> },
}
```

### 4.2 Access Flow

```
Agent (DID: did:exo:abc123) wants to read file with hash H

1. Agent -> Gatekeeper: request_access(did:exo:abc123, H, "read")
2. Gatekeeper -> Exochain: lookup FileEntry for hash H
3. Gatekeeper -> Policy Engine: evaluate Policy against agent's capabilities
4. Policy Engine -> CGR Engine: check governance invariants
   - Risk threshold: is this access risky? (effect.risk < 0.8)
   - Fairness floor: does access pattern maintain fairness? (effect.fairness >= 0.3)
   - Privacy check: does agent have consent for this data? (bailment contract)
5. CGR Engine -> result: Permit / Defer / Deny
6. If Permit:
   a. Gatekeeper -> StorageBackend: fetch data from location
   b. StorageBackend -> verify: BLAKE3(data) == content_hash
   c. Return data to agent
   d. Record access in witness chain (SHAKE-256 hash-linked, append-only)
7. If Defer:
   a. Escalate to supervisor/human for approval
   b. Record escalation in witness chain
   c. Agent receives a DeferToken with escalation context
8. If Deny:
   a. Return error with reason and WitnessReceipt
   b. Record denial in witness chain
   c. Agent can appeal via governance process (SOP-1304)
```

### 4.3 Bailment Model

Following exochain's bailment design:

- **PII and sensitive data NEVER touches the DAG** -- only content-addressed hashes
  and consent policies live in the exochain
- The actual data lives in the storage backend (S3, local disk, IndexedDB, etc.)
- **GDPR compliance**: delete the data from S3, the hash in the DAG becomes
  unreachable. The DAG entry persists (audit trail) but the data is gone.
- **Right to erasure**: revoke consent policy in exochain, Gatekeeper denies all
  future access. Combined with storage-side deletion, this is full erasure.
- **Audit trail**: witness chain records who accessed what, when, with what consent.
  This is a SHAKE-256 hash-linked chain (from ruvector-cognitive-container +
  rvf-crypto), making it tamper-evident.
- **Domain separators**: all signing operations use exochain's domain separator
  pattern (`b"WEFTOS-FILE-ACCESS-v1"`, `b"WEFTOS-CONSENT-GRANT-v1"`, etc.) to
  prevent cross-protocol signature confusion.

### 4.4 Cross-Node File Access

When an agent on Node A needs data stored on Node B's S3 bucket:

```
1. Agent A looks up FileEntry in exochain DAG
   - DAG is replicated to all nodes via delta-consensus
   - FileEntry.location = S3 { bucket: "node-b-data", key: "analysis/report.json" }

2. Access check via local Gatekeeper
   - Same governance rules on every node (hash-verified)
   - CGR engine evaluates effect algebra for this access

3. Data retrieval (two paths):
   a. Direct access: If Node A has S3 credentials for that bucket
      - Fetch directly from S3
      - Verify BLAKE3(data) == content_hash

   b. Proxied access: If Node A lacks credentials
      - IPC request to Node B via DID-addressed message
      - Node B's Gatekeeper checks Agent A's capabilities
      - Node B fetches data, signs the transfer with its key
      - Data sent to Node A via rvf-wire segment
      - Node A verifies BLAKE3(data) == content_hash

4. Access recorded in witness chain on both nodes
```

### 4.5 Browser Filesystem

Browser nodes use IndexedDB or OPFS as their storage backend. The same exochain DAG
metadata applies. Browser agents can:

- Store computation results in IndexedDB
- Access cloud data via WebSocket proxy through a cloud node
- Cache frequently accessed data locally (content-addressed, so cache is safe)
- The Gatekeeper runs in WASM, enforcing the same policies as native nodes
- Lightweight exochain snapshot (governance rules + relevant file refs only)
  is downloaded on connection -- not the full DAG

**Browser limitation**: Browser nodes cannot host the full exochain DAG due to storage
constraints. They receive a snapshot containing governance rules and file entries
relevant to their assigned agents. The cloud gateway node maintains the authoritative
DAG and proxies queries for entries not in the snapshot.

---

## 5. Governance as Kernel Safety

### 5.1 The Core Insight

Traditional OS security: the kernel protects itself from user-space processes via
hardware privilege rings (ring 0 vs ring 3). WeftOS governance: **the governance
layer protects itself from agents via mathematical proof** (CGR engine makes
violations type errors).

```
Traditional OS:
  Ring 0 (kernel) -- hardware-protected
  Ring 3 (userspace) -- restricted by hardware

WeftOS:
  Governance Kernel (CGR + constitutional invariants) -- mathematically protected
  Agent Space (all agents, regardless of role) -- restricted by type system
```

Agents cannot:
- Modify governance rules (constitutional invariants are immutable, compiled into kernel)
- Bypass capability checks (Gatekeeper is in the governance kernel, not agent space)
- Forge identity (DIDs are cryptographically bound to key material via BLAKE3 + Ed25519)
- Tamper with audit trails (witness chains are SHAKE-256 hash-linked, append-only)
- Exceed resource budgets (EpochController enforces fuel limits with partial results)
- Reorder messages to cause confusion (HLC provides causal ordering guarantees)

### 5.2 Three-Branch Separation (AI-SDLC Model)

| Branch | WeftOS Component | Cannot Be Modified By |
|--------|-----------------|----------------------|
| **Legislative** | weftapp.toml `[rules]`, Genesis protocol YAML, SOPs (43 documents across 5 series) | Agents (only governance admin via formal change control per SOP-1000 series) |
| **Executive** | Agent roles executing tasks within capability bounds | Itself (bounded by judicial review at every capability check) |
| **Judicial** | CGR engine + cognitum-gate + witness chain | Anyone (mathematically fixed at kernel compile time, 5 constitutional rules) |

### 5.3 Effect Algebra in Practice

Every capability request is scored on 5 dimensions (from AI-SDLC effect algebra):

```rust
pub struct EffectVector {
    /// Probability x Impact. > 0.8 is REJECT.
    pub risk: f32,
    /// Minimum floor 0.3. Below is REJECT.
    pub fairness: f32,
    /// Data protection compliance score.
    pub privacy: f32,
    /// Degree of untested territory.
    pub novelty: f32,
    /// Access control posture.
    pub security: f32,
}

impl EffectVector {
    /// Evaluate an effect vector against governance invariants.
    /// Maps to cognitum-gate three-way decisions.
    pub fn evaluate(&self) -> GovernanceDecision {
        // Constitutional invariant: risk ceiling
        if self.risk > 0.8 {
            return GovernanceDecision::Deny("Risk exceeds constitutional threshold (0.8)");
        }
        // Constitutional invariant: fairness floor
        if self.fairness < 0.3 {
            return GovernanceDecision::Deny("Fairness below constitutional floor (0.3)");
        }
        // Moderate risk: escalate (Defer maps to cognitum-gate Defer)
        if self.risk > 0.5 {
            return GovernanceDecision::Defer("Moderate risk, escalate to supervisor");
        }
        // High novelty: escalate (untested territory)
        if self.novelty > 0.7 {
            return GovernanceDecision::Defer("High novelty, require human oversight");
        }
        GovernanceDecision::Permit
    }
}
```

### 5.4 Agent Role Capabilities

| Role | Default Capabilities | Governance Level |
|------|---------------------|-----------------|
| DevOps | deploy, scale, configure, access infra secrets | High (risk > 0.6 triggers Defer) |
| Monitoring | read metrics, read logs, create alerts | Low (most actions Permit) |
| Analytics | read data stores, run queries, write reports | Medium (PII access triggers Defer) |
| Security | scan, audit, revoke access, quarantine | High (destructive actions trigger Defer) |
| Engineering | write code, run tests, create PRs | Medium (production access Deny by default) |
| Assistant | interact with users, execute approved tools | Low (tool scope limited by weftapp.toml) |
| Compliance | read audit trails, generate reports, flag issues | Read-only (cannot modify anything) |

### 5.5 Governance Lifecycle Gates

Following AI-SDLC's lifecycle gate model, apps deployed on WeftOS pass through
governance gates:

| Gate | Authority | What It Checks |
|------|-----------|---------------|
| **Install Gate** | CGR engine + app manifest | Effect algebra scores, required capabilities, resource budget |
| **Start Gate** | Governance kernel | Agent DID roster, capability assignments, governance policy binding |
| **Runtime Gate** | Continuous (per-action) | Every capability request scored by effect algebra |
| **Deployment Gate** | AI-IRB equivalent (configurable) | Cross-node deployment requires elevated governance review |
| **Retirement Gate** | Witness chain + audit | Clean shutdown, audit trail preservation, DID revocation |

---

## 6. IPC Across Nodes

### 6.1 Transport Abstraction

```rust
/// Transport trait for cross-node IPC.
/// Implementations handle the physical delivery; governance is handled
/// at the KernelIpc layer above.
pub trait IpcTransport: Send + Sync {
    async fn send(&self, target: &Did, message: &KernelMessage) -> Result<()>;
    async fn recv(&self) -> Result<KernelMessage>;
}

/// Local (same node): tokio mpsc channels (zero-copy)
pub struct LocalTransport {
    // tokio::sync::mpsc sender/receiver per agent PID
    // Fast path: no serialization, no network, no consensus overhead
}

/// Cross-node: ruvector-delta-consensus over libp2p
pub struct ClusterTransport {
    // DeltaConsensus for CRDT-based state sync
    // libp2p for P2P networking (TCP+Noise+Yamux from exochain)
    // HLC timestamps for causal ordering
    // rvf-wire segments for efficient serialization
}

/// Browser: WebSocket to gateway node
pub struct BrowserTransport {
    // WebSocket connection to nearest cloud node
    // rvf-wire serialization (same format as cluster transport)
    // Gateway node proxies to cluster transport
}
```

### 6.2 Message Routing

Messages are addressed by DID, not by node. The routing layer resolves DID to
current node:

```
1. Check local process table
   - Is this agent on my node? (PID lookup)
   - If yes: deliver via LocalTransport (mpsc, zero-copy)

2. Check agent directory (CRDT-synced across all nodes)
   - Which node has this DID?
   - Route via ClusterTransport (libp2p + delta-consensus)

3. Handle browser agents
   - Browser DIDs are marked in the agent directory
   - Route via BrowserTransport (WebSocket to browser's gateway)

4. Handle migrating agents
   - If agent is mid-migration, queue message
   - delta-consensus ensures delivery after migration completes
   - No message loss (CRDTs handle concurrent updates)

5. Causal ordering
   - Every message carries an HLC timestamp
   - Recipients process messages in causal order
   - O(1) storage per message (HLC) vs O(n) for vector clocks
```

### 6.3 Topic Pub/Sub Across Nodes

Topics are cluster-wide. A subscription on one node receives publishes from all nodes:

```rust
// DevOps agent on Node A subscribes to "deployment.events"
// Monitoring agent on Node B publishes to "deployment.events"
// Analytics agent in Browser C receives the publication

// Implementation: ruvector-nervous-system ShardedEventBus
// with delta-consensus for cross-node subscription synchronization
//
// Subscription state is a CRDT (grow-only set of DID + topic pairs)
// Publish is broadcast via delta-consensus gossip
// BudgetGuardrail prevents overloaded agents from receiving more events
```

### 6.4 Wire Format

All cross-node messages use rvf-wire segments (from 07-ruvector-deep-integration):

```
Offset  Size  Field
0       2     segment_type (u16): IPC_DIRECT=0x0100, IPC_TOPIC=0x0101
2       2     flags (u16): HAS_CORRELATION=0x01, REQUIRES_ACK=0x02
4       4     payload_length (u32)
8       32    from_did_hash (BLAKE3 of sender DID, 32 bytes)
40      32    to_did_hash (BLAKE3 of target DID or topic name, 32 bytes)
72      12    hlc_timestamp (u64 physical_ms + u32 logical)
84      16    correlation_id (u128, optional)
100     4     checksum (CRC32 of payload)
104     24    reserved (padding to 128-byte alignment)
--      var   payload (payload_length bytes)
```

Note: Cross-node IPC uses DID hashes (32 bytes) instead of PIDs (8 bytes) because
DIDs are the addressing primitive in the distributed OS. PIDs are node-local only.

---

## 7. Bootstrap and Lifecycle

### 7.1 Cluster Bootstrap

```
1. First node boots (the "genesis node")
   a. Kernel boot sequence (K0: process table, service registry, health)
   b. Initialize exochain DAG with genesis protocol
      - Constitutional invariants compiled into CGR engine
      - Genesis policies from YAML (per AI-SDLC spec)
      - Kernel DID created (the OS itself has an identity)
   c. Start ruvector-cluster discovery service
   d. Register node in cluster (self as sole member)
   e. Witness chain records genesis event

2. Additional cloud/edge nodes join
   a. Discover genesis node via DNS-SD / mDNS / static config
   b. Authenticate via libp2p Noise handshake
   c. Download exochain DAG from genesis node
      - Governance rules (constitutional invariants)
      - File metadata (content hashes, policies)
      - Agent directory (DID -> node mapping)
   d. Verify DAG integrity (MMR inclusion proofs, hash chain)
   e. Verify governance hash matches cluster
      - CRITICAL: if governance hash differs, node CANNOT join
      - Prevents split-brain governance
   f. Register in cluster via ruvector-cluster
   g. Start accepting agent assignments
   h. Begin delta-consensus synchronization

3. Browser nodes join
   a. Connect via WebSocket to a gateway node (cloud/edge)
   b. Download lightweight exochain snapshot
      - Governance rules (same on every node)
      - Agent directory (subset relevant to browser agents)
      - File metadata (subset for assigned tasks)
   c. Run Gatekeeper in WASM (same governance engine)
   d. Register browser-compatible agents (Assistant, Analytics dashboard)
   e. Cannot host heavy roles (DevOps, Security) -- resource constraints

4. Edge nodes with intermittent connectivity
   a. Same join process as cloud nodes
   b. When offline: operate with local exochain DAG snapshot
   c. delta-consensus handles eventual consistency on reconnection
   d. Governance rules are always available locally (no network needed)
   e. File access limited to local storage backend during offline
   f. Queued IPC messages delivered on reconnection (causal order preserved)
```

### 7.2 Agent Lifecycle (across nodes)

```
1. Agent Created
   a. DID generated from genesis key (BLAKE3 of Ed25519 public key)
   b. Role assigned (DevOps, Monitoring, etc.)
   c. Capabilities derived from role + custom overrides
   d. Governance policies attached (from weftapp.toml [rules])
   e. Resource budget allocated via EpochController
   f. Assigned to initial node (based on role + node capabilities)
   g. Witness chain records creation event

2. Agent Operating
   a. Executes tasks within capability bounds
   b. All actions scored by effect algebra (every capability check)
   c. All decisions recorded in witness chain (tamper-evident)
   d. Learning state updated via SONA trajectories (MicroLoRA <1ms)
   e. Communicates with any agent on any node via DID-addressed IPC
   f. Resource consumption tracked by EpochController
   g. Partial results returned if budget exhausted (graceful degradation)

3. Agent Migrating
   a. Migration triggered by:
      - Node failure (automatic, cluster detects via health check)
      - Load balancing (ruvector-nervous-system BudgetGuardrail)
      - Manual reassignment (operator command)
   b. State serialized:
      - SONA weights (learning state)
      - In-flight messages (queued in ShardedEventBus)
      - Working context (partial results, current task)
   c. State signed with agent's key, stored in exochain DAG
   d. Agent stopped on source node (ProcessState::Stopping -> Exited)
   e. Agent spawned on destination node with same DID
   f. State restored from DAG (verify signature, restore learning)
   g. Agent directory updated via delta-consensus CRDT
   h. IPC messages rerouted to new node (transparent to senders)
   i. Witness chain records migration event on both nodes

4. Agent Retiring
   a. DID key marked as revoked_at in exochain (key rotation model)
   b. Learning state archived in exochain DAG (for future agents to learn from)
   c. Audit trail preserved in witness chain (never deleted)
   d. Capabilities revoked across all nodes via delta-consensus
   e. In-flight messages drained or redirected to successor agent
   f. Resource budget reclaimed
   g. Witness chain records retirement event
```

---

## 8. Integration with Existing SPARC Phases

This ephemeral OS architecture extends (does not replace) the existing K0-K5 phases.
Each phase gains distributed capabilities while maintaining its single-machine
functionality:

| Phase | Single-Machine (Original) | Ephemeral OS (Extension) |
|-------|--------------------------|-------------------------|
| **K0** | Local kernel boot, process table, service registry | Cluster bootstrap, agent directory (DID->node), exochain genesis, governance hash verification |
| **K1** | Per-process sandbox + RBAC (binary Permit/Deny) | Per-DID capabilities (Permit/Defer/Deny), governance kernel (CGR + effect algebra), cross-node capability validation |
| **K2** | Local IPC (mpsc channels, JSON-RPC) | Cross-node IPC (delta-consensus + libp2p + rvf-wire), DID-addressed routing, HLC causal ordering, topic pub/sub across nodes |
| **K3** | Local WASM sandbox (wasmtime + rvf-wasm) | WASM tools distributed to any node, rvf containers portable across nodes, same dual runtime (micro + full) |
| **K4** | Local RVF containers (125ms boot) | RVF containers deploy to any node, cross-node service mesh, container migration with cryptographic attestation |
| **K5** | Local app framework (weftapp.toml lifecycle) | Distributed app lifecycle, cross-node deployment with governance gates, cluster-wide learning via SONA |

---

## 9. New Phase: K6 (Distributed Fabric)

A new phase after K5 that implements the ephemeral OS extensions described in this
document. K6 is subdivided into sub-phases that can be partially parallelized.

### 9.1 Sub-Phase Schedule

| Sub-phase | Title | Duration | Depends On |
|-----------|-------|----------|------------|
| K6.1 | Cluster bootstrap + agent directory | 2 weeks | K0 (process table, service registry), K1 (capabilities) |
| K6.2 | Cross-node IPC + DID routing | 2 weeks | K2 (local IPC), K6.1 (agent directory) |
| K6.3 | Cryptographic filesystem + storage backends | 2 weeks | K0 (service registry), K6.1 (exochain DAG) |
| K6.4 | Governance kernel (CGR engine integration) | 2 weeks | K1 (capability checking), K6.1 (constitutional invariants) |
| K6.5 | Agent migration + lifecycle | 1 week | K6.1 (agent directory), K6.2 (cross-node IPC) |
| K6.6 | Browser node support | 2 weeks | K6.2 (cross-node IPC), K6.3 (filesystem) |
| K6.7 | Edge node + offline sync | 1 week | K6.2 (cross-node IPC), K6.3 (filesystem) |

### 9.2 Dependency Graph

```
K0-K5 (all complete)
  |
  +---> K6.1 (Cluster bootstrap + agent directory)
  |       |
  |       +---> K6.2 (Cross-node IPC + DID routing)
  |       |       |
  |       |       +---> K6.5 (Agent migration) -- 1 week
  |       |       |
  |       |       +---> K6.6 (Browser node) -- 2 weeks [parallel w/ K6.5, K6.7]
  |       |       |
  |       |       +---> K6.7 (Edge node + offline) -- 1 week [parallel w/ K6.5, K6.6]
  |       |
  |       +---> K6.3 (Cryptographic filesystem) -- [parallel w/ K6.2]
  |       |
  |       +---> K6.4 (Governance kernel CGR) -- [parallel w/ K6.2, K6.3]
```

### 9.3 Parallelization Opportunities

- **K6.2, K6.3, K6.4** can run in parallel after K6.1 completes
- **K6.5, K6.6, K6.7** can run in parallel after K6.2 and K6.3 complete
- Total critical path: K6.1 (2w) -> K6.2 (2w) -> K6.6 (2w) = 6 weeks
- With parallelization, total wall time: ~6 weeks (vs 12 weeks sequential)

### 9.4 Sub-Phase Details

#### K6.1: Cluster Bootstrap + Agent Directory

**Goal**: Nodes can discover each other and maintain a shared agent directory.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/cluster.rs` | Create | Cluster bootstrap, node registration, health monitoring |
| `crates/clawft-kernel/src/agent_directory.rs` | Create | DID->NodeId mapping, CRDT-synced via delta-consensus |
| `crates/clawft-kernel/src/genesis.rs` | Create | Exochain genesis protocol, constitutional invariant loading |
| `crates/clawft-kernel/src/node.rs` | Create | NodeInfo, NodeType, NodeCapabilities types |
| `crates/clawft-kernel/src/boot.rs` | Modify | Add cluster join step to boot sequence |
| `crates/clawft-kernel/Cargo.toml` | Modify | Add `distributed` feature gate |

Feature gate: `distributed` (enables cluster, agent directory, genesis)

#### K6.2: Cross-Node IPC + DID Routing

**Goal**: Agents on different nodes can communicate via DID-addressed messages.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/transport.rs` | Create | IpcTransport trait, LocalTransport, ClusterTransport, BrowserTransport |
| `crates/clawft-kernel/src/router.rs` | Create | DID-based message routing, agent directory lookup |
| `crates/clawft-kernel/src/hlc.rs` | Create | HLC integration for causal ordering (wraps exo-core HLC) |
| `crates/clawft-kernel/src/ipc_adapter.rs` | Modify | Extend RuvectorIpc for cross-node delivery |
| `crates/clawft-kernel/src/wire.rs` | Modify | Add DID-hash fields to wire segment format |

Feature gate: `distributed` (same gate as K6.1)

#### K6.3: Cryptographic Filesystem + Storage Backends

**Goal**: Content-addressed file access with pluggable storage backends.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/filesystem.rs` | Create | FileEntry, StorageLocation, content-addressed access |
| `crates/clawft-kernel/src/storage/mod.rs` | Create | StorageBackend trait |
| `crates/clawft-kernel/src/storage/s3.rs` | Create | S3 storage backend |
| `crates/clawft-kernel/src/storage/local.rs` | Create | Local filesystem backend |
| `crates/clawft-kernel/src/storage/indexeddb.rs` | Create | Browser IndexedDB backend (WASM only) |
| `crates/clawft-kernel/src/storage/rvf.rs` | Create | RVF container segment backend |
| `crates/clawft-kernel/src/gatekeeper.rs` | Create | Gatekeeper trait impl (wraps exo-consent) |

Feature gate: `distributed-fs` (depends on `distributed`)

#### K6.4: Governance Kernel (CGR Engine)

**Goal**: CGR engine enforces constitutional invariants at the kernel level.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/governance.rs` | Create | GovernanceKernel, EffectVector, GovernanceDecision |
| `crates/clawft-kernel/src/cgr.rs` | Create | CGR engine integration (wraps AI-SDLC AEGIS concepts) |
| `crates/clawft-kernel/src/constitutional.rs` | Create | Constitutional invariants as Rust types |
| `crates/clawft-kernel/src/capability_adapter.rs` | Modify | Wire CGR decisions into capability checking |

Feature gate: `governance` (can be used independently of `distributed`)

#### K6.5: Agent Migration + Lifecycle

**Goal**: Agents can migrate between nodes with full state preservation.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/migration.rs` | Create | Migration protocol, state serialization, DID-keyed restore |
| `crates/clawft-kernel/src/agent_user.rs` | Create | AgentUser struct, AgentRole enum |
| `crates/clawft-kernel/src/supervisor.rs` | Modify | Add migration triggers to supervisor |
| `crates/clawft-kernel/src/agent_directory.rs` | Modify | Add migration state tracking |

Feature gate: `distributed` (same gate)

#### K6.6: Browser Node Support

**Goal**: Browser tabs can join the cluster and host lightweight agents.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/browser_node.rs` | Create | Browser node bootstrap, lightweight DAG snapshot |
| `crates/clawft-kernel/src/transport.rs` | Modify | BrowserTransport implementation |
| `crates/clawft-wasm/src/cluster.rs` | Create | WASM-side cluster participation |
| `crates/clawft-wasm/src/gatekeeper_wasm.rs` | Create | Gatekeeper running in browser WASM |

Feature gate: `browser-distributed` (requires `browser` + `distributed`)

#### K6.7: Edge Node + Offline Sync

**Goal**: Edge nodes work offline and sync on reconnection.

Files to create/modify:

| File | Action | Description |
|------|--------|-------------|
| `crates/clawft-kernel/src/offline.rs` | Create | Offline mode, local DAG snapshot, queue management |
| `crates/clawft-kernel/src/sync.rs` | Create | Reconnection sync protocol, conflict resolution |

Feature gate: `distributed` (same gate)

---

## 10. Key Types Summary

```rust
// -- Node identity and management --

pub struct NodeInfo {
    pub node_id: NodeId,
    pub node_type: NodeType,
    pub endpoints: Vec<Endpoint>,
    pub storage_backend: StorageBackendType,
    pub capabilities: NodeCapabilities,
    pub health: HealthStatus,
}

pub enum NodeType {
    Cloud,
    Edge,
    Browser,
    AirGapped,
    EphemeralWorker,
}

pub struct NodeCapabilities {
    /// Roles this node can host
    pub supported_roles: Vec<AgentRole>,
    /// Storage capacity in bytes
    pub storage_bytes: u64,
    /// Whether this node can act as a gateway for browser nodes
    pub is_gateway: bool,
    /// Whether this node has persistent storage
    pub has_persistent_storage: bool,
}

// -- Agent directory (cluster-wide, CRDT-synced) --

pub struct AgentDirectory {
    /// DID -> directory entry, synced via delta-consensus LWW register
    pub entries: HashMap<Did, AgentDirectoryEntry>,
}

pub struct AgentDirectoryEntry {
    pub did: Did,
    pub role: AgentRole,
    pub current_node: Option<NodeId>,
    pub state: AgentState,
    pub last_seen: HybridLogicalClock,
    /// Coherence score from prime-radiant SheafLaplacian
    pub coherence_score: f64,
}

pub enum AgentState {
    Starting,
    Running,
    Migrating { from: NodeId, to: NodeId },
    Suspended,
    Retired,
}

// -- Cluster membership --

pub struct ClusterState {
    pub nodes: HashMap<NodeId, NodeInfo>,
    pub agent_directory: AgentDirectory,
    /// Governance state hash -- MUST match across all nodes
    pub governance_hash: Hash,
    /// Latest exochain DAG head
    pub exochain_head: Hash,
    /// Cluster-wide witness chain head
    pub witness_head: Hash,
}
```

---

## 11. New Cargo Dependencies

### For the `distributed` feature gate

| Dependency | Source | Purpose | Phase |
|------------|--------|---------|-------|
| `exo-core` | git (exochain) | BLAKE3, Ed25519, HLC, domain separators | K6.1 |
| `exo-dag` | git (exochain) | DAG engine, MMR accumulator, SMT state | K6.1, K6.3 |
| `exo-identity` | git (exochain) | DID derivation, key rotation, risk attestation | K6.1 |
| `exo-consent` | git (exochain) | Bailment contracts, policies, Gatekeeper trait | K6.3 |
| `exo-api` | git (exochain) | libp2p transport (TCP+Noise+Yamux) | K6.2 |

All exochain dependencies are feature-gated behind `distributed`. They are native-only
(not available in WASM builds). The `browser-distributed` feature provides WASM-compatible
alternatives for browser nodes (WebSocket transport, IndexedDB storage, lightweight DAG).

### Integration with existing ruvector dependencies

The `distributed` feature builds on top of existing ruvector feature gates from
07-ruvector-deep-integration:

```toml
# crates/clawft-kernel/Cargo.toml additions

[features]
# Distributed OS fabric
distributed = [
    "ruvector-cluster",     # existing: service discovery, health
    "ruvector-ipc",         # existing: delta-consensus, event bus
    "ruvector-wire",        # existing: binary wire format
    "ruvector-crypto",      # existing: witness chain
    "dep:exo-core",         # new: BLAKE3, HLC, domain separators
    "dep:exo-dag",          # new: DAG engine, MMR
    "dep:exo-identity",     # new: DIDs, key rotation
]

distributed-fs = [
    "distributed",
    "dep:exo-consent",      # new: bailment, policies, Gatekeeper
]

governance = [
    "ruvector-supervisor",  # existing: cognitum-gate, cognitive-container
    "ruvector-apps",        # existing: sona, prime-radiant
]

distributed-full = [
    "distributed",
    "distributed-fs",
    "governance",
    "ruvector-containers",  # existing: RVF containers
]

browser-distributed = [
    "distributed",
    # Browser-compatible subset (no exo-api libp2p, use WebSocket)
]
```

---

## 12. CLI Commands (K6)

```
weft cluster status       -- cluster membership, node count, governance hash
weft cluster nodes        -- list all nodes with type, storage, health
weft cluster join <addr>  -- manually join a cluster by address
weft cluster leave        -- gracefully leave the cluster

weft agent directory      -- list all agents across all nodes (DID, role, node)
weft agent migrate <did> <node>  -- migrate agent to a different node
weft agent locate <did>   -- find which node hosts an agent

weft fs ls <path>         -- list files in cryptographic filesystem
weft fs cat <hash>        -- read file by content hash
weft fs put <file>        -- store file, return content hash
weft fs policy <hash>     -- show access policy for a file
weft fs audit <hash>      -- show witness chain entries for a file

weft governance status    -- governance hash, constitutional invariants
weft governance verify    -- verify governance rules match cluster
weft governance audit     -- show governance decision witness chain
```

---

## 13. Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Network partition splits cluster | High | Delta-consensus CRDTs handle eventual consistency; governance rules identical on all nodes (hash-verified); no split-brain for governance decisions because governance is immutable and identical everywhere |
| Browser node has limited resources | Medium | Lightweight exochain snapshot; proxy heavy operations through cloud gateway nodes; browser agents limited to Assistant/Analytics dashboard roles; NodeCapabilities.supported_roles enforces this |
| Cross-node IPC latency too high for real-time | Medium | Local fast-path for same-node IPC (mpsc, zero-copy); async messaging for cross-node; co-locate communicating agents via SONA routing learning; BudgetGuardrail prevents overload |
| Agent migration state too large to serialize | Medium | Incremental state sync via delta-consensus; SONA patterns compressed (MicroLoRA rank-1 is tiny); large data stays in filesystem (only content-hash refs migrate, not data) |
| Governance rules out of sync across nodes | Critical | Exochain DAG with hash verification at join time; node CANNOT participate if governance hash does not match cluster; constitutional invariants are immutable (compiled into CGR engine) |
| S3/GCS credentials management across nodes | High | Per-node credential vaults (never in exochain); Gatekeeper proxies access through authorized nodes; no credential sharing; access recorded in witness chain |
| DID key compromise | Critical | Key rotation via exochain key versioning (valid_from/revoked_at); compromised key revoked across all nodes via delta-consensus; new key issued; agent DID preserved (derived from genesis key, not current key) |
| Exochain DAG grows unbounded | Medium | MMR compaction at checkpoint intervals; old DAG entries pruned (data in storage backends, not DAG); configurable retention policy |
| libp2p NAT traversal for edge nodes | Medium | QUIC with NAT hole-punching; relay nodes for difficult NAT situations; fallback to WebSocket via cloud gateway |
| Browser tab closed mid-operation | Low | Browser agents are ephemeral by design; in-flight work queued at gateway node; agent state checkpointed periodically to IndexedDB |
| Performance overhead of per-action governance scoring | Medium | Effect algebra is O(1) arithmetic (5 float comparisons); CGR engine pre-compiles constitutional rules; cache recent decisions per DID+resource pair |
| Complexity of the full stack (exochain + ruvector + AI-SDLC) | High | Incremental adoption via feature gates; `distributed` can be enabled without `governance`; each K6 sub-phase is independently testable; comprehensive adapter layer isolates complexity |

---

## 14. Testing Contracts

### Feature Flag Test Matrix (K6)

| Build Configuration | Command | Must Pass |
|---|---|---|
| No distributed (default) | `scripts/build.sh test` | All existing K0-K5 tests |
| distributed only | `cargo test -p clawft-kernel --features distributed` | Cluster bootstrap, agent directory, cross-node IPC |
| distributed-fs | `cargo test -p clawft-kernel --features distributed-fs` | Filesystem, storage backends, Gatekeeper |
| governance only | `cargo test -p clawft-kernel --features governance` | CGR engine, effect algebra, constitutional invariants |
| distributed-full | `cargo test -p clawft-kernel --features distributed-full` | All K6 tests |
| WASM (browser) | `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser` | Must NOT pull exochain or ruvector deps |
| browser-distributed | `cargo check -p clawft-wasm --target wasm32-unknown-unknown --features browser-distributed` | Browser-compatible distributed subset |

### Integration Tests (K6)

| Test | Sub-phase | Description |
|------|-----------|-------------|
| `cluster_bootstrap_two_nodes` | K6.1 | Two nodes discover each other, share governance hash, sync agent directory |
| `agent_directory_crdt_sync` | K6.1 | Agent created on node A appears in directory on node B via delta-consensus |
| `cross_node_ipc_did_routing` | K6.2 | Agent on node A sends message to agent on node B by DID, message delivered |
| `hlc_causal_ordering` | K6.2 | Messages delivered in causal order under concurrent cross-node load |
| `filesystem_s3_access` | K6.3 | Agent reads file via exochain DAG metadata, data fetched from S3, BLAKE3 verified |
| `gatekeeper_deny_unauthorized` | K6.3 | Agent without read capability denied file access, witness chain records denial |
| `cgr_risk_threshold` | K6.4 | Capability request with risk > 0.8 denied by CGR engine |
| `cgr_fairness_floor` | K6.4 | Capability request with fairness < 0.3 denied by CGR engine |
| `effect_algebra_defer` | K6.4 | Capability request with risk 0.5-0.8 deferred to supervisor |
| `agent_migration_state_preserved` | K6.5 | Agent migrates from node A to B, SONA learning state preserved, IPC rerouted |
| `browser_node_joins_cluster` | K6.6 | Browser tab connects via WebSocket, receives governance snapshot, hosts agent |
| `edge_offline_sync` | K6.7 | Edge node operates offline, reconnects, delta-consensus syncs state |
| `governance_hash_mismatch_rejected` | K6.1 | Node with different governance hash cannot join cluster |

---

## 15. Agent Spawning Execution Plan (K6)

### Team Structure

| Agent | Type | Sub-phases | Responsibility |
|-------|------|------------|----------------|
| `cluster-lead` | `coder` | K6.1, K6.5 | Cluster bootstrap, agent directory, migration protocol |
| `ipc-agent` | `coder` | K6.2 | Cross-node IPC, DID routing, HLC integration |
| `filesystem-agent` | `coder` | K6.3 | Cryptographic filesystem, storage backends, Gatekeeper |
| `governance-agent` | `coder` | K6.4 | CGR engine, effect algebra, constitutional invariants |
| `browser-agent` | `coder` | K6.6 | Browser node support, WASM Gatekeeper, WebSocket transport |
| `edge-agent` | `coder` | K6.7 | Edge node support, offline mode, sync protocol |
| `test-agent` | `tester` | K6.1-K6.7 | Integration tests, feature flag matrix verification |
| `review-agent` | `reviewer` | K6.1-K6.7 | Code review, security audit, governance compliance |

### Execution Order

1. **Week 1-2**: `cluster-lead` executes K6.1 (cluster bootstrap + agent directory)
2. **Week 3-4**: In parallel:
   - `ipc-agent` executes K6.2 (cross-node IPC)
   - `filesystem-agent` executes K6.3 (cryptographic filesystem)
   - `governance-agent` executes K6.4 (CGR engine)
3. **Week 5**: In parallel:
   - `cluster-lead` executes K6.5 (agent migration)
   - `browser-agent` starts K6.6 (browser node -- 2 weeks)
   - `edge-agent` executes K6.7 (edge node + offline)
4. **Week 6**: `browser-agent` completes K6.6; integration testing across all sub-phases
5. `test-agent` and `review-agent` run continuously after each sub-phase

---

## 16. Quality Gates Between Sub-Phases

### K6.1 Gate (Cluster + Directory)
- [ ] Two native nodes can discover each other and form a cluster
- [ ] Agent directory CRDT syncs agent DID -> node mapping
- [ ] Governance hash verified at join time; mismatched node rejected
- [ ] Exochain DAG initialized with genesis protocol
- [ ] Witness chain records cluster events
- [ ] `weft cluster status` CLI command works

### K6.2 Gate (Cross-Node IPC)
- [ ] Message sent to DID delivered to correct node via libp2p
- [ ] HLC timestamps provide causal ordering
- [ ] rvf-wire segments used for cross-node messages
- [ ] Topic pub/sub works across nodes
- [ ] BudgetGuardrail prevents overloaded agent from receiving more messages
- [ ] `weft agent locate <did>` CLI command works

### K6.3 Gate (Filesystem)
- [ ] FileEntry stored in exochain DAG with content hash
- [ ] Data fetched from S3 backend, BLAKE3 verified
- [ ] Gatekeeper denies unauthorized access, records in witness chain
- [ ] Bailment model: data deleted from S3, hash in DAG becomes unreachable
- [ ] `weft fs ls`, `weft fs cat`, `weft fs put` CLI commands work

### K6.4 Gate (Governance)
- [ ] CGR engine rejects capability request with risk > 0.8
- [ ] CGR engine rejects capability request with fairness < 0.3
- [ ] Moderate risk (0.5-0.8) produces Defer decision
- [ ] Effect algebra scores every capability request on 5 dimensions
- [ ] Constitutional invariants cannot be modified at runtime
- [ ] `weft governance status` CLI command works

### K6.5 Gate (Migration)
- [ ] Agent migrates from node A to node B
- [ ] DID, capabilities, SONA learning state preserved
- [ ] IPC messages rerouted to new node transparently
- [ ] Witness chain records migration on both nodes
- [ ] `weft agent migrate <did> <node>` CLI command works

### K6.6 Gate (Browser)
- [ ] Browser tab connects to cluster via WebSocket
- [ ] Lightweight exochain snapshot downloaded and verified
- [ ] Gatekeeper runs in WASM with same governance rules
- [ ] Browser agent can communicate with cloud agents via DID-addressed IPC
- [ ] Browser agent restricted to supported_roles

### K6.7 Gate (Edge + Offline)
- [ ] Edge node operates with local DAG snapshot when offline
- [ ] Queued IPC messages delivered on reconnection
- [ ] Delta-consensus resolves conflicts on reconnection
- [ ] File access works against local storage backend during offline
- [ ] Governance rules available locally (no network dependency)

---

## 17. Definition of Done

### Architecture (this document)

- [x] Vision statement articulates ephemeral distributed OS model
- [x] Architecture layers defined with clear boundaries
- [x] Multi-tenancy model covers all node types (cloud, edge, browser, air-gapped)
- [x] Agent-as-user model with DID identity and role-based capabilities
- [x] Agent migration protocol specified
- [x] Cryptographic filesystem with bailment model
- [x] Cross-node file access flow defined
- [x] Governance-as-kernel-safety model with three-branch separation
- [x] Effect algebra integrated with CGR engine
- [x] IPC transport abstraction covers local, cluster, and browser
- [x] DID-addressed message routing specified
- [x] Cluster bootstrap protocol for all node types
- [x] Agent lifecycle (create, operate, migrate, retire) documented
- [x] Integration with existing K0-K5 phases mapped
- [x] K6 sub-phases defined with dependencies and schedule
- [x] Key types specified as Rust code
- [x] New Cargo dependencies listed with feature gates
- [x] CLI commands for K6 defined
- [x] Risk assessment with mitigations
- [x] Testing contracts with feature flag matrix
- [x] Agent spawning execution plan
- [x] Quality gates for each sub-phase

### Implementation (future K6 phases)

- [ ] Cluster bootstrap works with 2+ nodes
- [ ] Agent with DID can migrate between nodes and retain state
- [ ] Cross-node IPC delivers messages with causal ordering (HLC)
- [ ] Cryptographic filesystem reads data from S3 via exochain metadata
- [ ] Gatekeeper enforces access policies on file reads
- [ ] Browser node joins cluster via WebSocket and hosts agents
- [ ] Governance rules are identical on all nodes (hash verification)
- [ ] Witness chain records all cross-node operations
- [ ] Edge node works in offline mode and syncs on reconnection
- [ ] Effect algebra scoring works on capability requests
- [ ] CGR engine enforces constitutional invariants (risk ceiling, fairness floor)
- [ ] No exochain or ruvector type appears in clawft-kernel public API (adapter layer)
- [ ] All existing K0-K5 tests pass with `distributed` feature disabled
- [ ] WASM browser build unaffected by distributed feature
- [ ] Feature flag test matrix passes for all configurations
