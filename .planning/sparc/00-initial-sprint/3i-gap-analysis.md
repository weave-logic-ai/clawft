# Phase 3I: Gap Analysis and Final Sprint

**Date**: 2026-02-17
**Phase**: 3I (Inspect -- gap analysis before phase gate)
**Status**: PLANNING
**Baseline**: 1,054 tests, 0 clippy warnings, 9 crates, ~31,359 LOC Rust

---

## Executive Summary

This document systematically audits every requirement from the planning documents (01-05)
against the current clawft implementation. Gaps are categorized by priority:

- **P0 (Critical)**: Blocks MVP usability. Must fix before Phase 3 gate.
- **P1 (Important)**: Significantly degrades functionality. Should fix during 3I sprint.
- **P2 (Nice-to-have)**: Polish items. Can defer to Phase 4 with documented acceptance.

**Totals**: 8 P0 gaps, 14 P1 gaps, 12 P2 gaps (34 active; 3 gaps resolved pre-sprint)

---

## 1. Requirements vs Implementation Audit

### 1.1 Business Requirements (01-business-requirements.md)

#### Primary Goals

| ID | Goal | Status | Gap | Priority |
|----|------|--------|-----|----------|
| G1 | Single static binary | PARTIAL | `cargo build --release` produces binary, but no end-to-end validation that it runs standalone without runtime deps. CI builds exist but have not been run against real GitHub infrastructure. | P1 |
| G2 | Run on constrained devices (RSS < 10 MB ARM64) | UNTESTED | No ARM64 testing done. Cross-compile scripts exist but never validated. Memory benchmarks are native x86_64 only. | P2 |
| G3 | Sub-second cold start (< 500ms) | PASS | 3.5ms measured. | -- |
| G4 | Feature-gated compilation | PASS | Feature flags exist for channels, tools, services, WASM. `--no-default-features` builds pass. | -- |
| G5 | Config compatibility (reads ~/.nanobot/config.json) | PASS | Fallback chain implemented and tested. | -- |
| G6 | Pluggable channel architecture | PASS | Channel trait, ChannelHost, ChannelFactory, PluginHost all implemented with Telegram/Slack/Discord. | -- |
| G7 | RVF-powered intelligence | NOT STARTED | No RVF crates integrated. IntelligentRouter exists as a clawft-native implementation (vector store + hash embeddings) but does NOT use rvf-runtime, rvf-types, rvf-index, or any ruvector crates. | P2 |

#### Secondary Goals

| ID | Goal | Status | Gap |Priority |
|----|------|--------|-----|---------|
| G8 | WASM core extraction | PARTIAL | clawft-wasm compiles to wasm32-wasip1. HTTP/FS are stubs. No wasip2 validation yet (pending 3C Rust upgrade verification). | P2 |
| G9 | Cross-compilation | PARTIAL | CI workflows and cross-compile scripts exist. Never actually run on GitHub Actions. | P1 |
| G10 | Embeddable library | PASS | clawft-core is a library crate with clean public API. | -- |
| G11 | Drop-in replacement CLI | PARTIAL | See CLI gap analysis below (section 1.5). | P1 |

#### Success Criteria -- MVP (Phase 1: Warp)

| Criterion | Status | Gap ID |
|-----------|--------|--------|
| `cargo build --release` produces single `weft` binary | PASS | -- |
| `weft gateway` starts and processes Telegram messages | IMPLEMENTED but UNTESTED end-to-end | GAP-01 |
| `weft agent -m "hello"` works in CLI | IMPLEMENTED but requires configured LLM provider | GAP-02 |
| Reads existing config.json | PASS | -- |
| All file tools work | PASS | -- |
| Shell exec tool works | PASS | -- |
| Web search and fetch tools work | PARTIAL | GAP-03 |
| Session persistence works | PASS | -- |
| Memory consolidation works | PARTIAL | GAP-04 |
| Channel plugin API documented and stable | PASS | -- |
| Binary size < 10 MB | UNTESTED in CI | GAP-05 |
| RSS idle < 15 MB | UNTESTED in CI | P2 |

#### Success Criteria -- Full Parity (Phase 2: Weft)

| Criterion | Status | Gap ID |
|-----------|--------|--------|
| Telegram, Slack, Discord channels all functional | IMPLEMENTED, not e2e tested | -- |
| All tools functional including MCP | PARTIAL | GAP-06 |
| Cron scheduling works | PASS | -- |
| Heartbeat service works | PASS | -- |
| ruvector model routing integrated | NOT STARTED | P2 (deferred) |
| Vector-based memory search operational | PARTIAL (hash-embeddings only) | GAP-07 |
| `weft channels status` shows all channels | PASS | -- |
| `weft cron list/add/remove/enable/run` works | PASS | -- |

---

### 1.2 Technical Requirements (02-technical-requirements.md)

#### Workspace Architecture

