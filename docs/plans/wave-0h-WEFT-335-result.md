# Wave 0h — WEFT-335 result

**Ticket:** WEFT-335 — ws11: agent-core-v1.1 — observability path logging router decisions to substrate  
**Branch:** `wave0h/weft-335-router-obs`  
**Base:** `release/0.8-staging`  
**Status:** implemented  
**Worktree:** this agent worktree  

## Acceptance criteria

| AC | Status |
|----|--------|
| Every `ContextRouter::route` call appends a decision record | **Done** — `AgentLoop::handle_turn` times `route`, then `maybe_append_routing_decision` when a log is attached |
| Path: `substrate/<node>/agent/routing/recent/<ulid>` | **Done** — mesh-canonical `substrate/_derived/agent/routing/recent/<ulid>` (same `_derived/` migration as soul-journal / chat; grant topic `agent`) |
| Record includes: query, selected route, alternatives, confidence, latency | **Done** — `RouterDecisionRecord` + substrate JSON value |
| Bounded retention (last N or last X days) | **Done** — default last 10_000 (`DEFAULT_ROUTING_LOG_RETENTION`); in-memory ring + substrate tombstone prune |
| Test: 100 routes produce 100 substrate entries | **Done** — core in-memory, loop_core 100× handle_turn, service-agent MapClient + kernel grant |

## Design

```
handle_turn
  └─ Instant::now → context_router.route(&req) → latency_ms
       └─ routing_log.append(RouterDecisionRecord)   # best-effort, non-fatal
            └─ substrate/_derived/agent/routing/recent/<ulid>
```

Chat path is fail-open: log / grant errors are `warn!` only and never abort the turn.

### Record shape

```json
{
  "decision_id": "<ulid>",
  "query": "...",
  "selected_route": {
    "skills": [],
    "tool_subset": null,
    "complexity_hint": 0.0,
    "archetype": null
  },
  "alternatives": [],
  "confidence": null,
  "latency_ms": 12,
  "channel": "panel",
  "chat_id": "...",
  "fallback_used": false,
  "ts": "2026-…"
}
```

`HybridRouter` sets `ContextDecision.fallback_used = true` on fall-through so the v2→v2.5 fallback-rate gate can read it from the log without re-deriving traces.

### Grant

Daemon boot issues `DerivedWriteGrant` topic `agent` (`GrantScope::TopicPrefix`), covering `substrate/_derived/agent/routing/recent/…`.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-core/src/agent/routing_log.rs` | **New** — trait, record, in-memory log, constants, unit tests |
| `crates/clawft-core/src/agent/mod.rs` | `pub mod routing_log` |
| `crates/clawft-core/src/agent/context_router.rs` | `ContextDecision.fallback_used` |
| `crates/clawft-core/src/agent/context_router/hybrid.rs` | Mark fallback; unit test |
| `crates/clawft-core/src/agent/loop_core.rs` | `routing_log` field, builder, log after every `route`, tests |
| `crates/clawft-core/src/bootstrap.rs` | `build_daemon_agent_loop(..., routing_log)` |
| `crates/clawft-service-agent/src/routing_log.rs` | **New** — `SubstrateRouterDecisionLog` + retention prune |
| `crates/clawft-service-agent/src/lib.rs` | Export module + type |
| `crates/clawft-weave/src/daemon.rs` | Grant `agent` + attach substrate writer |
| `docs/plans/wave-0h-WEFT-335-result.md` | This file |

## How to test

```bash
scripts/build.sh test clawft-core clawft-service-agent
# or focused:
cargo test -p clawft-core --lib routing_log
cargo test -p clawft-core --lib handle_turn_appends_routing -- --nocapture
cargo test -p clawft-core --lib one_hundred_routes -- --nocapture
cargo test -p clawft-service-agent routing_log --lib
scripts/build.sh check
```

## Blocks / follow-ups

- **WEFT-336** (`weft routing trace` / `replay` + p99 / fallback-rate in `weft status`) reads this path.
- Confidence / alternatives are currently best-effort (metadata / future router fields); NullRouter logs empty decisions with measured latency — enough to seed the ≥1,000-decision count gate.
