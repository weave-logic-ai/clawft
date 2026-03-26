# 08 Gap-Filling Orchestrator

**Document ID**: 08
**Workstream**: W-KERNEL
**Duration**: Weeks 17-22 (parallel across K-levels)
**Goal**: Coordinate three parallel workstreams filling OS pattern gaps across K1-K6
**Depends on**: K0-K6 (all prior phases)
**Source**: OS Gap Analysis (2026-03-26) + Microkernel Comparative Analysis
**Split from**: `08-phase-os-gap-filling.md` (original monolith)

---

## Overview

Three parallel workstreams filling OS pattern gaps across K1-K6 to make
WeftOS production-ready: self-healing, observability, reliable IPC, resource
enforcement, and operational services.

This is not a new "K7 phase" -- it is a gap-filling exercise that strengthens
each existing K-level. The work is split into three focused phases, each
self-contained enough for independent implementation.

## Phase Map

| Phase | Focus | Scope | Est. Lines | Priority | Depends On |
|-------|-------|-------|:----------:|----------|------------|
| 08a | Self-Healing & Process | K1, K2b | ~1,200 new / ~350 changed | P0 | K0-K6 complete |
| 08b | Reliable IPC & Observability | K2, Cross-cutting | ~1,800 new / ~350 changed | P1 | 08a K1-G1 (restart strategies) |
| 08c | Content & Operations | K3, K3c, K5, K6 | ~2,750 new / ~330 changed | P2 | 08a, 08b partial |

**Estimated totals**: ~5,750 new lines, ~960 changed lines across all three phases.

## Feature Gate Structure

```toml
[features]
os-patterns = ["exochain"]       # Self-healing, probes, reliable IPC, timers, config
os-full = ["os-patterns", "blake3"]  # Adds content-addressed artifact store
```

All gap-filling code is gated behind `#[cfg(feature = "os-patterns")]` to
preserve the existing kernel build. The `blake3` dependency is only required
for the artifact store (K3 gap).

## Dependency Graph

```
08a (Self-Healing) ─────────────────────────┐
  |                                          |
  +-- K1-G1: Restart strategies              |
  +-- K1-G2: Process links/monitors          |
  +-- K1-G3: Resource enforcement            |
  +-- K1-G4: Disk quotas                     |
  +-- K2b-G1: Reconciliation controller      |
  +-- K2b-G2: Liveness/readiness probes      |
                                             |
08b (IPC & Observability) ──────────────────-+
  |  (starts after K1-G1 restart done)       |
  +-- K2-G1: Dead letter queue               |
  +-- K2-G2: Reliable delivery               |
  +-- K2-G3: Named pipes                     |
  +-- K2-G4: Trace IDs                       |
  +-- K2-G5: Signal vocabulary               |
  +-- Cross-G1: Metrics                      |
  +-- Cross-G2: Log service                  |
  +-- Cross-G3: Timer service                |
                                             |
08c (Content & Operations) ─────────────────-+
  |  (starts after DLQ + metrics done)
  +-- K3-G1: Artifact store
  +-- K3c-G1: WeaverEngine
  +-- K3c-G2: EmbeddingProvider
  +-- K3c-G3: Weaver CLI commands
  +-- K3c-G4: weave-model.json export/import
  +-- K3c-G5: Meta-Loom integration
  +-- K5-G1: Config/secrets
  +-- K5-G2: Auth agent
  +-- K5-G3: Tree views
  +-- K6-G1: Artifact exchange
  +-- K6-G2: Log aggregation
```

## Parallelization Strategy

- 08a MUST complete K1-G1 (restart strategies) before 08b starts DLQ work,
  because the DLQ retry path must handle agents that crash during redelivery
- 08a and 08b can partially overlap: Cross-G1 (metrics) does not depend on
  restart strategies and can start in week 17 alongside K1-G1
- 08c can start K3-G1 (artifact store) in parallel with 08b, since the
  artifact store has no IPC dependencies
- 08c K6 items (artifact exchange, log aggregation) require 08b metrics and
  log service to be operational
