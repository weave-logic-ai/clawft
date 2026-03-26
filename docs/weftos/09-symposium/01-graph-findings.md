# 01 ECC Graph Findings -- Detailed Analysis

**Purpose**: Detailed graph analysis data formatted for symposium discussion
**Source**: `.weftos/graph/` and `.weftos/analysis/gap-report.json`
**Generated**: 2026-03-26

---

## 1. Gap Report: Top 20 Ranked Findings

The gap analysis scored and ranked findings by severity, impact, and downstream blocking potential.

### Critical Severity

| Rank | Area | Detail | Impact |
|------|------|--------|--------|
| 1 | Phase K8 not started | OS Gap Filling: 117 exit criteria, 0 checked | Blocks 8 deliverables: self-healing supervisor, DLQ, named pipes, metrics, log service, artifact store, WeaverEngine, config/secret/auth |

### High Severity

| Rank | Area | Detail | Impact |
|------|------|--------|--------|
| 2 | Phase K4 incomplete | Container Integration: 86.7% done, 2 remaining | Blocks phase completion |
| 3 | Phase K5 incomplete | Application Framework: 94.1% done, 1 remaining | Blocks phase completion |
| 4 | Blocked: k2:D11 | Enable post-quantum signing immediately | Blocks k2:C6 |
| 5 | Blocked: k2:C6 | Enable post-quantum dual signing (DualKey) | K2.1 cannot fully complete |
| 6 | High fan-out, low test: boot.rs | 19 outgoing uses, 0 incoming, only 2 tests | Single point of failure |

### Medium Severity -- Decision Debt

| Rank | Decision | Title | Blocks |
|------|----------|-------|--------|
| 7 | k2:D8 | ExoChain-stored immutable API contracts | k2:C3, module:chain |
| 8 | k2:D10 | WASM-compiled shell with container sandbox | k2:C5, module:wasm_runner |
| 9 | k2:D20 | Configurable N-dimensional effect algebra | k2:C9, module:governance |
| 10 | k3:D1 | Hierarchical ToolRegistry | k3:AC-1, module:wasm_runner |
| 11 | k3:D2 | Context-based gate actions | k3:AC-2, module:agent_loop |
| 12 | k3:D4 | Wasmtime integration in K4 | k3:AC-5, module:wasm_runner |
| 13 | k3:D12 | Multi-layer sandboxing | k3:AC-3, module:wasm_runner |
| 14 | k2:D12 | Chain-agnostic blockchain anchoring trait | k2:C7 |
| 15 | k3:D3 | Implement all 25 remaining tools | k3:AC-4 |
| 16 | k3:D7 | Tree metadata for version history | k3:AC-6 |
| 17 | k3:D9 | Central authority + CA chain for tool signing | k3:AC-7 |
| 18 | k3:D10 | Separate ServiceApi and BuiltinTool concepts | module:service |

### Medium Severity -- Test Coverage

| Rank | Module | Lines | Tests | Category |
|------|--------|:-----:|:-----:|----------|
| 19 | wasm_runner.rs | 4,423 | 0 | K3 |
| 20 | chain.rs | 2,814 | 0 | K5 |

---

## 2. Module Dependency Analysis

### Crate Size and Test Distribution

| Crate | Files | Lines | Tests | Tests/KLoC | Risk |
|-------|------:|------:|------:|-----------:|------|
| clawft-kernel | 65 | 46,632 | 1,060 | 22.7 | Core -- needs higher ratio |
| clawft-core | 64 | 35,929 | 857 | 23.9 | Adequate |
| clawft-services | 40 | 14,399 | 203 | 14.1 | LOW -- needs attention |
| clawft-cli | 33 | 12,250 | 364 | 29.7 | Good |
| clawft-channels | 46 | 10,292 | 252 | 24.5 | Adequate |
| clawft-types | 21 | 7,875 | 228 | 28.9 | Good |
| clawft-tools | 17 | 5,869 | 121 | 20.6 | Adequate |
| clawft-plugin | 27 | 5,824 | 134 | 23.0 | Adequate |
| clawft-wasm | 11 | 5,665 | 172 | 30.4 | Good |
| clawft-weave | 18 | 5,214 | 17 | 3.3 | CRITICAL -- 17 tests for 5K lines |
| clawft-llm | 12 | 4,741 | 139 | 29.3 | Good |
| exo-resource-tree | 9 | 1,795 | 61 | 34.0 | Excellent |
| clawft-platform | 10 | 1,707 | 32 | 18.7 | Adequate |
| clawft-security | 3 | 945 | 19 | 20.1 | Adequate |

