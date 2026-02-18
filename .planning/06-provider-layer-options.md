# Provider Layer Options: Pro/Con Report

## Executive Summary

Four approaches were evaluated for clawft's LLM provider transport layer. Option B (graniet/llm) was **rejected** after deep analysis revealed 3-5 MB binary bloat from unconditional tokio[full], hardcoded reqwest, no WASM support, and zero additional provider coverage beyond what OpenAI-compatible config provides.

**Decision**: **Option A** -- extract barni-providers as a standalone crate named **`clawft-llm`**. Config-driven `OpenAiCompatProvider` covers all major providers via `base_url`. Optional litellm-rs sidecar (Option C3) for exotic providers with non-OpenAI wire formats.

---

## The Options

| Option | Approach | Summary |
|--------|----------|---------|
| **A** | Reuse `barni-providers` | Port existing barni crate into clawft workspace |
| **B** | Depend on `graniet/llm` | External crate dependency |
| **C** | Integrate `litellm-rs` | Fork/extract, sidecar, or cherry-pick |
| **D** | Build from scratch | Write all provider code new |

---

## Option A: Reuse barni-providers

### What Exists

`barni/src/layer3/barni-providers/` is a complete, tested LLM provider abstraction:

| Component | Status | Lines (est.) |
|-----------|--------|-------------|
| `Provider` trait (complete, stream, status) | Production | ~100 |
| `ProviderRegistry` (register, select, failover) | Production | ~200 |
| `AnthropicProvider` (full SSE streaming, tool calling) | Production | ~800 |
| `OpenAiProvider` | Production | ~600 |
| `BedrockProvider` | Production | ~500 |
| `GeminiProvider` | Production | ~400 |
| `CompletionRequest` builder (messages, tools, metadata) | Production | ~300 |
| `CompletionResponse` + `StreamChunk` + `StreamDelta` | Production | ~250 |
| `FailoverController` (4 strategies) | Production | ~300 |
| `CircuitBreaker` (lock-free atomics, WASM-safe) | Production | ~150 |
| `ModelCatalog` (11 models, pricing, categories) | Production | ~200 |
| `UsageTracker` (cost calculation, budget status) | Production | ~250 |
| `ProviderError` (comprehensive error enum) | Production | ~100 |

Supporting code in `barni-engine`:
| Component | Status |
|-----------|--------|
| `RetryPolicy` (exponential backoff, aggressive/conservative presets) | Production |
| `SseFormatter` (wire-format SSE output) | Production |
| `StreamStats` (TTFT, events/sec, text bytes) | Production |
| `TextAccumulator` (streaming text buffer) | Production |

Supporting code in `barni-common`:
| Component | Status |
|-----------|--------|
| `CircuitBreaker` (lock-free, WASM-safe) | Production |
| Error hierarchy (`BarniError`, `AuthError`, etc.) | Production |

### Pros

- **Zero external dependency risk** -- code you own, MIT-compatible
- **Already tested** with real Anthropic, OpenAI, Bedrock, Gemini APIs
- **WASM-safe CircuitBreaker** using atomics (not tokio/async)
- **SSE streaming** with proper delta types (`TextDelta`, `ToolInputJsonDelta`, `Usage`, `StopReason`)
- **Tool calling** fully modeled (`ContentBlock::ToolUse`, `ContentBlock::ToolResult`)
- **4 failover strategies** (Sequential, WeightedRoundRobin, LowestLatency, CostOptimized)
- **Cost tracking** with real pricing data for Claude, GPT-4, Gemini
- **Clean trait boundary** -- `Provider` trait is object-safe, async, `Send + Sync`
- **Builder pattern** for `CompletionRequest` with validation
- **Exactly the reqwest pattern** clawft needs (connection pooling, timeouts, custom headers)
- **No actix-web, no database, no CLI framework** -- pure library code
- **Same team, same conventions** -- consistent with clawft's architecture

### Cons

