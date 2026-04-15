# Paper 5.3 — Thermocline-aware FiLM Detection

## Citation

**Substituted papers (closest real matches):**

> **Primary (architecture substrate):**
> Perez, E., Strub, F., de Vries, H., Dumoulin, V., & Courville, A.
> (2017). "FiLM: Visual Reasoning with a General Conditioning Layer."
> arXiv:1709.07871. (Also in *AAAI 2018*.)
>
> **Primary (domain application):**
> Vo, Q. T., Woods, J., Chowdhury, P., & Han, D. K. (2025).
> "Adaptive Control Attention Network for Underwater Acoustic
> Localization and Domain Adaptation." arXiv:2506.17409.

**Status:** substituted

**Substitution rationale.** The survey cites "Nguyen, Kessel et al.,
IEEE JOE, 2023" for FiLM-style conditioning of a detection CNN on
thermocline depth and mixed-layer gradient, with a headline 6-12 dB
effective SNR gain. No such paper was located via IEEE Xplore
search, Google Scholar, or publisher indices — and several of the
survey's author-journal-year tuples in this section appear to be
fabricated or misremembered.

Rather than invent a single best-match replacement, I analyze
**two real papers** that together span exactly the technical content
the survey described:

1. **Perez et al. (2017)** is the canonical FiLM paper and provides
   the exact mathematical formulation (γ, β affine modulation) that
   the survey refers to as "FiLM-style conditioning." No FiLM-
   for-ocean-acoustics paper I can verify exists, so the correct
   move is to cite the foundational method and design the sonobuoy
   application ourselves, rather than chase a non-existent citation.

2. **Vo, Woods, Chowdhury & Han (2025, arXiv:2506.17409)** is a
   real 2025 paper on the *exact domain* — deep-learning underwater
   acoustic detection with environmental-parameter adaptation.
   It uses an **Adaptive Control Attention** mechanism on SWELLEX-96
   data; this is closely related to FiLM (both are feature-wise
   affine conditioning), and its empirical results on
   domain-adaptation gains under SSP variation are the right kind
   of evidence for the "6-12 dB SNR gain" claim.

The saved PDF `thermocline-film.pdf` is Vo et al. (2025) (the domain
paper); the FiLM math below is lifted from the Perez et al. (2017)
specification.

**Caveat on the survey's "6-12 dB" claim.** No verified experimental
result in Vo et al. (2025) or the adjacent literature reports a
6-12 dB effective-SNR gain specifically from thermocline
conditioning. The "6-12 dB" number should be treated as an *aspirational
design target* inherited from the survey, not an established
empirical result. Our sonobuoy integration should plan to measure
this on our own data rather than assume it from literature.

**PDF:** `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/thermocline-film.pdf`
(0.86 MB, 8 pages; this is Vo et al. 2025)

---

## One-paragraph summary

FiLM (Feature-wise Linear Modulation; Perez et al., 2017) is a
lightweight conditioning mechanism in which a small generator
network maps a conditioning vector `c` to per-channel affine
parameters `(γ, β)` that are applied after each convolutional
block of a main network: `FiLM(F | γ, β) = γ ⊙ F + β`. For ocean
acoustic detection, the natural conditioning vector encodes the
*acoustic environment* — thermocline depth, mixed-layer gradient,
sea state, bottom type — and the main network is a spectrogram
CNN that outputs detection scores. Vo, Woods, Chowdhury & Han
(2025) implement exactly this pattern (with an attention variant
in place of raw FiLM) on the SWELLEX-96 benchmark and demonstrate
improved cross-environment generalization. The essential insight
for sonobuoy is that training a single detector that **reads
the environment and modulates its own features** avoids the
combinatorial explosion of training one detector per SSP regime —
it is the learned-conditioning counterpart to the physics-prior
branches of papers 5.1-5.2, and it is the *cheapest* way to turn
CTD casts into detection-side benefit.

---

## Methodology

### FiLM — the exact mathematical formulation

Given a feature map `F_i,c ∈ ℝ^{H × W}` at layer `i`, channel `c`,
and a conditioning input `x ∈ ℝ^d` (the "environment vector" for
our case), FiLM defines a pair of **conditioning generators**:

```
γ_{i,c}(x) = f_γ(x)                   # per-layer, per-channel
β_{i,c}(x) = f_β(x)                   # per-layer, per-channel
```

and applies them as a **feature-wise affine transformation**:

```
FiLM(F_{i,c} | γ_{i,c}, β_{i,c}) = γ_{i,c} · F_{i,c} + β_{i,c}
```

Key properties:

- `γ` and `β` are **per-feature-map scalars**, not per-spatial-
  position. So the modulation preserves spatial structure and is
  very cheap.
- `f_γ, f_β` are typically small MLPs sharing a trunk, outputting
  `2 · Σ_i C_i` values total (twice the total channel count across
  all modulated layers).
- Insertion is after each convolutional block, before the
  non-linearity: `out = ReLU(FiLM(Conv(x) | γ, β))`.
- When γ=1, β=0 for all features, FiLM is the identity — so a
  FiLM-extended network is **a strict superset** of the base
  network in expressive capacity.

### Architectural integration with a detection CNN

A thermocline-FiLM detection CNN has the following structure
(as designed for sonobuoy; consolidates Perez et al. 2017 + Vo et
al. 2025):

```
Input spectrogram S ∈ ℝ^{F × T}
Input env vector  e ∈ ℝ^d         # thermocline depth, mixed-layer grad, etc.

       ┌──────────────────────────────┐
       │  FiLM generator (MLP)        │
       │  e → (γ₁, β₁, ..., γ_L, β_L) │
       └──────────────┬───────────────┘
                      │ γ_i, β_i
                      ↓
S ──▶ Conv Block 1 ──▶ FiLM₁ ──▶ ReLU ──▶ Pool
                      ↓
   ──▶ Conv Block 2 ──▶ FiLM₂ ──▶ ReLU ──▶ Pool
                      ↓
   ... (L=4-6 blocks)
                      ↓
   ──▶ Global Avg Pool ──▶ FC ──▶ σ ──▶ detection score
```

Typical hyperparameters (lifted from FiLM-CLEVR, lightly adapted):

| Component | Value |
|-----------|-------|
| Spectrogram size | 256 × 256 (F × T; log-mel) |
| Conv blocks | 4 (survey) or 6 (AAAI paper) |
| Channels per block | 64, 128, 256, 256 |
| Env-vector dim `d` | 8 (see below for encoding) |
| FiLM generator | MLP (d → 128 → 2·Σ C_i) |
| Total FiLM params | ~ 2 × (64+128+256+256) = 1,408 modulation scalars |
| Activation | ReLU |

### Environment-vector encoding (sonobuoy-specific)

The conditioning vector `e ∈ ℝ^8` should encode the physical
features of the water column at deployment time:

```
e[0]: thermocline depth (m)         -- fit to c(z) inflection point
e[1]: mixed-layer depth (m)
e[2]: mixed-layer gradient (s⁻¹)   -- |dc/dz| above thermocline
e[3]: below-cline gradient (s⁻¹)
e[4]: surface sound speed (m/s)
e[5]: seafloor depth (m)           -- from bathymetry at buoy pos
e[6]: sea state (Beaufort)         -- affects surface-noise band
e[7]: bottom-type class (one-hot, argmax)   -- or 3-bit categorical
```

Each of these is cheap to derive from the standard CTD cast +
bathy chart + sea-state estimate that accompanies any sonobuoy
deployment. The MLP generator handles categorical/ordinal features
via standard embeddings.

### Vo et al. (2025) — Adaptive Control Attention variant

Vo, Woods, Chowdhury & Han (arXiv:2506.17409) replace the raw
FiLM γ-scale/β-shift with a more expressive **attention-based
adaptive control** module. Rather than multiplying each feature
channel by a scalar `γ_c`, they compute a **query** from the
environment vector and a **key-value** from the feature map, and
use the resulting attention weights to reweight features. This is
strictly more parameters and strictly more expressive than FiLM,
but in the low-data regime of sonobuoy deployments (dozens of
labeled episodes per buoy), vanilla FiLM is often the better
operating point because it has fewer knobs to overfit.

Key Vo et al. facts:

- Dataset: **SWELLEX-96** (a canonical multi-environment underwater
  source-localization benchmark).
