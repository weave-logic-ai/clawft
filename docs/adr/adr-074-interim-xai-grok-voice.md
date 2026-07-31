# ADR-074: Interim primary voice path — xAI Grok Voice (local remains offline/fallback)

**Date**: 2026-07-30  
**Status**: Accepted  
**Deciders**: product + voice (owner: local stack is acceptable long-term but currently **sub-par** for realtime agent control; use best available cloud voice in the interim)  
**Depends-On**: ADR-053 (STT backend seam), ADR-061 (conversational loop shape), ADR-068 (full-duplex edge)  
**Relates-To**: ADR-060 (local Hermes LLM), ADR-073 (voice → `WindowIntent`), xAI Voice docs (`wss://api.x.ai/v1/realtime`, Grok STT/TTS APIs)

## Context

### What we built (and keep)

ADR-061 / 068 commit WeftOS to a **full-duplex, AEC-aware conversational loop** with:

- Native AEC (`clawft-voice-aec`)  
- Local STT backends (Parakeet / whisper via ADR-053)  
- Dual-layer local TTS (fast ack + slow expressive)  
- Talk-Mode in `clawft-channels`  

That architecture remains the **long-term target** (privacy, offline, cost control, no dependency on a single vendor).

### What is failing the product bar today

Measured and qualitative reality mid-2026:

- Local TTS still hits the **voice trilemma** (speed vs clone vs paralinguistics) — ADR-061 already accepted tradeoffs; experience still feels “lab” vs “Jarvis.”  
- End-to-end local latency and summarization quality lag **best-in-class cloud speech-to-speech** for multi-agent conductor demos (spawn agents, summarize walls of text, drive tools).  
- Market demos (e.g. Grok Voice Think Fast **2.0** in CNVS) set the bar users now expect for **voice-orchestrated agents**.

### What xAI offers (vendor facts)

From xAI public docs/news (2026):

| Capability | Surface | Notes |
|------------|---------|--------|
| **Speech-to-speech** | `wss://api.x.ai/v1/realtime` | OpenAI Realtime-compatible shape; models e.g. `grok-voice-latest` → think-fast line; tool use + multi-turn |
| **STT** | streaming + batch | Documented low-latency streaming STT |
| **TTS** | REST / voice APIs | Standalone TTS; multi-voice |
| **Auth** | `XAI_API_KEY` | Already a first-class LLM provider key in WeftOS config docs |

**Owner product call:** use **xAI voice as the interim primary** for Talk-Mode / agent conductor until a local replacement matches quality. Local stack stays default for offline and as fallback.

## Decision

### 1. Split “brain” from “mouth/ears” (already true) — extend the mouth/ears

- **Agent brain** may remain local Hermes / configured LLM (ADR-060) **or** ride inside Grok S2S when using full speech-to-speech mode.  
- Two supported **voice transport modes**:

| Mode | Behavior | When |
|------|----------|------|
| **`xai_s2s` (interim primary when online + key)** | Full-duplex WebSocket speech-to-speech via xAI Realtime; tools mapped to WeftOS `WindowIntent` / agent / MCP | Network + `XAI_API_KEY` |
| **`local` (always available)** | ADR-061 pipeline: AEC → local STT → agent → dual-layer TTS | Offline, no key, or explicit config |
| **`xai_hybrid` (optional)** | Local or xAI STT + agent (Hermes) + xAI TTS | Transition / cost control |

### 2. Config surface

```toml
[voice]
# primary | hybrid | local — see Decision §1
mode = "xai_s2s"   # interim product default when key present; else local

[voice.xai]
# empty → inherit XAI_API_KEY / providers.xai
api_key_env = "XAI_API_KEY"
realtime_model = "grok-voice-latest"   # tracks think-fast line per vendor alias policy
# optional: pin grok-voice-think-fast-2.0 when stable
endpoint = "wss://api.x.ai/v1/realtime"
```

**Selection rule:**

1. If `mode = local` → local only.  
2. Else if no API key / health check fails → **fail open to local** with a single user-visible notice (not silent degrade forever).  
3. Else use xAI path.

### 3. Implementation home

| Piece | Crate / surface |
|-------|-----------------|
| Realtime WebSocket client + session | `clawft-channels` (feature `voice-xai`) **or** thin `clawft-voice-xai` |
| Map vendor tool calls → WeftOS tools | Talk-Mode bridge; **must** implement `WindowIntent` verbs (ADR-073) |
| AEC | Keep native AEC in front of mic when using cloud S2S (still cancel speaker loop) |
| Config | `clawft-types` `VoiceConfig` + docs |
| Secrets | Never log audio; keys via env / existing secret store only |

### 4. Local stack is not deleted

- ADR-061 remains **Accepted** for architecture and for **offline / private** deployments.  
- Continuous improvement of local STT/TTS continues (WEFT-613 Chatterbox, ONNX, etc.) as the **replacement track**.  
- **Exit criterion for dropping xAI as primary:** documented A/B or owner sign-off that local TTFA + intelligibility + tool-calling UX matches or exceeds xAI for conductor demos.

### 5. Privacy / compliance defaults

- Cloud voice is **opt-in via API key** (presence of key + mode).  
- Document that audio leaves the machine in `xai_s2s` / hybrid TTS modes.  
- Enterprise profiles may force `mode = local`.

### 6. Relationship to CNVS demos

CNVS using Grok Voice is **evidence of product fit**, not a dependency. WeftOS binds the same class of voice model to **our** substrate, governance, and Agent Workspace (ADR-073).

## Non-goals

- Replacing Hermes as the only allowed agent brain.  
- Shipping without a local path.  
- Recording/storing raw audio in cloud mode without explicit retention policy (WEFT-223 still applies to local buffers).  
- Implementing every vendor voice (ElevenLabs etc.) before xAI path is solid.

## Implementation phases

| Phase | Cycle | Plane | Work | Exit |
|-------|-------|-------|------|------|
| **V0** | 0.8.x | **WEFT-689** | Config + feature flag; Realtime client connects; echo/hello path | Manual Talk-Mode test with key |
| **V1** | 0.8.x | **WEFT-690** | Tool bridge: spawn/focus agent, summarize, basic shell intents | CNVS-like “voice spawns N agents” on WeftOS shell |
| **V2** | 0.8.x | **WEFT-691** | Hybrid STT/TTS modes; metrics (TTFA, disconnects); graceful local fallback | Failover tested — see `docs/guides/voice.md` (hybrid composition, `VoiceTransportMetrics`, `probe_xai_voice_health` stub, `connect_hello_or_fallback`) |
| **V3** | Later | *(not filed)* | Local quality gate documented; optional demote xAI to non-default | Owner exit criterion |

## Consequences

### Positive

- Product voice quality unblocked while local engines mature.  
- Aligns with existing `XAI_API_KEY` / provider story.  
- Same `WindowIntent` path as keyboard/MCP (ADR-073).

### Negative

- Cloud dependency, cost, and privacy surface.  
- Two code paths to maintain until exit criterion.  
- Vendor model alias churn (`grok-voice-latest` retargets).

## References

- https://docs.x.ai/developers/model-capabilities/audio/speech-to-speech  
- https://docs.x.ai/developers/model-capabilities/audio/voice  
- https://x.ai/news/grok-stt-and-tts-apis  
- ADR-053, ADR-061, ADR-068, ADR-073  
