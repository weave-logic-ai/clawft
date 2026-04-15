# UATD-Net → DEMONet (Substituted)

## Citation header

- **Original survey claim**: "UATD-Net — Dual-branch CNN where one branch processes LOFAR
  spectrograms and the other processes DEMON spectra, fused by cross-attention.
  Yang et al., IEEE JOE, 2024. ~92% accuracy on 4 vessel classes."
- **Verified paper**: *DEMONet: Underwater Acoustic Target Recognition based on
  Multi-Expert Network and Cross-Temporal Variational Autoencoder.*
- **Authors**: Yuan Xie, Xiaowei Zhang, Jiawei Ren, Ji Xu (Institute of
  Acoustics, Chinese Academy of Sciences).
- **Venue**: arXiv preprint (2411.02758); submitted to Elsevier (Ocean
  Engineering / Expert Systems with Applications line).
- **Year**: 2024 (submitted 5 Nov 2024).
- **arXiv**: [2411.02758](https://arxiv.org/abs/2411.02758).
- **PDF**: `.planning/sonobuoy/papers/pdfs/uatd-net.pdf`
  (downloaded from `https://arxiv.org/pdf/2411.02758`).

## Status

**Substituted** (from "UATD-Net, Yang et al., IEEE JOE 2024" to "DEMONet, Xie et
al., arXiv 2411.02758, 2024").

### Substitution rationale

Exhaustive searches of arXiv, Google Scholar, and IEEE JOE's 2024 volume
returned no paper titled "UATD-Net" or matching the "LOFAR + DEMON dual-branch
with cross-attention by Yang et al." description. The survey-supplied citation
appears to conflate several real works:

1. **UATD** is a real acronym in the literature but refers to the *Underwater
   Acoustic Target Detection* dataset (multibeam FLS, Nature Sci. Data 2022,
   arXiv:2212.00352) — a forward-looking-sonar object-detection corpus, not a
   passive-recognition model.
2. **DEMONet** (this paper) is the closest real passive-acoustic architecture
   that explicitly uses DEMON spectra as one of two parallel information paths
   (the second being CQT/STFT/Mel spectrograms) and fuses them through a
   learned routing mechanism rather than cross-attention. It reports
   state-of-the-art results on DeepShip (80.45%) and its proprietary DTIL
   dataset (97.88%), and 84.92% on ShipsEar.
3. A secondary candidate is **ResA-LSTM** (dual-branch Residual-Attention +
   Bi-LSTM on Mel spectrograms, ShipsEar 98.55% / DeepShip 99.31%), but it
   does not use DEMON spectra so it is a weaker match for the survey intent.

DEMONet matches the survey's *intent* (dual-path architecture, DEMON + a
complementary time-frequency feature, SOTA on ShipsEar/DeepShip) while being a
verifiable real paper from a credible group with a clear arXiv id. The actual
accuracy numbers are close to the survey's "~92%" claim (84.9% on 9-class
ShipsEar, 80.5% on 4-class DeepShip with harder no-overlap splits).

---

## One-paragraph summary

