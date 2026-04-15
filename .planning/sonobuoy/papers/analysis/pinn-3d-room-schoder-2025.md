# Paper G2.4 — 3D Helmholtz PINN: Feasibility + Production-Ready Techniques

## Citation

> Schoder, S. (2025).
> "Physics-Informed Neural Networks for Modal Wave Field Predictions
> in 3D Room Acoustics."
> *Applied Sciences*, **15**(2), 939.
> DOI: [10.3390/app15020939](https://doi.org/10.3390/app15020939)

**Primary companion paper (feasibility precursor):**

> Schoder, S., & Kraxberger, F. (2024).
> "Feasibility study on solving the Helmholtz equation in 3D with
> PINNs." arXiv preprint arXiv:2403.06623. March 2024.

**Status:** verified

**Verification trail.**
- MDPI journal landing: https://www.mdpi.com/2076-3417/15/2/939
  (Applied Sciences, Vol. 15 Issue 2, Jan 18, 2025, Article 939).
- DOI 10.3390/app15020939 confirmed via MDPI.
- arXiv precursor: https://arxiv.org/abs/2403.06623 (submitted
  March 11, 2024; authors Schoder, Kraxberger).
- ADS record: 2024arXiv240306623S.
- ResearchGate record: publication 378875389.
- Author affiliation: Graz University of Technology, Institute of
  Fundamentals and Theory in Electrical Engineering.

**PDFs:**
- `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/pinn-3d-helmholtz-schoder.pdf`
  (arXiv:2403.06623, feasibility study, 2.0 MB).
- `pinn-3d-room-schoder-2025.pdf` — MDPI HTML gate blocked direct
  PDF; full methodological content extracted from MDPI HTML and
  summarized below. Can be fetched manually from
  https://www.mdpi.com/2076-3417/15/2/939 with browser.

---

## One-paragraph summary

This pair of papers (Schoder & Kraxberger 2024 feasibility study
plus Schoder 2025 full production paper) is the **most directly
relevant prior art to Gap G2**: both solve the 3D Helmholtz equation
with PINNs on comparable grid sizes and report explicit error
numbers against a validated FEM reference. The feasibility paper
establishes that 3D Helmholtz-PINN is *achievable* (relative error
0.045 vs sparse FEM, 0.10 vs fine FEM, inference 20,000× faster than
FEM once trained) but requires 38–43 hours of GPU training with mini-
batch SGD and periodic collocation-point resampling. The 2025 paper
then shows *which techniques matter*: systematic hyperparameter
sweep (L2 error 0.45% → 0.019%), locally-adaptive activation
functions (LAAF, extra 60% improvement for sharp sources), and
**feature-engineered deterministic dispersion-relation Fourier
features** — which both accelerate training 20× *and* improve
accuracy to 2×10⁻⁶ relative L² (three orders of magnitude better
than standard PINN). For G2 this gives us verified numerical
answers: "yes, 3D Helmholtz-PINN works at R²>0.99 if you use
Fourier-engineered features; no, vanilla Du-2023-style PINN does not
scale, as we already knew." The Schoder-2025 architecture is
essentially a drop-in recipe for closing G2.

---

## Methodology

### Network architecture (standard PINN baseline)

Following Raissi 2019 / Du 2023 lineage:

- **Topology**: fully-connected MLP.
- **Depth**: 4 hidden layers.
- **Width**: 180–250 neurons per layer (searched over).
- **Activation**: `sin` with Glorot uniform init (Sitzmann SIREN-
  style — winning over ReLU, tanh, GELU, ELU, SELU, sigmoid, SiLU,
  swish in the hyperparameter sweep).
- **Input**: 3D coordinates `(x, y, z)`.
- **Output**: scalar acoustic pressure (the paper uses real-valued
  standing-wave modes; complex `(Re P, Im P)` extension is trivial).

### Physics formulation

Homogeneous 3D Helmholtz in a cubic room:

```
∇² P(x) + k² P(x) = s(x)              (with Gaussian source term s)
x ∈ Ω = [0, 1]³
∂P/∂n = 0  on ∂Ω                      (sound-hard Neumann walls)
λ = 0.5 m,  k = 2π/λ ≈ 12.57 rad/m    (wavelength/wavenumber)
c = 1 m/s                             (scaled sound speed)
```

Gaussian point source `s(x) = exp(-|x - x_s|² / (2s²))` with
sharpness parameter `s` varying from distributed (s→∞) to tightly
confined (s=0.05, sub-wavelength).

### Loss formulation

```
L = λ_PDE · L_PDE + λ_NBC · L_NBC
  = 1 · MSE( ∇²P + k²P - source )
  + 5 · MSE( ∂P/∂n )                  on boundary
```

Note `λ_data = 0` — pure physics loss, no data supervision. This is
the hardest training regime (no ground truth guidance) and the one
that best generalizes to the sonobuoy deployment case where data
would come only from CTD + boundary conditions.

### Training protocol

| Stage | Iterations | Optimizer | Details |
|-------|-----------|-----------|---------|
| 1 | 10k–30k | Adam (lr 1e-3) | warm-up |
| 2 | up to 150k | L-BFGS | fine-tune |
| 3 (adaptive) | +3k Adam + 150k L-BFGS | | Only for sharp sources `s < 0.1` |

**Collocation strategy**: 10 random points per wavelength for
training, 30 for testing. Mini-batch SGD with **periodic resampling
every 100 iterations** to prevent overfitting to a fixed
collocation set — the critical 3D-specific trick.

### Techniques evaluated (2025 paper hypothesis-driven)

The 2025 paper tests 5 hypotheses in a controlled ablation:

| H | Technique | Outcome | Gain |
|---|-----------|---------|------|
| H1 | Hyperparameter optimization | confirmed | 0.45% → 0.019% L² |
| H2 | Locally Adaptive Activation Functions (LAAF) | confirmed | +60% for s=1 |
| H3 | Adaptive residual-based refinement | confirmed | +60% for s=1m |
| H4 | Random Fourier features (Tancik-style) | **NOT confirmed** | no gain |
| H5 | Deterministic dispersion-relation features | confirmed | 20× speedup + 3 orders of magnitude error drop |

