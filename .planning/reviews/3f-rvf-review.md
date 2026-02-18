# Expert Review: Phase 3F (Part 1) -- RVF/ruvector Full Integration

## Reviewer: AI/ML Systems Architect (vector DB + MCP specialist)
## Date: 2026-02-17
## Verdict: REVISE

---

## Scores

| Dimension | Score | Notes |
|-----------|-------|-------|
| Completeness | 4/5 | All 5 RVF levels covered thoroughly. MCP server bridge well-specified. Minor gap: no rvf-graph (Level 5) stub, though acknowledged as "nice to have". |
| Technical Accuracy | 3/5 | HNSW approach is sound in concept but several implementation details are speculative (see issues below). MCP tool definitions are reasonable but protocol version "2025-06-18" conformance needs verification against 3H plan. |
| Integration Points | 3/5 | Correctly identifies existing vector_store.rs and embeddings modules as replacement targets. However, the mapping from existing `VectorStore` API to `RvfVectorStore` has interface mismatches. Does not address how `IntelligentRouter`'s existing 550-line implementation transitions. |
| Scope Creep Risk | 2/5 | This is the weakest dimension. 44 engineer-days across 8 sprints for a full vector DB + neural routing + self-learning + crypto audit trail + MCP server + WASM microkernel is extremely aggressive. Multiple sprints depend on unproven external crate APIs. |
| Dependencies | 3/5 | Several conflicts identified with 3D, 3E, and 3H. Timeline overlap with 3D (both claim Weeks 15-17) is a scheduling concern. |
| Test Coverage | 4/5 | 126+ tests with a well-structured feature flag matrix. Missing: fuzz testing for RVF file format parsing, concurrent access tests for RvfVectorStore, and MCP protocol negative path tests. |
| WASM Microkernel | 3/5 | <8KB + 11.8KB claims need validation. The 250KB total WASM budget is tight against 3E's 300KB cap (and 3D's relaxed 350KB), leaving only ~50KB headroom for rvf-types (~30KB NEW). |
| Feature Flags | 4/5 | Well-designed tiered system. Combinatorial explosion is mitigated by the convenience bundles (`ruvector`, `ruvector-full`). Minor concern: 12 feature flags in clawft-core alone creates a large test matrix. |

**Weighted Average: 3.25/5**

---

## Strengths

1. **Excellent level decomposition**. The 5-level architecture (L0-L4) is one of the best aspects of this plan. Each level adds intelligence without requiring the next, and the feature flag design makes each level independently toggleable. The graceful degradation story is strong: disable any feature flag and the system falls back cleanly.

2. **Thorough current-state assessment**. Section 1 accurately catalogs every existing module (VectorStore, HashEmbedder, IntelligentRouter, SessionIndexer, NoopScorer, NoopLearner) and correctly identifies what each must be replaced with. The gap analysis in 3I confirms these are all Level 0 fallbacks in place.

3. **MCP server bridge is well-designed**. The 11-tool MCP interface covers the essential vector store operations. The hooks integration path (pre-task -> rvf_query, post-edit -> rvf_ingest, etc.) is practical. The JSON-RPC over stdio with Content-Length framing aligns with the 3H plan's MCP protocol work.

4. **Feature flag bundles prevent combinatorial explosion**. The `ruvector` and `ruvector-full` convenience features are good for users. Individual toggles (`ruvllm`, `tiny-dancer`, `sona`, `attention`) are good for developers.

5. **Risk register is honest**. Acknowledging that ruvllm's "minimal" feature might still pull candle (5-15MB), that tiny-dancer pulls native-only deps, and that ruvector crates are pre-1.0 -- these are real risks properly identified.

6. **Cross-stream dependency tracking is complete**. Section 12 correctly identifies all inputs (from Phases 1B, 1C, 2B, 3A) and outputs (to CLI, WASM, hooks, future channels). No missing edges in the dependency graph.

---

## Issues Found

### Critical

**C1: Timeline overlap with Phase 3D (WASI Docker) -- scheduling conflict**

Phase 3D claims Weeks 15-17. Phase 3F claims Weeks 15-22. Both modify `clawft-wasm`. Sprint 7 of 3F (Week 21) adds rvf-wasm to clawft-wasm, while 3D (Week 15-17) is rewriting the HTTP/FS stubs and adding a binary entry point. If 3D is not complete before 3F Sprint 7 starts, there will be merge conflicts and API instability in the WASM crate.

