# Track 6: Mentra Integration & Network Navigator

**Date**: 2026-03-28
**Chair**: mentra-planner
**Duration**: 45 min
**Status**: Symposium Output (Sprint 11)

**Panelists**: mentra-planner, mentra-ui, mentra-ux, mentra-development, mentra-tester, mesh-engineer, kernel-architect, ecc-analyst

---

## Opening Statement (mentra-planner)

This track defines how WeftOS becomes the intelligence layer for Mentra smart glasses. The core insight is that WeftOS already has the exact architecture Mentra needs: the JSON descriptor system (Section 9 of the GUI design notes) renders the same block definition to multiple targets, and "Mentra HUD" is already listed as a first-class render target alongside React, Terminal, 3D, Voice, MCP, and PDF. The kernel's ServiceApi, A2ARouter, CausalGraph, and HNSW search are the backend services that make glasses genuinely useful rather than just a notification display.

We focus on what is practical for Sprint 11-14 (WeftOS 0.2) versus what requires hardware partnership milestones.

---

## Analysis 1: How WeftOS Serves Mentra

### Deployment Model Decision

**kernel-architect**: The WeftOS kernel does NOT run on the glasses. The BES2700 (ARM Cortex-M33 + Cortex-A7) has 8MB PSRAM and runs MentraOS (RTOS). WeftOS is a Rust kernel requiring Tokio, allocators, and HNSW indexes that demand 50-200MB minimum. The correct architecture is:

```
DECISION: WeftOS runs cloud-side (or on a tethered phone).
           Mentra glasses are a thin terminal/HUD that renders
           JSON descriptors received from a remote WeftOS instance.
```

This matches Q9 from the ECC symposium: "mentra-cortex IS a kernel instance (Kernel\<AndroidPlatform\>)" -- meaning the phone/companion device runs the kernel, not the glasses themselves.

### The Rendering Pipeline

The JSON descriptor architecture from Section 9 of the GUI design notes is the integration point. One descriptor, multiple renderers:

```
                    JSON Descriptor (Lego Block)
                             |
          +------------------+------------------+
          |                  |                  |
     React Renderer    Terminal Renderer   Mentra HUD Renderer
     (Tauri desktop)   (Ink/xterm.js)     (400x240 constraint)
          |                  |                  |
     Full desktop GUI   CLI rich output    Glasses display
```

**mentra-ui**: The Mentra HUD renderer is a constraint-driven layout engine. It takes the same JSON descriptor and applies these hard limits:

| Constraint | Value | Rationale |
|-----------|-------|-----------|
| Resolution | 400x240 px | Mentra display hardware |
| Color depth | Monochrome green on black (or limited palette) | OLED power, readability |
| Font size | Minimum 16px equivalent | Readable at arm's length |
| Max text lines | 8-10 lines visible | Usable information density |
| Interaction | Voice only (no touch) | Glasses form factor |
| Refresh rate | 2-5 Hz for data, 30 Hz for animations | Battery life |

### Kernel Services Most Useful on Glasses

**ecc-analyst**: Ranked by utility-to-latency ratio for a HUD:

| Priority | Service | Kernel API | HUD Use Case |
|----------|---------|------------|--------------|
| 1 | Health/Status | `HealthSystem::overall()` | Always-on system heartbeat |
| 2 | Process List | `ProcessTable::list()` | Agent status at a glance |
| 3 | Alerts | `TopicRouter::subscribe("alerts")` | Push notifications |
| 4 | HNSW Search | `HnswService::search()` | Voice "find similar to X" |
| 5 | Mesh Topology | `ClusterMembership::peers()` | Network Navigator view |
| 6 | Chain Events | `ChainManager::tail()` | Recent governance decisions |
| 7 | Weaver Analysis | `ServiceApi::call("weaver.analyze")` | Code analysis summaries |

---

## Analysis 2: Network Navigator on Glasses

### The Core Challenge

