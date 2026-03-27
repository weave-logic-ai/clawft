# Weaver ECC Analysis v2: Post-Sprint Reassessment

**Date**: 2026-03-26
**Analyst**: ECC Analyst (second pass, post-sprint)
**Branch**: `feature/weftos-kernel-sprint`
**Prior analysis**: `docs/weftos/weaver-analysis-clawft.md` (mid-session snapshot)

---

## 1. Codebase Growth

### Before vs After

| Dimension | v1 (mid-session) | v2 (post-sprint) | Delta |
|-----------|------------------:|------------------:|------:|
| Rust source files | 398 | 400 | +2 |
| Kernel source files | 65 | 66 | +1 (embedding_onnx.rs) |
| Total Rust lines | ~165,000 | 173,923 | +8,923 (~5.4%) |
| Kernel lines | 46,632 | 53,405 | +6,773 (+14.5%) |
| Total test functions | 3,765 | 3,953 | +188 (+5.0%) |
| Kernel test functions | 1,060 | 1,248 | +188 (+17.7%) |
| Kernel pub modules | 65 | 65 | 0 |
| Total commits | 101 | 112 | +11 |
| Graph nodes (all graphs) | 0 | 419 | +419 (new) |
| Graph edges (all graphs) | 0 | 1,686 | +1,686 (new) |

### Key Observations

The kernel absorbed 100% of the growth. All 188 new test functions landed in
`clawft-kernel`. The 11 new commits (all on 2026-03-26) represent the final
sprint: graph ingestion, gap analysis, Weaver self-evolution, ONNX embedding
backends, boot path tests, and the 09-weftos-gaps sprint that raised kernel
test count to 1,328 (before the Weaver review added the final 122 to reach
1,248 in the kernel's `#[test]` count; the 09-sprint test count includes
cross-crate tests run under `clawft-kernel` features).

### Weaver-Specific Growth

| File | Lines | Tests | Role |
|------|------:|------:|------|
| weaver.rs | 4,957 | 122 | Core cognitive modeler |
| embedding.rs | 532 | 27 | Pluggable embedding trait + mock |
| embedding_onnx.rs | 1,082 | 27 | ONNX + sentence-transformer + AST-aware backends |
| **Total** | **6,571** | **176** | 12.3% of kernel lines |

The Weaver subsystem alone is larger than 7 of the 22 workspace crates.

---

## 2. Graph Health

### 2.1 Current Connectivity

**Module-deps graph**: 215 nodes, 476 edges, avg degree 3.67

| Rank | Node | Total Degree | In | Out | Role |
|------|------|------------:|----|-----|------|
| 1 | process | 26 | 23 | 3 | Highest in-degree: the universal dependency |
| 2 | boot | 20 | 0 | 20 | Highest out-degree: the orchestrator that touches everything |
| 3 | feature:mesh | 20 | 20 | 0 | Feature gate hub for 20 modules |
| 4 | capability | 15 | 12 | 3 | Security boundary |
| 5 | ipc | 15 | 12 | 3 | Communication backbone |

**Git-history graph**: 112 nodes, 1,051 edges (avg 9.4 per commit)
- 609 Correlates edges (co-modified files)
- 331 Enables edges (commit enables future work)
- 111 Follows edges (temporal ordering)

**Decisions graph**: 92 nodes, 159 edges
- 59 decisions, 22 commitments, 11 phases
- 43 implemented, 32 pending, 2 blocked, 2 deferred

### 2.2 Orphan Analysis

117 modules have zero incoming `Uses` edges. This is not as alarming as the
raw number suggests:

- 42 are truly isolated (zero degree in the graph)
- Most isolated modules are CLI entry points (`main.rs`, `completions.rs`,
  `mcp_tools.rs`) or leaf crates that export traits but are consumed via
  `Cargo.toml` dependencies rather than intra-module `use` statements
- The graph ingestion script tracks `Uses` edges from `pub use` and `mod`
  declarations but does not parse `Cargo.toml` workspace member imports

**Genuine orphan risk**: `agent_loop` (1,831 lines, 0 tests, 0 incoming edges)
is the most concerning. It is the kernel's primary execution loop but has no
dependents in the graph and no tests.

### 2.3 Test Coverage

