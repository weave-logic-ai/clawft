# NOAA Humpback-Song CNN (and the DIFAR/Conformer Gap)

## Citation Header

- **Verified Title**: *A Convolutional Neural Network for Automated Detection of Humpback Whale Song in a Diverse, Long-Term Passive Acoustic Dataset*
- **Authors**: Ann N. Allen, Matt Harvey, Lauren Harrell, Aren Jansen, Karlina P. Merkens, Carrie C. Wall, Julie Cattiau, Erin M. Oleson
- **Venue**: Frontiers in Marine Science, Vol. 8, Art. 607321, Marine Megafauna section
- **Year**: 2021 (with a 2021 erratum, DOI `10.3389/fmars.2021.708219`)
- **DOI**: `10.3389/fmars.2021.607321`
- **Primary URL**: https://www.frontiersin.org/articles/10.3389/fmars.2021.607321/full
- **Released model**: https://tfhub.dev/google/humpback_whale/1
- **Released data**: https://doi.org/10.25921/Z787-9Y54
- **PDF mirrored to**: `.planning/sonobuoy/papers/pdfs/noaa-difar-conformer.pdf` (Frontiers PDF, 2.3 MB)

## Status

**Substituted** — from *"Allen, Oleson et al., JASA Express Letters, 2024, Conformer-based DIFAR detector with MIL pooling"* to *Allen, Harvey, Harrell, Jansen, Merkens, Wall, Cattiau, Oleson, Frontiers in Marine Science, 2021, ResNet-50 CNN humpback song detector on NOAA HARP omnidirectional PAM data.*

Why substitute: extensive Cross-ref, Google Scholar, and arXiv searches
(terms `Allen Oleson JASA DIFAR Conformer`, `Conformer DIFAR marine
mammal`, `weakly labeled whale Conformer sonobuoy`) returned **no
publication matching the survey description** — there is no Conformer +
DIFAR + MIL paper from the Allen/Oleson group (or anyone else) in JASA
Express Letters 2024 or adjacent years. The closest *real* artefacts are:

- **Allen, Harvey, Harrell, …, Oleson (2021)** — same NOAA/Google
  collaboration, weak-label active-learning pipeline, CNN (not
  Conformer), omnidirectional HARP recorders (not DIFAR). Picked as
  primary substitute because author list, institution, weak-label
  methodology and NOAA-deployed-model angle are all correct; architecture
  and hydrophone type differ.
- **Nihal, Yen, Shi, Nakadai (2025), arXiv `2502.20838`** — *Weakly
  Supervised Multiple Instance Learning for Whale Call Detection and
  Temporal Localization in Long-Duration Passive Acoustic Monitoring.*
  The correct paper for the "MIL + weak labels + temporal localisation"
  part of the survey claim. Cited below as the canonical MIL reference.
- **Thode, A., Kim, K., Norris, T., et al. (2019).** *Displaying
  bioacoustic directional information from sonobuoys using "azigrams".*
  J. Acoust. Soc. Am. 146 (1), 95. DOI `10.1121/1.5114810`. — The
  canonical DIFAR + bioacoustic-bearing paper for anyone building on
  three-channel sonobuoy data. Cited below.

Taken together the substitution covers (1) the deep-learning detector
(Allen 2021), (2) the weak-label/MIL extension (Nihal 2025), and (3) the
DIFAR azigram bearing primitive (Thode 2019). No Conformer appears in any
of them; the "Conformer" in the survey description is almost certainly a
conflation with the NatureLM-audio or bioacoustic transformer literature.

---

## One-Paragraph Summary

Allen et al. train a ResNet-50 image classifier on log-mel spectrograms
to detect humpback whale (*Megaptera novaeangliae*) song in 187 000
hours of NOAA Pacific Islands Fisheries Science Center HARP recordings
— ~14 years of passive acoustic data from 13 sites in the North Pacific.
The modelling choices are deliberately pragmatic: a vanilla ImageNet-style
ResNet-50 with the first 7 × 7 conv stride reduced from 2 to 1 (because
spectrograms are lower resolution than photos), per-channel energy
normalisation (PCEN) with trainable parameters replacing log-mag
compression, 3.84-second context windows, and **four rounds of
model-dependent active learning** where each round targets the previous
model's highest-scoring false positives and hardest negatives. At
deployment the model reaches **average precision 0.97** and **AUC-ROC
0.992** across sites, vs. 0.74 precision / 0.91 AUC for the classic
Generalized Power Law detector. The scientific payoff is the first
detection of humpback song at Kingman Reef (5° N), well south of the
previously-known wintering range. For the sonobuoy stack the model is
the reference "whale-present?" clip-level detector; the MIL extension
(Nihal 2025) and the DIFAR bearing primitive (Thode 2019) must be
bolted on separately.

