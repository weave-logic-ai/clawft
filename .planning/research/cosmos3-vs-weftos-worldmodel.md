# Cosmos 3 vs the WeftOS world model — what to borrow, what not to

> Research note. DESIGN/EVAL only — no code lands from this file.
> Requested 2026-07-31: "look very closely at the cosmos3 model, and what we are trying to
> model… can we take some of what they are doing and affect the embeddings we are making
> for the world model stuff?"
> Three-lane analysis: Cosmos internals (from the 139-page Cosmos 3 technical report,
> arXiv 2606.02800), the WeftOS stack as it exists **in code** (not as specced), and the
> embedding/representation literature. Sources cited in the three companion files in the
> session scratchpad.

## TL;DR — three answers, in order of how much they should change plans

1. **There are no world-model embeddings to improve yet.** Entities are not embedded
   anywhere in shipped WeftOS code. This is the single most useful finding, and it turns the
   question from *retrofit* into *greenfield*.
2. **The tokenizer is the wrong borrow — twice over.** It is a category error for retrieval,
   *and* Cosmos 3 doesn't use NVIDIA's tokenizer anyway.
3. **The action encoding is the right borrow**, and it is concrete, small, and specified
   well enough to copy without any NVIDIA weights.

---

## 1. What WeftOS actually has today (verified in code, not from ADRs)

ADR-056/077/078 describe an ambitious structured world model. The shipped reality is early:

| claim | reality in code |
|---|---|
| BVH is the world-model truth (ADR-056) | `clawft-bvh` exists, **Phase A only**. Effectively orphaned — nothing in the spatial path depends on it (only `clawft-bench-voice`). |
| Splat feeds objects/surfaces/volumes (ADR-078) | `world_model.json` emits **one whole-scene AABB**; `objects:0, surfaces:0, volumes:0, bvh_published:false` — and those zeros are asserted in tests. Real partition is WEFT-709, **Todo**. |
| Rich query surface | point / aabb / sphere / ray. **No `query_knn`.** |
| `SpatialBackend`, kernel wiring, ChainAnchor, COW branches | not in code — docstring/spec only. |
| Entities carry meaning | leaf payload is an opaque `Vec<u8>`; tag is a "free u32 for now". No typed schema, no embedding field. |
| LeWM as infrastructure | **not built.** One April symposium diagram proposing ADRs 048–058 and 7 crates, none of which exist. Those ADR numbers on master are real but *unrelated* decisions — a number collision ADR-087 itself notes. |

What *is* embedded today: **text**, via E5/Qwen3 ONNX into an in-house HNSW, serving ECC
causal-graph and agent-context search. That is a separate subsystem from anything spatial.
ADR-056 explicitly **defers** BVH↔HNSW composition to a future ADR — so this is an
acknowledged seam, not an oversight.

**Consequence:** the design decision "what does a BVH leaf payload contain, and is it
embedded?" is still free. That is a good position to be in, and it is the decision this
report should inform.

---

## 2. What Cosmos 3 actually is (from the technical report, not the marketing)

**Dual-tower.** One transformer stack carrying *two full parameter sets* per layer — an
autoregressive "Reasoner" track and a diffusion "Generator" track, both co-initialised from
the same pretrained VLM (Qwen3-VL). They meet at exactly one place: a shared attention op
where AR does causal self-attention over itself, and DM does full bidirectional attention
over concatenated `[AR;DM]` K/V. **DM is conditioned on AR; AR never sees DM.** Not a shared
latent, not cross-attention blocks — K/V concatenation inside one attention operator.

**One objective, three behaviours.** A single rectified flow-matching objective (masked MSE
on velocity) across image/video/audio/action, one denoiser. Forward dynamics, inverse
dynamics and policy are **not separate models** — they are three *inference-time masking
patterns* over which tokens are clean vs noisy. Forward = denoise vision given clean actions;
inverse = denoise actions given clean vision; policy = denoise both.

**No entity representation. At all.** An exhaustive grep for object permanence / identity /
entity found only synthetic-data curation rules and human-eval rubrics — never an
architectural mechanism. World state is **fully implicit in the diffusion latent**: no slots,
no scene graph, no persistent object IDs.

**The tokenizer is not NVIDIA's.** Cosmos 3 does *not* use the 2025-generation "Cosmos
Tokenizer" (CV/DV, up to 2048× compression, NVIDIA-licensed). It uses a **frozen third-party
Wan2.2-TI2V-5B causal video VAE** (Alibaba, Apache-2.0), 4× temporal / 32×32 spatial, causal
and chunked at inference. Audio has its own separate frozen VAE. "Adopt the Cosmos tokenizer"
therefore means "go get Wan2.2 from Alibaba".

**Licensing.** Everything NVIDIA authored — code, checkpoints, 5 synthetic datasets, the
Cosmos-HUE eval benchmark — is OpenMDW-1.1 (Linux Foundation). The tokenizer (Wan2.2) and
backbone (Qwen3-VL) are third-party under their own licences and are independently reusable.
*Unverified:* a "Built on NVIDIA Cosmos" attribution requirement appears in secondary
coverage but not in the technical report — check openmdw.ai/license/1-1/ if compliance matters.

---

## 3. The central insight: opposite bets

| | Cosmos 3 | WeftOS |
|---|---|---|
| world state | **implicit** in a diffusion latent | **explicit** entities in a BVH |
| identity | none — no persistent object IDs | stable Object leaf IDs, matched against real COLMAP geometry |
| primary act | **generate** plausible futures | **reconstruct** the present |
| queries | none, in the retrieval sense | point/aabb/sphere/ray |

