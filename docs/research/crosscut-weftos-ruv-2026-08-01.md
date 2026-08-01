# WeftOS × rUv/Cognitum capability crosscut

**Status:** Darwin-loop classify artifact (S8 manual pass)  
**Date:** 2026-08-01  
**String:** `SEE → WIRE → BUILD → UPSTREAM`  
**Companions:**  
`docs/research/ruv-ecosystem-synergy-flywheel.md`,  
`.metaharness/flywheel/STRING.md`,  
`.metaharness/flywheel/GAPS.md`,  
`docs/research/ruv-worldgraph-vs-weftos.md`,  
`docs/research/graph-views.md`,  
ADR-090 / ADR-096 / ADR-097

**Method:** Traverse WeftOS inventory (ADRs 070–097, workspace crates,
`FEATURE_GATES.md`, `.metaharness/`, `.grok/rules`) and compare node-by-node
to rUv/Cognitum siblings. Prefer SEE/WIRE over BUILD/UPSTREAM. Confidence is
evidence quality, not implementation certainty.

**Do not:** treat this table as a mandate to raise ADR-041 scorecard vanity
dimensions. Score is a side-effect of real surface.

---

## 1. Inventory snapshot (WeftOS)

### 1.1 ADR titles 070–097

| ADR | Title |
|-----|--------|
| 070 | MCP server registry ownership — CLI durable config vs daemon runtime |
| 071 | WASM panel auth — per-panel token / capability model for the webview proxy |
| 072 | Webview substrate publish gate |
| 073 | Agent Workspace interaction principles (CNVS-informed) |
| 074 | Interim primary voice path — xAI Grok Voice (local remains offline/fallback) |
| 075 | Grok Build (and peer CLIs) as WeftOS MCP clients |
| 076 | MCP tool surface principles + unified capability catalog |
| 077 | Android native splat capture as a WeftOS edge node |
| 078 | Splat pipeline feeds a structured world model (not appearance-only) |
| 079 | Urth — multi-scale digital twin, sparse-first |
| 080 | Pending-skill review timing — CLI + non-blocking start notice |
| 081 | No first-party iMessage AppleScript channel (0.8.x / indefinite) |
| 082 | Graphify Rust Port — `clawft-graphify` knowledge-graph crate |
| 083 | Browser WASM support |
| 084 | Dependency-Graph Retrieval in Graphify (SGKR) — proposed |
| 085 | Entity Deduplication via HNSW Pre-filter (CodaRAG) |
| 086 | Codebook Cold-Start for Emerging Entities (TransFIR) |
| 087 | Spatio-Temporal Dual-Branch Architecture for Sensor Systems (K-STEMIT) |
| 088 | Optional VectorRef on BVH spatial leaf payloads |
| 089 | ExoChain DAG merge strategy and split-brain handling |
| 090 | LeWM ↔ ECC decoupling invariant (runtime-checkable rules) |
| 091 | LightRAG dual-level keyword retrieval in graphify |
| 092 | Cluster-wide GovernanceRule distribution |
| 093 | BVH × HNSW Phase F dual-index join |
| 094 | Spawn-at-user-level permission story |
| 095 | Batch graph analytics plane (disk-spill join-agg) — research hold |
| 096 | MetaHarness as foundational agent/fusion evolution layer |
| 097 | Universal MetaHarness governance over all WeftOS data surfaces |

### 1.2 Kernel feature gates (`docs/weftos/FEATURE_GATES.md`)

`native` (default) · `ecc` · `exochain` · `mesh` · `os-patterns` ·
`onnx-embeddings` · `wasm-sandbox` · `cluster` · `tilezero` · `containers`  
(+ related: `quic`, voice crate features, splat **runtime** tools)

### 1.3 Workspace members (high-signal names)

**Agent OS core:** `clawft-kernel`, `clawft-core`, `clawft-weave`, `clawft-cli`,
`clawft-types`, `clawft-platform`, `weftos`, `clawft-services`, `clawft-rpc`,
`clawft-tools`, `clawft-channels`, `clawft-llm`, `clawft-plugin*`,
`clawft-security`, `clawft-substrate`, `clawft-surface`, `clawft-canon`,
`clawft-app`, `clawft-gui-egui`

**World / graph / index:** `clawft-bvh`, `clawft-graphify`, `clawft-lsp-extract`,
`weftos-worldmodel*`, `weftos-sensor-pipeline*`, `clawft-worldmodel-service`,
`clawft-cow-memory`, `eml-core`

**Chain / crypto / tree:** `exo-core`, `exo-dag`, `exo-resource-tree`

