# rUv WorldGraph / OccWorld vs WeftOS world model + Graph Views

**Status:** Living research crosswalk (rUv Brain–grounded)  
**Date:** 2026-07-31  
**Companions:** ADR-078, ADR-090 (LeWM decoupling), ADR-095,  
`docs/research/graph-views.md`, `docs/research/metaharness-foundation.md`,  
`.planning/research/cosmos3-vs-weftos-worldmodel.md` (NVIDIA Cosmos — separate)

---

## 1. Thesis

rUv’s “world model” is a **stack**, not a single neural net:

| Layer | rUv artifact | WeftOS counterpart |
|-------|--------------|-------------------|
| **Current-state twin** | WorldGraph (ADR-139) — typed petgraph, provenance, privacy | BVH + chain (ADR-056/078) + **Graph Views** for fusion |
| **Metric / multi-modal constraints** | UWB/mmWave fusion → graph edges + Kalman (ADR-144, ADR-063) | ToF/pose/co-observe attach to fusion Views; VectorRef join |
| **Predictive / latent future** | OccWorld occupancy predictor (ADR-147); Cosmos deferred | **LeWM** (`weftos-worldmodel*`, ADR-090) |
| **Domain twins** | SkyGraph (tracks/weather/anomalies) | Urth LOD + region Views (ADR-079) |
| **Harness that evolves with churn** | MetaHarness flywheel / Darwin | **ADR-096** (foundation) — this doc §6 |

**Operational alignment:** rUv fuses sensors **into a maintained graph twin with
provenance**. WeftOS now names the same job **Graph Views** (F1–F10). That is
not accidental convergence — adopt the pattern deliberately; do not re-invent
a free-floating fusion daemon.

---

## 2. WorldGraph (ADR-139) — what to borrow

### 2.1 Properties (from rUv sources)

- Rust crate `wifi-densepose-worldgraph`: typed **serde enums** for nodes/edges
  (not boxed trait objects) — schema-versioned, RVF-friendly persistence.
- Provenance + privacy rollup on the graph container.
- Queries over environmental twin (occupancy / person tracks / object anchors).
- Independent of heavy modality crates via opaque content-address handles.

### 2.2 Map to Graph Views

| WorldGraph idea | Graph View / WeftOS |
|-----------------|---------------------|
| One twin per scene/room | One **purpose-scoped View** (`room-12-identity`) |
| Typed node/edge enums | View edge table + leaf tags (`weftos-leaf-types`) |
| Provenance handles | `source_system`, `chain_seq`, confidence on View edges |
| Privacy rollup | Substrate ACL (ADR-057) on View scope |
| Upsert range edge from UWB | Live **attach** of constraint edges (F3/F6) |
| petgraph at room scale | Hot materialization (V1); batch plane when cliffs |

### 2.3 Do **not** copy blindly

- RuView’s primary sensors are **RF / CSI / UWB**, not RGB-splat. Geometry SoT
  for WeftOS remains **BVH**, not a CSI occupancy grid.
- WorldGraph stays in-RAM petgraph; WeftOS must keep **ADR-095 batch spill** for
  multi-site / Urth association graphs.
- rUv privacy (BFLD) is RF-home specific; map principles, not packet magics.

---

## 3. OccWorld (ADR-147) vs LeWM (ADR-090)

| | OccWorld (rUv) | LeWM (WeftOS) |
|--|----------------|---------------|
| Job | Predict **future occupancy** voxels from history | Latent predictive substrate for sensors / mesh |
| Host | Python subprocess + thin Rust client | `weftos-worldmodel*` + `clawft-worldmodel-service` |
| Authority | Injects trajectory **priors** into tracker; privacy gate | **ECC remains authority** (R1–R5); WM = impulse/observation only |
| Cosmos | Deferred (VRAM) | Separate Cosmos research; same caution on Mac vs Jetson |

**Compose:** OccWorld-class **future free-space / occupancy** can become
features or soft edges on a fusion View; LeWM remains optional per ADR-090.
Neither replaces Graph View identity fusion or BVH promote.

---

## 4. SkyGraph / domain twins

SkyGraph treats the sky as a **continuously changing spatial graph** (tracks,
observations, weather, anomalies) + RuVector embeddings. Parallel for WeftOS:

- Urth region Views + BVH densify islands.
- Domain Views can subscribe to mesh peers the way SkyGraph stitches tracks.

Use as a **template for domain-specific Views**, not a dependency.

---

## 5. NVIDIA Cosmos notes (already local)

`.planning/research/cosmos3-*.md` evaluate **Cosmos3-Edge** as a WFM candidate
(Jetson/RTX path). rUv independently deferred Cosmos for VRAM and chose
OccWorld. WeftOS stance:

- Cosmos = optional **edge predictor / data generator**, not the twin.
- Twin + fusion = Graph Views + BVH + chain.
- Keep Cosmos notes; promote to `docs/research/` when scrubbed of session-only
  paths if desired.

---

## 6. MetaHarness as the evolve layer over world-model churn

WorldGraph + fusion Views **churn**: new sensors, new co-observe edges, new
structure extracts, failed promotes, better ANN soft-edges. Without a
**governed evolve loop**, policies calcify or change without proof.

rUv MetaHarness slogan (packages):

> **Freeze the model. Evolve the harness. Promote only what proves lift.**

Applied here:

| Churn | What mutates (harness surface) | What stays frozen |
|-------|--------------------------------|-------------------|
| New modality / better ToF | ViewSpec attach policies, window/caps | Foundation models (optional) |
| Identity errors | Soft-edge thresholds, WCC promotion gates | Leaf type registry stability |
| Agent context packs | contextBuilder / memoryPolicy for fusion queries | ECC R1–R5 |
| Cost of multi-sensor agents | cost router receipts | Kernel safety |
| Structure extract quality | scorePolicy / reviewer for promote F9 | Chain audit requirements |

Flywheel (`@metaharness/flywheel`, MCP `metaharness_flywheel`): evaluate
candidates → **immutable receipts** → explicit **promote** (signed,
confirm=true). Darwin mutates **one policy surface per generation** (planner,
contextBuilder, reviewer, retry, tool, memory, score).

Full foundation plan: `docs/research/metaharness-foundation.md` + **ADR-096**.

---

## 7. Recommended adoption order

1. **Vocabulary** — treat WorldGraph-class twins as Graph Views (done in ADR-095/078).  
2. **Schema** — typed View edge provenance inspired by WorldGraph enums.  
3. **MetaHarness foundation** — score/genome/flywheel in-repo; Grok host path (ADR-096).  
4. **First fusion View** — room/region identity with live attach + promote to BVH.  
5. **LeWM/OccWorld-class priors** — optional features on Views under ADR-090.  
6. **Batch plane** — only when a named View cliffs (ADR-095).

---

## 8. Document history

| Date | Change |
|------|--------|
| 2026-07-31 | Initial rUv Brain crosswalk; link MetaHarness evolve layer |
