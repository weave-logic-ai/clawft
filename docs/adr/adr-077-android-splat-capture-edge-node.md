# ADR-077: Android native splat capture as a WeftOS edge node

**Date**: 2026-07-30  
**Status**: Accepted (plan)  
**Deciders**: product + edge (owner: native Android beats web capture for camera/IMU/background; phone is a WeftOS peer, not a dumb uploader)  
**Depends-On**: ADR-026 (QUIC transport), ADR-024/025 (Noise + Ed25519 identity), splatd pipeline (`docs/weftos/splat-pipeline-design.md`), existing `Kernel<AndroidPlatform>` direction (Mentra / ECC symposium)  
**Relates-To**: ADR-056 (BVH world model later), ADR-073 (workspace is host-side), ADR-074 (voice is separate)

## Context

Phone walkaround video → COLMAP → Brush already works via **splatd** on Mac/cloud, but capture quality is the bottleneck:

- Gallery MP4s only carry a **global** display rotation, not per-frame attitude  
- Operators get no live guidance for missing angles / weak coverage  
- Web capture is limited (background, camera control, IMU rates, thermal, storage)

WeftOS long-term already treats the phone as **`Kernel<AndroidPlatform>`** (near-edge peer), not only a browser. A **native Android app** should:

1. Capture high-quality **frames + IMU/pose telemetry** with coverage UX  
2. Run **WeftOS edge components** (identity, mesh client, local agent hooks)  
3. **Stream or batch** to a main node (Mac splatd / cloud SPLAT_RUNNER / weave substrate)

## Decision

### 1. Native Android app is the capture client (not a PWA)

| Capability | Why native |
|------------|------------|
| Camera2 / CameraX | Full control of resolution, FPS, focus, exposure, HEIC/JPEG |
| IMU + rotation vector | High-rate attitude for pose priors / coverage map |
| ARCore (optional tier) | Visual-inertial odometry when available |
| Foreground service | Continuous capture while screen dimmed |
| Local disk + resumable upload | Multi‑GB sessions without browser quotas |
| JNI / UniFFI WeftOS crates | Same identity + transport as desktop kernel |

Web remains optional for **view-only** SOG (`examples/splat-viewer`); it is **not** the capture path of record.

### 2. Phone is a WeftOS edge node, not a pure HTTP uploader

```
┌─────────────────────────────────────────────────────────────┐
│  Android app (Kotlin UI + Rust edge core)                   │
│  ┌──────────────┐  ┌─────────────────────────────────────┐  │
│  │ Capture UX   │  │ WeftOS edge (Rust, aarch64-android) │  │
│  │ CameraX+IMU  │  │  · Ed25519 node identity            │  │
│  │ Coverage map │  │  · QUIC/Noise session to host       │  │
│  │ Session mgmt │  │  · Capture bus (frames + telemetry) │  │
│  └──────┬───────┘  │  · Optional: local agent / skills   │  │
│         │          │  · Health / substrate publish hooks │  │
│         └──────────┤─────────────────────────────────────┘  │
└────────────────────┼────────────────────────────────────────┘
                     │  (A) QUIC mesh  or  (B) HTTPS to splatd
                     ▼
┌─────────────────────────────────────────────────────────────┐
│  Main node: Mac WeftOS + splatd  OR  cloud SPLAT_RUNNER     │
│  · ingest capture session → frames/video + pose sidecar     │
│  · SfM + train + SOG                                        │
│  · push progress + result SOG back to phone for review      │
└─────────────────────────────────────────────────────────────┘
```

**Normative:** the phone holds a **node identity** and speaks a **versioned capture protocol**. Bulk pixels may still use HTTP range/multipart for simplicity in v1; control plane and telemetry prefer mesh (ADR-026).

### 3. Split: what runs on phone vs host

| On phone (edge) | On Mac / cloud (heavy) |
|-----------------|------------------------|
| Camera + IMU sampling | COLMAP / GLOMAP SfM |
| Coverage / missing-patch UI | Brush train (GPU) |
| Frame extract / compress (JPEG/HEIC) | SOG package |
| Pose sidecar (quats, timestamps, GPS) | Full agent workspace |
| Session packaging + stream | Job queue / governance |
| Lightweight WeftOS: identity, session, status, optional chat/agent | Full kernel, splatd, desktop |
| Optional: on-device preview / tiny AR | Optional: remote progress SOG preview |

**Do not** ship full COLMAP+Brush on phone for v1. Brush *can* target Android later; product v1 trains on host.

### 4. Capture product surface

**Session** = ordered frames + `poses.jsonl` + optional continuous video + coverage heatmap.

**Live guidance (MVP):**

