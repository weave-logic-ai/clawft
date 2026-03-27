# K5 Panel 3: Security and Identity Model

**Date**: 2026-03-25
**Panel**: Security Architecture
**Status**: COMPLETE

---

## 1. Node Identity

### Ed25519 Public Key as NodeId

Current WeftOS nodes use UUID-based `NodeId` values. K6 replaces this with
Ed25519 public keys as the canonical node identity:

```rust
/// Node identity derived from Ed25519 keypair.
/// The public key IS the node ID -- no separate UUID needed.
pub struct NodeIdentity {
    /// Ed25519 signing keypair (private key + public key)
    keypair: ed25519_dalek::SigningKey,
    /// Derived node ID: first 16 bytes of SHAKE-256(pubkey)
    node_id: [u8; 16],
    /// Human-readable display: hex(node_id[0..8])
    short_id: String,
}

impl NodeIdentity {
    /// Generate a new random identity.
    pub fn generate() -> Self { /* ... */ }

    /// Load from a persisted key file.
    pub fn from_file(path: &Path) -> Result<Self> { /* ... */ }

    /// The Ed25519 public key (32 bytes).
    pub fn public_key(&self) -> &ed25519_dalek::VerifyingKey { /* ... */ }

    /// Sign arbitrary data with this node's private key.
    pub fn sign(&self, data: &[u8]) -> ed25519_dalek::Signature { /* ... */ }
}
```

**Rationale**: Using the public key as identity means every node can
authenticate itself by signing a challenge -- no certificate authority or
shared secret distribution needed. The trust anchor is governance.genesis,
which records the founding node's public key and cluster policy.

### Identity Hierarchy

```
governance.genesis
    |
    +-- Founding node pubkey (trust root)
    |
    +-- Governance rules (who can join, what capabilities are valid)
    |
    +-- Cluster ID = SHAKE-256(genesis payload)
         |
         +-- Node A pubkey (joined, signed genesis_hash)
         |
         +-- Node B pubkey (joined, signed genesis_hash)
         |
         +-- Node C pubkey (joined, signed genesis_hash)
```

Every node in the cluster holds the same `genesis_hash`. When a new node
presents a `JoinRequest`, the receiving node verifies:

1. The `genesis_hash` matches (same cluster)
2. The `JoinRequest` is signed by the joining node's Ed25519 key
3. The GovernanceGate approves the join (policy check)

---

## 2. Noise Protocol Handshakes

### Why Noise (via snow)

The Noise Protocol Framework provides authenticated key exchange with:
- **Transport agnosticism** -- operates over any reliable byte stream
- **Forward secrecy** -- ephemeral keys ensure past sessions remain secure
- **Mutual authentication** -- both parties prove identity
- **Simplicity** -- no X.509 certificates, no PKI, no CRL

WeftOS uses `snow 0.9` as the Noise implementation, the same library used
by libp2p and WireGuard.

### Handshake Patterns

#### XX Pattern -- First Contact

Used when two nodes have never communicated before. Both sides transmit their
static (Ed25519-derived) public keys during the handshake.

```
Initiator (I)                          Responder (R)
    |                                       |
    |-- -> e -------------------------------->|  (I sends ephemeral pubkey)
    |                                       |
    |<- <- e, ee, s, es --------------------|  (R sends ephemeral + static,
    |                                       |   proves identity)
    |                                       |
    |-- -> s, se --------------------------->|  (I sends static,
    |                                       |   proves identity)
    |                                       |
    |   [Both sides authenticated,          |
    |    symmetric keys derived]            |
```

**Properties**: Mutual authentication, forward secrecy, identity hiding
(static keys encrypted after first message).

#### IK Pattern -- Known Peer Reconnection

Used when the initiator already knows the responder's static public key
(cached from a previous XX handshake or received via DHT).

```
Initiator (I)                          Responder (R)
    |                                       |
    |-- -> e, es, s, ss ------------------>|  (I encrypts to R's known key)
    |                                       |
    |<- <- e, ee, se ----------------------|  (R proves identity)
    |                                       |
    |   [Authenticated in 1 RTT]           |
```

**Properties**: 1-RTT mutual authentication (vs 1.5-RTT for XX), forward
secrecy, requires prior key knowledge.

### Noise Configuration

