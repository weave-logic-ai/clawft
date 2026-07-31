# Wave-0 result: WEFT-197 / 199 / 200 / 201

**Branch:** `feat/weft-197-201-multi-agent`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/weft-197-201-multi-agent`  
**Date:** 2026-07-31

## Summary

| Ticket | Slice shipped | AC |
|--------|---------------|----|
| **WEFT-197** | `weft doctor` multi-agent checks | Yes |
| **WEFT-199** | Document topology as prompt-only + `SwarmTopology` enum | Yes |
| **WEFT-200** | Inbound list_changed refresh + outbound advertise/emit | Yes |
| **WEFT-201** | Fragility docs + regression corpus (no LoRA yet) | Yes (docs path) |

## WEFT-197 — `weft doctor`

- New command: `weft doctor [--strict] [--multi-agent] [--config PATH]`
- Checks:
  - `claude` on PATH (or `CLAUDE_CLI`)
  - auto-delegation: compile-time `delegate` feature, `claude_enabled`, rule count
  - agent routes: `config.agent_routing` (≥1 route / catch-all only / empty)
- Root config field: `agent_routing: AgentRoutingConfig` (serde default)
- Pure helpers unit-tested in `commands/doctor.rs`

```bash
weft doctor
weft doctor --multi-agent --strict
```

## WEFT-199 — SwarmCoordinator topology

**Decision:** mesh / hierarchical / adaptive are **claude-flow prompt-only**.
Runtime remains **flat** fan-out/collect.

- `SwarmTopology` enum + `with_topology` / `topology()` on `SwarmCoordinator`
- Non-flat variants are labels only (`is_runtime_implemented() == false`)
- Docs: `docs/architecture/swarm-topology.md`
- CLAUDE.md swarm-coordination section updated

## WEFT-200 — `notifications/tools/list_changed`

**Outbound**

- `initialize` already advertised `tools.listChanged: true`
- `tools_list_changed_notification()` + `McpServerShell::emit_tools_list_changed`
- Hosts emit after registry mutation (`provider_mut().register` then emit)

**Inbound**

- `McpTransport::poll_notification` (default `None`)
- `StdioTransport` demuxes server→client notifications in the reader loop
- `MockTransport::inject_notification` for tests
- `McpClient` / `McpSession::refresh_tools_if_list_changed` → re-`tools/list`
- `McpBridge::refresh_inbound_tools_if_list_changed` updates namespaced tools

## WEFT-201 — Auto-delegation classifier

**Decision:** document fragility + golden corpus (LoRA blocked on micro-LoRA router).

- Guide: `docs/guides/auto-delegation-classifier.md`
- Corpus tests: `classifier_corpus_*` in `delegation/mod.rs`, CLI auto-delegation tests

## How to test

```bash
cd /Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/weft-197-201-multi-agent

scripts/build.sh test clawft-cli -- doctor
scripts/build.sh test clawft-services -- list_changed
scripts/build.sh test clawft-services -- classifier_corpus
scripts/build.sh test clawft-core -- topology
scripts/build.sh check
```

## Files touched (high level)

| Area | Files |
|------|--------|
| Doctor | `clawft-cli/.../doctor.rs`, `main.rs`, `mod.rs`, `help_text.rs` |
| Config | `clawft-types/src/config/mod.rs` (`agent_routing`) |
| Topology | `agent_bus/coordinator.rs`, `agent_bus/mod.rs`, CLAUDE.md, `docs/architecture/swarm-topology.md` |
| list_changed | `mcp/transport.rs`, `mcp/mod.rs`, `mcp/server.rs`, `mcp/bridge.rs` |
| Classifier | `delegation/mod.rs`, `agent.rs` tests, `docs/guides/auto-delegation-classifier.md` |
