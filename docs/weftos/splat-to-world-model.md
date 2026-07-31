# From capture / splat → structured world model

**Status:** Design (2026-07-30)  
**Product stance:** A train job is **not** “one pretty SOG of the room.” It is the start of **populating a spatial world model**: object boundaries, volumes, affordances, and links into ECC/BVH.

**Depends on:**

| Doc | Role |
|-----|------|
| [ADR-056](../adr/adr-056-bvh-spatial-index.md) | BVH spatial-temporal index; Object vs Event leaves; AABB broad-phase |
| [ADR-077](../adr/adr-077-android-splat-capture-edge-node.md) | Edge capture node |
| [splat-multimodal-sensing.md](./splat-multimodal-sensing.md) | Wavelengths + overlap for solid geometry |
| [splat-capture-sensor-head.md](./splat-capture-sensor-head.md) | Pi / ToF / IMU head |
| [splat-multi-camera-rig.md](./splat-multi-camera-rig.md) | Fixed multi-cam known poses |
| [splat-freeform-quilt.md](./splat-freeform-quilt.md) | Accumulating regional quilt over time |
| [urth-digital-twin.md](./urth-digital-twin.md) | **Urth** LOD; local structure densifies the twin |
| [splat-pipeline-design.md](./splat-pipeline-design.md) | splatd stages |
| `crates/clawft-bvh` | Phase A BVH + reserved splat tags |

**Also relates:** LeWM / latent world-model ADRs (sensor-primary latent model under ECC — greenfield); BVH is the **explicit geometric** half; latent codes are optional later.

---

## 1. Problem

Today’s pipeline:

```
capture → COLMAP → Brush → splat.ply / splat.sog → viewer
```

That yields a **single unstructured radiance field** (millions of Gaussians). Agents and governance need:

| Need | Example query |
|------|----------------|
| **What is where** | Objects in this AABB? |
| **Boundaries** | Where does “table” end and “floor” begin? |
| **Volumes** | Walkable free space? Occupied volume of “chair”? |
| **Identity over time** | Same object across rescan / branch? |
| **Evidence** | Which capture / chain seq produced this leaf? |

A monocular SOG cannot answer those without **structure on top**.

---

## 2. Two products, one capture

| Layer | Artifact | Consumer |
|-------|----------|----------|
| **Appearance** | `splat.sog` / PLY | Humans, Spark viewer, Agent Workspace backdrop |
| **World model** | BVH leaves + payloads + chain | Agents, mesh, governance, navigation, “needs human” attention |

```
                    multi-modal capture session
                              │
              ┌───────────────┴───────────────┐
              ▼                               ▼
     appearance train                    structure extract
     (Brush / 3DGS)                      (segment + geometry)
              │                               │
              ▼                               ▼
         splat.sog                    objects / free-space /
         (visual)                     surfaces / sensors as
                                      BVH Object leaves
                                              │
                                              ▼
                                    clawft-bvh + ExoChain
                                    (ADR-056 world index)
```

**Non-goal:** Treating the BVH as a window manager (ADR-073) or as a physics engine (ADR-056).  
**Goal:** **Geometric entities with bounds** that agents can query.

---

## 3. World-model entities (target schema)

### 3.1 Identity kinds (ADR-056)

| Kind | Use in splat pipeline |
|------|------------------------|
| **Object** | Persistent: room, wall, table, door, sensor head, “zone kitchen” — stable `LeafId` across rescans/branches |
| **Event** | One-shot: train job, camera pose sample, ToF frame, human “confirm this is a chair” |

### 3.2 Proposed leaf tags (extend `clawft_bvh::leaf::tags`)

Reserved / planned band `0x5350_xxxx` (“SP…”):

| Tag | Name | Bound | Payload (sketch) |
|-----|------|-------|------------------|
| `0x5350_0001` | **SPLAT_SCENE** | Whole reconstruction AABB | job_id, SOG URI, train metrics, chain_seq |
| `0x5350_0002` | **SPLAT_CAMERA** | Frustum or small AABB at pose | frame_id, quat, path, coverage_bin |
| `0x5350_0003` | **SPLAT_YARDSTICK** | Known length segment | metric scale factor |
| `0x5350_0010` | **WM_OBJECT** | Object AABB (or OBB in narrow-phase later) | class, confidence, instance_id, mesh/splat-subset ref |
| `0x5350_0011` | **WM_SURFACE** | Thin AABB / plane slab | normal, material hint, parent object |
| `0x5350_0012` | **WM_VOLUME** | Free / occupied / no-go volume | kind, traversability |
| `0x5350_0013` | **WM_SEGMENT** | Mask-backed region | mask URI, 2D/3D linkage |
| `0x5350_0014` | **WM_SENSOR_FOV** | Sensor observation volume | sensor_id, modality (tof/radar/rgb) |
| `0x5350_0015` | **WM_AFFORDANCE** | Interaction volume | openable, sittable, … (policy later) |

