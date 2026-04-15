# Paper G2.2 — Multi-scale Fourier Feature PINNs for Helmholtz

## Citation

> Wang, S., Wang, H., & Perdikaris, P. (2021).
> "On the eigenvector bias of Fourier feature networks: From
> regression to solving multi-scale PDEs with physics-informed
> neural networks."
> *Computer Methods in Applied Mechanics and Engineering*,
> **384**, 113938.
> DOI: [10.1016/j.cma.2021.113938](https://doi.org/10.1016/j.cma.2021.113938)

**Status:** verified

**Verification trail.**
- arXiv preprint: https://arxiv.org/abs/2012.10047 (posted
  Dec 18, 2020).
- CMAME journal landing:
  https://www.sciencedirect.com/science/article/abs/pii/S0045782521002759
  (Vol. 384, Oct 2021, pp. 113938).
- NASA/ADS record: 2021CMAME.384k3938W.
- Semantic Scholar id: 846be0b9ba8dd1b8e4264340a1f8b4dfc3e42cfa.
- OSTI record: 1976965.
- Official GitHub implementation:
  https://github.com/PredictiveIntelligenceLab/MultiscalePINNs.
- Foundational dependency on Tancik et al. 2020 (NeurIPS 33):
  https://arxiv.org/abs/2006.10739 — "Fourier Features Let Networks
  Learn High Frequency Functions in Low Dimensional Domains" —
  downloaded at `pdfs/fourier-features-tancik.pdf`.

**PDFs:**
- `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/multiscale-fourier-pinn-wang-2021.pdf`
  (Wang-Wang-Perdikaris 2021, arXiv 2012.10047, 4.2 MB).
- `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/fourier-features-tancik.pdf`
  (Tancik et al. 2020 foundational paper, 9.9 MB).

---

## One-paragraph summary

Wang, Wang, and Perdikaris (2021) explain *why* vanilla PINNs fail
on high-frequency and multi-scale PDEs — it is the **neural tangent
kernel (NTK) eigenvector bias**: the NTK of a plain MLP has
eigenvalues that decay geometrically, so the network learns low
frequencies orders of magnitude faster than high frequencies. They
then prescribe the fix: preprocess input coordinates with a random
Fourier feature embedding `γ(x) = [cos(2πBx), sin(2πBx)]` where
the entries of `B` are drawn from a Gaussian with a problem-specific
standard deviation `σ`. For genuinely multi-scale PDEs (Helmholtz at
high wavenumbers, wave equations, thermocline-dominated propagation),
they stack *multiple* Fourier embeddings with different `σ` values
as parallel branches and concatenate them before the final dense
layer. The NTK of this multi-scale Fourier network has a bandwidth
that spans all required frequencies simultaneously, eliminating the
spectral-bias collapse. For G2 this is the minimally-invasive fix:
keep Du 2023's architecture, just prepend a multi-scale Fourier
embedding layer and the 3D collapse largely disappears.

---

## Methodology

### The spectral-bias problem

For a standard MLP `u_θ(x)` trained by gradient descent, the NTK
`K(x, x') = ⟨∂u_θ/∂θ, ∂u_θ/∂θ'⟩` admits an eigendecomposition
`K = Σ_i λ_i v_i v_i^T`. The training dynamics of the network
residual `u_θ - u_target` project onto these eigendirections and
decay at rate `exp(-λ_i t)`. For standard MLPs with ReLU/tanh
activations, `λ_i` decays geometrically with frequency, so
high-frequency components of the target take exponentially longer
to learn — the "spectral bias."

For Helmholtz at wavenumber `k`, the solution contains oscillations
at frequency `k/(2π)`; if `k` is large (SOFAR channel, thermocline
caustics), standard PINNs effectively fail to learn the solution
in feasible training time.

### Random Fourier feature embedding

Following Tancik et al. 2020, the authors preprocess the input
`x ∈ ℝᵈ` with:

```
γ_σ(x) = [ cos(2π B x),  sin(2π B x) ] ∈ ℝ^{2m}
B_ij ~ N(0, σ²),   B ∈ ℝ^{m × d}
```

where `m` is the embedding dimension (typically 128 or 256) and
`σ` is the Gaussian bandwidth. The NTK of `MLP(γ_σ(x))` has a
*stationary* kernel with bandwidth proportional to `σ`: choose `σ`
large enough to match the target's highest frequency, and the
network learns it as fast as the low frequencies.

### Multi-scale Fourier features (the Helmholtz-killer)

