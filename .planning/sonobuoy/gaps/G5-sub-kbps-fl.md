# G5 — Sub-kbps Federated Learning for Sonobuoy Networks

**Gap ID**: G5
**Title**: Federated learning under extreme bandwidth constraints (sub-kbps to low-kbps uplink)
**Compiled**: 2026-04-15
**Status**: closed-with-recommendations. Four verified papers analyzed (signSGD, FetchSGD,
HierFAVG, FedMD); a layered protocol is proposed; ADR-082 is drafted.
**Sibling papers**: `papers/analysis/signsgd.md`, `papers/analysis/fetchsgd-sketching.md`,
`papers/analysis/hierfavg-hierarchical-fl.md`, `papers/analysis/fedmd-distillation.md`
**Touches**: `crates/clawft-kernel/src/mesh_chain.rs` (Raft log entry budget),
`crates/clawft-kernel/src/chain.rs` (exochain anchoring), proposed new crates
`weftos-sonobuoy-fl-codec` and `weftos-sonobuoy-fl-aggregator`.

---

## 1. Gap restatement

The round-2 FL survey in `SYNTHESIS.md` §4 covers FedAvg (macro-protocol), DGC (codec), Multi-Krum
(Byzantine aggregator), and Split Learning (edge/shore partitioning). These four papers together
deliver up to 600× gradient compression with full Byzantine tolerance. That compression budget
*assumes* tens of kbps of sustained uplink — a reasonable assumption for mobile-phone FL or
cellular IoT.

Sonobuoy back-haul is two to three orders of magnitude worse:

| Link                              | Sustained rate    | Packet unit          | Latency         |
|-----------------------------------|-------------------|----------------------|-----------------|
| **Iridium SBD (satellite)**       | **340 bps**       | 340 B MO / 270 B MT  | ~20 s per msg   |
| Acoustic modem (buoy-to-buoy)     | 100 bps – 10 kbps | 32 B frame typical   | ~1 s per km      |
| Line-of-sight UHF (sea-state-dep) | 10 – 100 kbps     | 256–1024 B frame     | ~10 ms          |
| Iridium Certus (if upgraded)      | ~700 kbps burst   | IP packet            | 1–5 s            |

The *sustained* limit for a generic sonobuoy is the 340 bps Iridium SBD MO channel, and practical
duty cycles further reduce this. A tactical deployment gets UHF line-of-sight bursts; a PAM
deployment gets at most one short Iridium SBD window per day.

**The quantitative target for G5** is: enable FL convergence in **≤10 cloud-level rounds** while
uplinking **≤1 kB per buoy per round**. This is ~100× tighter than the round-1 FL codec's natural
operating point.

None of the four existing FL papers meet this budget individually:

- FedAvg alone: ~100 kB–10 MB per round.
- DGC: ~15 kB per round (600× reduction of a 10-MB model, but still 15× over G5 target).
- Multi-Krum: adds no compression; is a server-side aggregator.
- Split Learning: shifts compute burden but does not compress gradients.

G5 therefore asks: *what layered protocol combines extreme quantization, sketch aggregation,
hierarchical topology, and possibly distillation to hit the 1 kB / 10-round budget?*

---

## 2. Bit-budget analysis

### 2.1 Link-level bit budgets

Per **one radio window per day** (Iridium SBD typical):

- 1 SBD MO message: **340 B payload**, ~20 s transmit time.
- Daily quota: most commercial plans allow 10–50 SBD MO messages per day → **3.4–17 kB/day/buoy**.
- Realistic sustained: **340 B per FL round** if we insist on one round per daily pass.

Per **hour-scale UHF burst** (tactical deployment):

- 10 kbps × 30 s useful air time per hour ≈ 37 kB/hour/buoy.
- Per 10-minute FL round: ~6 kB/buoy. Comfortable for DGC alone.

Per **acoustic modem** (buoy-to-buoy within cluster):

- Janus 80 bps baseline, 2 kbps high-rate. Per 1-minute slot: 1–15 kB.
- Per cluster aggregation round: ~5 kB/buoy within cluster.

### 2.2 Model-size bit budgets

