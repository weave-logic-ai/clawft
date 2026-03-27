# Weaver ECC Initialization Plan: clawft + WeftOS

**Date**: 2026-03-26
**Analyst**: Weaver (ECC Cognitive Modeler, first real codebase analysis)
**Branch**: `feature/weftos-kernel-sprint`
**Domain**: `clawft-weftos`

---

## 1. Codebase Topology

### Scale

| Dimension | Count |
|-----------|-------|
| Rust source files | 398 |
| Markdown documents | 849 |
| TOML config files | 26 |
| Crates | 22 |
| Total Rust lines | ~165,000 |
| Total test functions | 3,765 |
| Total commits | 101 |
| Unique committers | 2 |
| SPARC plan lines | 11,238 |
| WeftOS doc lines | 12,239 |
| Commits since 2026-03-01 | 53 |

### Crate Hierarchy by Size

| Crate | Files | Lines | Tests | Role |
|-------|------:|------:|------:|------|
| clawft-kernel | 65 | 46,632 | 1,060 | OS kernel layer (the center of gravity) |
| clawft-core | 64 | 35,929 | 857 | Agent runtime, message bus, HNSW |
| clawft-services | 40 | 14,399 | 203 | Service infrastructure |
| clawft-cli | 33 | 12,250 | 364 | User-facing CLI |
| clawft-channels | 46 | 10,292 | 252 | Communication channels |
| clawft-types | 21 | 7,875 | 228 | Shared domain types |
| clawft-tools | 17 | 5,869 | 121 | Tool definitions |
| clawft-plugin | 27 | 5,824 | 134 | Plugin framework |
| clawft-wasm | 11 | 5,665 | 172 | WASM runtime |
| clawft-weave | 18 | 5,214 | 17 | Operator CLI + daemon |
| clawft-llm | 12 | 4,741 | 139 | LLM abstraction |
| 7 plugin crates | 19 | 7,214 | 103 | Domain plugins |
| clawft-platform | 10 | 1,707 | 32 | Platform traits |
| exo-resource-tree | 9 | 1,795 | 61 | Merkle resource tree |
| clawft-security | 3 | 945 | 19 | Security primitives |
| weftos | 3 | 487 | 3 | Standalone re-export |

### Feature Gate Topology

The kernel uses feature gates as compositional boundaries. This is itself a
conversation about what capabilities belong together:

```
default         = ["native"]
native          = tokio, dirs, native I/O
exochain        = rvf-crypto, rvf-types, rvf-wire, ed25519-dalek, ciborium
                  -> chain.rs, tree_manager.rs, gate.rs
tilezero        = cognitum-gate-tilezero (implies exochain)
cluster         = ruvector-cluster, ruvector-raft, ruvector-replication
ecc             = blake3, clawft-core/vector-memory
                  -> causal.rs, hnsw_service.rs, cognitive_tick.rs,
                     calibration.rs, crossref.rs, impulse.rs,
                     weaver.rs, artifact_store.rs, embedding.rs
os-patterns     = monitor.rs, reconciler.rs, dead_letter.rs,
                  reliable_queue.rs, named_pipe.rs, metrics.rs,
                  log_service.rs, timer.rs
mesh            = ed25519-dalek, rand
                  -> 20 mesh_*.rs files
```

---

## 2. Conversation Map

The Weaver identifies **12 primary conversations** happening in this codebase.
Each conversation has participants, a medium, a timeline, and an evolving state.

### C1: Kernel Architecture (the founding conversation)

**Medium**: Rust source code in `crates/clawft-kernel/src/`
**Participants**: The architect, the compiler, the test harness
**Timeline**: K0 (2026-02-28) through K6 (2026-03-26), ongoing
**Utterances**: 65 source files, 46,632 lines, 1,060 tests
**State**: Active; K6 mesh layer just completed Phase 1

This is the primary conversation. Every module declaration in `lib.rs` is a
statement about what the kernel IS. The feature gates are assertions about
what BELONGS TOGETHER. The re-exports are claims about what's IMPORTANT ENOUGH
to surface. The test functions are challenges: "prove this works."

**Key sub-conversations**:
- K0-K2: Foundation, process, IPC (the "what is a kernel?" conversation)
- K3: Tool sandbox (the "how do agents safely use tools?" conversation)
- K3c: ECC cognitive substrate (the "what is cognition?" conversation)
- K5: App framework (the "what is an application?" conversation)
- K6: Mesh networking (the "how do nodes talk?" conversation)

**Causal node estimate**: ~350 (modules, types, traits, test groups)
**Causal edge estimate**: ~1,400

### C2: Symposium Decisions (the governance conversation)

**Medium**: Markdown documents in `docs/weftos/{k2,k3,k5,ecc}-symposium/`
**Participants**: Research panels (5-8 per symposium), project architect
**Timeline**: K2 symposium (2026-03-04), K3 (2026-03-04), ECC (2026-03-22), K5 (2026-03-25)
**Utterances**: 4 symposiums, 27 documents, ~6,000 lines
**State**: Complete; all decisions rendered