```rust
/// Build a snow Noise session for WeftOS mesh connections.
fn build_noise_params() -> snow::params::NoiseParams {
    "Noise_XX_25519_ChaChaPoly_BLAKE2b"
        .parse()
        .expect("valid noise params")
}

/// For known peers, use IK pattern for 1-RTT.
fn build_noise_params_ik() -> snow::params::NoiseParams {
    "Noise_IK_25519_ChaChaPoly_BLAKE2b"
        .parse()
        .expect("valid noise params")
}
```

| Component | Choice | Rationale |
|-----------|--------|-----------|
| DH | 25519 (X25519) | Fast, small keys, widely audited |
| Cipher | ChaChaPoly | AEAD, constant-time, no AES-NI dependency |
| Hash | BLAKE2b | Fast, 256-bit security, aligns with BLAKE3 migration path |

---

## 3. Post-Quantum Cryptography

### Current State

WeftOS already has post-quantum cryptography in the codebase:

- **chain.rs**: ML-DSA-65 dual signing for chain events via `DualSignature`
- **ruvector-dag**: `pqcrypto-dilithium` (ML-DSA-65) and `pqcrypto-kyber`
  (ML-KEM-768) compiled and available

### K6 Post-Quantum Strategy

| Layer | Classical | Post-Quantum | Timeline |
|-------|-----------|-------------|----------|
| Chain event signing | Ed25519 | ML-DSA-65 (dual) | Now (K5) |
| Node identity | Ed25519 | ML-DSA-65 (dual) | K6.0 |
| Noise handshake | X25519 | -- | K6.1 |
| Key exchange | X25519 | ML-KEM-768 (hybrid) | K6.4b (moved from K7+) |
| Message signing | Ed25519 | ML-DSA-65 (optional) | K6.3 |

### Dual Signing for Chain Events

Every chain event that crosses a node boundary gets dual-signed:

```
ChainEvent
  |-- hash: SHAKE-256(canonical fields)
  |-- ed25519_sig: Ed25519::sign(hash)
  |-- mldsa65_sig: ML-DSA-65::sign(hash)    [post-quantum]
```

A verifier checks both signatures. If either fails, the event is rejected.
This provides quantum resistance today without abandoning the classical
signature that existing tooling understands.

### ML-KEM-768 Hybrid Key Exchange (moved to K6.4b by D11)

The Noise handshake is upgraded to a hybrid scheme in K6.4b:

```
shared_secret = HKDF(X25519_shared || ML-KEM-768_shared)
```

Both key exchanges must succeed. An attacker must break both X25519 AND
ML-KEM-768 to recover the shared secret. This is the same pattern used by
Chrome, Signal, and AWS post-quantum TLS.

### Hybrid Noise + KEM Transport Encryption (K6.4b)

