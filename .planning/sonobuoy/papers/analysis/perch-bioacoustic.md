# Perch — Global Birdsong Embeddings for Bioacoustic Transfer

## Citation

- **Title**: Global birdsong embeddings enable superior transfer learning for bioacoustic classification
- **Authors**: Burooj Ghani, Tom Denton, Stefan Kahl, Holger Klinck
- **Affiliation**: Naturalis Biodiversity Center; Google Research; K. Lisa Yang Center for Conservation Bioacoustics, Cornell Lab of Ornithology
- **Venue**: Scientific Reports 13, 22876 (2023)
- **DOI**: 10.1038/s41598-023-49989-z
- **URL**: https://www.nature.com/articles/s41598-023-49989-z
- **Code / Weights**: https://github.com/google-research/perch and https://tfhub.dev/google/bird-vocalization-classifier

## Status

**substituted, with correction**. The sonobuoy survey cites "Hamer, Triantafillou et al. Google Research, arXiv:2307.15008, 2023" as the Perch paper. This is incorrect on two counts:

1. arXiv:2307.15008 is in fact Nicholas Carlini's "A LLM Assisted Exploitation of AI-Guardian" (Google DeepMind, 2023) — an adversarial-ML paper wholly unrelated to bioacoustics.
2. The authoritative Perch foundation-model paper is **Ghani, Denton, Kahl, Klinck (2023)** in *Scientific Reports* (DOI 10.1038/s41598-023-49989-z), which is the paper that introduces the Perch embeddings and empirically demonstrates their zero-shot transfer to cetacean, bat, frog, and dialect bioacoustic tasks — matching the survey's description almost exactly.

A related but distinct paper, **Hamer, Triantafillou, van Merriënboer, Kahl, Klinck, Denton, Dumoulin (2023)** — "BIRB: A Generalization Benchmark for Information Retrieval in Bioacoustics" (arXiv:2312.07439) — is the paper the survey's *author list* actually points at. BIRB is a benchmark that uses Perch as one of its baselines; it is not the Perch paper itself. The Hamer author list was probably conflated with the Perch paper when the survey was written.

This analysis covers the Ghani et al. 2023 *Scientific Reports* paper as the definitive Perch paper. Where relevant we also note BIRB context for the marine transfer evaluation.

## One-paragraph Summary

Perch is a supervised (not self-supervised) bioacoustic foundation model trained by Google Research to classify bird vocalizations across the ~10 000-species Xeno-Canto taxonomy. Architecturally it is a standard **EfficientNet-B1** convolutional backbone trained on log-mel spectrograms of 5-second, 32 kHz audio windows, with a taxonomic classification head that simultaneously predicts species, genus, family, and order. What makes Perch a *foundation model* is not its architecture but its embedding: the penultimate **1280-dimensional** feature vector transfers remarkably well to *out-of-scope* bioacoustic tasks — marine mammal vocalizations (Watkins 60-species, Top-1 0.83 / AUC 0.98 at 32-shot), bat echolocation (Top-1 0.86 / AUC 0.97), frog calls (Top-1 0.74 / AUC 0.96), and fine-grained bird dialects — consistently beating general-audio transformers (AudioMAE, YAMNet, VGGish, PSLA) and even a regional bird-specialist model. The paper's central empirical finding is that **training on a closely related bioacoustic task produces dramatically better transfer embeddings than training on broad AudioSet data**, even when the target task is taxonomically distant (birds → whales). This inverts the usual "bigger/broader pretraining is better" intuition and is the main reason Perch is the default bioacoustic pretrainer for passive acoustic monitoring pipelines as of late 2023.

## Methodology

### Architecture (Perch 1.0 / "base Google Perch")

