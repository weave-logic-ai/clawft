# Voice Guide

This guide covers the voice pipeline in clawft: speech-to-text (STT) via the
browser, cloud text-to-speech (TTS) through server-side proxying, and the
talk mode overlay for hands-free continuous conversation.

---

## 1. Overview

Voice features enable natural speech interaction with your weft agent. The
pipeline splits work between the browser and the server:

- **Speech-to-text (STT)** runs entirely in the browser via the Web Speech
  API. No server-side transcription service is required.
- **Text-to-speech (TTS)** supports three providers: browser-native Web
  Speech API (default), OpenAI TTS, and ElevenLabs. Cloud providers are
  proxied through the server to keep API keys off the client.
- **Talk mode** provides a full-screen overlay with waveform visualization,
  continuous listening, and tap-to-interrupt, enabling hands-free
  conversation with the agent.

---

## 2. Enabling Voice

Voice requires two configuration flags:

```json
{
  "voice": {
    "enabled": true
  },
  "gateway": {
    "api_enabled": true
  }
}
```

| Field | Purpose |
|-------|---------|
| `voice.enabled` | Activates the voice pipeline. Default: `false`. |
| `gateway.api_enabled` | Starts the HTTP/WebSocket server that the web dashboard connects to. Voice runs through the dashboard, so this must be `true`. |

The voice pipeline is browser-side for STT and server-side for cloud TTS
proxying. When both flags are enabled, the web dashboard exposes the voice
panel and talk mode controls.

---

## 3. TTS Provider Configuration

The `voice.tts` section selects which text-to-speech engine produces audio.
Only one provider is active at a time.

### 3.1 Browser (Default)

Uses the Web Speech API built into the browser. No API key is needed.

```json
{
  "voice": {
    "tts": {
      "provider": "browser"
    }
  }
}
```

Quality is functional but robotic. This is the zero-configuration fallback
that works in all modern browsers supporting the `SpeechSynthesis` interface.

### 3.2 OpenAI TTS

High-quality neural voices from OpenAI.

```json
{
  "voice": {
    "tts": {
      "provider": "openai",
      "model": "tts-1",
      "voice": "alloy",
      "speed": 1.0
    }
  }
}
```

**Models:**

| Model | Description |
|-------|-------------|
| `tts-1` | Fast, lower latency, cheaper. Good for real-time conversation. |
| `tts-1-hd` | Higher quality audio. Better for pre-recorded or polished output. |

**Voices:**

| Voice | Character |
|-------|-----------|
| `alloy` | Neutral, balanced |
| `echo` | Warm, conversational |
| `fable` | Expressive, storytelling |
| `onyx` | Deep, authoritative |
| `nova` | Friendly, upbeat |
| `shimmer` | Soft, gentle |

**Speed:** Accepts values from `0.25` to `4.0`. Default is `1.0`.

**API key:** Set `providers.openai.apiKey` in the config file, or set the
`OPENAI_API_KEY` environment variable. The config value takes precedence
when both are present.

### 3.3 ElevenLabs TTS

Highest quality and most natural sounding voices.

```json
{
  "voice": {
    "tts": {
      "provider": "elevenlabs",
      "model": "eleven_multilingual_v2",
      "voice": "Rachel"
    }
  }
}
```

**Models:**

| Model | Description |
|-------|-------------|
| `eleven_multilingual_v2` | Best quality, supports 29 languages. |
| `eleven_turbo_v2_5` | Low latency, optimized for real-time use. |
| `eleven_monolingual_v1` | English-only, legacy model. |

**Voices:** Use any voice ID from your ElevenLabs account, or one of the
preset names:

| Preset Name | Character |
|-------------|-----------|
| `Rachel` | Calm, narration |
| `Domi` | Strong, assertive |
| `Bella` | Soft, warm |
| `Antoni` | Well-rounded, male |
| `Josh` | Deep, young male |
| `Adam` | Clear, middle-aged male |
| `Sam` | Raspy, male |

Custom voice IDs from the ElevenLabs voice library or voice cloning are
also supported. Pass the voice ID string directly in the `voice` field.

**API key:** Set `providers.elevenlabs.apiKey` in the config file, or set
the `ELEVENLABS_API_KEY` environment variable. The config value takes
precedence when both are present.

---

## 4. Environment Variables

| Variable | Provider | Description |
|----------|----------|-------------|
| `OPENAI_API_KEY` | OpenAI TTS | Falls back to this when `providers.openai.apiKey` is empty. |
| `ELEVENLABS_API_KEY` | ElevenLabs | Falls back to this when `providers.elevenlabs.apiKey` is empty. |

The config value always takes precedence over the environment variable.
When neither is set and a cloud provider is selected, TTS requests will
fail with an authentication error.

---

## 5. Full Configuration Example

A complete `config.json` snippet with voice, provider keys, and gateway:

```json
{
  "voice": {
    "enabled": true,
    "tts": {
      "provider": "openai",
      "model": "tts-1",
      "voice": "nova",
      "speed": 1.0
    }
  },

  "providers": {
    "openai": {
      "apiKey": "sk-..."
    },
    "elevenlabs": {
      "apiKey": "xi-..."
    }
  },

  "gateway": {
    "host": "0.0.0.0",
    "port": 18790,
    "api_enabled": true
  }
}
```

To switch providers, change `voice.tts.provider` to `"elevenlabs"` or
`"browser"`. The provider-specific fields (`model`, `voice`, `speed`) are
only read for the active provider.

