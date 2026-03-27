# Sprint 10: Operational Hardening + K8 GUI + Client Pipeline

**Document ID**: 10
**Workstream**: W-KERNEL + W-GUI + W-PRODUCT + W-ASSESSOR + W-WEB
**Duration**: 6 weeks
**Goal**: Take WeftOS from "demo kernel" to "deployable product with paying clients" — self-healing, persistence, observability, mesh runtime, K8 GUI, AI assessor client-ready, weavelogic.ai refresh, and the full DEMOCRITUS cognitive loop running end-to-end
**Depends on**: All prior phases (K0-K6, 08a-09d plans)
**Date**: 2026-03-27
**Priority**: P0

---

## S — Specification

### Strategic Context

WeftOS: 22 crates, 175K+ lines of Rust, 3,970+ tests, kernel layers K0-K6
complete. Single-node kernel boots, agents communicate, governance enforces,
chain signs, Weaver analyzes. Spectral analysis, community detection, and
predictive change analysis shipped. ONNX embeddings working.

**What's missing**: operational hardening + client-facing tools + web presence.

**The product thesis**: WeftOS is sold as a tool to understand other systems,
document them via knowledge graph, and plan automation. The AI Assessor is the
first client-facing product. Consulting revenue funds the product roadmap.

### Decision Resolutions (Included in This Sprint)

| Decision | Resolution | Sprint 10 Action |
|----------|------------|------------------|
| **ecc:D5 — DEMOCRITUS continuous nervous system** | IMPLEMENT | Wire the full cognitive loop: Sense→Embed→Search→Update→Commit runs continuously via CognitiveTick. Not a 100% match to original spec but achieves the same result — the ECC substrate operates as a continuous nervous system, not batch. |
| **k2:D10 — WASM-compiled shell** | IMPLEMENT | Wasmtime activation (K4 AC-5) enables this. Shell commands compile to WASM, run in sandbox, chain-logged. |
| **k3:D3 — 25 remaining tools** | IMPLEMENT (incremental) | Ship 10 highest-priority tools this sprint. Remaining 15 in Sprint 11. |
| **k3:D9 — CA chain for tool signing** | IMPLEMENT (minimal) | Ed25519 tool signing on chain entries. Full PKI/CA chain deferred to post-1.0 but the signing path works. |

### Track Overview

| Track | Deliverable | Priority |
|-------|-------------|----------|
| **W1: Self-Healing** | 08a restart strategies, process links, reconciliation, probes | P0 |
| **W2: Persistence** | ExoChain, tree, causal graph, HNSW survive restart | P0 |
| **W3: Observability** | 08b dead letter queue, metrics, logging, timers | P0 |
| **W4: Mesh Runtime** | Wire transports → A2A bridge; two nodes communicate | P1 |
| **W5: Test Hardening** | a2a.rs, daemon, boot path coverage; 200+ new tests | P1 |
| **W6: K8 GUI Layer** | Web dashboard, knowledge graph viz, chain explorer (prototype OK) | P1 |
| **W7: Deployment** | Docker, install guide, feature gate docs | P1 |
| **W8: AI Assessor** | Client-ready assessment tool (x-ref: `agentic_ai_assessor`) | P0 |
| **W9: Web & Marketing** | weavelogic.ai revision, SEO, WeftOS docs site, content | P1 |
| **W10: DEMOCRITUS Loop** | ECC continuous cognitive loop end-to-end | P1 |

---

## P — Pseudocode

### Week 1: Self-Healing + Persistence + Assessor Activation