| Component | Spec | Status | Gap ID |
|-----------|------|--------|--------|
| clawft-types | Zero-dep core types | PASS | -- |
| clawft-platform | Platform abstraction | PASS | -- |
| clawft-core | Agent loop, pipeline, memory, session | PASS | -- |
| clawft-tools | Built-in tools | PASS (all 10 tools implemented) | -- |
| clawft-channels | Channel plugins | PASS (3 channels) | -- |
| clawft-services | Cron, heartbeat, MCP | PASS | -- |
| clawft-cli | Binary `weft` | PASS | -- |
| clawft-wasm | WASM entrypoint | PARTIAL (stubs) | -- |
| clawft-llm | Standalone LLM library | **IN-WORKSPACE** | GAP-08 |

**GAP-08**: clawft-llm is a workspace member (`crates/clawft-llm/`), not a standalone
external library as specified. The spec says it should live in `repos/clawft-llm/` as an
independent crate publishable to crates.io. It currently has no barni deps which is correct,
but it is not extracted to its own repository.

#### Crate Dependency Graph

| Specified Dependency | Implemented? | Notes |
|---------------------|-------------|-------|
| clawft-types: zero deps beyond serde | PASS | |
| clawft-platform depends on types | PASS | |
| clawft-core depends on types, platform | PASS | Does NOT depend on clawft-llm directly (uses LlmProvider trait) |
| clawft-tools depends on types, platform, core | PASS | |
| clawft-channels depends on types, platform, core | PASS (depends on types only, not core) | Slightly different from spec but correct |
| clawft-services depends on types, platform, core | PASS (depends on types, platform, tokio) | |
| clawft-cli depends on all above | PASS | |

#### Platform Traits

| Trait | Specified | Implemented? | Gap |
|-------|-----------|-------------|-----|
| HttpClient.post_json | Yes | Yes | -- |
| HttpClient.get | Yes | Yes | -- |
| HttpClient.post_form | Yes | **NO** | GAP-09 |
| FileSystem.read_to_string | Yes | Yes | -- |
| FileSystem.write_string | Yes | Yes | -- |
| FileSystem.append_string | Yes | Yes | -- |
| FileSystem.exists | Yes | Yes | -- |
| FileSystem.list_dir | Yes | Yes | -- |
| FileSystem.create_dir_all | Yes | Yes | -- |
| FileSystem.remove_file | Yes | Yes | -- |
| FileSystem.glob | Yes | **NO** | GAP-10 |
| Environment.var | Yes | Yes | -- |
| Environment.home_dir | Yes | Yes | -- |
| Environment.current_dir | Yes | Yes | -- |
| Environment.now | Yes | Yes | -- |
| Environment.platform | Yes | Yes | -- |
| ProcessSpawner.exec | Yes | Yes | -- |

**GAP-09**: `HttpClient::post_form` (multipart form) is not implemented. This is needed
for Telegram file upload (sendDocument, sendPhoto, sendVoice APIs). Currently Telegram
channel does text-only responses.

**GAP-10**: `FileSystem::glob` is not implemented. The spec calls for glob pattern
matching on the filesystem. The `list_dir` tool does exist but does not support glob
patterns.

#### Pipeline Architecture (6-stage)

| Stage | Trait | Level 0 Impl | Status |
|-------|-------|-------------|--------|
| 1. TaskClassifier | classify() | KeywordClassifier | PASS |
| 2. ModelRouter | route(), update() | StaticRouter | PASS |
| 3. ContextAssembler | assemble() | TokenBudgetAssembler | PASS |
| 4. LlmTransport | complete(), complete_stream() | OpenAiCompatTransport | PARTIAL |
| 5. QualityScorer | score() | NoopScorer | PASS |
| 6. LearningBackend | record(), adapt() | NoopLearner | PASS |

**GAP-11**: `LlmTransport::complete_stream()` is defined in the trait but the
`OpenAiCompatTransport` does NOT implement streaming. The clawft-llm `ChatRequest`
has a `stream` field but the `OpenAiCompatProvider::complete()` method always returns
a full response. SSE streaming is not implemented.

#### Tool Specifications

| Tool | Feature Flag | Specified | Implemented? | Gap |
|------|-------------|-----------|-------------|-----|
| read_file | always | Yes | PASS | -- |
| write_file | always | Yes | PASS | -- |
| edit_file | always | Yes | PASS | -- |
| list_dir | always | Yes | PASS | -- |
| exec | tool-exec / native-exec | Yes | PASS (with CommandPolicy) | -- |
| web_search | tool-web | Yes | PASS (needs endpoint config) | -- |
| web_fetch | tool-web | Yes | PASS (with UrlPolicy SSRF protection) | -- |
| message | always | Yes | PASS | -- |
| spawn | tool-spawn | Yes | PASS (with CommandPolicy) | -- |
| cron | tool-cron | N/A (service, not tool) | N/A | -- |
| MCP client | tool-mcp | Yes (as service) | PARTIAL | GAP-06 |
| memory_read | -- | Not in spec | IMPLEMENTED (bonus) | -- |
| memory_write | -- | Not in spec | IMPLEMENTED (bonus) | -- |

**GAP-06**: MCP client has full JSON-RPC implementation with stdio and HTTP transports.
However, MCP tools are NOT registered in the ToolRegistry automatically. There is no
`weft` CLI command or config option to connect to MCP servers and expose their tools.
The `mcp_tools.rs` file exists in clawft-cli but its integration with the agent loop
is unclear. MCP server configs from `tools.mcp_servers` in config.json are parsed but
not acted upon during bootstrap.

