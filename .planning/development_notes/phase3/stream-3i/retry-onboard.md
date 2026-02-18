# Stream 3I: Retry/Failover (GAP-18) + Onboard Command (GAP-12)

**Date**: 2026-02-17
**Agent**: 3I-C (coder)
**Status**: COMPLETE

---

## GAP-18: Retry + Failover in clawft-llm

### What was implemented

1. **`retry.rs`** -- Exponential backoff retry wrapper for any `Provider`
   - `RetryConfig`: configurable max_retries (default 3), base_delay (1s), max_delay (30s), jitter_fraction (0.25)
   - `RetryPolicy<P: Provider>`: wraps any provider with automatic retry on transient errors
   - `is_retryable()`: determines retry eligibility (429, 500, 502, 503, 504, timeouts, network errors)
   - `compute_delay()`: exponential backoff with capped maximum and configurable jitter
   - Rate-limited responses (429) respect the server's `retry_after_ms` suggestion when it exceeds the computed delay
   - Both `complete()` and `complete_stream()` are retried

2. **`failover.rs`** -- Provider failover chain
   - `FailoverChain`: ordered list of providers; tries each in sequence on failure
   - Retryable errors (server errors, timeouts) trigger failover to next provider
   - Non-retryable but failover-eligible errors (NotConfigured, ModelNotFound) also trigger failover
   - Non-retryable, non-failover-eligible errors (AuthFailed, InvalidResponse) return immediately without trying other providers
   - Both `complete()` and `complete_stream()` support failover

3. **Error type extension** -- `ProviderError::AllProvidersExhausted`
   - New variant added to `error.rs` with per-provider error summaries
   - Returned when all providers in the failover chain have been exhausted

### Design decisions

- **RetryPolicy is generic over P: Provider** so it can wrap any provider implementation (not just OpenAiCompatProvider). This enables composition: `RetryPolicy<OpenAiCompatProvider>`.
- **FailoverChain uses `Box<dyn Provider>`** because the chain may contain different provider types (e.g., OpenAI primary, Anthropic fallback).
- **Auth errors do NOT trigger failover** because a bad API key for provider A does not mean provider B should be tried -- it likely means the user misconfigured their key.
- **NotConfigured errors DO trigger failover** because if provider A's key is missing, trying provider B (which may have a key) is the right behavior.
- **Jitter uses system time nanos** rather than pulling in the `rand` crate as a runtime dependency. This is sufficient for avoiding thundering herd on retries.

### Test coverage

- 26 tests in `retry.rs`: config defaults, is_retryable classification, exponential backoff math, capped delay, jitter bounds, success-first-try, success-after-retries, exhausted retries, non-retryable passthrough, rate-limit delay selection
- 11 tests in `failover.rs`: empty chain, len/names, primary succeeds, failover on retryable error, failover on NotConfigured, no failover on auth error, all providers exhausted, three-provider chain, failover eligibility classification
- 1 test in `error.rs`: AllProvidersExhausted display format

---

## GAP-12: `weft onboard` Command

### What was implemented

1. **`commands/onboard.rs`** -- New CLI subcommand
   - `weft onboard`: interactive wizard for first-time setup
   - `weft onboard --yes` / `weft onboard -y`: non-interactive mode with defaults
   - `weft onboard --dir <path>`: override config directory (default: `~/.clawft/`)
   - Creates directory structure: `~/.clawft/`, `~/.clawft/workspace/`, `~/.clawft/workspace/{sessions,memory,skills}`
   - Generates `config.json` with agent defaults, provider config, tool settings, gateway settings
   - Interactive provider selection menu (OpenAI, Anthropic, OpenRouter, Groq, DeepSeek, custom endpoint, skip)
   - Optional API key prompt (reads from stdin)
   - Smart defaults: model name matches the selected provider (e.g., "anthropic/claude-sonnet-4-5-20250514" for Anthropic)

2. **CLI integration** -- Added `Onboard` variant to `Commands` enum in `main.rs`, wired into dispatch

3. **Pre-existing fix** -- Fixed `mcp_tools.rs` TestTransport mock to implement `send_notification` (broken by another agent's changes to `McpTransport` trait)

### Design decisions

- **Non-interactive mode (`--yes`)** generates config with OpenAI defaults. This supports scripted/CI use cases.
- **Existing config check**: in interactive mode, prompts before overwriting; in `--yes` mode, silently skips if config exists.
- **API keys are written to config.json** only if explicitly provided by the user during onboard. Otherwise, the config relies on env var lookup at runtime (never hardcoded).
- **No config.json validation against clawft-types::Config** during write -- the template is a known-good structure. Validation happens at load time via `serde_json::from_value`.

### Test coverage

- 11 unit tests in `onboard.rs`: config dir resolution, directory creation, JSON validity, provider-with-key, custom-base, no-provider, model defaults, selection constructors
- 4 CLI parsing tests in `main.rs`: basic parse, --yes flag, -y short flag, --dir override

---

## Files modified

### clawft-llm
- `src/retry.rs` -- NEW (exponential backoff retry)
- `src/failover.rs` -- NEW (provider failover chain)
- `src/error.rs` -- Added `AllProvidersExhausted` variant + test
- `src/lib.rs` -- Added module declarations and re-exports

### clawft-cli
- `src/commands/onboard.rs` -- NEW (onboard wizard)
- `src/commands/mod.rs` -- Added `pub mod onboard`
- `src/main.rs` -- Added `Onboard` command variant, dispatch arm, and CLI tests
- `src/mcp_tools.rs` -- Fixed TestTransport mock (missing `send_notification`)

## Test results

- clawft-llm: 115 tests passed, 0 failed
- clawft-cli: 172 unit tests + 29 integration tests passed, 0 failed
- clippy: 0 warnings on both crates
