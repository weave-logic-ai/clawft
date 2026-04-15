# Paper 4.2 — GNN-TDOA with Uncertain Sensor Positions (Graph Neural Networks for Sound Source Localization on Distributed Microphone Networks)

## Citation

Grinstein, E., Brookes, M., & Naylor, P. A. (2023). "Graph Neural
Networks for Sound Source Localization on Distributed Microphone
Networks." *Proc. IEEE ICASSP 2023*, arXiv:2306.16081 (submitted 28 Jun
2023). Department of EEE, Imperial College London.

## Status

**Substituted.** The survey cited "Comanducci, Antonacci et al.
arXiv:2311.00866, 2023" as a "message-passing GNN that takes noisy TDOA
measurements + noisy sensor GPS as joint inputs, outputs source location
with calibrated uncertainty."

After verification:

- **arXiv:2311.00866** is *"Generalizing Nonlinear ICA Beyond Structural
  Sparsity"* by Zheng & Zhang — nothing to do with TDOA or localization.
  The arXiv ID in the survey is wrong.
- **Comanducci & Antonacci (Politecnico di Milano)** have published on
  source localization on distributed microphone networks, but no paper
  of theirs matches the exact description "joint noisy TDOA + noisy GPS
  → calibrated uncertainty." Their 2020 TASLP paper (Comanducci et al.,
  "Source localization using distributed microphones based on ray-space
  transform") is the closest author-match but it is not a GNN.

The **closest real paper** matching the survey's description ("GNN that
takes sensor signals + metadata on sensor coordinates and handles
variable/uncertain sensor networks") is Grinstein, Brookes & Naylor
(2023), arXiv:2306.16081. It is a message-passing (Relation Network) GNN
that consumes microphone signals plus a metadata vector containing mic
coordinates + room dimensions. It does **not** output calibrated
uncertainty; it outputs a 2D spatial likelihood heatmap. It does **not**
specifically test sensor-position noise > 1 m (the survey's headline
claim) — it tests robustness to variable numbers of microphones.

The survey's claim "beats Gauss-Newton TDOA solvers under sensor-
position noise > 1 m" is **not validated** by any extant paper I
could locate. This claim should be downgraded in the sonobuoy plan:
we adopt Grinstein as the default localization backend, but we must
**run our own sensor-jitter ablation** before citing that number.

PDF: `.planning/sonobuoy/papers/pdfs/gnn-tdoa-uncertain.pdf` (293 KB,
5 pages).

## One-paragraph summary

Grinstein et al. adapt the **Relation Network** architecture from visual
question answering to sound source localization on Distributed
Microphone Arrays (DMAs). Each microphone is a node. For every pair
(i, j) the network computes a pairwise *relation* `F(x_i, x_j; φ)` —
implemented as `MLP(H(x_i, x_j; φ))` where `H` is a classical
signal-processing feature extractor (either GCC-PHAT cross-correlation
or Spatial Likelihood Function over a grid), and `φ` is a metadata
vector containing mic coordinates + room dimensions. All pairwise
relations are summed (as in the classical SLF/TDOA peak-picking
formulation), and a second MLP `G(Σ F)` post-processes the sum into a
final heatmap over a 25×25 spatial grid. The target heatmap peaks at
the true source location with exponential falloff. The method naturally
handles a **variable number of input microphones** (sum is permutation-
and arity-invariant) and is trained on graphs with 5 and 7 mics but
tested on 4-7 — demonstrating generalization to unseen array sizes. It
improves over classical SLF baselines by up to ~29% on 4-mic
configurations.

## Methodology

### Architecture — Relation Network GNN

Let `{x_1, ..., x_M}` be the microphone-signal vectors (length L = 500 ms
frames) and `φ = [p_x^1, p_y^1, ..., d_y, d_z]^T` the metadata vector
(per-mic coordinates + room dimensions).

**Relation function `F`** (per-edge):

```
F(x_i, x_j; φ) = MLP_F(H(x_i, x_j; φ))
```

where `H` is one of two preprocessing variants:

- **GNN-GCC**: `H = GCC-PHAT(x_i, x_j)` — generalized cross-correlation
  with phase transform. Output is a 1D cross-correlation function;
  the MLP must learn to project time lags into 2D space.
- **GNN-SLF**: `H = SLF(x_i, x_j, φ)` — cross-correlation pre-projected
  into the 2D spatial grid (equivalent to classical Steered Response
  Power). Output is an `N × N` grid (flattened to `N²` vector). The
  MLP must learn to *denoise* reverberation-corrupted likelihood maps.

Mic-pair coordinates + room dimensions are concatenated to the feature
vector before the MLP.

**Aggregation (sum):**

```
u = Σ_{(i,j)} F(x_i, x_j; φ)
```