### High-Risk Modules (>500 lines, <5 tests)

| Module | Crate | Lines | Tests | Feature Gate | Category |
|--------|-------|------:|------:|-------------|----------|
| wasm_runner.rs | kernel | 4,423 | 0 | -- | K3 |
| chain.rs | kernel | 2,814 | 0 | exochain | K5 |
| boot.rs | kernel | 1,953 | 2 | -- | K1 |
| daemon.rs | weave | 1,647 | 1 | -- | K1 |
| a2a.rs | kernel | 1,452 | 1 | -- | K1 |
| agent_loop.rs | kernel | 1,412 | 0 | -- | K1 |
| host.rs | channels | 561 | 0 | -- | K1 |
| agent_bus.rs | core | 462 | 1 | native | K1 |

**Combined risk**: 14,724 lines of untested or barely-tested code on the critical boot path.

### Feature Gate Island Analysis

Six feature gates create compositional boundaries:

| Gate | Modules | Lines | Tests | Dependencies |
|------|--------:|------:|------:|-------------|
| `native` | ~30 | ~25,000 | ~600 | tokio, dirs |
| `exochain` | ~5 | ~8,000 | ~100 | rvf-crypto, rvf-types, ed25519-dalek, ciborium |
| `ecc` | 9 | ~4,500 | 83 | blake3, clawft-core/vector-memory |
| `mesh` | 20 | ~3,543 | 133 | ed25519-dalek, rand |
| `os-patterns` | 8 | ~3,500 | ~80 | exochain |
| `cluster` | ~3 | ~2,000 | ~30 | ruvector-cluster, ruvector-raft |

**Composition risk**: No tests verify that `ecc + mesh + os-patterns` compile and work together. The `os-patterns` gate implies `exochain`, and `ecc` implies `blake3`, creating transitive dependency chains that have not been integration-tested as a group.

---

## 3. Decision Graph Analysis

### Decision Status by Symposium

| Symposium | Total | Implemented | Partial | Pending | Blocked | Deferred |
|-----------|:-----:|:-----------:|:-------:|:-------:|:-------:|:--------:|
| K2 | 22 | 12 | 1 | 7 | 1 | 0 |
| K3 | 14 | 0 | 0 | 11 | 0 | 2 |
| ECC | 8 | 7 | 0 | 1 | 0 | 0 |
| K5 | 15 | 15 | 0 | 0 | 0 | 0 |
| **Total** | **59** | **34** | **1** | **19** | **1** | **2** |

**Observation**: K5 symposium decisions are 100% implemented (all mesh/networking decisions were executed in K6). K3 symposium decisions are 0% implemented -- they target K4 work that has not started. K2 has 8 unresolved decisions creating persistent debt.

### Decision Dependency Chains

The following decisions have downstream blocking relationships:

```
k2:D8 (ExoChain API contracts) --blocks--> k2:C3 (chain-anchored contracts)
  |                                          |
  +--blocks--> module:chain (2,814 lines, 0 tests)

k2:D10 (WASM shell + sandbox) --blocks--> k2:C5 (shell pipeline)
  |
  +--blocks--> module:wasm_runner (4,423 lines, 0 tests)

k2:D11 (post-quantum signing) --blocked-by--> rvf-crypto API gap
  |
  +--blocks--> k2:C6 (DualKey activation)

k2:D20 (N-dimensional effect algebra) --blocks--> k2:C9 (configurable EffectVector)
  |
  +--blocks--> module:governance (910 lines, 38 tests)

k3:D1 (hierarchical ToolRegistry) --blocks--> k3:AC-1
k3:D2 (context gate actions)      --blocks--> k3:AC-2
k3:D4 (Wasmtime integration)      --blocks--> k3:AC-5
k3:D12 (multi-layer sandbox)      --blocks--> k3:AC-3
```

