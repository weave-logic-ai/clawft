# BEATs — Audio Pre-Training with Acoustic Tokenizers

## Citation

- **Title**: BEATs: Audio Pre-Training with Acoustic Tokenizers
- **Authors**: Sanyuan Chen, Yu Wu, Chengyi Wang, Shujie Liu, Daniel Tompkins, Zhuo Chen, Furu Wei
- **Affiliation**: Microsoft (MSRA, Microsoft Azure)
- **Venue**: ICML 2023
- **arXiv**: 2212.09058 (18 Dec 2022)
- **URL**: https://arxiv.org/abs/2212.09058
- **Code / Weights**: https://aka.ms/beats (redirect to microsoft/unilm repo)

## Status

**verified**. The paper matches the survey description (Chen, Wu et al., Microsoft, ICML 2023). Date, venue, authorship all confirmed.

## One-paragraph Summary

BEATs ("Bidirectional Encoder representation from Audio Transformers") replaces the spectrogram-reconstruction objective used by AudioMAE with a BERT-style discrete-label prediction task. The core contribution is an **iterative audio pre-training framework** that jointly co-trains a ViT audio encoder and a separate **acoustic tokenizer** that converts continuous spectrogram patches into semantic-rich discrete labels. Iteration 1 bootstraps with a frozen random-projection tokenizer; each subsequent iteration trains a new self-distilled Transformer tokenizer whose targets come from the previous iteration's audio SSL model (either SSL-only or fine-tuned). Pre-training is masked patch -> discrete label cross-entropy. With only 90 M parameters and no external supervised data, BEATs sets new SOTA on AudioSet-2M (48.6 mAP single-model, 50.6 mAP ensemble), ESC-50 (98.1%), and several speech benchmarks — beating AudioMAE-Large (304 M) on almost every axis. The discrete targets produce embeddings that are visibly more robust to reverberation/noise and cluster tightly by semantic label in t-SNE, making BEATs a substantially better source of retrieval-ready features than reconstruction-based SSL.

## Methodology

### Why discrete labels over reconstruction

The authors argue (and empirically show, via t-SNE) that MSE reconstruction burns model capacity on low-level time-frequency texture that is irrelevant to semantic audio understanding — the same dog bark in different rooms produces different spectrograms but the same semantic label. Discrete-label prediction, following BERT / BEiT / HuBERT, forces the encoder to abstract to semantic clusters.

### Architecture

- **Audio SSL model (encoder)**:
  - ViT backbone: 12 Transformer encoder layers, hidden dimension **768**, 8 attention heads, ~90 M parameters.
  - Matches AudioMAE (ViT-B, 86 M) and MaskSpec for fair comparison.
  - No decoder in the MAE sense — instead, a small label prediction head (Linear(768 -> K) with K = 1024 codebook size).
- **Acoustic tokenizer (iteration 1, random-projection)**:
  - Frozen linear projection W followed by nearest-neighbor lookup in a set of K=1024 frozen random codebook vectors `v_i ∈ R^{256}`.
  - Produces one discrete label per input patch.
  - `ẑ_t = argmin_i || v_i − W x_t ||_2^2`.
- **Acoustic tokenizer (iteration ≥ 2, self-distilled)**:
  - 12-layer Transformer encoder (tokenizer encoder) → learnable codebook `V = {v_i}_{i=1}^{1024}` with 256 dims per code → 3-layer Transformer estimator that predicts the teacher model's last-layer output from the quantized code sequence.
  - Vector quantization with L2-normalized codebook entries and straight-through gradient estimator (Van Den Oord 2017, BEiT-v2 style).
  - Training loss combines cosine similarity to teacher outputs plus commitment + codebook MSE terms.

### Input pipeline

- Waveform resampled to 16 kHz, mono.
- **128-dimensional mel filterbank**, 25 ms Povey window, 10 ms hop (Kaldi-compatible, same as AudioMAE).
- Features normalized to zero mean / unit variance.
- Patchification: spectrogram is split into non-overlapping patches; in BEATs the patches are `16 x 16` (same as AudioMAE / AST) yielding `T = 512` tokens for a 10-second AS-2M clip.