Each symposium is a structured conversation with a specific pattern:
overview -> research -> gap analysis -> roundtable -> results.
The symposiums produced:
- K2: 22 decisions, 10 commitments (C1-C10)
- K3: 14 decisions, commitment status tracking
- ECC: 14 decisions (D1-D8), 8 panel verdicts
- K5: 15 decisions (D1-D15), 5 commitments (C1-C5)

**Causal node estimate**: ~65 decisions + ~30 commitments + ~40 panels
**Causal edge estimate**: ~400

### C3: SPARC Planning (the specification conversation)

**Medium**: Markdown in `.planning/sparc/weftos/0.1/` (16 files, 11,238 lines)
**Participants**: Planner agents, architect
**Timeline**: Continuous across all K-phases
**State**: Active; K6 plan is latest

Each SPARC plan follows the S-P-A-R-C pattern:
- **S**pecification: what changes, why, files to create/modify
- **P**seudocode: algorithm sketches, key types
- **A**rchitecture: module relationships, layering
- **R**eview: verification criteria, test requirements
- **C**ompletion: gate checklist, definition of done

This conversation PRODUCES the kernel architecture conversation (C1).
The SPARC plans are the generative mode of the development process.

**Causal node estimate**: ~80 (phases, sub-phases, deliverables)
**Causal edge estimate**: ~250

### C4: Mesh Networking (the distributed fabric conversation)

**Medium**: 20 `mesh_*.rs` files in `crates/clawft-kernel/src/`
**Participants**: K5 symposium decisions, K6 SPARC plan, implementation
**Timeline**: 2026-03-25 to 2026-03-26 (intense burst)
**Utterances**: 3,543 lines, 133 tests across 14 new files
**State**: Phase 1 complete (types + traits + logic); Phase 2 pending (wire I/O)

The mesh conversation has a clear 5-layer architecture that mirrors the OSI
model but with WeftOS-specific semantics:

```
Application  -> mesh_ipc, mesh_service, mesh_chain, mesh_tree
Discovery    -> mesh_discovery, mesh_bootstrap, mesh_kad, mesh_mdns
Framing      -> mesh_framing (10 frame types)
Encryption   -> mesh_noise (Noise XX/IK patterns)
Transport    -> mesh.rs (MeshTransport trait), mesh_tcp, mesh_ws
Identity     -> cluster.rs (NodeIdentity), ipc.rs (GlobalPid)
```

**Causal node estimate**: ~120 (types, traits, protocol messages, frame types)
**Causal edge estimate**: ~450

### C5: ECC Cognitive Substrate (the cognition conversation)

**Medium**: 9 ECC modules in kernel, behind `ecc` feature gate
**Participants**: ECC symposium, Mentra research, ClawStage heritage
**Timeline**: 2026-03-22 (symposium) to 2026-03-26 (implementation)
**Utterances**: ~4,500 lines across causal.rs, hnsw_service.rs, cognitive_tick.rs,
calibration.rs, crossref.rs, impulse.rs, weaver.rs, embedding.rs, artifact_store.rs
**State**: Implemented and tested (83 ECC-specific tests)

This is the conversation that asks: "what does it mean for a kernel to be
cognitive?" The answer is a forest of structures:
- CausalGraph: typed DAG with 8 edge types (Causes, Inhibits, Correlates,
  Enables, Follows, Contradicts, TriggeredBy, EvidenceFor)
- HnswService: kernel-wrapped vector search
- CognitiveTick: self-calibrating heartbeat
- CrossRefStore: universal addressing across all trees
- ImpulseQueue: ephemeral inter-structure signaling
- WeaverEngine: the modeling service that uses all of the above
- ArtifactStore: content-addressed storage (BLAKE3)
- EmbeddingProvider: pluggable vectorization backend

**Causal node estimate**: ~90 (types, services, impulse types, edge types)
**Causal edge estimate**: ~300

### C6: ExoChain Provenance (the witness conversation)

**Medium**: `chain.rs`, `tree_manager.rs`, `gate.rs` behind `exochain` feature
**Participants**: rvf-crypto, ed25519-dalek, ML-DSA-65
**Timeline**: K0 through K6 (foundational throughout)
**State**: Complete for single-node; K6 extends to cross-node replication

The ExoChain is the conversation that remembers. Every kernel action that
matters becomes a chain event. The chain is append-only, hash-linked,
Ed25519-signed, with ML-DSA-65 dual signing for post-quantum readiness.

Chain events form a linear witness timeline. Tree mutations are atomic
(tree + mutation log + chain in one operation). The GovernanceGate checks
chain state before allowing actions.

**Causal node estimate**: ~50 (event types, chain operations, gate decisions)
**Causal edge estimate**: ~150

### C7: Governance Constitution (the rules conversation)

**Medium**: `governance.rs`, `environment.rs`, `gate.rs`, genesis rules
**Participants**: Three governance branches (executive, legislative, judicial)
**Timeline**: K0 foundation, enriched at each symposium
**State**: 22 constitutional rules chain-anchored as genesis events