These are not the same system at different maturity levels — they are **different bets**.
Cosmos buys generality and generation; it cannot tell you *which* object is which across
time. WeftOS buys queryable, persistent structure; it cannot predict anything.

This matters for a reason that is easy to miss: **the object-centric literature backs
WeftOS's bet.** Slot Attention → SAVi → DINOSAUR is the nearest academic analogue to
"structured world model with persistent entities", and it has documented slot-swap and
collapse failures under occlusion. ADR-078's geometric identity — matched against real camera
geometry — is *more reliable than anything in that literature*. That is a genuine
differentiator, not a gap to be closed by adopting someone else's latent.

---

## 4. What to borrow, ranked

### ✅ BORROW — the action encoding (the one genuinely Cosmos-specific idea worth taking)

Concrete and copyable without weights:
- **Continuous SE(3)-relative pose + grasp state**, not raw joint/PID control. Ego pose 9D
  (3D translation + 6D rotation) + effector pose(s) 9D each + grasp state (15D fingertip or
  1D open/close) — composing to ~57D for egocentric/humanoid.
- **Per-embodiment-domain linear adapters**: `z = W_in^(k)·x + b_in^(k)`, K domains with
  separate weight matrices over a shared backbone. Adding a new embodiment = a fresh adapter
  (they demonstrate 5× LR on it in their DROID recipe).
- **Absolute-time-aligned 3D RoPE**, with the action token sitting as the *edge* between
  video-state nodes `v_{t-1}` and `v_t`.

Why it fits WeftOS: the Object/Event leaf split already distinguishes persistent things from
one-shot happenings. "Action as the edge between states" maps onto that directly, and the
per-domain adapter pattern is exactly right for a fleet of heterogeneous sensors (sonobuoy /
ESP32 / Android splat capture) sharing one representation.

### ✅ BORROW — the masking-pattern trick (architectural, free)

One model + three masking patterns = forward dynamics, inverse dynamics, and policy. If
WeftOS ever grows a dynamics model, this is a cleaner factoring than three separate models,
and it costs nothing to adopt as a design principle now.

### ❌ DO NOT BORROW — the tokenizer, for embeddings

A category error, for a reason that survives the surprise about provenance. A video VAE is
**rate-distortion optimised** — trained for reconstruction fidelity, with no metric or
contrastive objective. Nearest-latent ≠ nearest-meaning. This is why VQGAN codebooks aren't
semantically organised and why Stable-Diffusion-style systems keep CLIP separate from the VAE
latent. Indexing Wan2.2 latents in an HNSW would retrieve "similar to a decoder", not
"similar in meaning". The legitimate use is as a cheap raw feature to train a metric head
*on top of* — not as a finished embedding.

### ❌ DO NOT BORROW — the implicit world state

It is the opposite of ADR-078's thesis, and the literature says WeftOS's version is the more
robust one for identity. Adopting it would trade a working differentiator for a weakness.

---

## 5. What to do about embeddings — the actual answer

Ranked by (value × feasibility). Machine noted, since the fleet now includes an RTX 4070
12 GB, an incoming Jetson, and rentable H100-class alongside the M5 Max.

1. **Decide what a leaf payload contains, before it ossifies.** The payload is an opaque
   `Vec<u8>` and the tag is a free `u32`. Adding an embedding *field* now — even unused —
   costs nothing and avoids a migration later. This is the cheapest high-value action
   available and needs no model at all.
2. **Two embedding channels, two jobs** (M5 Max, today). DINOv2/v3 is measurably stronger on
   geometry and 3D correspondence; CLIP/SigLIP is stronger on language-groundable semantics
   but is documented as "bag of words" on spatial relations (BLINK; "Spatial Blindspot of
   VLMs", 2601.09954 — near-chance on relational tasks). Do not ask one embedder to do both.
3. **Don't fix spatial reasoning in the embedder.** ADR-056's HNSW/BVH split ("they compose,
   they do not compete") is already the right answer, and the literature above is the
   evidence for it. This is validation, not new work.
4. **V-JEPA2 as the dynamics backbone, if LeWM is revived** (M5 Max via PyTorch+MPS today;
   an MLX port exists but its maturity is *unverified*). Its premise — *predict
   representations, not pixels* — is architecturally closer to LeWM's stated intent than
   Cosmos is. Needs adaptation to consume BVH/sensor state rather than raw video; that
   adaptation is real training work.
5. **DINOSAUR-style object proposals** feeding ADR-078's Phase-3 semantic labelling — with
   **the BVH keeping ID authority**, per §3. Candidate generation, not identity.
6. **Cosmos-style action-conditioned dynamics** — the genuinely Cosmos idea, but it needs
   training investment and there are no transferable weights (domain mismatch). H100-rental
   territory, not the 4070 or Jetson, and not until §1 and the WEFT-709 partition land.

---

## 6. Housekeeping finding, worth acting on separately

The `feature/lewm-worldmodel-rs-page` branch is a public-facing explainer that **writes as
though ADRs 048–058 already shipped**. They did not; those numbers belong to unrelated
accepted decisions, and the LeWM crates do not exist. This analysis was initially briefed on
the assumption LeWM was infrastructure — it isn't. Reconcile before that page goes anywhere
public.

## Provenance

Cosmos claims are quoted from the Cosmos 3 technical report (139pp, downloaded and grepped)
plus the Cosmos3-Edge HF card. WeftOS claims are verified against the code — leaf.rs,
query.rs, world_model.rs, Cargo dependency graph — not from ADR text. Embedding claims are
sourced to named papers. Nothing was run: Cosmos requires NVIDIA hardware (see
`cosmos3-edge-eval.md` for the per-machine breakdown).
