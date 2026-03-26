# WeftOS Gap-Filling Symposium

**Date**: 2026-03-26
**Purpose**: Synthesize ECC graph analysis, pending decisions, and outstanding work into the 09-weftos-gaps sprint plan
**Predecessor**: K5 Symposium (2026-03-25), K6 Implementation (2026-03-26)
**Branch**: `feature/weftos-kernel-sprint`

---

## 1. Session Context

Today's session accomplished a significant breadth of work across four workstreams:

1. **K6 Mesh Networking** -- Completed Phase 1 of the transport-agnostic encrypted mesh network (48/48 exit criteria). Delivered 20 `mesh_*.rs` files with 3,543 lines and 133 tests covering transport traits, Noise encryption, Kademlia DHT, mDNS discovery, SWIM heartbeats, chain replication, tree sync, and hybrid KEM post-quantum key exchange.

2. **OS Gap-Filling Plans (08a/b/c)** -- Split the monolithic `08-phase-os-gap-filling.md` into three focused SPARC phases: 08a Self-Healing & Process Management (P0), 08b Reliable IPC & Observability (P1), 08c Content Integrity & Operational Services (P2). Together these specify ~5,750 new lines and ~960 changed lines across 117 exit criteria.

3. **Weaver ECC Initialization** -- Conducted the first real codebase analysis via the Weaver cognitive modeler. Produced the 911-line `weaver-analysis-clawft.md` identifying 12 primary conversations, 7 meta-conversations, 8 recurrent patterns, and a confidence assessment of 0.62 (structural).

4. **Graph Ingestion & Gap Analysis** -- Ingested git history (102 nodes, 965 edges), module dependencies (214 nodes, 473 edges), SPARC plans (11 phases), and symposium decisions (59 decisions, 22 commitments) into `.weftos/graph/`. Ran gap analysis producing a ranked list of 20 top gaps.

**Current kernel test count**: 1,197 passed (up from 843 baseline at K5 symposium)

---

## 2. ECC Graph Findings

### Codebase Health Dashboard

| Metric | Value | Assessment |
|--------|-------|-----------|
| Total crates | 22 | -- |
| Total Rust source files | 398 | -- |
| Total Rust lines | ~165,000 | -- |
| Total kernel tests | 1,197 | GOOD -- 42% growth since K5 |
| Total workspace tests | 3,765 | -- |
| Orphan modules (no causal edges) | 116 | HIGH -- need wiring |
| Untested modules (per gap report) | 178 | CRITICAL -- structural |
| Pending decisions | 23 | HIGH -- decision debt |
| Pending commitments | 14 | HIGH -- promise debt |
| Blocked decisions | 1 (k2:D11) | MEDIUM -- post-quantum |
| Feature gate islands | 6 | MEDIUM -- composition risk |
| Weaver confidence | 0.78 (post-ingestion) | GOOD -- structural model solid |
| Phase K8 exit criteria | 0/117 | CRITICAL -- not started |

### Phase Completion Summary

| Phase | Title | Exit Criteria | Checked | Status |
|-------|-------|:------------:|:-------:|--------|
| K0 | Kernel Foundation | 20 | 20 | Complete |
| K1 | Supervisor + RBAC + ExoChain | 20 | 20 | Complete |
| K2 | Agent-to-Agent IPC | 15 | 15 | Complete |
| K2b | Kernel Work-Loop Hardening | 9 | 9 | Complete |
| K2.1 | Symposium Implementation | 28 | 28 | Complete |
| K3 | WASM Tool Sandboxing | 20 | 19 | **86.7% -- 1 remaining** |
| K3c | ECC Cognitive Substrate | 15 | 15 | Complete |
| K4 | Container Integration | 15 | 13 | **86.7% -- 2 remaining** |
| K5 | Application Framework | 17 | 16 | **94.1% -- 1 remaining** |
| K6 | Encrypted Mesh Network | 48 | 48 | Complete |
| K8 | OS Gap Filling | 117 | 0 | **Planned -- not started** |
| **Total** | | **324** | **204** | **63.0%** |

