# Paper 4.3 — Neural Beamforming for Sparse Arrays (Subspace Representation Learning for Sparse Linear Arrays)

## Citation

Chen, K.-L., & Rao, B. D. (2025). "Subspace Representation Learning for
Sparse Linear Arrays to Localize More Sources than Sensors: A Deep
Learning Methodology." *IEEE Transactions on Signal Processing*,
DOI: 10.1109/TSP.2025.3544170. arXiv:2408.16605v2 (submitted Aug 2024,
revised Mar 2025). Department of ECE, University of California, San
Diego.

## Status

**Substituted.** The survey cited "Chen, Wang et al., IEEE Transactions
on Signal Processing 2024," described as a "neural beamformer for
sparse/non-uniform underwater arrays, trained with physics-derived
propagation model as a differentiable layer."

After verification:

- **No Chen/Wang 2024 IEEE TSP paper matching this description exists.**
- The **closest real paper** with matching author (Chen), venue (IEEE
  TSP), year (2024 submission / 2025 publication), and core topic
  (deep learning for sparse linear arrays) is Chen & Rao (2025).
  Bhaskar Rao is a Life Fellow of IEEE at UCSD — not "Wang," but
  likely the misremembered co-author.
- The survey's specific technical claim — "physics-derived propagation
  model as a differentiable layer" — is **not** a feature of
  Chen & Rao. Their method is *explicitly* geometry-agnostic and data-
  driven, with **no embedded physics propagation layer.** The paper
  is arguably the opposite design philosophy: it *avoids* array-
  geometry priors so the network can handle imperfect arrays.
- The match on "sparse array" is strong. The match on "sparse
  *underwater* array" is only by analogy — the paper's motivating
  application list mentions sonar (hearing aids, wireless comms,
  sonar) but the experiments are on abstract SLA signal models, not
  hydrophone data.

We adopt Chen & Rao as the substitute because of the "sparse linear
array + Chen + IEEE TSP" match, with the caveat that the "physics-
derived differentiable propagation layer" is **not present** and
would need to be added by us — this is a gap, not a feature.

For the "physics-derived differentiable propagation" aspect of the
survey claim, the right paper to pair this with is `pinn-ssp-
helmholtz.pdf` (sibling agent downloaded) which is a physics-informed
neural network for underwater propagation — *that* paper provides the
differentiable propagation layer that Chen & Rao lacks.

PDF: `.planning/sonobuoy/papers/pdfs/neural-beamforming-sparse.pdf`
(787 KB, 16 pages).

## One-paragraph summary

Chen & Rao reformulate sparse-linear-array direction-of-arrival (DoA)
estimation as **subspace representation learning**. Classical sparse
array DoA estimation (SPA / Co-MLM / StructCovMLE) recovers the full
covariance matrix of a *virtual* uniform linear array (ULA) from the
sample covariance of a sparse physical array, then applies MUSIC.
DNN-based approaches (Wu et al., Barthelme & Utschick) do the same but
learn a covariance map instead of solving an SDP. Chen & Rao argue this
is harder than needed: MUSIC only needs the signal/noise **subspace**,
not the full covariance. They train a WRN-16-8 wide ResNet to output a
square matrix whose Gram matrix's eigenvectors span the target signal
subspace; training minimizes the **principal-angle distance** on the
Grassmannian manifold (making the loss invariant to basis rotations, a
much larger solution space than covariance L2). Root-MUSIC then finds
DoAs from the learned subspace. A 5-element Minimum Redundancy Array
(MRA) recovers up to 9 sources (more sources than sensors) via the
10-element virtual ULA in its co-array. The method is explicitly
**geometry-agnostic**: it does *not* assume knowledge of exact sensor
positions, which makes it robust to array imperfections. It outperforms
SPA (SDP baseline) and DCR-T / DCR-G-Fro / DCR-G-Aff (recent DNN
covariance-learning methods) across a wide range of SNRs (-10 to 20
dB), snapshot counts, and source counts, on both perfect and imperfect
arrays.

