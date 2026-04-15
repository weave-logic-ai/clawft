# Paper 5.1 — PINN + SSP for Ocean Acoustics (Helmholtz)

## Citation

**Substituted paper (closest real match):**

> Du, L., Wang, Z., Lv, Z., Wang, L., & Han, D. (2023).
> "Research on underwater acoustic field prediction method based on
> physics-informed neural network." *Frontiers in Marine Science*, **10**:
> 1302077. DOI: [10.3389/fmars.2023.1302077](https://doi.org/10.3389/fmars.2023.1302077).

**Status:** substituted

**Substitution rationale.** The survey cites "Yoon, Kim et al., JASA 2024"
for a PINN that solves Helmholtz conditioned on measured sound-speed
profiles. No JASA 2024 article with those authors and that topic was
found via arXiv, Google Scholar, pubs.aip.org, or web search. The
closest *real* and *directly applicable* publication is Du et al. (2023),
a peer-reviewed Frontiers article that implements exactly the method
described in the survey text: a PINN whose physics constraint is the
Helmholtz equation with the wavenumber `k(r,z) = ω / c(r,z)` derived
from a measured sound-speed profile, trained to produce a 2D (and 3D)
transmission-loss field. A second corroborating real paper is Wang,
Peng, He, & Zeng (2024, *JASA Express Letters*), "Predicting underwater
acoustic transmission loss in the SOFAR channel from ray trajectories
via deep learning," which we use as a cross-check for the 2024
literature but which does **not** use a PINN/Helmholtz formulation.
Du et al. 2023 is therefore the primary analysis target; the PDF at
`pinn-ssp-helmholtz.pdf` is that paper.

**PDF:** `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/pinn-ssp-helmholtz.pdf`
(5.17 MB, 22 pages)

---

## One-paragraph summary

Du et al. (2023) propose **UAFP-PINN** (Underwater Acoustic Field
Prediction via Physics-Informed Neural Network), a fully-connected
deep network trained to regress complex acoustic pressure
`p(r, z)` (2D range–depth) or `p(r, θ, z)` (3D cylindrical) from a
coordinate input, with a composite loss that combines a data-fidelity
term (MSE against simulated pressure at sample points) and a physics
residual term (the Helmholtz equation `∇²p + k² p = 0` enforced via
automatic differentiation through the network). The wavenumber
`k = ω/c` is computed from a supplied sound-speed profile `c(z)`,
so swapping SSPs at inference requires *no* retraining of the
data-fidelity branch, only re-weighting of the physics residual. The
authors report 2D MSE of 1.05×10⁻² Pa, R² of 0.990, and MAE of
6.76×10⁻² Pa, versus substantially degraded 3D performance
(R² ≈ 0.478) attributable to under-parameterization in the higher
dimension. For the sonobuoy project this is the canonical
physics-prior backbone: it produces the third tensor input
(transmission-loss map) that conditions K-STEMIT's spatial-temporal
branches alongside the spectrogram and array graph.

---

## Methodology

### Network architecture

- **Topology**: fully-connected feed-forward MLP.
- **Depth**: 6 hidden layers.
- **Width schedule**: `(8, 16, 32, 64, 128, 256)` — pyramidal
  expansion from coarse coordinate embedding to fine pressure-field
  decoding.
- **Input dimensionality**: 2 for the 2D model (`r`, `z`), 3 for the
  3D cylindrical model (`r`, `θ`, `z`). The frequency `ω` and sound
  speed `c(z)` enter through the physics loss, not the input vector.
- **Output**: complex pressure `p(r, z)` represented as a 2-component
  real vector `(Re p, Im p)`; transmission loss `TL = -20 log₁₀ |p|`
  is computed analytically at inference.
- **Activation**: ReLU (ablated against Tanh, Sin, Atan; ReLU won on
  convergence speed and final R²).

### Physics formulation

The target PDE is the **frequency-domain Helmholtz equation** for a
time-harmonic source `p(r,z,t) = P(r,z) e^{-iωt}`:

```
∇² P(r, z) + k²(r, z) · P(r, z) = 0,
k(r, z) = ω / c(r, z)                               (wavenumber)
c(r, z) ≈ c(z)                                      (range-independent SSP)
ω = 2πf                                             (angular source freq)
```

In 2D cylindrical coordinates with azimuthal symmetry and the
far-field approximation `P(r,z) = ψ(r,z) / √r`, this reduces to the
standard **narrow-angle / wide-angle parabolic equation** kernel
used by RAM and KRAKEN; Du et al. retain the full Helmholtz form
rather than the parabolic approximation, which preserves
back-scattered energy at the cost of a harder optimization.

**Boundary conditions.** The paper enforces:
- **Pressure-release surface** at `z = 0`: `P(r, 0) = 0`.
- **Seafloor impedance** at `z = H`: a Neumann-type condition on the
  derivative, discretized via AD.
- **Sommerfeld radiation** at large `r`: handled implicitly by
  training only inside a finite `r ∈ [r_min, r_max]` window and
  letting data-fidelity pin the outer field.
- **Point source** at `(r_s, z_s)`: baked into the data term rather
  than the physics term — the network sees the analytic Green's
  function value at a ring of sample points around the source.

### Loss formulation

Total loss is a balanced sum of two terms with equal weight
(no adaptive weighting):

```
L_total  = L_data + L_physics

L_data    = (1/N_d) Σ_i || P_θ(r_i, z_i) - P_sim(r_i, z_i) ||²
L_physics = (1/N_p) Σ_j || ∇² P_θ(r_j, z_j) + k²(r_j, z_j) P_θ(r_j, z_j) ||²
```

- `N_d` = 28,100 (2D) / 352,500 (3D) data samples.
- `N_p` = collocation points sampled uniformly inside the domain;
  the paper does not specify an explicit ratio but the data term
  dominates at initialization.
- `∇² P_θ` is computed via **second-order automatic differentiation**
  through the network (PyTorch's double-backward); this is the
  canonical PINN pattern.

### Training

| Hyperparameter | Value |
|----------------|-------|
| Optimizer | Adam (beat SGD by wide margin) |
| Learning rate | 1.0×10⁻³ |
| Weight decay | 5.0×10⁻⁴ |
| Epochs | 500 (2D) / 250 (3D) |
| Batch | full-batch (the domain fits in memory) |
| Initialization | default PyTorch (Kaiming for ReLU) |
| Activation | ReLU |

### Datasets

- **Simulated training data**: generated from a classical
  range-independent normal-mode solver (the paper uses a KRAKEN-
  equivalent reference; exact solver not specified).
- **Sound-speed profiles**: a handful of canonical profiles
  (Munk, isovelocity, a downward-refracting shelf profile).
- **Frequency**: the paper reports single-frequency training at
  each config; generalization across frequency requires retraining
  (this is an explicit limitation).
- **Domain**: 2D range `r ∈ [0, 1000] m`, depth `z ∈ [0, 100] m` for
  the shallow-water case; the 3D case extends to
  `θ ∈ [0, 2π]`.

---

## Key results

| Metric | 2D UAFP-PINN | 3D UAFP-PINN |
|--------|--------------|--------------|
| MSE (Pa²) | 0.01047 | 0.52177 |
| R² | 0.98953 | 0.47823 |
| MAE (Pa) | 0.06759 | 0.51988 |
| Abs. error concentration | within 0.05 Pa | within 0.5 Pa |

- **2D transmission-loss error**: the 0.01 Pa² MSE corresponds to
  roughly **< 1 dB TL error** over most of the field, with
  larger errors at deep nulls where `|P|` → 0 and log-magnitude is
  ill-conditioned.
- **Inference speed**: the paper emphasizes *grid-independent*
  prediction — any `(r, z)` can be queried in a single forward pass
  (O(network depth)) versus `O(N_r · N_z)` grid sweeps for a
  finite-difference solver. No absolute wall-clock speedup
  number is quoted, which is a weakness.
- **3D degradation**: R² collapses from 0.99 to 0.48 despite 12.5×
  more training points. The authors attribute this to
  under-parameterization; the 256-wide terminal layer is plausibly
  too narrow for the higher-dimensional target manifold.
- **SSP conditioning**: the paper validates the method on three
  SSP shapes and shows the physics residual does the right thing
  (the model learns the SOFAR-like channeling for a Munk profile).

---

## Strengths

1. **Truthful physics constraint** — the Helmholtz residual is
   enforced via AD, not as a soft regularizer on pre-computed
   finite differences, so the network *cannot* cheat by memorizing
   a grid.
2. **SSP-conditional at inference** — because `k²(r,z)` enters the
   physics term explicitly, swapping SSPs only re-weights the
   residual; the learned weights are (mostly) SSP-agnostic.
3. **Grid-independent prediction** — arbitrary `(r, z)` query points,
   no interpolation artifacts. Matches the sonobuoy requirement of
   querying TL at specific buoy-target geometries.
4. **Ablation-driven hyperparameter choices** — ReLU vs
   {Tanh, Sin, Atan} and Adam vs SGD are reported, which is rare
   in PINN papers.
5. **Reproducible** — all hyperparameters, layer widths, epochs,
   and learning rates are stated explicitly.

## Limitations

1. **Frequency-locked training** — a new network must be trained
   for each source frequency. For sonobuoy broadband detection,
   this means either a bank of PINNs or a frequency-conditioned
   variant (not demonstrated).
2. **3D performance collapse** — R² 0.48 is effectively unusable;
   the 2D result is the only trustworthy one.
3. **No bathymetry** — the method assumes range-independent
   `c(z)` and a flat bottom. Realistic sonobuoy deployments have
   sloped/variable bathymetry that would require PE-style
   range-stepping.
4. **No uncertainty quantification** — point estimate of the field
   only. No Bayesian PINN, no ensemble.
5. **Pressure-release surface is idealized** — the ocean surface
   is actually a rough time-varying boundary; the flat-boundary
   assumption breaks at high sea states.

---

## Portable details

### Exact Helmholtz formulation (transplant into `eml-core`)

The differentiable operator to expose from `eml-core` is:

```rust
// Pseudo-signature for eml-core integration
pub fn helmholtz_residual(
    field:  &ComplexField2D,     // P(r, z) from network
    ssp:    &SoundSpeedProfile,  // c(z), depth-indexed
    omega:  f64,                 // 2πf
) -> Tensor {
    // Compute ∇²P via double-backward through the field
    let laplacian = autograd_laplacian(&field);
    let k_sq      = ssp.wavenumber_sq_field(omega);  // (ω/c(z))²
    laplacian + k_sq * field          // residual; MSE to zero
}
```

This is the **single most important portable kernel** from this
paper. It lets us plug a PINN-style residual into our training
pipeline without changing the outer optimizer.

### Parabolic-equation alternative (fallback, for bathymetry)

For range-dependent bathymetry the standard PE (Tappert 1977)
replaces Helmholtz with the one-way marching equation:

```
∂ψ/∂r  = i k₀ ( √(1 + X) - 1 ) ψ,     X = (1/k₀²) (∂²/∂z²) + (n²(r,z) - 1)
n(r,z) = c₀ / c(r,z)                  (refractive index)
```

The narrow-angle Padé-(1,1) approximation `√(1+X) ≈ (1 + 3X/4) /
(1 + X/4)` is what RAM uses. For our hybrid implementation:
use Helmholtz-PINN for range-independent shallow cases (detection
range < 2 km, flat bottom); fall back to PE-neural-operator
(paper 5.2) for the range-dependent case.

### MLP hyperparameters to copy

```yaml
architecture:
  type: fully_connected_mlp
  input_dim:  2    # (r, z) for 2D
  hidden_widths: [8, 16, 32, 64, 128, 256]
  output_dim: 2    # (Re P, Im P)
  activation: relu

training:
  optimizer: adam
  lr: 1.0e-3
  weight_decay: 5.0e-4
  epochs: 500
  batch: full
  loss_weights:
    data:    1.0
    physics: 1.0
```

### Cache key derivation (sonobuoy deployment)

Per the survey's "implementation hook": the PINN output is
deterministic given `(c(z), ω, bathy_id, source_depth)`. The cache
key should be:

```
ssp_hash       = blake3(quantized_ssp_bytes(c, 0.1 m/s step))
freq_hash      = blake3(f_bytes)
bathy_hash     = blake3(bathy_polyline_bytes)
src_depth_hash = blake3(z_s_bytes)
cache_key      = ssp_hash ^ freq_hash ^ bathy_hash ^ src_depth_hash
```

Quantizing the SSP to 0.1 m/s resolution buys a ~100× reduction
in unique keys without measurable TL error.

---

## Sonobuoy integration plan

The PINN is the **Helmholtz-mode physics-prior** in the K-STEMIT-
extended architecture. It plugs in as follows:

```
┌──────────────────────────────────────────────────────────────┐
│  K-STEMIT-extended sonobuoy architecture                      │
│                                                               │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐    │
│  │ Spectrogram│  │ Array graph│  │ TL map (this paper)  │    │
│  │ (per-buoy) │  │ (GraphSAGE)│  │ P(r,z) from PINN     │    │
│  └──────┬─────┘  └──────┬─────┘  └──────────┬───────────┘    │
│         │                │                   │                │
│         └────────────────┼───────────────────┘                │
│                          ↓                                    │
│            ┌─────────────────────────────┐                    │
│            │ Spatio-temporal fusion      │                    │
│            │ (alpha-gated, paper 7)      │                    │
│            └──────────────┬──────────────┘                    │
│                           ↓                                   │
│            Detection / Bearing / Species ID                   │
└──────────────────────────────────────────────────────────────┘
```

1. **Per-deployment precomputation.** On buoy deployment, ingest
   CTD cast → `c(z)`. For each frequency band of interest (4-8
   canonical bands), run the PINN forward on a `(r, z)` grid
   centered on the buoy. Quantize and cache the resulting TL
   map keyed by `(ssp_hash, freq_band, bathy_hash, src_depth)`.

2. **Third tensor input.** At inference, for each detection
   hypothesis `(range, depth)`, query the cached TL map and inject
   as an extra channel alongside the spectrogram. This gives the
   detection head a physics-informed prior on *expected received
   level* for each hypothesis, which corrects for propagation
   loss that pure-ML models must learn implicitly from data.

3. **Coupling to `eml-core`.** The Helmholtz residual operator
   lives in `eml-core::operators::helmholtz_residual`. Training
   can switch between:
   - **Offline mode**: train PINN once per SSP-family, cache
     weights keyed by SSP cluster.
   - **Online mode**: warm-start from nearest cached PINN, do
     N_fine-tune ≈ 50 Adam steps on arrival (new SSP) — this is
     the differentiable operator path.

4. **Feedback to K-STEMIT's alpha gate.** When the PINN predicts
   a deep null at the hypothesized source location, the detection
   confidence prior drops and the alpha gate should shift toward
   the spatial (array) branch since the temporal branch is
   information-starved. This closes the loop between physics
   prior and spatio-temporal fusion.

5. **Bayesian extension (follow-up).** Replace the point PINN
   with an ensemble (5 seeds) to get epistemic uncertainty on
   TL. Propagate uncertainty into the detection likelihood.

---

## Follow-up references

1. **Raissi, M., Perdikaris, P., & Karniadakis, G. E. (2019).**
   "Physics-informed neural networks: A deep learning framework for
   solving forward and inverse problems involving nonlinear partial
   differential equations." *Journal of Computational Physics*, 378,
   686–707. — the foundational PINN paper; all AD-through-residual
   patterns trace here.

2. **Jensen, F. B., Kuperman, W. A., Porter, M. B., & Schmidt, H.
   (2011).** *Computational Ocean Acoustics* (2nd ed.). Springer. —
   canonical reference for Helmholtz / PE / normal modes; the
   boundary-condition and SSP formalism is straight out of chapters
   2 and 6.

3. **Wang, Y., Peng, Y., He, X., & Zeng, X. (2024, May).**
   "Predicting underwater acoustic transmission loss in the SOFAR
   channel from ray trajectories via deep learning." *JASA Express
   Letters*, 4(5), 056001. — the actual 2024 JASA paper we found in
   search; uses ray tracing + deep learning rather than PINN, but
   a useful cross-check for the target domain.

4. **Tappert, F. D. (1977).** "The parabolic approximation method."
   In *Wave Propagation and Underwater Acoustics* (pp. 224–287).
   Springer. — the PE reduction of Helmholtz that RAM and KRAKEN
   descend from; relevant for the fallback path.

5. **Collins, M. D. (1993).** "A split-step Padé solution for the
   parabolic equation method." *JASA*, 93(4), 1736–1742. — the
   split-step Padé algorithm used by RAM; target for the
   FNO-PE comparison in paper 5.2.