**Recommendation**: Phase 3F Sprint 7 (WASM microkernel) MUST have a hard dependency gate on 3D completion. Add this to the dependency list. Currently section 12 says "clawft-wasm platform | Phase 3A | WASI HTTP, FS, Env implementations" but 3A only delivers stubs. The real WASI implementations come from 3D.

**C2: ruvector crate maturity risk is underestimated**

The plan depends on 11 external crates from a single git repository (`ruvnet/ruvector`), all pre-1.0. The risk register rates API instability as "High likelihood, Medium impact" but the mitigation ("pin exact git commits, wrap behind clawft's own trait abstractions") does not account for the scenario where entire crate APIs do not exist yet or are non-functional. There is no evidence in this plan that these crates have been tested, that their `Cargo.toml` feature flags work as described, or that their public APIs match the wrapper code shown in the pseudocode.

**Recommendation**: Before Sprint 1 begins, add a "Sprint 0" (1-2 days) that validates:
- Each ruvector crate compiles independently with the expected features
- `rvf-runtime` can create and open a `.rvf` file
- `ruvllm` with `minimal` feature does not pull candle
- `micro-hnsw-wasm` compiles to wasm32-wasip1 under 12KB
- `sona` compiles with the expected public API

If any of these fail, the sprint plan needs immediate restructuring. This validation sprint is essential before committing to 44 engineer-days.

**C3: MCP protocol version alignment with Phase 3H**

Phase 3H specifies the MCP initialize handshake with `protocolVersion: "2025-06-18"` and Content-Length framing. Phase 3F's MCP server bridge (Sprint 6) independently implements JSON-RPC over stdio with Content-Length framing. These two implementations MUST share the transport layer. Currently 3F creates a new `clawft-rvf-mcp` crate with its own `transport.rs`, while 3H rewrites the existing `StdioTransport` in `clawft-services/src/mcp/transport.rs`.

**Recommendation**: The `clawft-rvf-mcp` crate should depend on the MCP transport from `clawft-services` (after 3H fixes it), not implement its own. Add 3H as a hard dependency for Sprint 6. Alternatively, define the transport interface now and have both 3F and 3H implement against it.

### Major

**M1: Binary size estimates are optimistic and untested**

Section 9 estimates the native binary at ~8.8MB with ruvector-full, well under the 12MB target. But the estimate for `ruvllm (minimal)` at ~2MB is a guess. The risk register acknowledges ruvllm might pull candle (5-15MB). If candle slips in, the total becomes 12-23MB, blowing the budget.

For WASM, the plan claims ~250KB total, but this includes "rvf-types ~30KB" marked as both "Existing" (in line 567) and "NEW" (in line 567). The existing 242KB baseline from 3E already accounts for micro-hnsw, temporal-tensor, and sona. Adding rvf-wasm (8KB) + rvf-types (30KB) brings the actual new addition to ~38KB, for a total of ~280KB. This leaves only 20KB headroom before hitting the 300KB budget from 3E. Phase 3D relaxes this to 350KB but only temporarily for HTTP/FS. There is no documented agreement that 3F gets the relaxed budget.

**Recommendation**: Add a Sprint 1 exit gate that measures actual binary size delta after adding rvf dependencies. If ruvllm-minimal exceeds 3MB or WASM exceeds 280KB, escalate immediately.

**M2: Progressive HNSW implementation details are vague**

Sprint 2 describes a "3-tier progressive HNSW" with Layer A (~70% recall), Layer B (~85% recall), and Layer C (~95% recall). However:
- The plan says it wraps `rvf-index`, but does not explain whether rvf-index natively supports progressive loading or if this is a new capability to be built on top of it.
- "Background task for progressive index building" -- how does this work without tokio in WASM? Does clawft-core spawn a thread? Use `spawn_blocking`? The plan does not address the async story.
- "Checkpoint every 100 insertions" -- what data structure is checkpointed? The HNSW graph? A partial index file?

**Recommendation**: Clarify whether progressive HNSW is a feature of rvf-index or a new abstraction built on top of brute-force + incremental HNSW construction. Document the async strategy.

**M3: SONA learning claims are unvalidated**

Sprint 4 claims "SonaLearner updates MicroLoRA weights after each response" and "EWC++ tested: learning new pattern does not degrade old pattern quality." These are strong claims for a 6-day sprint. MicroLoRA rank-2 updates on routing decisions require:
- A weight matrix (where does it live? .rvf file? In-memory?)
- A forward pass through the routing model
- A backward pass to compute gradients
- An EWC++ regularization term
- All without a GPU, running within the agent loop latency budget

