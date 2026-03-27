# K6 Test Strategy Review

Version: Pre-implementation Review
Date: 2026-03-26
Reviewer: QA Agent
Source: `.planning/sparc/weftos/07-phase-K6-mesh-framework.md`

---

## Exit Criteria Audit

39 exit criteria identified. Each assessed for automated testability, mock requirements, and recommended approach.

| # | Criterion | Auto-testable? | Mock needed? | Test approach |
|---|-----------|---------------|-------------|---------------|
| 1 | Two CloudNative nodes connect via QUIC with Noise XX encryption | Yes | InMemoryTransport OR loopback QUIC | Unit: Noise handshake over in-memory streams. Integration: two quinn endpoints on localhost |
| 2 | A Browser node connects via WebSocket with Noise encryption | Partial | MockWsTransport | Unit: Noise over in-memory stream with WS framing. Full browser test requires wasm-pack + headless browser (defer to E2E) |
| 3 | Nodes discover each other via seed peers (static bootstrap) | Yes | MockTransport + MockPeer | Unit: BootstrapDiscovery resolves seeds, dials them, populates ClusterMembership |
| 4 | Nodes discover each other via mDNS on LAN (mesh-discovery) | Partial | None (needs real multicast) | Integration: two processes on localhost with mDNS. CI: skip unless multicast available, gate behind `#[ignore]` |
| 5 | Nodes discover each other via Kademlia DHT (mesh-discovery) | Yes | In-process Kademlia instances | Unit: three in-process DHT nodes, put/get service records. No real network needed |
| 6 | KernelMessage routes transparently via RemoteNode target | Yes | InMemoryTransport + MockPeer | Unit: send KernelMessage with RemoteNode target, verify it arrives at mock peer's A2ARouter |
| 7 | Remote messages pass through GovernanceGate before delivery | Yes | MockGovernanceEngine | Unit: send remote message, verify gate.evaluate() called. Test both Permit and Deny paths |
| 8 | Chain events replicate incrementally (tail_from) | Yes | None (pure data structure) | Unit: populate chain, call tail_from(seq), verify correct slice returned |
| 9 | Cross-node chain events carry dual signatures (Ed25519 + ML-DSA-65) | Yes | None (crypto is deterministic) | Unit: sign event with both algorithms, verify both signatures. Test rejection on bad signature |
| 10 | Bridge events anchor remote chain head hashes | Yes | None | Unit: create bridge event, verify it contains remote node_id, head_hash, head_seq |
| 11 | Resource tree synchronizes between nodes (Merkle root) | Yes | InMemoryTransport | Unit: two TreeManagers with different state, run sync, verify convergence via root hash equality |
| 12 | Remote tree mutations verified against Ed25519 signature | Yes | None | Unit: apply mutation with valid sig (pass), apply with invalid sig (reject), apply with missing sig (reject) |
| 13 | Services on any node discoverable from any other node | Yes | InMemoryTransport + two mock kernels | Integration: register service on node A, resolve from node B via mesh |
| 14 | Process advertisements gossip via CRDT distributed table | Yes | None (CRDT is pure) | Unit: create ProcessAdvertisement on two nodes, merge, verify convergence |
| 15 | Stopped nodes detected as Unreachable via SWIM heartbeats | Yes | MockTransport with drop capability | Unit: start heartbeat, simulate peer timeout (mock returns no pong), verify state -> Unreachable |
| 16 | All existing single-node tests pass unchanged | Yes | None | CI: run `scripts/build.sh test` without mesh feature. Must be gated in CI pipeline |
| 17 | mesh feature gate compiles to zero networking code when disabled | Yes | None | Build: `cargo check -p clawft-kernel` (no mesh feature), verify no quinn/snow symbols. Use `cargo build --features mesh` vs without and compare binary |
| 18 | Maximum message size (16 MiB) enforced at deserialization | Yes | None | Unit: send frame with payload > 16 MiB, verify MeshError::MessageTooLarge. Test exactly 16 MiB (pass) and 16 MiB + 1 (fail) |
| 19 | Message deduplication prevents double-delivery | Yes | None | Unit: send same message ID twice through dedup filter, verify second is dropped. Test bloom filter false-positive rate under load |
| 20 | Hybrid Noise + ML-KEM-768 handshake protects against SNDL | Yes | None (crypto) | Unit: complete hybrid handshake, verify final key differs from classical-only key. Verify KEM ciphertext decapsulates correctly |
| 21 | KEM negotiation degrades gracefully when unsupported | Yes | None | Unit: one side advertises kem_supported=false, verify handshake completes with classical-only key. No error |
| 22 | DHT keys namespaced with governance genesis hash prefix | Yes | None | Unit: construct DHT key, verify format `svc:<genesis[0..16]>:<name>`. Different genesis -> different key |
| 23 | Service resolution cache with TTL-based expiry | Yes | None (use mock clock) | Unit: resolve service (cached), advance clock past TTL, resolve again (cache miss triggers DHT lookup) |
| 24 | Negative cache prevents DHT storms for missing services | Yes | None (use mock clock) | Unit: resolve unknown service (DHT returns empty), immediately resolve again, verify no second DHT call. Advance past TTL, verify retry |
| 25 | Replicated services resolve with round-robin selection | Yes | None | Unit: register 3 replicas, resolve 6 times, verify each replica selected twice |
| 26 | Connection pool reuses Noise+QUIC channels | Yes | InMemoryTransport | Unit: dial peer twice, verify single connection in pool. Verify pool.get returns existing channel |
| 27 | Circuit breaker prevents cascade failures from slow nodes | Yes | MockTransport with configurable latency | Unit: trigger 6 consecutive failures, verify state -> OPEN. Wait cooldown, verify HALF-OPEN. Succeed -> CLOSED |
| 28 | RegistryQueryService exposes service resolution via ServiceApi | Yes | None | Unit: register RegistryQueryService, call via ServiceApi::call("registry", "resolve", ...), verify response |
| 29 | MeshAdapter dispatches incoming mesh messages through local A2ARouter | Yes | MockA2ARouter | Unit: feed KernelMessage into MeshAdapter, verify A2ARouter.send() called with correct message |
| 30 | mesh.request() supports correlated request-response with timeout | Yes | InMemoryTransport | Unit: send request, respond with matching correlation_id, verify response received. Test timeout path (no response -> error) |
| 31 | Remote service calls use same governance gate as local calls | Yes | MockGovernanceEngine | Unit: verify same GovernanceRequest construction for local and remote paths. Deny remote -> error |
| 32 | Chain log replication syncs events between mesh peers (K6.4) | Yes | InMemoryTransport | Integration: two chains with divergent sequences, run sync, verify convergence |
| 33 | Tree Merkle diff transfers only changed subtrees (K6.4) | Yes | None | Unit: two trees differing by 1 leaf, run diff, verify only the changed path transferred (count bytes or nodes) |
| 34 | QUIC stream priorities set per SyncStreamType (D15) | Partial | Requires quinn internals | Unit: verify set_priority() called with correct values per stream type. Integration: hard to verify priority behavior without congestion |
| 35 | Backpressure: chain checkpoint catch-up when >1000 events behind (D15) | Yes | InMemoryTransport | Unit: create chain with 1500 events, peer at seq 0, verify checkpoint mode activated instead of event-by-event |
| 36 | SyncStateDigest exchanged on stream open (D15) | Yes | InMemoryTransport | Unit: open sync stream between two mock peers, verify SyncStateDigest sent/received with correct fields |
| 37 | Sync frames use RVF wire segments with SyncStreamType discriminator (D15) | Yes | None | Unit: encode SyncFrame into RVF segment, decode, verify stream_type and payload_type roundtrip |
| 38 | PeerMetrics tracks observability dimensions for affinity scoring (D15) | Yes | None | Unit: feed success/failure/latency events into PeerMetrics, verify avg_rtt, success_rate, error_rate calculations |
| 39 | KEM upgrade completes before sync streams are opened (D15) | Yes | InMemoryTransport + state tracking | Unit: instrument connection setup, verify KEM upgrade runs before any sync stream open call. Verify streams inherit hybrid key |

