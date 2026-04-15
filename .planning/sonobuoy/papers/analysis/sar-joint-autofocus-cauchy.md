# Paper G4.3 — Zhang, Pappas, Rizaev & Achim 2022, "Sparse Regularization with a Non-Convex Penalty for SAR Imaging and Autofocusing"

## Citation

Zhang, Z.-Y., Pappas, O., Rizaev, I. G., & Achim, A. (2022). "Sparse
Regularization with a Non-Convex Penalty for SAR Imaging and
Autofocusing." *Remote Sensing*, **14**(9), 2190.

- **DOI:** [10.3390/rs14092190](https://doi.org/10.3390/rs14092190)
- **Open-access PDF:** https://www.mdpi.com/2072-4292/14/9/2190
- **Preprint (earlier version):** arXiv:2108.09855,
  https://arxiv.org/abs/2108.09855 (2021).
- **Sister paper (shorter, Wirtinger focus):** Zhang, Z.-Y., Pappas, O.,
  Achim, A. (2020). "SAR Image Autofocusing using Wirtinger calculus
  and Cauchy regularization." arXiv:2012.09772 / IEEE ICIP 2021.
- **Authors' affiliations:** Visual Information Laboratory, University
  of Bristol, UK (Achim is the senior author).

## Status

**Verified.** Citation cross-checked via MDPI landing page,
arxiv.org/abs/2108.09855, and the GitHub repository
`oktaykarakus/CauchySAR-v1.0` which implements the Cauchy-SAR
framework that this paper extends. The MDPI open-access PDF (10
pages, 1.6 MB) downloaded and parsed cleanly. Published April 28,
2022, in *Remote Sensing* vol. 14 issue 9 special issue on SAR Image
Processing. DOI resolves. All equations below extracted from the
open-access HTML version of the article.

**PDF:** `.planning/sonobuoy/papers/pdfs/sar-joint-autofocus-cauchy-mdpi.pdf`
(10 pages, 1.6 MB) + `.planning/sonobuoy/papers/pdfs/sar-joint-autofocus-cauchy.pdf`
(arXiv preprint, 7 pages, 680 KB).

## One-paragraph summary

This paper formulates SAR image reconstruction and motion/phase-error
compensation as a **single inverse-problem optimization** rather than
the classical two-stage pipeline of "image first, autofocus after". The
forward model is written as `g = C(φ) f + n`, where `g` is the raw
phase-history measurements, `f` is the unknown complex SAR image,
`C(φ)` is a **phase-error-parametrized observation operator**
(`C_m(φ) = e^{jφ_m} C_m`), and `n` is Gaussian noise. The authors'
core contribution is a **joint minimization** over both `f` and `φ`
with a **non-convex Cauchy penalty** enforcing image sparsity:
`J(f, φ) = ‖g − C(φ) f‖²₂ − λ Σ_i ln(γ / (γ² + |f_i|²))`. They solve
this non-convex, non-smooth problem via **alternating minimization**:
the image sub-problem `f` is solved by either complex forward-backward
splitting (CFBA) or Wirtinger-calculus alternating minimization
(WAMA), and the phase-error sub-problem `φ` has a **closed-form
arctangent update** per range bin. Convergence is guaranteed on the
image sub-problem under the condition `γ > √(μλ/2)` (strict convexity),
and the method is benchmarked on simulated point-target scenes,
Pierson-Moskowitz sea-surface scenes, and **real** TerraSAR-X and
Sentinel-1 data — improving on sparsity-driven autofocus (SDA) in both
MSE and image entropy across SNR 15-30 dB.

## Methodology — joint image and phase error estimation

### Forward model (Eq. 1 in paper)

```
g = C(φ) f + n                                                  (Eq. 1)
```

- `g ∈ C^{M×1}` — measured phase-history vector (range × azimuth
  samples stacked).
- `f ∈ C^{N×1}` — unknown complex SAR image (reflectivity).
- `φ ∈ R^M` — phase error vector (one phase per range-azimuth cell, or
  one per azimuth cell for 1-D phase errors).
- `C(φ)` — operator `(C(φ))_m = e^{jφ_m} C_m`, where `C_m` is the
  m-th row of the deterministic Fourier-like SAR system matrix
  (e.g., the inverse of a chirp-scaling or polar-format operator).
- `n` — zero-mean Gaussian noise.

**Critical interpretation for G4:** `φ_m` in SAR literature parametrizes
**phase errors** introduced by trajectory / velocity uncertainty.
For our drifting-buoy multistatic SAS, `φ` becomes the vector of
per-buoy per-ping phase errors driven by unknown buoy velocity —
making the Zhang-Achim alternating-minimization machinery directly
applicable if we can write the forward operator `C` for the multistatic
SAS case.

### Cost function (Eq. 2)

```
J(f, φ) = ‖g − C(φ) f‖²₂ − λ Σᵢ₌₁ᴺ ln(γ / (γ² + |fᵢ|²))       (Eq. 2)
                              └─── Cauchy penalty (sparsity-promoting)───┘
```

- `λ > 0` — regularization weight.
- `γ > 0` — Cauchy scale parameter.
- Cauchy penalty: heavy-tailed, non-convex, promotes sparsity more
  aggressively than ℓ₁ without the staircase artifacts.

**Strict-convexity condition:** if `γ > √(μ λ / 2)` where μ is the
CFBA step size, the image sub-problem becomes strictly convex on the
local neighbourhood — algorithm converges to a local minimum.

### Alternating-minimization algorithm

Outer iteration *n*:

#### Step 1 — Image update (f-subproblem)

Fix `φ^{(n)}`, minimize J wrt `f`:

**Option A — WAMA (Wirtinger Alternating Minimization Autofocus):**

Use the Wirtinger-calculus gradient
`∇_f J = C(φ)ᴴ (C(φ) f − g) + λ W(f) f`,
where `W(f) = diag(1 / (γ² + |f_i|²))`. Solve the re-weighted
normal equations:

```
[ C(φ⁽ⁿ⁾)ᴴ C(φ⁽ⁿ⁾) + λ W(f⁽ⁿ⁾) ] f⁽ⁿ⁺¹⁾ = C(φ⁽ⁿ⁾)ᴴ g      (Eq. 5)
```

via conjugate-gradient on the complex system.

**Option B — CFBA (Complex Forward-Backward Splitting):**

```
f^{(n+1,k+1)} = prox_{μ·G}(f^{(n,k)} − 2μ ∇_f H(f^{(n,k)}))    (Eq. 6)
```

where `H(f) = ‖g − C(φ) f‖²` and `G(f) = −λ Σ ln(γ/(γ²+|f_i|²))`.
The proximal operator of the Cauchy penalty has a closed form via
solving a cubic per-element.

#### Step 2 — Phase-error update (φ-subproblem)

Fix `f^{(n+1)}`, minimize J wrt φ. The cost decouples per range bin
*m* (only the m-th data point depends on `φ_m`):

```
J(φ_m) = |g_m − e^{jφ_m} (C_m f^{(n+1)})|²
       = |g_m|² + |C_m f|² − 2 Re{ e^{−jφ_m} g_m* (C_m f) }
```

Minimizing wrt `φ_m` gives the **closed-form** update (Eq. 7):

```
φ_m^{(n+1)} = arctan2( Im{ [f^{(n+1)}]ᴴ C_mᴴ g_m },
                      Re{ [f^{(n+1)}]ᴴ C_mᴴ g_m } )            (Eq. 7)
```

This is a single arctan per range bin per outer iteration — **extremely
cheap**, O(M) per outer step. No iterations needed for the phase
sub-problem.

### Convergence criterion

```
‖f^{(n+1)} − f^{(n)}‖ / ‖f^{(n)}‖ < 10⁻³                        (Eq. 8)
```

with max 300 outer iterations. Inner CFBA has its own inner tolerance
(10⁻³, max 500 inner iterations).

### Hyperparameters used in experiments

- Step size `μ ≤ 1/L` where L = `‖C(φ)ᴴ C(φ)‖₂`.
- Cauchy scale `γ > √(μλ/2)` (strict-convexity gate).
- Typical `(λ, γ) = (0.1, 0.5)` for point-target scenes at SNR 25 dB.
- Simulated SAR: platform altitude 2.5 km, velocity 125 m/s,
  incidence 35°.
- Data sizes: 32×32 synthetic, 64×64 TerraSAR-X, 128×128 sea-surface.

## Key results

### Simulated point-target scene (Scene 1)

| Algorithm          | MSE (lower = better) | Entropy (lower = better) |
|--------------------|----------------------|-------------------------|
| SDA (baseline, ℓ₁) | 8.1 × 10⁻³           | 4.23                    |
| CFBA (Cauchy)      | 3.4 × 10⁻³           | 3.87                    |
| WAMA (Cauchy)      | 3.2 × 10⁻³           | 3.85                    |

~55% MSE improvement over ℓ₁ SDA at the same SNR.

### Real TerraSAR-X / Sentinel-1 scenes

Both CFBA and WAMA produce visually sharper images than SDA on urban
scenes, with measurable entropy reduction (~0.2 nats) and more
preserved edges. The phase-error estimates are consistent with classical
PGA estimates but with better convergence from noisy starts.

### Pierson-Moskowitz sea surface (Scenes 4, 5)

Critical for **maritime / submarine** analog: the method was tested on
simulated sea-surface scenes with 8 m/s and 4 m/s wind-speed
Pierson-Moskowitz spectra. Joint reconstruction produces measurably
better wave-field estimates (MSE reductions of 30-40%) than post-hoc
autofocusing — validating the approach for **ocean-surface scenes that
are structurally similar to our underwater target scenes**.

## Strengths

- **True joint optimization.** Image and phase errors are first-class
  optimization variables in the same cost function, not stages in a
  pipeline. This captures the feedback from image sharpness back to
  phase/velocity refinement that pipelined methods miss.
- **Non-convex Cauchy penalty** outperforms ℓ₁ (SDA) for sparse
  scenes while providing **strict-convexity guarantees** locally
  (given the `γ > √(μλ/2)` condition).
- **Closed-form phase update.** The `φ_m` arctan update is O(1) per
  bin per outer iteration — making the total cost essentially equal
  to the image-reconstruction cost (since phase update is free). This
  is much cheaper than PGA + reimaging loops.
- **Validated on real SAR data** (TerraSAR-X, Sentinel-1), not only
  simulation. Reproducible: GitHub repo exists for the baseline
  CauchySAR framework.
- **Published open-access in MDPI Remote Sensing** — fully accessible,
  no paywall, mature peer-review track.
- **Composable with deep priors.** The Cauchy penalty can be replaced
  with a learned-prior (plug-and-play) while keeping the alternating
  minimization structure intact — opens the door to hybrid
  classical / deep approaches.

## Limitations

- **Phase error is scalar per range bin.** `φ_m` is a single scalar
  per m — adequate for a single platform's trajectory phase drift
  but **does not** directly model the multi-dimensional velocity
  vector of a drifting buoy. For multistatic SAS we need `φ`
  parametrized by per-buoy 3-D velocity **v_buoy**, and `C(v)` built
  from the Kiang-2022 non-stop-and-go range equations. Conceptually
  straightforward but requires rederiving `C(v)`.
- **1-D phase model only.** The paper's primary experiments use
  1-D (per-azimuth) phase errors. Scarnati-Gelb 2018 (Paper G4.4)
  addresses 2-D (azimuth × frequency) phase errors, which Zhang-Achim
  mentions as future work.
- **No explicit velocity prior.** The framework lets `φ` range freely;
  it does not incorporate a physical prior like "buoy velocity is
  < 1 m/s" or "buoys drift coherently". Adding a Bayesian prior on
  `φ` or `v` (e.g., from ranging-derived estimates — our G1) would
  close this gap.
- **Computational cost.** CFBA needs 300-500 inner iterations of CG
  per outer step; WAMA needs ~50 CG iterations. For a 256×256 image
  on a 2.5 GHz laptop, full convergence takes minutes — **not
  real-time**, fine for post-processing of buoy data at a fusion
  centre but not for on-buoy streaming imaging.
- **No uncertainty quantification.** The output is a point estimate
  of `f` and `φ`; no posterior covariance. Bayesian extensions (MCMC
  or variational) are feasible but not in the paper.
- **Local optima.** Non-convex cost → sensitive to initialization.
  The paper uses the phase-corrected matched-filter image as the
  starting point, which is essentially "RDA first, then refine".

## Portable details — math we need for multistatic SAS

### Generalized forward model for N-buoy drifting multistatic SAS

Replace the scalar phase parameter `φ` with a per-buoy 3-D velocity
vector `v^(i)` for buoy *i* (each buoy is a receiver in a different
column-block of `C`). The operator `C(V)` where
`V = [v^(1), v^(2), ..., v^(N)]` is built from Kiang's non-stop-and-go
range equations:

```
R^{(i)}_n(η; v^(i)) = R^{(i)}_{tx,n}(η) + ‖p^(i)(η; v^(i)) − p_n‖
                    = derived from Kiang Eqs. 1-4, Eq. 21-25      (G4.3 ext)

φ^{(i)}_{m,n}(v^(i)) = −4π f_0 R^{(i)}_n(η_m; v^(i)) / c_s
```

So the observation operator becomes

```
C(V) f = Σ_n f_n · (e^{jφ^{(1)}_{m,n}(v^(1))}, …, e^{jφ^{(N)}_{m,n}(v^(N))})
```

and the alternating minimization now alternates between `f` and
**vector-valued** buoy velocities **v^(i)**, with the velocity
sub-problem becoming a **3-parameter non-linear least-squares** per
buoy (instead of a scalar arctan per range bin).

### Coupled velocity sub-problem (per buoy)

For buoy *i*, fix `f^{(n+1)}` and minimize:

```
J(v^(i)) = Σ_m |g^{(i)}_m − e^{jφ^{(i)}_m(v^(i))} (C^{(i)}_m f^{(n+1)})|²
         + ρ ‖v^(i) − v̂^(i)_prior‖²_{Σ_prior}                       (Eq. G4.3.1)
```

where the last term is a **Bayesian prior** on `v^(i)` from either:
- Ranging-derived velocity (G1 ping-pong): `v̂^(i)_prior` from TDOA,
  `Σ_prior` from ranging covariance.
- VAE-coupled motion estimate (G4.1/G4.2): `v̂^(i)_prior = μ_VAE,i`,
  `Σ_prior = Σ_VAE,i`.

This is a small 3-D nonlinear least-squares per buoy per outer
iteration — cheap to solve by Levenberg-Marquardt with analytical
Jacobians from the Kiang derivatives.

### Overall alternating-minimization loop for N-buoy multistatic SAS

```
Init: f^{(0)} = multistatic_RDA(g, V̂_prior)
      V^{(0)} = V̂_prior   (from ranging + VAE fusion)

for n = 0, 1, 2, ...:
    # Image sub-problem: Eq. 5 (WAMA) with multistatic C(V^{(n)})
    f^{(n+1)} = argmin_f [ ‖g − C(V^{(n)}) f‖² + λ Cauchy(f) ]

    # Velocity sub-problem (per buoy): Eq. G4.3.1
    for i = 1, ..., N in parallel:
        v^{(i,n+1)} = argmin_v [ ‖g^{(i)} − e^{jφ^{(i)}(v)} C^{(i)} f^{(n+1)}‖²
                                + ρ ‖v − v̂^{(i)}_prior‖²_Σ ]

    # Convergence check
    if ‖f^{(n+1)} − f^{(n)}‖ / ‖f^{(n)}‖ < 10⁻³: break
```

This is the **core algorithm** for ADR-081.

## Sonobuoy integration plan

### What ports directly into `clawft-sonobuoy-active::joint_reconstruction`

1. **Alternating-minimization outer loop** (fixed-point iteration
   between image and velocity).
2. **WAMA image update** (conjugate-gradient on re-weighted normal
   equations with Cauchy penalty).
3. **Cauchy penalty helpers** (proximal operator for CFBA; reweighting
   for WAMA).
4. **Convergence monitoring** (relative norm, max-iter gate).

### What needs extending from the paper

1. **Multistatic observation operator `C(V)`.** Build from Kiang's
   non-stop-and-go range equations generalized to N buoys.
2. **Vector-valued velocity sub-problem.** Replace per-bin arctan
   with per-buoy Levenberg-Marquardt 3-D NLS.
3. **Bayesian velocity prior.** Fuse ranging (G1) and VAE (G4.1/G4.2)
   into the sub-problem's Σ_prior.
4. **Posterior covariance** for both `f` and `V` via Laplace
   approximation at the converged point (needed for downstream
   tracking uncertainty).

### Proposed contribution to ADR-081

Zhang-Achim's alternating minimization is the **joint-reconstruction
engine** of ADR-081. It is the mathematical kernel that fuses:

- Kiang's multistatic non-stop-and-go range model (C(V) operator),
- G1 ranging-derived velocity priors (Bayesian term),
- Xenaki-style VAE motion estimates (additional Bayesian term),

into a **single optimization** that outputs the focused target image
AND the refined per-buoy velocities simultaneously — with
guaranteed local convergence and MSE improvements over pipelined
approaches.

## Follow-up references

1. **Karakus, O., Mayo, P., Achim, A. (2020).** "Convergence Guarantees
   for Non-Convex Optimisation with Cauchy-Based Penalties." *IEEE
   Trans. Signal Processing* **68**, 6159-6170. DOI:
   10.1109/TSP.2020.3025519. — The theoretical foundation for the
   Cauchy regularizer; `γ > √(μλ/2)` comes from here.
2. **Karakus, O. & Achim, A. (2020).** "On Solving SAR Imaging Inverse
   Problems Using Nonconvex Regularization with a Cauchy-Based
   Penalty." *IEEE Trans. Geoscience and Remote Sensing* **59**(7),
   5828-5840. DOI: 10.1109/TGRS.2020.3011631. — Earlier application of
   Cauchy to SAR imaging (without autofocus).
3. **Zhang, Z.-Y., Pappas, O., Achim, A. (2020).** "SAR Image
   Autofocusing using Wirtinger calculus and Cauchy regularization."
   arXiv:2012.09772 / *IEEE ICIP 2021*. — The earlier, shorter WAMA-
   focused companion.
4. **Onhon, N. Ö. & Çetin, M. (2011).** "A sparsity-driven approach
   for joint SAR imaging and phase error correction." *IEEE Trans.
   Image Proc.* **21**(4), 2075-2088. DOI:
   10.1109/TIP.2011.2179056. — The SDA baseline this paper improves on;
   canonical reference for sparsity-driven joint autofocus.
5. **GitHub:** `oktaykarakus/CauchySAR-v1.0` — Reference MATLAB /
   Python implementation of the Cauchy-SAR framework.

---

*This analysis is Paper G4.3 — the optimization-based joint image-and-
phase-error estimation paper that provides the mathematical backbone
for the full joint reconstruction step of our N-buoy multistatic SAS
pipeline. It pairs with Scarnati-Gelb 2018 (2-D phase model, Paper
G4.4) and with Xenaki 2022/2024 (motion estimator as Bayesian prior,
Papers G4.1/G4.2) and Kiang 2022 (non-stop-and-go range model) to
complete the recipe for ADR-081.*