DEMONet tackles the practical tension in Underwater Acoustic Target Recognition
(UATR): time-frequency spectrograms carry rich class-discriminative
information but are easily polluted by environmental noise and Doppler
artifacts, whereas physical features like DEMON (Detection of Envelope
Modulation on Noise) spectra are robust to environment but carry insufficient
class-discriminative structure on their own. Directly fusing them via feature
concatenation or model ensembling often *degrades* accuracy (shown by the
authors' own baseline experiments). DEMONet instead treats the DEMON spectrum
as a *router signal* for a sparse Mixture-of-Experts: a lightweight routing
layer reads the 1D DEMON spectrum and dispatches each input CQT spectrogram
to one of N identical expert convolutional heads, which then feeds a shared
ResNet-18 backbone. To make the routing robust the authors pre-train a
*cross-temporal VAE* whose objective is to reconstruct the DEMON spectrum of
one 30-second segment of a recording from another segment of the same
recording — forcing the latent code to keep only the time-invariant physical
structure (shaft-rate lines, blade harmonics) and discard transient noise.
A load-balancing loss prevents expert collapse. The design achieves
state-of-the-art on DeepShip (80.45 ± 0.67 %) and DTIL (97.88 ± 0.25 %) with
only 0.535 MB of extra parameters over a ResNet-18 baseline.

---

## Methodology

### Signal path

```
hydrophone array
    |
    v
beamform  -> single-channel signal
    |
    v
band-pass filter (dataset-specific passbands, see Table 2 of the paper)
    |
    +--> spectrogram branch -----+
    |       frame 50 ms / 25 ms  |
    |       Hanning + STFT       |
    |       -> STFT | Mel | CQT  |
    |                            |
    +--> DEMON branch            |
            250 Hz sub-band split|
            |x(t)| demod         |
            low-pass (Hamming,   |
              len 1024)          |
            FFT(|.|^2)           |
            concat -> 2D DEMON   |
              spectrum (28x1172) |
                                 |
                                 v
          Cross-Temporal VAE
          (pretrained, fixed in stage 2)
                                 |
                                 v
           reconstructed DEMON -> sum along modulation axis
                                 -> 1D DEMON (B x 1172)
                                 |
                                 v
                     Routing Layer (Linear 1172 -> N)
                                 |
                                 v            Sigmoid -> p_i
                         arg max p_i[j]
                                 |
                                 v
           N parallel Expert heads (7x7 conv + BN + ReLU + 3x3 maxpool)
                                 |
                                 v
                     ResNet-18 backbone (4 residual layers)
                                 |
                                 v
                     adaptive-avg-pool + linear -> logits
```

### DEMON extraction (Equation 3-4 of the paper)

Assuming a carrier with amplitude modulation:

- `x(t) = A (1 + m sin Ωt) cos ωt`
- `|x(t)| = A (1 + m sin Ωt) · (4/π) · (1/2 + (1/(1·3)) cos 2ωt - (1/(3·5)) cos 6ωt + ...)`

After low-pass filtering (second-order zero-phase, Hamming window, length
1024), a DC component plus the modulation line `(2A/π) m sin Ωt` survives;
the Fourier transform of the squared filtered signal gives the modulation
spectrum of the sub-band. Concatenating sub-band modulation spectra (one per
250 Hz band: 10-260, 260-510, …) gives the 2D-DEMON spectrum. Summing along
the modulation-frequency axis collapses it to a 1D-DEMON spectrum whose peaks
mark shaft frequency and blade-passing frequency.

### Cross-temporal VAE (Stage 1 training)

Given a recording `wav`, sample two 30 s segments `wav_A`, `wav_B`, and
compute their 2D-DEMON spectra `d_A, d_B ∈ R^{B×1×28×1172}`. The encoder
produces the posterior parameters of a 128-dim diagonal Gaussian:
`v = enc(d_A) ∈ R^{B×256×1×287}` split into `μ` and `log σ²` along the
channel axis. A Gaussian sample `z = μ + σ ⊙ ε, ε ~ N(0, I)` is decoded
into `d̂_A ∈ R^{B×1×28×1172}`. The training target is `d_B` (the *other*
segment), not `d_A`:

```
L_VAE = L_reconstruct + L_KL
      = MSE(d_B, d̂_A) + KL( N(μ, σ² I) || N(0, I) )
```

This cross-time reconstruction forces the latent to encode only structure
that is invariant across the two segments — i.e. the true physical
characteristics of the target.

### DEMONet (Stage 2 training)

Let `s_i` be the CQT spectrogram of the i-th sample and `d_i` its 2D-DEMON
spectrum. With the VAE frozen, compute the reconstructed 1D-DEMON `D̂_1d`
and feed it through the routing layer:

```
p_i = Sigmoid( W_route · D̂_{1d,i} )     with W_route ∈ R^{N × 1172}
I_i = argmax_j p_i[j]
z_i = F( Expert_{I_i}( s_i ) )
ŷ_i = argmax z_i
```

Only the chosen expert is activated per sample (top-1 MoE), identical in
structure to the Switch-Transformer sparse routing of Fedus, Zoph and Shazeer
(2022). The combined loss is

```
L = L_task + α · L_balance
  = CE(z_i, y_i) + α · N · Σ_{i=1..N} f_i · Σ_{i=1..N} p_i^T
```

where `f_i` is the count of inputs routed to expert i and `p_i` is the
routing probability mass on expert i. The balance term is minimised when
both `f` and `p` are uniform. Authors use `α = 1e-2`.

### Hyperparameters (paper-verbatim)

| Item | Value |
|------|-------|
| Frame length / shift | 50 ms / 25 ms |
| Window | Hanning |
| Mel filter banks | 300 |
| CQT octave / time resolution | 30 / 30 |
| Segment length (train / test) | 30 s with 15 s overlap |
| Optimizer | AdamW |
| Peak LR | 5e-3 |
| Weight decay | 1e-3 |
| Schedule | Cosine annealing, 5-epoch warmup |
| Epochs | 200 |
| GPUs | NVIDIA A10 |
| Random seeds reported | 123, 3407 |
| Balance coef α | 1e-2 |
| Experts N | 5 (best) |

### Datasets

- **DeepShip** (Irfan et al. 2021): 613 recordings, 47.07 h, 32 kHz,
  4 classes (Cargo, Passenger, Tanker, Tug). Passband 10–8000 Hz.
- **DTIL** (proprietary, Ren et al. 2019, Thousand Island Lake 2019):
  39 × 15-min recordings, 17.067 kHz, 2 classes (speedboat, experimental
  vessel). Passband 10–2000 Hz.
- **ShipsEar** (Santos-Domínguez et al. 2016): 90 recordings, ~2.94 h,
  52.734 kHz, 9-class subset. Passband 100–26367 Hz.

Segment-level evaluation with non-overlapping train/test tracks (no leakage
across tracks). Authors released their splits in a GitHub repo
(xy980523/ShipsEar-An-Unofficial-Train-Test-Split).

---

## Key results (verbatim)

### DeepShip (4-class) — Table 4 of the paper

| Method | Accuracy (%) | Params (MB) |
|--------|--------------|-------------|
| TDNN+WPCS (Ren 2019) | 73.20 ± 0.49 | 2.08 |
| AGNet (Xie 2022a) | 76.99 ± 0.10 | 28.63 |
| SIR-ResNet (Xie 2023a) | 77.27 ± 0.98 | 10.66 |
| ICL CQT+Amp (Xie 2023b) | 77.72 ± 1.00 | 21.81 |
| Conv-MoE (Xie 2024) | 78.22 ± 1.40 | 10.67 |
| Baseline (CQT + ResNet-18) | 77.15 ± 1.16 | 10.66 |
| DEMONet-5expert | 80.03 ± 0.91 | 10.67 |
| **CT-VAE + DEMONet-5expert** | **80.45 ± 0.67** | 0.535 + 10.67 |

### DTIL (2-class)

- Baseline: 97.18 ± 0.14 %
- CT-VAE + DEMONet-5expert: **97.88 ± 0.25 %** (SOTA)

### ShipsEar (9-class) — Table 7

| Input feature | Model | 9-class Acc (%) |
|---------------|-------|------------------|
| STFT Amp | ResNet-18 | 75.24 |
| STFT Amp | SIR-ResNet (+LMR) | 81.90 (82.97) |
| STFT Amp | Conv-MoE | 86.21 |
| STFT Amp | CT-VAE + DEMONet | **84.92 ± 2.16** |
| Mel | CT-VAE + DEMONet | 84.05 ± 0.43 |
| CQT | CT-VAE + DEMONet | 81.04 ± 1.72 |

Authors note DEMONet *underperforms* Conv-MoE on ShipsEar because
(a) < 500 training segments is too few for 5-expert specialization and
(b) ShipsEar's modulation structure is noisier than DeepShip's, so the DEMON
routing signal is less discriminative.

### Ablation (Table 5, DeepShip / DTIL)

| VAE | CT | Balance | DeepShip % | DTIL % |
|-----|----|----|-----------|--------|
| ✗ | ✗ | ✗ | 79.54 ± 1.87 | 97.32 ± 0.43 |
| ✗ | ✗ | ✓ | 80.03 ± 0.91 | 97.46 ± 0.43 |
| ✓ | ✗ | ✗ | 79.49 ± 1.33 | 97.37 ± 0.00 |
| ✓ | ✓ | ✗ | 79.79 ± 1.05 | 97.88 ± 0.25 |
| ✓ | ✓ | ✓ | **80.45 ± 0.67** | **97.88 ± 0.25** |

Vanilla VAE (without cross-time target) *hurts* DeepShip (79.49 vs. 79.54) —
the cross-temporal training target is the load-bearing trick.

### Reconstruction similarity (Table 6, cosine similarity between DEMON
segments of the same recording)

