# ClawFT Unified Sprint Plan

> Single sprint integrating codebase fixes (from full 9-crate review) with the OpenClaw-parity
> feature roadmap. Items are grouped into workstreams and ordered by dependency within each.

---

## Workstream A: Critical Fixes (Week 1-2)

These bugs and security issues must be resolved before any feature work builds on top.

### A1. Session key round-trip corruption
**File:** `clawft-core/src/session.rs`
**Type:** Bug

`session_path()` replaces `:` with `_`, and `list_sessions()` reverses `_` back to `:`. A key like `"telegram:user_123"` becomes filename `"telegram_user_123.jsonl"` and reloads as `"telegram:user:123"` -- a different key. Any channel or chat ID containing underscores silently corrupts session identity.

**Fix:** Use percent-encoding or a two-character escape sequence instead of 1:1 character substitution.

### A2. Unstable hash function in embeddings
**File:** `clawft-core/src/embeddings/hash_embedder.rs`
**Type:** Bug

Uses `std::collections::hash_map::DefaultHasher`, whose output is explicitly not stable across Rust versions or program runs. Persisted embeddings become silently invalid after a toolchain upgrade, producing incorrect similarity results.

**Fix:** Replace with a stable deterministic hash (`fnv`, `xxhash`, or `ahash` with fixed seed). Include a one-time re-index migration if any embeddings have been persisted.

### A3. Invalid JSON from error formatting
**File:** `clawft-core/src/agent/loop_core.rs`
**Type:** Bug

```rust
format!("{{\"error\": \"{}\"}}", e)
```
If the error message contains a double-quote, the result is malformed JSON sent to the LLM as a tool result.

**Fix:** Use `serde_json::json!({"error": e.to_string()}).to_string()`.

### A4. Plaintext credentials in config structs
**File:** `clawft-types/src/config.rs`
**Type:** Security

`imap_password`, `smtp_password`, `app_secret`, `client_secret`, `claw_token`, and `api_key` are stored as plain `String` fields with no `#[serde(skip_serializing)]` or `Debug` redaction. These appear in serialized JSON, debug output, and audit logs. `clawft-llm` correctly stores only the env var name (`api_key_env`) -- the types crate should follow this pattern.

**Fix:** Store env var names instead of raw secrets. Add custom `Debug` impls that redact sensitive fields. This is prerequisite for the Email and OAuth2 work in Workstream F.

### A5. API key echoed during onboarding
**File:** `clawft-cli/src/commands/onboard.rs`
**Type:** Security

`prompt_provider_config()` reads the API key via `reader.next_line()`, which echoes input to the terminal.

**Fix:** Use `rpassword::read_password()` to suppress terminal echo.

### A6. Incomplete private IP range in SSRF protection
**File:** `clawft-services/src/mcp/middleware.rs`
**Type:** Security

`UrlPolicy` only blocks `172.16.*` but RFC 1918 range `172.16.0.0/12` covers `172.16.*` through `172.31.*`. URLs like `http://172.30.0.1/` bypass the check.

**Fix:** Parse the second octet and check `(16..=31).contains(&n)`.

### A7. No HTTP request timeout on LLM provider client
**File:** `clawft-llm/src/openai_compat.rs`
**Type:** Reliability

`reqwest::Client::new()` is used without a timeout. A provider that never responds blocks the task indefinitely.

**Fix:** Use `reqwest::ClientBuilder` with `.timeout(Duration::from_secs(120))`.

### A8. `unsafe std::env::set_var` in parallel tests
**File:** `clawft-core/src/workspace.rs`
**Type:** Correctness

Tests call `unsafe { std::env::set_var(...) }` under Rust's default parallel test runner. This is UB in Rust 2024 edition.

**Fix:** Use `temp_env` crate or a mutex guard.

---

## Workstream B: Architecture Cleanup (Week 2-4)

Structural improvements that unblock feature work and reduce maintenance burden. These touch shared types and interfaces that later workstreams depend on.

### B1. Unify `Usage` type across crates
**Files:** `clawft-types/src/provider.rs` (`u32`), `clawft-llm/src/types.rs` (`i32`)
**Type:** Refactor

Token counts are `u32` in one crate and `i32` in the other with no conversion function. Token counts are never negative.

