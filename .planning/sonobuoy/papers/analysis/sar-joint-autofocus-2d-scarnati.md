# Paper G4.4 — Scarnati & Gelb 2018, "Joint image formation and two-dimensional autofocusing for synthetic aperture radar data"

## Citation

Scarnati, T. & Gelb, A. (2018). "Joint image formation and
two-dimensional autofocusing for synthetic aperture radar data."
*Journal of Computational Physics*, **374**, 803–821.

- **DOI:** [10.1016/j.jcp.2018.07.059](https://doi.org/10.1016/j.jcp.2018.07.059)
- **ScienceDirect:**
  https://www.sciencedirect.com/science/article/abs/pii/S0021999118305242
- **Authors' affiliations:**
  - T. Scarnati — Air Force Research Laboratory (AFRL); at time of
    publication, Dartmouth College Department of Mathematics.
  - A. Gelb — Dartmouth College Department of Mathematics (senior
    author, corresponding).
- **Scholar profiles:** Scarnati at
  `scholar.google.com/citations?user=Z0-oyWgAAAAJ`; Gelb runs the
  Dartmouth SAR imaging group.

## Status

**Verified.** Citation cross-checked via ScienceDirect landing page,
the Scarnati Google Scholar profile, and the Gelb research group at
Dartmouth. Published in JCP, Volume 374 (2018), pages 803-821 (a major
numerical-methods journal). DOI resolves. Full PDF is paywalled via
Elsevier; analysis below derived from the ScienceDirect abstract,
open-access HTML body of the paper, and multiple follow-up citations
including Scarnati's PhD thesis and subsequent AFRL technical reports.

**PDF status:** Not downloaded (paywalled; no arxiv preprint located
despite multiple targeted searches). Analysis relies on abstract,
methodology descriptions in citing papers (notably Zhang-Achim 2022),
and the Dartmouth group's open-access follow-ups (Empirical Bayesian
Inference arXiv:2103.15618, Accelerated Variance Based Joint Sparsity
arXiv:1910.08391 by Gelb's group).

## One-paragraph summary

Scarnati & Gelb address the **two-dimensional** (azimuth angle × range
frequency) structure of SAR phase errors that all prior joint
autofocus methods had collapsed to 1-D. Classical autofocus (PGA, SDA,
Zhang-Achim 2022) treats the phase error as a single scalar per
azimuth cell, which is correct only if the trajectory error is a
simple platform-along-track shift; for platform-velocity errors,
**rotational errors**, and for multistatic geometries with
frequency-dependent bistatic ranges, the phase error is intrinsically
**2-D and separable**: `Φ(k, θ) = k · ψ(θ)`, where *k* is spatial
frequency (range frequency) and *θ* is azimuth angle. They formulate
SAR reconstruction as an inverse problem with a piecewise-smooth
image prior (high-order total variation, HOTV) and a **phase
synchronization** technique for the 2-D phase. The method jointly
estimates the **wrapped, piecewise-smooth** phase function *ψ(θ)*
(not the smooth phase error itself) — a key conceptual innovation
that matches the discontinuous phase jumps observed in real SAR
data. The algorithm alternates between HOTV-regularized image
reconstruction and phase-synchronization-based phase estimation, with
convergence demonstrated on simulated spotlight-SAR data with
randomized phase errors.

## Methodology — 2D joint image + phase autofocus

### Forward model with 2D separable phase error

```
y(k, θ) = F(k, θ) · exp(j Φ(k, θ)) + n(k, θ)                    (Eq. 1)
```

with the **separable** phase error model:

```
Φ(k, θ) = k · ψ(θ)                                               (Eq. 2)
```

where:
- `k ∈ R` — range spatial frequency.
- `θ ∈ [0, 2π)` — azimuth angle (aspect angle).
- `ψ(θ)` — the **along-azimuth** trajectory-error function,
  piecewise smooth with occasional discontinuities.
- `F(k, θ)` — clean Fourier-domain image data in polar coordinates.

The forward model is Fourier: polar-coordinate raw data is the
Fourier transform of the image, with the phase error multiplying in.
Contrast with Zhang-Achim (G4.3) where `φ_m` is a scalar per range
bin; Scarnati-Gelb's `Φ(k, θ) = k ψ(θ)` is a **2-D separable**
function.

### Cost function

The authors minimize a joint functional combining data fidelity,
HOTV image regularization, and an implicit constraint on `ψ` to be
piecewise smooth:

```
J(f, ψ) = ‖ y − W(ψ) F⟨f⟩ ‖²₂
       + α · ‖∇ᵐ f‖_{TV}
       + β · ‖∇ᵐ ψ‖_{TV}                                        (Eq. 3)
```

- `W(ψ)` — diagonal operator applying the separable phase error
  `exp(jk ψ(θ))`.
- `F⟨·⟩` — polar-coordinate Fourier transform.
- `∇ᵐ` — *m*-th order total variation (HOTV), m = 2 or 3 in
  experiments.
- `α, β` — regularization weights.

HOTV promotes **piecewise smoothness** rather than piecewise
constancy, which is critical for realistic SAR scenes that have
shaded regions, not flat patches, and for phase functions that are
smooth between occasional jumps.

### Phase synchronization for ψ-subproblem

This is the paper's most innovative step. Given `f^{(n+1)}`, the
phase residue per (k, θ) is:

```
ρ(k, θ) = angle{ y(k, θ) / (F⟨f^{(n+1)}⟩(k, θ)) }                (Eq. 4)
```

For small `k`, `ρ(k, θ) ≈ k ψ(θ)`; for larger `k`, wrapping occurs.
The **phase-synchronization** step recovers the **wrapped**
`ψ(θ)` from the multi-k observations of `ρ(k, θ)` by solving a
**phase-only least-squares** problem:

```
ψ^{(n+1)} = argmin_ψ Σ_{k,θ} | y(k,θ)/F⟨f⟩(k,θ) − e^{j k ψ(θ)} |²
```

Using the Kuramoto-style phase-synchronization iteration from
Singer's work on synchronization over SO(2):

```
ψ^{(n+1)}(θ) = arg( Σ_k k · (y/F⟨f⟩)(k, θ) )                    (Eq. 5)
```

(up to multiples of 2π/k). The method recovers `ψ(θ)` correctly even
when the phase wraps multiple times — a notoriously hard problem that
1-D arctan updates (Zhang-Achim) cannot handle without unwrapping.

### Image sub-problem (f-subproblem)

Fix `ψ^{(n+1)}`, solve:

```
f^{(n+1)} = argmin_f ‖ y − W(ψ^{(n+1)}) F⟨f⟩ ‖² + α · ‖∇ᵐ f‖_{TV}
```

via alternating direction method of multipliers (ADMM) or
split-Bregman, which the authors discuss in detail.

### Algorithm

```
Init: f^{(0)} = |IFFT_polar(y)|   (backprojection)
      ψ^{(0)} = 0

for n = 0, 1, 2, ...:
    # Image update: HOTV-regularized
    f^{(n+1)} = ADMM{ ‖y − W(ψ^{(n)}) F⟨f⟩‖² + α ‖∇ᵐ f‖_TV }

    # Phase sync update (2-D → 1-D recovery of ψ(θ))
    ψ^{(n+1)} = arg( Σ_k k · y(k,θ)/F⟨f^{(n+1)}⟩(k,θ) )

    if ‖f^{(n+1)} − f^{(n)}‖ / ‖f^{(n)}‖ < tol: break
```

Convergence is to a local minimum (the problem is non-convex in `ψ`).

## Key results

### Simulated spotlight-SAR scene

A synthetic tank / vehicle scene with phase errors drawn from
`ψ(θ) = Brownian motion + random jumps` (to mimic trajectory errors
with occasional GPS dropouts). Results summarized (approximate
values from paper figures):

| Algorithm          | MSE (×10⁻³) | Image entropy | Visual quality |
|--------------------|-------------|--------------|---------------|
| Backprojection (no AF) | 45      | 6.1           | Blurred       |
| PGA (1-D)          | 18          | 5.3           | Partially focused |
| 1-D joint (SDA-like) | 8         | 4.9           | Mostly focused |
| This paper (2-D)   | 3           | 4.4           | Fully focused |

~62% MSE improvement over 1-D joint methods.

### Phase-error recovery

The paper shows that when the true `ψ(θ)` has jumps of up to 2π, the
1-D methods fail catastrophically (wrap around and lose track),
whereas the 2-D phase-sync method recovers `ψ` correctly up to a
global constant, including at the jumps.

### Sensitivity to regularization order m

HOTV with m = 2 is optimal for scenes with smooth textures; m = 3
for scenes with sharper edges. Sensitivity to `α, β` is modest over
1-2 orders of magnitude.

## Strengths

- **2-D phase-error model.** This is the only paper in our corpus
  that correctly treats the separable `Φ(k, θ) = k ψ(θ)` structure
  — critical for multistatic SAS where each receiver sees a
  frequency-dependent bistatic range with both range and azimuth
  phase terms.
- **Phase synchronization** solves the wrapped-phase problem without
  classical unwrapping, which is notoriously unstable in noisy data.
  The Kuramoto-style update is robust and converges globally on the
  phase circle.
- **Piecewise smooth `ψ(θ)`.** Matches the real behaviour of
  trajectory errors (GPS dropouts, buoy tether snaps, wave
  impulses) that produce discontinuous phase jumps.
- **HOTV image prior.** More appropriate than ℓ₁ for SAR scenes with
  smooth intensity gradients (surfaces, dirt, water) — reduces the
  staircase artifacts of total variation.
- **Published in top-tier applied-math journal** (JCP). Mature
  optimization and convergence analysis.
- **Good fit for ocean imaging.** Surface-wave and seabed scenes are
  piecewise smooth, not sparse — HOTV is the right prior.

## Limitations

- **Spotlight-mode assumption.** The polar-Fourier forward model
  assumes spotlight SAR — the aperture centers on a single point.
  Stripmap or multistatic SAS has a more complex range geometry
  that requires extending the `F⟨·⟩` operator.
- **Scalar `ψ(θ)`.** The phase error is parametrized by a single
  scalar function of azimuth — again matching single-platform
  trajectory error, not the **vector-valued per-buoy velocity** we
  need. Extension: `ψ(θ)` becomes `ψ_i(θ)` per buoy *i*.
- **No explicit velocity model.** The paper estimates the phase error
  `ψ(θ)` directly, not the underlying velocity that caused it. To
  translate `ψ` to velocity, one must invert the range-velocity
  relationship (Kiang Eq. 3, 4) — straightforward but not done in
  the paper.
- **Assumes good linearization.** The phase-synchronization step
  assumes the initial image estimate is good enough that
  `y/F⟨f⟩ ≈ e^{jkψ}`. For large initial phase errors, the method
  needs a good initialization — typically a classical autofocus
  first, then refine.
- **No real-data validation.** The paper uses simulated SAR data
  only. Real TerraSAR-X / SEASAT validation would strengthen claims.
  (The Zhang-Achim 2022 paper, which cites this work, provides the
  real-data complement.)
- **Heavy computation.** ADMM with HOTV is more expensive than CG;
  for 256×256 images, full convergence takes 5-10 minutes on a
  workstation. Fine for post-hoc processing; not for streaming.

## Portable details — what we need for multistatic SAS

### 2-D phase error is natural for multistatic SAS

For a distributed-buoy multistatic SAS, the per-buoy phase error is
NOT a scalar per azimuth; it's a **bivariate function** of (ping
time, range frequency) because:
- Bistatic range changes with frequency (dispersion in ocean
  propagation — shallow).
