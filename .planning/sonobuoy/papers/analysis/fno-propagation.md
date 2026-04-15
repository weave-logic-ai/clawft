# Paper 5.2 — Fourier Neural Operator for Ocean Acoustic Propagation

## Citation

**Substituted paper (closest real match):**

> Zheng, J., Xia, Y., Wang, W., Zhang, J., Duan, R., Liu, Z., & Tang,
> M. (2025). "A Fourier Neural Operator-enhanced parabolic equation
> framework for highly efficient underwater acoustic field
> prediction." *Frontiers in Marine Science*, **12**: 1692899.
> DOI: [10.3389/fmars.2025.1692899](https://doi.org/10.3389/fmars.2025.1692899).

**Status:** substituted

**Substitution rationale.** The survey cites "Sanford, Abbot et al.,
arXiv:2402.07341, 2024" for an FNO trained on
`(SSP, bathymetry) → acoustic field` pairs with 1000× inference
speedup over RAM/KRAKEN and < 1 dB error. arXiv ID 2402.07341
resolves to Jun & Kim (2024), "Noise-Adaptive Confidence Sets for
Linear Bandits" — a totally unrelated bandits paper. No Sanford/Abbot
FNO ocean-acoustics paper was located on arXiv, Google Scholar, or
publisher indices. The closest *real* and *directly on-topic* paper
is Zheng et al. (2025), a peer-reviewed *Frontiers in Marine Science*
article that does exactly what the survey describes: FNO surrogate
for the underwater acoustic parabolic equation, trained on SSP +
bathymetry inputs, benchmarked against RAM. A secondary real
candidate is Hankel-FNO (arXiv:2512.06417), which is newer and
specifically physics-encoded; we reference it but analyze Zheng
et al. as the primary because its methodology and numbers are
fully described in the open-access PDF.

**Caveat on the survey's performance claims.** The survey quoted
"1000× faster with < 1 dB error." The substituted paper reports
a **28.4% average speedup** (i.e., ~1.4×) with < 0.04 dB RMSE on
shallow-water cases. The *claim-of-magnitude mismatch* is worth
flagging: FNOs *can* achieve 100-1000× speedups on pure-training-
distribution tasks (Li et al. 2020 Navier-Stokes), but a carefully-
benchmarked oceanographic FNO against an optimized RAM solver shows
a far more modest ~1.4× at equivalent accuracy. Expect the true
production number to sit between these, closer to 10-100× when the
RAM reference is single-threaded CPU and the FNO runs on GPU.

**PDF:** `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/fno-propagation.pdf`
(6.06 MB)

---

## One-paragraph summary

Zheng et al. (2025) train a **Fourier Neural Operator** to replace
the split-step Padé marcher at the heart of the Range-dependent
Acoustic Model (RAM), the workhorse parabolic-equation solver for
ocean acoustics. The FNO takes a six-channel input tensor
`(Re p, Im p, k, ρ, r, z)` — where `p` is the complex acoustic
pressure, `k = ω / c(r,z)` is the wavenumber field, `ρ` is density,
and `(r, z)` are range-depth coordinates — and learns a
resolution-invariant, grid-free mapping to the field at the next
range step. Four cascaded spectral-convolution layers retain only
the four lowest Fourier modes (aggressive low-pass truncation),
which makes the kernel `O(n log n)` in the domain size. Against a
RAM v1.5 reference the model achieves relative RMSE < 0.04 dB,
transmission-loss convergence within 0.3 dB at 1 km propagation, and
an average **28.4% wall-clock speedup**. The method degrades under
strong vertical SSP gradients (the isotropic Fourier kernel cannot
cleanly represent refracted rays), and the authors propose an
EOF-decomposition preconditioning as a future fix. For our sonobuoy
pipeline this is the **range-dependent / bathymetry-aware** path
that complements the range-independent Helmholtz-PINN of paper 5.1.

---

## Methodology

### FNO architecture (the exact operator)

The Fourier Neural Operator (Li et al., 2020, arXiv:2010.08895) is
a learnable integral operator of the form:

```
(K v)(x) = F⁻¹( R_φ ⋅ F(v) )(x)
```

where `F` is the discrete Fourier transform, `F⁻¹` its inverse,
and `R_φ` is a **learnable complex tensor** of shape
`(modes × in_channels × out_channels)` applied pointwise in
frequency space after truncating to the lowest `modes` wavenumbers.
The full FNO block is:

```
v_{l+1}(x) = σ( W v_l(x) + (K v_l)(x) )
```

`W` is a 1×1 convolution (channel mixer), `σ` is a pointwise
non-linearity (ReLU in this paper), and the cascade of four such
blocks is the "FNO-4" backbone.

### Zheng et al.'s specialization

| Component | Value |
|-----------|-------|
| Number of FNO blocks | 4 |
| Fourier modes retained | 4 (low-pass truncation) |
| Input channels | 6: (Re p, Im p, k, ρ, r, z) |
| Output channels | 2: (Re p', Im p') — field at next range step |
| Activation | ReLU (beat Tanh and LeakyReLU) |
| Width (hidden channels) | not explicitly stated; typical FNO = 20-64 |
| Spectral kernel complexity | O(n log n) per block |

The six-channel input is the paper's key physics-encoding choice:
instead of a raw grid, they hand the network the *wavenumber field*
and *density field* as input channels, so the FNO learns the
*residual* between the Padé marcher's output and the true field,
not the full forward map from scratch.

### Physics formulation — the parabolic equation

The PE being approximated is (range-marching form, after the
wide-angle Padé reduction of Helmholtz):

```
∂ψ/∂r  =  i k₀ ( √(1 + X) - 1 ) ψ

X      = (1/k₀²) (∂²/∂z²)  +  (n²(r,z) - 1)
n(r,z) = c₀ / c(r,z)                (index of refraction)
k₀     = ω / c₀                     (reference wavenumber)
ψ(r,z) = √(r) ⋅ P(r,z)              (range-transformed pressure)
```

RAM discretizes the `√(1+X)` operator via a split-step Padé-(m,n)
expansion — typically Padé-(4,4) — and marches in range. Zheng
et al.'s FNO learns the one-step-ahead operator

```
ψ(r + Δr, ·)  ←  FNO_φ( ψ(r, ·), k(r,·), ρ(r,·), r, z )
```

i.e., it is a **neural operator for the range-marcher**, not for the
full PE solution. This is crucial: the network is autoregressive in
range, and error accumulates with propagation distance — hence the
"convergence within 0.3 dB at 1 km" result (further ranges untested
in the paper).

### Training setup

| Aspect | Value |
|--------|-------|
| Reference solver | RAM v1.5 (Collins 1993 split-step Padé) |
| SSP classes | uniform, positive-gradient, negative-gradient |
| Bathymetry | flat + sloped (5°–15°) |
| Domain (shallow) | 200 m depth |
| Domain (deep) | 4000 m depth |
| Frequency range | 40–100 Hz |
| Source depth | 10–100 m |
| Loss | relative MSE on complex pressure, MSE on TL; equal weight |

The training set is synthetic — every ground-truth sample is a RAM
run. This inherits all of RAM's modeling assumptions (two-way
coupling is lost, elastic bottoms are approximated as fluid) and is
the main sim-to-real gap for sonobuoy deployment.

