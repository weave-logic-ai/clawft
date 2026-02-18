# Web Search Config Fix (GAP-03) and JSON Repair (GAP-14)

**Date**: 2026-02-17
**Stream**: 3I (Gap Analysis Sprint)
**Status**: COMPLETE
**Gaps Resolved**: GAP-03 (P0), GAP-14 (P0)

---

## GAP-03: Web Search Tool Config Mismatch

### Problem

The `WebSearchTool` accepted an `endpoint: Option<String>` but the config type
(`WebSearchConfig` in clawft-types) had `api_key: String`. The `register_all`
function in clawft-tools was passing `None` as the endpoint, meaning web search
was always unconfigured regardless of the user's config.json settings.

### Solution

Replaced the raw `endpoint` parameter with a proper `WebSearchConfig` struct in
`clawft-tools/src/web_search.rs` that supports two modes:

1. **Brave Search API key mode**: Set `api_key` and the tool constructs the
   Brave Search endpoint (`https://api.search.brave.com/res/v1/web/search`)
   automatically, adding the `X-Subscription-Token` header.

2. **Custom endpoint mode**: Set `endpoint` for self-hosted or alternative
   search APIs. Takes precedence over API key when both are set.

### Changes

| File | Change |
|------|--------|
| `clawft-tools/src/web_search.rs` | Rewrote to accept `WebSearchConfig` struct. Added `build_request()` method. Added `from_endpoint()` for backward compatibility. |
| `clawft-tools/src/lib.rs` | Updated `register_all` signature to accept `WebSearchConfig` parameter. |
| `clawft-cli/src/commands/agent.rs` | Added `build_web_search_config()` helper. Updated `register_all` call. Resolves API key from env var `BRAVE_SEARCH_API_KEY` as fallback. |
| `clawft-cli/src/commands/gateway.rs` | Updated `register_all` call to pass web search config. |

### Config Resolution Order

1. `config.tools.web.search.api_key` (from config.json)
2. `BRAVE_SEARCH_API_KEY` environment variable (if config key is empty)
3. Not configured (tool returns informative error message)

### Tests Added

- `config_not_configured_when_empty` -- empty config is not configured
- `config_configured_with_api_key` -- API key makes it configured
- `config_configured_with_endpoint` -- endpoint makes it configured
- `config_not_configured_with_empty_strings` -- empty strings are not configured
- `build_request_brave_api_key` -- verifies URL and auth headers for Brave mode
- `build_request_custom_endpoint` -- verifies custom endpoint takes precedence
- `build_request_encodes_special_chars` -- URL encoding in queries
- `from_endpoint_with_some/none` -- backward compatibility

---

## GAP-14: JSON Repair for Malformed LLM Output

### Problem

LLMs frequently return tool call arguments with common JSON defects:
- Wrapped in markdown fences (` ```json ... ``` `)
- Trailing commas after last element
- Unquoted object keys (`{name: "value"}`)
- Truncated output (missing closing brackets/braces/quotes)

The existing code in `transport.rs` used `serde_json::from_str(arguments).unwrap_or({})`,
silently discarding tool call arguments on any parse failure.

### Solution

Created `clawft-core/src/json_repair.rs` (~320 lines including tests) with a
4-stage repair pipeline:

1. **Strip markdown fences**: Removes ` ```json ` / ` ``` ` wrappers
2. **Fix trailing commas**: Removes commas before `]` and `}`
3. **Fix unquoted keys**: Quotes bare identifiers followed by `:`
4. **Close truncated structures**: Appends missing `"`, `]`, `}` to balance

All stages respect string literals (do not modify content inside quoted strings)
and handle escape sequences.

### Integration Point

Wired into `clawft-core/src/pipeline/transport.rs` in the `convert_response()`
function. The `serde_json::from_str(arguments)` call was replaced with
`crate::json_repair::parse_with_repair(arguments)`. This uses a fast path
(direct parse) when the JSON is valid, falling back to repair only on failure.

### Changes

| File | Change |
|------|--------|
| `clawft-core/src/json_repair.rs` | NEW -- 4-stage JSON repair with 38 tests |
| `clawft-core/src/lib.rs` | Added `pub mod json_repair;` |
| `clawft-core/src/pipeline/transport.rs` | Replaced `serde_json::from_str` with `parse_with_repair` for tool call arguments |

### Test Coverage (38 tests)

- **Fence stripping**: json fence, plain fence, whitespace, no fence, missing close
- **Trailing commas**: object, array, nested, whitespace, preserved in strings
- **Unquoted keys**: simple, multiple, nested, underscore, preserved quoted keys, boolean values not quoted
- **Truncated**: missing brace, missing bracket, nested, balanced, unclosed strings
- **Integration**: fenced+trailing, truncated+unquoted, all combined, valid unchanged, deeply truncated
- **parse_with_repair**: fast path, repair path, truly broken, empty object, fenced array
- **Edge cases**: escaped quotes, empty input, just fences, multiple trailing commas
- **Real LLM scenarios**: tool call with fences+trailing comma, truncated tool call

### Performance Note

The `parse_with_repair` function tries `serde_json::from_str` first. Only on
failure does it run the 4-stage repair pipeline. This means there is zero
overhead for well-formed JSON (the common case). The repair pipeline itself is
O(n) in the input length with no allocations beyond the output string.

---

## Verification

```
cargo test -p clawft-tools -p clawft-core: 519 passed, 0 failed
cargo clippy -p clawft-core -p clawft-tools --no-deps -- -D warnings: 0 warnings
cargo test -p clawft-cli: 201 passed, 0 failed
```
