# AudioMAE — Masked Autoencoders that Listen

## Citation

- **Title**: Masked Autoencoders that Listen
- **Authors**: Po-Yao Huang, Hu Xu, Juncheng Li, Alexei Baevski, Michael Auli, Wojciech Galuba, Florian Metze, Christoph Feichtenhofer
- **Affiliation**: Meta AI; Carnegie Mellon University
- **Venue**: NeurIPS 2022
- **arXiv**: 2207.06405
- **URL**: https://arxiv.org/abs/2207.06405
- **Code / Weights**: https://github.com/facebookresearch/AudioMAE

## Status

**verified**. The paper exists as cited in the sonobuoy survey. Author list matches (Huang, Xu et al.) and venue (NeurIPS 2022) is correct.

## One-paragraph Summary

AudioMAE is a direct, nearly drop-in extension of He et al.'s image Masked Autoencoder (MAE) to the audio domain, operating on log-mel spectrograms as single-channel "images". The key idea is an asymmetric encoder/decoder ViT where the encoder processes only the non-masked ~20% of spectrogram patches at very high masking ratio (0.8), and a shallower decoder reconstructs the full patch-normalized spectrogram via mean-squared error. Two audio-specific refinements lift it above prior self-supervised audio models: (1) structured masking variants (time, frequency, time+frequency) used selectively at fine-tuning, and (2) local/shifted-window attention in the decoder to exploit the fact that spectrogram patches are far more locally correlated than natural-image patches. Without any out-of-domain supervision, AudioMAE sets then-SOTA results on six audio/speech benchmarks including AudioSet-2M (47.3 mAP base, 47.4 large) and ESC-50 (94.1% accuracy), and is the first audio-only SSL model to beat models initialized from ImageNet-supervised checkpoints.

## Methodology

### Input pipeline

- Waveform mono-mixed and resampled to 16 kHz.
- Feature: 128-band log-mel spectrogram, Kaldi-compatible, 25 ms Hann(ing) window, 10 ms hop.
- A 10-second AudioSet clip yields a `1 x 1024 x 128` spectrogram (time x freq).
- Per-task input shapes:
  - AudioSet (10 s): `1 x 1024 x 128`
  - ESC-50 (5 s): `1 x 512 x 128`
  - Speech Commands (1 s): `1 x 128 x 128`
  - VoxCeleb SID (10 s): `1 x 1024 x 128`

### Patch embedding

- Conv2d kernel `16 x 16`, stride `16 x 16` (non-overlapping by default).
- For AudioSet this produces `64 x 8 = 512` tokens per clip.
- Authors ablate overlapping (stride < patch); non-overlapping was preferred for compute, mild overlap (stride 10) gave small gains.

### Positional encoding

- Fixed 2D sinusoidal positional embeddings added to patch embeddings.
- In the decoder, the same sinusoidal embeddings are re-applied after re-ordering and mask-token insertion.
- No learned positional bias at pretraining; fine-tuning inherits the same sinusoidal scheme.

### Encoder

- Default backbone: ViT-Base (12 layers, hidden dim 768, 12 heads, MLP 3072, ~86 M parameters).
- Alternatives: ViT-Small and ViT-Large (300-304 M parameters, best AudioSet-2M: 47.4 mAP).
- Encoder processes **only** the unmasked tokens (~20% of the sequence) — this is the computational trick inherited from image MAE that makes aggressive masking affordable.

### Decoder (audio-specific)

- 16 Transformer blocks at decoder width 512 (sweet spot; authors ablate 256 / 512 / 768 width and depth 2 / 8 / 16).
- **Local window attention** is the key deviation from image MAE. Two variants:
  1. **Shifted-window local attention** (Swin-style): windowed self-attention with 50% shift between consecutive decoder layers; cyclic padding at edges.
  2. **Hybrid global+local**: local attention in all but the top few layers; last layers use full global self-attention so the reconstruction head still sees a global context.
- Rationale: spectrogram features are position-sensitive in both time and frequency (harmonics stack vertically, formants extend horizontally). Global self-attention is sub-optimal because the semantic content of a patch depends heavily on *which* frequency band it occupies.
- Final linear head projects each decoder token back to `16 x 16 = 256` pixel values for the masked patches.

### Masking strategy

- **Pretraining**: unstructured (random) masking at **80%** ratio.
- **Fine-tuning**: structured masking at lower ratio — "time+frequency" masking (a SpecAugment-like variant) — performs better; time-only and frequency-only are ablated and found weaker.
- Ablation shows pretraining masking ratios 0.6 / 0.7 / 0.8 / 0.9 are all reasonable, 0.8 is best for AudioSet-2M.

