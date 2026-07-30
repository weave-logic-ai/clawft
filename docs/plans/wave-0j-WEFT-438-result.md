# WEFT-438 result — sensors: legacy-flat vs node-scoped path naming + migration

**Ticket:** WEFT-438  
**Branch:** `wave0j/weft-438-sensor-paths`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-438 (wave-0j)

## Problem

ADOPTION.md and the sensor planning track
(`.planning/sensors/JOURNALED-SENSOR-MIC.md`,
`JOURNALED-NODE-ESP32.md` §7) described a post-Node/Actor-split layout
`substrate/<node-id>/sensor/<name>/<leaf>` but never recorded a
binding decision or shipped a migration. In-tree emitters still used
legacy flat paths (`substrate/sensor/mic`); consumers were split
(GUI discovery + whisper/classify already node-scoped; MicrophoneAdapter
+ fixtures still flat). Mic adapter migration (WEFT-418) and the
per-node write gate (task 24) could not land cleanly on top of that
ambiguity.

## Decision

**Node-scoped is canonical.** Documented in
`.planning/ontology/ADOPTION.md` §16 and implemented as the sole
source of truth in `clawft_substrate::sensor_paths`.

| Form | Pattern | Status |
|------|---------|--------|
| **Canonical** | `substrate/<node-id>/sensor/<name>/<leaf>` | Required for new emitters |
| **Legacy flat** | `substrate/sensor/<name>[/<leaf>]` | Dual-emitted until **0.9.0** / **2026-10-01** |
| **Mesh-owned** | `substrate/{kernel,cluster,chain,meta,_derived,actor,ui}/…` | Never node-scoped |

Mic leaves:

| Leaf | Role |
|------|------|
| `summary` | Full level object (`rms_db`, `peak_db`, …) |
| `rms` | Scalar for tray-chip / `mic_discovery` |
| `pcm_chunk` | Raw PCM window (whisper / classify) |

Host-local adapters without a provisioned identity use reserved node id
`host-local`.

## What shipped

### 1. `clawft-substrate::sensor_paths` (new)

Path builders, classify/parse, legacy↔canonical map, dual-emit plan,
removal constants, `is_mic_level_open_topic` open-target helper.
Re-exported from the crate root.

### 2. `MicrophoneAdapter` dual-emit (WEFT-438 migration)

- Emits **always**: `…/sensor/mic/summary` + `…/sensor/mic/rms`
- Dual-emits **legacy** `substrate/sensor/mic` while
  `DEFAULT_DUAL_EMIT_LEGACY` is true
- `with_node_id` / `with_dual_emit_legacy` builders
- `open()` accepts legacy flat **or** any node-scoped
  `…/sensor/mic/{summary,rms}`
- TopicDecl list covers host-local summary/rms + legacy + healthcheck

### 3. Whisper consumer

- `pcm_chunk_input_path(node_id)` — canonical helper
- `SUBSTRATE_PCM_INPUT_PATH` marked `#[deprecated]`
- Default `WhisperServiceConfig.input_path` uses node-scoped
  `substrate/n-test00/sensor/mic/pcm_chunk`
- `publish_wav` example updated

### 4. Surfaces / fixtures

- `weftos-chip-audio.toml` → host-local summary bindings
- `examples/example-workshop.toml` → host-local summary path

### 5. Docs

- ADOPTION.md §16 — decision, shim table, removal date, unblock list

## Acceptance

| Criterion | Status |
|-----------|--------|
| Decide on the canonical path layout in ADOPTION.md | **Done** — §16 |
| Land a migration that updates emitters and consumers | **Done** — mic dual-emit + whisper + fixtures |
| Document compatibility shims and a removal date | **Done** — 0.9.0 / 2026-10-01 in code + ADOPTION |

## Tests / build

```bash
scripts/build.sh test clawft-substrate
scripts/build.sh test clawft-service-whisper
scripts/build.sh check
```

- **clawft-substrate:** 176 passed (incl. 10 new `sensor_paths` + dual-emit mic tests)
- **clawft-service-whisper:** 49 passed
- **check:** pass (pre-existing clawft-kernel warnings only)

## Residual / follow-ups

1. **WEFT-418** — full mic cutover: emit `pcm_chunk` topic, drop
   legacy-only open after dual-emit window.
2. **Presence / rfkill** still publish flat `substrate/sensor/*` —
   migrate with the same helpers when claimed.
3. **Task 24** — per-node write gate can now assume the first path
   segment after `substrate/` is a node id for sensor trees.
4. **Flip `DEFAULT_DUAL_EMIT_LEGACY` → false** at 0.9.0 and remove
   `SUBSTRATE_PCM_INPUT_PATH` + legacy TopicDecl.

## Files

- `crates/clawft-substrate/src/sensor_paths.rs` — **new**
- `crates/clawft-substrate/src/lib.rs` — module + re-exports
- `crates/clawft-substrate/src/mic.rs` — dual-emit + node_id
- `crates/clawft-substrate/src/physical.rs` — path layout docs
- `crates/clawft-service-whisper/src/lib.rs` — helper + deprecate flat const
- `crates/clawft-service-whisper/src/service.rs` — default input path
- `crates/clawft-service-whisper/examples/publish_wav.rs`
- `crates/clawft-surface/fixtures/weftos-chip-audio.toml`
- `examples/example-workshop.toml`
- `.planning/ontology/ADOPTION.md` — §16
- `docs/plans/wave-0j-WEFT-438-result.md` — this file
