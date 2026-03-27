# 14 — ExoChain Cryptographic Substrate

> SPARC Addendum — WeftOS Kernel Sprint
> Phase: K0 (foundation), K1+ (global chain, consensus)

## 1. Overview

ExoChain provides the cryptographic audit layer for WeftOS. Every kernel
event, service registration, and cluster operation is recorded in an
append-only hash-linked chain, providing tamper-evident logging and
enabling future cross-node verification.

### 1.1 Design Rationale

WeftOS needs an immutable audit trail that:
- Records all state-changing operations (boot, service lifecycle, peer joins)
- Links events cryptographically (SHA-256 hash chain)
- Supports checkpointing for efficient restart
- Embeds into RVF segments for persistent storage
- Scales to multi-node consensus (K6)

### 1.2 Relationship to ExoChain Project

The ExoChain project (github.com/exochain/exochain) provides DAG-based
event sourcing primitives. WeftOS maintains a fork at
weave-logic-ai/exochain with WASM compatibility fixes:
- `serde_cbor` → `ciborium` (maintained, WASM-safe)
- Feature gates on native-only dependencies (libp2p, graphql)

## 2. Architecture

### 2.1 Chain Hierarchy

| Chain | ID | Scope | Phase |
|-------|-----|-------|-------|
| Local | 0 | Single node event log | K0 |
| Global Root | 1 | Cross-node anchoring | K6 |
| App Chains | 2+ | Per-application audit | K6+ |

### 2.2 ChainManager (K0)

`crates/clawft-kernel/src/chain.rs` — thread-safe chain manager:
- Genesis event at boot with chain_id=0
- `append(source, kind, payload)` → ChainEvent with SHA-256 linking
- Auto-checkpoint after configurable interval
- Status reporting for CLI/RPC

### 2.3 ChainEvent Structure

```
ChainEvent {
  sequence: u64,      // Monotonic counter
  chain_id: u32,      // 0=local, 1=global, 2+=app
  timestamp: DateTime, // UTC
  prev_hash: [u8; 32], // SHA-256 of previous event
  hash: [u8; 32],      // SHA-256 of this event
  source: String,      // e.g. "kernel", "service.cron"
  kind: String,        // e.g. "boot", "peer.join"
  payload: Option<Value>, // JSON payload
}
```

## 3. RVF Segment Embedding

ExoChain data is embedded in RVF files using three segment types:

| Type | Discriminant | Purpose |
|------|-------------|---------|
| ExochainEvent | 0x40 | Individual chain event |
| ExochainCheckpoint | 0x41 | State snapshot hash |
| ExochainProof | 0x42 | Merkle inclusion proof |

Each segment has a 64-byte `ExoChainHeader` prefix followed by
CBOR-encoded payload data.

### 3.1 ExoChainHeader (64 bytes, repr(C))

```
Offset  Size  Field
0x00    4     magic ("EXOC")
0x04    1     version (1)
0x05    1     subtype (0x40/0x41/0x42)
0x06    2     flags (reserved)
0x08    4     chain_id
0x0C    4     _reserved (alignment)
0x10    8     sequence
0x18    8     timestamp_secs
0x20    32    prev_hash
```

## 4. Boot Integration

During kernel boot (when exochain feature enabled):
1. ChainManager created with genesis event
2. Boot phases logged to chain: `boot.init`, `boot.config`, `boot.services`
3. TreeManager created wrapping ResourceTree + MutationLog + ChainManager
4. TreeManager bootstraps tree; emits `tree.bootstrap` chain event with node count + root hash
5. CronService registered via TreeManager; emits `tree.insert` chain event with `chain_seq` metadata
6. Cluster and ready events logged: `boot.cluster`, `boot.ready` (includes tree_root_hash)
7. On shutdown: `kernel.shutdown` chain event + chain checkpoint

### 4.1 TreeManager Facade

`crates/clawft-kernel/src/tree_manager.rs` — unified wrapper ensuring every tree
mutation produces both a `MutationEvent` and a `ChainEvent`:

```rust
pub struct TreeManager {
    tree: Mutex<ResourceTree>,
    mutation_log: Mutex<MutationLog>,
    chain: Arc<ChainManager>,
}
```

Methods: `bootstrap()`, `insert()`, `remove()`, `update_meta()`,
`register_service()`, `checkpoint()`, `root_hash()`, `stats()`.

Each mutation atomically: modifies tree → appends chain event → stores `chain_seq`
metadata on the tree node → appends MutationEvent to log.

### 4.2 Chain Integrity Verification

`ChainManager::verify_integrity()` walks all events and:
- Recomputes each event's SHA-256 hash from (sequence, chain_id, prev_hash, source, kind, timestamp)
- Verifies each event's `hash` matches the recomputed value
- Verifies each event's `prev_hash` matches the prior event's `hash`
- Returns `ChainVerifyResult { valid, event_count, errors }`

Available via `weaver chain verify` CLI command.

## 5. Configuration

```json
{
  "kernel": {
    "chain": {
      "enabled": true,
      "checkpointInterval": 1000,
      "chainId": 0
    },
    "resourceTree": {
      "enabled": true,
      "checkpointPath": null
    }
  }
}
```

## 6. Future Phases

### K1: Permission Chain Events
- Record permission check results in chain
- DelegationCert grant/revoke events

### K6: Global Root Chain
- BridgeEvent cross-chain anchoring
- ruvector-raft consensus for global chain
- Multi-node chain synchronization

### K6+: App Chains
- Per-application audit chains (chain_id >= 2)
- App-scoped event isolation

## 7. Exochain Fork Strategy

| Original | Fork Change | Reason |
|----------|------------|--------|
| serde_cbor | ciborium | Maintained, WASM-safe |
| libp2p (exo-api) | Feature-gated | Native-only dep |
| graphql (exo-api) | Feature-gated | Native-only dep |
| exo-dag | std feature gate | no_std compatibility |

## 8. DAA Rules Engine (K1+)

The Dynamic Agent Allocation (DAA) rules engine will serve as the
policy backend for chain event validation. Chain events pass through
DAA policy filters before being committed, enabling governance-aware
audit trails.
