# ADR-040: LWW-CRDT for Distributed Process Table

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: K5 Symposium Panel 1 (Mesh Architecture), K6 Implementation

## Context

WeftOS K6 introduces a distributed process table (`DistributedProcessTable` in `crates/clawft-kernel/src/mesh_process.rs`) that tracks processes running across all mesh nodes. Each node advertises its processes via `ProcessAdvertisement` structs carrying `GlobalPid`, `agent_type`, `capabilities`, `services`, `ProcessStatus` (Running/Suspended/Stopping/Unreachable), `last_updated` timestamp, and an optional `ResourceSummary` (memory_bytes, cpu_time_us, inbox_depth).

When two nodes independently update state for the same process -- for example, one node marks it Suspended while another records new resource metrics -- the table must converge without coordination. The ECC Symposium (D4) established the principle "CRDTs for convergence, Merkle for verification." The K6 implementation needed a specific CRDT strategy for the process table.

Three CRDT families were considered: LWW-Register (timestamp-based), OR-Set (add/remove semantics), and MV-Register (preserve all concurrent values). The process table has a natural "latest state wins" semantic -- a process is either Running or Suspended right now, and the most recent observation is the most accurate.

## Decision

The `DistributedProcessTable` uses Last-Writer-Wins (LWW) CRDT semantics for conflict-free merging. The `merge()` method compares `last_updated` timestamps; the entry with the higher timestamp wins. Ties are broken by keeping the local value (local bias). A companion `CrdtGossipState` struct provides delta-based gossip with logical clocks, where `merge()` applies the same LWW rule: higher clock wins, ties keep local.

The `ClusterServiceRegistry` (`mesh_service_adv.rs`) uses the same LWW strategy, keyed by `(service_name, node_id)` pairs.

A `ConsistentHashRing` (virtual-node-based) provides PID-to-node assignment for deterministic process placement. A `MetadataConsensus` module provides Raft-style log replication for authoritative operations (RegisterService, DeregisterService, RegisterProcess, DeregisterProcess) that require strong consistency rather than eventual convergence.

## Consequences

### Positive
- Simple implementation: `merge()` is a single timestamp comparison per entry, O(1) per advertisement
- Convergence is guaranteed as long as clocks are monotonically increasing on each node
- Delta gossip (`delta_since(clock)`) minimizes bandwidth by sending only entries newer than the peer's known clock
- `least_loaded_node()`, `find_by_type()`, and `find_by_capability()` queries work naturally over the merged state
- Compatible with `ruvector_delta_consensus` for a production CRDT backend when Phase 2 wires actual networking

### Negative
- Wall-clock sensitivity: if node A's clock is skewed forward, its stale updates will overwrite node B's correct updates until clocks converge
- No conflict visibility: concurrent status changes are silently resolved by timestamp, making debugging distributed races difficult
- The `Unreachable` status set by `mark_node_unreachable()` can be overwritten by a delayed advertisement from the supposedly-unreachable node
- Logical clocks in `CrdtGossipState` mitigate wall-clock skew but introduce a separate clock domain from the `last_updated` field on `ProcessAdvertisement`

### Neutral
- The choice of LWW over OR-Set means process "add" and "remove" are not distinguished at the CRDT level; `remove_node()` is an imperative purge, not a CRDT tombstone
- The `MetadataConsensus` Raft module provides a strong-consistency path for operations where LWW is insufficient, creating a dual-consistency architecture (eventual for gossip, strong for registry mutations)