**Fix:** Canonical `Usage` type in `clawft-types` with `u32` fields. `clawft-llm` imports and uses it.

### B2. Unify duplicate `LlmMessage` types
**Files:** `clawft-core/src/agent/context.rs`, `clawft-core/src/pipeline/traits.rs`
**Type:** Refactor

Two separate structs with identical fields. TODO comment acknowledges this.

**Fix:** Single type in `clawft-core/src/pipeline/traits.rs`, re-exported. Remove the duplicate from `context.rs`.

### B3. Split oversized files
**Type:** Refactor

| File | Lines | Target Split |
|------|-------|-------------|
| `clawft-types/src/config.rs` | ~1400 | `config/channels.rs`, `config/providers.rs`, `config/policies.rs`, `config/mod.rs` |
| `clawft-core/src/agent/loop_core.rs` | 1645 | Extract tool execution, streaming, message building |
| `clawft-core/src/pipeline/tiered_router.rs` | 1646 | Extract cost tracker, tier selection, classifier |
| `clawft-core/src/pipeline/transport.rs` | 1282 | Extract request building, response parsing |
| `clawft-core/src/tools/registry.rs` | 1242 | Extract individual tool implementations |
| `clawft-core/src/agent/skills_v2.rs` | 1159 | Extract YAML parsing, caching, registry |
| `clawft-core/src/pipeline/llm_adapter.rs` | 1127 | Extract retry logic, config override |
| `clawft-core/src/pipeline/traits.rs` | 1107 | Extract callback types, pipeline stages |
| `clawft-types/src/routing.rs` | ~950 | Extract permissions, delegation |

### B4. Unify cron storage formats
**Files:** `clawft-cli/src/commands/cron.rs` vs `clawft-services/src/cron_service/`
**Type:** Bug / Refactor

CLI uses `CronStore` (flat JSON); `CronService` uses JSONL event sourcing. Incompatible formats -- jobs created via CLI are invisible to the gateway and vice versa.

**Fix:** Unify on JSONL event sourcing. CLI commands drive the `CronService` API.

### B5. Extract shared tool registry builder
**Files:** `clawft-cli/src/commands/agent.rs`, `gateway.rs`, `mcp_server.rs`
**Type:** Refactor

Identical 6-step tool setup block copy-pasted into three files.

**Fix:** Extract `build_tool_registry(config, platform) -> ToolRegistry` into `commands/mod.rs`.

### B6. Extract shared policy types
**Files:** `clawft-services/src/mcp/middleware.rs`, `clawft-tools/`
**Type:** Refactor

`CommandPolicy` and `UrlPolicy` are defined in both crates. Bug fixes must be manually replicated.

**Fix:** Canonical definitions in `clawft-types`. Both crates import from there.

### B7. Deduplicate `ProviderConfig` naming collision
**Files:** `clawft-llm/src/config.rs`, `clawft-types/src/config.rs`
**Type:** Refactor

Both crates define `ProviderConfig` with different semantics (env var name vs plaintext key). Confusing and error-prone.

**Fix:** Rename `clawft-llm`'s to `LlmProviderConfig` or merge into a single type with the env-var-name pattern.

### B8. Consolidate `build_messages` duplication
**File:** `clawft-core/src/agent/context.rs`
**Type:** Refactor

`build_messages` and `build_messages_for_agent` share ~80% code.

**Fix:** Extract shared base with an `extra_instructions: Option<String>` parameter.

### B9. MCP protocol version constant
**Files:** `clawft-services/src/mcp/server.rs`, `mod.rs`
**Type:** Cleanup

`"2025-06-18"` hardcoded in multiple places.

**Fix:** Single `const MCP_PROTOCOL_VERSION` in `mcp/types.rs`.

---

## Workstream C: Plugin & Skill System (Week 3-8)

New crate for plugin infrastructure. Foundation for all extensibility work.

### C1. Define `clawft-plugin` trait crate
**Type:** Feature -- New Crate

Define unified plugin traits: `Tool`, `ChannelAdapter`, `PipelineStage`, `Skill`, `MemoryBackend`, `VoiceHandler`. All new capabilities (email, browser, voice, dev tools) implement these traits rather than modifying core.

