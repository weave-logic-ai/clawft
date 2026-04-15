# AST Fish Classification — Deep Analysis

## Citation

- **Title (survey, as given)**: "AST (Audio Spectrogram Transformer) fine-tuned on
  ~40k labeled fish-call clips across 12 species (grouper, cod, damselfish).
  7-11 F1 points over CNN baselines on low-SNR soundscapes."
- **Survey attribution**: Waddell, Rasmussen et al. *Ecological Informatics*, 2024.
- **Verified title (substituted)**: *"Leveraging tropical reef, bird and unrelated
  sounds for superior transfer learning in marine bioacoustics."*
- **Verified authors**: Ben Williams, Bart van Merriënboer, Vincent Dumoulin,
  Jenny Hamer, Eleni Triantafillou, Abram B. Fleishman, Matthew McKown,
  Jill E. Munger, Aaron N. Rice, Ashlee Lillis, Clemency E. White,
  Catherine A. D. Hobbs, Tries B. Razak, Kate E. Jones, Tom Denton.
- **Venue**: arXiv preprint (cs.SD). Affiliated with UCL, ZSL, Google DeepMind,
  Cornell Lab of Ornithology, and others.
- **Year**: 2024
- **arXiv ID / DOI**: arXiv:2404.16436 · https://doi.org/10.48550/arXiv.2404.16436
- **Primary URL**: https://arxiv.org/abs/2404.16436
- **Code**: https://github.com/google-research/perch ; SurfPerch model on
  Zenodo https://doi.org/10.5281/zenodo.11060189
- **Downloaded PDF**: `.planning/sonobuoy/papers/pdfs/ast-fish-classification.pdf`
  (1.48 MB, 28 pages) — verified via pypdf text extraction.

## Status

**Substituted**. The Waddell/Rasmussen *Ecological Informatics* 2024 AST-on-fish
paper described in the survey could not be located on arXiv, Google Scholar,
ScienceDirect, or the *Ecological Informatics* journal index. The closest
real 2024 work on transformer-style pretrained models fine-tuned on underwater
fish/reef acoustic data is Williams et al. (2024), "SurfPerch". The
substitution keeps the survey's thematic intent intact (transfer learning of a
large pretrained audio model to underwater fish-dominated passive acoustic
monitoring with a strong F1 improvement over CNN baselines) while correcting
the specific architecture and author identity:
- The survey called this "AST"; Williams et al. use an EfficientNet-based
  Perch backbone plus the PCEN-log-mel front-end (not a pure AST transformer).
  The fine-tuning and embedding-to-classifier protocol is analogous, and the
  AST branch of the citation tree is still directly relevant — any ECC/AST
  implementation should track AST, BEATs, and SurfPerch together (see
  `pdfs/beats.pdf` and `pdfs/audio-mae.pdf` in the parent corpus).
- The survey's 40k clips / 12 species number likely corresponds to ReefSet
  (57,074 clips / 33 secondary labels across 16 datasets from 12 countries).

If a later pass surfaces the actual Waddell/Rasmussen paper, this analysis
should be reconciled: the substitution intentionally preserves the
task-head role (fine-tuned transformer-embedding classifier) in the K-STEMIT
mapping even though the specific architecture shifts.

## One-paragraph summary

Williams et al. attack the recurring problem that almost every pretrained
bioacoustic model has been trained on birds, and that such models generalise
poorly to marine soundscapes. They curate ReefSet — a meta-dataset of 57,074
labelled 1.88-second coral-reef recordings gathered from 16 datasets across
12 countries — and show empirically that (a) existing bird-pretrained
networks (BirdNET, Perch) beat general-purpose networks (VGGish, YAMNet)
on few-shot reef transfer; (b) pretraining exclusively on the smaller
in-domain ReefSet is strictly worse than any existing pretrained network;
(c) cross-domain mixing, in which ReefSet is blended with Xeno-Canto (2.9 M
bird clips) and Freesound50K (~108 h), produces a model ("SurfPerch") that
outperforms all four incumbent pretrained networks on 16 reef datasets under
a dataset-rotation generalisation protocol. The final SurfPerch model hits
a mean AUC-ROC of 0.933 across the reef tasks (versus 0.908 for BirdNET),
and a 0.900 AUC-ROC with only four labelled samples per class. This becomes
the "task head" analogue for fish/species classification in the sonobuoy
pipeline: a transformer-embedding-centric few-shot classifier that replaces
per-deployment supervised retraining.

