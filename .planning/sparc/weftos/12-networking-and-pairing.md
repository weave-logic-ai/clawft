# WeftOS Kernel: Networking, Pairing, and Network Joining

```
ID:          W-KERNEL-12
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Networking, Pairing, and Network Joining
Status:      Proposed
Date:        2026-02-28
Depends-On:  08-ephemeral-os-architecture.md, 10-agent-first-single-user.md, 07-ruvector-deep-integration.md
```

---

## 1. Core Insight: Networks, Not Clusters

WeftOS networking is not about managing a fixed cluster. It is about nodes discovering
each other, establishing trust, exchanging capabilities, and forming dynamic fabrics.
The network model draws from DeFi protocols rather than traditional cluster management:

- **Permissionless**: Any node can attempt to join. Trust is earned, not granted.
  There is no central registry that must approve membership.

- **Self-organizing**: Nodes find each other through discovery protocols (mDNS for
  LAN, Kademlia DHT for WAN, bootstrap nodes for initial entry). No human operator
  is required to wire nodes together.

- **Capability-based**: Nodes advertise what they can do. Other nodes find them by
  capability, not by address. An agent that needs inference finds a node offering
  inference, regardless of where that node lives.

- **Layered trust**: Plain TCP for gossip and discovery. Encrypted channels for
  agent IPC. DeFi-style bonds for high-trust operations like model weight sync or
  governance participation. Not everything needs encryption -- but when it does, the
  stack provides post-quantum crypto.

### Previous Model (Doc 08: Cluster-Centric)

```
Node joins cluster --> cluster manager registers --> health checks maintain membership
```

### New Model (Network-Centric)

```
Node discovers peers --> pairing handshake (DID-based) --> trust earned over time --> bonds for high-value operations
```

The cluster model from doc 08 is a special case of this: a fully trusted, bonded
network where all nodes share governance hashes. This document generalizes that to
support partial trust, capability exchange, and permissionless discovery.

### Agents Working for Other Agents Across the Network

A key architectural direction: agents on remote nodes can work for agents on local
nodes. An OpenClaw-style tool agent, a Claude agent wrapper, or a Codex agent that
carries no model but can be reached over the network -- all of these are first-class
participants. From the local agent's perspective, calling a remote tool agent is
identical to calling a local one. The network service handles the routing.

---

## 2. Transport Layers

```
+----------------------------------------------------------+
| Layer 4: Agent IPC Protocol                              |
|   JSON-RPC / rvf-wire messages between agents            |
|   Addressed by DID or service name, not IP:port          |
+----------------------------------------------------------+
| Layer 3: Secure Channel (when needed)                    |
|   Noise protocol (libp2p default) or TLS 1.3             |
|   Post-quantum: ML-KEM-768 key exchange (QuDAG crypto)   |
|   Used for: agent IPC, model sync, capability exchange   |
+----------------------------------------------------------+
| Layer 2: Peer Discovery                                  |
|   mDNS (LAN), Kademlia DHT (WAN), Bootstrap nodes       |
|   Gossip protocol for capability advertisement           |
|   Used for: finding peers, service discovery             |
+----------------------------------------------------------+
| Layer 1: Base Transport                                  |
|   TCP/IP (default), QUIC (multiplexed), WebSocket (browser) |
|   Plain, unencrypted for discovery/gossip                |
|   Low overhead, standard networking                      |
+----------------------------------------------------------+
```

### Key Design Decisions

1. **TCP/IP is the default.** No complex protocol required for basic communication.
   Standard networking, standard ports, standard tooling. A `telnet` to the discovery
   port should show gossip traffic. This is intentional -- the base layer is simple.

2. **Encryption is opt-in per connection.** Discovery gossip is plain TCP. Agent IPC
   upgrades to Noise or TLS when the connection transitions from discovery to
   operational traffic. High-trust operations (model weight sync, governance
   participation) add post-quantum crypto via ML-KEM-768 from the QuDAG stack.

3. **Browser nodes use WebSocket/WebRTC.** This maps cleanly to the browser platform
   trait from doc 08. Browser nodes cannot listen on TCP (no server sockets in
   browsers), so they connect outbound via WebSocket to a cloud/edge relay node, or
   establish WebRTC peer connections for direct browser-to-browser communication.

4. **QUIC for multiplexed connections.** When a node maintains many concurrent peer
   connections (e.g., a cloud hub serving dozens of edge nodes), QUIC provides
   multiplexed streams over a single UDP socket with built-in congestion control.
   This is optional -- TCP remains the default for simplicity.

### Transport Types

```rust
/// Base transport protocol for peer connections.
pub enum TransportProtocol {
    /// Standard TCP/IP -- default, simplest, most compatible.
    Tcp,
    /// QUIC -- multiplexed streams, better NAT traversal, built-in TLS.
    Quic,
    /// WebSocket -- for browser nodes connecting to cloud/edge relays.
    WebSocket,
    /// WebRTC -- for direct browser-to-browser peer connections.
    WebRtc,
}

/// Encryption layer applied on top of base transport.
pub enum ChannelEncryption {
    /// No encryption. Used for discovery gossip and capability advertisement.
    /// Discovery traffic contains no sensitive data -- only DIDs, capabilities,
    /// and addresses, all of which are public.
    None,
    /// Noise protocol (libp2p default). Used for agent IPC between paired nodes.
    /// XX handshake pattern: mutual authentication with DID-derived static keys.
    Noise,
    /// TLS 1.3 with standard certificates. Used when interoperating with
    /// non-WeftOS systems that expect TLS.
    Tls13,
    /// Post-quantum encryption via ML-KEM-768 key encapsulation + ML-DSA
    /// signatures (from QuDAG crypto stack). Used for high-trust operations:
    /// model weight sync, governance participation, bond management.
    PostQuantum,
}

/// A fully configured transport channel between two nodes.
pub struct TransportChannel {
    /// Base transport protocol.
    pub protocol: TransportProtocol,
    /// Encryption layer (None for discovery, Noise for IPC, PostQuantum for bonds).
    pub encryption: ChannelEncryption,
    /// Remote peer's multiaddr (libp2p addressing format).
    pub remote_addr: Multiaddr,
    /// Local peer's DID.
    pub local_did: Did,
    /// Remote peer's DID (known after pairing handshake).
    pub remote_did: Option<Did>,
    /// Connection state.
    pub state: ChannelState,
    /// Throughput and latency metrics for this channel.
    pub metrics: ChannelMetrics,
}

pub enum ChannelState {
    /// Connection established, no pairing yet. Discovery only.
    Connected,
    /// Pairing handshake in progress.
    Pairing,
    /// Paired -- DID verified, IPC allowed.
    Paired,
    /// Bonded -- high-trust channel with post-quantum crypto.
    Bonded,
    /// Disconnected -- channel closed or timed out.
    Disconnected,
}

pub struct ChannelMetrics {
    /// Bytes sent on this channel.
    pub bytes_sent: u64,
    /// Bytes received on this channel.
    pub bytes_received: u64,
    /// Rolling average latency in milliseconds.
    pub avg_latency_ms: f64,
    /// Number of messages exchanged.
    pub message_count: u64,
    /// When this channel was established.
    pub established_at: DateTime<Utc>,
    /// Last activity timestamp.
    pub last_active: DateTime<Utc>,
}
```

---

## 3. Network Joining (DeFi-Inspired)

### 3.1 Discovery Phase

How a new node finds the network. Multiple discovery methods are supported
simultaneously -- a node uses whichever method succeeds first, then gossips its
presence to discovered peers.