### Conversation Health (from Weaver Analysis)

| ID | Conversation | Medium | State | Causal Nodes | Causal Edges | Health |
|----|-------------|--------|-------|:------------:|:------------:|--------|
| C1 | Kernel Architecture | 65 source files | Active | ~350 | ~1,400 | Strong -- center of gravity |
| C2 | Symposium Decisions | 27 documents | Complete | ~135 | ~400 | 23 pending decisions |
| C3 | SPARC Planning | 16 files | Active | ~80 | ~250 | Good -- drives C1 |
| C4 | Mesh Networking | 20 mesh_*.rs | Phase 1 done | ~120 | ~450 | Phase 2 pending (wire I/O) |
| C5 | ECC Cognitive Substrate | 9 ECC modules | Implemented | ~90 | ~300 | 83 tests, solid |
| C6 | ExoChain Provenance | chain.rs, tree_manager.rs | Complete (single-node) | ~50 | ~150 | Needs K6 cross-node ext. |
| C7 | Governance Constitution | governance.rs, gate.rs | Active | ~40 | ~120 | 22 constitutional rules |
| C8 | Tool Lifecycle | wasm_runner.rs, capability.rs | Complete | ~60 | ~200 | 27 tools cataloged |
| C9 | Agent Architecture | agency.rs, supervisor.rs | Active | ~70 | ~250 | Needs restart strategies |
| C10 | OS Pattern Library | 8 os-patterns modules | Complete | ~50 | ~160 | 0/117 exit criteria |
| C11 | Git Commit History | 102 commits | Active | ~101 | ~200 | Burst rhythm documented |
| C12 | Three-Mode Engine | Mentra research | Theoretical | ~20 | ~60 | Foundation for Weaver |

### Fragility Map -- Critical Boot Path

The boot path (`boot.rs`) is the single highest-risk module in the kernel. It has 19 outgoing dependencies but only 2 tests for 1,953 lines of code.

```
boot.rs (1,953 lines, 2 tests)
  |-- process.rs (1,118 lines, 57 tests)
  |     |-- supervisor.rs (760 lines, 5 tests)
  |           |-- agent_loop.rs (1,412 lines, 0 tests)  <-- ZERO TESTS
  |-- a2a.rs (1,452 lines, 1 test)
  |-- service.rs (530 lines, 10 tests)
  |-- chain.rs (2,814 lines, 0 tests)  <-- ZERO TESTS
  |-- wasm_runner.rs (4,423 lines, 0 tests)  <-- ZERO TESTS
  |-- app.rs (1,847 lines, 42 tests)
  |-- governance.rs (910 lines, 38 tests)
  |-- tree_manager.rs (1,942 lines, 31 tests)
```

**Zero-test modules on the critical path**:
- `agent_loop.rs` (1,412 lines) -- the actual agent execution loop
- `chain.rs` (2,814 lines) -- ExoChain implementation
- `wasm_runner.rs` (4,423 lines) -- WASM tool sandbox
- `host.rs` in clawft-channels (561 lines) -- channel host

**Low-test modules on the critical path**:
- `boot.rs` (1,953 lines, 2 tests) -- kernel boot sequence
- `a2a.rs` (1,452 lines, 1 test) -- agent-to-agent router
- `daemon.rs` in clawft-weave (1,647 lines, 1 test) -- weave daemon
- `bootstrap.rs` in clawft-core (654 lines, 3 tests) -- core bootstrap

---

## 3. Decision Backlog

### K3 Symposium Decisions (0% implemented, all pending)