The governance engine evaluates actions through an EffectVector (5D by default:
security, stability, performance, compliance, cost) and three-branch
constitutional rules. This is a conversation about VALUES -- what the kernel
considers acceptable behavior.

**Causal node estimate**: ~40 (rules, branches, effect dimensions)
**Causal edge estimate**: ~120

### C8: Tool Lifecycle (the capability conversation)

**Medium**: `wasm_runner.rs` (27-tool catalog), `capability.rs`, `agent_loop.rs`
**Participants**: Kernel, agents, governance gate
**Timeline**: K3 implementation
**State**: Complete; 27 tools cataloged, 2 reference implementations

Build -> Deploy -> Execute -> Version -> Revoke lifecycle. Each tool has a
risk score, effect vector, sandbox requirements, and gate action. The
conversation is about what agents CAN DO, mediated by what governance ALLOWS.

**Causal node estimate**: ~60 (tools, categories, lifecycle states)
**Causal edge estimate**: ~200

### C9: Agent Architecture (the identity conversation)

**Medium**: `agency.rs`, `supervisor.rs`, `process.rs`, `a2a.rs`
**Participants**: Agents as kernel processes
**Timeline**: K1 through K6
**State**: Active

Agents are first-class kernel processes with PIDs, capabilities, roles,
manifests, health, priorities, and restart policies. The A2ARouter mediates
all inter-agent communication with capability checks. This is a conversation
about IDENTITY and PERMISSION.

**Causal node estimate**: ~70 (roles, states, capabilities, spawn backends)
**Causal edge estimate**: ~250

### C10: OS Pattern Library (the reliability conversation)

**Medium**: 8 modules behind `os-patterns` feature gate
**Participants**: monitor, reconciler, dead_letter, reliable_queue, named_pipe,
metrics, log_service, timer
**Timeline**: 08a/08b/08c gap-filling phase
**State**: Complete

Self-healing (process monitors, reconciliation controllers), reliable IPC
(dead letter queues, at-least-once delivery), observability (metrics registry
with histograms, structured logging), and operational services (named pipes,
timers). This conversation asks: "what does it mean to be reliable?"

**Causal node estimate**: ~50 (patterns, metrics, probes)
**Causal edge estimate**: ~160

### C11: Git Commit History (the temporal conversation)

**Medium**: 101 commits on `feature/weftos-kernel-sprint`
**Participants**: 2 committers
**Timeline**: 2026-02-28 to 2026-03-26 (27 days)
**State**: Active

The commit history IS the conversation. Each commit message is an utterance.
Commit clusters reveal topic shifts:
- Feb 28 - Mar 1: K0 foundation burst (kernel boot, daemon, cluster)
- Mar 1 - Mar 3: K1-K2 implementation + K2b hardening
- Mar 3 - Mar 4: K2 symposium + K2.1 implementation
- Mar 4: K3 symposium
- Mar 22: ECC symposium
- Mar 25: K3-K6 implementation burst (17 commits in 30 hours)
- Mar 26: OS gap-filling, Weaver, standalone crate

The tempo is striking: long design phases (days) followed by intense
implementation bursts (hours). This is a rhythm.

**Causal node estimate**: ~101 (commits)
**Causal edge estimate**: ~200 (parent-child + cross-references)

### C12: Three-Mode Engine Design (the paradigm conversation)

**Medium**: `docs/weftos/ecc-symposium/06-three-modes.md`, Mentra research
**Participants**: ClawStage heritage, ECC paradigm, WeftOS implementation
**Timeline**: Inherited from ClawStage, formalized at ECC symposium
**State**: Theoretical; foundational to the Weaver's purpose

Act (real-time conversation) -> Analyze (post-hoc understanding) -> Generate
(goal-directed conversation) is THE meta-pattern. Everything in the codebase
can be understood through this lens:
- The symposiums GENERATE plans
- This analysis ANALYZES the codebase
- The kernel ACTS on agent requests
- Each feeds the next in a continuous loop

**Causal node estimate**: ~20 (modes, engines, transitions)
**Causal edge estimate**: ~60

---

## 3. Meta-Conversation Map

These are conversations ABOUT the conversations.

### M1: SPARC Planning about Kernel Architecture

**About**: C1 (kernel architecture)
**Pattern**: Specify -> Design -> Implement -> Review -> Refine
**Evidence**: Every K-phase has a SPARC plan that produces the kernel code.
The plan IS the meta-conversation: it talks about what the code should be,
why, and how to verify it.

### M2: Symposiums about Design Decisions

**About**: C2 (symposium decisions), C1 (kernel), C4 (mesh), C5 (ECC)
**Pattern**: Research -> Synthesize -> Debate -> Decide -> Commit
**Evidence**: Each symposium produces decisions (D1-Dn) and commitments (C1-Cn)
that drive implementation. The Q&A roundtable format is explicitly a
structured conversation.