```rust
/// Network discovery subsystem. Runs all configured discovery methods in parallel.
pub struct NetworkDiscovery {
    /// mDNS for local network discovery (zero-config LAN).
    /// Broadcasts presence on the local network segment.
    /// Works behind NATs, no internet required.
    pub mdns: Option<MdnsDiscovery>,

    /// Kademlia DHT for wide-area discovery.
    /// Content-addressed routing table -- nodes are found by their DID hash
    /// position in the DHT keyspace.
    pub dht: Option<KademliaDht>,

    /// Bootstrap nodes (well-known entry points, like DeFi RPC endpoints).
    /// These are stable addresses that new nodes can always connect to.
    /// They serve as DHT entry points and gossip relays.
    pub bootstrap_nodes: Vec<Multiaddr>,

    /// Gossip protocol for capability advertisement.
    /// Once connected to at least one peer, gossip spreads capability
    /// advertisements through the network.
    pub gossip: GossipProtocol,

    /// Discovery event callback channel.
    pub discovered_peers: mpsc::Sender<DiscoveredPeer>,
}

/// How a node discovers the network. Multiple methods can be active simultaneously.
pub enum DiscoveryMethod {
    /// Zero-config local network (mDNS). No configuration needed.
    /// Finds all WeftOS nodes on the same LAN segment.
    Local,

    /// Bootstrap node list (like DeFi RPC endpoints).
    /// These are stable, well-known addresses that serve as entry points.
    Bootstrap(Vec<Multiaddr>),

    /// Direct peer address (manual pairing).
    /// Used when the operator knows the exact address of a peer.
    Direct(Multiaddr),

    /// DHT lookup by capability.
    /// Find peers that advertise a specific capability (e.g., "inference",
    /// "storage", "relay") without knowing their address.
    CapabilitySearch(Vec<String>),
}

/// A peer discovered through any discovery method.
pub struct DiscoveredPeer {
    /// How this peer was discovered.
    pub method: DiscoveryMethod,
    /// Peer's advertised addresses (may have multiple).
    pub addresses: Vec<Multiaddr>,
    /// Peer's DID (if advertised during discovery).
    pub did: Option<Did>,
    /// Capabilities advertised by this peer (from gossip).
    pub capabilities: Vec<NodeCapability>,
    /// When this peer was discovered.
    pub discovered_at: DateTime<Utc>,
}
```

### 3.2 Pairing Handshake

Once a node finds a peer, they must pair before exchanging agent IPC. The handshake
is a 5-step protocol that establishes identity, verifies capability claims, and
optionally creates a trust bond.

```
Node A (joiner)                           Node B (existing)
     |                                         |
     |--- 1. HELLO (my DID, my capabilities) ->|
     |                                         |
     |<-- 2. CHALLENGE (nonce, your DID req) --|
     |                                         |
     |--- 3. PROVE (signed nonce + caps) ----->|
     |                                         |
     |<-- 4. ACCEPT (peer's caps, network map) |
     |                                         |
     |--- 5. BOND (optional: stake/reputation) |
     |                                         |
     [  Paired -- can now exchange IPC msgs  ]
```

**Step 1 (HELLO)**: The joiner announces its DID and the capabilities it offers.
This is sent over the plain TCP connection established during discovery. No
encryption yet -- the HELLO is public information.

**Step 2 (CHALLENGE)**: The existing node sends a random nonce and requests proof
that the joiner controls the DID's private key. This prevents DID spoofing.

**Step 3 (PROVE)**: The joiner signs the nonce with its DID key (Ed25519) and
re-sends its capabilities. The signature proves identity without revealing the
private key.

**Step 4 (ACCEPT)**: The existing node verifies the signature, accepts the pairing,
and shares its own capabilities plus a partial network map (known peers, their
capabilities, their trust levels). The connection upgrades to Noise encryption.

**Step 5 (BOND, optional)**: For operations requiring guarantees (model weight
exchange, governance participation), the joiner posts a bond. The bond is a signed
commitment of resources with slashing conditions.

```rust
/// The complete pairing handshake state machine.
pub struct PairingHandshake {
    /// Current step in the handshake.
    pub state: HandshakeState,
    /// Step 1: Joiner announces identity and capabilities.
    pub hello: Option<HelloMessage>,
    /// Step 2: Existing node challenges with nonce.
    pub challenge: Option<ChallengeMessage>,
    /// Step 3: Joiner proves identity by signing nonce.
    pub proof: Option<ProofMessage>,
    /// Step 4: Existing node accepts and shares network state.
    pub accept: Option<AcceptMessage>,
    /// Step 5: Optional bond for high-trust operations.
    pub bond: Option<BondMessage>,
    /// Timeout for the entire handshake (default: 30s).
    pub timeout: Duration,
    /// When the handshake started.
    pub started_at: DateTime<Utc>,
}

pub enum HandshakeState {
    /// Waiting to send or receive HELLO.
    Init,
    /// HELLO sent, waiting for CHALLENGE.
    HelloSent,
    /// CHALLENGE received, preparing PROVE.
    Challenged,
    /// PROVE sent, waiting for ACCEPT.
    ProveSent,
    /// ACCEPT received -- pairing complete.
    Accepted,
    /// BOND sent -- waiting for bond confirmation.
    BondPending,
    /// Fully paired and optionally bonded.
    Complete,
    /// Handshake failed (timeout, invalid signature, rejected).
    Failed(HandshakeError),
}

pub struct HelloMessage {
    /// Node's DID (exochain identity).
    pub did: Did,
    /// Protocol version (allows graceful protocol evolution).
    pub protocol_version: u32,
    /// Capabilities this node offers to the network.
    pub capabilities: Vec<NodeCapability>,
    /// Capabilities this node is seeking from peers.
    pub seeking: Vec<String>,
    /// Transport addresses this node can be reached at (multiaddr format).
    pub addresses: Vec<Multiaddr>,
    /// Node type (cloud, edge, browser, air-gapped).
    pub node_type: NodeType,
    /// Optional: human-readable node name for debugging.
    pub node_name: Option<String>,
}

pub struct ChallengeMessage {
    /// Random nonce (32 bytes). The joiner must sign this.
    pub nonce: [u8; 32],
    /// Challenger's DID (so the joiner knows who is challenging).
    pub challenger_did: Did,
    /// Minimum protocol version the challenger supports.
    pub min_protocol_version: u32,
    /// Timestamp for replay protection.
    pub timestamp: DateTime<Utc>,
}

pub struct ProofMessage {
    /// The nonce from the challenge, echoed back.
    pub nonce: [u8; 32],
    /// Ed25519 signature of: BLAKE3(nonce || challenger_did || timestamp).
    /// Domain separator: b"WEFTOS-PAIRING-PROOF-v1"
    pub signature: [u8; 64],
    /// The joiner's public key (used to verify signature against DID).
    pub public_key: [u8; 32],
    /// Capabilities (re-sent, now cryptographically bound to the DID).
    pub capabilities: Vec<NodeCapability>,
}

pub struct AcceptMessage {
    /// The accepter's capabilities.
    pub capabilities: Vec<NodeCapability>,
    /// Partial network map: known peers and their capabilities.
    /// This helps the joiner bootstrap its view of the network.
    pub known_peers: Vec<PeerSummary>,
    /// The trust level assigned to the joiner.
    pub initial_trust: TrustLevel,
    /// Signed by the accepter's DID key.
    pub signature: [u8; 64],
}

pub struct BondMessage {
    /// Bond details (amount, scope, duration, slash conditions).
    pub bond: Bond,
    /// Signed by the bonder's DID key.
    pub signature: [u8; 64],
}

/// Summary of a known peer, shared during ACCEPT.
pub struct PeerSummary {
    /// Peer's DID.
    pub did: Did,
    /// Peer's known addresses.
    pub addresses: Vec<Multiaddr>,
    /// Peer's advertised capabilities.
    pub capabilities: Vec<NodeCapability>,
    /// Trust level the accepter has with this peer.
    pub trust_level: TrustLevel,
    /// Last time the accepter communicated with this peer.
    pub last_seen: DateTime<Utc>,
}

/// Errors that can occur during the pairing handshake.
pub enum HandshakeError {
    /// Handshake timed out before completion.
    Timeout,
    /// Joiner's DID signature did not verify.
    InvalidSignature,
    /// Protocol version mismatch.
    ProtocolMismatch { local: u32, remote: u32 },
    /// Peer explicitly rejected the pairing.
    Rejected { reason: String },
    /// Network error during handshake.
    TransportError(String),
    /// Peer is already paired (duplicate pairing attempt).
    AlreadyPaired,
}
```

