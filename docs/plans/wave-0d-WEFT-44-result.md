# WEFT-44 result — service-llm non-string content (vision / structured)

**Ticket:** WEFT-44  
**Branch:** `wave0d/weft-44-llm-content`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-44 (wave-0d)  
**Plane id:** `fe86f600-79a6-43f7-a3d1-3e0d2b58418c`

## Problem

`ChatMessage.content` was a plain `String` with a null→`""` deserializer
(8b05d868 / Nemotron tool-call path). OpenAI vision and Anthropic-style
APIs emit `content` as an **array of content blocks**:

```json
"content": [
  {"type": "text", "text": "What is in this image?"},
  {"type": "image_url", "image_url": {"url": "…", "detail": "auto"}}
]
```

Serde rejected the array (`expected a string`), so vision / multimodal
responses failed as `LlmError::Malformed` instead of parsing cleanly.

## What shipped

### `clawft-service-llm` — typed content

| Type | Role |
|------|------|
| `MessageContent` | `Text(String)` \| `Blocks(Vec<ContentBlock>)` |
| `ContentBlock` | `{ type, text?, image_url?, source? }` — resilient struct |
| `ImageUrl` | OpenAI `{ url, detail? }` |

- **Deserialize** accepts string \| array \| null \| missing (null/missing → empty `Text`).
- **Serialize** emits string for `Text`, array for `Blocks` (request wire stays correct).
- **`as_text` / `into_text` / `is_empty` / `blocks`** — flatten text parts for string consumers; image-only arrays → `""`.
- **`PartialEq<str>` / `Display` / `From<String>`** — ergonomic comparisons and constructors.
- **`ChatMessage::with_blocks`** — multimodal request constructor.

### Consumers adapted (flatten Array → String where RPC/string expected)

| File | Change |
|------|--------|
| `crates/clawft-core/src/pipeline/service_llm_adapter.rs` | `value_to_content` preserves blocks; null→empty text |
| `crates/clawft-weave/src/daemon.rs` | `llm.prompt` wraps RPC strings; completion uses `as_text()` |
| `crates/clawft-weave/src/conv_postmortem.rs` | `content.as_text().trim()` |
| `crates/clawft-core/src/agent/context_router/llm_classifier.rs` | classifier body via `as_text()` |

### Tests

- Unit: plain string, null (Nemotron), missing, OpenAI vision blocks, Anthropic-style image+text, multi-text flatten, image-only empty, serialize text/blocks.
- Integration (wiremock): vision-shaped assistant response, Anthropic-style response array; existing null-content tool-call regression retained.
- Adapter: vision blocks preserved inbound; null content → empty text.

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Typed enum String \| Array\<ContentBlock\> \| Null | **Done** (`MessageContent`; null → empty `Text`) |
| Consumers adapt (flatten for string paths) | **Done** |
| Fixtures: OpenAI vision, Anthropic-style, Nemotron null | **Done** |
| Backward-compat: string responses still parse | **Done** (existing tests green) |

## Verification

```bash
scripts/build.sh check          # pass
scripts/build.sh test clawft-service-llm   # 36 passed
cargo nextest run -p clawft-core service_llm_adapter  # 13 passed
cargo clippy -p clawft-service-llm --all-targets -- -D warnings  # pass
```

Note: full-workspace `scripts/build.sh clippy` currently fails on a
pre-existing `collapsible_if` in `loop_core.rs` from WEFT-651
(`ea6ff4aa`) — out of scope for this ticket.

## Worktree / merge

- **Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb45e-9f20-75c3-aedd-b5acb94783b9`
- **Branch:** `wave0d/weft-44-llm-content` (off `release/0.8-staging`)
- **Do not merge to master.**
