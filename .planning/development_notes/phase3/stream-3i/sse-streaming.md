# GAP-11: SSE Streaming Implementation

**Date**: 2026-02-17
**Agent**: 3I-A (SSE Streaming)
**Status**: COMPLETE
**Gap ID**: GAP-11

## Summary

Implemented SSE (Server-Sent Events) streaming for the LLM pipeline, enabling real-time token-by-token response output. This resolves GAP-11 from the 3I gap analysis.

## Changes Made

### New Files

1. **`clawft/crates/clawft-llm/src/sse.rs`** (270 lines)
   - SSE line parser (`parse_sse_line()`)
   - Handles `data:` lines, `[DONE]` sentinel, empty lines, comments
   - Converts `StreamDelta` JSON into `StreamChunk` variants
   - 24 unit tests covering all edge cases

### Modified Files

2. **`clawft/crates/clawft-llm/src/types.rs`**
   - Added `StreamChunk` enum: `TextDelta`, `ToolCallDelta`, `Done`
   - Added internal types: `StreamDelta`, `StreamDeltaChoice`, `StreamDeltaContent`, `StreamDeltaToolCall`, `StreamDeltaFunction`, `StreamDeltaUsage`
   - These types mirror the OpenAI `chat.completion.chunk` SSE format

3. **`clawft/crates/clawft-llm/src/provider.rs`**
   - Added `complete_stream()` method to `Provider` trait with default implementation that returns "not supported"
   - Uses `tokio::sync::mpsc::Sender<StreamChunk>` as the streaming channel

4. **`clawft/crates/clawft-llm/src/openai_compat.rs`**
   - Implemented `complete_stream()` on `OpenAiCompatProvider`
   - Sends streaming request with `stream: true`
   - Reads SSE byte stream via `reqwest::Response::bytes_stream()`
   - Buffers partial lines, parses complete lines via `parse_sse_line()`
   - Sends `StreamChunk` values to the channel
   - Handles receiver-drop gracefully (stops processing)

5. **`clawft/crates/clawft-llm/src/lib.rs`**
   - Registered `sse` module
   - Re-exported `parse_sse_line` and `StreamChunk`

6. **`clawft/crates/clawft-llm/Cargo.toml`**
   - Added `futures-util` dependency (for `StreamExt` on byte stream)

7. **`clawft/crates/clawft-core/src/pipeline/traits.rs`**
   - Added `StreamCallback` type alias: `Box<dyn Fn(&str) -> bool + Send + Sync>`
   - Added `complete_stream()` to `LlmTransport` trait with default fallback
   - Added `complete_stream()` to `PipelineRegistry` orchestrating stages 1-6

8. **`clawft/crates/clawft-core/src/pipeline/transport.rs`**
   - Added `complete_stream()` to `LlmProvider` trait with default "not supported"
   - Implemented `complete_stream()` on `OpenAiCompatTransport`
   - Spawns streaming task, forwards text deltas to callback, handles partial responses
   - 4 new streaming tests

9. **`clawft/crates/clawft-core/src/pipeline/llm_adapter.rs`**
   - Implemented `complete_stream()` on `ClawftLlmAdapter`
   - Bridges clawft-llm's `StreamChunk`-based API to clawft-core's `String`-based channel
   - Accumulates text for final synthetic `ChatResponse`

## Architecture

```
CLI/Channel
    |
    v
PipelineRegistry::complete_stream(request, callback)
    |
    +---> Stage 1: Classify
    +---> Stage 2: Route
    +---> Stage 3: Assemble context
    +---> Stage 4: LlmTransport::complete_stream()
    |         |
    |         v
    |    OpenAiCompatTransport::complete_stream()
    |         |
    |         v
    |    ClawftLlmAdapter::complete_stream()
    |         |
    |         v
    |    OpenAiCompatProvider::complete_stream()
    |         |
    |         +---> HTTP POST (stream: true)
    |         +---> reqwest bytes_stream()
    |         +---> parse_sse_line() per line
    |         +---> StreamChunk -> mpsc channel
    |
    +---> Stage 5: Score (after stream done)
    +---> Stage 6: Learn (after stream done)
```

## Streaming Data Flow

1. `OpenAiCompatProvider` sends HTTP request with `stream: true`
2. SSE bytes arrive in chunks from the server
3. Each line is parsed by `parse_sse_line()` into `StreamChunk` values
4. `StreamChunk::TextDelta` -> forwarded to adapter's string channel -> callback
5. `StreamChunk::ToolCallDelta` -> accumulated (not forwarded to text callback)
6. `StreamChunk::Done` -> stream ends, final response assembled
7. Callback returns `false` -> early abort, stream stops cleanly

## Test Coverage

- 24 SSE parser tests (parse_sse_line edge cases)
- 4 streaming transport tests (collect, abort, fallback, stub)
- Total new tests: 28
- All pre-existing tests continue to pass

## Design Decisions

1. **Channel-based streaming**: Used `tokio::sync::mpsc` instead of `Pin<Box<dyn Stream>>` to avoid adding `futures` as a required dependency of clawft-llm. The channel approach is simpler and works well with tokio.

2. **Callback-based pipeline streaming**: The pipeline uses `StreamCallback = Box<dyn Fn(&str) -> bool>` which is simpler for CLI consumers (just print each delta) while still supporting abort.

3. **Default fallback**: Both `Provider::complete_stream` and `LlmTransport::complete_stream` have default implementations, so existing code that only uses `complete()` is unaffected.

4. **Partial response recovery**: If the stream fails mid-way but text was collected, the transport returns a partial response rather than losing the output.

5. **SSE parser is standalone**: `parse_sse_line()` is a pure function with no side effects, making it easy to test and reuse.