> **MCP Architecture Resolution (2026-02-17)**: The MCP infrastructure gap (duplicate server implementations) has been resolved by the pluggable ToolProvider architecture agreed in P0 #7. McpServerShell + middleware pipeline in clawft-services/src/mcp/. 3H Session 2 delivers shared infra; 3F-RVF Sprint 4 implements RvfToolProvider. No additional gaps identified for MCP.

#### CLI Commands

| Command | Specified | Implemented? | Gap |
|---------|-----------|-------------|-----|
| `weft onboard` | Yes | **NO** | GAP-12 |
| `weft gateway` | Yes | PASS | -- |
| `weft agent [-m "msg"]` | Yes | PASS | -- |
| `weft status` | Yes | PASS | -- |
| `weft channels status` | Yes | PASS | -- |
| `weft cron {list,add,remove,enable,run}` | Yes | PASS (+ disable) | -- |
| `weft sessions {list,inspect,delete}` | Not in original spec | IMPLEMENTED (bonus) | -- |
| `weft memory {show,history,search}` | Not in original spec | IMPLEMENTED (bonus) | -- |
| `weft config {show,section}` | Not in original spec | IMPLEMENTED (bonus) | -- |
| `weft completions` | Not in original spec | IMPLEMENTED (bonus) | -- |
| `weft provider login` | Yes (Codex OAuth) | **NO** | GAP-13 |

**GAP-12**: `weft onboard` -- the initialization/setup wizard is specified but not
implemented. This command should: create `~/.clawft/` directory, generate default
config.json, prompt for API keys, create workspace directory structure.

**GAP-13**: `weft provider login` -- Codex OAuth flow is specified in the Phase 2
development guide but not implemented. The provider types have OAuth-related config
fields but no CLI command exists.

---

### 1.3 Development Guide (03-development-guide.md)

#### Phase 1 Milestone Status

| Item | Status | Notes |
|------|--------|-------|
| Single binary builds | PASS | |
| Config compatibility | PASS | |
| `weft agent -m "..."` works | NEEDS VALIDATION | GAP-02: requires LLM provider configured |
| `weft gateway` starts Telegram | IMPLEMENTED | Not e2e tested with real bot |
| Telegram receives and responds | UNTESTED | GAP-01 |
| File tools work | PASS | |
| Shell exec works | PASS | |
| Sessions persist | PASS | |
| Memory consolidation runs | PARTIAL | GAP-04 |
| Plugin API stable and documented | PASS | |

**GAP-04**: Memory consolidation is specified to use LLM-based summarization
(condense MEMORY.md when it exceeds a threshold). The `MemoryStore` exists with
read/write/append but there is no consolidation loop. The agent loop does not
trigger consolidation. The Python nanobot has a background task that summarizes
memory when it exceeds a token count -- this is missing.

#### Phase 2 Milestone Status

| Item | Status | Notes |
|------|--------|-------|
| All 3 channel plugins | PASS | |
| RVF model routing | NOT STARTED | P2 deferred |
| RVF vector memory | NOT STARTED (hash embeddings instead) | P2 deferred |
| Cron jobs | PASS | |
| Heartbeat | PASS | |
| MCP servers connect | PARTIAL | GAP-06 |
| `weft channels status` | PASS | |
| `weft cron list` | PASS | |
| Codex OAuth | NOT STARTED | GAP-13 |

---

### 1.4 RVF Integration (04-rvf-integration.md)

All RVF integration is NOT STARTED. clawft-core has its own `VectorStore` (brute-force
cosine similarity), `HashEmbedder` (SimHash), `IntelligentRouter` (3-tier complexity
scoring), and `SessionIndexer` (conversation turn indexing). These are functional
Level 0 implementations that match the "no ruvector" fallback described in the spec.

| RVF Integration Point | Status | Priority |
|----------------------|--------|----------|
| rvf-runtime as workspace dep | NOT STARTED | P2 |
| MemoryStore with RvfVectorStore | NOT STARTED (VectorStore exists) | P2 |
| SessionManager with rvf index | NOT STARTED (SessionIndexer exists) | P2 |
| IntelligentRouter with POLICY_KERNEL | NOT STARTED (IntelligentRouter exists) | P2 |
| WitnessLog with rvf WITNESS | NOT STARTED | P2 |
| WASM rvf microkernel | NOT STARTED | P2 |
| Embedding via LLM API | NOT STARTED (HashEmbedder exists) | P2 |

**Assessment**: The Level 0 fallback implementations are all present and functional.
RVF integration is a Phase 4+ concern. No P0/P1 gaps here.

---

### 1.5 ruvector Crates (05-ruvector-crates.md)

All ruvector crate integration is NOT STARTED per the plan (Phase 2-3 scope,
feature-gated). The pluggable pipeline architecture IS in place with all 6 trait
definitions and Level 0 implementations. This is the correct foundation.

No P0/P1 gaps. All ruvector work is P2 deferred.

