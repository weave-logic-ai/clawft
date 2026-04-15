# FishGraph GNN — Deep Analysis

## Citation

- **Title (survey, as given)**: "FishGraph. Graph neural network (GAT) where
  nodes are hydrophones carrying spectrograms and edges encode inter-hydrophone
  TDOA. Jointly predicts source (x,y) and species label."
- **Survey attribution**: Martinez, Chou et al. *IEEE OCEANS*, 2024.
- **Verified title (substituted)**: *"Graph Neural Networks for Sound Source
  Localization on Distributed Microphone Networks."*
- **Verified authors**: Eric Grinstein, Mike Brookes, Patrick A. Naylor
  (Department of Electrical and Electronic Engineering, Imperial College London).
- **Venue**: IEEE ICASSP 2023 (poster).
- **Year**: 2023
- **arXiv ID**: arXiv:2306.16081
- **Primary URL**: https://arxiv.org/abs/2306.16081
- **Code**: https://github.com/egrinstein/gnn_ssl
- **Downloaded PDF**:
  `.planning/sonobuoy/papers/pdfs/fishgraph-gnn.pdf` (293 kB, 5 pages).

## Status

**Substituted** — from "Martinez, Chou et al., IEEE OCEANS 2024 (FishGraph,
GAT over hydrophones)" to "Grinstein, Brookes, Naylor, ICASSP 2023 (RelNet
GNN for distributed microphone SSL)".

Reasoning: Google Scholar, arXiv, IEEE Xplore OCEANS 2024 proceedings, and
general web searches produced no paper titled "FishGraph" or any matching
work by authors named Martinez and Chou combining GAT + TDOA edge weights +
joint species + source localisation on hydrophone arrays. The closest real
paper that actually implements a GNN over a distributed acoustic-sensor
array with pairwise edge features derived from TDOA / GCC-PHAT is Grinstein
et al. (2023). The architectural substrate (GNN over a variable-N sensor
array with pairwise cross-correlation edge features for source localisation
on an exponential spatial likelihood field) is exactly the capability the
survey claims for "FishGraph", so the substitution preserves the intended
K-STEMIT slot. The TDOA-Gaussian edge-weight formula
`exp(-Δt² / (2σ²))` requested in the survey is not the exact form the
Grinstein paper uses; the Grinstein paper instead uses a GCC-PHAT /
Spatial Likelihood Function (SLF) preprocessing that is functionally
equivalent (smooth delay-based similarity), and the portable-details
section below includes *both* formulations so downstream Rust work can
pick whichever is more appropriate for hydrophone TDOA under ocean noise.

Note: the survey's joint "species + source (x,y)" task is not in Grinstein.
For the sonobuoy use case, the species head is supplied by the
SurfPerch / AST embedding branch (see `ast-fish-classification.md`); the
Grinstein GNN provides the source-location head, and the two can be
merged via a small multi-head output layer on top of the RelNet aggregator.

## One-paragraph summary

Grinstein, Brookes, and Naylor argue that Distributed Microphone Arrays
(DMAs) break most end-to-end deep-learning pipelines because the number of
channels is not fixed at training time (devices drop, batteries die,
buoys drift). They propose a Relation-Network GNN in which each node is a
microphone / buoy signal frame, each undirected edge is a pair (i, j), and
the edge feature is a classical signal-processing construct — GCC-PHAT or
the SLF cross-correlation heat-map — concatenated with microphone
coordinates and room-dimension metadata. An MLP `F(xᵢ, xⱼ; ϕ)` summarises
each pair into a spatial likelihood grid; all pairs are **summed** (a
permutation- and count-invariant aggregation), and a second MLP `G(Σ F)`
produces the final N×N source-likelihood map over the room floor. The
target map is a **Gaussian-like spatial likelihood** `y(u,v) = exp(-‖p_{u,v} − p_s‖₂)`
centred on the true source. Trained on a synthetic Pyroomacoustics corpus
with 5 or 7 microphones, the network generalises to unseen 4- and 6-mic
configurations at test time, and the SLF variant beats classical TDOA and
SLF baselines by up to 29 % on mean localisation error (in metres) in the
4-mic case. The paper is a clean, minimal GNN-for-SSL recipe that ports
directly to the sonobuoy array topology.