## Methodology

### Signal model

`M`-element ULA with spacing `d = λ/2` (virtual reference). `N`-element
SLA formed by selecting sensors `S ⊂ {1, ..., M}`, `|S| = N < M`. Goal:
localize `k` sources at DoAs `θ = {θ₁, ..., θₖ} ⊂ [-π/2, π/2]`, with
`k` possibly larger than `N` but bounded by `M − 1`.

Narrowband far-field model:

```
R₀ = A(θ) P Aᴴ(θ)                   (ULA noiseless covariance)
Rₛ = selection(R₀, S)                (SLA noiseless covariance)
R̂ₛ = (1/T) Σₜ yₜ yₜᴴ                  (sample covariance from T snapshots)
```

Example: `N = 5` MRA with sensor positions `S = {1, 2, 5, 8, 10}`
recovers the 10-element virtual ULA (`M = 10`) and can resolve up to
9 sources.

### Architecture: WRN-16-8 (Wide Residual Network)

- **Backbone:** Wide Residual Network 16-8. 3 stages, 2 blocks per
  stage, 16 layers total, widening factor 8 (8× wider than standard
  ResNet). Pre-activation residual form. No batch normalization.
  ReLU throughout.
- **Parameter count:** ~11 million.
- **Input:** `R2×N×N` tensor (real + imaginary parts of `R̂ₛ`, the
  N×N sample SCM of the SLA).
- **Output:** `R2×M×M` tensor (real + imag of learned square matrix
  `X` — the subspace representation). Note `M > N`, so the network
  is *expanding*.
- **Output layer:** affine — dimension tailored per method variant
  (DCR-T, DCR-G-Fro, DCR-G-Aff, subspace-learning, or gridless).

### Subspace extraction (post-network)

After the network produces `X ∈ ℂ^{M×M}`, compute:

```
G = X Xᴴ                         (Gram matrix)
eigendecompose G = U Λ Uᴴ        (principal eigenvectors)
signal subspace Ûₛ = U[:, 1..k]  (top-k eigenvectors)
noise subspace Ûₙ = U[:, k+1..M]
```

Then apply **root-MUSIC** on `Ûₙ` to recover the DoAs `θ̂`.

### Loss functions — principal-angle distances on Grassmannians

The target signal subspace is `Uₛ = A(θ) · (anything)` — the column
span of the steering matrix. Since Uₛ is only defined up to a rotation
within itself, the loss must be *invariant to the basis*. Chen & Rao
propose losses built from principal angles `φ₁, ..., φₖ` between the
target subspace `Uₛ` and the learned subspace `Ûₛ`:

| Distance name                        | Formula                                    |
| ------------------------------------ | ------------------------------------------ |
| Geodesic (Grassmannian)              | `(Σᵢ φᵢ²)^(1/2)`                          |
| Chordal (projection Frobenius norm)  | `(Σᵢ sin²φᵢ)^(1/2)`                       |
| Projection 2-norm                    | `sin φₖ`                                   |
| Chordal Frobenius norm               | `2 (Σᵢ sin²(φᵢ/2))^(1/2)`                 |
| Chordal 2-norm                       | `2 sin(φₖ/2)`                             |
| Fubini-Study                         | `arccos(|det Uₛᴴ Ûₛ|)`                     |

Any of these is a valid loss — all are invariant to rotations within
the subspace. Experimentally, **chordal and Fubini-Study** tend to
give the best DoA MSE.

The critical formula is the principal-angle cosines:

```
cos φᵢ = σᵢ (Uₛᴴ Ûₛ)        (i-th singular value of the cross-projection)
```

### Consistent rank sampling (training acceleration)

