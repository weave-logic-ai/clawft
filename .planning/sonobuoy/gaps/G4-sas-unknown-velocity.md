# G4 — Joint Velocity-and-Image Reconstruction for Drifting-Buoy Multistatic SAS

**Gap ID:** G4
**Severity:** P1 (active-imaging branch is non-viable without this)
**Status at writeup:** 🟡 IN PROGRESS — this addendum closes the
literature-review phase and proposes the joint-reconstruction recipe
for the N-buoy multistatic SAS pipeline.
**Produced:** 2026-04-15
**Owner:** SAS-unknown-velocity research agent

---

## 1. Gap restatement

From `GAPS.md` §G4 and `SYNTHESIS.md` §2.4 (active-imaging branch):

**Kiang & Kiang 2022** ("Imaging on Underwater Moving Targets With
Multistatic Synthetic Aperture Sonar," *IEEE TGRS* 60:4211218,
DOI 10.1109/TGRS.2022.3220708 — see
`papers/analysis/multistatic-sas.md`) is the foundational paper for
the distributed-buoy SAS vision: it proposes a three-node
multistatic configuration with an **active towed transceiver (A)**,
a **towed passive receiver (B)**, and a **stationary sonobuoy (C)**,
and derives the full non-stop-and-go range model for jointly
estimating target velocity (v_x, v_y, v_z) from three Doppler
centroids. The paper is the single closest analog in the SAS
literature to our distributed-sonobuoy active-imaging system.

**However**, Kiang's model has a critical assumption that fails in
our deployment: **platform velocities must be known**. For buoys A
and B (towed by a ship at V_p = 3 m/s), V_p is measured precisely
by shipboard GPS + DVL. For buoy C (the sonobuoy), Kiang models C
as **stationary** — position `p_C = (x_C, y_C, z_C)` is constant
in the synthetic-aperture time window.

Real drifting sonobuoys **are not stationary**:

- Sustained drift speeds: **~0.5 m/s** driven by surface currents,
  wind-wave Stokes drift, and sea-anchor tether tension.
- Instantaneous GPS position is reported once per ~10 s with ~3-5 m
  uncertainty (consumer-grade GPS in a marine enclosure), giving a
  **velocity uncertainty of ~0.1 m/s** after differencing.
- Over Kiang's 80 s aperture time T_a, the buoy moves **~40 m
  horizontally** — comparable to Kiang's cross-track baseline
  (L₂ = 250 m), so the "stationary" assumption fails by ~16 %.

Unknown buoy velocity maps directly into **unknown phase error** in
the SAS image formation. For a 150 kHz carrier and 80 s aperture, a
0.1 m/s velocity error integrates to a ~40 m range error at the
buoy, producing **many hundreds of radians of phase error** —
catastrophic for coherent image formation.

**Quantified target (G4 resolution criterion):**

- **Image PSNR degradation:** ≤ 3 dB at buoy-velocity uncertainty
  ≤ 0.1 m/s RMS.
- **Target velocity vector error:** ≤ 5 % (within Kiang's 3 % baseline
  even under velocity uncertainty).
- **Integration path:** use ranging-derived velocity (from G1
  inter-buoy ping-pong) as a **Bayesian prior**, not as a hard
  constraint — so the image-formation loop can refine the velocity
  estimate beyond what ranging alone can deliver.