| Class | Raw | After CT-VAE |
|-------|-----|---------------|
| Cargo | 0.9854 | 0.9878 |
| Passenger | 0.9506 | 0.9716 |
| Tanker | 0.9772 | 0.9836 |
| Tug | 0.9835 | 0.9871 |

The VAE consistently tightens inter-segment similarity, especially for
Passenger ships where variability is highest.

### Routing assignment (Figure 5)

Without balance loss: 76-81% of cargo/tanker/tug samples route to Expert 2;
Experts 3 and 5 see <5% each.

With balance loss: all 4 vessel classes distribute across experts roughly
15-25% each. The accuracy gain from balance alone is modest (+0.7 %) but the
model is much more likely to generalise to unseen signal regimes because no
single expert dominates.

---

## Strengths

- **Principled use of a physics feature without mapping it to labels.** The
  authors' negative result (Table 3) that directly fusing DEMON with CQT
  *degrades* accuracy by 2.7 % is itself a valuable finding; DEMONet's
  routing-only use is an elegant way to keep DEMON as an inductive bias
  without letting it dominate.
- **Cross-temporal VAE is a transferable idea.** Training a representation
  with segment-pair reconstruction (rather than identity reconstruction) to
  isolate time-invariant content generalises far beyond UATR — it would
  apply to bioacoustic species ID, mechanical-fault diagnostics, and
  any task where the target has a slow-varying physical signature under a
  fast-varying nuisance.
