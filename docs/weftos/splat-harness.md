# Splat harness (video → splat → view)

## Crates

| Crate | Role |
|-------|------|
| `clawft-splat-pipeline` | Stage library (ffmpeg / COLMAP / Brush / SOG) |
| `clawft-splatd` | Binary **`splatd`** — HTTP job server (`:7860`) |
| `clawft-bvh` | ADR-056 Phase A BVH (spatial world model scaffold) |

Standalone runner (no daemon):

```bash
cargo build -p clawft-splatd --release
SPLATD_CONFIG=config/splatd.toml ./target/release/splatd
```

## Tools on PATH

```bash
brew install ffmpeg colmap
# Brush 0.3.0 macOS arm64 release as brush-cli (git build needs newer rustc)
npm i -g @playcanvas/splat-transform   # optional SOG compress
```

## Harness

```bash
# Full path: start splatd if needed, train, open viewer
scripts/splat-harness.sh /path/to/walkaround.mov

# View an existing job (copies SOG into viewer same-origin — preferred)
scripts/splat-harness.sh --job <uuid>

# View a local artifact only (same-origin; no splatd CORS needed)
scripts/splat-harness.sh --view-only ./splat.sog
# e.g. finished job:
# scripts/splat-harness.sh --view-only data/splat/jobs/<uuid>/artifacts/splat.sog

# Static viewer server
scripts/splat-harness.sh --serve-viewer
```

Viewer: `examples/splat-viewer/index.html`

- `?url=.harness/<file>` — **preferred** (same origin as viewer; harness `--job` does this)
- `?job=<id>&splatd=http://127.0.0.1:7860` — cross-origin; needs splatd CORS (all routes)
- **Axes / grid** overlays (toggle in UI)
- **`window.WeftSplatHarness`** — plugin API for future BVH boxes, measure rays, coords

### “Failed to fetch”

Usually the page on `:8765` calling splatd on `:7860` without CORS on **job JSON**
(artifacts alone used to set `Access-Control-Allow-Origin`; `GET /v1/jobs/{id}` did not).
splatd now applies a global CORS layer. Still safest: `--job` / `--view-only` same-origin copy.

## Capture quality (deeper design)

- [splat-multimodal-sensing.md](./splat-multimodal-sensing.md) — wavelengths, overlap layers, S25 inventory, hardware tiers  
- [splat-capture-sensor-head.md](./splat-capture-sensor-head.md) — Pi + VL53/IMU/radar 3D-printed head  
- [splat-to-world-model.md](./splat-to-world-model.md) — objects, volumes, BVH population  
- [splat-multi-camera-rig.md](./splat-multi-camera-rig.md) — fixed multi-cam known poses  
- [splat-freeform-quilt.md](./splat-freeform-quilt.md) — free-form accumulating quilt  
- [splat-train-backends.md](./splat-train-backends.md) — pluggable train models (Brush, Instant-NuRec, …)  
- [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md) — Android edge capture node  
- [ADR-078](../adr/adr-078-splat-feeds-world-model.md) — splat feeds world model, not appearance-only

## Quality: “exploded” / only one pocket looks real

Typical causes (this job exhibited several):

| Symptom driver | What we saw / do |
|----------------|------------------|
| **Multiple COLMAP models** | `sparse/0` (161 imgs) + `sparse/1` (22). Brush loads **all** of them → dual worlds. Pipeline now **keeps largest only**. |
| **Partial registration** | 161/210 frames in main model; rest discarded or tiny models. |
| **Weak tracks** | Mean track length ~5; walkaround + 2 fps sequential, no loop closure. |
| **Phone rotation** | Display Matrix −90°; ffmpeg autorotate → 900×1600 (OK if consistent). |
| **Too many gaussians** | ~2.7M splats + floaters look like confetti. |

**Re-train this job without re-extracting frames** (after rebuild):

```bash
# Keep only largest model (already true if re-run through new sfm stage)
rm -rf data/splat/jobs/<id>/dataset/sparse/1
# Re-run brush on cleaned dataset (manual example):
cd data/splat/jobs/<id>
brush-cli dataset --total-steps 30000 --max-resolution 1600 --max-splats 2000000 \
  --export-every 30000 --export-path artifacts --export-name splat.ply
# then compress + harness --view-only artifacts/splat.sog
```

**Better capture next time:** slow orbit with lots of overlap, return to start (loop), avoid pure rotation in place, good lighting, 3–4 fps sampling, enable `loop_detection` if vocab tree available.

## World-model notes

- Product decision: **ADR-078** — dual output (SOG appearance + structure).
- BVH leaf tags in `clawft_bvh::leaf::tags`: `SPLAT_SCENE`, `SPLAT_CAMERA`, `WM_OBJECT`, `WM_SURFACE`, `WM_VOLUME`, …
- Design: [splat-to-world-model.md](./splat-to-world-model.md).
- Full BVH insert from train results is phased (W0+); harness today is **view-first**.
- Reconstruction (COLMAP+Brush) only — generative image→3D is not measurement-safe.
