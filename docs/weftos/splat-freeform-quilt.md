# Free-form splat quilt (accumulating regional models)

**Status:** Design (2026-07-30)  
**Mode:** Time-open reconstruction — *seams and differences are acceptable*; the asset is a **quilt that improves with every contribution**

**Related:**

| Doc | Role |
|-----|------|
| [splat-multi-camera-rig.md](./splat-multi-camera-rig.md) | Fixed multi-cam, seamless single-train preference |
| [splat-to-world-model.md](./splat-to-world-model.md) | Objects / volumes / BVH |
| [urth-digital-twin.md](./urth-digital-twin.md) | **Urth** planet-scale sparse twin; quilt is L4 densify |
| [splat-multimodal-sensing.md](./splat-multimodal-sensing.md) | Overlap & modalities |
| [ADR-078](../adr/adr-078-splat-feeds-world-model.md) | Dual appearance + structure |
| [ADR-079](../adr/adr-079-urth-digital-twin.md) | Urth digital twin |
| [ADR-056](../adr/adr-056-bvh-spatial-index.md) | Spatial index; Object vs Event |

---

## 1. Product intent

**Free-form splatting** means we do **not** require one perfect, seamless reconstruction every session.

Instead:

> A **region of space** holds a **quilt**: the living sum (or best fusion) of many splats / observations of that area over time.  
> As long as each contribution has **known camera stats** (pose, intrinsics, time, device), we can keep **compiling more images of the same objects** and improve coverage, detail, and world-model confidence.

| Cares about | Does *not* require |
|-------------|---------------------|
| Same world frame | Pixel-perfect seam matching |
| Known camera metadata | Single capture session |
| Improving coverage over time | One global Lighting/exposure match |
| Object identity growth | Zero double-geometry artifacts |

**Contrast with fixed multi-cam rig mode:** there we prefer one clean train. Here we prefer **never-finished, always-better** regional assets.

```
t0: phone walk  ──► patch SOG_A ──┐
t1: 4 fixed cams ──► patch SOG_B ──┼──► quilt region R (world frame)
t2: Pi head pass ──► patch SOG_C ──┘         │
                                             ▼
                                    world-model Objects
                                    gain evidence / tighter AABBs
```

---

## 2. Core concepts

### 2.1 Region

A **region** is a named AABB (or volume) in the world model:

- `region/warehouse-aisle-3`  
- `region/home-living-room`  
- `region/site-latlon-…` (outdoor)

All contributions must register into the **same coordinate frame** (or provide a transform into it).

### 2.2 Contribution (Event)

One capture session or multi-cam burst:

| Field | Required |
|-------|----------|
| `contribution_id` | Yes |
| `region_id` | Yes |
| `t_range` | Yes |
| `cameras[]` | Yes — see §3 |
| `images` / video refs | Yes |
| `train_job_id` / `sog_uri` | When appearance trained |
| `quality` metrics | Optional (n_views, coverage, reproj) |

Stored as **Event** leaves or chain events (ADR-056 identity).

### 2.3 Quilt (Object)

The **quilt** is the living appearance + structure state of a region:

| Part | Role |
|------|------|
| **Appearance layers** | List of patch SOGs (or one fused SOG) with priority / recency |
| **Structure** | `WM_OBJECT` / `WM_SURFACE` / `WM_VOLUME` with multi-evidence |
| **Index** | Which contributions touch which sub-volumes |

Quilt is an **Object** (or set of Objects) that **revises** as contributions arrive—not a single immutable SOG file.

### 2.4 “Sum of all splats”

Operational meanings (pick explicitly in implementation):

| Strategy | Behavior | Seams |
|----------|----------|--------|
| **Layer stack** | Render/query composites multiple patch SOGs by AABB | Visible seams OK |
| **Priority / recency** | Newer or higher-quality patch wins in overlap | Soft transitions later |
| **Periodic retrain** | All images with known poses → full retrain of region | Best long-term quality |
| **Local retrain** | Only sub-volume images → patch SOG replace | Needs good pose lock |