**Output:** New crate `clawft-plugin` with trait definitions, manifest schema (JSON/YAML), and `SKILL.md` compatibility types.

### C2. WASM plugin host
**Type:** Feature -- `clawft-wasm` + `clawft-plugin`
**Deps:** C1

Implement WASM plugin host using `wasmtime` + `wit` component model for typed interfaces. Plugins ship as `.wasm` + manifest + optional `SKILL.md`.

**Includes:**
- Complete `WasiFileSystem` (currently all stubs returning `Unsupported`)
- Wire `init()` and `process_message()` in `clawft-wasm`
- Implement `WasiEnvironment` against the actual `Platform::Environment` trait (currently standalone struct with matching signatures but no trait impl)
- WASM HTTP client implementation
- Size budget enforcement: <300KB uncompressed, <120KB gzipped

### C3. Skill Loader (OpenClaw-compatible)
**Type:** Feature -- `clawft-core/src/agent/skills_v2.rs`
**Deps:** C1

Parse `SKILL.md` (YAML frontmatter -> tool description + execution hints), auto-register as WASM or native wrapper. Support `ClawHub` discovery (HTTP index + git clone).

**Prerequisite fix:** Replace the hand-rolled YAML parser in `skills_v2.rs` (item B3/30) with `serde_yaml::from_str` before building on top. Current parser doesn't handle nested structures, multi-line values, or quoted strings.

### C4. Dynamic skill loading & hot-reload
**Type:** Feature
**Deps:** C2, C3

Runtime loading with sandbox isolation. `weft skill install github.com/openclaw/skills/coding-agent` works. Agent can `weft skill create "new skill for X"` and compile to WASM.

### C5. Wire interactive slash-command framework
**File:** `clawft-cli/src/interactive/`
**Type:** Feature / Cleanup
**Deps:** C3

The `builtins` and `registry` modules are dead code -- `agent.rs` implements commands inline with `match`. Wire agent commands through the registry to support dynamic skill-contributed commands.

### C6. Extend MCP server for loaded skills
**Type:** Feature
**Deps:** C3

Auto-expose loaded skills/tools through the MCP server for VS Code/Copilot/Claude Desktop integration.

### C7. Update PluginHost to unify channels + tools
**File:** `clawft-channels/src/host.rs`
**Type:** Refactor
**Deps:** C1

Unify under plugin trait system. Add `SOUL.md`/`AGENTS.md` personality injection into Learner/Assembler pipeline stages.

**Includes:** Make `start_all`/`stop_all` concurrent (currently sequential loops).

---

## Workstream D: Pipeline & LLM Reliability (Week 2-5)

Improvements to the agent loop, LLM transport, and routing pipeline.

### D1. Parallel tool execution
**File:** `clawft-core/src/agent/loop_core.rs`
**Type:** Performance

When the LLM returns multiple tool calls, they execute sequentially in a `for` loop.

**Fix:** Use `futures::future::join_all` for concurrent execution.

### D2. Streaming failover correctness
**File:** `clawft-llm/src/failover.rs`
**Type:** Bug

Mid-stream provider failure sends partial data from the first provider followed by full data from the next, concatenated on the same channel.

**Fix:** Implement "reset stream" that discards partial output before failover. At minimum, document the limitation.

### D3. Structured error variants for retry
**File:** `clawft-llm/src/retry.rs`
**Type:** Refactor

`is_retryable()` uses fragile string prefix matching (`"HTTP 500"`, etc.).

**Fix:** Add `ServerError { status: u16 }` variant to `ProviderError`.

### D4. Configurable retry policy
**File:** `clawft-core/src/pipeline/llm_adapter.rs`
**Type:** Feature

Retry count (3), backoff delay, and eligible status codes are hardcoded.

**Fix:** Make configurable via `ClawftLlmConfig`.

### D5. Record actual latency
**Files:** `clawft-core/src/pipeline/traits.rs`, `src/agent/loop_core.rs`
**Type:** Feature

`ResponseOutcome.latency_ms` is hardcoded to `0` everywhere.

**Fix:** Record wall-clock latency around provider calls. Required for routing feedback and observability.

### D6. Thread `sender_id` for cost recording
**File:** `clawft-core/src/pipeline/tiered_router.rs`
**Type:** Feature

