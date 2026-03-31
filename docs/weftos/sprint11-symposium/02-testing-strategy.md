# Track 2: Testing Symposium

**Chair**: tdd-london-swarm
**Panelists**: tdd-london-swarm, production-validator, tester, security-auditor, performance-benchmarker, test-sentinel
**Date**: 2026-03-27
**Branch**: `feature/weftos-kernel-sprint`
**Codebase**: 22 crates, 181,703 lines Rust, 5,040 test annotations

---

## 1. Test Coverage Map

### 1.1 Per-Crate Test Census

Tests were counted by grepping for `#[test]` and `#[tokio::test]` annotations across all source files. The "density" column is tests per 1,000 source lines.

| # | Crate | Source Lines | Sync | Async | Total | Density | Assessment |
|---|-------|-------------|------|-------|-------|---------|------------|
| 1 | **clawft-kernel** | 60,618 | 1,364 | 278 | **1,642** | 27.1 | Well-tested (heaviest crate, proportional) |
| 2 | **clawft-core** | 35,270 | 857 | 248 | **1,105** | 31.3 | Well-tested |
| 3 | **clawft-cli** | 11,715 | 364 | 19 | **383** | 32.7 | Well-tested |
| 4 | **clawft-services** | 14,399 | 203 | 132 | **335** | 23.3 | Adequate |
| 5 | **clawft-channels** | 10,292 | 252 | 79 | **331** | 32.2 | Well-tested |
| 6 | **clawft-tools** | 5,218 | 121 | 107 | **228** | 43.7 | Well-tested (highest density) |
| 7 | **clawft-types** | 7,875 | 228 | 0 | **228** | 28.9 | Well-tested |
| 8 | **clawft-wasm** | 5,665 | 172 | 0 | **172** | 30.4 | Well-tested |
| 9 | **clawft-llm** | 4,148 | 139 | 30 | **169** | 40.7 | Well-tested |
| 10 | **clawft-plugin** | 5,824 | 134 | 32 | **166** | 28.5 | Adequate |
| 11 | **exo-resource-tree** | 1,809 | 61 | 0 | **61** | 33.7 | Well-tested |
| 12 | **clawft-platform** | 1,707 | 32 | 14 | **46** | 26.9 | Adequate |
| 13 | **clawft-weave** | 5,214 | 17 | 6 | **23** | 4.4 | **NEEDS WORK** -- daemon, codec, RPC under-tested |
| 14 | **clawft-plugin-containers** | 1,241 | 18 | 4 | **22** | 17.7 | Adequate (plugin scope is narrow) |
| 15 | **clawft-plugin-treesitter** | 1,135 | 14 | 6 | **20** | 17.6 | Adequate |
| 16 | **clawft-plugin-browser** | 827 | 13 | 7 | **20** | 24.2 | Adequate |
| 17 | **clawft-security** | 945 | 19 | 0 | **19** | 20.1 | Adequate |
| 18 | **clawft-plugin-cargo** | 679 | 17 | 2 | **19** | 28.0 | Adequate |
| 19 | **clawft-plugin-oauth2** | 1,317 | 19 | 0 | **19** | 14.4 | Needs work -- OAuth flows need async tests |
| 20 | **clawft-plugin-git** | 1,212 | 14 | 1 | **15** | 12.4 | **NEEDS WORK** -- integration tests sparse |
| 21 | **clawft-plugin-calendar** | 803 | 8 | 5 | **13** | 16.2 | Adequate for scope |
| 22 | **weftos** | 492 | 3 | 1 | **4** | 8.1 | **NEEDS WORK** -- facade crate, nearly untested |

**Totals**: 4,069 sync + 971 async = **5,040** test annotations across 181,703 source lines (27.7 tests per KLOC).

### 1.2 Intra-Kernel Module Breakdown

The kernel crate (1,642 tests) deserves deeper analysis since it contains most of the critical system code.

