# weft-splat — phone → Gaussian splat pipeline for WeftOS

status: v0 scaffold — 2026-07-27
pattern: HTTP-as-stage (per weftos `.planning/sensors/PIPELINE-PRIMITIVE-JOURNAL.md`)
role in weftos: **sensor-2 camera probe** — the journal names camera as the next
pipeline-primitive probe because it forces the binary payload path. This repo is
the heavy processor; weftos gains only a thin client crate later.

## 1. Shape

```
  phone capture                     splatd (this repo)                     delivery
 ┌────────────┐   upload/path   ┌──────────────────────────────┐   ┌──────────────────┐
 │ walkaround │ ──────────────▶ │ probe → frames → sfm → train │──▶│ .ply  .sog       │
 │ video      │  POST /v1/jobs  │  (ffprobe/ffmpeg/colmap/     │   │ manifest.json    │
 └────────────┘                 │   brush-cli/splat-transform) │   │ Spark web viewer │
                                └──────────────────────────────┘   └──────────────────┘

  Preferred capture path (planned): native Android edge node — ADR-077 /
  docs/plans/android-splat-capture-edge-node.md — stills + IMU poses +
  coverage UX, stream to Mac/cloud splatd (not Gallery MP4 alone).

  Multi-modal sensing + lab sensor head + world model:
    docs/weftos/splat-multimodal-sensing.md
    docs/weftos/splat-capture-sensor-head.md
    docs/weftos/splat-to-world-model.md
    docs/weftos/splat-multi-camera-rig.md   # fixed known-pose multi-cam
    docs/weftos/splat-freeform-quilt.md     # accumulating regional quilt
    docs/weftos/splat-train-backends.md     # Brush / Instant-NuRec / model collection
    ADR-078 (appearance SOG + structure leaves → BVH)
```

Exactly like `clawft-service-whisper` ↔ whisper.cpp: **splatd is a separate
process with its own lifecycle, its own model of backpressure (serial job
queue), and an HTTP surface.** The weftos daemon never links COLMAP or a
trainer; it talks to splatd over HTTP via a future `clawft-service-splat`
client crate.

## 2. Pipeline stages

| # | stage    | tool              | in → out |
|---|----------|-------------------|----------|
| 1 | probe    | ffprobe           | video → metadata (duration, fps, resolution) |
| 2 | frames   | ffmpeg            | video → `dataset/images/*.jpg` (target fps, max frames, downscale) |
| 3 | sfm      | colmap (or glomap mapper) | images → `dataset/sparse/0` (poses + sparse points) |
| 4 | train    | brush-cli         | COLMAP dataset → `splat.ply` (Rust/wgpu; Metal on the Mac, CUDA/Vulkan on a GPU box, same binary family everywhere) |
| 5 | compress | splat-transform   | splat.ply → `splat.sog` (~1–10 % of PLY size, streams over the web) |
| 6 | package  | splatd            | artifacts + metrics + timings → `manifest.json` |

Every stage is a subprocess with its own log file (`logs/<stage>.log`),
timing, and exit-code check. Stage params and tool paths live in
`config/splatd.toml` — nothing is hardcoded, so swapping colmap→glomap or
brush→another trainer is config, not code.

Why Brush for training: it is Rust (matches the weftos codebase), runs on
macOS/Metal, Linux, Windows, Android **and in-browser via WebGPU**, and takes
COLMAP datasets directly. It does not do SfM itself — hence stage 3.

## 2b. Shasta: splatd is the SPLAT_RUNNER

The `~/dev/shasta` repo (shasta-os-016/019/020) already fixes the consumer
side: `splat.train` is a Vercel durable workflow — `beginRun → prepareFrames →
dispatchGpuJob → awaitArtifact → registerArtifact` — that fails with a
`FatalError` until a GPU runner exists behind **`SPLAT_RUNNER_URL`**.

**splatd is that runner.** The mapping:

| shasta-os-020 step | splatd call |
|--------------------|-------------|
| `dispatchGpuJob`   | `POST /v1/jobs` (JSON `{source_url, callback_url}`; splatd downloads the capture from Supabase Storage) |
| `awaitArtifact`    | webhook — splatd POSTs the JobRecord to `callback_url` on terminal state (poll `GET /v1/jobs/{id}` as fallback) |
| fetch + register   | `GET /v1/jobs/{id}/artifacts/splat.sog` → upload to Supabase Storage → `captures` row (`published: false`, per the No-Go #4 gate) |

Placement per shasta-os-019 §3 is configuration, not code: today
`SPLAT_RUNNER_URL` points at the M5 Pro Mac (Brush on Metal); later the same
splatd image runs as a Cloud Run Job with an L4 (Brush on Vulkan — same wgpu
binary). Nothing in the Vercel workflow changes. The `/blueprint` progression
viewer (splat-sphere + milestone slider) consumes the `.sog` artifacts via
Spark/three.js — `viewer/index.html` here is the reference implementation to
lift into the Shasta app.

One runner, two consumers: WeftOS reaches it through the substrate contract
(§5), Shasta through the durable workflow. Both speak the same v1 HTTP API.

## 3. HTTP surface (v1)

```
POST /v1/jobs                video OR image-set session (weft.capture.v1)
                             → 202 {"job_id": "..."}
GET  /v1/jobs                → list (id, status, stage, created_ts)
GET  /v1/jobs/{id}           → full JobRecord (status, stage, timings, metrics, error)
GET  /v1/jobs/{id}/log/{stage} → text log for a stage
GET  /v1/jobs/{id}/artifacts/{name} → artifact bytes (splat.ply, splat.sog, manifest.json)
GET  /healthz                → HealthReport-shaped snapshot (see §5)
```

### POST /v1/jobs — accepted bodies

| Mode | Content-Type | Body |
|------|--------------|------|
| Video upload | `multipart/form-data` | field **`video`** (file) + optional `callback_url` |
| Session ZIP | `multipart/form-data` | field **`session`** (`.zip` of weft.capture.v1 package) + optional `callback_url` |
| Local video | `application/json` | `{"source_path":"/abs/video.mp4","callback_url"?}` |
| Remote video | `application/json` | `{"source_url":"https://…","callback_url"?}` |
| Local session | `application/json` | `{"session_path":"/abs/session_dir","callback_url"?}` |

Do not send both video and session in one request.

**Image-set pipeline:** when a session is ingested, splatd materializes
`session/` under the job workspace, **skips ffmpeg** frame extract, copies
frames into `dataset/images/`, then runs COLMAP → Brush → compress as usual.
Schema: [`capture-protocol-v1.md`](./capture-protocol-v1.md).

```bash
# Video (unchanged)
curl -sS -X POST "$SPLATD/v1/jobs" -F "video=@./walk.mp4"

# Session path on the runner host
curl -sS -X POST "$SPLATD/v1/jobs" -H 'content-type: application/json' \
  -d '{"session_path":"/tmp/weft-session"}'

# Session ZIP upload
curl -sS -X POST "$SPLATD/v1/jobs" -F "session=@./session.zip"
```

Backpressure: one worker, serial queue (mirrors whisper's
one-in-flight-per-instance answer). A second POST while a job runs is queued,
not rejected; queue depth is visible in `/healthz`.

## 4. Job layout on disk

```
data/jobs/<job-id>/
  input.<ext>          # ingested video (video jobs)
  session/             # weft.capture.v1 package (image-set jobs)
    session.json
    poses.jsonl
    frames/
  dataset/
    images/            # extracted OR copied frames (Brush COLMAP layout)
    sparse/0/          # COLMAP reconstruction
  artifacts/
    splat.ply          # raw trained splat
    splat.sog          # compressed, web-streamable
    manifest.json      # artifacts + metrics + stage timings
  logs/<stage>.log
  job.json             # persisted JobRecord (survives restart)
```

## 5. WeftOS integration contract (future clawft-service-splat)

Mirrors the whisper substrate paths:

- **Capture announce (input):** `substrate/sensor/camera/capture`
  ```json
  { "capture_id": "c-2026-07-27-001", "kind": "video",
    "uri": "file:///.../IMG_1234.mov", "node_id": "<node>", "ts": 0 }
  ```
  Note: the payload is a *reference*, not frames. A 4K video does not fit the
  b64-in-JSON MVP path — this is precisely the binary-payload pressure the
  journal predicts camera will apply (§Q1 / A3). The reference-payload form is
  the interim answer; the binary substrate side-channel remains the real fix.

- **Derived output:** `substrate/_derived/splat/<source-node-id>/<capture-id>`
  ```json
  { "job_id": "...", "status": "done", "stage": "package",
    "artifacts": { "ply": ".../splat.ply", "sog": ".../splat.sog",
                   "viewer_url": "http://<host>:7860/viewer?job=<id>" },
    "metrics": { "n_frames": 180, "n_registered": 174, "n_splats": 1200000 },
    "timings_ms": { "frames": 4000, "sfm": 300000, "train": 900000 } }
  ```

- **Health:** `/healthz` returns the node/sensor HealthReport shape from
  `.planning/sensors/HEALTHCHECK-CONTRACT.md` (`status`, `uptime_s`,
  `last_publish_ts`, plus service-specific `queue_depth`, `jobs_done`,
  `jobs_failed`) so the Explorer can render splatd like any other sensor.

The thin client crate follows the whisper split exactly: subscribe to the
capture path, POST to splatd, poll `/v1/jobs/{id}`, publish the derived
payload. `wav`/`windower` ⇢ (nothing — video arrives whole); `client` ⇢
`SplatClient`; `service` ⇢ `SplatService`.

## 6. Compute placement

Today: splatd runs on the M5 Pro MacBook (Brush on Metal, COLMAP CPU — fine
for phone captures of ~100–300 frames). Later: the same binary + config runs
on a Linux/NVIDIA box or cloud GPU; because the daemon only knows an HTTP URL,
relocating compute is a config change (`SPLATD_URL`), and a GPU node joining
the mesh can advertise the service the same way whisper-across-daemons is
sketched in the journal (§A9, shared GPU worker variant).

## 7. Delivery

`viewer/index.html` is a single-file Spark (three.js) viewer that loads
`/v1/jobs/{id}/artifacts/splat.sog` (or a `?url=` override). Spark renders
PLY/SPZ/SOG in any modern browser, including on the phone that shot the video.

## 8. Non-goals for v0

Auth, multi-tenant queues, mesh advertisement, live RTSP ingest (design keeps
a slot: an RTSP source becomes a different ingester that still lands a video
file in the job dir), dedicated-rig multi-camera datasets (COLMAP handles them;
the frames stage is what changes).