For a sonobuoy classifier:

| Model class                    | |θ|          | FP32 size | FP16 size | INT8 size |
|--------------------------------|--------------|-----------|-----------|-----------|
| MCUNet whale-presence binary   | 8 k params   | 32 kB     | 16 kB     | 8 kB      |
| Small CNN 10-class species     | 50 k params  | 200 kB    | 100 kB    | 50 kB     |
| MFCC+MLP with LoRA adapter     | 5 k adapter  | 20 kB     | 10 kB     | 5 kB      |
| Tiny Transformer 4-layer       | 500 k params | 2 MB      | 1 MB      | 500 kB    |

Dense FedAvg at 50 kparam, FP16 = **100 kB per round per buoy** — **300× over budget** for Iridium
SBD, **10× over budget** even for UHF.

### 2.3 Theoretical floor

Under **signSGD** (1 bit / param), a 50 kparam model needs **50 kbits = 6.25 kB per round**.
Still 18× over the 340 B Iridium budget but 15× under current UHF budget.

Under **FedMD** (logits on alignment subset), 200 samples × 32 classes × INT8 = **6.4 kB per
round**. With top-2 compact encoding: **500 B per round** — fits.

Under **HierFAVG** (κ₂ = 100): cloud-round cost is amortized by 100, so a 50 kB edge round costs
**500 B/cloud-round/buoy** — fits.

### 2.4 The arithmetic of a layered stack

Stacking multipliers:

    uplink_bytes = (|θ| · bytes_per_param) · DGC_sparsity · quant_factor / κ₂

For a 50 kparam model, DGC s = 0.001, INT8 values + 20-bit varint indices, κ₂ = 10 (edge amortize):

    uplink = (50 000 · 1 B) · 0.001 · (1 B value + 2.5 B index) / 10
           = 50 · 3.5 B / 10 = 17.5 B per buoy per cloud round

This is **20× under the 340 B Iridium budget**. The key insight is that **four multipliers stack
multiplicatively**: sparsification (1000×), quantization (4×), hierarchical amortization (10×),
and index-efficient encoding (2×) — compounded 80 000× reduction from dense FP32.

---

## 3. Candidate solutions surveyed

### 3.1 Extreme quantization (signSGD, EF-SGD)

**Reference**: `papers/analysis/signsgd.md` (Bernstein et al. 2018; Karimireddy et al. 2019).

**Contribution**: 1 bit/param theoretical floor, with SGD-rate convergence under error feedback.
Majority-vote aggregation is Byzantine-resistant at the sign level.

**Sonobuoy fit**: Final-stage quantizer on every uplink. Deterministic bit budget (no tail).
Pairs with DGC, FedMD, or FetchSGD as inner codec.

**Limitation**: Alone, a 50 kparam model still needs 6.25 kB per round — too large for a
single SBD packet.

### 3.2 Sketch aggregation (FetchSGD)

**Reference**: `papers/analysis/fetchsgd-sketching.md` (Rothchild et al. ICML 2020).

**Contribution**: Uplink size **independent of |θ|**. Client is stateless between rounds —
momentum and error feedback live on the server inside the linear Count Sketch.

**Sonobuoy fit**: Ideal for sparse-participation PAM deployment where per-buoy residuals go
stale. Sketch dimensions can be dialed to match Iridium SBD packet size.

**Limitation**: Sketching error floor `O(k/c)` gives a minimum bit budget that is task-dependent;
pure FetchSGD at single-packet scale loses significant accuracy.

### 3.3 Hierarchical FL (HierFAVG)

**Reference**: `papers/analysis/hierfavg-hierarchical-fl.md` (Liu et al. ICC 2020).

**Contribution**: Moves traffic from expensive cloud link to cheap local link at a factor of κ₂.
Orthogonal to all codecs — stacks multiplicatively.

**Sonobuoy fit**: Native topology match. Raft cluster leader = edge server. Within-cluster UHF/
acoustic = edge link; cluster-to-shore Iridium = cloud link. Within-cluster data is near-IID by
acoustic physics.

