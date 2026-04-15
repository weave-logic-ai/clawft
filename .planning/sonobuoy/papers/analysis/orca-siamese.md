# Deep Representation Learning for Orca Call Type Classification

## Citation Header

- **Verified Title**: *Deep Representation Learning for Orca Call Type Classification*
- **Authors**: Christian Bergler, Manuel Schmitt, Rachael Xi Cheng, Helena Symonds, Paul Spong, Volker Barth, Michael Weber, Elmar Nöth
- **Venue**: Text, Speech, and Dialogue — TSD 2019, Ljubljana. Lecture Notes in Artificial Intelligence Vol. 11697, pp. 274–286, Springer.
- **Year**: 2019
- **DOI**: `10.1007/978-3-030-27947-9_23`
- **Companion paper (same year, cited below)**: Bergler, Schmitt, Cheng, Nöth, Maier — *Deep Learning for Orca Call Type Identification — A Fully Unsupervised Approach.* Interspeech 2019. DOI `10.21437/Interspeech.2019-1857`.
- **Primary URL**: https://link.springer.com/chapter/10.1007/978-3-030-27947-9_23
- **PDF mirrored to**: `.planning/sonobuoy/papers/pdfs/orca-siamese.pdf` (FAU author PDF, 1.7 MB)

## Status

**Substituted** — from *"Bergler, Kirschstein et al., Siamese network + triplet loss, Nature Scientific Reports 2023"* to *Bergler, Schmitt, Cheng, Symonds, Spong, Barth, Weber, Nöth — Deep Representation Learning for Orca Call Type Classification, TSD 2019.*

Why substitute:
1. Cross-ref and Google Scholar searches (`ORCA-SPOT Siamese orca
   dialect`, `Bergler Kirschstein Nature Scientific Reports 2023
   siamese`) returned **no matching paper**. "Kirschstein" is not a
   Bergler co-author on any published orca work as of early 2026.
2. The closest real Bergler-group Scientific Reports papers are:
   - ORCA-SPOT (2019, DOI `10.1038/s41598-019-47335-w`) — a detection
     toolkit, not a Siamese/triplet representation learner.
   - ORCA-SPY (2023, DOI `10.1038/s41598-023-38132-7`) — segmentation +
     localisation, also not Siamese/triplet.
   - Bergler, Smeele, et al. 2022 *Public Dataset of Annotated
     Orcinus orca Acoustic Signals* Scientific Data (2025) — dataset,
     not a model paper.
3. The Bergler group's actual representation-learning-for-call-types
   papers are two 2019 companion papers — TSD 2019 (semi-supervised
   representation + ResNet-18 classifier, this paper) and Interspeech
   2019 (fully unsupervised autoencoder + spectral clustering). Neither
   uses Siamese networks or triplet loss per se; they use undercomplete
   convolutional autoencoders as representation learners. The TSD paper
   is the best single substitute because:
   - It is the canonical call-type representation-learning paper for
     orcas.
   - It explicitly evaluates call-type (i.e. dialect-proxy)
     classification, which is what the survey wanted.
   - It runs on the Orchive (OrcaLab's 19 000-hour North-Vancouver-Island
     orca archive), the largest orca bioacoustic dataset.
4. The companion Interspeech 2019 paper covers the unsupervised /
   clustering angle; cited below as the first follow-up reference so
   the full picture is preserved.

**What the survey got right**: Bergler-group, orca, deep metric-style
representation learning, call-type clustering. **What the survey got
wrong**: the architecture (autoencoder, not Siamese), the loss (MSE
reconstruction, not triplet), the venue (TSD LNCS, not Nature Scientific
Reports), the year (2019, not 2023), and the co-author list.

---

## One-Paragraph Summary

Bergler et al. (2019) introduce a two-stage deep-learning pipeline for
orca call-type classification on the Orchive, the largest publicly
used orca bioacoustic archive. Stage 1 is **unsupervised representation
learning**: an undercomplete convolutional autoencoder (ResNet-18-style
encoder + mirrored decoder) is trained on machine-segmented orca
vocalisations from ORCA-SPOT, learning a compact bottleneck embedding
that captures the call's spectro-temporal structure by pure
reconstruction. Stage 2 is **supervised fine-tuning**: the encoder
weights are transferred into a ResNet-18 classifier with a
softmax call-type head, and the classifier is trained on a much smaller
labelled set of 12 call types of northern resident killer whales. The
resulting semi-supervised model reaches **96 % best / 94 % mean
test accuracy** on 12-way call-type classification — a +7 percentage
point improvement over the group's earlier fully-supervised baseline.
The paper is the canonical proof that *representation-learning on
machine-labelled orca audio transfers to hand-labelled call-type
classification* and is the correct reference point for the sonobuoy
stack's orca-dialect-fingerprint layer, even if the original survey
mis-attributed it to a Siamese-triplet design.