### Commitment Fulfillment Rate

| Symposium | Total Commitments | Implemented | Pending | Blocked | Rate |
|-----------|:-----------------:|:-----------:|:-------:|:-------:|-----:|
| K2 | 10 | 4 (C1, C8, C10 + partial C4) | 5 | 1 (C6) | 40% |
| K3 | 7 | 0 | 7 | 0 | 0% |
| K5 | 5 | 5 | 0 | 0 | 100% |
| **Total** | **22** | **9** | **12** | **1** | **41%** |

---

## 4. Git History Patterns

### Development Rhythm (102 commits, 27 days)

| Period | Dates | Commits | Focus |
|--------|-------|:-------:|-------|
| Foundation | Feb 28 - Mar 1 | ~15 | K0 kernel boot, daemon, cluster |
| Implementation | Mar 1 - Mar 3 | ~12 | K1-K2 + K2b hardening |
| Symposium cycle | Mar 3 - Mar 4 | ~8 | K2 + K3 symposiums, K2.1 implementation |
| Quiet period | Mar 5 - Mar 21 | ~5 | Planning, documentation |
| ECC symposium | Mar 22 | ~3 | ECC research and decisions |
| Implementation burst | Mar 25 - Mar 26 | ~17 | K3-K6, governance, OS gap-filling, standalone crate |
| Weaver + analysis | Mar 26 | ~8 | Graph ingestion, gap analysis, this symposium |

**Key insight**: The development rhythm shows a clear pattern of 3-5 day planning/documentation phases followed by 24-48 hour implementation bursts. The Mar 25-26 burst produced 17 commits in 30 hours, delivering code across 4 K-levels.

### Commit Frequency Distribution

| Day Type | Avg Commits | Observation |
|----------|:-----------:|-------------|
| Planning days | 1-2 | `docs:` prefix, symposium documents |
| Implementation days | 8-12 | `feat:`, `test:` prefix, code files |
| Consolidation days | 3-4 | `chore:`, `fix:`, `refactor:` prefix |

---

## 5. Orphan Module Inventory

116 modules have no incoming or outgoing causal edges in the Weaver's graph. These represent "unwired" knowledge -- the Weaver knows they exist but has no model of how they relate to the rest of the system.

### Top 20 Orphan Modules by Size

| Module | Crate | Lines | Tests | Feature Gate |
|--------|-------|------:|------:|-------------|
| boot.rs | kernel | 1,953 | 2 | -- |
| tree_manager.rs | kernel | 1,942 | 31 | exochain |
| app.rs | kernel | 1,847 | 42 | -- |
| engine.rs | wasm | 1,777 | 35 | wasm-plugins |
| daemon.rs | weave | 1,647 | 1 | -- |
| routing_validation.rs | core | 1,638 | 45 | -- |
| a2a.rs | kernel | 1,452 | 1 | -- |
| agent_loop.rs | kernel | 1,412 | 0 | -- |
| weaver.rs | kernel | 1,369 | 27 | ecc |
| rvf_tools.rs | services | 1,333 | 9 | rvf |
| canvas.rs | types | 983 | 41 | -- |
| file_tools.rs | tools | 961 | 0 | -- |
| routing.rs | types | 948 | 21 | -- |
| process.rs | kernel | 1,118 | 57 | -- |
| supervisor.rs | kernel | 760 | 5 | -- |
| governance.rs | kernel | 910 | 38 | -- |
| mesh_process.rs | kernel | 800 | 20 | mesh |
| mesh_kad.rs | kernel | 714 | 23 | mesh |
| mesh_noise.rs | kernel | 693 | 19 | mesh |
| metrics.rs | kernel | 630 | 19 | os-patterns |

**Reduction strategy**: The top 20 orphans account for ~23,000 lines. Wiring these with `Uses`, `EvidenceFor`, and `GatedBy` edges would reduce orphan count by ~17% and significantly improve the Weaver's structural model.

---

## 6. Weaver Confidence Breakdown

### Post-Ingestion Confidence (0.78)