### Summary

- **Fully auto-testable**: 34 of 39 (87%)
- **Partially auto-testable**: 5 of 39 (13%) -- criteria 2, 4, 17, 34 require special environments (browser, multicast, congestion simulation)
- **Not auto-testable**: 0

---

## Test Infrastructure Needed

### InMemoryTransport

The single most important test infrastructure piece. Without it, every mesh test requires actual QUIC connections, which are slow, flaky, and require TLS certificates.

```rust
/// A transport that operates entirely in-memory using tokio channels.
/// Implements MeshTransport, enabling all mesh logic to be tested
/// without real networking.
pub struct InMemoryTransport {
    /// Maps address -> sender for the "listening" side
    listeners: Arc<DashMap<String, mpsc::Sender<InMemoryStream>>>,
}

pub struct InMemoryStream {
    tx: mpsc::Sender<Vec<u8>>,
    rx: mpsc::Receiver<Vec<u8>>,
}

#[async_trait]
impl MeshTransport for InMemoryTransport {
    fn name(&self) -> &str { "in-memory" }

    async fn listen(&self, addr: &str) -> Result<TransportListener> {
        let (tx, rx) = mpsc::channel(32);
        self.listeners.insert(addr.to_string(), tx);
        Ok(TransportListener::InMemory(rx))
    }

    async fn connect(&self, addr: &str) -> Result<MeshStream> {
        let listener_tx = self.listeners.get(addr)
            .ok_or(MeshError::ConnectionRefused)?;
        let (a_tx, b_rx) = mpsc::channel(256);
        let (b_tx, a_rx) = mpsc::channel(256);
        listener_tx.send(InMemoryStream { tx: b_tx, rx: a_rx }).await?;
        Ok(MeshStream::InMemory(InMemoryStream { tx: a_tx, rx: b_rx }))
    }

    fn supports(&self, addr: &str) -> bool {
        addr.starts_with("mem://")
    }
}
```