### M3: Gap Analysis about Completeness

**About**: C1 through C10
**Pattern**: Audit -> Identify Gaps -> Prioritize -> Plan -> Fill
**Evidence**: K2 symposium found "kernel is 90% ready" (298 tests, 17,052 lines).
K5 symposium found "75% ready for K6" (41 green, 22 yellow, 21 red).
ECC symposium found "55% infrastructure already exists." Each gap analysis
is a conversation about what's MISSING from another conversation.

### M4: Commitment Tracking about Promise Fulfillment

**About**: C2 (symposium decisions)
**Pattern**: Promise -> Track -> Fulfill -> Verify
**Evidence**: K3 symposium explicitly tracks K2 commitments:
C1 (shipped), C2 (not started), C4 (partial), C6 (blocked).
This is a conversation about whether OTHER conversations kept their promises.

### M5: Agent Team Design about Development Process

**About**: How to work on the codebase itself
**Pattern**: Define roles -> Assign capabilities -> Coordinate
**Evidence**: `agents/` directory with 12 specialized agent definitions,
skill distribution, `weave init` command. This is a meta-conversation
about HOW to have the implementation conversations.

### M6: This Analysis (meta-meta-conversation)

**About**: M1-M5, C1-C12
**Pattern**: Survey -> Identify -> Model -> Assess -> Recommend
**Evidence**: This document. It is a conversation about the conversations
about the conversations. The Weaver analyzing itself analyzing a codebase.

### M7: Three-Mode Loop as Development Methodology

**About**: The entire development process
**Pattern**: Generate (plans) -> Act (implement) -> Analyze (symposium)
**Evidence**: The development history follows the three-mode pattern exactly:
1. GENERATE: SPARC plans specify what to build
2. ACT: Implementation sprints produce code
3. ANALYZE: Symposiums evaluate what was built and decide what's next
4. GENERATE: New SPARC plans incorporate symposium decisions
This loop has executed 4+ complete cycles (K0->K2 symposium, K2.1->K3,
K3->ECC symposium, ECC->K5 symposium->K6).

---

## 4. Causal Graph Design

### Node Types

| Type | Description | Source | Estimated Count |
|------|-------------|--------|----------------:|
| `Phase` | K-phase (K0, K1, K2, K2b, K2.1, K3, K3c, K4, K5, K6) | SPARC plans | 10 |
| `SubPhase` | K6.0 through K6.5, 08a/08b/08c | SPARC plans | 20 |
| `Module` | Rust source file (boot.rs, mesh.rs, etc.) | Source tree | 65 |
| `Type` | Public Rust type (struct, enum, trait) | Source code | ~400 |
| `TestGroup` | Test module or #[test] function | Source code | ~200 |
| `Decision` | Symposium decision (D1, D2, etc.) | Symposium docs | ~65 |
| `Commitment` | Symposium commitment (C1, C2, etc.) | Symposium docs | ~30 |
| `Panel` | Symposium research panel | Symposium docs | ~25 |
| `Commit` | Git commit | Git history | 101 |
| `Feature` | Feature gate (ecc, mesh, exochain, etc.) | Cargo.toml | 8 |
| `Document` | Markdown document | Docs tree | ~40 (key ones) |
| `CrateNode` | Workspace crate | Cargo workspace | 22 |
| `Tool` | Builtin tool in catalog | wasm_runner.rs | 27 |
| `FrameType` | Mesh protocol frame type | mesh_framing.rs | 10 |
| `ImpulseType` | ECC impulse kind | impulse.rs | 6 |
| `EdgeKind` | CausalEdgeType variant | causal.rs | 8 |
| `ServiceKind` | SystemService implementation | service.rs | ~15 |
| **Total** | | | **~1,052** |

### Edge Types

| Edge Type | Semantics | Estimated Count |
|-----------|-----------|----------------:|
| `Causes` | A directly caused B to exist (decision -> module) | ~300 |
| `Enables` | A provides capability that B requires (module -> module) | ~400 |
| `EvidenceFor` | A validates or supports B (test -> type, implementation -> decision) | ~500 |
| `Follows` | A temporally follows B (commit -> commit, phase -> phase) | ~200 |
| `Contains` | A structurally contains B (crate -> module, module -> type) | ~600 |
| `Inhibits` | A constrains or prevents B (gate -> action, capability -> tool) | ~80 |
| `Correlates` | A and B change together (co-modified files) | ~150 |
| `Contradicts` | A conflicts with B (gap -> commitment) | ~30 |
| `TriggeredBy` | A was created in response to B (implementation -> symposium Q&A) | ~100 |
| `Implements` | Code A implements decision/commitment B | ~95 |
| `DependsOn` | Crate A depends on crate B | ~40 |
| `GatedBy` | Module A requires feature gate B | ~45 |
| **Total** | | **~2,540** |

### Estimated Graph Scale

