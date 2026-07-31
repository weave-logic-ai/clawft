# Graph Views — named, multi-source, optionally live graphs

**Status:** Living research / product-architecture note  
**Date:** 2026-07-31  
**Companions:** ADR-095 (hot vs batch planes), ADR-046 (forest of trees),  
ADR-058 (`SessionView`), ADR-067 (conversation graph **UI** view), ADR-069  
(atom primary index / panopticon), ADR-078 (splat → world model),  
ADR-087 (spatio-temporal sensor dual-branch), ADR-096 (MetaHarness foundation),  
`docs/research/batch-graph-analytics-disk-spill.md`,  
`docs/research/diskann-and-large-scale-indexes.md`,  
`docs/research/ruv-worldgraph-vs-weftos.md`,  
`docs/research/metaharness-foundation.md`,  
`docs/weftos/splat-to-world-model.md`, `docs/weftos/splat-multimodal-sensing.md`

---

## 0. Operational thesis (sensor fusion)

> **Sensor fusion, operationally, is the construction and maintenance of Graph Views.**

Not a separate mega-pipeline that magically merges every modality into one
global tensor. Operationally:

1. **Create a View** for a purpose (this room, this array, this rescan job,
   this Urth region).
2. **Attach sources** (RGB/depth streams, BVH leaves, co-observe edges,
   pose tracks, ANN soft-edges, peer Views, imports).
3. **Feed live** where sensors run; **window and cap** so the hot set stays
   bounded.
4. **Fuse inside the View** — geometry join (BVH), feature join
   (`VectorRef` / HNSW / DiskANN), structural identity (edges + optional
   batch WCC), learned dual-branch scores (ADR-087) as node/edge features.
5. **Publish results** outward: promote stable object leaves into the world
   model (ADR-078 BVH + chain), write features back, export for agents/UI.

Appearance fusion (multi-cam train, free-form quilt) remains a **source
pipeline** into Views and SOG artifacts. **Identity / structure / multi-modal
association** — the part agents and governance need — lives in Graph Views.

```text
 sensors · cameras · ToF · IMU · mesh peers · imports
              │ live attach (capped)
              ▼
     ┌────────────────────┐
     │  Fusion Graph View │  purpose: room-12-identity / array-7 / …
     │  (hot + optional   │
     │   batch analytics) │
     └─────────┬──────────┘
               │ promote / write-back
       ┌───────┴────────┐
       ▼                ▼
  BVH + chain      agents / GUI
  (world model)    (query the View)
```

---

## 1. Motivation

We already have many **source graphs** (forest members, BVH, graphify KG,
sensor association edges, ExoChain-derived links). Product work needs
something different:

> A **View** — a graph that is **created for a purpose**, may be **fed live**,
> and may **pull or project** graph data from other sources (and other Views).

Examples of purpose:

| View intent | Likely sources | Live? |
|-------------|----------------|-------|
| **Sensor fusion (primary)** — room / region identity | BVH leaves, co-observe, tracks, ANN, multi-cam | Yes (sensor) |
| This conversation’s causal walk | CausalGraph + CrossRefs + SessionView | Yes (tick) |
| Code assessment of monorepo X | graphify extract | Mostly batch rebuild |
| Agent task: “what depends on auth?” | graphify + ADR-084 pipelines | On demand |
| Mesh peer health influence | SWIM / mesh events | Yes (gossip) |
| Urth region LOD association | multi-site structure + base maps | Mixed |

Without a View concept, every consumer either (a) hits the global forest raw
and drowns in irrelevant edges, or (b) reimplements ad-hoc subgraph filters —
and sensor fusion becomes an unscoped “merge everything” job that cannot be
capped, audited, or batch-analyzed.

**Naming hygiene:** ADR-067’s “conversation graph view” is a **UI surface**.
This note uses **Graph View** (capital V) for the **data/product object**.
UI may *render* a Graph View; they are not the same layer.

---

## 2. Definition (working)

A **Graph View** is a first-class graph object with:

1. **Identity** — stable `view_id` (and optional human name / purpose string).
2. **Spec** — declarative description of *how* the graph is produced
   (sources, filters, joins, window, live vs snapshot).
3. **Materialization mode** — virtual / incremental / full snapshot
   (see §4).
4. **Vertex + edge tables** (logical) — same shape as GraphFrames /
   ADR-095 batch plane: rows with ids, types, payloads, optional features
   (`component_id`, `rank`, `VectorRef`, …).
5. **Provenance** — which source systems and which batch jobs contributed
   which edges (chain / witness friendly where required).
6. **Caps** — `max_nodes`, `max_edges`, complexity budget, retention,
   privacy / ACL scope (ADR-057).

A View is **not** a new forest tree by default (ADR-046 still owns durable
domain structures). A View is usually a **projection / composition /
subscription** over forest + external feeds. Some Views may later promote
into durable trees if they become load-bearing domains.

