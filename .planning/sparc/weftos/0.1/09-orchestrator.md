# 09 WeftOS Gaps Sprint -- Orchestrator

**Document ID**: 09
**Workstream**: W-KERNEL
**Duration**: 4 sprints (estimated 4 weeks, partial parallelization possible)
**Goal**: Address all gaps identified by ECC graph analysis -- test coverage inversion, decision debt, Weaver runtime activation, and integration polish
**Depends on**: K0-K6 complete, 08 orchestrator defined (not yet implemented)
**Source**: ECC graph analysis (2026-03-26), gap-report.json, weaver_todo.md, symposium overview
**Branch**: `feature/weftos-kernel-sprint`

---

## Overview

Four sprints addressing the gaps identified by the Weaver's ECC graph analysis.
The graph revealed a test coverage inversion (most-depended-on modules have the
fewest tests), 23 pending decisions creating decision debt, a non-operational
Weaver runtime, and untested feature gate compositions. This sprint series
resolves all four categories before the 08a/08b/08c implementation work begins.

**Rationale**: Sprint 09 is a *foundation hardening* effort. The 08 orchestrator
defines ~5,750 new lines across 117 exit criteria, but attempting that
implementation without first covering the critical path with tests and resolving
decision debt would compound risk. Sprint 09 builds the safety net that Sprint
08 (now renumbered as Sprint 10) will land on.

**Current kernel test count**: 1,197 passed (baseline for this sprint)
**Current Weaver confidence**: 0.78 (structural)
**Pending decisions**: 23 (19 pending + 2 deferred + 1 blocked + 1 partial)
**Untested critical-path lines**: 14,724

---

## Sprint Map

| Sprint | Focus | Scope | Est. New Tests | Est. Lines Changed | Duration | Depends On |
|--------|-------|-------|:--------------:|:------------------:|----------|------------|
| 09a | Test Coverage & Stability | 8 high-risk modules, feature composition, orphan wiring | 120+ new tests | ~2,500 test code | 5 days | -- |
| 09b | Decision Resolution | 23 pending decisions, 14 commitments, 1 blocker | 10-15 decision tests | ~800 code + docs | 5 days | -- (parallel with 09a) |
| 09c | Weaver Runtime | EmbeddingProvider, CognitiveTick, confidence scoring, export | 25+ new tests | ~1,200 new code | 5 days | 09a (test infra) |
| 09d | Integration & Polish | Feature matrix CI, doc audit, gate script, confidence target | 30+ composition tests | ~600 code + scripts | 5 days | 09a, 09b, 09c |

**Estimated totals**: ~185+ new tests, ~5,100 lines of test code and implementation.

---

## Dependency Graph

```
09a (Test Coverage) ────────────────────────────┐
  |                                              |
  +-- Module: boot.rs (15+ tests)                |
  +-- Module: agent_loop.rs (20+ tests)          |
  +-- Module: chain.rs (15+ tests)               |
  +-- Module: wasm_runner.rs (20+ tests)         |
  +-- Module: a2a.rs (10+ tests)                 |
  +-- Module: daemon.rs (10+ tests)              |
  +-- Module: host.rs (8+ tests)                 |
  +-- Module: agent_bus.rs (5+ tests)            |
  +-- Feature composition tests                  |
  +-- Orphan wiring (top 20)                     |
                                                 |
09b (Decision Resolution) ──────────────────────-+  (parallel with 09a)
  |                                              |
  +-- K3 decisions: D1, D2, D4, D12 (implement) |
  +-- K3 decisions: D5, D11, D13, D14 (defer)   |
  +-- K2 decisions: D8, D10, D12, D20 (resolve) |
  +-- K2 blocked: D11 (unblock or defer)         |
  +-- ECC: D5 (resolve)                          |
  +-- Commitment updates                         |
                                                 |
09c (Weaver Runtime) ──────────────────────────--+  (after 09a test infra)
  |                                              |
  +-- ONNX embedding backend                     |
  +-- CognitiveTick integration (4 items)        |
  +-- Confidence scoring (2 items)               |
  +-- Export CLI (1 item)                        |
  +-- Meta-Loom persistence (1 item)             |
                                                 |
09d (Integration & Polish) ─────────────────────-+  (after 09a, 09b, 09c)
  |
  +-- Feature gate composition CI matrix
  +-- Documentation audit (5 unchecked items)
  +-- Fumadocs site updates
  +-- scripts/09-gate.sh
  +-- Weaver confidence target 0.92
  +-- Incomplete phase cleanup (K3, K4, K5)
```