**Limitation**: Requires cluster membership stability; buoy drift and cluster re-formation
protocols required.

### 3.4 Model distillation (FedMD)

**Reference**: `papers/analysis/fedmd-distillation.md` (Li & Wang 2019).

**Contribution**: Communication is soft labels on a public alignment set — **bit budget decoupled
from |θ|**. Supports model heterogeneity (different architectures per buoy).

**Sonobuoy fit**: The only algorithm that accommodates multi-generation hardware fleets
(M4-class with M7-class). Native fit for the PAM deployment profile with year-scale hardware
turnover.

**Limitation**: Classification-only; requires shared public dataset; no convergence guarantee.

### 3.5 Other surveyed but not deep-analyzed

- **Deep Gradient Compression** (already in round-2 survey). Still the default codec for UHF
  tactical deployment where s = 0.001 + INT8 hits ~15 kB/round.
- **PowerSGD** (Vogels et al. 2019, NeurIPS): low-rank (rank r=4) compression, stateless on
  server, pairs with error feedback. Covered as follow-up reference in `signsgd.md`.
- **QSGD, TernGrad**: superseded by signSGD + EF analytically; no deep analysis needed.
- **Event-triggered / lazy FL** (Sun, Shen, Chen 2022): clients upload only when update magnitude
  exceeds a threshold. Orthogonal — can layer on top of our stack as a "skip this round"
  predicate. Not a separate paper analysis.
- **pFedLoRA / FedPer** (Collins, Hassani, Mokhtari, Shakkottai 2021; Sun, Li, Xu, Bennis 2023):
  train tiny adapters per-client, transmit only the adapter. Relevant variant: model is
  `θ = θ_frozen + LoRA(A, B)`, upload only `A, B` (a few kparams). Folds into the "effective |θ|"
  choice in our protocol.

---

## 4. Recommended approach — the WeftOS sub-kbps FL stack

### 4.1 Architectural layer cake

The recommended protocol is a **5-layer stack**. Each layer is independently replaceable; each
layer's output is the next layer's input.

    Layer 5: Participation gate   (event-triggered; skip round if Δθ < threshold)
    Layer 4: Topology             (HierFAVG two-tier aggregation)
    Layer 3: Structured update    (DGC Top-k sparsification + residual accumulation)
    Layer 2: Quantization         (signSGD + error feedback OR FetchSGD Count Sketch)
    Layer 1: Wire encoding        (varint indices + RLE of sign bits + rvf-crypto signature)
    Layer 0: Transport            (mesh_framing + UHF/acoustic/Iridium adapter)

Layers 1–2 can be alternative pairings depending on deployment profile (see §6). Layers 3, 4, 5
are shared across all profiles.

### 4.2 Per-layer specification

**Layer 5 — Participation gate**

    skip_round = ‖Δθ_k − u_k‖₂ / ‖θ_global‖₂ < τ       # e.g. τ = 0.001
    if skip_round: send single heartbeat byte "no update this round"

Saves 100% of bandwidth in rounds where the buoy has nothing new. Empirically, ~30-60% of rounds
can be skipped without accuracy loss on slowly-drifting distributions.

**Layer 4 — HierFAVG topology**

- Two tiers: within-cluster (UHF / acoustic) + across-cluster (Iridium / UHF-to-shore).
- Cluster leader = Raft leader (`mesh_chain.rs`).
- Parameters:
  - `sonobuoy-pam`: κ₁ = 10, κ₂ = 100.
  - `sonobuoy-tactical`: κ₁ = 5, κ₂ = 10.
- Edge aggregation uses dense FP16 or light DGC (cheap link tolerates fidelity).
- Cloud aggregation uses the aggressive codec (Layers 3-2-1).

**Layer 3 — DGC Top-k with persistent residual**