- **Near-zero parameter overhead.** DEMONet adds only 0.01-0.03 MB over the
  ResNet-18 baseline; the VAE adds 0.535 MB. Against 10.66 MB baseline, the
  ~5 % accuracy gain on DeepShip is essentially free.
- **Released train/test splits** (DeepShip Table 10, ShipsEar Table 11)
  addressing a persistent reproducibility flaw in UATR literature — most
  prior work withheld splits and reported incomparable numbers.
- **Honest negative-result section on ShipsEar.** Authors explicitly flag
  that DEMONet is data-hungry and fails gracefully on the 3-hour ShipsEar.

## Limitations

- **Only one physical feature (DEMON).** Authors acknowledge in the
  conclusion that they want to extend to multiple physical characteristics
  (LOFAR line spectra, cyclostationary features, cepstral tracks). In its
  current form the router sees only shaft/blade modulation, not tonal
  narrowband lines, which would help distinguish cargo vs. tanker with
  similar propeller counts.
- **Top-1 hard routing hurts gradients.** Sparse argmax routing means only
  one expert gets gradient per sample, so experts specialise slowly. The
  load-balancing loss partially mitigates but a soft top-k router (Switch
  v2, Expert-Choice) could train faster.
- **Scale-fragile on small data.** 5-expert DEMONet underperforms
  1-expert baselines on ShipsEar (< 500 training segments). The method
  therefore does not transfer directly to data-scarce sensor domains unless
  combined with pretraining or few-shot augmentation.
- **No in-water detection evaluation.** All experiments are closed-set
  recognition on curated ship datasets. Performance under distribution shift
  (different sea states, unseen vessel classes, bioacoustic contamination)
  is not reported.
- **No latency/throughput numbers.** For an on-buoy deployment the DEMON
  extraction pipeline (sub-band filters + demod + FFT per sub-band) is the
  dominant cost; the paper does not quantify it.

---

## Portable details (Rust implementation notes)