**Voice:** `clawft-voice-talk`, `clawft-voice-aec`, `clawft-voice-onnx`,
`clawft-voice-tts`, `clawft-service-whisper`, `clawft-bench-voice`

**Splat / edge:** `clawft-splat-pipeline`, `clawft-splatd`,
`clawft-android-edge`, `clawft-sonobuoy-ranging`  
*(out-of-workspace but real: `clawft-edge-pad*`, leaf scene/renderer/sim,
`lgfx-bus-rgb-rs`)*

**WASM:** `clawft-wasm`, `clawft-wasm-host`

~54 foundation-scored members; leaf/edge/idf crates intentionally excluded.

### 1.4 `.metaharness` structure

```
.metaharness/
  commands/          weft-fusion-view, weft-gate, weft-plane-dag
  eval/              fusion + governance anchors
  flywheel/          STRING.md, GAPS.md, policy-root.json, receipts/
  flywheel-score/    ceilings, score invariants
  tasks/             fusion-view, gate, plane-dag
  weftos/            surfaces.yaml, views/
  genome-latest.json, score-latest.json, weftos-score-latest.json
  patterns-manifest.md
```

Scripts: `scripts/metaharness/{score,weftos-score,seed-patterns,flywheel-*,run-task,validate-views}.*`

### 1.5 `.grok` host overlay

```
.grok/rules/{ruflo-grok.md, metaharness.md}
.grok/skills/{agent-teams-grok, handoff, metaharness-tasks, plane-dag}
.grok/agents/{ruflo-architect, ruflo-coder, ruflo-reviewer, ruflo-tester, world-builder}
```

### 1.6 Pins / deps (ecosystem)

| Pin / dep | Where |
|-----------|--------|
| `ruflo` / `@claude-flow/cli` **3.32.38** | `package.json` `weftos.rufloPin` |
| `@metaharness/flywheel` | package.json |
| `agentic-flow` ^2.1.0 | package.json (dev) |
| `cognitum-gate-tilezero` 0.1 | workspace Cargo.toml; kernel `tilezero` |
| `rvf-runtime` / `rvf-types` 0.2, `weftos-rvf-crypto` | workspace |
| `ruvector-{cluster,raft,replication,diskann,sona}` | workspace optional features |

---

## 2. Summary counts by mode

Modes below use the **primary** mode for each row (some rows note secondary).

| Mode | Count | Meaning |
|------|------:|---------|
| **SEE** | 9 | Exists; agents / scanners can’t find it reliably |
| **WIRE** | 12 | Exists; not fully on the agent / CI path |
| **BUILD** | 7 | Missing product surface in WeftOS |
| **UPSTREAM** | 6 | Lives in rUv/Cognitum (or we contribute reference) |
| **SEE (WeftOS-only)** | 4 | Native assets agents miss when hunting rUv twins |
| **WIRE (WeftOS-only)** | 3 | Native assets not fully path-connected |
| **Total rows** | **41** | Including dual-listed WeftOS-only |

**Preference order for next gen:** SEE → WIRE → BUILD → UPSTREAM.

---

## 3. Full classify table