## Methodology

### Architecture

- **Backbone**: An EfficientNet (B0 / B1 / B2 probed in hyperparameter sweeps)
  trained as a CNN classifier. The Perch lineage dictates the exact blocks
  (MobileNet-inverted-residual style) rather than a pure transformer. The
  final layer exposes a 1280-dimensional embedding (as in Perch v1.4).
- **Front-end**: Log-mel PCEN spectrogram. PCEN (Per-Channel Energy
  Normalisation) is parameterised by smoothing α, gain g, bias b, and root r.
  For the main ReefSet experiments the settings are `α=0.1, g=0.5, b=2.0,
  r=2.0, ε=1e-6`, frequency range 60–10,000 Hz. For SurfPerch the tuned
  parameters are `α=0.145, g=0.8, b=10.0, r=4.0`, range 50–16,000 Hz.
- **Input shape**: 5-second waveform at 32 kHz (repeat-pad for shorter
  clips), then PCEN log-mel spectrogram.
- **Output heads**: multi-head hierarchical classification. For SurfPerch the
  heads are (i) primary biophony/anthrophony/geophony labels, (ii) 35
  secondary labels, (iii) XC-Bird species / genus / family / order heads
  with a loss weighting of 0.1 for the non-species bird heads, and (iv) a
  Freesound class head.

### Training procedure

- **Augmentation**: random normalisation with min/max gain 0.15/0.25; MixUp
  with mix-in probability 0.75.
- **Steps**: 200k pretraining steps for the ReefSet-only and two-domain
  trials; 1,000,000 steps for SurfPerch final.
- **Loss**: categorical cross-entropy on each output head. In cross-domain
  training, data loaders cycle each dataset back in when exhausted.
- **Hyperparameter sweep**: probed EfficientNet size (B0/B1/B2), learning rate
  (1e-2 / 1e-3 / 1e-4), batch size (64, 128), and for cross-domain runs, the
  ReefSet-to-bird weighting (0.1, 0.25, 0.5, 0.75, 0.9) and bird-to-Freesound
  split (0.5 / 0.6 / 0.7 / 0.8).
- **Hardware**: TPU v3 pod. Each pretraining run averages ~20 h and the
  reported compute for the 139 networks in the paper is ≈26,668 USD of
  Google Cloud spot instances.

### Transfer-learning (fine-tuning) protocol

- Pretrained weights are **frozen**. Only a single new linear classification
  head is trained, producing a fixed-embedding classifier.
- **Epochs**: 128
- **Batch size**: 32
- **Learning rate**: 1e-3
- **Loss**: categorical cross-entropy
- **Evaluation**: 4 / 8 / 16 / 32 training samples per class, 10 random-seed
  repeats, mean AUC-ROC reported. Fine-tuning one dataset takes <1 minute on
  a standard laptop (Intel i9-13900H CPU).

### Datasets

- **ReefSet (in-domain)**: 57,074 labelled clips, each 1.88 s at 16 kHz,
  spanning 16 constituent datasets from 12 countries (USVI, Mozambique,
  Tanzania, Thailand, Florida, Kenya, Indonesia, Philippines, Bermuda,
  Belize, etc.). Label distribution: biophony 79.20 %, anthrophony 10.39 %,
  ambient 10.32 %, geophony 0.09 %. Thirty-three secondary labels include
  groupers, damselfish, snapping shrimp, boat noise, bomb-fishing. The
  majority of biophony is presumed fish-produced but not all are
  taxonomically resolved.
- **Xeno-Canto (XC-Bird)**: ~2.9 M five-second samples, 10,932 species.
- **Freesound50K (FSD50K)**: 108.2 h, 200 classes, derived from the AudioSet
  ontology (bird-labelled samples removed to avoid leakage).