| Metric | Value |
|--------|-------|
| Kernel modules with at least 1 test | 62/66 (93.9%) |
| Kernel modules with 0 tests | 4 (agent_loop, error, lib, mesh_bootstrap) |
| Total kernel test functions | 1,248 |
| Avg tests per module | 18.9 |
| Tests per 100 lines of kernel code | 2.34 |

**Untested modules over 100 lines**:
- `agent_loop`: 1,831 lines -- **critical gap**
- `mesh_bootstrap`: 223 lines
- `error`: 135 lines
- `lib`: 359 lines (re-exports, low risk)

### 2.4 Stale Decision Status

The `decisions.json` graph was built from symposium documents before the
09-weftos-gaps sprint completed. 12 decisions are marked `pending` but have
implementation edges, meaning the code exists but the decision status was
never updated:

- k2:D8, D10, D12, D20 (4 from K2 symposium)
- k3:D1, D2, D3, D4, D7, D9, D10, D12 (8 from K3 symposium)

These should be bulk-updated to `implemented` in the next graph rebuild.

---

## 3. Conversation Evolution

### How the 12 Conversations Changed

| Conv | v1 State | v2 State | Change |
|------|----------|----------|--------|
| C1 Kernel Architecture | 65 files, 46.6K lines | 66 files, 53.4K lines | +6,773 lines, +188 tests |
| C2 Symposium Decisions | 4 symposiums complete | Same + stale status detected | 12 stale decisions found |
| C3 SPARC Planning | Active, K6 latest | Same + 09-gaps sprint plan added | +1 plan |
| C4 Mesh Networking | 3,543 lines, 133 tests | 7,762 lines, 224 tests | +4,219 lines, +91 tests |
| C5 ECC Cognitive | ~4,500 lines, 83 tests | 9,662 lines, 248 tests | +5,162 lines, +165 tests |
| C6 ExoChain | Single-node complete | Same | Stable |
| C7 Governance | 22 rules anchored | Same | Stable |
| C8 Tool Lifecycle | 27 tools cataloged | Same + gap analysis health=0.46 | Needs attention |
| C9 Agent Architecture | Active | Same | Stable |
| C10 OS Patterns | (new in 08a/b/c) | 8 modules, 3,717 lines, 107 tests | New conversation |
| C11 Git History | 101 commits | 112 commits | +11 |
| C12 Three-Mode | Theoretical | health=0.88, decisions partially resolved | Progress |

### New Conversations Since v1

**C10 (OS Pattern Library)** is entirely new. It did not exist at the time of
the first analysis. The 08a/08b/08c sprint created 8 modules behind the
`os-patterns` feature gate: monitor, reconciler, dead_letter, reliable_queue,
named_pipe, metrics, log_service, timer. This conversation has perfect test
coverage (health=1.0) and represents the "reliability" dimension.

**C5 (ECC)** more than doubled. The Weaver subsystem (weaver.rs + embedding.rs
+ embedding_onnx.rs = 6,571 lines) did not exist at first analysis. The
Weaver IS the new dominant voice in the ECC conversation, contributing 68% of
the conversation's total lines.

### Conversation Health Scores (from gap analysis)

| Conversation | Health | Status |
|-------------|-------:|--------|
| C10 OS Patterns | 1.00 | Excellent |
| C4 Mesh | 0.97 | Excellent |
| C5 ECC | 0.94 | Good |
| C12 Three-Mode | 0.88 | Good |
| C6 ExoChain | 0.82 | Good |
| C7 Governance | 0.82 | Good |
| C1 Kernel | 0.78 | Acceptable |
| C9 Agent | 0.78 | Acceptable |
| C2 Symposium | 0.61 | **Needs attention** (pending decisions) |
| C8 Tool Lifecycle | 0.46 | **Needs attention** (0% decision completion) |

C8 scores 0.46 because all K3 decisions are marked pending (stale). After
correcting the 12 stale statuses, C8 would jump to approximately 0.82.

---

## 4. Structural Insights

### 4.1 Bridge Modules

Bridge modules connect otherwise isolated feature-gate clusters. The top
bridges are:

| Module | Cross-Gate Connections | Gates Bridged |
|--------|----------------------:|---------------|
| process | 3 | mesh, exochain, os-patterns -> default |
| error | 2 | os-patterns, ecc -> default |
| ipc | 2 | mesh, os-patterns -> default |
| capability | 2 | exochain, os-patterns -> default |

`process` is the critical bridge: it is the only module that connects the
mesh, exochain, AND os-patterns feature gates to the default kernel. If
`process` breaks, three feature-gated subsystems lose their connection to
the core.