This is the key permutation-invariant step — any number of mics can be
summed, so the network handles variable `M`.

**Fusion function `G`:**

```
ŷ = G(u) = MLP_G(u)
```

Output is the same size as `F` — a flattened `25 × 25` heatmap. The
target heatmap is

```
y(u, v) = exp(−‖p_{u,v} − p_s‖₂)
```

so the target is a 2D Gaussian-like bump centered at the true source
location `p_s`.

**Source location estimate** = `argmax_{u,v} ŷ(u,v)`.

### Training

| Parameter         | Value                                          |
| ----------------- | ---------------------------------------------- |
| Optimizer         | Adam                                           |
| Learning rate     | `5e-4`                                         |
| Batch size        | 32                                             |
| Max epochs        | 100 (early stop after 3 validation-plateau)    |
| Grid              | `25 × 25`                                      |
| Input frame `L`   | 500 ms                                         |
| DFT (GCC-PHAT)    | 1024 samples                                   |
| MLP_F layers      | 3 layers, 625 neurons each, ReLU (output: none) |
| MLP_G layers      | 3 layers, 625 neurons each, ReLU (output: none) |
| Loss              | Mean Absolute Error (MAE, L1) on heatmap       |

### Dataset

Simulated room acoustics. Training: mic counts in `{5, 7}`. Testing:
`{4, 5, 6, 7}` — the 4-mic and 6-mic cases are **unseen** and test
generalization to new array sizes. Room dimensions, mic coordinates, and
source position vary per sample. Speech source; reverberation modeled
by image-source.

## Key results

**Mean Euclidean localization error (metric: meters)** on test set, by
mic count (from Fig. 2 of the paper; values are visual-readout
approximations — exact numbers not tabulated in the 4-page ICASSP
short):

| # mics | TDOA (classical) | SLF (classical) | GNN-GCC | **GNN-SLF** |
| ------ | ---------------- | --------------- | ------- | ----------- |
| 4      | ~0.70            | ~0.55           | ~0.45   | **~0.39**   |
| 5      | ~0.60            | ~0.45           | ~0.35   | **~0.30**   |
| 6      | ~0.55            | ~0.40           | ~0.30   | **~0.28**   |
| 7      | ~0.50            | ~0.35           | ~0.30   | **~0.26**   |

At 4 mics, GNN-SLF gives ~29% improvement over the classical SLF
baseline. At 4 and 6 mics (unseen during training), performance is in
line with trained sizes — confirming generalization to new array sizes.

**Feature-extractor ordering from lowest to highest error**:
`GNN-SLF > GNN-GCC > classical SLF > classical TDOA`.

The SLF-based GNN benefits from its input already being
space-structured; the GCC-based GNN must internally learn the
delay-to-space mapping.

## Strengths

1. **Handles variable `M` natively.** Trained on 5/7 mics, works on
   4/5/6/7. No retraining. Exactly what we need for sonobuoys where
   buoys drop out (battery death, GPS loss, hardware fault).
2. **Permutation-invariant via sum aggregation.** Matches classical
   SSL where the order of microphones is irrelevant.
3. **Pairwise relations respect acoustic physics.** `F(x_i, x_j)` is a
   *pair*-level feature (mimics how classical TDOA/SLF compute
   inter-sensor delays). This is the right inductive bias for
   multi-sensor localization.
4. **Metadata vector can absorb sensor coordinates.** The network is
   conditioned on `φ`, so in principle the **same trained model**
   can be rerun with *updated* coordinates. This is the mechanism by
   which it can handle drifting sonobuoys at inference time (though
   not validated at noise > 1 m).
5. **Hybrid with classical DSP.** By making `H` a known feature
   (GCC-PHAT/SLF), the GNN is bootstrapped by decades of signal-
   processing wisdom; it only has to learn the *residual* correction
   needed to handle reverb.

## Limitations

1. **No uncertainty calibration.** Output is an MAE-regressed heatmap.
   The highest value is just taken as the point estimate. No posterior,
   no confidence intervals, no ensemble/dropout-based uncertainty.
   The survey's "calibrated uncertainty" claim is **not supported**.
2. **Sensor-position noise not ablated.** The paper does not
   perturb the mic coordinates in `φ` at test time. The claim that the
   method is robust to sensor-position noise > 1 m is an **untested
   extrapolation**.
3. **Single static source only.** Multi-source extension is listed as
   future work.
4. **Fully-connected graph** (all pairs). For `M = 64` buoys, 2016
   pairs — still tractable, but we should check scaling.
5. **Fixed 25×25 grid.** For ocean basins, this resolution is coarse.
   Hierarchical (coarse-to-fine) grids or a gridless regression head
   would be more appropriate for sonobuoys.
6. **Trained on rooms, not oceans.** Same transfer concern as GNN-BF
   (4.1).

