# Sprint 10: Operational Hardening + K8 GUI Layer

**Document ID**: 10
**Workstream**: W-KERNEL + W-GUI + W-PRODUCT
**Duration**: 6 weeks (Weeks 1-6)
**Goal**: Take WeftOS from "demo kernel" to "deployable product" — self-healing, persistence, observability, mesh runtime, and the K8 GUI layer for system visualization and client onboarding
**Depends on**: All prior phases (K0-K6, 08a-09d plans)
**Date**: 2026-03-27
**Priority**: P0

---

## S — Specification

### Strategic Context

WeftOS has a complete architecture: 22 crates, 175K+ lines of Rust, 3,970+ tests,
all kernel layers K0-K6 implemented. The single-node kernel boots, agents
communicate, governance enforces, the chain signs, and the Weaver analyzes.

What's missing is the operational layer that makes it trustworthy for real use,
and the GUI layer that makes it visible and sellable.

**The product thesis**: WeftOS is sold as a tool to understand other systems —
codebases, infrastructure, processes — document them deeply via knowledge graph,
and plan automation/AI/agentic workflows. The Weaver's self-analysis capability
IS the product, applied to client environments.

**The K8 thesis**: Every OS needs a way to see what it's doing. K8 is the
graphical layer — a web-based (and optionally native) dashboard that shows
processes, chain state, knowledge graph, governance decisions, and mesh topology.
This is also the client-facing onboarding experience: walk through questions,
build the knowledge graph conversationally, see the analysis visualized.

### What This Sprint Delivers

| Track | Deliverable | Priority |
|-------|-------------|----------|
| **W1: Self-Healing** | 08a restart strategies, process links, reconciliation, probes | P0 |
| **W2: Persistence** | ExoChain, tree, causal graph survive restart | P0 |
| **W3: Observability** | 08b dead letter queue, metrics, logging, timers | P0 |
| **W4: Mesh Runtime** | Wire transports → A2A bridge; two nodes communicate | P1 |
| **W5: Test Hardening** | a2a.rs, daemon, boot path coverage | P1 |
| **W6: K8 GUI Layer** | Web dashboard, knowledge graph viz, chain explorer | P1 |
| **W7: Deployment** | Docker, install guide, feature gate docs | P1 |
| **W8: Client Onboarding** | Conversational discovery flow, external codebase analysis | P2 |

---

## P — Pseudocode

### Week 1: Self-Healing + Persistence Foundation

```
PARALLEL:
  Track W1 (Self-Healing):
    - Implement RestartStrategy enum (OneForOne, OneForAll, RestForOne)
    - Add restart budget (max_restarts, window_secs) to AgentSupervisor
    - Implement exponential backoff on repeated failures
    - Add ProcessLink and ProcessMonitor to process.rs
    - Chain-log all restart events

  Track W2 (Persistence):
    - Add SQLite backend for ExoChain persistence (or RocksDB if perf needed)
    - Implement causal graph save/load to disk (JSON-lines or bincode)
    - Implement HNSW index persistence
    - Tree manager checkpoint-to-disk on shutdown already exists — verify it works
    - Write recovery test: boot → add data → shutdown → boot → verify data

  Track W5 (Tests):
    - Add 20+ tests to a2a.rs (message routing, topic pub/sub, service resolution)
    - Add 10+ tests to boot.rs (error paths, partial boot recovery)

CLOSE:
  - K3 (1 remaining criterion)
  - K4 (2 remaining criteria)
  - K5 (1 remaining criterion)
```

### Week 2: Observability + Mesh Glue Begins

```
PARALLEL:
  Track W3 (Observability):
    - Implement DeadLetterQueue with capture, query, retry
    - Implement ReliableDelivery with ack tracking + retry + backoff
    - Activate MetricsRegistry (os-patterns module exists — validate + integrate)
    - Activate LogService (os-patterns module exists — validate + integrate)
    - Activate TimerService (os-patterns — validate + integrate)
    - Wire metrics into boot.rs: register kernel metrics on startup

  Track W4 (Mesh Runtime):
    - Create MeshRuntime struct that:
      1. Binds a MeshTransport listener (TCP or WS)
      2. Accepts incoming connections
      3. Performs handshake (Noise or plaintext for dev)
      4. Spawns per-connection frame reader
      5. Dispatches incoming MeshFrame by type
    - Implement the A2A bridge:
      - RemoteNode handler in a2a.rs wraps KernelMessage in MeshIpcEnvelope
      - Incoming MeshIpcEnvelope unwraps and injects into local A2A router

  Track W1 (continued):
    - Implement ReconciliationController (desired vs actual state)
    - Implement liveness + readiness probes
    - Add continuous resource enforcement (CPU, memory, message count)
```