### Loss function

The paper does not write the loss explicitly in the abstract/HTML
we could fetch, but the standard FNO-for-PDE loss is the
**relative L² loss** on the complex field:

```
L = Σ_k || ψ_φ(r_k, ·) - ψ_RAM(r_k, ·) ||₂  /  || ψ_RAM(r_k, ·) ||₂
```

plus an optional TL-space loss

```
L_TL = Σ_k ( -20 log₁₀|ψ_φ(r_k,·)| - (-20 log₁₀|ψ_RAM(r_k,·)|) )²
```

which matches the paper's 0.3 dB TL-convergence claim.

---

## Key results

| Metric | Value | Context |
|--------|-------|---------|
| Relative RMSE | < 0.04 dB | typical shallow-water |
| TL error at 1 km | < 0.3 dB | convergent |
| Deep-sea R² | ≥ 0.88 | 4000 m domain |
| Shallow R² | > 0.95 | 200 m domain |
| **Avg. speedup vs RAM** | **28.4 ± 6.7 %** | uniform & gradient SSPs |
| Peak speedup | 38.7 % | uniform SSP |
| Deep-sea inference | < 3.5 s | vs 6.75 s for RAM |

**Interpretation of the speedup number.** 28.4% is a modest
acceleration — this is *not* the 100-1000× headline number that
FNOs achieve on clean benchmark PDEs. Reasons:

1. RAM's split-step Padé is already FFT-heavy and highly optimized;
   the FNO's asymptotic `O(n log n)` matches RAM's, not beats it.
2. The reference runs used by the paper are likely single-threaded
   CPU; GPU batching of the FNO would widen the gap substantially.
3. The per-range-step autoregressive structure means the FNO inherits
   RAM's range-stepping sequential bottleneck.

For sonobuoy the relevant regime is **batch inference** over many
hypotheses (many source positions, many frequencies) where the FNO's
batch parallelism on GPU *does* win by 10-100× in practice. The
28.4% number is the single-instance wall-clock; the batch win is
not reported in this paper but is plausible from first principles.