- Spherical (or icosahedral) **coverage bins** filled by device orientation while shooting  
- “Missing patch” list: empty bins above horizon / floor / back-wall  
- Distance / motion hints: move slower, more overlap, close the loop  
- Global upright from Android rotation vector (better than MP4 Display Matrix alone)

**v1.5+:**

- ARCore camera pose when available → stronger priors for SfM  
- Live link: host runs quick SfM preview and returns **sparse cloud + red “holes”** to the phone HUD  
- Resumable multi-session for large spaces  

### 5. Wire protocol (capture → host)

Define **`weft.capture.v1`** (name flexible) with two transports:

| Mode | Use |
|------|-----|
| **Batch** | ZIP/tar of frames + poses → `POST /v1/jobs` (extend splatd for image set, not only video) |
| **Stream** | Open session; chunk frames + pose lines over QUIC streams or HTTPS chunked upload; host finalizes job |

Minimum payload fields per frame:

```json
{
  "t_ns": 0,
  "frame_id": 0,
  "quat_wxyz": [1,0,0,0],
  "accel": null,
  "gyro": null,
  "gps": null,
  "cam": { "w": 1080, "h": 1920, "focal_hint": null },
  "coverage_bin": 42
}
```

Host maps poses into COLMAP prior path when mature; until then poses still power **coverage QA** and future prior injection.

### 6. App packaging

| Layer | Tech |
|-------|------|
| UI | Kotlin + Jetpack Compose + CameraX |
| Edge core | Rust crate(s) via UniFFI or `jni` → AAR |
| Identity / mesh | Reuse clawft-weave / platform traits; `AndroidPlatform` |
| Config | Pairing QR (host mDNS + node pubkey) or manual host URL + token |
| Store | Sideload first; Play later |

Repo layout (proposal):

```
apps/android-splat-capture/     # Gradle app
crates/clawft-android-edge/     # UniFFI surface: session, identity, upload client
# or extend clawft-platform + thin android crate
```

### 7. Pairing with Mac WeftOS

1. Mac runs `splatd` + optional WeftOS daemon  
2. Phone scans QR: `weftos://pair?host=…&port=…&node=…&v=1`  
3. Mutual identity exchange (Ed25519); store trusted host  
4. Capture → stream → job id → progress notifications → pull SOG for on-device viewer (WebView/Spark or system)

## Non-goals (v1)

- iOS app (same protocol later; Android first)  
- Full on-device Gaussian training  
- Replacing Mentra glasses stack  
- Running full Agent Workspace on phone  
- Web-as-primary capture path  

## Consequences

### Positive

- Capture quality becomes a product surface, not “hope the Gallery video is good”  
- Same identity/mesh story as the rest of WeftOS  
- Mac/cloud keep heavy GPU work  
- Aligns with existing `Kernel<AndroidPlatform>` doctrine  

### Negative / costs

- Android + Rust toolchain burden  
- Pairing and NAT traversal complexity (LAN first; cloud relay later)  
- ARCore device fragmentation  
- Two transports (mesh + HTTP) until unified  

## Implementation phases

| Phase | Plane | Work | Exit |
|-------|-------|------|------|
| **A0** | **WEFT-704** | This ADR + plan; extend splatd to accept **image set + poses** jobs | Spec frozen |
| **A1** | **WEFT-705** | Android MVP: CameraX stills, IMU coverage sphere, local session export ZIP | Offline capture quality ↑ |
| **A2** | **WEFT-706** | Pairing + HTTPS upload to Mac splatd; progress poll | End-to-end phone→Mac→SOG |
| **A3** | **WEFT-707** | WeftOS edge core (identity + session client) in-app | Real node, not only REST |
| **A4** | *(later)* | QUIC/Noise stream mode; resumable chunks | Robust LAN/cloud stream |
| **A5** | *(later)* | Host→phone coverage holes from quick SfM preview | Closed-loop guided capture |
| **A6** | *(later)* | Optional ARCore VIO priors into pipeline | Better roll/pose for SfM |

## References

- `docs/plans/android-splat-capture-edge-node.md` — operational plan  
- `docs/weftos/capture-protocol-v1.md` — **frozen** weft.capture.v1 layout + splatd image-set API (A0 / WEFT-704)  
- `docs/weftos/splat-multimodal-sensing.md` — modalities, overlap, S25 sensors, hardware tiers  
- `docs/weftos/splat-capture-sensor-head.md` — Pi + lab ToF/IMU/radar head  
- `docs/weftos/splat-pipeline-design.md`  
- Mentra / ECC: phone as `Kernel<AndroidPlatform>`  
- ADR-026 QUIC, ADR-024 Noise, ADR-025 identity  