**Key properties:**
- Deterministic -- no timing variability from OS network stack
- Fast -- channel send/recv is nanoseconds, not milliseconds
- Controllable -- can inject errors, delays, drops for fault testing
- No port conflicts in CI

**Extensions for fault injection:**
```rust
pub struct FaultyTransport {
    inner: InMemoryTransport,
    /// Drop every Nth message (0 = no drops)
    drop_rate: AtomicU32,
    /// Add artificial latency (milliseconds)
    latency_ms: AtomicU32,
    /// Fail next N connection attempts
    fail_next: AtomicU32,
}
```

### MockPeer

Simulates a remote WeftOS node without running a full kernel.

```rust
/// A lightweight simulated node for testing cross-node interactions.
/// Holds just enough state to participate in mesh protocols.
pub struct MockPeer {
    pub identity: NodeIdentity,
    pub transport: InMemoryTransport,
    pub chain: LocalChain,
    pub tree: TreeManager,
    pub services: Vec<ServiceEntry>,
    pub governance_genesis: [u8; 32],
    /// Received messages, for assertion
    pub inbox: Arc<Mutex<Vec<KernelMessage>>>,
}

impl MockPeer {
    /// Create a peer with a fresh identity and empty state.
    pub fn new(genesis: [u8; 32]) -> Self { /* ... */ }

    /// Create a peer with pre-populated chain events for sync testing.
    pub fn with_chain_events(genesis: [u8; 32], events: Vec<ChainEvent>) -> Self { /* ... */ }

    /// Create a peer that rejects all governance checks (for denied-path testing).
    pub fn deny_all(genesis: [u8; 32]) -> Self { /* ... */ }

    /// Create a peer with a different genesis (for cluster isolation testing).
    pub fn foreign_cluster() -> Self { /* ... */ }

    /// Run the peer's accept loop in background, returns JoinHandle.
    pub fn listen(&self, addr: &str) -> JoinHandle<()> { /* ... */ }
}
```