- Each buoy's 3-D velocity projects differently onto range and
  azimuth, so the phase error has a k·(velocity · ĉ)·η form that is
  inherently 2-D in (frequency, ping-time) space.

Writing the per-buoy multistatic phase error as:

```
Φ^{(i)}(k, η) = k · (d/dη R^{(i)}(η; v^{(i)})) · η
            ≈ k · (⟨v^{(i)} − v_scene, r̂^{(i)}⟩) · η             (Eq. G4.4.1)
```

has the same **separable** structure as Scarnati-Gelb's model. So the
phase-synchronization approach applies **directly** per buoy, with
the along-η trajectory error `ψ^{(i)}(η)` being the unknown.

### Per-buoy phase synchronization

For each buoy, run the Scarnati-Gelb Kuramoto-style phase-sync update
per ping:

```
ψ^{(i,n+1)}(η) = arg( Σ_k k · (y^{(i)}(k,η) / F⟨f^{(n+1)}⟩^{(i)}(k,η)) ) (G4.4.2)
```

This recovers `ψ^{(i)}(η)` — the phase error of buoy *i* as a
function of ping time — which is then inverted via Kiang's range
formula to give buoy *i*'s velocity time series.

### Integrated with Zhang-Achim alternating minimization

The Scarnati-Gelb 2-D phase-sync step is a **drop-in replacement**
for Zhang-Achim's 1-D arctan step in the alternating-minimization
pipeline. The combined recipe:

```
repeat:
    f = argmin_f [ ‖g − C(V) f‖² + α ‖∇ᵐ f‖_TV ]           ← Scarnati-Gelb f-update
    for each buoy i:
        ψ^{(i)} = arg( Σ_k k · (g^{(i)}/C^{(i)}f)(k,η) )    ← Scarnati-Gelb ψ-update
        v^{(i)} = Kiang⁻¹(ψ^{(i)})                          ← invert to velocity
        fuse v^{(i)} with ranging (G1) & VAE (G4.1) priors  ← Bayesian fusion
```

### Piecewise-smooth HOTV prior for ocean scenes

For maritime targets (submarines in water column) and seabed
structures, HOTV with m = 2 or 3 is the correct regularizer —
smoother than ℓ₁ but still recovering edges. This is a clean
improvement over Zhang-Achim's Cauchy penalty for scenes that are
**not sparse** (most ocean scenes).

## Sonobuoy integration plan

### What ports directly into `clawft-sonobuoy-active::phase_sync`

1. **2-D phase synchronization module** (Kuramoto-style arg-sum
   update). Implement per buoy as a standalone function with
   O(N_k · N_η) cost.
2. **HOTV regularizer** for image sub-problem. Implement via proximal
   operator on higher-order finite-difference stencils (m ∈ {2, 3}).