| Decision | Title | Blocks | Priority |
|----------|-------|--------|----------|
| k3:D1 | Hierarchical ToolRegistry with kernel base + per-agent overlays | commitment:k3:AC-1, module:wasm_runner | HIGH |
| k3:D2 | Context-based gate actions: generic tool.exec with rich context | commitment:k3:AC-2, module:agent_loop | HIGH |
| k3:D3 | Implement all 25 remaining tools in K4 | commitment:k3:AC-4 | MEDIUM |
| k3:D4 | Wasmtime integration in K4 alongside container runtime | commitment:k3:AC-5, module:wasm_runner | HIGH |
| k3:D5 | Disk-persisted compiled module cache | -- | LOW |
| k3:D6 | Configurable WASI with read-only sandbox default | -- | MEDIUM |
| k3:D7 | Tree metadata for version history | commitment:k3:AC-6 | MEDIUM |
| k3:D8 | Informational revocation; governance gate handles enforcement | -- | MEDIUM |
| k3:D9 | Central authority + CA chain for tool signing | commitment:k3:AC-7 | MEDIUM |
| k3:D10 | Separate ServiceApi and BuiltinTool concepts | module:service | MEDIUM |
| k3:D11 | Routing-time gate deferred to K5 cluster build | -- | DEFERRED |
| k3:D12 | Multi-layer sandboxing: governance + environment + sudo override | commitment:k3:AC-3, module:wasm_runner | HIGH |
| k3:D13 | WASM snapshots unnecessary now; noted for K6 | -- | DEFERRED |
| k3:D14 | tiny-dancer scoring for native vs WASM routing | -- | LOW |

### K2 Symposium Pending (8 of 22 decisions unresolved)

| Decision | Title | Status | Blocks |
|----------|-------|--------|--------|
| k2:D8 | ExoChain-stored immutable API contracts | pending | commitment:k2:C3, module:chain |
| k2:D10 | WASM-compiled shell with container sandbox | pending | commitment:k2:C5, module:wasm_runner |
| k2:D11 | Enable post-quantum signing immediately | **blocked** | commitment:k2:C6 |
| k2:D12 | Chain-agnostic blockchain anchoring trait | pending | commitment:k2:C7 |
| k2:D13 | Zero-knowledge proofs as foundational capability | pending | -- |
| k2:D17 | Layered routing: tiny-dancer learned + governance enforced | pending | -- |
| k2:D18 | SONA at K5, training data pipeline from K3 forward | pending | -- |
| k2:D20 | Configurable N-dimensional effect algebra | pending | commitment:k2:C9, module:governance |

### ECC Symposium Pending (1 of 8)

| Decision | Title | Status |
|----------|-------|--------|
| ecc:D5 | DEMOCRITUS as continuous nervous system operation | pending |

### Blocked Decisions

| Decision | Title | Blocker | Impact |
|----------|-------|---------|--------|
| k2:D11 | Enable post-quantum signing immediately | rvf-crypto API gap -- ML-DSA-65 DualKey exists but is not yet callable from kernel | Blocks commitment:k2:C6 (post-quantum dual signing), which blocks K2.1 completion |

### Commitment Tracking

| Commitment | Description | Status | Phase |
|-----------|-------------|--------|-------|
| k2:C1 | SpawnBackend enum | **implemented** | K2.1 |
| k2:C2 | ServiceApi internal surface trait | pending | K3 |
| k2:C3 | Chain-anchored service contracts | pending | K3 |
| k2:C4 | Dual-layer gate in A2ARouter | partial | K3 |
| k2:C5 | WASM-compiled shell pipeline | pending | K3/K4 |
| k2:C6 | Post-quantum dual signing (DualKey) | **blocked** | K2.1 |
| k2:C7 | ChainAnchor trait for blockchain anchoring | pending | K3/K4 |
| k2:C8 | SpawnBackend::Tee variant | **implemented** | K2.1 |
| k2:C9 | Configurable N-dimensional EffectVector | pending | K3 |
| k2:C10 | K6 SPARC spec (draft) | **implemented** | pre-K6 |
| k3:AC-1 | Hierarchical ToolRegistry with Arc-shared base | pending | K4 |
| k3:AC-2 | Enriched gate context (tool name + effect vector) | pending | K4 |
| k3:AC-3 | Multi-layer FsReadFileTool sandboxing | pending | K4 |
| k3:AC-4 | All 25 remaining tool implementations | pending | K4 |
| k3:AC-5 | Wasmtime activation + disk cache + WASI scope | pending | K4 |
| k3:AC-6 | Tree version history persistence | pending | K4 |
| k3:AC-7 | CA chain signing for tools | pending | K4 |
| k5:C1 | MessageTarget::RemoteNode variant | **implemented** | K6.0 |
| k5:C2 | GlobalPid composite identifier | **implemented** | K6.0 |
| k5:C3 | MeshTransport trait | **implemented** | K6.1 |
| k5:C4 | mesh feature gate structure | **implemented** | K6.0 |
| k5:C5 | Cluster-join authentication protocol | **implemented** | K6.0 |