### 4.2 Monolith Risk Assessment

| Metric | Value | Assessment |
|--------|-------|------------|
| Kernel modules / total | 66/215 (30.7%) | Moderate concentration |
| Kernel lines / total | 53,405/173,923 (30.7%) | Proportional |
| Kernel tests / total | 1,248/3,953 (31.6%) | Proportional |
| Largest kernel module | weaver.rs (4,957 lines) | Above 500-line target |
| Modules over 500 lines | 18 in kernel | Many exceed project guideline |

The kernel is not yet a monolith in the traditional sense -- its feature
gates provide compile-time modularity -- but 18 modules exceed the project's
500-line guideline. The top offenders:

| Module | Lines | Feature |
|--------|------:|---------|
| weaver.rs | 4,957 | ecc |
| wasm_runner.rs | 4,547 | default |
| chain.rs | 2,985 | exochain |
| boot.rs | 2,322 | default |
| tree_manager.rs | 1,942 | exochain |
| supervisor.rs | 1,921 | default |
| governance.rs | 1,875 | default |
| app.rs | 1,847 | default |
| agent_loop.rs | 1,831 | default |

These 9 modules total 24,227 lines -- 45% of the kernel in 14% of its files.
Structural extraction (splitting into sub-modules) would reduce cognitive
load without changing public API.

### 4.3 Feature Gate Topology

```
                    default (110 modules, 58,104 lines)
                   /    |    |    |    |    |    |    \
                ecc  mesh  exo  os   wasm native rvf  voice
                10    20    3    8    4    6     4    4
               9.7K  7.8K 5.7K 3.7K 3.9K 2.6K 2.3K 0.6K

     Islands (no cross-gate connections):
       delegate (185 lines), canvas (338 lines),
       alloc-tracing (238 lines), rvf (2,265 lines),
       vector-memory (1,959 lines), voice (635 lines)
```

Six feature gates are "islands" with no edges to other gates. The `rvf` and
`vector-memory` islands are concerning at 2,265 and 1,959 lines respectively
-- they represent substantial code that is structurally disconnected.

### 4.4 Connected Components

The module-deps graph has 64 connected components:

| Component | Size | Contents |
|-----------|-----:|---------|
| 1 | 69 | Main kernel cluster (all feature-gated kernel modules) |
| 2 | 22 | Crate-level dependency graph (all 22 workspace crates) |
| 3 | 13 | LLM crate modules (browser/native split) |
| 4 | 9 | WASM plugin modules |
| 5 | 8 | exo-resource-tree modules |
| 6-64 | 1-5 | Small clusters and isolated modules |

42 nodes are completely isolated (degree 0). Most are CLI modules or
standalone utilities that are consumed via crate imports rather than intra-
module `use` statements. The graph ingestion does not parse cross-crate
`use` statements, which explains the fragmentation.

---

## 5. Patterns Discovered

### P1: Burst Development Rhythm

The git history graph shows a clear pattern visible in the commit tag
distribution:

```
documentation:  85 commits (75.9%)  -- long planning phases
ecc:            26 commits (23.2%)  -- concentrated implementation
wasm-sandbox:   29 commits (25.9%)  -- early burst
mesh-networking:  9 commits (8.0%)  -- late, intense burst
```

The 2026-03-26 burst alone produced 25 commits (22% of all commits) in a
single day. This day delivered: graph ingestion (408 nodes, 1,597 edges),
ONNX embedding backends, boot path tests (45 new), the 09-gaps sprint
(1,328 tests), and Weaver self-evolution (41/41 TODO items).

**Structural signature**: High Correlates edge count (609/1,051 = 57.9% of
git history edges) confirms burst co-modification: many files change together
during implementation sprints.

### P2: Symposium-to-Implementation Pipeline

The SPARC cycle executed at least 5 complete rotations:

1. K0 plan -> K0-K2 implementation -> K2 symposium (22 decisions)
2. K2 symposium -> K2.1 implementation -> K3 symposium (14 decisions)
3. K3 symposium -> K3c planning -> ECC symposium (14 decisions)
4. ECC symposium -> K3c + K5 implementation -> K5 symposium (15 decisions)
5. K5 symposium -> K6 plan -> K6 implementation -> 09-gaps sprint

