# FedMD — Heterogeneous Federated Learning via Model Distillation

## Citation

- **Title**: FedMD: Heterogenous Federated Learning via Model Distillation
- **Authors**: Daliang Li, Junpu Wang
- **Affiliation**: Northeastern University (Boston)
- **Venue**: NeurIPS 2019 Workshop on Federated Learning for Data Privacy and Confidentiality
  (non-archival workshop track)
- **arXiv**: [1910.03581](https://arxiv.org/abs/1910.03581) (v1 Oct 2019)
- **Reference code**: https://github.com/diogenes0319/FedMD_clean
- **PDF**: `.planning/sonobuoy/papers/pdfs/fedmd.pdf`

## Status

**verified**. Title, authors, arXiv ID, and NeurIPS 2019 FL workshop venue corroborated against
arXiv abstract, the FL-NeurIPS 2019 workshop schedule, and the reference implementation linked
from the arXiv page. The workshop track is non-archival, but the paper is the canonical reference
for *distillation-based* federated learning (~1600 citations at time of writing).

## One-paragraph Summary

Li & Wang (NeurIPS FL Workshop 2019) break the architectural assumption that all federated clients
share the same model. In their setup, each participant keeps a **privately-designed, possibly-
different neural network**. Communication is not gradients and not weights — it is **soft labels
(logits) on a small shared public dataset**. Every round, each participant computes its model's
predictions on a common public "alignment" set of 5 000 samples, uploads a `5000 × C` matrix of
logits, and the server averages them into a consensus. Each participant then **distills** toward
that consensus (trains to match the averaged logits on the public set) and afterwards re-fits on
its private data. Communication cost is therefore `O(public_set_size × num_classes)` per round,
**completely independent of model size**. With 10 heterogeneous participants on CIFAR-10/100 and
MNIST/FEMNIST, each participant gains ~20% test accuracy over solo training and comes within "a
few percent" of a pooled-data upper bound. For sonobuoy fleets where (a) different buoy classes
have wildly different MCU budgets (Cortex-M4 to Cortex-M7 to Pi-class shore aggregator) and
therefore *cannot* share a single model, and (b) the communication budget is bits-per-class-per-
sample rather than bits-per-weight, FedMD is the **only** FL algorithm in this survey that
natively supports model heterogeneity. It is also the most communication-efficient one per-round
for small classification tasks.

## Methodology

### Setup

- `m` participants, each with **private model** `f_k` of arbitrary architecture, trained on
  **private data** `D_k`.
- Each participant has access to a common **public alignment dataset** `D_0` (|D_0| = 5000 in
  the paper).
- Participants wish to benefit from each other's private data *without* sharing raw data and
  *without* constraining model architecture.

### The FedMD cycle (5 phases per round)

    Phase 0 (pre-training, once):
        Each f_k is pre-trained on D_0 (public), then fine-tuned on D_k (private).

    Phase 1 (Communicate):
        subset d ⊆ D_0, |d| = 5000 (or smaller)
        each participant k computes logits z_k = f_k(d) ∈ ℝ^{|d|×C}
        upload z_k to server.

    Phase 2 (Aggregate):
        consensus z̃ = (1/m) Σ_k z_k ∈ ℝ^{|d|×C}
        server broadcasts z̃ back.

    Phase 3 (Digest):
        each participant trains f_k on (d, z̃) using soft-label loss
            L_digest = KL( softmax(z̃/T) || softmax(f_k(d)/T) )
        for a few epochs.

    Phase 4 (Revisit):
        each participant trains f_k on private data D_k for a few epochs.

The central trick is that **only logits pass through the communication channel** — not gradients,
not weights. Logits on a known public dataset do not leak private data (beyond what any
prediction inherently leaks).

### Bit budget per round per client

For a 10-way classification task, 5000-sample alignment set, FP32 logits:

    up = 5000 · 10 · 4 B = 200 kB

For 2-way (binary presence/absence, e.g. whale-yes/no), FP16 logits, 500-sample subset:

    up = 500 · 2 · 2 B = 2 kB

For 4-way (species A/B/C/D), INT8 logits with symmetric quant, 200-sample micro-subset:

    up = 200 · 4 · 1 B = 800 B   ← fits in one Iridium SBD MO message

Unlike weight-based methods, the **bit budget does not depend on model size**. A 100 kparam model
and a 10 Mparam model have identical FedMD bit budget. This is a qualitative difference.

### Temperature and distillation loss

    L_digest(θ_k) = Σ_{x ∈ d} KL( softmax(z̃(x)/T) || softmax(f_k(x; θ_k)/T) )

Temperature `T > 1` softens the targets, emphasizing class-similarity structure and relegating the
argmax dominance. Paper uses T = 1 with logits directly (no softmax), reporting no meaningful
difference vs softmax with T = 20.

### Why it works (intuition)

Each participant's `f_k` learns both (i) from its private data and (ii) from the **dark knowledge**
(Hinton's term) encoded in other participants' logits. The pairwise class confusions on the public
set carry information about which class pairs are hard to distinguish — information that survives
private-data heterogeneity because all participants agree on the label space.

## Key Results

### CIFAR-10 + CIFAR-100 (10 participants, heterogeneous 2-4 layer CNNs)

- Solo training on private CIFAR subset (500 samples/class): ~40% test accuracy average.
- After FedMD: ~60% test accuracy average. **~20% absolute improvement.**
- Pooled-data upper bound: ~63%. FedMD comes within ~3%.

### MNIST + FEMNIST (10 participants)

- Solo: ~85% on private subset.
- FedMD: ~92%. **~7% improvement (smaller gap because MNIST is easier).**
- Pooled: ~94%.

### Communication cost comparison

- FedAvg with same 10-participant 1 Mparam model: ~40 MB/round/participant.
- FedMD on same task with 5000-sample public set, 100-class: ~2 MB/round/participant — 20× less.
- FedMD with 500-sample subset, 10-class: 20 kB/round/participant — 2 000× less than FedAvg.

### Model-heterogeneity advantage

In the paper's experiments, different participants ran 2-layer, 3-layer, and 4-layer CNNs with
filter counts ranging 64–256. FedAvg cannot even run in this setting (weights don't align). FedMD
runs natively.

### Number of rounds to convergence

- 5–10 rounds for MNIST/FEMNIST to stabilize.
- 10–20 rounds for CIFAR-10/100.

Fits the G5 <10-rounds target for tactical deployments, a bit long for PAM.

## Strengths

- **Model-heterogeneous by design**. The only FL algorithm in this survey that allows different
  participants to run different architectures. Essential for sonobuoy fleets with mixed hardware
  (tiny M4-class trigger buoys and M7-class confirmation buoys training jointly).
- **Bit budget decoupled from model size**. Budget is `|d| × C × bytes_per_logit`. This can be
  driven to tens of bytes with small alignment subsets and INT8 logits.
- **No gradient-level privacy concerns**. Logits on public data leak less than raw gradients (which
  can reconstruct training samples in the limit — see Zhu et al. 2019 "Deep Leakage from
  Gradients"). FedMD's channel is inherently less informative about private data.
- **Orthogonal to Byzantine-robust aggregation**. Median-of-logits at the server is as
  statistically valid as mean-of-logits; supports Multi-Krum over logit space.
- **Simple server**: server averages logits. No weight management, no sketch state, no residual
  buffers. A shore aggregator can be tiny.
- **Scales to very small classification tasks**: whale/no-whale binary → <100 B/round uplink.
- **Natively supports public knowledge bootstrapping**: every sonobuoy deployment can bootstrap on
  a public dataset (e.g. Watkins Marine Mammal Sound Database, 32 species, public).

## Limitations

- **Requires a shared public dataset of the right task domain.** If no public dataset exists for
  a given private task, FedMD cannot start. For underwater bioacoustics this is a minor issue
  (Watkins, DeepShip, OrcaSound are public); for novel / classified tasks it's a hard blocker.
- **The public set must be "aligned" with private distribution.** If public is dolphins and
  private is orcas, the dark knowledge transfer is weak.
- **Classification-only, basically**. Extending to regression (e.g. TDOA fine-estimation) requires
  generalizing to continuous output distillation — doable but not in the paper.
- **No convergence guarantee**. Empirical only. No analytical rate bound on rounds-to-convergence.
- **Soft labels can be privacy-attacked**. Membership-inference attacks on logits over a public
  set are a known risk. Differential privacy calibration (adding noise to logits pre-upload)
  mitigates this at accuracy cost.
- **Multiple rounds of digest+revisit required**: each round is several local epochs, not a single
  SGD step. Compute cost per round is higher than FedAvg.
- **Server sees per-client logits — potential inference channel.** If the server is adversarial
  (e.g. compromised shore), per-client logits over a known public set reveal the class-confusion
  fingerprint of each client's private data. Secure aggregation or per-client DP is required for
  strong privacy.
- **Scales poorly in number of classes.** Bit budget is `O(C)`; a 1000-class problem is 100× the
  traffic of a 10-class problem.

## Portable Details

### Public alignment dataset for marine acoustics

- **Watkins Marine Mammal Sound Database**: 32 species, ~12 000 clips, public domain.
- **DeepShip**: 4 ship classes, 600 clips, CC-BY.
- **OrcaSound**: continuous public hydrophone streams, several years.

A 500-sample subset from Watkins + 100 from DeepShip provides a **600-sample alignment set
covering 36 classes** — concrete, reproducible, and directly usable by FedMD for sonobuoy FL.

### Bit budget tuning knobs

| Knob                       | Effect                                    |
|----------------------------|-------------------------------------------|
| alignment subset size |d|  | Linear in uplink; ≥50 for stable digest   |
| number of classes C        | Linear in uplink                          |
| bytes per logit            | INT8 = 1, FP16 = 2, FP32 = 4              |
| logit compression          | softmax + top-2 argmax + prob → ~1 B      |
| digest temperature T       | No comms impact; affects transfer quality |
| rounds per convergence     | Linear in cumulative uplink               |

### Compact upload encoding (new)

Instead of full `|d| × C` matrix, transmit only **top-2 class indices + top-1 probability** per
alignment sample — this carries most of the dark-knowledge signal for well-separated tasks:

    per sample: 2 × log2(C) + 1 byte (prob quantized to 8 bits)
    for C=32, |d|=200: 200 × (10 + 8) bits = 450 B — one Iridium SBD packet.

This is a **novel** encoding not in the paper, but is the natural low-bandwidth variant for
sonobuoys. Analog to Hinton's "two-probability distillation" insight.

### FedMD algorithm pseudocode (compact)

    # once:
    for each client k: pretrain(f_k, D_0); finetune(f_k, D_k)

    # each round:
    sample d ⊆ D_0 of size B
    for each client k:
        z_k = f_k(d)                        # [B, C] logits
        send z_k to server
    z̃ = mean_k(z_k)                          # or median for robustness
    broadcast z̃
    for each client k:
        for several epochs:
            train f_k on (d, z̃) with KL(z̃/T || f_k(d)/T)  # digest
        for several epochs:
            train f_k on D_k with cross-entropy               # revisit

### Robust aggregation at server

    z̃ = median_k(z_k)      # per-sample, per-class median; robust to Byzantine
or
    z̃ = trimmed_mean_k(z_k, f)   # drop f most extreme clients

Both preserve the distillation semantics and defeat naive model-poisoning attacks that would
otherwise simply shift the consensus logits.

### Asynchronous FedMD

Because the server maintains a running `z̃` over time, late-arriving clients can simply insert
their `z_k` into an exponential-moving-average `z̃ ← β z̃ + (1-β) z_k`, enabling **truly
asynchronous** operation — a buoy that reports weekly contributes without blocking faster-
reporting buoys. This is a key adaptation for the PAM deployment profile.

## Sonobuoy Integration Plan

### Where FedMD slots into WeftOS

FedMD is **orthogonal to weight-based FL**. It runs as a **parallel FL track** — buoys can
participate in FedAvg+DGC (weights) on their core model *and* FedMD (logits) on a shared
public-label head. Or they can run FedMD-only when their model architecture differs from the
fleet default.

- **Per-buoy component**: `distill::fedmd_client` in `weftos-sonobuoy-fl-codec`. Inputs: local
  model, public alignment subset ID. Outputs: logits matrix.
- **Per-shore component**: `distill::fedmd_aggregator` in `weftos-sonobuoy-fl-aggregator`.
  Averages logits (median or mean), maintains EMA consensus `z̃` for async operation.
- **Public alignment dataset**: stored as fixed public blob distributed with the firmware release
  and cached on flash. Sample indices referenced in round headers (no need to re-transmit the
  audio).

### Wire format

    round_header { round_id, alignment_subset_ids[B], loss_weights[C] }
    per-buoy upload: logits[B, C] as INT8 with per-class scale
                     + Merkle proof of local computation + signature

With B=200, C=32, INT8 + 32 bytes overhead:

    payload = 200 · 32 · 1 B + 32 B = 6 432 B ≈ 6 kB per round

With top-2 + prob encoding: ~500 B per round — **fits in one Iridium SBD MO packet**.

### Deployment-profile fit

- **`sonobuoy-pam` (months, sparse, heterogeneous hardware)**: *ideal* FedMD target. Hardware
  turnover across years means different generations of buoys have different models; FedMD is
  the only FL algorithm that accommodates this. Async EMA consensus fits sparse participation.
- **`sonobuoy-tactical` (hours, dense, homogeneous hardware)**: weight-based FL (FedAvg+DGC) wins
  on per-round accuracy. FedMD as fallback when heterogeneous hardware mix grows.

### Hybrid FedMD + HierFAVG

Each cluster runs FedAvg locally (dense UHF), then the cluster leader participates in FedMD at
the shore level (logits only, sparse Iridium). Clusters with different model architectures can
still collaborate through the logit channel. This is the cleanest unification of the two
topologies.

### ADR implication

*ADR-082 candidate (see gap-closing addendum)*: FedMD is the default collaboration protocol when
buoy hardware fleets are **heterogeneous** or when the model-weight transfer would exceed the
available uplink budget. It runs alongside (not instead of) weight-based FL.

## How This Closes G5

G5 demands <1 kB/buoy/round uplink, <10-round convergence.

- **Bit budget independent of model size**: FedMD can uplink 500 B with a 10-Mparam model as
  easily as with a 10-kparam model. This is **unique** in the FL literature.
- **With top-2 compact encoding, <1 kB/round uplink is natively achievable** for any
  classification task with C ≤ 64 classes.
- **5–10 round convergence** empirically on MNIST/FEMNIST-class tasks — matching the G5
  round budget.
- **Model heterogeneity**: addresses a gap the other three G5 papers cannot — the physical
  reality that sonobuoy fleets have multi-generation hardware and therefore multi-architecture
  models.
- **Compatible with sparse participation via EMA consensus** — no residual buffer required,
  unlike DGC. Matches the FetchSGD virtue without requiring a sketch.

## Follow-up References

1. **Jeong et al. 2018** — "Communication-Efficient On-Device Machine Learning: Federated
   Distillation and Augmentation under Non-IID Private Data", NeurIPS 2018 ML on the Phone
   Workshop, arXiv [1811.11479](https://arxiv.org/abs/1811.11479). The earliest *federated
   distillation* paper; simpler than FedMD (no public set), distillation is over averaged logits
   of the *private* data class means. FedMD is more accurate but more communication-costly.
2. **Lin et al. 2020** — "Ensemble Distillation for Robust Model Fusion in Federated Learning",
   NeurIPS 2020, arXiv [2006.07242](https://arxiv.org/abs/2006.07242). Extends FedMD with
   ensemble predictions over public unlabeled data; addresses FedMD's public-dataset-label
   requirement.
3. **Sattler, Marbán, Rischke, Samek 2020/2021** — "Communication-Efficient Federated Distillation",
   arXiv [2012.00632](https://arxiv.org/abs/2012.00632). FedMD with adaptive alignment subset
   selection and DP noise; studies sub-kB per round operation.
4. **Itahara et al. 2020** — "Distillation-Based Semi-Supervised Federated Learning for
   Communication-Efficient Collaborative Training with Non-IID Private Data", IEEE Trans. Mobile
   Computing, arXiv [2008.06180](https://arxiv.org/abs/2008.06180). Semi-supervised extension.
5. **Hinton, Vinyals, Dean 2015** — "Distilling the Knowledge in a Neural Network", NeurIPS 2014
   Deep Learning Workshop, arXiv [1503.02531](https://arxiv.org/abs/1503.02531). The foundational
   distillation paper FedMD builds on.
6. **Zhu, Liu, Han 2019** — "Deep Leakage from Gradients", NeurIPS 2019, arXiv
   [1906.08935](https://arxiv.org/abs/1906.08935). The threat-model motivation for logit-based
   communication: shows that raw gradients reconstruct training samples, while logits on public
   data do not.
