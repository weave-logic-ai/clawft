# D-Observability: Metrics & Cost -- Notes

**Items**: D5 (latency recording), D6 (sender_id threading)
**Week**: 4-5

---

## Completed: 2026-02-20

### D5: Actual Latency Recording
- `Instant::now()` timing around LLM transport calls
- `latency_ms` populated in all `ResponseOutcome` records (both complete() and complete_stream() paths)
- No more hardcoded zeros

### D6: sender_id Threading
- `sender_id` propagated from `InboundMessage` -> `ChatRequest` -> `RoutingDecision` -> `CostTracker`
- Per-user cost recording now functional (no longer a no-op)