**Summary**: 8 of 22 commitments implemented. 1 blocked. 13 pending (all in K3/K4 scope).

---

## 4. Outstanding Work Packages

### Weaver TODO (20 remaining items)

**EmbeddingProvider Backends** (5 items):
- [ ] ONNX backend (all-MiniLM-L6-v2, 384 dimensions) -- behind `onnx-embeddings` feature
- [ ] LLM API backend (call clawft-llm provider for embeddings)
- [ ] Sentence-transformer backend for documentation chunks
- [ ] AST-aware backend for Rust code (function signatures, type definitions)
- [ ] Batch embedding pipeline (process multiple chunks in one call)

**Cognitive Tick Integration** (4 items):
- [ ] Register Weaver with CognitiveTick as a tick consumer
- [ ] Budget-aware tick processing (respect tick_budget_ratio)
- [ ] Incremental git polling (detect new commits since last tick)
- [ ] File watcher integration (detect source file changes)

**Confidence Improvement** (4 items):
- [ ] Confidence scoring based on edge coverage (% of modules with causal edges)
- [ ] Gap detection: modules with no incoming/outgoing causal edges
- [ ] Suggestion engine: recommend data sources based on confidence gaps
- [ ] Confidence history tracking in meta-Loom

**Export / Import** (4 items):
- [ ] weave-model.json export CLI command (`weaver ecc export`)
- [ ] weave-model.json import on edge devices
- [ ] Model diff (compare two exported models)
- [ ] Model merge (stitch models from different analysis sessions)

**Self-Improvement** (3 items):
- [ ] Track which analysis strategies improved confidence (meta-Loom)
- [ ] Cross-domain pattern library persistence
- [ ] Recommend tick interval adjustments based on change frequency

### Weaver Self-Assessment: What It Can and Cannot Do

**Can do now**:
- Structural analysis of Rust codebase (file inventory, line counts, test counts)
- Module dependency graph extraction from `lib.rs` and `Cargo.toml`
- Git history ingestion and temporal pattern recognition
- SPARC plan and symposium decision graph ingestion
- Gap report generation with ranked severity
- Confidence assessment with evidence-based scoring

**Cannot do yet**:
- Real embedding-based similarity search (only mock embeddings exist)
- Live incremental monitoring (no CognitiveTick integration)
- Runtime behavior analysis (no execution traces)
- Cross-node causal modeling (mesh Phase 2 not wired)
- Self-improving strategy selection (meta-Loom types exist but no persistence)
- Model export/import for edge devices

### 08 Exit Criteria Status

**08a Self-Healing & Process Management**: 0/29 passing
- 0/19 K1 Process & Supervision Gate items
- 0/10 K2b Hardening Gate items
- Target: 60+ new tests

**08b Reliable IPC & Observability**: 0/33 passing
- 0/17 K2 IPC & Communication Gate items
- 0/16 Cross-Cutting Gate items
- Target: 80+ new tests

**08c Content & Operations**: 0/42 passing
- Target: 60+ new tests
- Includes WeaverEngine, config/secrets, auth, artifact store, tree views, artifact exchange, log aggregation

**Full Gap-Filling Gate**: 0/12 meta-criteria passing
- All 08a/08b/08c must pass
- All existing tests pass (1,197+ baseline)
- 200+ new tests across all three phases
- Clippy clean, feature gated, no new mandatory dependencies

