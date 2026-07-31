# BVH leaf schema — the embedding question

**Status**: **RESOLVED — superseded by ADR-088** (`docs/adr/adr-088-bvh-leaf-vector-ref.md`).
Implementation landed in `crates/weftos-leaf-types/src/spatial/vector_ref.rs` + every payload
in `primitives.rs`. This note is retained as the reasoning trail behind that decision.
**Date**: 2026-07-31 · **Updated**: 2026-07-31 (same day — decision taken and built)
**Scope**: what a BVH leaf payload should carry, and specifically whether leaves reference an
embedding — decided *before* WEFT-709 starts minting leaves at scale.

> ## ⚠ CORRECTIONS — read before relying on anything below
>
> This note was written fast and contained two factual errors. Both are fixed inline, but if
> you read an earlier copy:
>
> 1. **`query_knn` EXISTS.** An earlier revision of §2.1 and §5 said the tree had no k-NN
>    query. It does — `crates/clawft-bvh/src/query.rs:44`, re-exported at `lib.rs:26`, used in
>    `store.rs:15`. The error came from a `grep … | head -5` that truncated before reaching it.
>    This materially affects Option 4 below, which is more viable than originally stated.
> 2. **BVH Phases A–E are genuinely implemented.** An earlier analysis claimed
>    WEFT-716–720 were falsely marked Done. They are not: `ChainSink` is at
>    `clawft-bvh/src/chain.rs:50`; `SpatialBackend`/`SpatialService` are at
>    `clawft-kernel/src/spatial_backend.rs`, `spatial_bvh.rs`, `spatial_service.rs`;
>    `clawft-kernel` (via the `ecc` feature), `clawft-weave` and `clawft-bench-voice` all
>    depend on `clawft-bvh`. That claim rested on checking crate roots rather than walking
>    module trees, and should be disregarded.
>
> **The load-bearing facts for the decision were re-verified and hold**: `world_model.json`
> still emits `objects:0, surfaces:0, volumes:0, bvh_published:false` (asserted in tests), and
> **WEFT-709 is still Todo** — so the timing argument stands.

---

## 1. The one-paragraph version

BVH Phase A (`clawft-bvh`) and Phase B (`weftos-leaf-types` spatial registry, WEFT-717) are
both **done**: there is a real tag registry and a real set of typed payload schemas. What is
**not** decided is whether a spatial leaf carries any link to a feature vector. Today it
carries none, and the only embedding subsystem in the codebase (text → HNSW, for ECC) is
structurally separate from anything spatial. ADR-056 deliberately deferred joining them. That
deferral was correct at the time; the point of this note is that **WEFT-709 is the moment it
stops being free to defer**, because that is when leaves start being produced in volume.

---

## 2. What actually exists today (verified in code, 2026-07-31)

### 2.1 The tree — `crates/clawft-bvh`

`Leaf` is deliberately generic (`src/leaf.rs:20-30`):

```rust
pub struct Leaf {
    pub bound: Aabb,                 // broad-phase bound
    pub identity_kind: IdentityKind, // Object (stable id) | Event (immutable one-shot)
    pub tag: u32,                    // registry tag
    pub payload: Vec<u8>,            // opaque (CBOR/JSON bytes)
}
```

The opaque `Vec<u8>` is **not** an oversight — it is the intended layering. The tree stays
schema-agnostic; meaning lives in the registry. `clawft-bvh` does depend on
`weftos-leaf-types` (`crates/clawft-bvh/Cargo.toml:17`), so the two halves are joined.

Query surface (`src/query.rs`): `query_point`, `query_aabb`, `query_sphere`, `query_ray`,
**and `query_knn`** (line 44, re-exported at `lib.rs:26`). An earlier revision of this note
wrongly said k-NN was absent — see the corrections box at the top.

### 2.2 The registry — `crates/weftos-leaf-types/src/spatial/` (WEFT-717, Done)

Substantially built, ~795 lines:

- `tags.rs` (324 lines) — `SpatialLeafTag` enum with stable `u32` mappings: `Sphere`, `Aabb`,
  `Obb`, `Capsule`, `SweptAabb`, `Frustum`, `RadialSphereEvent`, and more.
- `primitives.rs` (471 lines) — typed payload structs: `SpherePayload`, `AabbPayload`,
  `ObbPayload`, `CapsulePayload`, `SweptAabbPayload`, `FrustumPayload`,
  `RadialSphereEventPayload`, `BeamTracePayload`, `SensorRead4DPayload`, `SplatScenePayload`.

**As originally written (before ADR-088): none of them carried an embedding, vector id, or
feature handle.** That was the gap this note identified. **It has since been closed** — see
§6.1 for what shipped.

> **Correction to an earlier analysis.** A first-pass review of this area reported that
> `weftos-leaf-types` "currently has zero spatial content". That is **wrong** — the registry
> and typed schemas exist and are Done (WEFT-717). The real gap is narrower and more specific:
> *typed payloads exist; none of them reference an embedding.* This note supersedes that
> claim. The distinction matters, because "write a payload schema" is already answered and
> "should a payload point at a vector" is not.

### 2.3 The producer — `crates/clawft-splat-pipeline/src/world_model.rs`

W0 emits an export-only stub (`world_model.rs:63-72`):

```json
{ "objects": 0, "surfaces": 0, "volumes": 0, "bvh_published": false,
  "scene": { "tag": "SPLAT_SCENE", "tag_u32": 0x53500001, ... } }
```

One whole-scene AABB, nothing decomposed, nothing published into a live tree. Those zeros are
asserted in the crate's own tests (`world_model.rs:292-294`). **WEFT-709** — the geometric
partition into surfaces and object AABBs — is **Todo**. That ticket is the hinge this whole
note turns on.

### 2.4 The embedding subsystem — separate, and deliberately so

What *is* embedded today is **text**, not space:

- `crates/clawft-kernel/src/embedding_e5.rs`, `embedding_qwen3.rs`, `embedding_onnx.rs`
- indexed by `vector_hnsw.rs`, `hnsw_service.rs`, `hnsw_eml.rs`

This serves ECC causal-graph and agent-context similarity. It has no spatial input and no
BVH linkage.

ADR-056 is explicit that this separation is a design position, not an accident
(`docs/adr/adr-056-*.md:45-47`):

> **Not a replacement for HNSW.** HNSW answers feature similarity; [BVH answers spatial
> containment] … ADR-011 "raw HNSW sufficient" is preserved.

and it defers the join ("Temporal Similarity Search via HNSW Fingerprinting") to a future ADR.

---

## 3. How we got here

| when | what | consequence |
|---|---|---|
| ADR-011 | "raw HNSW sufficient" for feature similarity | text embedding path established, no spatial component |
| ADR-056 | BVH as spatial index; explicitly **not** a replacement for HNSW; composition deferred | two indices, two jobs, join postponed *on purpose* |
| WEFT-716 (Done) | BVH Phase A — tree, leaves, queries | opaque payload by design |
| WEFT-717 (Done) | BVH Phase B — `weftos-leaf-types` spatial tag registry + typed payloads | meaning now lives in the registry; still no embeddings |
| ADR-077 (Accepted) | Android splat capture as an edge node | a real producer of spatial data |
| ADR-078 (Accepted) | splat feeds a **structured** world model, not appearance-only | objects/surfaces/volumes are the target, not a SOG |
| WEFT-708 (Done) | W0 world-model export | one scene AABB, `bvh_published:false` |
| **WEFT-709 (Todo)** | **W1 geometric partition → real object/surface leaves** | **the point at which payload shape ossifies** |

Nothing above is wrong. The sequencing is sound: build the tree, then the registry, then the
producer. The observation is only about *timing* — the cheapest moment to add an optional
field is before the field has millions of instances.

---

## 4. Why this is being raised now

Three inputs converged.

