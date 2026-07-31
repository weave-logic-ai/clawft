# Multi-camera fixed rigs (known positions)

**Status:** M0 schema shipped (WEFT-711, 2026-07-31) — design + `cameras.json` parse / multi-folder layout in `clawft-splat-pipeline::multi_cam`  
**Mode:** Infrastructure / room install — *not* free-walk phone capture  

**Related:**

| Doc | Role |
|-----|------|
| [splat-freeform-quilt.md](./splat-freeform-quilt.md) | Time-accumulating quilt when seams are OK |
| [splat-to-world-model.md](./splat-to-world-model.md) | Objects / volumes / BVH (ADR-078) |
| [splat-multimodal-sensing.md](./splat-multimodal-sensing.md) | Overlap layers, modalities |
| [splat-pipeline-design.md](./splat-pipeline-design.md) | Host splatd stages |
| [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md) | Edge nodes (phone/Pi can *be* a fixed cam) |

---

## 1. Intent

Use **3–4+ cameras in prime spots with known poses** to reconstruct a space and feed appearance + world model—without relying on a single handheld walk.

```
Cam A ──┐
Cam B ──┼── known T_world_cam_i + time sync ──► multi-view train
Cam C ──┤
Cam D ──┘
         │
         ▼
   one world frame → SOG + structure leaves
```

**Strengths:** stable graph, repeatable sessions, metric scale if calibrated, great for always-on rooms.  
**Tradeoffs:** install/calibration cost; coverage limited to FOV union unless cams move or you add more.

---

## 2. Short answers

| Question | Answer |
|----------|--------|
| 3–4 cams, known positions? | **Yes** — preferred for fixed spaces |
| Overlay / composite splat? | **Yes** — one shared world frame (default: single train) |
| How far off-axis? | Governed by **overlap + baseline + face-on views**, not one angle |
| Add more cameras? | **Yes** — grow an **overlap graph**; each new cam must see neighbors |

---

## 3. How it works

### 3.1 Known poses

Obtain `T_world_cam` (and intrinsics) by one of:

| Method | Notes |
|--------|--------|
| Survey / CAD | Tape, level, room plan, total station |
| Marker board | ArUco / ChArUco on floor/walls; solve once |
| Free SfM then freeze | COLMAP once; **lock** extrinsics for all future sessions |
| COLMAP **rig** model | Rigid multi-sensor head (fixed relatives); one moving platform |

Product default for room installs: **calibrate once → lock**.

### 3.2 Session

1. Sync clocks (or assume static scene).  
2. Capture burst or short clip from all cams.  
3. Package: per-cam image sets + `cameras.json` (intrinsics + extrinsics).  
4. Host: multi-view SfM (optional if poses locked) → train → SOG + structure.

### 3.3 Overlay patterns

| Pattern | Description | When |
|---------|-------------|------|
| **A. Single train** | All images, one COLMAP model, one Brush run | **Default** |
| **B. Time slices** | Fixed cams, many timestamps | Slow head pan or scene change |
| **C. Spatial tiles** | Per-zone sub-splats in one world frame | Large buildings; watch seams |
| **D. Appearance + structure** | SOG + WM_* leaves | Agents need bounds (ADR-078) |

**Overlay** here means: everything in one **gravity-aligned world frame**, cameras as fixed nodes—not four disconnected splat islands.

---

## 4. Geometry: coverage and off-axis

### 4.1 Between neighboring cameras

| Quantity | Target |
|----------|--------|
| FOV overlap on shared surfaces | **~50–70%** |
| Baseline | Often **0.2–1×** distance to nearest important surfaces (e.g. 1–3 m baseline for 3–6 m room depth) |
| Vergence | Look into a **shared volume**, not parallel to infinity |
| Inter-axis angle | Often **30–90°** between neighbors; corner **~90°** OK if both see shared wall/floor |
| Too parallel + far | Weak depth / mushy structure |
| Too little overlap | Dual models / explode |
| Grazing-only | Bad texture; need some face-on coverage |

### 4.2 Surface view angle

| Angle from surface normal | Effect |
|---------------------------|--------|
| 0–45° | Strong features, good Gaussians |
| 45–70° | Usable if textured + overlapping |
| >70–80° grazing | Elongated Gaussians, holes |

A camera may sit “far off” in the room **if** another cam sees the same surfaces more face-on **and** FOV overlap exists.

### 4.3 Example 4-cam room layout

```
        Cam2 (high, corner)
       /                    \
Cam1 ●──────────────────────● Cam3
 (eye height)            (eye height)
       \                    /
        Cam4 (door / high)
```

