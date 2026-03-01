# Phase K6: Distributed Fabric Types -- Development Notes

**Start**: 2026-03-01
**Status**: Complete
**Gate**: check + test + clippy = PASS

## Decisions

### 1. Three separate modules (not one monolith)
**Problem**: K6 in the orchestrator covers "Distributed Fabric" -- a very broad scope including cluster membership, environments, governance, cryptographic filesystem, cross-node IPC, and learning loops. Implementing all in one module would exceed the 500-line limit and mix concerns.

**Decision**: Split into three focused modules: `cluster.rs` (node fabric), `environment.rs` (governance scoping), `governance.rs` (rule evaluation and effect scoring). Each under 500 lines with focused tests.

**Rationale**: Separation of concerns. Cluster handles the physical layer (which nodes exist). Environment handles the governance scope (what rules apply where). Governance handles the actual rule evaluation (permit/deny/escalate).

### 2. ClusterMembership without networking
**Decision**: `ClusterMembership` tracks peer nodes via `DashMap<NodeId, PeerNode>`. It has `add_peer()`, `remove_peer()`, `heartbeat()`, `update_state()`, and `active_peers()`. Actual peer discovery, heartbeat monitoring, and cross-node communication are NOT implemented.

**Rationale**: Same types-only pattern as K3/K4. The networking layer (libp2p, WebSocket, QUIC) is a separate concern that would require heavy dependencies. The membership tracker is useful for testing and for the CLI `weave network peers` command.

### 3. EnvironmentManager with standard set
**Decision**: `EnvironmentManager` has a `create_standard_set()` method that registers dev (risk 0.9, explore), staging (risk 0.6, validate), and prod (risk 0.3, exploit) environments. The active environment is tracked via `RwLock<Option<EnvironmentId>>`.

**Rationale**: Provides a useful out-of-the-box experience. The standard set matches the governance table from doc 09. RwLock (not DashMap) for the active environment because it's a single value, not a collection.

### 4. GovernanceEngine with effect algebra
**Decision**: `GovernanceEngine` evaluates `GovernanceRequest` objects by computing the `EffectVector` magnitude (L2 norm of 5D vector: risk, fairness, privacy, novelty, security) and comparing against the environment's risk threshold. Decisions are: Permit, PermitWithWarning, EscalateToHuman, Deny.

**Rationale**: The effect algebra is the core of the CGR engine from the AI-SDLC governance model. The L2 norm gives a single scalar for comparison. The four-level decision model supports graduated responses from advisory to hard block.

### 5. GovernanceEngine::open() for development
**Decision**: `GovernanceEngine::open()` creates an engine with risk_threshold=1.0 and no human approval, which permits all actions regardless of effect magnitude.

**Rationale**: Development environments need an open governance engine. This is the default when no governance rules are configured.

## What Was Skipped

1. **Cross-node IPC** -- Message routing between nodes requires networking layer.
2. **Cryptographic filesystem** -- Content-addressed storage requires BLAKE3, exochain DAG.
3. **DID identity** -- Agent identity via DIDs requires exochain-identity crate.
4. **SONA learning loop** -- Trajectory recording requires ruvector SONA crate.
5. **Peer discovery** -- Service discovery requires networking (libp2p, mDNS, or bootstrap peers).
6. **Heartbeat monitoring** -- Automated heartbeat requires async runtime and timers.
7. **Agent migration** -- Moving agents between nodes requires serialization and cross-node IPC.

## Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/cluster.rs` | ~320 | ClusterMembership, PeerNode, NodeState, 15 tests |
| `crates/clawft-kernel/src/environment.rs` | ~380 | EnvironmentManager, GovernanceScope, LearningMode, 16 tests |
| `crates/clawft-kernel/src/governance.rs` | ~370 | GovernanceEngine, EffectVector, GovernanceRule, 17 tests |

## Files Modified

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/lib.rs` | Added cluster, environment, governance modules; doc comments; re-exports |

## Test Summary

- 15 new tests in cluster.rs (config, peers, state transitions, heartbeat, capacity)
- 16 new tests in environment.rs (register, active, standard set, governance scope serde)
- 17 new tests in governance.rs (effect algebra, rule evaluation, escalation, serde)
- All 243 kernel tests pass (K6a commit)
- All workspace tests pass

---

## K6b: Agency Model (Doc 10 -- Agent-First Architecture)

### 6. Agency as hierarchical spawn permissions
**Decision**: `Agency` struct tracks `max_children`, `allowed_roles`, `capability_ceiling`, and current `children` PIDs. Factory methods provide standard configurations: `Agency::root()` (unlimited), `Agency::supervisor(n)` (services + workers), `Agency::service(n)` (workers only), `Agency::none()` (no agency).

**Rationale**: The agent-first model from doc 10 says "agency = ability to spawn agents." Hierarchical agency prevents privilege escalation: a service agent cannot spawn a supervisor, and a worker cannot spawn anything.

### 7. AgentManifest for .agent.toml files
**Decision**: `AgentManifest` captures all fields from doc 10's `.agent.toml` format: name, version, role, capabilities, agency, tools, topics, resources, interface, health, dependencies, filesystem access. Serde derives make it format-agnostic (JSON for tests, TOML when the dependency is added).

**Rationale**: Agent manifests are the core of the agent-first architecture. Each OS service (memory, cron, tool-registry, etc.) is described by a manifest file. The types enable validation and lifecycle management without requiring the actual manifest loader.

### 8. AgentCapabilities::root() extension
**Decision**: Added `AgentCapabilities::root()` constructor to capability.rs (via the agency module's impl block). Root capabilities set all flags to true and all resource limits to `u64::MAX`.

**Rationale**: Root agent (PID 0, user 1) needs unlimited capabilities. The `root()` constructor provides a clear semantic for "no restrictions."

### 9. Clippy: collapsible-if in Agency::can_spawn()
**Problem**: Nested `if let Some(max) = self.max_children { if self.children.len() >= max { ... } }`.

**Decision**: Collapsed to `if let Some(max) = self.max_children && self.children.len() >= max { ... }`.

**Rationale**: Clippy compliance. Same pattern as K1 capability checker.

### 10. AgentResources manual Default impl
**Problem**: `#[derive(Default)]` gave `max_memory_mb: 0` but the serde default functions provided meaningful defaults (256 MB). The derive and serde defaults disagreed.

**Decision**: Removed `#[derive(Default)]` and added manual `Default for AgentResources` impl that calls the same default functions as serde.

**Rationale**: Consistency between `AgentResources::default()` and deserialized-from-empty-JSON behavior.

## K6b Files Created

| File | Lines | Purpose |
|------|-------|---------|
| `crates/clawft-kernel/src/agency.rs` | ~589 | AgentRole, Agency, AgentManifest, 16 tests |

## K6b Test Summary

- 16 new tests in agency.rs (roles, agency hierarchy, child tracking, manifest serde, resources)
- All 259 kernel tests pass
- All workspace tests pass
