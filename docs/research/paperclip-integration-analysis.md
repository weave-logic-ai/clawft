# Paperclip Integration Analysis

**Date**: 2026-04-01
**Status**: Research complete, integration planned for Sprint 13

## What is Paperclip?

Open-source orchestration platform for "zero-human companies" — teams of AI agents organized into corporate structures. 43.6K GitHub stars in first month (March 2026). MIT license. TypeScript/Node.js.

- **Repo**: https://github.com/paperclipai/paperclip
- **Commercial**: https://paperclip.inc
- **npm CLI**: `paperclipai` (108K monthly downloads)
- **Plugin SDK**: `@paperclipai/plugin-sdk` (73K monthly)

## Architecture

- CEO → Manager → Worker agent hierarchy
- Heartbeat-driven execution (wake, check queue, execute, sleep)
- 7 agent runtime adapters (Claude Code, Codex, Cursor, Gemini, OpenClaw, OpenCode, Pi)
- JSON-RPC 2.0 plugin system (out-of-process, capability-gated)
- Embedded PostgreSQL for state
- React dashboard UI
- Budget enforcement per agent
- Full audit trail

## WeftOS Comparison

### Paperclip Ahead
- Visual dashboard (org charts, cost tracking, task boards)
- Instant setup (`npx paperclipai`)
- Business metaphor (companies, roles, budgets)
- Active plugin ecosystem (10+ community plugins in month 1)
- Multi-runtime adapters (7 agent runtimes)
- Company templates marketplace ("Clipmart")

### WeftOS Ahead
- Compiled Rust performance (sub-ms kernel ops)
- 7-stage agent pipeline with tiered routing
- Full OS kernel (PIDs, process supervision, IPC, mesh)
- Cryptographic governance (ExoChain provenance, capability security)
- WASM sandboxing for tools/agents
- Distributed mesh networking (Noise protocol, heartbeat, discovery)
- Causal knowledge graph (ECC)
- Native HNSW vector search
- 9 platform targets (native + WASM + WASI)

### Key Insight
**Paperclip is a control plane; WeftOS is a kernel.** They solve different layers:
```
Paperclip (companies, goals, budgets, org charts, dashboard)
    ↓
WeftOS kernel (governance, provenance, sandbox, mesh, IPC)
    ↓
clawft runtime (7-stage pipeline, skills, tools, memory)
    ↓
LLM providers (Claude, Hermes, Ollama, local models)
```

## Integration Path

### 1. WeftOS Adapter for Paperclip
- New adapter: `@paperclipai/adapter-weftos`
- Implements heartbeat protocol, delegates to clawft pipeline
- WeftOS kernel API endpoint for task execution requests

### 2. Governance Bridge
- Paperclip approval gates → WeftOS policy kernel
- Cryptographic attestation on decisions
- ExoChain provenance for audit trail

### 3. Memory Upgrade
- Replace Paperclip's PARA file memory with WeftOS HNSW vector store
- Semantic search over agent memory across heartbeats

### 4. Rust-Native Patterns to Adopt
| Pattern | Implementation | Effort |
|---------|---------------|--------|
| Company/org-chart model | `clawft-types` structs | Small |
| Heartbeat scheduler | Extend `cron.rs` | Small |
| Budget enforcement UI | Surface `cost_tracker.rs` in GUI | Small |
| Plugin JSON-RPC host | `clawft-plugin` stdin/stdout | Medium |
| Ticket/issue system | New kernel service | Medium |
| Goal alignment tree | Extend `exo-resource-tree` | Medium |
| Dashboard (Paperclip-style) | Tauri GUI blocks | Large |
| Multi-company isolation | Kernel namespace isolation | Medium |
| Plugin marketplace | `create-weftos-plugin` scaffolding | Medium |