### Documentation Gaps

From the orchestrator checklist, the following documentation items are unchecked:

- [ ] Documentation updated in Fumadocs site (for 08a)
- [ ] Documentation updated in Fumadocs site (for 08b)
- [ ] Documentation updated in Fumadocs site (for 08c)
- [ ] `scripts/08-gate.sh` passes all checks
- [ ] `scripts/build.sh gate` passes with `os-patterns` feature

---

## 5. Proposed Sprint Structure: 09-weftos-gaps

### Sprint 1: Test Coverage & Stability (1 week)

**Goal**: Eliminate zero-test critical path modules and establish regression safety net.

| Work Item | Target | Files |
|-----------|--------|-------|
| Boot path tests | 15+ tests for boot.rs | `crates/clawft-kernel/src/boot.rs` |
| Agent loop tests | 20+ tests for agent_loop.rs | `crates/clawft-kernel/src/agent_loop.rs` |
| Chain tests | 15+ tests for chain.rs | `crates/clawft-kernel/src/chain.rs` |
| WASM runner tests | 20+ tests for wasm_runner.rs | `crates/clawft-kernel/src/wasm_runner.rs` |
| A2A router tests | 10+ tests for a2a.rs | `crates/clawft-kernel/src/a2a.rs` |
| Feature composition tests | Gate interaction tests | Cross-feature test module |
| Orphan module wiring | Reduce orphan count by 50% | Module dependency analysis |

**Exit criteria**: Zero critical-path modules with 0 tests. Boot path has 15+ tests.

### Sprint 2: Decision Resolution (1 week)

**Goal**: Clear the decision backlog and unblock downstream work.

| Work Item | Decisions | Approach |
|-----------|-----------|----------|
| K3 triage | 14 decisions | Implement D1, D2, D4, D12 (HIGH); defer D5, D11, D13, D14 |
| K2 pending | 8 decisions | Resolve D8, D10, D20 (block commitments); defer D13, D17, D18 |
| Unblock k2:D11 | 1 blocked | Investigate rvf-crypto ML-DSA-65 API; implement DualKey call path |
| K2 commitments | C2, C3, C4, C5, C7, C9 | Implement or explicitly defer with rationale |
| K3 commitments | AC-1 through AC-7 | Implement AC-1, AC-2, AC-3 (HIGH); schedule AC-4/5/6/7 for K4 |
| ECC pending | ecc:D5 | Resolve DEMOCRITUS continuous operation decision |

**Exit criteria**: Pending decisions reduced from 23 to <8. Blocked count = 0.

### Sprint 3: Weaver Runtime (1 week)

**Goal**: Make the Weaver operational with live data ingestion and real embeddings.

| Work Item | Weaver TODO Items | Priority |
|-----------|-------------------|----------|
| ONNX embedding backend | 1 item | P0 -- enables real similarity search |
| CognitiveTick integration | 4 items | P0 -- enables live monitoring |
| Confidence scoring | 2 items (edge coverage + gap detection) | P1 |
| Export CLI | 1 item (weave-model.json export) | P1 |
| Meta-Loom persistence | 1 item (strategy tracking) | P2 |

**Exit criteria**: Weaver confidence >0.85. Real embeddings operational. Tick integration live.

### Sprint 4: Integration & Polish (1 week)

**Goal**: Feature gate composition tests, documentation, and sprint gate script.

| Work Item | Scope |
|-----------|-------|
| Feature composition matrix | Test all 6 gate combinations (native, exochain, ecc, mesh, os-patterns, os-full) |
| `scripts/09-gate.sh` | Sprint gate verification script |
| Fumadocs updates | 08a/08b/08c documentation pages |
| `scripts/build.sh gate` | Verify passes with os-patterns feature |
| Incomplete phase cleanup | K3 (1 remaining), K4 (2 remaining), K5 (1 remaining) exit criteria |

**Exit criteria**: `scripts/09-gate.sh` passes. All feature gate combos build and test clean.

---

## 6. Questions for Discussion