## Methodology

### Problem setup

- 2-D source position estimation `p̂_s ∈ ℝ²` inside a reverberant 3-D
  room of known dimensions `d = [d_x, d_y, d_z]ᵀ`.
- `M` microphones at known 3-D positions `p_m`; microphone `m` captures
  `x_m(t) = a_m · s(t − τ_m) + ϵ_m(t)`, where `τ_m = c⁻¹ ‖p_m − p_s‖₂`
  and `ϵ_m` aggregates noise and reverberation.
- Signals are processed in frames of length `L` (500 ms at the paper's
  sampling rate) as vectors `x_m(t)`.

### Node features

- Each node `m` carries the time-domain frame `x_m`. Raw waveforms, not
  spectrograms — spectrograms are only implicit inside the GCC-PHAT
  preprocessing.

### Edge features (preprocessing function `H`)

The paper explores two alternatives:

1. **GCC-PHAT** (Generalised Cross-Correlation with Phase Transform):

   ```
   R_ij(τ) = IFFT{ X_i(f) · X_j*(f) / |X_i(f) · X_j*(f)| }
   ```

   The central 200 lag bins are kept (rationale: max TDOA is bounded by
   the room diagonal). DFT size is 1024 samples.

2. **Spatial Likelihood Function (SLF)**: project the cross-correlation
   onto a 25×25 spatial grid `{p_{u,v}}`, where each cell receives the
   cross-correlation value at the lag corresponding to its theoretical
   TDOA `τ_{ij}(u,v) = c⁻¹ (‖p_{u,v} − p_i‖₂ − ‖p_{u,v} − p_j‖₂)`.

The edge feature is then

```
F(x_i, x_j; ϕ) = MLP_F( concat( H(x_i, x_j), p_i, p_j, d ) )
```

where `ϕ` is the metadata vector containing microphone coordinates and
room dimensions. Including `(p_i, p_j, d)` in the MLP input is what
enables generalisation to unseen arrays — the network treats the metadata
as part of the edge feature vector rather than baking a fixed geometry
into its weights.

### Aggregation (Relation Network)

```
ŷ = RN(X) = G( Σ_{i ≠ j} F(x_i, x_j; ϕ) )      (Eq. 3)
```

- `F`: MLP, 3 layers, output size 625 (= 25 × 25), ReLU.
- `G`: MLP, 3 layers, output size 625, ReLU except output (linear).
- Aggregation: pure summation. This is the key property that makes the
  network permutation-invariant and channel-count-agnostic.

### Target (spatial likelihood map)

```
y(u, v) = exp( -‖p_{u,v} − p_s‖₂ )      (Eq. 6)
```

Peak value = 1 at the true source, decaying exponentially with metric
distance. Peak-picking `argmax` recovers the estimated location. The
paper notes this formulation naturally extends to multi-source by picking
multiple peaks.

### Loss

Mean Absolute Error between predicted and target 25×25 grid (so-called
L1 loss):

```
L(y, ŷ) = (1 / |G|) · Σ_{u,v} | y(u,v) − ŷ(u,v) |
```

### Optimizer

- **Adam** (Kingma-Ba) with learning rate `λ = 5e-4`.
- Joint updates for `F` and `G`:
  `w_F ← w_F − λ · ∂L/∂w_F`, same for `w_G` (Eq. 5).

### Dataset

- **Synthetic, Pyroomacoustics image-source method.** No real hydrophone
  data.
- Per-sample room dimensions sampled `U[3, 6] m × U[3, 6] m × U[2, 4] m`.
- T60 reverberation sampled `U[0.3, 0.6] s` via Eyring's formula.
- Source uses speech clips from the VCTK corpus.
- **Per-channel SNR = 30 dB** (WGN added to each channel independently).
- **Training set: 15,000 examples, validation: 5,000, test: 10,000.**
- **Training mic counts: {5, 7}**; test mic counts: {4, 5, 6, 7}. Out-of-
  distribution mic counts (4 and 6) are the generalisation test.

### Hardware

Not reported explicitly; given the synthetic-only data, batch size 32,
and the small network, a single consumer GPU is sufficient.

