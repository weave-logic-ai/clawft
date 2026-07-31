# Multi-modal sensing for solid Gaussian splat capture

**Status:** Living design note (2026-07-30)  
**Related:**

| Doc | Role |
|-----|------|
| [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md) | Android / edge capture node |
| [ADR-078](../adr/adr-078-splat-feeds-world-model.md) | Splat → structured world model (objects/volumes) |
| [splat-to-world-model.md](./splat-to-world-model.md) | Boundaries, volumes, BVH population |
| [splat-multi-camera-rig.md](./splat-multi-camera-rig.md) | Fixed multi-cam known poses |
| [splat-freeform-quilt.md](./splat-freeform-quilt.md) | Accumulating free-form quilt |
| [splat-capture-sensor-head.md](./splat-capture-sensor-head.md) | Pi + 3D-print head from lab sensors |
| [splat-pipeline-design.md](./splat-pipeline-design.md) | Host splatd stages |
| [splat-harness.md](./splat-harness.md) | Train/view harness + quality notes |
| [android-splat-capture-edge-node.md](../plans/android-splat-capture-edge-node.md) | Phone app plan |
| Galaxy S25 sensor notes | §8 below |

Gaussian splatting (and COLMAP before it) mostly care about one thing first: **many views of the same surface, with consistent appearance and known geometry**. Extra wavelengths and sensors help when they **add constraints** (pose, scale, texture where RGB fails)—not when they replace multi-view RGB.

---

## 1. What “overlap” means (three layers)

| Layer | Meaning | Without it |
|-------|---------|------------|
| **A. View overlap (RGB)** | Same surface in ≥3–5 frames with baseline | Dual models, explode, floaters |
| **B. Temporal / pose continuity** | Smooth trajectory + timestamps | Broken tracks, bad roll |
| **C. Cross-modal overlap** | Depth/IR/radar registered to same pixels/poses | Pretty RGB but wrong scale/holes |

**Start with A+B.** Layer C upgrades scale, low light, and guided recapture.

### View-overlap heuristics (layer A)

| Rule | Guidance |
|------|----------|
| Neighbor FOV | **~60–80%** shared with previous frame |
| Angular step | **5–15°** for rooms (slower at corners) |
| Baseline | Small translations, not pure spin in place |
| Loop | Return toward start so the view graph can close |
| Photometry | Lock exposure / white balance / focus when possible |

### Pose continuity heuristics (layer B)

| Rule | Guidance |
|------|----------|
| Timestamps | Shared clock (`t_ns`) on every sample |
| Attitude | Rotation vector / VIO / IMU fusion @ 50–200 Hz |
| Motion | Prefer step–shoot over blurry continuous pan |
| Coverage UI | Fill view-direction bins before “session complete” |

### Cross-modal heuristics (layer C)

| Rule | Guidance |
|------|----------|
| Rigid extrinsics | Calibrate `T_cam_tof`, `T_cam_imu` once |
| Sync | Depth/radar attached to nearest RGB frame time |
| Role | Sidecars for scale/QA/priors—not sole train input |

---

## 2. Wavelength / modality catalog

### 2.1 Visible RGB (~400–700 nm) — primary

| Variant | Role | Overlap strategy |
|---------|------|------------------|
| Normal wide / phone main | Texture for SfM + splat color | Primary train path |
| Ultrawide | More FOV → easier A-layer | Separate calibration or one lens in v1 |
| Tele | Distant detail | Secondary pass after structure exists |
| 360 dual-fisheye | Instant full-sphere A-layer | Unwrap/equirect; dual-cam model |
| Global shutter / action cam | Less rolling-shutter warp | Better when moving |
| Locked exposure / WB / focus | Photometric consistency | Critical for feature matching |

Multi-modal 3DGS research still almost always hangs geometry on **RGB multi-view** or RGB+depth SLAM (vision + depth + IMU).

### 2.2 Near-IR / IR-assisted (~700–1000 nm)

| Source | Role | Notes |
|--------|------|--------|
| IR flood + mono IR cam | Texture on flat walls where RGB is bland | Robot depth / active stereo class |
| Phone IR (face unlock) | Not a mapping camera | Locked down, wrong FOV |
| Active stereo IR pattern | Dense depth indoors | RealSense-class; strong C-layer |

