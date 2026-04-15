# BirdNET Embeddings as a Bioacoustic Transfer Layer for Cetaceans

## Citation Header

- **Verified Title**: *Global birdsong embeddings enable superior transfer learning for bioacoustic classification*
- **Authors**: Burooj Ghani, Tom Denton, Stefan Kahl, Holger Klinck
- **Venue**: Nature Scientific Reports, Vol. 13, Article 22876
- **Year**: 2023 (published online 21 December 2023; arXiv v1 12 July 2023, v2 17 November 2023)
- **DOI**: `10.1038/s41598-023-49989-z`
- **arXiv**: `2307.06292` (cs.SD / eess.AS)
- **Primary URL**: https://www.nature.com/articles/s41598-023-49989-z
- **PDF mirrored to**: `.planning/sonobuoy/papers/pdfs/birdnet-cetaceans.pdf` (arXiv PDF, 2.2 MB)

## Status

**Substituted** — from *"Ghani, Kahl et al., Methods in Ecology and Evolution, 2024"* (as quoted in the sonobuoy survey) to the real paper *Ghani, Denton, Kahl & Klinck, Scientific Reports, 2023* (DOI `10.1038/s41598-023-49989-z`). The survey misremembered the venue and co-authors and predated the paper by a year.

Rationale for the substitution:
1. Cross-ref search for "BirdNET cetaceans Ghani Kahl" returned no Methods in Ecology and Evolution 2024 entry.
2. The Scientific Reports paper is the canonical BirdNET-as-embedding paper by Ghani/Kahl/Klinck (the BirdNET team) and *explicitly* evaluates on the Watkins Marine Mammal Sound Database — exactly the "whale classifier" claim from the survey.
3. The survey's "1024-d BirdNET embedding" hook is literally the BirdNET 2.3 embedding size reported in this paper. That size is diagnostic: it shipped in BirdNET 2.3 which this paper introduces/evaluates.

Related work in the same lineage that the survey may have conflated:
- Williams et al., *Leveraging tropical reef, bird and unrelated sounds for superior transfer learning in marine bioacoustics* (arXiv 2404.16436, Phil. Trans. R. Soc. B 2025) — "SurfPerch", uses same embedding-transfer recipe for reef soundscapes.
- van Merriënboer et al., *Perch 2.0: The Bittern Lesson for Bioacoustics* (arXiv 2508.04665, 2025) — successor model; evaluated on DCLDE 2026 cetacean species, NOAA PIPAN whales.
- Allen-Ankins et al., *The use of BirdNET embeddings as a fast solution to find novel sound classes* (Frontiers Ecol. Evol. 10.3389/fevo.2024.1409407, 2024/2025) — Euclidean-distance nearest-neighbour recipe on 1024-d BirdNET vectors.

---

## One-Paragraph Summary

Ghani, Denton, Kahl and Klinck demonstrate that feature embeddings from
large-scale *bird-only* acoustic classifiers (BirdNET and Google Perch)
generalise far better than general-purpose audio embeddings (YAMNet, VGGish,
AudioMAE) when the downstream task is *any* animal sound classification task
— including problems with zero bird content such as bat echolocation calls,
cetacean vocalisations, and frog choruses. The recipe is deliberately plain:
run the frozen bird network over the downstream audio, pool the last
penultimate-layer activations into a fixed-length vector (320-d, 1024-d or
1280-d depending on the model), and train a single feed-forward classifier
on top. On the Watkins Marine Mammal Sound Database (32 cetacean and
pinniped species) a Perch-embedding classifier reaches ROC-AUC **0.98** at
32 shots per class and BirdNET 2.3 ties at **0.98**, while AudioMAE reaches
only 0.96 and VGGish collapses to 0.56 — a direct refutation of the assumption
that one needs a bespoke whale encoder. The paper's operational takeaway is
that a single 1024-d BirdNET embedding vector is a usable *universal
bioacoustic descriptor* for a wide taxonomic range, which is precisely what
makes it the default choice for the K-STEMIT HNSW bioacoustic index.

---

## Methodology

### Architecture

Four-plus-two encoders are compared, all used as frozen feature extractors:

| Model | Backbone | Training corpus | Input window | Embedding dim |
|-------|----------|------------------|--------------|---------------|
| BirdNET 2.2 | EfficientNet-B1 | Xeno-canto + Macaulay + labelled soundscape | 3.0 s | **320** |
| BirdNET 2.3 | EfficientNet-B1 | Same as 2.2, updated | 3.0 s | **1024** |
| Google Perch | EfficientNet-B1 | Xeno-canto (July 2022 full corpus) | 5.0 s | **1280** |
| YAMNet | MobileNet-v1 | AudioSet | 0.96 s | **1024** |
| VGGish | Modified VGG | YouTube-8M | 0.96 s | **128** |
| AudioMAE | ViT-Large with MAE | AudioSet | 10.0 s | **1024** |

All backbones operate on log-mel-scaled spectrograms derived from each
model's canonical sample rate. For this paper none of the backbones are
fine-tuned; embeddings are pre-computed once per clip and cached.

### Classifier head

The downstream head is a single **fully-connected feed-forward layer with
sigmoid activation per class**, i.e. independent binary classifiers per
label (multi-label setup), trained with **binary cross-entropy**. This is
equivalent in practice to linear logistic regression on the frozen
embeddings. Models are trained to convergence on the fixed embeddings. Every
configuration is repeated with **five seeded shuffles** of the training set
and the mean ± stdev of ROC-AUC is reported.

### Loss

Per-class binary cross-entropy:

```
L = − Σ_c [ y_c · log σ(w_c · z + b_c) + (1 − y_c) · log(1 − σ(w_c · z + b_c)) ]
```

where `z` is the frozen embedding vector and `(w_c, b_c)` are the trainable
per-class weights of the sigmoid head.

### Datasets

Evaluation is deliberately cross-taxon:

- **Yellowhammer Dialects (YD)** — 2 regional dialect classes, 48 kHz, 3.5 s clips.
- **Godwit Calls (GC)** — 5 call types, 44.1 kHz, 3.0 s clips.
- **RFCX Bird Species** — 13 Amazonian species, 48 kHz, 5.0 s clips.
- **Watkins Marine Mammal Sounds Database (WMMSD)** — 32 species spanning Odontoceti, Mysticeti, Phocidae and Otariidae; 22.05 kHz; 0.1–10.0 s variable clips; mean ~60 samples per class.
- **RFCX Frogs** — 12 Amazonian species.
- **North-American Bats (BT)** — 4 species (LABO 1 124, MYLU 1 119, MYSE 360, PESU 948 recordings), pitch-shifted to 44.1 kHz, 1.0–13.0 s variable.

The few-shot protocol sweeps the number of training shots per class through
{4, 8, 16, 32, 64, 128, 256}. The canonical headline number uses 32 shots.

### Training setup

Fixed-embedding + linear head means:
- No data augmentation on embeddings (augmentation would have required
  end-to-end training, which defeats the "frozen encoder" premise).
- 5 random seeds per configuration.
- Train-to-convergence with early stopping on validation ROC-AUC.
- Optimiser/LR not explicitly pinned in the paper; scikit-style logistic
  regression defaults are effectively equivalent.

### Hardware

Not pinned. Because training is linear logistic regression on ≤1280-d
vectors with ≤ 256 × 32 = 8 192 samples, a CPU implementation runs in
seconds. The expensive step is one-time embedding extraction, which
requires a GPU for BirdNET/Perch inference on the underlying audio.

---

## Key Results (actual numbers)

All numbers are **ROC-AUC at 32 shots per class**, mean over five seeds.
Best model per row is **bold**.

| Dataset | Perch (1280-d) | BirdNET 2.3 (1024-d) | BirdNET 2.2 (320-d) | AudioMAE | YAMNet | VGGish |
|---|---|---|---|---|---|---|
| Godwit Calls (5 cls) | **0.99** | 0.99 | ~0.97 | 0.96 | 0.91 | 0.86 |
| Yellowhammer Dialects (2 cls) | **0.91** | 0.91 | ~0.87 | 0.66 | 0.55 | 0.51 |
| Bat Species (4 cls) | **0.97** | 0.96 | ~0.93 | 0.85 | 0.83 | 0.80 |
| **Watkins Marine Mammals (32 cls)** | **0.98** | 0.98 | ~0.96 | 0.96 | 0.96 | 0.56 |
| RFCX Frogs (12 cls) | **0.96** | 0.95 | ~0.92 | 0.89 | 0.86 | 0.85 |
| RFCX Birds (13 cls) | **0.97** | 0.96 | ~0.94 | 0.85 | 0.84 | 0.81 |

