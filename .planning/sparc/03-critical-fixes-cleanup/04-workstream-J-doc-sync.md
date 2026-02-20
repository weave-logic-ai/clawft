# Workstream J: Documentation Sync

**Element**: 03 -- Critical Fixes & Cleanup
**Priority**: P1-P2
**Timeline**: Weeks 3-5
**Status**: Planning
**Dependencies**: None (reads code, updates docs)
**Items**: J1-J7

---

## Summary

Seven documentation items where docs have drifted from the actual codebase. After adding gemini and xai providers, multiple provider counts are wrong. The assembler description contradicts the implementation. Token budget source references a deprecated config field. Identity bootstrap, rate-limit retry, and CLI log level changes are undocumented.

---

## J1: Fix Provider Counts (P1)

### Problem

Multiple documentation files cite incorrect provider counts. The codebase has two distinct "provider" registries:

1. **`clawft-llm` built-in providers** (`crates/clawft-llm/src/config.rs`): **9 providers** (openai, anthropic, groq, deepseek, mistral, together, openrouter, gemini, xai). Test at line 126 asserts `providers.len() == 9`.

2. **`clawft-types` provider spec registry** (`crates/clawft-types/src/provider.rs`): **15 entries** in the `PROVIDERS` static array (custom, openrouter, aihubmix, anthropic, openai, openai_codex, deepseek, gemini, zhipu, dashscope, moonshot, minimax, vllm, groq, xai). Test at line 447 asserts `PROVIDERS.len() == 15`.

The docs reference different numbers depending on context:

### Files to Update

#### `docs/architecture/overview.md`

**Current text (lines 123-133):**
```
7 providers ship out of the box, each configured with a base URL, API key environment variable, and model prefix:

| Provider | Prefix | Base URL | API Key Env |
|----------|--------|----------|-------------|
| openai | `openai/` | `api.openai.com/v1` | `OPENAI_API_KEY` |
| anthropic | `anthropic/` | `api.anthropic.com/v1` | `ANTHROPIC_API_KEY` |
| groq | `groq/` | `api.groq.com/openai/v1` | `GROQ_API_KEY` |
| deepseek | `deepseek/` | `api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| mistral | `mistral/` | `api.mistral.ai/v1` | `MISTRAL_API_KEY` |
| together | `together/` | `api.together.xyz/v1` | `TOGETHER_API_KEY` |
| openrouter | `openrouter/` | `openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |
```

**Correct text:**
```
9 providers ship out of the box, each configured with a base URL, API key environment variable, and model prefix:

| Provider | Prefix | Base URL | API Key Env |
|----------|--------|----------|-------------|
| openai | `openai/` | `api.openai.com/v1` | `OPENAI_API_KEY` |
| anthropic | `anthropic/` | `api.anthropic.com/v1` | `ANTHROPIC_API_KEY` |
| groq | `groq/` | `api.groq.com/openai/v1` | `GROQ_API_KEY` |
| deepseek | `deepseek/` | `api.deepseek.com/v1` | `DEEPSEEK_API_KEY` |
| mistral | `mistral/` | `api.mistral.ai/v1` | `MISTRAL_API_KEY` |
| together | `together/` | `api.together.xyz/v1` | `TOGETHER_API_KEY` |
| openrouter | `openrouter/` | `openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |
| gemini | `gemini/` | `generativelanguage.googleapis.com/v1beta/openai` | `GOOGLE_GEMINI_API_KEY` |
| xai | `xai/` | `api.x.ai/v1` | `XAI_API_KEY` |
```

#### `docs/guides/providers.md`

**Current text (line 19):**
```
- **Zero boilerplate for built-in providers**: seven providers are pre-configured
  and ready to use with a single environment variable each.
```

**Correct text:**
```
- **Zero boilerplate for built-in providers**: nine providers are pre-configured
  and ready to use with a single environment variable each.