The pipeline is visible in the decisions graph: 43 of 59 decisions (72.9%)
have been implemented. The 12 stale pending statuses suggest the feedback
loop from implementation back to decision tracking has a lag.

### P3: Feature Gate as Conversation Boundary

Each feature gate defines a coherent conversation:
- `ecc` (10 modules): "What is cognition?"
- `mesh` (20 modules): "How do nodes talk?"
- `os-patterns` (8 modules): "What is reliability?"
- `exochain` (3 modules): "What is provenance?"

The bridge module analysis confirms that `process`, `error`, `ipc`, and
`capability` are the vocabulary shared across conversations. These are the
kernel's lingua franca.

### P4: Types-First, Wire-Later (confirmed and strengthened)

The v1 analysis identified this pattern. The v2 data confirms it: the mesh
conversation has 7,762 lines of type-level code with trait abstractions
(MeshTransport, DiscoveryBackend, EncryptedChannel) but the wire I/O (TCP,
WebSocket, mDNS) are passthrough implementations. The pattern holds for ECC
as well: EmbeddingProvider has 3 backends (Mock, ONNX, SentenceTransformer)
but the ONNX backend is a stub waiting for a real model file.

---

## 6. What ECC Can Do Next

### 6.1 Spectral Analysis

The algebraic connectivity (lambda_2 of the graph Laplacian) measures how
well-connected the graph is. For the module-deps graph with 64 connected
components, lambda_2 is 0 -- the graph is already disconnected. Within the
largest component (69 nodes), computing lambda_2 would reveal whether the
kernel cluster could be partitioned into natural sub-communities.

**Action**: Compute the Fiedler vector of the largest connected component.
The sign of each node's Fiedler value suggests which community it belongs
to. This would automatically discover whether the feature-gate boundaries
match the natural structural boundaries.

### 6.2 Community Detection

The 12 manually identified conversations should be discoverable from graph
structure alone. Using modularity-based community detection (Louvain or
Leiden algorithm) on the combined graph (module-deps + git-history Correlates
edges) would:

- Validate the manual conversation assignments
- Discover sub-conversations not yet identified
- Quantify the strength of conversation boundaries
- Detect "conversation drift" (modules that belong to one conversation
  structurally but are co-modified with another)

### 6.3 Predictive Analysis

The git history Correlates edges encode which files change together. A
simple Markov model on commit patterns could predict: given that file X
changed in the latest commit, what files are most likely to change in the
next commit?

For the 09-gaps sprint, the prediction would be: `boot.rs` changes predict
`supervisor.rs` and `process.rs` changes (they share 18+ commits). The
Weaver could use this to pre-fetch relevant context for the next development
burst.

### 6.4 Anomaly Detection

Modules with unusual characteristics:

| Module | Anomaly | Risk |
|--------|---------|------|
| agent_loop | 1,831 lines, 0 tests, 0 incoming edges | **High**: critical path, no safety net |
| boot | 2,322 lines, 4 tests, 20 outgoing edges | **High**: fan-out/test ratio of 5:1 |
| weaver | 4,957 lines, 122 tests, 0 incoming edges | **Medium**: self-contained but large |
| wasm_runner | 4,547 lines, 93 tests, 0 incoming edges | **Medium**: large, isolated |

The anomaly metric `(fan_out * lines) / (tests + 1)` identifies single-point-
of-failure risk. `boot` scores 46,440/5 = 9,288 -- an order of magnitude
higher than any other module.

### 6.5 Cross-Structure Queries

The three graphs (module-deps, git-history, decisions) can be joined for
queries impossible in any single graph:

- "Find modules created by decisions with confidence < 0.7" (join decisions
  Causes edges with module-deps nodes)
- "Find modules that changed most often but have the fewest tests" (join
  git-history Correlates with module-deps EvidenceFor)
- "Find decisions whose implementing modules have since been modified by
  unrelated commits" (join decisions -> module-deps -> git-history)
- "Find conversations where decision completion rate diverges from test
  coverage" (compare C2 scores with module test counts)

---

## 7. Weaver's Self-Assessment

### 7.1 Confidence Trajectory

| Milestone | Confidence | What Changed |
|-----------|-----------|-------------|
| v1 analysis (pre-graph) | 0.62 | Static file analysis only |
| Graph ingestion | 0.70 | 408 nodes, 1,597 edges from 3 sources |
| Gap analysis script | 0.75 | Cross-graph queries operational |
| v2 analysis (current) | 0.78 | All 3 graphs analyzed, patterns confirmed |