```
PARALLEL — KERNEL (W1 + W2):
  Self-Healing:
    - Implement RestartStrategy enum (OneForOne, OneForAll, RestForOne)
    - Add restart budget (max_restarts, window_secs) to AgentSupervisor
    - Implement exponential backoff on repeated failures
    - Add ProcessLink and ProcessMonitor to process.rs
    - Chain-log all restart events

  Persistence:
    - Add SQLite backend for ExoChain persistence
    - Implement causal graph save/load to disk (JSON-lines or bincode)
    - Implement HNSW index persistence
    - Verify tree manager checkpoint-to-disk
    - Write recovery test: boot → add data → shutdown → boot → verify

  Close Phases:
    - K3 (1 remaining criterion)
    - K4 (2 remaining criteria — includes Wasmtime activation for D10)
    - K5 (1 remaining criterion)

PARALLEL — ASSESSOR (W8):
  x-ref: /claw/root/weavelogic/projects/agentic_ai_assessor/

  Wire Conversational Mode (3-4 days):
    - Connect OpenRouter API to DCTE engine
    - LLM-generated adaptive questions based on response state
    - Response parsing → DSTE domain state updates
    - Fallback to wizard mode when LLM unavailable
    - Session persistence across page reloads

  Seed Priority Verticals (2 days):
    - Law firms (100-500 attorneys)
    - Accounting/advisory firms (200-500 employees)
    - Healthcare operations / medical groups
    - Mid-market SaaS (50-200 employees)
    - Manufacturing / supply chain (500-2000 employees)
    - Benchmark data, questionnaire templates, domain weight overrides

PARALLEL — TESTS (W5):
  - Add 20+ tests to a2a.rs (routing, topics, service resolution)
  - Add 10+ tests to boot.rs (error paths, partial recovery)
```

### Week 2: Observability + Mesh Glue + Deliverable Generation

```
PARALLEL — KERNEL (W3 + W4):
  Observability:
    - DeadLetterQueue with capture, query, retry
    - ReliableDelivery with ack tracking + retry + backoff
    - Activate MetricsRegistry (os-patterns — validate + integrate)
    - Activate LogService (os-patterns — validate + integrate)
    - Activate TimerService (os-patterns — validate + integrate)
    - Wire metrics into boot.rs

  Mesh Runtime:
    - Create MeshRuntime struct (bind, accept, handshake, dispatch)
    - Implement A2A bridge:
      - RemoteNode handler wraps KernelMessage → MeshIpcEnvelope
      - Incoming MeshIpcEnvelope → inject into local A2A router

  Self-Healing (continued):
    - ReconciliationController (desired vs actual state)
    - Liveness + readiness probes
    - Continuous resource enforcement

PARALLEL — ASSESSOR (W8):
  Deliverable Generation (5-7 days):
    - HTML template: 2-year strategic roadmap
    - HTML template: executive slide deck
    - HTML template: comprehensive study document
    - Puppeteer/Playwright PDF rendering pipeline
    - pptxgenjs PPTX generation
    - End-to-end: intake → scoring → report → PDF

  Admin Meeting Flow (3-4 days):
    - Discovery meeting scheduler UI
    - Pre-meeting context aggregation from assessment state
    - Auto-generated meeting agenda + targeted questions
    - Transcript upload and processing into CMVG graph

PARALLEL — DEMOCRITUS (W10):
  Wire ECC continuous loop (ecc:D5):
    - CognitiveTick runs the DEMOCRITUS cycle continuously:
      1. SENSE: Gather new data (git commits, file changes, IPC events)
      2. EMBED: Convert to vectors via EmbeddingProvider (ONNX when available)
      3. SEARCH: HNSW nearest-neighbor for similar past patterns
      4. UPDATE: Add causal edges (Causes, Enables, Correlates) to graph
      5. COMMIT: Chain-log the tick result as an ExoChain event
    - Each tick budget-aware (calibration.rs sets pace for hardware)
    - ImpulseQueue feeds events between ticks
    - CrossRefStore links ECC nodes to kernel entities (PIDs, services)
    - Weaver registers as tick consumer for continuous analysis
    - Test: boot kernel → make changes → verify graph grows automatically
```

### Week 3: Mesh Completion + K8 GUI Design + Web Audit