Additional findings:

- **Few-shot scaling**: Bird-pretrained models retain ROC-AUC > 0.5 with
  **as few as 4 training shots per class** on every evaluated task — a
  near-order-of-magnitude data efficiency advantage over general audio models.
- **Bird-to-marine transfer**: The Watkins result — 0.98 ROC-AUC for a
  32-way cetacean + pinniped species classifier from a frozen bird encoder
  — is the paper's most striking number because there is literally **zero
  whale audio** in the BirdNET training set.
- **Embedding dimensionality alone is not the story**: YAMNet is 1024-d
  (same as BirdNET 2.3) but reaches only 0.96 on Watkins — the training
  *distribution* (species-labelled bird calls) matters more than the
  latent size.
- BirdNET 2.3 vs BirdNET 2.2: going from 320-d to 1024-d gives roughly a
  +0.02 ROC-AUC bump across tasks; marginal but consistent.

---

## Strengths

- **Zero-domain adaptation**: the recipe (frozen encoder + linear head)
  has essentially no hyperparameters, which makes it deployable by field
  biologists rather than by ML engineers.
- **Broad taxonomic sweep**: bird dialect, bird species, bat, cetacean,
  frog — the same pipeline beats domain-generic embeddings on *all* of them.
- **Reproducibility**: every embedding model is publicly available, every
  dataset is either public (WMMSD, RFCX) or redistributable; the five-seed
  protocol makes the numbers trustworthy.
- **Few-shot robustness**: the ROC-AUC > 0.5 at 4-shot result is directly
  usable for active-learning pipelines — you can bootstrap a new species
  classifier from a handful of examples.
- **Implementation simplicity**: production deployment reduces to
  "precompute embeddings, store them, do logistic regression" — a
  rust-native reimplementation needs only the sigmoid head, not the backbone.

## Limitations

- **No end-to-end fine-tuning comparison**: the paper deliberately freezes
  the encoder to isolate the transfer-learning question, but in practice
  fine-tuning usually closes the remaining AUC gap. Whether doing so
  regresses other tasks (catastrophic forgetting) is not studied.
- **Classifier is linear**: an MLP head with a hidden layer may extract
  non-linear structure in the embedding space that sigmoid-head logistic
  regression cannot — unexplored.
- **Clip-level, no localisation**: every evaluation uses pre-segmented
  clips. For passive acoustic monitoring the model gives no temporal
  localisation of the call inside the clip — this is a separate problem
  that the downstream MIL paper in the sonobuoy stack has to solve.
- **Single-sensor**: all evaluations are mono audio. The paper does not
  address how embeddings should be aggregated across a DIFAR sonobuoy's
  three-channel (pressure + X + Y) stream.
- **Watkins bias**: WMMSD is a curated clean-clip database. Real PAM data
  is noisier, with overlapping species and severe class imbalance; the
  paper does not evaluate on field data directly.

---

## Portable Details (Rust-implementable)

### Algorithmic recipe

```
ALGORITHM EmbedAndClassify(audio_clip, head)
  1. spec = log_mel_spectrogram(audio_clip,
                                 sr = 48_000,
                                 win_ms = 25,
                                 hop_ms = 10,
                                 n_mels = 96)
  2. z   = BirdNET_EfficientNetB1_penultimate(spec)      # 1024-d
  3. p_c = sigmoid(head.W · z + head.b)                   # per class
  4. return p_c
```

### Exact hyperparameters worth freezing in the K-STEMIT pipeline

- **Input**: 48 kHz mono, 3.0 s window, log-mel spectrogram with
  `n_mels = 96`, `win_ms = 25`, `hop_ms = 10`.
- **Embedding dimension**: `1024` (BirdNET 2.3). Use float32 per slot,
  i.e. 4 096 bytes per clip. Quantise to int8 only at HNSW-index time.
- **Classifier head**: single dense layer with bias, sigmoid activation,
  BCE loss, Adam at `lr = 1e-3`, early stop on val ROC-AUC, 5-seed
  ensemble. For 32 shots × 32 classes that's trivially trainable on CPU.
- **Few-shot floor**: `n_shots_per_class ≥ 4` is the minimum for
  ROC-AUC > 0.5; treat `n_shots < 4` as the active-learning corner case.