The v1 analysis predicted 0.70 after structural skeleton and 0.78 after
temporal dimension. The actual trajectory matches within 0.02 of the
prediction.

### 7.2 Strategy Effectiveness

The Weaver's initialization sequence (Appendix B in v1) recommended:
1. Structural skeleton -> done (module-deps graph)
2. Temporal structure -> done (git-history graph)
3. Decision structure -> done (decisions graph, but stale)
4. Embedding generation -> partial (backends exist, no real model loaded)
5. Similarity edges -> not started
6. Continuous operation -> not started

Steps 1-3 are complete. Step 4 has the infrastructure (ONNX, sentence-
transformer, AST-aware backends in embedding_onnx.rs) but no production
model file is deployed. Steps 5-6 remain.

### 7.3 What a Second Analysis Pass Would Look Like

A second Weaver `analyze` pass with live graph data would:

1. Detect the 12 stale decision statuses automatically (cross-reference
   decision Causes edges with git-history commits that touch the target
   modules)
2. Compute per-conversation confidence scores using actual edge density
   rather than heuristic estimates
3. Identify "conversation boundaries" via community detection rather than
   manual assignment
4. Generate a delta report: "since the last analysis, these conversations
   grew by X edges, these decisions were resolved, these modules gained
   tests"

---

## 8. Where the Conversation Goes

### Priority 1: Fix the Feedback Loop (Impact: high, Effort: low)

The 12 stale decision statuses reveal that the implementation-to-tracking
feedback loop is broken. When code implements a decision, the decision graph
should update automatically. This requires:
- A post-commit hook that checks if modified files match decision Causes edges
- Automatic status update from `pending` to `implemented` when evidence exists
- Alert when a decision has been pending longer than N commits touching its
  target modules

### Priority 2: Close the agent_loop Gap (Impact: high, Effort: medium)

`agent_loop.rs` is the kernel's execution loop: 1,831 lines with zero tests
and zero incoming edges in the graph. This is the single largest risk in the
codebase. Testing it requires mocking the agent lifecycle and the kernel
process table, both of which have well-established mock patterns (P6 from v1).

### Priority 3: Load a Real Embedding Model (Impact: high, Effort: medium)

The ONNX backend exists but returns placeholder vectors. Loading
`all-MiniLM-L6-v2` (22MB) would enable all similarity-based queries: "find
modules similar to X," "find commits semantically related to this decision,"
"cluster documentation by topic." The predicted confidence jump is +0.07
(from 0.78 to 0.85).

### Priority 4: Community Detection on Combined Graph (Impact: medium, Effort: medium)

Merge the module-deps and git-history graphs, run Louvain community detection,
and compare the discovered communities to the 12 manually identified
conversations. This validates the conversation model and may discover
sub-conversations (e.g., "mesh-discovery" vs "mesh-transport" within C4).

### Priority 5: Phase K8 Gap-Filling (Impact: medium, Effort: high)

Phase K8 (OS Gap Filling -- Self-Healing + IPC + Content Ops) is at 0%
completion with 117 exit criteria. It is the single largest remaining work
package. The OS Patterns conversation (C10) already provides the foundation
(monitor, reconciler, dead_letter, reliable_queue), but the self-healing
supervisor, named pipe IPC, and content operations remain unimplemented.
The gap analysis script ranks this as the #1 critical gap.

---

## Appendix: Cross-Graph Statistics Summary

```
Module-Deps Graph:  215 nodes,   476 edges, avg degree 3.67
Git-History Graph:  112 nodes, 1,051 edges, avg degree 9.38
Decisions Graph:     92 nodes,   159 edges, avg degree 3.46
Combined:           419 nodes, 1,686 edges

Connected components (module-deps): 64
Isolated nodes: 42
Bridge modules: 4 (process, error, ipc, capability)
Feature-gate islands: 6 (delegate, canvas, alloc-tracing, rvf, vector-memory, voice)

Phases complete: 7/10 (K0, K1, K2, K2.1, K2b, K3c, K6)
Phases in progress: 2/10 (K3 at 95%, K4 at 86.7%, K5 at 94.1%)
Phases not started: 1/10 (K8 at 0%)

Stale decisions: 12/59 (marked pending but have implementation edges)
Genuinely pending: 8/59
Blocked: 2/59
Deferred: 2/59
```