### Pretraining objective

- Mean squared error between reconstructed patch and target patch.
- **Patch-normalized targets**: each target patch is normalized by its own mean and variance before comparison (same trick as image MAE).
- Loss computed **only on masked patches**.
- Formally, with patches `p_i`, target `t_i = (p_i - mu_i) / sqrt(var_i + eps)`, predicted `t_hat_i`:

  `L = (1 / |M|) * sum_{i in M} || t_hat_i - t_i ||_2^2`

### Pretraining recipe

- Dataset: AudioSet-2M (~2 M 10-second YouTube clips, ~1.96 M usable after link rot).
- Optimizer: AdamW, `beta1 = 0.9`, `beta2 = 0.95`, weight decay `1e-4`.
- Base learning rate: `2e-4` (with linear warmup then cosine decay).
- Effective batch size 512, 32 epochs pretraining.
- Hardware: 64x V100 GPUs, ~36 hours wall-clock.
- Per-clip augmentation: random start time, cyclic time shift, SpecAugment-style frequency/time masks during fine-tuning.

### Fine-tuning recipe

- Discard decoder entirely; keep encoder + task head (mean-pooled CLS or token-mean representation followed by linear classifier).
- Apply structured masking on the encoder input during fine-tuning (unusual — authors found it regularizes better than SpecAugment alone).
- Base LR for fine-tuning is task-dependent:
  - AS-2M: `2e-4`
  - AS-20K: `1e-3`
  - ESC-50 / SPC / SID: `1e-3`
- Weight decay `1e-4`, AdamW betas unchanged.
- Mixup augmentation (alpha = 0.5) and label smoothing (0.1) for AudioSet tasks.

## Key Results

### AudioSet-2M (mAP, higher is better)

| Model                              | External Data | mAP  |
|------------------------------------|---------------|------|
| AST (Gong 2021)                    | ImageNet      | 45.9 |
| PaSST                              | ImageNet      | 47.1 |
| HTS-AT                             | ImageNet      | 47.1 |
| **Audio-MAE (ViT-B)**              | None          | **47.3** |
| **Audio-MAE (ViT-L)**              | None          | **47.4** |

First audio-only SSL model to beat ImageNet-initialized transformers.

### AudioSet-20K (mAP)

- AudioMAE ViT-B: **37.1**, ViT-L: **37.6** (SSL SOTA at publication).

### ESC-50 (accuracy)

- AudioMAE (SSL only): **94.1%**
- AudioMAE with in-domain supervised pretraining: 97.4%

### Speech Commands V2 / V1 (accuracy)

- SPC-2: **98.3%**, SPC-1: **96.9%**.

### VoxCeleb SID (speaker ID accuracy)

- Reported; ViT-L achieves SOTA among SSL audio models at publication.

### Bioacoustic / underwater transfer

The AudioMAE paper itself does **not** report on marine or bioacoustic benchmarks directly. However, the Perch paper (Ghani et al. 2023, Sci Rep) **does** evaluate AudioMAE as an embedding source for bioacoustic few-shot transfer:

- Watkins Marine Mammal Sounds Database (WMMSD, 60 cetacean/pinniped species, 32 training examples per class):
  - AudioMAE Top-1 **0.74**, ROC-AUC **0.96**.
  - Perch (bird-specific): Top-1 0.83, ROC-AUC 0.98.
- Bat calls (4 species, BT dataset, pitch-shifted to audible):
  - AudioMAE Top-1 **0.63**, ROC-AUC **0.85** (clearly weaker than bird-trained models: 0.86 / 0.97).
- RFCX Frogs: AudioMAE Top-1 **0.56**, AUC **0.89**.
- Yellowhammer dialects (fine-grained): AudioMAE Top-1 **0.61**, AUC **0.66** (struggles on fine-grained).
- Godwit calls: AudioMAE Top-1 **0.85**, AUC **0.96** (closer to birds than marine).

Ghani et al. note that AudioMAE (1) was the best **general** audio model among transformers but (2) was consistently outperformed by bird-specialist embeddings on marine and bat tasks, and (3) benefited from skipping resampling (feeding >16 kHz audio through directly) and a 2-layer probe head.

## Strengths