---

## Methodology

### Architecture

- **Backbone**: ResNet-50 (He et al. 2016) initialised from ImageNet.
- **Modification**: the first 7 × 7 convolutional layer's stride is
  reduced from 2 → 1, because a 128 × 96 spectrogram is roughly
  4× lower-resolution than a 224 × 224 ImageNet image and the full
  2× stride would lose too much time/frequency detail. No other
  architectural change.
- **Output**: binary sigmoid head (song present / absent per 3.84 s clip).

### Input representation

- **STFT**: Hann window, **1 024 samples ≈ 100 ms**.
- **Time hop**: swept over {10, 30, 50} ms; optimal context was a
  **3.84-second window** of stacked frames giving a **128 × 96
  (time × freq)** spectrogram.
- **Frequency range**: swept over {0–5 kHz, 100 Hz–1 kHz, 50 Hz–2 kHz};
  optimal was **0–5 kHz**.
- **Compression**: **Per-Channel Energy Normalisation (PCEN)** with
  trainable per-channel parameters (Wang, Getreuer, Hughes & Lyon 2017);
  beat log and cube-root compression consistently.
- **Mel filterbank**: triangular filterbank over squared FFT magnitudes.

### Loss

Binary cross-entropy on the clip-level present/absent label. The paper
does not introduce a new loss. Negative clips are sampled
**implicitly** from unlabelled hour-long intervals (operator-verified as
whale-free); this is the "weak label" aspect — negative labels are at
hour scale, positives are at call scale.

### Training data and active learning

- **Raw corpus**: 187 000 h from 13 HARP (High-frequency Acoustic
  Recording Package) bottom-moored hydrophones in the North Pacific,
  2005–2019. Depths 111–1 266 m. Duty cycles continuous to 5 min per
  40 min. Raw sample rate 200 kHz, decimated to **10 kHz** (10 Hz–5 kHz
  bandwidth) for the detector.
- **Annotated subset**: 291.8 h (0.12 % of corpus) with strong,
  call-level labels.
- **Validation**: 6.25 h held out with 300 stratified segments per site.
- **Active learning**: four rounds. Each round:
  1. Run current model over unlabeled data.
  2. Pull the top-scoring false positives (hard negatives) and
     low-scoring ambiguous clips.
  3. Send to analysts for annotation.
  4. Retrain.

### Training schedule

- On-the-fly augmentation: random time-axis shift of ±16 spectrogram
  bins (≈ ±160 ms at the 10 ms hop).
- Shuffle buffer of **2 048** contexts.
- Optimiser/LR/batch-size not explicitly pinned in the paper. Given
  publication timing and Google infrastructure, typical Adam at
  `lr ≈ 1e-4` with batch 64 is a safe reconstruction.

### Hardware

Google TPU v2/v3 pods implied by the Google co-authorship; not
explicitly pinned in the text.

---

## Key Results

All numbers are segment-level (3.84 s, threshold 0.5 on sigmoid):

| Model | Precision | AUC-ROC |
|---|---|---|
| Energy detector | 0.22 | — |
| Generalized Power Law (GPL) | 0.74 | 0.91 |
| ResNet-50 + PCEN + active learning | **0.97** | **0.992** |

Per-site performance envelope:
- Precision range: **0.93–1.00** across 13 sites.
- AUC range: **0.98–1.00**.

Scientific findings enabled by the detector:

- Humpback song detected at **Kingman Reef (5° N)** — a previously
  unreported wintering-range detection, pushing the known population's
  southern boundary south of Hawaii.
- Multi-year seasonal occupancy patterns resolved at all 13 sites
  without additional analyst effort.

### What the paper does *not* do

- **No direction-of-arrival.** All data are from single-channel
  pressure hydrophones; no bearing, no DIFAR.
- **No MIL.** Labels are strong (analyst-annotated call boundaries);
  only the implicit negatives are weak.