- K2-G5 (KernelSignal vocabulary) is needed by K1-G2 (process links) and
  K1-G3 (resource warnings) -- schedule K2-G5 early in 08b or move to 08a

## Agent Assignments

| Agent | Phase(s) | Tasks |
|-------|----------|-------|
| process-supervisor | 08a | K1-G1, K1-G2, K1-G3, K1-G4, K2b-G1, K2b-G2 |
| kernel-architect | 08b | K2-G1, K2-G2, K2-G3, Cross-G3 |
| mesh-engineer | 08b, 08c | K2-G4, Cross-G1, K6-G1, K6-G2 |
| chain-guardian | 08c | K3-G1 (artifact store uses BLAKE3) |
| weaver | 08c | K3c-G1, K3c-G2, K3c-G3, K3c-G4, K3c-G5 (see `agents/weftos/weaver.md`) |
| governance-counsel | 08c | K5-G1 (config/secrets), K5-G2 (auth) |
| sandbox-warden | 08c | K5-G3 (tree views), K3-G1 |
| observability-agent | 08b | Cross-G1 (metrics), Cross-G2 (log service) |
| gap-test-agent | All | Integration tests, gate verification |
| gap-review-agent | All | Code review, security audit |

## Master Checklist

### 08a Self-Healing Gate (MUST pass before 08b DLQ starts)

- [ ] Agent crash triggers automatic restart within 1 second
- [ ] `OneForOne` strategy restarts only the failed agent
- [ ] `OneForAll` strategy restarts all sibling agents
- [ ] `RestForOne` strategy restarts the failed agent and later siblings
- [ ] Restart budget exceeded triggers supervisor escalation
- [ ] Exponential backoff prevents restart storms (100ms -> 30s max)
- [ ] `AgentRestartPolicy::Never` prevents restart
- [ ] Restart resets agent `ProcessState` to `Starting`
- [ ] Restart preserves original `SpawnRequest` configuration
- [ ] Process links deliver crash signals bidirectionally
- [ ] Process monitors deliver `ProcessDown` unidirectionally
- [ ] Unlink removes crash notification
- [ ] Remote monitors fire on node death (via mesh heartbeat)
- [ ] Resource limits enforced continuously (not just at spawn)
- [ ] CPU time limit triggers agent stop
- [ ] Message count limit enforced per agent
- [ ] Warning emitted at 80% of limit threshold
- [ ] Grace period before force cancel (5 seconds)
- [ ] Per-agent disk quota tracked and enforced
- [ ] All self-healing actions logged to ExoChain
- [ ] Reconciliation controller detects dead agents within 5 seconds
- [ ] Reconciliation controller spawns replacements automatically
- [ ] Reconciliation controller detects extra agents and stops them
- [ ] Reconciliation controller respects governance gate
- [ ] Reconciliation controller registered as `SystemService`
- [ ] Liveness probe failure triggers restart
- [ ] Readiness probe failure removes from `ServiceRegistry`
- [ ] Readiness recovery re-adds to `ServiceRegistry`
- [ ] Probe failure threshold prevents flapping
- [ ] Default probe methods return `Live` / `Ready` (backward compatible)

### 08b IPC & Observability Gate