---

## Parallelization Strategy

- **09a and 09b can run fully in parallel.** They share no code paths: 09a writes
  tests for existing modules; 09b resolves decisions and updates documentation.
  The only shared artifact is the codebase, and they touch different files.

- **09c depends on 09a** for the test infrastructure patterns. The Weaver tests
  need the `TestKernel` boot helper and mock provider patterns established in
  09a. 09c can start 1-2 days into 09a once the first batch of test helpers
  is merged.

- **09c and 09b can partially overlap.** 09b's K3 decision implementations
  (D1 hierarchical ToolRegistry, D4 Wasmtime) may affect `wasm_runner.rs`, which
  09c's Weaver does not touch. No conflict.

- **09d depends on all three predecessors.** It validates the composite output
  and builds the gate script that checks everything.

**Optimal timeline with parallelization**: ~3 weeks (vs 4 sequential).

```
Week 1: 09a starts ─────── 09b starts (parallel)
Week 2: 09a finishes ───── 09b finishes ──── 09c starts
Week 3: 09c finishes ───── 09d starts and finishes
```

---

## Agent Assignments

| Agent | Sprint(s) | Work Packages | Rationale |
|-------|-----------|---------------|-----------|
| **test-sentinel** | 09a, 09d | All test implementation (09a), feature composition matrix (09d) | Primary testing expertise, gate verification |
| **kernel-architect** | 09a, 09b, 09c, 09d | Test plan review (09a), decision architectural review (09b), SystemService review (09c), final review (09d) | Cross-cutting architectural authority |
| **process-supervisor** | 09a | boot.rs, supervisor.rs, agent_loop.rs test scenarios | Owns process lifecycle domain |
| **chain-guardian** | 09a, 09b | chain.rs tests (09a), D11 post-quantum unblocking (09b) | Owns ExoChain and cryptographic signing |
| **sandbox-warden** | 09a, 09b | wasm_runner.rs tests (09a), K3 WASM decisions D1/D4/D12 (09b) | Owns WASM sandbox domain |
| **governance-counsel** | 09b | Decision triage and resolution, commitment tracking updates | Owns governance and decision process |
| **weaver** | 09c | ONNX backend, CognitiveTick, confidence scoring, export, Meta-Loom | Owns Weaver implementation |
| **ecc-analyst** | 09c, 09d | CognitiveTick budget validation (09c), confidence improvement (09d) | Owns ECC analysis and spectral health |
| **mesh-engineer** | 09a, 09d | host.rs tests (09a), feature composition risk analysis (09d) | Owns mesh and cross-feature interactions |
| **doc-weaver** | 09d | Documentation audit, Fumadocs updates, decision doc finalization | Owns documentation deliverables |
| **app-deployer** | 09a | agent_bus.rs tests, AppManager integration test scenarios | Owns app lifecycle and service wiring |

---

## Master Checklist

### Sprint 09a: Test Coverage & Stability