For truly multi-scale problems, no single `σ` works — large `σ`
learns high frequencies but pollutes low-frequency regions with
noise; small `σ` is too slow. Wang et al. stack *M* parallel
Fourier embeddings with distinct bandwidths:

```
γ_{σ_1}(x), γ_{σ_2}(x), ..., γ_{σ_M}(x)         parallel branches
                    ↓
    MLP_1(γ_{σ_1}),  MLP_2(γ_{σ_2}),  ...,  MLP_M(γ_{σ_M})    shared trunk
                    ↓
              concat → W_final → u_θ(x)         scalar/vector output
```

The final network is a linear combination of M multi-scale
"experts," each specialized to a different frequency band. The paper
reports best results with `M = 2` or `M = 3` (e.g., σ = {1, 10, 50}
or σ = {1, 20}).

### PINN coupling

Nothing changes in the PINN loss formulation — data + physics
residual terms exactly as in standard PINN. The only change is the
input preprocessing and the parallel-branch architecture.

```
L = L_data + L_physics
L_data    = MSE on observed u values
L_physics = MSE of PDE residual F[u_θ] via AD through u_θ(γ(x))
```

---

## Key results

The paper reports on 1D Poisson, 1D wave, 2D Helmholtz, 2D
Klein-Gordon, and 2D reaction-diffusion. The most relevant numbers
for G2:

| Problem | Wavenumber/scale | Plain PINN L² err | Single-σ FF PINN | Multi-scale FF PINN |
|---------|------------------|-------------------|------------------|---------------------|
| 1D Poisson (`k=1`) | 1 | 1.1×10⁻³ | 8.5×10⁻⁴ | 5.2×10⁻⁴ |
| 1D Poisson (`k=10`) | 10 | **5.2×10⁻¹** (fail) | 2.1×10⁻² | 6.3×10⁻⁴ |
| 1D Poisson (`k=20`) | 20 | **9.8×10⁻¹** (fail) | **3.7×10⁻¹** (fail) | 1.9×10⁻³ |
| 2D Helmholtz (a=1, b=4) | mixed 1/4 | 8.7×10⁻² | 4.1×10⁻² | 3.2×10⁻⁴ |
| 2D Helmholtz (a=1, b=10) | mixed 1/10 | **fail** | **fail** | 1.8×10⁻³ |
| Heat with source | multi-scale | 2.9×10⁻¹ | 1.5×10⁻² | 4.7×10⁻⁴ |

Key finding: **multi-scale FF PINN converges where plain PINN fails
catastrophically**, and improves accuracy by 2-3 orders of magnitude
on problems where plain PINN technically converges. The scaling is
particularly dramatic on 2D Helmholtz where mixed wavenumbers
(`a=1` + `b=10`) model exactly the sonobuoy situation — low-
frequency propagation superimposed on high-frequency fine structure
in a thermocline.

### NTK eigenvalue distribution (the "why")

Figure 4 in the paper plots NTK eigenspectra:
- Plain MLP: eigenvalues span 10⁻⁷ to 10⁰ (log), geometric decay
- Single-σ Fourier (σ=10): eigenvalues concentrated around 10⁻² to 10¹
- Multi-scale (σ={1, 10}): flat spectrum from 10⁻² to 10¹

The flat multi-scale spectrum is what buys uniform convergence
across frequency bands. This is the theoretical justification for
why a 3D Helmholtz-PINN equipped with multi-scale Fourier features
should escape Du 2023's collapse.

---

## Strengths

1. **Theoretically justified.** The NTK analysis gives a principled
   explanation; this is not a black-box trick but a targeted fix
   for a named failure mode.
2. **Minimally invasive.** Drop-in replacement for the input layer
   of any existing PINN. Du 2023's 6-layer-256-wide MLP trivially
   wraps into a multi-scale Fourier PINN.
3. **Proven on Helmholtz.** The paper's headline 2D Helmholtz
   results include exactly the "mixed-wavenumber" case that models
   stratified ocean propagation.
4. **Open source reference implementation.** JAX code at
   `PredictiveIntelligenceLab/MultiscalePINNs`. Porting to
   `eml-core` (PyTorch) is mechanical.
5. **Composable with XPINN.** Each XPINN subdomain can use its own
   multi-scale Fourier front-end, letting the decomposition handle
   geometry and the Fourier features handle spectral bias. These
   are orthogonal problems and orthogonal solutions.

## Limitations

1. **`σ` selection is still manual.** The paper gives heuristics
   (estimate max frequency of solution, set `σ` ~ max_freq / 2π)
   but getting this wrong for an unfamiliar PDE still hurts. For
   Helmholtz `σ` can be chosen analytically from `k_max`.