- Prefer shared **floor** texture in multiple views (graph glue + scale).  
- Avoid all cams on one wall.  
- High corners help ceilings; mid-wall helps furniture face-on.

### 4.4 Resolution vs distance

Aim for readable texture on key objects (order-of-magnitude **~5–15 mm/px** on important surfaces, or at least train-res “sharp enough”). Far low-res cams → soft wallpaper, weak object AABBs.

---

## 5. Time synchronization

| Quality | Use |
|---------|-----|
| Hardware genlock / trigger | Best (dynamic scenes) |
| NTP/PTP software | Good for semi-static |
| Same-second software fire | OK if **scene static** |
| Unsynced + motion | Ghosts, double geometry |

Static empty room: ~100–500 ms skew often tolerable. People walking: invest in sync.

---

## 6. Extending the install

Treat cameras as a **graph**:

```
New camera must:
  1. Overlap ≥1 existing cam (prefer 2)
  2. Pose in same world frame
  3. Shared timebase (or static assumption)
  4. Similar exposure/WB policy when possible
```

| Growth | Strategy |
|--------|----------|
| Larger room | Chain of overlaps across volume |
| Next room | Doorway cam bridges clusters |
| Detail zone | Close cam with known pose on object of interest |
| World model | Extra views complete incomplete `WM_OBJECT` faces |

**Rebuild policy (v1):** retrain with full image set. Incremental local patches are a later optimization (see free-form quilt for a different product stance).

---

## 7. Known poses vs free SfM

| Mode | Pros | Cons |
|------|------|------|
| Free SfM every time | No survey | Can fail; free scale; dual models |
| **Locked known extrinsics** | Stable, metric if scaled | Up-front calibration |
| COLMAP rig | Fixed multi-head | One rigid body |

**Room prime spots → lock extrinsics.**

---

## 8. Host / WeftOS integration

| Piece | Role |
|-------|------|
| Per-cam edge (Pi/phone/IP) | Capture + optional local IMU/ToF |
| `cameras.json` / COLMAP `images.bin` seed | Intrinsics + extrinsics |
| splatd | Ingest multi-cam session package |
| `SPLAT_CAMERA` leaves | Fixed camera Events in BVH |
| Structure stage | Objects/volumes from multi-view (ADR-078) |
| Viewer | One SOG + frustum + AABB overlays |

### Session package sketch

```
session-<uuid>/
  cameras.json           # protocol weft.multicam.v1; id, model, K, T_world_cam, sync
  cam_a/frames/...
  cam_b/frames/...
  poses optional if moving heads
  session.json
```

### M0 implementation (WEFT-711)

| Piece | Location |
|-------|----------|
| Schema types | `clawft_splat_pipeline::{CamerasDocument, CameraEntry}` |
| Parse / detect | `parse_cameras_json`, `is_multi_cam_session`, `parse_multi_cam_session` |
| Protocol id | `weft.multicam.v1` (`MULTICAM_PROTOCOL_V1`) |
| Dry-run flatten | `flatten_frames` → ordered list for existing image-set train path |

**Manual 2-cam dry-run:** build a directory with `cameras.json` (≥2 entries) and `cam_a/frames`, `cam_b/frames` image files; call `parse_multi_cam_session`. Full splatd multi-folder ingest job API is M1.

---

## 9. Relation to free-form quilt

| Fixed multi-cam (this doc) | Free-form quilt |
|----------------------------|-----------------|
| Prefer seamless **single** train | Accept seams; **accumulate** over time |
| Install-time calibration | Continuous adds with known camera stats |
| “Room is covered” | “Region improves with every pass” |

Both share: **known camera stats**, one world frame, world-model leaves. Differ on **fusion policy** and tolerance of inconsistency.

→ [splat-freeform-quilt.md](./splat-freeform-quilt.md)

---

## 10. Phases

| Phase | Plane | Work |
|-------|-------|------|
| **M0** | **WEFT-711** | ✅ Spec `cameras.json` + multi-folder parse in `clawft-splat-pipeline` (splatd job API still single-session; M1 wires multi-folder ingest) |
| **M1** | *(later)* | 2-cam known-pose prototype (room corner) |
| **M2** | 3–4 cam room; locked extrinsics; single train |
| **M3** | Live frustums + coverage map of FOV union |
| **M4** | Structure leaves from multi-view |
| **M5** | Optional: feed into free-form quilt accumulator |

---

## 11. History

- 2026-07-31: M0 — `cameras.json` schema + multi-folder parse (WEFT-711) in `clawft-splat-pipeline::multi_cam`.
- 2026-07-30: Split from multi-cam discussion into standalone infrastructure capture mode.