### Training schedule

- Max 100 epochs.
- Early stopping with patience 3 on validation loss.
- Batch size 32.
- All grids 25×25.
- Frame length L = 500 ms (`L` samples at sample rate; paper uses VCTK
  defaults).
- GCC-PHAT: 1024-sample DFT, keep central 200 correlation bins.

## Key results

Mean Euclidean localisation error (metres) on the held-out test set:

| Mic count | TDOA baseline | SLF baseline | GNN-GCC (ours) | **GNN-SLF (ours)** |
|-----------|---------------|--------------|----------------|--------------------|
| 4         | ≈0.6          | ≈0.52        | ≈0.55          | **≈0.37**          |
| 5         | ≈0.5          | ≈0.42        | ≈0.48          | **≈0.28**          |
| 6         | ≈0.45         | ≈0.38        | ≈0.43          | **≈0.25**          |
| 7         | ≈0.4          | ≈0.34        | ≈0.40          | **≈0.22**          |

*Numbers extracted from Fig. 2 of the paper; exact values not tabulated
in the text.*

Key claims in the text:

- GNN-SLF **beats all baselines across all mic counts**.
- Best relative improvement: **29 %** over classical SLF in the **4-mic**
  case. This is the regime where fewer measurements are available, and
  the network's learned denoising of the SLF maps matters most.
- **GNN-GCC** (raw GCC-PHAT edge features) performs only marginally
  better than classical TDOA — telling, because it says *learning to
  project time lags into 2-D space is harder than post-processing an
  already-spatialised SLF map*. For a Rust port this implies the
  front-end should deliver spatialised features, not raw cross-correlation.
- The GNN **trained on {5, 7}** mics generalises cleanly to **{4, 6}**
  mic configurations — the primary motivating result for DMA deployments.

## Strengths

- **Count-invariant aggregation** (summation over pairs) is a structural
  match for sonobuoy arrays where buoy count is unstable (battery
  depletion, GPS loss, hardware failure).
- **Interpretable spatial target** (exponential-decay heatmap) avoids the
  optimisation pathologies of direct `(x, y)` regression — the network
  has a dense, smooth target instead of a sparse point.
- **Metadata fusion is clean**: concatenating `(p_i, p_j, d)` into the
  edge-feature MLP enables zero-shot generalisation to unseen room
  dimensions and microphone coordinates.
- **Classical-SP-aligned design**: the RelNet `Σ_{i<j} F(x_i, x_j)` is
  a direct generalisation of the classical Spatial Likelihood Function,
  so classical beamforming baselines are an apples-to-apples ceiling.
- **Minimal hyperparameters**: only 3 MLP layers on each side, one
  learning rate, one batch size. The full config fits on a single slide.

## Limitations

- **Simulation-only evaluation.** No real-room recordings, no underwater
  data, no multi-path/ambient-noise ablation. Transfer to hydrophone
  arrays requires additional validation.
- **Single source only.** Multi-source extension is discussed as future
  work but not benchmarked.
- **Fixed room-size, 2-D localisation.** Ocean soundscapes are
  quasi-unbounded and 3-D (water column depth matters). The target grid
  formulation would need expansion to 3-D or a different coordinate
  system.
- **No species / classification head.** The model is purely localisation.
  For the K-STEMIT sonobuoy slot we need to add a species output head
  on top of the aggregator (trivial architecturally, but not validated
  in the paper).
- **Complete graph cost.** Summation is over all `O(M²)` pairs. For
  sonobuoy arrays with M = 100+ buoys this is still fine, but memory
  scales quadratically — a k-nearest-neighbour graph would be more
  efficient and is flagged as future work in the K-STEMIT mapping doc.
- **Time-synchronisation assumption.** The classical GCC-PHAT/SLF
  baselines assume tight inter-sensor clock sync; the paper inherits
  this. Real sonobuoys require GPS-disciplined clocks or separate
  sync estimation.

## Portable details

### Complete module pseudocode (Rust-ish)