| Module | File(s) | Tests | Lines | Assessment |
|--------|---------|-------|-------|------------|
| Weaver / KnowledgeBase | weaver.rs | 123 | ~4,500 | Well-tested |
| WASM Runner / Tools | wasm_runner.rs | 111 | ~3,500 | Well-tested |
| Supervisor / Monitor / Process | supervisor.rs, monitor.rs, process.rs | 105 | ~2,800 | Well-tested |
| Governance Engine | governance.rs | 67 | ~2,000 | Well-tested |
| Boot sequence | boot.rs | 64 | ~3,200 | Adequate (high cfg density limits coverage) |
| ExoChain | chain.rs | 60 | ~3,000 | Adequate |
| A2A Messaging | a2a.rs | 56 | ~2,000 | Adequate |
| Capabilities | capability.rs | 50 | ~1,500 | Well-tested |
| ECC: CausalGraph | causal.rs | 46 | ~1,800 | Adequate |
| Service Registry | service.rs | 45 | ~1,200 | Well-tested |
| Embedding (ONNX) | embedding_onnx.rs | 43 | ~1,200 | Well-tested |
| Container Manager | container.rs | 42 | ~1,400 | Adequate |
| App Manager | app.rs | 42 | ~1,200 | Adequate |
| IPC / Named Pipes | ipc.rs, named_pipe.rs | 48 | ~1,800 | Adequate |
| Tree Manager | tree_manager.rs | 31 | ~1,000 | Needs work (14 chain-append sites, only 31 tests) |
| Mesh (all modules) | mesh_*.rs | 274 | ~6,000 | Well-tested (highest absolute count in kernel) |
| ECC: Cognitive Tick | cognitive_tick.rs | 20 | ~400 | Adequate |
| Metrics | metrics.rs | 19 | ~600 | Adequate |
| Embedding | embedding.rs | 19 | ~500 | Adequate |
| Dead Letter Queue | dead_letter.rs | 13 | ~400 | Needs work -- core reliability feature |
| HNSW Service | hnsw_service.rs | 13 | ~500 | Needs work -- search correctness critical |
| Gate (Capability/TileZero) | gate.rs | 12 | 809 | **NEEDS WORK** -- security-critical, under-tested |
| DEMOCRITUS Loop | democritus.rs | 12 | 774 | **NEEDS WORK** -- core cognitive loop |
| Calibration | calibration.rs | 10 | ~300 | Adequate for scope |
| Topic Router | topic.rs | 11 | ~400 | Needs work |
| **Persistence** | persistence.rs | **3** | ~170 | **CRITICAL GAP** -- data durability has 3 tests |

### 1.3 Key Findings

1. **No crate has zero tests.** The lowest is `weftos` (4 tests) which is a thin facade over `clawft-kernel`.
2. **clawft-weave** (the daemon crate, 5,214 lines, 23 tests) is significantly under-tested for its role as the runtime entry point. The daemon, client, RPC codec, and protocol layers have minimal coverage.
3. **Persistence layer has only 3 tests** for a subsystem responsible for data durability across restarts. This is the single most dangerous gap.
4. **Gate security checks have 12 tests for 809 lines** of security-critical code (capability checking, TileZero receipts).
5. **DEMOCRITUS loop has 12 tests for 774 lines** -- the continuous cognitive loop that drives the ECC substrate.
6. **No property-based tests, no snapshot tests, no fuzz harnesses, no benchmarks** exist anywhere in the codebase.

---

## 2. Test Methodology Matrix

Not every subsystem needs the same kind of test. This section maps the RIGHT methodology to each area.

### 2.1 Golden Artifact Testing (Snapshot Tests)

**Tool**: `insta` crate for Rust snapshot testing.

Golden artifacts freeze the serialized form of critical data structures. Any change to serialization format causes a test failure, forcing deliberate review.

| Target | File | Why Snapshot? | Priority |
|--------|------|---------------|----------|
| `ChainEvent` JSON serialization | chain.rs | Wire format is the audit trail. Accidental field changes break chain replay. | **P0** |
| `ChainEvent` CBOR/RVF encoding | chain.rs | Binary format used in chain replication. Format drift breaks mesh sync. | **P0** |
| `AnchorReceipt` serialization | chain.rs | Ed25519/ML-DSA-65 signed receipts must be bit-stable. | **P1** |
| `GovernanceResult` JSON | governance.rs | GUI Chain Viewer will render these. Format must be stable. | **P1** |
| `EffectVector` serialization | governance.rs | 5-dimensional vector format shared with GUI Governance Console. | **P2** |
| `A2A` message envelope | a2a.rs | Inter-agent message format. Changes break backward compatibility. | **P1** |
| `MeshIpcEnvelope` | mesh_ipc.rs | Two-node communication format. Must be stable for mesh protocol. | **P1** |
| `KernelConfig` / `Config` parsing | clawft-types | `weave.toml` schema. Config parsing changes break user deployments. | **P1** |
| `ToolSignature` | wasm_runner.rs | Ed25519 signatures over tools. Bit-level stability required. | **P0** |
| `ProcessState` FSM transitions | process.rs | State machine is foundation of process management. | **P2** |

