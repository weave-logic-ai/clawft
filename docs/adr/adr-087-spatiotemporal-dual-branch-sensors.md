# ADR-087: Spatio-Temporal Dual-Branch Architecture for Sensor Systems (K-STEMIT)

- **Status**: Proposed (Candidate)
- **Closes / tracks**: WEFT-372 (candidate write-up); implementation not yet scheduled
- **Date**: 2026-07-31
- **Deciders**: Pending (sensor / sonobuoy / EML maintainers)
- **Historical alias**: Phase-2 paper survey listed this as **“ADR-053”**. Numbers
  **050–053 were already claimed** (voice STT already occupies `adr-053-…`).
  Next free indices start at **084**. This document is the canonical candidate
  for survey item 053.
- **Related**: ADR-009 (sparse Lanczos), ADR-047 (cognitive tick), ADR-056
  (BVH spatial index), ADR-077 / ADR-078 (splat / world model — adjacent spatial
  story), ADR-079 (Urth — multi-scale twin vision), **ADR-095** (batch WCC/PageRank
  plane when sensor association graphs exceed RAM — dual-branch GNN stays local;
  DiskANN for embedding scale)
- **Source**:
  - `.planning/development_notes/knowledge-graph-paper-survey-phase2.md` (Paper 7: K-STEMIT)
  - ArXiv 2604.09922 — *K-STEMIT: Spatio-Temporal GNN for Subsurface Estimation*
  - `.planning/sonobuoy/k-stemit-sonobuoy-mapping.md` (radar→acoustic mapping; if present)
  - `crates/clawft-kernel/eml_coherence.rs`, `hnsw_service.rs`, mesh modules
  - `eml-core` model architecture

## Context

K-STEMIT (ArXiv 2604.09922) is a multi-branch GNN for ice-layer thickness from
radar, with:

- **Spatial branch** — GraphSAGE on geographic proximity graphs,
- **Temporal branch** — gated temporal convolution (GLU),
- **Adaptive fusion** — learnable \(\alpha \in [0,1]\):
  \(h = \alpha h_{spatial} + (1-\alpha) h_{temporal}\),
- **Physics-informed node features** from climate priors.

Survey maps this in **two** ways:

| Track | Priority | Notes |
|-------|----------|--------|
| **Sonobuoy / hydrophone array** | **P0** (sonobuoy project) | Dual-branch detect → bearing → species-ID; dedicated planning under `.planning/sonobuoy/` |
| **General WeftOS EML coherence** | **P2** | Decompose `GraphFeatures` into structural vs temporal branches + \(\alpha\) fusion for \(\lambda_2\) prediction |

This ADR **candidates** the architectural stance for both tracks without
shipping a sonobuoy product crate or rewriting EML immediately.

## Decision (Proposed)

### 1. Adopt dual-branch spatio-temporal decomposition as the sensor GNN pattern

**Propose** that WeftOS sensor / array pipelines that learn over graph-structured
observations **prefer a dual-branch design**:

- **Spatial (structural) branch** — neighborhood aggregation over a proximity or
  topology graph (GraphSAGE-style message passing or a WeftOS-native equivalent).
- **Temporal branch** — sequence / gated temporal features (interaction rates,
  edge age, sensor frames).
- **Adaptive fusion** — scalar or low-dim gate \(\alpha\) (learned or EML-predicted)
  combining branches, rather than a single flat feature vector.

Domain-specific readout (thickness, bearing, species, mesh health) sits **after**
fusion.

### 2. Sonobuoy / underwater path (product track)

When the sonobuoy project advances:

- Treat K-STEMIT’s dual-branch + physics-informed features as the **default
  reference architecture** for the unified detect → bearing-estimate → species-ID
  stack (see sonobuoy mapping notes).
- Prefer a **dedicated crate** (or clearly bounded module) rather than bloating
  `clawft-graphify`; graphify remains code/forensic KG (ADR-082).
- Physical priors (array geometry, sound-speed profile, etc.) enter as **node /
  edge features**, analogous to K-STEMIT’s MAR climate fields — not hard-coded
  only in post-processing.

### 3. EML coherence refinement (platform track, lower urgency)

**Propose** (P2) that `GraphFeatures` used by `eml_coherence` **may** split into
structural vs temporal groups with an \(\alpha\)-fusion head inside `eml-core`,
targeting better \(\lambda_2\) prediction without a full GNN runtime in the
cognitive tick.

This is **optional** and must respect the tick budget (ADR-047). It does **not**
require GraphSAGE in-kernel for v0 of the EML path — feature split + EML fusion
is enough for an experiment.

### 4. Relationship to BVH / world model

- **ADR-056 BVH** answers geometric queries; this ADR answers **learned**
  spatio-temporal representation for sensors.
- They compose: BVH / spatial index can supply neighborhoods; dual-branch model
  scores or estimates properties over those neighborhoods.
- **Not** a replacement for HNSW semantic search (ADR-011).

### 5. Explicit non-goals

- **Not** adopting K-STEMIT’s ice-radar loss or MAR climate features for core
  WeftOS.
- **Not** making graphify a scientific GNN framework.
- **Not** blocking 0.8 graphify P0 work (ADR-084) on sonobuoy delivery.

## Consequences

### Positive

- Shared pattern language for sensor GNNs and (later) EML feature design.
- Aligns sonobuoy P0 research with a surveyed SOTA decomposition.
- Clear crate boundary: sensors ≠ code KG.

### Negative / risks

- Dual-track ADR can stall if sonobuoy and EML owners diverge; keep decisions
  sectioned.
- Full GraphSAGE + gated conv may exceed edge device budgets; need slim variants.
- Physics-informed features are domain-specific engineering, not free.

### Follow-ups

- Ratify sonobuoy crate name / workspace placement when project kicks off.
- Spike EML feature-split + \(\alpha\) fusion with offline \(\lambda_2\) labels.
- Cross-link BVH neighborhood feeds once sensor leaves exist.
- Do **not** block graphify ADR-084/085 on this ADR.

## Alternatives considered

| Alternative | Why not (for now) |
|-------------|-------------------|
| Single flat MLP over mixed features | Survey: misses spatial/temporal decoupling gains |
| Force all sensors through graphify | Wrong domain; ADR-082 scope is code/forensic KG |
| Wait for Urth (ADR-079) only | Planetary twin is vision-scale; sensors need nearer-term pattern |
| Copy K-STEMIT weights/architecture verbatim | Domain mismatch (ice radar ≠ hydrophones / EML) |

## Open questions

1. Is sonobuoy dual-branch **Accepted** only after a separate product ADR, or is
   this candidate sufficient to start spikes?
2. Minimum viable spatial graph for hydrophones: fully connected haversine vs
   k-NN vs BVH-derived edges?
3. Should EML fusion land in `eml-core` generically or only behind an `ecc`
   feature in kernel?