### MockClock

Required for testing TTL-based caches, heartbeat timeouts, and circuit breaker cooldowns without real-time waits.

```rust
/// Controllable clock for deterministic time-dependent tests.
pub struct MockClock {
    now: Arc<AtomicU64>,  // milliseconds since epoch
}

impl MockClock {
    pub fn new() -> Self { /* starts at 0 */ }
    pub fn advance(&self, duration: Duration) { /* atomic add */ }
    pub fn now(&self) -> Instant { /* read current value */ }
}
```

### Test Helpers

Common setup patterns to reduce boilerplate across K6 tests:

```rust
/// Create a pair of connected peers over InMemoryTransport,
/// with completed Noise handshake.
pub fn connected_pair() -> (MockPeer, MockPeer, NoiseChannel, NoiseChannel) {
    let transport = InMemoryTransport::new();
    let peer_a = MockPeer::new(GENESIS);
    let peer_b = MockPeer::new(GENESIS);
    // ... handshake ...
    (peer_a, peer_b, channel_a, channel_b)
}

/// Create N peers in a mesh, all connected to each other.
pub fn mesh_of(n: usize) -> Vec<MockPeer> { /* ... */ }

/// Create a NodeIdentity for testing (deterministic from seed).
pub fn test_identity(seed: u64) -> NodeIdentity { /* ... */ }

/// Create a signed chain event for replication testing.
pub fn signed_event(seq: u64, kind: &str, identity: &NodeIdentity) -> ChainEvent { /* ... */ }

/// Assert that two chains have identical events up to min(len_a, len_b).
pub fn assert_chains_converged(a: &LocalChain, b: &LocalChain) { /* ... */ }
```

**Recommended file location:** `crates/clawft-kernel/src/mesh_test_support.rs` gated behind `#[cfg(test)]`.

---

## Missing Test Scenarios

The exit criteria cover the "happy path" and some security boundaries, but omit several important failure and edge-case scenarios:

### 1. Connection Lifecycle Edge Cases
- **Reconnection after disconnect**: peer drops, reconnects, verify state recovery
- **Concurrent dial from both sides**: A dials B while B dials A simultaneously -- should result in one connection, not two
- **Connection pool exhaustion**: max connections reached, verify new dial is queued or rejected gracefully
- **Idle connection eviction**: verify connections unused for 60s are closed and cleaned from pool

### 2. Handshake Failure Modes
- **Noise handshake timeout**: peer sends first message but never responds -- verify timeout fires
- **Corrupted handshake payload**: garbage bytes after Noise decryption -- verify clean error, no panic
- **Wrong static key**: peer uses unexpected Ed25519 key -- verify rejection
- **Replay of old handshake**: replay captured handshake bytes -- verify rejection (Noise nonces prevent this, but test it)

### 3. Message Routing Edge Cases
- **Message to disconnected node**: send to RemoteNode when peer is not connected -- verify error propagated to caller
- **Message to self**: RemoteNode with own node_id -- should short-circuit to local delivery
- **Message during governance rule update**: governance rules change mid-flight -- verify consistent evaluation
- **Zero-length payload**: KernelMessage with empty payload -- verify framing handles it

### 4. Chain Sync Edge Cases
- **Empty chain sync**: new node joins with empty chain -- verify full sync works
- **Chain fork detection**: two nodes extended independently after partition -- verify bridge events created
- **Corrupted chain event**: event with invalid hash chain -- verify rejection, no corruption of local chain
- **Large chain catch-up**: 10,000+ events behind -- verify checkpoint mode activates and completes