### 3.3 Node Capabilities

Capabilities are what nodes advertise and what other nodes search for. The
capability system is the foundation of the network's self-organizing property.

```rust
/// A capability that a node offers to the network.
pub enum NodeCapability {
    /// This node runs agents with specific roles (doc 10).
    AgentRoles(Vec<AgentRole>),

    /// This node provides inference (with model info, doc 11).
    Inference {
        models: Vec<String>,
        tiers: Vec<ModelTier>,
        max_concurrent: u32,
    },

    /// This node has storage (type + capacity).
    Storage {
        backend: StorageBackend,
        capacity_mb: u64,
        available_mb: u64,
    },

    /// This node provides specific tools (OpenClaw-style tool agents).
    Tools(Vec<ToolCapability>),

    /// This node can relay messages to other nodes (NAT traversal helper).
    Relay {
        max_relay_connections: u32,
        supported_protocols: Vec<TransportProtocol>,
    },

    /// This node acts as a bootstrap/discovery hub.
    Bootstrap,

    /// This node wraps an external LLM API (Claude, Codex, etc.)
    /// and exposes it as an inference service over the network.
    ExternalLlm {
        provider: String,
        models: Vec<String>,
        rate_limit_rpm: Option<u32>,
    },

    /// This node provides compute resources for training (doc 11).
    TrainingCompute {
        gpu_available: bool,
        memory_mb: u64,
    },

    /// Custom capability with arbitrary metadata.
    Custom {
        name: String,
        version: String,
        metadata: serde_json::Value,
    },
}

/// A tool capability advertisement.
pub struct ToolCapability {
    /// Tool name (e.g., "diff-analyzer", "code-formatter").
    pub name: String,
    /// Tool version.
    pub version: String,
    /// Input schema (JSON Schema for tool arguments).
    pub input_schema: Option<serde_json::Value>,
    /// Whether this tool runs in a WASM sandbox.
    pub sandboxed: bool,
    /// Average execution time in milliseconds.
    pub avg_latency_ms: Option<f64>,
}
```

### 3.4 Trust Levels

Like DeFi protocols, trust is layered and earned. A new peer starts at `Unknown`
and progresses through levels based on successful interactions, not based on who
they claim to be.

```rust
/// Trust level between two peers. Determines what operations are allowed
/// and what transport encryption is used.
pub enum TrustLevel {
    /// Just discovered, no history. Can gossip and discover.
    /// Transport: plain TCP.
    /// Operations: discovery gossip, capability queries.
    Unknown,

    /// Completed pairing handshake. DID verified. Can exchange basic IPC.
    /// Transport: Noise-encrypted channel.
    /// Operations: agent IPC, tool invocation, basic data exchange.
    Paired,

    /// Established reputation through successful interactions.
    /// Transport: Noise + signed messages.
    /// Operations: all Paired operations + higher rate limits, priority routing.
    Trusted {
        /// Reputation score (0.0-1.0). Earned through successful interactions.
        reputation_score: f64,
        /// Number of successful interactions with this peer.
        interactions: u64,
        /// When this trust level was first established.
        trusted_since: DateTime<Utc>,
    },

    /// Bonded with stake/collateral. For high-value operations.
    /// Transport: post-quantum encrypted (ML-KEM-768).
    /// Operations: model weight sync, governance participation, financial ops.
    Bonded {
        /// Bond amount in internal resource units.
        bond_amount: u64,
        /// When the bond expires (must be renewed).
        bond_expiry: DateTime<Utc>,
        /// Bond scope (what operations the bond covers).
        bond_scope: BondScope,
    },
}

impl TrustLevel {
    /// Minimum encryption required for this trust level.
    pub fn required_encryption(&self) -> ChannelEncryption {
        match self {
            TrustLevel::Unknown => ChannelEncryption::None,
            TrustLevel::Paired => ChannelEncryption::Noise,
            TrustLevel::Trusted { .. } => ChannelEncryption::Noise,
            TrustLevel::Bonded { .. } => ChannelEncryption::PostQuantum,
        }
    }

    /// Numeric ordering for comparison (Unknown < Paired < Trusted < Bonded).
    pub fn level(&self) -> u8 {
        match self {
            TrustLevel::Unknown => 0,
            TrustLevel::Paired => 1,
            TrustLevel::Trusted { .. } => 2,
            TrustLevel::Bonded { .. } => 3,
        }
    }
}
```

**Trust progression**: Unknown -> Paired -> Trusted -> Bonded

**Trust degradation**: Trust can decrease. A peer that serves invalid data, goes
offline for extended periods, or violates protocol rules loses reputation. Bonds
can be slashed. A bonded peer that violates slash conditions drops to Trusted
(or lower).

```rust
/// Trust state machine transitions.
pub struct TrustEngine {
    /// Trust state per peer DID.
    peer_trust: DashMap<Did, TrustState>,
    /// Configuration for trust progression and degradation.
    config: TrustConfig,
}

pub struct TrustState {
    /// Current trust level.
    pub level: TrustLevel,
    /// Interaction history (last N interactions).
    pub history: Vec<InteractionRecord>,
    /// When trust was last evaluated.
    pub last_evaluated: DateTime<Utc>,
    /// Violations recorded against this peer.
    pub violations: Vec<Violation>,
}

pub struct InteractionRecord {
    /// What operation was performed.
    pub operation: String,
    /// Whether it succeeded.
    pub success: bool,
    /// Latency of the operation.
    pub latency_ms: f64,
    /// When the interaction occurred.
    pub timestamp: DateTime<Utc>,
}

pub struct Violation {
    /// Type of violation.
    pub kind: ViolationType,
    /// When the violation occurred.
    pub timestamp: DateTime<Utc>,
    /// Evidence (e.g., hash of invalid data, expected vs actual).
    pub evidence: String,
}

pub enum ViolationType {
    /// Peer served data that did not match its content hash.
    InvalidData,
    /// Peer was unreachable for longer than allowed.
    ExtendedOffline { duration: Duration },
    /// Peer violated a governance rule.
    GovernanceViolation { rule: String },
    /// Peer failed to relay a message it committed to relay.
    RelayFailure,
    /// Peer sent malformed protocol messages.
    ProtocolViolation { detail: String },
}

pub struct TrustConfig {
    /// Number of successful interactions to progress from Paired to Trusted.
    pub interactions_for_trust: u64,
    /// Minimum reputation score to maintain Trusted status.
    pub min_reputation: f64,
    /// How quickly reputation decays without interaction (days).
    pub reputation_decay_days: u32,
    /// Number of violations before trust is downgraded.
    pub max_violations_before_downgrade: u32,
    /// Whether to auto-ban peers that fall below Unknown.
    pub auto_ban_on_zero_trust: bool,
}
```

### 3.5 Bond Mechanism (DeFi-Style)

For operations that need economic guarantees. Bonds are not real money -- they are
internal resource units that represent a node's commitment to good behavior. The
mechanism is modeled after DeFi staking: post a bond, fulfill obligations, get the
bond back. Misbehave, and the bond is slashed.

