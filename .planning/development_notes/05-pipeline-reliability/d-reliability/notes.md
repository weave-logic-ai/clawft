# D-Reliability: Correctness & Resilience -- Notes

**Items**: D3 (structured errors), D4 (retry policy), D7 (FnMut callbacks), D2 (streaming failover), D8 (bounded bus)
**Week**: 3-4

---

## Completed: 2026-02-20

### D3: Structured Error Variants
- Extended `ProviderError` enum with proper variants for all error types
- `is_retryable()` now pattern-matches on variants instead of string prefixes
- All `msg.starts_with("HTTP 5xx")` patterns eliminated

### D4: Configurable Retry Policy
- `RetryConfig` struct: `max_retries`, `base_delay_ms`, `max_delay_ms`
- Wraps provider via `RetryPolicy::new(provider, config)` at construction time
- Retry handled at provider level, not in adapter loop (avoids duplicate retry)

### D7: StreamCallback FnMut
- Changed from `Box<dyn Fn(&str) -> bool + Send + Sync>` to `Box<dyn FnMut(&str) -> bool + Send>`
- Dropped `Sync` requirement since callback is invoked from a single task
- All `complete_stream` call sites updated

### D2: Streaming Failover
- Partial output discarded before switching to fallback provider
- Clean output guaranteed on mid-stream failure

### D8: Bounded Message Bus
- Switched from `mpsc::unbounded_channel()` to `mpsc::channel(1024)` default
- `with_capacity(n)` constructor for custom sizing
- `send()` is now async (returns Future) -- all call sites updated
- Two compile errors fixed in test code (bootstrap.rs:479, loop_core.rs:871)