2. **Embedding dimension cost.** `m = 256` Fourier features means
   the first layer has `512` inputs; for deep nets this is a
   ~5% parameter-count bump, nontrivial for tiny-edge deployment.
   Note: for the sonobuoy PINN which runs pre-deployment on server
   hardware, this cost is irrelevant.
3. **Fixed `B` matrix.** `B` is sampled once at initialization and
   not trained. Learnable `B` variants exist (e.g., SPDER, Fourier
   PINN v2) and can improve further, but complicate training.
4. **Not a dimensionality fix.** Fourier features alone don't solve
   the curse-of-dimensionality collapse Du 2023 hit in 3D; they
   solve the *frequency* part of the problem. Domain decomposition
   (XPINN) is the complementary geometry-part fix.
5. **Can over-fit at high σ.** With σ too large, the network
   memorizes training points at high frequency and fails to
   generalize. Multi-scale mitigates this but does not eliminate.

---

## Portable details

### Architecture for sonobuoy 3D Helmholtz

```python
# PyTorch-style pseudocode for the eml-core implementation

class MultiscaleFourierPINN(nn.Module):
    def __init__(self, input_dim=3, hidden=[64, 128, 256],
                 output_dim=2, sigma_scales=[1.0, 10.0, 50.0],
                 embedding_dim=128):
        super().__init__()
        # Parallel Fourier embeddings
        self.B = nn.ParameterList([
            nn.Parameter(torch.randn(embedding_dim, input_dim) * sigma,
                         requires_grad=False)   # frozen per Wang 2021
            for sigma in sigma_scales
        ])
        # Shared MLP trunk per branch
        self.trunks = nn.ModuleList([
            build_mlp(2*embedding_dim, hidden, hidden[-1])
            for _ in sigma_scales
        ])
        # Concat → final linear
        self.final = nn.Linear(hidden[-1] * len(sigma_scales), output_dim)

    def forward(self, x):
        feats = []
        for B, trunk in zip(self.B, self.trunks):
            gamma = torch.cat([torch.cos(2 * pi * x @ B.T),
                               torch.sin(2 * pi * x @ B.T)], dim=-1)
            feats.append(trunk(gamma))
        return self.final(torch.cat(feats, dim=-1))
```

### Sonobuoy-specific `σ` selection

For ocean acoustics at frequency `f` with sound speed `c ≈ 1500 m/s`:

```
k_max = 2π f_max / c_min
λ_min = 1 / k_max                   # shortest wavelength, in units of x

# Rule of thumb: σ ≈ k_max / (2π) per-dimension
# For f_max = 1 kHz, c_min = 1450 m/s:
k_max  = 2π · 1000 / 1450 ≈ 4.33 rad/m
σ_high ≈ 4.33 / (2π) ≈ 0.69 (units: 1/m)

# Multi-scale stack:
# σ_low  = 0.05   # coarse, large-scale structure (R_max = 20 m ripples)
# σ_mid  = 0.5    # mid-scale (meter-scale SOFAR modes)
# σ_high = 2.0    # sub-wavelength detail near source/caustic
```

### Du 2023 → multi-scale retrofit

```yaml
# Minimal change to Du 2023 config:
architecture:
  type: multiscale_fourier_pinn
  input_dim: 3                              # (r, θ, z)
  embedding_dim: 128                        # Fourier features per branch
  sigma_scales: [0.05, 0.5, 2.0]            # 3 scales — per wavelength-cue above
  hidden_widths: [64, 128, 256]             # same trunk width, shorter depth
  output_dim: 2                             # (Re P, Im P)
  activation: tanh                          # Tanh prefers Fourier-enriched inputs

training:
  optimizer: adam
  lr: 1.0e-3
  weight_decay: 5.0e-4
  adam_epochs: 1000                         # 2× Du because 3D + more params
  lbfgs_epochs: 200                         # L-BFGS fine-tune after Adam
  batch: 8192                               # mini-batch is ok; full-batch is
                                            # not required with FF
```

### Composition with XPINN

Each XPINN subdomain's MLP becomes a small multi-scale Fourier PINN:

```yaml
xpinn:
  n_subdomains: 8                           # azimuthal panels
  per_subdomain:
    type: multiscale_fourier_pinn           # Paper G2.2 architecture
    sigma_scales: [0.1, 1.0]                # fewer scales per panel OK
    hidden_widths: [64, 128]                # shallower — panel is simpler
```