```text
┌──────── source graphs (forest + externals) ─────────┐
│ Causal · HNSW · ExoChain · Resource · BVH · graphify │
│ sensors · mesh · foreign Graph Views · imports      │
└─────────────────────┬───────────────────────────────┘
                      │ ViewSpec (filters, joins, feeds)
                      ▼
              ┌───────────────┐
              │  Graph View   │  ← purpose-built, named
              │  (hot and/or  │
              │   batch tier) │
              └───────┬───────┘
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
   Agent context   GUI render    Batch analytics
   (k-hop pack)    (ADR-067…)    (WCC/PageRank → features)
```

---

## 3. Sources (multi-graph ingest)

A ViewSpec should allow **multiple source bindings**, each typed:

| Source kind | Examples | Ingest pattern |
|-------------|----------|----------------|
| **Forest structure** | CausalGraph, CrossRef, ResourceTree | Snapshot RPC / substrate path / impulse subscription |
| **Spatial** | BVH region / tag filter | Spatial query → leaf vertices + adjacency of co-location |
| **Vector** | HNSW / DiskANN namespace | ANN candidate edges (similarity as soft edges) |
| **Code KG** | graphify `graph.json` / live rebuild | File or stream of entity/rel rows |
| **Sensor stream** | splat events, tracks, co-observe | Live append to edge table |
| **Peer View** | another `view_id` (mesh or local) | Import subgraph or subscribe to delta |
| **Foreign import** | Parquet/CSV edge list, partner API | Batch load + provenance tag |
| **Derived** | output of batch WCC/PageRank on this or parent View | Feature columns write-back |

**Edge provenance fields (recommended):**
`source_system`, `source_view_id?`, `observed_at`, `confidence`,
`chain_seq?`, `edge_kind`.

CrossRefs (ADR-046) remain the forest’s native graft mechanism; Views may
**materialize** CrossRef-like edges into their own edge table for query
locality without mutating the source forest.

---

## 4. Live feeds vs snapshots

| Mode | Semantics | Use when |
|------|-----------|----------|
| **Snapshot** | Build once (or on demand); immutable until rebuild | Forensic export, graphify assessment, audit package |
| **Live head** | Append/update as impulses / sensor events / turns arrive | Conversation View, room identity View, mesh health |
| **Windowed live** | Live but only last *T* / last *N* events | High-rate sensors; bound RAM |
| **Hybrid** | Snapshot base + live delta log | Urth region: base LOD + local densify |

Live Views must declare:

- **Admission policy** (which event types append vertices/edges).
- **Compaction** (when to fold deltas, drop stale speculative nodes —
  same lifecycle ideas as ADR-062 Speculative→Committed→Pruned).
- **Backpressure** (drop, sample, or spill to batch tier when rate exceeds
  hot budget — ADR-095 / ADR-047).

---

## 4b. Sensor fusion loop (canonical operational sequence)

This is the intended **runtime story** for multi-modal / multi-device fusion
(ADR-078 structure path, multi-cam / free-form quilt, sensor heads, arrays).

| Step | Action | Notes |
|------|--------|--------|
| **F1 Create** | `view.create` purpose = fusion scope (room, region, array, job) | ACL + retention set here |
| **F2 Bind geometry** | Attach BVH region / tag filter as hard spatial source | “Where” substrate (ADR-056) |
| **F3 Bind sensors** | Attach live streams: frames, ToF, IMU-derived pose, co-observe, tracks | Windowed; rate limits |
| **F4 Bind appearance / codes** | Soft edges from visual (or other) embeddings via HNSW/DiskANN | Candidates only until confirmed |
| **F5 Bind structure extract** | WM partition outputs, instance proposals, free-space volumes | From splat/structure pipeline |
| **F6 Hot fuse** | Incremental edge admission: co-location, track continuity, human confirm | Tick-friendly; caps |
| **F7 Batch fuse (when needed)** | Export View edges → WCC / PageRank / large CC under memory cap | ADR-095; identity components |
| **F8 Dual-branch score (optional)** | Spatial vs temporal features + α fusion (ADR-087) | Features on View nodes/edges |
| **F9 Promote** | Stable components → BVH object leaves + chain evidence events | World model SoT (ADR-078) |
| **F10 Serve** | Agents/UI query the View (k-hop, spatial-first join) or read promoted leaves | Do not dump raw multi-sensor graph into LLM |

**What is *not* sensor fusion operationally:** a single global always-on graph of
every sensor on the mesh with no purpose, no caps, and no View identity.

**Appearance-only paths** (one SOG train, quilt layer list) still matter for
humans and backdrops; they **feed** fusion Views (and BVH structure stages)
rather than replace them.

---

## 5. Materialization tiers (ties to ADR-095 + DiskANN)

| Tier | Storage | Query path | Scale |
|------|---------|------------|-------|
| **V0 Virtual** | Spec only; query pushes down into sources | Hot plane k-hop / BVH / HNSW per source | Small, few sources |
| **V1 Incremental hot** | In-memory petgraph / DashMap edge table | Local walk, UI, agent pack | 10³–10⁶ edges |
| **V2 Columnar snapshot** | Parquet/Arrow vertex+edge | Batch WCC/PageRank; offline export | Large; spill-friendly |
| **V3 Hybrid vector attach** | View nodes hold `VectorRef`; cold ANN via DiskANN | Feature-first / spatial-first join (ADR-093) | Huge embedding sets |