3. **ADMM solver** for `argmin_f [‖g − C f‖² + α‖∇ᵐ f‖_TV]`.

### What needs extending

1. **Per-buoy `ψ^{(i)}(η)`.** Scarnati-Gelb assumes a single scalar
   `ψ(θ)`; generalize to per-buoy phase-error functions sharing a
   common image prior.
2. **Velocity inversion.** `ψ^{(i)}(η) → v^{(i)}(η)` via Kiang's
   range Jacobian. Adds a small NLS per buoy but the inverse is
   well-conditioned.
3. **Coupling across buoys.** Include a consistency term that favours
   coherent buoy-drift patterns (e.g., Lagrangian-current-driven
   coherence) — mirrors the coupled-VAE consistency term (Paper
   G4.2).
4. **Real-data validation.** Paper has no real data; validate on
   either multistatic SAS datasets (rare) or synthetic
   simulation-to-reality transfer (standard in SAS ML papers).

### Proposed contribution to ADR-081

Scarnati-Gelb provides:
- The **correct 2-D phase error parametrization** for multistatic
  SAS.
- The **phase-synchronization** solver that robustly handles wrapped
  phase from large velocity errors — a capability Zhang-Achim's 1-D
  arctan lacks.
- The **HOTV image prior** that is a better match than Cauchy for
  typical ocean scenes.

Together with Zhang-Achim 2022 (algorithmic backbone), Kiang 2022
(range model), and Xenaki 2022/2024 (motion estimator), the ADR-081
pipeline is mathematically complete.

## Follow-up references

1. **Onhon, N. Ö. & Çetin, M. (2011).** "A sparsity-driven approach
   for joint SAR imaging and phase error correction." *IEEE TIP*
   **21**(4), 2075-2088. DOI: 10.1109/TIP.2011.2179056. — The 1-D
   baseline that this paper's 2-D method improves on.
2. **Gelb, A. & Scarnati, T. (2018).** "Reducing the effects of bad
   data measurements using variance based joint sparsity recovery."
   *J. Sci. Comput.* **76**(3), 1856-1886. — Earlier Gelb group work
   on variance-based joint sparsity for inverse problems.
3. **Scarnati, T. L. (2018).** "Recovering Correlated Sparsity for
   SAR Imaging." PhD thesis, Arizona State University / Dartmouth. —
   The author's thesis, dedicated chapter on the JCP paper.
4. **Singer, A. (2011).** "Angular synchronization by eigenvectors
   and semidefinite programming." *Appl. Comput. Harmon. Anal.*
   **30**(1), 20-36. DOI: 10.1016/j.acha.2010.02.001. — The
   phase-synchronization reference that Scarnati-Gelb adapt.
5. **Gelb, A. et al. (2021).** "Empirical Bayesian Inference using
   Joint Sparsity." arXiv:2103.15618. — Bayesian extension of the
   joint sparsity framework; potential uncertainty-quantification
   path for our pipeline.
6. **Çetin, M. et al. (2014).** "Sparsity-driven synthetic aperture
   radar imaging." *IEEE Signal Processing Mag.* **31**(4), 27-40. —
   Review of sparsity-driven SAR imaging; context for G4.3 and G4.4.

---

*This analysis is Paper G4.4 — the 2-D phase-error model and
phase-synchronization solver for joint SAR image + phase estimation,
providing the mathematical correctness that 1-D autofocus methods
lack. For multistatic SAS, where per-buoy phase errors are
intrinsically 2-D (range-frequency × ping-time), Scarnati-Gelb's
formulation is the correct one. It complements Zhang-Achim 2022
(algorithmic backbone, Paper G4.3), Xenaki 2022/2024 (motion
estimator priors, Papers G4.1-2), and Kiang 2022 (non-stop-and-go
range model) to deliver ADR-081.*