**mesh-engineer**: "Show me the network" on a 400x240 display cannot be the full force-directed 3D graph from the desktop GUI. We need a purpose-built topology view.

### HUD Topology View Design

**mentra-ui**: The Network Navigator on HUD uses a radial layout, not force-directed:

```
+----------------------------------------+
|  NETWORK: 5 peers  |  latency: 12ms   |
|----------------------------------------|
|                                        |
|              [YOU]                      |
|             /  |  \                     |
|           /    |    \                   |
|        [P1]  [P2]  [P3]               |
|        edge  cloud  edge               |
|         OK    OK    WARN               |
|                                        |
|            [P4]---[P5]                 |
|            cloud   edge                |
|             OK     DOWN                |
|                                        |
|----------------------------------------|
|  "detail peer-3" | "refresh" | "back" |
+----------------------------------------+
```

Design rules for the HUD topology:
- Max 8 nodes visible (pagination beyond that)
- Each node: 1-line label (name + type + status icon)
- Color coding: green=OK, yellow=WARN, red=DOWN
- Central node is always "self"
- Edges shown as ASCII lines; thickness not possible at this resolution
- Voice commands: "detail [peer]", "refresh", "back", "show services on [peer]"

### JSON Descriptor for Network Navigator

```json
{
  "root": "net-nav",
  "meta": { "target_hint": "mentra-hud", "refresh_hz": 2 },
  "elements": {
    "net-nav": {
      "type": "Column",
      "children": ["header", "topo-graph", "voice-hints"]
    },
    "header": {
      "type": "StatusBar",
      "props": {
        "left": { "$state": "/kernel/cluster/peer_count", "format": "NETWORK: {v} peers" },
        "right": { "$state": "/kernel/cluster/avg_latency_ms", "format": "latency: {v}ms" }
      }
    },
    "topo-graph": {
      "type": "RadialTopology",
      "props": {
        "center": { "$state": "/kernel/cluster/self_node" },
        "peers": { "$state": "/kernel/cluster/peers" },
        "max_visible": 8,
        "layout": "radial"
      },
      "on": {
        "voice_select": {
          "action": "kernel_exec",
          "params": { "command": "weftos cluster peer-detail {selected}" }
        }
      }
    },
    "voice-hints": {
      "type": "HintBar",
      "props": {
        "hints": ["detail [name]", "refresh", "back"]
      }
    }
  }
}
```

The same descriptor on the desktop renders as a full interactive graph with drag, zoom, and tooltips. On HUD it renders as the constrained radial view above.

### Peer Discovery Indicators

**mesh-engineer**: The always-on ambient display (outside of full views) should show a persistent mesh indicator:

```
+------------------------------------------+
|  [5 peers] [12ms] [3 agents]  11:42 AM  |
+------------------------------------------+
```

This is a `StatusBar` Lego block with `$state` bindings to `/kernel/cluster/peer_count`, `/kernel/cluster/avg_latency_ms`, and `/kernel/processes/active_count`. It lives in the Mentra "ambient layer" -- visible when no full view is active.

---

## Analysis 3: WeaveLogic Tools on Mentra

### AI Assessor Through Voice

**mentra-ux**: The AI Assessor interaction model for glasses:

```
VOICE FLOW: AI Assessment

User: "Assess this project"
  |
  v
WeftOS: kernel_exec("weaver.assess --target .")
  |
  v
HUD shows: "Assessing... analyzing 47 files"
  |  (streaming progress via A2UI updateDataModel)
  v
HUD shows summary card:
+----------------------------------------+
|  ASSESSMENT: myproject                 |
|----------------------------------------|
|  Health:     [========  ] 78%          |
|  Test cover: [======    ] 62%          |
|  Tech debt:  14 items (3 critical)     |
|  Security:   2 warnings                |
|                                        |
|  Top issue: auth module missing        |
|  input validation (line 142-168)       |
|                                        |
|----------------------------------------|
|  "details" | "fix top" | "full report" |
+----------------------------------------+
```