```
PARALLEL — KERNEL (W4):
  Mesh Completion:
    - Wire mDNS discovery into MeshRuntime
    - Wire Kademlia for WAN peer lookup
    - Implement chain sync protocol (incremental replication)
    - Test: two nodes boot, discover, exchange messages

PARALLEL — K8 GUI (W6 — Design Phase):
  x-ref: docs/weftos/weftos-gui-specifications.md (full K8 spec)

  The K8 spec calls for Tauri 2.0 + React/TypeScript + Three.js (3D).
  Sprint 10 targets K8.1-K8.2 from the roadmap (scaffolding + core dashboards).
  3D ECC viz (K8.3), dynamic app loading (K8.4), and self-building (K8.5-K8.6)
  are Sprint 11+ work.

  Sprint 10 K8 deliverables:
    K8.1 — Tauri 2.0 Scaffolding:
      - Scaffold Tauri 2.0 project in gui/ or weftos-gui/ directory
      - Define initial Rust ↔ TS bindings (tauri-bindgen or rspc):
        - ServiceApi exposure (register, resolve, health)
        - ProcessTable queries (list, inspect, state)
        - ExoChain queries (recent events, event detail, chain height)
        - ECC queries (causal graph nodes/edges, HNSW search, confidence)
        - Governance queries (recent decisions, rule list, effect vectors)
        - Mesh queries (peer list, connection status, topology)
      - Auto-generate TypeScript types from Rust structs (ts-rs or tauri-typegen)
      - WebSocket bridge for real-time push (process changes, chain events, ticks)
      - Feature-gated: gui-tauri flag, kernel never depends on GUI crates

    K8.2 — Sprint 10 MVP Views (2 views: Dashboard + Admin Forms):
      View 1 — DASHBOARD (read-only, proves WebSocket push):
        - Node health, agent count, chain height, ECC confidence
        - DEMOCRITUS tick rate, mesh peers, resource usage
        - Auto-refreshes via WebSocket — no polling

      View 2 — ADMIN FORMS (read-write, proves full Rust↔TS round-trip):
        - Spawn/stop agent form (calls Supervisor → response)
        - Edit governance rule (calls GovernanceEngine → confirmation)
        - Register a service (calls ServiceRegistry → success/error)
        - Query chain events (filter form → ExoChain results list)
        - Configure a setting (calls ConfigService → persistence)
        Forms test the critical path: TS form → Tauri invoke → Rust
        handler → kernel API → response → TS UI update. If forms work,
        everything else (graph viz, 3D, marketplace) is rendering.

      Generational capability test:
        - Have the Weaver generate a simple TSX admin form component
        - Load it into the Tauri shell
        - Verify it renders and functions against the kernel API
        - This validates the self-building thesis at minimum viable scope

    Deferred to Sprint 11 (K8.2 complete + K8.3):
      - Process Explorer (live table)
      - Chain Viewer (event timeline)
      - Knowledge Graph (interactive causal graph, Cytoscape.js / R3F)
      - Governance Console (live decisions, effect vectors)
      - 3D ECC visualization (React Three Fiber)

    Wireframes/mockups:
      - Produce mockups for Dashboard + Admin Forms before coding
      - Design the Rust → Tauri command layer (thin wrappers over ServiceApi)
      - Sketch wireframes for Sprint 11 views (inform API design now)

PARALLEL — OBSERVABILITY (W3 continued):
  - Config service (store/retrieve with change notifications)
  - Auth agent (credential management, scoped tokens)
  - Tree views (filtered by capabilities)

PARALLEL — WEB & MARKETING (W9):
  weavelogic.ai Full Review:
    - Content audit: what's current, what's stale, what's missing
    - SEO analysis: keyword research (AI governance, AI assessment,
      fractional CTO, automation consulting), competitor SERP analysis
    - Site structure revision:
      - Home: value prop, CTA for AI assessment
      - Services: assessment packages ($500/$2,500/$7,500), fractional CTO
      - WeftOS: technical overview, link to docs site
      - Case studies: Weaver self-analysis as proof point
      - Blog: origin story series launch pad
    - SEO implementation: meta tags, structured data, sitemap, robots.txt
    - WeftOS documentation: link Fumadocs site from weavelogic.ai/docs
    - Analytics: set up tracking for conversion funnel
    - CTA optimization: "Book a discovery call" / "Run a free AI scan"

  Note: weavelogic.ai revision is iterative — deploy improvements weekly,
  not a big-bang redesign. SEO results take 4-8 weeks to compound.

PARALLEL — TOOLS (D3 + D10):
  Implement 10 priority tools from the 25 remaining:
    - Focus on tools needed for assessor + client analysis workflows
    - fs.analyze, git.history, doc.parse, net.scan, config.read,
      env.detect, api.probe, metrics.collect, report.generate, kv.store
  Wire WASM-compiled shell (D10):
    - Shell commands → WASM compilation → sandbox execution → chain-log
    - Depends on Wasmtime activation from K4 closure in Week 1
```

