# Urth — sparse-first multi-scale digital twin

**Status:** Vision + architecture (2026-07-30)  
**Name:** **Urth** — WeftOS planetary twin (not “Earth” / map-brand product naming)  
**North star:** A navigable **digital twin** of the planet that starts empty and densifies where we observe — Snow Crash–scale ambition, engineering-honest sparsity.  
**ADR:** [ADR-079](../adr/adr-079-urth-digital-twin.md)  
**Expert:** `.grok/agents/world-builder.md` · `.claude/agents/world-builder.md`

Geodesy under the hood remains **WGS84 / ECEF** (the real planet). **Urth** is the software world graph.

---

## 1. The Snow Crash idea (what we take / leave)

| Take | Leave |
|------|--------|
| Continuous **shared world** people and agents inhabit | Exact Metaverse protocol / franchise |
| **Detail where attention is** | Claim of complete photoreal planet day one |
| Multiple ways in (street, room, avatar-scale) | Single proprietary client only |
| Economy of **contribution** (capture, map, curate) | Speculative crypto / spam land grabs without governance |

**Engineering translation:** multi-resolution spatial graph + free-form capture quilts + open basemaps + BVH queries.

---

## 2. Sparse-first reality

At launch Urth will have:

| Layer | Coverage |
|-------|----------|
| Open basemap | Global, **low** geometric fidelity |
| Local quilts | **Tiny** high-detail blobs (your home, one campus) |
| Objects | Few instances with multi-evidence |
| Most of Urth | `unobserved` — honest unknown |

That is a **feature**: the twin grows like a living map, not a one-shot scan.

```
        ┌─────────────────────────────────────┐
        │  L0–L2 open basemap (thin)          │
        │  ████████████████████████████████   │
        └───────────────┬─────────────────────┘
                        │ contain
        ┌───────────────▼─────────────────────┐
        │  L3 site (OSM buildings + anchors)  │
        └───────────────┬─────────────────────┘
                        │ densify
        ┌───────────────▼─────────────────────┐
        │  L4 quilt (splats, multi-cam, phone)│  ← most engineering now
        │  L5 objects / free space            │
        └─────────────────────────────────────┘
```

---

## 3. Level-of-detail (LOD) stack

| LOD | Name | Geometry | Appearance | Writers |
|-----|------|----------|------------|---------|
| L0 | Planetary | Ellipsoid, DEM coarse | Globe imagery (licensed) | System |
| L1 | Admin | Borders, major roads | Optional | System + open data |
| L2 | City | Street graph, building footprints | Map tiles | System + open data |
| L3 | Site | Campus/parcel, anchors | Optional orthophoto | Ops + capture |
| L4 | Zone/room | Free-form quilt AABB | SOG layers | Capture edges |
| L5 | Object | Instance AABB / surface / volume | Optional subset GS | Structure + human |

Root id: **`urth`** / `region/urth/…`.  
**Query rule:** same BVH API; results filtered by LOD and confidence.

---

## 4. Open-source / open-data feeds (bootstrap)

| Class | Examples | Twin role | Caution |
|-------|----------|-----------|---------|
| **Streets / POI** | OpenStreetMap (Overpass, Geofabrik extracts) | L1–L3 vectors → WM_OBJECT stubs | Attribution; freshness |
| **Buildings** | OSM + public footprint datasets | Building AABBs | Height often missing |
| **Terrain** | SRTM, Copernicus GLO-30, regional LiDAR open | L0–L2 surfaces | Resolution |
| **Imagery** | Sentinel-2, NAIP (US), other licensed tiles | Basemap texture | **License before cache** |
| **Places** | Wikidata / OSM POIs | Named objects | Sparse indoors |
| **Transport** | GTFS (where open) | Routes as graphs | Not 3D |
| **Weather** | Open-Meteo etc. | Event layers | Ephemeral |
| **Address** | OpenAddresses (regional) | Anchors | Coverage gaps |

**Indoor** open data is weak → capture + multi-cam dominate L4–L5.

Ingest path: **batch ETL** into region leaves. Store provenance + license on every leaf.

---

## 5. Local observation path (already designed)