`update()` cannot record costs -- `sender_id` not available on `RoutingDecision`. `CostTracker` infrastructure is built but integration is a no-op.

**Fix:** Thread `sender_id` through the pipeline.

### D7. Change `StreamCallback` to `FnMut`
**File:** `clawft-core/src/pipeline/traits.rs`
**Type:** Fix

`Fn` prevents token accumulators or progress trackers from working as callbacks.

**Fix:** Change to `FnMut`.

### D8. Bounded message bus channels
**File:** `clawft-core/src/bus.rs`
**Type:** Reliability

Uses unbounded channels. No backpressure; fast producer with slow consumer grows memory without limit.

**Fix:** Switch to `bounded_channel` with configurable buffer size.

### D9. MCP transport concurrency
**File:** `clawft-services/src/mcp/transport.rs`
**Type:** Performance

`StdioTransport` serializes concurrent calls completely. `HttpTransport` creates a new `reqwest::Client` per instance.

**Fix:** Implement request-ID multiplexer for stdio. Accept `Arc<reqwest::Client>` for HTTP. Redirect child stderr to log stream.

### D10. Cache skill/agent bootstrap files
**File:** `clawft-core/src/agent/context.rs`
**Type:** Performance

`build_system_prompt` reads files from disk on every LLM call.

**Fix:** Cache content with mtime checking.

### D11. Async file I/O in skills loader
**File:** `clawft-core/src/agent/skills_v2.rs`
**Type:** Performance

`std::fs::read_dir` and `std::fs::read_to_string` block the Tokio executor.

**Fix:** Replace with `tokio::fs` equivalents.

---

## Workstream E: Channel Enhancements (Week 4-8)

Improvements to existing channels and new channel plugins.

### E1. Discord Resume (OP 6)
**File:** `clawft-channels/src/discord/channel.rs`
**Type:** Feature

On reconnect, always re-identifies instead of resuming. `session_id` and `resume_url` are stored but unused. `ResumePayload` is dead code.

**Fix:** Implement Gateway Resume when `session_id` is available.

### E2. Email channel plugin
**Type:** Feature -- New Plugin
**Deps:** C1, A4

IMAP + SMTP via `lettre` + `imap` crates. Gmail OAuth2 via `oauth2` crate. Full read/reply/attach, proactive inbox triage via cron. Implemented as a `clawft-plugin` `ChannelAdapter`.

### E3. WhatsApp channel
**Type:** Feature -- New Plugin
**Deps:** C1

Via official WhatsApp Cloud API wrapper. Implemented as plugin.

### E4. Signal / iMessage bridge
**Type:** Feature -- New Plugin
**Deps:** C1

Via `signal-cli` subprocess or macOS bridge. Implemented as plugin.

### E5. Matrix / IRC channels
**Type:** Feature -- New Plugin
**Deps:** C1

Generic Matrix and IRC channel adapters.

### E6. Enhanced heartbeat / proactive check-in
**File:** `clawft-services/src/heartbeat/`
**Type:** Feature
**Deps:** B4

Enhance existing CronService with "check-in" mode for proactive agent behavior (inbox triage, status summaries).

---

## Workstream F: Software Dev & App Tooling (Week 5-10)

Developer tools and application integrations, all implemented as plugins.

### F1. Git tool plugin
**Type:** Feature -- New Plugin
**Deps:** C1

Via `git2` crate: clone, commit, branch, PR, diff, blame. Integrated as MCP-exposed tool.

### F2. Cargo/build integration
**Type:** Feature -- New Plugin
**Deps:** C1

Build, test, clippy, publish. Integrated as skill with tool calls.

### F3. Code analysis via tree-sitter
**Type:** Feature -- New Plugin
**Deps:** C1

AST-level code parsing and analysis. LSP client for IDE-like code intelligence.

### F4. Browser CDP automation
**Type:** Feature -- New Plugin
**Deps:** C1

Using `chromiumoxide` (async Rust CDP client). Headless/full control: screenshot, form fill, scraping. Sandboxed via separate process.

### F5. Calendar integration
**Type:** Feature -- New Plugin
**Deps:** C1

Google Calendar / Outlook / iCal via APIs. OAuth2 flow.

### F6. Generic REST + OAuth2 helper
**Type:** Feature -- New Plugin
**Deps:** C1

