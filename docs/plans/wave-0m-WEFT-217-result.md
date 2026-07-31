# WEFT-217 Result — Real DSP for EchoCanceller / NoiseSuppressor

**Date**: 2026-07-31  
**Branch**: `weft-217-real-dsp`  
**Worktree**: `~/.grok/worktrees/mathewbeane-weftos/subagent-019fb673-a49c-7ef0-b9a7-497abec389b9`  
**Ticket**: WEFT-217  
**Status**: Implemented (scaffold DSP; product AEC remains `clawft-voice-aec`)

---

## Context

Audit finding: plugin `echo.rs` / `noise.rs` maintained state (reference buffer,
noise-floor EMA) but `process()` was identity passthrough — deceptive.

WEFT-671 disposition notes product AEC/NS live in **`clawft-voice-aec`**
(WebRTC AEC3 + NS) and marks WEFT-217 as a supersession candidate. This
delivery still implements **real pure-Rust DSP** on the scaffold surface so
the API is no longer a lie, while documenting the production path honestly.

Live path check (WEFT-671): `clawft-channels::voice` consumes
`clawft-voice-aec::AecProcessor` — no parallel plugin EchoCanceller on the
talk loop. Channels was **not** modified.

---

## Implementation

### `EchoCanceller` — NLMS adaptive filter

| Item | Detail |
|------|--------|
| Algorithm | Normalized LMS FIR on far-end reference |
| Alignment | `feed_reference` queues samples; `process` drains 1:1 into delay line |
| Filter length | `tail_length_ms × 16 kHz` (default 128 ms → 2048 taps) |
| Suppression | `suppression_level` scales the adaptive estimate before subtraction |
| Disabled | Identity + no queue / adapt |

### `NoiseSuppressor` — spectral subtraction

| Item | Detail |
|------|--------|
| Algorithm | STFT magnitude spectral subtraction (frame 256, hop 128, periodic Hann COLA) |
| Noise PSD | Seeded on first frame; EMA only on noise-like frames |
| Aggressiveness | 0–3 → over-subtraction + spectral floor |
| FFT | In-tree radix-2 Cooley–Tukey (no new deps) |
| RNNoise | **Not** bundled — needs model + native deps; prefer WebRTC NS via `clawft-voice-aec` |
| Disabled | Identity |

### Docs

- Module docs on `echo.rs` / `noise.rs` state limitations vs WebRTC AEC3/NS
- `voice/mod.rs` WEFT-217 exception note under deprecated scaffold

---

## Acceptance

| AC | Evidence |
|----|----------|
| 1. NLMS / freq-domain AEC using reference | NLMS FIR + reference queue in `echo.rs` |
| 2. Spectral subtraction / similar using noise floor | STFT subtraction + `noise_floor` RMS EMA in `noise.rs` |
| 3. Synthetic tests prove attenuation | `synthetic_echo_is_attenuated` (residual/mic &lt; 0.25); `synthetic_noise_is_attenuated` (out/in &lt; 0.5) |
| 4. Honest docs if not RNNoise | Module + this note; not pure passthrough |

---

## Test

```bash
./scripts/build.sh test clawft-plugin --features voice
# → 335 passed, 0 failed
```

Focused:

```bash
cargo test -p clawft-plugin --features voice --lib -- voice::echo voice::noise
# → 17 passed
```

---

## Files changed

- `crates/clawft-plugin/src/voice/echo.rs` — NLMS AEC + tests
- `crates/clawft-plugin/src/voice/noise.rs` — spectral subtraction + tests
- `crates/clawft-plugin/src/voice/mod.rs` — disposition note
- `docs/plans/wave-0m-WEFT-217-result.md` — this report

---

## Merge notes for lead

```text
Worktree: /Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb673-a49c-7ef0-b9a7-497abec389b9
Branch:   weft-217-real-dsp
Base:     release/0.8-staging @ 4d8fa5d2
```

Suggested close comment:

> WEFT-217: plugin EchoCanceller = pure-Rust NLMS; NoiseSuppressor = STFT
> spectral subtraction; synthetic attenuation tests green. Production full-duplex
> remains clawft-voice-aec (WebRTC). No RNNoise (documented).