- [ ] Dead letter queue captures messages to nonexistent PIDs
- [ ] Dead letter queue captures messages when inbox is full
- [ ] Dead letter queue queryable by target PID, reason, time range
- [ ] Dead letter queue retries messages on demand
- [ ] Dead letter queue size bounded with FIFO eviction
- [ ] Reliable send tracks acknowledgment with timeout
- [ ] Retry with exponential backoff on ack timeout
- [ ] Max retries exceeded routes to dead letter queue
- [ ] Named pipes created and connected successfully
- [ ] Named pipes survive agent restart
- [ ] Named pipes support multiple concurrent senders
- [ ] Named pipe capacity limits enforced
- [ ] Unused named pipes cleaned up after TTL
- [ ] Named pipe access respects `IpcScope` capabilities
- [ ] Distributed trace IDs propagate through IPC chain
- [ ] External messages receive new trace IDs at entry
- [ ] New `KernelSignal` variants serialize/deserialize correctly
- [ ] Counter metric type: increment and get (atomic)
- [ ] Gauge metric type: set, increment, decrement
- [ ] Histogram metric type: record and percentile query (p50, p95, p99)
- [ ] Built-in kernel metrics populated (messages, agents, tools, chain)
- [ ] `MetricsRegistry` registered as `SystemService` at boot
- [ ] Structured log entries stored in ring buffer
- [ ] Log query by PID, service, level, time range, trace_id
- [ ] Log subscription delivers entries in real-time
- [ ] Log rotation at configurable size/time thresholds
- [ ] Expired log files cleaned up automatically
- [ ] `LogService` registered as `SystemService` at boot
- [ ] One-shot timer fires at specified time
- [ ] Repeating timer fires at specified interval
- [ ] Sub-second timer precision (100ms tolerance)
- [ ] Timer cancellation prevents fire
- [ ] Owner PID death cancels owned timers
- [ ] Timer service registered as `SystemService`

### 08c Content & Operations Gate

- [ ] Artifacts stored by BLAKE3 hash
- [ ] Artifact retrieval verifies hash on load
- [ ] Duplicate artifacts deduplicated automatically
- [ ] WASM modules loaded from artifact store by hash
- [ ] File backend writes to correct path structure
- [ ] Reference counting and garbage collection work
- [ ] WeaverEngine registers as SystemService at boot (ecc feature)
- [ ] Modeling session start/stop via `weaver ecc` CLI
- [ ] Confidence evaluation produces gap analysis with suggestions
- [ ] Model export produces valid weave-model.json
- [ ] Git log ingestion creates causal nodes and edges
- [ ] File tree ingestion creates namespace structure
- [ ] Meta-Loom records Weaver's own modeling decisions as causal events
- [ ] CognitiveTick handler respects budget allocation
- [ ] EmbeddingProvider trait defined with mock implementation
- [ ] Cross-domain WeaverKnowledgeBase persists strategy patterns
- [ ] ONNX embedding backend (behind feature flag)
- [ ] CLI `weaver ecc` commands wired through daemon socket IPC
- [ ] Export + import roundtrip preserves causal graph structure
- [ ] Meta-Loom events under `meta-loom/{domain}` namespace
- [ ] Learned strategies applied to new sessions in similar domains
- [ ] Config service stores and retrieves configuration
- [ ] Config change notifications delivered to subscribers
- [ ] Config changes logged to ExoChain
- [ ] Secret service encrypts at rest
- [ ] Secret service delivers scoped, time-limited access
- [ ] Unauthorized PID cannot read secrets
- [ ] Auth service registers and manages credentials
- [ ] Auth service issues scoped, time-limited tokens
- [ ] Agents never receive raw credentials
- [ ] Credential access logged to ExoChain
- [ ] Per-agent tree views filter by capabilities
- [ ] Restricted agents cannot see unauthorized tree paths
- [ ] Namespace-scoped agents see correct paths
- [ ] Write to unauthorized tree path returns `PermissionDenied`
- [ ] Artifact exchange protocol transfers between mesh nodes
- [ ] Hash mismatch on receipt is rejected
- [ ] Artifact gossip announces new hashes
- [ ] Remote log entries carry source `node_id`
- [ ] Log query filters by `node_id`
- [ ] Aggregated query merges from multiple nodes

### Full Gap-Filling Gate (all three phases)

- [ ] All 08a exit criteria pass
- [ ] All 08b exit criteria pass
- [ ] All 08c exit criteria pass
- [ ] All existing tests pass (843+ baseline)
- [ ] New tests: target 200+ across all three phases
- [ ] Clippy clean for all new code
- [ ] Feature gated behind `os-patterns` / `os-full`
- [ ] No mandatory new dependencies for existing build
- [ ] `blake3` only required when `os-full` enabled
- [ ] `scripts/08-gate.sh` passes all checks
- [ ] `scripts/build.sh gate` passes with `os-patterns` feature
- [ ] Documentation updated in Fumadocs site