The plan does not explain how SONA learns from routing outcomes. What is the training signal? How is "quality" measured before QualityScorer is wired (which depends on ruvllm from Sprint 3)?

**Recommendation**: Scope SONA integration to recording trajectories only in Sprint 4. Defer actual micro-LoRA weight updates to a follow-up sprint after the quality scoring feedback loop is validated. The three-tier learning loop (instant/hourly/weekly) is a multi-month effort, not a 6-day sprint.

**M4: ApiEmbedder dependency on clawft-llm Provider::embedding() may not exist**

Sprint 1 Task 4 says "Implement ApiEmbedder calling clawft-llm Provider::embedding()." The codebase map shows clawft-llm has an `OpenAiCompatProvider` with a `complete()` method but there is no evidence of an `embedding()` method on the Provider trait. The provider.rs is 44 lines -- barely enough for the trait definition, let alone embeddings.

**Recommendation**: Verify the Provider trait in clawft-llm supports embeddings. If not, add embedding support to clawft-llm first (additional 1-2 days in Sprint 1). The OpenAI-compatible embedding endpoint is `POST /v1/embeddings`, which is a different API shape from chat completions.

### Minor

**m1: Feature flag `rvf` does not include `rvf-quant`**

In section 3, the feature definition is:
```toml
rvf = ["dep:rvf-runtime", "dep:rvf-types", "dep:rvf-index", "dep:rvf-quant"]
```

But the description in section 9 (Size Budget) lists rvf-quant as part of the base `rvf` feature at ~300KB combined. This is correct in the Cargo.toml definition. However, the `temporal-tensor` feature is separate from `rvf` -- the quantization module in `clawft-core/src/rvf/quantizer.rs` uses `ruvector-temporal-tensor`, not `rvf-quant`. The relationship between `rvf-quant` (RVF format quantization) and `ruvector-temporal-tensor` (temperature-based tiering) is unclear.

**Recommendation**: Document the distinction. `rvf-quant` handles storage-level quantization in the .rvf binary format. `ruvector-temporal-tensor` handles access-frequency-based tier management. Both are needed for the full quantization story but serve different purposes.

**m2: WitnessLog Ed25519 adds a crypto dependency**

Sprint 5 introduces Ed25519 signatures via rvf-crypto. This pulls in `ed25519-dalek` or similar. The plan lists rvf-crypto at ~50KB, but does not account for the ring/dalek dependency tree. If ed25519-dalek is compiled with its default features, it can add 200-400KB to native and is problematic for WASM.

**Recommendation**: Specify that rvf-crypto must use `ed25519-dalek` with `no_std` or minimal features. Verify this is the case in the rvf-crypto crate. Document the WASM story -- is rvf-crypto intended to work in WASM?

**m3: First-startup migration (Sprint 5) should be Sprint 1-2**

The MEMORY.md bootstrap migration is deferred to Sprint 5, but it is logically part of the RVF foundation (Sprint 1). Users who enable the `rvf` feature will have an empty RVF store until Sprint 5 delivers bootstrap.rs. For 4 sprints, the RVF store and the MEMORY.md file coexist without synchronization.

**Recommendation**: Move a minimal bootstrap (parse MEMORY.md, batch embed, write to .rvf) to Sprint 2, after the RVF store and HNSW are available. The full AgentDB adapter and agentic-flow integration can stay in Sprint 5.

**m4: Test infrastructure uses HashEmbedder as mock, which may mask real embedding issues**

The test utilities section (Section 8) proposes `mock_embedder(dim) -> Box::new(HashEmbedder::new(dim))`. HashEmbedder produces unit-length SimHash vectors that have specific distribution properties (all dimensions active, deterministic). Real API embeddings from OpenAI's `text-embedding-3-small` have very different distributions (sparse activations, variable norms before normalization). Tests using HashEmbedder may pass but fail with real embeddings.

**Recommendation**: Add at least one integration test that uses pre-computed real embeddings (fixture vectors from a known embedding model) to validate HNSW recall, quantization accuracy, and search quality metrics.

---

## Cross-Phase Conflicts