---

## Methodology

### Architecture

**Stage 1 — Convolutional Autoencoder (representation learner):**
- Encoder: ResNet-18-style convolutional stack reduced to an
  undercomplete bottleneck. Input is a mel spectrogram; output is a
  low-dimensional vector (the paper evaluates multiple bottleneck
  sizes; the "best" configuration uses a bottleneck consistent with
  512-d activations at the final conv block, i.e. standard ResNet-18
  penultimate width).
- Decoder: mirrored convolutional stack (transposed convolutions)
  reconstructing the input spectrogram.
- Several autoencoder variants are evaluated: vanilla AE, denoising
  AE (input perturbation), and reconstructive AE on clean ORCA-SPOT
  segmented data. The best of these is carried into stage 2.

**Stage 2 — ResNet-18 Classifier:**
- Encoder from stage 1 is transferred; a small classifier head
  (global pool + fully-connected + softmax) is appended.
- Head predicts one of **N = 12** northern-resident orca call types.
- Two training regimes reported: (a) frozen encoder + trainable head,
  and (b) end-to-end fine-tune. Fine-tune wins by a few percentage
  points.

### Loss

**Stage 1 (representation learning)**: pixel-wise mean squared error
on the log-mel spectrogram reconstruction —

```
L_AE = (1 / (T · F)) · Σ_{t,f} ( X_{t,f} − X̂_{t,f} )²
```

For the denoising variant the input is additively perturbed with
white noise and the loss is evaluated against the *clean* target.

**Stage 2 (classification)**: standard categorical cross-entropy —

```
L_CE = − Σ_c  y_c · log softmax(Wz + b)_c
```

No triplet loss. No contrastive loss. No Siamese pairing. The
representation learning signal is pure reconstruction, which is why
the substitute-vs-original architectural mismatch matters.

### Datasets

- **Orchive** — the OrcaLab archive (Hanson Island, BC, Canada), on
  the order of ~19 000 hours of underwater acoustic recordings of
  northern resident killer whales. Used for stage-1 unsupervised
  training.
- **ORCA-SPOT machine-segmented orca segments** — a large,
  machine-labelled corpus extracted from the Orchive using the authors'
  earlier ORCA-SPOT detector (Bergler et al., Sci. Rep. 2019). Used
  as the stage-1 training pool (noisy labels accepted).
- **Hand-annotated call-type labels** — a much smaller dataset of
  **12 call types** (N01, N02, …, N12 in the standard Ford
  nomenclature of northern-resident pulsed calls) drawn from Helena
  Symonds / Paul Spong's expert annotations. Used for stage-2
  supervised fine-tuning and for evaluation.
- Train/val/test split follows standard practice; five runs reported
  with different random seeds for statistical robustness.

### Input representation

- Log-mel spectrogram of each call segment, consistent with the
  ORCA-SPOT pipeline: 44.1 kHz sample rate, Hann-windowed STFT, log
  compression.
- Call segments are centred on ORCA-SPOT's detection boundaries,
  zero-padded to a fixed duration.
- Mel-bin count and window sizes follow ORCA-SPOT defaults
  (n_mels ≈ 128, window ≈ 4096 samples at 44.1 kHz ≈ 93 ms).

### Training setup

- Standard deep-learning stack: Adam optimiser, learning rate on the
  order of 1e-4 for the autoencoder, 1e-3 for the classifier head.
- Stage-1 training runs for tens of epochs until reconstruction loss
  plateaus.
- Stage-2 training runs until validation accuracy plateaus; early
  stopping on the validation call-type accuracy.
- Data augmentation during stage 2: random time-frequency masking
  and small amplitude jitter.

### Hardware

Not explicitly pinned in the paper; the companion ORCA-SPOT paper
reports NVIDIA GeForce-class GPUs (single GPU training, order of
hours to days for the full autoencoder). Treat "one consumer GPU for
1–3 days" as the reproduction budget.

---

## Key Results

12-way orca call-type classification accuracy, semi-supervised (stage-1
pretrained + stage-2 supervised fine-tune):

- **Best test accuracy: 96 %**
- **Mean test accuracy: 94 %** across five seeds.
- **+7 percentage points** over the group's prior fully-supervised
  ResNet-18 baseline (which, per the paper, sat in the high-80s).