```

**Current text (line 63):**
```
`ProviderRouter::with_builtins()` registers seven providers out of the box.
```

**Correct text:**
```
`ProviderRouter::with_builtins()` registers nine providers out of the box.
```

**Current table (lines 66-74):** Missing gemini and xai rows. Add them after the openrouter row:
```
| `gemini` | `https://generativelanguage.googleapis.com/v1beta/openai` | `GOOGLE_GEMINI_API_KEY` | `gemini-2.5-flash` | Google Gemini via OpenAI-compatible endpoint. |
| `xai` | `https://api.x.ai/v1` | `XAI_API_KEY` | `grok-3-mini` | xAI Grok models. |
```

#### `docs/getting-started/quickstart.md`

**Current text (line 79):**
```
clawft ships with seven built-in LLM providers.
```

**Correct text:**
```
clawft ships with nine built-in LLM providers.
```

**Current table (lines 82-91):** Missing gemini and xai rows. Add:
```
| Gemini       | `GOOGLE_GEMINI_API_KEY` | `gemini/gemini-2.5-flash`                        |
| xAI          | `XAI_API_KEY`           | `xai/grok-3-mini`                                |
```

#### `docs/reference/config.md`

**Current text (line 121-122):**
```
Seven providers are built-in and require only an environment variable to activate.
```

**Correct text:**
```
Nine providers are built-in and require only an environment variable to activate.
```

**Current table (lines 128-137):** Already includes 8 rows (has gemini). Add xai row:
```
| 9 | **xai**      | `xai/`        | `https://api.x.ai/v1`                    | `XAI_API_KEY`         | `grok-3-mini`                  |
```

#### `crates/clawft-types/src/lib.rs`

**Current text (line 12):**
```
//! - **[`provider`]** -- LLM response types and the 14-provider registry
```

**Correct text:**
```
//! - **[`provider`]** -- LLM response types and the 15-provider registry
```

#### `crates/clawft-types/src/provider.rs`

**Current text (line 153):**
```
/// All 14 providers ported from the Python `nanobot/providers/registry.py`.
```

**Correct text:**
```
/// All 15 providers ported from the Python `nanobot/providers/registry.py`.
```

### Acceptance Criteria

- [ ] All doc files reference "9" for `clawft-llm` built-in routing providers
- [ ] All doc files reference "15" for `clawft-types` provider spec registry (where applicable)
- [ ] Provider tables in all doc files include gemini and xai rows
- [ ] `lib.rs` comment updated to "15-provider registry"
- [ ] `provider.rs` comment updated to "All 15 providers"

---

## J2: Fix Assembler Truncation Description (P1)

### Problem

`docs/architecture/overview.md` line 454 states:

**Current text:**
```
| 3 | Assembler | `ContextAssembler` | `TokenBudgetAssembler` | Passes through all messages with a token estimate based on `max_tokens`. No truncation at Level 0. |
```

This is incorrect. The `TokenBudgetAssembler` in `crates/clawft-core/src/pipeline/assembler.rs` actively truncates messages using a first-message + last-messages preservation strategy:

1. Always keeps the first message (system prompt)
2. Always keeps the last message (current user input)
3. Walks backwards from second-to-last, adding messages until budget is exhausted
4. Drops middle messages when budget is exceeded
5. Token estimation uses `content.len() / 4 + 4` heuristic

**Correct text:**
```
| 3 | Assembler | `ContextAssembler` | `TokenBudgetAssembler` | Truncates conversation history to fit within a token budget. Always preserves the first (system prompt) and most recent messages, dropping older messages from the middle. Token estimation uses a `chars / 4` heuristic. |
```

### Acceptance Criteria

- [ ] Pipeline stage table row for Assembler updated with accurate truncation description
- [ ] No claim of "no truncation" remains for Level 0 assembler

---

## J3: Fix Token Budget Source Reference (P1)

### Problem

`docs/guides/routing.md` Section 9 "Configuration" (lines 815-825) states:

**Current text:**
```
### Token Budget

The context assembler's token budget is derived from `agents.defaults.max_tokens`:
```

This is incorrect. As of the tiered router implementation, the token budget is sourced from `max_context_tokens` across routing tiers, not from `agents.defaults.max_tokens` (which is the *output* token limit).

Evidence from `crates/clawft-core/src/pipeline/llm_adapter.rs` lines 515-522:
```rust
let context_budget = config
    .routing
    .tiers
    .iter()
    .map(|t| t.max_context_tokens as usize)
    .max()
    .unwrap_or(128_000);
let assembler = Arc::new(TokenBudgetAssembler::new(context_budget));
```

**Correct text:**
```
### Token Budget

When tiered routing is enabled (`routing.mode = "tiered"`), the context assembler's
token budget is derived from the largest `max_context_tokens` across all configured
routing tiers. This is the *input* context window, not the output token limit.