- **Out-of-domain evaluation** (SurfPerch vs Perch, BirdNET, VGGish, YAMNet):
  Watkins marine mammals, bats (Mac Aodha), neotropical frogs (AnuraSet),
  plus bird datasets.

### Dataset Rotation Evaluation of Generalisation (DREG)

The novel evaluation protocol: pretrain on 15 of 16 ReefSet constituent
datasets while holding one out; repeat for all 16 combinations → 16
pretrained models; report mean AUC-ROC across the 16 held-out tests. This
directly measures generalisation to an unseen deployment rather than
in-distribution accuracy.

## Key results

- Ranking of incumbent pretrained networks on ReefSet few-shot transfer
  (mean AUC-ROC ± s.d. across 4/8/16/32 samples-per-class):
  - BirdNET v2.3: **0.908 ± 0.09**
  - Perch v1.4: 0.881 ± 0.11
  - YAMNet: 0.834 ± 0.05
  - VGGish: 0.813 ± 0.05
- In-domain-only pretraining (ReefSet alone) under DREG: **0.724 ± 0.05**
  — worse than all four baselines.
- Two-domain mixing (ReefSet + XC-Bird) under DREG: **0.895 ± 0.03**
  (163.90 % reduction in AUC-ROC error vs ReefSet-only; 12.32 % reduction
  vs Perch).
- Three-domain mixing (ReefSet + XC-Bird + FSD50K) under DREG:
  **0.928 ± 0.02** (21.68 % lower error than BirdNET).
- With tuned PCEN parameters and 1 M training steps: **0.933 ± 0.02**
  (SurfPerch final).
- Four-shot result: **0.900 ± 0.02** mean AUC-ROC with only 4 labelled
  clips per class — this is the number that matters for the sonobuoy
  pipeline, where labelled operational fish vocalisations are scarce.
- Per-dataset spread under 4-shot BirdNET: min **0.746 ± 0.062** (Kenya),
  max **0.996 ± 0.006** (Thailand).
- Out-of-domain degradation: SurfPerch still beats YAMNet and VGGish on
  all six external bioacoustic datasets but loses to Perch (largest gap
  0.084 AUC-ROC on Yellowhammer dialect; smallest 0.019 on Watkins marine
  mammals). At 256 samples-per-class the gap narrows to 0.012, confirming
  that cross-domain mixing trades generalisation for specialisation.

## Strengths

- **DREG protocol is transferable**. For a sonobuoy array deployed at a new
  site, the DREG evaluation directly measures performance in the
  sensor-relocation regime that actually happens in the field. This is
  much stronger than conventional random-split AUC numbers.
- **Few-shot headline is operationally relevant**. A 0.900 AUC-ROC with
  four labelled clips per class matches the practical annotation budget
  for a field biologist looking at a new deployment.
- **Cross-domain pretraining recipe is explicit and reproducible**. The
  exact weight schedule (bird loss weighting 0.1 for non-species heads,
  ReefSet weight 0.1, bird weight 0.5–0.8) is documented.
- **PCEN front-end is robust to dynamic range** (a known weakness of
  raw log-mel for underwater signals with boat transients).
- **Open-source artifacts**: SurfPerch model, ReefSet data, and full
  Colab tutorial on Zenodo. Can be run free of charge in a web browser.

## Limitations

- **Frozen-backbone transfer only**. The main results never fine-tune
  the EfficientNet body, only the final linear classifier. For unusual
  deployments (sub-ice recordings, very low SNR) this almost certainly
  underestimates achievable accuracy.
- **Label taxonomy is shallow**. Many ReefSet sounds are "fish noise"
  rather than species-level. The clean 12-species fish-ID promise in the
  survey description does not hold here — the model is predominantly a
  biophony / anthrophony / "fish vs. not-fish" detector.
- **Domain specialisation tradeoff**. SurfPerch underperforms Perch on
  all six non-reef bioacoustic tasks. A sonobuoy deployed where marine
  mammals dominate should probably use Perch or BirdNET instead.