```rust
/// A bond posted by a node for high-trust operations.
pub struct Bond {
    /// Unique bond identifier.
    pub id: String,
    /// Who posted the bond.
    pub node_did: Did,
    /// Bond amount (in internal resource units, not real money).
    pub amount: u64,
    /// What the bond covers.
    pub scope: BondScope,
    /// When the bond was posted.
    pub created_at: DateTime<Utc>,
    /// When the bond expires (must be renewed to maintain bonded status).
    pub expires_at: DateTime<Utc>,
    /// Conditions that trigger slashing.
    pub slash_conditions: Vec<SlashCondition>,
    /// Current status of the bond.
    pub status: BondStatus,
    /// Signed by the bonding node's DID key.
    /// Domain separator: b"WEFTOS-BOND-v1"
    pub signature: [u8; 64],
}

/// What operations a bond covers.
pub enum BondScope {
    /// Bond covers all operations with this peer.
    General,
    /// Bond covers a specific capability.
    Capability(String),
    /// Bond covers model weight sync (prevents model poisoning).
    ModelSync,
    /// Bond covers governance participation (voting, proposals).
    Governance,
    /// Bond covers relay services (guarantees message delivery).
    Relay,
}

/// Conditions that trigger bond slashing.
pub enum SlashCondition {
    /// Node goes offline for longer than the specified duration.
    Offline { max_duration: Duration },
    /// Node serves data that does not match its content hash.
    /// Verified by witness chain evidence from reporting peers.
    InvalidData,
    /// Node violates governance rules (verified by CGR engine, doc 08).
    GovernanceViolation,
    /// Node fails to relay messages it committed to relay.
    RelayFailure { max_failures: u32 },
    /// Node's latency exceeds committed SLA.
    LatencySlaViolation { max_p99_ms: u64 },
}

/// Current status of a bond.
pub enum BondStatus {
    /// Bond is active and protecting operations.
    Active,
    /// Bond has been partially slashed.
    PartiallySlashed { remaining: u64, reason: String },
    /// Bond has been fully slashed.
    FullySlashed { reason: String },
    /// Bond has expired (must be renewed).
    Expired,
    /// Bond was voluntarily withdrawn (node left bonded status).
    Withdrawn,
}

/// Bond manager tracks all bonds between the local node and its peers.
pub struct BondManager {
    /// Bonds posted by this node.
    pub outbound_bonds: DashMap<Did, Bond>,
    /// Bonds posted by peers with this node.
    pub inbound_bonds: DashMap<Did, Bond>,
    /// Bond slashing engine.
    pub slasher: BondSlasher,
    /// Configuration.
    pub config: BondConfig,
}

pub struct BondConfig {
    /// Default bond amount for general bonds.
    pub default_amount: u64,
    /// Default bond duration before expiry.
    pub default_duration: Duration,
    /// Percentage of bond slashed per violation (0-100).
    pub slash_percentage: u8,
    /// Whether to require bonds for model weight sync.
    pub require_bond_for_model_sync: bool,
    /// Whether to require bonds for governance participation.
    pub require_bond_for_governance: bool,
}
```

---

## 4. Network Service Agent

The network service is a kernel service agent (doc 10) that manages all P2P
networking: peer discovery, pairing handshakes, trust management, capability
advertisement, and cross-node message relay. Like all WeftOS services, it is
defined by an agent manifest.

```toml
# .agents/network-service.agent.toml

[agent]
name = "network-service"
version = "0.1.0"
description = "P2P networking, peer discovery, pairing, and cross-node relay"
role = "service"

[capabilities]
tools = [
    "peer_discover",
    "peer_pair",
    "peer_list",
    "peer_trust",
    "peer_bond",
    "network_status",
    "capability_search",
    "capability_advertise",
]
ipc_scope = "topic"
topics_publish = [
    "network.peer_joined",
    "network.peer_left",
    "network.peer_paired",
    "network.capability_update",
    "network.trust_changed",
    "network.bond_event",
]
topics_subscribe = [
    "network.discover",
    "network.pair",
    "network.bond",
]
can_spawn = false
filesystem_access = ["data/network/"]

[resources]
max_memory_mb = 128
max_concurrent_requests = 200
priority = "high"

[interface]
protocol = "ipc"
request_topic = "network.request"
response_mode = "direct"

[health]
check_interval = "15s"
timeout = "5s"
restart_policy = "always"
max_restarts = 10

[dependencies]
requires = ["message-bus"]
after = ["kernel-init"]
```

### Network Service Responsibilities

1. **Peer discovery**: Runs mDNS, DHT, and bootstrap discovery in parallel. Reports
   discovered peers via `network.peer_joined` topic.

2. **Pairing handshake**: Manages the 5-step pairing protocol. Verifies DID
   signatures, exchanges capabilities, upgrades connection encryption.

3. **Trust management**: Tracks trust state per peer. Records interaction outcomes.
   Applies reputation decay. Triggers trust level transitions.

4. **Bond management**: Accepts and posts bonds. Monitors slash conditions.
   Executes slashing when conditions are met.

5. **Capability advertisement**: Gossips local capabilities to peers. Maintains
   a capability index for incoming queries from other agents.

6. **Cross-node relay**: Routes IPC messages to remote peers. Integrates with
   the local message bus (doc 10, PID 1) for transparent DID-based routing.

---

## 5. Cross-Node Agent Communication

How agents on different nodes talk to each other. From an agent's perspective,
local and remote IPC are identical -- the network service handles all transport
details transparently.

```
Node A                          Node B
+--------------+                +--------------+
| Agent X      |                | Agent Y      |
| (PID 8)      |                | (PID 3)      |
+------+-------+                +------+-------+
       | IPC: send to                  |
       | did:weft:agent-y              |
       v                              |
+--------------+                       |
| Message Bus  | -- "DID not local" -- |
| (PID 1)      |                       |
+------+-------+                       |
       | route to                      |
       | network-service               |
       v                               |
+--------------+    TCP/Noise    +--------------+
| Network Svc  |<=============>| Network Svc  |
| (PID 6)      |    encrypted   | (PID 6)      |
+--------------+    channel     +------+-------+
                                       |
                                +--------------+
                                | Message Bus  |
                                | (PID 1)      |
                                +------+-------+
                                       | deliver to
                                       | Agent Y
                                       v
                                +--------------+
                                | Agent Y      |
                                | (PID 3)      |
                                +--------------+
```

### Routing Logic