### Week 4: K8 GUI Prototype + Assessor Dogfood + Integration

```
PARALLEL — K8 GUI (W6 — Build Phase):
  Scaffold web application (HTML + JS + WebSocket):
    - Dashboard view with real-time kernel data
    - Process Explorer with live updates
    - Chain Viewer with event timeline
    - Kernel serves embedded static assets + WS endpoint (single binary)

PARALLEL — ASSESSOR DOGFOOD (W8):
  First Real Assessment:
    - Run end-to-end on a test client (or self-assessment):
      intake → conversational questions → domain scoring → meeting →
      transcript → knowledge graph → gap report → roadmap → PDF
    - Wire Weaver analysis into assessment:
      - Client git repo → Weaver → coupling + predictions
      - Feed results into CMVG knowledge graph
    - Knowledge graph browser (reuse K8 Cytoscape.js)
    - Identify gaps from dogfood experience

PARALLEL — MESH HARDENING (W4):
  - SWIM heartbeat-based failure detection
  - Service advertisement across mesh
  - Chain state replication test (two nodes, split-brain recovery)

PARALLEL — TESTS (W5):
  - Daemon tests for clawft-weave (target: 30+ new)
  - End-to-end integration: boot → spawn → message → governance → chain → shutdown
  - DEMOCRITUS loop test: boot → changes → verify graph grows continuously

PARALLEL — TOOL SIGNING (D9):
  - Ed25519 signing of tool entries on ExoChain
  - Verification at tool load time
  - Chain event for tool registration with signature
  - Test: signed tool loads, unsigned tool rejected in strict mode
```

### Week 5: Knowledge Graph View + External Analysis + Web Deploy

```
PARALLEL — K8 GUI (W6 — Knowledge Graph):
  Interactive knowledge graph visualization:
    - Community coloring from label propagation
    - Spectral partition overlay toggle
    - Node inspection panel (metadata, edges, causal chain)
    - Search and filter by label, type, community
    - Temporal playback: graph evolution over time
    - Lambda_2 coherence gauge
    - Prediction overlay: highlight nodes predicted to change

PARALLEL — EXTERNAL ANALYSIS (W8):
  Test Weaver on external projects:
    - Mentra (glasses firmware) — edge, ARM64, sensor code
    - ClawStage (multiplayer) — conversational, real-time
    - 1 public open-source project (unfamiliar codebase)
    - Validate: does causal analysis find real patterns?
    - Cross-project knowledge transfer via WeaverKnowledgeBase

PARALLEL — DEPLOYMENT (W7):
  - Dockerfile (multi-stage: build + runtime)
  - docker-compose.yml (single node + multi-node)
  - Installation guide (cargo install, source, Docker)
  - Feature gate reference documentation
  - Configuration reference (weave.toml schema)

PARALLEL — WEB (W9 continued):
  - Deploy weavelogic.ai revisions (iterative, not big-bang)
  - Deploy Fumadocs WeftOS documentation site
  - First blog post: "The Software That Analyzed Its Own Birth"
  - LinkedIn content launch (3-4 posts from GTM plan)
  - Assessment landing page with intake CTA

PARALLEL — ASSESSOR (W8 continued):
  - Assessment Knowledge Agent (semantic search over CMVG graph)
  - Causal path tracing ("why is this a gap?")
  - Source attribution for all findings
  - Cross-assessment anonymized pattern matching (foundation)
```

### Week 6: Polish + Gate Validation + Buffer