### Module boundaries for `weftos-sonobuoy` / `eml-core`

1. **`demon::extract_2d(signal, fs, subband_width_hz, lp_win_len)` → `Array2<f32>`**.
   Exact algorithm:
   1. Split `[f_lo, f_hi]` into contiguous sub-bands of width
      `subband_width_hz = 250`.
   2. For each sub-band, apply a band-pass FIR (linear phase).
   3. Take `|x(t)|` (rectifier).
   4. Low-pass with a zero-phase second-order Hamming-window FIR of
      length 1024.
   5. FFT of `|x_lp(t)|^2`; take modulus.
   6. Stack sub-band rows into a 2-D matrix (n_subbands × n_mod_bins).
   Output dimension for DeepShip-equivalent 8 kHz passband: 28 × 1172.

2. **`demon::collapse_1d(spec_2d)` → `Array1<f32>`**. `sum_axis(Axis(0))`.

3. **`cross_temporal_vae::CTVAE`** (EML-core burn or tch-rs model):
   - Encoder: four `Conv2d` with `(k=4,s=2,p=1), (k=4,s=2,p=1), (k=5,s=1,p=0),
     (k=3,s=1,p=0)` and channels `1 → 64 → 64 → 64 → 256`, BN + ReLU between.
   - Latent split: 128-dim μ and 128-dim log σ².
   - Reparameterisation: `z = μ + exp(0.5 log σ²) ⊙ ε`.
   - Decoder: inverse ConvTranspose2d stack, ends with `Tanh`.
   - Loss: `MSE(x_B, decoder(encoder(x_A))) + KL(q(z|x_A) || N(0,I))`
     with sampling of segment pairs from same recording.

4. **`routing::Router`**: `Linear(1172, N_experts)` → `Sigmoid` → `argmax`.
   N = 5 is the sweet spot.

5. **`experts::Expert[k]`**: `Conv2d(1 → 64, k=7, s=2, p=3)` + BN + ReLU +
   `MaxPool(k=3, s=2, p=1)`. Identical shape per expert; only weights differ.

6. **`backbone::ResNet18Body`**: four residual stages with 2 basic blocks
   each, channel plan `64-64 | 64-128 | 128-256 | 256-512`, adaptive avg
   pool to `(1,1)`, final `Linear(512 → n_classes)`.

### Balance loss (PyTorch-style, translate to burn/tch)

```rust
// f_i : counts per expert over the batch
// p_i : mean routing probability per expert over the batch
fn balance_loss(f: &Tensor, p: &Tensor, n_experts: usize) -> Tensor {
    let alpha = 1e-2f32;
    alpha * (n_experts as f32) * f.dot(&p.transpose())
}
```

### Training schedule

- AdamW(lr=5e-3, wd=1e-3), cosine LR, 5-epoch warmup, 200 epochs.
- Segment length 30 s, 15 s overlap, 50 ms / 25 ms framing.
- Eval at segment level, not file level, to disambiguate ties.
- Seeds: fix two (e.g. 123, 3407); report mean ± std across seeds.

### Reproducibility checklist

| Check | Paper value |
|-------|-------------|
| Train/test split published | Yes (Tables 10–11, GitHub) |
| Hyperparameters complete | Yes (Section 4) |
| Code released | Partial (split only) |
| Hardware cost | 1× A10, 200 epochs, ~half a day per config |

---

## Sonobuoy integration plan

DEMONet slots cleanly into the **temporal branch** of the K-STEMIT-extended
dual-branch architecture documented in
`.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`. The proposed shape:

- **New Rust crate** `weftos-sonobuoy-temporal` (workspace member) whose
  public API exposes `fn recognize_segment(signal: &[f32], fs: f32) ->
  TemporalEmbedding`. Internally it builds the 2D-DEMON spectrum, runs the
  frozen cross-temporal VAE, reconstructs the 1D-DEMON spectrum, routes
  through one of N experts, and emits a 512-dim embedding from the
  ResNet-18 backbone's penultimate layer.