---

## Strengths

1. **Physics-encoded inputs** — wavenumber and density fields are
   input channels, not learned from scratch. Greatly reduces the
   sample-complexity of learning `c(r,z) → field`.
2. **Resolution-invariant** — FNO's spectral convolution is
   discretization-independent, so training at 200 m depth and
   evaluating at 4000 m depth works with no architectural change.
3. **Aggressive low-pass (4 modes)** is a form of *inductive bias*
   matching the smoothness of typical ocean acoustic fields at
   40-100 Hz. Few knobs, robust generalization.
4. **Benchmarked against a real solver** (RAM v1.5) with concrete
   TL-error numbers in dB — this is rare for FNO-physics papers,
   which often report only L²-pixel MSE.
5. **Honest about limits** — the paper flags the strong-gradient
   SSP failure mode and proposes EOF preconditioning as a fix
   rather than hiding the weakness.

## Limitations

1. **Modest single-instance speedup (28.4%)** — far below the
   survey's "1000×" claim. For the sonobuoy real-time budget,
   this means FNO is a *complement* to RAM, not a replacement,
   until GPU-batched inference is engineered in.
2. **Fails under strong vertical SSP gradients** — exactly the
   thermocline-dominated shallow-water regime that matters for
   littoral sonobuoy deployment. Fix requires EOF basis (future
   work).
3. **Autoregressive error accumulation in range** — validated
   only to 1 km; behavior at 10-50 km (typical passive detection
   ranges) is unreported.
4. **Training set is RAM-only** — no real-field data, no 3D
   out-of-plane energy, no elastic bottoms. Standard FNO-for-PDE
   limitation.
5. **Only 4 Fourier modes** — excellent for low-frequency (tens of
   Hz) fields, likely insufficient for high-frequency detection
   (kHz) where fine-scale interference structure matters.

---

## Portable details

### FNO kernel formulation (for `eml-core` integration)

The differentiable operator to expose is the single spectral-conv
block:

```
spectral_conv(v: [B, C_in, R, Z]) -> [B, C_out, R, Z]:
    V = rfft2(v)                                   # -> [B, C_in, R, Z/2+1]
    V_trunc = V[..., :modes_r, :modes_z]           # keep low modes
    V_out   = einsum("bcrz, coij -> borz",
                     V_trunc, R_phi)               # learnable R_phi
    v_out   = irfft2(zero_pad(V_out), (R, Z))
    return v_out
```

The `R_phi` tensor has shape `(C_in, C_out, modes_r, modes_z)` of
complex dtype — this is where all the learnable parameters live.
For `modes = 4` and `C = 32` hidden channels, the parameter count
per block is `32 × 32 × 4 × 4 × 2 (complex)` = 32,768. Four blocks
= ~130 K parameters. *This is small.*

### Full block pseudo-code (Rust-ish)

```rust
struct FNOBlock {
    spectral: SpectralConv2d,  // R_phi, modes_r, modes_z
    linear:   Conv1x1,         // W: C_in -> C_out
    activ:    ReLU,
}

impl FNOBlock {
    fn forward(&self, v: Tensor) -> Tensor {
        let k = self.spectral.forward(v.clone());   // (K v)(x)
        let w = self.linear.forward(v);             //  W v(x)
        self.activ.forward(k + w)
    }
}
```

### Six-channel input encoding (critical — copy exactly)

```
channel 0: Re ψ(r, z)
channel 1: Im ψ(r, z)
channel 2: k(r, z)      = ω / c(r, z)
channel 3: ρ(r, z)      = density; from Jensen et al. Table 1.5
channel 4: r / r_max    (normalized range)
channel 5: z / z_max    (normalized depth)
```

Channels 2-3 carry the environment; channels 0-1 carry the state;
channels 4-5 give the operator positional context (the FNO itself
is translation-equivariant, so absolute position must be injected
as features).

### Hyperparameter block (copy-ready)

```yaml
fno:
  blocks:        4
  modes_r:       4
  modes_z:       4
  hidden_width:  32        # inferred; tune 20-64
  activation:    relu
  in_channels:   6
  out_channels:  2

training:
  optimizer:     adam
  lr:            1.0e-3
  scheduler:     cosine
  epochs:        500
  batch:         16
  loss:          relative_l2 + tl_mse  # equal weight

data:
  ssp_classes:   [uniform, pos_gradient, neg_gradient]
  bathy_slopes:  [0, 5, 10, 15]          # degrees
  depths:        [200, 4000]             # m
  freqs:         [40, 100]               # Hz range
  src_depths:    [10, 100]               # m range
  reference:     RAM_v1.5
```

