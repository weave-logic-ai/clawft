# Stream 3I-D: Memory Wiring, Tool Parsing Audit, and Test Coverage

**Agent**: 3I-D
**Date**: 2026-02-17
**Status**: Complete

## Tasks Completed

### Task 1: Memory Wiring (GAP-15) -- ~2h estimate

**What**: Wired VectorStore-based semantic search into `memory_read` tool behind the
`vector-memory` feature flag.

**Files Modified**:
- `crates/clawft-tools/Cargo.toml` -- Added `vector-memory = ["clawft-core/vector-memory"]` feature
- `crates/clawft-tools/src/memory_tool.rs` -- Added `search_paragraphs_vector()` function and cfg-gated `execute()` branch
- `crates/clawft-core/src/embeddings/hash_embedder.rs` -- Made `compute_embedding()` public

**Design Decisions**:
- Used `#[cfg(feature = "vector-memory")]` conditional compilation to avoid adding the
  VectorStore dependency to the default build
- The vector search splits content into paragraphs (blank-line delimited), embeds each
  with `HashEmbedder`, stores in a `VectorStore`, and searches by cosine similarity
- Falls back to the original substring search when the feature is disabled
- HashEmbedder operates synchronously (no async needed), so `compute_embedding` was
  made `pub` to allow direct calls from the tool without an async runtime

**Tests Added**: 5 tests in `memory_tool.rs::vector_tests` module (gated behind
`#[cfg(feature = "vector-memory")]`)

---

### Task 2: Tool Call Parsing Audit (GAP-17) -- ~1h estimate

**What**: Audited `ContentBlock::ToolUse` extraction in `transport.rs::convert_response()`
and added comprehensive edge case tests.

**Files Modified**:
- `crates/clawft-core/src/pipeline/transport.rs` -- Added 11 edge case tests

**Edge Cases Covered**:
1. Missing `tool_use_id` (generates fallback ID)
2. Empty arguments string
3. Nested JSON arguments
4. Missing `function` field in tool call
5. Multiple tool calls in single response
6. Null content with tool calls
7. Missing `message` field entirely
8. Unknown `finish_reason` values
9. Null `finish_reason`
10. Arguments as JSON object (not string)

**Note**: Another concurrent agent modified `transport.rs` to add SSE streaming and
`json_repair` support. My edge case tests are compatible with those changes.

---

### Task 3: Tool Result Truncation (GAP-19) -- ~1h estimate

**What**: Verified that `MAX_TOOL_RESULT_BYTES` (64KB) truncation is enforced in the
agent tool loop, and added a test proving it.

**Files Modified**:
- `crates/clawft-core/src/agent/loop_core.rs` -- Added `OversizedToolTransport`,
  `BigOutputTool`, and `tool_result_truncation_enforced` test

**Verification**: The test creates a tool that produces 200KB of output and confirms
the truncated result sent to the LLM is <= 65,536 bytes.

---

### Task 4: Mock HTTP Provider Tests (TEST-01) -- ~3h estimate

**What**: Created comprehensive mock HTTP server tests for `OpenAiCompatProvider::complete()`
using `wiremock`.

**Files Created**:
- `crates/clawft-llm/tests/mock_http.rs` -- 17 integration tests

**Files Modified**:
- `crates/clawft-llm/Cargo.toml` -- Added `wiremock = "0.6"` to dev-dependencies

**Test Coverage**:
| Test | Scenario |
|------|----------|
| `complete_success_text_response` | 200 OK with text content, usage stats |
| `complete_success_with_tool_calls` | 200 OK with tool_calls in response |
| `complete_401_returns_auth_failed` | 401 -> `ProviderError::AuthFailed` |
| `complete_403_returns_auth_failed` | 403 -> `ProviderError::AuthFailed` |
| `complete_429_returns_rate_limited_with_retry_after` | 429 with `retry_after_ms` |
| `complete_429_default_retry_when_no_retry_after` | 429 without retry hint -> default 1000 |
| `complete_429_with_retry_after_seconds` | 429 with `retry_after` in seconds |
| `complete_404_returns_model_not_found` | 404 -> `ProviderError::ModelNotFound` |
| `complete_500_returns_request_failed` | 500 -> `ProviderError::RequestFailed` |
| `complete_malformed_json_returns_invalid_response` | Bad JSON -> `InvalidResponse` |
| `complete_empty_choices_parses_successfully` | Empty choices array |
| `complete_sends_authorization_header` | Verifies Bearer token header |
| `complete_forwards_custom_headers` | Verifies custom headers forwarded |
| `complete_sends_request_body_correctly` | Full request with tools, temp, max_tokens |
| `complete_missing_api_key_returns_not_configured` | No API key -> `NotConfigured` |
| `complete_multiple_choices` | Multiple choices in response |
| `complete_usage_without_prompt_finish_reason_null` | Null finish_reason |

---

### Task 5: Agent Loop E2E Test (TEST-04) -- ~2h estimate

**What**: Created end-to-end agent loop tests verifying the full tool-use round trip:
mock LLM returns tool_use -> tool executes -> result sent back -> LLM returns text.

**Files Modified**:
- `crates/clawft-core/src/agent/loop_core.rs` -- Added 4 e2e tests with recording transports

**Tests Added**:
| Test | Scenario |
|------|----------|
| `e2e_tool_roundtrip_message_chain` | Full single-tool round trip with message chain verification |
| `e2e_multi_tool_calls_all_results_returned` | Two tool calls in one response, both results returned |
| `e2e_direct_text_response_no_tools` | Direct text response (no tools), session persistence verified |
| `e2e_tool_execution_failure_handled_gracefully` | Unknown tool -> error sent to LLM -> LLM recovers |

**Key Design**: The `E2eRecordingTransport` and `MultiToolE2eTransport` record every
request they receive, enabling assertions on the full intermediate message chain:
- Verifies tool result messages have correct `tool_call_id`
- Verifies tool output content is included in the result
- Verifies session persistence after the full cycle
- Verifies graceful error handling for unknown tools

---

### Additional Fix

**File**: `crates/clawft-cli/src/mcp_tools.rs` line 54
**Issue**: Pre-existing `clippy::collapsible_if` warning
**Fix**: Collapsed nested `if` into `if-let` chain using Edition 2024 let-chains

---

## Test Summary

| Crate | Tests | Status |
|-------|-------|--------|
| clawft-core | 304 | All pass |
| clawft-llm (lib) | 65 | All pass |
| clawft-llm (integration: mock_http) | 17 | All pass |
| clawft-tools | 97 | All pass |
| **Total** | **483** | **All pass** |

**Clippy**: Clean on workspace (non-test), pre-existing warnings in bootstrap.rs test
code and clawft-wasm test code are not from this stream.
