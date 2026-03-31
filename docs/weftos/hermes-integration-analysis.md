# Hermes Agent: Integration & Competitive Analysis for WeftOS

**Date**: 2026-03-30
**Status**: Research complete, decisions pending

## Executive Summary

Hermes is three things: open-weight LLMs (NousResearch), an agent framework (18.5K stars, Python, MIT), and a function-calling protocol. **clawft is the direct Rust parallel to the Hermes agent framework** — both implement the same agent pattern (loop, skills, memory, tools, channels) but clawft does it in compiled Rust with a more sophisticated pipeline.

WeftOS is the **kernel/OS layer** that clawft agents run on. Hermes agents could also run on WeftOS as Python processes alongside clawft agents.

## Critical Correction: clawft IS the Hermes Equivalent

The initial analysis overlooked that clawft-core already implements nearly everything Hermes has — in Rust, with 17,795 lines of agent + pipeline code:

### Direct Feature Mapping

| Feature | Hermes (Python) | clawft (Rust) | clawft Status |
|---------|----------------|---------------|---------------|
| Agent loop | `AIAgent.run_conversation()` | `AgentLoop` in `loop_core.rs` (2,235 lines) | **Done** — consume→process→respond with tool iteration |
| Skills (SKILL.md) | YAML frontmatter + markdown | `skills_v2.rs` (1,430 lines) — same format, 3-level priority registry | **Done** |
| Skill auto-creation | After 5+ tool calls | `skill_autogen.rs` (638 lines) — pattern detection, threshold=3, pending approval | **Done** |
| Skill hot-reload | N/A | `skill_watcher.rs` (555 lines) — file system watcher | **Ahead** |
| Memory | MEMORY.md + USER.md | `memory.rs` (461 lines) — MEMORY.md + HISTORY.md, platform-abstracted | **Done** |
| Context builder | With compression | `context.rs` (980 lines) — system + skills + memory + history | **Missing compression** |
| Tool registry | Python singleton | `tools/registry.rs` — compiled, type-safe | **Done** |
| Pipeline | Simple LLM call | 7-stage: Classifier→Router→Assembler→Transport→Scorer→Learner | **Way ahead** |
| Tiered routing | Keyword heuristic | `tiered_router.rs` (1,650 lines) — complexity-based, cost-tracked, permission-aware | **Way ahead** |
| Cost tracking | N/A | `cost_tracker.rs` (954 lines) — per-tier budget enforcement | **Ahead** |
| Rate limiting | N/A | `rate_limiter.rs` (632 lines) | **Ahead** |
| Permissions | Command allowlist | `permissions.rs` (757 lines) — full permission resolver | **Ahead** |
| Security | Tirith binary scanner | `verification.rs` + `security/` module — sanitization, validation | **Done** |
| Channels | Telegram/Discord/Slack/etc | `clawft-channels` crate — same platforms | **Done** |
| LLM providers | OpenAI SDK (universal) | `clawft-llm` — multi-provider native adapters | **Done** |
| Delegation | `delegate_task` (max 3) | Auto-delegation trait in loop_core | **Done** |
| WASM/Browser | N/A | `clawft-wasm` — runs in browser | **Ahead** |
| Pipeline learning | N/A | `pipeline/learner.rs` (139 lines — stub) | **Stub — needs GEPA** |
| Pipeline scoring | N/A | `pipeline/scorer.rs` (154 lines) | **Basic — needs evolution** |
| Self-evolution | GEPA + DSPy + Atropos | N/A | **Gap — key adoption target** |
| Context compression | Structured summarization | N/A | **Gap — needs implementation** |
| User modeling | Honcho dialectic | N/A | **Gap** |
| RL trajectories | ShareGPT JSONL + Atropos | N/A | **Gap** |
| Session search | SQLite FTS5 + LLM summary | N/A | **Gap — but ECC could serve this** |

### What clawft Has That Hermes Doesn't

- **7-stage pipeline** (Classifier→Router→Assembler→Transport→Scorer→Learner) vs Hermes's simple LLM call
- **Tiered routing** with complexity scoring, cost budgets, and permission-awareness
- **Platform abstraction** (native + WASM/browser + WASI)
- **Compiled Rust performance** vs Python runtime
- **Skill hot-reload watcher** for live development
- **Security sanitization module** with depth validation

### What Hermes Has That clawft Needs (Priority Order)