**(a) WEFT-709 is the ossification point.** Today, changing a payload struct costs a
recompile. After W1 lands and starts minting object/surface leaves from every splat capture,
it costs a migration over persisted data.

**(b) External comparison says our structure is the differentiator — protect it.** A
three-lane analysis of NVIDIA's Cosmos 3 against this stack
(`.planning/research/cosmos3-vs-weftos-worldmodel.md`) found that Cosmos keeps world state
**entirely implicit** in a diffusion latent — an exhaustive read of its 139-page technical
report found *no* object, identity, or entity mechanism anywhere. Meanwhile the object-centric
literature (Slot Attention → SAVi → DINOSAUR) has documented slot-swap and identity collapse
under occlusion. **ADR-078's geometric identity, matched against real COLMAP camera geometry,
is more reliable than anything in that literature.** That is a genuine advantage — and it is
exactly the thing a leaf schema either preserves or quietly discards.

**(c) The embedding literature says one vector cannot do both jobs.** DINOv2/v3 (self-
supervised, no language) is measurably stronger on geometry and 3D correspondence;
CLIP/SigLIP is stronger on language-groundable semantics but is documented as near-chance on
spatial relations (BLINK; "Spatial Blindspot of VLMs", arXiv 2601.09954). Neither encodes
affordance or dynamics at all. So "add an embedding" is under-specified — *which* embedding,
serving *which* query, is the actual question.

---

## 5. The open decision

**Should a spatial leaf reference a feature vector, and if so, how?**

Options, with the trade-off stated honestly:

1. **Nothing (status quo).** Keep payloads purely geometric. Cheapest. Cost: when the BVH↔HNSW
   join is designed later, it must either sit entirely outside the leaf (a side table keyed by
   leaf id) or force a payload migration.
2. **Optional vector handle.** Add an `Option<VectorRef>` (index id + vector id, or a u64
   handle into the existing HNSW) to the payload structs — *unused at first*. Near-free now;
   removes the migration risk; commits to nothing about which embedder or which index.
3. **Inline vector.** Store the embedding in the leaf. Rejected on sight: bloats every leaf,
   duplicates what an index already does well, and hard-codes a dimensionality.
4. **Side table only.** Keep leaves clean, join externally by leaf id. Viable, and arguably
   cleanest — geometric `query_knn` already exists for proximity, but feature similarity still
   needs an external leaf↔vector mapping (or the `VectorRef` handle from Option 2).

**Not in scope for this decision**: choosing an embedding model, building the BVH↔HNSW join,
or implementing WEFT-709's partition. The point is only to avoid foreclosing options.

### Related capability — already present

The tree **does** have `query_knn` (`src/query.rs:44`). Note this is *spatial* k-NN — k
nearest leaves to a point in space — not feature similarity. The BVH owning spatial proximity
while HNSW owns feature similarity is precisely the split ADR-056 argues for, and the two
compose: BVH narrows by geometry, HNSW ranks by meaning.

---

## 6. Outcome — Option 2, shipped

### 6.1 What was built (ADR-088)

`crates/weftos-leaf-types/src/spatial/vector_ref.rs`:

```rust
pub struct VectorRef {
    #[serde(default)]
    pub index_id: u32,   // which index/backend namespace
    pub vector_id: u64,  // opaque id within it; the BVH never interprets this
}

pub mod index_ids {
    pub const ECC_HNSW: u32 = 0;          // default kernel HNSW service
    pub const VISUAL_FEATURES: u32 = 1;   // reserved: DINO-style geometric features
    pub const LANGUAGE_FEATURES: u32 = 2; // reserved: CLIP/SigLIP-style semantic features
}
```

Threaded through **every** payload struct in `primitives.rs` as:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub vector: Option<VectorRef>,
```

### 6.2 Why this is the right shape

- **Backward compatible by construction.** `#[serde(default, skip_serializing_if)]` means
  every existing CBOR/JSON payload stays valid and absent handles cost zero bytes. The
  migration risk that motivated the note is gone.
