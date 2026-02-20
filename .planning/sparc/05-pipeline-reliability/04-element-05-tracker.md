# Element 05: Pipeline & LLM Reliability -- Execution Tracker

## Summary

- **Total items**: 11 (D1-D11)
- **Workstream**: D (Pipeline & LLM Reliability)
- **Timeline**: Weeks 2-5
- **Status**: Complete (all 11 D-items done)
- **Dependencies**: None (can run parallel with Element 03)
- **Blocks**: Element 09 (D6 sender_id for multi-agent cost tracking), Element 09/M1 (D9 MCP concurrency for FlowDelegator)

---

## Execution Schedule

Element 05 has 11 items (D1-D11) across 4 phases spanning Weeks 2-5.

### Week 2-3 (D-Perf -- 3 items, all independent/parallel)

- [x] D1 -- Parallel tool execution (`futures::join_all`) -- clawft-core/src/agent/loop_core.rs -- DONE 2026-02-20
- [x] D10 -- Cache skill/agent bootstrap files (mtime check) -- clawft-core/src/agent/context.rs -- DONE 2026-02-20
- [x] D11 -- Async file I/O in skills loader (`tokio::fs`) -- clawft-core/src/agent/skills_v2.rs -- DONE 2026-02-20

### Week 3-4 (D-Reliability -- 5 items, D3->D4 sequential; D7->D2 recommended; D8 independent)

- [x] D3 -- Structured error variants for retry -- clawft-llm/src/retry.rs, clawft-types/src/error.rs -- DONE 2026-02-20
- [x] D4 -- Configurable retry policy (depends on D3) -- clawft-core/src/pipeline/llm_adapter.rs -- DONE 2026-02-20
- [x] D7 -- StreamCallback to FnMut -- clawft-core/src/pipeline/traits.rs -- DONE 2026-02-20
- [x] D2 -- Streaming failover correctness (reset stream; D7 recommended first) -- clawft-llm/src/failover.rs -- DONE 2026-02-20
- [x] D8 -- Bounded message bus channels -- clawft-core/src/bus.rs -- DONE 2026-02-20

### Week 4-5 (D-Observability -- 2 items, independent)

- [x] D5 -- Record actual latency in ResponseOutcome -- clawft-core/src/pipeline/traits.rs, src/agent/loop_core.rs -- DONE 2026-02-20
- [x] D6 -- Thread sender_id for cost recording -- clawft-core/src/pipeline/tiered_router.rs -- DONE 2026-02-20

### Week 4-5 (D-Transport -- 1 item, blocks Element 09/M1)

- [x] D9 -- MCP transport concurrency (request-ID multiplexing) -- clawft-services/src/mcp/transport.rs -- DONE 2026-02-20

---

## Per-Item Status Table

| Item | Description | Priority | Week | Crate(s) | Status | Owner | Branch | Key Deliverable |
|------|-------------|----------|------|----------|--------|-------|--------|-----------------|
| D1 | Parallel tool execution (`join_all`) | P1 | 2-3 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | Concurrent tool calls via join_all |
| D2 | Streaming failover correctness | P1 | 3-4 | clawft-llm | **Done** | Agent-05 | sprint/phase-5 | Stream reset on mid-stream fail |
| D3 | Structured error variants | P1 | 3-4 | clawft-llm, clawft-types | **Done** | Agent-05 | sprint/phase-5 | ProviderError enum; no string-prefix matching |
| D4 | Configurable retry policy | P1 | 3-4 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | RetryConfig in config |
| D5 | Record actual latency | P2 | 4-5 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | Real latency_ms in ResponseOutcome |
| D6 | Thread sender_id for cost | P2 | 4-5 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | End-to-end sender_id propagation |
| D7 | StreamCallback to FnMut | P1 | 3-4 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | FnMut type alias; stateful callbacks |
| D8 | Bounded message bus | P1 | 3-4 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | Bounded channels with backpressure |
| D9 | MCP transport concurrency | P1 | 4-5 | clawft-services | **Done** | Agent-05 | sprint/phase-5 | Request-ID multiplexing for stdio |
| D10 | Cache bootstrap files | P2 | 2-3 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | mtime-based cache |
| D11 | Async file I/O in skills | P2 | 2-3 | clawft-core | **Done** | Agent-05 | sprint/phase-5 | tokio::fs; zero blocking std::fs |

---

## Internal Dependency Graph

```
D3 (structured errors) -------> D4 (configurable retry)
  Retry policy should use structured ProviderError variants,
  not string-prefix matching. D3 MUST complete before D4.

D7 (FnMut callbacks) --------> D2 (streaming failover) [recommended]
  Streaming failover benefits from FnMut callbacks for
  stateful reset tracking. D7 before D2 is recommended
  but not strictly required.

D1, D5, D6, D8, D9, D10, D11 are fully independent.
  Can be implemented in any order or in parallel.

D1, D10, D11 (D-Perf) are all independent of each other.
  Can be worked in parallel by multiple developers.
```