When using static routing (default), the assembler uses `agents.defaults.max_tokens`
as its budget, though this is technically the output limit -- a known inconsistency
that will be addressed in a future update.

The `max_context_tokens` field on each tier controls how much conversation history
the assembler retains:

    routing.tiers[].max_context_tokens  ->  assembler budget (input window)
    agents.defaults.max_tokens          ->  LLM max_tokens param (output limit)
```

### Acceptance Criteria

- [ ] Token budget documentation distinguishes input context window from output token limit
- [ ] References `max_context_tokens` from routing tiers as the primary source for tiered mode
- [ ] Notes the static-routing fallback behavior

---

## J4: Document Identity Bootstrap Behavior (P2)

### Problem

The `SOUL.md` and `IDENTITY.md` override mechanism is not documented anywhere. When placed in the workspace root or `.clawft/` subdirectory, these files replace the default agent identity preamble.

Evidence from `crates/clawft-core/src/agent/context.rs` lines 111-190:

```rust
let bootstrap_files = ["SOUL.md", "IDENTITY.md", "AGENTS.md", "USER.md", "TOOLS.md"];
// ...
// Searches: workspace root first, then .clawft/ subdirectory.
for filename in &bootstrap_files {
    // ...
    let candidates = [
        ws_path.join(filename),
        ws_path.join(".clawft").join(filename),
    ];
    // First match wins
}

let has_custom_identity =
    loaded_files.contains_key("SOUL.md") || loaded_files.contains_key("IDENTITY.md");

if has_custom_identity {
    // Custom identity from bootstrap files (replaces default preamble)
} else {
    // Default identity: "You are clawft, a helpful AI assistant..."
}
```

### Target File

`docs/guides/skills-and-agents.md` -- add a new "Identity Bootstrap" section after the Agents section.

**Text to add:**
```markdown
## Identity Bootstrap

clawft supports customizing the agent's core identity through bootstrap files
placed in the workspace directory. These files are loaded during system prompt
assembly and can override the default agent persona.

### Bootstrap Files

The following files are checked, in order, from two locations:
1. Workspace root (e.g. `~/.clawft/workspace/`)
2. `.clawft/` subdirectory of the workspace

| File | Purpose | Effect |
|------|---------|--------|
| `SOUL.md` | Core agent identity and persona | Replaces the default "You are clawft..." preamble |
| `IDENTITY.md` | Agent identity (alternative to SOUL.md) | Same as SOUL.md -- either one triggers custom identity |
| `AGENTS.md` | Multi-agent context | Appended after the identity section |
| `USER.md` | User-specific context | Appended after the identity section |
| `TOOLS.md` | Tool-specific instructions | Appended after the identity section |

### Behavior

- If **either** `SOUL.md` or `IDENTITY.md` exists, the default identity preamble
  ("You are clawft, a helpful AI assistant...") is replaced with the file contents.
- If **neither** exists, the hardcoded default identity is used.
- `AGENTS.md`, `USER.md`, and `TOOLS.md` are always appended (if they exist),
  regardless of whether a custom identity is active.
- For each file, the workspace root is checked first. If found, the `.clawft/`
  subdirectory is not checked (first match wins).
- Empty files are silently skipped.
```

### Acceptance Criteria

- [ ] Bootstrap file loading order is documented
- [ ] The override behavior (SOUL.md/IDENTITY.md replacing default preamble) is described
- [ ] File search paths (workspace root, then `.clawft/`) are listed
- [ ] All 5 bootstrap files are documented with their purpose

---

## J5: Document Rate-Limit Retry Behavior (P2)

### Problem

The `ClawftLlmAdapter` in `crates/clawft-core/src/pipeline/llm_adapter.rs` implements a 3-retry loop with exponential backoff on rate-limit (HTTP 429) errors. This is not documented in the providers guide.

Evidence from lines 103-129:
```rust
const MAX_RETRIES: u32 = 3;
let mut last_err = String::new();