Voice follow-ups:
- "Details" -- paginate through findings
- "Fix top issue" -- triggers Weaver fix workflow, shows diff summary on HUD
- "Full report" -- generates PDF descriptor, sends to phone/email

### Knowledge Graph Navigation

**ecc-analyst**: The CausalGraph and HNSW search are the most powerful glasses features. Voice-driven semantic search through the knowledge graph:

```
User: "What caused the auth failure last Tuesday?"
  |
  v
WeftOS: hnsw_search("auth failure") + causal_graph.query(time_range, edges=Causes)
  |
  v
HUD: Causal chain display (linearized for HUD)
+----------------------------------------+
|  CAUSAL CHAIN: auth failure            |
|----------------------------------------|
|  1. PR #47 merged (Mon 14:22)          |
|     Changed: auth/validate.rs          |
|  2. Deploy to staging (Mon 15:01)      |
|     Triggered: CI pipeline             |
|  3. Auth timeout errors (Tue 02:14)    |
|     5,247 events in 30 min             |
|  4. Auto-rollback (Tue 02:18)          |
|     Restored: v0.9.3                   |
|                                        |
|----------------------------------------|
|  "expand 1" | "show diff" | "back"    |
+----------------------------------------+
```

This is a killer feature. The CMVG causal graph turns "what happened?" into a navigable timeline on the HUD.

### Codebase Analysis via Weaver

```
User: "Analyze this codebase"
  |
  v
WeftOS: weaver.analyze --scope full --output summary
  |
  v
HUD: Progressive summary (streams as analysis completes)
+----------------------------------------+
|  WEAVER ANALYSIS                       |
|----------------------------------------|
|  Language:  Rust (87%) + TS (13%)      |
|  Crates:    22 | Tests: 4,970+         |
|  Binary:    8.2 MB | WASM: 98 KB      |
|                                        |
|  Architecture: Microkernel             |
|  Patterns: ServiceApi, A2ARouter,      |
|    GovernanceGate, ExoChain            |
|                                        |
|  Recommendation: Extract 3 patterns    |
|  from clawft-kernel (>2000 lines)      |
|----------------------------------------|
|  "patterns" | "debt" | "full" | "back" |
+----------------------------------------+
```

---

## Analysis 4: Platform Integration Architecture

### Connection Stack

**mentra-development**: The full connection path:

```
+-------------------+
| Mentra Glasses    |
| BES2700           |
| MentraOS (RTOS)   |
| - Voice capture   |
| - HUD renderer    |
| - BT/WiFi radio   |
+--------+----------+
         | BLE 5.0 / WiFi
         v
+-------------------+
| Companion Device  |   (Phone or laptop -- optional relay)
| MentraOS App      |
| - Audio processing|
| - Descriptor cache|
| - Offline mode    |
+--------+----------+
         | HTTPS / WebSocket
         v
+-------------------+
| MentraOS AppServer|   (Cloudflare Workers)
| - Auth / sessions |
| - WebSocket relay |
| - Descriptor cache|
| - Rate limiting   |
+--------+----------+
         | WebSocket (persistent)
         v
+-------------------+
| WeftOS Kernel     |   (Cloud instance or on-prem)
| - ServiceApi      |
| - A2ARouter       |
| - GovernanceGate  |
| - CausalGraph     |
| - HNSW search     |
| - Weaver          |
+--------+----------+
         | A2A / MCP
         v
+-------------------+
| OpenClaw Gateway  |
| - Claude API      |
| - Tool execution  |
+-------------------+
```

### Transport Protocol Selection

**mentra-development**: We need two protocols for different segments:

| Segment | Protocol | Rationale |
|---------|----------|-----------|
| Glasses to AppServer | WebSocket over WiFi (primary) or BLE GATT (fallback) | Low overhead, bidirectional, existing MentraOS support |
| AppServer to WeftOS | WebSocket with A2UI streaming protocol | Matches Section 9 architecture: `createSurface` / `updateComponents` / `updateDataModel` |
| WeftOS to OpenClaw | MCP over HTTP | Existing integration, stateless |

The A2UI protocol from Google's a2ui project (referenced in Section 9) is ideal for the AppServer-to-WeftOS segment because it handles:
- Surface lifecycle (create/destroy views)
- Incremental data updates (only changed `$state` values pushed)
- Transport-agnostic (works over WebSocket, A2A, or MCP)

### Latency Budget

**mentra-ux**: End-to-end latency budget for voice commands:

```
LATENCY BUDGET: Voice Command -> HUD Response

Phase                          Target    Max
---------------------------------------------
Voice capture + VAD            50ms      100ms
Audio to glasses BLE buffer    20ms      50ms
BLE/WiFi to companion          30ms      80ms
Companion to AppServer         40ms      100ms
AppServer to WeftOS            20ms      50ms
WeftOS command execution       100ms     500ms
  (simple: 10-50ms)
  (HNSW search: 15-50ms)
  (Weaver analysis: 1-5s async)
Response to AppServer          20ms      50ms
AppServer to companion         40ms      100ms
Companion to glasses           30ms      80ms
HUD render                     10ms      30ms
---------------------------------------------
TOTAL (simple command)         360ms     640ms
TOTAL (search command)         400ms     800ms
TOTAL (analysis command)       1-5s      (async with progress)
```

Target: simple commands under 500ms perceived latency. Analysis commands show immediate "working..." with streaming progress.

### Offline Capability

**mentra-development**: What works without a network connection:

| Feature | Offline Support | How |
|---------|----------------|-----|
| Cached status display | Yes | Last-known state in companion cache |
| Cached topology view | Yes | Last snapshot of peer list |
| Voice command parsing | Partial | On-device wake word + basic commands; complex NLU needs cloud |
| HNSW search | No | Requires kernel |
| Weaver analysis | No | Requires kernel + LLM |
| Alert history | Yes | Cached on companion |
| Time/date/battery | Yes | Local |

The companion device maintains a descriptor cache with TTL. Cached descriptors render with stale data indicators:

```
+----------------------------------------+
|  NETWORK: 5 peers  |  [CACHED 2m ago] |
|----------------------------------------|
```

---

## Analysis 5: MVP Scope for WeftOS 0.2

### What Must Exist in WeftOS

**kernel-architect**: The Mentra integration depends on these kernel capabilities:

| Capability | Status | Sprint Target |
|-----------|--------|---------------|
| JSON descriptor format definition | Designed (Section 9) | Sprint 11: formalize schema |
| WebSocket server in kernel | Not started | Sprint 12: implement |
| StateStore binding (`$state` -> kernel) | Not started | Sprint 12: implement |
| A2UI streaming protocol | Not started | Sprint 13: implement |
| HUD renderer (constraint engine) | Not started | Sprint 13-14: implement |
| Voice command parser | Not started | Sprint 14+: implement |

### MVP: 2 HUD Views + 1 Voice Command

**mentra-planner**: The minimum Mentra integration for 0.2:

**MVP View 1: System Status Card**
```
+----------------------------------------+
|  WEFTOS STATUS                         |
|----------------------------------------|
|  Kernel:   Running                     |
|  Services: 14 active                   |
|  Agents:   3 running, 1 idle           |
|  Health:   [==========] 100%           |
|  Chain:    block #4,271                 |
|  Uptime:   14h 22m                     |
|                                        |
|----------------------------------------|
|  "agents" | "network" | "refresh"      |
+----------------------------------------+
```

JSON descriptor: binds to `/kernel/state`, `/kernel/services/count`, `/kernel/processes/summary`, `/kernel/health/overall`, `/kernel/chain/height`, `/kernel/uptime`.