### Week 3: Mesh Completion + K8 GUI Design Sprint

```
PARALLEL:
  Track W4 (Mesh — continued):
    - Wire mDNS discovery into MeshRuntime (LAN auto-discovery)
    - Wire Kademlia for WAN peer lookup
    - Implement chain sync protocol (incremental replication)
    - Test: two nodes boot, discover each other, exchange messages

  Track W6 (K8 GUI — Design Phase):
    - Produce wireframes/mockups for 5 core views:
      1. DASHBOARD: System overview — node health, agent count, chain height,
         ECC confidence, mesh peers, resource usage
      2. PROCESS EXPLORER: Live process table with state, PID, capabilities,
         resource usage, parent/child relationships (like Activity Monitor)
      3. CHAIN VIEWER: ExoChain event timeline — filterable by event type,
         agent, governance branch. Click to see full event + signatures
      4. KNOWLEDGE GRAPH: Interactive causal graph visualization —
         nodes as circles, edges as typed arrows, community coloring,
         search by label, click to inspect. Fiedler partition overlay.
         This IS the client-facing analysis view.
      5. GOVERNANCE CONSOLE: Live governance decisions, effect vector
         scores, rule evaluation trace, appeal history

    - Platform decisions:
      - Web: Leptos (Rust-native WASM) vs React (ecosystem) vs Svelte (lightweight)
      - Desktop: Tauri (Rust + web) vs terminal TUI (ratatui)
      - Recommendation: Leptos for tight Rust integration, compiles to WASM,
        single binary with embedded static assets

    - Knowledge Graph viz library evaluation:
      - D3.js force-directed (proven, flexible)
      - Cytoscape.js (graph-specific, good for large graphs)
      - vis.js Network (simpler, fast setup)
      - Recommendation: Cytoscape.js — built for graph data, supports
        compound nodes, typed edges, community coloring, search

  Track W3 (Observability — continued):
    - Config service (store/retrieve with change notifications)
    - Auth agent (credential management, scoped tokens)
    - Tree views (filtered by capabilities)
```

### Week 4: K8 GUI Prototype + Integration Testing

```
PARALLEL:
  Track W6 (K8 GUI — Build Phase):
    - Scaffold the web application
    - Implement Dashboard view with real-time kernel data
    - Implement Process Explorer with live updates
    - Implement Chain Viewer with event timeline
    - Wire kernel → GUI data flow:
      - Option A: WebSocket from kernel to browser
      - Option B: REST API polling
      - Option C: Kernel serves embedded static assets + WS endpoint
      - Recommendation: C — single binary, no separate server

  Track W8 (Client Onboarding — Design):
    - Design the conversational discovery flow:
      1. User installs lightweight WeftOS node
      2. Node presents a chat-like interface (web-based)
      3. Scripted questions gather environment info:
         - What tools/platforms are you using?
         - What are your main workflows?
         - What are your biggest pain points?
         - What systems generate the most manual work?
      4. Answers build knowledge graph nodes in real-time
      5. If permission granted: scan git repos, analyze documentation,
         map infrastructure
      6. Weaver runs analysis → produces gap report, coupling analysis,
         causal patterns, prediction of next problems
      7. Visual presentation of findings in Knowledge Graph view
    - This is the Weave-NN vision realized: the knowledge graph as a
      formal contract between the AI system and the human planners

  Track W5 (Test Hardening — continued):
    - Add daemon tests for clawft-weave (target: 30+ tests)
    - End-to-end integration test: boot → spawn agent → send message →
      receive response → governance check → chain event → shutdown

  Track W4 (Mesh — hardening):
    - Heartbeat-based failure detection (SWIM protocol activation)
    - Service advertisement across mesh
    - Chain state replication test (two nodes, split-brain recovery)
```

### Week 5: K8 Knowledge Graph View + External Analysis

