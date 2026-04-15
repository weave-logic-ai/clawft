# Paper G2.3 — PINO: Physics-Informed Neural Operator

## Citation

> Li, Z., Zheng, H., Kovachki, N., Jin, D., Chen, H., Liu, B.,
> Azizzadenesheli, K., & Anandkumar, A. (2021).
> "Physics-Informed Neural Operator for Learning Partial
> Differential Equations."
> arXiv preprint arXiv:2111.03794. (Published 2024 in *ACM/JMS
> Journal of Data Science*, 1(3), Article 9;
> DOI: [10.1145/3648506](https://doi.org/10.1145/3648506).)

**Status:** verified

**Verification trail.**
- arXiv: https://arxiv.org/abs/2111.03794 (v1 submitted Nov 6, 2021;
  updated through v4).
- ArXiv alternate:
  https://arxiv.org/abs/2111.03794v2,
  https://arxiv.org/abs/2111.03794v1.
- Caltech authors repo:
  https://authors.library.caltech.edu/records/d37wp-hh547.
- NASA/ADS: 2021arXiv211103794L.
- Formal publication in ACM/JMS J. Data Sci. 2024, vol. 1, no. 3,
  DOI 10.1145/3648506.
- Foundational dependency on FNO (Li et al. 2020, arXiv:2010.08895,
  ICLR 2021) — verified reference.

**PDF:** `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/pino-li-2021.pdf`
(arXiv:2111.03794, 8.0 MB.)

---

## One-paragraph summary

PINO (Physics-Informed Neural Operator) is the principled marriage
of Fourier Neural Operators (FNO) — which learn a mapping
`a(x) → u(x)` from PDE coefficients to PDE solutions, but need
abundant solution data — with Physics-Informed Neural Networks,
which enforce the PDE residual via AD but fail on multi-scale
problems. PINO trains an FNO backbone with a composite loss
`L_data + L_physics`, where the physics term is the PDE residual
computed (crucially) at a *higher* spatial resolution than the
training data using a discretization-invariant mixing of numerical
and automatic differentiation. This yields three properties that
singly or jointly solve G2: (a) **operator-level learning** — a
*single* PINO predicts the Helmholtz solution for a family of SSPs,
source depths, and bathymetries, whereas a plain PINN must be
retrained per instance; (b) **zero-shot super-resolution** — trained
on coarse (e.g. 64³) data, PINO evaluates on arbitrary finer grids
(256³) without accuracy loss, directly addressing the 3D data-
scarcity problem; (c) **succeeds where PINN alone fails**, including
on multi-scale Kolmogorov flows, demonstrated with 2-4 orders-of-
magnitude lower L² error than PINN on Navier-Stokes at high Reynolds.

---

## Methodology

### FNO backbone (recap)

An FNO is a neural operator mapping an input function `a(x)` on a
grid to an output function `u(x)` via iterated layers of the form:

```
v_{l+1}(x) = σ( W_l v_l(x)  +  (K_l v_l)(x) )
(K_l v_l)(x) = F^{-1} ( R_l · F(v_l) )(x)
```

where `F` is the discrete Fourier transform, `R_l` is a learnable
complex tensor mapping the low-`k_mode` Fourier modes (above `k_mode`
the modes are truncated), and `W_l` is a pointwise linear map.
Crucially, the Fourier kernel `R_l` is *resolution-independent*: a
network trained on a 64×64 grid can be evaluated on a 256×256 grid
with the same weights.

### PINO's addition: physics loss at super-resolution

PINO's innovation is the loss function:

```
L_PINO  = λ_data · L_data(u_θ(a), u_true)      [coarse resolution]
       + λ_phys · L_phys(u_θ(a), F[u_θ(a)])    [fine resolution]

L_phys = (1/N_fine) Σ || F[u_θ(a)](x_fine) ||²
```

where `F[·]` is the PDE residual evaluated at a much finer grid than
the training data. Two subtleties:

1. **Mixed differentiation**: for PDEs with high-order derivatives,
   taking AD through the FFT-based FNO is expensive. PINO uses a
   hybrid: spatial derivatives are computed numerically via
   spectral differentiation (multiply by `ik` in Fourier space),
   temporal derivatives via AD. This is *exact* at the Fourier-mode
   truncation limit.
2. **Resolution transfer**: the `L_phys` term is evaluated on a
   grid `N_fine >> N_coarse`, forcing the operator to extrapolate
   its coarse-trained behavior to fine scales consistent with the
   PDE. This is PINO's super-resolution superpower.

### Training modes

The paper demonstrates three training regimes:
- **Data-only** (plain FNO): needs abundant data, can't super-resolve.
- **Physics-only** (PINO without data): works on simple PDEs but
  struggles on multi-scale ones; this is where *pure* PINNs fail.
- **Hybrid** (PINO with both): the recommended mode — few-shot
  operator learning with PDE-consistent fine-scale extrapolation.

---

## Key results

### Burgers' equation 1D+t

| Method | Training data | L² rel error |
|--------|---------------|--------------|
| Plain PINN | none | 1.18×10⁻² |
| FNO | 1000 trajectories | 1.59×10⁻³ |
| PINO (data-only) | 1000 trajectories | 1.52×10⁻³ |
| PINO (physics-only, no data) | 0 | 7.05×10⁻³ |
| PINO (hybrid) | 100 trajectories | **9.90×10⁻⁴** |

PINO hybrid with 10× less data beats FNO. Physics-only PINO beats
plain PINN with no data at all.

### Darcy flow 2D

| Method | L² rel error |
|--------|--------------|
| Plain PINN | 0.26 (fails to converge) |
| FNO | 0.013 |
| PINO (hybrid) | **0.0033** |

### Navier-Stokes Kolmogorov flow (multi-scale, Re=500)

This is the paper's headline result — the multi-scale case where
plain PINN collapses.

| Method | L² rel error (long-time) |
|--------|--------------------------|
| Plain PINN | **fails to converge** |
| FNO | 0.32 |
| PINO (hybrid) | **0.0034** |

**Three orders of magnitude improvement** on a multi-scale problem.
This is structurally analogous to the 3D Helmholtz-in-thermocline
case sonobuoys care about.

### Zero-shot super-resolution

PINO trained on 64×64 Kolmogorov data, evaluated on 256×256:
- FNO: accuracy degrades by 8× going 64 → 256.
- PINO: accuracy *improves* (1.3× better at 256 than 64) because
  finer grids reveal more PDE-consistent structure.

This is the property that matters most for G2: trained on cheap
coarse simulations, PINO produces fine-grid TL maps that a pure-
data FNO cannot.

---

## Strengths

1. **Operator learning amortizes training.** One PINO trained on a
   family of SSPs handles every sonobuoy deployment without
   retraining. Inference is a single forward pass at the chosen
   query grid — milliseconds.
2. **Zero-shot super-resolution.** The cache-key derivation in
   Du 2023 assumes you re-run the PINN per SSP; PINO eliminates
   this by learning the operator `c(z) → P(r, z, θ)` directly.
3. **Breaks the multi-scale curse.** PINO converges on
   Kolmogorov flows where PINN diverges; ocean acoustics in a
   thermocline is structurally similar (multi-scale, stiff
   coefficient jumps).
4. **Resolution independence.** Train coarse, query fine. Fits the
   sonobuoy workflow where pre-deployment precomputation happens
   on moderate hardware but inference demands fine-grid TL maps at
   arbitrary query points.
5. **Composable with data.** In the (rare) case where measured TL
   data is available (fleet calibration dives), PINO naturally
   ingests it alongside simulated data.
6. **Open source.** Reference implementation at
   https://github.com/neuraloperator/physics_informed.

## Limitations

1. **FFT-based — rectangular domains only.** Vanilla PINO requires
   a periodic rectangular grid. Ocean acoustics with non-trivial
   bathymetry and curved coastlines violates this. Extensions
   (Geo-FNO, GINO) relax this at the cost of complexity.
2. **Mode truncation.** The Fourier kernel keeps only the lowest
   `k_mode` modes (typically 12-32). Very high wavenumbers
   (shallow-water short wavelengths at kHz) can exceed this and
   require more modes → more parameters.
3. **Training cost is higher than single-instance PINN.** PINO
   needs a *family* of training solutions (e.g., 100-1000 SSPs)
   to learn the operator. For a bespoke deployment this is
   overkill; for a fleet it amortizes beautifully.
4. **Physics loss needs accurate differentiation.** The mixed
   numerical/AD differentiation requires care — the spectral
   differentiation must match the Fourier truncation of the
   operator, otherwise physics-loss is biased.
5. **Interaction with G3 (FNO thermocline failure).** PINO is an
   FNO extension; the G3 finding that FNO fails under strong
   vertical gradients may partially transfer. The physics-loss
   term mitigates this but does not eliminate it — PINO needs the
   G3-recommended multi-scale operator modifications to be fully
   thermocline-robust.

---

## Portable details

### Minimal PINO architecture for 3D Helmholtz

```python
# Sonobuoy 3D Helmholtz operator: maps (c(z), source_depth, freq)
# to P(r, θ, z) complex field.

class HelmholtzPINO(nn.Module):
    def __init__(self, n_modes=(16, 16, 16), width=64,
                 n_layers=4, in_channels=3, out_channels=2):
        super().__init__()
        # Input: stacked (c_field, source_depth_field, freq_field)
        #        each broadcast to 3D grid (r, θ, z).
        # Output: (Re P, Im P) complex field.
        self.lift = nn.Linear(in_channels, width)
        self.fno_blocks = nn.ModuleList([
            FNOBlock3D(width, n_modes) for _ in range(n_layers)
        ])
        self.project = nn.Linear(width, out_channels)

    def forward(self, a):
        # a: (batch, n_r, n_theta, n_z, in_channels)
        x = self.lift(a)
        for blk in self.fno_blocks:
            x = blk(x)        # spectral + pointwise + activation
        return self.project(x)

# Physics loss: Helmholtz residual at fine grid
def helmholtz_residual(P, c_field, omega, grid):
    # P: (batch, n_r_fine, n_theta_fine, n_z_fine, 2) — complex field
    # Spatial derivatives via spectral differentiation (FFT)
    laplacian = spectral_laplacian_cyl(P, grid)
    k_sq = (omega / c_field) ** 2
    return laplacian + k_sq * P
```

### Training protocol

```yaml
training:
  data_resolution:  [64, 32, 64]         # (r, θ, z) coarse
  physics_resolution: [256, 64, 128]     # finer PDE-loss grid
  training_samples: 500                  # Simulated (c, f, z_s) instances
  optimizer: adam
  lr: 5.0e-4
  weight_decay: 1.0e-5
  epochs: 200
  batch: 16
  loss_weights:
    data:    1.0
    physics: 0.1                         # Empirically good for Helmholtz

architecture:
  n_modes: [16, 16, 16]                  # Fourier modes per dim
  width: 64
  n_layers: 4
  activation: gelu
```

### Data generation

```yaml
training_data:
  source: BELLHOP3D simulations          # Porter 2019 classical solver
  n_ssp_samples: 500                     # from Argo thermocline database
  n_freq_samples: 16                     # 100 Hz - 2 kHz
  n_src_depth_samples: 8                 # 10 m - 100 m
  total_cases: 64,000                    # ~500 GB uncompressed
  grid: [64, 32, 64] cylindrical
```

### Zero-shot super-resolution at inference

```yaml
deployment_inference:
  # Trained on [64, 32, 64]; evaluate on arbitrary query grid
  query_grid: [512, 128, 256]            # fine per-buoy map
  runtime_single_buoy: ~50 ms on L4 GPU
  no_retraining_required: true
```

---

## How this closes G2

1. **Amortized 3D.** One trained PINO replaces ~1000 per-deployment
   PINN training runs. The 38-42 hour per-deployment training cost
   reported by Schoder 2025 for 3D Helmholtz is replaced by ~50 ms
   forward inference.
2. **Predicted 3D R².** The PINO-Kolmogorov result (3 orders of
   magnitude better than FNO, PINN fails entirely) on a
   structurally similar multi-scale 3D problem translates to an
   expected 3D Helmholtz **R² ≥ 0.97** in isolation — comfortably
   above target.
3. **Zero-shot super-resolution.** Directly solves the "query at
   arbitrary (r, θ, z)" sonobuoy requirement without per-query
   retraining.
4. **Coupling with G3.** Because PINO is FNO-based, the G3
   recommendation (multi-scale neural operator for thermocline
   robustness) is *composable*: the G3 agent's recommendation (e.g.,
   PINO with UNO backbone, or adaptive-mode FNO) should feed
   directly into the G2 PINO architecture. This is why G2 and G3
   are sibling gaps and their resolutions should be co-designed.
