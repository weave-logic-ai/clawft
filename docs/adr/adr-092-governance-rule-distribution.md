# ADR-092: Cluster-wide GovernanceRule distribution

- **Status**: Accepted (foundation)
- **Closes / tracks**: WEFT-146
- **Date**: 2026-07-31
- **Related**: ADR-089 (ExoChain DAG merge / split-brain), governance engine

## Context

`Vec<GovernanceRule>` has historically been static and per-node. Under
multi-node mesh operation, nodes can drift on policy (one node permits
what another denies). Cross-node governance needs:

1. Versioned rule sets
2. Gossip / distribution of rules
3. Conflict resolution after split
4. Escalation when local policy denies but a peer holds a newer rule

## Decision

Introduce `crates/clawft-kernel/src/rule_distribution.rs`:

| Type | Role |
|------|------|
| `VersionedRule` | rule + version + origin_node + updated_at |
| `RuleGossipEnvelope` | wire payload for full/partial push |
| `RuleDistribution` | local store: upsert, merge (LWW), escalate |
| `EscalationOutcome` | `LocalStand` \| `Forward { peer, rule_id, peer_version }` |

**Merge**: last-writer-wins on `(version, updated_at_unix, origin_node)`.

**Escalation**: on local `Deny`, if any remote hint has a strictly higher
version for a relevant rule, return `Forward` and append an
`EscalationRecord`.

Mesh transport wiring (actually sending `RuleGossipEnvelope` over
`mesh_ipc`) is left to a follow-up; the types are serde-ready.

## Consequences

- Engines can inject `RuleDistribution::active_rules()` instead of a
  static vec.
- Tests cover split/merge and escalation.
- Full K6 production gossip loop + chain anchoring of rule mutations
  remain follow-ups (not blocking WEFT-146 foundation AC).