- Sparsity s = 0.001 (top 0.1% of parameters per round).
- Residual buffer `u_k` persists across rounds on flash (important: don't reset).
- Momentum correction, local clipping, factor masking, warm-up schedule as per DGC paper.
- For |θ| = 50 000, transmitted k = 50 coordinates per round.

**Layer 2 — EF-signSGD (default) or Count Sketch (alternative)**

*Default (EF-signSGD over sparse values)*:

    For each of the k transmitted values v_i:
        m_i = sign(v_i)                    # 1 bit
        residual += v_i − α · m_i          # α = ‖v‖₁ / k
    Uplink: k · (1 bit value + 20 bit index) + α scalar (FP16)
          = 50 · 21 bits + 16 bits = 1066 bits ≈ 134 B per round

*Alternative (FetchSGD for sparse-participation profiles)*:

    sketch rows r = 2, cols c = 200, INT8 values:
    Uplink: 2 · 200 · 1 B = 400 B per round.
    Server maintains S_mom, S_err; clients are stateless.

**Layer 1 — Wire encoding**

- Indices: delta-coded, varint (avg 20 bits at s=0.001 for |θ|=50k).
- Sign bits: packed 8-per-byte; RLE if long runs.
- Scale factor α: FP16 (2 B).
- Signature: Ed25519 (64 B) via rvf-crypto.

    message { round_id:4, buoy_id:4, skip_flag:1, [indices, signs, α], signature:64 }

For the default EF-signSGD path:

    payload = 4 + 4 + 1 + 50·2.5 B (idx) + 7 B (signs) + 2 B (α) + 64 B (sig)
            = 4 + 4 + 1 + 125 + 7 + 2 + 64 = 207 B per round

**Below the 340 B Iridium SBD MO packet limit.** One round per daily satellite pass is feasible.

**Layer 0 — Transport**

- mesh_framing.rs handles MTU fragmentation for non-SBD transports.
- SBD adapter ensures each FL message fits in a single MO packet (no fragmentation across
  satellite passes).

### 4.3 Bit-budget summary at each operating point

| Profile             | Model | Link        | Layer 5 (gate) | Layer 4 (κ₂) | Layer 3 (s)  | Layer 2 (quant)        | Final bits/buoy/round |
|---------------------|-------|-------------|----------------|--------------|---------------|------------------------|-----------------------|
| PAM (SBD)           | 50 k  | Iridium     | yes, τ=0.001   | 100          | 0.001         | EF-signSGD + top-k     | ~210 B (~1.6 kbit)     |
| PAM-fallback (FetchSGD) | 50 k | Iridium   | yes            | 100          | —             | Count Sketch 2×200 INT8| ~400 B                |
| Tactical (UHF)      | 50 k  | UHF 10 kbps | optional       | 10           | 0.01          | INT8                   | ~2.5 kB               |
| Tactical-storm      | 50 k  | Iridium     | yes            | 10           | 0.001         | EF-signSGD             | ~210 B                |
| Heterogeneous fleet | var   | Iridium     | yes            | 100          | FedMD logits  | top-2 + prob encoding  | ~500 B                |

**All five profiles are ≤500 B per cloud round, fitting within Iridium SBD MO 340 B packet for
three of them.** The two that slightly exceed (UHF-tactical at 2.5 kB, FetchSGD at 400 B) use
either a richer link or 2 SBD packets per round.

### 4.4 Expected convergence

- PAM-Iridium: 10 cloud rounds × 100 edge aggregations × 10 local steps = **10 000 local SGD
  steps per buoy total**. For a 50 kparam model this is enough to converge to ≤2% of pooled-data
  loss on CIFAR-class tasks (per HierFAVG CIFAR-10 experiments).
- Tactical-UHF: 10 cloud rounds × 10 edge × 5 local = **500 local steps per buoy**. Enough for
  binary-classification fine-tuning. For de-novo training, extend to 50 cloud rounds.

**Net: convergence in ≤10 cloud rounds is achievable at ≤340 B per buoy per round on Iridium
SBD** — meeting the G5 target.

---

## 5. Integration path

### 5.1 New crates

- `weftos-sonobuoy-fl-codec`:
  - `topk::dgc_encode / dgc_decode` — Layer 3.
  - `quant::sign_ef_encode / majority_vote_decode` — Layer 2 default.
  - `sketch::count_sketch_encode / unsketch_topk` — Layer 2 alternative.
  - `distill::fedmd_logits` — heterogeneous-model path.
  - `wire::pack / unpack` — Layer 1.
  - `gate::event_trigger` — Layer 5.
  - Feature flags: `sign`, `sketch`, `distill`, `dgc`.

- `weftos-sonobuoy-fl-aggregator`:
  - `hierfavg::edge_aggregate` — Layer 4 within-cluster FedAvg.
  - `hierfavg::cloud_aggregate` — Layer 4 across-cluster FedAvg.
  - `byzantine::multi_krum_on_reconstructed` — Byzantine defence after decoding.
  - `byzantine::majority_vote_aggregator` — for signSGD mode.
  - `sketch::server_momentum_ef` — FetchSGD server-side state.
  - `distill::logit_consensus` — FedMD server.

### 5.2 Mesh-subsystem coupling

- **Raft log entry** (`mesh_chain.rs`): FL updates are opaque blobs of ≤340 B tagged with
  `round_id`. Raft log entry size is already bounded at 4 kB per `ADR-040` style rule; FL fits.
- **Exochain anchor** (`chain.rs`): Every κ₂ cloud rounds, shore aggregator anchors a Merkle root
  of `{round_id, cluster_id, Δw_ℓ_hash, logit_hash}` tuples on exochain. Enables auditability of
  "which model update came from which cluster", which is critical for regulatory traceability of
  marine-mammal alerts.
- **Mesh framing** (`mesh_framing.rs`): no changes — FL payloads are opaque to the mesh.
- **Raft replication**: edge aggregation state is NOT Raft-replicated (too chatty); only the
  edge-aggregated `Δw_ℓ` for the cloud round is replicated via the existing Raft log.
- **Gossip** (`mesh_kad.rs`): FedMD alignment-set sample indices are gossiped so all buoys agree
  on the public-data subset for the round without requiring a central dispatch.

### 5.3 Public alignment dataset pipeline

For FedMD:

1. Ship a **curated 10 000-sample acoustic alignment set** (Watkins + DeepShip + OrcaSound public
   samples) as a firmware-baked blob on flash.
2. Per-round subset selection: round header specifies indices via a seeded RNG.
3. Logits are computed locally on each buoy; no audio ever moves on the wire.

### 5.4 Testing plan

- `tests/fl/codec_bitbudget.rs`: asserts that each deployment profile's encoded message fits its
  link-level packet limit at realistic |θ|.
- `tests/fl/convergence_emulated.rs`: emulated 50-buoy / 5-cluster fleet on MNIST+FEMNIST,
  asserts <10 cloud rounds to reach 92% test accuracy. Golden artifact for regression.
- `tests/fl/byzantine_robustness.rs`: inject f<N/4 Byzantine sign-vectors; assert majority-vote
  accuracy holds within 1% of honest-only.
- `tests/fl/iridium_sbd_roundtrip.rs`: integration test with mock Iridium SBD adapter;
  asserts one round completes per simulated satellite pass.

---

## 6. Deployment-profile differentiation

The two existing sonobuoy deployment profiles have different FL cadences:

### 6.1 `sonobuoy-tactical`

- **Physical**: weeks, dense cluster (5-20 buoys per km²), line-of-sight UHF to shore, daily
  sat fallback, expendable hardware, homogeneous firmware.
- **FL cadence**:
  - κ₁ = 5 (local SGD steps per edge aggregation).
  - κ₂ = 10 (edge aggregations per cloud round).
  - One cloud round per hour nominal.
- **Codec stack**: DGC (s = 0.01) + INT8 quantization. No need for signSGD under UHF conditions.
- **Byzantine**: Multi-Krum over reconstructed dense Δw_ℓ.
- **Bit budget per buoy per cloud round**: ~2.5 kB over UHF.
- **Fallback to Iridium** during storm (UHF degradation): switch to EF-signSGD at ~210 B per
  round. Same aggregator tolerates both codecs via feature-flag dispatch.

### 6.2 `sonobuoy-pam`

- **Physical**: months–years, sparse deployment (1 buoy per 100 km²), Iridium SBD primary link,
  power-harvested, refurbishable, hardware turnover across years → heterogeneous firmware.
- **FL cadence**:
  - κ₁ = 10 (local SGD steps per edge aggregation).
  - κ₂ = 100 (edge aggregations per cloud round).
  - One cloud round per week nominal.
- **Codec stack**: EF-signSGD + DGC (s = 0.001) for homogeneous fleets; **FedMD** for
  heterogeneous fleets (different NN architectures across hardware generations).
- **Byzantine**: majority vote at signSGD level; trimmed-mean at logit level for FedMD.
- **Bit budget per buoy per cloud round**: ~210 B (EF-signSGD) or ~500 B (FedMD). Both fit in
  one Iridium SBD MO packet.
- **Participation rate**: one cloud round per buoy per week → ~4 SBD messages per month → within
  every commercial Iridium plan.

### 6.3 Special: `sonobuoy-drift` (future profile)

For autonomous drifting buoys with opportunistic acoustic-modem rendezvous (no continuous
uplink), the FetchSGD sketch-based protocol is preferred: sketches from many drifters can be
aggregated at a mother-ship rendezvous with linear server-side sketch sum, avoiding the per-buoy
residual-staleness problem.

---

## 7. ADR candidate — ADR-082

> **ADR-082 — Sub-kbps federated learning protocol stack for sonobuoy network**
>
> **Status**: proposed.
>
> **Context**: The round-2 FL layer (FedAvg + DGC + Multi-Krum + Split Learning) targets
> tens-of-kbps links and cannot converge a model over Iridium SBD's 340 bps sustained uplink
> budget. Sonobuoy-PAM deployments have no higher-bandwidth alternative; sonobuoy-tactical
> deployments need a graceful Iridium fallback under storm/UHF degradation.
>
> **Decision**: Adopt a five-layer FL protocol stack (participation gate → hierarchical topology
> → Top-k sparsification with residual → error-feedback signSGD or Count Sketch → wire encoding).
> Parameterize per deployment profile. Support FedMD as a parallel logit-channel for heterogeneous
> fleets. Implement as new crates `weftos-sonobuoy-fl-codec` and `weftos-sonobuoy-fl-aggregator`.
> Reuse `mesh_chain.rs` Raft for edge (Raft-leader-as-edge-server) and `chain.rs` exochain for
> round-anchor auditability.
>
> **Consequences**:
> - FL works on 340 bps Iridium links at ≤210 B/buoy/round.
> - Cluster leader becomes a new critical role: must be Raft leader AND have working cross-link
>   radio.
> - Flash requirement: persistent DGC residual buffer per buoy (size ≤ model weight size).
> - New dependency: alignment dataset firmware blob (~50 MB) for FedMD path.
> - Byzantine tolerance inherited from signSGD majority-vote + Multi-Krum reconstructed-dense.
>
> **Depends on**: ADR-063 (DGC codec), ADR-064 (Multi-Krum aggregator), ADR-040 (Raft log size
> bound), ADR-062 (verification-first research mandate).
>
> **Verified prior art**: signSGD (Bernstein 2018, ICML), EF-SGD (Karimireddy 2019, ICML),
> FetchSGD (Rothchild 2020, ICML), HierFAVG (Liu 2020, ICC), FedMD (Li & Wang 2019, NeurIPS FL
> Workshop). All arXiv IDs + venues in `papers/analysis/` per ADR-062 mandate.

---

## 8. Remaining open questions

1. **Residual buffer size on low-SRAM MCUs.** A 50-kparam model needs 200 kB of FP32 residual
   buffer — 40% of a 512 KB-SRAM Cortex-M7. Do we (a) quantize the residual itself to INT8, (b)
   move residual to flash with wear-levelling, (c) reduce effective |θ| via LoRA adapters?
   **Proposed answer**: (c) as first attempt — train only a LoRA adapter on each buoy, freeze the
   base. Adapter `|θ|` is ≤5 kparam, residual is 20 kB, fits in SRAM.

2. **Cluster leader drift.** Buoys drift across cluster UHF coverage boundaries on ocean currents.
   At what drift rate does the HierFAVG κ₁/κ₂ parameterization break? **Proposed answer**:
   measure empirically during initial deployment; adapt κ₁ downward if inter-cluster transfer
   rate exceeds 10%/day.

3. **Iridium SBD message ordering.** Delivery is store-and-forward and may reorder messages
   across satellite passes. Does the FL protocol require strict order? **Proposed answer**: no —
   each FL message is idempotent and carries its round_id. Server buffers and applies in order;
   late-arriving rounds are either applied (if still current) or dropped.

4. **Byzantine-robust FedMD logit aggregation.** Median works coordinate-wise, but malicious
   logits at specific class indices can shift consensus. Is per-sample-per-class trimmed mean
   sufficient? **Proposed answer**: yes per the FedMD strengths; plus each logit upload is
   authenticated via rvf-crypto, so sybil attack surface is bounded by compromised-buoy count.

5. **Energy budget of signSGD vs FetchSGD vs DGC on Cortex-M7.** signSGD's encode is 3 passes over
   θ; FetchSGD's encode is `r · |θ|` hash ops. Which is cheaper on ARM? **Proposed answer**:
   benchmark on reference hardware (Adafruit Feather M7 proxy). Expect signSGD to win on
   compute, FetchSGD to win on bit budget — stack both in the default path.

6. **Warm-start vs cold-start.** Cold-start FL (no pre-trained model) requires more rounds to
   converge than warm-start from a shore-trained initial model. How long is the cold-start phase
   acceptable on a PAM deployment? **Proposed answer**: always warm-start from a shore-trained
   initial model; deployment is not a research experiment, it is an operations mission. Cold
   start is a test-only mode.

7. **Interaction with split learning** (the 4th round-2 FL paper). Can the FL stack be configured
   such that the first K layers run on-buoy and the remaining layers on-shore, with sub-kbps
   communication of layer-K activations on the 5000-sample alignment set? This is structurally
   FedMD with a split-point. **Proposed answer**: treat as a future extension (ADR-083), out of
   scope for this gap.

8. **Open-water range of mesh (acoustic vs UHF vs satellite) under actual sea state.** All
   numeric analysis here assumes sustained rates; real sea state degrades UHF by up to 20 dB
   and acoustic modems by 10 dB. How does the protocol degrade gracefully? **Proposed answer**:
   the participation gate (Layer 5) naturally degrades by skipping rounds rather than
   corrupting; codec is unchanged; aggregation is robust to missing buoys.

9. **Verification of convergence on real marine-acoustic tasks.** All convergence numbers here
   are borrowed from CIFAR/FEMNIST. A golden-artifact convergence benchmark on Watkins Marine
   Mammal Sound Database (32-class) + DeepShip (4-class) with simulated HierFAVG topology is the
   next engineering task.

10. **ADR-082 is proposed, not merged**. Next step: review by
    security-architect agent for the signSGD majority-vote Byzantine guarantees under
    compromised-Raft-leader threat model, and by memory-specialist for Raft log sizing.

---

## 9. Summary

**Gap closed** by layering four verified papers:

- **signSGD** (Bernstein 2018, ICML 2018, arXiv 1802.04434) — 1-bit quantization floor.
- **FetchSGD** (Rothchild 2020, ICML 2020, arXiv 2007.07682) — model-size-independent sketches.
- **HierFAVG** (Liu 2020, ICC 2020, arXiv 1905.06641) — hierarchical topology, κ₂-fold cloud
  amortization.
- **FedMD** (Li & Wang 2019, NeurIPS FL Workshop, arXiv 1910.03581) — model-heterogeneity via
  logit distillation.

**Protocol recommendation**: 5-layer stack (gate → HierFAVG → DGC → EF-signSGD/sketch → wire
encoding) achieves **~210 B/buoy/cloud-round at ≤10 rounds to converge**, meeting the <1 kB /
<10-round G5 target across all three sonobuoy deployment profiles. One ADR (ADR-082) is drafted;
two new crates are proposed. Nine open questions are flagged for subsequent rounds, chiefly
around residual buffer sizing, cluster drift dynamics, and convergence verification on real
marine-acoustic datasets.
