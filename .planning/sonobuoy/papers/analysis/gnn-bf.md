# Paper 4.1 — GNN-BF (Multi-Channel Speech Enhancement Using Graph Neural Networks)

## Citation

Tzirakis, P., Kumar, A., & Donley, J. (2021). "Multi-Channel Speech
Enhancement Using Graph Neural Networks." *Proc. IEEE International
Conference on Acoustics, Speech and Signal Processing (ICASSP 2021)*,
pp. 3415-3419. arXiv:2102.06934. Facebook Reality Labs Research.

## Status

**Substituted.** The survey cited "Tzirakis, Kumar et al., ICASSP 2024"
but the real Tzirakis/Kumar/Donley paper is **ICASSP 2021**. The 2024 year
is a transcription error — the authors, topic, venue, and survey
description all match arXiv:2102.06934 exactly. No newer Tzirakis/Kumar
ICASSP paper on GNN beamforming exists.

The survey's one-line characterization ("Replaces classical MVDR /
delay-and-sum beamforming with a GNN ... beats MVDR under reverberation")
is essentially accurate. The survey adds the phrase "sensor position
jitter"; the paper itself does *not* evaluate sensor jitter directly
(only array geometry, number of mics, and SNR). That claim is
extrapolated.

PDF: `.planning/sonobuoy/papers/pdfs/gnn-bf.pdf` (829 KB, 5 pages).

## One-paragraph summary

This is the first paper to treat multi-channel speech enhancement as a
graph learning problem. Each microphone channel is a node, all pairs are
fully connected, and the edge weights — the *adjacency matrix* — are
**learned dynamically** by an MLP that takes the concatenated node
embeddings and outputs a per-edge similarity. A U-Net encoder-decoder
operates per channel on the complex STFT; the GCN operates inside the
U-Net's embedding space, aggregating cross-channel information via
`H = g(D⁻¹ᐟ² A D⁻¹ᐟ² H W)` (Kipf-Welling propagation with learned A); the
decoder produces per-channel masks; a learned weighted sum fuses them,
and the result is complex-multiplied with a reference-mic STFT to give
the enhanced spectrogram. On LibriSpeech + simulated room acoustics with
linear, circular, and distributed 2/4/8-mic arrays, the GNN beats a
CRNN-C baseline (a recent DNN+beamforming method) by up to 4.9 dB SDR at
-7.5 dB input SNR and is robust across array geometries without
retraining or explicit geometric priors. An ablation shows +0.04 STOI,
+0.05 PESQ, +0.68 dB SDR purely from adding the GCN inside U-Net.

## Methodology

### Architecture

Five stages, in order:

1. **Per-channel STFT encoder.** Hanning-window STFT with length 1024,
   513 frequency bins, real+imag stacked → 2-channel tensor per
   microphone.
2. **U-Net encoder.** Shared-weight U-Net encoder mapping each channel
   to an embedding of shape `(C, T, F')` in a reduced spectral
   dimension.
3. **Graph construction.** Complete undirected graph `G = (V, E)` with
   `|V| = M` nodes (mics). Edge weights are not hand-designed; they are
   learned via a non-linear MLP `F`:

   ```
   w_ij = F([f_vi ‖ f_vj]),    A_ij = softmax_row(w_ij)
   D_ii = Σ_j A_ij
   ```

   where `f_vi ∈ ℝ^N` are the U-Net encoder embeddings of mic `i`,
   `‖` is concatenation. This makes the adjacency matrix a *function of
   the audio content*, so it adapts per utterance.
4. **GCN layer.** Two GCN layers (Kipf-Welling spectral form), hidden
   size = embedding dim:

   ```
   H^(l) = g(D^{-1/2} A D^{-1/2} H^(l-1) W^(l-1))
   ```

   `g` is SELU. This is the actual beamforming step: each node's output
   is a weighted combination of all other nodes' features, with weights
   = the learned adjacency.
5. **Per-channel decoder + fused output.** U-Net decoder (shared
   weights) per channel → per-channel complex mask. An attention layer
   computes a weighted sum across channels. That weighted sum is
   complex-multiplied with the STFT of the reference mic to produce
   the enhanced STFT.

### Training

- **Optimizer:** Adam, fixed learning rate `1e-5`, mini-batch of 20
  frames.
- **Input:** complex STFT, Hanning window of 1024, 513 frequency bins.
- **Epochs:** not explicitly stated; training runs until early-stopping
  on validation loss.
- **Hardware:** not reported.

### Loss

Four loss components are evaluated; the best combination is
`L_Mag + L_Raw`:

```
L_Mag      = ‖|M̂| − |M|‖₁          (magnitude L1)
L_Complex  = ‖M̂ − M‖₁              (complex L1)
L_Raw      = ‖x̂ − x‖₁              (raw waveform L1, via iSTFT)
L_Phase    = cosine loss on phase
```

Ablation (Table 3):

| Loss                 | PESQ | STOI | SDR  |
| -------------------- | ---- | ---- | ---- |
| L_Mag                | 2.07 | 0.71 | 7.09 |
| L_Complex            | 2.09 | 0.72 | 7.43 |
| L_Mag + L_Raw (used) | 2.13 | 0.73 | 7.73 |

### Datasets

- **LibriSpeech** (≈1000 h English speech, 16 kHz) for clean speech.
- **Simulated room acoustics** via an image-source model. Three array
  geometries:
  - Linear
  - Circular
  - Distributed (random placement in room)
- Mic counts: 2, 4, 8.
- **SNR mixing:** input SNRs drawn from `{-7.5, -5, 0, 5, 7.5} dB`.
- Train/val/test split not quantitatively described.

## Key results

**Table 1 — STOI / PESQ / SDR across array types** (excerpt; higher is
better; SDR in dB):

| Method (# mics)            | Linear (STOI/PESQ/SDR) | Circular          | Distributed       |
| -------------------------- | ---------------------- | ----------------- | ----------------- |
| Noisy (4)                  | 0.65 / 1.69 / 0.59     | 0.66 / 1.62 / 0.22 | 0.66 / 1.64 / 0.24 |
| CRNN-C [14] (4)            | 0.68 / 1.75 / 3.17     | 0.70 / 1.88 / 4.89 | 0.67 / 1.71 / 2.25 |
| **Proposed (4)**           | **0.71 / 2.10 / 7.04** | **0.73 / 2.21 / 8.53** | **0.71 / 2.02 / 6.72** |
| Proposed single-channel    | 0.69 / 1.96 / 6.48     | 0.69 / 1.98 / 6.38 | 0.68 / 1.94 / 5.77 |

**Table 2 — SNR sweep, 4-mic linear array:**

| Input SNR (dB) | Noisy SDR | CRNN-C SDR | **Proposed SDR** |
| -------------- | --------- | ---------- | ---------------- |
| -7.5           | -6.05     | -1.08      | **3.84**         |
| -5             | -3.34     | 0.82       | **5.74**         |
| 0              | 0.30      | 2.61       | **6.84**         |
| 5              | 5.03      | 6.36       | **8.86**         |
| 7.5            | 6.96      | 7.32       | **9.94**         |

**Interpretation:** 9.89 dB SDR improvement over noisy at -7.5 dB SNR;
4.92 dB improvement over the CRNN-C neural baseline. The gain over the
baseline *grows* as SNR drops — exactly where MVDR-style methods
degrade.

**Table 4 — ablation (with vs. without the GCN, U-Net backbone
otherwise identical):**

| Variant  | PESQ | STOI | SDR  |
| -------- | ---- | ---- | ---- |
| w/o GCN  | 2.08 | 0.71 | 7.05 |
| w/ GCN   | 2.13 | 0.73 | 7.73 |

So the GCN itself buys ~0.68 dB SDR on top of a strong U-Net. Modest
but meaningful; the *larger* gain comes from the complex-masking
architecture.

## Strengths

1. **Dynamic adjacency.** Edge weights are learned per utterance, not
   fixed to a static array geometry. For drifting sonobuoys this is
   exactly the right inductive bias — the graph reconfigures itself.
2. **Geometry-agnostic.** One trained model handles linear, circular,
   and distributed arrays without retraining, and the performance is
   comparable across geometries (Table 1). Conventional beamformers
   need explicit calibration per geometry.
3. **Robust at low SNR.** Gains over baseline widen as SNR drops
   (Table 2) — the harder the problem, the more graph structure helps.
   Deep-ocean sonobuoy scenarios live in the low-SNR regime.
4. **Clean factorization.** Encoder → graph → decoder is modular. The
   same spine can be reused for detection, localization, or
   classification by swapping the decoder head.
5. **Simple, closed-form GCN.** The `D^(-1/2) A D^(-1/2) H W` layer is
   trivial to implement (one matmul + normalization), cheap to
   differentiate, no iterative solver required.

## Limitations

1. **Edge weights are content-similarity, not coherence.** The learned
   `w_ij` captures feature similarity, not acoustic inter-sensor
   coherence explicitly. The survey claim that edges are
   "coherence-based" is aspirational — the paper doesn't enforce this;
   it's what the MLP *might* learn from data.
2. **Sensor position jitter not tested.** The paper doesn't run any
   ablation where mic positions drift. We only *infer* robustness from
   the fact that distributed-array performance is comparable to
   circular.
3. **Fully-connected graph.** `O(M²)` edges — fine for 2-8 mics, ugly
   for 50+ buoys. A sonobuoy-scale array (20-100 buoys) would need
   KNN sparsification.
4. **Single static source.** All experiments are one speaker. Real
   sonobuoy fields have multiple whales + shipping traffic + ambient.
   The graph-fusion weighting step doesn't model multi-source
   separation.
5. **Rooms, not oceans.** Reverberation is from image-source room
   models (T60-like), not ocean multipath or refraction. Transfer to
   underwater is plausible but unverified.
6. **No uncertainty calibration.** The output is a point estimate
   (enhanced STFT). For downstream decisions (track fusion, ATR) a
   calibrated posterior would be preferable.

## Portable details (for Rust implementation)

### Key equations

**Adjacency computation** (learned, per-utterance):

```
w_ij = F_θ(concat(f_i, f_j))            F_θ: MLP, 2 hidden layers, SELU
A_ij = softmax_row(w_ij)                # normalize each row to sum=1
D_ii = Σ_j A_ij                          # ≡ 1 after softmax
```

Because rows sum to 1 after softmax, `D = I`. The GCN layer therefore
reduces to:

```
H^(l+1) = g(A H^(l) W^(l))
```

**GCN layer** (two layers):

```
H^(1) = SELU(A H^(0) W^(0)),   W^(0) ∈ ℝ^(N × N)
H^(2) = SELU(A H^(1) W^(1)),   W^(1) ∈ ℝ^(N × N)
```

where `N = embedding dimension` of the U-Net encoder.

### Hyperparameters (from paper)

| Parameter            | Value                    |
| -------------------- | ------------------------ |
| STFT window          | Hanning, length 1024     |
| STFT bins            | 513                      |
| Frame rate           | 16 kHz sampling          |
| Optimizer            | Adam                     |
| Learning rate        | `1e-5`                   |
| Mini-batch           | 20 frames                |
| GCN layers           | 2                        |
| GCN hidden units     | = U-Net embedding dim    |
| GCN activation       | SELU                     |
| Edge-MLP `F`         | not fully specified; likely 2 layers ReLU/SELU, hidden ≈ embedding dim |
| Losses               | `L_Mag + L_Raw` (L1)     |

### Rust/Candle implementation sketch

```rust
// Shape notation: B = batch, M = mics, T = time frames, N = embedding dim.

// Per-channel STFT (complex → 2-channel real tensor of shape (B, M, 2, T, 513))
let stft = complex_stft(signal, window=hanning(1024), hop=256);

// U-Net encoder (shared across mics, operates per-channel):
//   in: (B*M, 2, T, 513) → out: (B*M, C_enc, T', N)
let embed = unet_encoder.forward(&stft);
let embed = embed.reshape((B, M, -1));  // (B, M, D) with D = C_enc*T'*N

// Build adjacency by pairwise concat-MLP:
//   w_ij = edge_mlp(concat(h_i, h_j))  for all (i,j)
let pairs = concat_pairs(&embed);                        // (B, M, M, 2D)
let w = edge_mlp.forward(&pairs).squeeze(-1);            // (B, M, M)
let a = softmax(&w, dim=2);                              // row-normalized

// Two-layer GCN:
let h = embed;
let h = selu(&a.matmul(&h).matmul(&w_gcn_0));            // (B, M, D)
let h = selu(&a.matmul(&h).matmul(&w_gcn_1));            // (B, M, D)

// Per-channel decode + per-channel complex mask:
let masks = unet_decoder.forward(&h);                    // (B, M, 2, T, 513)

// Attention-weighted sum across M mics:
let att = attention_weights(&masks);                     // (B, M, 1, T, 513)
let fused_mask = (att * masks).sum_dim(1);               // (B, 2, T, 513)

// Reference-mic complex mul:
let enhanced_stft = complex_mul(&fused_mask, &stft.select(ref_mic));
let enhanced = istft(enhanced_stft, window=hanning(1024), hop=256);
```

The critical data structure for sonobuoys is the **(B, M, M) dynamic
adjacency tensor** — it is cheap, fully differentiable, and implicitly
encodes inter-sensor coherence by similarity of learned features.

### Loss implementation

```rust
let l_mag = (enhanced_mag - target_mag).abs().mean();
let l_raw = (enhanced - target).abs().mean();          // after iSTFT
let loss = l_mag + l_raw;
```

### Computational cost

- Edge-MLP cost: `O(M² · D)` per sample. For M=20 sonobuoys and D=256,
  this is ~100k ops per sample — negligible.
- GCN layers: `O(M² · D + M · D²)` per layer. Dominated by `D²` when
  `M << D`.

## Sonobuoy integration plan

This paper occupies the **spatial-graph branch** of the
K-STEMIT-extended architecture. In K-STEMIT's dual-branch spatio-
temporal GNN, the spatial branch was a static haversine-weighted
GraphSAGE. Tzirakis GNN-BF replaces that with a **dynamic,
content-conditioned adjacency** — far more robust for drifting
sonobuoys where the haversine distance is only approximately correct
and acoustic coherence (the thing we actually care about) may not
track distance monotonically when refraction or shadow zones
intervene.

**Where it slots in:**

1. **Front-end per-buoy encoder** — replace K-STEMIT's GLU-gated
   temporal convolution with Tzirakis's U-Net encoder (operates on
   the STFT of each buoy's hydrophone signal). Output: per-buoy
   embedding `f_v ∈ ℝ^N`.
2. **Graph layer** — Tzirakis's learned-adjacency GCN, operating on
   node features `{f_v}`. Edge weights are learned from pairwise
   concat-MLP. This is the drop-in replacement for K-STEMIT's
   GraphSAGE with haversine weights. The adjacency now adapts to:
   - Current acoustic coherence between buoys (learned proxy).
   - Multipath regime (learned from data).
   - Sensor dropout (rows/columns with low A_ij get down-weighted
     automatically).
3. **Decoder heads** — per-buoy complex mask (like Tzirakis) for the
   **detection/enhancement** head, and a separate **bearing head**
   that takes the graph-pooled embedding and regresses a
   direction-of-arrival.
4. **Fusion with `clawft-kernel/src/quantum_register.rs`** — the
   quantum cognitive layer's force-directed graph-to-register mapping
   can consume the Tzirakis-learned adjacency matrix directly (one
   attention-pool step). Mapping: A-matrix → register topology
   (weighted, symmetric, row-normalized — already satisfies the
   quantum layout invariants).

**Changes needed for sonobuoy specifically:**

- Replace `|M|=2..8` with `|M|=8..64`. Move from fully-connected to
  k-NN graph (k=8) once `M>16`, using haversine as the KNN metric but
  letting the edge-MLP *re-weight* within those edges. This preserves
  Tzirakis's adaptivity while cutting `O(M²)` to `O(kM)`.
- Replace room-acoustics training data with `pinn-ssp-helmholtz` +
  `fno-propagation` simulations (already downloaded — see sibling
  agents' PDFs).
- Use complex STFT at a lower sample rate (2-16 kHz depending on
  target taxon) instead of 16 kHz Hanning-1024.
- Add a bearing-regression head alongside the enhancement decoder.
  Share the GCN trunk; only the final heads differ.

**Proposed ADR**: ADR-054 — *Dynamic-adjacency GCN for spatial branch
of sonobuoy dual-branch architecture.* Supersedes the haversine-
static-adjacency choice from K-STEMIT when dealing with drifting
free-floating sonobuoys.

## Follow-up references

1. **Santoro et al., "A simple neural network module for relational
   reasoning,"** NeurIPS 2017. The Relation Network that inspired the
   pair-concat-MLP edge computation. [arXiv:1706.01427]
2. **Kipf & Welling, "Semi-Supervised Classification with Graph
   Convolutional Networks,"** ICLR 2017. Origin of the `D^(-1/2) A
   D^(-1/2) H W` propagation used in the GCN layer. [arXiv:1609.02907]
3. **Hamilton, Ying & Leskovec, "Inductive Representation Learning on
   Large Graphs" (GraphSAGE),** NeurIPS 2017. The K-STEMIT spatial
   branch uses this; good reference for swapping Tzirakis's
   fully-connected GCN with neighborhood-sampled GraphSAGE when scaling
   to 64+ buoys. [arXiv:1706.02216]
4. **Heymann, Drude & Haeb-Umbach, "Neural network based spectral mask
   estimation for acoustic beamforming,"** ICASSP 2016. The CRNN-C
   baseline Tzirakis outperforms — represents the mainstream "DNN
   mask estimation + classical MVDR" approach that GNN-BF displaces.
5. **Veličković et al., "Graph Attention Networks,"** ICLR 2018.
   Replaces Tzirakis's concat-MLP with multi-head attention for the
   edge weighting, often higher-quality at slightly more compute.
   Natural next-step generalization. [arXiv:1710.10903]

---

*Generated 2026-04-15. PDF at `/claw/root/weavelogic/projects/clawft/.planning/sonobuoy/papers/pdfs/gnn-bf.pdf`. Source: arXiv:2102.06934 (ICASSP 2021).*