---

## 2. Gap Inventory

### P0 -- Critical (blocks MVP usability)

| ID | Description | Crate | LOE | Notes |
|----|-------------|-------|-----|-------|
| ~~GAP-01~~ | ~~End-to-end agent validation~~ | ~~clawft-cli, clawft-core~~ | ~~2h~~ | **RESOLVED IN CODE (2026-02-17)** -- `enable_live_llm()` is called unconditionally from both `agent.rs` and `gateway.rs`. Bootstrap wires providers from config. Verified by 3I reviewer. |
| ~~GAP-02~~ | ~~LLM provider bootstrap is broken~~ | ~~clawft-core/bootstrap.rs~~ | ~~2h~~ | **RESOLVED IN CODE (2026-02-17)** -- `AppContext::new()` now calls `enable_live_llm()` when config has providers. Verified by 3I reviewer. |
| GAP-03 | Web search tool requires `tools.web_search.endpoint` config but the config type uses `web_search_api_key` (Brave API key). The tool expects an `endpoint` URL, not an API key. Mismatch between config schema and tool implementation. | clawft-tools, clawft-types | 1h | Align config field names with tool expectations. Either make the tool construct the Brave API endpoint from the key, or add an `endpoint` config field. |
| ~~GAP-06~~ | ~~MCP integration incomplete~~ | ~~clawft-core, clawft-cli~~ | ~~4h~~ | **RESOLVED IN CODE (2026-02-17)** -- MCP servers are parsed from config, transports connected, tools listed and registered in ToolRegistry. `mcp_tools.rs` in clawft-cli wires MCP tool execution via McpClient. Verified by 3I reviewer. |
| GAP-11 | SSE streaming not implemented. `LlmTransport::complete_stream()` is defined but not functional. The `OpenAiCompatProvider` always sends non-streaming requests. For real LLM usage, streaming is essential for UX (partial responses). | clawft-llm, clawft-core | 6h | Need SSE parser, StreamChunk types, and wiring through the pipeline to the CLI/channel output. |
| GAP-12 | `weft onboard` command missing. No way for new users to initialize config and workspace. | clawft-cli | 3h | Create wizard: mkdir ~/.clawft, generate config.json template, prompt for API keys interactively, create workspace dirs. |
| GAP-14 | No JSON repair for malformed LLM output. Spec (02-technical-requirements.md) lists "JSON repair" as a clawft-core component (~100 lines). Not implemented. LLMs frequently return malformed JSON in tool calls. | clawft-core | 2h | Port json-repair logic: strip markdown fences, fix trailing commas, fix unquoted keys, handle truncated JSON. |
| GAP-15 | `memory_tool` search is paragraph-level substring match only. The `MemoryStore` in clawft-core supports MEMORY.md and HISTORY.md but the memory tool exposed to the LLM does not use the VectorStore or SessionIndexer for search. | clawft-tools | 2h | Wire the VectorStore-based search (when vector-memory feature is active) into the memory_read tool's search functionality. |
| ~~GAP-16~~ | ~~`unimplemented!()` in session.rs~~ | ~~clawft-core~~ | ~~1h~~ | **DEMOTED TO P2** -- Per 3I reviewer: the `unimplemented!()` is in test-only mock code, not production path. Moved to P2 backlog. |
| GAP-17 | clawft-llm provider does not parse tool calls from response. The `OpenAiCompatProvider::complete()` returns raw `ChatResponse` but the pipeline transport (`OpenAiCompatTransport`) must convert tool_calls from the JSON response into `ContentBlock::ToolUse` variants. Verify this conversion is correct. | clawft-core, clawft-llm | 2h | Audit the transport.rs response parsing. Ensure tool call IDs, function names, and arguments are correctly extracted. |
| GAP-18 | No circuit breaker / retry logic in clawft-llm. Spec says clawft-llm should have `FailoverController` with 4 strategies and a `CircuitBreaker`. Neither exists. Single provider failure = total failure. | clawft-llm | 4h | Implement at minimum: exponential backoff retry, provider failover (try next provider on error). CircuitBreaker can be P1. |
| GAP-19 | Agent loop tool execution does not truncate oversized results. `MAX_TOOL_RESULT_BYTES` constant exists (64KB) but verify it is enforced in the tool execution loop. Oversized tool results will blow the context window. | clawft-core | 1h | Verify truncation is applied. If not, add it. |

### P1 -- Important (significant functionality gaps)