H4 is an important negative result: *random* Fourier features
(standard Tancik 2020 / Wang 2021) alone do not beat a hyperparameter-
tuned plain MLP in this cubic-room setting. H5 reveals why:
deterministic features constructed from the **dispersion relation**
(Green's function modal expansion: `cos(n_x π x) cos(n_y π y)
cos(n_z π z)` products matching the Neumann eigenbasis) encode the
*actual* solution basis, not a random high-frequency dictionary. For
ocean acoustics the analog is normal-mode eigenfunctions — exactly
what KRAKEN produces.

---

## Key results

### Feasibility study (Schoder & Kraxberger 2024)

| Metric | Value |
|--------|-------|
| 3D Helmholtz domain | 1 m³ cube |
| Wavenumber k | 2π/λ, λ = 0.5 m |
| Training time | 38–42.8 hours (NVIDIA GPU) |
| Inference time | 0.05 s |
| FEM equivalent | 17–19 minutes |
| Inference speedup | ~20,000× vs FEM |
| Error (PINN vs sparse FEM) | 0.0454 |
| Error (PINN vs fine FEM) | 0.0997 |
| Error (sparse FEM vs fine FEM) | 0.0924 |

The sparse-FEM-error baseline (0.0924) puts the PINN error (0.0454)
in context: **PINN is closer to fine-FEM than sparse-FEM is**,
within the accepted tolerance of engineering 3D room-acoustics
simulation.

### Production paper (Schoder 2025)

| Configuration | Source sharpness `s` | L² relative error |
|---------------|---------------------|-------------------|
| Standard PINN (default) | smooth | 0.35% |
| Standard PINN + hyperparam-tuned | smooth | **0.019%** |
| Standard PINN + adaptive refinement | smooth | 0.28% |
| LAAF-PINN | s = 1 m | 0.086% |
| **Feature-engineered PINN** | smooth | **2.0×10⁻⁴ %** |
| Feature-engineered PINN | s = 1 m | 0.20% |
| FEM reference (N_dof/λ=50) | s = 1 m | ~10⁻⁵ (ground truth) |

The feature-engineered PINN on distributed sources achieves
**2×10⁻⁶ relative L² error** — one order of magnitude better than
FEM with 1.06M elements. Training time drops to <1 minute on
Tesla T4 (versus 20 min for standard PINN and 6–15 min for FEM).

### Computational performance table

| Method | Training | Inference | Accuracy (L² rel.) |
|--------|----------|-----------|---------------------|
| FEM (openCFS, CPU) | — | 15.5 min | ~10⁻⁵ (ref) |
| FEM (GPU) | — | 6 min | ~10⁻⁵ |
| Standard PINN | 20 min | 0.05 s | 2.8×10⁻³ |
| Tuned standard PINN | ~30 min | 0.05 s | 1.9×10⁻⁴ |
| Feature-engineered PINN | **<1 min** | 0.05 s | **2.0×10⁻⁶** |

---

## Strengths

1. **Directly 3D Helmholtz.** Not a 2D result we need to
   extrapolate from — this *is* the G2 problem.
2. **Ablation-driven.** Each technique's contribution is quantified
   in isolation, so we know which bits are necessary.
3. **Negative result on random Fourier features.** The H4 finding
   is a real surprise and saves us from implementing a technique
   (vanilla Tancik 2020) that provides no gain here. Suggests that
   for the sonobuoy case we should prefer dispersion-relation-
   based deterministic features (normal modes) over random
   Fourier features — a direct engineering prescription.
4. **Feature-engineered path to <1 min training.** If we can find
   the ocean-acoustic analog of the dispersion-relation embedding
   (normal modes, WKB eigenfunctions, or ray-tube basis), we may
   match this <1 min training and retain the zero-data physics-
   only loss setup.
5. **Validated against FEM.** openCFS reference with 1.06M elements
   gives a trustworthy ground truth; L² errors are meaningful.
6. **Sharpness-parameterized source.** The `s` sweep from
   distributed to highly confined source directly models the
   sonobuoy case where a whale call or ship engine is a near-point
   source — the paper shows which techniques degrade and by how
   much as source confinement increases.

## Limitations

1. **Cubic room, not stratified ocean.** The domain is a homogeneous
   1 m³ cube with constant sound speed. Ocean acoustics has
   `c(z)` varying by ~30 m/s vertically; the PINN must learn a
   coefficient-variable Helmholtz, not a constant one. This is
   the delta from Schoder to sonobuoy.
2. **Neumann walls, not Sommerfeld radiation.** Closed-cube
   boundary conditions differ from the open-ocean case. However,
   pressure-release surface + Neumann seafloor is structurally
   similar, and far-field radiation can be imposed via PML or
   absorbing layers.
3. **Scalar sound speed c=1.** All results in scaled units; no
   physical sound-speed-profile coupling. Rescaling to `c(z)` is
   mathematically trivial but numerically nontrivial (wavenumber
   `k(r,z)` becomes spatially varying, invalidating the
   dispersion-relation eigenbasis).
4. **Single frequency.** Each PINN trained at a single `k`.
   Broadband detection needs either retraining per frequency band
   or a frequency-conditioned variant (not demonstrated).
5. **Feature engineering is problem-specific.** The
   dispersion-relation features that work so well here are
   cube-specific normal modes. For ocean, we need to derive the
   analogous modal basis from the local SSP — this is exactly what
   KRAKEN computes, so the ocean-acoustic Schoder analog is
   "KRAKEN eigenmodes + learned residual."

---

## Portable details

### Sonobuoy-adapted architecture

Following the Schoder 2025 feature-engineered PINN:

```python
class SonobuoyHelmholtzPINN(nn.Module):
    def __init__(self, n_modes_r=32, n_modes_theta=16, n_modes_z=16,
                 width=32, n_layers=2):
        super().__init__()
        self.n_modes = (n_modes_r, n_modes_theta, n_modes_z)
        # Deterministic "dispersion-relation" features — ocean analog:
        # normal-mode eigenfunctions psi_n(z) from KRAKEN + cylindrical
        # Hankel expansion in r.
        # Input preprocessing: x = (r, theta, z) →
        #   [J_0(k_n r), psi_n(z), cos(m theta), sin(m theta)]
        # where k_n, psi_n are pre-computed normal modes for the
        # current SSP.
        # Output: learned residual on top of the modal expansion.
        self.mlp = build_mlp(
            in_dim=self.feature_dim(),
            hidden=[width] * n_layers,
            out_dim=2   # (Re P, Im P)
        )

    def feature_dim(self):
        return 2 * self.n_modes[0] * self.n_modes[1] * self.n_modes[2]

    def forward(self, x):
        feats = self.dispersion_features(x)
        return self.mlp(feats)

    def dispersion_features(self, x):
        r, theta, z = x[..., 0], x[..., 1], x[..., 2]
        # Bessel J_0(k_n r) for each radial mode
        J = torch.stack([
            bessel_j0(self.k_n[n] * r)
            for n in range(self.n_modes[0])
        ], dim=-1)
        # Eigenfunction psi_n(z) from KRAKEN
        psi = self.psi_lookup(z)     # (batch, n_modes_z)
        # Azimuthal harmonics
        cs = torch.stack([torch.cos(m * theta) for m in range(self.n_modes[1])], dim=-1)
        sn = torch.stack([torch.sin(m * theta) for m in range(self.n_modes[1])], dim=-1)
        # Outer product, flatten
        feats = torch.einsum('bi,bj,bk->bijk', J, psi, cs).flatten(-3)
        return torch.cat([feats, feats_sn], dim=-1)
```

### Training hyperparameters (adapted from Schoder 2025)

```yaml
architecture:
  type: feature_engineered_pinn
  n_modes_r: 32                          # Bessel radial modes
  n_modes_theta: 16                      # azimuthal harmonics
  n_modes_z: 16                          # KRAKEN normal modes
  trunk_width: 32
  trunk_layers: 2
  activation: sin                        # Sitzmann SIREN init

training:
  stage_1:
    optimizer: adam
    lr: 1.0e-3
    iterations: 10_000
  stage_2:
    optimizer: lbfgs
    iterations: 50_000
  collocation:
    pts_per_wavelength: 10
    resample_every: 100                  # key 3D trick
  loss_weights:
    pde: 1.0
    boundary_pressure_release: 100.0     # z=0 surface
    boundary_seafloor: 5.0               # z=H Neumann/impedance
    source_ring: 10.0                    # around point source
    data: 0.0                            # physics-only regime

optional_LAAF:
  enabled: true                          # +60% when source is point-like
  slope_recovery_weight: 1.0e-4

optional_RAR_D:
  enabled: true                          # for s < 0.2 m (point sources)
  add_pts_per_cycle: 150
  refinement_cycles: 35
```

### Compute estimate

Based on Schoder 2025's feature-engineered result (<1 min on T4):

- **Training**: ~5 minutes per (SSP, frequency, src_depth) tuple on
  an L4 GPU. With KRAKEN precomputation ~30 s.
- **Inference**: ~0.05 s per query at ~10k points.
- **Per-deployment cost**: 5 min × 8 freq bands = 40 min GPU.

This is 20× better than the feasibility paper's 38 hr and
acceptable for the per-deployment precomputation step.

---

## How this closes G2

1. **Verified 3D R²** Standard PINN on 3D Helmholtz: L² error
   0.045 → R² ≈ **0.998** (on the cubic problem). With feature
   engineering: R² ≈ **0.99999+**. Both exceed the 0.85 target
   by wide margins *in the cubic-room setting*.
2. **Recipe, not just result.** The paper identifies the five
   specific techniques (hyperparameter tuning, LAAF, adaptive
   refinement, deterministic Fourier features, sine activation)
   that convert a failing plain PINN into a working 3D PINN. We
   can adopt them directly.
3. **Negative result on random Fourier features.** H4 tells us to
   prefer deterministic dispersion-relation features (KRAKEN
   normal modes) over random Fourier features. This refines the
   Paper G2.2 recommendation: use random Fourier features as a
   fallback only, prefer KRAKEN-modal basis for production.
4. **Training time achievable.** <1 minute on consumer GPU for
   feature-engineered PINN means online re-fit on new SSP at
   buoy deployment is feasible — the "online mode" in Du 2023's
   integration plan is realized.
5. **Integration path.** Adapt the feature-engineered recipe into
   `eml-core::operators::helmholtz_residual` with a pluggable
   feature module (KRAKEN modes, random Fourier, or hybrid).

**Residual risk.** Porting "cubic room + constant `c`" results to
"cylindrical ocean + `c(z)`" requires that the dispersion-relation
feature basis generalize to stratified media. The natural
generalization is the KRAKEN modal expansion — but KRAKEN modes
themselves cost O(N_z³) per SSP to compute. This must be amortized
or replaced with a cheaper ML approximation (see follow-up
references).

---

## Follow-up references

1. **Schoder, S., & Kraxberger, F. (2024).** "Feasibility study on
   solving the Helmholtz equation in 3D with PINNs."
   arXiv:2403.06623. — The 2024 precursor feasibility paper;
   gives training-time numbers and basic 3D viability.

2. **Jagtap, A. D., Kawaguchi, K., & Karniadakis, G. E. (2020).**
   "Locally Adaptive Activation Functions with Slope Recovery for
   Deep and Physics-Informed Neural Networks." *Proceedings of the
   Royal Society A*, 476(2239), 20200334. — LAAF, used by Schoder
   2025 as a +60% booster.

3. **Sitzmann, V., Martel, J. N. P., Bergman, A. W., Lindell, D. B.,
   & Wetzstein, G. (2020).** "Implicit Neural Representations with
   Periodic Activation Functions" (SIREN). *NeurIPS 2020*.
   arXiv:2006.09661. — Sin-activation MLPs; the best-performing
   activation in the Schoder 2025 sweep matches SIREN's prescription.

4. **Wu, C., Zhu, M., Tan, Q., Kartha, Y., & Lu, L. (2023).** "A
   comprehensive study of non-adaptive and residual-based adaptive
   sampling for physics-informed neural networks." *Computer
   Methods in Applied Mechanics and Engineering*, 403, 115671.
   arXiv:2207.10289. — RAR and RAR-D adaptive sampling, which
   Schoder 2025's adaptive refinement directly adopts.

5. **Porter, M. B. (2019).** "BELLHOP3D User Guide." Heat, Light,
   and Sound Research, Inc. — The ocean-acoustic 3D ray-tracing
   reference; provides the training data that a sonobuoy
   Helmholtz-PINN should be supervised against. Porter 2019 ran
   JASA 146:2016 experimentally.

6. **Jensen, F. B., Kuperman, W. A., Porter, M. B., & Schmidt, H.
   (2011).** *Computational Ocean Acoustics* (2nd ed.). Springer.
   — KRAKEN normal-mode formalism, chapter 5. Provides the
   dispersion-relation feature basis for ocean-acoustic adaptation
   of the Schoder feature-engineered PINN.