for attempt in 0..=MAX_RETRIES {
    match self.provider.complete(&request).await {
        Ok(response) => return Ok(convert_response_to_value(&response)),
        Err(ProviderError::RateLimited { retry_after_ms }) => {
            if attempt == MAX_RETRIES {
                last_err = format!("rate limited after {} retries", MAX_RETRIES);
                break;
            }
            let backoff_floor = 1000u64 * 2u64.pow(attempt);
            let wait = retry_after_ms.max(backoff_floor);
            // ...
            tokio::time::sleep(std::time::Duration::from_millis(wait)).await;
        }
        Err(e) => return Err(e.to_string()),
    }
}
```

### Target File

`docs/guides/providers.md` -- add to the "Error Handling" section or as a new "Retry Behavior" section.

**Text to add:**
```markdown
## Retry Behavior

The `ClawftLlmAdapter` (the bridge between the pipeline and `clawft-llm` providers)
automatically retries rate-limited requests (HTTP 429) with exponential backoff:

| Attempt | Backoff Floor | Actual Wait |
|---------|---------------|-------------|
| 1 | 1000 ms | max(provider-suggested, 1000 ms) |
| 2 | 2000 ms | max(provider-suggested, 2000 ms) |
| 3 | 4000 ms | max(provider-suggested, 4000 ms) |

- **Maximum retries**: 3 (4 total attempts including the initial request)
- **Backoff strategy**: Exponential with `1000 * 2^attempt` floor
- **Provider hint**: If the provider's `retry_after_ms` is larger than the
  backoff floor, the provider's suggestion is used instead
- **Non-retriable errors**: Authentication failures, model-not-found, parse
  errors, and network failures fail immediately without retry

After exhausting all retries, a "rate limited after 3 retries" error is returned.

Note: This retry logic is separate from the `RetryPolicy` configuration in
`docs/reference/config.md`, which applies at the routing tier level.
```

### Acceptance Criteria

- [ ] Retry count (3) is documented
- [ ] Backoff strategy (exponential with floor) is described
- [ ] Provider-suggested wait override behavior is noted
- [ ] Non-retriable error types are listed

---

## J6: Document CLI Log Level Change (P2)

### Problem

The default non-verbose log level was changed from `info` to `warn`. This is not reflected in `docs/reference/cli.md`.

Evidence from `crates/clawft-cli/src/main.rs` line 274:
```rust
let default_filter = if cli.verbose { "debug" } else { "warn" };
```

### Target File

`docs/reference/cli.md` -- update the Global Flags section.

**Current text (implied behavior from docs):**
No mention of default log level.

**Text to add (in the Global Flags section or a new "Logging" section):**
```markdown
### Logging

The default log level is `warn`. Only warnings and errors are displayed unless
overridden.

| Configuration | Log Level |
|---------------|-----------|
| Default (no flags) | `warn` |
| `--verbose` / `-v` | `debug` |
| `RUST_LOG=info` | `info` (overrides both default and `--verbose`) |
| `RUST_LOG=clawft_core=trace` | Per-crate override |

The `RUST_LOG` environment variable takes precedence over the `--verbose` flag.
When `RUST_LOG` is set, it is used directly; when unset, the `--verbose` flag
selects between `warn` (default) and `debug`.
```

### Acceptance Criteria

- [ ] Default log level (`warn`) is documented
- [ ] `--verbose` flag behavior (switches to `debug`) is documented
- [ ] `RUST_LOG` precedence over `--verbose` is noted

---

## J7: Plugin System Documentation (P2)

### Problem

The plugin/skill system framework needs initial documentation in preparation for Element 04 (Plugin & Skill System). Full J7 completion is deferred until after C1-C6 land.

### Scope for Element 03

- Document the existing `Channel` / `ChannelHost` / `ChannelFactory` trait architecture as the plugin framework foundation
- Document the `ToolProvider` / `CompositeToolProvider` extensibility model from the MCP subsystem
- Add a "Plugin Architecture" placeholder section noting that the full plugin system (WASM host, skill loader v2) is coming in Element 04

### Dependencies

- **After C1-C6 (Element 04)**: Full plugin system docs including WASM host, skill loader v2, and plugin lifecycle

### Target Files

- `docs/architecture/overview.md` -- add "Plugin Architecture" placeholder section
- `docs/guides/skills-and-agents.md` -- reference future plugin system

### Acceptance Criteria

- [ ] Existing channel plugin architecture is documented as the framework foundation
- [ ] ToolProvider extensibility model is referenced
- [ ] Clear "coming in Element 04" marker for WASM host and skill loader v2
- [ ] No stale claims about plugin capabilities that don't yet exist