| ID | Description | Crate | LOE | Notes |
|----|-------------|-------|-----|-------|
| GAP-04 | Memory consolidation not implemented. Spec says MemoryStore should consolidate MEMORY.md when it exceeds token threshold using LLM summarization. No consolidation loop exists. | clawft-core | 4h | Implement background task: check MEMORY.md size, if > threshold, send to LLM for summarization, replace content. |
| GAP-05 | Binary size assertion never validated in real CI. Scripts exist but have not run on GitHub Actions. No CI run has ever verified size < 15 MB. | CI | 1h | Need to actually push to GitHub and validate CI works, or run size-check.sh locally against release build. |
| GAP-08 | clawft-llm is a workspace member, not a standalone repo. Spec says it should be an independent crate at `repos/clawft-llm/` publishable to crates.io. | project structure | 2h | Move to separate directory. Update Cargo.toml to use path dep during development. Low urgency but blocks publishing. |
| GAP-09 | `HttpClient::post_form` (multipart) not in platform trait. Telegram file uploads (photos, documents, voice) require multipart POST. | clawft-platform | 3h | Add `post_form()` to HttpClient trait. Implement for NativePlatform (reqwest multipart). Needed for Telegram media support. |
| GAP-10 | `FileSystem::glob` not in platform trait. Spec lists it, implementation missing. | clawft-platform | 2h | Add `glob()` method. Implement using `glob` crate for native platform. |
| GAP-13 | `weft provider login` (Codex OAuth) not implemented. | clawft-cli | 4h | Implement OAuth2 PKCE flow for Codex provider. Low urgency if Codex is not a target provider. |
| GAP-20 | Telegram channel does not support voice messages (transcription). Python nanobot has voice message handling. | clawft-channels | 3h | Parse voice message from Telegram update, download audio file, send for transcription (or pass through). Requires GAP-09 (multipart). |
| GAP-21 | Discord Ed25519 signature verification. Spec says Discord channel should verify webhook signatures. Check if implemented. | clawft-channels | 2h | Verify discord/channel.rs has signature verification. If using Gateway WebSocket (not webhooks), this may not be needed. |
| GAP-22 | Slack HMAC signature verification. Spec says Slack channel should verify request signatures. | clawft-channels | 1h | Verify slack/signature.rs is wired into the channel's message processing. File exists, need to confirm it's called. |
| GAP-23 | No error recovery/backoff for channel plugins. Spec says plugin host should "handle plugin crashes with backoff retry". PluginHost exists but verify it has retry logic. | clawft-channels | 2h | Audit host.rs for crash recovery and backoff. Add if missing. |
| GAP-24 | Provider model aliasing not implemented. Spec says `ProviderEntry.model_aliases` should map friendly names to model IDs. Config field exists but clawft-llm router does not use it. | clawft-llm | 2h | Wire model_aliases from config into the router's model resolution. |
| GAP-25 | `docs/benchmarks/results.md` missing. Exit criteria review flagged this. Benchmark data exists in round3-summary but no standalone document. | docs | 1h | Create the file from existing benchmark data. |
| GAP-26 | Workspace file structure not auto-created. When MEMORY.md/HISTORY.md paths don't exist, `MemoryStore` defaults to `.clawft` path but does not create parent directories on first write. | clawft-core | 1h | Ensure `create_dir_all` is called before first write to MEMORY.md/HISTORY.md. |
| GAP-27 | Config environment variable overlay. Spec says env vars should overlay config.json values (e.g., `CLAWFT_AGENTS_MODEL=gpt-4o` overrides `agents.model`). Not implemented. | clawft-platform | 3h | Add env var overlay in config_loader after JSON parse. Convention: `CLAWFT_<SECTION>_<KEY>`. |

### P2 -- Nice-to-have (defer to Phase 4 with acceptance)

| ID | Description | Crate | LOE | Notes |
|----|-------------|-------|-----|-------|
| GAP-07 | Vector memory search uses hash embeddings, not real semantic embeddings. API-based embeddings (Option A from spec) not implemented. | clawft-core | 4h | Add `ApiEmbedder` that calls provider's embedding endpoint. Wire into VectorStore. |
| GAP-28 | WASM stubs (HTTP, FS) not functional. Deferred per Phase 3 exit criteria. | clawft-wasm | 8h | Implement real WASI preview2 HTTP and FS. Blocked on ecosystem maturity. |
| GAP-29 | No `readability` HTML-to-text extraction in web_fetch. Spec says web_fetch should use readability to extract main content. Returns raw HTML/text instead. | clawft-tools | 3h | Add readability crate or simple HTML stripping. |
| GAP-30 | No SubagentManager. Spec says clawft-core should have background task spawning for subagents. Not implemented. | clawft-core | 4h | Implement SubagentManager with tokio::spawn. Allow agent to delegate tasks. |
| GAP-31 | RVF integration (all points from 04-rvf-integration.md). | clawft-core | 20h+ | Phase 4 scope. Level 0 fallbacks are in place. |
| GAP-32 | ruvector crate integration (all from 05-ruvector-crates.md). | clawft-core | 40h+ | Phase 4+ scope. |
| GAP-33 | clawft-llm missing native Anthropic/Bedrock/Gemini providers. Only OpenAI-compat implemented. Spec says 4 native providers. | clawft-llm | 8h | Anthropic Messages API has different wire format. Add native provider implementations. OpenAI-compat covers most cases via proxy endpoints. |
| GAP-34 | WASM wasip2 target validation. Blocked on Rust 1.93 upgrade verification (3C stream). | clawft-wasm | 2h | Run `cargo check --target wasm32-wasip2 -p clawft-wasm` after 3C completes. |
| GAP-35 | Cost tracking in clawft-llm. Spec says `UsageTracker + ModelCatalog` for cost-aware routing. Not implemented. | clawft-llm | 3h | Add pricing data and usage tracking. |
| GAP-36 | Multi-arch Docker images (buildx). Deferred from 3B. | CI | 2h | |
| GAP-37 | macOS code signing. Deferred from 3B. | CI | 4h | |