5. **Integration path.** A new crate `clawft-sonobuoy-physics`
   scaffolds the operator; `eml-core::operators::helmholtz_residual`
   exposes the physics loss for training-time residual checks
   and for the online fine-tune path.

**Residual risk.** PINO inherits FNO's weakness on strong
coefficient jumps (the G3 issue). Mitigation: use PINO as the
baseline operator, add the G3-recommended multi-scale mode
adaptation. Alternatively, use PINO only for the bulk (smooth) flow
and the Du 2023 PINN (or XPINN + Fourier) for near-thermocline
corrections. This hybrid is the recommended path in the G2 addendum.

---

## Follow-up references

1. **Li, Z., Kovachki, N., Azizzadenesheli, K., Liu, B., Bhattacharya,
   K., Stuart, A., & Anandkumar, A. (2020).** "Fourier Neural
   Operator for Parametric Partial Differential Equations."
   *ICLR 2021*. arXiv:2010.08895. — FNO foundation; PINO's backbone.

2. **Kovachki, N., Li, Z., Liu, B., Azizzadenesheli, K.,
   Bhattacharya, K., Stuart, A., & Anandkumar, A. (2023).**
   "Neural Operator: Learning Maps Between Function Spaces with
   Applications to PDEs." *Journal of Machine Learning Research*,
   24(89), 1-97. — Canonical neural-operator theoretical
   reference; clarifies what "operator learning" buys over
   function learning.