| Node | rUv capability (1 line) | WeftOS counterpart | Mode | Next action | Conf |
|------|-------------------------|--------------------|------|-------------|------|
| **ruflo** | Orchestrator: swarm, MCP, memory, hooks, team bus, doctor | Pin 3.32.38; `.grok/rules/ruflo-grok.md`; MCP `claude-flow`; scripts fallback | **WIRE** | Exercise metaharness_* + team_bus from Grok session; publish MCP parity matrix | high |
| **agentdb** | RVF-backed patterns, causal, HNSW; `patterns` ns multi-host | `.swarm/agentdb-memory.db` via Ruflo; `seed-patterns.sh`; `patterns-manifest.md` | **WIRE** | Dual-host round-trip CI (Grok store → Claude retrieve) — GAPS S6 | high |
| **agenticow** | COW memory branching / checkpoint orchestration | `crates/clawft-cow-memory` (RVF store; Phases 2–3 hermes/exochain unwired) | **WIRE** | Wire COW into hermes-loop or document composition vs AgentDB (no dual SoT) | high |
| **metaharness score/genome/flywheel/darwin** | READ score+genome; WRITE darwin; promote flywheel | `.metaharness/*`; `scripts/metaharness/*`; measure receipts; Darwin **off** until confirm | **WIRE** | Keep genome next to foundation score; add interop fixtures to measure; Darwin only with S3 approval | high |
| **@metaharness/kernel + hosts** | 9 host adapters (Claude Code…RVM); no host-grok | Overlay only: `.grok/` rules/skills/agents — **not** `@metaharness/host-grok` | **UPSTREAM** | Package host-grok reference (S1) for rUv contribution; freeze overlay contract | high |
| **@metaharness/router** | Cost-optimal model routing + savings (ADR-148/149) | Not primary WeftOS router; dep may exist transitively via agentic-flow | **BUILD** | Optional wire + savings ledger only after approval (GAPS S2); never hard-dep `weft` | med |
| **ruvector** | HNSW, RVF, cluster/raft/replication, diskann | `rvf-*`, `ruvector-cluster/raft/replication/diskann`, `ecc`+`cluster` features, BVH-on-RVF | **SEE** | Index in WeftOS brain / platform matrix; ensure agents land on FEATURE_GATES + ADR-093 | high |
| **cognitum-gate-tilezero** | Permit/Defer/Deny + crypto receipts | Kernel `tilezero` feature; `clawft-kernel` gate; implies `exochain` | **WIRE** | CI smoke for Permit/Defer/Deny receipt format (GAPS C3) | high |
| **Cognitum MaaS/Fugu / api.cognitum.one** | Metered chat, `cog_` keys, approval pods, catalog | NONE as first-class LLM provider; local/Ollama/OpenAI more common | **BUILD** | Optional Cognitum-compatible provider + degrade-to-local (GAPS C1) | med |
| **Cognitum Seed + cogs** | Edge vector store, cognitive-pipeline cogs, Pi envelope | Parallel: edge-pad / leaf / android-edge / sensor pipeline — **no pair API** | **BUILD** | Seed peer map + optional probe (GAPS C2); treat as peer not dual OS | med |
| **worldgraph / WorldGraph twin** | Typed petgraph twin + provenance; OccWorld predictor | Graph Views + BVH + chain (research crosswalk); LeWM optional (ADR-090) | **BUILD** | ViewSpec ↔ WorldGraph provenance field map / adapter (GAPS S4); do not merge crates | high |
| **RuView / RF sensing** | RF/CSI/UWB densepose / occupancy sensors | Contrast only — geometry SoT is BVH/splat, not CSI grid | **SEE** | Keep contrast note in WorldGraph crosswalk; no product port | high |
| **midstream / QUIC** | MidStream temporal-compare + midstreamer-quic crate | Voice: `clawft-voice-talk/midstream.rs` scaffold; transport: ADR-026 QUIC mesh (own) | **WIRE** | Vendor DTW/LCS subset on voice path (WEFT-617); keep QUIC WeftOS-owned | high |
| **qudag / synaptic-mesh** | QuDAG PQ mesh networking; synaptic-mesh swarm fabric | Partial: ML-DSA dual-sign (ADR-028), mesh/QUIC; QuDAG only as research/skill refs | **SEE** | Document “borrow crypto primitives, own mesh”; no full QuDAG dependency | med |
| **agentic-flow meta-harness** | Freeze model / evolve harness (ADR-075/076 lineage) | Doctrine ADR-096; npm `agentic-flow`; product is Rust OS not Node harness | **SEE** | Pattern-store doctrine only; do not replace MH flywheel with agentic-flow runtime | med |
| **GEPA / Darwin evolve** | GEPA prompt evolution; MH Darwin harness mutants | GEPA: `clawft-core` pipeline TrajectoryLearner (ADR-017); Darwin: policy-only, off | **WIRE** | Keep GEPA production-wired under audit; Darwin variants only under `.metaharness/variants` + confirm | high |
| **ruvllm / SONA / MicroLoRA** | Small LLM stack; SONA neural; MicroLoRA adapt | `ruvector-sona` via `hybrid-rerank` feature on clawft-core; no MicroLoRA product path | **SEE** | Document hybrid-rerank feature path in FEATURE_GATES / agent patterns; MicroLoRA = future/none | med |
| **Grok host pathfinding** | (WeftOS-specific) first Grok+Ruflo+MH production loop | `.grok/*`, ADR-075 MCP client bridge, team bus skills, flywheel alignment flags | **WIRE** | Close MCP parity + dual-host CI; treat overlay as upstream contribution surface | high |
| **WeftOS brain (S7)** | ruvbrain twin over *our* ADRs/crates | NONE — manual crosscut only (this doc) | **BUILD** | Scaffold `.metaharness/brain/` + `search_weftos` optional CLI (GAPS S7) | high |
| **crosscut job (S8)** | Automated traverse/compare → classify JSON | This markdown; no JSON job yet | **BUILD** | `scripts/metaharness/crosscut.sh` → classify JSON for Darwin proposer | med |
| **kernel ECC** | (WeftOS-native) cognitive substrate authority | `clawft-kernel` `ecc` feature; R1–R5 ADR-090 frozen | **SEE** | Agent task cards + patterns: “ECC authority, never Darwin-mutate” | high |
| **ExoChain** | (WeftOS-native) crypto audit DAG | `exochain` feature; `exo-dag`/`exo-core`; ADR-022/089 | **SEE** | Surface in doctor/genome OS matrix; link governance events to TileZero | high |
| **BVH** | (WeftOS-native) spatial-temporal index on RVF | `clawft-bvh`; ADR-056/088/093 | **SEE** | Patterns for BVH×HNSW Phase F join; Graph Views attach path | high |
| **Graph Views** | WorldGraph-class operational fusion | Research `graph-views.md` + MH fusion-view task; ViewSpec fixtures | **WIRE** | Formal promote path: eval → receipt → human promote (partial today) | high |
| **LeWM (ADR-090)** | OccWorld-class predictor (contrast) | `weftos-worldmodel*`, service binary; ECC remains authority | **SEE** | Index R1–R5 in patterns; agents must not route authority through WM | high |
| **voice stack** | MidStream / agentic voice siblings partial | `clawft-voice-*`, channels Talk Mode, ADR-053/061/068/074 | **WIRE** | Complete midstream vendor + wake path honesty; keep Grok voice interim documented | high |
| **splat** | (WeftOS-native) Gaussian splat → world model | `clawft-splatd`/`splat-pipeline`; ADR-077/078; android-edge | **SEE** | Pattern: splat feeds Views/BVH not appearance-only; runtime tool deps in doctor | high |
| **edge-pad / leaf** | Cognitum Seed peer class (hardware edge) | `clawft-edge-pad*`, leaf scene/renderer (out-of-workspace) | **SEE** | Platform matrix row in foundation score / brain; Seed peer map (C2) | med |
| **QUIC mesh (WeftOS)** | rUv midstreamer-quic / QuDAG transport peers | ADR-026 primary QUIC; kernel `quic`/`mesh` features | **SEE** | Own transport narrative; do not adopt midstreamer-quic on voice tick | high |
| **swarm topology** | Ruflo hierarchical/mesh swarm prompts | Runtime `SwarmCoordinator` **flat only**; claude-flow names ≠ Rust types | **SEE** | Patterns + `docs/architecture/swarm-topology.md` so agents stop inventing coordinators | high |
| **production promote keys** | MH Ed25519 promote + verifyReplayBundle | Process-local signer only | **BUILD** | CI promote keys + replay verify (GAPS S5) — approval | high |
| **dual-host patterns proof** | Shared AgentDB patterns across hosts | Manifest + seed; no automated round-trip | **WIRE** | Interop fixture test non-empty keys + store/search two-host | high |