A subtle training-efficiency contribution: to train *one* network that
handles all source counts `k ∈ {1, ..., M-1}`, use a batch sampling
strategy where each minibatch contains only samples with the same `k`.
This lets each batch use the same fixed-rank loss computation without
rank-handling branches — parallelizable on GPU.

### Training

- **Optimizer:** SGD with Nesterov momentum (no weight decay).
- **Epochs:** 50.
- **LR scheduler:** one-cycle.
- **Peak learning rate** (from grid search): 0.05 (DCR-T), 0.01
  (DCR-G-Fro), ~0.02-0.05 (Chen & Rao's method), 0.2 (end-to-end
  gridless variant).

### Dataset (synthetic)

- **Array:** 5-element MRA, `S = {1, 2, 5, 8, 10}`, virtual ULA `M=10`.
- **Sources:** `k ∈ {1, 2, ..., 9}`, uniformly sampled DoAs in `[π/6,
  5π/6]`.
- **Snapshots `T`:** 50 (default; swept in ablations).
- **SNR:** default 20 dB; swept over `{-10, -8, ..., 18, 20} dB`.
- **Signal model:** equal-power Gaussian sources + Gaussian noise.
- Covariance samples generated analytically from source/noise model.

### Imperfect arrays (ablation)

Two models of array imperfection tested:

1. **Gain/phase errors**: `yₜ → Γ yₜ` where `Γ` is diagonal with
   random amplitude and phase errors per sensor.
2. **Position errors**: sensor positions perturbed by `σ_pos ∈ {λ/8,
   λ/4}`. The geometry-agnostic network adapts without retraining on
   the perturbation distribution.

## Key results

- **Main result**: The subspace-learning approach outperforms SPA
  (SDP baseline), DCR-T (Toeplitz-prior DNN), DCR-G-Fro (Frobenius
  DNN), and DCR-G-Aff (affine DNN) across the tested range of SNRs
  (-10 to 20 dB), snapshots (10 to 200), and source counts (1 to 9)
  on a 5-element MRA.
- **MSE at 20 dB SNR, T=50, k=3** (approximate, from Fig. in paper):
  - CRB (Cramér-Rao bound): ~10⁻⁴ deg²
  - Chen & Rao (subspace learning): ~2 × 10⁻⁴ deg²
  - DCR-T: ~5 × 10⁻⁴ deg²
  - SPA: ~7 × 10⁻⁴ deg²
- **More sources than sensors**: 5-element MRA handles k = 6, 7, 8, 9
  sources. At k=9 (maximum for `M=10`), subspace-learning variant
  is the only DNN method that continues to outperform SPA at
  low-to-moderate SNR.
- **Generalization to different MRA configurations**: The paper
  demonstrates that the same methodology works for 4-element and
  6-element MRAs with only minor hyperparameter adjustments.
- **Gridless end-to-end variant** (bypass root-MUSIC): uses the same
  WRN-16-8 but with `M-1` affine heads (one per source count). The
  subspace-learning approach *consistently outperforms* the gridless
  end-to-end variant at high SNR — the subspace → root-MUSIC pipeline
  preserves accuracy at high SNR while end-to-end saturates.

## Strengths

1. **Principled loss design.** The Grassmannian-manifold principal-
   angle loss is *basis-invariant*, giving the DNN a much larger
   solution space than covariance-L2 losses. Elegant and rigorous.
2. **Subspace > covariance > angles.** Numerical evidence that
   learning subspaces (a reduced representation) is easier for a DNN
   than learning full covariances *or* DoAs directly. Counter-intuitive
   but empirically validated.
3. **Geometry-agnostic.** Works without knowing exact sensor positions
   — exactly what sonobuoys need, where GPS is noisy and may be
   missing entirely.
4. **More sources than sensors.** Recovers up to `M-1` sources with
   only `N` physical sensors (e.g., 9 sources with 5 sensors via MRA).
   For sonobuoys in a dense biological scene (multiple whales +
   shipping + ambient sources), this is decisive.
