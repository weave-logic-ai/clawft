# clawft-llm Provider Crate - Development Notes

## Status: COMPLETE

**Date**: 2026-02-16
**Crate**: `clawft-llm` (standalone, no workspace dependencies)
**Location**: `clawft/crates/clawft-llm/`

## Files Implemented

| File | Lines | Purpose |
|------|-------|---------|
| `src/lib.rs` | 43 | Module declarations, re-exports, crate-level docs |
| `src/error.rs` | 105 | `ProviderError` enum via thiserror, `Result<T>` alias |
| `src/types.rs` | 231 | `ChatMessage`, `ChatRequest`, `ChatResponse`, `ToolCall`, `Usage` |
| `src/config.rs` | 164 | `ProviderConfig` struct, `builtin_providers()` for 7 providers |
| `src/provider.rs` | 42 | `Provider` async trait (complete method) |
| `src/openai_compat.rs` | 243 | `OpenAiCompatProvider` — works with any OpenAI-compat API |
| `src/router.rs` | 268 | `ProviderRouter` — prefix-based model-to-provider routing |

**Total**: ~1,096 lines (all files under 500-line limit)

## Test Results

- **56 tests passing**, 0 failures
- Coverage across all modules: error display strings, serde roundtrips, URL construction, prefix routing, API key resolution, retry-after parsing
- Doc-tests use `ignore` attribute (require live API access)

## Architecture Decisions

1. **Standalone crate**: Zero dependencies on clawft-types or any other workspace crate. Types are self-contained.
2. **OpenAI-compat as the universal adapter**: Single `OpenAiCompatProvider` handles all 7+ built-in providers via `base_url` configuration.
3. **Prefix routing**: Model names like `"anthropic/claude-sonnet-4-5-20250514"` are split at the prefix to route to the correct provider. Unprefixed models go to the default (first configured) provider.
4. **API key resolution**: Supports both explicit keys (`with_api_key()`) and environment variable lookup. The env var name is configurable per provider.
5. **Error mapping**: HTTP status codes map to semantic errors: 429 -> RateLimited, 401/403 -> AuthFailed, 404 -> ModelNotFound.
6. **Debug safety**: `OpenAiCompatProvider`'s Debug impl masks API keys with `***`.

## Built-in Providers

OpenAI, Anthropic, Groq, DeepSeek, Mistral, Together AI, OpenRouter

## Dependencies

`async-trait`, `reqwest` (stream), `tokio`, `tracing`, `thiserror`, `serde`, `serde_json`, `uuid`

## Integration Points

- **clawft-core** will import `Provider` trait and `ChatRequest`/`ChatResponse` types
- **clawft-core pipeline** stages use this crate for LLM calls
- **clawft-services** may instantiate `ProviderRouter` from user config
- No circular dependencies possible (standalone crate)

## Open Items for Future Waves

- Streaming support (SSE parsing for `stream: true` requests)
- Retry logic with exponential backoff (currently returns `RateLimited` error for caller to handle)
- Request/response middleware hooks
- Token counting / budget tracking
- litellm-rs sidecar integration for exotic providers