---

## Cross-Element Dependencies

| Source (Element 05) | Target (Other Element) | Type | Impact |
|---------------------|------------------------|------|--------|
| D6 (sender_id) | Element 09 (multi-agent cost tracking) | Blocks | sender_id needed for per-agent cost attribution in L4 planning strategies |
| D9 (MCP concurrency) | Element 09/M1 (FlowDelegator) | Blocks | FlowDelegator requires MCP transport concurrency for delegation |
| D11 (async file I/O) | Element 04/C3 (async skill loader) | Overlaps | Both convert std::fs to tokio::fs in skills_v2.rs; coordinate to avoid conflict |
| D6 (sender_id) | Element 06 (channels) | Implicit | Channels produce InboundMessage which carries sender_id; contract should be stable before new channels |
| D8 (bounded bus) | Element 06 (channels) | Implicit | New channels in Element 06 must use bounded bus API; D8 should stabilize before E-Enterprise phase |

---

## Exit Criteria

### D-Perf

- [ ] **D1**: Multiple tool calls execute concurrently; 3 tools with 100ms simulated latency complete in <200ms (timing test)
- [ ] **D10**: Bootstrap files (`SOUL.md`, `AGENTS.md`, skills) cached with mtime invalidation; second LLM call in same session skips disk
- [ ] **D11**: Skills loader uses `tokio::fs`; no blocking `std::fs` calls on the async executor path

### D-Reliability

- [ ] **D2**: Streaming failover produces clean output (no partial concatenation); first provider fails mid-stream, second provider's complete output is delivered cleanly
- [ ] **D3**: Retry logic uses `ProviderError` enum variants; no string-prefix matching in `is_retryable()`
- [ ] **D4**: Retry policy configurable via config.json (count, backoff delay, eligible status codes)
- [ ] **D7**: `StreamCallback` accepts `FnMut` closures; a stateful token-counting callback compiles and runs
- [ ] **D8**: Message bus has configurable buffer size with backpressure

### D-Observability

- [ ] **D5**: `latency_ms` populated in all `ResponseOutcome` records (no hardcoded zeros)
- [ ] **D6**: `sender_id` propagated from `InboundMessage` through `ChatRequest` and `RoutingDecision` to `CostTracker.update()`; integration test verifies end-to-end flow

### D-Transport

- [ ] **D9**: MCP stdio transport supports concurrent requests via request-ID multiplexing (verified by concurrent call test)

### Regression

- [ ] All existing tests pass

---

## Review Gates

| Gate | Scope | Requirement |
|------|-------|-------------|
| D-Perf Review | D1, D10, D11 | Standard code review; D1 race condition tests verified |
| D-Reliability Review | D3, D4, D7, D2, D8 | Code review; verify D3->D4 ordering was followed; streaming failover test coverage |
| D-Observability Review | D5, D6 | Code review; D6 end-to-end integration test required |
| D-Transport Review | D9 | Code review; concurrent MCP test with 10+ simultaneous requests |
| Security Review | D1 (file locks), D8 (overflow) | Verify per-path advisory locks (D1) and backpressure default (D8) |

---

## Risk Register

Scoring: Likelihood (Low=1, Medium=2, High=3) x Impact (Low=1, Medium=2, High=3, Critical=4)

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| D1: Race conditions when parallel tools write to the same file | Medium | High | 6 | Per-path advisory locks as specified in tech spec; integration test with concurrent writes to same path |
| D2: Partial output already rendered to user before failover detected | Medium | Medium | 4 | StreamFailoverController discards partial output; document that already-rendered text cannot be retracted in streaming UIs |
| D9: Multiplexer complexity in stdio transport | Medium | Medium | 4 | Request-ID correlation map; timeout for orphaned requests; integration test with 10+ concurrent calls |
| D8: Overflow policy misconfiguration causes message loss | Low | High | 4 | Default to backpressure (block sender); document drop-oldest and drop-newest alternatives |
| D3->D4 ordering violation causes string-matching regression | Low | Medium | 2 | D3 must merge before D4 branch created; CI check for string-prefix matching in retry logic |

---

## Progress Summary

| Phase | Items | Pending | In Progress | Completed | % Done |
|-------|-------|---------|-------------|-----------|--------|
| D-Perf (D1, D10, D11) | 3 | 0 | 0 | 3 | 100% |
| D-Reliability (D3, D4, D7, D2, D8) | 5 | 0 | 0 | 5 | 100% |
| D-Observability (D5, D6) | 2 | 0 | 0 | 2 | 100% |
| D-Transport (D9) | 1 | 0 | 0 | 1 | 100% |
| **Total** | **11** | **0** | **0** | **11** | **100%** |
