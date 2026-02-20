# Development Notes: Element 03 - Critical Fixes & Cleanup

**Workstreams**: A, B, I, J
**Weeks**: 1-5

---

## Planning Context (Initial)

Element 03 is the foundation layer: 33 items across 4 workstreams resolving all bugs, security issues, architecture debt, and documentation drift before feature work begins.

### Key Source Code Observations

#### Provider Count Discrepancy (J1)

Two separate provider registries exist and must not be confused:

- **`clawft-llm/src/config.rs` -- `builtin_providers()`**: 9 entries (openai, anthropic, groq, deepseek, mistral, together, openrouter, gemini, xai). These are the OpenAI-compatible routing providers used by `ProviderRouter`.

- **`clawft-types/src/provider.rs` -- `PROVIDERS` static array**: 15 entries (adds custom, aihubmix, openai_codex, zhipu, dashscope, moonshot, minimax, vllm). This is the legacy registry ported from Python nanobot.

The docs variously say "7", "8", or reference "14" -- all wrong. The `lib.rs` doc comment says "14-provider registry" when the test asserts 15.

#### Assembler Behavior (J2)

The architecture overview claims "No truncation at Level 0" for `TokenBudgetAssembler`. This is factually wrong. The assembler in `pipeline/assembler.rs`:
- Estimates tokens as `content.len() / 4 + 4` per message
- Truncates by keeping first (system) and last (current) messages
- Walks backwards from second-to-last to fill remaining budget
- Drops middle messages when budget is exceeded

#### Token Budget Source (J3)

The routing guide says budget comes from `agents.defaults.max_tokens`. In tiered mode, `build_live_pipeline()` in `llm_adapter.rs` derives the budget from:
```rust
config.routing.tiers.iter().map(|t| t.max_context_tokens as usize).max().unwrap_or(128_000)
```
This is the *input* context window, not the output token limit.

#### Identity Bootstrap (J4)

`context.rs` loads 5 bootstrap files (SOUL.md, IDENTITY.md, AGENTS.md, USER.md, TOOLS.md) from workspace root then `.clawft/` subdirectory. If SOUL.md or IDENTITY.md exists, the default "You are clawft..." preamble is replaced. Not documented anywhere.

#### Rate-Limit Retry (J5)

`ClawftLlmAdapter` retries rate-limited requests up to 3 times with exponential backoff floor of `1000 * 2^attempt` ms, using `max(provider_hint, backoff_floor)`. Not documented in the providers guide.

#### CLI Log Level (J6)

Default non-verbose level is `warn`, not `info`. Line 274 of `main.rs`:
```rust
let default_filter = if cli.verbose { "debug" } else { "warn" };
```

### SPARC Documents Created

- `04-workstream-J-doc-sync.md` -- Workstream J task document (7 items, J1-J7)
- `05-element-03-tracker.md` -- Full element tracker (33 items across A/B/I/J)

### Cross-Element Dependencies Identified

- A4 -> Element 04 C1 (PluginHost SecretRef)
- A2 -> Element 08 H2 (stable embeddings)
- A6 -> Element 04 C2 (WASM SSRF check)
- A9 -> Element 04 C2 (feature gates)
- B3 -> Element 04 C3 (file splits before skill loader)
- B5 -> Element 09 L1 (shared registry builder)

## Implementation Log

### 2026-02-20: Workstream A Complete (9/9 items)

**Branch**: `sprint/phase-5`
**Commit**: `63ebe99` feat(03): complete Workstream A critical fixes (A1-A9)
**Agent**: Agent-03 (hive-mind worker)
**Validation**: 1,903 tests passing, zero clippy warnings

All P0 security items (A1, A2, A4, A5, A6) and P1 items (A3, A7, A8, A9) completed in a single agent session. Key decisions:

- **A4**: Used `SecretString` wrapper (not `SecretRef` env-var-name pattern) -- stores the actual value internally but redacts on Debug/Display. `expose()` method returns the inner string. This was simpler than the env-var indirection pattern and still prevents accidental logging.
- **A7**: Added `timeout_secs: Option<u64>` to `ProviderConfig` so providers can override the 120s default.
- **A9**: Feature-gated entire MCP tools module behind `services` feature, with no-op stubs when disabled.

### Remaining: Workstreams B, I, J (24 items) - In Progress