### Masking strategy

- Pre-training mask ratio: **75%** (i.e. `|M| = 0.75 T`). Significantly lower than AudioMAE's 80%, reflecting that discrete-label prediction is harder than reconstruction and benefits from more visible context.
- Unstructured random masking over the patch sequence.
- Fine-tuning uses **SpecAugment-style structured masking** in time and frequency dimensions on the full sequence (no MAE-style "only unmasked" trick at fine-tuning).

### Pre-training objective

Mask-and-predict with cross entropy over discrete labels:

```
L_MAM = − Σ_{t ∈ M} log p(ẑ_t | X_U)
```

where `X_U = {x_t : t ∉ M}` is the unmasked patch sequence fed to the encoder, and `ẑ_t` are the tokenizer's discrete labels at masked positions. The label predictor (linear head on encoder output) is only queried at masked positions.

Note: unlike AudioMAE, the encoder sees only unmasked tokens (same speed-up trick), but because targets are discrete class indices rather than 256-D reconstruction vectors, the gradient signal is cleaner.

### Tokenizer training (self-distilled) loss

```
L_tok = Σ_X Σ_t [ cos(o_t, ô_t) − ||sg[ℓ2(e_t)] − ℓ2(v_{ẑ_t})||_2^2 − ||ℓ2(e_t) − sg[ℓ2(v_{ẑ_t})]||_2^2 ]
```

where `e_t` is the tokenizer encoder's output, `v_{ẑ_t}` is the nearest codebook entry, `o_t` is the tokenizer-estimator's prediction, `ô_t` is the teacher model's last-layer output, `sg[·]` is stop-gradient, and `ℓ2(·)` is L2 normalization. EMA updates for codebook entries (same as VQ-VAE / BEiT).

### Iterative pre-training recipe

| Iter | Tokenizer              | Teacher               | SSL Data | Supervised Data |
|------|------------------------|-----------------------|----------|-----------------|
| 1    | Random-Projection      | —                     | AS-2M    | —               |
| 2    | Self-Distilled         | BEATs_iter1 (SSL)     | AS-2M    | —               |
| 3    | Self-Distilled         | BEATs_iter2 (SSL)     | AS-2M    | —               |
| 3+   | Self-Distilled         | BEATs_iter2 fine-tuned on AS-20K or AS-2M | AS-2M    | AS-20K or AS-2M |

Each iteration: pre-train the audio SSL model to convergence, then train a new tokenizer using that model as the teacher, then restart SSL from scratch with the new tokenizer. Converges in 3 iterations.

### Pre-training hyperparameters

- Optimizer: AdamW, β1 = 0.9, β2 = 0.98, weight decay 0.01.
- Schedule: linear decay with warmup (SSL), cosine decay (fine-tune).
- SSL: 400 k steps, batch size **5.6 K audio-seconds** (i.e. ~560 10-second clips), peak LR **5e-4**.
- Self-distilled tokenizer (SSL teacher): 400 k steps, batch size 1.4 K seconds, LR **5e-5**.
- Self-distilled tokenizer (supervised teacher): 400 k steps, batch size 1.4 K seconds, LR 5e-5.
- Codebook: K = 1024 entries, each 256 dimensional, L2-normalized, EMA-updated.

### Fine-tuning recipe

- Discard the tokenizer and label predictor; append a task-specific linear classifier on the **mean-pooled** encoder output.
- Input: spectrogram with SpecAugment time+frequency masks applied.
- Feed **full** (unmasked) sequence to encoder at fine-tuning time.
- Loss: cross-entropy (single-label tasks), BCE (multi-label e.g. AudioSet), with optional Mixup.
- 50 k steps on AS-2M, 80 k on AS-20K, cosine LR decay, linear warmup.
- Typical fine-tune LR: 1e-4 to 5e-5 depending on task.

## Key Results

### AudioSet-2M (single-model mAP)