Helps **featureless white walls** where COLMAP starves.

### 2.3 Thermal LWIR (~8–14 µm)

| Role | Overlap use |
|------|-------------|
| Heat texture independent of lighting | Night / dark rooms; second appearance channel |
| Not a substitute for RGB detail | Low res, soft edges; fuse as extra modality |

Research exists for multi-modal / thermal Gaussian splatting (joint RGB+thermal reconstruction). Worth it if you already have LWIR and care about **low light / machinery**. Skip for sunny room walkthroughs.

### 2.4 Depth (geometric, not a color band)

| Method | Range / density | Overlap role |
|--------|-----------------|--------------|
| ToF multi-zone (VL53L5CX / L7CX) | Short, **sparse** 4×4–8×8 | Hole flags, near obstacles, crude scale |
| ToF imaging (Kinect-class) | Room-scale denser | Depth maps for SLAM / priors |
| Stereo RGB or IR | Metric if calibrated | Strong C-layer with RGB |
| Structured light | Indoor dense | High quality close range |
| LiDAR (spinning / solid-state) | Metric sparse–dense | Scale + structure; not texture |
| mmWave radar | Sparse range, some clutter penetration | Occupancy / range gates |
| Ultrasound / sonar | Range, domain-specific | Water / open; multipath indoors |
| ARCore / depth-from-motion | Soft depth while moving | Free on phone; needs motion |

**Depth does not create multi-view texture.** It **anchors** views that RGB already overlaps.

WeftOS already models multi-zone ToF:

- Substrate path: `substrate/sensor/tof`
- UI: VL53L5CX (4×4) / VL53L7CX (8×8) heatmap (`weftos-chip-tof` fixture)

### 2.5 Polarization / multispectral / event / X-ray (specialist)

| Modality | When interesting | Indoor splat v1 |
|----------|------------------|-----------------|
| Polarimetric | Specular / glass / materials | Research |
| Multispectral | Agriculture / materials | Overkill |
| Event cameras | High-speed, HDR motion | Exotic SfM |
| X-ray / radiative GS | See-through lab setups | Lab only |

### 2.6 Motion / position (not wavelengths; overlap glue)

| Sensor | Role |
|--------|------|
| Gyro + accel + magnetometer | Attitude continuity; coverage bins |
| VIO (ARCore, OpenVINS, …) | Ordered, spaced trajectory |
| Wheel odom / step counter | Weak indoor scale |
| GPS / baro / UWB | Outdoor geolock / floor; not dense overlap |
| Motor encoders on gimbal | Known angular steps → **planned** overlap |

---

## 3. Methods that create solid overlap

### Pattern 1 — Photogrammetry orbit (gold standard)

```
Slow walk / arc  →  60–80% FOV overlap  →  COLMAP  →  Brush
```

**Tools:** phone, Pi cam, mirrorless, gimbal.  
**Add-ons:** IMU poses, fill lights.  
**Overlap control:** coverage sphere UI (ADR-077), not more radar.

### Pattern 2 — 360 continuous

```
360° video  →  dense angular samples  →  frames  →  SfM
```

**Tools:** Insta360-class (~$400–600).  
**Overlap:** built-in; risk is stitch/fisheye model.  
**Best cheap “don’t miss the ceiling” RGB path.**

### Pattern 3 — Step–shoot on a path / gimbal

```
Move Δθ or Δx → stop → shoot RGB (+ depth) → repeat
```

**Tools:** Pi pan/tilt + ToF kit.  
**Overlap:** engineered fixed step size.  
**Avoid:** continuous blurry pan.

### Pattern 4 — RGB-D SLAM then splat

```
RGB + depth + IMU  →  poses + cloud  →  splat train with poses
```

**Tools:** RealSense, Azure Kinect, iPhone LiDAR apps, phone ARCore.  
**Effect:** SLAM enforces consecutive-frame consistency (layers A+B+C).

### Pattern 5 — Multi-modal GS (advanced)

```
RGB Gaussians + thermal (or depth) channels, joint train
```