| Phase | Conflict | Severity | Resolution |
|-------|----------|----------|------------|
| 3D (WASI Docker) | Both modify clawft-wasm in overlapping weeks (15-17 vs 15-22). 3D rewrites HTTP/FS; 3F Sprint 7 adds rvf-wasm. | HIGH | Gate 3F Sprint 7 on 3D completion. |
| 3E (Optimization) | 3E targets <=300KB WASM binary. 3F adds ~38KB to WASM. Combined may exceed budget. | MEDIUM | Share WASM size budget. 3F must operate within 3E's post-optimization headroom. |
| 3H (Tool Delegation) | Both implement MCP JSON-RPC transport independently. 3H rewrites StdioTransport with Content-Length framing; 3F creates new transport.rs in clawft-rvf-mcp. | HIGH | Share transport layer. 3F's MCP crate should depend on clawft-services MCP transport (post-3H). |
| 3I (Gap Analysis) | 3I identifies GAP-07 (vector memory uses hash embeddings only) and GAP-31 (RVF integration = 20h+) as P2 deferred. 3F is the resolution of these gaps but 3I estimates 20h+ while 3F estimates 44 days. Scope discrepancy. | LOW | 3I's estimate is for basic RVF integration. 3F's scope is the full intelligence stack. Different scope levels. |

---

## Missing Requirements

1. **Concurrent write safety**: The plan does not address concurrent access to `.rvf` files. If `weft gateway` has multiple channel plugins processing messages simultaneously, the `RvfVectorStore` will receive concurrent `add()` and `search()` calls. Is rvf-runtime thread-safe? Does it use file-level locking? The plan says "rvf-runtime has crash-safe design (no WAL needed)" but says nothing about concurrent access.

2. **Store migration/versioning**: When the RVF format or index structure changes between clawft versions, how are existing `.rvf` files migrated? The plan has "first-startup migration" from MEMORY.md to .rvf, but no subsequent migration path.

3. **Disk space management**: No limits on .rvf file growth. If the vector store grows to 100MB of embeddings, what happens? No compaction policy beyond manual `rvf_compact` calls. No automatic garbage collection.

4. **Embedding dimension mismatch handling**: The plan specifies ApiEmbedder with `text-embedding-3-small` (384-dim) and HashEmbedder as fallback. What happens if someone switches embedders after populating the store? 384-dim vectors from API mixed with 384-dim vectors from HashEmbedder will corrupt search quality. There should be a dimension and model metadata field in the store header.

5. **MCP server process lifecycle**: The plan says the MCP server runs as a subprocess (`weft rvf-mcp`). Who starts it? Who monitors it? What if it crashes? The hooks integration diagram shows hooks calling MCP tools, but the hooks system is in the claude-flow CLI, not in clawft. How does claude-flow discover and start the clawft RVF MCP server?

6. **Fallback behavior documentation**: When `#[cfg(feature = "rvf")]` is disabled but a `.rvf` file exists on disk, what happens? When `#[cfg(feature = "sona")]` is disabled mid-session, are learned routing weights lost? The "graceful degradation" story is mentioned but not specified.

---

## Scope Assessment

### Essential Sprints (MUST deliver)

| Sprint | Week | Focus | Justification |
|--------|------|-------|---------------|
| 1 | 15 | RVF Runtime Foundation + ApiEmbedder | Core capability. Without persistent vector storage, nothing else works. |
| 2 | 16 | Progressive HNSW + Persistence | Search performance is unusable at scale without HNSW. |
| 6 | 20 | MCP Server Bridge | Primary integration mechanism for hooks and external tools. |

### Important but Reducible

| Sprint | Week | Focus | Reduction Opportunity |
|--------|------|-------|-----------------------|
| 3 | 17 | ruvllm + tiny-dancer (L1-L2) | Could ship L1 only (ruvllm complexity analysis). Tiny-dancer neural routing (L2) can be deferred. 4 days instead of 6. |
| 5 | 19 | rvf-crypto + AgentDB + Bootstrap | Bootstrap is essential (move to Sprint 2). AgentDB adapter is valuable. rvf-crypto WITNESS can be deferred to post-release. 4 days instead of 6. |
| 8 | 22 | Polish, Benchmarks, CLI | Benchmarks are essential. CLI commands can ship incrementally. 4 days instead of 5. |

### Deferrable (move to Phase 4)

| Sprint | Week | Focus | Rationale |
|--------|------|-------|-----------|
| 4 | 18 | SONA + Attention (L3-L4) | Self-learning (SONA) requires validated quality scoring feedback loop that does not exist yet. Attention assembly (ruvector-attention) requires large-context scenarios that are not the primary use case. Both L3 and L4 are independent of L0-L2 functionality. |
| 7 | 21 | WASM Microkernel | Depends on 3D completion. WASM is not the primary deployment target. Can ship native-only first. |