```rust
struct RelNetSSL {
    mlp_f: MLP,   // 3 layers, output_dim = 625
    mlp_g: MLP,   // 3 layers, output_dim = 625
}

fn forward(x: &[Frame; M], meta: &Metadata) -> [f32; 625] {
    let mut u = [0.0f32; 625];
    for i in 0..M {
        for j in (i + 1)..M {
            let edge = preprocess_slf(&x[i], &x[j], meta);   // 25x25 grid
            let feat = concat(&edge, &meta.pair(i, j));      // + p_i, p_j, d
            let f_ij = mlp_f.forward(&feat);                 // [625]
            u = add(u, f_ij);
        }
    }
    mlp_g.forward(&u)   // [625] spatial heatmap
}
```

### Spatial Likelihood Function preprocessing

For each pair `(i, j)` and each grid cell `(u, v)` centred at `p_{u,v}`:

```
τ_ij(u, v) = (1 / c) · ( ‖p_{u,v} − p_i‖₂ − ‖p_{u,v} − p_j‖₂ )
SLF_ij(u, v) = R_ij( round(τ_ij(u, v) · f_s) )
```

where `R_ij(·)` is the GCC-PHAT cross-correlation at integer lag,
`f_s` is the sample rate, and `c` is the sound speed (for underwater
≈1500 m/s instead of 343 m/s).

### Gaussian TDOA edge weight variant (matching survey's requested formula)

The survey requested the edge weight `exp(-Δt² / (2σ²))`. For a Rust
implementation this can replace the SLF preprocessing when cheaper
edges are needed:

```rust
fn tdoa_edge(x_i: &Frame, x_j: &Frame, sigma: f32) -> f32 {
    let r = gcc_phat(x_i, x_j);               // cross-correlation
    let lag = argmax_lag(&r);
    let delta_t = lag as f32 / f_s;
    (-(delta_t * delta_t) / (2.0 * sigma * sigma)).exp()
}
```

