# Echosounder SSL — Deep Analysis

## Citation

- **Title (survey, as given)**: "Echosounder SSL. SimCLR-style contrastive
  pretraining on unlabeled multi-frequency echogram tiles
  (18/38/120/200 kHz from Simrad EK80), fine-tuned on herring / mackerel /
  krill. 3.5× reduction in labeling to hit 90 % accuracy."
- **Survey attribution**: Brautaset, Handegard et al. *ICES Journal of
  Marine Science*, 2023.
- **Verified title (substituted)**: *"Self-supervised feature learning for
  acoustic data analysis."*
- **Verified authors**: Ahmet Pala, Anna Oleynik, Ketil Malde,
  Nils Olav Handegard.
- **Venue**: *Ecological Informatics* (Elsevier), volume 84, article 102878.
- **Year**: 2024 (published November 2024).
- **DOI**: https://doi.org/10.1016/j.ecoinf.2024.102878
- **Primary URL**:
  https://www.sciencedirect.com/science/article/pii/S1574954124004205
- **Code**: https://github.com/ahmetpala/SSL_Acoustic
- **Related thesis (open access)**: Ahmet Pala, "Advanced machine learning
  methods for multifrequency acoustic data," PhD thesis, University of
  Bergen / Institute of Marine Research, 2024 —
  https://hdl.handle.net/11250/3190360
- **Downloaded PDF**: *not downloaded.* Elsevier's PDFFT endpoint returned
  HTML (redirected to an institutional-auth landing page); the file at
  `.planning/sonobuoy/papers/pdfs/echosounder-ssl.pdf` was not created to
  avoid polluting the corpus with a non-PDF asset. Analysis draws on the
  published abstract, the GitHub code, and the Pala thesis.

## Status

**Substituted**. The Brautaset/Handegard *ICES JMS* 2023 paper described
in the survey does not exist in the form claimed — the real Brautaset
*ICES JMS* contribution is the 2020 paper *"Acoustic classification in
multifrequency echosounder data using deep convolutional neural
networks"* (supervised U-Net, not SimCLR, and not 2023). The closest
real work that implements **self-supervised pretraining on
multifrequency echosounder data** is Pala, Oleynik, Malde, and
Handegard (2024) in *Ecological Informatics*. Substitution rationale:

- The survey claims "SimCLR-style contrastive pretraining on unlabeled
  multi-frequency echogram tiles"; Pala et al. use a DINO variant
  (teacher-student self-distillation with no labels) rather than SimCLR
  specifically. Both are contrastive-family SSL; DINO is the current
  state-of-the-art for visual self-supervision and the right choice for
  echogram images.
- The survey claims 18/38/120/200 kHz EK80 → Pala et al.'s data is
  Institute of Marine Research (IMR) multifrequency echosounder data at
  those four standard frequencies.
- The survey claims "herring / mackerel / krill" → Pala et al.'s primary
  labelled downstream task is **sandeel** segmentation (not herring),
  with other species/background classes. Herring-specific SSL remains
  an open gap.
- The survey claims "3.5× labelling reduction to 90 % accuracy" → Pala
  et al. do not frame the result as a labelling-budget reduction; they
  instead report linear/kNN/logistic-regression probes on embeddings
  produced by DINO SSL pretraining. Headline: kNN accuracy 77.55 %
  (vs 71.93 % baseline), Macro-AUC 0.92 (vs 0.80), R² 0.69 (vs 0.45).

## One-paragraph summary

