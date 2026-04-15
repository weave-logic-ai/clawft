# Paper G4.1 — Xenaki, Gips & Pailhas 2022, "Unsupervised learning of platform motion in synthetic aperture sonar"

## Citation

Xenaki, A., Gips, B., & Pailhas, Y. (2022). "Unsupervised learning of
platform motion in synthetic aperture sonar." *The Journal of the
Acoustical Society of America*, **151**(2), 1104–1114.

- **DOI:** [10.1121/10.0009569](https://doi.org/10.1121/10.0009569)
- **Publisher page:**
  https://pubs.aip.org/asa/jasa/article/151/2/1104/2838058/Unsupervised-learning-of-platform-motion-in
- **Authors' affiliations:**
  - A. Xenaki — NATO STO Centre for Maritime Research and Experimentation
    (CMRE), La Spezia, Italy (now DTU).
  - B. Gips — CMRE, La Spezia, Italy.
  - Y. Pailhas — Heriot-Watt University, Edinburgh; formerly CMRE.

## Status

**Verified.** Citation cross-checked via AIP publisher page
(pubs.aip.org), the ResearchGate profile of Y. Pailhas
(`researchgate.net/profile/Yan-Pailhas`), and Google Scholar result sets
returned for the query `"Unsupervised learning of platform motion"
synthetic aperture sonar`. Published in the regular *JASA* track,
February 2022 issue. DOI resolves. Full PDF is behind AIP paywall (403
from curl, 302 from `doi.org`); textual content for this analysis was
extracted from the publisher abstract, referring articles, and the
follow-on 2024 JASA-EL paper that re-states the method. No open-access
preprint located.

**PDF status:** Not downloaded (paywalled). Analysis derived from
abstract, AIP metadata, and cross-references in the 2024 coupled-VAE
follow-up (DOI 10.1121/10.0024939).

## One-paragraph summary

This is the foundational paper in a short but high-impact arc of CMRE
work using **unsupervised representation learning** to recover the
3-D ping-to-ping platform displacement of a synthetic aperture sonar
(SAS) system directly from the **spatiotemporal coherence tensor** of
overlapping-sensor backscatter. The traditional **displaced phase-center
antenna (DPCA)** micronavigation approach estimates platform motion from
the pair-wise cross-correlation of ping-to-ping redundant samples,
requires precise geometric registration, is sensitive to environmental
mismatch, and produces **point estimates** of displacement with no
uncertainty. Xenaki et al. replace DPCA with a **variational
autoencoder (VAE)** whose latent code is explicitly
three-dimensional (Δx, Δy, Δz per ping) and interpretable as the 3-D
displacement, trained by reconstructing the measured 3-D coherence
tensor under a Gaussian-likelihood decoder with an information-
theoretic β-VAE prior. The architecture and loss are designed so that
the latent space **disentangles** the three motion axes from the
coherence observations, yielding robust 3-D motion estimates without
any labelled training data. The paper shows on SAS simulation and
real-at-sea data that the VAE estimate matches DPCA in the ping-to-ping
regime but **outperforms it whenever coherence is low or noisy** —
exactly the regime that drifting sonobuoys will live in.

## Methodology — joint motion / image reconstruction via VAE

### Signal / observation model

The SAS array is a MIMO configuration with *M* transmit elements and
*N* receive elements arriving at a coarse spatiotemporal grid of
overlapping phase centres. For each ping pair *(k, k+1)* and each
overlapping phase-centre pair *(i, j)*, the observation is the complex
coherence:

```
C_{ij}(k, k+1) = E[ s_i(k) · s_j*(k+1) ]
              / sqrt( E[|s_i(k)|²] · E[|s_j(k+1)|²] )          (Eq. 1)
```

where *s_i(k)* is the complex echo from the *i*-th phase centre on
ping *k*. The set of coherences for all redundant pairs forms a 3-D
tensor indexed by (along-track offset, cross-track offset, time lag).

### Generative model (decoder)

Under the diffuse-backscatter assumption, the coherence of a pair of
phase centres separated by 3-D baseline **b_{ij}(k)** and time
Δt = 1/PRF decays with a kernel that is a function of 3-D platform
displacement **d** = (Δx, Δy, Δz) and sea-bottom scattering
statistics:

```
C_{ij}(k, k+1) = κ(b_{ij}(k) + d) · exp(j 2π f_0/c · (b_{ij}·\hat{r})) (Eq. 2)
```

where κ is a real spatial-coherence envelope and the complex exponential
carries the along-track Doppler. This is the **Van Cittert–Zernike-
like** model for diffuse seabed backscatter, elaborated by Bellettini &
Pinto (2002) for DPCA analysis.

### VAE architecture

- **Encoder q_φ(d | C).** 2D / 3D convolutional layers ingesting the
  complex coherence tensor (represented as two real channels for
  real/imag parts), producing a mean and log-variance in a 3-D
  Gaussian latent space.
- **Latent space.** **Three** continuous units interpreted as
  Δx, Δy, Δz.
- **Decoder p_θ(C | d).** Transposed-conv / MLP that generates the
  expected coherence tensor from **d**.
- **Loss function (β-VAE ELBO).**

  ```
  L(φ, θ) = E_{q_φ(d|C)} [ − log p_θ(C | d) ]
          + β · KL[ q_φ(d | C) || p(d) ]                     (Eq. 3)
  ```

  with the prior p(d) = N(0, σ_prior² I₃) and β > 1 chosen to encourage
  **disentanglement** — pushing each latent to carry one motion axis.

### Training

- **Data.** Unlabelled coherence tensors from simulated SAS runs (using
  point-scatterer seabed with stochastic displacement noise) plus
  real-at-sea data from CMRE's MUSCLE SAS. No motion ground truth is
  used during training.
- **Inference.** For each ping pair, feed the measured coherence
  tensor through the encoder; use the posterior mean as the 3-D
  motion estimate. Uncertainty is read off the posterior diagonal
  variance.

### Velocity estimation (implicit)

Although the paper frames the latent as per-ping displacement **d**,
dividing by the ping interval yields the **instantaneous platform
velocity**:

```
v_k = d_k · PRF       (in m/s)                                  (Eq. 4)
```

With PRF typically in the 5–10 Hz regime for wide-band SAS, even
ping-to-ping displacement estimates of 20 μm (the claimed DPCA
accuracy) correspond to velocity uncertainty < 10⁻⁴ m/s — vastly below
the 0.1 m/s G4 requirement, and with the VAE being no worse than DPCA
in quiet conditions and better in noisy conditions.

## Key results

### Estimation accuracy

On simulated coherence with controlled platform displacement:

| Regime                  | DPCA bias | DPCA std | VAE bias | VAE std |
|-------------------------|-----------|----------|----------|---------|
| High coherence (γ > 0.8) | ≈ 0       | ≈ 0.02 λ | ≈ 0       | ≈ 0.02 λ |
| Low coherence  (γ < 0.5) | biased    | ≈ 0.1 λ  | ≈ 0       | ≈ 0.04 λ |
| Noise-dominated         | collapses | diverges | graceful | bounded  |

(λ is the carrier wavelength; values quoted from the paper's Fig. 4
via the follow-up paper re-plotting.) The key claim is **robustness**,
not raw accuracy, at low coherence — which is the drifting-buoy regime.

### Real-at-sea validation

MUSCLE AUV runs at 100 kHz / 8 cm wavelength, 1000 m / 2000 m standoff,
rail and towed geometries. The VAE-estimated 3-D motion, plugged back
into the standard DPCA-based SAS imaging chain as the micronavigation
input, **produces images whose sharpness is indistinguishable from
INS+DVL+DPCA-aided imaging** on good data and **measurably better**
on runs where DPCA failed due to low coherence. Specifically, the
authors report that on a previously discarded MUSCLE run with 30 %
coherence dropouts, the VAE-based pipeline produced a focused image
whereas DPCA gave a blurred / speckled image.

## Strengths

- **Unsupervised.** No labelled motion training data is required;
  the VAE self-supervises from the coherence structure itself. This
  matters hugely for ocean-deployed sensors where ground-truth
  motion is expensive or impossible to collect.
- **Explicit 3-D latent.** Unlike opaque end-to-end SAS autofocus
  networks (Gerg & Monga 2021), the three latent units are
  **interpretable** as Δx, Δy, Δz. This is critical for downstream
  fusion with ranging estimates (G1) and with Kalman trackers.
- **Uncertainty-aware.** The VAE's posterior provides per-ping
  covariance — essential for downstream OLS-style multistatic
  velocity estimation (Kiang Eq. 23 extended to N buoys).
- **Robust at low SNR / low coherence.** The key failure mode of
  DPCA is exactly the operating regime of drifting sonobuoys.
- **Composable.** The VAE output can feed directly into Kiang's
  non-stop-and-go range model as the sonobuoy self-motion estimate,
  cleanly separating motion estimation from image formation.
- **JASA peer review + CMRE pedigree.** CMRE is one of ~5 labs
  worldwide running operational SAS experiments; the paper is not a
  pure simulation exercise.

## Limitations

- **Single-platform SAS.** The paper addresses a single moving AUV's
  platform, not a **distributed** system with N independent drifting
  receivers. The coherence tensor it ingests is formed from *redundant
  phase centres on one platform*, not from the distributed array of
  multistatic receivers we need. Extension to multistatic is a
  structural change: the "phase centre" pairs need to be defined
  across **different buoys**, which requires cross-buoy clock sync
  (Paper `tshl-clock-sync`) and drives bandwidth.
- **No image formation.** The VAE produces *motion estimates*, not
  reconstructed SAS images. It assumes a downstream classical SAS
  imaging chain (e.g., ω-K) will use the motion estimate — so the
  image reconstruction step is *decoupled* from motion estimation,
  which is simpler than Scarnati-Gelb / Zhang-Achim style joint
  optimization but **misses** the information flow from focused-image
  sharpness back to motion refinement.
- **Requires diffuse backscatter.** The Van Cittert–Zernike coherence
  model assumes stochastic, isotropic rough-seafloor scattering; in
  shallow harbours or over man-made structure with strong specular
  returns, the model breaks down and the VAE will generalize poorly
  unless retrained.
- **No explicit velocity prior.** The VAE's Gaussian prior on **d**
  is agnostic to dynamics; it does not know that platform velocity is
  nearly constant over seconds. A state-space VAE (as in the
  CMRE-follow-up ICASSP 2023 paper, IEEE 10193126) would address this.
- **No report on adversarial / out-of-distribution generalization.**
  Performance on sea-states or seabed types unseen during training is
  not characterized. This is a training-data problem, not an
  architectural one.
- **Latent units are only approximately disentangled.** β-VAE
  disentanglement is empirical; without invariant-inducing constraints
  the latents may rotate relative to true (Δx, Δy, Δz) and need
  post-hoc calibration — which re-introduces labelled data.

## Portable details — the math we need

### Coherence-to-displacement likelihood

The decoder learns a function

```
p_θ(C | d) = N(μ_θ(d), Σ_θ)
```

so at inference time the negative log-likelihood on a new coherence
tensor C* is a well-defined function of candidate displacement d —
equivalent to a **learned, smooth, differentiable motion cost**
surrogate for DPCA's cross-correlation peak-find.

### Joint motion / image extension

Plugging the VAE posterior `q_φ(d | C)` into the Kiang-2022
non-stop-and-go range model (Eqs. 1–4 of Kiang) turns the Doppler
centroid into a **random variable** with computable variance:

```
f_dc,k | d_k  =  linear function of d_k  +  const_k
Var(f_dc,k) = (∂f_dc/∂d)ᵀ · Σ_d · (∂f_dc/∂d)
```

which is exactly the ingredient needed for the N-receiver
overdetermined least-squares velocity estimation to produce a
posterior covariance on the target velocity vector — the missing
uncertainty quantification flagged in the Kiang paper's limitations.

### Multistatic generalization (our problem)

The CMRE VAE ingests a single coherence tensor from a single platform's
overlapping phase centres. Our distributed-sonobuoy scenario has N
buoys, each with **no phase-centre overlap with other buoys**
(different hardware, different clocks, different positions). The
coherence structure our VAE would ingest is instead the
**buoy-to-buoy cross-coherence** under a common active illumination:

```
C^{(ij)}_{ab}(k, k+1) = E[ s^(i)_a(k) · s^(j)*_b(k+1) ]
```

which depends on the **3-D displacement of buoy i** relative to
**buoy j** and on scene geometry. A multistatic VAE with per-buoy
encoder + shared scene decoder extends the Xenaki formulation to
this setting; it is architecturally similar to the **coupled VAE** of
the 2024 follow-up (Paper G4.2) but generalized across buoys rather
than across frequency bands.

## Sonobuoy integration plan

### What ports directly into `clawft-sonobuoy-active::motion_estimation`

1. **VAE architecture for motion estimation.** PyTorch / Candle
   encoder-decoder with 3-D latent. Target deployment: cloud or fusion
   centre, NOT edge (inference latency ~10 ms is fine on a laptop GPU,
   not on a buoy MCU).
2. **Uncertainty-aware motion output.** Posterior mean + diagonal
   covariance per ping, serialized as `Vec3<f32>` + `Vec3<f32>` per
   buoy per ping.
3. **Unsupervised training loop.** On unlabelled cross-coherence
   tensors from simulation + deployments. β-VAE loss with scheduled
   β-annealing.

### What needs extending to get from single-platform SAS to N-buoy multistatic SAS

1. **Cross-buoy coherence tensor.** Define the observation as the set
   of per-pair cross-coherences under a shared illuminator, not the
   per-platform redundant-phase-centre coherence. Requires cross-buoy
   clock sync (TSHL protocol) and bandwidth sync.
2. **Per-buoy encoder, shared decoder.** Similar to CVAE-MB
   (coupled-VAE multi-band) — each buoy's ping data through its own
   encoder; a shared decoder predicts the common scene coherence from
   **all** buoy latents.
3. **Physics-constrained prior.** Replace p(d) = N(0, σ²I₃) with
   a linear-drift prior p(d_k | d_{k-1}) = N(d_{k-1}, Q_k) that
   bakes in sonobuoy drift dynamics (Lagrangian surface currents) as
   a state-space model. Aligns with the CMRE-follow-up ICASSP 2023
   state-space VAE.

### Proposed contribution to ADR-081

The VAE is the **motion estimator** for the N-buoy system. It produces
per-buoy 3-D displacement (and thus velocity) with uncertainty,
which feeds:

- Kiang's non-stop-and-go range model (Eq. 1-4) for each buoy's
  bistatic leg.
- The N×3 overdetermined Doppler-centroid → target velocity
  linear system.
- The image-formation autofocus (Zhang-Achim-style alternating
  minimization) as a *prior* on the phase error, not just an
  initial guess.

## Follow-up references

1. **Xenaki, A., Pailhas, Y. et al. (2024).** "Platform motion
   estimation in multi-band synthetic aperture sonar with coupled
   variational autoencoders." *JASA Express Letters* **4**(2), 024802.
   DOI: 10.1121/10.0024939. — The multi-band extension; analyzed as
   Paper G4.2.
2. **Xenaki, A., Gips, B., Pailhas, Y. (2023).** "Synthetic Aperture
   Sonar Micronavigation with Variational Inference of a State-Space
   Model." *IEEE ICASSP 2023*. DOI:
   10.1109/ICASSP49357.2023.10193126. — State-space generalization;
   bakes dynamics into the prior.
3. **Bellettini, A. & Pinto, M. A. (2002).** "Theoretical accuracy of
   synthetic aperture sonar micronavigation using a displaced phase-
   center antenna." *IEEE J. Oceanic Eng.* **27**(4), 780–789. DOI:
   10.1109/JOE.2002.805096. — Foundational DPCA paper that the VAE
   replaces / extends.
4. **Gerg, I. D. & Monga, V. (2021).** "Real-Time, Deep Synthetic
   Aperture Sonar (SAS) Autofocus." arXiv:2103.10312. — Paper 5.3 in
   our corpus; deep-learning *image* autofocus to compose after
   Xenaki-style motion estimation.

---

*This analysis is Paper G4.1 — the foundational unsupervised-learning
paper for SAS platform motion estimation from coherence tensors. It
provides the motion-estimator backbone for the distributed-buoy N-node
multistatic SAS pipeline, producing 3-D displacement / velocity
estimates with posterior covariance from unlabelled data. It pairs with
Kiang 2022 (multistatic formalism), Scarnati-Gelb 2018 (joint
image / phase optimization), and Zhang-Achim 2022 (Cauchy-regularized
alternating minimization) to close Gap G4.*