**v1 product:** **layer stack + periodic full retrain** when enough new images accumulate.  
**v1.5:** priority by coverage/quality.  
**v2:** continuous local fusion.

“Sum” is **not** naively averaging Gaussian parameters without a shared frame—that produces garbage. Sum means **accumulate observations under known cameras**, then **fuse by policy**.

---

## 3. Known camera stats (contract)

Every image (or video frame used for train) must carry enough metadata to place it in the quilt:

```json
{
  "image_id": "img-…",
  "contribution_id": "c-…",
  "t_ns": 123,
  "camera": {
    "device_id": "pi-head-3",
    "model": "OPENCV",
    "width": 1600,
    "height": 900,
    "params": [fx, fy, cx, cy, …],
    "T_world_cam": { "R": […], "t": […] },
    "pose_source": "locked_rig|vio|colmap|manual|aruco",
    "pose_confidence": 0.0
  },
  "path": "frames/00042.jpg"
}
```

| Stat | Why |
|------|-----|
| Intrinsics | Projection / train |
| Extrinsics in **region frame** | Multi-session alignment |
| Time | Dynamic scenes, lineage |
| Device / model | Batching, bias, diagnostics |
| Pose source + confidence | Fusion weight; reject junk |

**Without known (or well-estimated) pose in a shared frame, the image cannot join the quilt**—it can only start a **new** free SfM island (discouraged for free-form mode).

Pose may come from:

- Fixed multi-cam lock ([multi-camera rig](./splat-multi-camera-rig.md))  
- Phone VIO / ARCore  
- COLMAP relative to region anchors (markers, prior model)  
- Sensor-head IMU + prior map  

---

## 4. Free-form capture loop

```
1. Enter region (map / QR / geo / prior scene id)
2. Capture with live camera stats streaming
3. Upload contribution (images + cameras.jsonl)
4. Host: validate poses in region frame
5. Optional: train patch SOG for this contribution
6. Quilt: register layer + update structure evidence
7. Optional: queue region retrain if delta large enough
8. World model: tighten Object AABBs / spawn new unknowns
```

**Seams:** If contribution lighting or color differs, **keep both layers**; agents use structure AABBs; humans tolerate visual quilt until retrain.

---

## 5. Overlap rules (relaxed vs single-session)

Free-form **relaxes** single-session perfection:

| Rule | Free-form stance |
|------|------------------|
| Full closed loop every time | Not required |
| Perfect exposure match | Not required |
| Dual COLMAP within one contribution | Still avoid (bad pose) |
| Overlap with **prior quilt** | **Desired** — “glue” to existing model |
| Off-axis / partial object | **OK** — later pass fills other side |

**Still required within a contribution:** enough multi-view for that train to not explode (or skip train and only add structure evidence if poses are excellent and you only update WM).

**Cross-contribution:** prefer that new cameras **see something already in the quilt** (or shared markers) so pose stays in the same frame without drift.

---

## 6. World model under free-form

Free-form is **especially** aligned with ADR-078:

| Mechanism | Effect over time |
|-----------|------------------|
| More views of same object | Higher confidence, tighter AABB, better class |
| Partial views | Object stays `incomplete` until faces filled |
| Conflicting geometry | Branch or lower-confidence Event; don’t silent overwrite |
| Rescan after move | New Events; Object revision or “moved” lineage |

Structure can improve **even when appearance quilt still shows seams**.

```
contribution_1: sees table top only     → WM_OBJECT table (incomplete)
contribution_2: sees table legs         → same instance_id, fuller AABB
contribution_3: full retrain            → prettier SOG + stable object
```

---

## 7. Storage model (sketch)

```
regions/<region_id>/
  frame.json                 # origin, gravity, scale
  quilt.json                 # layer list, fusion policy
  contributions/<cid>/
    cameras.jsonl
    frames/…
    metrics.json
    artifacts/patch.sog      # optional
  appearance/
    layers.json              # ordered SOG refs + AABBs
    fused.sog                # optional periodic bake
  structure/
    world_model.json         # export
    # live: BVH leaves
```