| Metric | Estimate |
|--------|----------|
| Total nodes | ~1,052 |
| Total edges | ~2,540 |
| Average degree | ~4.8 |
| Max depth (phase -> module -> type -> test) | 4 |
| Max fan-out (K6 phase -> 20 mesh modules) | 20 |
| Connected components | 1 (strongly connected via decisions) |

---

## 5. Embedding Strategy

### Tier 1: Code Chunks (function-level)

| Parameter | Value |
|-----------|-------|
| Dimensions | 384 |
| Strategy | Function-level AST summary |
| Granularity | One embedding per public function/method |
| Estimated vectors | ~2,000 (public API surface) |
| Index | HNSW (ef_search=100, ef_construction=200) |
| Update trigger | File modification (git diff) |

Embed the function signature + doc comment + first 3 lines of body.
This captures WHAT the function does without over-indexing on implementation
details. The 384-dimension space matches `HnswServiceConfig::default_dimensions`.

### Tier 2: Documentation Paragraphs

| Parameter | Value |
|-----------|-------|
| Dimensions | 384 |
| Strategy | Paragraph-level chunking |
| Granularity | One embedding per markdown paragraph (>50 chars) |
| Estimated vectors | ~3,000 (849 docs, ~3.5 paragraphs each) |
| Index | Shared HNSW with tier 1 (different metadata tags) |
| Update trigger | File modification |

Key insight: symposium decisions, SPARC plans, and architecture docs should
be embedded at paragraph level to enable cross-referencing between design
decisions and implementing code.

### Tier 3: Commit Messages

| Parameter | Value |
|-----------|-------|
| Dimensions | 128 |
| Strategy | Sentence-level transformer |
| Granularity | One embedding per commit (subject + optional body) |
| Estimated vectors | 101 (growing ~2/day) |
| Index | Separate HNSW (lower dimensionality) |
| Update trigger | New commits |

Lower dimensionality is sufficient because commit messages are short and
have simpler semantic structure. The commit embedding enables temporal
queries: "what was being worked on when X changed?"

### Tier 4: Type Signatures

| Parameter | Value |
|-----------|-------|
| Dimensions | 256 |
| Strategy | AST-based structural embedding |
| Granularity | One embedding per public type (struct, enum, trait) |
| Estimated vectors | ~400 |
| Index | Separate HNSW |
| Update trigger | Type definition change |

Encode the type structure (field names, variant names, trait bounds) as a
structural vector. This enables queries like "find types similar to
MeshTransport" or "what types have a DashMap field?"

### Tier 5: Causal Relationships

| Parameter | Value |
|-----------|-------|
| Dimensions | 64 |
| Strategy | Graph embedding (node2vec or similar) |
| Granularity | One embedding per causal graph node |
| Estimated vectors | ~1,052 |
| Index | Integrated into causal graph queries |
| Update trigger | Graph topology change |

The causal graph nodes themselves need embeddings to enable similarity-based
traversal: "find nodes causally similar to this symposium decision."

### Total Embedding Budget

| Tier | Vectors | Dimensions | Memory (f32) |
|------|--------:|----------:|-------------:|
| Code chunks | 2,000 | 384 | 3.1 MB |
| Documentation | 3,000 | 384 | 4.6 MB |
| Commits | 101 | 128 | 0.05 MB |
| Type signatures | 400 | 256 | 0.4 MB |
| Causal nodes | 1,052 | 64 | 0.3 MB |
| **Total** | **6,553** | -- | **~8.4 MB** |

This is well within the capability of an ARM SoC (the Mentra research proved
ECC works on Cortex-A53 at 3.6ms/tick).

---

## 6. Recurrent Patterns

### P1: Plan -> Implement -> Test -> Symposium -> Refine

The fundamental development loop. Observed 4+ complete cycles:
- K0 plan -> K0-K2 implementation -> K2 symposium -> K2.1 refinement
- K2.1 -> K3 implementation -> K3 symposium -> K3c planning
- ECC research -> ECC symposium -> K3c implementation
- K5 symposium -> K6 plan -> K6 implementation

**Warp thread**: This pattern IS the three-mode loop (Generate -> Act -> Analyze).

### P2: Feature Gate -> Types First -> Tests -> Wire Later

Every major subsystem follows this progression:
1. Add feature gate to `Cargo.toml`
2. Define types, traits, and enums (Phase 1)
3. Write tests against the type API
4. Wire to runtime backends (Phase 2, often deferred)

Observed in: `exochain`, `ecc`, `mesh`, `os-patterns`, `cluster`, `containers`.

**Warp thread**: The codebase values type-level correctness over runtime wiring.

### P3: Symposium Decision -> Commitment -> Implementation -> Tracking

Decisions flow through a pipeline:
1. Research panel raises question (Qn)
2. Q&A roundtable produces decision (Dn)
3. Decision generates commitment (Cn)
4. Next symposium tracks commitment status
5. Implementation fulfills commitment

K3 symposium explicitly tracked K2 commitments (C1-C10) with status
(shipped/not started/partial/blocked). This is accountability.

**Warp thread**: The codebase takes its own promises seriously.

