# SPARC Feature Element 05: Pipeline & LLM Reliability

**Workstream**: D (Pipeline & LLM Reliability)
**Timeline**: Weeks 2-5
**Status**: Complete (all 11 items done)
**Dependencies**: None (can run parallel with 03)
**Blocks**: 09 (D6 sender_id needed for multi-agent cost tracking), 09/M1 (D9 MCP concurrency needed for FlowDelegator)

---

## 1. Summary

Improve the agent loop, LLM transport, and routing pipeline for reliability, performance, and observability.

---

## 2. Phases

### Phase D-Perf: Performance (Week 2-3)

| Item | Description | Crate/File | Impact |
|------|-------------|------------|--------|
| D1 | Parallel tool execution (`futures::join_all`) | `clawft-core/src/agent/loop_core.rs` | Major latency reduction |
| D10 | Cache skill/agent bootstrap files (mtime check) | `clawft-core/src/agent/context.rs` | Faster LLM calls |
| D11 | Async file I/O in skills loader (`tokio::fs`) | `clawft-core/src/agent/skills_v2.rs` | Unblock Tokio executor |

**Note:** D1, D10, and D11 are independent of each other and can be implemented in parallel.

### Phase D-Reliability: Correctness & Resilience (Week 3-4)

| Item | Description | Crate/File | Impact |
|------|-------------|------------|--------|
| D3 | Structured error variants for retry | `clawft-llm/src/retry.rs`, `clawft-types/src/error.rs` | Reliable retry decisions |
| D4 | Configurable retry policy (depends on D3) | `clawft-core/src/pipeline/llm_adapter.rs` | User-tunable resilience |
| D2 | Streaming failover correctness (reset stream) | `clawft-llm/src/failover.rs` | No garbled output |
| D7 | StreamCallback to FnMut | `clawft-core/src/pipeline/traits.rs` | Stateful callbacks work |
| D8 | Bounded message bus channels | `clawft-core/src/bus.rs` | Backpressure, no OOM |

**Internal dependency ordering:**
- D3 must complete before D4 (retry policy should use structured error variants, not strings)
- D7 before D2 is recommended (streaming failover benefits from FnMut callbacks for stateful reset tracking)
- D8 is independent of D2-D7

### Phase D-Observability: Metrics & Cost (Week 4-5)

| Item | Description | Crate/File | Impact |
|------|-------------|------------|--------|
| D5 | Record actual latency in ResponseOutcome | `clawft-core/src/pipeline/traits.rs`, `src/agent/loop_core.rs` | Real observability data |
| D6 | Thread sender_id for cost recording | `clawft-core/src/pipeline/tiered_router.rs` | Per-user cost attribution |

### Phase D-Transport: MCP Transport (Week 4-5)

| Item | Description | Crate/File | Impact |
|------|-------------|------------|--------|
| D9 | MCP transport concurrency (request-ID multiplexing) | `clawft-services/src/mcp/transport.rs` | Better throughput |

**Cross-element dependency:** D9 blocks 09/M1 (FlowDelegator requires MCP transport concurrency for delegation).

---

## 3. Exit Criteria

### D-Perf
- [x] D1: Multiple tool calls execute concurrently; 3 tools with 100ms simulated latency complete in <200ms (timing test) -- DONE 2026-02-20
- [x] D10: Bootstrap files (`SOUL.md`, `AGENTS.md`, skills) cached with mtime invalidation; second LLM call in same session skips disk -- DONE 2026-02-20
- [x] D11: Skills loader uses `tokio::fs`; no blocking `std::fs` calls on the async executor path -- DONE 2026-02-20

### D-Reliability
- [x] D2: Streaming failover produces clean output (no partial concatenation); first provider fails mid-stream, second provider's complete output is delivered cleanly -- DONE 2026-02-20
- [x] D3: Retry logic uses `ProviderError` enum variants; no string-prefix matching in `is_retryable()` -- DONE 2026-02-20
- [x] D4: Retry policy configurable via config.json (count, backoff delay, eligible status codes) -- DONE 2026-02-20
- [x] D7: `StreamCallback` accepts `FnMut` closures; a stateful token-counting callback compiles and runs -- DONE 2026-02-20
- [x] D8: Message bus has configurable buffer size with backpressure -- DONE 2026-02-20

### D-Observability
- [x] D5: `latency_ms` populated in all `ResponseOutcome` records (no hardcoded zeros) -- DONE 2026-02-20
- [x] D6: `sender_id` propagated from `InboundMessage` through `ChatRequest` and `RoutingDecision` to `CostTracker.update()`; integration test verifies end-to-end flow -- DONE 2026-02-20

### D-Transport
- [x] D9: MCP stdio transport supports concurrent requests via request-ID multiplexing (verified by concurrent call test) -- DONE 2026-02-20

### Regression
- [x] All existing tests pass -- 2,407 tests, 0 failures (2026-02-20)

---

## 4. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| D1: Race conditions when parallel tools write to the same file | Medium | High | 6 | Per-path advisory locks as specified in tech spec; integration test with concurrent writes to same path |
| D2: Partial output already rendered to user before failover detected | Medium | Medium | 4 | StreamFailoverController discards partial output; document that already-rendered text cannot be retracted in streaming UIs |
| D9: Multiplexer complexity in stdio transport | Medium | Medium | 4 | Request-ID correlation map; timeout for orphaned requests; integration test with 10+ concurrent calls |
| D8: Overflow policy misconfiguration causes message loss | Low | High | 4 | Default to backpressure (block sender); document drop-oldest and drop-newest alternatives |
