# ADR-088: Optional VectorRef on BVH spatial leaf payloads

**Date**: 2026-07-31  
**Status**: Accepted  
**Deciders**: ECC / spatial maintainers; splat / world-model (ADR-077/078)  
**Source**: `docs/design/bvh_schema_updates.md` (2026-07-31); ADR-056; WEFT-717; WEFT-709

## Context

BVH Phase A (`clawft-bvh`) and Phase B (`weftos-leaf-types` spatial registry,
WEFT-717) are landed: leaves are `(Aabb, identity_kind, tag, payload)` with
typed CBOR payloads and **no** link to a feature vector. Text embeddings and
HNSW remain a separate ECC path (ADR-011, ADR-056 “not a replacement for HNSW”).

ADR-056 deferred joining BVH and HNSW (“Temporal Similarity Search via HNSW
Fingerprinting”) to a future ADR. That deferral remains correct for *similarity
ownership*. What is no longer free is **payload shape**: WEFT-709 (ADR-078 W1
geometric partition) will mint object/surface leaves at volume. Changing
payloads after that is a migration tax.

The design note evaluates four options (nothing / optional handle / inline
vector / side table only) and recommends **optional vector handle**.

`query_knn` on the BVH tree (geometric proximity by AABB center) exists for
broad-phase convenience; it is **not** feature k-NN. Feature similarity stays
on HNSW.

## Decision

### 1. Optional `VectorRef` on spatial payloads (Option 2)

Add a shared type:

```rust
pub struct VectorRef {
    pub index_id: u32,   // namespace; 0 = default ECC HNSW
    pub vector_id: u64,  // opaque id within that index
}
```

Every spatial payload in `weftos-leaf-types::spatial::primitives` carries:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub vector: Option<VectorRef>,
```

- **Default / absent** = pure geometry (backward compatible CBOR).
- **Some** = external join key; unused until a consumer fills it.
- **No inline embeddings** in leaves (rejected: bloat, fixed dim, duplicates index).

### 2. Index ownership unchanged

- BVH answers **where / when / shape** (containment, ray, frustum, spatial kNN).
- HNSW (or other `VectorBackend`) answers **feature similarity**.
- Join is a consumer concern (resolve `VectorRef` after a spatial query, or
  filter HNSW hits by leaf id). Phase F / “BVH × HNSW fingerprinting” remains
  a separate future ADR — this ADR only reserves the handle.

### 3. Timing relative to WEFT-709

WEFT-709 **must** treat `vector: None` as the W1 default. Producers may later
populate handles without a payload schema break.

## Consequences

### Positive

- Near-zero cost today; removes leaf-schema migration risk before W1 scale.
- Keeps ADR-056 separation of spatial vs feature indices.
- Does not force embedder choice (DINOv2 vs CLIP, etc.).

### Negative / residual

- Call sites constructing payloads must set `vector: None` (or `Default`).
- Side-table-only designs remain valid; this does not forbid them.
- Full join API, embedder selection, and dual-index query plans are **not**
  specified here (tracked as follow-up Plane work).

## Alternatives considered

| Option | Verdict |
|--------|---------|
| 1. Nothing (status quo) | Rejected for W1 timing — decision by default is still a decision; preferred to record Option 2 |
| 2. Optional `VectorRef` | **Accepted** |
| 3. Inline vector | Rejected |
| 4. Side table only | Viable long-term companion; not exclusive of Option 2 |

## Related

- ADR-056 (BVH spatial index) — amended by this ADR for payload handles
- ADR-011 (raw HNSW sufficient for feature similarity)
- ADR-077 / ADR-078 (splat capture / world model)
- Design note: `docs/design/bvh_schema_updates.md`
- Implementation: `weftos-leaf-types::spatial::vector_ref`, payload `vector` fields
- Plane: WEFT-721 (schema), WEFT-722 (decision), WEFT-723 (Phase F join deferred 0.9.x), WEFT-709 (W1 must default `vector: None`)