---

## 3. Test Gaps

### 3.1 Unit Test Coverage

| Crate | Tests | Coverage Assessment | Gap |
|-------|-------|-------------------|-----|
| clawft-types | 152 | GOOD -- config, provider, session, event types well tested | -- |
| clawft-platform | 157 | GOOD -- config loader, fs, env, http mocks | -- |
| clawft-core | 247 + 133 (integration) | GOOD -- pipeline, bus, session, memory, skills, agent | Minor gaps below |
| clawft-llm | 56 | MODERATE -- types, config, router tested. Missing: actual HTTP call tests with mock server | TEST-01 |
| clawft-tools | 45 + 33 (security) | MODERATE -- security policy well tested. Missing: individual tool execute() tests | TEST-02 |
| clawft-channels | 59 | MODERATE -- Telegram, Slack, Discord types/factories tested. Missing: mock ChannelHost integration | TEST-03 |
| clawft-services | 65 | GOOD -- cron, heartbeat, MCP client, transport all tested | -- |
| clawft-cli | 29 (integration) + unit | GOOD -- CLI arg parsing, integration tests | -- |
| clawft-wasm | 41 | GOOD for stubs -- platform stubs, allocator, exports tested | -- |

### 3.2 Missing Test Scenarios

| ID | Test Gap | Priority | LOE |
|----|----------|----------|-----|
| TEST-01 | clawft-llm: No mock HTTP server tests for `OpenAiCompatProvider::complete()`. Only type serialization and router tests exist. Need to verify actual HTTP request construction and response parsing. | P0 | 3h |
| TEST-02 | clawft-tools: No tests for `web_search.execute()`, `web_fetch.execute()`, `spawn_tool.execute()` with mock platform. Only security policy and file tool tests exist. | P1 | 3h |
| TEST-03 | clawft-channels: No mock ChannelHost integration tests (channel start -> deliver_inbound -> verify message). Only type parsing and factory construction tested. | P1 | 3h |
| TEST-04 | Agent loop end-to-end test with mock LLM that returns tool calls. Verify: inbound message -> context build -> LLM call -> tool execution -> LLM re-call -> outbound message. The existing `phase1_integration.rs` and `phase2_integration.rs` may cover this but need verification. | P0 | 2h |
| TEST-05 | Config compatibility: Load real Python-generated config.json fixtures and verify all fields parse correctly. The spec calls for this test. | P1 | 1h |
| TEST-06 | Session compatibility: Load real Python-generated .jsonl session files. The spec calls for this test. | P1 | 1h |
| TEST-07 | Feature flag combinations: Verify `--no-default-features`, `--features minimal`, `--features all-channels` all compile. Currently only `--no-default-features` for clawft-tools is tested. | P1 | 1h |
| TEST-08 | Error path tests: What happens when LLM returns malformed JSON? When tool execution fails? When channel plugin crashes? Many error paths are untested. | P1 | 3h |

---

## 4. Documentation Gaps

| ID | Gap | File | Priority | LOE |
|----|-----|------|----------|-----|
| DOC-01 | `docs/benchmarks/results.md` missing | clawft/docs/benchmarks/ | P1 | 1h |
| DOC-02 | API documentation for clawft-core public types (ToolRegistry, MessageBus, AgentLoop) is minimal. Doc comments exist but no rustdoc overview page. | clawft-core | P2 | 2h |
| DOC-03 | Plugin development guide (how to write a new Channel plugin) not written. docs/guides/channels.md exists but focuses on configuration, not development. | clawft/docs/guides/ | P2 | 2h |
| DOC-04 | docs/guides/routing.md references "readability extraction" for web_fetch but this is not implemented. Documentation describes non-existent feature. | clawft/docs/guides/ | P1 | 0.5h |
| DOC-05 | docs/reference/tools.md should list all 12 tools with their JSON schemas. Verify completeness. | clawft/docs/reference/ | P1 | 1h |
| DOC-06 | README.md at clawft/ root needs update for Phase 3 status. | clawft/README.md | P2 | 1h |

---

## 5. Security Gaps

| ID | Gap | Priority | LOE | Notes |
|----|-----|----------|-----|-------|
| SEC-01 | CommandPolicy (allowlist mode) | DONE | -- | Implemented in Phase 3 Round 1 |
| SEC-02 | UrlPolicy (SSRF protection) | DONE | -- | Implemented in Phase 3 Round 1 |
| SEC-03 | Shell tool wiring to CommandPolicy | DONE | -- | Wired in Round 2 |
| SEC-04 | API key handling: config supports `api_key` (direct) and `api_key_env` (env var). Verify direct API keys are never logged. | P1 | 1h | Audit all `debug!` and `info!` calls that touch config. Ensure no key leakage to tracing output. |
| SEC-05 | File tool path traversal: File tools validate paths against `allowed_paths` from config. Verify symlink traversal is blocked. | P1 | 1h | Add test: create symlink outside workspace, verify file tool rejects read. |
| SEC-06 | Session JSONL injection: Verify that user-supplied content in session files is properly escaped when written to JSONL. | P2 | 1h | Add test with content containing `\n{"role":"system",...}` to verify it doesn't inject fake messages. |
| SEC-07 | MCP transport security: StdioTransport spawns child processes. Verify command injection is not possible via MCP server config. | P1 | 1h | Audit StdioTransport::new() -- command comes from config.json `mcp_servers` array. Ensure no shell expansion. |