**When:** night, industrial heat, research.  
**Not** day-one for WeftOS room SOG.

### Pattern 6 — Sparse depth / radar as prior only

```
RGB multi-view (main)
  + VL53/mmWave ranges for scale & “missing wall” QA
  + IMU for attitude
```

**Correct reuse of lab ToF / radar kits.**

---

## 4. Failure modes → which sensor helps

| Failure mode | What fails | Best fix |
|--------------|------------|----------|
| Exploded dual models | Weak view graph | More RGB overlap; single COLMAP model; guidance |
| Floaters / noise | Bad poses / motion blur | Step-shoot, lock exposure, better light |
| Blank walls | No features | IR texture / projector, temporary markers |
| Dark scenes | No texture | Lights; thermal only as research path |
| Wrong scale | Pure monocular | Depth / stereo / LiDAR / known baseline |
| Missed ceiling/corners | Path skill | 360 cam or coverage UI |
| Glass / black | Depth fails | RGB multi-view; don’t trust ToF alone |

---

## 5. Practical stacks (cheap → rich)

### Tier 0 — Minimum solid start

- RGB stills or high-bitrate video  
- IMU attitude  
- Coverage map (view bins)  
- Host: keep **largest COLMAP model** only  

Solves most “explode” without exotic wavelengths.

### Tier 1 — Best inexpensive multi-sensor

| Piece | Role |
|-------|------|
| Phone or Pi Cam RGB | A-layer |
| BNO / phone rotation vector | B-layer |
| VL53L7CX | Near depth QA + scale hints |
| LED fill light | Feature density |
| Optional gimbal step-shoot | Planned overlap |

### Tier 2 — Strong geometry start

| Piece | Role |
|-------|------|
| RGB-D (RealSense / Kinect / iPhone LiDAR) | Dense depth + RGB |
| Or 360 RGB | Angular completeness |
| IMU / VIO | Trajectory |

### Tier 3 — Domain / research

| Piece | Role |
|-------|------|
| Thermal LWIR + RGB | Multi-modal GS (night/industrial) |
| mmWave | Occupancy through clutter |
| Sonar | Water / special domains |
| Polarimetric / event | Specular / high-speed niches |

### Tier 4 — Turnkey pro spatial camera

| Piece | Role |
|-------|------|
| Multi-cam + LiDAR (e.g. PortalCam-class ~$5k) | Strong A+C out of box; not “lab scrap” cheap |

---

## 6. Off-the-shelf vs custom (hardware position)

### Ideal hardware wish list

| Need | Why | Ideal |
|------|-----|--------|
| Overlap | SfM fails without shared features | Wide FOV or multi-cam |
| Fixed optics | One calibration | Lock zoom/focus/WB/exposure |
| Sharpness + light | Soft frames → floaters | Large sensor, OIS, LEDs |
| Motion discipline | Pure spin kills tracks | Slow path, step-shoot |
| Attitude / trajectory | Coverage + priors | IMU / VIO / SLAM |
| 360° coverage | Missed ceilings | Dual-fisheye or multi-pass |
| Scale (optional) | Metric rooms | LiDAR / stereo / depth |
| Time sync | Fusion | Shared clock |

### Inexpensive market options that work *really* well

| Budget | Option | Expectation |
|--------|--------|-------------|
| **$0** | S25 / phone + capture app | Solid rooms if path is good |
| **~$50–150** | Gimbal + LED | Biggest jump per dollar on phone |
| **~$400–600** | **Insta360 X4/X5 / X4 Air** | Best cheap dedicated full-sphere RGB |
| **~$500–800 used** | iPhone Pro + LiDAR apps | Scale/mesh; less native to Android edge |
| **~$80–200 parts** | Pi + VL53 + IMU + print (if you own sensors) | Custom WeftOS edge; see sensor-head doc |
| **~$5k** | PortalCam-class | Near-ideal; different product tier |

**Calendar vs strategy:** Insta360 or phone app for pretty rooms soon; **Pi/ToF head** for WeftOS substrate identity and lab sensor reuse.

### Phone reality: Galaxy S25 series (summary)

Full table in §8. Headline:

| | |
|--|--|
| **LiDAR / ToF imaging** | **No** dedicated LiDAR or product ToF depth cam |
| **RGB** | Excellent multi-lens (Ultra: 200 MP main + 50 MP UW + dual tele) |
| **IMU / GPS / baro** | Yes — use natively |
| **ARCore depth** | Software depth-from-motion, not hardware LiDAR |
| **UWB** | S25+ / Ultra (peer ranging, not room mesh) |

---

## 7. One diagram

```
                 ┌──────────────────────────────────────┐
                 │     SOLID START = VIEW GRAPH         │
                 │  many overlapping RGB observations   │
                 └───────────────┬──────────────────────┘
                                 │
         ┌───────────────────────┼───────────────────────┐
         ▼                       ▼                       ▼
   Texture helpers         Pose continuity          Geometric anchors
   lights, IR pattern      IMU, VIO, encoders       ToF, stereo, LiDAR
   thermal (dark)          timestamps               radar range (sparse)
                                 │
                                 ▼
                    COLMAP / SLAM poses + cloud
                                 │
                                 ▼
                         Brush / 3DGS train
```

---

## 8. Galaxy S25 series — developer sensor inventory

For capture-app design (not marketing).

### Cameras (RGB only for mapping)

**S25 Ultra**

| Lens | Spec (marketing) | Splat use |
|------|------------------|-----------|
| Main wide | 200 MP, OIS | Primary; downsample long edge ~1600–2000 |
| Ultrawide | 50 MP | Extra FOV; separate intrinsics if multi-lens |
| Tele 3× | 10 MP | Detail pass, not base room graph |
| Tele 5× | 50 MP optical | Same |
| Front | 12 MP | Usually ignore |

**S25 / S25+**

| Lens | Spec |
|------|------|
| Main | 50 MP wide + OIS |
| Ultrawide | 12 MP |
| Tele | 10 MP, 3× optical |

### Wavelengths / special modes

| Mode | Available? | Notes |
|------|------------|--------|
| RGB still / video | Yes | Core path |
| Expert RAW / Log | Yes | Low light / grading; SfM still wants consistent frames |
| Thermal / multispectral map | **No** | — |
| Hardware LiDAR | **No** | — |
| Dedicated ToF depth camera | **No** | Dual Pixel AF ≠ dense depth API |

### Depth

| Source | Hardware? | Notes |
|--------|-----------|--------|
| LiDAR | No | — |
| Product ToF | No | — |
| ARCore Depth / Raw Depth | Software | Motion stereo; feature-detect at runtime |
| Portrait soft depth | Computational | Not metric SfM |

### Motion & position

| Sensor | Use |
|--------|-----|
| Accel / gyro / mag / rotation vector | Coverage bins + `poses.jsonl` |
| GPS / GNSS | Outdoor geolock |
| Barometer | Relative height |
| UWB (S25+/Ultra) | Peer ranging; not room LiDAR |

### Capture app priorities on S25

**Must:** main RGB + rotation vector + timestamps + coverage bins.  
**Should:** optional GPS; Camera2 exposure metadata.  
**Could:** ARCore pose/depth; Expert RAW dark rooms.  
**Skip:** tele-only whole rooms; expecting LiDAR PLY from the phone.

---

## 9. Recommended WeftOS phase order

| Phase | Focus |
|-------|--------|
| **1** | Engineered RGB multi-view + IMU coverage (phone ADR-077 and/or Pi head) |
| **2** | VL53 + IMU sidecar → depth/pose package next to frames; host QA |
| **3** | Denser depth (RealSense) **or** 360 RGB if coverage still fails |
| **4** | Thermal / radar / sonar only for domain scenarios; never replace RGB multi-view |
| **5** | Structure stage → object AABBs / free-space → BVH ([ADR-078](../adr/adr-078-splat-feeds-world-model.md)) |

Host pipeline remains: **single largest COLMAP model** before train (see splat-harness quality notes).  
World-model population: [splat-to-world-model.md](./splat-to-world-model.md).

---

## 10. History

- 2026-07-30: Initial write-up from capture quality review (exploded splat job), S25 inventory, hardware tiers, multi-modal design, and lab sensor-head discussion.