This gives a scalar edge weight per pair rather than a 625-dim grid;
it is the natural input for a GAT where edge weight modulates attention
(the survey's original intent). The sonobuoy implementation will likely
want **both**: a scalar Gaussian-TDOA weight for GAT-style attention
plus a full SLF heatmap edge feature for the RelNet-SSL aggregator.

### Target heatmap generation (training)

```rust
fn target_map(p_source: [f32; 2], grid: &Grid25x25) -> [f32; 625] {
    let mut y = [0.0f32; 625];
    for (idx, p_uv) in grid.iter().enumerate() {
        let d = l2_dist(p_source, *p_uv);
        y[idx] = (-d).exp();
    }
    y
}
```

### Loss

```rust
let loss = (pred - target).abs().mean();   // MAE / L1 over 625 cells
```

### Inference (peak picking)

```rust
let argmax = y_hat.iter().enumerate()
                 .max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).unwrap().0;
let p_hat = grid[argmax];
```

### Hyperparameters summary

| Parameter                       | Value                       |
|---------------------------------|-----------------------------|
| Grid resolution                 | 25 × 25 (625 cells)         |
| Frame length                    | 500 ms                      |
| GCC-PHAT DFT                    | 1024 samples                |
| GCC-PHAT lag bins kept          | 200 (central)               |
| MLP F                           | 3 layers, out = 625, ReLU   |
| MLP G                           | 3 layers, out = 625, ReLU   |
| Optimizer                       | Adam                        |
| Learning rate                   | 5e-4                        |
| Batch size                      | 32                          |
| Max epochs                      | 100                         |
| Early-stopping patience         | 3                           |
| Loss                            | MAE on spatial grid         |
| Training dataset size           | 15 000                      |
| Validation dataset size         | 5 000                       |
| Test dataset size               | 10 000                      |
| Room dims (training)            | U[3, 6] × U[3, 6] × U[2, 4] m|
| T60                             | U[0.3, 0.6] s               |
| SNR                             | 30 dB (WGN per channel)     |
| Training mic counts             | {5, 7}                      |
| Test mic counts                 | {4, 5, 6, 7}                |
| Sound speed `c` (to override)   | 343 m/s in air; 1500 m/s UW |

## Sonobuoy integration plan

Grinstein's RelNet-SSL plugs into the **spatial branch** of the
K-STEMIT-extended sonobuoy architecture (see
`.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`, section "GraphSAGE
spatial processing on buoy array geometry"). Concretely:

1. **Replace the fully-connected GraphSAGE with a RelNet aggregator.**
   K-STEMIT's original GraphSAGE requires fixed-size neighbourhood
   aggregation and implicit per-node embeddings; RelNet-SSL's
   `Σ_{i≠j} F(x_i, x_j)` is a cleaner fit for a variable-count sonobuoy
   array and directly produces the localisation heatmap.
2. **Edge features: dual path.** Implement both (a) the SLF 25×25 heat-map
   preprocessing in `weftos-signal` for the RelNet branch, and (b) the
   Gaussian TDOA scalar `exp(-Δt² / (2σ²))` for a future GAT branch.
   The SLF path needs a `c_underwater = 1500 m/s` override plus a
   configurable grid size (sonobuoy arrays cover km, not metres).
3. **Metadata vector ϕ**: reuse the buoy-position and ocean-metadata
   vector already wired into the K-STEMIT mapping (sound-speed profile,
   thermocline, currents). Extend ϕ with per-buoy GPS quality and clock-
   sync confidence so the network can down-weight unsynced buoys.
4. **Species head**: concatenate the SurfPerch / AST embedding from
   `ast-fish-classification.md` (1280-dim) with the summed-edge-pairs
   aggregate `u` before `G`; extend `G` with a second output head
   `G_species: ℝ^625+1280 → ℝ^C` for species classification. This is
   the **joint (x, y) + species** capability the survey asked for, now
   supplied by combining two real papers.
5. **HNSW indexing**. Store each buoy frame's 1280-dim SurfPerch embedding
   plus the RelNet spatial heatmap fingerprint in HNSW
   (`clawft-kernel/src/hnsw_service.rs`). New deployments can then
   retrieve "closest past deployment" from the embedding index to
   bootstrap the adaptive α between spatial and temporal branches.
6. **EML learned-function integration**. The edge-feature MLP `F` is a
   natural EML block: three linear layers + ReLU, stateless, amenable
   to EML's on-device fine-tuning. Ship `F` as an `eml-core` block and
   allow per-deployment adaptation as buoy geometry changes.
7. **ADR candidate**: ADR-054 (Relation-Network aggregation for
   variable-count distributed sensor arrays) complements the existing
   ADR-053 draft (dual-branch spatio-temporal).

## Follow-up references

1. **Santoro, A. et al.** "A simple neural network module for relational
   reasoning." *NeurIPS 2017.* — The RelNet primitive on which the
   paper's architecture is built. Directly portable.
2. **Knapp, C. & Carter, G.C.** "The generalized correlation method for
   estimation of time delay." *IEEE TASSP* 24(4), 320–327 (1976).
   — GCC-PHAT, the edge-feature preprocessor. Canonical signal-processing
   reference.
3. **DiBiase, J.H.** "A High-Accuracy, Low-Latency Technique for Talker
   Localization in Reverberant Environments Using Microphone Arrays."
   PhD thesis, Brown University, 2000. — SRP/SLF equivalence; source
   for the 25×25 grid formulation.
4. **Battaglia, P.W. et al.** "Relational inductive biases, deep
   learning, and graph networks." Technical Report, Google DeepMind,
   2018. — Broader context for RelNet and its extensions.
5. **Kipf, T.N. & Welling, M.** "Semi-Supervised Classification with
   Graph Convolutional Networks." *ICLR 2017.* — GCN baseline for the
   GAT direction (the survey originally asked for GAT, and this is the
   canonical starting point if we go that way instead of RelNet).
6. **Luo, Y.-K. et al.** "Evaluating railway noise sources using
   distributed microphone array and graph neural networks."
   *Transportation Research Part D* 107 (2022). — The only other
   GNN-for-distributed-microphone-array paper cited; reads like a
   sibling work for broadband-source mapping.

---

**Sources consulted**:
- [arXiv:2306.16081 — Graph Neural Networks for Sound Source Localization on Distributed Microphone Networks](https://arxiv.org/abs/2306.16081)
- [Author code repository](https://github.com/egrinstein/gnn_ssl)
- Pyroomacoustics library (cited as the data-generation tool)
- VCTK corpus (speech source)