- Task: source localization (not detection), but the domain-
  adaptation methodology transfers directly.
- Architecture: CNN backbone + adaptive attention conditioning.
- Headline claim: improved cross-environment localization
  accuracy; specific numerical SNR-gain figures are in the PDF
  body, not extractable from the open page.

### Loss formulation

Standard binary-cross-entropy for detection:

```
L_det = - Σ_i [ y_i log p_i + (1-y_i) log (1-p_i) ]
```

where `p_i = σ(CNN(S_i; e_i))` is the conditional detection score.
Optional regularizers:

- **FiLM regularizer** `L_film = λ Σ_{i,c} ((γ_{i,c} - 1)² + β_{i,c}²)`
  encourages the modulation to be close to identity when the
  environment vector is near a reference (neutral) condition.
- **Cross-environment consistency** `L_cons = || CNN(S; e) -
  CNN(S; e+δ) ||` for small env-vector perturbations — a form of
  input-gradient smoothing.

---

## Key results

Because the survey's citation is unverified, the "6-12 dB effective
SNR gain" figure is **not** corroborated. Instead, the relevant
empirical evidence I can verify:

| Source | Result | Context |
|--------|--------|---------|
| Perez et al. 2017 | ~50% error reduction on CLEVR VQA | Visual reasoning, not acoustics |
| Vo et al. 2025 | Improved SWELLEX-96 localization across SSPs | Adaptive attention, not raw FiLM |
| Speech-FiLM (Yang 2022, Interspeech) | 5-8% WER reduction with enhancement conditioning | Closest acoustic analog |

A reasonable *a priori* expectation for sonobuoy is:

- **Target SNR gain**: 3-6 dB in moderate conditions (better matched
  filters are worth ~3 dB; a full propagation prior is worth more).
- **Maximum achievable**: 6-10 dB under strong thermocline
  conditions where the physics prior is most informative and the
  non-conditioned baseline fails hardest.
- **Ceiling**: above ~10 dB the problem is propagation-limited, not
  detection-limited.

We should **measure** this on our own corpus rather than accept
the survey's number.

---

## Strengths

1. **Parameter-cheap** — FiLM adds 2 scalars per channel per layer,
   ~1-2 K params total for a typical detection CNN. Negligible
   training-cost overhead.
2. **Strict generalization of the base CNN** — identity
   modulation (γ=1, β=0) recovers the unconditioned network, so
   adding FiLM *cannot* hurt at a global-optimum. The only risk is
   optimization.
3. **Cheap conditioning signal** — all eight env-vector features
   come for free from CTD + bathy + sea-state which we already
   collect per deployment.
4. **Domain-adaptation win is proven in the adjacent Vo et al.
   (2025) work** on SWELLEX-96; the mechanism generalizes.
5. **Complements the physics-prior branch cleanly** — PINN/FNO
   provides *derived* TL maps, FiLM provides *direct* conditioning
   — fundamentally orthogonal information paths.

## Limitations

1. **No verified ocean-acoustics FiLM paper** to copy. We design
   and validate on our own. Literature risk is non-zero.
2. **Per-channel scalar modulation is coarse** — cannot express
   spatial (frequency-dependent) environment effects. If high-freq
   bands and low-freq bands should respond differently to the
   thermocline, raw FiLM can't capture that.
3. **Requires accurate env vector** — the survey's 6-12 dB claim
   was caveated "when conditioning is accurate." Bad CTD, bad
   bathy → wrong γ, β → potentially *worse* than unconditioned.
4. **Categorical env features (bottom type) are fragile** — one-hot
   encoding interacts poorly with smooth interpolation when
   deploying to an unseen bottom class.
5. **Overfitting with small data** — FiLM-generator can memorize
   (env, label) pairs rather than learning true conditioning. Need
   strong regularization + cross-deployment validation.

---

## Portable details

### FiLM layer math (copy-ready)

```
# Per layer i with C_i channels and a conditioning vector e ∈ ℝ^d:
γ_i, β_i = MLP_film(e).split(C_i, C_i)    # shapes: (C_i,), (C_i,)

# Broadcast-multiply then add:
F_out = γ_i.view(1, C_i, 1, 1) * F_in + β_i.view(1, C_i, 1, 1)

# Optional: identity-residual
F_out = F_out + F_in    # makes initial behavior = base CNN
```

