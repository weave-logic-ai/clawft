# ADR-093: BVH × HNSW Phase F dual-index join

**Date**: 2026-07-31  
**Status**: Accepted  
**Deciders**: ECC / spatial maintainers  
**Source**: ADR-056 (deferred join), ADR-088 (`VectorRef`), WEFT-723, WEFT-709

## Context

ADR-056 deliberately separated **spatial containment** (BVH) from **feature
similarity** (HNSW). ADR-088 reserved an optional `VectorRef` on every spatial
leaf payload so WEFT-709 producers would not ossify pure-geometry leaves.

WEFT-709 now mints object/surface/volume records (export path). Consumers need
a **composition path** that answers:

> “Near here, what is most similar in feature space?”

without inlining embeddings into the BVH or teaching the BVH about cosine
distance.

Geometric `query_knn` already exists (AABB-center proximity). It is **not**
feature k-NN.

## Decision

### 1. Ownership (unchanged)

| Index | Answers |
|-------|---------|
| BVH | where / when / shape (point, AABB, sphere, ray, frustum, spatial kNN) |
| HNSW / `VectorBackend` | feature similarity |

### 2. Join shapes (both supported)

**Spatial-first (default product path)**  
1. Run a spatial query → `LeafId[]`  
2. Decode optional `VectorRef` from each leaf payload CBOR  
3. Optionally look up / rank by feature distance for those vector ids  

**Feature-first**  
1. Run HNSW kNN  
2. Filter hits whose metadata/key maps to a leaf id in a spatial candidate set  

### 3. Implementation home

Helpers live in `clawft-bvh::vector_join` (no kernel cycle):

- `vector_ref_from_payload`
- `join_spatial_with_vector_refs`
- `filter_feature_hits_by_spatial`
- `rank_spatial_hits_by_feature_distance`

Kernel / weave services compose these with live `SpatialBackend` +
`VectorBackend` instances. Embedder choice (which `index_id`) remains a
producer concern (`index_ids::ECC_HNSW` / `VISUAL_FEATURES` / …).

### 4. Explicit non-goals

- No inline embeddings in leaves (ADR-088 Option 3 still rejected)
- No second similarity index inside the BVH tree
- No mandatory dual-index on every query (pure geometry remains valid)

## Consequences

### Positive

- Join is testable with fixture payloads (no live HNSW required)
- ADR-056 / ADR-088 separation preserved
- WEFT-709 `vector: null` leaves stay valid forever

### Residual

- Live mesh/daemon “similar near me” product API still needs a service façade
  wiring both backends (follow-up if not already covered by SpatialService)
- Temporal fingerprinting (concept paper §10) remains optional future work

## Related

- ADR-056, ADR-088
- `docs/design/bvh_schema_updates.md`
- WEFT-709 (producers), WEFT-723 (this decision + helpers)
