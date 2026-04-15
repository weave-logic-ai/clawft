# Smoothness-Inducing Regularization for UATR

## Citation header

- **Title**: *Underwater Acoustic Target Recognition based on
  Smoothness-inducing Regularization and Spectrogram-based Data Augmentation.*
- **Authors**: Ji Xu, Yuan Xie, Wenchao Wang (Institute of Acoustics, Chinese
  Academy of Sciences; Key Laboratory of Speech Acoustics and Content
  Understanding). Corresponding author: Ji Xu.
- **Venue**: *Ocean Engineering* (Elsevier), volume 281, 2023, article 114926.
- **arXiv**: [2306.06945](https://arxiv.org/abs/2306.06945) (v1 Jun 2023,
  v3 Apr 2024).
- **DOI**: [10.1016/j.oceaneng.2023.114926](https://doi.org/10.1016/j.oceaneng.2023.114926).
- **PDF**: `.planning/sonobuoy/papers/pdfs/smoothness-uatr.pdf`
  (downloaded from `https://arxiv.org/pdf/2306.06945`).

## Status

**Verified**. Authorship corrected: the survey attributed this to "Xu, Ren et
al." but the actual author list is **Xu, Xie, Wang** (no Ren). Title and
content match exactly.

---

## One-paragraph summary

In low-data underwater recognition regimes, naive audio data augmentation
(adding synthetic noise, pitch shifts, environmental simulation) often *hurts*
generalisation because simulated signals systematically deviate from real
ocean acoustic distributions, pulling the model toward non-physical regions of
feature space. Xu, Xie, and Wang propose two complementary low-cost
regularisers that avoid this failure mode. First, **smoothness-inducing
regularization** (SIR) uses simulated signals only in a KL-divergence penalty
term rather than as extra training examples — the model is forced to produce
similar class posteriors for a clean sample and its simulated perturbation
without ever being told "this perturbation is still class y". Second, **Local
Masking and Replicating (LMR)** is a domain-specific mixup for spectrograms
that samples a second spectrogram from the batch, selects a random a×b patch
location, masks that patch in the source, and copies the patch from the
second spectrogram into it, with the label blend weighted by the patch-area
fraction. Combined on a ResNet-18-with-attention backbone, these give
+6-9 % absolute accuracy on ShipsEar over a vanilla cross-entropy baseline
and robust gains under low-SNR test conditions (80 % at -15 dB SNR vs. 75 %
for traditional augmentation).

---

## Methodology

### Backbone architecture

ResNet-18 with a multi-head attention pooling head, identical to the
DEMONet paper from the same group (Xie et al. 2024). Input feature is one of
STFT amplitude, Mel, CQT, or Bark-scaled spectrogram. 30-second segments;
50 ms frame length, 25 ms shift, 300 Mel/Bark bins, CQT octave resolution 30.

### Smoothness-Inducing Regularization (SIR)

For each clean input `x_i` with label `y_i`, the authors construct a
perturbed pair `x̃_i` by applying a simulation-like transform (noise
injection, reverberation, Doppler, time-stretch). The standard classifier
produces logits `z_i = f(x_i)` and `z̃_i = f(x̃_i)`. Softmax gives
`p_i = softmax(z_i)`, `p̃_i = softmax(z̃_i)`.

The objective is:

```
L = L_CE(z_i, y_i) + α · [ KL(p_i || p̃_i) + KL(p̃_i || p_i) ]
```

The symmetric KL (Jensen-Shannon-like but unnormalised) is critical — a
one-sided KL skews toward whichever branch is more confident and can
collapse. α is typically in [0.5, 2.0] depending on dataset (larger for
data-scarce ShipsEar, smaller for data-abundant DeepShip). Crucially, the
perturbed sample's *label* never enters the loss; only its posterior
agreement with the clean posterior matters.

This is an adaptation of SMART (Jiang et al. 2020, SMoothness-inducing
Adversarial Regularization for NLP) and R3F/R4F to acoustic spectrograms.

### Local Masking and Replicating (LMR)

Given two randomly paired spectrograms `(s_i, y_i)` and `(s_j, y_j)` in the
same batch, pick a random rectangle of size `a × b` at location `(r, c)`.
Construct a mixed spectrogram `Z_ij`:

```
Z_ij[h, w] = s_j[h, w]   if  r ≤ h < r+a  and  c ≤ w < c+b
             s_i[h, w]   otherwise
```

Let `λ = 1 - (a·b) / (H·W)` be the fraction of `s_i` that survives.
The loss for the mixed sample is the label-mixed cross-entropy:

```
L_LMR = λ · L_CE(z_ij, y_i) + (1 - λ) · L_CE(z_ij, y_j)
```

LMR fires with 50 % probability per batch during training; remaining 50 %
use normal cross-entropy on unmixed samples. `a` and `b` are sampled
uniformly from `[0.2·H, 0.6·H]` and `[0.2·W, 0.6·W]` respectively.

This differs from SpecAugment (time/frequency masking to zero) in that the
masked region is *replaced* by content from another class, and differs from
mixup in that it is a spatially localised cut-paste rather than a pixel-wise
linear blend. The authors argue this is better for spectrograms because
linear blends destroy narrowband line spectra whereas LMR preserves them
pixel-for-pixel within the non-masked region.

### Datasets (Section 4.1 of the paper)

| Dataset | Train | Val | Test | Hours | Classes |
|---------|-------|-----|------|-------|---------|
| ShipsEar | 400 | 71 | 116 | 2.94 | 9 |
| DeepShip | 6742 | 1190 | 2547 | 47.07 | 4 |
| DTIL (proprietary) | 1655 | 292 | 354 | 10.25 | 2 |

Same splits as used in the subsequent DEMONet paper (authors released them
together on GitHub).

### Hyperparameters

| Item | Value |
|------|-------|
| Frame length / shift | 50 ms / 25 ms |
| Filter banks | 300 (Mel or Bark) |
| CQT octave resolution | 30 |
| Optimizer | AdamW |
| Peak LR | 5e-4 |
| Weight decay | 1e-5 |
| Batch size | 64 |
| Epochs | 100 |
| SIR coefficient α | 0.5-2.0 (dataset-dependent) |
| LMR activation probability | 0.5 |
| LMR patch dims | a ~ U(0.2H, 0.6H), b ~ U(0.2W, 0.6W) |

---

## Key results (verbatim from the paper and HTML tables)

### ShipsEar (9-class)

| Method | Feature | Accuracy (%) |
|--------|---------|--------------|
| Baseline ResNet-18 | STFT | 75.24 |
| + SIR | STFT | 81.90 |
| + SIR + LMR | STFT | **82.97** |
| Baseline | Mel | 77.14 |
| + SIR + LMR | Mel | **83.45** |
| Baseline | CQT | 73.33 |
| + SIR + LMR | CQT | 82.76 |

Best: **83.45 %** (Mel + SIR + LMR). Baseline → SIR gap is +6.66 %, and
LMR adds another +1.07 %.

### DeepShip (4-class)

| Method | Feature | Accuracy (%) |
|--------|---------|--------------|
| Baseline | CQT | 77.15 |
| + SIR + LMR | CQT | **80.05** |

DeepShip has 14× more training data than ShipsEar, so the absolute gains
are smaller (+2.9 % vs. +8 %), consistent with SIR/LMR acting primarily as
overfitting regularisers.

### DTIL (2-class)

| Method | Accuracy (%) |
|--------|--------------|
| Baseline | 97.18 |
| + SIR + LMR | **97.80** |

### Low-SNR robustness (Section 5 of paper)

At -15 dB SNR the baseline drops to ~75 %, traditional noise-augmentation
models sit around 75-77 %, and the SIR model holds at ~80 %. SIR's gap
widens as SNR worsens, which is the primary practical argument for it in
deployed sonar systems.

### LMR vs. mixup

Authors report LMR beats mixup by 2.86-9.04 % on data-scarce datasets and
ties it on data-abundant DeepShip. The localised nature of LMR is cited as
the reason: narrowband line spectra (the highest-SNR physical features in
ship noise) are destroyed by linear pixel blends but preserved by rectangle
cut-paste.

### Negative controls

Traditional additive-Gaussian-noise augmentation *degrades* Bark-spectrogram
DeepShip accuracy by 0.97 %. Simulated reverberation augmentation degrades
ShipsEar. These results motivate the smoothness-regulariser-only use of
simulated data.

---

## Strengths

- **Simulation-as-regulariser, not as training data.** The central insight
  that simulated underwater signals should only shape the posterior
  *smoothness* and never directly supervise labels is clean, well-motivated
  by negative controls, and directly portable to other physics-bounded
  sensing domains (radar, seismology, medical ultrasound).
- **LMR is conceptually minimal and composable.** It has three knobs
  (activation prob, patch dimensions) and stacks additively with SIR. No
  new network components.
- **Symmetric KL is the right formulation.** The authors explicitly justify
  using both `KL(p||p̃)` and `KL(p̃||p)` and show collapse with one-sided KL
  in ablations — a detail often skipped in adjacent papers.
- **Low-SNR gains are the real story.** In-distribution accuracy gains on
  ShipsEar are nice, but the ~5 % gap at -15 dB SNR is the deployment
  payoff. Sonobuoys routinely operate at negative-SNR conditions when
  shipping lane noise dominates.
- **Shares splits with the DEMONet paper** so the two can be stacked: use
  SIR+LMR as the training regularisers on top of the DEMONet architecture.

## Limitations

- **Not effective on large in-distribution datasets.** +2.9 % on DeepShip
  and +0.6 % on DTIL are marginal; the method's core benefit is in
  data-scarce or distribution-shift regimes.
- **Perturbation choice is dataset-specific.** Authors use a hand-designed
  set of simulation operators (noise, reverb, Doppler, time-stretch). The
  paper does not provide an ablation on which operator matters most or a
  learned-perturbation variant.
- **Only closed-set recognition tested.** No open-set (unknown-class
  reject), no detection, no bearing estimation. Whether SIR helps in
  dense-prediction acoustic tasks is open.
- **SIR α is tuned manually per dataset.** No principled schedule; adaptive
  α (e.g., ramp up as training loss drops) is an obvious extension.
- **LMR patch shape is axis-aligned rectangles.** For spectrograms where
  harmonic ridges are vertical and transient events are horizontal, a
  1D-oriented mask (strict time-only or frequency-only) would be more
  targeted — this matches SpecAugment's original design but LMR does not
  evaluate it.

---

## Portable details (Rust implementation notes)

### Symmetric smoothness loss (tch-rs style)

```rust
fn smoothness_loss(
    logits_clean: &Tensor,
    logits_perturbed: &Tensor,
    alpha: f32,
) -> Tensor {
    let p  = logits_clean.softmax(-1, Kind::Float);
    let pt = logits_perturbed.softmax(-1, Kind::Float);
    // Numerical-stability: add small epsilon inside log
    let eps = 1e-8;
    let kl_fwd = (p.clone() * (p.clone() + eps).log() - p.clone() * (pt.clone() + eps).log())
                 .sum_dim_intlist(&[-1], false, Kind::Float);
    let kl_bwd = (pt.clone() * (pt.clone() + eps).log() - pt.clone() * (p.clone() + eps).log())
                 .sum_dim_intlist(&[-1], false, Kind::Float);
    (kl_fwd + kl_bwd).mean(Kind::Float) * alpha
}
```

### LMR augmentation (pure Rust / ndarray)

```rust
/// Local Masking and Replicating.
/// Pairs two spectrograms s_i, s_j with labels y_i, y_j.
/// Returns the mixed spectrogram and the mixing fraction lambda in [0, 1]
/// where 1.0 means s_i is fully preserved.
pub fn lmr_mix(
    s_i: &Array2<f32>,
    s_j: &Array2<f32>,
    rng: &mut impl Rng,
) -> (Array2<f32>, f32) {
    let (h, w) = s_i.dim();
    let a = rng.gen_range((0.2 * h as f32) as usize ..= (0.6 * h as f32) as usize);
    let b = rng.gen_range((0.2 * w as f32) as usize ..= (0.6 * w as f32) as usize);
    let r = rng.gen_range(0 ..= h - a);
    let c = rng.gen_range(0 ..= w - b);
    let mut out = s_i.clone();
    out.slice_mut(s![r..r+a, c..c+b])
       .assign(&s_j.slice(s![r..r+a, c..c+b]));
    let lambda = 1.0 - (a as f32 * b as f32) / (h as f32 * w as f32);
    (out, lambda)
}
```

### LMR loss

```rust
fn lmr_loss(
    logits_mix: &Tensor,
    y_i: &Tensor,
    y_j: &Tensor,
    lambda: f32,
) -> Tensor {
    let ce_i = logits_mix.cross_entropy_loss::<Tensor>(y_i, None, Reduction::Mean, -100, 0.0);
    let ce_j = logits_mix.cross_entropy_loss::<Tensor>(y_j, None, Reduction::Mean, -100, 0.0);
    lambda * ce_i + (1.0 - lambda) * ce_j
}
```

### Full training step

```rust
for (xb, yb) in loader {
    // Standard forward
    let zb = model.forward(&xb);
    let mut loss = zb.cross_entropy_loss::<Tensor>(&yb, None, Reduction::Mean, -100, 0.0);

    // Smoothness branch
    if rng.gen::<f32>() < 1.0 {  // always on in this paper
        let xb_tilde = perturb(&xb, &mut rng);     // noise/reverb/Doppler
        let zb_tilde = model.forward(&xb_tilde);
        loss = loss + smoothness_loss(&zb, &zb_tilde, alpha);
    }

    // LMR branch
    if rng.gen::<f32>() < 0.5 {
        let (x_mix, y_i, y_j, lambda) = build_lmr_batch(&xb, &yb, &mut rng);
        let z_mix = model.forward(&x_mix);
        loss = loss + lmr_loss(&z_mix, &y_i, &y_j, lambda);
    }

    optimizer.backward_step(&loss);
}
```

### Perturbation operators used in SIR (Section 3.2)

- **Gaussian noise injection** at SNR drawn from `U(-10 dB, 20 dB)`.
- **Reverberation** via a simulated ocean-channel impulse response (paper
  uses a parametric Rayleigh-distributed multipath).
- **Doppler** via small uniform resampling factor drawn from
  `U(1 - 0.01, 1 + 0.01)`.
- **Time-stretch** by factor drawn from `U(0.9, 1.1)`.

Apply one uniformly at random per batch.

### Reproducibility checklist

| Item | Value |
|------|-------|
| Code released | Paper references GitHub (xy980523/ShipsEar-An-Unofficial-Train-Test-Split) for splits only |
| Splits published | Yes |
| Full hyperparameters | Yes (Section 4) |
| Seeds | 2 seeds, mean ± std |

---

## Sonobuoy integration plan

Smoothness-inducing regularisation and LMR are **training-time only**
techniques with zero inference-time cost, so they should be enabled by
default across every learned component of the sonobuoy pipeline that
touches spectrograms. In the K-STEMIT dual-branch architecture
(`.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`):

- **Temporal branch (DEMONet-style, `weftos-sonobuoy-temporal` crate)**: wrap
  the forward pass in a SIR harness. Apply LMR to input CQT spectrograms
  before they feed the routing layer. Because the DEMONet paper itself does
  not use SIR/LMR, this stacking is an obvious experimental win and worth a
  benchmark on the DeepShip test split to quantify the synergy.
- **EML-core (`eml-core`)**: expose SIR and LMR as first-class training
  mix-ins in the EML learned-function training loop. The symmetric-KL
  smoothness penalty is exactly the kind of "coherence-between-perturbations"
  signal that EML-coherence already formalises; make them the same thing by
  registering SIR as an `eml_core::coherence::Penalty` implementation so
  that perturbation-robustness is uniformly defined across EML.
- **Ocean-channel simulator**: the SIR perturbation operators (noise,
  reverb, Doppler) want a principled ocean-acoustic propagation model. This
  is the same model K-STEMIT's physics-informed node features need
  (sound-speed profile, thermocline, bathymetry). Build a single
  `weftos-sonobuoy-propagation` crate that serves both branches: it
  generates training-time perturbations for SIR *and* runtime priors for
  the spatial branch's GraphSAGE.
- **HNSW vector service** (`clawft-kernel/src/hnsw_service.rs`): no direct
  impact, but a SIR-trained encoder should produce embeddings that are more
  stable under query-time environmental drift, which makes HNSW retrieval
  across days/weeks more reliable.
- **Quantum register mapping** (`clawft-kernel/src/quantum_register.rs`):
  SIR's symmetric-KL criterion has a natural quantum analogue
  (symmetrised-quantum-relative-entropy between two density operators); a
  research follow-up is to replace the classical SIR with a quantum
  coherence penalty when the quantum register is available.
- **Priority**: P1. Lower priority than building the temporal encoder
  itself (DEMONet = P0), but enable by default once training infrastructure
  exists. Near-zero cost, consistent win in data-scarce regimes — and
  sonobuoy data will almost certainly be data-scarce at launch.

---

## Follow-up references (cited works that matter)

1. **Jiang, Y., He, P., Chen, W., Liu, X., Gao, J., Zhao, T. 2020** — *SMART:
   Robust and Efficient Fine-Tuning for Pre-trained Natural Language Models
   through Principled Regularized Optimization.* ACL 2020. The original
   smoothness-inducing adversarial regularisation formulation this paper
   ports to acoustics. Essential background reading.
2. **DeVries, T., Taylor, G.W. 2017** — *Improved Regularization of
   Convolutional Neural Networks with Cutout.* arXiv:1708.04552. Cutout is
   the direct ancestor of LMR's masking half; LMR extends it by replacing
   cut-out pixels with content from another sample rather than zeroing.
3. **Park, D.S. et al. 2019** — *SpecAugment: A Simple Data Augmentation
   Method for Automatic Speech Recognition.* Interspeech 2019. The
   incumbent baseline LMR is positioned against. Critical for any honest
   ablation reproducing this paper.
4. **Zhang, H., Cisse, M., Dauphin, Y.N., Lopez-Paz, D. 2018** — *mixup:
   Beyond Empirical Risk Minimization.* ICLR 2018. The pixel-wise linear
   blend baseline LMR reports 2.86-9.04 % gains over on ShipsEar/DTIL.
5. **Irfan, M. et al. 2021** — *DeepShip: An underwater acoustic benchmark
   dataset…* Expert Systems with Applications 183, 115270. The same
   DeepShip benchmark used in the DEMONet analysis (`uatd-net.md`); this
   paper releases the same splits.
