# Swarm topology (WEFT-199)

## Decision

**Document mesh / hierarchical / adaptive as claude-flow prompt-only.**
WeftOS `SwarmCoordinator` implements **flat fan-out / collect** only.

| Label | Runtime in WeftOS? | Where it lives |
|-------|--------------------|----------------|
| `flat` | **Yes** — lead dispatches to registered workers, collects by `reply_to` | `crates/clawft-core/src/agent_bus/coordinator.rs` |
| `mesh` | No (reserved label) | claude-flow swarm prompts / Ruflo |
| `hierarchical` | No (reserved label) | claude-flow swarm prompts / Ruflo |
| `adaptive` | No (reserved label) | claude-flow swarm prompts / Ruflo |

## Why

CLAUDE.md and claude-flow agent catalogs advertise
`mesh-coordinator`, `hierarchical-coordinator`, and `adaptive-coordinator`.
Those are **LLM role prompts** for multi-agent coding swarms (Task / agent
teams). They are not separate Rust types and do not change
`AgentBus` routing.

Shipping a full topology axis (peer mesh messaging, coordinator trees,
dynamic rewiring) is research-heavy and blocked on L2 per-agent runtimes.
Until then, hosts may call `SwarmCoordinator::with_topology(Mesh|…)` to
**tag intent for logs/metrics**; dispatch stays flat.

## Code surface

```rust
use clawft_core::agent_bus::{SwarmCoordinator, SwarmTopology};

let (coord, bus) = SwarmCoordinator::with_capacity("lead", workers, 64);
assert_eq!(coord.topology(), SwarmTopology::Flat);
assert!(SwarmTopology::Flat.is_runtime_implemented());
assert!(!SwarmTopology::Hierarchical.is_runtime_implemented());
```

## CLAUDE.md alignment

The project-config line **Topology: hierarchical-mesh** describes the
**claude-flow coding swarm default**, not the in-process
`SwarmCoordinator` graph.

## Follow-ups

- Real mesh / hierarchical runtime graphs (post L2 `AgentRuntime`)
- Optional mapping of claude-flow topology flags → `SwarmTopology` tags
  when spawning WeftOS workers from Ruflo