- **No Conformer / transformer.** Pure ResNet-50 CNN.
- **No species classification.** Binary humpback song / no-song;
  species-level ID is out of scope.

---

## Strengths

- **Scale**: 187 000 h processed, 14 years, 13 sites — by far the
  largest open-benchmark humpback corpus at publication time.
- **Real operational gain**: +0.23 precision over the state-of-the-art
  classical GPL detector, which translates directly into fewer
  false-positive analyst hours.
- **Active learning is practical**: the four-round analyst loop is a
  template any sonobuoy deployment can copy, and the paper quantifies
  its effect.
- **PCEN beats log-mag consistently**: a clean, portable finding for
  any spectrogram-based whale detector.
- **Model and data are public**: TF Hub checkpoint + DOI-cited HARP
  dataset make it immediately usable as a baseline.

## Limitations

- **Not DIFAR, not multi-channel**: single pressure hydrophone only;
  cannot be used for bearing without augmentation.
- **Not MIL**: requires call-level annotations for positives; weak-only
  supervision (e.g. file-level presence) would need a different head.
- **Binary per species**: scaling to a multi-species sonobuoy
  detector requires N separate detectors or a shared multi-label head,
  neither of which this paper evaluates.
- **Segment-level output**: no within-clip localisation — call onset
  times come from threshold crossings, not a localisation head.
- **Spectrogram resolution**: 128 × 96 is coarser than most modern
  bioacoustic detectors (typically 256 × 128+); fine-structure calls
  (e.g. odontocete clicks) would likely need a different front-end.

---

## Portable Details (Rust-implementable)

### Exact spectrogram pipeline

```
ALGORITHM AllenSpectrogram(pcm_10kHz)
  1. FOR each 3.84-second window at 1.92-second hop:
        stft = STFT(window,
                    hann(1024),        # 1024-sample Hann
                    hop = 100 ms)      # paper's optimal hop
        mag  = |stft|²
        mel  = mel_filterbank(mag,
                              n_mels = 96,
                              f_min  = 0 Hz,
                              f_max  = 5000 Hz)
        pcen = PCEN(mel,
                    alpha = trainable,
                    delta = trainable,
                    r     = trainable,
                    eps   = 1e-6,
                    smoothing = IIR with time constant T≈60 ms)
  2. return pcen                          # shape: 128 × 96
```

### PCEN equation (portable)

Per-channel energy normalisation:

```
M_t[c] = (1 − s) · M_{t−1}[c] + s · E_t[c]           # IIR smoothing
PCEN_t[c] = ( E_t[c] / (ε + M_t[c])^α + δ )^r − δ^r
```

where `E_t[c]` is the squared magnitude at mel bin `c`, time frame `t`,
and `(α, δ, r, s)` are trainable per-channel parameters (Wang et al. 2017).
`s` is the smoother coefficient derived from a time constant `T ≈ 60 ms`.

### ResNet-50 modification

```
# Standard TorchVision-style ResNet-50 except:
conv1 = nn.Conv2d(1, 64, kernel_size=7, stride=1, padding=3, bias=False)
#                                       ^^^^^^^^^ stride 2 → 1
```

### Training hyperparameters worth freezing

- Input: 10 kHz mono PCM, 3.84-second windows, 1.92-second hop.
- Spectrogram: Hann 1024 / 100 ms hop, 96 mel bins, 0–5 kHz, PCEN.
- Model: ResNet-50 with `conv1.stride=1`, single-class sigmoid head.
- Augmentation: ±16 time-bin random shift.
- Loss: binary cross-entropy.
- Active-learning cycles: 4 rounds of hard-negative mining.
- Threshold: 0.5 on sigmoid (operator-tunable per site).

### What to add on top of Allen-2021 to get the survey's stated paper

For the sonobuoy stack we actually need the features the (non-existent)
"Conformer + DIFAR + MIL" paper promised. Implement them as three
modular add-ons:

1. **MIL head (Nihal 2025)**: replace the sigmoid head with a
   dual-stream attention pooling (DSMIL) head. For a bag of `K`
   3.84-s instances:
   ```
   a_k = softmax_k( w_a · tanh(W · z_k) )         # attention weights
   Z   = Σ_k a_k · z_k                             # bag embedding
   p   = σ(w_o · Z + b)                            # bag label
   ```
   Train with bag-level BCE; the `a_k` weights give per-instance
   localisation for free.