5. **Single network across source counts.** Consistent rank sampling
   lets one trained DNN handle 1 through `M-1` sources, eliminating
   the "one DNN per k" weakness of Wu et al. (2022).
6. **Theoretical result.** Chen & Rao *prove* it is possible for a
   DNN to approximate signal subspaces to arbitrary accuracy. Not
   just an empirical claim.

## Limitations

1. **No physics propagation layer.** The method is *explicitly*
   data-driven and geometry-agnostic. It does not embed a
   differentiable propagation model. This is the opposite of what
   the survey described. For underwater applications where
   propagation is range-dependent and frequency-dependent, a
   physics-informed front-end would likely improve performance.
2. **Linear array, narrowband, far-field.** All assumptions are
   classical. Wideband underwater sources, near-field sources,
   and non-linear (e.g., planar or volumetric) array geometries
   are out of scope.
3. **Requires known source count `k`.** All reported results assume
   `k` is given. Real sonobuoys must *estimate* `k` first (source-
   count detection via information-criteria like AIC/MDL, or a
   separate learned head).
4. **Not trained on underwater data.** Training signal is abstract
   complex Gaussian narrowband sources; no cetacean or ship
   signatures; no ocean-acoustic multipath; no thermocline-induced
   refraction.
5. **Large network.** ~11M parameters. Not trivially deployable on
   a sonobuoy itself; must run on a ship/shore processor.
6. **MRA, not arbitrary sparse arrays.** The experiments assume
   minimum-redundancy arrays with a co-array that densely covers
   the virtual ULA. Arbitrary buoy drop positions don't
   automatically form MRAs; some layout optimization or co-array
   interpolation is needed.

## Portable details (for Rust implementation)

### Key equations

**Signal model** (from paper Section II):

```
R₀ = A(θ) P Aᴴ(θ) + η I_M      (virtual ULA covariance)
A(θ) = [a(θ₁), a(θ₂), ..., a(θ_k)]
a(θ) = [1, e^{-j 2π d sin θ / λ}, ..., e^{-j 2π (M-1) d sin θ / λ}]ᵀ
Rₛ = selection(R₀, S)            (SLA covariance)
R̂ₛ = (1/T) Σₜ yₜ yₜᴴ              (sample covariance from snapshots)
```

**Network:**

```
X = WRN_16_8(R̂ₛ) : ℂ^{N×N} → ℂ^{M×M}      (real+imag in 2 channels)
G = X Xᴴ                                    (ℂ^{M×M} Hermitian PSD)
eigendecompose G = U Λ Uᴴ
Ûₛ = U[:, 1..k]                             (signal subspace)
Ûₙ = U[:, k+1..M]                           (noise subspace)
```

**Principal angles:**

```
cos φᵢ = σᵢ(Uₛᴴ Ûₛ)                          (SVD of cross-projection)
```

**Loss (chordal, Frobenius):**

```
L = Σᵢ sin² φᵢ  =  k − ‖Uₛᴴ Ûₛ‖²_F
```

This has a clean equivalent form avoiding SVD:

```
L = k − tr(Pₛ P̂ₛ)     where  Pₛ = Uₛ Uₛᴴ, P̂ₛ = Ûₛ Ûₛᴴ  (projectors)
```

which is `2 × sin² θ_F` — a smooth, differentiable projector distance.

**Root-MUSIC** on `Ûₙ` then finds DoAs. Standard Schmidt formula.

### Hyperparameters (paper-exact)