- [ ] `boot.rs` has 15+ tests covering all 12 boot stages *(currently 4 -- needs 11 more)*
- [x] `agent_loop.rs` has 20+ tests covering agent execution lifecycle *(26 tests, verified 2026-03-26)*
- [x] `chain.rs` has 15+ tests covering append, verify, segment, merkle *(60 tests, verified 2026-03-26)*
- [x] `wasm_runner.rs` has 20+ tests covering fuel, memory, capability enforcement *(93 tests, verified 2026-03-26)*
- [ ] `a2a.rs` has 10+ tests covering routing, topic dispatch, error paths *(currently 1 -- needs 9 more)*
- [ ] `daemon.rs` (clawft-weave) has 10+ tests covering socket IPC, lifecycle *(currently 1 -- needs 9 more)*
- [x] `host.rs` (clawft-channels) has 8+ tests covering channel host operations *(11 tests, verified 2026-03-26)*
- [x] `agent_bus.rs` (clawft-core) has 5+ tests covering bus send/recv *(9 tests, verified 2026-03-26)*
- [x] Feature gate composition: `native + ecc + mesh` builds and tests clean *(verified 2026-03-26)*
- [x] Feature gate composition: `native + os-patterns` builds and tests clean *(verified 2026-03-26)*
- [x] Feature gate composition: `native + exochain + ecc` builds and tests clean *(verified 2026-03-26)*
- [ ] Orphan module count reduced by 20+ (from 116 to <96)
- [x] All existing 1,197 tests still pass (no regressions) *(1,220 passing, verified 2026-03-26)*
- [ ] Total kernel test count reaches 1,320+ (120+ new) *(currently 1,220 -- needs ~100 more)*
- [ ] `scripts/build.sh clippy` clean for all modified files

### Sprint 09b: Decision Resolution

- [ ] k3:D1 (Hierarchical ToolRegistry) resolved with implementation plan
- [ ] k3:D2 (Context-based gate actions) resolved with implementation plan
- [ ] k3:D4 (Wasmtime integration) resolved with implementation plan
- [ ] k3:D12 (Multi-layer sandboxing) resolved with implementation plan
- [ ] k3:D3 (25 remaining tools) resolved: implement or defer with rationale
- [ ] k3:D5 (Disk-persisted module cache) formally deferred with rationale
- [ ] k3:D6 (Configurable WASI) resolved
- [ ] k3:D7 (Tree metadata) resolved
- [ ] k3:D8 (Informational revocation) resolved
- [ ] k3:D9 (CA chain for tool signing) resolved
- [ ] k3:D10 (Separate ServiceApi/BuiltinTool) resolved
- [ ] k3:D11 (Routing-time gate deferred) confirmed deferred
- [ ] k3:D13 (WASM snapshots) confirmed deferred
- [ ] k3:D14 (tiny-dancer scoring) resolved
- [ ] k2:D8 (ExoChain API contracts) resolved
- [ ] k2:D10 (WASM shell + sandbox) resolved
- [ ] k2:D11 (Post-quantum signing) unblocked or formally deferred
- [ ] k2:D12 (Chain-agnostic anchoring) resolved
- [ ] k2:D13 (Zero-knowledge proofs) resolved
- [ ] k2:D17 (Layered routing) resolved
- [ ] k2:D18 (SONA training pipeline) resolved
- [ ] k2:D20 (N-dimensional effect algebra) resolved
- [ ] ecc:D5 (DEMOCRITUS continuous operation) resolved
- [ ] Pending decisions reduced from 23 to <5
- [ ] Blocked decisions reduced from 1 to 0
- [ ] Commitment tracking document updated
- [ ] All decision resolutions recorded in symposium log

### Sprint 09c: Weaver Runtime

- [ ] ONNX embedding backend operational behind `onnx-embeddings` feature
- [ ] LLM API embedding backend operational as fallback
- [ ] Weaver registered with CognitiveTick as tick consumer
- [ ] Budget-aware tick processing respects `tick_budget_ratio`
- [ ] Incremental git polling detects new commits since last tick
- [ ] File watcher integration detects source file changes
- [ ] Confidence scoring based on edge coverage implemented
- [ ] Gap detection for modules with no causal edges implemented
- [ ] `weaver ecc export` CLI command produces valid weave-model.json
- [ ] Meta-Loom persistence tracks strategy effectiveness
- [ ] Weaver confidence reaches 0.85+ (from 0.78)
- [ ] 25+ new Weaver tests pass
- [ ] `scripts/build.sh test` passes with `ecc` feature enabled