**Implementation**: Add `insta` to `[dev-dependencies]` of clawft-kernel, clawft-types, and clawft-services. Create `snapshots/` directories. Each snapshot test serializes the structure and calls `insta::assert_snapshot!()`. Snapshot files are committed to git; changes require explicit `cargo insta review`.

**Estimated effort**: 4 hours for the P0 items, 6 hours total for P0+P1.

### 2.2 Metamorphic Testing

Metamorphic testing validates systems where there is no simple oracle (no single "expected output") by checking that mathematical relations between inputs and outputs hold.

#### CausalGraph (causal.rs -- 46 tests, needs metamorphic relations)

| Relation | Description | Test |
|----------|-------------|------|
| **Node addition is monotonic** | `add_node` must increase `node_count()` by exactly 1 | Add N nodes, assert count == N |
| **Edge addition preserves nodes** | Adding an edge between A and B must not remove either node | Add edge, assert both nodes still retrievable |
| **Transitive reachability** | If A causes B and B causes C, then A must be reachable-ancestor of C | Build chain, test transitive query |
| **Remove preserves unrelated** | Removing node X must not affect the subgraph not connected to X | Remove node, verify disjoint subgraph intact |
| **Serialization roundtrip is identity** | `deserialize(serialize(graph)) == graph` for any graph | Property test with random graph |

#### HNSW Search (hnsw_service.rs -- 13 tests, needs metamorphic relations)

| Relation | Description | Test |
|----------|-------------|------|
| **Recall monotonicity** | Adding a new vector must not reduce recall for existing queries | Insert V, query Q, record results; insert V2 (unrelated), re-query Q, assert recall >= previous |
| **Self-retrieval** | Searching for an inserted vector must return that vector as the top result | Insert V, search for V, assert V is rank 1 |
| **Distance ordering** | Results must be sorted by ascending distance from query | For every search result, assert `dist[i] <= dist[i+1]` |
| **Dimension independence** | Rotating all vectors by the same orthogonal matrix must not change relative rankings | Apply rotation, re-query, assert same ordering |
| **k-sensitivity** | Requesting k+1 results must return a superset of the k results | Search with k=5 and k=10, assert first 5 match |

#### GovernanceEngine EffectVectors (governance.rs -- 67 tests)

| Relation | Description | Test |
|----------|-------------|------|
| **Threshold monotonicity** | Lowering the risk threshold must not change Deny to Permit | Evaluate at threshold T1 > T2, assert if Deny at T1 then Deny at T2 |
| **Dimension dominance** | If all dimensions of V1 <= V2, then V1's decision must be <= V2's | Compare two vectors, assert ordering |
| **Zero vector always permits** | EffectVector(0,0,0,0,0) must always produce Permit | Assert invariant |
| **Max vector always denies** | EffectVector(1,1,1,1,1) must always produce Deny (or EscalateToHuman) | Assert invariant |

**Estimated effort**: 8 hours for all three areas.

### 2.3 Fuzz Testing

