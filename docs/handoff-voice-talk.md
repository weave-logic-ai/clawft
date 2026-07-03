# Handoff — Native voice Talk-Mode: working conversation loop, remaining polish

**Date:** 2026-07-03
**Repo:** `/Users/mathewbeane/weftos` (branch `feat/hermes-loop-base`)
**Status:** `weft voice talk` holds a real spoken conversation end-to-end on this Mac:
mic → adaptive VAD → smart-turn endpointing → parakeet STT → **Hermes-4.3-36B** (`:8090`)
→ Orpheus TTS ("dan", single voice for ack + answer) → speaker, with every turn
**anchored to the witness chain** via the daemon. One day of live shakedown took it from
"never responds" to conversational; the remaining gaps are tracked and listed below.

**The prime directive learned today:** `~/llm` voicelab (the Python lab, david profile,
`~/llm/voicelab/configs/david.toml`) is the **tuned reference implementation**. Every
substantive bug in the native port was a divergence from a decision the lab had already
made. **When the native stack misbehaves, diff against the lab FIRST.**

---

## Architecture (what runs where)

| Stage | Impl | Endpoint / model |
|---|---|---|
| Capture + AEC | cpal + `clawft-voice-aec` (`run_capture`, shared `AecProcessor`) | default input device |
| VAD | `EnergyVad` + **adaptive `NoiseFloor`** (floor+8 dB, init cap -53) | in-process |
| Endpointing | smart-turn v3.1 ONNX + `SemanticEndpointer` (500 ms check / 2500 ms ceiling) | `~/.weftos/models/smart-turn/` |
| STT | parakeet-tdt-0.6b sherpa int8 | `~/.weftos/models/parakeet/…int8/` |
| Speaker ID | ECAPA ONNX (self-exported) + persistent registry | `~/.weftos/models/ecapa/`, `~/.weftos/speakers.json` |
| Brain | `LocalProviderVoiceLlm::hermes()` — **hermes-4.3-36b** | llama.cpp `:8090` (ADR-060, `~/llm/bin/serve-llamacpp`) |
| TTS fast (fallback + uncached acks) | Kokoro ONNX, style = **am_michael** | `~/.weftos/models/kokoro/` (`kokoro.onnx`+`tokens.txt`+`style.bin`) |
| TTS slow (answers + cached acks) | Orpheus via **Ollama** `orpheus-tts` voice **dan** + SNAC ONNX | `:11434` + `~/.weftos/models/snac/snac_24khz.onnx` |
| Chain anchoring | `agent.turn.record` RPC → `SubstrateConversationSink` + `KernelTurnAnchor` | kernel daemon (`weaver kernel start`) |

Run: `weft voice talk` (or `RUST_LOG=info weft voice talk` to watch the turn lifecycle:
`utterance captured` → `user turn transcript=` → `committed reply answer=`).
Mic diagnosis: `weft voice test-mic --duration 5` (device list, dBFS meter, VAD verdict).
Rebuild: `scripts/build.sh native --features voice-onnx` then install to `~/.cargo/bin`
(atomic `cp`→`mv`; a running daemon/session keeps its old inode until restarted).

## What was fixed today (commit trail, all on `feat/hermes-loop-base`)

1. `86bdfbca` — E2E test raced the loop's Frontier→Committed tick.
2. `63dc49d4` — **`agent.turn.record` RPC**: voice turns anchor to substrate + chain
   (WEFT-607 closed). `weft voice talk` prints "Turn anchoring: ON" when the daemon runs.
3. `27d628b7` — **Kokoro spoke gibberish**: model eats espeak-ng IPA phonemes, engine fed
   raw chars. Now shells out to `espeak-ng` (brew) per sentence. + turn-lifecycle info logs.
4. `d8eb5ee3` — **deaf in a loud room**: fixed -45 dBFS gate vs -37 dBFS room tone; adaptive
   `NoiseFloor`. + real `weft voice test-mic`.
5. `0c563977` — slow-tier zero-audio fallback to fast tier; barge margin/grace; min-turn
   guard (250 ms); `TtsSink::wait_drained` (capture resumed mid-playback and heard itself).
