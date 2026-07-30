# WEFT-223 result — SC-2 audio buffer zeroization and `voice.audio_retention`

**Ticket:** WEFT-223  
**Branch:** `wave0k/weft-223-audio-zeroize`  
**Wave:** 0k  
**Date:** 2026-07-30  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e93-77a3-a283-b6ad27013fb7`  
**Status:** Implementation complete

## Problem

No zeroization of raw audio buffers after use; no `voice.audio_retention` config (`none` / `session` / `persist`). Raw PCM could linger in heap memory (and potentially on disk) longer than necessary (audit SC-2 / `.planning/sparc/voice/06-voice-security-review.md`).

## What shipped

### 1. Config — `voice.audio_retention` (`clawft-types`)

| Variant | Memory | Disk |
|---------|--------|------|
| **`none`** (default) | zeroize after STT / drop | never |
| **`session`** | hold until session end, then zeroize | never |
| **`persist`** | as session | may write under `{workspace}/.clawft/audio/` |

- Field: `VoiceConfig.audio_retention` with serde aliases `audio_retention` / `audioRetention`
- Wire form: lowercase strings `"none"` | `"session"` | `"persist"`
- Helpers: `allows_disk_write()`, `allows_session_hold()`, `is_none()`

### 2. Zeroize crate + secure PCM types

- Workspace pin: `zeroize = { version = "1.8", features = ["derive"] }`
- Direct deps: `clawft-types`, `clawft-plugin`
- New module `clawft_types::audio_buffer`:
  - **`SecureAudioBuffer`** — owned `Vec<i16>`; zeroizes on `Drop` / `clear`
  - **`SecureAudioRing`** — fixed-capacity ring; zeroizes on eviction and drop
  - **`zeroize_samples` / `zeroize_vec`** — helpers for bare slices/vecs
  - Debug/Display redact sample values (`[REDACTED]`)

### 3. Capture honours retention (`clawft-plugin` voice scaffold)

`AudioCapture`:

- Always maintains a ~400 ms `SecureAudioRing` (bounds memory exposure)
- `AudioRetention::None` (default): no session accumulation; clear ring on stop
- `Session`: accumulate in `SecureAudioBuffer`; no disk
- `Persist`: `persist_session_audio(workspace, name)` → `{workspace}/.clawft/audio/{name}.pcm`
- Drop always clears session + ring (zeroize via secure types)

### 4. Transcript writer honours retention

`TranscriptLogger`:

- Default retention `None`
- `with_retention(...)` constructor
- Optional `TranscriptEntry.audio_path` **stripped** unless retention is `Persist`
- Never embeds sample bytes in JSONL (text-only by default)

### 5. Live Talk Mode path (`clawft-channels`)

- Pre-onset ring is `SecureAudioRing` (was bare `VecDeque<i16>`)
- `FinalizedTurn` zeroizes samples on drop
- `Utterance` zeroizes samples on drop + redacted `Debug`
- Abandoned sub-minimum blips and leftover in-progress utterance zeroized on exit

## Acceptance mapping

| AC | Status |
|----|--------|
| zeroize crate added; capture and ring buffers zeroed on drop | **Done** |
| `voice.audio_retention` in clawft-types; honoured by capture and transcript writer | **Done** |
| Default: `none` (do not retain raw audio) | **Done** |
| Tests | **Done** (types + plugin capture/transcript + channels still green) |
| `scripts/build.sh test --features voice` passes | **Done** (scoped: `clawft-types clawft-plugin clawft-channels`) |

## Files changed

| File | Change |
|------|--------|
| `Cargo.toml` | Workspace `zeroize` pin |
| `Cargo.lock` | Lockfile update |
| `crates/clawft-types/Cargo.toml` | `zeroize` dep |
| `crates/clawft-types/src/lib.rs` | `audio_buffer` module + re-exports |
| `crates/clawft-types/src/audio_buffer.rs` | **New** — SecureAudioBuffer/Ring + tests |
| `crates/clawft-types/src/config/voice.rs` | `AudioRetention` + `VoiceConfig.audio_retention` |
| `crates/clawft-types/src/config/mod.rs` | Default + serde tests |
| `crates/clawft-plugin/Cargo.toml` | `clawft-types` + `zeroize` |
| `crates/clawft-plugin/src/voice/mod.rs` | Re-export retention/secure types |
| `crates/clawft-plugin/src/voice/capture.rs` | SC-2 ring/session/persist |
| `crates/clawft-plugin/src/voice/transcript_log.rs` | SC-2 path gating |
| `crates/clawft-channels/src/voice/mod.rs` | Re-export SC-2 helpers |
| `crates/clawft-channels/src/voice/talkmode.rs` | Secure preroll + zeroize on drop |
| `crates/clawft-channels/src/voice/stt.rs` | `Utterance` Drop zeroize |
| `docs/plans/wave-0k-WEFT-223-result.md` | This result |

## How to test

```bash
# Focused SC-2 units
cargo test -p clawft-types --lib audio_buffer
cargo test -p clawft-types --lib voice_audio_retention
cargo test -p clawft-plugin --features voice --lib voice::capture
cargo test -p clawft-plugin --features voice --lib voice::transcript_log
cargo test -p clawft-channels --features voice --lib voice::

# AC command (packages first, then --features)
scripts/build.sh test clawft-types clawft-plugin clawft-channels --features voice
```

### Verified this worktree

- `scripts/build.sh test clawft-types clawft-plugin clawft-channels --features voice` → **1051 passed**, 2 skipped
- SC-2 filter: capture retention (5) + transcript retention (3) + audio_buffer (5) + config serde (1) all **PASS**

## Residual / follow-ups

1. **Wire config at bootstrap** — daemon / talk-mode binder should pass `cfg.voice.audio_retention` into `AudioCapture::with_retention` and `TranscriptLogger::with_retention` (plumbing only; types + honour sites exist).
2. **cpal production path** — when real `cpal::Stream` lands in plugin capture (or stays in `clawft-voice-aec`), funnel frames through `push_frame` / `SecureAudioRing` so the contract cannot regress.
3. **Session purge on exit** — `Session` retention currently lives only while the capture handle is alive; a longer-lived session store would need an explicit end-of-session `clear_session` call from the Talk Mode controller.

## Commit

- **Branch:** `wave0k/weft-223-audio-zeroize`
- **Message:** `feat(voice): WEFT-223 SC-2 audio zeroize + voice.audio_retention`