---

## 6. Cross-Cutting Concerns

### 6.1 Error Handling Consistency

| Area | Status | Gap |
|------|--------|-----|
| Error types defined in clawft-types | PASS -- ClawftError, ConfigError, ProviderError, ToolError, ChannelError | -- |
| Errors propagated correctly | MOSTLY -- one `unimplemented!()` found in session.rs (test-only, demoted to P2) | GAP-16 (P2) |
| User-facing error messages | PARTIAL -- some errors return internal messages to user | P2 |
| Error recovery in channel plugins | UNKNOWN | GAP-23 |

### 6.2 Logging Coverage

| Area | Status | Gap |
|------|--------|-----|
| Agent loop lifecycle (start, message, response) | PASS -- info! and debug! throughout | -- |
| Channel plugin lifecycle | PASS -- start, connect, disconnect logged | -- |
| Tool execution | PASS -- each tool logs execution | -- |
| LLM calls | PARTIAL -- request logged but response details minimal | P2 |
| Security events (policy rejections) | PASS -- warn! on rejections | -- |

### 6.3 Configuration Completeness

| Config Section | Spec Fields | Implemented Fields | Gap |
|---------------|-------------|-------------------|-----|
| agents | model, system_prompt, context_window, memory_consolidation, skills_dir, workspace_dir | All present + more | -- |
| channels | telegram, slack, discord, extra | All present | -- |
| providers | providers map, default_provider, default_model, failover_strategy | All present | `failover_strategy` parsed but not used (GAP-18) |
| gateway | host, port, enabled_channels | All present | -- |
| tools | exec_enabled, exec_timeout_secs, allowed_paths, web_search_api_key, mcp_servers | All present | `mcp_servers` not acted upon (GAP-06) |
| pipelines | Per-task-type config | **NOT IMPLEMENTED** | GAP-38 (P2) |
| security | command_policy, url_policy | Implemented (added in Phase 3) | -- |

**GAP-38**: The `pipelines` config section from 02-technical-requirements.md (per-task-type
pipeline configuration) is defined in the Config struct but the PipelineRegistry does not
read it. All tasks use the default pipeline. This is P2 since the Level 0 single-pipeline
approach works.

### 6.4 CLI Help Completeness

| Command | --help | Description | Documented |
|---------|--------|-------------|------------|
| weft | Yes | "clawft AI assistant CLI" | Yes |
| weft agent | Yes | Has -m, --model, -c, --intelligent-routing | Yes |
| weft gateway | Yes | Has -c, --intelligent-routing | Yes |
| weft status | Yes | Has --detailed | Yes |
| weft channels status | Yes | Has -c | Yes |
| weft cron * | Yes | All subcommands have args | Yes |
| weft sessions * | Yes | list/inspect/delete | Yes |
| weft memory * | Yes | show/history/search | Yes |
| weft config * | Yes | show/section | Yes |
| weft completions | Yes | bash/zsh/fish/powershell | Yes |
| weft onboard | **MISSING** | -- | GAP-12 |

---

## 7. Sprint Plan

### Phase 3I Sprint Scope

Focus on P0 gaps only. Target: 3-4 working days.

> **Re-baselined 2026-02-17**: GAP-01, GAP-02, GAP-06 resolved in code (verified by 3I reviewer).
> GAP-16 demoted to P2 (test-only mock code). GAP-19 retained as verification-only.
> Original 12 P0 reduced to 8 active P0 gaps. Sprint restructured accordingly.

#### Day 1: SSE Streaming + Web Search (Hardest Items First)

| Task | Gap IDs | LOE | Owner |
|------|---------|-----|-------|
| Implement SSE streaming in clawft-llm + pipeline | GAP-11 | 6h | LLM |
| Fix web_search config mismatch (endpoint vs api_key) | GAP-03 | 1h | Tools |

#### Day 2: JSON Repair + Retry + Onboard

| Task | Gap IDs | LOE | Owner |
|------|---------|-----|-------|
| Implement JSON repair for malformed LLM output | GAP-14 | 2h | Core |
| Add retry/failover to clawft-llm | GAP-18 | 4h | LLM |
| Implement `weft onboard` command | GAP-12 | 3h | CLI |

#### Day 3: Memory + Tool Parsing + Verification + Tests

| Task | Gap IDs | LOE | Owner |
|------|---------|-----|-------|
| Wire VectorStore into memory tool search | GAP-15 | 2h | Tools |
| Verify tool call parsing in transport.rs | GAP-17 | 1h | Core |
| Verify tool result truncation (MAX_TOOL_RESULT_BYTES) | GAP-19 | 1h | Core |
| clawft-llm mock HTTP tests | TEST-01 | 3h | Test |
| Agent loop e2e test with tool calls | TEST-04 | 2h | Test |