## Timeline

| Week | 08a | 08b | 08c |
|------|-----|-----|-----|
| 17 | K1-G1 restart + K1-G2 links | Cross-G1 metrics (parallel OK) | -- |
| 18 | K1-G3 resource + K2b-G1 reconciliation | K2-G1 DLQ + K2-G4 trace IDs | K3-G1 artifact store |
| 19 | K1-G4 quotas + K2b-G2 probes | K2-G2 reliable delivery + K2-G5 signals | K3c-G1 WeaverEngine + K3c-G5 Meta-Loom |
| 20 | Review + integration test | K2-G3 pipes | K3c-G3 CLI + K3c-G4 export/import + K5-G1 config |
| 21 | -- | Cross-G2 log service + Cross-G3 timer | K5-G2 auth + K5-G3 tree views + K6-G1 exchange |
| 22 | -- | Review + integration | K6-G2 log agg + Weaver e2e + review |

## Boot Sequence Integration

Gap-filling services register during `boot.rs` startup, after existing
K0-K6 services. Order respects dependencies:

```
Existing boot (K0-K6):
  1. ProcessTable            (K0)
  2. KernelIpc + MessageBus  (K0)
  3. A2ARouter + TopicRouter (K2)
  4. ServiceRegistry         (K1)
  5. HealthSystem            (K1)
  6. AgentSupervisor         (K1)
  7. WasmToolRunner          (K3)
  8. AppManager              (K5)
  9. CronService             (K5)
  10. GovernanceEngine       (K5)
  11. ChainManager + TreeManager (ExoChain)
  12. Mesh                   (K6, if enabled)

Gap-fill additions (when os-patterns enabled):
  13. MetricsRegistry        (CC-G1)
  14. LogService             (CC-G2)
  15. DeadLetterQueue        (K2-G1, needs A2ARouter)
  16. ReliableQueue          (K2-G2, needs A2ARouter + DLQ)
  17. NamedPipeRegistry      (K2-G3, needs A2ARouter)
  18. ReconciliationController (K2b-G1, needs ProcessTable + AppManager)
  19. TimerService           (CC-G3, needs A2ARouter)
  20. ConfigService          (K5-G1, needs TreeManager)
  21. AuthService            (K5-G2, needs ConfigService)
  22. ArtifactStore          (K3-G1, needs TreeManager, os-full only)
  23. Restart watchdogs      (K1-G1, attached per spawned agent)
  24. Probe runners          (K2b-G2, attached per registered service)
  25. Resource enforcers     (K1-G3, attached per spawned agent)
```

## Testing Verification Commands

```bash
# Build with OS patterns feature
scripts/build.sh native --features os-patterns

# Build with full OS features (includes blake3)
scripts/build.sh native --features os-full

# Run gap-fill tests
scripts/build.sh test -- --features os-patterns

# Verify base build unchanged (no gap-fill deps)
scripts/build.sh check

# Full phase gate
scripts/build.sh gate
```

## Weaver Agent & Skill Reference

The WeaverEngine (K3c gaps) has two companion documents:

- **Agent definition**: `agents/weftos/weaver.md` -- the kernel-native
  cognitive modeler agent that implements K3c-G1 through K3c-G5. This agent
  runs the HYPOTHESIZE-OBSERVE-EVALUATE-ADJUST loop on causal models via the
  WeaverEngine SystemService.

- **Operator skill**: `agents/weftos-ecc/WEAVER.md` -- the Claude interaction
  guide that describes HOW to interact with the WeaverEngine (session
  workflows, CLI commands, confidence interpretation, modeling strategy). This
  skill describes the operator interface; the K3c implementation is the kernel
  service underneath.

The skill is the user-facing guide. The 08c plan is the implementation spec.

## Cross-References

- **08a**: `08a-self-healing.md` -- Self-Healing & Process Management
- **08b**: `08b-reliable-ipc.md` -- Reliable IPC & Observability
- **08c**: `08c-content-ops.md` -- Content Integrity & Operational Services
- **Original**: `08-phase-os-gap-filling.original.md` (archived)