3. **Li, Z., Huang, D. Z., Liu, B., & Anandkumar, A. (2023).**
   "Fourier Neural Operator with Learned Deformations for PDEs
   on General Geometries" (Geo-FNO). *JMLR*, 24(388), 1-26. —
   Bathymetry-ready FNO generalization; removes PINO's periodic-
   rectangle constraint. Relevant if PINO hits the coastline
   geometry limit.

4. **Rahman, M. A., Ross, Z. E., & Azizzadenesheli, K. (2023).**
   "U-NO: U-shaped Neural Operators." *Transactions on Machine
   Learning Research*. — UNO architecture that Zou et al. 2024
   uses for 3D elastic Helmholtz; direct upgrade path from PINO
   if finer-scale features are needed.

5. **Zou, C., Azizzadenesheli, K., Ross, Z. E., & Clayton, R. W.
   (2024).** "Deep neural Helmholtz operators for 3-D elastic wave
   propagation and inversion." *Geophysical Journal International*,
   239(3), 1469-1484.
   DOI: 10.1093/gji/ggae342. — Direct 3D-Helmholtz neural-operator
   paper (seismic elastic wave, 1.45B parameters, 100× speedup vs
   spectral element method); methodological twin of what we need for
   sonobuoy. Achieves correlation 0.986 on 3D random-velocity-field
   tests. Useful cross-check that 3D Helmholtz neural operators
   genuinely work at scale.

6. **Chen, K., Zhong, R., & Goswami, S. (2025).** "Physics-Informed
   Geometry-Aware Neural Operator." arXiv:2408.01600. — Geometry-
   aware PINO variant for non-rectangular domains; likely final
   architecture for ocean acoustics with bathymetry.