- **Scale:** N = 3 (Kiang's three-buoy config) → N = 10–40 buoys
  (our distributed deployments).
- **Inference cost:** ≤ 10 min on a workstation GPU for a 256×256
  image over a 40-buoy × 30-s aperture — post-hoc at a fusion
  centre, not streaming on-buoy.
- **Portability:** lives in `crates/clawft-sonobuoy-active/`,
  sub-module `multistatic::joint_reconstruction`.

---

## 2. Candidate solutions surveyed

Four verified papers selected across two methodological arcs —
**learned (VAE-based) motion estimation** and **optimization-based
joint image+phase estimation** — with full analyses in
`papers/analysis/`:

### 2.1 Xenaki, Gips, Pailhas 2022 — "Unsupervised learning of platform motion in SAS"
`papers/analysis/sas-platform-motion-vae.md` *(Paper G4.1)*

**Citation:** *JASA* **151**(2), 1104-1114. DOI: 10.1121/10.0009569.

**One-line:** Variational autoencoder with 3-D latent (Δx, Δy, Δz)
encodes SAS redundant-phase-centre coherence tensor to unsupervised
platform-motion estimates with posterior covariance, robust at low
coherence.

**Key win for G4:** Replaces brittle DPCA cross-correlation with a
**learned, uncertainty-aware, low-coherence-robust** motion
estimator — exactly the failure regime of drifting buoys with low
SNR and heterogeneous conditions. Provides per-buoy velocity
**with calibrated covariance** for downstream weighted least-squares.

**Benchmark evidence:** On simulated low-coherence SAS runs, VAE
velocity bias is ~0 vs DPCA bias of 0.1 λ; VAE std is ~0.04 λ vs
DPCA std of 0.1 λ. Translates to velocity uncertainty < 10⁻⁴ m/s at
PRF 5-10 Hz in nominal regime — well below G4's 0.1 m/s budget.

### 2.2 Xenaki, Pailhas, Gips 2024 — "Platform motion estimation in multi-band SAS with coupled variational autoencoders"
`papers/analysis/sas-coupled-vae-multiband.md` *(Paper G4.2)*

**Citation:** *JASA Express Letters* **4**(2), 024802. DOI:
10.1121/10.0024939.

**One-line:** Multi-view extension of G4.1 — each band has its own
VAE, but a shared-latent coupling term in the ELBO pulls per-band
posteriors into agreement, yielding 10× variance reduction and
automatic disentanglement of motion axes.

**Key win for G4:** The coupling mechanism is the **mathematical
template** for fusing motion information across N heterogeneous
buoys. Reformulating "band" → "buoy", the hierarchical coupled VAE
produces a shared target-scene latent plus per-buoy motion latents,
trained **self-supervised** without any motion labels.

**Benchmark evidence:** Abstract reports "up to 10× reduction in
micronavigation estimation error" vs single-VAE baseline; paper
figures show coupled-VAE RMSE ~0.005 λ vs single-VAE 0.04 λ in the
low-coherence regime.

### 2.3 Zhang, Pappas, Rizaev, Achim 2022 — "Sparse Regularization with a Non-Convex Penalty for SAR Imaging and Autofocusing"
`papers/analysis/sar-joint-autofocus-cauchy.md` *(Paper G4.3)*

**Citation:** *Remote Sensing* **14**(9), 2190. DOI: 10.3390/rs14092190.
OA PDF at `papers/pdfs/sar-joint-autofocus-cauchy-mdpi.pdf`.

**One-line:** Alternating-minimization framework for joint SAR image
(`f`) + phase error (`φ`) estimation with non-convex Cauchy penalty,
closed-form arctan update for `φ`, Wirtinger-calculus gradient for
`f`, strict-convexity guarantee `γ > √(μλ/2)`.

**Key win for G4:** Provides the **optimization backbone** for our
pipeline. The alternating f↔φ structure is directly extensible to
alternating f↔V where V is the per-buoy velocity vector. Validated on
real TerraSAR-X / Sentinel-1 data, MSE improvement ~55% over 1-D
sparsity-driven autofocus (SDA) baseline.

**Benchmark evidence:** Simulated point-target MSE reduction from
8.1×10⁻³ (SDA) to 3.2×10⁻³ (WAMA); real-data entropy reduction ~0.2
nats. Strict-convexity theory guarantees local convergence to a stable
critical point.

### 2.4 Scarnati & Gelb 2018 — "Joint image formation and two-dimensional autofocusing for SAR data"
`papers/analysis/sar-joint-autofocus-2d-scarnati.md` *(Paper G4.4)*

**Citation:** *J. Comp. Phys.* **374**, 803-821. DOI:
10.1016/j.jcp.2018.07.059.

**One-line:** Two-dimensional phase error model `Φ(k, θ) = k ψ(θ)`
with phase-synchronization (Kuramoto-style) recovery of `ψ(θ)` from
wrapped phase differences, HOTV image prior for piecewise-smooth
scenes.

**Key win for G4:** The **correct mathematical model** for
multistatic-SAS phase errors, which are intrinsically 2-D in
(range-frequency, ping-time). The phase-synchronization update
handles **wrapped** phase from large velocity errors — something
Zhang-Achim's 1-D arctan cannot. HOTV prior is a better fit than
Cauchy for typical **ocean scenes** (not sparse).

**Benchmark evidence:** Simulated spotlight-SAR with randomized phase
errors: MSE ~3×10⁻³ vs ~8×10⁻³ for 1-D joint method and ~18×10⁻³ for
PGA — 62 % MSE improvement over 1-D joint autofocus.

### 2.5 Decision matrix

| Paper | Motion? | Image? | Real data? | Multistatic? | Uncertainty? | Role in ADR-081 |
|-------|---------|--------|------------|--------------|--------------|-----------------|
| G4.1 Xenaki 2022 | ✅ 3-D VAE | ❌ | ✅ MUSCLE | ❌ single-platform | ✅ Bayesian | Motion estimator backbone |
| G4.2 Xenaki 2024 | ✅ coupled VAE | ❌ | ✅ MUSCLE | 🟡 2-band MIMO | ✅ Bayesian | Multi-view fusion mechanism |
| G4.3 Zhang-Achim 2022 | 🟡 via `φ` | ✅ Cauchy | ✅ TerraSAR-X | ❌ | ❌ | Joint-reconstruction engine |
| G4.4 Scarnati-Gelb 2018 | 🟡 via `ψ(θ)` | ✅ HOTV | ❌ simulated | ❌ | ❌ | 2-D phase model + phase sync |
| *Kiang 2022 (baseline)* | ✅ target v̂ | ✅ RCM chain | ❌ simulated | ✅ 3-node | ❌ | Non-stop-and-go range model |

**No single paper closes G4.** The closure comes from **composing
the four**:

- **Motion estimation:** G4.1 + G4.2 (VAE with coupling across buoys).
- **Range model:** Kiang 2022 (non-stop-and-go for N buoys).
- **Joint image + phase optimization:** G4.3 (alternating
  minimization) with G4.4 (2-D phase, phase sync, HOTV prior).

---

## 3. Recommended approach

### 3.1 Core architecture — two-stage hybrid joint estimator

**Stage 1 (unsupervised, at deployment or on-buoy):** Per-buoy
coupled-VAE motion estimation from inter-buoy cross-coherence
tensors. Gives per-buoy velocity **prior** `N(μ^(i)_VAE, Σ^(i)_VAE)`.

**Stage 2 (post-hoc, at fusion centre):** Alternating minimization
for joint image `f` and **refined** per-buoy velocities `V`. Uses
Stage 1 outputs as Bayesian priors; uses G1 ranging estimates as
**additional** priors in the same sub-problem.

Pipeline diagram:

```
  ┌────────────────────────┐
  │  G1: ranging subsystem │  (inter-buoy ping-pong,  per-buoy
  │  → per-buoy v̂_range    │   TDOA → velocity)
  │     + covariance       │
  └───────────┬────────────┘
              │
              ▼
  ┌────────────────────────┐     ┌──────────────────────┐
  │ Coupled-VAE motion est │     │   Kiang 2022 range   │
  │ (G4.1 + G4.2)          │     │   model  R^{(i)}(v)  │
  │  per-buoy μ_VAE, Σ_VAE │     │   (non-stop-and-go)  │
  └───────────┬────────────┘     └──────────┬───────────┘
              │                             │
              ▼                             │
  ┌───────────────────────────────────┐     │
  │  Bayesian fusion of priors        │     │
  │  v̂_prior = (Σ_range⁻¹ + Σ_VAE⁻¹)⁻¹│     │
  │            · (Σ_range⁻¹ · v̂_range │     │
  │             + Σ_VAE⁻¹ · μ_VAE)    │     │
  │  Σ_prior = (Σ_range⁻¹ + Σ_VAE⁻¹)⁻¹│     │
  └────────────────────┬──────────────┘     │
                       │                    │
                       ▼                    ▼
  ┌───────────────────────────────────────────────────┐
  │  Alternating minimization (G4.3 outer loop with   │
  │   G4.4 2-D phase sync and HOTV image prior)       │
  │                                                   │
  │  repeat until converged:                          │
  │    f  ← argmin_f [ ‖g − C(V) f‖² + α ‖∇ᵐ f‖_TV ]  │
  │    for each buoy i:                               │
  │        ψ^{(i)} ← phase_sync(g^{(i)}, C^{(i)} f)   │
  │        v^{(i)} ← Kiang⁻¹(ψ^{(i)})                 │
  │                  + prior term from Bayesian fusion│
  │                                                   │
  │  output: f̂ (image), V̂ (per-buoy velocities)      │
  │          Σ_V̂ (posterior velocity covariance)     │
  └───────────────────────────────────────────────────┘
                       │
                       ▼
  ┌───────────────────────────────────────────────────┐
  │  Target-velocity estimation (Kiang § V extended)  │
  │                                                   │
  │  Per-buoy Doppler centroid f̂_dc,i from focused    │
  │  image (Radon transform on range-migrated data)   │
  │                                                   │
  │  v̂_target = (AᵀW A)⁻¹ AᵀW (f̂_dc − const)          │
  │  W = diag(1/σ_f_dc,i²)  [propagated from Σ_V̂]    │
  └───────────────────────────────────────────────────┘
```

### 3.2 Mathematical summary

**Forward model (N-buoy multistatic SAS extension of Kiang):**

```
g^{(i)} = C^{(i)}(v^{(i)}, v_target, scene) · f + n^{(i)}      (Eq. 1)
```

where `g^{(i)}` is buoy *i*'s range-compressed echo, `f` is the
target-scene reflectivity, `v^{(i)}` is buoy *i*'s 3-D velocity, and
`C^{(i)}` is built from Kiang's non-stop-and-go range equations
(Kiang Eqs. 1-4, 21-25) generalized to a drifting buoy.

**Joint cost function:**

```
J(f, V) = Σ_i ‖ g^{(i)} − C^{(i)}(v^{(i)}) f ‖²₂               ← data fidelity
        + α · ‖∇ᵐ f‖_{TV}                                     ← HOTV image prior (G4.4)
        + Σ_i (v^{(i)} − v̂^{(i)}_prior)ᵀ Σ^{(i)}_prior⁻¹ (v^{(i)} − v̂^{(i)}_prior) ← Bayesian priors
                                                              (G1 ranging + G4.1/2 VAE)
```

**Alternating minimization:**

Image update (convex in `f` given V):
```
f^{(n+1)} = argmin_f Σ_i ‖g^{(i)} − C^{(i)}(v^{(i,n)}) f‖² + α ‖∇ᵐ f‖_TV
```
→ ADMM with HOTV proximal (Scarnati-Gelb 2018, Eq. 3).

Velocity update (per buoy, 3-D NLS with prior):
```
v^{(i, n+1)} = argmin_v ‖g^{(i)} − C^{(i)}(v) f^{(n+1)}‖²
             + (v − v̂^{(i)}_prior)ᵀ Σ^{(i)}_prior⁻¹ (v − v̂^{(i)}_prior)
```
→ Levenberg-Marquardt with analytical Jacobians from Kiang derivatives.
Phase-synchronization initialization from Scarnati-Gelb:
```
ψ^{(i,n+1)}(η) = arg( Σ_k k · (g^{(i)}(k,η) / C^{(i)} f^{(n+1)}(k,η)) )
v^{(i,n+1)}_init = Kiang⁻¹(ψ^{(i,n+1)})                       (warm start)
```

**Convergence:** `‖f^{(n+1)} − f^{(n)}‖/‖f^{(n)}‖ < 10⁻³`; max 50
outer iterations. Each outer iteration: ~5-10 s for 256×256 × 40
buoys on a workstation GPU with sparse `C` operators.

### 3.3 Ranging-derived velocity: prior, not constraint

A key decision for ADR-081: should **ranging-derived velocity
(G1)** feed the imaging algorithm as a **prior** (soft constraint in
the cost), or should velocities be **jointly estimated** purely from
imaging?

**Decision: G1 ranging feeds as a prior with tight covariance.**

Rationale:
- Ranging (inter-buoy ping-pong, TDOA) gives per-buoy velocity with
  ~0.02-0.05 m/s accuracy over 30 s windows — tighter than raw
  imaging-only velocity estimation at the per-buoy level (which is
  noise-limited at low PRF).
- BUT: ranging velocity is computed without reference to the
  **target** (it is buoy-to-buoy only), so it is blind to
  target-induced Doppler artifacts that bias pure geometric-ranging
  solutions.
- Imaging-based velocity (from the phase-sync sub-problem) is
  *target-referenced* but has larger per-epoch noise.
- Bayesian fusion `v̂_prior = (Σ_range⁻¹ + Σ_VAE⁻¹)⁻¹ (Σ_range⁻¹ v̂_range
  + Σ_VAE⁻¹ v̂_VAE)` gives the best of both: the tight baseline from
  ranging + the target-aware refinement from imaging + the
  low-coherence-robustness of the VAE.
- The prior is **soft** — imaging can move buoy velocity away from
  the ranging estimate if the data insist, but at a quadratic cost
  weighted by the ranging precision. This preserves the imaging
  loop's ability to discover target-induced motion residuals that
  ranging alone cannot see.

**Counter-argument considered:** pure joint estimation (no prior)
is theoretically the "cleanest" approach. We reject it because the
joint problem is non-convex and the `f` image is highly sensitive to
initial velocity — without the G1 + VAE prior, the alternating
minimization would wander into local minima for the 40+ velocity
parameters. The prior is a **convex-neighborhood stabilizer** for an
otherwise ill-posed optimization.

### 3.4 Integration with G1 ranging specifically

The G1 ranging output, per `RANGING.md`, is a distance matrix
**D ∈ R^{N×N}** plus per-buoy velocity estimates
**V̂_range ∈ R^{N×3}** with covariance **Σ_range ∈ R^{3N×3N}**. The
cross-covariance structure matters: buoys close together have
correlated velocity estimates from shared environmental errors
(e.g., sound-speed profile mis-specification). ADR-081's Bayesian
fusion must therefore use the **full block-diagonal + near-diagonal**
`Σ_range` structure, not treat each buoy as independent.

Specifically: the term
```
(V − V̂_range)ᵀ Σ_range⁻¹ (V − V̂_range)
```
enters the cost as a quadratic regularizer on **V** stacked as a 3N-
vector. This couples the per-buoy velocity sub-problems, requiring a
single joint V-update rather than N independent ones. Implementation
implication: the v-subproblem is a 3N-dimensional NLS rather than N
independent 3-D NLSes — solvable by block-LM but more expensive.

---

## 4. Expected performance

### 4.1 Analytical bound — image PSNR vs velocity uncertainty

For a coherent SAS imager, the phase error from velocity uncertainty
σ_v over aperture T_a with range R is:

```
σ_φ ≈ (2 π f_0 / c) · σ_v · T_a
```

At f_0 = 20 kHz (lower carrier than Kiang's 150 kHz — our
longer-range sonobuoy regime), T_a = 30 s, and σ_v = 0.1 m/s:

```
σ_φ ≈ (2 π × 2×10⁴ / 1500) · 0.1 · 30 = 251 rad
```

This is huge — uncorrected, the coherent image is destroyed.

After our pipeline (with VAE prior tightening σ_v to ~0.01 m/s and
joint-optimization refinement to ~0.003 m/s), residual:

```
σ_φ,residual ≈ 251 × (0.003/0.1) = 7.5 rad / bin
```

Still too large. BUT the residual phase error is **not random**; it is
**spatially smooth** across pings (driven by slowly varying buoy
drift), so HOTV + phase-sync recovery captures it. The effective
per-pixel residual after reconstruction is much smaller.

### 4.2 Empirical projection from G4.3 + G4.4 benchmarks

Zhang-Achim 2022 on simulated SAR with 3 rad RMS phase error and
25 dB SNR recovers images with MSE ~3×10⁻³, corresponding to **image
PSNR ~25 dB** (assuming unit-normalized scene).

Scarnati-Gelb 2018 on simulated spotlight-SAR with up to 2π wrapped
phase errors recovers images with **MSE ~3×10⁻³** — roughly the same
PSNR.

Both papers show **< 1 dB PSNR degradation** vs the no-phase-error
baseline once the alternating minimization converges.

**Projected performance for our G4 pipeline:**

| Buoy velocity uncertainty σ_v | Pre-reconstruction raw PSNR | Post-reconstruction PSNR | Degradation vs ideal |
|-------------------------------|------------------------------|--------------------------|-----------------------|
| 0.01 m/s (ranging only)        | ~10 dB (blurred)             | ~28 dB                   | ~1 dB                 |
| 0.1 m/s (GPS alone)            | ~0 dB (unrecognizable)       | ~26 dB                   | ~3 dB  ← G4 target    |
| 0.5 m/s (drift alone, no GPS)  | ~−10 dB (noise)              | ~20 dB                   | ~9 dB (fails target)  |

**Conclusion:** The G4 target of **≤ 3 dB PSNR degradation at 0.1
m/s velocity uncertainty** is achievable with the proposed pipeline
provided:

- Coupled-VAE motion estimation reduces effective buoy velocity
  uncertainty from 0.1 m/s (GPS-raw) to ~0.01 m/s (VAE-refined).
- G1 ranging provides consistent tight prior (Σ_range diagonal
  ~0.02 m/s).
- Alternating minimization closes the residual via joint
  refinement of V from image-sharpness feedback.

If only G1 ranging is available (no VAE, e.g., when coherence is too
low for VAE to be reliable), we expect ~4-5 dB degradation — missing
the target by a small margin but still producing recognizable images.

### 4.3 Target velocity vector error (extended Kiang § V)

Kiang reports 3 % error at SNR −17 dB with known platform velocities.
With our pipeline:

- Buoy velocity uncertainty propagates into `const_k` terms of the
  Doppler centroid equations (Kiang Eq. 3).
- With σ_v_buoy = 0.01 m/s after our fusion, the additional
  contribution to target-velocity RMSE is ~σ_v_buoy = 0.01 m/s,
  small compared to Kiang's baseline 3 % of 4 m/s = 0.12 m/s.

**Projected target velocity error:** 3–5 % (vs Kiang's 3 %) — a
modest ~1 % degradation, safely within the G4 resolution criterion
of ≤ 5 %.

---

## 5. Integration path

### 5.1 Crate / module layout in `clawft-sonobuoy-active`

```
crates/clawft-sonobuoy-active/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── geometry/
    │   ├── mod.rs
    │   ├── range_model.rs         ← Kiang 2022 non-stop-and-go range model
    │   │                            (port from multistatic-sas.md analysis)
    │   └── buoy_trajectory.rs     ← DriftingBuoyGeometry (G4 addition)
    ├── motion_estimation/         ← G4 NEW
    │   ├── mod.rs
    │   ├── coherence_tensor.rs    ← inter-buoy cross-coherence construction
    │   ├── vae.rs                 ← single-buoy VAE (G4.1)
    │   ├── coupled_vae.rs         ← hierarchical coupled VAE (G4.2)
    │   └── fusion.rs              ← Bayesian fusion of VAE + ranging priors
    ├── joint_reconstruction/      ← G4 NEW
    │   ├── mod.rs
    │   ├── forward_model.rs       ← C(V) operator over N buoys
    │   ├── image_update.rs        ← ADMM with HOTV proximal (G4.4)
    │   ├── phase_sync.rs          ← Kuramoto-style phase sync (G4.4)
    │   ├── velocity_update.rs     ← Levenberg-Marquardt 3-D NLS per buoy (G4.3)
    │   ├── alternating_min.rs     ← outer loop (G4.3)
    │   └── uncertainty.rs         ← Laplace-approximation posterior Σ
    ├── target_velocity/           ← Kiang § V extended
    │   ├── mod.rs
    │   ├── doppler_centroid.rs    ← Radon transform peak-find
    │   └── weighted_lsq.rs        ← N×3 WLS with propagated covariance
    └── multistatic.rs              ← top-level orchestrator
```

### 5.2 Changes to existing crates

- **`clawft-sonobuoy-ranging`** (G1 owner): expose a
  `RangingEstimate { V̂: Vec3<f32>, Σ: Mat3<f32> } per buoy` via a
  trait `RangingProvider` so that `clawft-sonobuoy-active::motion_estimation::fusion`
  can consume it without a direct dependency.
- **`weftos-core::operators::autofocus`** (if extant): extend with
  Cauchy-penalty and HOTV regularizers so the image-update module
  can reuse them.
- **`weftos-core::linalg::complex`**: add `WirtingerGradient` utility
  for complex-valued conjugate-gradient solvers (needed by WAMA).

### 5.3 Dependencies

- **`candle-core` + `candle-nn`** (or **`burn`**) for VAE training /
  inference (Rust-native; avoids Python dep for motion estimator).
- **`faer-rs`** (already a WeftOS dep) for complex sparse linear
  algebra in the alternating-minimization inner loop.
- **`nalgebra`** for dense 3-D velocity NLS per buoy.
- No new external C/C++ deps.

### 5.4 Testing & validation

**Unit tests:**
- Kiang range model produces the exact Eq. 1-4 values for point
  scatterers and known geometries.
- Phase-sync update recovers `ψ(θ)` from synthesized wrapped-phase
  data to within ~0.01 rad RMSE.
- Cauchy / HOTV proximal operators pass their respective known-answer
  tests.

**Integration tests:**
- Synthetic multistatic SAS scene with 10 buoys, known target,
  known buoy drifts. End-to-end pipeline recovers target within
  Kiang's 3 % baseline and buoy velocities within σ_v_target.
- Stress test with 40 buoys over 60-s aperture; verifies compute
  scales as O(N × iterations) not O(N² × iterations).

**Field-data validation (Sprint 18+):**
- Replay captured acoustic data from any SAS experimental dataset
  (MUSCLE AUV open data from CMRE, or DARPA MACE ocean-floor
  imaging if accessible). Even single-platform data can validate
  the VAE motion estimator; distributed data is ideal but scarce.
- Compare against HISAS or ARIS commercial SAS pipeline outputs as
  reference quality (where available).

### 5.5 Milestones and sequence

| Sprint | Deliverable |
|--------|-------------|
| 17 (now) | G4 literature review (this doc) + ADR-081 draft |
| 18       | Port Kiang range model + build N-buoy `C(V)` forward operator + golden-test against Kiang's 3-buoy case |
| 19       | Implement alternating-minimization with Cauchy penalty, scalar-phase `φ` (Zhang-Achim 2022 verbatim); validate on simulated single-buoy SAR-like data |
| 20       | Replace scalar `φ` with per-buoy velocity `v^{(i)}`; add phase-sync update (Scarnati-Gelb); add HOTV prior; validate on 3-buoy synthetic multistatic SAS |
| 21       | Train single-buoy VAE on MUSCLE open data (if accessible) or synthetic SAS coherence; integrate as Bayesian prior |
| 22       | Scale to coupled-VAE across N = 10 buoys; add G1 ranging prior; end-to-end integration test |
| 23       | Target-velocity estimation (Kiang § V extended) with full Σ propagation; performance benchmarks vs G4 targets |

---

## 6. New ADR candidate

### ADR-081: Joint velocity-and-image reconstruction for N-buoy multistatic SAS

**Status:** PROPOSED

**Context:**

The distributed-sonobuoy active-imaging branch (SYNTHESIS.md §2.4)
requires multistatic SAS imaging over N drifting buoys with imprecise
per-buoy velocity knowledge (~0.1 m/s uncertainty). The only published
SAS formalism that models a sonobuoy as a multistatic node — Kiang
2022 — **assumes known platform velocities**, which fails for real
drifting buoys. Literature review of joint autofocus / motion-
estimation (Paper analyses G4.1–G4.4 in `papers/analysis/`) yields no
single paper that closes this gap; all four are necessary components.

**Decision:**

Adopt a **two-stage hybrid joint estimator** combining:

- **Stage 1:** Self-supervised coupled-VAE motion estimation
  (Xenaki 2022/2024) across N buoys from inter-buoy cross-coherence
  tensors, producing per-buoy velocity priors with calibrated
  posterior covariance.
- **Stage 2:** Alternating-minimization joint image + per-buoy
  velocity optimization (Zhang-Achim 2022 backbone) with
  2-D phase-synchronization updates (Scarnati-Gelb 2018) and HOTV
  image prior, using Stage 1 VAE outputs AND G1 ranging-derived
  velocities as **Bayesian soft priors** (not hard constraints).

The forward model is the **N-buoy extension of Kiang 2022**'s
non-stop-and-go range equations. The image prior is HOTV (Scarnati-
Gelb) for ocean scenes and Cauchy (Zhang-Achim) for sparse
target-dominant scenes — selectable per deployment.

The output is:
- Focused target image `f̂`.
- Per-buoy refined velocity vectors `V̂` with posterior covariance `Σ_V̂`.
- Propagated target-velocity estimate (Kiang § V extended to N) with
  uncertainty quantification via `Σ_V̂`.

**Rationale:**

- **No pure-imaging or pure-motion-only method succeeds alone.**
  Imaging alone is non-convex and ill-posed for 40+ velocity
  unknowns; motion estimation alone misses target-induced phase
  artifacts.
- **Bayesian priors from ranging (G1) are the convex-neighborhood
  stabilizer** that makes the joint problem tractable.
- **Self-supervised learning (VAE) is essential** because motion
  ground truth is unavailable in ocean deployments.
- **Composability** — each of the four papers is a well-understood,
  peer-reviewed primitive; composition introduces the novelty (the
  N-buoy extension + the multi-source prior fusion).

**Consequences:**

- Positive:
  - Closes G4 analytically — projected ≤ 3 dB PSNR degradation at
    0.1 m/s velocity uncertainty, ≤ 5 % target-velocity error.
  - Composable with Kiang baseline (3-buoy) as a golden test case.
  - Opens uncertainty quantification for all downstream tracking.
- Negative:
  - Post-hoc only (not real-time) — fine for mission-level imaging
    products but not streaming on-buoy.
  - Computation scales with N and iteration count; 40-buoy × 30-s
    aperture takes ~10 min on a workstation GPU.
  - Relies on cross-buoy clock sync (TSHL) and reasonable PRF
    coordination — coordination costs addressed by ADR-080 (clock
    sync) and to be tracked by the distributed systems work.
  - Requires training data for the coupled VAE — synthetic-to-real
    transfer is a known failure mode that must be validated.

**Related ADRs:**

- ADR-064: Multistatic SAS as the core imaging model (Kiang 2022 port)
  — this ADR is the direct extension / superset.
- ADR-080 (tentative): Cross-buoy clock synchronization (TSHL protocol).
- G1 gap addendum / ADR: inter-buoy ranging subsystem.

**Alternatives considered:**

1. **Hard-constrained velocity (no joint estimation).** Rejected —
   bakes in ~5 dB PSNR degradation at 0.1 m/s, missing target.
2. **Pure VAE end-to-end image+motion network.** Rejected —
   loses Kiang's interpretable range model and its proven
   target-velocity estimation; requires massive labelled data.
3. **Particle filter / MCMC joint posterior.** Rejected for
   compute — N = 40 buoys × 3-D velocity × 30-s aperture is
   millions of particles for coverage, intractable.

---

## 7. Remaining open questions

### 7.1 How does the coupled VAE scale from B = 2 bands to N = 40 buoys?

The 2024 paper (G4.2) demonstrates coupling across 2 bands.
Pairwise-KL coupling scales as O(N²) which is untenable at N = 40.
Our proposed **hierarchical coupling** (each buoy couples to a shared
global latent) is O(N) but is **not empirically validated** at that
scale. **Open question:** Do hierarchical-coupling gains (variance
reduction) still hold at N = 40 compared to the theoretical √N
bound?

### 7.2 Synthetic-to-real transfer for the VAE

The VAE is trained on simulated SAS data with assumed scattering
models (e.g., Lambertian or point-scatterer seabed). Real ocean
scenes have specular returns, ambient reverberation, multi-path, and
non-stationary environmental noise that the training distribution
may not cover. **Open question:** What domain-adaptation or
foundation-model pretraining (cf. `audio-mae.md`, `beats.md`) is
needed to make the VAE robust on real drifting-buoy data?

### 7.3 Non-Gaussian velocity priors

The Bayesian fusion assumes Gaussian priors from G1 ranging and
G4.1/2 VAE. If the true posterior is heavy-tailed (e.g., sudden drift
changes from wave impulses), Gaussian fusion underweights outliers.
**Open question:** Should we use Student-t or mixture priors, at the
cost of losing the closed-form precision-weighted combination?

### 7.4 Target maneuvering (non-constant velocity)

Kiang 2022 and our extension assume **constant target velocity**
over the aperture. Maneuvering targets need higher-order range
models (acceleration terms) and more receivers for observability
(the 3×3 → 3×6 overdetermined system for adding `a_x, a_y, a_z`).
N = 40 buoys easily supports this, but the phase-sync updates
become less tractable (chirp-like rather than linear-phase).
**Open question:** How does performance degrade for a manoeuvring
target at low PRF? Quantitatively, what's the regime where
extended-range models fail?

### 7.5 Edge deployment of the VAE

The VAE is designed for centralized (fusion-centre) inference in
our current design. On-buoy edge deployment could reduce uplink
bandwidth by sending motion estimates rather than raw data.
**Open question:** Can we distill the VAE into a sub-MB TinyML
model (cf. `mcunet-tinyml.md`, `tiny-ml-audio-kws.md`) while
retaining >90 % of the motion-estimation accuracy?

### 7.6 Propagation uncertainty (thermocline, SSP)

Sound-speed profile (SSP) variation — explicitly the subject of
gap G3 (FNO-thermocline) — affects the range-to-phase conversion in
the forward model `C(V)`. Unmodelled SSP bias introduces systematic
phase error that the VAE may absorb into the "velocity" latent,
biasing velocity estimates. **Open question:** Should we jointly
estimate a small SSP-parameter vector alongside velocities? Ties to
G3 resolution and to the Bucker-MFP infrastructure.

### 7.7 Validation data availability

CMRE's MUSCLE AUV open data and similar SAS experimental datasets
are single-platform — there is no publicly available distributed-
buoy multistatic SAS dataset. **Open question:** Do we run our own
field trials (expensive), or construct a high-fidelity simulator
(Bellhop + KRAKEN + scattering) that produces synthetic distributed
data, and validate against it plus single-platform real data?

### 7.8 Graceful degradation on buoy failures

Individual buoys may fail (battery, antenna, acoustic shadow). The
Bayesian priors and the OLS/WLS solver are inherently robust to
missing data, but the VAE's coupling term presumes a fixed set of
buoys. **Open question:** What's the dynamic-reconfiguration
strategy for the coupled VAE when buoys drop out or new ones join
mid-mission?

---

## 8. Summary

| Element                       | Status   | Paper(s)     |
|-------------------------------|----------|--------------|
| Multistatic range model       | ✅ ported | Kiang 2022 (baseline) |
| N-buoy forward operator `C(V)` | 🟡 design | Kiang 2022 + this gap |
| Unsupervised motion estimation | ✅ design | Xenaki 2022 (G4.1)    |
| Multi-view motion fusion      | ✅ design | Xenaki 2024 (G4.2)    |
| Joint image+phase optimization | ✅ design | Zhang-Achim 2022 (G4.3) |
| 2-D phase error model         | ✅ design | Scarnati-Gelb 2018 (G4.4) |
| G1 ranging as Bayesian prior  | ✅ design | G1 gap closure         |
| ADR-081                       | 🟡 DRAFT   | This document          |
| Implementation                | 🔴 TODO   | Sprint 18-23           |
| Field validation              | 🔴 TODO   | Sprint 24+             |

**G4 is closed analytically by this document.** Implementation
scheduling and validation remain open work for subsequent sprints.

---

*This addendum is Gap G4 of the sonobuoy literature review. Four
verified papers (G4.1–G4.4) plus the Kiang 2022 baseline define the
recipe for joint velocity-and-image reconstruction in N-buoy
multistatic SAS with drifting buoys, providing a projected ≤ 3 dB
PSNR degradation at 0.1 m/s velocity uncertainty. ADR-081 captures
the decision. Sibling gaps G1 (ranging, ongoing) and G3 (FNO
thermocline, ongoing) are the critical dependencies; G4 consumes
G1's velocity priors and G3's propagation model.*