**Rule of thumb:**

- Start V0/V1 for product Views.
- Promote a View to V2 when global algorithms or RAM cliffs hit
  (ADR-095 activation).
- Use DiskANN on the **embedding columns** of a View’s vertices, not as a
  substitute for the View’s association edges.

Batch jobs should target **a View’s edge table** (or a named export of it),
not “the entire forest,” so analytics stay purpose-scoped.

---

## 6. Relationship to existing WeftOS “views”

| Existing term | Layer | Relation to Graph View |
|---------------|-------|------------------------|
| `SessionView` (ADR-058) | Context / memory tier frontier | Specialized **session** Graph View (or input to one) |
| Conversation graph view (ADR-067) | **GUI** | Renders a conversation Graph View; needs RPC export of nodes/edges |
| `ui://graph` / GraphViewer | UI primitive | Generic renderer of `{nodes, edges}` — adapter for any Graph View |
| Atom / panopticon locator (ADR-069) | Index projection | Locators can point into View-local ids vs forest uids |
| graphify rebuild output | Source / snapshot View | Natural “code assessment View” for a path |
| BVH region query result | Ephemeral spatial View | Can be bound as a source for “what’s in this volume” View |

---

## 7. Product / API sketch (non-normative)

Illustrative only — not a ship commitment:

```text
view.create  --name "room-12-identity" --spec room_identity_v1.yaml
view.attach  --view room-12 --source bvh:region/room-12
view.attach  --view room-12 --source sensor:co_observe --live
view.attach  --view room-12 --source hnsw:visual --as soft_edges
view.export  --view room-12 --format parquet   # batch plane input
view.query   --view room-12 --mode k-hop --from leaf:… --depth 2
view.features --view room-12 --job wcc         # batch write-back
```

Spec YAML/TOML might include:

```yaml
view_id: room-12-identity
purpose: multi-cam object identity for room 12
materialization: incremental_hot
sources:
  - kind: bvh
    filter: { region_id: room-12, tags: [WM_OBJECT, WM_SURFACE] }
  - kind: sensor_stream
    topic: co_observe
    live: true
    window: { max_edges: 500000 }
  - kind: vector
    index_id: VISUAL_FEATURES
    soft_edge: { k: 8, min_score: 0.82 }
caps:
  max_nodes: 200000
  max_edges: 2000000
  acl: substrate_path_or_principal
```

---

## 8. Governance, ACL, mesh

- Views inherit **substrate ACL** scope (ADR-057); a View must not leak
  edges the principal cannot read from any source.
- Mesh: Views may be **local-only**, **exportable snapshots**, or
  **subscribed peer Views** (eventual consistency; same caveats as
  forest CrossRefs).
- Chain: creating/attaching sources / running batch feature jobs that
  change identity should emit ExoChain events (ADR-022) when product
  requires audit.

---

## 9. Open questions

1. Are Graph Views first-class kernel objects (service + persistence) or
   graphify/weave-layer only until proven?
2. Virtual (pushdown) vs always-materialized default for multi-source Views?
3. Soft edges from ANN: store as first-class edges or recompute on query?
4. Can a View be a source of another View (DAG of Views)? Cycles forbidden?
5. Promotion path: when does a View become a new `StructureTag` tree?
6. How do live Views interact with speculative conversation nodes
   (ADR-062 lifecycle)?
7. UI: one GraphViewer bound to `view_id` vs per-domain surfaces?

---

## 9b. Evolving Views under churn (MetaHarness)

Fusion Views **churn** as sensors, structure extract, and soft-edges improve.
Policy for attach windows, ANN thresholds, and promote gates should evolve with
**MetaHarness flywheel discipline** (ADR-096): evaluate candidates → immutable
receipts → explicit promote — not silent edits only. Freeze foundation models
and ECC authority; mutate the **harness / ViewSpec**. See
`docs/research/metaharness-foundation.md` §4.

## 10. What to do now vs later

| Now (research hold) | Later (when product pulls) |
|---------------------|----------------------------|
| Treat **Graph Views as the operational model for sensor fusion** in planning | `view.*` RPCs / weave commands for F1–F10 |
| Keep this doc + ADR-095 §1b | First shipping fusion View: room/region identity |
| MetaHarness foundation (ADR-096) for policy flywheel | ViewSpec anchors + flywheel promote |
| Use purpose-scoped exports for experiments | Live attach for multi-cam / sensor-head feeds |
| Conversation UI continues ADR-067 path | Unify under Graph View id for data feed |
| DiskANN cold tier for embeddings | Batch WCC/PageRank **per fusion View** edge table |
| Structure extract still → BVH (ADR-078) | Promote step F9 wires View components → leaves |

---

## 11. Document history

| Date | Change |
|------|--------|
| 2026-07-31 | Initial note — Graph Views as purpose-built multi-source live/snapshot graphs; ties to ADR-095 planes |
| 2026-07-31 | **Sensor fusion operational thesis** + F1–F10 loop; primary product use of Views |
| 2026-07-31 | §9b MetaHarness flywheel under View churn; rUv WorldGraph + ADR-096 links |
