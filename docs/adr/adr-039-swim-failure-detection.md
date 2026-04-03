# ADR-039: SWIM Protocol for Mesh Failure Detection

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K6 Development (mesh_heartbeat.rs implementation), K6.5 mesh framework design

## Context

In a multi-node WeftOS mesh, nodes can fail or become unreachable due to crashes, network partitions, or resource exhaustion. The system needs a failure detection mechanism that is scalable (constant per-node network overhead), distributed (no single failure detector), and provides tunable sensitivity (tradeoff between detection speed and false positive rate).

Traditional heartbeat approaches (all-to-all pinging) have O(n^2) network overhead. Gossip-based approaches detect failures slowly. The SWIM protocol (Scalable Weakly-consistent Infection-style process group Membership) provides O(1) per-node probing overhead with bounded false positive rates via indirect probing through witness nodes.

K6.5 implemented the SWIM-based failure detector in `crates/clawft-kernel/src/mesh_heartbeat.rs`, gated behind `#[cfg(feature = "mesh")]`.

## Decision

Peer liveness detection uses a simplified SWIM protocol implemented in the `HeartbeatTracker` struct with the following components:

**State machine** (`HeartbeatState` enum):
- `Alive` -- node is healthy, responding to pings
- `Suspect` -- node missed a direct ping, awaiting indirect probe results
- `Dead` -- node confirmed unreachable (indirect probes also failed or suspect timeout elapsed)

Transitions: `Alive -> Suspect` (on first missed ping), `Suspect -> Dead` (when `suspect_timeout` expires), `Suspect -> Alive` or `Dead -> Alive` (on successful ping response via `record_alive()`).

**Per-peer state** (`PeerHeartbeat` struct):
- `state: HeartbeatState` -- current health state
- `last_seen: Instant` -- last successful heartbeat time
- `suspect_since: Option<Instant>` -- when suspect state was entered
- `missed_count: u32` -- consecutive missed pings
- `last_ping_seq: u64` -- sequence number for correlation

**Configuration** (`HeartbeatConfig`):
- `probe_interval`: 1 second (how often to probe a random peer)
- `ping_timeout`: 500ms (direct ping response deadline)
- `indirect_timeout`: 1 second (indirect probe response deadline)
- `indirect_witnesses`: 3 (number of peers asked to relay indirect probes)
- `suspect_timeout`: 5 seconds (time in Suspect before declaring Dead)

**Wire messages**:
- `PingRequest` -- carries `from_node`, `sequence` (for correlation), `indirect` flag, and optional `on_behalf_of` (original requester for indirect probes)
- `PingResponse` -- carries `from_node`, `sequence`, `process_count`, and `service_count` (piggybacked cluster state)

**Deduplication**: The mesh dedup filter (`mesh_dedup.rs`, `DedupFilter`) prevents duplicate ping/response processing with a default TTL of 60 seconds and 10K entry capacity.

**Observability**: `PeerMetrics` tracks per-peer RTT (exponential moving average, `rtt = 0.8*old + 0.2*new`), message counts, byte counts, and error counts. The `affinity_score()` method (`rtt_ms + error_rate * 1000.0`) is used by service resolution (`mesh_service.rs`) to prefer lower-latency, lower-error peers.

**Unreachability heuristic**: A peer is considered unreachable if its state is `Dead` OR if `last_seen` exceeds `3 * suspect_timeout` (15 seconds by default), providing a safety net independent of the state machine.

## Consequences

### Positive
- SWIM's O(1) per-node probing overhead scales to large clusters without increasing network traffic per node
- Indirect probing (via 3 witness nodes) reduces false positives from transient network issues -- a node is not declared Dead until both direct and indirect probes fail
- Piggybacked `process_count` and `service_count` in `PingResponse` provides cluster state information at zero additional network cost
- Configurable timeouts allow tuning the detection speed vs. false positive tradeoff per deployment environment (e.g., tighter timeouts for LAN, looser for WAN)
- `PeerMetrics` and `affinity_score()` enable intelligent service routing based on observed network quality

### Negative
- SWIM's probabilistic failure detection means false positives are possible under sustained network partitions -- a healthy node behind a temporary partition may be declared Dead and trigger unnecessary service migration, chain replication handoff, and distributed process table cleanup
- The `Suspect -> Dead` transition uses wall-clock `Instant` comparison, which is monotonic but not synchronized across nodes -- different nodes may declare the same peer Dead at different times
- The simplified implementation does not yet include SWIM's protocol extensions (suspicion subprotocol with incarnation numbers, protocol period dissemination via piggyback) which would further reduce false positive rates

### Neutral
- The state machine is tested with unit tests covering `Alive -> Suspect`, `Suspect -> Dead` (with zero-duration suspect timeout), and `Alive -> Alive` (on successful response) transitions
- `PingRequest`/`PingResponse` use serde for serialization, enabling transport over any `MeshTransport` implementation (QUIC, WebSocket, etc.)
- Dead peer detection cascades to `mesh_process.rs` (`DistributedProcessTable`), `mesh_service_adv.rs` (`ClusterServiceRegistry::remove_node()`), and chain replication (`mesh_chain.rs`) -- these downstream effects are managed by their respective subsystems