```
PARALLEL:
  Track W6 (K8 GUI — Knowledge Graph):
    - Implement interactive knowledge graph visualization
    - Community coloring from label propagation results
    - Spectral partition overlay toggle
    - Node inspection panel (metadata, edges, causal chain)
    - Search and filter
    - Temporal playback: show graph evolution over time
    - Lambda_2 health indicator: "graph coherence" gauge

  Track W8 (Client Onboarding — Prototype):
    - Implement git-based codebase analysis for external repos:
      - Clone target repo
      - Run Weaver: git history ingestion, module dependency mapping
      - Compute coupling, burst patterns, change predictions
      - Generate gap report
    - Test on 2-3 real projects:
      - Mentra (glasses firmware) — fast embedded data
      - ClawStage (multiplayer) — conversational patterns
      - A public open-source project (test with unfamiliar codebase)
    - Validate: does the causal analysis find real patterns in projects
      the Weaver has never seen before?

  Track W7 (Deployment):
    - Dockerfile (multi-stage: build + runtime)
    - docker-compose.yml (single node + multi-node configs)
    - Installation guide (cargo install + from source + Docker)
    - Feature gate reference documentation
    - Configuration reference (weave.toml schema)
```

### Week 6: Polish + Gate Validation + Documentation

```
PARALLEL:
  Track W6 (K8 GUI — Polish):
    - Governance Console view
    - Mobile-responsive layout
    - Dark/light theme
    - Performance optimization for large graphs (1000+ nodes)
    - Embedded help / tooltips

  Track W7 (Documentation):
    - Operational runbook (monitoring, debugging, recovery)
    - Security model documentation (threat model, trust boundaries)
    - API reference generation (rustdoc hosted)
    - Deploy Fumadocs site

  GATE VALIDATION:
    - All 08a exit criteria checked (29 items)
    - All 08b core exit criteria checked (~22 items)
    - 08c config + auth checked (~14 items)
    - Mesh: two-node communication demonstrated
    - K8 GUI: 5 views functional with real kernel data
    - 200+ new tests added this sprint
    - Clippy clean, all feature gates verified
    - External codebase analysis produces meaningful results

  BUFFER:
    - Overflow from any track
    - Bug fixes discovered during integration
```

---

## A — Architecture

### K8: The GUI Layer

K8 is the seventh kernel layer: the human interface to WeftOS. It does not add
new kernel capabilities — it makes existing capabilities visible and interactive.

```
K8: GUI / Human Interface
  ├── Dashboard (system overview, health gauges)
  ├── Process Explorer (live process table)
  ├── Chain Viewer (ExoChain event timeline)
  ├── Knowledge Graph (interactive causal graph visualization)
  ├── Governance Console (live decision trace)
  └── Onboarding Flow (conversational discovery → knowledge graph)

K6: Mesh Networking
K5: Application Framework
K4: Container Integration
K3c: ECC Cognitive Substrate
K3: WASM Tool Sandbox / Governance
K2: Agent-to-Agent IPC
K1: Process Management / Supervisor
K0: Boot / Config / Daemon
```

### Knowledge Graph as Client Contract

The Weave-NN insight, realized in WeftOS:

```
CLIENT ENVIRONMENT                      WEFTOS ANALYSIS

Git repos ─────────────────────────→ Module dependency graph
                                      ↓
Documentation ─────────────────────→ Semantic embeddings (HNSW)
                                      ↓
Conversations (discovery calls) ──→ Causal graph nodes
                                      ↓
Infrastructure scans ──────────────→ System topology nodes
                                      ↓
                                    KNOWLEDGE GRAPH
                                      ↓
                              ┌───────┴────────┐
                              ↓                ↓
                        Causal Analysis    Predictions
                        (what caused       (what will
                         what)              change next)
                              ↓                ↓
                              └───────┬────────┘
                                      ↓
                              Client Report
                              (gaps, coupling,
                               automation opportunities,
                               2-year roadmap)
```

This graph becomes the formal contract: both the AI system and the human
planners reference the same causal model. Updates to the graph from either
side are chain-logged and tamper-evident.

### Iterative Hardening Strategy

Each real project stress-tests different WeftOS capabilities:

| Project | What It Tests | Unique Pressure |
|---------|---------------|-----------------|
| **clawft itself** | Self-analysis, Weaver, ECC | Recursive proof (already done) |
| **Mentra glasses** | Edge deployment, ARM64, mesh, fast data (video/audio/IMU) | Latency, memory constraints, sensor fusion |
| **ClawStage** | Multi-agent conversation, multiplayer, real-time | Concurrency, IPC throughput, topic routing |
| **Client engagements** | External codebase analysis, onboarding flow | Unfamiliar codebases, knowledge graph quality |