### 5. Tree Sync Edge Cases
- **Empty tree sync**: one node has empty tree -- verify full snapshot transfer
- **Conflicting mutations**: same tree path mutated on both nodes -- verify conflict resolution strategy
- **Tree sync during active mutations**: local mutations happening while sync is running -- verify no corruption

### 6. Discovery Edge Cases
- **All seed peers unreachable**: every bootstrap address fails -- verify graceful degradation, periodic retry
- **DHT poisoning**: malicious peer advertises services it does not host -- verify circuit breaker catches it
- **Rapid peer churn**: peers joining and leaving every few seconds -- verify membership table stays consistent

### 7. Performance Edge Cases (not in exit criteria)
- **Throughput under load**: N concurrent cross-node messages per second -- what is the ceiling?
- **Memory usage with many peers**: 50+ connected peers -- verify no unbounded growth
- **Bloom filter saturation**: dedup filter after 1M messages -- verify false positive rate acceptable

### 8. Security Edge Cases (not fully covered)
- **Unauthorized ServiceApi call**: remote node calls a service method that requires elevated capability
- **Forged GlobalPid**: remote message claims to be from a PID that does not exist on the source node
- **Rate limiting on join**: 100 rapid join requests from same address -- verify rate limiting

---

## Manual Testing Guide Additions

The following steps should be added to `manual-testing-guide.md` as a new **Pass 6: Mesh Networking**.

### Pass 6: Mesh Networking (10 min)

Requires two terminal sessions and the `mesh` feature enabled.

#### 6.0 Build with Mesh Feature
```bash
scripts/build.sh native --features mesh
```
- [ ] **Expected**: Build succeeds with mesh dependencies (quinn, snow)

#### 6.1 Start Node A
```bash
weaver kernel start --bind quic://127.0.0.1:9001
```
- [ ] **Expected**: "Mesh listener started on quic://127.0.0.1:9001"

#### 6.2 Start Node B with Seed Peer
```bash
# In second terminal
weaver kernel start --bind quic://127.0.0.1:9002 --seed-peer quic://127.0.0.1:9001
```
- [ ] **Expected**: "Connected to seed peer", "Noise handshake complete"

#### 6.3 Verify Cluster Membership
```bash
# On either node
weaver cluster peers
```
- [ ] **Expected**: Two nodes listed, both state "Connected"

#### 6.4 Cross-Node Service Discovery
```bash
# On Node B
weaver service resolve <service-name-on-A>
```
- [ ] **Expected**: Service resolved with remote node_id and PID

#### 6.5 Cross-Node Message
```bash
# On Node A, spawn an agent
weaver agent spawn mesh-test-agent
# On Node B, send message to agent on Node A
weaver agent send --node <node-A-id> <pid> "hello across mesh"
```
- [ ] **Expected**: Message delivered, chain event logged on both nodes

#### 6.6 Chain Sync Verification
```bash
# On Node B
weaver chain local | head -5
```
- [ ] **Expected**: Boot events from Node A visible (replicated)

#### 6.7 Node Failure Detection
```bash
# Stop Node A (Ctrl+C or weaver kernel stop)
# Wait 15 seconds
# On Node B
weaver cluster peers
```
- [ ] **Expected**: Node A shows state "Unreachable"

#### 6.8 Cleanup
```bash
weaver kernel stop  # on all running nodes
```
- [ ] **Expected**: Clean shutdown on all nodes

### Pass 7: Mesh Security (5 min)

#### 7.1 Foreign Cluster Rejection
```bash
# Start Node C with different genesis
weaver kernel start --bind quic://127.0.0.1:9003 --genesis-override test-foreign
```
Attempt to connect to existing cluster:
```bash
weaver cluster join quic://127.0.0.1:9001
```
- [ ] **Expected**: "Connection rejected: genesis mismatch"