- **Similarity metric for HNSW**: the paper uses raw Euclidean distance
  in Allen-Ankins' follow-up. BirdNET embeddings are not L2-normalised
  out of the box; normalise them (`z / ‖z‖₂`) before inserting into an
  HNSW index so that cosine ≡ Euclidean and both distance metrics are
  interchangeable.

### Equations

Forward pass of the classifier head on a clip's embedding `z ∈ ℝ^1024`:

```
logits = W · z + b            W ∈ ℝ^{C×1024}, b ∈ ℝ^C
p       = σ(logits)
```

Training loss on a minibatch of size `N`:

```
L = − (1 / N) Σ_i Σ_c [ y_{ic} log p_{ic} + (1 − y_{ic}) log(1 − p_{ic}) ]
```

Few-shot sampling rule used in the paper for each `n_shots ∈ {4, 8, 16, 32,
64, 128, 256}`: stratified random sample of `n_shots` clips per class with
the remainder as the held-out test set, repeated across five seeds.

---

## Sonobuoy Integration Plan

This paper defines the default bioacoustic vector used throughout the
K-STEMIT-extended sonobuoy stack. Concretely: the audio pipeline in
`clawft-kernel` accepts a PCM stream from a DIFAR sonobuoy (pressure
channel) at any sample rate, resamples to 48 kHz, segments into
**3.0-s overlapping windows with 1.5 s hop**, runs BirdNET 2.3
EfficientNet-B1 to the penultimate layer, and emits a **1024-d float32
vector per window**. These vectors are the unit of storage in the HNSW
vector service (`clawft-kernel/src/hnsw_service.rs`); they are inserted
cosine-normalised, indexed once, and used for three downstream tasks —
(a) species-label prediction via a Ghani-style sigmoid head trained
per-deployment on a handful of labelled shots, (b) nearest-neighbour
retrieval for "find more calls like this one" during human-in-the-loop
annotation, and (c) unsupervised change-point detection against a
deployment-specific baseline cluster. The DIFAR X/Y directional channels
bypass this vector path entirely and go to the bearing estimator
(`noaa-difar-conformer` analysis). Because the encoder is frozen, the
embedding step is a deterministic pure function of PCM bytes — exactly
the property K-STEMIT's replay-log design assumes — and BirdNET's
public TFLite model (`~46 MB`) can be loaded once and reused across
every sonobuoy channel without retraining.

---

## Follow-up References

1. **Williams, B., van Merriënboer, B., Dumoulin, V., et al.** *Leveraging
   tropical reef, bird and unrelated sounds for superior transfer learning
   in marine bioacoustics.* Phil. Trans. R. Soc. B 380 (2025).
   arXiv `2404.16436`. — Shows the same recipe extended to reef
   soundscapes with the "SurfPerch" checkpoint; gives a direct comparison
   point for marine-specific pretraining.
2. **van Merriënboer, B., Dumoulin, V., Hamer, J., Harrell, L., Burns, A.,
   Denton, T.** *Perch 2.0: The Bittern Lesson for Bioacoustics.*
   arXiv `2508.04665` (2025). — Successor model; adds self-distillation
   with prototype-learning; outperforms BirdNET 2.3 on DCLDE 2026 and
   NOAA PIPAN whale benchmarks.
3. **Allen-Ankins, S., Hoefer, S., Bartholomew, J., Brodie, S.,
   Schwarzkopf, L.** *The use of BirdNET embeddings as a fast solution
   to find novel sound classes in audio recordings.* Frontiers in
   Ecology and Evolution (2024/2025). DOI `10.3389/fevo.2024.1409407`.
   — Operational recipe: BirdNET-2.3 1024-d embeddings + Euclidean
   distance + optional linear SVM, demonstrated on geckos and mammals.
4. **Kahl, S., Wood, C. M., Eibl, M., Klinck, H.** *BirdNET: A deep
   learning solution for avian diversity monitoring.* Ecological
   Informatics 61 (2021). DOI `10.1016/j.ecoinf.2021.101236`.
   — The original BirdNET paper; defines the EfficientNet-B1 backbone,
   data pipeline and evaluation protocol.
5. **Sayigh, L., et al.** *The Watkins Marine Mammal Sound Database.*
   Woods Hole Oceanographic Institution (ongoing, metadata: Mar. Mamm.
   Sci. 2016). https://cis.whoi.edu/science/B/whalesounds/ —
   The 32-species cetacean+pinniped dataset used for the key marine
   transfer number in this paper.