2. **DIFAR bearing (Thode 2019)**: take the pressure channel `P`, the
   two dipole channels `X`, `Y` (all 10 kHz after standard DIFAR
   FM-demod at 15 kHz). For each TF bin `(t, f)` compute the active
   intensity vector:
   ```
   I_x(t, f) = Re{ X*(t, f) · P(t, f) }
   I_y(t, f) = Re{ Y*(t, f) · P(t, f) }
   θ(t, f)  = atan2( I_y(t, f), I_x(t, f) )       # azigram bearing
   ```
   The azigram (Thode et al. 2019, JASA 146:95) is the directional
   complement to the Allen detector's clip score.
3. **Conformer upgrade (optional)**: swap the ResNet-50 backbone for a
   Conformer encoder (Gulati et al. 2020). For bioacoustic sequences
   use 6 blocks, `d_model = 144`, 4 heads, conv-kernel 31. No paper
   publishes this configuration for humpback detection; treat it as
   research, not production.

---

## Sonobuoy Integration Plan

In the K-STEMIT-extended sonobuoy pipeline this paper supplies the
**clip-level "whale present?" head**. A deployed DIFAR sonobuoy emits a
three-channel 15 kHz FM stream; after standard demodulation we obtain
three 10 kHz PCM channels `(P, X, Y)`. Channel `P` is fed into the
Allen-2021 pipeline exactly as published (HARP input sample rate matches):
PCEN-mel, 3.84 s windows at 1.92 s hop, ResNet-50 with `conv1.stride=1`,
single sigmoid head. The TF Hub checkpoint is loaded directly and can be
re-exported to ONNX for the Rust inference path in `clawft-kernel`.
Simultaneously, channels `(P, X, Y)` feed the Thode-2019 azigram module,
which produces a per-TF-bin bearing. At the orchestration layer, whenever
the Allen detector's sigmoid fires above threshold on a clip, the
azigram's bearing histogram over the firing window is snapshotted and
attached to the detection event — giving a `(time, species_prob,
bearing_deg, bearing_confidence)` tuple per call. To close the "MIL +
temporal localisation" gap in the original survey claim, the sigmoid
head is replaced with a DSMIL pooling head (Nihal 2025) that accepts
bags of 5–15 instances (≈ 10–30 s of audio) and outputs both a bag label
and per-instance attention weights, giving sub-clip localisation without
requiring call-level labels. Species-level classification is delegated
to the BirdNET-cetacean embedding head (see `birdnet-cetaceans.md`);
this paper's model stays strictly binary.

---

## Follow-up References

1. **Nihal, R. A., Yen, B., Shi, R., Nakadai, K.** *Weakly Supervised
   Multiple Instance Learning for Whale Call Detection and Temporal
   Localization in Long-Duration Passive Acoustic Monitoring.*
   arXiv `2502.20838` (2025). — Fills in the MIL-on-weak-labels gap
   that the survey mis-attributed to Allen/Oleson. DSMIL-LocNet
   architecture, bag-level-only labels, F1 0.8–0.9.
2. **Thode, A., Kim, K. H., Norris, T., Oleson, E. M., et al.**
   *Displaying bioacoustic directional information from sonobuoys using
   "azigrams".* J. Acoust. Soc. Am. 146 (1), 95 (2019).
   DOI `10.1121/1.5114810`. — The canonical DIFAR-bearing primitive.
   Required for any sonobuoy localisation head.
3. **Wang, Y., Getreuer, P., Hughes, T., Lyon, R. F., Saurous, R. A.**
   *Trainable frontend for robust and far-field keyword spotting.*
   ICASSP 2017. — The PCEN layer; directly lifted into Allen 2021.
4. **Gulati, A., Qin, J., Chiu, C.-C., et al.** *Conformer:
   Convolution-augmented Transformer for Speech Recognition.*
   Interspeech 2020. — Reference point for the "Conformer" piece of the
   survey description; a plausible future drop-in backbone.
5. **McCullough, J. L. K., Simonis, A. E., Sakai, T., Oleson, E. M.**
   *Acoustic classification of false killer whales in the Hawaiian
   islands based on comprehensive vocal repertoire.* JASA Express
   Letters 1 (7), 071201 (2021). DOI `10.1121/10.0005512`. — A real
   Oleson-group JASA-EL paper; useful contrast point for species-level
   cetacean classification (not MIL, not Conformer, not DIFAR).