- **Only 4 native providers** currently (Anthropic, OpenAI, Bedrock, Gemini). However, the config-driven `OpenAiCompatProvider` covers 15+ additional providers via `base_url` (Groq, DeepSeek, Mistral, OpenRouter, xAI, Cerebras, SambaNova, DeepInfra, HuggingFace, Cohere, etc.)
- **Needs porting** from barni workspace into clawft workspace (rename, adjust deps)
- **May have barni-specific assumptions** (tenant IDs, compliance hooks) that need stripping
- **No model aliasing** (e.g., `"gpt4"` -> `"gpt-4"`)
- **No prefix-based routing** (e.g., `"anthropic/claude-3-sonnet"` auto-routes)

### Level of Effort

| Task | Effort |
|------|--------|
| Copy barni-providers into clawft workspace | 0.5 days |
| Rename types (`BarniError` -> `ClawftError`, strip tenant/compliance) | 1 day |
| Adapt to clawft-platform traits (`HttpClient` abstraction) | 1 day |
| Add model alias + prefix routing | 0.5 days |
| Add Groq provider (OpenAI-compatible endpoint) | 0.5 days |
| Add DeepSeek provider (OpenAI-compatible endpoint) | 0.5 days |
| Add Mistral provider (OpenAI-compatible endpoint) | 0.5 days |
| Wire into pluggable pipeline (TaskClassifier, ModelRouter, etc.) | 1 day |
| Tests + validation | 1 day |
| **Total** | **~6.5 days** |

### What You Get

- 4 native providers (Anthropic, OpenAI, Bedrock, Gemini) + unlimited OpenAI-compat via config
- Full streaming with tool calling
- Failover + circuit breaker + retry
- Cost tracking + model catalog
- Clean trait boundary for ruvector intelligence layer to plug into
- WASM-safe primitives
- Code you fully control
- 10,563 lines of production-tested code

---

## Option B: Depend on graniet/llm

### What It Provides