1. **Conceptually minimal** extension of image MAE; trivially easy to reimplement from the CV-MAE codebase by swapping the input pipeline and positional encodings.
2. **Compute-efficient pretraining** — only 20% of tokens go through the encoder, so a ViT-B can be pretrained in ~36 GPU-hours on a reasonable cluster.
3. **No out-of-domain supervision** needed to hit SOTA; removes the long-standing ImageNet initialization dependency of audio transformers (AST, PaSST).
4. **Strong general-purpose embeddings** — a single pretrained checkpoint transfers across environmental, speech, and (partially) bioacoustic domains.
5. **Open code and weights** at `facebookresearch/AudioMAE`; community re-implementations (e.g., Fonseca's re-implementation used by Perch) exist.

## Limitations

1. **Fixed 16 kHz input** — discards energy above 8 kHz Nyquist, which is catastrophic for bat echolocation (20-100 kHz) and marine mammal clicks. The Perch paper shows a partial workaround (feed native-rate audio *as if* it were 16 kHz) but this is a hack.
2. **Reconstruction loss is low-level** — as BEATs argues, MSE on patch-normalized spectrograms burns capacity on time-frequency texture rather than semantic content. BEATs beats AudioMAE on identical benchmarks with the same backbone.
3. **Fine-grained discrimination is weak** — the Yellowhammer dialect task (Top-1 0.61) shows embeddings blur sub-species call variation, a serious concern for cetacean dialect work.
4. **Decoder complexity** — the local/shifted-window decoder adds implementation complexity that reimplementations sometimes skip, hurting reproducibility.
5. **1024-dim mean-pooled embedding** (when used as a feature extractor) is neither small (bad for HNSW memory) nor semantically clean (bad for retrieval).

## Portable Details (reimplementation spec)

| Item                   | Value                                                      |
|------------------------|------------------------------------------------------------|
| Input sample rate      | 16000 Hz (pad/crop/resample to target)                     |
| Mel filterbank         | 128 bands, Kaldi-compatible                                |
| Window                 | 25 ms Hann, hop 10 ms                                      |
| Spectrogram shape      | `[1, T, 128]` where T = seconds * 100                      |
| Patch size             | 16 x 16, stride 16 (non-overlap) or stride 10 (overlap)    |
| Patch embed            | Conv2d(1, 768, kernel=16, stride=16)                       |
| Positional encoding    | Fixed 2D sinusoidal, added post-patch-embed                |
| Encoder                | ViT-B: 12 layers, dim 768, 12 heads, MLP ratio 4           |
| Encoder input          | Only unmasked tokens (~20% of full sequence)               |
| Decoder                | 16 layers, dim 512, shifted-window local attention         |
| Decoder positional enc | Fixed 2D sinusoidal, re-applied after re-ordering          |
| Decoder head           | Linear(512 -> 256) producing 16x16 patch pixel values      |
| Mask ratio (pretrain)  | 0.80 (unstructured random)                                 |
| Mask ratio (finetune)  | ~0.30 structured (time+frequency)                          |
| Loss                   | MSE on patch-normalized masked patches only                |
| Patch norm             | (p - mu_patch) / sqrt(var_patch + 1e-6)                    |
| Pretrain optimizer     | AdamW, beta1=0.9, beta2=0.95, wd=1e-4                      |
| Pretrain LR            | 2e-4 base, linear warmup 4 epochs, cosine decay            |
| Pretrain batch size    | 512 clips                                                  |
| Pretrain epochs        | 32 on AS-2M                                                |
| Finetune LR            | 2e-4 (AS-2M), 1e-3 (AS-20K / ESC / SPC / SID)              |
| Finetune weight decay  | 1e-4                                                       |
| Finetune schedule      | Cosine, warmup 2-4 epochs                                  |
| Embedding dim (ViT-B)  | 768 per token; 768 mean-pooled (sometimes 1024 after proj) |
| Embedding dim (ViT-L)  | 1024 per token                                             |
| Params (ViT-B)         | ~86 M                                                      |
| Params (ViT-L)         | ~304 M                                                     |

### Loss math (concise)

- Given patches `P in R^{N x D}` with `D = 16*16`, masking subset `M ⊂ {1..N}` at ratio 0.8.
- Encoder sees only `P_{-M}` (unmasked).
- Decoder input = concat(encoded unmasked, learnable `[MASK]` tokens), re-ordered to canonical time-freq positions, plus sinusoidal PE.
- Predicts `hat{P} in R^{N x 256}`.
- Target `T_i = (P_i - mean(P_i)) / sqrt(var(P_i) + 1e-6)` for i in M.
- `L_recon = (1/|M|) * sum_{i in M} || hat{P}_i - T_i ||_2^2`.

## Sonobuoy Integration Plan

### Fit for temporal branch

AudioMAE is a reasonable but not ideal pretrainer for the sonobuoy temporal branch because:

- **Pros**: Well-documented, open weights, ViT-B backbone slots into existing Rust ViT ports (e.g., candle-transformers has ViT); 768-dim embeddings are a sensible size for downstream heads.
- **Cons**: 16 kHz ceiling loses the upper band of odontocete clicks and ultrasonic biologics; reconstruction embeddings are semantically noisy vs BEATs; Perch beats AudioMAE on every bioacoustic benchmark tested.

### Recommended usage

1. **Use AudioMAE encoder as the second-tier fallback** for the temporal branch, behind Perch (which should be the primary bioacoustic pretrainer). AudioMAE is the right choice when the target taxon lacks any avian similarity (e.g., anthropogenic noise classification, seismic events).
2. **Adapt the input front-end**. For sonobuoy audio (typically 8-48 kHz depending on platform):
   - For broadband cetacean analysis: follow Ghani et al.'s trick — feed native-rate mel spectrograms through AudioMAE *without* resampling, treating the model as a generic 128-band spectrogram encoder. This preserves ultrasonic energy at the cost of mel-band misalignment.
   - Preferred long-term: re-pretrain a "AudioMAE-Ocean" variant on a marine corpus at 32 kHz or 96 kHz input, retaining the 16x16 patch size and 128-band mel front end.
3. **Pooling strategy**: mean-pool the final encoder tokens to produce a single `R^{768}` per segment. Alternatively, learn a `[CLS]`-style attentive-pooling head.

### HNSW compatibility

- Kernel default dimensionality: **384** (`HnswServiceConfig::default_dimensions`, `crates/clawft-kernel/src/hnsw_service.rs:38`). The field is *informational*, not enforced by the store, so larger vectors are accepted.
- AudioMAE native embedding is 768 (ViT-B) or 1024 (ViT-L). Two good options:
  1. **Project to 384**: add a learned Linear(768 -> 384) + LayerNorm head fine-tuned with a contrastive objective (InfoNCE on same-species positive pairs). Keeps the HNSW service at default dims and roughly halves RAM.
  2. **Keep native 768**: switch `HnswServiceConfig::default_dimensions` to 768 for the temporal namespace. Adds ~2x memory but preserves representation fidelity.
- Cosine similarity is the natural metric; L2-normalize embeddings before insertion so cosine and L2 distances agree (HNSW typically uses Euclidean distance internally; normalized vectors give cosine-equivalent ordering).

### Availability

- Weights: Meta releases ViT-B and ViT-L AudioMAE checkpoints at https://github.com/facebookresearch/AudioMAE (MIT + "CC-BY-NC-4.0 for audio samples"). Non-commercial clause on samples, but the **weights themselves are MIT-licensed**.
- Re-implementation: Eduardo Fonseca's TF2 re-implementation (used by Perch evaluation) is commonly referenced; ports to PyTorch Lightning exist.
- Rust inference: feasible via `candle-transformers` ViT + custom patch embed; ONNX export is straightforward.

## Follow-up References

1. **He, Chen, Xie, et al. (2022)** — "Masked Autoencoders Are Scalable Vision Learners", CVPR 2022. The image-MAE paper that AudioMAE directly extends.
2. **Gong, Chung, Glass (2021)** — "AST: Audio Spectrogram Transformer", INTERSPEECH 2021. The supervised ViT baseline AudioMAE competes against.
3. **Chen et al. (2022)** — "BEATs: Audio Pre-Training with Acoustic Tokenizers", ICML 2023. Direct successor/competitor; beats AudioMAE on every benchmark via discrete-label prediction. See `beats.md`.
4. **Baade, Peng, Harwath (2022)** — "MAE-AST: Masked Autoencoding Audio Spectrogram Transformer", arXiv:2203.16691. Concurrent work with a similar MAE extension.
5. **Niizumi et al. (2022)** — "Masked Spectrogram Modeling Using Masked Autoencoders for Learning General-purpose Audio Representation" (MSM-MAE), arXiv:2204.12260. Another concurrent audio-MAE variant worth benchmarking against.
6. **Ghani, Denton, Kahl, Klinck (2023)** — "Global birdsong embeddings enable superior transfer learning for bioacoustic classification", Scientific Reports 13:22876. Empirical evidence for bioacoustic transfer limits of AudioMAE. See `perch-bioacoustic.md`.