Formal registry still lands in `weftos-leaf-types` per ADR-056; this table is the **product contract** for implementers.

### 3.3 Object payload (minimum)

```json
{
  "instance_id": "obj-7f3a…",
  "class": "chair",
  "class_source": "openclip|sam|human|tof_cluster",
  "confidence": 0.82,
  "aabb": { "min": […], "max": […] },
  "parent_scene": "splat-job-4030…",
  "evidence": {
    "frame_ids": [12, 13, 40],
    "chain_seq": null,
    "mask_uri": "artifacts/masks/chair_7f3a.webp"
  },
  "appearance_ref": {
    "kind": "gaussian_subset|none",
    "uri": null
  }
}
```

Appearance can stay **one scene SOG** initially; subsets/clips are optimization later.

---

## 4. Pipeline: beyond “train → SOG”

Extend the mental model of splatd / post-train:

```
1  capture session     RGB (+ IMU, ToF, …)
2  sfm                 poses + sparse cloud
3  train               splat appearance
4  compress            SOG
5  structure           ← NEW product stage
   5a  ground / gravity align (IMU / yardstick)
   5b  geometric partition (planes, clusters, free space)
   5c  semantic segment (2D masks → project to 3D)
   5d  instance AABBs + WM_* leaves
6  publish             BVH insert + chain + substrate paths
7  package             manifest includes world-model summary
```

Stages 5–6 can run **async after SOG** so viewer still ships fast; world model catches up.

### 4.1 Geometric structure (no ML required first)

| Technique | Output |
|-----------|--------|
| RANSAC planes (floor/walls/ceiling) | `WM_SURFACE` + room shell volumes |
| Euclidean cluster on sparse/dense cloud | Candidate `WM_OBJECT` AABBs (class = `unknown`) |
| Height maps / occupancy grids | `WM_VOLUME` free vs occupied |
| ToF / stereo fusion | Better near-field occupied volume |
| Multi-model COLMAP reject | Avoid dual-world garbage objects |

This alone turns “one explode cloud” into **room shell + blobs with bounds**.

### 4.2 Semantic structure (ML / human)

| Technique | Output |
|-----------|--------|
| 2D open-vocab segment (SAM / YOLO-world / …) on keyframes | Masks |
| Project masks along rays using poses + depth/splat depth | 3D instance clusters |
| CLIP / detector class labels | `class` + confidence |
| Human confirm in Agent Workspace | High-trust Object leaves |
| Rescan / track instance_id | Object stability across sessions |

### 4.3 Multi-modal contribution to **objects**, not just pretty pixels

| Modality | World-model role |
|----------|------------------|
| **RGB multi-view** | Texture, semantics, primary geometry |
| **IMU / VIO** | Gravity-aligned world frame; camera Events |
| **ToF multi-zone** | Near occupancy volumes; hole “unknown” volumes |
| **Stereo / LiDAR** | Metric AABBs, free-space carving |
| **mmWave** | Occupied range gates; motion “someone entered” Events |
| **Thermal** | Heat sources as objects / affordances (equipment) |
| **Sonar** | Domain volumes (fluid, range) when applicable |

See [splat-multimodal-sensing.md](./splat-multimodal-sensing.md).

---

## 5. Capture changes when the goal is a world model

Capture is no longer “walk until SOG looks ok.” It is **evidence gathering for entities**.

| Capture principle | Why |
|-------------------|-----|
| **Coverage of surfaces that bound objects** | Undersides / backs matter for AABB quality |
| **Scale anchors** | Yardstick, known door height, stereo, LiDAR |
| **Revisit after structure** | Host sends “low evidence on object X” → recapture that bin |
| **Stable lighting / markers** | Better instance match across sessions |
| **Session = scene revision** | New train → new Event leaves; Objects update or branch |
| **Free-form multi-session** | Many contributions with known camera stats → quilt + multi-evidence Objects ([splat-freeform-quilt.md](./splat-freeform-quilt.md)) |
| **Fixed multi-cam** | Known poses, strong single-session graph ([splat-multi-camera-rig.md](./splat-multi-camera-rig.md)) |