Run `weftos init` on each project. The Weaver should transfer learned patterns
(via WeaverKnowledgeBase) from clawft to new projects.

### Technology Choices (K8 GUI)

**Recommended stack:**

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Framework | **Leptos** (Rust → WASM) | Same language as kernel, compiles to single binary, SSR capable |
| Graph viz | **Cytoscape.js** | Purpose-built for graph data, handles 10K+ nodes, typed edges, community coloring |
| Charting | **Chart.js** or **Plotly.js** | Lightweight, works with Leptos via JS interop |
| Transport | **WebSocket** from kernel | Real-time push for live process/chain/metric updates |
| Packaging | **Embedded static assets** | `include_dir!()` macro — single `weft` binary serves the GUI |
| Desktop | **Tauri** (optional later) | Same web frontend wrapped in native window, if needed |

**Alternative if Leptos is too much scope:** Plain HTML + vanilla JS + WebSocket.
Ship a static `index.html` embedded in the binary. No build tooling needed.
This could be the Week 3-4 prototype, with Leptos as the Week 5-6 production version.

---

## R — Refinement

### Risk Register

| Risk | Mitigation |
|------|------------|
| K8 GUI scope creep — "one more view" | Hard limit: 5 views in Sprint 10. Everything else is Sprint 11. |
| Leptos learning curve | Fallback: plain HTML + JS prototype first. Leptos if time allows. |
| Mesh runtime integration harder than estimated | Prioritize single-hop message delivery first. Multi-hop, chain sync, DHT can be Sprint 11. |
| Persistence layer takes longer than expected | SQLite is the safe choice. RocksDB only if SQLite perf is insufficient. |
| External codebase analysis quality is poor | Start with git history only (proven on clawft). Add doc analysis incrementally. |
| 6 weeks isn't enough | Weeks 1-4 are the MVP. Weeks 5-6 are polish. Can ship at Week 4 if needed. |

### Sprint 10 Exit Criteria (Minimum)

- [ ] Crashed agent auto-restarts within 1 second
- [ ] Kernel state survives clean restart (chain, tree, graph persisted)
- [ ] `weave status` shows live metrics (process count, chain height, memory)
- [ ] Two nodes on same LAN discover each other and exchange a message
- [ ] K8 Dashboard loads in a browser showing real kernel data
- [ ] K8 Knowledge Graph view renders the clawft causal graph interactively
- [ ] External codebase analysis produces a gap report for a new project
- [ ] 200+ new tests added
- [ ] Docker image builds and boots a working kernel

### Sprint 10 Stretch Goals

- [ ] K8 Chain Viewer with event timeline and signature verification
- [ ] K8 Governance Console with live decision trace
- [ ] Conversational onboarding flow (chat-like discovery interface)
- [ ] Mentra project analyzed by Weaver (cross-project knowledge transfer)
- [ ] ClawStage integration test (multiplayer agent conversation)
- [ ] Tauri desktop wrapper for K8 GUI

---

## C — Completion

### Definition of Done

Sprint 10 is complete when:
1. Self-healing works (08a criteria validated)
2. Observability works (08b core criteria validated)
3. Data survives restart (persistence layer verified)
4. Two-node mesh demonstrated
5. K8 GUI shows 3+ views with real data (Dashboard, Process, Knowledge Graph)
6. External codebase analysis works on at least 1 project outside clawft
7. Docker deployment documented and tested
8. All new code has tests, clippy passes, feature gates verified

### What Ships After Sprint 10

| Item | Sprint |
|------|--------|
| K8 GUI: remaining views (Governance Console, advanced Chain Viewer) | Sprint 11 |
| Full mesh: multi-hop routing, DHT, chain replication | Sprint 11 |
| 25 remaining WASM tools | Sprint 11 |
| Conversational onboarding flow (production) | Sprint 11 |
| Tauri desktop packaging | Sprint 11 |
| Open source launch preparation | Sprint 11 |
| WASM-compiled shell (D10) | Sprint 12 |
| Blockchain anchoring (D12) | Post-1.0 |
| ZK proofs (D13) | Post-1.0 |
| Trajectory learning (D17) | Post-1.0 |