```rust
/// Cross-node message routing integrated with the local message bus.
pub struct NetworkRouter {
    /// Local process table for PID-to-DID resolution.
    process_table: Arc<ProcessTable>,
    /// Peer connection map (DID -> active transport channel).
    peer_channels: DashMap<Did, TransportChannel>,
    /// Capability index (capability -> list of peers offering it).
    capability_index: DashMap<String, Vec<Did>>,
    /// Trust engine for checking trust levels before routing.
    trust_engine: Arc<TrustEngine>,
}

impl NetworkRouter {
    /// Route a message addressed by DID.
    /// Returns Ok if delivered locally or sent to the correct remote peer.
    pub async fn route_by_did(
        &self,
        from_pid: Pid,
        target_did: &Did,
        message: KernelMessage,
    ) -> Result<DeliveryStatus> {
        // 1. Check local process table -- is this agent on our node?
        if let Some(entry) = self.process_table.find_by_did(target_did) {
            // Local delivery via message bus (fast path, no network)
            return self.deliver_local(entry.pid, message).await;
        }

        // 2. Check peer channels -- do we have an active connection to the
        //    node hosting this agent?
        if let Some(channel) = self.peer_channels.get(target_did) {
            // Check trust level
            let trust = self.trust_engine.get_trust(target_did);
            let required_encryption = trust.required_encryption();

            if channel.encryption < required_encryption {
                return Err(NetworkError::InsufficientEncryption {
                    have: channel.encryption.clone(),
                    need: required_encryption,
                });
            }

            // Send via established channel
            return self.send_remote(&channel, message).await;
        }

        // 3. Check capability index -- is this DID known in the network?
        //    Route via relay if needed.
        if let Some(relay_peer) = self.find_relay_for(target_did) {
            return self.send_via_relay(&relay_peer, target_did, message).await;
        }

        Err(NetworkError::PeerNotFound(target_did.clone()))
    }

    /// Route a message by capability (find any peer with the capability).
    pub async fn route_by_capability(
        &self,
        capability: &str,
        min_trust: TrustLevel,
        prefer_local: bool,
        message: KernelMessage,
    ) -> Result<DeliveryStatus> {
        // 1. If prefer_local, check local agents first
        if prefer_local {
            if let Some(entry) = self.process_table.find_by_capability(capability) {
                return self.deliver_local(entry.pid, message).await;
            }
        }

        // 2. Find peers with this capability
        let peers = self.capability_index.get(capability)
            .map(|p| p.clone())
            .unwrap_or_default();

        // 3. Filter by trust level and sort by latency
        let eligible: Vec<_> = peers.iter()
            .filter(|did| self.trust_engine.get_trust(did).level() >= min_trust.level())
            .filter_map(|did| {
                self.peer_channels.get(did).map(|ch| (did.clone(), ch.metrics.avg_latency_ms))
            })
            .collect();

        if eligible.is_empty() {
            return Err(NetworkError::NoCapabilityMatch(capability.to_string()));
        }

        // 4. Pick the lowest-latency eligible peer
        let (best_did, _) = eligible.iter()
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();

        let channel = self.peer_channels.get(best_did).unwrap();
        self.send_remote(&channel, message).await
    }
}
```

**Key points:**

- Agents address each other by DID, not by node address or PID.
- Local message bus detects "not local" and routes to network service.
- Network service handles transport, encryption, and peer routing.
- Receiving node's network service delivers to local message bus.
- From the agents' perspective, local and remote IPC look identical.
- Capability-based routing finds the best peer without needing its address.

---

## 6. Capability-Based Service Discovery

Agents do not need to know WHERE a service is. They ask the network service for a
capability, and get back the best matching peer.

```rust
/// A capability query from an agent looking for a service.
pub struct CapabilityQuery {
    /// What capability is needed (e.g., "inference", "storage", "code-review").
    pub capability: String,
    /// Minimum trust level the provider must have.
    pub min_trust: TrustLevel,
    /// Prefer local (same node) results over remote.
    pub prefer_local: bool,
    /// Maximum acceptable latency to the provider.
    pub max_latency_ms: Option<u64>,
    /// Maximum number of results to return.
    pub max_results: usize,
}

/// A capability advertisement from a node.
pub struct CapabilityAdvertisement {
    /// Node's DID.
    pub node_did: Did,
    /// Capabilities offered by this node.
    pub capabilities: Vec<NodeCapability>,
    /// Trust level between the querying node and this node.
    pub trust_level: TrustLevel,
    /// Estimated latency from querying node (measured, not claimed).
    pub estimated_latency_ms: u64,
    /// Last health check result for this node.
    pub health: HealthStatus,
    /// When this advertisement expires (must be re-advertised).
    pub expires_at: DateTime<Utc>,
    /// Signature by the advertising node's DID key.
    pub signature: [u8; 64],
}

/// Result of a capability query.
pub struct CapabilityMatch {
    /// The peer that matched.
    pub peer_did: Did,
    /// The matching capability.
    pub capability: NodeCapability,
    /// Trust level with this peer.
    pub trust_level: TrustLevel,
    /// Measured latency to this peer.
    pub latency_ms: u64,
    /// Score combining trust, latency, and capability fitness.
    pub score: f64,
}

/// Gossip protocol for capability advertisements.
pub struct GossipProtocol {
    /// How often to re-advertise capabilities (default: 60s).
    pub advertisement_interval: Duration,
    /// TTL for advertisements (default: 5 minutes). After this,
    /// a capability is considered stale and removed from the index.
    pub advertisement_ttl: Duration,
    /// Maximum gossip fanout (how many peers to forward to).
    pub max_fanout: usize,
    /// Rate limiter to prevent gossip storms.
    pub rate_limit: RateLimit,
}

pub struct RateLimit {
    /// Maximum advertisements per minute from a single peer.
    pub max_per_minute: u32,
    /// Backoff multiplier when rate is exceeded.
    pub backoff_multiplier: f64,
    /// Maximum backoff duration.
    pub max_backoff: Duration,
}
```

---

## 7. Browser Node Networking

Browser nodes have constraints that require specialized handling. They cannot
listen on TCP, cannot participate fully in the DHT, and have limited resources.
But they can still pair, exchange IPC, and host lightweight agents.

```rust
/// Browser node networking modes.
pub enum BrowserNetworkMode {
    /// Connect to a known cloud node via WebSocket.
    /// Browser -> WS -> Cloud Node -> Network
    /// The cloud node acts as a relay, forwarding messages between the
    /// browser and the rest of the network.
    WebSocketRelay {
        /// Address of the relay node (cloud or edge).
        relay_addr: String,
        /// Whether to auto-reconnect on disconnection.
        auto_reconnect: bool,
        /// Maximum reconnection attempts before giving up.
        max_reconnect_attempts: u32,
    },

    /// Direct peer-to-peer via WebRTC (if both peers support it).
    /// Browser <-> WebRTC <-> Browser/Edge
    /// Requires a signaling server for initial connection setup.
    WebRtcDirect {
        /// Signaling server address for WebRTC connection setup.
        signaling_server: String,
        /// ICE servers for NAT traversal (STUN/TURN).
        ice_servers: Vec<String>,
    },

    /// Light client: receive gossip, cannot serve peers.
    /// Used when the browser just needs to observe network state
    /// without actively participating.
    LightClient {
        /// Address of the gossip source node.
        gossip_node: String,
        /// Subscribe to specific capability topics only.
        subscribe_topics: Vec<String>,
    },
}

/// Browser-specific pairing constraints.
pub struct BrowserPairingConfig {
    /// Browser nodes pair via WebSocket, not direct TCP.
    pub transport: TransportProtocol, // always WebSocket

    /// Browser nodes cannot be challenged for server-side nonces.
    /// The relay node performs the challenge on behalf of the browser.
    pub relay_assisted_pairing: bool,

    /// Maximum number of peers a browser node can maintain.
    /// Browser resource constraints limit concurrent connections.
    pub max_peers: usize,

    /// Capabilities a browser node can offer (limited subset).
    pub allowed_capabilities: Vec<String>,
}
```

---

## 8. Network Topology Patterns

```
1. Single Node (development)
   [Node] -- no network needed, all agents are local

2. Star (simple deployment)
   [Cloud Hub] <-- WebSocket --> [Browser 1]
        ^                        [Browser 2]
        |--- TCP ------> [Edge 1]

3. Mesh (production)
   [Cloud A] <-- TCP --> [Cloud B]
       ^                    ^
       |                    |
   [Edge 1] <-- TCP --> [Edge 2]
       ^
       |-- WS --> [Browser 1]

4. Hybrid (enterprise)
   [Corp Cloud A] <== PostQuantum ==> [Corp Cloud B]     (bonded)
          ^                                  ^
          |-- TCP/Noise -------> [Edge Cluster]           (paired)
          |                          |
          |-- WS/TLS ---------> [Browser Users]           (paired)
          |
          |-- Sneakernet -----> [Air-gapped Node]         (offline)

5. Federation (cross-organization)
   [Org A Network]                [Org B Network]
   [Node A1] <== Bonded ==> [Node B1]
   [Node A2]                [Node B2]
   Internal: full mesh      Internal: full mesh
   Cross-org: bonded pair   Cross-org: bonded pair
```

---

## 9. Client Access Layer (User-to-OS)

