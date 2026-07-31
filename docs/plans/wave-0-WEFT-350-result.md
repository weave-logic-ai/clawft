# WEFT-350 result — Phase 2 voice + streaming chat path

**Branch:** `feat/weft-350-voice-stream-chat`  
**Base:** `release/0.8-staging`  
**Status:** Implemented (agent-core)

## Ticket

ws11: agent-core — Phase 2 voice + streaming chat path.

Gap: `TurnContent::Audio` / `Mixed` existed as wire shapes but were never
constructed; `agent.chat_stream` cascaded typewriter frames without
partial-token `delta` or TTS `speakable` units; no STT→chat_stream→TTS
hook docs.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Loop populates `TurnContent::Audio` / `Mixed` when audio inputs arrive | **Done** — `Turn.audio` + sink `content_rich` / `content_type` |
| Streaming chat path usable for voice (partial tokens / stream events) | **Done** — `delta` + `speakable` on `AgentChatStreamFrame`; daemon cascade uses `cascade_stream_frames` |
| Voice + text mixed turns persisted via substrate sink | **Done** — Mixed when transcript + `AudioRef` |
| Voice-friendly hooks / docs for STT→chat_stream→TTS | **Done** — `voice_stream` module + `docs/guides/voice-stream-chat.md` |
| Tests for stream path | **Done** — types + service-agent + substrate_sink |
| Browser streaming alignment (WEFT-390) | **Aligned** — same progressive semantics; voice prefers substrate frames |

## Design

```text
STT transcript + optional AudioRef
    → agent.chat_stream (AgentChatMessage.audio | metadata.audio)
    → loop sink_append_user → TurnContent::Mixed|Audio|Text
    → cascade frames: { text, delta, speakable, phase, seq, done }
    → TTS consumes speakable units
```

## Files

| Path | Change |
|------|--------|
| `crates/clawft-types/src/turn_content.rs` | **New** — `TurnContent`, `AudioRef`, `audio_from_metadata` |
| `crates/clawft-types/src/agent_chat.rs` | Message `audio`; stream `delta`/`speakable`; `SpeakableTracker`; `cascade_stream_frames` |
| `crates/clawft-core/src/agent/sink.rs` | `Turn.audio`; `Turn::plain` / `with_audio` |
| `crates/clawft-core/src/agent/loop_core.rs` | `sink_append_user` + metadata audio extract |
| `crates/clawft-service-agent/src/substrate_sink.rs` | Persist `content_type` + `content_rich` from `TurnContent` |
| `crates/clawft-service-agent/src/voice_stream.rs` | **New** — STT→params, TTS unit helpers |
| `crates/clawft-service-agent/src/service.rs` | Promote message audio into inbound metadata/media |
| `crates/clawft-weave/src/daemon.rs` | Voice cascade via `cascade_stream_frames` |
| `docs/guides/voice-stream-chat.md` | **New** — operator / integrator guide |

## Tests

```bash
cargo test -p clawft-types --lib
# 402 passed (incl. turn_content + cascade + stream voice fields)

cargo test -p clawft-service-agent --lib voice_stream
# 4 passed

cargo test -p clawft-service-agent --lib service::tests
# inbound audio promotion + existing adapters

cargo test -p clawft-service-agent --test substrate_sink
# 19 passed (incl. append_turn_with_audio_persists_mixed_content)

cargo test -p clawft-core --lib agent::sink
# 11 passed

cargo check -p clawft-service-agent -p clawft-types -p clawft-core -p clawft-weave
# ok
```

## Follow-ups

- Mid-generation LLM tokens writing the same frame shape (true streaming
  into `SpeakableTracker` instead of post-dispatch cascade).
- Publish assistant-side TTS audio refs as `TurnContent::Mixed` on
  outbound turns once synthesis returns a substrate path.