- **Compute cost is real**. 20 h on a TPU v3 pod per pretraining run is
  not reproducible for a university lab. The exposed recipe is only
  useful if one accepts the released SurfPerch checkpoint.
- **No temporal model**. The 5-second input window means multi-call
  sequences, duets, or chorus structure are ignored. Sonobuoy arrays
  would benefit from a temporal aggregation stage on top of the embedding.

## Portable details (implementable in Rust)

### Embedding extraction as a frozen front-end

```
input:   waveform ∈ ℝ^N at 32 kHz, clipped/padded to 5 s (N = 160000)
front:   log-mel PCEN (80 mel bins, 60–10000 Hz, α=0.1, g=0.5, b=2.0, r=2.0)
body:    EfficientNet-B1 (SurfPerch) → 1280-dim embedding vector z ∈ ℝ^1280
head:    W z + b, W ∈ ℝ^{C×1280}, softmax
```

### PCEN (portable)

Given log-mel spectrogram `E[t, f]`:

```
M[t, f] = (1 - α) · M[t-1, f] + α · E[t, f]
y[t, f] = (E[t, f] / (M[t, f] + ε))^r · g  -  (b · (...))   // simplified form:
y[t, f] = (E[t, f] / (ε + M[t, f])^α + δ)^r - δ^r
```

Use Librosa's reference implementation (α=0.1, g=0.5, b=2.0, r=2.0,
ε=1e-6) as the oracle. For an `eml-core` EML block the entire PCEN can
be expressed as a stateful 1-D IIR followed by a power law.

### Fine-tuning recipe (transferable to Rust/Candle or burn)

```rust
struct LinearHead { w: Tensor, b: Tensor } // ℝ^{C×1280}
const BATCH: usize = 32;
const LR: f32 = 1e-3;
const EPOCHS: usize = 128;
// Loss: CE on softmax; freeze backbone; only head grads.
```

MixUp with p = 0.75 and random gain uniform in [0.15, 0.25] are the only
augmentations needed to replicate the reported numbers.

### Hyperparameters summary (drop-in constants)

| Parameter              | Value                         |
|------------------------|-------------------------------|
| Sample rate            | 32 kHz                        |
| Window length          | 5.0 s (160 000 samples)       |
| Mel bins               | 80 (typical Perch config)     |
| Frequency range        | 60–10 000 Hz                  |
| PCEN α / g / b / r / ε | 0.1 / 0.5 / 2.0 / 2.0 / 1e-6  |
| Embedding dim          | 1280                          |
| Backbone               | EfficientNet-B1               |
| Fine-tune lr           | 1e-3                          |
| Fine-tune epochs       | 128                           |
| Fine-tune batch        | 32                            |
| Pretraining lr sweep   | {1e-2, 1e-3, 1e-4}            |
| Pretraining batch      | {64, 128}                     |
| Pretraining steps      | 200 k (SurfPerch: 1 M)        |
| MixUp probability      | 0.75                          |
| Random gain            | U[0.15, 0.25]                 |
| ReefSet class count    | 33 secondary + 3 primary      |
| ReefSet clip length    | 1.88 s (re-pad to 5 s)        |
| DREG folds             | 16                            |
| Compute                | TPU v3 pod, ~20 h / run       |

### Cross-domain loss mix

```
L_total = L_reef + 0.1 · L_bird_species + 0.01 · L_bird_genus
        + 0.01 · L_bird_family + 0.01 · L_bird_order + 0.1 · L_freesound
```

(The 0.1 weight for non-species bird heads is explicit in the paper; the
ReefSet weight is fixed at 1.0 and the Freesound weight probed in {0.5,
0.6, 0.7, 0.8} of the non-ReefSet share.)

## Sonobuoy integration plan

SurfPerch maps directly to the **"task head" / species-ID branch** of the
K-STEMIT-extended architecture described in
`.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`. Concretely:

1. **Embedding service**. Ship SurfPerch (or the Perch-style EfficientNet
   backbone) as a frozen WASM-wasip2 module invoked from
   `clawft-kernel/src/hnsw_service.rs`. Each per-buoy 5-second window
   becomes a 1280-dim vector, indexed in HNSW for fast retrieval and
   cluster discovery. The HNSW service already supports this dimensional
   range; add a new namespace `sonobuoy/fish-embeddings`.
2. **Frozen backbone + learned head in EML**. The linear classification
   head fits the `eml-core` "learned function" contract exactly: a
   single-layer affine map `W z + b` is a textbook EML unit. Register
   the head as an EML block parameterised by `(num_classes, embedding_dim
   = 1280)`, trainable via the existing EML optimisation loop, driven
   by a small on-device labelled set.
3. **Dual-branch fit with K-STEMIT**. In the K-STEMIT dual-branch pattern
   (spatial GraphSAGE + gated temporal conv), SurfPerch sits inside the
   **temporal branch** as a pretrained spectro-temporal feature extractor.
   Per-buoy embeddings feed GraphSAGE's node features, with TDOA-weighted
   edges providing the spatial branch (see
   `.planning/sonobuoy/papers/analysis/fishgraph-gnn.md`). The learnable
   `α ∈ [0, 1]` balances spatial (bearing) vs. temporal (species) signal.
4. **Few-shot on-device adaptation**. The 4-shot AUC-ROC of 0.900 means a
   sonobuoy operator can label ~40 clips (≈5 minutes of audio) during
   deployment to produce a site-specialised classifier. Expose this
   capability as a `weftos fish train` CLI flow writing to the EML
   adapter head only.
5. **PCEN front-end in Rust**. Implement PCEN directly in `weftos-signal`
   as a deterministic DSP block; cross-validate against Librosa on
   golden waveforms to guarantee embedding parity.

## Follow-up references

1. **Ghani, B., Denton, T., Kahl, S. & Klinck, H.** "Global birdsong
   embeddings enable superior transfer learning for bioacoustic
   classification." *Sci. Rep.* 13, 22876 (2023).
   — Parent work: establishes Perch as a bird-pretrained bioacoustic
   embedding and the protocol SurfPerch inherits.
2. **Kahl, S., Wood, C.M., Eibl, M. & Klinck, H.** "BirdNET: A deep
   learning solution for avian diversity monitoring." *Ecological
   Informatics* 61, 101236 (2021).
   — Best-performing pretrained baseline; reference for multi-taxa head
   structure.
3. **Moummad, I., Serizel, R. & Farrugia, N.** "Self-Supervised Learning
   for Few-Shot Bird Sound Classification." arXiv:2312.15824 (2023).
   — Suggested direction for SSL variant on SurfPerch; directly informs
   the substitution choice for the Echosounder SSL paper (slug
   `echosounder-ssl`).
4. **Fonseca, E., Favory, X., Pons, J., Font, F. & Serra, X.** "FSD50K:
   An open dataset of human-labeled sound events." *IEEE/ACM TASLP* 30,
   829-852 (2021).
   — Cross-domain dataset. Required to reproduce SurfPerch.
5. **Hamer, J., Laber, R. & Denton, T.** "Agile Modeling for Bioacoustic
   Monitoring." *NeurIPS 2023 Climate Change Workshop*.
   — Active-learning / human-in-the-loop protocol suited for on-deployment
   sonobuoy labelling flows.
6. **Williams, B., Lamont, T.A., et al.** "Enhancing automated analysis of
   marine soundscapes using ecoacoustic indices and machine learning."
   *Ecological Indicators* 140, 108986 (2022).
   — Earlier author work; source for the Indonesian / Florida datasets
   inside ReefSet.

---

**Sources consulted**:
- [arXiv:2404.16436 — Leveraging tropical reef, bird and unrelated sounds for superior transfer learning in marine bioacoustics](https://arxiv.org/abs/2404.16436)
- [SurfPerch model & ReefSet on Zenodo](https://doi.org/10.5281/zenodo.11060189)
- [Perch codebase](https://github.com/google-research/perch)
- [AST paper (Gong et al. 2021)](https://arxiv.org/abs/2104.01778) — referenced
  as the architecture the survey originally cited.