#### 7.2 KEM Handshake Verification
```bash
weaver cluster peers --verbose
```
- [ ] **Expected**: Each peer shows "encryption: hybrid-noise-kem" or "encryption: noise-classical" with KEM status

---

## Risk Assessment

### Highest Risk: Real Network Dependencies in CI

The criteria that are hardest to test without real infrastructure:

1. **mDNS discovery (criterion 4)**: Requires multicast networking, which many CI runners and containers do not support. This will likely be a persistent `#[ignore]` test that only runs in specific environments. Risk: regressions slip through because the test rarely runs.

2. **QUIC with real quinn (criterion 1)**: While testable on localhost, QUIC involves UDP which some CI environments block or behave differently on. The InMemoryTransport mitigates this for unit tests, but integration tests need real QUIC to verify quinn configuration (certificate handling, connection migration).

3. **Browser WebSocket (criterion 2)**: Requires wasm-pack, a headless browser, and a running WS relay. This is a full E2E setup. Recommendation: defer browser E2E tests to a dedicated CI job and rely on InMemoryTransport for the WS protocol logic.

### Medium Risk: Timing-Dependent Tests

4. **SWIM heartbeat detection (criterion 15)**: Depends on timeouts. Using real time makes tests slow and flaky. The MockClock approach mitigates this, but the implementation must be designed to accept an injectable clock from the start -- retrofitting is painful.

5. **Circuit breaker state transitions (criterion 27)**: Same timing concern. The 30-second cooldown must be testable with a mock clock.

6. **TTL-based caches (criteria 23, 24)**: Must use injectable time source, not `Instant::now()`.

**Recommendation**: Define a `Clock` trait early in K6.0 and inject it into all time-dependent components.

### Low Risk but High Impact if Missed

7. **Feature gate isolation (criterion 17)**: If mesh code leaks into the non-mesh build, it pulls in quinn/snow/x25519-dalek as mandatory dependencies, breaking WASM targets. This should be caught by CI (`cargo check --target wasm32-unknown-unknown` without mesh feature), but a single missing `#[cfg(feature = "mesh")]` gate breaks everything.

8. **Chain integrity during sync (criterion 32)**: A bug in chain replication could corrupt the local chain. The chain's hash-chain verification should catch this, but the test must verify that a corrupted replicated event is rejected WITHOUT modifying any existing local chain state.

### What Might Slip Through

- **Memory leaks from connection pool**: Connections that are never cleaned up under specific failure patterns (e.g., handshake fails after stream is allocated). Need explicit pool cleanup tests.
- **Bloom filter false positives**: The dedup bloom filter will eventually saturate. The plan does not specify a rotation strategy. After enough messages, legitimate messages could be silently dropped.
- **Governance rule synchronization**: If governance rules differ between nodes (e.g., during a rule update rollout), remote calls may be inconsistently permitted/denied. The plan does not specify how governance rule updates propagate.
- **Concurrent chain sync and local append**: If a node is appending local events while also receiving replicated events from chain sync, ordering and sequence numbering must be thread-safe. This race condition is subtle and may not surface in single-threaded tests.

### Recommendation Priority

| Priority | Action | Phase |
|----------|--------|-------|
| P0 | Implement InMemoryTransport + MockPeer before any mesh code | K6.0 |
| P0 | Define Clock trait for injectable time | K6.0 |
| P0 | Add `cargo check --target wasm32-unknown-unknown` (no mesh) to CI | K6.0 |
| P1 | Add FaultyTransport for fault injection | K6.1 |
| P1 | Write Noise handshake failure mode tests alongside implementation | K6.1 |
| P1 | Define bloom filter rotation/reset strategy for dedup | K6.3 |
| P2 | Add concurrent chain sync + local append stress test | K6.4 |
| P2 | Add connection pool leak detection test | K6.5 |
| P3 | Set up browser E2E CI job | Post-K6 |