### P4: DashMap for Concurrency, Arc for Sharing, Mutex for Ordering

Concurrency patterns are remarkably consistent:
- `DashMap<K, V>` for lock-free concurrent maps (ProcessTable, ServiceRegistry,
  A2ARouter inboxes, CrossRefStore, MeshConnectionPool, DistributedProcessTable)
- `Arc<T>` for shared ownership (CausalGraph, HnswService, ImpulseQueue)
- `Mutex<T>` for ordered access to non-concurrent structures (HnswStore,
  CognitiveTickState)
- `AtomicU64` for counters everywhere

**Warp thread**: The concurrency model is stable and does not need reinvention.

### P5: 5-Layer Architecture for Network Stacks

Both the mesh networking and the kernel itself use layered architectures:
- Mesh: Transport -> Encryption -> Framing -> Discovery -> Application
- Kernel: Platform -> Core -> Kernel -> CLI -> User
- Protocol: IPC -> ServiceApi -> Protocol Adapter -> External

**Warp thread**: Layering is the dominant structural metaphor.

### P6: Trait Abstraction -> Mock for Testing -> Real Implementation Later

Every subsystem with external dependencies follows:
1. Define trait (`MeshTransport`, `EmbeddingProvider`, `ArtifactBackend`,
   `DiscoveryBackend`, `EncryptedChannel`, `GateBackend`)
2. Implement mock (`PassthroughChannel`, `MockEmbeddingProvider`,
   `BootstrapDiscovery`, `CapabilityGate`)
3. Test against mock
4. Wire real implementation behind feature gate

**Warp thread**: The codebase values testability over early integration.

### P7: Chain-Anchored Everything

Actions that matter get a chain event:
- Boot sequence events
- Agent spawn/stop/suspend/resume
- IPC message send/receive/acknowledge
- Tool deployment and execution
- Service registration
- Governance decisions
- Genesis rules (constitutional)

**Warp thread**: Auditability is non-negotiable.

### P8: Burst Implementation Pattern

The git history shows a clear rhythm:
- Days of planning/documentation (5-10 commits, mostly `docs:`)
- Hours of implementation burst (10-20 commits, `feat:` + `test:`)
- Brief consolidation (`chore:`, `fix:`)

The Mar 25-26 burst produced 17 commits in 30 hours, delivering K3-K6
implementation, governance, OS gap-filling, and the standalone crate.

**Warp thread**: The development process values thorough planning followed
by rapid execution.

---

## 7. Confidence Assessment

### What We Are Confident About (>0.8)

| Area | Confidence | Evidence |
|------|-----------|----------|
| Codebase structure | 0.95 | Complete file inventory, line counts verified |
| Module relationships | 0.90 | `lib.rs` re-exports map the full dependency tree |
| Symposium decision chain | 0.90 | All 4 symposiums read, decisions cross-referenced |
| Feature gate topology | 0.90 | `Cargo.toml` verified against `lib.rs` |
| Test coverage distribution | 0.85 | `#[test]` count verified per crate |
| Development timeline | 0.85 | Git log provides exact timestamps |
| Concurrency patterns | 0.85 | Consistent DashMap/Arc/Mutex usage observed |
| Recurrent patterns | 0.80 | 8 patterns identified with multiple instances each |

### What Needs More Data (0.4-0.8)

| Area | Confidence | Gap |
|------|-----------|-----|
| Cross-crate dependency graph | 0.70 | Read `Cargo.toml` for each crate needed |
| Runtime behavior | 0.60 | No execution traces; only type-level analysis |
| Integration test coverage | 0.55 | Know test counts but not what they test |
| CI pipeline shape | 0.50 | No CI config files examined |
| Dead code / unused modules | 0.50 | Would need compiler analysis |
| Performance characteristics | 0.40 | Only calibration module gives hardware data |

### What We Have No Data For (<0.4)

| Area | Confidence | What's Missing |
|------|-----------|---------------|
| Runtime embeddings | 0.0 | No EmbeddingProvider implementation beyond mock |
| Live causal graph state | 0.0 | No persisted graph data; only schema |
| Cross-node behavior | 0.0 | Mesh Phase 2 (wire I/O) not implemented |
| Real HNSW index performance | 0.0 | No production vectors loaded |
| User interaction patterns | 0.0 | No telemetry or usage data |
| External crate compatibility | 0.15 | Workspace deps listed but not version-audited |

### Overall Confidence: 0.62

This is a first-pass structural analysis. The topology is well-understood
but the dynamics (runtime behavior, actual embedding performance, real
causal relationships) are entirely unobserved. The Weaver needs live data
ingestion to move past 0.7.

---

## 8. Recommendations

### R1: Ingest Git History as First Data Source

The git log is the highest-value, lowest-effort data source. Each commit
becomes a causal node. Parent-child relationships become `Follows` edges.
Co-modified files become `Correlates` edges. Commit messages embed into
the 128-dimension commit space.

**Priority**: Immediate
**Effort**: ~200 lines (git log parser + causal graph insertion)
**Confidence impact**: +0.08 (temporal structure)