### Recommended Revised Timeline

| Sprint | Week | Focus | LOE |
|--------|------|-------|-----|
| 0 | 14 | Dependency validation (compile all ruvector crates) | 2d |
| 1 | 15 | RVF Runtime + ApiEmbedder + Bootstrap (from Sprint 5) | 6d |
| 2 | 16 | Progressive HNSW + Session/Router persistence | 5d |
| 3 | 17 | ruvllm (L1 only) + QualityScorer | 4d |
| 4 | 18 | MCP Server Bridge (moved from Sprint 6) | 6d |
| 5 | 19 | rvf-crypto + AgentDB (no agentic-flow) | 4d |
| 6 | 20 | Polish, Benchmarks, CLI, Level integration tests | 5d |
| **Total** | **7 weeks** | | **32d** |
| **Deferred** | Phase 4 | tiny-dancer (L2), SONA (L3), Attention (L4), WASM microkernel | ~16d |

This reduces from 44 to 32 engineer-days while delivering L0-L1 intelligence, persistent RVF storage, HNSW search, MCP bridge, and crypto audit trail. L2-L4 and WASM are deferred to Phase 4 where they can be validated incrementally.

---

## Recommendations

1. **Add a Sprint 0 validation gate** (2 days). Compile every ruvector dependency. Measure binary size impact. Validate public APIs match the plan's pseudocode. This is the single highest-risk item in the plan and costs almost nothing to de-risk.

2. **Defer SONA and Attention (L3-L4) to Phase 4**. The learning loop requires quality scoring, which requires ruvllm, which is delivered in Sprint 3. Testing that SONA actually improves routing quality requires weeks of real-world data, not a 6-day sprint. Ship L0-L1 first, validate, then add L2-L4.

3. **Share MCP transport with Phase 3H**. Do not build a second JSON-RPC transport. The `clawft-rvf-mcp` crate should import `McpSession` and `StdioTransport` from `clawft-services` after 3H delivers Content-Length framing. This reduces Sprint 6 effort and eliminates a protocol conformance risk.

4. **Move MEMORY.md bootstrap to Sprint 1-2**. Users who enable `rvf` need their existing memory indexed immediately. Waiting until Sprint 5 creates a 4-week window where RVF is enabled but empty.

5. **Add concurrent access tests**. The VectorStore will be accessed from multiple async tasks in the gateway. Add tests for concurrent `add()` + `search()` operations. If rvf-runtime is not thread-safe, document the locking strategy.

6. **Pin ruvector crates to exact git commits on day 1**. The plan says this in the notes but should be an explicit Sprint 1 task. Create a `[patch]` section in workspace Cargo.toml if needed.

7. **Add embedding model metadata to the RVF store**. Record which embedder (hash vs API) and which model/dimension was used when creating the store. Reject queries with dimension mismatches at the API level, not as silent corruption.

8. **Clarify the WASM budget with Phase 3E**. Get explicit agreement that the 300KB budget accommodates ~38KB of new RVF components. If 3E delivers a 280KB binary, there is only 20KB of headroom -- insufficient for rvf-wasm (8KB) + rvf-types (30KB). Either negotiate a higher cap or defer rvf-types to native-only.

---

## Timeline Assessment

**Original**: 8 weeks, 44 engineer-days, 126+ tests
**Assessment**: Aggressive. Approximately 40% overscoped for the first release.

The plan tries to deliver a complete vector database, 5 intelligence levels, a neural routing engine, a self-learning system, an attention-based context assembler, a cryptographic audit trail, an MCP server, and a WASM microkernel in 8 weeks. Each of these is a substantial engineering effort on its own. Combined, they create a dependency chain where any upstream delay (particularly in ruvector crate maturity) cascades through every subsequent sprint.

The realistic path is: deliver L0-L1 with persistence, HNSW, complexity-aware routing, MCP bridge, and crypto trail in 7 weeks (32 days). Then deliver L2-L4 and WASM in a follow-up phase with validated dependencies. This halves the risk while delivering ~70% of the user-facing value.

**Key risk signal**: If Sprint 0 (dependency validation) reveals that any ruvector crate does not compile or has a significantly different API than expected, the entire 8-week plan must be replanned. The plan has no contingency for this scenario.
