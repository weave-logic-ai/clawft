# ADR-086: Codebook Cold-Start for Emerging Entities (TransFIR)

- **Status**: Proposed (Candidate)
- **Closes / tracks**: WEFT-372 (candidate write-up); implementation not yet scheduled
- **Date**: 2026-07-31
- **Deciders**: Pending (ECC / causal graph / HNSW maintainers)
- **Historical alias**: Phase-2 paper survey listed this as **“ADR-052”**. Numbers
  **050–053 were already claimed**. Next free indices start at **084**. This
  document is the canonical candidate for survey item 052.
- **Related**: ADR-009 (sparse Lanczos / spectral), ADR-046 (forest), ADR-047
  (cognitive tick), ADR-056 (BVH), ADR-062 (ECC graph-walk), ADR-082 (graphify),
  ADR-085 (entity dedup — complementary, not the same problem)
- **Source**:
  - `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` (Paper 6: TransFIR)
  - ArXiv 2604.10164 (ICLR 2026) — *TransFIR: Inductive Reasoning for TKGs with Emerging Entities*
  - `crates/clawft-kernel/causal.rs`, `democritus.rs`, `hnsw_service.rs`
  - `crates/clawft-graphify/cluster.rs`

## Context

When a **new** entity appears in the CausalGraph or graphify output (new module,
person, service, conversation node), it often starts with **zero edges** and only
an initial embedding. DEMOCRITUS can discover neighbors via HNSW, but early ticks
have little structure. Label-propagation communities treat isolates as singleton
communities.

TransFIR (ArXiv 2604.10164, ICLR 2026) addresses emerging entities in temporal
KGs via:

1. **Codebook vector quantization** — assign entity embedding to nearest of K
   learnable prototypes.
2. **Interaction chain encoding** — Transformer over chronological interactions
   (relation-guided attention).
3. **Cluster-pooled pattern transfer** — inherit dynamic prototype patterns from
   known cluster members; anti-collapse losses on the codebook.

Reported gains: ~28.6% average MRR on standard TKG benchmarks.

Phase-2 survey priority: **P1**. Full Transformer chain encoding is **L** effort
and WeftOS has no general Transformer runtime in-kernel today. Survey recommends
a **tractible P1**: codebook classifier + **EML-based** pattern transfer (not
full TransFIR Transformer). Full pipeline = P2.

## Decision (Proposed)

### 1. Maintain a semantic codebook for cold-start classification

**Propose** a codebook \(C = \{c_1,\ldots,c_K\}\) of cluster prototypes (survey
suggests **K ≈ 64** as a starting point), stored as vectors addressable via HNSW
or a small dedicated table in the ECC / HNSW service layer.

On **new causal node / graphify entity** creation (after embedding):

1. \(\pi(e) = \arg\min_k \| h_e - c_k \|^2\) (or cosine argmax equivalent).
2. Attach cluster id \(\pi(e)\) as metadata / CrossRef.
3. **Do not** require prior interaction history for step 1.

### 2. Pattern transfer without in-kernel Transformer (v0)

**Propose** v0 transfer as:

- Aggregate a **dynamic prototype** per cluster from known members’ edge-type
  histograms / mean embeddings (running mean; tick-safe).
- Use **EML** (existing `eml-core` multi-head approx) to map
  \((h_e, c^{dyn}_{\pi(e)}) \rightarrow\) soft prior weights over candidate
  relation types or neighbor seeds.
- **Pre-populate** low-confidence / speculative causal edges (or candidate
  lists) for DEMOCRITUS to **validate or prune** — not hard truth.

This preserves tick budget and avoids a new Transformer dependency.

### 3. Scope of application

| Surface | v0 intent |
|---------|-----------|
| `CausalGraph` new nodes | Primary — cold-start edge priors |
| Graphify incremental scan | Secondary — predict likely relations for new modules |
| Conversation / ADR-062 nodes | Optional later; same codebook or domain-specific codes |

### 4. Training / update policy

- Initialize codebook via k-means or VQ on existing HNSW population offline or
  during warm-up.
- Online updates: slow EMA of cluster means; avoid collapse with commitment /
  usage regularization (TransFIR anti-collapse idea; simplified).
- Version the codebook with embedding model id (ADR-059).

### 5. Explicit non-goals (v0)

- **No** mandatory Transformer interaction-chain encoder in clawft-kernel.
- **No** replacement for HNSW search or entity dedup (ADR-085).
- **No** automatic high-confidence causal commits without DEMOCRITUS / coherence
  validation (ADR-047 tick integrity).

## Consequences

### Positive

- Reduces cold-start blindness for new modules and agents in multi-session ECC.
- Fits EML + HNSW architecture already shipped.
- Graphify incremental scans converge faster toward useful connectivity.

### Negative / risks

- Bad codebook → systematic wrong priors; need prune-friendly weights and metrics.
- Cluster count K and domain mix (code vs forensic vs conversation) may need
  multiple codebooks.
- EML approximation will underperform full TransFIR on pure TKG benchmarks;
  accept for operational fit.

### Follow-ups

- Prototype K=64 codebook + unit tests on synthetic emerging nodes.
- Compare edge-prediction MRR vs HNSW-only baseline on graphify-fed graphs.
- P2: evaluate optional ONNX Transformer chain encoder if accuracy plateaus.
- Coordinate with temporal edge scoring (RoMem survey P1 / possible future ADR).

## Alternatives considered

| Alternative | Why not (for now) |
|-------------|-------------------|
| Wait for neighbors only (status quo) | Cold-start lag on new modules / entities |
| Full TransFIR Transformer in-kernel | No runtime; L effort; survey defers |
| Only label propagation | Fails for degree-0 nodes |
| Larger HNSW `ef` only | Improves recall, not structural prior transfer |

## Open questions

1. One global codebook vs per-domain (code / forensic / conversation)?
2. Should priors write `CausalEdge` immediately or only a side “candidate” store?
3. Interaction with ADR-085 dedup: classify after merge decision or before?