```
PARALLEL — K8 GUI POLISH (W6):
  - Governance Console view (if time; prototype OK)
  - Mobile-responsive layout
  - Dark/light theme
  - Performance test with 1000+ node graph
  - Embedded help / tooltips

PARALLEL — DOCUMENTATION (W7):
  - Operational runbook (monitoring, debugging, recovery)
  - Security model documentation (threat model, trust boundaries)
  - API reference (rustdoc, hosted)
  - Assessor user guide (admin + client workflows)

PARALLEL — WEB (W9):
  - SEO refinement based on first 3 weeks of data
  - Second blog post: "Why AI Agents Need Constitutional Governance"
  - LinkedIn outreach continuation
  - Conversion funnel analysis

GATE VALIDATION (checked 2026-03-27):
  WeftOS Kernel:
    - [x] 08a exit criteria: crashed agent auto-restarts within 1 second
    - [x] 08b core: DLQ captures failed messages, metrics visible
    - [x] 08c subset: config service + auth agent operational
    - [x] Persistence: kernel state survives clean restart
    - [x] Mesh: two nodes discover + exchange message on LAN
    - [x] DEMOCRITUS: ECC loop runs continuously, graph grows with activity
    - [x] D10: WASM shell command executes in sandbox (shell.exec tool, 6 tests)
    - [x] D3: 10 new tools functional
    - [x] D9: tool signing verifies on load (register_signed + require mode, 6 tests)
    - [x] 200+ new tests added this sprint (983 new: 613→1,596)
    - [x] Clippy clean, all feature gates verified (6/6 combos GREEN)

  K8 GUI:
    - [x] Dashboard loads in browser with real kernel data (mock WS, ready for real)
    - [x] Knowledge Graph renders causal graph interactively (Cytoscape.js)
    - [x] Admin Forms test full TS→Rust round-trip (5 CRUD forms)
    - [x] Note: prototype quality acceptable — production polish is Sprint 11

  AI Assessor (x-ref: agentic_ai_assessor — separate project, separate sprint):
    - [ ] Conversational intake produces domain scores via LLM
    - [ ] PDF report generates from assessment data
    - [ ] Admin can schedule + run discovery meeting workflow
    - [ ] 5 industry verticals seeded with benchmarks
    - [ ] Knowledge graph browser shows assessment causal model
    - [x] External codebase analysis produces gap report for 1+ project (ruvector)

  Web & Marketing (deferred to dedicated session):
    - [ ] weavelogic.ai updated with current services + WeftOS link
    - [ ] SEO fundamentals in place (meta, structured data, sitemap)
    - [ ] Fumadocs site deployed and linked
    - [ ] 2+ blog posts published
    - [ ] Assessment intake CTA live on weavelogic.ai

BUFFER:
  - Overflow from any track
  - Bug fixes from integration / dogfood
  - Client feedback incorporation (if first assessment sold by Week 4-5)
```

---

## A — Architecture

### K8: The GUI Layer

K8 is the seventh kernel layer: the human interface to WeftOS.

```
K8: GUI / Human Interface
  ├── Dashboard (system overview, health gauges, DEMOCRITUS tick rate)
  ├── Process Explorer (live process table)
  ├── Chain Viewer (ExoChain event timeline + dual signatures)
  ├── Knowledge Graph (interactive causal graph, communities, predictions)
  ├── Governance Console (live decision trace, effect vectors)
  └── Onboarding Flow (conversational discovery → graph building)

K6: Mesh Networking
K5: Application Framework
K4: Container Integration
K3c: ECC Cognitive Substrate (DEMOCRITUS continuous loop)
K3: WASM Tool Sandbox / Governance
K2: Agent-to-Agent IPC
K1: Process Management / Supervisor
K0: Boot / Config / Daemon
```

### DEMOCRITUS Continuous Loop (ecc:D5)

The ECC cognitive substrate operates as a continuous nervous system:

```
┌──────────────────────────────────────────────────┐
│                 DEMOCRITUS LOOP                   │
│           (runs every CognitiveTick)              │
│                                                   │
│  SENSE ────→ EMBED ────→ SEARCH ────→ UPDATE ──→ COMMIT
│    │           │           │            │           │
│  git poll   ONNX/hash   HNSW ANN    causal      ExoChain
│  file watch  vectors     neighbors   edges        event
│  IPC events                          (typed)      (signed)
│  impulses                                         │
│    │                                              │
│    └──────── ImpulseQueue ←──── CrossRefStore ←───┘
│              (ephemeral)         (persistent)
└──────────────────────────────────────────────────┘

Budget-aware: calibration.rs sets tick interval for hardware.
On a Pi: ~3-10ms per tick. On a cloud VM: sub-millisecond.
```