Reusable OAuth2 flow for all API integrations. Used by email, calendar, and future integrations.

### F7. Docker/Podman orchestration tool
**Type:** Feature -- New Plugin
**Deps:** C1

Container lifecycle management from agent context.

### F8. MCP deep IDE integration
**Type:** Feature
**Deps:** C6

Expose agent as VS Code extension backend. Agent edits code live in IDE through MCP.

---

## Workstream G: Voice (Week 6-10)

### G1. STT integration
**Type:** Feature -- New Plugin
**Deps:** C1

`whisper-rs` (local, ONNX) or `vosk-api`. Feature-gated behind `voice`.

### G2. TTS integration
**Type:** Feature -- New Plugin
**Deps:** C1

`piper-rs` (local, high-quality) or `tts` crate + ElevenLabs cloud fallback. Feature-gated behind `voice`.

### G3. VoiceChannel
**Type:** Feature
**Deps:** G1, G2, C1

Audio -> STT -> pipeline -> TTS response. Support microphone/file input, streaming output. Integrates as a `ChannelAdapter` plugin.

---

## Workstream H: Memory & Workspace (Week 4-8)

### H1. Markdown workspace
**Type:** Feature
**Deps:** C1

`~/.clawft/workspace` with `SKILL.md`, `SOUL.md`, `USER.md`, conversation logs. Coexists with JSONL/vector memory. Auto-summarization of long conversations.

### H2. Long-term vector store improvements
**Type:** Feature
**Deps:** A2

After fixing the unstable hash (A2), implement proper vector memory with persistence, auto-indexing, and semantic search.

### H3. Standardize timestamp representations
**Files:** Various across `clawft-types`
**Type:** Refactor

| Type | Current |
|------|---------|
| `InboundMessage.timestamp` | `DateTime<Utc>` |
| `CronJob.created_at_ms` | `i64` (ms) |
| `WorkspaceEntry.last_accessed` | `Option<String>` |

**Fix:** Standardize on `DateTime<Utc>` throughout.

---

## Workstream I: Type Safety & Cleanup (Week 2-6)

Smaller fixes that improve correctness and maintainability.

### I1. `DelegationTarget` serde consistency
**File:** `clawft-types/src/routing.rs`

Serializes as PascalCase (`"Local"`, `"Claude"`) while all other enums use `snake_case`.

**Fix:** Add `#[serde(rename_all = "snake_case")]`.

### I2. String-typed policy modes to enums
**File:** `clawft-types/src/config.rs`

`CommandPolicyConfig::mode` and `RateLimitConfig::strategy` are `String` fields accepting specific values.

**Fix:** Define proper enums.

### I3. `ChatMessage::content` serialization
**File:** `clawft-llm/src/types.rs`

`None` content serializes as `"content": null` which some providers reject.

**Fix:** Add `skip_serializing_if = "Option::is_none"`.

### I4. Job ID collision fix
**File:** `clawft-cli/src/commands/cron.rs`

`generate_job_id()` uses seconds + PID. Same-second collisions.

**Fix:** Use `uuid::Uuid::new_v4()` (already in workspace deps).

### I5. `camelCase` normalizer acronym handling
**File:** `clawft-platform/src/config_loader.rs`

`"HTMLParser"` becomes `"h_t_m_l_parser"`.

**Fix:** Add consecutive-uppercase handling.

### I6. Dead code removal
- `evict_if_needed` in `clawft-core/src/pipeline/rate_limiter.rs` (`#[allow(dead_code)]`)
- `ResumePayload` in `clawft-channels/src/discord/events.rs` (dead until E1)
- Interactive slash-command framework in `clawft-cli/src/interactive/` (dead until C5)
- `--trust-project-skills` and `--intelligent-routing` CLI flags (no-ops)

**Fix:** Remove dead code, or add `// TODO(workstream)` with clear references to the feature work that will use it.

### I7. Fix always-true test assertion
**File:** `clawft-core/src/pipeline/transport.rs`

```rust
assert!(result.is_err() || result.is_ok());
```

**Fix:** Assert the expected specific outcome.

### I8. Share `MockTransport` across crates
**File:** `clawft-services/src/mcp/transport.rs`

`#[cfg(test)]` prevents downstream crates from reusing it.