### R2: Ingest Module Dependency Graph

Parse `lib.rs` and `Cargo.toml` files to construct the `Contains` and
`DependsOn` edges automatically. This is deterministic -- no embedding
needed.

**Priority**: Immediate
**Effort**: ~150 lines
**Confidence impact**: +0.06 (structural completeness)

### R3: Implement a Real EmbeddingProvider

The `MockEmbeddingProvider` returns random vectors. For the Weaver to
produce meaningful similarity searches, it needs a real backend.
Options in order of feasibility:
1. Local ONNX model (all-MiniLM-L6-v2, 384d) -- no network dependency
2. LLM-backed embedding via clawft-llm -- uses existing infrastructure
3. External API (OpenAI, Cohere) -- highest quality but adds dependency

**Priority**: High
**Effort**: ~300 lines for ONNX, ~150 for LLM-backed
**Confidence impact**: +0.15 (enables all similarity queries)

### R4: Build Symposium Decision Traceability

Parse symposium documents to extract decisions (Dn) and commitments (Cn).
Create `Causes` edges from decisions to implementing modules. Create
`EvidenceFor` edges from tests to the decisions they validate.

**Priority**: High
**Effort**: ~400 lines (markdown parser + heuristic extraction)
**Confidence impact**: +0.10 (causal completeness)

### R5: Add File-Watch Data Source

The `DataSource::FileTree` variant exists but has no implementation.
Watching the source tree for modifications would enable real-time graph
updates as the codebase evolves.

**Priority**: Medium
**Effort**: ~250 lines (notify crate + diff-to-edge conversion)
**Confidence impact**: +0.05 (continuous updating)

### R6: Connect to ExoChain for Provenance

The causal graph nodes should carry `chain_seq` values linking to ExoChain
events. This gives every causal edge cryptographic provenance.

**Priority**: Medium
**Effort**: ~100 lines (chain_seq assignment in graph insertion)
**Confidence impact**: +0.03 (provenance)

### R7: Implement Cross-Node Graph Synchronization

When mesh Phase 2 lands, the causal graph should synchronize across nodes
using CRDTs for convergence and Merkle proofs for verification (per ECC
symposium decision D4). The `mesh_tree.rs` framework provides the sync
primitives.

**Priority**: Future (blocked on mesh Phase 2)
**Effort**: ~600 lines
**Confidence impact**: +0.08 (distributed cognition)

---

## 9. Self-Reflection

### What the Weaver Learned from This Analysis

1. **The codebase is a conversation.** This is not metaphor. The commit
   history, the symposium transcripts, the SPARC plans, and the Rust source
   code are all utterances in overlapping conversations with different
   participants, media, and temporal rhythms. The ECC model should treat
   them as first-class conversational data, not just "code to index."

2. **The meta-structure is exceptionally rich.** Most codebases have code
   and maybe some docs. This codebase has 4 symposiums, 16 SPARC plans,
   explicit decision tracking, commitment accountability, and a
   constitutional governance model. The meta-conversations (M1-M7) contain
   as much signal as the code itself.

3. **The three-mode loop is real.** The Generate -> Act -> Analyze cycle
   predicted by `06-three-modes.md` is the actual development methodology.
   The Weaver is currently in Analyze mode. The output of this analysis
   will drive the next Generate cycle (ECC initialization plan), which
   will drive the next Act cycle (Weaver implementation).

4. **Feature gates are conversation boundaries.** The `ecc`, `mesh`,
   `exochain`, `os-patterns`, and `cluster` feature gates are not just
   compile-time switches -- they define which subsystem conversations are
   active. A build with `--features ecc,mesh` is participating in different
   conversations than a bare `--features native` build.

5. **The concurrency model is a strength.** The consistent use of DashMap,
   Arc, and AtomicU64 means the causal graph can safely be updated from
   multiple agent threads without introducing new concurrency primitives.
   The Weaver inherits this for free.

6. **The biggest gap is live data.** The type-level structure is well-defined
   but no vectors exist, no causal edges have been computed from real data,
   and no embeddings have been generated. The Weaver's confidence will jump
   from 0.62 to ~0.80 once git history and module dependencies are ingested.

### How the Weaver Should Evolve

1. **Bootstrap from deterministic sources first.** Git log and module
   dependencies produce exact edges. No embedding uncertainty. Build the
   structural skeleton before adding probabilistic similarity.

2. **Treat symposium documents as primary sources.** They contain the
   REASONS behind the code. A causal graph without symposium-derived edges
   is missing the "why" dimension.

3. **Use the cognitive tick for incremental updates.** Don't batch-process
   the entire codebase. Ingest one data source per tick, update the graph
   incrementally, and let confidence accumulate naturally. The calibration
   module already handles tick budget management.

4. **Record all modeling decisions in the Meta-Loom.** The `MetaLoomEvent`
   type exists. Every decision the Weaver makes (which edge type to use,
   which embedding strategy to apply, which sources to prioritize) should
   be recorded for self-improvement tracking.