**MVP View 2: Agent List**
```
+----------------------------------------+
|  AGENTS: 4 total                       |
|----------------------------------------|
|  PID  Agent        State    CPU        |
|  001  weaver       running  12%        |
|  002  coder-1      running   8%        |
|  003  reviewer     idle      0%        |
|  004  tester-1     running   5%        |
|                                        |
|                                        |
|                                        |
|----------------------------------------|
|  "spawn [type]" | "kill [pid]" | "back"|
+----------------------------------------+
```

JSON descriptor: binds to `/kernel/processes` with columns `[pid, agent_id, state, cpu_percent]`.

**MVP Voice Command: "Status"**
```
User: "Hey WeftOS, status"
  -> kernel_exec("weftos status --format hud")
  -> Returns MVP View 1 descriptor
  -> HUD renders it
```

### What We Build in Each Sprint

```
Sprint 11 (current):
  - Formalize JSON descriptor Zod schema (catalog)
  - Define Mentra HUD renderer constraints spec
  - Define WebSocket message protocol for descriptor push
  - Write this architecture document (this file)

Sprint 12:
  - Implement WebSocket server in WeftOS kernel
  - Implement StateStore binding ($state -> ServiceApi)
  - Build MVP View 1 + View 2 descriptors
  - Unit test: descriptor -> expected HUD text output

Sprint 13:
  - Implement HUD constraint renderer (Rust or TS)
  - Implement A2UI streaming (createSurface, updateDataModel)
  - Integration test: WeftOS kernel -> WebSocket -> descriptor -> HUD text
  - Prototype: Cloudflare Worker as AppServer relay

Sprint 14:
  - Voice command pipeline (wake word + simple command parser)
  - Network Navigator view (Analysis 2)
  - End-to-end test: voice -> kernel -> HUD
  - Mentra SDK integration (if hardware available)
```

### Dependency Graph

```
Sprint 11          Sprint 12          Sprint 13          Sprint 14
+-----------+      +-----------+      +-----------+      +-----------+
| Descriptor|----->| WebSocket |----->| HUD       |----->| Voice     |
| Schema    |      | Server    |      | Renderer  |      | Pipeline  |
+-----------+      +-----------+      +-----------+      +-----------+
                         |                  |                  |
+-----------+      +-----------+      +-----------+      +-----------+
| Constraint|----->| StateStore|----->| A2UI      |----->| Network   |
| Spec      |      | Binding   |      | Streaming |      | Navigator |
+-----------+      +-----------+      +-----------+      +-----------+
                         |                  |
                   +-----------+      +-----------+
                   | MVP Views |----->| AppServer |
                   | 1 + 2     |      | Relay     |
                   +-----------+      +-----------+
```

---

## ECC Contribution

**ecc-analyst**: This track adds the following to the CMVG knowledge graph:

### New Causal Nodes

| Node ID | Type | Content |
|---------|------|---------|
| `mentra-arch-decision` | Decision | WeftOS runs cloud-side, glasses are thin HUD terminal |
| `mentra-transport` | Decision | WebSocket + A2UI protocol stack |
| `mentra-mvp-scope` | Specification | 2 views + 1 voice command for 0.2 |
| `mentra-latency-budget` | Constraint | <500ms for simple commands |
| `mentra-offline-model` | Design | Companion cache with TTL + stale indicators |
| `mentra-hud-constraints` | Constraint | 400x240, mono, 8-10 lines, voice-only input |

### Causal Edges

```
json-descriptor-arch (Section 9) --Enables--> mentra-hud-renderer
mentra-arch-decision --Causes--> mentra-transport
mentra-transport --Enables--> mentra-mvp-scope
mentra-hud-constraints --Constrains--> mentra-mvp-scope
kernel-serviceapi (K2) --Enables--> mentra-statestore-binding
mesh-framework (K6) --Enables--> mentra-network-navigator
ecc-integration (K3c) --Enables--> mentra-causal-chain-view
```