**Fix:** Expose behind a `test-utils` feature flag.

---

## Workstream J: Documentation & Docs Sync (Week 3-5)

### J1. Fix provider counts
**Files:** `docs/architecture/overview.md`, `docs/guides/providers.md`, `docs/getting-started/quickstart.md`, `docs/reference/config.md`, `clawft-types/src/lib.rs`

Docs say 7-8 providers; actual is 9 (gemini, xai missing). `lib.rs` says 14; `PROVIDERS` has 15.

### J2. Fix assembler truncation description
**File:** `docs/architecture/overview.md`

Says "no truncation at Level 0" but `TokenBudgetAssembler` actively truncates with first+last preservation.

### J3. Fix token budget source reference
**File:** `docs/guides/routing.md`

Says budget comes from `agents.defaults.max_tokens`; code now sources from `max_context_tokens` across routing tiers.

### J4. Document identity bootstrap behavior
**Files:** `docs/guides/skills-and-agents.md` or `docs/guides/configuration.md`

`SOUL.md` and `IDENTITY.md` override default agent identity preamble when placed in workspace root or `.clawft/`. Not documented anywhere.

### J5. Document rate-limit retry behavior
**File:** `docs/guides/providers.md`

3-retry with 500ms minimum wait in `ClawftLlmAdapter` -- undocumented.

### J6. Document CLI log level change
**File:** `docs/reference/cli.md`

Default non-verbose level changed from `info` to `warn`.

### J7. Plugin system documentation
**Deps:** C1-C6

Full docs for the plugin/skill system: architecture, creating plugins, SKILL.md format, ClawHub registry, WASM compilation guide.

---

## Workstream K: UI, Deployment & Community (Week 8-12)

### K1. Web dashboard
**Type:** Feature

`weft ui` via Axum + Leptos. Agent management, conversation view, skill browser.

### K2. Docker images
**Type:** DevOps

Multi-arch images with voice deps. One-click VPS scripts.

### K3. Enhanced sandbox
**Type:** Security
**Deps:** C2

WASM + seccomp/landlock. Per-skill permission system. Audit logs.

### K4. ClawHub skill registry
**Type:** Feature
**Deps:** C3, C4

CLI for publishing/installing skills. Community examples repo. Skill templates.

### K5. Benchmarks vs OpenClaw
**Type:** Testing

Feature parity test suite. Performance comparison (binary size, cold start, memory, throughput).

---

## Cross-Cutting Concerns

These apply across multiple workstreams:

1. **Keep core tiny** -- Heavy deps (`wasmtime`, `whisper-rs`, `chromiumoxide`, `git2`) go in optional plugins behind feature flags. Target: <10 MB base binary, sub-100ms cold start.

2. **Offline capability** -- All local-first where possible. Cloud is always a fallback, never required.

3. **No core forks** -- After Workstream C, all new capabilities are plugins. No more modifying `clawft-core` for features.

4. **Recommended new dependencies** (minimal, Rust-native):
   - `wasmtime` + `wit-bindgen` (plugins)
   - `lettre`, `imap` (email)
   - `chromiumoxide` (browser)
   - `whisper-rs`, `piper-rs` (voice)
   - `git2` (git ops)
   - `oauth2` (auth flows)
   - `tree-sitter` (code analysis)

---

## Timeline Summary

| Weeks | Focus |
|-------|-------|
| 1-2 | **A**: Critical fixes, **I**: Type safety quick wins |
| 2-4 | **B**: Architecture cleanup, **D** (early): Pipeline fixes |
| 3-5 | **J**: Documentation sync, **C** (start): Plugin trait crate |
| 4-8 | **C**: Plugin system, **E**: Channel enhancements, **H**: Memory |
| 5-10 | **D** (complete): Pipeline reliability, **F**: Dev tools & apps |
| 6-10 | **G**: Voice integration |
| 8-12 | **K**: UI, deployment, community |

**MVP milestone (Week 8):** Plugin system working, email channel, 3 ported OpenClaw skills, all critical/high fixes resolved.

**Full vision (Week 12):** Voice-enabled, browser automation, dev tool suite, ClawHub registry, web dashboard, Docker images. ClawFT passes OpenClaw feature parity + outperforms on speed/memory.