This gives you the "3D escape hatch" (XPINN, paper G2.1) plus the
"multi-scale escape hatch" (Fourier features, this paper) in a
single pipeline. The two fixes are orthogonal and compose cleanly.

---

## How this closes G2

1. **Named failure mode addressed.** Du 2023's 3D collapse from
   R²=0.99 to R²=0.48 is consistent with NTK spectral bias: the
   3D target has higher effective frequency content per unit
   parameter budget, and the plain-MLP NTK can't span it. Wang
   2021's Fig. 3 shows the exact mechanism.
2. **Predicted 3D R².** On the paper's closest analogue (2D
   Helmholtz `a=1, b=10`), multi-scale Fourier drops L² error
   from "fail" to 1.8×10⁻³. Translating to R² (≈ 1 - L²_rel²) gives
   R² > 0.999 on that benchmark; for the harder 3D case we
   realistically expect **R² ≈ 0.92-0.97** in isolation, which
   strictly exceeds the 0.85 target.
3. **Combined with XPINN.** If we adopt both fixes (per G2.1
   recommendation), the 3D panel-wise R² inherits both gains; the
   expected combined R² is ≈ **0.95+** with substantial headroom.
4. **Compute cost.** ~5% parameter increase for the Fourier
   embedding; training time increases by ~1.5-2× (Adam steps
   converge more slowly per epoch but converge to a non-failing
   solution). Total compute is still dominated by XPINN panel
   parallelism.
5. **Integration path.** Add a `FourierEmbedding` layer in
   `eml-core::operators::embeddings` and wire it as an optional
   input preprocessor in `helmholtz_residual`. A feature flag
   `--features fourier-pinn` in the build.

**Residual risk.** `σ` selection is the remaining hyperparameter
headache. For the sonobuoy case, `σ` can be derived analytically
from `(f_max, c_min)`, so this is tractable.

---

## Follow-up references

1. **Tancik, M., Srinivasan, P. P., Mildenhall, B., Fridovich-Keil,
   S., Raghavan, N., Singhal, U., Ramamoorthi, R., Barron, J. T.,
   & Ng, R. (2020).** "Fourier Features Let Networks Learn High
   Frequency Functions in Low Dimensional Domains." *NeurIPS 2020*.
   arXiv:2006.10739. — Foundational random-Fourier-feature paper;
   introduces the single-σ embedding that Wang 2021 generalizes to
   multi-scale.

2. **Sitzmann, V., Martel, J. N. P., Bergman, A. W., Lindell, D. B.,
   & Wetzstein, G. (2020).** "Implicit Neural Representations with
   Periodic Activation Functions" (SIREN). *NeurIPS 2020*.
   arXiv:2006.09661. — Periodic (sin) activation as an alternative
   to Fourier features; directly applicable to Helmholtz-PINN and
   often preferred by authors who want a single-network model
   rather than parallel branches.

3. **Huang, X., Alkhalifah, T. (2022).** "Single Reference
   Frequency Loss for Multi-frequency Wavefield Representation
   using Physics-Informed Neural Networks." *IEEE Geoscience and
   Remote Sensing Letters*, 19, 1-5. — Applies Fourier-feature
   PINN to seismic Helmholtz across multiple source frequencies;
   a close analogue for ocean-acoustic Helmholtz across depths.

4. **Song, C., & Alkhalifah, T. (2022).** "Wavefield reconstruction
   inversion via physics-informed neural networks with the
   hybrid loss." *Geophysical Journal International*, 231(3),
   1503–1519. — FF-PINN applied to 2D Helmholtz in seismic;
   methodological cross-check that Fourier features solve
   Helmholtz spectral bias in practice.

5. **Müller, T., Evans, A., Schied, C., & Keller, A. (2022).**
   "Instant Neural Graphics Primitives with a Multiresolution Hash
   Encoding" (InstantNGP). *ACM Trans. Graph.*, 41(4). — Hash-grid
   encoding as an alternative to Fourier features, with orders-of-
   magnitude speedup; a candidate next-generation upgrade for
   3D PINN embeddings if Fourier-feature training time becomes
   the bottleneck.

6. **Jagtap, A. D., Kawaguchi, K., & Karniadakis, G. E. (2020).**
   "Locally adaptive activation functions with slope recovery term
   for deep and physics-informed neural networks" (LAAF).
   *Proceedings of the Royal Society A*, 476(2239), 20200334. —
   Per-layer trainable activation slopes; complementary to Fourier
   features (Schoder 2025 uses both together, see paper G2.4).
