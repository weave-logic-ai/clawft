# Business Requirements: Architecture Cleanup & Pipeline Reliability

> Draft sections for inclusion in `01-business-requirements.md`.
> Covers Workstream B (Architecture Cleanup) and Workstream D (Pipeline & LLM Reliability).

---

## 5d. Architecture Cleanup

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| AC-1 | As a developer integrating with clawft-llm, I want a single canonical `Usage` type with consistent token count fields across all crates so that I do not need to write manual conversion logic or encounter type mismatches at crate boundaries | P1 |
| AC-2 | As a developer working on the agent loop, I want a single `LlmMessage` type rather than two identical structs in different modules so that refactors and bug fixes apply uniformly and I do not accidentally use the wrong import | P1 |
| AC-3 | As a contributor, I want all source files to stay under 500 lines so that code review is manageable, IDE navigation is responsive, and merge conflicts are localized to the subsystem I am changing | P1 |
| AC-4 | As an operator running both the CLI and the gateway, I want cron jobs created via `weft cron add` to be visible in the gateway (and vice versa) so that scheduled tasks are consistent regardless of which entry point created them | P1 |
| AC-5 | As a contributor, I want the tool-registry setup logic to exist in one place so that adding a new tool does not require updating three separate files with identical boilerplate | P2 |
| AC-6 | As a security engineer, I want `CommandPolicy` and `UrlPolicy` defined once in `clawft-types` so that a bug fix to SSRF validation or command filtering is automatically picked up by every crate that enforces those policies | P1 |
| AC-7 | As a developer reading the codebase, I want `ProviderConfig` in `clawft-llm` and `clawft-types` to have distinct names reflecting their distinct semantics so that I do not accidentally pass one where the other is expected | P2 |
| AC-8 | As a developer extending the agent context, I want `build_messages` and `build_messages_for_agent` to share a single parameterized implementation so that changes to message formatting are made once and tested once | P2 |
| AC-9 | As a contributor maintaining the MCP server, I want the protocol version string defined as a single constant so that a version bump is a one-line change rather than a grep-and-replace across multiple files | P3 |

### Feature Summary

| Feature | Description | Affected Crates |
|---------|-------------|-----------------|
| Canonical `Usage` type | Single `Usage` struct (u32 fields) in `clawft-types`, imported by `clawft-llm` | `clawft-types`, `clawft-llm` |
| Unified `LlmMessage` | One struct in `clawft-core/pipeline/traits.rs`, re-exported where needed | `clawft-core` |
| File decomposition | Split 9 files exceeding 500 lines into focused submodules | `clawft-types`, `clawft-core`, `clawft-cli` |
| Unified cron storage | Single JSONL event-sourcing format; CLI commands delegate to `CronService` API | `clawft-cli`, `clawft-services` |
| Shared tool-registry builder | `build_tool_registry(config, platform)` extracted into a shared module | `clawft-cli` |
| Canonical policy types | `CommandPolicy`, `UrlPolicy` defined once in `clawft-types` | `clawft-types`, `clawft-services`, `clawft-tools` |
| Disambiguated ProviderConfig | `clawft-llm` type renamed to `LlmProviderConfig` (or merged with env-var-name pattern) | `clawft-llm`, `clawft-types` |
| Shared `build_messages` base | Single function with `extra_instructions: Option<String>` parameter | `clawft-core` |
| MCP protocol version constant | `const MCP_PROTOCOL_VERSION` in `mcp/types.rs` | `clawft-services` |

### Non-Goals (Architecture Cleanup)

- Full re-architecture of the crate dependency graph (only resolving type conflicts and duplication within the existing structure)
- Merging `clawft-types` and `clawft-llm` into a single crate (they serve different audiences)
- Automated refactoring tooling (manual, reviewer-approved changes only)
- Changing any public API contract that external consumers depend on in this phase (deprecation-first approach)

---

## 5e. Pipeline & LLM Reliability

### User Stories

| ID | Story | Priority |
|----|-------|----------|
| PR-1 | As a user invoking multiple tools in a single prompt, I want those tool calls to execute concurrently so that my total wait time is bounded by the slowest tool rather than the sum of all tools | P1 |
| PR-2 | As a user relying on streaming responses, I want seamless provider failover so that a mid-stream provider failure does not produce garbled or duplicated output in my chat | P1 |
| PR-3 | As an operator, I want retry logic to distinguish between retryable errors (HTTP 500, 502, 503, 429) and permanent errors (HTTP 400, 401, 403) via structured error variants so that the system never wastes time retrying unrecoverable failures | P1 |
| PR-4 | As an admin, I want to configure retry count, backoff delay, and eligible status codes in `clawft.toml` so that I can tune retry behavior for my provider mix without recompiling | P1 |
| PR-5 | As an admin monitoring system performance, I want actual wall-clock latency recorded for every LLM call so that the routing feedback loop and observability dashboards reflect real-world provider response times rather than hardcoded zeros | P1 |
| PR-6 | As an admin tracking per-user spend, I want `sender_id` threaded through the pipeline to the cost tracker so that cost budgets and audit logs attribute spend to the correct user | P1 |
| PR-7 | As a developer building a streaming token counter or progress bar, I want `StreamCallback` to accept `FnMut` so that my callback can accumulate state across invocations without requiring interior mutability workarounds | P2 |
| PR-8 | As an operator, I want the internal message bus to use bounded channels with configurable buffer sizes so that a slow consumer triggers backpressure instead of unbounded memory growth under sustained load | P1 |
| PR-9 | As a user issuing concurrent MCP tool calls, I want stdio-based MCP transports to multiplex requests by ID and HTTP-based transports to share a connection pool so that concurrent tool invocations are efficient and do not serialize unnecessarily | P2 |
| PR-10 | As a user whose agent loads `SOUL.md`, `AGENTS.md`, and skill files, I want those files cached in memory (with mtime invalidation) so that every LLM call does not re-read the filesystem | P2 |
| PR-11 | As a developer, I want the skills loader to use async file I/O (`tokio::fs`) instead of blocking `std::fs` so that directory scans do not stall the Tokio executor and delay other concurrent tasks | P2 |

