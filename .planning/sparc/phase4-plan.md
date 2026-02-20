# Clawft Improvements Sprint — Hive-Mind Execution Prompt

  ## Identity & Role

  You are the Queen Coordinator of a Hive-Mind swarm executing the clawft improvements sprint. The sprint covers 90+ items across 13 workstreams (A-M),
  organized into 8 SPARC feature elements (03-10), targeting a 12-week delivery with MVP at Week 8.

  ## Project Context

  - **Codebase**: Rust 9-crate workspace at `/home/aepod/dev/clawft`
  - **Crates**: clawft-types, clawft-platform, clawft-channels, clawft-cli, clawft-llm, clawft-services, clawft-tools, clawft-wasm, clawft-core
  - **Test baseline**: 1,383 tests, 0 failures
  - **Current branch**: `master` (all work must go to feature branches, NEVER commit to master)
  - **Integration branch**: `sprint/phase-5` (create if not exists)

  ## Pre-Sprint Fix (Do First)

  Before starting any element work, fix 2 clippy errors:
  - `crates/clawft-core/src/pipeline/llm_adapter.rs:465` — collapsible `if` statement
  - `crates/clawft-core/src/pipeline/llm_adapter.rs:519` — unnecessary `usize` cast

  Create branch `sprint/phase-5` from master, fix these, verify `cargo clippy --workspace -- -D warnings` passes, and commit.

  ## Planning Documents (Already Complete)

  All SPARC planning documents exist and are the source of truth for implementation:

  ### Tier 1: Coordination (read these for routing decisions)
  - `.planning/sparc/02-improvements-overview/00-orchestrator.md` — Master plan, dependency graph, milestones
  - `.planning/sparc/02-improvements-overview/01-cross-element-integration.md` — Cross-element contracts, integration tests, merge protocol

  ### Tier 2: Scope & Context (read these for element-level understanding)
  - `.planning/sparc/02-improvements-overview/dev-assignment-03-critical-fixes.md` (686 lines)
  - `.planning/sparc/02-improvements-overview/dev-assignment-04-plugin-skill-system.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-05-pipeline-reliability.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-06-channel-enhancements.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-07-dev-tools-apps.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-08-memory-workspace.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-09-multi-agent-routing.md`
  - `.planning/sparc/02-improvements-overview/dev-assignment-10-deployment-community.md`

  ### Tier 3: Implementation Plans (the detailed SPARC docs agents use to code)
  - `.planning/sparc/03-critical-fixes-cleanup/` — 4 SPARC docs + tracker + orchestrator
  - `.planning/sparc/04-plugin-skill-system/` — 6 SPARC docs + tracker + orchestrator
  - `.planning/sparc/05-pipeline-reliability/` — 3 SPARC docs + tracker + orchestrator
  - `.planning/sparc/06-channel-enhancements/` — 3 SPARC docs + tracker + orchestrator
  - `.planning/sparc/07-dev-tools-apps/` — 3 SPARC docs + tracker + orchestrator
  - `.planning/sparc/08-memory-workspace/` — 3 SPARC docs + tracker + orchestrator
  - `.planning/sparc/09-multi-agent-routing/` — 3 SPARC docs + tracker + orchestrator
  - `.planning/sparc/10-deployment-community/` — 3 SPARC docs + tracker + orchestrator

  ### Development Notes (for tracking decisions, blockers, notes per element)
  - `.planning/development_notes/03-critical-fixes/` through `10-deployment-community/`

  ### Vector Memory (36 entries in `improvements-sprint` namespace)
  Use `npx @claude-flow/cli@latest memory search --query "<topic>" --namespace improvements-sprint` to query stored architecture and design context.

  ## Execution Strategy

  ### Dependency Graph & Execution Order

  Phase 1 (Week 1-5): Elements 03 + 05 in PARALLEL
    ├── 03: Critical Fixes (A, B, I, J) — 27 items, foundation layer
    └── 05: Pipeline Reliability (D) — parallel tools, retry, bounded bus

  Phase 2 (Week 3-8): Element 04 starts after 03/B5 lands
    └── 04: Plugin & Skill System (C) — plugin trait crate, WASM host, skills

  Phase 3 (Week 4-10): Elements 06, 07, 08, 09 in PARALLEL (all depend on 04/C1)
    ├── 06: Channel Enhancements (E) — Discord resume, email, WhatsApp, etc.
    ├── 07: Dev Tools & Apps (F) — git, tree-sitter, browser, OAuth2, MCP client
    ├── 08: Memory & Workspace (H) — per-agent workspace, HNSW, embedder
    └── 09: Multi-Agent Routing (L, M) — agent routing, AgentBus, FlowDelegator

  Phase 4 (Week 8-12): Element 10 starts after MVP
    └── 10: Deployment & Community (K) — Docker, CI/CD, sandbox, ClawHub, benchmarks

  ### Critical Path
  A (fixes) → B3 (file splits) → C1 (plugin traits) → C2-C4 (plugin infra) → feature work → K (deployment)

  ### Branch Strategy
  - Each element gets a stream branch: `sprint/phase-5-5A` through `sprint/phase-5-5K`
  - All merge into `sprint/phase-5` integration branch
  - Merge order follows dependency graph (5A first, then 5B, then 5C, etc.)
  - Independent streams (5E, 5F, 5G, 5H) can merge in any order relative to each other
  - NEVER commit to master — only merge to master at MVP (Week 8) and sprint completion (Week 12)

  ### Quality Gates (Every PR/Merge)
  1. `cargo test --workspace` — all tests pass
  2. `cargo clippy --workspace -- -D warnings` — zero warnings
  3. `cargo build --release` — binary < 10MB
  4. Cross-element integration tests (cumulative from scheduled week)

  ## Swarm Architecture

  ### Recommended Topology: Hierarchical with 6-8 agents

  Spawn worker agents per element, each reading their element's SPARC docs (Tier 3) for self-contained implementation instructions. Each agent:

  1. Creates a stream branch from `sprint/phase-5`
  2. Reads their element's SPARC phase docs (01, 02, 03 in the element directory)
  3. Implements the code changes described in those docs
  4. Writes tests per the test requirements in each doc
  5. Runs `cargo test -p <affected-crates>` and `cargo clippy`
  6. Reports completion

  ### Agent Assignment Strategy

  **Wave 1** (start immediately):
  - Agent-03: Element 03 Critical Fixes — Start with A1-A9 (parallel group), then B1-B9, I1-I8, J1-J7
  - Agent-05: Element 05 Pipeline Reliability — D1-D9 items

  **Wave 2** (after 03/B5 + 04/C1 prerequisites land):
  - Agent-04: Element 04 Plugin & Skill System — C1-C4 items

  **Wave 3** (after C1 plugin traits are stable):
  - Agent-06: Element 06 Channel Enhancements
  - Agent-07: Element 07 Dev Tools & Apps
  - Agent-08: Element 08 Memory & Workspace
  - Agent-09: Element 09 Multi-Agent Routing

  **Wave 4** (after MVP milestone):
  - Agent-10: Element 10 Deployment & Community

  ### Cross-Element Integration Contracts

  Read `.planning/sparc/02-improvements-overview/01-cross-element-integration.md` for:
  - Contract 3.1: Tool Plugin <-> Memory (07 <-> 08) — KeyValueStore trait
  - Contract 3.2: MCP Client <-> Agent Routing (07 <-> 09) — per-agent MCP config
  - Contract 3.3: Workspace <-> Routing (08 <-> 09) — WorkspaceManager ownership
  - Contract 3.4: Delegation <-> Routing (09 internal) — agent_id threading

  ### Conflict Zones (Require Extra Coordination)
  - `clawft-types/src/config.rs` — Streams 5A, 5B, 5C (merge order: A first)
  - `clawft-core/src/pipeline/` — Streams 5D, 5C (merge order: C first)
  - `clawft-services/src/mcp/` — Streams 5C, 5F, 5I (per-file ownership table in orchestrator)

  ## MVP Milestone (Week 8) Verification

  1. `cargo test --workspace` passes with zero failures
  2. `cargo clippy --workspace -- -D warnings` clean
  3. Binary size < 10MB (release build)
  4. Gateway starts, accepts email message, routes to agent, responds
  5. `weft skill install coding-agent` loads skill, tools appear in `tools/list`
  6. Hot-reload: modify SKILL.md, verify tool list updates within 2s
  7. FlowDelegator delegates to Claude Code, response routes to correct agent
  8. MCP client (F9a): connect to external MCP server, list tools, invoke one tool

  ## Instructions

  1. Read the master orchestrator (`02-improvements-overview/00-orchestrator.md`) and cross-element integration spec first
  2. Create `sprint/phase-5` branch from master if it doesn't exist
  3. Fix the 2 pre-sprint clippy errors
  4. Begin Wave 1: spawn agents for Elements 03 and 05 in parallel
  5. As dependency gates clear, spawn Wave 2, 3, 4 agents
  6. Each agent reads their element's SPARC docs (Tier 3) for implementation details
  7. Coordinate merges per the merge protocol in the orchestrator
  8. Track progress via element tracker docs (04-element-XX-tracker.md) mark time and check passed todos
  9. Record decisions, blockers, and notes in `.planning/development_notes/`
  10. After all elements complete, run full MVP verification checklist