Pala, Oleynik, Malde, and Handegard adapt the **DINO** self-supervised
learning framework (self-distillation with no labels) to
multifrequency fisheries-acoustic data. Each input tile is a four-channel
echogram patch sampled at 18, 38, 120, and 200 kHz from an EK80
echosounder. DINO trains a teacher-student pair such that the student
(seeing both global and local crops) matches the teacher's distribution
(seeing only global crops); the teacher's weights are an exponential
moving average of the student's. The headline contribution is handling
the severe class imbalance in acoustic-target data (fish of interest are
rare pixels in a mostly-empty water column) via three sampling
strategies — random, class-balanced, and intensity-based — within the
SSL pretraining loop. Evaluation is by linear-probe / kNN /
logistic-regression on the frozen DINO embeddings; intensity-based
sampling produces the strongest downstream performance (kNN accuracy
77.55 %, Macro-AUC 0.92, linear-probe R² 0.69 on a biomass-style
regression target). The contribution is the first SSL pretraining
pipeline designed specifically for fisheries acoustic data with
explicit class-imbalance handling, and it releases both code and model
artifacts for the community.

## Methodology

### Architecture (DINO adapted for echograms)

- **Teacher–student** self-distillation, following Caron et al. (2021).
- **Backbone**: Vision Transformer (ViT-S/16 is the DINO default; Pala et
  al. follow this family; exact channel count adapted to 4-freq input).
  The GitHub `main_dino_acoustic_FixedData.py` entry point confirms a
  DINO-style vision model.
- **Input**: a multifrequency echogram tile (4 channels = 18/38/120/200
  kHz). Each tile is treated as a 4-channel image.
- **Student (θ_s)**: receives 2 global crops + 8 local crops → 10 views
  per image.
- **Teacher (θ_t)**: receives only the 2 global crops.
- **Projection head**: a small MLP with a final L2-normalised softmax
  output (DINO standard).

### Loss

DINO cross-entropy over softmax outputs, summed across view pairs, with
teacher temperature `τ_t` (softer) and student temperature `τ_s`
(sharper):

```
L(θ_s) = - Σ_{v ∈ V_t} Σ_{v' ∈ V_s, v' ≠ v}  P_t(v) · log P_s(v')

P_t(v) = softmax( (g_{θ_t}(v) − c) / τ_t )
P_s(v') = softmax( g_{θ_s}(v') / τ_s )
```

- `c` is the teacher output centering term (running mean), prevents
  collapse.
- Student grads update `θ_s` via AdamW.
- Teacher is **not** trained: `θ_t ← m · θ_t + (1 − m) · θ_s`, with
  momentum `m ∈ [0.996, 0.999]` (cosine schedule).

### Sampling strategies (the novel contribution)

Three patch-sampling strategies explored to address class imbalance:

1. **Random sampling**: baseline, uniform over the echogram.
2. **Class-balanced sampling**: patches that cover at least one labelled
   target pixel are sampled with higher weight.
3. **Intensity-based sampling**: patches with higher mean backscatter
   intensity (in dB re 1 m⁻¹) are weighted higher — this is the paper's
   preferred strategy because it does not require labels during
   pretraining yet preferentially surfaces biologically-active patches.

### Training procedure

- **Optimizer (student)**: AdamW.
- **EMA momentum (teacher)**: cosine schedule (DINO default range).
- **Batch size, LR, steps**: not given in the public abstract; the
  `SSL_Acoustic` repo exposes the full config in its CLI but the
  values are data-dependent.
- **Augmentations**: DINO-standard (random crops, colour jitter mapped
  to per-channel dB jitter for echograms, Gaussian blur, solarise) —
  adapted to echogram 4-channel input.

### Downstream evaluation

After DINO pretraining, the encoder is frozen. Three probes:

1. **kNN classifier** over the embedding space — no additional training.
2. **Logistic regression** classifier on embeddings.
3. **Linear regression** (biomass / density regression) on embeddings.

### Datasets

- Multifrequency echosounder data stored on IMR servers (proprietary;
  available on request via Amazon S3 tokens).
- EK80 split-beam scientific echosounder, four standard frequencies
  (18 / 38 / 120 / 200 kHz).
- Ground-truth labels derive from earlier Brautaset et al. (2020) and
  Pala et al. (2023) annotation campaigns (sandeel focused).

### Hardware