| Model                            | Params | Data     | mAP  |
|----------------------------------|--------|----------|------|
| AST (IN-pretrained)              | 86 M   | IN + AS  | 45.9 |
| PaSST                            | 86 M   | IN       | 47.1 |
| AudioMAE (ViT-B)                 | 86 M   | AS       | 47.3 |
| AudioMAE Large                   | 304 M  | AS       | 47.4 |
| MaskSpec                         | 86 M   | AS       | 47.1 |
| **BEATs_iter1** (random-proj)    | 90 M   | AS       | **47.9** |
| **BEATs_iter2**                  | 90 M   | AS       | **48.1** |
| **BEATs_iter3**                  | 90 M   | AS       | **48.0** |
| **BEATs_iter3+** (SL teacher)    | 90 M   | AS+AS-20K| **48.6** |

BEATs single-model beats AudioMAE-Large by **1.2 mAP** at less than a third of the parameters.

### AudioSet-2M (ensemble mAP)

| Model                            | Ensemble | mAP  |
|----------------------------------|----------|------|
| PaSST                            | ensemble | 49.6 |
| **BEATs (5 models)**             | 5        | **50.4** |
| **BEATs (10 models)**            | 10       | **50.6** |

Sets new SOTA with no external supervised data.

### AudioSet-20K (mAP)

- AudioMAE ViT-B: 37.1; BEATs_iter3+: **38.9** (SL teacher) / **41.8** with additional AS-20K supervision in tokenizer.

### ESC-50 (accuracy)

- AudioMAE (SSL-only): 94.1
- HTS-AT (IN + AS supervised): 97.0
- **BEATs_iter3+**: **98.1** (SSL-only, no ImageNet)
- Best reported: 98.1 (25% relative error reduction over prior SOTA of 94.1).

### Speech Commands V2 / V1 (accuracy)

- SPC-2 (KS2): BEATs **98.3**, matching AudioMAE; SPC-1 (KS1): BEATs **98.1**.

### IEMOCAP Emotion Recognition (ER) accuracy

- Previous SSL SOTA (Wav2vec2): 63.4
- **BEATs_iter2: 66.1**
- BEATs_iter1: 65.9

### Bioacoustic / marine / underwater transfer

The BEATs paper **does not evaluate on bioacoustic or underwater benchmarks**. Post-publication community benchmarks (e.g., BIRB, BEANS, cetacean classification efforts) generally show BEATs tracking AudioMAE's performance pattern — i.e., it transfers to bioacoustics **better than AudioMAE** on most tasks thanks to cleaner semantic embeddings, but still under-performs Perch on avian data and remains inferior to domain-specialist marine models (e.g., the "LeBellier" bioacoustic models). Notably, the "Foundation Models for Bioacoustics — a Comparative Review" (arXiv:2508.01277) places BEATs in the top tier of general-audio models for bioacoustic transfer but consistently below Perch 2.0 for marine mammal tasks.

Key transfer properties relevant to sonobuoy work:

- BEATs embeddings are **semantically clustered** (see Figure 4 in the paper's t-SNE visualizations): audio with the same semantic label but very different acoustic conditions (reverb, DNS noise) lands near in embedding space. AudioMAE's reconstruction-based embeddings scatter under the same perturbations.
- This property is precisely what you want for **underwater retrieval** where propagation-induced distortion (multipath, absorption) is severe.

## Strengths

1. **Best-in-class general-audio SOTA at publication** on AS-2M, AS-20K, ESC-50, KS1, KS2, ER — a clean sweep.
2. **Parameter efficient**: 90 M params beats AudioMAE-Large's 304 M on almost every task.
3. **Semantically clean embeddings**: t-SNE visualization shows tight intra-class clustering under noise/reverb perturbations, far better than reconstruction-based SSL. Directly relevant to HNSW retrieval quality.
4. **Ensemble-friendly**: 10-model ensemble reaches 50.6 mAP on AS-2M without any external labels.
5. **Iterative framework is general**: the tokenizer/SSL co-training recipe applies to any modality and is reusable.

## Limitations

1. **Iterative pre-training is expensive**: each of the 3 iterations is a full 400 k-step SSL run plus a 400 k-step tokenizer run — ~6x the wall-clock of AudioMAE for the same encoder.
2. **16 kHz input + 128-band mel** inherits the exact same ultrasonic / infrasonic blind spots as AudioMAE. For sonobuoy applications that need >8 kHz bandwidth (odontocete echolocation clicks reach 150 kHz) this is a hard constraint without re-pretraining.
3. **Discrete targets are brittle to distribution shift**: the codebook is learned on AudioSet, so for out-of-distribution marine audio many patches will collapse to a handful of "noise" codes, reducing the effective capacity the encoder actually uses.
4. **Tokenizer complexity**: the self-distilled tokenizer adds a non-trivial engineering surface (VQ codebook, straight-through gradients, EMA updates, teacher management) that AudioMAE avoids.
5. **License is research-oriented**: Microsoft's repo uses a research-use license on the pretrained weights — check compatibility before shipping BEATs in a commercial sonobuoy product.

## Portable Details (reimplementation spec)

| Item                          | Value                                                |
|-------------------------------|------------------------------------------------------|
| Input sample rate             | 16000 Hz                                             |
| Mel filterbank                | 128 bands, Povey window, 25 ms, 10 ms hop            |
| Feature normalization         | Zero-mean, unit-variance (global)                    |
| Patch size                    | 16 x 16, non-overlapping                             |
| Patch embed                   | Linear projection from 256-D flattened patch -> 768  |
| Positional encoding           | Fixed 2D sinusoidal (absolute), per-patch            |
| Encoder                       | 12 layers, dim 768, 8 heads, MLP 3072, 90 M params   |
| Mask ratio (pretrain)         | 0.75 unstructured                                    |
| Label predictor               | Linear(768 -> 1024) at masked positions only         |
| Codebook size K               | 1024                                                 |
| Codebook dim                  | 256 (L2-normalized)                                  |
| Tokenizer encoder             | 12-layer Transformer                                 |
| Tokenizer estimator           | 3-layer Transformer                                  |
| Pretrain loss                 | Cross-entropy on discrete labels at masked positions |
| Pretrain optimizer            | AdamW, β1=0.9, β2=0.98, wd=0.01                      |
| Pretrain LR                   | 5e-4 peak, linear decay, warmup                      |
| Pretrain batch                | 5.6 K audio-seconds (~560 10-s clips)                |
| Pretrain steps                | 400 k per iteration                                  |
| Tokenizer LR                  | 5e-5 peak                                            |
| Tokenizer batch               | 1.4 K audio-seconds                                  |
| Tokenizer steps               | 400 k                                                |
| Finetune optimizer            | AdamW, β1=0.9, β2=0.98, wd=0.01                      |
| Finetune LR                   | 1e-4 to 5e-5 task-dependent, cosine decay            |
| Finetune steps                | 50 k (AS-2M) / 80 k (AS-20K / ESC)                   |
| Finetune input masking        | SpecAugment (time+freq)                              |
| Embedding dim (per token)     | 768                                                  |
| Embedding dim (mean-pooled)   | 768                                                  |
| Params (encoder)              | 90 M                                                 |

### Loss math (concise)

- Patches `P in R^{N x 256}`, encoder hidden dim 768.
- Tokenizer produces `z in [0, K-1]^N`.
- Mask subset `M` with |M| = 0.75 N.
- Encoder processes `P_{-M}` -> hidden states `H_{-M} in R^{|-M| x 768}`.
- Mask tokens inserted, sequence re-ordered, fed through label head `W_lab in R^{768 x 1024}`.
- `p_t = softmax(W_lab H_t)` at masked positions.
- `L = -(1/|M|) Σ_{t in M} log p_t[ẑ_t]`.

## Sonobuoy Integration Plan

### Fit for temporal branch

BEATs is the **best general-audio pretrainer** for the sonobuoy temporal branch when bioacoustic-specialist models (Perch) are not taxon-appropriate. Its discrete-label embeddings have two properties that are extremely valuable for underwater retrieval:

1. **Noise robustness** — directly demonstrated by the t-SNE figures under DNS noise and RIR reverb. Underwater multipath is structurally similar to severe RIR; BEATs should degrade more gracefully than AudioMAE or reconstruction-based models.
2. **Semantic clustering** — retrieval via HNSW cosine nearest-neighbor is dominated by *semantic* similarity rather than acoustic texture, which is what sonobuoy analysts actually want ("find me more clicks like this one", not "find me more spectrograms with similar noise floor").

### Recommended usage pattern

1. **Tier 1 pretrainer**: BEATs encoder as the shared backbone for the non-bird temporal branch (marine mammals, anthropogenic noise, environmental sounds). Perch remains primary for avian / aerial bioacoustics.
2. **Inference**: run the 12-layer encoder on full (unmasked) sequences. Take the mean-pooled final hidden state (`R^{768}`) as the per-segment embedding.
3. **Optional fine-tuning**: with a labeled marine subset (e.g., DOSITS annotations, Watkins, NOAA Passive Acoustic archives), add a 3-5 k step supervised fine-tune with AdamW, LR 5e-5, SpecAugment on, BCE loss for multi-label event detection.
4. **Input adaptation**: sonobuoy recordings are often 32-48 kHz. Option A (fastest): resample to 16 kHz and accept high-frequency loss. Option B (better): keep native rate but re-fit the mel-filterbank to cover the full band (128 bands from 0 to Nyquist) — the encoder weights remain valid for the patch-level abstraction even as the mel centers shift, though absolute accuracy drops and a short linear-probe fine-tune is warranted.
5. **Clone+adapt**: an aggressive option is to run one additional BEATs iteration on a marine corpus (e.g., 200 hours of labeled + 5 k hours unlabeled passive acoustic) with the AS-2M tokenizer as teacher, then refit the tokenizer against the marine-tuned encoder. This is the "BEATs-Ocean" equivalent of the "AudioMAE-Ocean" plan.

### HNSW compatibility

- BEATs native embedding is **768-D** (ViT-B-like).
- Kernel default dim is 384 (`clawft-kernel/src/hnsw_service.rs:38`); informational only.
- Recommended path:
  1. L2-normalize the 768-D embedding before insertion.
  2. Insert as-is and reconfigure the temporal namespace to `default_dimensions: 768`.
  3. Alternatively, project to 384 with a Linear(768 -> 384) + LayerNorm head trained with InfoNCE over same-event pairs; this keeps the service dimensionality unified with other 384-D branches.
- For retrieval, cosine similarity; with L2-normalized vectors, `dist_L2^2 = 2 − 2·cos`, so the standard Euclidean HNSW ordering is correct.

### Availability

- **Code**: `microsoft/unilm` -> `beats` subdirectory. Well-maintained.
- **Weights**: BEATs_iter3+ (AS-2M fine-tuned teacher) checkpoints released for AS-2M and AS-20K fine-tuning, plus base SSL. Research license; confirm before shipping commercially.
- **ONNX export**: trivial (standard ViT encoder). Rust inference via `candle-transformers` ViT + custom mel front end is straightforward.
- **Embedding service**: 768-D f32 vectors, ~3 KB per segment. At 1 M segments that is ~3 GB raw; HNSW adds ~30-50% overhead. Reasonable for the sonobuoy target scale.

## Follow-up References

1. **Xu, Huang et al. (2022)** — "Audio-MAE" (arXiv:2207.06405). Direct predecessor/competitor; BEATs' primary benchmark. See `audio-mae.md`.
2. **Baevski, Zhou, Mohamed, Auli (2020)** — "wav2vec 2.0" (NeurIPS 2020). The contrastive + discrete-code SSL framework for speech that inspired BEATs' discrete targets.
3. **Bao, Dong, Wei (2021)** — "BEiT: BERT Pre-Training of Image Transformers" (ICLR 2022). The visual analogue that BEATs most directly generalizes to audio.
4. **Hsu et al. (2021)** — "HuBERT: Self-Supervised Speech Representation Learning by Masked Prediction of Hidden Units" (TASLP 2021). The speech parallel — iterative clustering + masked label prediction.
5. **Chiu et al. (2022)** — "Self-supervised Learning with Random-Projection Quantizer for Speech Recognition". Provenance of the iter-1 random-projection tokenizer idea.
6. **Peng et al. (2022)** — "BEiT v2: Masked Image Modeling with Vector-Quantized Visual Tokenizers". Source of the L2-normalized codebook and EMA tricks adopted by BEATs' self-distilled tokenizer.