**Tool**: `cargo-fuzz` with `libfuzzer` (Rust's built-in fuzzing support).

Fuzz targets are prioritized by **attack surface** -- external inputs that reach parsing code.

| Priority | Target | Entry Point | Attack Surface | Why |
|----------|--------|-------------|----------------|-----|
| **P0** | A2A message parser | `a2a.rs` deserialization | Network (mesh, external agents) | Malformed inter-agent messages from untrusted peers |
| **P0** | ExoChain deserialization | `chain.rs` JSON + CBOR parse | Disk (persistence), Network (chain replication) | Corrupted chain files or malicious chain sync |
| **P0** | WASM tool boundary | `wasm_runner.rs` tool input/output | WASM sandbox boundary | Malicious WASM tools passing crafted data |
| **P1** | Config parsing | `clawft-types` TOML/JSON parse | User-supplied `weave.toml` | Malformed configuration |
| **P1** | RVF codec | `clawft-weave/rvf_codec.rs` | Network (daemon protocol) | Binary protocol from external clients |
| **P1** | MeshIpcEnvelope | `mesh_ipc.rs` | Network (peer mesh) | Peer-to-peer attack vector |
| **P2** | Tool signature verification | `wasm_runner.rs` signature check | Tool distribution | Crafted signatures to bypass verification |
| **P2** | HNSW deserialization | `hnsw_service.rs` load from disk | Disk | Corrupted index file |
| **P3** | Shell command parsing | `wasm_runner.rs` ShellCommand | Internal (WASM tool) | Command injection via shell.exec tool |

**Implementation**: Create `fuzz/` directory at workspace root. Each target gets a `fuzz_target!` macro with structured input via `arbitrary::Arbitrary` where possible.

**Estimated effort**: 6 hours for P0 targets, 4 hours for P1, 2 hours for P2.

### 2.4 End-to-End Integration Tests

The codebase has 8 E2E tests in `crates/clawft-kernel/tests/e2e_integration.rs`:

1. `boot_spawn_message_shutdown` -- full lifecycle
2. `governance_blocks_unauthorized_action` -- gate enforcement
3. `service_lifecycle` -- service registry
4. `persistence_roundtrip` -- save/load cycle
5. `multi_agent_ipc` -- two-agent message passing
6. `topic_pubsub` -- pub/sub routing
7. `agent_restart_on_failure` -- self-healing
8. `dead_letter_queue_captures` -- DLQ capture

And feature composition tests in `crates/clawft-kernel/tests/feature_composition.rs` covering boot under different feature flag combinations.

**Missing E2E scenarios:**

| # | Scenario | What It Tests | Priority |
|---|----------|---------------|----------|
| E1 | Boot -> DEMOCRITUS tick -> causal node created -> shutdown -> reboot -> verify node persisted | Full cognitive persistence loop | **P0** |
| E2 | Two-node mesh: boot node A, boot node B, discover, exchange A2A message, verify on both chains | Mesh protocol end-to-end | **P0** |
| E3 | WASM tool execution with chain logging: register signed tool -> execute -> verify chain event | Tool signing + execution + audit | **P1** |
| E4 | Governance escalation: agent requests high-risk action -> GovernanceEngine denies -> DLQ captures -> chain logs denial | Full governance pipeline | **P1** |
| E5 | Restart budget exhaustion: spawn agent -> crash 6 times in 60s -> verify budget exceeded escalation | Self-healing limits | **P1** |
| E6 | Tree mutation cascade: create resource tree -> mutate 14 operations -> verify all 14 chain events present | Tree manager audit completeness | **P2** |
| E7 | Feature composition: boot with `full` features -> exercise all subsystems -> clean shutdown | Feature interaction testing | **P2** |
| E8 | Config-driven boot: parse `weave.toml` -> boot kernel with derived config -> verify all services match config | Configuration fidelity | **P2** |

**Estimated effort**: 3 hours per E2E test, 12 hours for P0+P1.

### 2.5 Property-Based Testing (proptest)

**Tool**: `proptest` crate.

Property-based testing generates thousands of random inputs and verifies that invariants hold.

| Target | Invariant | Strategy |
|--------|-----------|----------|
| **ExoChain hash linking** | For any sequence of appended events, `event[i].prev_hash == event[i-1].hash` | Generate random payloads, append N events, verify chain |
| **ProcessState FSM** | `can_transition_to()` is consistent: if transition A->B is valid and B->C is valid, the system can reach C from A | Generate random state sequences, verify FSM |
| **RestartBudget window** | Budget tracker resets after window expires: after `within_secs` seconds, restart count resets | Generate random crash sequences with timestamps |
| **EffectVector magnitude** | `magnitude()` == sqrt(sum of squares) for all possible f64 inputs in [0,1] | Generate random 5-tuples |
| **CausalGraph acyclicity** | Adding edges should never create cycles (if graph is meant to be a DAG) | Generate random graphs, verify topological ordering |
| **Topic subscription** | A subscriber always receives messages published after subscription | Generate publish/subscribe interleavings |
| **Capability checker** | An agent with capability set S can perform action A iff A is in S | Generate random capability sets and actions |
| **Config roundtrip** | `parse(serialize(config)) == config` for all valid configs | Generate random Config structs |

**Estimated effort**: 6 hours for the first 4 invariants, 4 hours for the rest.

---

## 3. Top 10 Test Gaps (Ordered by Risk)

### Gap 1: Persistence Layer (CRITICAL -- 3 tests for data durability)

**Risk**: Data loss on restart. The persistence subsystem (`persistence.rs`) has exactly 3 tests: `config_paths`, `load_missing_returns_defaults`, and `save_and_load_all_roundtrip`. This covers the happy path only.

**Missing**:
- Corrupt file recovery (truncated JSON, invalid CBOR)
- Concurrent save during active writes
- Disk-full handling
- Partial save (crash during write) -- write-ahead-log or fsync guarantees
- Large dataset persistence (>10K chain events, >100K HNSW vectors)

**Recommendation**: Add 15-20 tests covering error paths and edge cases. Add a fuzz target for deserialization of persisted files.

### Gap 2: Gate Security Checks (HIGH -- 12 tests for 809-line security boundary)

**Risk**: Authorization bypass. The `gate.rs` file implements `CapabilityGate` and `TileZeroGate` -- the security boundary for all agent actions. With 12 tests for 809 lines, the negative-path coverage (denial, invalid tokens, expired receipts, replay attacks) is insufficient.

**Missing**:
- Replay attack on TileZero receipts (same receipt used twice)
- Invalid/malformed capability strings
- Race condition: capability revoked during check
- TileZero receipt with wrong Ed25519 signature
- Boundary: empty capability set, wildcard capabilities

**Recommendation**: Add 20+ security-focused tests. This is the highest-priority security gap.

### Gap 3: DEMOCRITUS Loop (HIGH -- 12 tests for 774-line cognitive core)

**Risk**: Silent cognitive degradation. The DEMOCRITUS loop (`democritus.rs`) drives the continuous Sense-Embed-Search-Update-Commit cycle. With only 12 async tests for 774 lines, the loop's behavior under resource pressure, budget exhaustion, and error conditions is untested.

**Missing**:
- Budget exhaustion mid-tick (what happens when calibration budget is exceeded?)
- ImpulseQueue overflow
- Concurrent tick execution (reentrancy guard)
- Embed failure recovery (ONNX model not available)
- CrossRefStore consistency under concurrent access

**Recommendation**: Add 15+ tests, prioritizing error recovery and budget edge cases.

### Gap 4: Chain Audit Trail Completeness (HIGH -- identified in Track 1)

**Risk**: Compliance gap. Track 1 found that process restarts, governance decisions (non-TileZero), IPC failures, and DEMOCRITUS ticks are NOT chain-logged. This means these events are also NOT tested for chain logging.

**Missing**:
- Test that supervisor restart creates a chain event
- Test that GovernanceEngine deny creates a chain event
- Test that DLQ insertion creates a chain event
- Test that DEMOCRITUS commit creates a chain event

**Recommendation**: First implement the `ChainLoggable` trait (Track 1 recommendation), then add chain-logging tests for each event category. 8 tests, coupled with Track 1 code changes.

### Gap 5: clawft-weave Daemon (MEDIUM -- 23 tests for 5,214-line daemon)

**Risk**: Runtime startup failures. The `clawft-weave` crate is the actual daemon binary. It contains RVF codec, RPC protocol, client library, and daemon lifecycle management. At 4.4 tests/KLOC (lowest density of any crate), the daemon startup, shutdown, connection handling, and protocol error paths are nearly untested.

**Missing**:
- Daemon bind failure (port in use)
- Client connection timeout
- RVF codec malformed frame
- RPC dispatch for unknown method
- Graceful shutdown with active connections

**Recommendation**: Add 20+ tests across codec, RPC, and daemon modules. Add fuzz target for RVF codec.

### Gap 6: No Snapshot Tests for Wire Formats (MEDIUM)

**Risk**: Silent format breakage. The codebase serializes `ChainEvent`, `A2A` messages, `MeshIpcEnvelope`, `ToolSignature`, and `GovernanceResult` to JSON and CBOR for persistence and network transport. No snapshot tests exist to detect accidental format changes.

**Recommendation**: Add `insta` snapshot tests for all wire formats (see Section 2.1). 10 hours.

### Gap 7: No Property-Based Tests (MEDIUM)

**Risk**: Missed invariant violations. The hash-chain linking in ExoChain, the FSM transitions in ProcessState, and the mathematical properties of EffectVector are all candidates for property-based testing that would catch edge cases human-written tests miss.

**Recommendation**: Add `proptest` for chain linking, FSM transitions, and EffectVector math (see Section 2.5). 6 hours.

### Gap 8: No Fuzz Harnesses (MEDIUM)

**Risk**: Crash/panic on malformed input. Every network-facing parser (A2A, mesh IPC, RVF codec) and every disk-facing deserializer (chain, HNSW, config) is a fuzz target. Zero fuzz harnesses exist.

**Recommendation**: Add `cargo-fuzz` targets for P0 items (see Section 2.3). 6 hours.

### Gap 9: Feature Flag Combination Testing (LOW-MEDIUM)

**Risk**: Build failure under untested flag combinations. 93 feature flags across 20 crates create a combinatorial space. Currently 6 combinations are tested (per the plenary). The `feature_composition.rs` test file covers boot under different combos but does not exercise subsystem interactions.

**Missing**:
- `ecc` without `exochain` -- does the DEMOCRITUS loop degrade gracefully?
- `os-patterns` without `ecc` -- do metrics, DLQ, and timer work without cognitive substrate?
- `tilezero` without `governance` -- does the gate fall back correctly?
- `mesh` without `exochain` -- can mesh nodes communicate without chain replication?

**Recommendation**: Expand `feature_composition.rs` with 4 additional negative/degraded-mode tests. 3 hours.

### Gap 10: 15 Registry Structs -- Inconsistent Test Coverage (LOW)

**Risk**: Broken registration/lookup for some registries. Track 1 found 15 Registry structs. Test density varies wildly: `ServiceRegistry` has 45 tests; `ClusterServiceRegistry` and `WorkspaceRegistry` have near-zero dedicated tests.

**Missing**:
- Uniform CRUD coverage for all 15 registries
- Concurrent access tests for DashMap-backed registries (ServiceRegistry, ProcessTable, MetricsRegistry, MonitorRegistry, NamedPipeRegistry)
- Registration collision (duplicate key) handling

**Recommendation**: Audit each registry's test coverage; add 2-3 tests to each under-tested registry. 4 hours.

---

## 4. CI Pipeline Additions

### 4.1 Current State

The build script (`scripts/build.sh`) supports:
- `native` / `native-debug` -- release and debug builds
- `test` -- workspace test run
- `check` -- compile check
- `clippy` -- lint
- `gate` -- 11-check phase gate

### 4.2 Recommended Additions

| Stage | Command | When | What | Effort |
|-------|---------|------|------|--------|
| **Snapshot check** | `cargo insta test --check` | Every PR | Fail if any snapshot changed without explicit review | 1 hour (after insta is added) |
| **Property tests** | `PROPTEST_CASES=1000 cargo test --features full -- proptest` | Every PR | Run 1,000 cases per property test | 1 hour |
| **Fuzz regression** | `cargo fuzz run <target> -- -max_total_time=60` | Nightly | Run each fuzz target for 60 seconds; corpus grows over time | 2 hours |
| **Fuzz corpus check** | `cargo fuzz run <target> <corpus_dir> -- -runs=0` | Every PR | Replay existing corpus (catches regressions fast) | 1 hour |
| **Feature matrix** | `for combo in native "native,exochain" "native,ecc" "native,os-patterns" "native,full"; do cargo check --features "$combo"; done` | Every PR | Verify all key feature combinations compile | 1 hour (extend build.sh) |
| **Coverage report** | `cargo llvm-cov --workspace --lcov > lcov.info` | Weekly | Track coverage trend; fail if below threshold | 2 hours |
| **Miri (unsafe check)** | `cargo +nightly miri test -p exo-resource-tree -p clawft-kernel` | Weekly | Detect undefined behavior in unsafe code | 1 hour |
| **Doc tests** | `cargo test --doc --workspace` | Every PR | Verify all `///` code examples compile and run | 30 minutes |

### 4.3 Gate Script Extension

Add to `scripts/build.sh`:

```bash
# After existing gate checks:
gate_snapshot)  cargo insta test --check ;;
gate_proptest)  PROPTEST_CASES=256 cargo test --features full -- proptest ;;
gate_fuzz)      for t in $(cargo fuzz list 2>/dev/null); do cargo fuzz run "$t" -- -runs=0; done ;;
gate_features)  for f in native "native,exochain" "native,ecc" "native,os-patterns" "native,full"; do
                  cargo check --no-default-features --features "$f" || exit 1
                done ;;
```

### 4.4 Feature Flag Matrix Strategy

With 93 flags, full combinatorial testing is impossible. Instead:

1. **Tier 1 (every PR)**: 6 canonical combinations (current): `native`, `native+exochain`, `native+ecc`, `native+os-patterns`, `native+exochain+ecc+os-patterns`, `full`
2. **Tier 2 (nightly)**: 4 degraded-mode combinations: `ecc` alone, `os-patterns` alone, `tilezero+exochain`, `mesh+exochain`
3. **Tier 3 (weekly)**: Random 10 combinations generated by script, tested for compile-only (catch feature gate interaction bugs)

This keeps PR latency under 5 minutes while providing good coverage across the 93-flag space.

---

## 5. Sprint 11 Testing Work Items

### 5.1 Summary Table

| # | Work Item | Methodology | Tests Added | Effort | Priority |
|---|-----------|-------------|-------------|--------|----------|
| T1 | Persistence error-path tests | Unit + fuzz | 20 | 6h | **P0** |
| T2 | Gate security tests | Unit (negative path) | 20 | 4h | **P0** |
| T3 | DEMOCRITUS loop error/budget tests | Unit + async | 15 | 4h | **P0** |
| T4 | Wire format snapshot tests (insta) | Snapshot | 12 | 4h | **P0** |
| T5 | ExoChain property tests (proptest) | Property-based | 5 props x 1000 cases | 3h | **P1** |
| T6 | Fuzz harnesses for P0 targets | Fuzz | 3 targets | 6h | **P1** |
| T7 | Missing E2E tests (E1-E4) | Integration | 4 | 12h | **P1** |
| T8 | Chain audit trail completeness tests | Integration | 8 | 4h | **P1** |
| T9 | clawft-weave daemon tests | Unit + integration | 20 | 6h | **P2** |
| T10 | Feature composition expansion | Integration | 4 | 3h | **P2** |
| T11 | Registry coverage equalization | Unit | 15 | 4h | **P2** |
| T12 | HNSW metamorphic tests | Metamorphic | 5 | 3h | **P2** |
| T13 | CI pipeline additions | Infrastructure | N/A | 4h | **P1** |

**Total**: ~130 new tests, ~63 hours of effort.

### 5.2 Phasing

**Week 1 (P0)**: T1, T2, T3, T4 -- 18 hours, 67 new tests. Addresses the 4 most dangerous gaps.

**Week 2 (P1)**: T5, T6, T7, T8, T13 -- 29 hours, 17+ new tests plus property/fuzz infrastructure. Establishes new testing methodologies.

**Week 3 (P2)**: T9, T10, T11, T12 -- 16 hours, 44 new tests. Coverage equalization and secondary gaps.

### 5.3 Dependencies

- T4 (snapshot tests) requires adding `insta` to workspace `[dev-dependencies]`
- T5 (property tests) requires adding `proptest` to workspace `[dev-dependencies]`
- T6 (fuzz) requires `cargo-fuzz` installed and `fuzz/` directory created
- T8 (chain audit) depends on Track 1's `ChainLoggable` trait implementation
- T13 (CI) depends on T4, T5, T6 being merged first

---

## 6. ECC Contribution: CMVG Nodes and Edges

The following causal nodes and edges should be added to the Sprint 11 CMVG from Track 2's findings:

```
NODES:
  [N12] Test Coverage Map Established
       status: ACHIEVED
       evidence: 5,040 tests counted, per-crate and per-module breakdown, 6 methodology gaps identified
       artifact: This document (02-testing-strategy.md)

  [N13] Persistence Testing Gap Identified
       status: IDENTIFIED
       risk: CRITICAL -- 3 tests for data durability subsystem
       evidence: persistence.rs has only config_paths, load_missing, roundtrip tests
       remediation: T1 work item (20 tests, 6h)

  [N14] Security Gate Testing Gap Identified
       status: IDENTIFIED
       risk: HIGH -- 12 tests for 809-line authorization boundary
       evidence: gate.rs missing negative-path, replay, and race-condition tests
       remediation: T2 work item (20 tests, 4h)

  [N15] Testing Methodology Gaps Identified
       status: IDENTIFIED
       evidence: Zero snapshot tests, zero property tests, zero fuzz harnesses, zero benchmarks
       risk: Silent format breakage, missed invariants, crash-on-malformed-input
       remediation: T4 (insta), T5 (proptest), T6 (cargo-fuzz) work items

  [N16] Testing Strategy Defined
       status: PROPOSED
       depends_on: N12, N13, N14, N15
       artifact: 13 work items, 130 new tests, 63 hours effort
       deliverable: Sprint 11 test infrastructure and coverage improvement

  [N17] CI Pipeline Testing Additions
       status: PROPOSED
       depends_on: N15, N16
       artifact: Snapshot check, property test run, fuzz regression, feature matrix, coverage report
       deliverable: Extended scripts/build.sh gate command

EDGES:
  N1  --[Reveals]--> N13    Kernel completion reveals persistence gap
  N1  --[Reveals]--> N14    Kernel completion reveals gate security gap
  N1  --[Reveals]--> N15    Kernel completion reveals methodology gaps
  N12 --[Enables]--> N16    Coverage map enables targeted strategy
  N13 --[Motivates]--> N16  Persistence gap is P0 in strategy
  N14 --[Motivates]--> N16  Gate gap is P0 in strategy
  N15 --[Motivates]--> N16  Methodology gaps are P1 in strategy
  N16 --[Enables]--> N17    Strategy defines CI requirements
  N8  --[Compounds]--> N13  Audit trail gaps compound persistence risk
  N9  --[Enables]--> N16    ChainLoggable trait enables audit trail tests (T8)

CAUSAL CHAIN:
  N1 (achieved) --> N12 (achieved) --> N16 (proposed) --> N17 (proposed)
  N1 (achieved) --> N13 (identified) --> T1 work item --> [persistence safety]
  N1 (achieved) --> N14 (identified) --> T2 work item --> [security gate coverage]
  N1 (achieved) --> N15 (identified) --> T4/T5/T6 work items --> [methodology foundations]
  N8 (Track 1) --> N13 --> [audit + persistence compound risk]
```

---

## Appendix A: Test Methodology Decision Tree

```
Is the output a serialized wire format or config schema?
  YES --> Golden Artifact (insta snapshot)
  NO  |
      v
Does the system have clear input/output pairs?
  YES --> Unit test (standard assert)
  NO  |
      v
Can you define mathematical relations between inputs and outputs?
  YES --> Metamorphic test
  NO  |
      v
Is the input from an untrusted source (network, disk, user)?
  YES --> Fuzz test (cargo-fuzz)
  NO  |
      v
Does the code maintain a mathematical invariant?
  YES --> Property-based test (proptest)
  NO  |
      v
Does the feature span multiple subsystems?
  YES --> E2E integration test
  NO  --> Unit test with mock
```

## Appendix B: Crate Dependency for New Test Infrastructure

```toml
# Workspace Cargo.toml [workspace.dev-dependencies]
insta = { version = "1.39", features = ["json", "yaml"] }
proptest = "1.5"

# Per-crate fuzz targets (separate Cargo.toml in fuzz/)
[dependencies]
libfuzzer-sys = "0.4"
arbitrary = { version = "1", features = ["derive"] }
```

---

## Session Notes

- All test counts are from direct `#[test]` and `#[tokio::test]` annotation grep on the `feature/weftos-kernel-sprint` branch as of 2026-03-27.
- "Tests per KLOC" is a rough density metric; it does not distinguish between trivial and thorough tests.
- The 5,040 total matches the plenary's count (4,069 sync + 971 async).
- No test was manually inspected for quality (assertion depth, mock fidelity) in this pass. A follow-up "test quality audit" is recommended for Sprint 12.
- The persistence gap (3 tests) was independently verified by reading the full `persistence.rs` test module. Only three `#[test]` functions exist: `config_paths`, `load_missing_returns_defaults`, `save_and_load_all_roundtrip`.
