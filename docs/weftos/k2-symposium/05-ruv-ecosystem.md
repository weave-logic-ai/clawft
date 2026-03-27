# RUV Ecosystem Audit

**Presenter**: RUV Ecosystem Expert
**Scope**: Full inventory of ruvector/ruvnet ecosystem crates

---

## Executive Summary

The ruv ecosystem contains **143 production-ready crates** across 7
repositories. WeftOS currently uses **4 fully** and **3 partially**,
for a utilization rate of **2.8% full / 4.9% total**.

The foundation is compatible with ruv's vision, but K2-K6 should
systematically adopt available crates rather than building from scratch.

## Utilization Scorecard

| Category | Available | Fully Used | Partially Used | Unused |
|----------|-----------|------------|----------------|--------|
| RuVector Crates | 101 | 3 | 3 | 95 |
| Exochain Crates | 6 | 1 | 0 | 5 |
| Agentic-Flow | 15 | 0 | 0 | 15 |
| Ruflo | 10 | 0 | 0 | 10 |
| DAA | 6 | 0 | 0 | 6 |
| QuDAG | 4 | 0 | 0 | 4 |
| AI-SDLC | 1 | 0 | 0 | 1 |
| **Total** | **143** | **4** | **3** | **136** |

## What WeftOS Currently Uses

### Fully Used (4 crates)

**rvf-crypto**: SHAKE-256 witness chains, Ed25519 signing
```rust
use rvf_crypto::hash::shake256_256;
use rvf_crypto::{create_witness_chain, sign_segment, verify_segment};
```

**rvf-types**: Type definitions, ExoChainHeader, segment constants
```rust
use rvf_types::{ExoChainHeader, EXOCHAIN_MAGIC, SEGMENT_HEADER_SIZE};
```

**rvf-wire**: Segment reading, padding calculation, event serialization
```rust
use rvf_wire::writer::{calculate_padded_size, write_exochain_event};
use rvf_wire::{decode_exochain_payload, read_segment, validate_segment};
```

**exo-resource-tree**: Merkle-hashed resource namespace (local fork)
```rust
use exo_resource_tree::{MutationEvent, MutationLog, NodeScoring, ResourceTree};
```

### Partially Used (3 crates)

**rvf-crypto** (additional unused features):
- ML-DSA-65 dual-key signing (post-quantum)
- verify_witness_chain() -- only creation used
- Lineage record serialization -- imported but not called

**cognitum-gate-tilezero** (behind tilezero feature):
- Uses check_action() three-way decisions
- Unused: merge_supergraph(), replay_decision(), evidence_weight()

**ruvector-cluster** (behind cluster feature):
- Uses types only (ClusterConfig, ClusterMembership)
- Unused: ClusterManager, ConsistentHashRing, DiscoveryService

## High-Priority Integration Targets

### K1-K2 Immediate Wins

| Crate | Phase | Value | Effort |
|-------|-------|-------|--------|
| ruvector-tiny-dancer-core | K0/K1 | Sub-ms agent routing via FastGRNN | Medium |
| mcp-gate | K1 | Expose capabilities to Claude Desktop/VS Code | Low |
| cognitum-gate-kernel | K0 | Kernel-level audit verification | Low |

**ruvector-tiny-dancer-core**: Provides FastGRNN-based semantic routing
with <1ms latency. Self-learning dispatch that improves routing over
time. Circuit breaker for fault tolerance.

**mcp-gate**: MCP server exposing coherence gate as tools. Wraps
existing gate.rs with MCP protocol for external consumption.

### K2 IPC Enhancement

| Crate | Phase | Value | Effort |
|-------|-------|-------|--------|
| exo-core (HLC) | K2 | O(1) causal ordering for IPC | Medium |
| ruvector-delta-consensus | K2 | CRDT-based pub/sub, conflict-free | High |
| ruvector-nervous-system | K2 | Bio-inspired routing, budget guardrails | Medium |

**exo-core**: Hybrid Logical Clocks provide causal ordering without
vector clock overhead. Better than timestamps for K2 IPC message ordering.

**ruvector-nervous-system**: Bio-inspired routing with oscillatory
patterns, ShardedEventBus for high-throughput event delivery, and
BudgetGuardrail for agent resource limits.

### K3-K4 Runtime

| Crate | Phase | Value | Effort |
|-------|-------|-------|--------|
| ruvector-snapshot | K3/K4 | Point-in-time state snapshots | Medium |
| RVF segments (WASM_SEG) | K4 | Self-describing containers | High |
| ruvector-cognitive-container | K0 ref | Epoch-budgeted execution pattern | Low |

### K5 Learning

| Crate | Phase | Value | Effort |
|-------|-------|-------|--------|
| sona | K5 | Self-optimizing neural architecture | High |
| prime-radiant | K1/K5 | Topological coherence, hallucination detection | Medium |

**sona**: LoRA adapters, EWC++ for catastrophic forgetting prevention,
ReasoningBank for pattern learning, trajectory-based self-improvement.

### K6 Distributed

| Crate | Phase | Value | Effort |
|-------|-------|-------|--------|
| exo-dag | K6 | DAG-based audit trail, MMR proofs | High |
| exo-identity | K6 | W3C DIDs for persistent agent identity | Medium |
| exo-consent | K6 | Bailment contracts, Gatekeeper trait | Medium |
| ruvector-verified | K6 | Proof-gated mutations | High |

## Compatibility Assessment

### Aligned

- Witness chains and cryptographic audit (rvf-crypto)
- Three-way capability decisions (cognitum-gate-tilezero)
- Resource tree hierarchies (exo-resource-tree)
- Consensus preparation (ruvector-raft feature-gated)

### Divergent but Compatible

- IPC lacks causal ordering (should use HLC or delta-consensus)
- Routing is basic (should use nervous-system or tiny-dancer)
- No learning infrastructure (should use sona for K5)
- Basic health checks (should use prime-radiant coherence)

### Missing Critical Capabilities

- No intelligent agent routing (tiny-dancer available)
- No snapshot infrastructure (ruvector-snapshot available)
- No MCP integration (mcp-gate available)
- No cognitive container pattern (ruvector-cognitive-container available)

## Unused Capabilities in Partially-Used Crates

### rvf-crypto

Not using: DualKey (ML-DSA-65 post-quantum), verify_witness_chain(),
lineage_record_to_bytes(), lineage_witness_entry()

### cognitum-gate-tilezero

Not using: merge_supergraph(), replay_decision(), evidence_weight(),
PermitToken serialization, WitnessReceipt chain verification

### ruvector-cluster

Not using: ClusterManager, ConsistentHashRing, DiscoveryService,
health_checks (peer monitoring)

## Recommendations

### Phase-Aligned Integration Plan

1. **K1 (now)**: tiny-dancer-core, mcp-gate, cognitum-gate-kernel
2. **K2 (now)**: nervous-system, delta-consensus, exo-core HLC
3. **K3/K4 (next)**: ruvector-snapshot, RVF format consideration
4. **K5 (future)**: sona, prime-radiant
5. **K6 (future)**: exo-dag, exo-identity, ruvector-verified, AEGIS CGR

### Quick Wins (< 1 week each)

- Enable verify_witness_chain() (already imported, just not called)
- Enable lineage verification (imported, not called)
- Adopt cognitum-gate-kernel for periodic audit verification
- Wrap gate.rs with mcp-gate for external tool exposure

### Strategic Decision

The ruv ecosystem provides production-ready implementations of nearly
everything WeftOS plans to build for K3-K6. Adopting these crates
accelerates development while ensuring compatibility with the broader
ruv vision. The question is: build custom or integrate existing?