---

## 6. Talk Mode

Talk mode is a full-screen overlay activated from the voice panel in the
web dashboard. It provides hands-free, continuous conversation with the
agent.

### States

The overlay cycles through four states:

| State | Indicator | Behavior |
|-------|-----------|----------|
| **Idle** | Pulsing circle | Waiting for the user to begin speaking. |
| **Listening** | Waveform animation | Capturing speech via Web Speech API. |
| **Processing** | Spinner | Transcript sent to agent, awaiting response. |
| **Speaking** | Waveform playback | TTS audio playing through speakers. |

### Interaction

- **Start listening:** Tap the center icon or begin speaking (continuous
  recognition auto-activates after the agent finishes speaking).
- **Interrupt:** Tap the center icon during the speaking state to stop
  playback and return to the listening state.
- **Exit:** Tap the close button or press Escape to leave talk mode.

Speech recognition runs continuously between responses. After the agent
finishes speaking, the microphone re-engages automatically so the user can
respond without tapping anything.

---

## 7. Voice Mode Prompt

When a message arrives from the voice channel (`chat_id="voice"`), a
system prompt is injected that instructs the LLM to respond in natural
conversational language:

- No Markdown formatting (no headers, bold, lists, or code blocks).
- No URLs or links in the response text.
- Uses contractions and casual phrasing.
- Keeps answers brief and to the point.

The response text is also stripped of any remaining Markdown artifacts
before it is sent to the TTS engine. This prevents the TTS from reading
out formatting characters like asterisks or hash marks.

If voice responses are unexpectedly verbose or formatted, verify that the
inbound message has `chat_id` set to `"voice"` so the voice system prompt
is applied.

---

## 8. Architecture

The voice pipeline has six stages:

```
Browser                              Server
------                              ------

1. Microphone
   |
   v
2. Web Speech API (STT)
   |  transcript text
   v
3. POST /api/sessions/voice/messages -----> Agent pipeline
                                             (6-stage processing)
                                                  |
                                                  v
4. WebSocket broadcast <---------------------- Response text
   |
   v
5. TTS rendering
   |  POST /api/voice/tts (cloud)
   |  -- or --
   |  SpeechSynthesis API (browser)
   |
   v
6. Web Audio API playback
```

**Step by step:**

1. The browser captures speech from the microphone.
2. The Web Speech API transcribes speech to text in real time (STT). This
   runs entirely client-side with no server round-trip.
3. The transcript is sent to the backend via `POST /api/sessions/voice/messages`.
   The backend tags the message with `chat_id="voice"` so the voice system
   prompt is applied.
4. The agent processes the message through the standard 6-stage pipeline
   (context assembly, routing, LLM call, tool execution, response
   formatting, delivery). The response is broadcast to the talk mode
   overlay via WebSocket.
5. The response text is rendered to audio. For cloud providers, the
   dashboard sends the text to `POST /api/voice/tts`, which proxies the
   request to OpenAI or ElevenLabs and returns an audio buffer. For the
   browser provider, the `SpeechSynthesis` API generates audio locally.
6. The audio is played back through the Web Audio API, and the overlay
   transitions to the speaking state with waveform visualization.

---

## 9. Troubleshooting

### "Speech recognition not supported"

The Web Speech API is not available in this browser. Use a Chromium-based
browser (Chrome, Edge, Brave) or Safari. Firefox does not support the Web
Speech API for speech recognition.

### "No API key configured"

A cloud TTS provider is selected but no API key was found. Set the
environment variable (`OPENAI_API_KEY` or `ELEVENLABS_API_KEY`) or add the
key to the `providers` section of your config file.

### TTS sounds robotic

The browser TTS provider uses the operating system's built-in speech
synthesis, which varies in quality. Switch to `"openai"` or `"elevenlabs"`
for neural-quality voices:

```json
{
  "voice": {
    "tts": {
      "provider": "openai",
      "model": "tts-1",
      "voice": "nova"
    }
  }
}
```

### Cannot interrupt speech

Tap the center icon during the speaking state. The overlay must be in the
speaking state (waveform playback animation) for interrupt to take effect.
If the overlay is in the processing state (spinner), the audio has not
started yet and there is nothing to interrupt.

### Voice responses are too verbose

The voice mode system prompt instructs the LLM to keep responses concise.
If responses are unexpectedly long or formatted with Markdown, verify that:

1. The inbound message has `chat_id` set to `"voice"`.
2. The voice system prompt is not being overridden by a `SOUL.md` or
   `IDENTITY.md` file that conflicts with the conversational instructions.

### No audio playback

Check that the browser has permission to play audio. Some browsers require
a user interaction (click or tap) before allowing audio playback. Talk mode
handles this by requiring the user to tap the icon to start, which counts
as a user interaction.

### High latency on TTS

- Switch from `tts-1-hd` to `tts-1` (OpenAI) for faster generation.
- Switch from `eleven_multilingual_v2` to `eleven_turbo_v2_5` (ElevenLabs)
  for lower latency.
- Use the `"browser"` provider for zero-latency local synthesis at the
  cost of voice quality.

---

## Further Reading

- [Configuration Guide](configuration.md) -- Full config file reference.
- [Providers Guide](providers.md) -- Provider routing and API key management.
- [Channels Guide](channels.md) -- Channel plugin architecture.