### Sprint 09d: Integration & Polish

- [x] Feature gate CI matrix: `native` builds and all tests pass *(verified 2026-03-26)*
- [x] Feature gate CI matrix: `native + exochain` builds and all tests pass *(verified 2026-03-26)*
- [x] Feature gate CI matrix: `native + ecc` builds and all tests pass *(verified 2026-03-26)*
- [x] Feature gate CI matrix: `native + mesh` builds and all tests pass *(verified 2026-03-26)*
- [x] Feature gate CI matrix: `native + os-patterns` builds and all tests pass *(verified 2026-03-26)*
- [x] Feature gate CI matrix: `native + ecc + mesh + os-patterns` builds and all tests pass *(verified 2026-03-26)*
- [x] `scripts/09-gate.sh` exists and passes all checks *(created 2026-03-26, 20/20 gates pass)*
- [ ] K3 remaining exit criterion completed (1 item at 86.7%)
- [ ] K4 remaining exit criteria completed (2 items at 86.7%)
- [ ] K5 remaining exit criterion completed (1 item at 94.1%)
- [ ] Fumadocs documentation updated for 08a scope
- [ ] Fumadocs documentation updated for 08b scope
- [ ] Fumadocs documentation updated for 08c scope
- [ ] `scripts/build.sh gate` passes with `os-patterns` feature
- [ ] `scripts/08-gate.sh` structure defined (even if 08 not yet implemented)
- [ ] Weaver confidence reaches 0.92 target
- [ ] Total workspace test count reaches 3,950+ (185+ new across all sprints)
- [x] No regressions: all pre-existing tests pass *(1,220 kernel tests pass, verified 2026-03-26)*
- [ ] All clippy warnings resolved

---

## Gate Script

`scripts/09-gate.sh` should verify:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== 09 WeftOS Gaps Sprint Gate ==="

# 1. Base build check
echo "[1/11] Base build (no features)..."
scripts/build.sh check

# 2. Native build
echo "[2/11] Native build..."
scripts/build.sh native-debug

# 3. Feature matrix builds
echo "[3/11] Feature matrix: ecc..."
cargo check -p clawft-kernel --features ecc 2>&1 | tail -1
echo "[4/11] Feature matrix: mesh..."
cargo check -p clawft-kernel --features mesh 2>&1 | tail -1
echo "[5/11] Feature matrix: os-patterns..."
cargo check -p clawft-kernel --features os-patterns 2>&1 | tail -1
echo "[6/11] Feature matrix: ecc+mesh+os-patterns..."
cargo check -p clawft-kernel --features ecc,mesh,os-patterns 2>&1 | tail -1

# 4. All tests pass
echo "[7/11] Workspace tests..."
scripts/build.sh test

# 5. Clippy clean
echo "[8/11] Clippy..."
scripts/build.sh clippy

# 6. Test count verification
echo "[9/11] Test count check..."
TEST_COUNT=$(cargo test --workspace 2>&1 | grep -oP '\d+ passed' | grep -oP '\d+' | head -1)
if [ "$TEST_COUNT" -lt 1320 ]; then
    echo "FAIL: Test count $TEST_COUNT < 1320 minimum"
    exit 1
fi
echo "  Test count: $TEST_COUNT (minimum: 1320)"

# 7. Critical path zero-test check
echo "[10/11] Critical path test coverage..."
for module in boot agent_loop chain wasm_runner a2a; do
    COUNT=$(grep -c '#\[test\]' "crates/clawft-kernel/src/${module}.rs" 2>/dev/null || echo 0)
    if [ "$COUNT" -lt 5 ]; then
        echo "FAIL: ${module}.rs has only $COUNT tests (minimum: 5)"
        exit 1
    fi
    echo "  ${module}.rs: $COUNT tests"