| Parameter                  | Value                                |
| -------------------------- | ------------------------------------ |
| Backbone                   | WRN-16-8 (no BN, pre-act residual)   |
| Parameter count            | ~11 M                                |
| Input shape                | (2, N, N) = (2, 5, 5) for MRA(5)     |
| Output shape               | (2, M, M) = (2, 10, 10)              |
| Activation                 | ReLU                                 |
| Optimizer                  | SGD + Nesterov momentum              |
| Weight decay               | 0                                    |
| Epochs                     | 50                                   |
| LR schedule                | One-cycle (Smith 2017)               |
| Peak LR (this method)      | ~0.02-0.05 (grid-search)             |
| Peak LR (DCR-T)            | 0.05                                 |
| Peak LR (end-to-end)       | 0.2                                  |
| Loss                       | Chordal / Fubini-Study               |
| Default snapshots `T`      | 50                                   |
| Default SNR                | 20 dB                                |
| Default source count `k`   | varies, per evaluation               |

### Rust/Candle implementation sketch

```rust
// Chordal (projector Frobenius) loss — the recommended variant.
// Works without explicit SVD: only needs projector products.

fn chordal_loss(
    x: &Tensor,       // (B, 2, M, M) — real+imag of learned X
    u_s_target: &Tensor, // (B, M, k) — target signal subspace basis
    k: usize,
) -> Result<Tensor> {
    // X Xᴴ Gram matrix
    let x_complex = complex_from_real_imag(x)?;           // (B, M, M)
    let g = x_complex.matmul(&x_complex.conj_transpose())?;

    // Eigendecompose — top-k eigenvectors
    let (_, u) = hermitian_eig(&g)?;                      // (B, M, M)
    let u_s_hat = u.narrow(-1, M - k, k)?;                // top-k cols

    // Cross-projector product
    // L = k - || U_s^H Û_s ||_F^2
    let cross = u_s_target.conj_transpose().matmul(&u_s_hat)?;
    let norm2 = cross.abs().powf(2.0)?.sum_all()?;
    let loss = Tensor::from_slice(&[k as f32])? - norm2;
    Ok(loss)
}

// Training loop (per batch of identical k via consistent-rank sampling):
for (batch_r_hat, batch_u_s, k) in consistent_rank_batches(&dataset) {
    let x = wrn_16_8.forward(&batch_r_hat)?;
    let loss = chordal_loss(&x, &batch_u_s, k)?;
    optimizer.backward_step(&loss)?;
}
```

**Differentiating through eigendecomposition** is the only subtle
part. Candle and PyTorch both support autograd through `torch.linalg.
eigh` for Hermitian matrices; the gradient formula requires no
degenerate eigenvalues, so we assume generic `k` (nonrepeating).

**Root-MUSIC at inference** is a classical polynomial-rooting step
(Schmidt 1986) — no autograd needed. Can be implemented with any
polynomial root solver (e.g., companion-matrix eigenvalues).

## Sonobuoy integration plan

This paper slots into the **localization / bearing estimation
back-end** for the sonobuoy spatial branch, complementing
(not replacing) the Grinstein GNN-TDOA (paper 4.2).

**Division of labor:**

- **Grinstein GNN-TDOA**: best for *sparse heatmap over 2D ocean
  grid* with unknown source count and drifting buoys. Handles
  distributed, non-linear, drifting arrays. Fast per-inference.
- **Chen & Rao subspace learning**: best for *high-precision DoA*
  when multiple sources are simultaneously present and we need to
  resolve more sources than sensors. Requires known `k` (or a
  separate estimator).

**Where it fits:**

1. **Stage 1 (coarse, fast):** Grinstein GNN-TDOA produces a 2D
   heatmap over the ocean grid. Peak detection → candidate source
   positions and coarse bearing estimates.
2. **Stage 2 (fine, multi-source):** For each cluster of buoys that
   share a line of sight to a region of high heatmap intensity,
   extract the N×N sample covariance at that band. Run Chen & Rao
   subspace learning to get precise bearings for up to `M − 1` sources
   in that direction.
3. **Source-count estimation:** Separate head (can be AIC/MDL on the
   eigenvalues of the learned signal subspace, or a small classifier)
   to choose `k` before stage 2.

**Needed extensions for the sonobuoy context:**

