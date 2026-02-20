# Development Notes: Element 05 - Pipeline & LLM Reliability

**Workstream**: D
**Weeks**: 2-5

---

## Implementation Log

### 2026-02-20: All D-items Complete (11/11)

**Branch**: `sprint/phase-5`
**Commit**: `07ceb05` feat(05): complete Pipeline & LLM Reliability (D1-D11)
**Agent**: Agent-05 (hive-mind worker)
**Validation**: 1,903 tests passing, zero clippy warnings

#### D-Perf (D1, D10, D11)
- **D1**: Parallel tool execution via `futures::join_all`. Tool calls execute concurrently instead of sequentially.
- **D10**: `BootstrapCache` added to `ContextBuilder` with `CachedFile` struct (content + mtime). `load_cached_file()` checks mtime via `tokio::fs::metadata()`.
- **D11**: All `std::fs` calls in `skills_v2.rs` replaced with `tokio::fs`. `load_dir()`, `load_legacy_skill()`, `discover()` all made async. 12 SkillRegistry tests + 3 CLI tests converted to `#[tokio::test]`.

#### D-Reliability (D3, D4, D7, D2, D8)
- **D3**: `ProviderError` enum extended. `is_retryable()` now pattern-matches on variants, no string-prefix matching.
- **D4**: `RetryConfig` struct with `max_retries`, `base_delay_ms`, `max_delay_ms`. Configurable via config.
- **D7**: `StreamCallback` changed from `Box<dyn Fn>` to `Box<dyn FnMut>`, `Sync` dropped.
- **D2**: Streaming failover resets/discards partial output before switching to fallback provider.
- **D8**: `MessageBus` switched from `mpsc::unbounded_channel()` to `mpsc::channel(buffer_size)`. Default capacity 1024. `with_capacity()` constructor added. All `send()` calls now async.

#### D-Observability (D5, D6)
- **D5**: `Instant::now()` timing around LLM calls. `latency_ms` populated in all `ResponseOutcome` records.
- **D6**: `sender_id` threaded from `InboundMessage` through `ChatRequest` to `CostTracker`.

#### D-Transport (D9)
- **D9**: Request-ID multiplexing for MCP stdio transport. Requests sent without holding locks for full cycle. Response router matches by ID.

#### Additional Fixes
- Fixed compile errors from bounded `mpsc::Sender::send()` now returning a `Future` (bootstrap.rs, loop_core.rs)
- Fixed 3 clippy `collapsible_if` warnings (context.rs, tiered_router.rs, middleware.rs)