### Coherence Check

The Weaver notes no contradictions with other symposium tracks:
- Track 4 (UI/UX) defined the JSON descriptor architecture that we depend on. Consistent.
- Track 2 (Testing) should add HUD renderer constraint tests to the matrix. Flagged.
- Track 3 (Release) should include the descriptor schema in the 0.2 release scope. Flagged.

---

## Q&A Outcomes

### Q18: Should WeftOS run on-device (ARM64) or cloud-only for Mentra?

**RESOLVED**: Cloud-only (or companion device). The BES2700 SoC lacks the memory and runtime for WeftOS. The companion device (phone) can run a lightweight WeftOS instance with `Kernel<AndroidPlatform>` for low-latency local commands. Full analysis requires cloud.

### Q19: What is the voice latency budget for Mentra to WeftOS?

**RESOLVED**: 500ms target for simple commands (status, list). 800ms acceptable for search. Analysis commands are async with streaming progress. See latency budget table in Analysis 4.

### Q20: Is there a Mentra prototype available for testing integration?

**NOTED**: No hardware prototype available for Sprint 11-12. Testing uses:
1. Text-based HUD simulator (renders constraint output to terminal)
2. Mock WebSocket client simulating glasses traffic patterns
3. Actual Mentra hardware integration deferred to Sprint 14+ or when available

### Q21: How does the ECC cognitive substrate enhance contextual computing on glasses?

**RESOLVED**: Three ways:
1. **Causal navigation**: "What caused X?" queries traverse CausalGraph edges, rendering linearized chains on HUD (Analysis 3)
2. **Semantic search**: HNSW enables "find similar to X" voice commands that surface relevant knowledge without exact keyword matching
3. **Contextual alerts**: CognitiveTick can detect anomalies in the causal graph and push contextual alerts to the HUD before the user asks

### Privacy: Always-on analysis through glasses?

**NOTED**: WeftOS governance applies. All voice commands pass the GovernanceGate. The ExoChain witnesses every query. No audio is stored on-device beyond the command buffer. The companion device does not retain audio after transcription. Cloud processing follows the same governance rules as any other kernel command. Full privacy specification deferred to a dedicated security review (Track 2 follow-up).

---

## Summary of Decisions

| # | Decision | Owner | Sprint |
|---|----------|-------|--------|
| D6.1 | WeftOS runs cloud/companion, not on glasses | kernel-architect | -- |
| D6.2 | JSON descriptor format is the integration contract | mentra-planner | 11 |
| D6.3 | WebSocket + A2UI protocol for transport | mentra-development | 12-13 |
| D6.4 | MVP: 2 HUD views + 1 voice command for 0.2 | mentra-planner | 12-14 |
| D6.5 | HUD constraints: 400x240, mono, 8-10 lines, voice input | mentra-ui | 11 |
| D6.6 | Offline: companion cache with TTL, stale indicators | mentra-development | 13 |
| D6.7 | Latency target: <500ms simple, <800ms search | mentra-ux | 12+ |
| D6.8 | Testing via text-based HUD simulator until hardware available | mentra-tester | 12 |

---

## Action Items for Sprint 11

| Item | Owner | Deliverable |
|------|-------|-------------|
| Formalize Mentra HUD constraint spec | mentra-ui | `docs/weftos/mentra-hud-constraints.md` |
| Define JSON descriptor Zod schema (shared with Track 4) | kernel-architect | `gui/src/types/descriptor-schema.ts` |
| Write WebSocket message protocol spec | mentra-development | `docs/weftos/mentra-ws-protocol.md` |
| Create text-based HUD simulator scaffold | mentra-tester | `tests/mentra/hud-simulator/` |
| Add Mentra integration to 0.2 release scope | mentra-planner | Update `.planning/ROADMAP.md` |

---

*Track 6 complete. Handoff to Track 7: Deep Research -- Algorithmic Optimization & Performance.*
