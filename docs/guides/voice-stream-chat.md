# Voice + streaming chat path (WEFT-350)

Connect STT → `agent.chat_stream` → TTS using the agent-core substrate
stream. This is the Phase 2 voice path for the chat agent (see
`docs/plans/chat-agent-v1.md` §17).

## Pipeline

```text
 mic / file
    │
    ▼
 STT (audio_transcribe / voice_listen / parakeet)
    │  transcript + optional AudioRef (substrate path to PCM)
    ▼
 agent.chat_stream  { messages: [{role, content, audio?}], conv_id, metadata }
    │
    ├─ user turn persisted as TurnContent::Text | Audio | Mixed
    │     substrate/_derived/chat/<conv>/turns/<ulid>
    │
    └─ progressive frames  substrate/_derived/chat/<conv>/stream
           { text, delta, speakable, phase, seq, done }
                    │
                    ▼
              TTS (audio_synthesize / voice_speak / Orpheus)
                 consume `speakable` units as they close
```

## Wire shapes

### Input audio on the chat RPC

Either attach audio on the trailing user message:

```json
{
  "conv_id": "voice-1",
  "messages": [{
    "role": "user",
    "content": "what's the weather",
    "audio": {
      "substrate_path": "substrate/_derived/chat/voice-1/audio/01HQ…",
      "mime": "audio/wav",
      "duration_ms": 1400
    }
  }]
}
```

…or put the same object under `metadata.audio` (or flattened
`metadata.audio_substrate_path` + optional mime/duration).

### Stream frames (voice-friendly)

| Field | Role |
|-------|------|
| `text` | Accumulated assistant draft (self-healing for panel poll) |
| `delta` | Partial token(s) since last frame (token UIs / analytics) |
| `speakable` | Closed sentence/clause ready for TTS |
| `phase` | `thinking` / `generating` / `done` / `error` / `awaiting_defer` |
| `seq` | Monotonic frame counter |
| `done` | Terminal marker |

Path: `substrate/_derived/chat/<conv_id>/stream` (same grant as status/meta).

### TurnContent on substrate turns

| Shape | When |
|-------|------|
| `{"text":"…"}` | Text-only turns |
| `{"audio":{…AudioRef}}` | Audio without transcript |
| `{"mixed":[{"text":"…"},{"audio":{…}}]}` | Transcript + utterance audio |

JSONL also keeps flat `content` (text/transcript) and `content_type`
(`text` / `audio` / `mixed`) for legacy readers.

## Helper API (`clawft-service-agent::voice_stream`)

```rust
use clawft_service_agent::voice_stream::{
    build_voice_chat_params, simulate_stream, tts_units_from_frames, AudioRef,
};

// After STT:
let params = build_voice_chat_params(
    "voice-1",
    transcript,
    Some(AudioRef::new(path, "audio/wav", duration_ms)),
);
// → agent.chat_stream with params

// After collecting stream frames (or offline dry-run):
let frames = simulate_stream(&assistant_text);
for unit in tts_units_from_frames(&frames) {
    // voice_speak / audio_synthesize(unit)
}
```

Types also live in `clawft_types::turn_content` and
`clawft_types::agent_chat::{SpeakableTracker, cascade_stream_frames}`.

## Alignment with browser streaming (WEFT-390)

Browser WASM `stream_chat` and the daemon `agent.chat_stream` path share
the same progressive *semantics* (accumulated text + terminal result).
Voice consumers should prefer the substrate stream frames (`delta` /
`speakable`) so native Talk Mode and the panel stay on one contract.

## Related

- ADR-061 conversational voice agent loop
- WEFT-253 `agent.chat_stream` (panel progressive UI)
- WEFT-390 browser `stream_chat`
- `docs/guides/voice.md` product Talk Mode