1. **Context compression** — structured summarization when approaching context limits (HIGH — affects every long session)
2. **GEPA prompt evolution** — genetic evolution of skill prompts using the pipeline scorer as fitness function (HIGH — self-improvement flywheel)
3. **User modeling** — dialectic understanding across sessions (MEDIUM — enhances personalization)
4. **Session search** — FTS5 search across past conversations (MEDIUM — ECC causal graph could serve this role)
5. **RL trajectory generation** — ShareGPT-format training data from real runs (LOW — valuable for model training, not agent runtime)

## The Architecture (Corrected)

```
WeftOS Kernel (governance, provenance, sandbox, mesh, ECC)
    │
    ├── clawft agent runtime (Rust, 17K+ lines, 7-stage pipeline)
    │   ├── Skills (SKILL.md, auto-creation, hot-reload, GEPA evolution)
    │   ├── Memory (MEMORY.md, HISTORY.md, ECC causal graph)
    │   ├── Context (builder + compression)
    │   ├── Pipeline (classify → route → assemble → transport → score → learn)
    │   ├── Tools (compiled registry + WASM sandbox)
    │   ├── Channels (Telegram, Discord, Slack, ...)
    │   └── LLM providers (Claude, OpenAI, Hermes models via OpenRouter)
    │
    ├── Hermes agents (Python, via ACP/IPC — optional, for open-weight local models)
    │
    ├── claude-flow orchestration (swarm coordination, MCP)
    │
    └── OpenClaw (legacy compatibility)
```

clawft is NOT a separate thing from WeftOS — it IS the user-space agent runtime that runs on WeftOS's kernel primitives.

## Key Findings

### Gaps to Close (from Hermes)

1. **Context compression** — add to `context.rs`, use Hermes's structured summarization approach
2. **GEPA prompt evolution** — expand `pipeline/learner.rs` from 139-line stub to genetic prompt optimizer using `pipeline/scorer.rs` as fitness function, with ECC tracking prompt lineage and governance gating deployment of evolved prompts
3. **User modeling** — add Honcho-style dialectic user understanding, integrate with ECC knowledge graph
4. **Hermes models as clawft-llm provider** — add OpenAI-compatible endpoint support for locally-hosted Hermes LLMs

### What WeftOS + clawft Offer That Hermes Cannot Match

1. **ExoChain** — cryptographic provenance (Ed25519 + ML-DSA-65)
2. **Governance Engine** — three-branch constitutional, effect vectors, dual-layer gates
3. **ECC Cognitive Substrate** — causal DAG, spectral analysis, cognitive tick
4. **WASM Sandboxing** — deterministic, capability-constrained tool execution
5. **Mesh Networking** — encrypted P2P multi-node coordination
6. **Self-Healing** — supervisor restart strategies, DLQ
7. **Compiled Performance** — Rust kernel vs Python runtime
8. **7-stage Pipeline** — vs Hermes's simple LLM call
9. **Governed Prompt Evolution** — GEPA with ExoChain provenance and governance gates (Hermes has no safety layer on prompt mutations)

### Integration Paths

| Path | Feasibility | Value |
|------|------------|-------|
| Hermes models as clawft-llm provider | High | Open-weight local LLMs for air-gapped deployments |
| Hermes agents as WeftOS processes | Medium | PID tracking, governance, provenance for Hermes agents |
| WeftOS tools exposed via OpenAI format | High | Hermes consumes WeftOS ToolRegistry |
| Hermes via claude-flow swarms | Medium | Heterogeneous swarms (local + cloud models) |
| agentskills.io compatibility | High | Cross-platform skill portability |

### What WeftOS Should Adopt

1. **Procedural memory / skills** — implement in ECC substrate (graph nodes, not flat files)
2. **Context compression** — structured summarization for long sessions
3. **User modeling** — build into knowledge graph
4. **Messaging gateway** — as a kernel service module
5. **agentskills.io compatibility** — access the growing skill ecosystem

### Joint Architecture Vision

```
WeftOS Kernel (governance, provenance, sandbox, mesh, ECC)
    |
    ├── Hermes Agent (local model, skills, messaging)
    ├── Claude Agent (cloud API, plan mode)
    ├── Custom Agent (WASM-sandboxed)
    └── OpenClaw Agent (legacy compatibility)
```

### Threat Assessment: Moderate

Not a direct competitor (different layers), but:
- 18.5K stars captures mindshare in the autonomous agent space
- Skills system is ahead of WeftOS's procedural knowledge capabilities
- `agentskills.io` standard is being established by the Hermes community
- OpenClaw migration included — NousResearch positioning for consolidation

### Opportunity: High

- WeftOS governance/provenance has no Hermes equivalent — this is the enterprise moat
- "Hermes runs on WeftOS" positions WeftOS as infrastructure that makes agents enterprise-ready
- Supporting Hermes models gives WeftOS cloud-independent LLM capability