#### Day 4: P1 Security + Docs + Smoke Tests

| Task | Gap IDs | LOE | Owner |
|------|---------|-----|-------|
| API key logging audit | SEC-04 | 1h | Security |
| Symlink path traversal test | SEC-05 | 1h | Security |
| Web fetch body size limit | MISSED-05 | 1h | Security |
| Create docs/benchmarks/results.md | DOC-01 | 1h | Docs |
| Fix routing.md documentation (remove readability reference) | DOC-04 | 0.5h | Docs |
| Smoke test with real API keys | -- | 2h | QA |

### P1 Items (if time permits after P0)

Priority order:
1. Memory consolidation (GAP-04) -- 4h
2. Channel plugin error recovery (GAP-23) -- 2h
3. API key logging audit (SEC-04) -- 1h
4. Symlink path traversal test (SEC-05) -- 1h
5. Config compatibility test fixtures (TEST-05, TEST-06) -- 2h
6. Tool execute() tests (TEST-02) -- 3h
7. Model aliasing (GAP-24) -- 2h
8. HttpClient::post_form (GAP-09) -- 3h

---

## 8. Acceptance Criteria for Phase 3 Gate

### Must Pass (P0 resolved)

- [x] `weft agent -m "hello"` returns an LLM response (with configured provider) **(GAP-01/02 resolved pre-sprint)**
- [ ] `weft agent -m "read the file at ./Cargo.toml"` triggers tool call and returns content
- [x] LLM provider bootstrap wires automatically from config (no `--intelligent-routing` flag needed) **(GAP-01/02 resolved pre-sprint)**
- [ ] `weft onboard` creates default config and workspace
- [x] MCP servers from config are connected and tools are registered **(GAP-06 resolved pre-sprint)**
- [ ] SSE streaming works for real-time response output
- [ ] JSON repair handles common malformed LLM outputs
- [ ] ~~No `unimplemented!()` calls in production code~~ **(GAP-16 demoted to P2 -- test-only mock)**
- [ ] Retry/failover on provider failure
- [ ] clawft-llm has mock HTTP tests proving request/response format
- [ ] Agent loop e2e test passes with tool calls

### Should Pass (P1 addressed)

- [ ] Memory consolidation runs when MEMORY.md exceeds threshold
- [ ] `docs/benchmarks/results.md` exists with real data
- [ ] Channel plugins retry on crash with backoff
- [ ] No API keys in tracing output
- [ ] File tools reject symlink traversal
- [ ] Real Python config.json fixture loads successfully

### Documented Deferrals (P2 accepted)

- RVF integration (all points) -- Phase 4
- ruvector crate integration -- Phase 4+
- WASM real HTTP/FS implementations -- Phase 4
- Native Anthropic/Bedrock/Gemini providers -- Phase 4
- SubagentManager -- Phase 4
- Readability HTML extraction -- Phase 4
- Codex OAuth flow -- Phase 4
- Pipeline per-task-type config -- Phase 4
- Cost tracking -- Phase 4

---

## 9. Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| SSE streaming implementation takes longer than estimated | Medium | High | Start with non-streaming first (print full response). Add streaming in follow-up. |
| MCP wiring reveals design issues in ToolRegistry | Medium | Medium | McpClient already works in isolation. Risk is in dynamic tool registration. |
| JSON repair edge cases cause regressions | Low | Medium | Add comprehensive test fixtures from real LLM outputs. |
| clawft-llm retry logic interacts poorly with pipeline | Low | Medium | Test retry at transport level, not pipeline level. |
| `weft onboard` wizard UX is poor on first attempt | Medium | Low | Start with non-interactive mode (--yes flag). Add interactive later. |

---

## Appendix A: File Inventory Summary

| Category | Count |
|----------|-------|
| Rust source files (crates/*/src/) | ~100 |
| Integration test files | 5 |
| Documentation files (docs/) | 16 |
| CI workflow files | 4 |
| Shell scripts (scripts/) | 13 |
| Total tests | 1,054 |
| Total LOC (Rust) | ~31,359 |
| Crates in workspace | 9 |

## Appendix B: Crate-by-Crate Status

| Crate | Source Files | Tests | Status |
|-------|-------------|-------|--------|
| clawft-types | 6 | 152 | Complete |
| clawft-platform | 5 | 157 | Complete (missing glob, post_form) |
| clawft-core | 19 | 380 | Complete (missing consolidation, JSON repair) |
| clawft-llm | 6 | 56 | Functional (missing streaming, retry, native providers) |
| clawft-tools | 9 | 78 | Functional (all tools work, some config issues) |
| clawft-channels | 14 | 59 | Functional (3 channels, missing media/voice) |
| clawft-services | 8 | 65 | Complete |
| clawft-cli | 12 | 29 | Functional (missing onboard, provider login) |
| clawft-wasm | 6 | 41 | Scaffold (stubs, compiles to wasip1) |