## Portable details (for Rust implementation)

### Full equation set

```
M microphones, each with signal frame x_i ∈ ℝ^L  (L = 500 ms)
Metadata vector φ ∈ ℝ^{2M + 2}  (2D coords + 2 room dims)

Per-pair relation:
   F(x_i, x_j; φ) = MLP_F(concat(H(x_i, x_j), φ_{ij}))
   φ_{ij} = [p_i, p_j, d_y, d_z]  (coords of the pair + room)

Feature H:
   GCC-PHAT:  h_ij = IFFT( X_i · conj(X_j) / |X_i · conj(X_j)| )
   SLF:       h_ij(u,v) = h_ij_GCC[ τ(p_{u,v}, p_i, p_j) ]
              where τ(p, p_i, p_j) = (‖p − p_i‖ − ‖p − p_j‖) / c

Aggregation:
   u = Σ_{i<j} F(x_i, x_j; φ)                    // 625-dim vector

Fusion:
   ŷ = MLP_G(u)                                  // 625-dim (25×25 heatmap)

Target:
   y(u, v) = exp(−‖p_{u,v} − p_s‖₂)
   Loss = mean | ŷ − y |                         // MAE
```

### Hyperparameters (paper-exact)

| Parameter                | Value              |
| ------------------------ | ------------------ |
| Frame length `L`         | 500 ms             |
| DFT size (GCC-PHAT)      | 1024               |
| Grid size                | 25 × 25 (= 625)    |
| MLP_F layers × neurons   | 3 × 625, ReLU      |
| MLP_G layers × neurons   | 3 × 625, ReLU      |
| Optimizer                | Adam               |
| Learning rate            | `5e-4`             |
| Batch size               | 32                 |
| Max epochs               | 100 (early stop 3) |
| Loss                     | L1 / MAE           |

### Rust/Candle implementation sketch

```rust
// Inputs
// signals:     (B, M, L)         f32, audio frames
// coords:      (B, M, 2)         f32, mic coordinates
// room_dims:   (B, 2)            f32
// target_pos:  (B, 2)            f32, only at training

// Build pairs (i<j) — M*(M-1)/2 pairs
let pairs = all_pairs(M);          // Vec<(usize, usize)>

// Per-pair GCC-PHAT
let mut relations = Vec::new();
for (i, j) in &pairs {
    let x_i = signals.select(1, *i);
    let x_j = signals.select(1, *j);
    let gcc = gcc_phat(&x_i, &x_j, dft_size=1024);            // (B, L_cc)
    let slf = project_to_grid(&gcc, coords, i, j, grid=25);   // (B, 625)
    let phi_ij = concat_metadata(coords, i, j, room_dims);    // (B, 6)
    let feat = concat(slf, phi_ij);                           // (B, 631)
    let f_ij = mlp_f.forward(&feat);                          // (B, 625)
    relations.push(f_ij);
}

// Sum aggregation
let u = stack(relations, dim=1).sum_dim(1);                   // (B, 625)

// Fusion
let heatmap = mlp_g.forward(&u);                              // (B, 625)

// Source estimate
let idx = heatmap.argmax(dim=1);
let p_hat = grid_to_pos(idx, grid=25, room_dims);
```

The **SLF projection step** is the differentiable equivalent of a
classical Steered Response Power map; it requires per-pair:

```rust
for (u, v) in grid.iter() {
    let p_uv = grid_to_pos(u, v);
    let tau_ij = (p_uv.dist(&coords[i]) - p_uv.dist(&coords[j])) / c;
    let lag_samples = (tau_ij * sample_rate).round() as i64;
    slf_grid[u, v] = gcc[midpoint + lag_samples];
}
```

This is fully differentiable in `coords` (via `grad_τ/grad_p`), which
is the **key hook for sonobuoys** — at inference time, we can *update
the coordinates in φ* to match GPS, and if we also treat the
coordinates as learnable we can jointly refine sensor positions and
source position via gradient descent. This is how we can bolt on the
"sensor-position uncertainty" capability that the survey claimed
already existed.

### Modifications needed for sensor-position uncertainty

The Grinstein paper *does not* handle this. We must add it. Two
options:

1. **Input perturbation during training.** Add Gaussian noise
   `φ → φ + ε, ε ~ N(0, σ² I)` to training samples, `σ ∈ {0.1, 0.5,
   1.0, 2.0} m`. Network learns to be robust. Simplest,
   recommended first.
2. **Joint optimization at inference.** Treat mic coordinates as
   latent variables; backpropagate through the SLF projection to
   refine them jointly with the source estimate. More principled,
   more complex.
3. **Probabilistic head.** Replace the MAE heatmap target with a
   Gaussian Process / mixture-density-network head so the output
   is a calibrated posterior `p(p_s | signals, φ)`. Enables
   "calibrated uncertainty" as the survey claimed.