- **EML-core fit**: the per-expert conv + backbone stack is a learned
  function in the sense of `eml-core`; the encoder/decoder/experts become
  EML modules registered in the EML learned-function registry, making them
  hot-swappable and learnable via the EML training loop.
- **HNSW integration** (`clawft-kernel/src/hnsw_service.rs`): the 512-dim
  backbone embedding is the natural key for per-vessel HNSW retrieval.
  During operation, each 30-s segment's embedding is upserted under a
  vessel-track GUID; bearing estimates from the spatial branch disambiguate
  tracks, and HNSW k-NN over all historical embeddings per track provides
  few-shot vessel re-identification across a patrol.
- **Quantum register** (`clawft-kernel/src/quantum_register.rs`): the top-1
  expert selection maps naturally to a 3-qubit expert-ID register (log2(5) ≈
  3 qubits; actually 3 qubits for 5 states, one-hot-encoded). This lets
  downstream cognitive-layer circuits reason jointly over "which expert
  fired" (physical regime: submarine, cargo, small craft, biological,
  unknown) and the spatial-branch bearing estimate.
- **Dual-branch fusion**: in K-STEMIT's learnable-α spatio-temporal fusion
  `y = α · spatial_feat + (1-α) · temporal_feat`, the temporal_feat is this
  DEMONet embedding; spatial_feat comes from the GraphSAGE array-geometry
  branch. α adapts per task: detection ≈ 0.2 (mostly temporal), bearing ≈
  0.8 (mostly spatial), species ID ≈ 0.4-0.6 (balanced).
- **EML coherence gate**: because DEMONet is explicitly trained to be
  robust to environmental interference (via CT-VAE), it is a natural
  candidate for being the "ground-truth" temporal branch during
  EML-coherence training — any K-STEMIT-style fused model's temporal output
  should agree with standalone DEMONet under environmental perturbation.

**Build sequencing (suggested ADR update)**:

1. Port DEMON feature extraction to pure Rust (crate
   `weftos-sonobuoy-dsp`). Pure-CPU; no GPU needed. This is the lowest-risk
   P0 step and unblocks the rest of the sonobuoy pipeline.
2. Ingest DeepShip/ShipsEar as parquet shards via
   `weftos-sonobuoy-data`; verify DEMON extraction matches the reference
   implementation within 1 e-5 MSE.
3. Port the CT-VAE to burn/tch. Train on CPU or a single consumer GPU for
   DeepShip (the VAE is tiny; 0.535 MB).
4. Port DEMONet's routing + experts + backbone. Reproduce the 80.45 %
   DeepShip number before moving on to ShipsEar.
5. Wire into the K-STEMIT dual-branch crate as the temporal encoder.

---

## Follow-up references (cited works that matter)

1. **Irfan et al. 2021** — *DeepShip: An underwater acoustic benchmark
   dataset and a separable convolution based autoencoder for classification.*
   Expert Systems with Applications 183, 115270. The benchmark itself; must
   be downloaded and ingested to replicate any DEMONet result.
2. **Santos-Domínguez et al. 2016** — *Shipsear: An underwater vessel noise
   database.* Applied Acoustics 113, 64-69. Smaller (2.9 h) but older
   reference benchmark; useful for data-scarcity ablations.
3. **Fedus, Zoph, Shazeer 2022** — *Switch Transformers: Scaling to Trillion
   Parameter Models with Simple and Efficient Sparsity.* JMLR 23:5232-5270.
   The Mixture-of-Experts + load-balance-loss foundation; DEMONet is a
   domain-specialised adaptation of the same routing scheme.
4. **He, Zhang, Ren, Sun 2016** — *Deep residual learning for image
   recognition.* CVPR. The backbone DEMONet reuses.
5. **Xu, Xie, Wang 2023** — *Underwater Acoustic Target Recognition based on
   Smoothness-inducing Regularization and Spectrogram-based Data
   Augmentation.* Ocean Engineering 281, 114926 (arXiv:2306.06945).
   Complementary paper from the same group; DEMONet's default loss could
   additionally be augmented with the smoothness-inducing KL terms from
   this work for synergistic robustness. Analysed separately in
   `smoothness-uatr.md`.