done

# 8. Decision debt check
echo "[11/11] Decision debt..."
echo "  (Manual check: verify pending decisions < 5 in symposium log)"

echo ""
echo "=== 09 Gate: ALL CHECKS PASSED ==="
```

---

## Expert Review Summary

### test-sentinel

**Concern**: "The proposed test counts (15+ for boot.rs, 20+ for agent_loop.rs)
are ambitious but achievable if we focus on behavior-level tests rather than
line-level coverage. boot.rs has 1,953 lines but only 12 distinct boot stages --
testing each stage's registration and health check is 12 tests, plus 3 error
paths gives us 15. For wasm_runner.rs at 4,423 lines, the priority should be
fuel exhaustion, memory limit, capability denial, and module loading -- not
internal parsing details. 20 behavioral tests will cover more risk than 50
unit tests on private helpers."

**Resolution**: Adopted. Test plans target behavioral coverage per boot stage
and per security boundary, not line coverage.

**Concern**: "The 1,320 total kernel test target is conservative. With 120 new
tests against the 1,197 baseline, that is only a 10% increase. I recommend
tracking tests-per-KLoC for the 8 high-risk modules separately."

**Resolution**: Added per-module test count exit criteria rather than just a
global count.

### kernel-architect

**Concern**: "Sprints 09a and 09b running in parallel is fine, but 09c should
not start until 09a's `TestKernel` boot helper is merged. The Weaver tests
depend on being able to boot a kernel with `ecc` feature in test context."

**Resolution**: 09c formally depends on 09a. Parallelization window narrowed to
09a+09b concurrent, then 09c, then 09d.

**Concern**: "The orphan wiring target (reduce by 50%) is too aggressive for
Sprint 09. Wiring requires understanding each module's causal role, which is
modeling work, not just graph insertion. Recommend reducing to top 20 orphans
by size."

**Resolution**: Changed from '50% reduction' to '20+ orphans wired (top 20 by
size)', which maps to the concrete list in 01-graph-findings.md section 5.

### process-supervisor

**Concern**: "Testing restart strategies without real crashes is possible using
`CancellationToken` and `JoinHandle::abort()`. The test pattern should spawn a
mock agent that panics on command, then verify the supervisor's response. This
does NOT require actual process crashes -- the `tokio::task::JoinError` from a
panicked task is sufficient to trigger the restart path."

**Resolution**: Documented in 09a work packages. Test pattern uses controlled
panics via `JoinHandle`, not real crashes.

### chain-guardian

**Concern**: "The D11 post-quantum blocker is real. rvf-crypto exposes
`DualKey::sign()` but the kernel's `chain.rs` only calls `Ed25519Signer`. The
fix is ~50 lines: add a `DualSigner` trait wrapper that calls both signers and
concatenates the signatures. But this requires the `rvf-crypto` crate to be in
the workspace dependency graph, which it already is under `[dependencies]` in
clawft-kernel/Cargo.toml behind the `exochain` feature."

**Resolution**: D11 resolution path documented in 09b. Estimated at 50-100
lines. If rvf-crypto API proves insufficient, fallback is to defer k2:C6 with
documented rationale.

**Concern**: "chain.rs tests (15+) should include at least 3 tests for hash
chain integrity (append-then-verify, tampered entry detection, segment boundary
verification) and 3 tests for RVF serialization roundtrip."

**Resolution**: Added to 09a work packages as mandatory test categories.

### weaver

**Concern**: "ONNX may be overkill for Sprint 09. The all-MiniLM-L6-v2 model
is 22MB and requires the `ort` crate (Rust ONNX Runtime bindings), which brings
in a C++ dependency. An alternative is to use the existing `clawft-llm` provider
trait to call an external embedding API (e.g., the local LLM server that is
already running on this box). This gives real embeddings without the ONNX
dependency, and we can add ONNX in a later sprint."

**Resolution**: Sprint 09c implements BOTH backends: LLM API backend as the
primary (uses existing infrastructure), ONNX as secondary behind
`onnx-embeddings` feature gate. LLM API is the P0 deliverable; ONNX is P1.

### governance-counsel

**Concern**: "14 K3 decisions at once is too many to resolve properly. Each
decision needs: (1) re-read original rationale, (2) check if circumstances
changed, (3) decide implement/defer/supersede, (4) document rationale. At 30
minutes per decision, that is 7 hours of focused decision work. Recommend
batching: resolve the 4 HIGH priority decisions (D1, D2, D4, D12) first, then
triage the rest as implement-minimal or defer-with-rationale in a single sweep."

**Resolution**: Adopted. 09b structures decisions in two tiers: HIGH priority
(4 decisions, full resolution with implementation plans) and remainder (10
decisions, batch triage with rationale). K2 decisions follow the same pattern.

### mesh-engineer

**Concern**: "The riskiest feature composition is `mesh + os-patterns`. The mesh
feature brings in `ed25519-dalek` and `rand`; os-patterns brings in `exochain`
which also uses `ed25519-dalek`. If they specify different versions, the build
breaks. This should be tested early in 09a, not deferred to 09d."

**Resolution**: Added to 09a as an early validation step: verify
`mesh + os-patterns` compiles before writing tests.

### doc-weaver

**Concern**: "The 5 unchecked documentation items from the 08 orchestrator are
for content that does not yet exist (08a/08b/08c have not been implemented).
We should create documentation *stubs* with architecture overviews and TBD
sections for implementation details, rather than leaving them completely
unchecked until Sprint 10."

**Resolution**: 09d creates documentation stubs for 08a/08b/08c with
architecture diagrams from the existing plans, marked as 'Planned -- not yet
implemented'.

---

## Consensus Record

All twelve WeftOS agents reviewed the 09 sprint plan:

1. **kernel-architect**: Approved with dependency ordering adjustment (09c depends on 09a).
2. **process-supervisor**: Approved with test pattern clarification (controlled panics, not real crashes).
3. **mesh-engineer**: Approved with early composition validation requirement.
4. **chain-guardian**: Approved with D11 resolution path and chain.rs test category requirements.
5. **governance-counsel**: Approved with batched decision resolution structure.
6. **sandbox-warden**: Approved. Noted that wasm_runner.rs tests should cover all three sandbox layers.
7. **test-sentinel**: Approved with behavioral coverage approach and per-module tracking.
8. **weaver**: Approved with LLM API backend as primary, ONNX as secondary.
9. **ecc-analyst**: Approved. Requested CognitiveTick budget allocation of 15% for Weaver (below 20% default).
10. **doc-weaver**: Approved with documentation stub approach for 08a/08b/08c.
11. **app-deployer**: Approved. No blocking concerns for Sprint 09.
12. **defi-networker**: Approved. No Sprint 09 work assigned; DeFi layer is post-1.0.

**Consensus reached**: 2026-03-26. All agents agree that Sprint 09 is a necessary
foundation hardening before 08a/08b/08c implementation begins.

---

## Cross-References

- **09a**: `09a-test-coverage.md` -- Test Coverage & Stability
- **09b**: `09b-decision-resolution.md` -- Decision Resolution
- **09c**: `09c-weaver-runtime.md` -- Weaver Runtime
- **09d**: `09d-integration-polish.md` -- Integration & Polish
- **Symposium**: `docs/weftos/09-symposium/00-symposium-overview.md`
- **Graph findings**: `docs/weftos/09-symposium/01-graph-findings.md`
- **Gap report**: `.weftos/analysis/gap-report.json`
- **Weaver TODO**: `.weftos/weaver_todo.md`
- **08 orchestrator**: `.planning/sparc/weftos/0.1/08-orchestrator.md`