---

## 4. Top 10 high-leverage SEE / WIRE items

Prefer these before any new product feature. Each is “already built / almost
built” lift for pathfinding and honest readiness.

| # | Item | Mode | Why leverage |
|---|------|------|--------------|
| 1 | **Dual-host pattern round-trip CI** | WIRE | Proves Claude↔Grok memory compat — core synergy objective |
| 2 | **Grok MCP metaharness parity matrix** | WIRE | Unblocks score/genome/team from Grok without script scavenger hunt |
| 3 | **TileZero receipt CI smoke** | WIRE | Cognitum gate kinship without new provider; fail-closed story |
| 4 | **COW memory composition doc + hermes wire plan** | WIRE | Avoid AgentDB vs COW dual SoT; finish agenticow port story |
| 5 | **Graph Views promote discipline** | WIRE | Same shape as MH promote; fusion safety without new twin crate |
| 6 | **ruvector / cluster / sona feature SEE** | SEE | Agents rediscover `hybrid-rerank`, DiskANN, cluster via index not cargo archaeology |
| 7 | **ECC / LeWM / ExoChain SEE in patterns** | SEE | Frozen ADR-090 surfaces agents must not Darwin-touch |
| 8 | **Swarm topology SEE** | SEE | Stops false BUILD of hierarchical Rust coordinators |
| 9 | **Midstream temporal-compare vendor** | WIRE | Voice tick gap already decided (WEFT-617); small, high signal |
| 10 | **Splat / edge-pad SEE on platform matrix** | SEE | Hardware story agents miss when only reading Node MH paths |

---

## 5. Top 5 BUILD