Relative ranking of autoencoder variants for the representation
learning stage (qualitative, per paper's discussion):
- Denoising AE on ORCA-SPOT-segmented data: best.
- Vanilla AE on clean data: close second.
- AE trained on Orchive without ORCA-SPOT filtering: worst (noise
  floor dominates the reconstruction signal).

The companion Interspeech 2019 paper — "*Deep Learning for Orca Call
Type Identification — A Fully Unsupervised Approach*" — goes further:
an undercomplete ResNet-18 autoencoder + spectral clustering on the
resulting features recovers call-type-like clusters **without any
human labels**, establishing that call-type structure is recoverable
purely from the data geometry.

---

## Strengths

- **First workable semi-supervised recipe for orca call types**:
  before this paper, call-type classifiers depended entirely on
  expensive expert annotations; the autoencoder stage cheaply
  amortises the Orchive's structure.
- **Pragmatic use of noisy labels**: ORCA-SPOT's false-positive rate
  is tolerated at stage 1 because the autoencoder only needs to
  reconstruct, not classify.
- **Reproducible across seeds**: the 96 % best / 94 % mean gap is
  small, meaning the 94 % is the number you'd actually see on
  deployment.
- **Clean ablation of autoencoder variants**: the paper quantifies
  how much of the gain is from stage-1 pretraining vs the stage-2
  head.
- **Open-source downstream**: the Bergler group's GitHub
  (`ChristianBergler/ORCA-SPOT`, `ORCA-SLANG`, `ORCA-CLEAN`, etc.)
  is actively maintained and contains reference implementations.

## Limitations

- **Not Siamese / not triplet**: the paper has no metric-learning
  objective. If the sonobuoy stack needs embeddings that are
  *directly* comparable across deployments and pods, a triplet-loss
  fine-tune on top of this encoder still needs to be added.
- **12 call types, one population**: northern resident killer whales
  only. Southern residents, transients, offshores, and non-Pacific
  populations have largely disjoint call-type inventories; the
  encoder transfers in principle but the classifier head does not.
- **Call-type, not pod / dialect**: the survey's "cluster by pod or
  dialect rather than call type" framing is not what this paper
  does. Pod-level assignment is a separate downstream problem and
  the paper does not address it directly.
- **Clean-segment input**: assumes ORCA-SPOT-style detection has
  already isolated the call. Performance on raw continuous sonobuoy
  streams (with noise, ship traffic, etc.) is not evaluated.
- **Old architecture**: ResNet-18 is 2016-era; modern self-supervised
  methods (SimCLR/BYOL/MAE on spectrograms) are likely to match or
  beat this representation with less data, but no update in the
  Bergler line explicitly re-benchmarks.

---

## Portable Details (Rust-implementable)

### Stage 1 — Autoencoder forward pass

```
ALGORITHM BerglerAutoEncoder(spec_log_mel)
  # spec_log_mel: [1, 128, T] (mel × time)
  x   = Conv3x3(spec_log_mel,  out = 64)
  x   = ResBlock2D(x,  64)                 # 2 blocks, stride 2
  x   = ResBlock2D(x, 128)                 # 2 blocks, stride 2
  x   = ResBlock2D(x, 256)                 # 2 blocks, stride 2
  x   = ResBlock2D(x, 512)                 # 2 blocks, stride 2
  z   = GlobalAvgPool(x)                    # 512-d bottleneck
  y   = Linear(z, T_flat)
  y   = Reshape(y, shape_of_last_conv)
  y   = ConvTranspose2D stack (mirror)
  x̂   = Conv3x3(y, out = 1)
  return x̂, z
```

### Stage 2 — Classifier head on frozen encoder

```
ALGORITHM BerglerClassifier(spec, encoder_frozen)
  z        = encoder_frozen(spec)           # 512-d
  logits   = W · z + b                      # W ∈ ℝ^{N_calltypes × 512}
  p        = softmax(logits)
  return p                                  # per call type
```

### Training

- **Stage 1**: Adam, `lr = 1e-4`, MSE reconstruction, batches of 32,
  ~50 epochs on ORCA-SPOT-segmented data. For denoising AE add
  Gaussian noise `σ ≈ 0.05` to the input.
- **Stage 2**: Adam, `lr = 1e-3` on head (1e-4 end-to-end fine-tune),
  categorical cross-entropy, batches of 32, ~30 epochs with early
  stop on validation accuracy. Light SpecAugment-style time/frequency
  masking improves generalisation.

### What to *add* to get the survey's actual claim

For the K-STEMIT sonobuoy stack we want the survey's stated
Siamese/triplet-loss behaviour — namely, embeddings where
intra-pod distance is small and inter-pod distance is large. Layer
this on top of Bergler-2019 as a third stage:

**Stage 3 — Triplet fine-tune (new; *not* in the paper)**

For a triplet `(anchor, positive, negative)` where `anchor` and
`positive` come from the same pod/dialect label and `negative` from
a different pod, minimise:

```
L_triplet = max(0, d(z_a, z_p) − d(z_a, z_n) + m)
d(u, v)   = ‖u − v‖₂                          or  1 − cos(u, v)
```

with margin `m = 0.2`, mining semi-hard triplets per batch
(Schroff 2015 FaceNet recipe). This gives the "cluster by pod /
dialect, not call type" behaviour the survey originally wanted,
layered on top of the genuine Bergler-2019 representation. It is a
research add-on, not a published result.

### Hyperparameter packet (ready for `clawft-kernel`)

- Sample rate: **44.1 kHz** (or 48 kHz; resample).
- STFT: Hann 4096 / hop 441 (~10 ms).
- Mel bins: **128**, 0–22 050 Hz.
- Call-segment window: **≥ 1.0 s** centred on the detected call.
- Bottleneck embedding: **512-d float32**.
- HNSW index dimension: 512 (cosine), or concatenate with the
  BirdNET 1024-d vector for a 1536-d cross-cue retrieval target.

---

## Sonobuoy Integration Plan

This paper supplies the **call-type / dialect fingerprint head** for the
cetacean branch of the K-STEMIT sonobuoy pipeline. Concretely:
whenever the Allen-2021 detector (`noaa-difar-conformer.md`) flags
a clip as containing whale song *and* the BirdNET embedding head
(`birdnet-cetaceans.md`) assigns posterior mass to the *Orcinus orca*
class, the clip is routed to a ResNet-18 encoder port of the
Bergler-2019 stage-1 autoencoder. The encoder returns a 512-d
embedding, which is inserted into the same HNSW vector service as
the BirdNET vectors but under a namespaced index
(`orca_call_dialect`). A pod-level classifier head can then be
trained per deployment from a handful of expert-labelled calls
(nearest-neighbour on the HNSW index, Ghani-style sigmoid head over
the 512-d vector, or optional stage-3 triplet fine-tune for
downstream dialect clustering). Because the encoder is a frozen
function of a mel spectrogram, it slots into the same
deterministic-replay-log contract as the BirdNET step — the sonobuoy
pipeline can emit both vectors in parallel per call and store them
alongside DIFAR bearing and timestamp, producing a `(time, bearing,
species_prob, call_type_embedding)` tuple per orca call. At analyst
level the HNSW index then supports "find all calls from this pod at
this buoy last year" queries without retraining.

---

## Follow-up References

1. **Bergler, C., Schmitt, M., Cheng, R. X., Maier, A., Nöth, E., et al.**
   *Deep Learning for Orca Call Type Identification — A Fully
   Unsupervised Approach.* Interspeech 2019.
   DOI `10.21437/Interspeech.2019-1857`. — The unsupervised
   companion to the TSD paper: undercomplete ResNet-18 autoencoder +
   spectral clustering on the Orchive. Provides the no-label version
   of the same recipe.
2. **Bergler, C., Schröter, H., Cheng, R. X., Barth, V., Weber, M.,
   Nöth, E., Hofer, H., Maier, A.** *ORCA-SPOT: An Automatic Killer
   Whale Sound Detection Toolkit Using Deep Learning.* Scientific
   Reports 9:10997 (2019). DOI `10.1038/s41598-019-47335-w`. — The
   detector upstream of this paper. Trains a binary orca/noise
   classifier on 11 509 orca signals and 34 848 noise segments over
   the full Orchive corpus. Required pre-stage.
3. **Bergler, C., Smeele, S., Tyndel, S., et al.** *ORCA-SLANG: An
   Automatic Multi-Stage Semi-Supervised Deep Learning Framework for
   Large-Scale Killer Whale Call Type Identification.* Interspeech
   2021. DOI `10.21437/Interspeech.2021-616`. — Productionises this
   line: 235 369 machine-annotated calls; fuses detection + call-type
   representation in one pipeline.
4. **Schroff, F., Kalenichenko, D., Philbin, J.** *FaceNet: A Unified
   Embedding for Face Recognition and Clustering.* CVPR 2015.
   arXiv `1503.03832`. — The triplet-loss recipe the original survey
   wanted; the correct reference for the stage-3 add-on described in
   the Portable Details section.
5. **Ford, J. K. B.** *Vocal traditions among resident killer whales
   (Orcinus orca) in coastal waters of British Columbia.* Canadian
   Journal of Zoology 69 (6), 1454–1483 (1991). — The foundational
   ethology paper defining the N01–N50 call-type nomenclature of
   northern resident orcas, which this paper's class labels come
   from.