## Sonobuoy integration plan

This paper is the **localization / bearing estimation head** for the
sonobuoy spatial branch.

**Where it slots in:**

1. **Downstream of the Tzirakis GNN-BF encoder (paper 4.1).** The
   enhanced per-buoy signals + buoy coordinates feed into Grinstein's
   Relation Network.
2. **Replaces** K-STEMIT's bearing-estimation-by-aggregation-over-
   haversine-weighted-graph with an **explicit pair-level SLF**. This
   is closer to classical DSP and easier to calibrate.
3. **Pairs with the TDOA feature extractor** — since each buoy's drift
   makes the TDOA-to-position mapping time-varying, the SLF projection
   *must* be recomputed per buoy-coordinate update. The model is
   conditioned on `φ`, so it just takes the refreshed coordinates as
   input; no retraining.

**Concrete architecture for sonobuoy localization head:**

```
per-buoy STFT (16 kHz, 1024 Hanning)
       │
       ▼
U-Net encoder (Tzirakis) → shared embedding f_v ∈ ℝ^N      ←── paper 4.1
       │
       ▼
Dynamic adjacency GCN (Tzirakis)                            ←── paper 4.1
       │
       ▼
Per-pair GCC-PHAT + SLF grid projection                     ←── this paper
       │
       ▼
Relation MLP F: (slf + φ_{ij}) → 625-dim per-edge           ←── this paper
       │
       ▼
Sum over all (i,j) pairs                                    ←── this paper
       │
       ▼
Fusion MLP G: 625 → 625                                     ←── this paper
       │
       ▼
Source heatmap (25×25 lat/lon grid, ocean-scaled)
       │
       ├─── argmax → bearing/position estimate
       └─── second-moment / MC-dropout → posterior variance
```

**Changes for sonobuoy-scale**:
- Grid 25×25 is too coarse for ocean. Use hierarchical grids: 64×64
  over a 10 km × 10 km tile, then refine to 256×256 over the best
  10-tile.
- Replace `c = 343 m/s` (speech) with a depth-dependent sound-speed
  profile from the thermocline-FiLM paper (sibling agent's download).
  The SLF projection step becomes `τ_{ij}(p) = ∫ ds/c(z)` along the
  ray — use a precomputed ray table.
- Augment training with synthetic sensor-position noise (σ up to 5 m
  for GPS-denied buoys) to cover the survey's claimed regime.
- Add a second head for **uncertainty**: either an MC-dropout pass
  at inference (cheap) or a dedicated variance output (MDN-style).

**Quantum register bridge**: The summed SLF heatmap is a natural input
for the quantum cognitive layer's amplitude-encoding step. Each of the
625 grid cells maps to one basis state of a 10-qubit register (with
24 padding states). Grinstein's heatmap *is* a valid probability
distribution after softmax, so `|ψ⟩ = Σ_{u,v} √ŷ(u,v) |u,v⟩` is a
direct amplitude encoding — no extra learning required.

**Proposed ADR**: ADR-055 — *Relation-Network GNN with per-pair SLF as
default localization head, with sensor-position noise augmentation
during training.*

## Follow-up references

1. **Santoro et al., "A simple neural network module for relational
   reasoning,"** NeurIPS 2017 — origin of Relation Networks. The
   direct inspiration for Grinstein's architecture.
   [arXiv:1706.01427]
2. **Knapp & Carter, "The generalized correlation method for
   estimation of time delay,"** IEEE TASSP 1976. Canonical reference
   for GCC-PHAT. Essential reading for reimplementing `H(x_i, x_j)`.
3. **DiBiase, Silverman & Brandstein, "Robust localization in
   reverberant rooms,"** in *Microphone Arrays* (Springer 2001) —
   introduces SRP-PHAT, which is equivalent to Grinstein's SLF.
4. **Chakrabarty & Habets, "Multi-speaker DOA estimation using deep
   convolutional networks trained with noise signals,"** IEEE
   JSTSP 2019. State-of-the-art CNN approach that Grinstein implicitly
   positions against; provides the baseline to track for multi-source
   extensions.
5. **Comanducci et al., "Source localization using distributed
   microphones in reverberant environments based on deep learning
   and ray space transform,"** IEEE/ACM TASLP 2020. The *real*
   Comanducci/Antonacci paper that the survey likely confused with
   the 2311.00866 citation. Useful for the "ray-space transform"
   idea that could be a compact physics-informed feature for
   sonobuoys.

---

*Generated 2026-04-15. PDF at `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/gnn-tdoa-uncertain.pdf`. Source: arXiv:2306.16081 (ICASSP 2023). Original survey citation (Comanducci/Antonacci 2311.00866) does not exist; the closest real paper is Grinstein et al.*
