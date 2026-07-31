# WEFT-613 Result: Chatterbox cloned-voice fast tier (native port)

**Date**: 2026-07-31  
**Ticket**: WEFT-613  
**Branch / worktree**: `release/0.8-staging` (Grok Build worktree)  
**Status**: **Partial close — scaffold + selection shipped; full inference deferred to 0.9.x**

---

## Context

Voicelab **david** profile uses **Chatterbox** with a cloned reference voice
(`am_onyx`, ~0.64 s TTFA) as the dual-layer **fast** tier so acks match slow-tier
timbre. The native stack ships **Kokoro** (works, preset voice) + **Orpheus**
(slow, expressive). Full Chatterbox → native Rust/ONNX is its own project; the
ticket text already noted deferral.

Upstream constraints (unchanged):

| Constraint | Implication |
|------------|-------------|
| Voice-tts **No Python** invariant | Cannot shell out to official Chatterbox Python |
| No official ONNX/Candle export | Cannot implement real tensor IO honestly |
| WEFT-657 pocket-tts | Separate watch item; same “wait for export” class |
| ADR-061 voice trilemma | Accepted default already allows preset slow + clone fast; native 0.8 has preset+preset until clone lands |

---

## Decision (0.8)

**Ship option A (scaffold) + C (selection hooks); accept option B (defer full port) for inference.**

1. **Accept Kokoro / Orpheus timbre split for 0.8** as the live native default.
2. **Do not pretend** a full TTS port is done — no fabricated PCM, no fake ONNX session.
3. **Scaffold** `ChatterboxTts` with model-dir discovery, bundle staging probe, and
   `CHATTERBOX_INFERENCE_IMPLEMENTED = false` readiness gate.
4. **Wire selection** so when (and only when) clone inference is runtime-ready,
   the david-intent fast path is chosen automatically.
5. **Defer** real Chatterbox/sherpa-onnx voice-clone inference + pinned model
   download to **0.9.x** (or earlier if an official export appears).

### Alternatives considered

| Option | Outcome |
|--------|---------|
| Full Chatterbox native port now | Rejected — multi-week ML port, no export |
| sherpa-onnx VC as drop-in | Deferred — needs eval + model staging; candidate for 0.9.x |
| Accept Kokoro only, no hooks | Rejected — loses selection path for david profile |
| Force broken Chatterbox when files staged | Rejected — would brick Talk-Mode audio |

---

## What shipped (0.8 slice)

| Artifact | Role |
|----------|------|
| `crates/clawft-voice-tts/src/chatterbox.rs` | Bundle probe, status, `TtsEngine` Fast tier, graceful error |
| `crates/clawft-voice-tts/src/fast_tier.rs` | `FastEnginePreference`, pure `resolve_fast_engine`, `build_native_fast_engine*` |
| `crates/clawft-voice-talk/src/tts.rs` | `native_dual_layer` uses selection; `native_dual_layer_with_fast_pref` |
| `voice.tts.fast_engine` (`clawft-types`) | Config wire: `auto` \| `kokoro` \| `chatterbox` |
| Env `WEFTOS_FAST_TTS` | Same tokens as config |
| Env `WEFTOS_CHATTERBOX_DIR` | Bundle override (staging) |
| Catalog id `chatterbox-clone-am_onyx` | Discovery only; **no** download (unpinned hash) |

### Operator staging layout (future)

```
.weftos/models/chatterbox/
  chatterbox.onnx      # required for bundle_staged
  reference.wav        # clone reference (david: am_onyx)
  voice_id.txt         # optional, default am_onyx
```

Staging alone does **not** enable inference until
`CHATTERBOX_INFERENCE_IMPLEMENTED` is flipped with a real session path.

### Preference semantics

| Preference | Clone ready | Selected |
|------------|-------------|----------|
| `auto` (default) | no | Kokoro |
| `auto` | yes | Chatterbox |
| `kokoro` | any | Kokoro |
| `chatterbox` | no | Kokoro + fallback reason |
| `chatterbox` | yes | Chatterbox |

---

## Plane / cycle disposition

| Field | Value |
|-------|-------|
| 0.8 close | Scaffold + selection + docs + unit tests |
| Residual | Native ONNX/Candle (or sherpa-onnx VC) inference, pinned download, live TTFA parity vs voicelab ~0.64 s, timbre match to Orpheus |
| Target cycle | **0.9.x** |
| Related | WEFT-657 (pocket-tts watch), ADR-061 §4, ADR-074 local replacement track |

**Recommend Plane**: move residual inference work to cycle **0.9.x** with comment
pointing at this result doc; close or partial-close WEFT-613 against the scaffold
SHA once merged. Do not keep “full native Chatterbox” as a 0.8 must-ship.

---

## Tests

```bash
scripts/build.sh test -p clawft-voice-tts
scripts/build.sh test -p clawft-voice-talk tts
scripts/build.sh test -p clawft-types tts_fast_engine
```

(Or package-focused `cargo test` if debugging; product path is `scripts/build.sh`.)

---

## Follow-ups (0.9.x acceptance sketch)

1. Official or pinned community ONNX/Candle export for Chatterbox **or** adopt
   sherpa-onnx voice-cloning TTS after eval.
2. Implement session load + tensor IO under `onnx` feature; set
   `CHATTERBOX_INFERENCE_IMPLEMENTED = true`.
3. Pin SHA-256 for `chatterbox-clone-am_onyx` catalog entry; enable `weft voice setup`.
4. Live TTFA / intelligibility gate vs voicelab david profile.
5. Optional: style/embedding bridge so Kokoro can approximate clone timbre as
   interim if full Chatterbox remains blocked.