Sections 1-8 cover node-to-node networking. But there is another equally important
connection type: **how users interact with WeftOS**. A user can connect to WeftOS
through any of these interfaces, and all are equivalent -- none is privileged:

### 9.1 Access Methods

| Interface | Transport | Protocol | Analogy |
|-----------|-----------|----------|---------|
| `weft` CLI | Unix socket / stdin | JSON-RPC | Opening a terminal |
| Shell script | Unix socket / pipe | JSON-RPC | Bash script on Linux |
| MCP client | stdio / HTTP | MCP protocol | Another program talking to the OS |
| Web UI (React, `ui/`) | WebSocket | JSON-RPC over WS | GUI desktop session |
| Chat client | WebSocket | JSON-RPC over WS | Interactive chat session |
| Remote CLI | TCP to network service | JSON-RPC over Noise | SSH-like remote terminal |
| REST/GraphQL API | HTTP | REST/GraphQL | Programmatic access |

### 9.2 Sessions

Every client connection creates a **session**, the WeftOS equivalent of a terminal
session (TTY) on Linux:

```rust
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,

    /// User who owns this session (user 1 in single-user mode)
    pub user: UserId,

    /// How the user connected
    pub access_method: AccessMethod,

    /// The gateway agent handling this session
    pub gateway_pid: Pid,

    /// When the session was created
    pub created_at: DateTime<Utc>,

    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,

    /// Session state (conversation history, context)
    pub state: SessionState,
}

pub enum AccessMethod {
    /// Local CLI (weft command, shell scripts)
    Cli { pid: u32 },
    /// MCP client (local agent communicating via MCP protocol)
    Mcp { transport: McpTransport },
    /// Web UI (browser connecting via WebSocket)
    WebUi { remote_addr: String },
    /// Chat session (WebSocket-based conversational interface)
    Chat { remote_addr: String, channel: String },
    /// Remote CLI (TCP connection from another machine)
    RemoteCli { remote_addr: String, peer_did: Option<Did> },
    /// REST API (stateless, session per request or long-lived)
    Api { remote_addr: String },
}

pub enum McpTransport {
    /// MCP over stdin/stdout (local process)
    Stdio,
    /// MCP over HTTP (local or remote)
    Http { addr: String },
    /// MCP over WebSocket
    WebSocket { addr: String },
}
```

### 9.3 Gateway Agents

Each access method has a **gateway agent** that translates between the external
protocol and kernel IPC:

| Gateway Agent | Access Methods | Already Exists? |
|---------------|---------------|-----------------|
| `cli-gateway` | `weft` CLI, shell scripts | Partially (`clawft-cli/`) |
| `api-gateway` | Web UI, REST API, WebSocket | Partially (`clawft-services/src/api/`) |
| `mcp-gateway` | MCP clients (local agents) | Partially (`clawft-services/src/mcp/`) |
| `chat-gateway` | Chat sessions | New (wraps api-gateway) |

Gateway agents are service agents like any other:

```toml
# .agents/api-gateway.agent.toml

[agent]
name = "api-gateway"
version = "0.1.0"
description = "HTTP/WebSocket gateway for Web UI and API clients"
role = "service"

[capabilities]
tools = ["session_create", "session_join", "session_list", "session_close"]
ipc_scope = "all"         # gateway needs to route to any service
topics_publish = ["session.created", "session.closed"]
topics_subscribe = ["session.request"]
can_spawn = true          # spawns per-session handler agents
filesystem_access = []

[resources]
max_memory_mb = 256
priority = "high"

[interface]
protocol = "rest"
listen_addr = "0.0.0.0:3000"
response_mode = "direct"

[health]
check_interval = "10s"
timeout = "3s"
restart_policy = "always"

[dependencies]
requires = ["message-bus"]
after = ["kernel-init"]
```

### 9.4 Session Lifecycle

```
User opens terminal / browser / MCP client
    |
    v
Gateway agent receives connection
    |
    v
Create Session (or join existing by session ID)
    |
    v
Session routes commands to kernel via IPC
    |
    ├── "weave kernel status"  -> kernel status handler
    ├── "weft agent spawn"    -> root agent via IPC
    ├── "weft model list"     -> inference-service via IPC
    ├── chat message          -> agent loop via IPC
    └── tool call             -> tool-registry via IPC
    |
    v
Results returned to client via gateway
```

**Joining sessions**: Like `tmux attach`, a user can join an existing session
from a different interface. For example, start a task via CLI, then join the
same session from the web UI to see progress visually:

```
weave session list                    -- list active sessions
weave session join <session-id>       -- join existing session from CLI
```

The web UI can also list and join sessions -- like opening a new tab in the
same terminal multiplexer.

### 9.5 Interaction with Existing Code

The project already has the foundations:

| Component | Location | Role |
|-----------|----------|------|
| CLI | `crates/clawft-cli/` | `weft` command, subcommand routing |
| API server | `crates/clawft-services/src/api/` | Axum HTTP/WebSocket server |
| MCP server | `crates/clawft-services/src/mcp/` | MCP protocol handler |
| React UI | `ui/` | Web frontend (already scaffolded) |
| WebSocket handler | `crates/clawft-services/src/api/ws.rs` | Real-time communication |
| Auth middleware | `crates/clawft-services/src/api/auth.rs` | Session authentication |

These become gateway agents in the agent-first model. The existing code does not
change significantly -- it just gets wrapped in agent manifests and communicates
via IPC instead of direct function calls.

---

## 10. Configuration

```toml
# weft.toml -- networking section

[network]
# Whether networking is enabled. When false, the node operates in
# single-node mode with no peer discovery or cross-node IPC.
enabled = true

# TCP listen address for incoming peer connections.
listen_addr = "0.0.0.0:9100"

# Default transport protocol.
protocol = "tcp"                        # tcp | quic | websocket

# Node name (human-readable, for debugging and peer summary).
node_name = "my-weftos-node"

[network.discovery]
# Zero-config LAN discovery via mDNS.
mdns = true

# Kademlia DHT for wide-area discovery.
dht = true

# Well-known bootstrap nodes (entry points to the network).
# Like DeFi RPC endpoints -- stable addresses that new nodes connect to.
bootstrap_nodes = [
    "/ip4/seed1.weftos.net/tcp/9100",
    "/ip4/seed2.weftos.net/tcp/9100",
]

# How often to re-run discovery (seconds).
discovery_interval_secs = 30

# Maximum peers to maintain active connections with.
max_peers = 50

[network.encryption]
# Default encryption for new connections (before pairing).
default = "none"                        # none | noise | tls | post-quantum

# Encryption for agent IPC between paired nodes.
ipc_channels = "noise"

# Encryption for model weight sync (doc 11).
model_sync = "post-quantum"

# Encryption for discovery gossip (should be "none" for performance).
discovery = "none"

[network.trust]
# Whether nodes must pair before IPC is allowed.
require_pairing = true

# Minimum trust level for agent IPC.
min_trust_for_ipc = "paired"            # paired | trusted | bonded

# Minimum trust level for model weight exchange.
min_trust_for_model_sync = "bonded"

# Number of successful interactions to reach Trusted.
interactions_for_trust = 50

# How quickly reputation fades without interaction (days).
reputation_decay_days = 30

# Whether to auto-ban peers that reach zero reputation.
auto_ban = false

[network.bonding]
# Whether DeFi-style bonding is enabled.
enabled = false

# Default bond amount in resource units.
bond_amount = 1000

# Default bond duration in hours.
bond_duration_hours = 168               # 7 days

# Hours of offline before bond is slashed.
slash_offline_hours = 24

# Percentage of bond slashed per violation.
slash_percentage = 10

[network.gossip]
# How often to re-advertise capabilities (seconds).
advertisement_interval_secs = 60

# TTL for capability advertisements (seconds).
advertisement_ttl_secs = 300            # 5 minutes

# Maximum gossip fanout per advertisement.
max_fanout = 6

# Maximum advertisements per minute from a single peer.
rate_limit_per_minute = 30

[network.browser]
# Configuration for browser node relay.
relay_enabled = false
relay_listen_addr = "0.0.0.0:9101"      # WebSocket listen for browsers
max_browser_connections = 100
```

