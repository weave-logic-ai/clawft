# ADR-078: Splat pipeline feeds a structured world model (not appearance-only)

**Date**: 2026-07-30  
**Status**: Accepted  
**Deciders**: product + ECC / spatial (owner: reconstruction is input to BVH entities—objects, surfaces, volumes—not only a SOG for viewing)  
**Depends-On**: ADR-056 (BVH), splat pipeline design, multi-modal capture design  
**Relates-To**: ADR-077 (capture edge), ADR-073 (workspace visualizes; BVH is truth), LeWM latent world-model track (compose later), **ADR-095** (batch graph analytics + Graph Views — multi-sensor identity / association; DiskANN for vector relief), **`docs/research/graph-views.md`** (Graph Views = **operational model for sensor fusion**)

## Context

The phone → COLMAP → Brush → SOG path produces a **dense appearance field**. Operators and agents also need a **world model**:

- object **boundaries** and instance identity  
- **volumes** (occupied / free / no-go)  
- surfaces (floor, walls)  
- spatial queries (“what is near X?”)  
- chain-backed evidence  

ADR-056 already defines BVH leaves as `(AABB, Object|Event, tag, payload)`. Phase A of `clawft-bvh` reserves splat tags (`SPLAT_SCENE`, …) but the pipeline does not yet **publish structure**.

Without this decision, we risk shipping “pretty rooms” that cannot ground agents.

## Decision

### 1. Dual output is normative

Every successful reconstruction job **should** produce:

| Output | Required for |
|--------|----------------|
| Appearance (`splat.sog` / ply) | Viewing, demos, visual context |
| Structure (`world_model` export + optional BVH publish) | Agents, mesh, governance |

Appearance may ship first (latency); structure may complete asynchronously, but the **product definition** of “done” for world-model use cases includes structure.

### 2. Structure is first-class pipeline stage(s)

After SfM (and typically after or parallel to train):

1. Metric / gravity alignment when possible  
2. Geometric partition (planes, clusters, free space)  
3. Optional semantic instance labeling  
4. Emit `WM_*` / splat leaf records  
5. Publish into BVH + chain when ECC/daemon available  

Detailed schema and phases: `docs/weftos/splat-to-world-model.md`.

### 3. Object vs Event semantics for capture

| Leaf kind | Examples |
|-----------|----------|
| **Object** | Room, furniture instance, wall, free-space region, sensor fixture |
| **Event** | Train job, camera frame observation, ToF sample, human confirmation |

Objects keep stable IDs across rescans when matching succeeds; Events are immutable evidence.

### 4. Multi-modal capture optimizes for structure, not only beauty

Capture (phone, Pi head, 360) is designed so evidence supports:

- view graph quality (RGB overlap)  
- scale and occupancy (depth / ToF / stereo)  
- recapture of **incomplete objects**, not only empty view bins  

See multi-modal sensing + sensor-head docs.

### 4b. Two appearance fusion modes (both feed the world model)

| Mode | Doc | Stance |
|------|-----|--------|
| **Fixed multi-cam / single train** | `splat-multi-camera-rig.md` | Prefer seamless one-shot reconstruction |
| **Free-form quilt** | `splat-freeform-quilt.md` | Accept seams; accumulate contributions over time with **known camera stats** |

Structure (objects/volumes) **gains multi-evidence** in both modes; free-form makes continuous improvement the default product.

### 5. Viewer and Agent Workspace are projections

- Spark / harness viewer: appearance ± AABB overlays  
- Agent Workspace: may open SOG and object lists  
- **Source of truth for “what is where”:** BVH + chain, not the SOG file alone  

### 5b. Sensor fusion is operationally Graph Views (research stance)

Multi-modal / multi-device **association and identity** (same object across
cameras, sessions, modalities) is **not** specified as a separate unbounded
fusion daemon. Per ADR-095 §1b and `docs/research/graph-views.md`:

1. A **purpose-scoped Graph View** is created (room, region, job, array).  
2. Capture, structure extract, co-observe, tracks, and ANN candidates **attach**
   as sources (live where appropriate, with caps).  
3. Fusion logic runs **in that View** (hot incremental + optional batch WCC /
   rank when the edge table cliffs).  
4. **Stable results promote** into BVH object leaves + chain evidence (this ADR’s
   structure path remains the durable world-model SoT).

Appearance fusion modes (§4b multi-cam / free-form quilt) remain valid **input
pipelines**; they feed Views and structure stages rather than replacing them.

### 6. Latent world models compose later

LeWM-class latent dynamics may consume the same sensors and leaves. They do **not** replace explicit AABB/object identity for addressable agent queries in v1 of this path.

## Non-goals

- Perfect open-world instance segmentation in v1  
- BVH as physics engine or window manager  
- Requiring LiDAR-equipped phones  
- Blocking SOG export until full semantics finish  

## Consequences

### Positive

- Capture and train investment feeds agents, not only demos  
- Clear tag/payload contract for implementers  
- Aligns splat work with ADR-056 and edge sensors already in substrate  

### Negative

- Extra pipeline complexity and failure modes (structure stage)  
- Semantics need ML and/or human confirmation for high trust  
- Dual “done” states (appearance ready vs world model ready)  

## Implementation

Follow phases **W0–W6** in `docs/weftos/splat-to-world-model.md`.

| Phase | Plane |
|-------|-------|
| W0 | **WEFT-708** |
| W1 | **WEFT-709** |

Minimal W0: publish `SPLAT_SCENE` (+ optional camera Events) when a job completes.

## References

- `docs/weftos/splat-to-world-model.md`  
- `docs/weftos/splat-multimodal-sensing.md`  
- `docs/weftos/splat-capture-sensor-head.md`  
- `docs/weftos/splat-multi-camera-rig.md`  
- `docs/weftos/splat-freeform-quilt.md`  
- `crates/clawft-bvh/src/leaf.rs` tags  
- ADR-056, ADR-077  
