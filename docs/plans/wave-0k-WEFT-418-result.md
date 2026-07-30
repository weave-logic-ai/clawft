# WEFT-418 result — migrate mic adapter to node-scoped summary + pcm

**Ticket:** WEFT-418  
**Branch:** `wave0k/weft-418-mic-paths`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-418 (wave-0k)

## Problem

After WEFT-438, `MicrophoneAdapter` dual-emitted level data on
`substrate/<node-id>/sensor/mic/{summary,rms}` (+ legacy flat) but still
did **not** emit a PCM stream. Whisper / classify consumers expected
`substrate/<node-id>/sensor/mic/pcm_chunk`; only the ESP32 bridge /
`publish_wav` harness published there. The in-tree adapter was half-
migrated.

## What shipped

### 1. Windowed-Append PCM topic (WEFT-418 core)

`MicrophoneAdapter` now Appends each 500 ms s16le window to:

| Path | Verb | Policy |
|------|------|--------|
| `substrate/<node>/sensor/mic/pcm_chunk` | `StateDelta::Append` | `DropOldest`, `max_len = MIC_PCM_WINDOW_MAX_LEN` (8 ≈ 4 s @ 2 Hz) |
| `substrate/sensor/mic/pcm_chunk` | `Append` (dual-emit) | same, while `DEFAULT_DUAL_EMIT_LEGACY` |

Chunk shape is whisper-compatible (`data`/`pcm_b64`, `encoding`,
`format`, `sample_rate`, `channels`, `samples`, `seq`, `chunk_ms`,
`tick`) per `JOURNALED-SENSOR-MIC.md` §2.2 +
`clawft_service_whisper::windower::PcmChunk`.

Level emissions (summary / rms / legacy Replace) are unchanged from
WEFT-438.

### 2. `sensor_paths` helpers

- `MicPcmEmitPlan` + `mic_pcm_emit_plan(node, dual)`
- `MIC_PCM_WINDOW_MAX_LEN = 8`
- `is_mic_pcm_open_topic` / `is_mic_open_topic` (level ∪ pcm)
- Re-exports from crate root

### 3. TopicDecl + open surface

- Host-local + legacy pcm TopicDecls with windowed `max_len`
- `open()` accepts any node-scoped
  `…/sensor/mic/{summary,rms,pcm_chunk}` plus legacy flats + healthcheck
- Shared producer loop; channel depth raised to 16

### 4. Docs

- `.planning/ontology/ADOPTION.md` §16 — WEFT-418 cutover table;
  unblocks list marks WEFT-418 done

## Acceptance

| Criterion | Status |
|-----------|--------|
| Mic adapter publishes under `substrate/<node-id>/sensor/mic/{summary,pcm}` | **Done** — leaves are `summary` + `pcm_chunk` (canonical naming from JOURNALED-SENSOR-MIC / WEFT-438) |
| pcm ships as a windowed-Append topic | **Done** — `StateDelta::Append` + `max_len` ring |
| Cutover documented in ADOPTION.md | **Done** — §16 WEFT-418 cutover |

## Tests / build

```bash
scripts/build.sh test clawft-substrate
```

- **clawft-substrate:** 197 passed (was 176 after WEFT-438; +path/pcm tests)
- New mic tests: `poller_appends_pcm_chunk_on_node_scoped_path`,
  `poller_skips_legacy_pcm_when_dual_emit_disabled`,
  `build_pcm_chunk_shape_is_whisper_compatible`, expanded open/TopicDecl

## Residual / follow-ups

1. **0.9.0** — flip `DEFAULT_DUAL_EMIT_LEGACY → false`; drop legacy
   TopicDecls + `SUBSTRATE_PCM_INPUT_PATH`.
2. **Task 24** — per-node write gate can assume node-scoped sensor trees.
3. Presence / rfkill still on flat paths — migrate with same helpers.
4. Host-audio (CPAL) backing still file-stub; shape is ready.

## Files

- `crates/clawft-substrate/src/mic.rs` — pcm Append + TopicDecl + tests
- `crates/clawft-substrate/src/sensor_paths.rs` — pcm emit plan + open helpers
- `crates/clawft-substrate/src/lib.rs` — re-exports + module docs
- `crates/clawft-substrate/Cargo.toml` — native `base64` for pcm encode
- `.planning/ontology/ADOPTION.md` — §16 cutover
- `docs/plans/wave-0k-WEFT-418-result.md` — this file