---

## 11. New Files

| File | Phase | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/network.rs` | K6 | `NetworkService` agent, transport management, peer lifecycle |
| `crates/clawft-kernel/src/discovery.rs` | K6 | `NetworkDiscovery`, mDNS, Kademlia DHT, bootstrap, gossip protocol |
| `crates/clawft-kernel/src/pairing.rs` | K6 | `PairingHandshake`, `HelloMessage`, `ChallengeMessage`, `ProofMessage`, `AcceptMessage` |
| `crates/clawft-kernel/src/trust.rs` | K6 | `TrustLevel`, `TrustEngine`, `TrustState`, reputation scoring and decay |
| `crates/clawft-kernel/src/bond.rs` | K6 | `Bond`, `BondScope`, `SlashCondition`, `BondManager`, `BondSlasher` |
| `crates/clawft-kernel/src/peer.rs` | K6 | `PeerInfo`, `PeerState`, `PeerSummary`, capability advertisement |
| `crates/clawft-kernel/src/relay.rs` | K6 | Cross-node message relay, DID-based routing, capability-based routing |
| `crates/clawft-kernel/src/capability_index.rs` | K6 | `CapabilityQuery`, `CapabilityMatch`, gossip-based index maintenance |
| `crates/clawft-kernel/src/transport.rs` | K6 (extend) | Add `TransportChannel`, `ChannelEncryption`, `ChannelState` (extends doc 08 `IpcTransport`) |
| `.agents/network-service.agent.toml` | K6 | Network service agent manifest |

---

## 12. Impact on Existing Phases

| Phase | Impact |
|-------|--------|
| K0 | `NetworkConfig` added to `KernelConfig` (optional section). Network service is an optional service agent -- nodes without `[network]` config skip it entirely. |
| K2 | Message bus gains "not local" routing: when a message targets a DID not in the local process table, it routes to the network service agent instead of returning an error. This is the key integration point. |
| K6 (doc 08) | Doc 08's K6.1 (cluster bootstrap) is now a special case of network joining where all nodes are bonded and share governance hashes. Doc 08's K6.2 (cross-node IPC) uses the network service for DID routing. |
| K6 (doc 08 K6.5) | Agent migration (doc 08 K6.5) uses the network service to route IPC to the agent's new node after migration. The network service updates its peer channel when it receives a migration notification. |
| K6 (doc 08 K6.6) | Browser node support (doc 08 K6.6) uses the browser networking mode from this document. WebSocket relay, WebRTC direct, and light client modes. |
| Doc 11 | Model weight sync between nodes requires bonded trust level. The network service enforces this by checking trust before allowing model sync traffic. |

---

## 13. CLI Commands

```
weave network status                         -- show network state (peers, trust levels, bonds)
weave network peers                          -- list connected peers with capabilities and trust
weave network peers --trust bonded           -- filter peers by trust level
weave network discover                       -- run discovery (mDNS + DHT + bootstrap)
weave network discover --method mdns         -- run only mDNS discovery
weave network pair <address>                 -- initiate pairing with a specific node address
weave network pair --did <did>               -- initiate pairing with a node by DID (requires DHT)
weave network unpair <peer>                  -- remove pairing with a peer
weave network bond <peer> [amount]           -- post a bond with a peer
weave network bond --scope model-sync <peer> -- post a scoped bond
weave network unbond <peer>                  -- withdraw bond from a peer
weave network trust <peer>                   -- show trust details for a peer
weave network trust --history <peer>         -- show interaction history with a peer
weave network capabilities                   -- show this node's advertised capabilities
weave network capabilities --search <query>  -- find peers by capability
weave network capabilities --peer <did>      -- show capabilities of a specific peer
weave network topology                       -- show network topology graph (ASCII)
weave network topology --format dot          -- output topology as Graphviz DOT
weave network relay status                   -- show relay status (if this node is a relay)
weave network gossip stats                   -- show gossip protocol statistics
```

---

## 14. Testing Strategy

| Test | Description |
|------|-------------|
| `mdns_local_discovery` | Two nodes on same LAN find each other via mDNS within 5 seconds |
| `dht_wan_discovery` | Node finds peer via Kademlia DHT after bootstrap node lookup |
| `bootstrap_discovery` | New node connects to bootstrap node and receives peer list |
| `pairing_handshake_success` | Full HELLO -> CHALLENGE -> PROVE -> ACCEPT flow completes |
| `pairing_invalid_did` | Reject pairing when DID signature does not verify against nonce |
| `pairing_protocol_mismatch` | Reject pairing when protocol versions are incompatible |
| `pairing_timeout` | Handshake fails cleanly when peer does not respond within timeout |
| `pairing_duplicate` | Second pairing attempt with already-paired node returns AlreadyPaired |
| `trust_progression` | Peer transitions Unknown -> Paired -> Trusted after configured interactions |
| `trust_degradation` | Trust drops when node serves invalid data or goes offline |
| `trust_reputation_decay` | Reputation score decays toward zero when no interactions occur |
| `bond_creation` | Bond posted successfully, both peers see the bond in their bond manager |
| `bond_slash_offline` | Bond is slashed when node goes offline longer than slash condition |
| `bond_slash_invalid_data` | Bond is slashed when node serves data with mismatched content hash |
| `bond_renewal` | Expired bond is renewed before grace period, trust maintained |
| `bond_expiry` | Expired bond not renewed, peer drops from Bonded to Trusted |
| `cross_node_ipc` | Agent on node A sends IPC to agent on node B via DID routing |
| `did_routing_local` | IPC addressed by local DID routes via message bus, not network |
| `did_routing_remote` | IPC addressed by remote DID routes via network service |
| `capability_search` | Query for "inference" capability returns peers with inference models |
| `capability_search_trust_filter` | Query with min_trust=Trusted filters out Paired-only peers |
| `capability_search_latency_sort` | Results sorted by latency, lowest first |
| `capability_gossip` | Capability advertisement propagates to peers within gossip TTL |
| `capability_expiry` | Stale capability advertisement removed after TTL expires |
| `encryption_upgrade` | Connection upgrades from plain TCP to Noise when pairing completes |
| `encryption_post_quantum` | Bonded connection uses ML-KEM-768 for key exchange |
| `browser_websocket_pair` | Browser node pairs with cloud node via WebSocket relay |
| `browser_ipc` | Browser agent sends IPC to cloud agent via WebSocket relay |
| `browser_reconnect` | Browser node reconnects to relay after disconnection |
| `network_partition` | Nodes continue working during partition, re-pair on reconnect |
| `air_gapped_sync` | Air-gapped node pairs and syncs state when physically reconnected |
| `gossip_rate_limit` | Peer exceeding gossip rate limit is backoff-throttled |
| `relay_message_delivery` | Message relayed through intermediate node reaches target |
| `concurrent_pairing` | Multiple nodes pair with the same peer simultaneously without deadlock |

---

## 15. Risks and Mitigations

| ID | Risk | Severity | Mitigation |
|----|------|----------|------------|
| R1 | Eclipse attack (malicious nodes surround target, controlling all its peer connections) | High | Multiple independent bootstrap nodes. DHT peer diversity enforced (prefer peers from different IP ranges). Out-of-band verification for critical peers (e.g., governance nodes verify via DNS TXT records). |
| R2 | Sybil attack (one entity runs many fake nodes to gain disproportionate influence) | High | Bond mechanism raises the cost of sybil identities -- each fake node must post a bond. Reputation earned over time, not instantly -- new nodes start at Unknown regardless of how many they spin up. Rate limiting on pairing prevents rapid sybil flooding. |
| R3 | Plain TCP discovery leaks node existence to network observers | Medium | Discovery gossip contains no sensitive data -- only DIDs (public), capabilities (public), and addresses (necessary for connectivity). For stealth deployments, disable mDNS and use only direct pairing or pre-configured bootstrap nodes. Optional: encrypt gossip with a shared network key. |
| R4 | NAT traversal for edge nodes behind restrictive firewalls | Medium | STUN/TURN via bootstrap nodes for NAT hole-punching. QUIC handles NAT better than TCP (UDP-based, maintains mappings). WebRTC with ICE for browser-to-edge connections. Relay nodes as last resort for nodes behind symmetric NATs. |
| R5 | Bond value unclear without real token economy | Low | Start with internal resource units (not real money). Bond amounts are configurable per deployment. DAA token economy integration can come post-K6 as an optional extension. The mechanism is the same regardless of what the "units" represent. |
| R6 | Browser nodes cannot participate fully in the network | Low | By design. Browser nodes are light clients: they consume services but do not provide relay, bootstrap, or heavy compute capabilities. Cloud and edge nodes do the heavy lifting. Browser capabilities are scoped to Assistant and UI-facing roles. |
| R7 | Key compromise leaks all peer relationships and trust state | High | Key rotation via exochain DID key versioning (valid_from/revoked_at). Compromised key is revoked across all peers via gossip. New key is issued, agent DID preserved (DID is derived from genesis key, not current key). Peers re-verify the new key during next interaction. |
| R8 | Gossip protocol storms under high peer churn | Medium | Gossip rate limiting per peer (max advertisements per minute). TTL on advertisements prevents unbounded propagation. Exponential backoff when rate exceeded. Gossip fanout capped to prevent amplification. |
| R9 | Pairing handshake vulnerable to man-in-the-middle before Noise upgrade | Medium | The PROVE step (step 3) includes a cryptographic signature of the nonce, binding the handshake to the DID. An attacker who intercepts HELLO/CHALLENGE cannot produce a valid PROVE without the private key. Noise XX handshake at step 4 provides forward secrecy for all subsequent traffic. |
| R10 | Network service becomes a bottleneck for all cross-node traffic | Medium | Network service only handles connection management and routing decisions. Actual data transfer uses direct peer channels established during pairing. Message bus integration is zero-copy for local routing. For high-throughput scenarios, multiple relay agents can be spawned. |

---

## 16. Cross-References

| Document | Relationship |
|----------|-------------|
| `08-ephemeral-os-architecture.md` | Layer 0 (Node Fabric) provides the physical substrate; this document defines the networking protocols that run on it. Layer 2 (Identity + IPC Fabric) uses the network service for cross-node DID routing. K6.1 cluster bootstrap is a special case of network joining. |
| `10-agent-first-single-user.md` | Network service is an agent (PID 6) following the service agent manifest pattern. It uses IPC topics for all communication, consistent with the agent-first architecture. Root agent spawns it after message-bus. |
| `07-ruvector-deep-integration.md` | ruvector-cluster for baseline service discovery. ruvector-delta-consensus for CRDT-based peer state sync. rvf-wire for cross-node message serialization. rvf-crypto for witness chain recording of pairing and trust events. |
| `09-environments-and-learning.md` | Network trust levels interact with environment governance: a peer's trust level constrains which environments it can participate in. Production environments may require Bonded trust for all peers. |
| `11-local-inference-agents.md` | Model weight sync uses bonded channels for exchange (doc 11 section 7). The network service enforces that model sync traffic only flows over post-quantum encrypted, bonded connections. Capability advertisement includes inference models available on each node. |
| `00-orchestrator.md` | Adds networking to K6 scope. Network service is an optional core service, enabled by `[network]` configuration. |
| QuDAG (`ruv/packages/qudag/`) | libp2p transport stack, Kademlia DHT implementation, ML-KEM-768/ML-DSA post-quantum crypto. The network service wraps QuDAG behind the adapter pattern (depend, do not fork; wrap, do not expose). |
| Exochain (`ruv/packages/exochain/`) | DIDs for peer identity, HLC for causal ordering of cross-node messages, DAG for signed peer state. DID derivation, key rotation, and signature verification use exochain primitives. |

---

## 17. Cargo Dependencies

### New dependencies for networking (feature-gated)

```toml
# crates/clawft-kernel/Cargo.toml additions