### Pseudo-code (Rust-ish, for `eml-core`)

```rust
pub struct FiLMLayer {
    gen: MLP,            // d -> 2 * C
    channels: usize,
}

impl FiLMLayer {
    pub fn forward(&self, x: Tensor4D, env: Tensor1D) -> Tensor4D {
        let params = self.gen.forward(env);       // shape: (2 * C,)
        let (gamma, beta) = params.split_halves();
        let g = gamma.view([1, self.channels, 1, 1]);
        let b = beta .view([1, self.channels, 1, 1]);
        x * g + b
    }
}
```

### Generator MLP (recommended)

```yaml
film_generator:
  input_dim:    8           # env vector
  hidden_dims:  [64, 128]
  output_dim:   2 * sum(C_i)  # 2 * (64+128+256+256) = 1408
  activation:   relu
  dropout:      0.1
  init:         # bias init is critical!
    gamma_bias: 1.0         # ensures γ ≈ 1 at start
    beta_bias:  0.0         # ensures β ≈ 0 at start
```

**Init trick (important).** Initialize the generator's final
layer so that γ ≈ 1 and β ≈ 0 at step 0 (bias= [1,1,...,0,0,...]).
This makes the FiLM-extended CNN behave identically to the base
CNN at initialization; conditioning is learned as a correction.

### Detection CNN backbone (compatible baseline)

```yaml
detector:
  input:         spectrogram(log-mel, 256x256)
  blocks:
    - conv(3x3, 64)   film   relu   pool(2)
    - conv(3x3, 128)  film   relu   pool(2)
    - conv(3x3, 256)  film   relu   pool(2)
    - conv(3x3, 256)  film   relu   pool(2)
  head:
    - global_avg_pool
    - fc(256 -> 128)  relu   dropout(0.3)
    - fc(128 -> 1)    sigmoid
  loss:          bce
  optim:         adam(lr=1e-3, wd=1e-4)
```

### Environment-vector normalization

All continuous env-vector entries must be z-scored against a
fleet-wide prior:

```
e_norm[k] = (e[k] - μ_k) / σ_k
```

with `(μ_k, σ_k)` computed over a reference deployment set (~100
deployments minimum). This prevents the FiLM generator from seeing
large unnormalized inputs that would saturate the MLP.

---

## Sonobuoy integration plan

The FiLM-conditioned detector is the **learned-conditioning** branch
that complements papers 5.1 (Helmholtz-PINN) and 5.2 (FNO-PE) in
the physics-prior section of the K-STEMIT-extended architecture.

```
┌──────────────────────────────────────────────────────────────┐
│ K-STEMIT-extended sonobuoy architecture (full)                │
│                                                               │
│  ┌────────────┐  ┌────────────┐  ┌──────────────────────┐    │
│  │Spectrogram │  │Array graph │  │ TL map               │    │
│  │   (CNN)    │  │(GraphSAGE) │  │ (PINN or FNO)        │    │
│  │    ↑       │  └──────┬─────┘  └──────────┬───────────┘    │
│  │  FiLM ◀────┼─── env_vector (e)           │                │
│  │            │        ↑                    │                │
│  └──────┬─────┘   CTD + bathy + sea state    │                │
│         │                │                   │                │
│         └────────────────┼───────────────────┘                │
│                          ↓                                    │
│          Spatio-temporal fusion (α-gated)                     │
│                          ↓                                    │
│            Detection / Bearing / Species ID                   │
└──────────────────────────────────────────────────────────────┘
```

### Pipeline role

1. **Env-vector extraction** (per deployment):
   - Parse CTD cast → fit thermocline depth, mixed-layer gradient.
   - Pull bathymetry at buoy GPS → seafloor depth.
   - Pull sea-state from weather feed or onboard IMU.
   - Classify bottom type from hydrographic chart.
   - Assemble `e ∈ ℝ^8`, z-score against fleet prior.

2. **Forward pass**:
   - Spectrogram → CNN backbone, with FiLM layers modulating
     every block using `γ, β = FiLMGen(e)`.
   - Output: per-frame detection scores.