| System | Role in Urth |
|--------|----------------|
| Free-form quilt | Densify L4 appearance + multi-evidence objects |
| Multi-cam rig | High-quality site contributions |
| Phone / Pi head | Mobile densification |
| Train backends | Brush / optional Instant-NuRec for patches |
| Structure stage | WM_* from geometry + semantics |
| BVH + chain | Queryable, auditable truth |

A capture does not need a full-planet Gaussian — it **attaches** to `region/urth/…` under WGS84 with a local ENU frame.

---

## 6. Coordinate systems

```
WGS84 / ECEF  ←→  site ENU (east-north-up)  ←→  room ENU
     region.geo_anchor              T_site_room
```

| Capture has GPS? | Behavior |
|------------------|----------|
| Yes | Auto-suggest region; store uncertainty |
| No | User picks site; optional survey of one anchor point |
| Fixed cams | Site calibration once (markers / survey) |

---

## 7. Fusion & honesty rules

| Rule | |
|------|--|
| **Metric beats generative** | Open DEM/OSM and capture structure > AI-filled buildings for “truth” |
| **Confidence always stored** | OSM building conf 0.4; multi-view chair 0.9 |
| **Unknown is explicit** | Unobserved space is not solid dirt |
| **Seams OK at L4** | Quilt layers; bake when ready |
| **LOD streaming** | Clients pull coarse first, dense on demand |
| **Governance** | Who can write which region (capability tokens) |

---

## 8. Client experience (north star)

1. **Globe (Urth)** — basemap, glowing sites with dense data.  
2. **Fly to site** — OSM buildings + sparse objects.  
3. **Enter room** — free-form SOG quilt + AABB overlays.  
4. **Agent** — `query_sphere(agent, r)` across LODs.  
5. **Capture** — “improve this place” → contribution → denser Urth.

Snow Crash energy without lying about empty space.

---

## 9. Multi-group collaboration

| Group | Owns |
|-------|------|
| **World-builder expert** | LOD policy, open data, fusion ethics, densification roadmap |
| **Reconstruction curator** (`~/llm`) | Train backends, model registry |
| **Capture / edge** | Phone, Pi head, multi-cam |
| **ECC / BVH** | Spatial index, chain |
| **Mesh / product** | Multi-node write, clients |

World-builder **coordinates**; does not replace reconstruction or capture engineers.

---

## 10. Phased delivery

| Phase | Goal | Exit |
|-------|------|------|
| **E0** | ADR-079 + this doc + agent | Doctrine frozen (**WEFT-713**) |
| **E1** | Region IDs: urth → site → room + ECEF↔ENU helpers | One pilot site geo-anchored |
| **E2** | OSM (+ optional DEM) ingest for pilot city/site | Coarse WM leaves without capture |
| **E3** | Quilt contributions attach under site | Local densification on globe |
| **E4** | Client: basemap + “data glow” + enter dense zone | Demo Snow Crash-lite |
| **E5** | Multi-writer governance + sharding | Multi-node Urth |

Pilot: **one real site you capture** + OSM neighborhood — not whole-planet tiles day one.

---

## 11. Risks

| Risk | Mitigation |
|------|------------|
| Scope explosion | Pilot site first; L0–L2 thin |
| License violation | Provenance on every basemap leaf |
| Fake density | No generative-as-truth |
| Single BVH OOM | Shard by geohash / region |
| Pose drift vs basemap | Markers, re-anchor jobs |
| Brand confusion | Product name **Urth**, not third-party map brands |

---

## 12. Success criteria (early)

1. Globe UI labeled **Urth** shows open basemap + one dense quilt site.  
2. Agent lists objects in dense site without reading SOG.  
3. New capture **improves** that site’s quilt and object evidence.  
4. Unobserved areas remain **unknown**, not hallucinated city.  
5. World-builder agent used for LOD/source decisions in PRs.

---

## 13. History

- 2026-07-30: Vision from world-model + free-form quilt; Snow Crash north star; sparse-first doctrine.  
- 2026-07-30: Product renamed **Urth** (distinct from map-brand “Earth” naming).  