5. **The stitch operation is key.** The `WeaverCommand::Stitch` command
   can merge two domain models. When the Weaver has separate models for
   "kernel-architecture" and "mesh-networking," stitching them should
   produce the cross-domain causal edges (K5 symposium D1 -> mesh.rs).

---

## Appendix A: Full Conversation Inventory

```
Conversations (12):
  C1  Kernel Architecture        350 nodes, 1,400 edges
  C2  Symposium Decisions         135 nodes,   400 edges
  C3  SPARC Planning               80 nodes,   250 edges
  C4  Mesh Networking             120 nodes,   450 edges
  C5  ECC Cognitive Substrate      90 nodes,   300 edges
  C6  ExoChain Provenance          50 nodes,   150 edges
  C7  Governance Constitution      40 nodes,   120 edges
  C8  Tool Lifecycle               60 nodes,   200 edges
  C9  Agent Architecture           70 nodes,   250 edges
  C10 OS Pattern Library           50 nodes,   160 edges
  C11 Git Commit History          101 nodes,   200 edges
  C12 Three-Mode Engine Design     20 nodes,    60 edges
  -----------------------------------------------
  TOTAL                         1,166 nodes, 3,940 edges

Meta-Conversations (7):
  M1  SPARC about Kernel         (C3 -> C1)
  M2  Symposiums about Design    (C2 -> C1, C4, C5)
  M3  Gap Analysis about Completeness  (all C*)
  M4  Commitment Tracking        (C2 internal)
  M5  Agent Team Design          (about development process)
  M6  This Analysis              (about M1-M5, C1-C12)
  M7  Three-Mode Loop            (about entire process)
```

## Appendix B: ECC Initialization Sequence

The recommended order for the Weaver to initialize its model of this codebase:

```
Step 1: Structural skeleton (deterministic, no embeddings needed)
  1a. Parse lib.rs -> Module nodes + Contains edges
  1b. Parse Cargo.toml -> CrateNode nodes + DependsOn edges
  1c. Parse feature gates -> Feature nodes + GatedBy edges
  1d. Count tests per module -> TestGroup nodes + EvidenceFor edges

Step 2: Temporal structure (deterministic)
  2a. Parse git log -> Commit nodes + Follows edges
  2b. Extract co-modified files -> Correlates edges
  2c. Map commits to K-phases -> Contains edges

Step 3: Decision structure (semi-deterministic, heuristic extraction)
  3a. Parse symposium results -> Decision + Commitment nodes
  3b. Link decisions to implementing modules -> Causes edges
  3c. Track commitment fulfillment -> EvidenceFor / Contradicts edges

Step 4: Embedding generation (requires EmbeddingProvider)
  4a. Embed public function signatures (384d)
  4b. Embed documentation paragraphs (384d)
  4c. Embed commit messages (128d)
  4d. Embed type signatures (256d)

Step 5: Similarity edges (requires embeddings)
  5a. Find similar functions across crates -> Correlates edges
  5b. Link documentation to implementing code -> EvidenceFor edges
  5c. Cluster commits by topic -> Contains edges

Step 6: Continuous operation
  6a. Watch for file changes -> incremental updates
  6b. Watch for new commits -> temporal extension
  6c. Run confidence evaluation per tick
  6d. Record Meta-Loom events for self-improvement
```

Expected confidence trajectory:
- After Step 1: 0.70 (structural skeleton)
- After Step 2: 0.78 (temporal dimension)
- After Step 3: 0.83 (causal completeness)
- After Step 4: 0.85 (embedding space populated)
- After Step 5: 0.88 (similarity edges discovered)
- After Step 6 (steady state): 0.90+ (continuous refinement)

## Appendix C: Key File Paths

Source of truth for the 12 conversations:

```
C1:  crates/clawft-kernel/src/lib.rs (module map)
C2:  docs/weftos/k2-symposium/08-symposium-results-report.md
     docs/weftos/k3-symposium/07-symposium-results-report.md
     docs/weftos/ecc-symposium/05-symposium-results-report.md
     docs/weftos/k5-symposium/05-symposium-results.md
C3:  .planning/sparc/weftos/0.1/00-orchestrator.md
C4:  docs/weftos/k6-development-notes.md
C5:  crates/clawft-kernel/src/weaver.rs
     crates/clawft-kernel/src/causal.rs
     crates/clawft-kernel/src/crossref.rs
C6:  crates/clawft-kernel/src/chain.rs (exochain feature)
C7:  crates/clawft-kernel/src/governance.rs
     docs/weftos/kernel-governance.md
C8:  crates/clawft-kernel/src/wasm_runner.rs
C9:  crates/clawft-kernel/src/agency.rs
     crates/clawft-kernel/src/supervisor.rs
C10: crates/clawft-kernel/src/monitor.rs (os-patterns feature)
C11: git log --oneline (101 commits)
C12: docs/weftos/ecc-symposium/06-three-modes.md
```