1. **Wideband.** Ocean sources are wideband (whale calls span 10 Hz
   - 20 kHz). Use sub-band decomposition: run Chen & Rao per sub-band,
   then fuse DoAs across bands (incoherent averaging of angular
   spectra or sub-band MUSIC).
2. **Non-linear array.** Sonobuoy fields are 2D, not linear. Two
   options: (a) use a 2D manifold extension of MUSIC + Chen & Rao
   (train on 2D steering matrices `a(θ, φ) = exp(j 2π d · r(θ,φ) / λ)`);
   (b) select virtual linear sub-arrays from the buoy field and apply
   Chen & Rao to each.
3. **Physics-informed propagation layer (the survey's claim).** This
   is *missing* from Chen & Rao. We can add it by pre-processing the
   raw hydrophone snapshots through a differentiable sound-speed-
   profile (SSP) propagator from `pinn-ssp-helmholtz.pdf` (sibling
   agent download). This replaces the `a(θ) = exp(-j 2π d sin θ / λ)`
   steering vector with a ray-traced `a(θ, z_src) = exp(-j 2π ∫ ds/c(z))`.
   Since this is fully differentiable, training still works.
4. **Imperfect array augmentation.** Chen & Rao already tested
   λ/4 position perturbations. For sonobuoys, augment with meter-
   scale GPS noise at the training distribution level.

**Quantum register bridge**: The learned signal subspace `Ûₛ ∈
ℂ^{M×k}` is naturally an amplitude-encoded quantum state — each column
is a unit-norm vector and the columns are orthogonal. This matches the
quantum register structure at `clawft-kernel/src/quantum_register.rs`
precisely. The quantum layer can take `Ûₛ` directly and perform
Grover-style amplitude amplification on the bearing axis, which is a
candidate for a quantum speedup on the "search for k sources in M-angle
grid" step.

**Proposed ADR**: ADR-056 — *Subspace-representation-learning DoA
back-end with physics-informed steering vector for sonobuoy multi-
source resolution.* Must be paired with a source-count estimator
(ADR-057 placeholder).

## Follow-up references

1. **Schmidt, "Multiple emitter location and signal parameter
   estimation,"** IEEE TAP 1986. Canonical MUSIC paper. Required
   reading for the root-MUSIC post-processing step.
2. **Yang, Xie & Zhang, "Sparse and parametric approach (SPA),"**
   IEEE TSP 2014. The SDP-based sparse array DoA baseline that Chen
   & Rao outperform. Relevant because SPA is also convex-optimization-
   based and can serve as a high-quality but slow fallback.
3. **Moffet, "Minimum-redundancy linear arrays,"** IEEE TAP 1968.
   The 5-element MRA `{1, 2, 5, 8, 10}` used in experiments. For
   sonobuoys, MRA design concepts inform how we *lay out* buoys to
   maximize co-array coverage with minimal physical sensors.
4. **Wu, Zhang & Liang, "Learning-based DoA estimation with sparse
   linear arrays,"** IEEE TSP 2022. The DCR-T / DCR-G-Fro / DCR-G-Aff
   baseline Chen & Rao compare against. Represents the previous SOTA
   in DNN sparse-array DoA.
5. **Edelman, Arias & Smith, "The geometry of algorithms with
   orthogonality constraints,"** SIAM J. Matrix Anal. 1998. The
   Grassmannian-geometry reference for principal-angle distances.
   Essential background for understanding *why* the subspace loss
   works.
6. **Zagoruyko & Komodakis, "Wide Residual Networks,"** BMVC 2016 —
   origin of WRN-16-8. Useful if we need to scale the backbone up
   or down for sonobuoy compute constraints.

---

*Generated 2026-04-15. PDF at `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/neural-beamforming-sparse.pdf`. Source: arXiv:2408.16605 (IEEE TSP 2025). Original survey citation (Chen/Wang 2024 TSP with differentiable physics layer) does not match any real paper; Chen & Rao is the closest verifiable substitute.*