### Feature Summary

| Feature | Description | Config Layer |
|---------|-------------|-------------|
| Parallel tool execution | `futures::join_all` for concurrent tool calls returned by the LLM | N/A (automatic) |
| Streaming failover | Discard partial output and reset stream before retrying on a fallback provider | Global |
| Structured error variants | `ProviderError::ServerError { status }` replaces string-prefix matching in retry logic | N/A (internal) |
| Configurable retry policy | `retry_count`, `backoff_base_ms`, `retryable_status_codes` in `ClawftLlmConfig` | Global + Project |
| Latency recording | Wall-clock timing around every provider call; populates `ResponseOutcome.latency_ms` | N/A (automatic) |
| Per-user cost attribution | `sender_id` propagated from channel through pipeline to `CostTracker` | Global |
| `FnMut` stream callbacks | `StreamCallback` type alias changed from `Fn` to `FnMut` | N/A (API) |
| Bounded message bus | `bounded_channel` with configurable capacity; backpressure when full | Global |
| MCP transport concurrency | Request-ID multiplexer for stdio; shared `Arc<Client>` for HTTP | Global |
| Bootstrap file caching | In-memory cache for `SOUL.md`, `AGENTS.md`, skill files with mtime checks | N/A (automatic) |
| Async skills I/O | `tokio::fs` replaces blocking `std::fs` in the skills loader | N/A (internal) |

### Non-Goals (Pipeline & LLM Reliability)

- Implementing new LLM providers (provider transport is unchanged; only reliability of existing transports is improved)
- Automatic provider selection or re-ranking (that is the TieredRouter's domain in Section 5c)
- Client-side response caching or deduplication (may be explored in a future phase)
- Guaranteeing exactly-once tool execution (at-least-once with idempotency guidance is the target)
- Real-time alerting or pager integration (observability data is recorded; alerting is out of scope)

---

## Success Criteria Additions

### Architecture Cleanup (Phase 3G+)

- [ ] Single `Usage` type in `clawft-types` used by all crates; no duplicate token-count types remain
- [ ] Single `LlmMessage` type; grep for duplicate struct definitions returns zero matches
- [ ] All source files under 500 lines (measured by `tokei` or `wc -l`, excluding generated code)
- [ ] Cron jobs created via CLI visible in gateway and vice versa (integration test)
- [ ] Tool-registry setup exists in one shared function; no duplicated setup blocks across entry points
- [ ] `CommandPolicy` and `UrlPolicy` each defined exactly once in `clawft-types`
- [ ] No naming collision between `ProviderConfig` types across crates
- [ ] `build_messages` duplication eliminated; single parameterized function with test coverage
- [ ] MCP protocol version defined as a single constant; grep for the raw version string returns only the constant definition

### Pipeline & LLM Reliability (Phase 4)

- [ ] Multiple tool calls from a single LLM response execute concurrently (verified by timing test: 3 tools with 100ms simulated latency each complete in < 200ms total)
- [ ] Streaming failover discards partial output before retrying (verified by test: first provider fails mid-stream, second provider's complete output is delivered cleanly)
- [ ] Retry logic uses structured error variants; no string-prefix matching in `is_retryable()`
- [ ] Retry count, backoff delay, and eligible status codes configurable via `clawft.toml`
- [ ] `ResponseOutcome.latency_ms` populated with real wall-clock values; no hardcoded zeros
- [ ] `sender_id` available in `CostTracker.update()` for per-user cost attribution
- [ ] `StreamCallback` accepts `FnMut` closures; a stateful token-counting callback compiles and runs correctly
- [ ] Message bus uses bounded channels; a slow-consumer test confirms backpressure rather than unbounded memory growth
- [ ] MCP stdio transport supports concurrent requests via request-ID multiplexing (verified by concurrent call test)
- [ ] Bootstrap files (`SOUL.md`, `AGENTS.md`, skills) cached with mtime invalidation; second LLM call in same session does not hit disk (verified by I/O tracing or mock filesystem)
- [ ] Skills loader uses `tokio::fs`; no blocking `std::fs` calls on the async executor path

---

## Risk Register Additions

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| File decomposition (AC-3) introduces import path breakage for downstream consumers | Medium | Medium | 4 | Re-export all public types from the original module path; deprecation warnings on old paths for one release cycle |
| Cron storage migration (AC-4) loses existing scheduled jobs | Low | High | 4 | Write a one-time migration that converts flat JSON to JSONL event-sourcing format; back up before migration |
| Parallel tool execution (PR-1) introduces race conditions in shared mutable state | Medium | High | 6 | Tools must be stateless or use `Arc<Mutex<_>>` for shared state; add concurrent-execution integration tests |
| Bounded channels (PR-8) cause message drops under burst load | Medium | Medium | 4 | Use `send_timeout` with configurable duration; log warnings when channel reaches 80% capacity; provide tuning guidance in docs |
| Streaming failover (PR-2) cannot fully discard partial output already rendered to user | Medium | Medium | 4 | Buffer streamed tokens until a "safe checkpoint" (sentence boundary or tool-call boundary) before rendering; document the tradeoff between latency and failover cleanliness |
| MCP multiplexer (PR-9) complexity increases stdio transport fragility | Medium | Medium | 4 | Implement with thorough request-ID tracking; fall back to serialized execution if multiplexer encounters protocol errors |