| # | Item | Scope | Gate |
|---|------|-------|------|
| 1 | **WeftOS brain (S7)** | Optional index over ADRs/crates/research; `search_weftos` | Approval; ADR-150 removable |
| 2 | **crosscut.sh (S8)** | Automate this table → JSON for Darwin proposer | After S7 or manual corpus |
| 3 | **Cognitum/Fugu optional LLM provider (C1)** | `cog_` + budget + local degrade | Approval; never hard cloud |
| 4 | **Seed peer map + probe (C2)** | Document pair/push/query vs edge-pad | Research → optional probe |
| 5 | **Production promote keys + replay verify (S5)** | CI signer / verifyReplayBundle | Approval; no private keys in git |

*Honorable:* `@metaharness/router` + savings (S2); ViewSpec↔WorldGraph adapter (S4).

---

## 6. Top 5 UPSTREAM

| # | Item | What we contribute / need | Direction |
|---|------|---------------------------|-----------|
| 1 | **`@metaharness/host-grok`** | Reference overlay from `.grok/` + team-bus contract | We → rUv |
| 2 | **Agent-OS monorepo inventory archetype** | Scorecard stops mis-ranking as `mcp-server-harness` | We → MH inventory |
| 3 | **Patterns namespace dual-host convention** | Shared key/value rules for multi-host Ruflo memory | We ↔ Ruflo docs |
| 4 | **WorldGraph provenance field schema** | Shared edge provenance vocabulary (not crate merge) | We ↔ WorldGraph |
| 5 | **Cognitum Seed / Fugu client contracts** | Stable pair API + MaaS auth if cloud path | We need / may PR client notes |

---

## 7. Suggested first Darwin generation (one lever only)

**Lever (policy only):** set / emphasize

```text
prefer_intervention = see_wire_build_upstream
```

**Concrete mutate (harness docs, not Rust):**

Add to flywheel measure **one** alignment check that fails closed if missing:

> **Interop fixture:** `patterns-manifest.md` keys non-empty **and**
> `scripts/metaharness/seed-patterns.sh --dry-run` succeeds.

**Why this gen only:**

- Pure **WIRE** — no ECC, no ViewSpec champion swap, no Darwin WRITE on product.
- Directly serves dual-host + Ruflo memory objectives.
- Receiptable under existing `flywheel-measure.mjs` without production keys.
- Score rises only as a **side-effect** of real surface (seed/manifest honesty).

**Frozen (do not touch this gen):** ADR-090 R1–R5, gate phases, dual-sign kinds,
substrate default-deny, ViewSpec champions, kernel crates.

**Promote rule:** human confirm after measure receipt shows
`dualHostPatterns` + new fixture green; no auto-promote.

---

## 8. Mode histogram (primary mode, rUv+Cognitum + native)

```
SEE      ████████████░░░░  ~13
WIRE     ██████████████░░  ~15
BUILD    ███████░░░░░░░░░  ~7
UPSTREAM ██████░░░░░░░░░░  ~6
```

(Approximate; dual-mode rows counted once at primary.)

---

## 9. Pathfinder stack (locked)

| Layer | Owner |
|-------|--------|
| Executor | Grok Build |
| Orchestrator / ledger | Ruflo / claude-flow MCP |
| Evolve / promote | MetaHarness flywheel (+ optional Darwin WRITE) |
| Product runtime | WeftOS Rust (no Node/MH link requirement) |
| Cloud gate / meter (optional) | Cognitum TileZero / MaaS |

> Freeze the model. Evolve the harness. Promote only what proves lift.

---

## 10. Next manual Darwin gens (queue, not this file’s mutate)

| Gen | Lever / node | Mode |
|-----|--------------|------|
| 0 (this) | patterns seed dry-run fixture | WIRE |
| 1 | TileZero CI smoke evidence bit | WIRE |
| 2 | Grok MCP parity checklist in `.metaharness/tasks/` | WIRE |
| 3 | host-grok package skeleton (docs only) | UPSTREAM prep |
| 4 | WeftOS brain corpus list (no index yet) | BUILD prep / SEE |

---

## 11. Sources consulted

- `docs/research/ruv-ecosystem-synergy-flywheel.md`
- `.metaharness/flywheel/STRING.md`, `GAPS.md`, `policy-root.json`, `receipts/latest.json`
- `docs/weftos/FEATURE_GATES.md`
- Root `Cargo.toml` workspace members
- `docs/adr/adr-070` … `adr-097` titles
- `docs/research/ruv-worldgraph-vs-weftos.md`, `graph-views.md`, `weft-617-midstream-eval.md`
- `package.json` pins; `.metaharness/README.md`; `.grok/` tree
- `crates/clawft-cow-memory`, kernel `tilezero`, `ruvector-*` workspace deps

---

*End of S8 manual crosscut — 2026-08-01.*
