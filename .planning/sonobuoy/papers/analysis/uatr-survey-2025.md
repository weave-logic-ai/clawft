# UATR Deep Learning Survey → Feng et al. 2024 (Substituted)

## Citation header

- **Original survey claim**: "Luo, Chen et al. *Comprehensive survey covering
  2019-2024 UATR literature with taxonomy of time-domain vs time-frequency vs
  graph-based approaches. Flags DeepShip, ShipsEar, OceanShip as canonical
  benchmarks.* arXiv:2503.01718, 2025."
- **Verified paper**: *Artificial Intelligence-Based Underwater Acoustic Target
  Recognition: A Survey.*
- **Authors**: Sheng Feng, Shuqing Ma, Xiaoqian Zhu, Ming Yan. National
  University of Defense Technology, Changsha (College of Computer Science;
  College of Meteorology and Oceanography).
- **Venue**: *Remote Sensing* (MDPI), volume 16, issue 17, article 3333.
- **Year**: 2024 (published 8 Sept 2024).
- **DOI**: [10.3390/rs16173333](https://doi.org/10.3390/rs16173333).
- **URL**: <https://www.mdpi.com/2072-4292/16/17/3333>.
- **PDF**: Download failed. MDPI returned 404 for the direct PDF endpoint
  under both `www.mdpi.com/2072-4292/16/17/3333/pdf` and
  `pub.mdpi-res.com/2072-4292/16/17/3333/pdf`. Likely anti-bot; manual
  browser download should succeed. HTML-extracted content below.

## Status

**Substituted** (from "Luo, Chen et al., arXiv:2503.01718, 2025" to "Feng, Ma,
Zhu, Yan, *Remote Sensing* 16(17):3333, 2024").

### Substitution rationale

- arXiv:2503.01718 is a **real** paper but it is *"Learning surrogate
  equations for the analysis of an agent-based cancer model"* by Burrage,
  Burrage, Kreikemeyer, Uhrmacher, Weerasinghe (2025), published at
  Frontiers in Applied Mathematics. Nothing to do with UATR.
- Exhaustive arXiv title/abstract searches (`ti:"underwater acoustic target
  recognition" AND ti:survey`, `abs:UATR AND abs:survey`, etc.) returned
  zero UATR survey papers on arXiv in 2024 or 2025.
- The closest actual UATR survey matching the description ("comprehensive,
  taxonomy of time-domain vs time-frequency, canonical benchmarks") is
  **Feng et al. 2024** in MDPI *Remote Sensing*. A secondary candidate
  ("Navigating the Depths: A Comprehensive Survey of Deep Learning for
  Passive Underwater Acoustic Target Recognition", IEEE Access 12, 2024)
  exists, but Feng et al. is the more heavily cited and more
  taxonomy-structured of the two; I substitute to it.
- Note: the survey description mentions "graph-based" approaches and
  "OceanShip" as benchmarks. Feng et al. covers time-domain and
  time-frequency extensively but does **not** feature a separate
  graph-based branch of the taxonomy; it also does not emphasise OceanShip.
  So the survey citation is an imperfect match — but it is the closest
  *real* UATR survey.
- **No PDF** could be auto-downloaded; analysis below is based on the HTML
  of the published article and the R Discovery abstract mirror.

---

## One-paragraph summary

Feng et al. (2024) survey the AI-based UATR literature from the signal
processing era through the modern deep learning and transformer epoch. They
organise the field along two orthogonal axes — *feature extraction* and
*classifier family* — and catalogue representative works in each cell of
that matrix. Feature extraction divides into (1) *physical-significance
features* (LOFAR, DEMON), (2) *joint time-frequency features* (STFT,
wavelets, HHT, WVD, plus auditory-perceptual features MFCC/Mel-Fbank/GFCC,
and multidimensional fusion at frontend or backend), and (3) *autoencoded
features* (AE, RBM, and self-supervised learning). Classifiers divide into
*machine learning* (SVM, KNN, HMM, GMM, DT, RF, LR) and *deep learning*
(RNN, CNN, attention networks, transformers, GANs). They flag ShipsEar,
DeepShip (with an explicit note about random-vs-causal partitioning bias),
UCI Sonar, and SUBECO as the key benchmarks. The closing chapter is the
most useful part: it names four cross-cutting research challenges that
sonobuoy-style distributed systems will definitely hit — complex propagation
conditions, interpretability deficits, weak cross-environment
generalisation, and adversarial fragility — and argues that current
transformer-based models are the leading accuracy baseline but none of
them address these four challenges head-on.

---

## Methodology

Literature review, not an empirical paper. The authors:

1. **Scoped**: AI-driven UATR literature, emphasis on the 2015-2024 window
   with historical coverage back to 1990s SVM/GMM classifiers.
2. **Organised** along the feature × classifier matrix described above.
3. **Compared** representative methods on ShipsEar and DeepShip, reporting
   their original authors' numbers (not re-running experiments).
4. **Called out** four research challenges and speculated on future
   directions (interpretability, adversarial robustness, transformer
   scaling, physics-informed learning).

The article structure (Sections 1-5):

- **1. Introduction**: UATR fundamentals, the receive-feature-classify
  workflow, and the motivation that AI-based UATR solves problems
  traditional beam-/filter-based approaches cannot.
- **2. Feature Extraction Methods**: physical-significance → time-frequency
  → autoencoding; also covers frontend-fusion vs. backend-fusion of
  multidimensional features.
- **3. Machine Learning Methods**: distance-based, probabilistic,
  tree-based, regression.
- **4. Deep Learning Methods**: RNN (LSTM, GRU, CNN-LSTM/GRU hybrids); CNN
  (ResNet, DenseNet, MobileNet, Xception, with depth-wise/dilated/Inception
  blocks); Attention (CAM, SAM, bidirectional); Transformer (UATR-Tx,
  Spectrogram Transformer Model [STM], Swin-based TFST, MobileViT,
  SSL-Transformers); GAN for few-shot augmentation.
- **5. Challenges and Future Prospects**: complex conditions,
  interpretability, generalisation, robustness.

---

## Key results (verbatim from HTML extraction)

The paper does not contribute new empirical results. It cites third-party
accuracy numbers; the ones that matter for sonobuoy planning:

| Method (cited) | Features | Dataset | Accuracy |
|----------------|----------|---------|----------|
| SVM | Multidimensional fusion | 4-class UATR | 97.0 % |
| KNN | Refined composite multiscale dispersion entropy | 4-class | 96.25 % |
| CNN-LSTM | Time-frequency | Real ocean data | 95.2 % |
| Random Forest | Spectral ridge | Whale calls, 4-class | 99.69 % |
| LOFAR+DEMON fusion | Comb-filtered combined | Ship estimation | 98 % |

These numbers are cherry-picked from each paper's best setting and do not
use a uniform test split, so they overstate the realistic state of the art.
The more reliable numbers are those from papers that release splits (like
DEMONet and the Smoothness-UATR paper analysed alongside this one), which
sit around 80 % on DeepShip and 85 % on ShipsEar for 9-class recognition.

### Datasets highlighted

- **ShipsEar** (Santos-Domínguez 2016): 90 recordings, 9 canonical class
  split used in most reported numbers.
- **DeepShip** (Irfan 2021): 613 recordings, 4-class (Cargo, Passenger,
  Tanker, Tug). Survey explicitly flags that *random* vs. *causal* (track-
  disjoint) partitioning produces hugely different accuracies — a known
  information-leakage trap that many earlier reported numbers fall into.
- **UCI Sonar**: classical baseline, no longer representative.
- **SUBECO**: marine acoustic recordings, less-used.
- The survey mentions proprietary datasets from South China Sea and the
  Norwegian Institute of Marine Research but does not discuss **OceanShip**
  (which the original survey description flagged).

### Four research challenges (Section 5)

1. **Complex recognition conditions**: multipath, attenuation, reverberation,
   variable acoustic channels. Current DL models assume a stationary
   distribution that does not hold at sea.
2. **Poor interpretability**: post-hoc explanation of a ResNet classifier's
   decision is not enough for navy/surveillance deployment.
3. **Weak cross-environment generalisation**: models trained in one ocean
   basin frequently fail in another.
4. **Adversarial fragility**: UATR models are easily perturbed. No UATR
   paper to date explicitly tests adversarial robustness at scale.

The survey argues (1) and (3) are the most urgent and least-studied.

---

## Strengths

- **Clean two-axis taxonomy.** The feature × classifier grid is a genuinely
  useful mental model for navigating the field; it is the main reason to
  read this survey.
- **Honest treatment of DeepShip split bias.** The authors explicitly
  discuss random-vs-causal partitioning and note that most reported numbers
  are inflated by track-level leakage. This is the single most important
  methodological point for anyone benchmarking UATR.
- **Broad temporal coverage.** Covers the 1990s SVM/GMM era through the
  2023-2024 transformer epoch, so newcomers can see the arc without
  reading 150 primary papers.
- **Explicit research-challenges section** (5) that maps directly onto the
  sonobuoy project's risk register (environmental generalisation,
  interpretability, adversarial robustness all matter for a deployed
  distributed array).
- **Open-access MDPI.** Free to read, cite, and reproduce figures.

## Limitations

- **No graph-based branch.** The original sonobuoy survey description
  claimed the paper covers "time-domain vs time-frequency vs graph-based"
  approaches; Feng et al. does not have a graph-based taxonomy branch, so
  the mapping to K-STEMIT (a GNN) must come from the K-STEMIT paper
  itself, not from this survey.
- **Accuracy numbers are not normalised.** Cross-paper comparisons use
  different splits, different SNRs, and different evaluation units
  (segment-level vs. file-level). Survey does not re-benchmark; numbers are
  cited as-is.
- **Weak on data scarcity / few-shot learning.** Mentions GANs briefly but
  does not survey the contrastive / self-supervised pretraining literature
  that has exploded post-2022 (Data2Vec, BEATs, WavLM, underwater-adapted
  variants).
- **Missing OceanShip and QiandaoEar22.** Two of the more recent
  large-scale UATR benchmarks do not appear in the dataset discussion.
- **Limited quantitative depth on specific architectures.** For each
  architecture family the survey gives a one-paragraph summary without
  layer counts, parameter budgets, or training recipes — you need the
  primary papers to implement anything.
- **No discussion of deployment constraints.** Power, memory, and latency
  budgets for on-buoy inference are never mentioned, despite being the
  dominant engineering constraints for sonobuoy systems.

---

## Portable details (for implementation planning)

### Taxonomy as a Rust enum (useful for `weftos-sonobuoy-catalog`)

```rust
pub enum UatrFeature {
    PhysicalSignificance(PhysFeat),
    TimeFrequency(TfFeat),
    AuditoryPerceptual(AudFeat),
    Autoencoded(AeFeat),
    MultiFusion(FusionKind),
}

pub enum PhysFeat {
    Lofar,       // Low-Frequency Analysis Recording
    Demon,       // Detection of Envelope Modulation on Noise
    Cepstrum,    // Passive cepstral track
}

pub enum TfFeat {
    Stft(f32 /* window_s */),
    Wavelet(WaveletKind),
    Cwt,
    HilbertHuang,
    WignerVille,
    Cqt { octave_res: u32 },
}

pub enum AudFeat { Mfcc, MelFbank, Gfcc, Bark }

pub enum AeFeat { Autoencoder, RestrictedBoltzmann, SslContrastive, SslGenerative }

pub enum FusionKind { Frontend /* concat features */, Backend /* ensemble */ }

pub enum UatrClassifier {
    Classical(ClassicalKind),
    DeepLearning(DlKind),
}

pub enum ClassicalKind { Svm, Knn, Hmm, Gmm, DecisionTree, RandomForest, LogReg }

pub enum DlKind {
    Rnn(RnnKind),    // LSTM / GRU / CNN-hybrid
    Cnn(CnnKind),    // ResNet / DenseNet / MobileNet / Xception
    Attention(AttnKind), // CAM / SAM / bidirectional
    Transformer(TxKind), // UatrTx / STM / SwinTfst / MobileViT / SslTx
    Gan,             // few-shot augmentation
}
```

This enum hierarchy is a direct map of the survey's taxonomy and lets
downstream code (the EML learned-function registry, the K-STEMIT configurator)
reason uniformly about which feature-classifier combinations it has
implemented.

### Benchmark hygiene checklist (lifted from the survey's challenges section)

- Always use *causal* / track-disjoint splits on DeepShip, never random.
- Report segment-level accuracy and variance across seeds, not file-level.
- Publish the exact split (by recording ID) so others can reproduce.
- Include at least one low-SNR evaluation (paper suggests -15 to 0 dB).
- Report per-class accuracy as well as overall — ShipsEar's 9-class split is
  heavily imbalanced.

### Recommended benchmarks for `weftos-sonobuoy` CI

Based on this survey's dataset discussion and the absent-in-survey
OceanShip / QiandaoEar22 literature:

| Dataset | Role | Source |
|---------|------|--------|
| DeepShip | Primary, large-scale | github.com/irfankamboh/DeepShip |
| ShipsEar | Small-data stress test | atlanttic.uvigo.es/underwaternoise |
| OceanShip | Large, diverse (15 classes, 121 h) | arXiv:2401.02099 |
| QiandaoEar22 | Specific-ship ID, 22 classes | arXiv:2406.04353 |
| UCI Sonar | Sanity check / smoke test | UCI ML repo |

The survey covers the first two; the last three come from adjacent 2024
literature that the survey missed.

---

## Sonobuoy integration plan

Feng et al. 2024 is a **scoping document**, not an implementation target.
Its contribution to the sonobuoy project is:

1. **Architecture registry**: use the taxonomy above to structure the
   EML-core learned-function registry so that every new UATR component
   (DEMONet, SIR, K-STEMIT) is tagged by `(UatrFeature, UatrClassifier)`
   and discoverable by task planners.
2. **ADR-053 (Spatio-Temporal Dual-Branch Architecture for Sensor Systems)
   risk register**: directly import Feng's four challenges as risk items.
   Mitigations for each:
   - *Complex conditions* → physics-informed node features (K-STEMIT
     approach, sound-speed-profile injection into
     `weftos-sonobuoy-propagation`).
   - *Interpretability* → EML-core's learned-function introspection plus
     routing-layer inspection (DEMONet's routing assignments are directly
     human-readable by physics regime).
   - *Cross-environment generalisation* → SIR + LMR training regularisers
     from the sister paper; cross-basin evaluation in CI.
   - *Adversarial robustness* → P2 research item; the quantum-register
     symmetric-divergence extension mentioned in the smoothness-UATR
     analysis is a natural fit here.
3. **Benchmark harness**: `weftos-sonobuoy-bench` crate that reproduces the
   survey's cited accuracy numbers (with corrected splits) and gates CI
   on regression from a documented baseline. Use both DeepShip (primary)
   and ShipsEar (data-scarcity regression detector).
4. **Priority**: P2 for the survey itself — it is documentation, not
   code. P0 for the taxonomy enum (it unblocks consistent labelling across
   the EML registry) and P1 for the benchmark harness.
5. **Integration with `clawft-kernel/src/hnsw_service.rs`**: use
   feature-classifier pair tags as HNSW metadata to support queries like
   "find all stored embeddings produced by a DEMON-routed + transformer
   classifier trained on DeepShip".
6. **Integration with `clawft-kernel/src/quantum_register.rs`**: the
   classifier-family enum (4 DL kinds) maps naturally onto a 2-qubit
   register for downstream cognitive-layer reasoning about which model
   family produced a given prediction — useful when ensembling outputs
   from heterogeneous recognisers.

---

## Follow-up references (cited works that matter)

1. **Irfan, M. et al. 2021** — *DeepShip: An underwater acoustic benchmark
   dataset and a separable convolution based autoencoder for
   classification.* Expert Systems with Applications 183, 115270. The key
   benchmark in the survey; used by both the DEMONet and Smoothness-UATR
   papers analysed alongside this one.
2. **Santos-Domínguez, D. et al. 2016** — *ShipsEar: An underwater vessel
   noise database.* Applied Acoustics 113, 64-69. The small-data benchmark.
3. **Chen, Y. et al. 2023 (UATR-Transformer)** — the survey's marquee
   transformer example. First paper to apply pure Transformer
   encoder-decoder to UATR spectrograms.
4. **Liu, C. et al. 2022 (Swin-TFST)** — Swin-Transformer adapted to UATR.
   Referenced as the best reported accuracy in the transformer branch of
   the taxonomy.
5. **Goodfellow, I. et al. 2014 + Zhu et al. 2022 (GAN augmentation for
   UATR)** — the survey's justification for GAN augmentation under data
   scarcity; relevant if sonobuoy field data collection is expensive.

### Additional post-survey references worth chasing

- **Xie, Y. et al. 2024** — *DEMONet: Underwater Acoustic Target Recognition
  based on Multi-Expert Network and Cross-Temporal Variational Autoencoder.*
  arXiv:2411.02758. Published *after* this survey; fills a gap the survey
  flags (using physical features as routing signals).
- **Xu, J., Xie, Y., Wang, W. 2023** — *Underwater Acoustic Target
  Recognition based on Smoothness-inducing Regularization and
  Spectrogram-based Data Augmentation.* Ocean Engineering 281, 114926.
  Addresses the survey's "weak cross-environment generalisation" challenge.
- **Sun, Y. et al. 2024 (OceanShip)** — arXiv:2401.02099. 121 h, 15-class,
  large-scale benchmark missing from Feng et al. Critical for modern UATR
  benchmarking.
- **2024 (QiandaoEar22)** — arXiv:2406.04353. Specific-ship ID under
  multi-target interference; complements DeepShip.