| Area | Confidence | Evidence |
|------|-----------|----------|
| Codebase structure | 0.95 | Complete file inventory, verified line counts |
| Module relationships | 0.90 | `lib.rs` re-exports fully mapped |
| Symposium decision chain | 0.90 | All 4 symposiums ingested, cross-referenced |
| Feature gate topology | 0.90 | `Cargo.toml` verified against `lib.rs` |
| Test coverage distribution | 0.85 | `#[test]` count verified per crate |
| Development timeline | 0.85 | Git log with exact timestamps (102 nodes) |
| Concurrency patterns | 0.85 | Consistent DashMap/Arc/Mutex usage |
| Recurrent patterns | 0.80 | 8 patterns with multiple instances |

### Confidence Gaps (areas needing data)

| Area | Confidence | What Would Improve It |
|------|-----------|----------------------|
| Cross-crate dependency graph | 0.70 | Full Cargo.toml parsing for all 22 crates |
| Runtime behavior | 0.60 | Execution traces, test coverage reports |
| Integration test coverage | 0.55 | Map tests to specific behaviors |
| CI pipeline shape | 0.50 | Read CI config files |
| Dead code / unused modules | 0.50 | Compiler dead-code analysis |
| Performance characteristics | 0.40 | Benchmark data from calibration module |

### Zero-Confidence Areas

| Area | What's Missing |
|------|---------------|
| Runtime embeddings | No real EmbeddingProvider (only mock) |
| Live causal graph state | No persisted graph -- only schema |
| Cross-node behavior | Mesh Phase 2 (wire I/O) not implemented |
| Real HNSW performance | No production vectors loaded |
| User interaction patterns | No telemetry or usage data |

### Confidence Trajectory

```
Initial structural analysis:     0.62
After git history ingestion:     +0.08 = 0.70
After module dependency ingestion: +0.06 = 0.76
After decision/phase ingestion:    +0.02 = 0.78
Target after ONNX embeddings:     +0.05 = 0.83
Target after CognitiveTick:       +0.04 = 0.87
Target after runtime traces:      +0.05 = 0.92
```

---

## 7. Estimated Causal Graph Scale

From the Weaver analysis, the full causal graph when complete would contain:

| Metric | Current (ingested) | Estimated Full |
|--------|-------------------:|---------------:|
| Nodes | ~408 | ~1,052 |
| Edges | ~1,638 | ~2,540 |
| Average degree | ~4.0 | ~4.8 |
| Max depth | 3 | 4 |
| Max fan-out | 19 (boot.rs) | 20 (K6 -> mesh_*) |
| Connected components | Multiple (orphans) | 1 (target) |

### Embedding Memory Budget

| Tier | Vectors | Dimensions | Memory (f32) |
|------|--------:|----------:|-------------:|
| Code chunks | 2,000 | 384 | 3.1 MB |
| Documentation | 3,000 | 384 | 4.6 MB |
| Commits | 101 | 128 | 0.05 MB |
| Type signatures | 400 | 256 | 0.4 MB |
| Causal nodes | 1,052 | 64 | 0.3 MB |
| **Total** | **6,553** | -- | **~8.4 MB** |

This is well within the capability of the deployment target (ARM SoC demonstrated at 3.6ms/tick on Cortex-A53 in Mentra research).

---

## 8. Action Items for Symposium

Based on the graph findings, these are the concrete actions to discuss:

1. **Boot path hardening** -- boot.rs has 1,953 lines, 19 outgoing deps, 2 tests. This is the kernel's single point of failure.

2. **Zero-test module triage** -- agent_loop.rs (1,412 lines), chain.rs (2,814 lines), wasm_runner.rs (4,423 lines) have zero tests. Combined 8,649 lines of untested critical code.

3. **K3 decision batch** -- 14 decisions at 0% implementation. Which do we implement vs defer?

4. **Commitment debt** -- 12 pending commitments (5 from K2, 7 from K3). The K3 commitments target K4 work. Should K4 be formally scoped?

5. **Feature composition testing** -- 6 feature gates, no cross-gate integration tests. The `os-patterns + ecc + mesh` combination is untested.

6. **Weaver ONNX activation** -- The mock embedding provider limits the Weaver to structural analysis only. ONNX backend unlocks semantic search.

7. **clawft-weave test coverage** -- 5,214 lines with only 17 tests (3.3 tests/KLoC). This crate contains the daemon that operators interact with.