The Noise XX handshake uses X25519 Diffie-Hellman, which is vulnerable to
quantum computing (Shor's algorithm). To protect mesh traffic against
store-now-decrypt-later attacks, K6.4b adds a hybrid key exchange:

**Protocol**:
```
Noise XX (classical)     →  classical_ss (X25519)
ML-KEM-768 (PQ, inside   →  pq_ss (Kyber768)
  Noise channel)
HKDF(classical || pq)   →  final session key
```

The KEM exchange runs inside the already-encrypted Noise channel, so it's
protected during transit. Both X25519 AND ML-KEM must be broken to
compromise the session.

**Existing infrastructure**: `ruvector-dag/src/qudag/crypto/ml_kem.rs` provides
`MlKem768::generate_keypair()`, `encapsulate()`, `decapsulate()`. Behind
`production-crypto` feature for real pqcrypto-kyber; HKDF placeholder without.

**Negotiation**: `HandshakePayload.kem_supported: bool` + optional `kem_public_key`.
Nodes that don't support KEM stay on classical Noise (graceful degradation).

**Cost**: ~2.4KB extra + ~1ms per connection. Zero per-message overhead.

---

## 4. Threat Model

### 4.1 Eavesdropping

**Threat**: Attacker intercepts network traffic between nodes.

**Mitigation**: All inter-node traffic is encrypted via Noise Protocol
(ChaChaPoly AEAD). The Noise handshake provides forward secrecy -- even if
a node's long-term key is compromised, past sessions remain encrypted.

**Residual risk**: None with properly implemented Noise.

### 4.2 Man-in-the-Middle (MITM)

**Threat**: Attacker interposes between two nodes during handshake.

**Mitigation**: Noise XX/IK handshakes authenticate both parties using their
Ed25519 static keys. After the first successful handshake, nodes cache each
other's public keys and use the IK pattern, which binds the initiator's
first message to the responder's known key.

**Residual risk**: First-contact MITM is possible if the attacker controls
the discovery layer. Mitigated by verifying `genesis_hash` in the WeftOS
handshake layer (above Noise).

### 4.3 Sybil Attack

**Threat**: Attacker creates many fake node identities to overwhelm the
cluster or manipulate DHT routing.

**Mitigation**:
- GovernanceGate evaluates every `JoinRequest` -- governance rules can
  require proof-of-work, rate-limit joins, or require manual approval
- `ClusterConfig.max_nodes` caps cluster size
- `genesis_hash` verification prevents cross-cluster infiltration

**Residual risk**: Within governance policy limits, Sybil resistance depends
on the strength of governance rules.

### 4.4 Eclipse Attack

**Threat**: Attacker surrounds a target node with malicious peers, isolating
it from honest nodes.

**Mitigation**:
- Kademlia's structured routing table makes eclipse attacks expensive
  (attacker must control specific key-space regions)
- `ClusterMembership` detects when a node becomes `Unreachable` and alerts
- Seed peer list in `ClusterConfig` provides fallback connectivity

**Residual risk**: Small clusters (<20 nodes) are more vulnerable. Mitigation:
ensure seed peers are geographically and network-diverse.

### 4.5 Replay Attack

**Threat**: Attacker records and replays legitimate messages.

**Mitigation**:
- Noise Protocol's nonce-based encryption prevents replay at the transport
  layer -- each message has a unique nonce
- `KernelMessage.id` provides application-layer deduplication
- `KernelMessage.timestamp` enables staleness detection
- Chain events have monotonic `sequence` numbers

**Residual risk**: Application-layer deduplication must be implemented in K6.3
(currently a RED item).

### 4.6 Denial of Service (DoS)

**Threat**: Attacker floods a node with connection attempts or messages.

**Mitigation**:
- 16 MiB maximum message size prevents memory exhaustion
- `ResourceLimits` enforce per-agent message count budgets
- QUIC (quinn) has built-in connection-level flow control
- Rate-limiting on `add_peer()` (YELLOW item, to be added)

**Residual risk**: CPU exhaustion from Noise handshake floods. Mitigation:
implement handshake rate limiting with exponential backoff.

### 4.7 Quantum Computing

**Threat**: Future quantum computer breaks X25519 (Noise DH) and Ed25519
(signatures), enabling retroactive decryption and forgery.

**Mitigation**:
- Chain events are dual-signed with ML-DSA-65 (quantum-resistant) today
- ML-KEM-768 hybrid key exchange in K6.4b (moved from K7+ by D11)
- Forward secrecy limits exposure -- past ephemeral keys are discarded

**Residual risk**: Noise handshake key exchange is not quantum-resistant until
K6.4b hybrid ML-KEM-768 upgrade. "Harvest now, decrypt later" attacks on
transport-layer data are possible until K6.4b is deployed.

### Threat Summary

| Threat | Severity | Mitigation Layer | K6 Status |
|--------|----------|-----------------|-----------|
| Eavesdropping | High | Noise encryption | Addressed in K6.1 |
| MITM | High | Noise auth + genesis_hash | Addressed in K6.1 |
| Sybil | Medium | GovernanceGate + max_nodes | Addressed in K6.0 |
| Eclipse | Medium | Kademlia + seed peers | Addressed in K6.2 |
| Replay | Medium | Noise nonce + message ID | Addressed in K6.3 |
| DoS | Medium | Rate limiting + QUIC flow control | Partial in K6.1 |
| Quantum | Low (future) | Dual signing now, hybrid KEM in K6.4b | Addressed (K6.4b) |

---

## 5. Governance Gate Enforcement on Cluster Join

### Join Request Flow

```rust
/// A signed request to join a cluster.
#[derive(Serialize, Deserialize)]
pub struct JoinRequest {
    /// Joining node's Ed25519 public key (32 bytes)
    pub pubkey: [u8; 32],
    /// SHAKE-256 hash of the cluster's governance.genesis event
    pub genesis_hash: [u8; 32],
    /// Node platform (CloudNative, Edge, Browser, Wasi)
    pub platform: NodePlatform,
    /// Advertised capabilities
    pub capabilities: Vec<String>,
    /// Unix timestamp (seconds)
    pub timestamp: u64,
    /// Ed25519 signature over (genesis_hash || platform || capabilities || timestamp)
    pub signature: [u8; 64],
}
```

### Verification Steps

```
1. Deserialize JoinRequest (enforce 16 MiB max)
2. Verify signature against pubkey
3. Verify genesis_hash matches local genesis
4. Verify timestamp is within acceptable skew (60s)
5. Construct GovernanceRequest:
     agent_id = hex(pubkey[0..8])
     action = "cluster.join"
     resource = platform.to_string()
     context = { capabilities, pubkey }
6. Evaluate against GovernanceEngine
7. If Allowed:
     - Create PeerNode with id = hex(SHAKE-256(pubkey)[0..16])
     - Call ClusterMembership::add_peer()
     - Send JoinResponse { accepted: true, peer_list, rules, chain_head }
8. If Denied:
     - Send JoinResponse { accepted: false, reason }
     - Close connection
```

---

## 6. Browser Node Security Model

Browser nodes connect via WebSocket and run in a constrained environment:

| Aspect | Browser Constraint | WeftOS Handling |
|--------|-------------------|----------------|
| No raw TCP/UDP | WebSocket or WebRTC only | `MeshTransport::WebSocket` impl |
| No filesystem | Cannot persist keys across sessions | Session-scoped ephemeral keys or IndexedDB |
| Same-origin policy | WS connections subject to CORS | WeftOS mesh endpoint sets appropriate headers |
| No long-lived connections | Browser may kill idle WS | Heartbeat keepalive over WS, auto-reconnect |
| Limited crypto | SubtleCrypto API available | WASM-compiled snow for Noise handshakes |
| Untrusted environment | User can inspect/modify JS | Browser nodes get `IpcScope::Restricted` by default |

### Browser Node Capabilities

Browser nodes are second-class citizens by design:

- **Cannot host services** -- can only consume services from Cloud/Edge nodes
- **Cannot extend chains** -- can only read and verify chain events
- **Cannot modify the resource tree** -- read-only tree access
- **Restricted governance** -- GovernanceGate applies `browser_policy` rules
- **Session-scoped identity** -- ephemeral Ed25519 keypair per session unless
  the user opts into IndexedDB persistence

This is enforced by governance rules, not code -- a cluster operator can
grant browser nodes additional capabilities if their threat model permits it.

---

## 7. Key Management

### Key Storage

| Platform | Key Location | Protection |
|----------|-------------|-----------|
| CloudNative | `$WEFTOS_DATA/identity.key` | File permissions (0600) |
| Edge | `$WEFTOS_DATA/identity.key` | File permissions + optional TPM |
| Browser | IndexedDB or session-only | SubtleCrypto non-extractable |
| Wasi | Injected via WASI env | Host-controlled |
| Embedded | Flash/EEPROM | Hardware-specific |

### Key Rotation

Key rotation is a governance-gated operation:

1. Node generates new Ed25519 keypair
2. Node signs a `KeyRotation` chain event with the OLD key, embedding the
   NEW public key
3. Node signs the same event with the NEW key (proves possession)
4. Peers verify both signatures and update their peer records
5. Old key is revoked after a configurable grace period

---

## 8. Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| S1 | Ed25519 pubkey as NodeId | Eliminates UUID, enables challenge-response auth |
| S2 | Noise XX for first contact | Mutual auth + forward secrecy without PKI |
| S3 | Noise IK for known peers | 1-RTT reconnection performance |
| S4 | ChaChaPoly + BLAKE2b in Noise | No AES-NI dependency, BLAKE3 migration path |
| S5 | ML-DSA-65 dual signing on chain events | Quantum-resistant provenance today |
| S6 | ML-KEM-768 hybrid KEM deferred to K7+ (updated by D11 to K6.4b) | Implementations not mature enough; D11 moved hybrid KEM to K6.4b using existing ruvector-dag ML-KEM |
| S7 | Browser nodes are restricted by default | Untrusted environment, governance-gated upgrade |
| S8 | governance.genesis as cluster trust root | Already exists, natural authority anchor |
| S9 | 16 MiB max message size | Prevents memory exhaustion |
| S10 | Key rotation via dual-signed chain event | Auditable, verifiable, governance-gated |
| S11 | Hybrid Noise + ML-KEM-768 key exchange in K6.4b for PQ transport protection | Store-now-decrypt-later defense, leverages existing ruvector-dag ML-KEM |
