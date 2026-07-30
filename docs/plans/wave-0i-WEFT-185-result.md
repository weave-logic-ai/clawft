# WEFT-185 result — AgentBus + SwarmCoordinator worker loops + demo

**Ticket:** WEFT-185  
**Branch:** `wave0i/weft-185-swarm-demo`  
**SHA:** (see `git rev-parse wave0i/weft-185-swarm-demo`)  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4c3-9a5a-7d11-afcd-7ea5ca084de6`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-185 (wave-0i)

## Problem

Types and unit tests for `AgentBus` / `SwarmCoordinator` existed, but no
production path constructed an `AgentBus` or ran a `worker_message_loop`.
`AgentBus::with_capacity` / coordinator capacity wiring was unused outside
tests. `dispatch_subtask` and `broadcast_task` were only exercised by unit
tests — no live worker loops, no demo.

## What shipped

### Core — `clawft-core` agent_bus module split

| Module | Role |
|--------|------|
| `agent_bus/bus.rs` | `AgentBus`, `AgentInbox`, capacity + TTL delivery |
| `agent_bus/worker.rs` | `WorkerHandler`, `EchoWorker`, `RuntimeBackedWorker`, `worker_message_loop`, `spawn_worker_loop` |
| `agent_bus/coordinator.rs` | `SwarmCoordinator` fan-out/collect + spawn + demo |

New coordinator APIs:

- `SwarmCoordinator::with_capacity` — constructs a bounded `AgentBus` (production convenience; was the audit gap)
- `spawn_workers` / `spawn_handlers` — register agents and spawn background loops
- `collect_replies` / `dispatch_and_collect` — correlate `reply_to` end-to-end
- `shutdown_workers` — clean teardown via `task = "shutdown"`
- `run_swarm_demo(n)` — library-level demo workflow

### Production wiring

- `AppContext::enable_agent_bus(capacity)` — constructs + attaches `AgentBus` once
- `weft agent` and `weft gateway` call `enable_agent_bus(None)` on bootstrap
- CLI demo builds `AgentRuntime::for_agent` per worker so loops are isolation-backed

### Demo surfaces

| Surface | Command |
|---------|---------|
| CLI | `weft swarm demo [--workers N] [--workspace PATH]` |
| Example binary | `cargo run -p clawft-core --example swarm_demo --features native -- [N]` |
| Library | `clawft_core::agent_bus::run_swarm_demo(n)` |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Production wiring constructs an `AgentBus` | **Yes** — `AppContext::enable_agent_bus` + agent/gateway bootstrap |
| Spawn worker loops backed by `AgentRuntime` | **Yes** — CLI demo; `RuntimeBackedWorker` + `AgentRuntime::for_agent` |
| Demo coordinator workflow with `dispatch_subtask` e2e | **Yes** — `weft swarm demo` + example + `run_swarm_demo` |
| Tests / e2e | **Yes** — 16 agent_bus lib tests + bootstrap enable + CLI demo unit test + clap parse |
| `scripts/build.sh test` (scoped) | **Yes** for agent_bus + clawft-cli; one pre-existing unrelated fail in `workspace::config::tests::load_merged_config_mcp_servers` |

## Files

| Path | Change |
|------|--------|
| `crates/clawft-core/src/agent_bus/mod.rs` | **new** module root (replaces flat `agent_bus.rs`) |
| `crates/clawft-core/src/agent_bus/bus.rs` | **new** — bus + inbox |
| `crates/clawft-core/src/agent_bus/worker.rs` | **new** — loops + handlers |
| `crates/clawft-core/src/agent_bus/coordinator.rs` | **new** — SwarmCoordinator + demo |
| `crates/clawft-core/src/bootstrap.rs` | `enable_agent_bus` + unit test |
| `crates/clawft-core/examples/swarm_demo.rs` | **new** example binary |
| `crates/clawft-cli/src/commands/swarm_cmd.rs` | **new** — `weft swarm demo` |
| `crates/clawft-cli/src/commands/mod.rs` | export `swarm_cmd` |
| `crates/clawft-cli/src/commands/agent.rs` | `enable_agent_bus` on bootstrap |
| `crates/clawft-cli/src/commands/gateway.rs` | `enable_agent_bus` on bootstrap |
| `crates/clawft-cli/src/main.rs` | `Commands::Swarm` + parse test |
| `crates/clawft-cli/src/help_text.rs` | swarm help topic + general listing |
| `docs/guides/skills-and-agents.md` | document worker loops + demo |
| `docs/plans/wave-0i-WEFT-185-result.md` | this file |

## How to test

```bash
# Unit / e2e (core)
cargo test -p clawft-core --lib agent_bus --features native
cargo test -p clawft-core --lib enable_agent_bus --features native

# CLI demo unit + parse
cargo test -p clawft-cli demo_two_workers
cargo test -p clawft-cli cli_swarm_demo_parses

# Live demos
cargo run -p clawft-core --example swarm_demo --features native -- 3
cargo run -p clawft-cli -- swarm demo --workers 2

# Scoped package tests
scripts/build.sh test clawft-core clawft-cli
```

## Notes / non-goals

- Topology axis (mesh / hierarchical / adaptive) is **WEFT-199**, not this ticket.
- Worker handlers are echo/runtime-bound; full LLM agent-loop-as-worker is a
  follow-up (still uses the same `WorkerHandler` seam).
- Pre-existing fail: `workspace::config::tests::load_merged_config_mcp_servers`
  (null MCP config deserialize) — unrelated to WEFT-185.
