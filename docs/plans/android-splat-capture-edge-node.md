# Plan: Android native splat capture edge node

**Status:** Planned (2026-07-30)  
**ADR:** [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md)  
**Related:**

- [splat-pipeline-design](../weftos/splat-pipeline-design.md)
- [splat-harness](../weftos/splat-harness.md)
- [splat-multimodal-sensing](../weftos/splat-multimodal-sensing.md) — wavelengths, overlap, S25 inventory, hardware tiers
- [splat-capture-sensor-head](../weftos/splat-capture-sensor-head.md) — Pi + ToF/IMU lab head
- [splat-to-world-model](../weftos/splat-to-world-model.md) — object boundaries/volumes → BVH
- [ADR-078](../adr/adr-078-splat-feeds-world-model.md)

## 1. Problem

Gallery video → splatd works, but quality is luck:

- No live coverage guidance  
- No per-frame attitude (MP4 only has global Display Matrix)  
- Web capture is a poor sensor stack  

We need a **native Android app** that captures well **and** acts as a **WeftOS edge peer** streaming to Mac or cloud.

## 2. Goals

| Goal | Measure |
|------|---------|
| Guided capture | Coverage map shows filled vs missing view bins during session |
| Pose sidecar | Every frame has timestamp + rotation quaternion (GPS optional) |
| Host train | Session lands on splatd (Mac or cloud) without manual USB |
| WeftOS edge | Phone has node identity; can pair with host; control via versioned protocol |
| Review | Phone can open returned SOG (WebView/Spark or in-app) |

## 3. Non-goals (v1)

- Full COLMAP/Brush on phone  
- iOS  
- Full desktop Agent Workspace on device  
- Mesh multi-hop / WAN without relay  

## 4. Architecture

```
┌─ Phone ──────────────────────────────────────────────────┐
│  Compose UI                                              │
│    · viewfinder · coverage sphere · missing list · pair  │
│  Capture service (CameraX + SensorManager / ARCore)      │
│  Edge core (Rust UniFFI)                                 │
│    · identity · session state · upload/stream client     │
└─────────────┬────────────────────────────────────────────┘
              │  LAN: HTTPS + optional QUIC
              │  Cloud: HTTPS to SPLAT_RUNNER / gateway
              ▼
┌─ Host (Mac WeftOS + splatd) or Cloud ────────────────────┐
│  Ingest weft.capture.v1 → frames + poses.jsonl           │
│  SfM (COLMAP) → train (Brush) → SOG                      │
│  Progress events → phone                                 │
└──────────────────────────────────────────────────────────┘
```

### 4.1 On-device modules

| Module | Responsibility |
|--------|----------------|
| **Camera** | CameraX ImageCapture / ImageAnalysis; target 1080–1600 long edge; fixed or auto exposure |
| **IMU** | `TYPE_ROTATION_VECTOR` / game rotation; sample 50–100 Hz; attach nearest sample to each frame |
| **Coverage** | Discretize SO(3) or view direction on sphere (e.g. 8×16 bins); mark filled when frame captured while looking that way |
| **Guidance** | Rank empty bins; show arrow / text (“turn left / look up / walk back”); loop-closure hint |
| **Session store** | App-private dir: `frames/######.jpg`, `poses.jsonl`, `session.json` |
| **Edge core** | Pairing, auth, upload, job status |
| **Viewer** | After job: load SOG URL in WebView (reuse Spark harness) |

### 4.2 Host extensions (splatd / WeftOS)

| Change | Why |
|--------|-----|
| `POST /v1/jobs` accept **image archive + poses** (not only video) | Phone sends stills with telemetry |
| Optional `poses.jsonl` consumed later as COLMAP priors | Quality roadmap |
| Job metrics: `n_frames`, coverage % from session | Operator feedback |
| Progress webhook / poll already exists | Phone HUD |

### 4.3 WeftOS components on phone (phased)

| Phase | Component |
|-------|-----------|
| A3 | Ed25519 node id, config store, HTTP client with capability token |
| A4 | QUIC + Noise session to host weave/splat gateway |
| Later | Thin local agent (status, “start capture”, notifications); substrate publish of capture events |

Aligns with symposium language: phone is near-edge peer (`AndroidPlatform`), heavy train stays host.

## 5. Capture UX (operator flow)

1. **Pair** once with Mac (QR from `weft status` / splatd pair page)  
2. **New session** — room name, target quality (fast / balanced / high)  
3. **Record** — viewfinder + translucent sphere heatmap  
4. **Missing patches** panel — tappable targets that show desired look direction  
5. **Stop** — session summary (frame count, coverage %, weak zones)  
6. **Send** — stream/upload; progress bar (SfM / train)  
7. **Review** — open SOG on phone; “reshoot zone X” if host later sends hole list (A5)

## 6. Data format (session package)

```
session-<uuid>/
  session.json          # id, device, started_at, camera intrinsics hints
  poses.jsonl           # one JSON object per line
  frames/
    000001.jpg
    000002.jpg
    ...
  optional/
    preview.mp4         # low-res continuous if useful
    coverage.bin        # compact bin occupancy
```

`poses.jsonl` line example:

```json
{"frame_id":1,"t_ns":123,"path":"frames/000001.jpg","quat_wxyz":[0.7,0,0,0.7],"coverage_bin":12}
```

## 7. Transport modes

| Mode | When | Mechanism |
|------|------|-----------|
| **Export ZIP** | Offline / no host | Share sheet or Files |
| **HTTPS batch** | v1 LAN/cloud | Multipart or pre-signed upload of ZIP |
| **HTTPS stream** | v1.5 | Session create → PUT frames → finalize |
| **QUIC mesh** | A4 | Control + telemetry streams; bulk may still HTTP |

LAN-first assumption: phone and Mac on same Wi‑Fi. Cloud: token + splatd URL.

## 8. Why this fixes “roll / explode” better than Gallery video

| Source | Gallery MP4 | Native app |
|--------|-------------|------------|
| Global upright | Display Matrix only | Continuous rotation vector |
| Per-frame attitude | ❌ | ✅ sidecar |
| Coverage feedback | ❌ | ✅ live |
| Multi-model COLMAP | Silent | Fewer bad frames via guidance; host still keeps largest model |
| Future SfM priors | ❌ | Poses ready for injection |

IMU alone is not metric VIO; ARCore (A6) upgrades further. Guidance alone already reduces pure-spin and thin-coverage sessions.

## 9. Tech stack choices

| Choice | Decision | Rationale |
|--------|----------|-----------|
| UI | Kotlin + Compose | Standard Android camera apps |
| Camera | CameraX | Lifecycle-safe, wide device support |
| Sensors | SensorManager first; ARCore optional | Works without Play Services VIO |
| Rust bridge | UniFFI → Kotlin | Matches WeftOS crate reuse |
| Min SDK | 29+ (Android 10) | Rotation vector + scoped storage patterns |
| Train | Host only | Battery/GPU reality |

## 10. Repo layout (proposed)

```
apps/android-splat-capture/          # Gradle project
  app/src/main/...
crates/clawft-android-edge/          # UniFFI: pair, session upload, identity
docs/plans/android-splat-capture-edge-node.md  # this file
docs/adr/adr-077-...md
```

Do not put the Gradle tree under `crates/`. Keep Android tooling isolated.

## 11. Phased delivery

### A0 — Spec + host ingest (0.8.x) — **WEFT-704**

- [x] ADR-077  
- [x] splatd: accept `multipart` image set **or** zip with `poses.jsonl`  
- [x] Document `weft.capture.v1` schema (`docs/weftos/capture-protocol-v1.md`)  
- [x] Plane tickets  

### A1 — Offline capture MVP — **WEFT-705**

- [x] CameraX capture + IMU attach  
- [x] Coverage sphere UI + missing bins  
- [x] Local session package + share ZIP  
- [x] Manual: unzip → host train path  

### A2 — Phone → Mac splatd — **WEFT-706**

- [x] Pairing UX (URL + token or QR)  
- [x] Upload session → job_id  
- [x] Progress UI  
- [x] Open result SOG in WebView  

### A3 — WeftOS edge core in-app — **WEFT-707**

- [x] Rust identity + config via UniFFI  
- [x] Same node id across restarts  
- [x] Capability-scoped host token  

### A4 — Stream mode + resiliency *(later)*

- [ ] Chunked/resumable upload  
- [ ] QUIC/Noise optional path  
- [ ] Background foreground service  

### A5 — Host-guided holes *(later)*

- [ ] Fast SfM or sparse preview on host  
- [ ] Return empty-region hints to phone mid-session  

### A6 — ARCore priors *(later)*

- [ ] Optional VIO pose  
- [ ] Pipeline: COLMAP with prior poses when present  

## 12. Risks

| Risk | Mitigation |
|------|------------|
| Thermal throttling | Adaptive FPS; pause guidance; lower res |
| IMU drift | Coverage is relative; ARCore later; don’t overclaim metric accuracy |
| NAT / hotel Wi‑Fi | Cloud splatd + HTTPS first |
| Device variance | CameraX test matrix; min SDK 29 |
| Dual world COLMAP | Host keep-largest-model (already fixed in pipeline) |

## 13. Success criteria (v1 = A2)

1. Operator completes a room capture with ≥80% coverage bins filled (heuristic) without leaving the app.  
2. Session uploads to Mac splatd and produces SOG without USB.  
3. Pose sidecar present for ≥95% of frames.  
4. Phone can view result SOG.  
5. Edge identity exists (A3) or is stubbed with clear upgrade path.

## 14. Open questions

1. **Video vs stills primary?** Stills+poses preferred for COLMAP; optional low-res video for humans.  
2. **ARCore required?** No for MVP; feature-detect.  
3. **Single app vs WeftOS Android shell?** Start dedicated splat capture app; merge into broader WeftOS Android shell later.  
4. **Cloud auth product?** Reuse XAI/host tokens vs device-paired certs — decide at A2.

## 15. Immediate next engineering steps

1. Freeze `weft.capture.v1` JSON schema in `docs/weftos/capture-protocol-v1.md`.  
2. Extend splatd job create for image-set ingest.  
3. Scaffold `apps/android-splat-capture` with CameraX hello + orientation bin painter.  
4. Pair with existing retrain/quality notes so host side remains single-model train.
