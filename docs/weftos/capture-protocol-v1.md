# weft.capture.v1 — capture session package

**Status**: frozen for A0 (WEFT-704)  
**ADR**: [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md)  
**Plan**: [android-splat-capture-edge-node.md](../plans/android-splat-capture-edge-node.md)  
**Consumers**: Android capture app (A1+), lab sensor head, splatd image-set ingest

This document is the **normative layout and schema** for a batch capture session
that host-side splatd accepts as an alternative to a single video file.

---

## 1. Protocol id

| Field | Value |
|-------|--------|
| Protocol | `weft.capture.v1` |
| Transport (batch) | HTTPS multipart or local path (see §5) |
| Transport (stream) | Deferred to A4 (QUIC/chunked) |

`session.json` SHOULD set `"protocol": "weft.capture.v1"`. Hosts MAY reject
unknown protocol strings when the field is present; when omitted, treat as v1
if the directory layout matches §2.

---

## 2. Session directory layout

```
session-<uuid>/                 # or any directory name; contents matter
  session.json                  # required for full clients; optional for host path ingest
  poses.jsonl                   # required (one JSON object per line)
  frames/                       # required
    000001.jpg
    000002.jpg
    …
  cameras.json                  # optional — multi-cam (see §4)
  optional/                     # optional extras
    preview.mp4
    coverage.bin
  depth/                        # optional sensor head (non-normative for A0 train)
  radar/                        # optional
```

**Minimum for splatd image-set ingest (A0):**

- `frames/` contains one or more image files (`.jpg` / `.jpeg` / `.png` / `.webp` / `.heic`)
- `poses.jsonl` exists (may be empty only if frames are still present; empty poses lose coverage QA)

Hosts materialize this tree under `data/jobs/<job-id>/session/` before the pipeline runs.

### frames/

- Filenames SHOULD be zero-padded decimal sequences (`000001.jpg`, …) for stable sort.
- Relative paths in `poses.jsonl` are resolved against the **session root** (not `frames/`).
- Typical entry: `"path": "frames/000001.jpg"`.
- Supported extensions for A0 train path: `.jpg`, `.jpeg`, `.png`, `.webp`. HEIC may be present in packages from phone export; host convert-on-ingest is optional and not required for A0.

---

## 3. session.json

Top-level object. Unknown fields are ignored by v1 hosts (forward compatible).

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `protocol` | string | recommended | `"weft.capture.v1"` |
| `id` | string | recommended | Session UUID or stable id |
| `device` | object | optional | Capture device metadata |
| `device.model` | string | optional | e.g. `"Pixel 8"`, `"Samsung SM-S931B"` |
| `device.platform` | string | optional | `"android"` \| `"ios"` \| `"lab"` \| … |
| `device.node_id` | string | optional | WeftOS Ed25519 node id (hex) when paired |
| `started_at` | string | recommended | RFC 3339 timestamp |
| `ended_at` | string | optional | RFC 3339 |
| `camera` | object | optional | Primary camera intrinsics hints |
| `camera.width` | u32 | optional | Sensor/output width (px) |
| `camera.height` | u32 | optional | Sensor/output height (px) |
| `camera.focal_hint` | f64 | optional | Approximate focal length in px (SfM prior later) |
| `camera.model` | string | optional | `"pinhole"` \| `"opencv"` \| … (hint only in A0) |
| `coverage` | object | optional | Summary of coverage bins |
| `coverage.n_bins` | u32 | optional | Total bins in the sphere/icosahedron map |
| `coverage.filled` | u32 | optional | Bins with ≥1 frame |
| `n_frames` | u32 | optional | Authoritative count; host may re-count files |
| `notes` | string | optional | Free text |

### Example

```json
{
  "protocol": "weft.capture.v1",
  "id": "a1b2c3d4-e5f6-7890-abcd-ef1234567890",
  "device": {
    "model": "Pixel 8",
    "platform": "android",
    "node_id": null
  },
  "started_at": "2026-07-30T18:00:00Z",
  "ended_at": "2026-07-30T18:08:12Z",
  "camera": {
    "width": 1080,
    "height": 1920,
    "focal_hint": 1400.0,
    "model": "pinhole"
  },
  "coverage": { "n_bins": 128, "filled": 97 },
  "n_frames": 240
}
```

---

## 4. poses.jsonl

Newline-delimited JSON. **One object per frame** (or per sample attached to a frame).

### Required fields (A0)

| Field | Type | Description |
|-------|------|-------------|
| `frame_id` | u64 | Monotonic id within the session (1-based recommended) |
| `t_ns` | u64 | Capture timestamp in **nanoseconds** (device clock; monotonic preferred) |
| `path` | string | Relative path from session root, e.g. `"frames/000001.jpg"` |
| `quat_wxyz` | `[f64; 4]` | Device / camera attitude as **unit quaternion, w-x-y-z** |

### Optional fields

| Field | Type | Description |
|-------|------|-------------|
| `coverage_bin` | u32 | Index into the capture app's coverage map |
| `accel` | `[f64; 3]` \| null | m/s², device frame |
| `gyro` | `[f64; 3]` \| null | rad/s |
| `gps` | object \| null | `{ "lat", "lon", "alt_m?", "acc_m?" }` |
| `cam` | object \| null | Per-frame overrides: `{ "w", "h", "focal_hint" }` |
| `position_m` | `[f64; 3]` \| null | Metric translation if VIO/ARCore available (A6) |
| `quality` | f64 \| null | Client quality score 0–1 |

### Line example

```json
{"frame_id":1,"t_ns":123456789012,"path":"frames/000001.jpg","quat_wxyz":[0.7071,0,0,0.7071],"coverage_bin":12}
```