### Q1: K3 Decision Triage Strategy

Should we resolve ALL 14 K3 decisions or triage some as permanently deferred? The K3 symposium produced decisions about K4 implementation details (Wasmtime, tool implementations, WASI config). Some may be superseded by the 08c content ops plan.

**Recommendation**: Implement D1 (hierarchical ToolRegistry), D2 (context gate actions), D4 (Wasmtime), D12 (multi-layer sandbox). Defer D5, D11, D13, D14 as post-1.0.

### Q2: Post-Quantum Signing Blocker

Decision k2:D11 is blocked because rvf-crypto has ML-DSA-65 DualKey but the kernel does not call it. Is the rvf-crypto API gap fixable within the sprint, or should we work around it?

**Options**:
- (a) Add the DualKey call path to `chain.rs` (estimated ~100 lines, depends on rvf-crypto surface)
- (b) Mark k2:C6 as deferred-to-post-1.0 with rationale
- (c) Investigate whether K6 dual signing for cross-node events (k5:D9, already implemented) subsumes this

### Q3: Weaver ONNX Embedding Model

Should the Weaver's ONNX embedding backend (all-MiniLM-L6-v2, 384d) use the same HNSW index as clawft-core's HnswStore, or maintain a separate index? Sharing reduces memory but couples the Weaver to the core runtime.

**Recommendation**: Separate HNSW index. The Weaver's embedding space (codebase semantics) is different from the core's (agent conversation memory). Cross-referencing via CrossRefStore handles the linkage.

### Q4: Boot Path Test Coverage Target

What is the right test coverage target for `boot.rs`? The module has 1,953 lines and 19 outgoing dependencies. Full coverage is expensive.

**Recommendation**: Target the 12 boot stages documented in `08-orchestrator.md` (ProcessTable through Mesh). One integration test per stage verifying the service is registered and healthy. That gives ~15 tests covering the critical path without testing internal wiring.

### Q5: Feature Composition in CI

Should feature composition testing (all 6 gate combinations) be part of the standard CI pipeline, or a separate pre-release gate?

**Options**:
- (a) CI: build-check all combos on every PR (adds ~3 minutes to CI)
- (b) Gate: `scripts/build.sh gate` includes all combos (pre-commit only)
- (c) Nightly: scheduled job tests all combos, PR CI tests only `native` + `os-patterns`

**Recommendation**: Option (c). Daily CI is fast; nightly catches composition regressions.

### Q6: 08a/08b/08c Implementation Scope

The 08 orchestrator specifies ~5,750 new lines across 117 exit criteria. Should the 09-weftos-gaps sprint attempt all 117, or prioritize a subset?

**Recommendation**: Sprint 09 should focus on the test coverage, decision resolution, and Weaver runtime (Sprints 1-3 above). The 08a/08b/08c implementation should be a separate sprint (10-weftos-os-patterns) once the testing foundation is solid.

---

## Appendix: Source Data References

| Data Source | Path | Nodes | Edges |
|-------------|------|:-----:|:-----:|
| Gap report | `.weftos/analysis/gap-report.json` | -- | -- |
| Decisions graph | `.weftos/graph/decisions.json` | 92 | ~200 |
| Module deps graph | `.weftos/graph/module-deps.json` | 214 | 473 |
| Git history graph | `.weftos/graph/git-history.json` | 102 | 965 |
| Weaver analysis | `docs/weftos/weaver-analysis-clawft.md` | -- | -- |
| Weaver TODO | `.weftos/weaver_todo.md` | -- | -- |
| 08 Orchestrator | `.planning/sparc/weftos/0.1/08-orchestrator.md` | -- | -- |
| 08a Self-Healing | `.planning/sparc/weftos/0.1/08a-self-healing.md` | -- | -- |
| 08b Reliable IPC | `.planning/sparc/weftos/0.1/08b-reliable-ipc.md` | -- | -- |
| 08c Content Ops | `.planning/sparc/weftos/0.1/08c-content-ops.md` | -- | -- |