- **A handle, not an embedding.** Dimensionality and embedder choice stay out of the leaf
  schema entirely — the BVH never interprets `vector_id`.
- **The reserved `index_ids` encode the two-channel finding.** `VISUAL_FEATURES` and
  `LANGUAGE_FEATURES` are separate namespaces because one vector cannot serve both jobs:
  DINOv2/v3 is stronger on geometry and 3D correspondence, CLIP/SigLIP on language-groundable
  semantics, and CLIP-family models are documented as near-chance on spatial relations
  (BLINK; arXiv 2601.09954). Reserving both now costs nothing and avoids a later renumber.
- **ADR-056's split is preserved.** BVH owns spatial containment and spatial k-NN; the vector
  index owns feature similarity. They compose via the handle rather than one absorbing the other.

Options 1, 3 and 4 were not taken. Option 3 (inline vectors) remains a bad idea for the reasons
in §5. Option 4 (side table) is still available *on top of* this — a handle does not preclude an
external mapping.

---

## 7. Where to dig deeper

**In-repo**
- `crates/clawft-bvh/src/leaf.rs` — the `Leaf` struct, `IdentityKind`, tag field
- `crates/clawft-bvh/src/query.rs` — query surface including geometric `query_knn`
- `crates/weftos-leaf-types/src/spatial/tags.rs` — `SpatialLeafTag` registry (WEFT-717)
- `crates/weftos-leaf-types/src/spatial/primitives.rs` — typed payloads + optional `vector`
- `crates/weftos-leaf-types/src/spatial/vector_ref.rs` — `VectorRef` / `index_ids` (ADR-088)
- `docs/adr/adr-088-bvh-leaf-vector-ref.md` — accepted Option 2
- `crates/clawft-splat-pipeline/src/world_model.rs` — the W0 stub and its tests
- `crates/clawft-kernel/src/{embedding_e5,embedding_qwen3,vector_hnsw,hnsw_service}.rs` — the
  existing, separate embedding path
- `docs/adr/adr-056-*` — BVH; the "not a replacement for HNSW" position and the deferral
- `docs/adr/adr-077-android-splat-capture-edge-node.md`, `adr-078-splat-feeds-world-model.md`
- `.planning/bvh-spatial-index/PLAN.md` — WEFT-716/717/718 phase breakdown
- `docs/plans/plane-board-inventory.md` — WEFT-709 / 717 status

**Analysis**
- `.planning/research/cosmos3-vs-weftos-worldmodel.md` — Cosmos 3 vs this stack; what to
  borrow (action encoding), what not to (the tokenizer — a category error for retrieval, and
  Cosmos 3 does not even use NVIDIA's own tokenizer, it uses a frozen Wan2.2 VAE)
- `.planning/research/cosmos3-edge-eval.md` — per-machine runnability across the fleet
  (M5 Max / RTX 4070 / incoming Jetson / rented H100)

**External**
- Cosmos 3 technical report — arXiv 2606.02800
- "Spatial Blindspot of VLMs" — arXiv 2601.09954 (CLIP-family near-chance on spatial relations)
- BLINK benchmark — VLM spatial-relation evaluation
- Slot Attention / SAVi / DINOSAUR — object-centric learning; note the identity-persistence
  failures under occlusion that make our geometric identity the stronger option
- V-JEPA 2 — "predict representations, not pixels"; the closest published match to a latent
  world-model track, and runnable on the M5 Max today

---

## 8. Provenance

Every in-repo claim here was read from the code on 2026-07-31, not inferred from ADR text —
including the correction in §2.2, which reverses an earlier reviewer's statement about
`weftos-leaf-types` being empty. External claims are cited to named papers and to the Cosmos
technical report.

**Post-decision**: ADR-088 Accepted; optional `VectorRef` + payload `vector` fields shipped in
`weftos-leaf-types`. This note is the reasoning trail; the binding decision is ADR-088
(Plane WEFT-721 / WEFT-722; Phase F join deferred as WEFT-723).