### quilt.json (sketch)

```json
{
  "region_id": "home-living",
  "frame": "gravity_aligned_v1",
  "fusion": {
    "mode": "layer_stack",
    "retrain_every_n_contributions": 5,
    "min_pose_confidence": 0.5
  },
  "layers": [
    { "contribution_id": "c1", "sog": "…", "aabb": {…}, "priority": 1 },
    { "contribution_id": "c2", "sog": "…", "aabb": {…}, "priority": 2 }
  ]
}
```

---

## 8. Host pipeline extensions

| Capability | Role |
|------------|------|
| Region registry | Create/list regions, frame definition |
| Contribution ingest | images + camera stats validation |
| Patch train | Optional per-contribution SOG |
| Quilt update | Register layer; trigger retrain job |
| Region retrain | All accepted images + poses → one SOG |
| Structure merge | Multi-evidence Objects (instance match) |
| Reject path | Low pose_confidence or bad frame |

splatd (or a sibling **quilt service**) owns region state; WeftOS BVH publishes structure.

---

## 9. Edge / capture implications

| Source | How it joins the quilt |
|--------|------------------------|
| Phone walk | VIO pose in region (markers or prior map) |
| Fixed 3–4 cams | Locked extrinsics; each burst = contribution |
| Pi sensor head | Same; ToF improves structure |
| 360 cam | Dense views; still need region pose of the 360 body |

**UI:** “You improved coverage of *table* by 20%” > “SOG looks seamless.”

---

## 10. Risks and mitigations

| Risk | Mitigation |
|------|------------|
| Frame drift over months | Periodic retrain; markers; re-anchor |
| Pose junk pollutes quilt | Confidence gate; human/agent reject |
| Disk growth | Drop old layers after fused bake; keep image archive cold |
| Lighting seams | Accept in layer mode; retrain for demos |
| Duplicate objects | Instance matching + human confirm |
| Scale inconsistency | Yardstick / stereo / shared depth anchors |

---

## 11. Free-form vs multi-cam rig (summary)

| | Multi-cam rig | Free-form quilt |
|--|---------------|-----------------|
| Doc | [splat-multi-camera-rig.md](./splat-multi-camera-rig.md) | **This doc** |
| Goal | Clean, repeatable room capture | Continuous improvement of a region |
| Seams | Minimize | Tolerate |
| Time | Session-closed | Open-ended |
| Cameras | Often fixed install | Any known-stats source |
| Train | Prefer single full train | Layer + periodic full / local |
| World model | Snapshot structure | **Growing multi-evidence Objects** |

They compose: a fixed 4-cam burst is just a **high-quality contribution** into a free-form region.

---

## 12. Implementation phases

| Phase | Plane | Work | Exit |
|-------|-------|------|------|
| **Q0** | **WEFT-710** | This doc; region + contribution schema | Spec frozen |
| **Q1** | *(later)* | Ingest contribution with camera stats; store without fusion | Multi-session archive |
| **Q2** | Layer-stack quilt render / export (AABB-culled SOGs) | Visible “sum” of patches |
| **Q3** | Structure multi-evidence merge for Objects | World model improves over time |
| **Q4** | Periodic region retrain job | Baked fused SOG |
| **Q5** | Live edge: stream stats + images into region | Continuous free-form capture |
| **Q6** | Local retrain / quality-weighted layers | Less full retrain cost |

**Train backends** for patch vs bake: [splat-train-backends.md](./splat-train-backends.md)
(Brush default; Instant-NuRec optional for pose-rich previews; registry in `~/llm`).

---

## 13. Success criteria

1. Two contributions, different days, join one region with known poses.  
2. Quilt shows **more coverage** than either alone (layer or retrain).  
3. At least one `WM_OBJECT` gains evidence across contributions.  
4. Bad-pose contribution can be rejected without destroying region.  
5. Seams may exist; agents still query structure successfully.

---

## 14. History

- 2026-07-30: Free-form accumulating quilt split out from multi-cam discussion; pairs with fixed-rig doc.