- **Backbone**: EfficientNet-B1 (Tan & Le 2019), ~7.8 M parameters.
- **Input**: 5-second audio windows resampled to **32 kHz**.
- **Spectrogram front-end**: log-mel spectrogram, specific parameters per Google Research's `chirp` codebase (160 mel bands, short windows). The input image is roughly `[224 x 160]` after spectrogram + resize.
- **Embedding**: penultimate layer output, **1280 dimensions** (EfficientNet-B1's final conv feature map, globally pooled).
- **Classification head**: multi-head taxonomic classifier that simultaneously predicts species, genus, family, and order for every training recording. This taxonomic supervision is a key ingredient — the shared lower layers are forced to learn features useful at multiple levels of abstraction, which generalizes better than species-only training.

### Training data

- **Xeno-Canto (XC)** snapshot as of July 2022 — the full corpus of user-uploaded bird recordings, ~1 M+ recordings, ~10 000 species, weakly labeled (one label per file).
- Activity detection used to select training windows from each file (since XC files often contain long silences).
- No AudioSet, no ImageNet, no external modalities.

### Training recipe

- Loss: cross-entropy (species) + auxiliary cross-entropy heads for genus, family, order (multi-task).
- Augmentation:
  - **MixUp** (Zhang 2017).
  - Random gain adjustment.
  - Random time-shifting up to ±1 s.
- Supervised training only (no SSL phase).
- Trained by Google Research; exact learning-rate schedule / step count not fully specified in the Sci Rep paper but published in the `chirp` codebase — roughly AdamW, cosine decay, ~1 M steps at batch size ~256, base LR ~1e-3.

### Embedding dimensionality ablation

The paper runs a dedicated ablation varying Perch's embedding dimension by re-training with different penultimate-layer widths: **160, 320, 640, 960, 1280, 2560**. Findings:

- 320 (matching BirdNET 2.2): significantly degraded on all downstream tasks.
- **1280: default, best cost/quality trade-off**.
- 2560: modest further improvement on some downstream tasks, not uniform.
- Conclusion: embedding capacity genuinely matters; 1280 is near the knee of the curve.

### Downstream evaluation protocol

The Sci Rep paper is **a pretraining-as-a-service evaluation**, not a new pretraining method. The protocol:

1. Freeze the pre-trained model entirely.
2. Extract embeddings by running audio through the model. For examples shorter than the model's window, zero-pad. For longer, window and average per-frame embeddings.
3. Train a **linear probe** (single linear layer, sigmoid, BCE loss) on k training examples per class, with k ∈ {4, 8, 16, 32, 64, 128, 256}.
4. Evaluate macro-ROC-AUC and Top-1 accuracy; average over 5 seeds.
5. Compare: Perch vs BirdNET 2.2/2.3 vs Sierra Birds (regional) vs AudioMAE, PSLA, YAMNet, VGGish (general audio) vs MFCC (no-pretraining baseline).

## Key Results

### Headline: 32-shot linear probe ROC-AUC

| Model           | Embed dim | Godwit | Yellowhammer | Bats   | Watkins (marine) | RFCX Frogs | RFCX Birds |
|-----------------|-----------|--------|--------------|--------|------------------|------------|------------|
| **Perch**       | 1280      | **0.99** | **0.91**     | **0.97** | **0.98**         | **0.96**   | **0.97**   |
| BirdNET 2.3     | 1024      | 0.99   | 0.91         | 0.96   | 0.98             | 0.95       | 0.96       |
| Sierra Birds    | 1280      | 0.93   | 0.57         | 0.93   | 0.96             | 0.92       | 0.94       |
| AudioMAE        | 1024      | 0.96   | 0.66         | 0.85   | 0.96             | 0.89       | 0.85       |
| PSLA            | —         | 0.80   | 0.51         | 0.57   | 0.76             | 0.59       | 0.60       |
| YAMNet          | 1024      | 0.91   | 0.55         | 0.83   | 0.96             | 0.86       | 0.84       |
| VGGish          | 128       | 0.86   | 0.51         | 0.80   | 0.56             | 0.85       | 0.81       |
| BEANS MFCC      | —         | 0.53   | 0.51         | 0.53   | 0.56             | 0.52       | 0.58       |

(Watkins = Watkins Marine Mammal Sounds Database, 60 cetacean + pinniped species.)

### Top-1 accuracy (32-shot)

| Model       | GC   | YD   | BT   | WMMSD | Frogs | Birds |
|-------------|------|------|------|-------|-------|-------|
| **Perch**   | 0.92 | **0.87** | 0.86 | **0.83** | 0.74  | **0.83** |
| BirdNET 2.3 | 0.91 | 0.84 | 0.85 | 0.81  | 0.73  | 0.78  |
| AudioMAE    | 0.85 | 0.61 | 0.63 | 0.74  | 0.56  | 0.43  |

### Underwater / marine specifically

- **Watkins Marine Mammal Sounds Database**: 60 species of cetaceans (odontocetes, mysticetes) + pinnipeds. Perch 32-shot: **Top-1 0.83, AUC 0.98** — nearly saturating. This is the headline underwater-transfer result. Critically, it is achieved by a model that has seen **zero whales in training**; the transfer is purely on the basis that bird vocalizations and cetacean vocalizations share common spectral-temporal structure (frequency modulation, harmonic content, pulsed patterns).
- **RFCX Frogs** (Rainforest Connection tropical frog subset): Perch 0.74 / 0.96. Frogs are somewhat underwater-adjacent (amphibians, shallow ponds); Perch handles them easily.
- Bats: 0.86 / 0.97 despite requiring ultrasonic audio (pitch-shifted to audible before inference — the paper explicitly describes this preprocessing).

### Few-shot scaling

At k = 4 training examples per class, Perch still produces ROC-AUC > 0.5 by "a considerable margin" on every benchmark — i.e., it is meaningfully useful even in 4-shot regimes. The general-audio transformers (AudioMAE, YAMNet) degrade much faster as k shrinks.

### AudioMAE deep-dive

The paper runs an extensive ablation on AudioMAE (the strongest general-audio competitor), trying:

1. Native-sample-rate audio fed *as if* it were 16 kHz (preserves ultrasonic energy).
2. 2-layer probe head (batch-norm → 2048 hidden → ReLU → output) instead of linear.
3. Combinations of the above.

The best AudioMAE configuration (2-layer probe, no resampling) still uniformly underperforms Perch's 1-layer linear probe on every bioacoustic dataset.

## Strengths

1. **Supervised pretraining with taxonomic multi-task heads** produces surprisingly general bioacoustic features — bird-trained embeddings dominate even for whales, bats, and frogs, violating the intuition that general-purpose SSL should win.
2. **32 kHz input + 1280-dim embedding** is the right operating point for passive acoustic monitoring. The 32 kHz sample rate covers the full bird band and most cetacean bands.
3. **Small backbone** (EfficientNet-B1, ~8 M params) → fast CPU inference (~200 ms per 5-s window on a 4.3 GHz AMD CPU per Table 1 of the paper). Trivially deployable on sonobuoy edge hardware.
4. **Open weights** (TFHub) and open evaluation datasets. Google Research's `chirp` codebase is well-maintained.
5. **Few-shot efficiency**: useful at k = 4 examples per class. Perfect for rare species and novel call types where only a handful of curated examples exist.

## Limitations

1. **Supervised, not self-supervised** — cannot easily ingest unlabeled audio streams. Extending Perch requires curated Xeno-Canto-style labels; a true foundation model would learn from raw passive recordings.
2. **Bird-centric training distribution** — transfer works but is still inferior to specialist marine models (e.g., the NOAA DCLDE-trained models) when those exist for a specific taxon. Perch 2.0 (arXiv:2508.04665, 2025) partially addresses this by adding multi-taxa supervision.
3. **5-second fixed window** is too long for transient click-type signals (sperm whale clicks are ~5 ms) and too short for extended songs (humpback arias can exceed a minute). Windowing/averaging loses temporal structure.
4. **CNN backbone limits attention-based downstream uses** — you cannot easily extract per-patch attention maps the way you can from a ViT. For explainability / dense prediction, a ViT-based pretrainer (AudioMAE, BEATs) is more flexible.
5. **Taxonomic head tied to training species set**; fine-tuning on a new species set requires retraining or extending the classifier head while keeping the frozen backbone.

## Portable Details (reimplementation spec)

| Item                      | Value                                                       |
|---------------------------|-------------------------------------------------------------|
| Input sample rate         | **32000 Hz** (resample to match)                            |
| Input window              | **5 seconds** fixed; zero-pad shorter, frame-and-average longer |
| Spectrogram               | Log-mel, 160 mel bands (per chirp codebase)                 |
| Backbone                  | EfficientNet-B1 (Tan & Le 2019)                             |
| Parameters                | ~7.8 M                                                      |
| Embedding dim             | **1280** (globally-pooled penultimate feature map)          |
| Classification head       | Multi-task taxonomic: species + genus + family + order logits |
| Training data             | Xeno-Canto full corpus (July 2022), ~10 000 species         |
| Training augmentation     | MixUp (α ~0.5), random gain, random time-shift ±1 s         |
| Training objective        | Multi-class / multi-label sigmoid cross-entropy per head    |
| Activity detection        | Used to crop training windows from weakly-labeled full files|
| Downstream probe (default)| Single Linear(1280 → num_classes) + sigmoid + BCE loss      |
| Probe LR                  | ~1e-3 Adam, train to convergence                            |
| Probe k (default eval)    | 32 examples per class, 5 seeds                              |
| Recommended evaluation    | Macro ROC-AUC and Top-1 accuracy averaged over seeds        |
| Native inference cost     | ~200 ms / 5-s window on 4.3 GHz AMD CPU                     |
| Release format            | TensorFlow SavedModel on TFHub; also Keras H5               |

### Adapting to non-5-s / non-32 kHz input

- **Shorter clip**: pad with silence to 5 s, log in metadata that the effective signal duration was shorter so downstream models can compensate.
- **Longer clip**: frame into overlapping 5-s windows (hop 2.5 s typical), embed each, and either (a) mean-pool for a single clip-level 1280-D vector or (b) keep the per-window sequence for temporal models.
- **Higher-rate source** (e.g., 96 kHz sonobuoy): resample to 32 kHz and accept aliasing-aware low-pass filtering; this loses ultrasonic detail but is the intended operating mode.
- **Ultrasonic target** (bats, clicks): pitch-shift via sample-rate manipulation (as in the BT dataset of the paper) to move the signal into the 1-16 kHz band before feeding, then embed. This is a workaround, not a true fix.

## Sonobuoy Integration Plan

### Role in the temporal branch

Perch is the **primary bioacoustic pretrainer** for the sonobuoy temporal branch, for three reasons:

1. Empirically dominant on every marine, bat, frog, and bird transfer benchmark vs general-audio SSL baselines (AudioMAE, YAMNet, VGGish, PSLA).
2. Fits the edge-compute profile: EfficientNet-B1 runs at <200 ms per 5-s window on modest CPU; a sonobuoy analysis node can handle real-time.
3. The 32 kHz input + 5-s window matches the typical bandwidth and framing of civilian passive acoustic monitoring; no bespoke front-end engineering needed.

### Recommended usage pattern

1. **Extract 1280-D embeddings** via the frozen TFHub checkpoint. Port to ONNX for Rust inference via `tract-onnx` or `candle-onnx`.
2. **Framing**:
   - Resample incoming sonobuoy audio to 32 kHz.
   - Frame at 5 s with 50% overlap (hop 2.5 s).
   - Embed each frame; retain both the per-frame sequence (for temporal models) and the mean-pooled per-minute vector (for retrieval).
3. **Classification head**: for each target taxon (cetacean species of interest, anthropogenic noise categories, geophony), train a linear probe on 16-64 curated examples per class. Expected ROC-AUC > 0.95 on Watkins-like conditions.
4. **Composition with BEATs / AudioMAE**: for non-bioacoustic audio (vessel noise, seismic events, anthropogenic signals), route to BEATs; keep Perch as the bioacoustic specialist. A simple gating classifier ("biological vs non-biological") can route.
5. **Fine-tuning**: if a large labeled marine corpus is available, fine-tune Perch's top few conv blocks with a small LR (1e-5) on the target domain for 2-5 k steps. The Perch 2.0 paper (arXiv:2508.04665) documents self-distillation variants for exactly this regime.

### HNSW compatibility

- Perch embedding is **1280-D** (f32) — **3.3× larger** than the kernel's informational default of 384.
- Memory cost: 1 M segments × 1280 × 4 bytes = 5.1 GB raw, plus HNSW graph overhead (~30-50%) → ~7 GB for a 1 M-segment corpus. Acceptable for a server-class sonobuoy analysis node; potentially too large for embedded.
- Options:
  1. **Keep native 1280**: set the temporal namespace's `HnswServiceConfig::default_dimensions` to 1280. Best retrieval quality. Recommended default.
  2. **Project to 384**: add a Linear(1280 → 384) + LayerNorm head, trained either (a) via InfoNCE on same-species positive pairs or (b) via PCA fit on a representative subset. The paper's own 320-D Perch variant *significantly* degrades downstream quality, so 384 is at the ragged edge of what is usable.
  3. **Scalar/product quantization** (AgentDB / HNSW supports int8 / PQ): 4× memory reduction with minimal recall loss. Preferred production path for the embedded profile.
- **Normalization**: L2-normalize embeddings before insertion so cosine and Euclidean orderings agree.

### Availability

- **Weights**: https://tfhub.dev/google/bird-vocalization-classifier (Apache 2.0).
- **Code**: google-research/chirp (Apache 2.0, actively maintained by Tom Denton and team).
- **ONNX export**: straightforward via TF → ONNX converter; a community port `perch-onnx` exists.
- **Successor**: **Perch 2.0** (arXiv:2508.04665, van Merriënboer, Dumoulin, Hamer et al., Aug 2025) expands training to 14 597 species across birds, mammals, amphibians, insects; uses self-distillation + prototype-learning classifier + source-prediction criterion; reportedly beats specialist marine models on marine transfer **despite near-zero marine training data**. Weights not released as of this writing; the Sci Rep Perch 1.0 remains the shipping checkpoint.

## Follow-up References

1. **van Merriënboer, Dumoulin, Hamer, Harrell, Burns, Denton (2025)** — "Perch 2.0: The Bittern Lesson for Bioacoustics", arXiv:2508.04665. The multi-taxa successor; required reading for anyone shipping a marine-focused pipeline.
2. **Hamer, Triantafillou, van Merriënboer, Kahl, Klinck, Denton, Dumoulin (2023)** — "BIRB: A Generalization Benchmark for Information Retrieval in Bioacoustics", arXiv:2312.07439. The bioacoustic retrieval benchmark that Perch dominates; use it for evaluation of the sonobuoy retrieval layer.
3. **Kahl, Wood, Eibl, Klinck (2021)** — "BirdNET: A deep learning solution for avian diversity monitoring", *Ecological Informatics* 61. The main competitor to Perch; overlapping training corpus but different architecture and no taxonomic head.
4. **Tan & Le (2019)** — "EfficientNet: Rethinking Model Scaling for Convolutional Neural Networks", ICML 2019. The backbone.
5. **Hagiwara (2023)** — "BEANS: The Benchmark of Animal Sounds", ICASSP 2023. The bioacoustic benchmark suite providing several of the non-bird evaluations used in the Ghani et al. paper.
6. **Hamer et al. (2023), "Perch 2.0 transfers 'whale' to underwater tasks"** (arXiv:2512.03219, follow-up to Perch 2.0). If shipping a cetacean-focused sonobuoy pipeline, this is the most recent empirical evidence for underwater transferability.