### Quaternion convention

- Order: **`[w, x, y, z]`** (scalar first).
- Represents rotation from a host-defined “capture world” (often gravity-aligned with heading free) to the camera optical frame, or the device attitude used for coverage bins. Exact SfM prior injection is **out of scope for A0** — poses power coverage QA and future prior stages.
- Prefer unit length; hosts MAY renormalize.

### Empty / missing poses

- If `poses.jsonl` is missing but `frames/` is non-empty, A0 hosts MAY still accept the package (train path only) and log a warning.
- Prefer always writing one pose line per frame from the phone app (A1).

---

## 5. Optional multi-cam: cameras.json

For fixed multi-camera rigs (see [splat-multi-camera-rig.md](./splat-multi-camera-rig.md)). **Not required for phone single-cam A0–A2.**

When present, session layout MAY use per-camera frame dirs:

```
session-<uuid>/
  session.json
  cameras.json
  cam_a/frames/…
  cam_b/frames/…
  poses.jsonl            # optional if heads are static
```

### cameras.json sketch

```json
{
  "protocol": "weft.capture.v1",
  "cameras": [
    {
      "id": "cam_a",
      "model": "OPENCV",
      "width": 1920,
      "height": 1080,
      "params": [fx, fy, cx, cy, k1, k2, p1, p2],
      "T_world_cam": {
        "quat_wxyz": [1, 0, 0, 0],
        "t_m": [0, 0, 0]
      },
      "frames_dir": "cam_a/frames"
    }
  ],
  "sync": { "mode": "hardware" | "ntp" | "free_run" }
}
```

Multi-cam ingest in splatd is tracked separately (**WEFT-711 / M0**). v1 single-cam hosts ignore unknown camera folders if `frames/` is present.

---

## 6. Host API — splatd `POST /v1/jobs`

splatd accepts **either** a video job (existing) **or** an image-set session.

### 6.1 Video (unchanged)

```bash
# Multipart video upload
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -F "video=@./walkaround.mp4" \
  -F "callback_url=https://example.com/hooks/splat"

# Local path already on the runner
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -H "content-type: application/json" \
  -d '{"source_path":"/abs/path/to/video.mp4","callback_url":null}'

# Remote URL (splatd downloads)
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -H "content-type: application/json" \
  -d '{"source_url":"https://storage.example/cap.mp4?sig=…","callback_url":"https://…"}'
```

### 6.2 Image-set session (A0)

```bash
# Multipart ZIP of a session directory (field name: session)
# ZIP root should contain frames/ and poses.jsonl (with or without a single top-level folder)
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -F "session=@./session-demo.zip" \
  -F "callback_url=https://example.com/hooks/splat"

# Absolute path to an unpacked session directory on the runner
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -H "content-type: application/json" \
  -d '{"session_path":"/abs/path/to/session_dir","callback_url":null}'
```

**Response (both):** `202 Accepted` with `{"job_id":"<uuid>"}`.

### 6.3 Tiny session for manual smoke

```bash
mkdir -p /tmp/weft-session/frames
# Drop a few JPEGs:
#   /tmp/weft-session/frames/000001.jpg
#   /tmp/weft-session/frames/000002.jpg
printf '%s\n' \
  '{"protocol":"weft.capture.v1","id":"demo","n_frames":2}' \
  > /tmp/weft-session/session.json
printf '%s\n' \
  '{"frame_id":1,"t_ns":1000,"path":"frames/000001.jpg","quat_wxyz":[1,0,0,0],"coverage_bin":0}' \
  '{"frame_id":2,"t_ns":2000,"path":"frames/000002.jpg","quat_wxyz":[1,0,0,0],"coverage_bin":1}' \
  > /tmp/weft-session/poses.jsonl

curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -H "content-type: application/json" \
  -d "{\"session_path\":\"/tmp/weft-session\"}"
# → {"job_id":"…"}
# Poll: GET /v1/jobs/{id}
```

ZIP form of the same tree:

```bash
(cd /tmp && zip -r weft-session.zip weft-session)
curl -sS -X POST "http://127.0.0.1:7860/v1/jobs" \
  -F "session=@/tmp/weft-session.zip"
```

---

## 7. Pipeline mapping (host)

| Stage | Video path | Image-set path |
|-------|------------|----------------|
| probe | ffprobe input video | Skip tool; optional metrics from `session.json` / first frame |
| frames | ffmpeg extract → `dataset/images/` | Copy/link `session/frames/*` → `dataset/images/` |
| sfm | COLMAP feature/match/map | Same (images only; poses not injected as priors in A0) |
| train | Brush | Same |
| compress | splat-transform | Same |
| package | manifest.json | Same; `source` reflects session path / `session-upload` |

Poses are retained under `session/poses.jsonl` for coverage metrics and future COLMAP prior injection (not A0).

---

## 8. Job workspace (after ingest)

```
data/jobs/<job-id>/
  session/                 # image-set only
    session.json
    poses.jsonl
    frames/…
  input.<ext>              # video only
  dataset/
    images/                # COLMAP/Brush input (always)
    sparse/0/
  artifacts/
  logs/
  job.json
```

---

## 9. Versioning

- Additive optional fields: no version bump.
- Breaking rename/remove of required pose fields or layout: bump to `weft.capture.v2` and dual-read for one release when possible.
- This file is the A0 freeze; Android app (A1) and pairing upload (A2) MUST emit this layout.

## 10. References

- ADR-077 Android edge capture  
- `docs/weftos/splat-pipeline-design.md` — HTTP surface  
- `docs/weftos/splat-multi-camera-rig.md` — multi-cam extension  
- `docs/weftos/splat-capture-sensor-head.md` — depth/radar sidecars  