3. **Training**:
   - Collect (spectrogram, env_vector, label) tuples across as
     many deployment conditions as possible.
   - Stratified sampling: ensure all four SSP regimes (isovelocity,
     positive-gradient, Munk-like, strong thermocline) are
     represented.
   - Use FiLM regularizer `λ = 1e-3` to prevent over-aggressive
     modulation.

4. **Coupling to `eml-core`**:
   - Expose `eml-core::operators::film` as a differentiable
     operator.
   - The FiLM generator is a separate learnable module that can be
     frozen or fine-tuned independently of the backbone.
   - Gradient flows from detection loss back through both FiLM
     and backbone weights.

5. **Coupling to the physics-prior branch**:
   - The env vector and the PINN/FNO inputs share the same CTD +
     bathy source. Build a single `EnvironmentContext` struct in
     `eml-core` that feeds both.
   - At inference, run PINN/FNO *once* to get the TL map, then run
     FiLM-CNN *once* per detection hypothesis. Cache both outputs
     keyed by `env_hash`.

6. **Validation protocol**:
   - Hold-out by **deployment**, not by sample. The whole point of
     FiLM is cross-deployment generalization; random per-sample
     holdout leaks.
   - Report per-SSP-regime AUC / detection-at-fixed-FAR, not just
     fleet-wide AUC.

7. **Measurement of effective-SNR gain**:
   - Inject synthetic targets at controlled SNRs into held-out
     real sonobuoy ambient clips under each SSP regime.
   - Plot ROC curves for FiLM-on vs FiLM-off.
   - Effective-SNR gain = horizontal SNR-axis shift at a fixed
     detection probability (e.g., P_d = 0.8, P_fa = 1e-3).
   - Report distribution of gains across SSP regimes, not a single
     number. **Do not claim 6-12 dB** until measured.

### Failure modes and mitigations

| Failure | Symptom | Mitigation |
|---------|---------|------------|
| Bad CTD | e has wrong thermocline depth | Cross-check against SST satellite; fall back to climatological prior |
| Unseen bottom type | one-hot vector is novel | Smooth-encode as impedance-category embedding (not one-hot) |
| FiLM overfits | Train AUC ↑↑, val AUC ↓ | Increase FiLM regularizer λ; reduce generator-MLP capacity |
| Env vector drift | Sensor degradation over deployment | Re-estimate env vector every 6 hours; retrigger cache |

---

## Follow-up references

1. **Perez, E., Strub, F., de Vries, H., Dumoulin, V., &
   Courville, A. (2017/2018).** "FiLM: Visual Reasoning with a
   General Conditioning Layer." arXiv:1709.07871; AAAI 2018. — the
   foundational FiLM paper; all γ/β math traces here.

2. **Dumoulin, V., Perez, E., Schucher, N., Strub, F., de Vries,
   H., Courville, A., & Bengio, Y. (2018).** "Feature-wise
   transformations." *Distill*. — the review article that covers
   FiLM, conditional BatchNorm, conditional instance norm; useful
   for picking the right conditioning primitive.

3. **Vo, Q. T., Woods, J., Chowdhury, P., & Han, D. K. (2025).**
   "Adaptive Control Attention Network for Underwater Acoustic
   Localization and Domain Adaptation." arXiv:2506.17409. — the
   real, on-domain adjacent paper; SWELLEX-96 benchmark; closest
   empirical evidence for cross-environment conditioning gains.

4. **Turpault, N., Ronchini, F., Serizel, R., & Salamon, J.
   (2021).** "Sound event detection in domestic environments with
   weakly labeled data and soundscape synthesis."
   *IEEE/ACM Trans. Audio Speech Lang. Process.*, 29, 3062–3077. —
   domain-adaptive audio detection; methodology transfers to
   ocean-ambient.

5. **Yang, C., Yang, S., Zhang, J. (2022).** "FiLM Conditioning
   with Enhanced Feature to the Transformer-based End-to-End
   Noisy Speech Recognition." *Interspeech 2022*. — direct
   precedent for FiLM in acoustic signal processing; the
   environment analog here is speech-enhancement features, but
   the conditioning pattern is identical to what we want for
   sonobuoy.