6. `ba588b9f` — **Orpheus prompt missing `<|eot_id|>`** → zero audio tokens (WEFT-612 closed;
   found by diffing the lab's `tts_orpheus_ollama.py`). + lab sampling params.
7. `2c350f4b` — voicelab knob parity (500/2500 ms, cosine 0.45), spoken self-enrollment
   ("my name is X"), persistent speaker registry (WEFT-611 closed).
8. `55d33c92` — **barge-in now OPT-IN** (`barge_in_enabled`, default off): AEC residual of
   the bot's own reply tripped the gate at grace expiry and cancelled every answer.
9. `c22a8494` — drain mic during playback hold (channel-full warn flood); prebuffer the
   slow tier (Ollama renders slower than realtime → stutter).
10. `3fd12d3b` — whole-utterance SNAC decode (per-batch seams doubled syllables — same
    artifact the lab documented).
11. `37ae3046` — **THE audio-corruption root cause**: Ollama's stream opens with control
    tokens (`custom_token_4,5,1`); counting them shifted the %7 SNAC slot phase → 251/251
    codes invalid → pure noise. Lab drops `c<0` before phasing; now mirrored.
12. `08df53a3` — **single-voice acks**: ack set is closed (2 strings), pre-rendered through
    Orpheus at session start, played from cache. Subject-echo ack parked until WEFT-613.

## Verification tooling (use these before claiming anything works)

- **`crates/clawft-voice-talk/tests/speak_wav.rs`** (`--ignored`): drives the REAL
  DualLayerTts (Kokoro + Orpheus/Ollama) into a WAV; whisper it back via
  `~/llm/bin/whisper <wav> --model mlx-community/whisper-small.en-mlx`. This harness is
  what isolated the phase bug ("audio exists" ≠ "audio is speech" — assert intelligibility).
- `live_native_talk_session` (tests/assembly.rs, `--ignored`): WAV → full loop → committed
  ECC turn. `live_orpheus_ollama`, `live_kokoro_synthesis*`: per-engine gates.
- Live-session logs: `RUST_LOG=info,clawft_channels=debug` shows barge-in + blip drops.
- **Do NOT run a live `weft voice talk` while the operator is testing** — two sessions
  answer the same room and it reads as chaos (this happened; memory-noted).

## Open items (Plane)

- **WEFT-606** (0.8.x): daemon-hosted Talk-Mode tick service + real chain_seq/HLC in the
  forest (the RPC bridge is the stepping stone).
- **WEFT-613** (0.9.x): voice-matched fast tier — the david demo used a **Chatterbox clone
  of dan** so both tiers were one voice; restores the subject-echo ack. *(Pending comment
  correction: item text says am_onyx; operator confirms the demo cloned dan.)*
- **WEFT-614** (0.9.x): grounded agent brain (web_search / tool-calling) — ride WEFT-606
  and route the voice brain through the kernel agent loop.
- **PENDING (Tailscale re-auth needed for `plane.sh` via aepod-xpc)**: file "Re-enable
  barge-in" item — barge_in_enabled default-off until AEC echo-return-loss is verified;
  leads: lab knobs (`barge_threshold=0.6 barge_min_s=0.30 barge_rms=0.012`) and
  render-reference alignment (chunks push the AEC reference at queue time; playback happens
  seconds later — likely misalignment). Plus the WEFT-613 dan-clone comment above.
  Plane key: `ssh aepod@100.79.110.69` → `~/.claude.json` `mcpServers.plane.env.PLANE_API_KEY`.

## Known polish gaps (unfiled observations)

- **Answer latency**: prebuffered Orpheus means "One sec." → quiet render pause → answer.
  Lab's `slow_lead_chunks` (stream with a one-sentence head start, seam-primed with
  `context_frames=32`) is the latency fix if the pause annoys; `take_batch` was kept for it.
- **STT quality**: parakeet int8 mishears through-air/speaker audio badly ("two plus two"
  → "Ubazu"); direct voice is decent. Lab's speed tier used the same parakeet — but eval
  vs whisper-large via the substrate whisper service is worth a look if mishearing persists.
- Kokoro `style.bin` = one carved vector (voice 6 am_michael, row 60 of
  `csukuangfj/kokoro-en-v0_19` voices.bin); proper per-length style rows would improve
  prosody. Slight onset clip on the first word (noted at WEFT-608 closure).
- Orpheus voice knob: `dan` today; alternatives `leo`/`zac` via
  `native_dual_layer_with_ollama(url, model, voice)` — operator perceived dan as
  "English accent"; awaiting verdict now that everything is one voice.
- `weft voice talk` has no CLI flags yet (mic selection, voice, barge-in opt-in, RUST_LOG
  presets) — everything is code defaults; a `--config`/flags pass would help the next
  debugging session. `weft voice setup` is still a stub (models were staged by hand —
  layout in the table above; sources: HF `csukuangfj/…parakeet…int8`, `pipecat-ai/smart-turn-v3`,
  `csukuangfj/kokoro-en-v0_19`, `onnx-community/snac_24khz-ONNX`, ECAPA self-exported via
  `~/llm/.venv` torch+speechbrain).

## Memory anchors

`weftos-current-state` and `llm-lab-controller` memories hold the compressed version of
this handoff; `weftos-operational-gotchas` has the config-overlay and test-environment
traps (kernel daemon must be STOPPED for `clawft-rpc` no-daemon tests; mdns test needs
multicast; `hnsw_eml::benchmark_full_report` legitimately runs ~22 min in debug).