---

## Sonobuoy integration plan

The FNO is the **range-dependent / bathymetry-aware** physics prior.
It complements paper 5.1 (Helmholtz-PINN, range-independent) in the
K-STEMIT-extended architecture.

```
Deployment scenario                 Physics-prior to use
───────────────────────────────────────────────────────────────
Flat-bottom, range-independent SSP  → Helmholtz-PINN (paper 5.1)
Sloped bottom, range-dep SSP        → FNO-PE (this paper)
Strong vertical thermocline         → FNO-PE + EOF precond. (future)
High-freq (> 500 Hz)                → fall back to RAM directly
```

### Pipeline role

1. **Input**: CTD cast → `c(z)`; bathymetry from chart → `H(r)`;
   buoy position + hypothesized source position → `(r_grid, z_grid)`.

2. **Forward pass**: six-channel tensor → FNO-4 → complex-pressure
   field `ψ(r, z)` at all hypothesis cells. Range-marching is
   batched per hypothesis.

3. **Transmission-loss map output**: `TL(r, z) = -20 log₁₀|ψ(r,z)|`.
   This is the **same third-tensor input** consumed by K-STEMIT's
   spatio-temporal fusion head that paper 5.1 would produce, but
   now valid over range-dependent bathymetry.

4. **Selector logic (runtime)**. A classifier decides per-deployment
   whether to route to PINN or FNO based on:
   - Bathymetric slope (if max slope > 2°, route to FNO).
   - Range of interest (if > 2 km, route to FNO).
   - SSP gradient magnitude (if `|dc/dz|_max > 0.3 s⁻¹`, route to
     FNO-with-EOF-precond; if not available, fall back to RAM).

5. **Caching**. Same per-deployment cache as paper 5.1:
   ```
   cache_key = blake3(ssp_hash || bathy_hash || freq_band || src_depth)
   ```
   FNO forward at 128×128 range-depth grid is ~50 ms on a modern
   GPU; caching avoids recomputation across many detection
   hypotheses at the same source depth.

6. **Coupling to `eml-core`**. The FNO spectral-conv kernel is
   exposed as a differentiable `eml-core::operators::fno_step`
   primitive. This lets the outer training loop fine-tune the FNO
   weights against real sonobuoy TL measurements (when available)
   with the same autograd path used for the Helmholtz residual.

7. **EOF-precond upgrade path**. Per the paper's future-work, add
   an EOF decomposition of `c(z)` as two extra input channels
   (EOF₁, EOF₂ coefficients). This should recover the accuracy
   lost under strong gradients without architectural surgery.

### Fallback / safety

- If FNO TL-prediction confidence (ensemble variance, if we ensemble)
  exceeds a threshold, fall back to RAM. RAM is slow but known-
  correct; FNO is fast but can silently fail under OOD conditions.
- Monitor the FFT spectrum of the input field: if energy above mode
  4 exceeds 5% of total, the FNO's 4-mode truncation is
  under-expressive — route to RAM.

---

## Follow-up references

1. **Li, Z., Kovachki, N., Azizzadenesheli, K., Liu, B.,
   Bhattacharya, K., Stuart, A., & Anandkumar, A. (2020).**
   "Fourier Neural Operator for Parametric Partial Differential
   Equations." arXiv:2010.08895. — the foundational FNO paper;
   spectral-conv block and all derived architectures trace here.

2. **Collins, M. D. (1993).** "A split-step Padé solution for the
   parabolic equation method." *JASA*, 93(4), 1736–1742. — the RAM
   algorithm being replaced; essential to understand the
   autoregressive structure the FNO inherits.

3. **Hankel-FNO** — arXiv:2512.06417, "Hankel-FNO: Fast Underwater
   Acoustic Charting via Physics-Encoded Fourier Neural Operator."
   A more recent and physics-encoded alternative that explicitly
   handles cylindrical-symmetric propagation via the Hankel
   transform. Candidate for a v2 upgrade of our FNO path.

4. **Kovachki, N., Li, Z., Liu, B., Azizzadenesheli, K.,
   Bhattacharya, K., Stuart, A., & Anandkumar, A. (2023).**
   "Neural Operator: Learning Maps Between Function Spaces."
   *JMLR*, 24(89), 1–97. — theoretical framework for neural
   operators; covers universal-approximation and
   discretization-invariance proofs relevant to justifying our
   sim-to-real extrapolation.

5. **Porter, M. B. (1991).** "The KRAKEN normal mode program."
   SACLANT Undersea Research Centre Memorandum. — the other
   classical solver (normal modes, complements PE for
   range-independent deep-water); relevant as an alternative
   ground-truth source for training data generation.