[features]
# Networking subsystem (peer discovery, pairing, trust, bonding)
networking = [
    "ruvector-cluster",     # existing: service discovery, health
    "ruvector-ipc",         # existing: delta-consensus for peer state sync
    "ruvector-wire",        # existing: binary wire format for cross-node messages
    "ruvector-crypto",      # existing: witness chain for pairing/trust events
    "dep:exo-core",         # existing (from distributed): BLAKE3, HLC, domain separators
    "dep:exo-identity",     # existing (from distributed): DID derivation, key verification
]

# Full distributed networking (networking + doc 08 distributed)
distributed-networking = [
    "networking",
    "distributed",          # existing: cluster bootstrap, agent directory, cross-node IPC
]

[dependencies]
# Networking-specific (all optional, feature-gated behind "networking")
# No new external crates -- networking uses QuDAG and exochain via existing deps
# libp2p transports come from exo-api (already in distributed feature)
```

The networking feature builds on top of existing ruvector and exochain dependencies.
No new external crates are introduced. The transport layer (TCP, QUIC, WebSocket)
uses libp2p from `exo-api`. The DHT uses Kademlia from `exo-api`. Post-quantum
crypto uses ML-KEM-768 from QuDAG's `exo-core`.

---

## 18. Definition of Done

### For this architecture document

- [x] Core insight: networks not clusters, permissionless discovery, capability-based
- [x] Transport layers defined with encryption opt-in per connection
- [x] Network joining protocol (DeFi-inspired) with 5-step pairing handshake
- [x] Trust levels (Unknown / Paired / Trusted / Bonded) with progression and degradation
- [x] Bond mechanism with slashing conditions
- [x] Network service agent defined with manifest and capabilities
- [x] Cross-node agent communication flow with transparent DID routing
- [x] Capability-based service discovery with gossip protocol
- [x] Browser node networking modes (WebSocket relay, WebRTC direct, light client)
- [x] Network topology patterns (single, star, mesh, hybrid, federation)
- [x] Configuration in TOML format with all tunables
- [x] New files listed with phase assignment
- [x] Impact on existing phases documented
- [x] CLI commands for network management
- [x] Testing strategy with unit and integration test cases
- [x] Risk assessment with mitigations
- [x] Cross-references to all dependent documents
- [x] Cargo dependency analysis (no new external crates)

### For implementation (future K6 phases)

- [ ] Network service agent boots and starts discovery
- [ ] mDNS discovers peers on LAN within 5 seconds
- [ ] Kademlia DHT discovers peers via bootstrap nodes
- [ ] Pairing handshake completes with DID verification
- [ ] Trust progression works: Unknown -> Paired -> Trusted after interactions
- [ ] Bond posting and slashing works correctly
- [ ] Cross-node IPC routes messages by DID transparently
- [ ] Capability search finds peers by advertised capability
- [ ] Browser node pairs via WebSocket relay
- [ ] Gossip protocol distributes capability advertisements
- [ ] Encryption upgrades from None to Noise on pairing
- [ ] Post-quantum encryption active on bonded connections
- [ ] All tests in section 13 pass
- [ ] No QuDAG or exochain type appears in clawft-kernel public API
- [ ] WASM browser build unaffected by networking feature
- [ ] Single-node mode works with networking disabled