Edge UI (phone / Pi head) should eventually show:

- Coverage bins (views)  
- **Object completeness** (missing faces of detected AABB)  
- **Unknown volume** (ToF/radar says occupied, no RGB instance yet)

---

## 6. Composition with BVH queries (agent API)

Once published, agents use spatial queries (ADR-056), not pixel scrapes:

```text
query_aabb(room_region) → [WM_OBJECT chair, WM_OBJECT table, WM_VOLUME free, …]
query_ray(from_cam, dir) → first Object / Surface
query_sphere(agent_pos, r) → nearby affordances
```

**WindowIntent / Agent Workspace** (ADR-073) may *visualize* these leaves; the **source of truth** is BVH + chain, not the SOG alone.

**Splat viewer** = appearance layer + optional overlay of AABB wireframes from world-model export.

---

## 7. Substrate / chain

| Path / event | Content |
|--------------|---------|
| `substrate/scene/<id>/appearance` | SOG URI, job metrics |
| `substrate/scene/<id>/objects` | List / stream of Object leaves |
| `substrate/sensor/tof` | Live depth (existing) |
| ExoChain | Every insert/update of WM_* leaves (ADR-056 §6) |

No silent in-memory-only world model for production paths.

---

## 8. Relationship to latent world models (LeWM)

| Explicit (this doc) | Latent (LeWM-class ADRs) |
|---------------------|---------------------------|
| AABB, class, instance_id | Learned codes / JEPA-style dynamics |
| Queryable by agents now | Training / prediction later |
| Fed by capture + structure stage | May consume same sensors + leaves as features |

**Compose, don’t choose:** BVH leaves are the **addressable world**; latent models can sit **on top** as predictors. Capture design should still produce **explicit geometry + identity**.

---

## 9. Implementation phases

| Phase | Plane | Work | Exit |
|-------|-------|------|------|
| **W0** | **WEFT-708** | This doc + tag table; wire SPLAT_SCENE leaf on job done (AABB from sparse cloud) | Scene is a BVH Object/Event, not only a file |
| **W1** | **WEFT-709** | Plane/cluster geometric partition → WM_SURFACE + unknown WM_OBJECT AABBs | Room shell + blobs without ML |
| **W2** | *(later)* | Free-space WM_VOLUME from cloud / depth | Navigation-relevant volumes |
| **W3** | *(later)* | 2D segment → project → labeled WM_OBJECT | Named furniture-class instances |
| **W4** | *(later)* | Capture feedback: missing object faces / unknown volume | Closed-loop world-model capture |
| **W5** | *(later)* | Instance track across rescans + chain lineage | Stable Object IDs over time |
| **W6** | *(later)* | Optional Gaussian subset per object / semantic splat channels | Appearance bound to objects |

W0–W2 need **no** new capture hardware—only structure stage + BVH publish. Multi-modal capture improves W1–W4 quality.

---

## 10. Manifest extension (sketch)

```json
{
  "artifacts": {
    "sog": "splat.sog",
    "ply": "splat.ply",
    "world_model": "world_model.json"
  },
  "world_model": {
    "frame": "scene_gravity_aligned",
    "scale_m": 1.0,
    "objects": 12,
    "surfaces": 6,
    "volumes": 3,
    "bvh_published": false
  }
}
```

`world_model.json` is a portable export; **live** truth is BVH when daemon is up.

---

## 11. Non-goals

- Full semantic NeRF/splat research SOTA in v1  
- Physics simulation inside BVH  
- Replacing HNSW similarity search  
- Auto-perfect instance IDs without human/policy for high-stakes governance  
- Assuming phone LiDAR (S25 has none)—use geometry + optional external depth  

---

## 12. Success criteria (product)

1. After a room capture, an agent can list **objects with AABBs** without looking at the SOG.  
2. Free-space volume exists for “can I walk here?”-class queries (even coarse).  
3. Rescan updates Objects with lineage, not only a second orphaned SOG.  
4. Appearance SOG remains viewable independently (progressive enhancement).  
5. All structure commits are chain-auditable when ECC is on.

---

## 13. History

- 2026-07-30: Drafted after multi-modal capture design; product goal expanded from single-object/scene splat to world-model population (boundaries, volumes, BVH).