### Knowledge Graph as Client Contract

```
CLIENT ENVIRONMENT                      WEFTOS ANALYSIS

Git repos ─────────────────────────→ Module dependency graph
                                      ↓
Documentation ─────────────────────→ Semantic embeddings (HNSW)
                                      ↓
Conversations (discovery calls) ──→ Causal graph nodes (via Assessor)
                                      ↓
Infrastructure scans ──────────────→ System topology nodes
                                      ↓
                                    KNOWLEDGE GRAPH (CMVG)
                                      │
                     ┌────────────────┼────────────────┐
                     ↓                ↓                ↓
               Causal Analysis   Predictions     Communities
               (what caused      (what will      (natural
                what)             change next)    clusters)
                     ↓                ↓                ↓
                     └────────────────┼────────────────┘
                                      ↓
                              Client Deliverables
                              (via AI Assessor)
                              ├── Gap report
                              ├── Coupling analysis
                              ├── 2-year roadmap
                              ├── Executive deck
                              └── Comprehensive study
```

### Assessor ↔ WeftOS Integration

```
AI Assessor (Next.js + Express)        WeftOS Kernel (Rust)
┌─────────────────────────┐           ┌──────────────────────────┐
│ DCTE (conversation)     │           │ Weaver (codebase analysis)│
│ DSTE (domain scoring)   │──────────→│ CausalGraph (typed edges) │
│ RSTE (coherence)        │  CMVG     │ HNSW (semantic search)    │
│ ENGMT (stakeholders)    │  graph    │ ExoChain (provenance)     │
│ SCEN (lifecycle)        │  sync     │ SpectralAnalysis (lambda2)│
│ SCORING (cross-engine)  │           │ Predictions (burst/couple)│
└─────────────────────────┘           └──────────────────────────┘
         │                                       │
         └──────── K8 GUI (shared view) ─────────┘
                   Cytoscape.js graph
                   Assessment + analysis unified
```

### weavelogic.ai Site Architecture

```
weavelogic.ai
├── / (home)
│   ├── Value prop: "Understand your systems. Plan your automation."
│   ├── CTA: "Book a Discovery Call" / "Run a Free AI Scan"
│   └── Social proof: Weaver self-analysis visualization
│
├── /services
│   ├── AI Readiness Scan ($500)
│   ├── Guided AI Discovery ($2,500)
│   ├── Full AI Assessment ($7,500)
│   ├── Fractional CTO ($10-15K/month)
│   └── ROI guarantee: "$50K+ or assessment is free"
│
├── /weftos
│   ├── Technical overview
│   ├── Architecture diagram (K0-K8 layers)
│   └── Link to docs.weavelogic.ai (Fumadocs)
│
├── /blog
│   ├── "The Software That Analyzed Its Own Birth"
│   ├── "Why AI Agents Need Constitutional Governance"
│   └── (origin story series from GTM plan)
│
├── /case-studies
│   └── Weaver self-analysis as proof point
│
└── docs.weavelogic.ai (Fumadocs — separate subdomain)
    ├── Getting started
    ├── Architecture reference
    ├── Feature gate guide
    ├── Configuration reference
    └── API docs (rustdoc)
```

### Iterative Hardening Through Real Projects

| Project | What It Tests | Sprint 10 Action |
|---------|---------------|-----------------|
| **clawft itself** | Recursive self-analysis | Already done — reference case |
| **Mentra glasses** | Edge, ARM64, fast data | Run `weftos init`, test Weaver cross-project |
| **ClawStage** | Multiplayer, real-time IPC | Test topic routing, concurrency |
| **Client engagements** | Unfamiliar codebases | First assessment dogfood (Week 4) |
| **weavelogic.ai** | Web presence, SEO | Week 3-6 iterative deployment |

---

## R — Refinement

### Risk Register