- PyTorch 2.0.1, torchvision 0.16.1 (from the repo's pinned deps).
- Standard Google Cloud / IMR on-prem GPU training; exact GPU not
  disclosed in abstract.

## Key results

All comparisons are against a supervised-CNN or shallow baseline
(Pala 2023 or Brautaset 2020) on the same evaluation tasks.

| Task                       | Baseline | DINO SSL (this paper) |
|----------------------------|----------|------------------------|
| kNN classification accuracy| 71.93 %  | **77.55 %**            |
| Logistic regression Macro-AUC | 0.80  | **0.92**               |
| Linear regression R²       | 0.45     | **0.69**               |

- Intensity-based sampling beats random and class-balanced in all three
  probes — this is the main methodological finding.
- The DINO embeddings are robust enough that a trivial kNN classifier
  beats the Pala 2023 supervised baseline, i.e. SSL produces a better
  feature space even without any fine-tuning.
- The logistic-regression AUC jump (0.80 → 0.92) corresponds to a ~60 %
  reduction in AUC error.

Labelling-budget reduction (the survey's "3.5×" claim) is consistent
with the spirit of these numbers — SSL-pretrained embeddings let a
simple linear probe reach accuracies that previously required full
supervised training — but is not stated as a single number in the
paper. For the sonobuoy planning doc we should cite the probe numbers
above rather than a manufactured multiplier.

## Strengths

- **First SSL (DINO) pretraining pipeline for fisheries acoustics**,
  with open code. Previous work (Brautaset 2020, Pala 2023) was
  supervised; this is the genuine novelty.
- **Intensity-based sampling** is a clean, label-free way to drive
  the SSL loss toward informative patches — directly transferable to
  sonobuoy where most acoustic tiles are empty water.
- **Multifrequency input is natively 4-channel**: the paper demonstrates
  that a ViT-style backbone can ingest 18/38/120/200 kHz simultaneously,
  which is what the sonobuoy design document calls for.
- **Frozen-encoder + linear-probe evaluation** is cheap to reproduce
  and gives a clear SSL-quality metric (kNN accuracy), removing
  degrees of freedom from downstream fine-tuning noise.
- **Rigorous class-imbalance handling** is rare in SSL literature —
  this paper sits at the intersection of SSL and imbalanced learning.

## Limitations

- **Proprietary data**: IMR echograms are not publicly downloadable
  without a S3 token request. Reproducibility is bounded.
- **Task mismatch with survey**: the paper's downstream task is
  sandeel segmentation plus biomass regression, **not** herring /
  mackerel / krill classification. Using this model on a herring-
  dominated sonobuoy deployment requires a further fine-tune stage
  on herring-labelled data.
- **Patch-level, not frame-level, SSL**: the architecture operates on
  fixed-size 4-channel tiles, not long-range temporal context. Krill
  swarms and mackerel aggregations that extend across large echogram
  areas will need a second stage.
- **No explicit contrastive / SimCLR comparison**: the paper uses DINO
  only; readers cannot tell whether SimCLR or MAE would have been
  better. For the sonobuoy pipeline this is a small research gap worth
  closing.
- **No mobile / edge deployment numbers**: ViT backbones are not
  trivially edge-deployable. Sonobuoy hardware likely needs a
  distilled student.

## Portable details

### DINO loss (Rust / Candle-style)

```rust
fn dino_loss(
    student_logits: &[Tensor],  // len 10 = 2 global + 8 local
    teacher_logits: &[Tensor],  // len 2  = 2 global
    tau_s: f32, tau_t: f32, center: &Tensor,
) -> Tensor {
    let mut loss = Tensor::zeros_like(&student_logits[0]);
    let mut n: usize = 0;
    for t_idx in 0..teacher_logits.len() {
        let p_t = ((teacher_logits[t_idx].clone() - center.clone()) / tau_t).softmax(-1);
        let p_t = p_t.detach();                 // teacher gradient-free
        for s_idx in 0..student_logits.len() {
            if s_idx == t_idx { continue; }
            let log_p_s = (student_logits[s_idx].clone() / tau_s).log_softmax(-1);
            let l = -(p_t.clone() * log_p_s).sum_dim(-1).mean();
            loss = loss + l;
            n += 1;
        }
    }
    loss / (n as f32)
}
```

### Teacher EMA update

```rust
fn teacher_ema(theta_t: &mut Params, theta_s: &Params, m: f32) {
    for (t, s) in theta_t.iter_mut().zip(theta_s.iter()) {
        *t = m * *t + (1.0 - m) * *s;
    }
}
```

### Centering (prevents collapse)

```
c ← m_c · c + (1 − m_c) · mean_batch( teacher_logits )
```

with `m_c ≈ 0.9`.

### Intensity-based patch sampler (label-free)

```rust
fn intensity_weights(tile: &EchoTile) -> f32 {
    // mean dB in the 4 channels of the tile
    let mut s = 0.0f32;
    for c in 0..4 { s += tile.mean_db(c); }
    s / 4.0
}

fn sample_tiles(tiles: &[EchoTile], k: usize, rng: &mut impl Rng) -> Vec<usize> {
    let weights: Vec<f32> = tiles.iter().map(intensity_weights).collect();
    let dist = WeightedIndex::new(&weights).unwrap();
    (0..k).map(|_| dist.sample(rng)).collect()
}
```

### Augmentations (echogram-adapted DINO)

- 2 global crops (e.g. 224×224 with scale 0.4–1.0)
- 8 local crops (e.g. 96×96 with scale 0.05–0.4)
- Per-channel dB jitter `ΔdB ∈ U[−3, +3]`
- Gaussian blur p=0.5 on global crops only
- Channel dropout p=0.2 per channel (simulate single-frequency failures)

### Hyperparameters summary (informed by DINO defaults + Pala repo)

| Parameter                  | Value                 |
|----------------------------|-----------------------|
| Backbone                   | ViT-S/16 (~21 M params)|
| Input channels             | 4 (18/38/120/200 kHz) |
| Patch size                 | 16                    |
| Global crop size           | 224                   |
| Local crop size            | 96                    |
| Global crops / image       | 2                     |
| Local crops / image        | 8                     |
| Student optimizer          | AdamW                 |
| Teacher update             | EMA, momentum 0.996→0.999 cosine |
| Student τ_s                | 0.1                   |
| Teacher τ_t                | 0.04 → 0.07 warmup    |
| Centering momentum         | 0.9                   |
| Pretraining loss           | DINO cross-entropy    |
| Downstream probes          | kNN, logreg, linreg   |
| kNN k                      | 20 (DINO default)     |
| Sampling strategy          | intensity-based (preferred) |

## Sonobuoy integration plan

Pala et al. 2024 plugs into the **pretraining / representation-learning
stage** of the sonobuoy pipeline. Unlike the AST substitute
(`ast-fish-classification.md`) which supplies a ready-made classifier,
this paper supplies a **methodology** for building a domain-native
embedding when no sufficiently large pretrained model exists for the
specific hydrophone deployment. Concretely:

1. **Swap inputs from echogram tiles to hydrophone spectrogram tiles.**
   The DINO training loop is agnostic to the input modality — 4-channel
   echograms become, e.g., 4-hydrophone spectrogram stacks (one
   spectrogram per buoy in a subarray, same time window). The
   intensity-based sampler is even more critical for hydrophone data
   since 99 %+ of a passive-acoustic stream is empty ocean.
2. **Target placement in K-STEMIT architecture**: Pala-style SSL is the
   **feature-learning front-end** for the temporal branch of
   `.planning/sonobuoy/k-stemit-sonobuoy-mapping.md`. Either (a) it
   replaces the AST / SurfPerch embedding for deployments where
   pretrained bioacoustic models don't transfer (sub-ice, Arctic,
   krill-dominant), or (b) it layers *on top of* AST by initialising
   the ViT from AST's PCEN front-end and SSL-fine-tuning on site.
3. **EML-based student head**: the student projection head (MLP → L2
   → softmax over prototype bank) is a clean EML block. Ship as
   `eml-core::dino_head` with configurable prototype dimension (DINO
   default 65 536 for big pretraining, 4096 for small deployments).
4. **HNSW-indexed embeddings**: store each pretrained buoy-window
   embedding in `clawft-kernel/src/hnsw_service.rs` under namespace
   `sonobuoy/ssl-embeddings`. The HNSW index then serves as a
   retrieval-augmented classifier — kNN over HNSW implements the
   paper's downstream kNN probe for free, at web-scale.
5. **Class-imbalance sampler in Rust**: implement the intensity-based
   sampler as a streaming `Sampler` trait in `weftos-signal`, since
   sonobuoy data arrives as a live stream rather than a fixed tile
   dataset. Label-free sampling is a hard requirement for sonobuoy
   because labels are literally unavailable at inference time.
6. **Distillation for edge deployment**: plan a follow-up that distils
   the ViT-S/16 teacher into a MobileViT or EfficientFormer student so
   the embedding front-end can run on buoy hardware (WASI-p2 or
   embedded RISC-V). ADR candidate: ADR-055.
7. **ADR catalog entry**: add ADR-055 ("Self-supervised hydrophone
   pretraining with DINO and intensity-based sampling") as the canonical
   home for this methodology.

## Follow-up references

1. **Caron, M., Touvron, H., Misra, I., Jégou, H., Mairal, J., Bojanowski,
   P., & Joulin, A.** "Emerging Properties in Self-Supervised Vision
   Transformers." *ICCV 2021*. — Original DINO paper; the backbone
   formulation used here.
2. **Chen, T., Kornblith, S., Norouzi, M., & Hinton, G.** "A Simple
   Framework for Contrastive Learning of Visual Representations
   (SimCLR)." *ICML 2020*. — The framework the survey originally
   cited; maintains as a baseline for any ablation.
3. **Brautaset, O., Waldeland, A.U., Johnsen, E., Malde, K., Eikvil, L.,
   Salberg, A.-B., & Handegard, N.O.** "Acoustic classification in
   multifrequency echosounder data using deep convolutional neural
   networks." *ICES Journal of Marine Science* 77(4), 1391-1400 (2020).
   — Supervised U-Net predecessor; the "real" Brautaset paper the
   survey appears to have misremembered.
4. **Pala, A., Oleynik, A., Handegard, N.O.** "Addressing class imbalance
   in deep learning for acoustic target classification." *ICES Journal
   of Marine Science* 80(10), 2530 (2023). — Pala's supervised
   NearMiss-2 work that directly motivates the SSL sampler.
5. **Moummad, I., Serizel, R., & Farrugia, N.** "Self-Supervised Learning
   for Few-Shot Bird Sound Classification." arXiv:2312.15824 (2023).
   — Adjacent SSL-for-bioacoustics work. Useful as a contrastive
   counterpoint to Pala's DINO choice.
6. **Grill, J.-B. et al.** "Bootstrap Your Own Latent (BYOL)." *NeurIPS
   2020*. — Alternative to DINO/SimCLR; worth benchmarking for the
   sonobuoy pipeline because its teacher-student setup is closest to
   DINO's but without the centering-trick.

---

**Sources consulted**:
- [Pala, Oleynik, Malde, Handegard. Self-supervised feature learning for acoustic data analysis. *Ecological Informatics* 84, 102878 (2024). DOI:10.1016/j.ecoinf.2024.102878](https://www.sciencedirect.com/science/article/pii/S1574954124004205)
- [SSL_Acoustic GitHub (Ahmet Pala)](https://github.com/ahmetpala/SSL_Acoustic)
- [Ahmet Pala PhD thesis, UiB / IMR (via hdl.handle.net/11250/3190360)](https://hdl.handle.net/11250/3190360)
- [Caron et al., DINO, ICCV 2021](https://arxiv.org/abs/2104.14294)