The [`llm` crate](https://github.com/graniet/llm) (v1.3.7, MIT, 306 stars, 57K downloads) provides:

- **15 providers** via feature flags: OpenAI, Anthropic, Ollama, DeepSeek, xAI, Phind, Google, Groq, Azure, Cohere, Mistral, OpenRouter, HuggingFace, Bedrock, ElevenLabs
- **Public `ChatProvider` trait** -- extensible for custom providers
- **Full streaming** including streaming tool calls (`StreamChunk::ToolUseStart/Delta/Complete`)
- **Full tool calling** in both sync and streaming modes
- **Chain system** for multi-step workflows
- **Evaluator** for scoring LLM outputs
- **Memory module** for conversation history
- **Resilient LLM wrapper** with retry logic

### Pros

- **15 providers out of the box** -- all clawft needs and more
- **Excellent feature flags** -- compile only needed providers (`default-features = false`)
- **Public `ChatProvider` trait** -- can add custom providers
- **Streaming tool calls** -- `StreamChunk` enum with `ToolUseStart`, `ToolUseInputDelta`, `ToolUseComplete`
- **Active development** (last commit Feb 2, 2026)
- **57K crates.io downloads** -- reasonable adoption
- **MIT license**
- **Built-in retry/resilience** via `resilient_llm` module
- **Chain system** could replace some agent loop logic

### Cons

- **External dependency** -- API changes at their pace, not yours
- **Always-compiled deps** (`serde_yaml`, `toml`, `dirs`, `regex`, `chrono`, `rand`) even with minimal features
- **`default` feature pulls in CLI + huge dep tree** -- MUST use `default-features = false`
- **`tokio` with `full` features is unconditional** -- pulls entire runtime (timers, net, io, fs, process, signal, sync, macros, rt-multi-thread). Bad practice for a library.
- **Hardcoded `reqwest`** -- no `HttpClient` abstraction, no middleware injection, no mock transport
- **Dynamic dispatch** (`Box<dyn ChatResponse>`) -- minor perf cost
- **reqwest 0.12** -- may conflict if other deps use 0.11 or 0.13
- **No WASM compatibility** -- no `cfg(target_arch = "wasm32")` anywhere, tokio `full` + `dirs` + `reqwest` stream all block WASM
- **Closed provider enum** -- `LLMBackend` is a fixed enum (not `#[non_exhaustive]`), custom providers bypass the builder entirely
- **Super-trait breadth** -- `LLMProvider` requires implementing `ChatProvider + CompletionProvider + EmbeddingProvider + SpeechToTextProvider + TextToSpeechProvider + ModelsProvider` even for chat-only use
- **String-only errors** -- `LLMError` variants all carry `String`, lose typed error context (HTTP status codes, timeout vs connection, etc.)
- **No failover controller** (has retry but not multi-provider failover)
- **No circuit breaker**
- **No cost tracking / model catalog**
- **No prefix-based model routing** (e.g., `"anthropic/claude-3-sonnet"`)
- **Different type system** -- must map between `llm::ChatMessage` and clawft types
- **No semver tags/releases** -- no git tags despite being at v1.3.7 on crates.io
- **Estimated 3-5 MB binary impact** -- unconditional deps well above clawft's 2 MB target delta
- **Edition 2021** -- fine for MSRV 1.85+

### Adding OpenAI-Compatible Providers

For OpenAI-compatible APIs (Groq, Mistral, OpenRouter, etc.), `llm` provides `OpenAICompatibleProvider<T>` which requires implementing `OpenAIProviderConfig` (~6 const values + optional `custom_headers()`). This is ~15 lines per new provider -- a useful shortcut if adopting this crate.

### Level of Effort

| Task | Effort |
|------|--------|
| Add dependency, configure feature flags | 0.5 days |
| Build adapter layer: `llm::ChatProvider` -> clawft `LlmTransport` trait | 2 days |
| Type mapping (ChatMessage <-> clawft Message, Tool types, StreamChunk) | 1.5 days |
| Build FailoverController (not provided by llm crate) | 1.5 days |
| Build CircuitBreaker (not provided) | 0.5 days |
| Build ModelCatalog + cost tracking (not provided) | 1 day |
| Add prefix-based routing (not provided) | 0.5 days |
| Wire into pluggable pipeline | 1 day |
| Tests + validation | 1 day |
| **Total** | **~9.5 days** |

### What You Get

- 15 providers (all from `llm` crate)
- Streaming + tool calling via adapter
- Must build your own failover, circuit breaker, cost tracking
- External dependency risk
- Faster provider coverage but slower infrastructure

---

## Option C: Integrate litellm-rs

### What It Provides

[litellm-rs](https://github.com/majiayu000/litellm-rs) (v0.3.2, MIT, 17 stars) is a full AI gateway server with:

- **33 wired providers** (16 in Provider enum, rest via OpenAI-compatible routing)
- **7 routing strategies** (SimpleShuffle, LeastBusy, UsageBased, LatencyBased, CostBased, RateLimitAware, RoundRobin)
- **Full SSE streaming**
- **Full tool calling**
- **Retry/fallback with cooldown**
- **Rate limiting** (3 strategies)
- **SDK module** with `LLMClient`

### Sub-Options

#### C1: Direct Library Dependency

```toml
litellm-rs = { version = "0.3", default-features = false, features = ["lite"] }
```

**Pros**: 33 providers, routing strategies, one line to add
**Cons**: Pulls in actix-web unconditionally (~200 transitive crates), all 100+ provider modules compile, crypto deps (argon2, aes-gcm) unconditional

**LOE**: 2 days (add dep + adapter layer)
**Verdict**: **NOT RECOMMENDED** -- actix-web unconditional dependency adds ~200 transitive crates and couples clawft to a web framework it doesn't need

#### C2: Fork and Extract Provider Layer

Fork litellm-rs, extract `core::providers/` + `core::completion/` + `core::router/` into a lean crate. Remove actix-web, auth crypto, CLI.

**Pros**: 16 real provider implementations, 7 routing strategies, clean separation exists architecturally
**Cons**: Fork maintenance burden, young project (7 months), may inherit bugs

**LOE**: 5-7 days (fork, extract, strip actix-web, test, integrate)
**Verdict**: High effort for uncertain quality

#### C3: Sidecar Service

Run litellm-rs as a separate process, communicate via HTTP.

**Pros**: Zero code coupling, process isolation, full 33 providers + routing
**Cons**: ~1-5ms latency per request (localhost HTTP), process management, config files, separate binary to ship, deployment complexity contradicts "single binary" goal

**LOE**: 2 days (build binary, write config, spawn/manage process)
**Verdict**: **Violates clawft's single-binary goal (G1)**

#### C4: Cherry-Pick Provider Code

Copy individual provider implementations (e.g., Anthropic client ~780 lines) into clawft.

**Pros**: Zero dependency, take only what you need, adapt to clawft's types
**Cons**: No upstream bug fixes, manual sync, 2-3 days per provider, edition 2024 syntax needs adaptation

**LOE**: 2-3 days per provider x however many needed
**Verdict**: Pragmatic but redundant given barni-providers already exists

### Aggregate litellm-rs Assessment

| Sub-Option | LOE | MSRV OK? | Single Binary? | Provider Count | Risk |
|------------|-----|----------|---------------|---------------|------|
| C1: Direct dep | 2 days | YES (1.87 OK with clawft 1.85+) | Yes | 33 | High (actix bloat) |
| C2: Fork+extract | 5-7 days | YES | Yes | 16 | Medium (maintenance) |
| C3: Sidecar | 2 days | N/A | NO | 33 | Medium (ops complexity) |
| C4: Cherry-pick | 6-9 days | YES | Yes | 3-5 | Low |

---

## Option D: Build From Scratch

### What It Means

Write all provider implementations, failover, circuit breaker, streaming, tool calling from zero.

### Pros

- Total control, zero external risk
- Exactly the types and patterns you want
- Can target WASM from day one

### Cons

- **~15-25 days** of pure provider implementation work
- Each provider has API quirks (Anthropic's `content_block_delta` vs OpenAI's `choices[0].delta`)
- SSE parsing edge cases are subtle
- Reinventing what barni-providers already solved

### Verdict

**NOT RECOMMENDED** when barni-providers exists.

---

## Comparison Matrix

| Criterion | A: barni-providers | B: graniet/llm | C: litellm-rs | D: Scratch |
|-----------|-------------------|---------------|---------------|-----------|
| **LOE** | ~4.5 days (standalone lib) | ~9.5 days | 1.5 days (sidecar) / 5-7 days (fork) | 15-25 days |
| **Providers (day 1)** | 4 native + unlimited OpenAI-compat via config | 15 | 16-33 | 0 |
| **Providers (week 2)** | 4 native + all OpenAI-compat + optional sidecar(33) | 15 | 16-33 | 3-5 |
| **Streaming** | Full (tested) | Full | Full | Must build |
| **Tool calling** | Full (tested) | Full | Full | Must build |
| **Failover** | 4 strategies (tested) | None (must build) | 7 strategies | Must build |
| **Circuit breaker** | Yes (WASM-safe) | None (must build) | Yes | Must build |
| **Cost tracking** | Yes (tested) | None (must build) | Yes | Must build |
| **Retry logic** | Exp. backoff (tested) | Built-in (resilient_llm) | Yes | Must build |
| **MSRV** | Same as barni (1.85+) | 1.85+ (edition 2021) | 1.87 (edition 2024, compatible) | 1.85+ |
| **WASM compat** | CircuitBreaker yes; HTTP no | No | No | Can design for it |
| **External dep risk** | None (your code) | Medium | High (young project) | None |
| **Single binary** | Yes | Yes | No (C3) or Yes | Yes |
| **Binary size impact** | Minimal (~100 KB delta) | Heavy (~3-5 MB from tokio full + unconditional deps) | Heavy (actix-web ~200 crates) | Minimal |
| **HTTP client abstraction** | reqwest (swappable via platform trait) | Hardcoded reqwest (no abstraction) | actix-web (coupled) | Your design |
| **License** | MIT (barni) | MIT | MIT | N/A |
| **Code ownership** | Full | External | External/fork | Full |
| **Type mapping needed** | Minimal (rename) | Heavy (6 sub-traits, different types) | Heavy (different types) | None |

---

## Recommendation

### Revised: Standalone clawft-llm library + optional sidecar extension

#### Key Decision: Extract as Standalone Library

Instead of copying barni-providers into clawft, **extract it as a standalone crate** (`clawft-llm`) that both barni and clawft depend on. The coupling to barni internals is minimal (3 imports: `CircuitBreaker`, `now_ms()`, `RequestId/SessionId/TenantId`) -- all easily internalized.

Benefits:
- **Single source of truth** -- bug fixes and new providers benefit both projects
- **Publishable** -- can go to crates.io or stay as a git dependency
- **Clean API** -- forces removal of barni-specific assumptions
- **Reusable** -- any future Rust project can import it

#### MSRV: 1.85+ (Edition 2024)

barni already uses `rust-version = "1.85"`. clawft will match. Local toolchain is 1.93.0. This removes MSRV as a concern for any dependency.

#### Phase 1 (Warp, weeks 1-8)

1. Extract `barni-providers` into standalone `clawft-llm` crate (~2 days)
   - Move `CircuitBreaker` into the crate (currently ~150 lines in barni-common)
   - Replace `RequestId/SessionId/TenantId` with generic `Option<String>` metadata
   - Replace `now_ms()` with inline `std::time` call
   - Update barni workspace to depend on the new crate
2. Add **generic OpenAI-compatible endpoint** support (~0.5 days)
   - Config-driven: any `base_url` + `api_key` + optional custom headers
   - Covers Groq, DeepSeek, Mistral, OpenRouter, Together, Fireworks, Perplexity, xAI, etc.
   - No per-provider structs needed -- one `OpenAiCompatProvider` handles all
3. Add prefix-based model routing + model aliasing (~0.5 days)
   - `"anthropic/claude-3-sonnet"` auto-routes to Anthropic provider
   - `"fast"` alias resolves to configured model
4. Wire into the pluggable pipeline traits (~1 day)
5. **Result**: 4 native providers (Anthropic, OpenAI, Bedrock, Gemini) + unlimited OpenAI-compatible endpoints via config, full streaming, tool calling, failover, circuit breaker, cost tracking
6. **LOE**: ~4.5 days (reduced from 6.5 by not needing per-provider impls for OpenAI-compat endpoints)

#### Phase 2 (Weft, weeks 7-11) -- optional extensions

**Extension A: litellm-rs sidecar** (for users needing non-OpenAI-compatible providers)
- Install litellm-rs as a separate binary (`cargo install` or container)
- clawft detects sidecar at `http://localhost:<port>` via config
- `SidecarTransport` forwards requests for providers with custom wire formats (Cohere, HuggingFace, etc.)
- Core `weft` binary remains single-binary; sidecar is opt-in
- Also provides litellm-rs's 7 routing strategies and rate limiting for managed deployments
- **LOE**: ~1.5 days

**Extension B: ruvector intelligence layer**
- Plug ruvector crates into the pipeline traits (as planned in 05-ruvector-crates.md)
- `TaskClassifier` = ruvllm complexity analyzer
- `ModelRouter` = tiny-dancer FastGRNN + ruvllm HNSW
- `QualityScorer` = ruvllm quality engine
- `LearningBackend` = SONA micro-LoRA

#### Why graniet/llm is NOT needed

The industry has converged on the OpenAI chat completions wire format. With a config-driven `OpenAiCompatProvider` that accepts any `base_url`, clawft-llm natively handles:

| Provider | Base URL | Notes |
|----------|----------|-------|
| Groq | `api.groq.com/openai/v1` | OpenAI-compatible |
| DeepSeek | `api.deepseek.com` | OpenAI-compatible; reasoner model adds `reasoning_content` field |
| Mistral | `api.mistral.ai/v1` | OpenAI-compatible; temp range 0.0-0.7 |
| OpenRouter | `openrouter.ai/api/v1` | OpenAI-compatible, aggregates 200+ models |
| Together | `api.together.xyz/v1` | OpenAI-compatible |
| Fireworks | `api.fireworks.ai/inference/v1` | OpenAI-compatible |
| Perplexity | `api.perplexity.ai` | OpenAI-compatible; response includes `search_results` metadata |
| xAI/Grok | `api.x.ai/v1` | OpenAI-compatible |
| Ollama | `localhost:11434/v1` | OpenAI-compatible; no real auth (pass any API key) |
| Azure OpenAI | `{resource}.openai.azure.com/openai/v1` | Requires `api-version` query param; deployment-based URLs |
| Cerebras | `api.cerebras.ai/v1` | Mostly compatible; some params return 400 |
| SambaNova | `api.sambanova.ai/v1` | OpenAI-compatible |
| DeepInfra | `api.deepinfra.com/v1/openai` | OpenAI-compatible |
| HuggingFace | `router.huggingface.co/v1` | OpenAI-compatible; model names need `:provider` suffix |
| Cohere | Compatibility API endpoint | OpenAI compat layer; limited `reasoning_effort` options |
| Anthropic | Native: `/v1/messages` | Native provider in clawft-llm; compat layer exists but not for production |
| Google Gemini | `generativelanguage.googleapis.com/v1beta/openai` | Native provider in clawft-llm; also has OpenAI compat endpoint |
| AWS Bedrock | `bedrock-runtime.{region}.amazonaws.com/openai/v1` | Native SigV4 provider in clawft-llm; also has OpenAI compat endpoint |
| Any future | any `base_url` | Industry standard: most new providers adopt OpenAI format |

**Key finding**: Even providers that historically had proprietary APIs (Anthropic, Cohere, HuggingFace, Gemini) now offer OpenAI-compatible endpoints. This means the `OpenAiCompatProvider` covers virtually every provider in production use. The optional litellm-rs sidecar is only needed for truly exotic wire formats or for users who prefer its 7 routing strategies.

Adding graniet/llm would cost 3-5 MB of binary bloat for zero additional coverage.

### Why This Works

- **clawft-llm is the ship** (HTTP transport, streaming, failover) -- standalone library, shared between projects
- **OpenAI-compatible config** means any new provider is a 3-line config entry, not code
- **litellm-rs sidecar is the fleet expansion** (33 providers, routing strategies) -- optional for managed/enterprise deployments
- **ruvector is the navigator** (routing intelligence, learning, quality scoring) -- plugs into pipeline traits
- **Two tiers, not four** -- simpler architecture, easier to maintain

### Architecture Diagram

```
config.json
    |
    v
[PipelineRegistry] -- selects pipeline per TaskType
    |
    v
[TaskClassifier]   -- KeywordClassifier (L0) | ruvllm::ComplexityAnalyzer (L1+)
    |
    v
[ModelRouter]      -- StaticRouter (L0) | tiny-dancer::FastGRNN (L2+)
    |
    v
[ContextAssembler] -- TokenBudget (L0) | ruvector-attention (L4+)
    |
    v
                     +-- AnthropicProvider (native, Anthropic Messages API)
                     +-- OpenAiProvider (native, OpenAI Chat Completions)
[LlmTransport] -----+-- BedrockProvider (native, AWS SigV4)
                     +-- GeminiProvider (native, Google Generative AI)
                     +-- OpenAiCompatProvider (config-driven, ANY base_url)
                     +-- SidecarTransport (optional, litellm-rs for Cohere/HF/exotic)
    |
    v
[QualityScorer]    -- NoopScorer (L0) | ruvllm::QualityEngine (L1+)
    |
    v
[LearningBackend]  -- NoopLearner (L0) | sona::SonaEngine (L3+)
```

Each box is a trait. Each implementation is swappable. Different TaskTypes get different pipelines. The transport layer (clawft-llm) is always available; intelligence layers (ruvector) are opt-in.