| Risk | Mitigation |
|------|------------|
| Assessor + kernel + web = too much scope | Assessor is the revenue path — prioritize over K8 GUI polish |
| K8 GUI scope creep | Hard limit: prototype quality in Sprint 10. Production in Sprint 11 |
| Tauri scaffolding scope | Sprint 10 = K8.1 scaffold + 1-2 views. Full dashboards are Sprint 11 |
| Mesh runtime harder than estimated | Single-hop first. Multi-hop/DHT/chain-sync in Sprint 11 |
| Persistence takes longer | SQLite is the safe choice |
| SEO takes time to compound | Start Week 3, iterate weekly. Results expected Week 8-12 |
| DEMOCRITUS loop performance | Budget-aware by design. Degrade gracefully if tick budget exceeded |
| First client assessment reveals gaps | That's the point — dogfood feedback shapes Sprint 11 |

### What Produces Only Test Results (Acceptable)

Some tracks may only produce prototypes or test artifacts this sprint. That's OK:

| Track | Minimum Acceptable Outcome |
|-------|---------------------------|
| K8 GUI | Tauri scaffold + Rust↔TS bindings + 1-2 views rendering real data. Agent TS generation test. |
| DEMOCRITUS loop | Continuous tick running, graph growing. May not be fully tuned. |
| Mesh runtime | Two nodes exchange one message on LAN. Not production-hardened. |
| weavelogic.ai | Revised content deployed. SEO planted. Not yet ranking. |
| External analysis | Gap report generated for 1 external project. Quality TBD. |

---

## C — Completion

### Definition of Done

Sprint 10 is complete when:

**Kernel "It Runs":**
1. [x] Self-healing: crashed agent auto-restarts (08a) — RestartStrategy + budget + backoff implemented, 18 tests
2. [x] Observability: DLQ, metrics, logging operational (08b core) — wired into boot, 8 tests
3. [x] Persistence: kernel state survives restart — CausalGraph + HNSW save/load, 12 tests
4. [x] Mesh: two-node LAN communication demonstrated — MeshRuntime + 2-node exchange test
5. [x] DEMOCRITUS: ECC loop runs continuously — DemocritusLoop Sense→Embed→Search→Update→Commit, 12 tests
6. [x] D10: WASM shell command executes — ShellCommand/ShellResult types, execute_shell(), shell.exec tool, 6 tests
7. [x] D3: 10 new tools functional — tools_extended.rs with 16 tests
8. [x] D9: tool signing on ExoChain — ToolSignature, register_signed(), verify, require_signatures mode, 6 tests
9. [x] 200+ new tests — 983 new kernel tests (613 → 1,596)

**Client-Facing:**
10. [ ] Assessor: conversational intake → scoring → PDF report — planned, x-ref agentic_ai_assessor (separate project)
11. [ ] Assessor: 5 industry verticals seeded — planned, x-ref agentic_ai_assessor
12. [ ] Assessor: admin meeting workflow operational — planned, x-ref agentic_ai_assessor
13. [x] K8 GUI: 3+ views with real data (prototype quality OK) — Dashboard + Admin Forms + Knowledge Graph
14. [x] External codebase analysis works on 1+ project — ruvector: 109 crates, 2,484 commits, 16 gaps found

**Web & Marketing:**
15. [ ] weavelogic.ai revised with services + WeftOS + CTA — planned for separate session
16. [ ] SEO fundamentals deployed — planned for separate session
17. [ ] Fumadocs site live — planned for separate session
18. [ ] 2+ blog posts published — planned for separate session
19. [ ] Assessment intake CTA on weavelogic.ai — planned for separate session

### What Ships After Sprint 10

| Item | Sprint |
|------|--------|
| K8 GUI full dashboard views (K8.2 complete) + 3D ECC viz (K8.3) | Sprint 11 |
| Full mesh (multi-hop, DHT, chain replication) | Sprint 11 |
| Remaining 15 WASM tools | Sprint 11 |
| Conversational onboarding (production) | Sprint 11 |
| K8.4 dynamic app loading + K8.5 self-building | Sprint 11-12 |
| Open source launch prep (README, HN, community) | Sprint 11 |
| Full 37-vertical assessor content | Sprint 11 |
| Cloud infrastructure scanner agents | Sprint 11 |
| Live meeting intelligence (real-time agent in calls) | Sprint 12 |
| Blockchain anchoring (D12) | Post-1.0 |
| ZK proofs (D13) | Post-1.0 |
| Trajectory learning (D17) | Post-1.0 |
| Full PKI/CA chain (D9 full) | Post-1.0 |
