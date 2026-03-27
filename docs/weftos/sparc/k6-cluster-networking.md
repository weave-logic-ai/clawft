# K6 SPARC Specification: Cluster Networking

**Phase**: K6 — Multi-Node Cluster Networking
**Status**: Specification (pre-implementation)
**Depends on**: K5 (Application Marketplace + Discovery)
**Gate from**: K2 C10

## Overview

K6 extends WeftOS from a single-node kernel into a multi-node cluster with
peer-to-peer networking, distributed process scheduling, and cross-node IPC.
The foundation is the `ClusterMembership` module (K0) and the optional
`ruvector-cluster` integration (K2).

## Architecture

### Network Topology

```
 Node A (leader)          Node B (worker)         Node C (worker)
 ┌───────────────┐        ┌───────────────┐       ┌───────────────┐
 │  Kernel       │◄──────►│  Kernel       │◄─────►│  Kernel       │
 │  ├ ProcessTab │  gRPC  │  ├ ProcessTab │ gRPC  │  ├ ProcessTab │
 │  ├ ServiceReg │  /QUIC │  ├ ServiceReg │ /QUIC │  ├ ServiceReg │
 │  ├ ChainMgr   │        │  ├ ChainMgr   │       │  ├ ChainMgr   │
 │  └ ClusterSvc │        │  └ ClusterSvc │       │  └ ClusterSvc │
 └───────────────┘        └───────────────┘       └───────────────┘
         │                         │                       │
         └─────────────── Raft consensus ──────────────────┘
```

### Transport Layer

| Protocol | Use Case | Latency Target |
|----------|----------|----------------|
| QUIC (ruvector-cluster) | Peer discovery, heartbeats | <10ms |
| gRPC | Cross-node IPC, process migration | <50ms |
| ExoChain RVF | Chain sync, state anchoring | Best-effort |

### Component Interfaces

#### ClusterNetworkService

```rust
#[async_trait]
pub trait ClusterNetworkService: SystemService {
    /// Join the cluster with the given seed nodes.
    async fn join(&self, seeds: &[SocketAddr]) -> Result<(), ClusterError>;

    /// Leave the cluster gracefully.
    async fn leave(&self) -> Result<(), ClusterError>;

    /// Route a message to a remote node's IPC.
    async fn route_remote(
        &self,
        target_node: &NodeId,
        message: KernelMessage,
    ) -> Result<(), ClusterError>;

    /// Request process migration to another node.
    async fn migrate_process(
        &self,
        pid: Pid,
        target_node: &NodeId,
    ) -> Result<Pid, ClusterError>;
}
```

#### DistributedProcessTable

```rust
pub trait DistributedProcessTable {
    /// Locate which node owns a given PID.
    fn locate(&self, pid: Pid) -> Option<NodeId>;

    /// Allocate a PID on the least-loaded node.
    fn allocate_remote(&self) -> Result<(NodeId, Pid), ClusterError>;

    /// Sync process state from a peer node.
    fn sync_from(&self, node: &NodeId, entries: Vec<ProcessEntry>);
}
```

#### CrossNodeIpc

```rust
pub trait CrossNodeIpc {
    /// Send a message to a process on any node.
    fn send_cross_node(
        &self,
        target: MessageTarget,
        message: KernelMessage,
    ) -> Result<(), KernelError>;

    /// Subscribe to messages from remote processes.
    fn subscribe_remote(
        &self,
        source_node: &NodeId,
        topic: &str,
    ) -> Result<Subscription, KernelError>;
}
```

## Governance Gates

All K6 operations pass through the governance engine:

| Operation | Gate Rule | Effect Vector |
|-----------|-----------|---------------|
| `cluster.join` | Require judicial + legislative | network: 0.8 |
| `cluster.leave` | Require executive | network: 0.6 |
| `process.migrate` | Require legislative | compute: 0.5, network: 0.4 |
| `ipc.cross_node` | Require executive (auto) | network: 0.2 |
| `chain.sync` | Require judicial | integrity: 0.7, network: 0.3 |

## K5 / K6 Boundary

| Capability | K5 | K6 |
|------------|----|----|
| App discovery & install | Yes | Inherited |
| Single-node services | Yes | Inherited |
| Peer-to-peer networking | No | Yes |
| Cross-node IPC | No | Yes |
| Process migration | No | Yes |
| Distributed chain sync | No | Yes |
| Multi-node scheduling | No | Yes |

## Implementation Phases

### K6.1: Transport + Discovery (~400 lines)
- QUIC transport via ruvector-cluster
- Seed node discovery and heartbeat protocol
- `ClusterNetworkService` implementation
- Integration with existing `ClusterMembership`

### K6.2: Cross-Node IPC (~300 lines)
- gRPC message serialization for `KernelMessage`
- Remote message routing through `A2ARouter`
- `CrossNodeIpc` trait implementation
- Topic subscription forwarding

### K6.3: Distributed Process Table (~250 lines)
- PID allocation with node prefix (e.g., `node_id << 48 | local_pid`)
- Process state synchronization via Raft
- Load-aware scheduling heuristic
- Process migration protocol

### K6.4: Chain Synchronization (~200 lines)
- RVF segment exchange between nodes
- Chain fork detection and resolution
- `ChainAnchor` implementation for cluster root chain
- Merkle proof generation for cross-node verification

## Verification

```bash
# Unit tests
cargo test -p clawft-kernel --features native,exochain,cluster -- cluster_network

# Integration test (requires 2+ nodes)
cargo test -p clawft-kernel --features native,exochain,cluster -- integration::cluster

# Performance (latency targets)
cargo bench -p clawft-kernel --features native,cluster -- cluster_latency
```

## Dependencies

- `ruvector-cluster` (QUIC transport, Raft consensus)
- `tonic` / `prost` (gRPC for cross-node IPC)
- `rvf-wire` + `rvf-crypto` (chain sync)
- K0-K5 kernel modules (all inherited)
