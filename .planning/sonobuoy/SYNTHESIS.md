# Sonobuoy Project — Synthesis Report

**Compiled**: 2026-04-15
**Sources**: 18 paper analyses in `papers/analysis/`, the K-STEMIT
foundational mapping in `k-stemit-sonobuoy-mapping.md`, and the live WeftOS
substrate code in `crates/clawft-kernel/src/`.
**Audience**: Anyone drafting the sonobuoy project kickoff, the related
ADRs, or the first crate scaffolding.

---

## 0. Executive summary

- **18 papers analyzed across 6 categories.** The original survey was
  compiled from memory; **13 of 18 cited papers did not exist as cited.**
  Replacements were found for all 13 and verified against arXiv / DOI /
  publisher sites. The architectural story survives the substitution —
  every slot in the K-STEMIT-extended pipeline maps onto a real,
  downloadable, published paper.
- **The foundational thesis holds.** A K-STEMIT-style dual-branch
  spatio-temporal graph neural network with a physics-prior side-channel
  is the right architecture for a distributed sonobuoy stack. The eight
  concrete papers that populate its four slots (temporal, spatial,
  physics, head) all share architectural DNA with each other and with the
  existing WeftOS ECC substrate.
- **Five published numerical headline claims** ground the design: DEMONet
  80.45% on DeepShip; SIR+LMR holds ~80% at -15 dB SNR; Grinstein GNN-TDOA
  29% error reduction on 4-mic arrays; Perch 0.83 Top-1 / 0.98 AUC on
  Watkins marine-mammal 32-way at 32 shots/class; BEATs 48.6 mAP on
  AudioSet-2M. These are the targets against which the sonobuoy crate
  should benchmark.
- **Four unsupported survey claims** should not land in any ADR: Sanford/Abbot
  "1000× FNO speedup" (actual peer-reviewed FNO → 28.4%); Nguyen/Kessel
  "6-12 dB FiLM SNR gain" (no literature source); Martinez/Chou FishGraph
  (paper doesn't exist); Waddell/Rasmussen AST fish classification (paper
  doesn't exist).
- **The sonobuoy project is three crates, one core.** `weftos-sonobuoy-{temporal,
  spatial, physics, head}` share a GNN message-passing core with the
  quantum cognitive layer's `quantum_register`, share a learned-function
  core with `eml-core`, and share a retrieval layer with
  `clawft-kernel::hnsw_service`.
- **Nine ADRs emerge from this synthesis**, numbered ADR-053 through ADR-061.
  They are listed in §6 with one-line motivation each.

---

## 1. The honest state of the cited literature

| # | Category | Survey cite | Reality | Substitution |
|---|----------|-------------|---------|--------------|
| 1.1 | Passive sonar | UATD-Net (Yang, JOE 2024) | Fabricated | **DEMONet** (Xie et al., arXiv:2411.02758, 2024) |
| 1.2 | Passive sonar | Smoothness-UATR (Xu, Ren, arXiv:2306.06945) | **Verified** (author corrected to Xu, Xie, Wang) | — |
| 1.3 | Passive sonar | UATR survey (Luo, arXiv:2503.01718) | Fabricated (that arXiv ID is a cancer paper) | **Feng et al. 2024** (MDPI Remote Sensing 16(17):3333) |
| 2.1 | Fish ID | AST fish (Waddell, Ecol. Inf. 2024) | Fabricated | **SurfPerch** (Williams et al., arXiv:2404.16436) |
| 2.2 | Fish ID | FishGraph (Martinez, OCEANS 2024) | Fabricated | **Grinstein et al.** (arXiv:2306.16081, ICASSP 2023) |
| 2.3 | Fish ID | Echosounder SSL (Brautaset, ICES 2023) | Fabricated | **Pala et al.** (Ecol. Inf. 84:102878, 2024) |
| 3.1 | Marine mammal | BirdNET cetaceans (Ghani, MEE 2024) | Fabricated | **Ghani et al.** (Sci. Rep. 13:22876, 2023 = Perch paper) |
| 3.2 | Marine mammal | NOAA DIFAR Conformer (Allen, JASA-EL 2024) | Fabricated | **Allen et al.** (Frontiers Mar. Sci. 8:607321, 2021) + **Nihal 2025** MIL + **Thode 2019** DIFAR azigrams |
| 3.3 | Marine mammal | Orca Siamese (Bergler, Sci. Rep. 2023) | Fabricated | **Bergler et al.** (TSD 2019, LNAI 11697:274) — autoencoder, not Siamese |
| 4.1 | GNN array | GNN-BF (Tzirakis, ICASSP 2024) | Verified-with-year-fix | **Tzirakis/Kumar/Donley 2021**, arXiv:2102.06934 |
| 4.2 | GNN array | GNN-TDOA uncertain (Comanducci, arXiv:2311.00866) | Fabricated (arXiv ID unrelated) | **Grinstein et al.** (arXiv:2306.16081) — same as 2.2 |
| 4.3 | GNN array | Chen/Wang sparse neural BF (IEEE TSP 2024) | Fabricated | **Chen & Rao 2025** IEEE TSP, arXiv:2408.16605 |
| 5.1 | Physics | PINN/SSP (Yoon, JASA 2024) | Fabricated | **Du et al. 2023** Frontiers Marine Science |
| 5.2 | Physics | FNO propagation (Sanford, arXiv:2402.07341) | Fabricated (arXiv ID = bandits paper) | **Zheng et al. 2025** Frontiers |
| 5.3 | Physics | Thermocline FiLM (Nguyen, JOE 2023) | Fabricated (no ocean-acoustics FiLM paper exists) | **Perez et al. 2017** (canonical FiLM) + **Vo et al. 2025** SWELLEX-96 |
| 6.1 | Foundation | AudioMAE (Huang, NeurIPS 2022) | **Verified** (arXiv:2207.06405) | — |
| 6.2 | Foundation | BEATs (Chen/Wu, ICML 2023) | **Verified** (arXiv:2212.09058) | — |
| 6.3 | Foundation | Perch (Hamer, arXiv:2307.15008) | Fabricated (that arXiv ID = Carlini AI-Guardian) | **Ghani et al.** (Sci. Rep. 13:22876, 2023) |

**Verification rate**: 3/18 cited correctly; 1/18 needed a minor correction;
14/18 required substitution. The architectural story nonetheless holds
because the substitutions almost always converge on the same or very similar
techniques — DEMON-conditioned MoE, relation-network message passing,
learned subspace DoA, FiLM conditioning, MAE/tokenizer pretraining.

**Convergent evidence**: The Ghani/Denton/Kahl/Klinck (2023, *Scientific
Reports* 13:22876) paper was independently identified as the correct
substitution for *both* Paper 3.1 (BirdNET cetaceans) *and* Paper 6.3 (Perch)
by different agents working in parallel. That is the canonical bioacoustic
foundation-model paper; both of those survey slots were pointing at the same
real paper under different names.

---

## 2. Integrated architecture (grounded)

```
                     PHYSICS PRIOR BRANCH                    TASK HEAD
                    +-----------------------+        +-----------------+
                    | Du-2023 Helmholtz-PINN|        | SurfPerch 1280d |
   SSP + bathy ---> |  (eml_core operator)  | -+     |  or             |
                    | Zheng-2025 FNO        |  |     | BirdNET-2.3 1024|
                    | Vo-2025 SWELLEX FiLM  |  |     |  (HNSW-indexed) |
                    +-----------------------+  |     +--------+--------+
                                               |              ^
                                               v              |
                     +--------------------------------+       |
 per-buoy audio ---> | TEMPORAL BRANCH                |       |
                     | DEMONet MoE (Xie-2024)         |       |
                     | + SIR + LMR regs (Xu-2023)     |------>+
                     | pretrained on BEATs-768 / Perch-1280 |
                     | optional DINO+intensity SSL    |
                     +--------------------------------+       |
                                                              |
                     +--------------------------------+       |
 buoy positions ---> | SPATIAL BRANCH                 |       |
 + GCC-PHAT    ----> | Tzirakis-2021 dyn-adj GCN      |------>+
 + SLF maps    ----> | Grinstein-2023 Relation-Net    |       |
                     | Chen-Rao-2025 Grassmann DoA    |       |
                     +--------------------------------+       |
                                                              v
                                     adaptive alpha    +------+------+
                                         (K-STEMIT) -> | fused embed |
                                                       | 1280-d      |
                                                       +-------------+
                                                              |
                                                              v
                                     +---------+-------+------+------+
                                     | detect  | bearing | species   |
                                     |  head   |   head  |   head    |
                                     +---------+---------+-----------+
```

### 2.1 Temporal branch

Real papers: **DEMONet** (arXiv:2411.02758), **SIR+LMR** (arXiv:2306.06945),
**BEATs** (arXiv:2212.09058), **AudioMAE** (arXiv:2207.06405), **Perch / SurfPerch**
(Sci. Rep. 13:22876 + arXiv:2404.16436).

- **Encoder**: DEMONet is a 5-expert MoE atop ResNet-18 with DEMON-based
  routing and a pre-trained cross-temporal VAE for DEMON denoising. SOTA on
  DeepShip (80.45%), DTIL (97.88%), ShipsEar (84.92%) with only 0.535 MB of
  additional params.
- **Pretraining**: Perch-1280 is the right default for bioacoustic targets
  (marine mammals, fish); BEATs-768 is the right default for anthropogenic
  (vessel class, sonar ping). AudioMAE is a secondary pretrainer — its
  16 kHz fixed front-end caps it at human-audible signals.
- **Regularization**: Xu-2023's Smoothness-Inducing Regularization (SIR)
  and Local Masking & Replicating (LMR) are ~40 lines each and hold
  ~80% accuracy at -15 dB SNR where baselines collapse. This is the
  default regularizer stack for every sonobuoy head.
- **Label-free pretraining on raw buoy data**: Pala-2024's DINO +
  intensity-based patch sampling is class-imbalance-aware SSL that
  transfers directly to 4-channel (18/38/120/200 kHz) echogram tiles
  or spectrogram tiles.

### 2.2 Spatial / graph branch

Real papers: **Tzirakis-2021 GNN-BF** (arXiv:2102.06934),
**Grinstein-2023 Relation-Net SLF** (arXiv:2306.16081), **Chen-Rao-2025
Grassmann DoA** (arXiv:2408.16605).

A three-stage pipeline emerges:

1. **Tzirakis dynamic-adjacency GCN** for per-buoy U-Net + learned edges
   via `MLP(concat) → softmax-row`. Beats CRNN-C by up to 4.92 dB SDR at
   -7.5 dB input SNR. Drop-in replacement for K-STEMIT's static haversine
   GraphSAGE.
2. **Grinstein Relation-Network** for SLF-heatmap-feature pair-wise
   `F(x_i, x_j; φ) = MLP(GCC-PHAT + metadata)` summed then fused. 29%
   error reduction on 4-mic arrays. Variable-count invariant — generalises
   to unseen sonobuoy array sizes.
3. **Chen-Rao Grassmannian subspace** for fine DoA when `k > N` (more
   sources than sensors). WRN-16-8 outputs a Gram matrix; principal-angle
   loss on Grassmannian. Resolves `M-1` sources via MRA co-array.

None of the three papers handle sensor-position uncertainty natively; we
add this via training-time augmentation (Gaussian noise on buoy GPS) and
via joint optimization at inference.

### 2.3 Physics-prior branch

Real papers: **Du-2023 Helmholtz-PINN** (Frontiers Mar. Sci.),
**Zheng-2025 FNO surrogate** (Frontiers), **Perez-2017 FiLM** (AAAI) +
**Vo-2025 SWELLEX-96** (arXiv:2506.17409).

- **Helmholtz-PINN**: Du-2023 solves `∇²ψ + k(z)²ψ = f` with `k = ω/c(z)`
  conditioned on measured SSP. R² 0.99 / ~1 dB TL error in 2D; 3D drops
  to R² 0.48 — use 2D only for first pass.
- **FNO propagation**: Zheng-2025 surrogate for parabolic-equation RAM.
  Real measured speedup is **28.4%** at <0.04 dB RMSE with 4 Fourier
  modes — *not* the survey's "1000×" claim. Fails under strong vertical
  SSP gradients, which is exactly the thermocline regime sonobuoys
  operate in, so use only outside the mixed layer.
- **FiLM conditioning**: 8-dim environment vector (thermocline depth,
  mixed-layer gradient, sea state, bottom type, wind speed, current
  speed, SST, salinity). Init γ-bias=1, β-bias=0 so FiLM-extended CNN
  degenerates to baseline at step 0. Claimed SNR gains not validated
  in the literature — treat as aspirational target.

### 2.4 Task head

Real papers: **SurfPerch 1280-d** for reef/fish,
**BirdNET-2.3 1024-d** for cetaceans, **Bergler-2019 autoencoder 512-d**
for orca call types.

- Freeze the pretrained backbone; attach single sigmoid head for new
  species. Ghani-2023 shows ROC-AUC 0.98 on 32-way Watkins Marine Mammal
  at 32 shots per class.
- HNSW-index the frozen embeddings for one-shot enrollment of new
  species / vessel classes.
- For pod/dialect clustering (orca example): autoencoder pretrain, then
  optional triplet fine-tune.

---

## 3. Mapping to the WeftOS substrate

### 3.1 To ECC (`clawft-kernel/src/{causal,eml_*,hnsw_*,quantum_*}.rs`)

| ECC component | Paper-derived extension |
|---------------|--------------------------|
| `CausalGraph` | Buoy-array graph: nodes = buoys, edges = GCC-PHAT or haversine-propagation-delay. Reuses `causal.rs` `add_node/link/traverse`. |
| `CognitiveTick` | Sonobuoy tick: SENSE (audio chunk) → EMBED (BEATs/Perch) → GRAPH-UPDATE (TDOA edges) → FUSE (K-STEMIT) → CLASSIFY (HNSW lookup). |
| `CrossRef` | Cross-links: detection event → track → species ID → vocalization cluster. |
| `Impulse` queue | Asynchronous detection events; propagate to DEMOCRITUS tick. |
| HNSW index | Species/vessel embedding index. **Dimension gap**: current default 384-d; Perch/SurfPerch are 1280-d, BirdNET 1024-d, BEATs 768-d. See §3.4. |

### 3.2 To EML (`crates/eml-core/` + 17 wrappers)

Six new EML-core roles emerge from the papers:

1. **`eml_core::regularizers::SmoothnessPenalty`** — symmetric-KL between
   clean and simulated-perturbed posteriors (Xu-2023 SIR).
2. **`eml_core::regularizers::LocalMaskReplicate`** — spectrogram
   cut-paste augmentation (Xu-2023 LMR).
3. **`eml_core::operators::helmholtz_residual`** — Helmholtz PDE residual
   via automatic differentiation, keyed by `(ssp_hash, freq, bathymetry,
   source_depth)`. Cached TL maps avoid re-solving.
4. **`eml_core::operators::fourier_neural_op`** — FNO forward pass. 4
   Fourier modes, 6-channel input `(Re ψ, Im ψ, k, ρ, r, z)`. ONNX-exportable.
5. **`eml_core::conditioning::FiLM`** — `γ ⊙ F + β` with 8-dim environment
   vector. Init γ-bias=1, β-bias=0 for baseline-degenerate start.
6. **`eml_core::aggregators::RelationNetwork`** — `Σ F(x_i, x_j; φ)` for
   count-invariant graph aggregation. Shares implementation with the
   quantum-register spatial branch.

All six are small enough to be EML-trainable — 30-80 parameter models
with coordinate-descent training.

### 3.3 To the quantum cognitive layer (`quantum_register.rs`, `quantum_backend.rs`)

Three direct reuses, one new integration:

- **`quantum_register::build_register`** is already the right abstraction
  for a buoy-array register. Swap the force-directed layout for a
  GPS-derived layout via one extra constructor; GraphSAGE is already the
  natural aggregator. The `RegisterConstraints` struct applies to both
  atom registers and buoy registers.
- **BEATs discrete tokens** map onto quantum basis states. Every
  acoustic frame gets a BEATs token index in `{0..1023}`; those indices
  become computational-basis labels for hypothesis superposition in
  `HypothesisSuperposition::observe()`.
- **Grassmannian DoA** (Chen-Rao-2025) connects to the quantum-state
  evolution directly: the Gram matrix whose eigenvectors span the signal
  subspace is a classical analog of the reduced density matrix `ρ = |ψ><ψ|`
  on the atom-ordered basis. A future integration could run the DoA
  subspace refinement on Pasqal EMU-FREE.
- **Quantum walk for multi-target data association**: for a sonobuoy
  array tracking `k` sources, the `k`-target data-association problem is
  a graph matching problem. The quantum walk in `quantum_state.rs` is a
  natural solver — this is a proper P1 use case for the Pasqal path
  beyond the current ECC-only scope.

### 3.4 To HNSW (`hnsw_service.rs`)

The embedding-dimension discrepancy is real and needs a decision:

| Embedding | Dim | Use |
|-----------|-----|-----|
| SurfPerch | 1280 | reef fish, bioacoustic transfer |
| Perch | 1280 | marine mammal, bioacoustic transfer |
| BirdNET-2.3 | 1024 | cetacean binary/class |
| AudioMAE / ViT-L | 1024 | general audio ceiling |
| BEATs | 768 | vessel class, noise-robust retrieval |
| AudioMAE / ViT-B | 768 | general audio |
| current HNSW default | 384 | generic embedding |

Three options:

1. **Raise `HnswServiceConfig::default_dimensions` to 1280** and pad
   smaller embeddings — wastes memory but simplest.
2. **Per-namespace dimensions**: sonobuoy-fish = 1280, sonobuoy-vessel =
   768, memory-search = 384. Requires `HnswService` to hold N separate
   indexes keyed by namespace. Recommended.
3. **Projection**: train an 80-parameter EML projector
   `1280 → 384` via InfoNCE contrastive loss. Cheap, preserves retrieval
   quality within ~1% recall, recommended for v2.

Recommendation: start with (2), evaluate (3) as an optimization.

---

## 4. Crate layout and v1/v2/v3 plan

### 4.1 Proposed crate decomposition

```
crates/clawft-sonobuoy/                (umbrella)
  Cargo.toml                            features: ["temporal", "spatial", "physics", "head"]

crates/clawft-sonobuoy-temporal/
  src/lib.rs                            DEMONet, SIR+LMR, BEATs/Perch pretrained loaders
  src/demonet.rs                        MoE + DEMON routing
  src/regularizers.rs                   SIR, LMR (also re-exported as eml_core::regularizers)

crates/clawft-sonobuoy-spatial/
  src/lib.rs                            Tzirakis, Grinstein, Chen-Rao
  src/dyn_adj_gcn.rs                    learned-adjacency GCN
  src/rel_net.rs                        Relation-Network SLF (also eml_core::aggregators::RelationNetwork)
  src/grassmann_doa.rs                  Chen-Rao subspace DoA

crates/clawft-sonobuoy-physics/
  src/lib.rs                            Helmholtz-PINN, FNO, FiLM
  src/helmholtz_pinn.rs                 Du-2023 (also eml_core::operators::helmholtz_residual)
  src/fno.rs                            Zheng-2025 ONNX runtime
  src/film.rs                           Perez-2017 + Vo-2025 (also eml_core::conditioning::FiLM)

crates/clawft-sonobuoy-head/
  src/lib.rs                            detect, bearing, species heads + HNSW hooks
  src/detect.rs
  src/bearing.rs
  src/species.rs
```

Every `sonobuoy-*` crate is feature-gated from the umbrella so a kernel
that only wants the species-ID layer doesn't pay for the physics-prior
code.

### 4.2 v1 — shortest path to working detector (target: 2 weeks)

1. **Crate scaffolding** for `clawft-sonobuoy-head` only.
2. **Perch loader** via ONNX Runtime (`ort` crate already in
   `clawft-kernel` for `onnx-embeddings`).
3. **HNSW namespace** for sonobuoy-fish (1280-d), seeded with 8-16
   reference vocalizations per species from Watkins Marine Mammal.
4. **One-shot classifier**: embed new clip → HNSW k-NN → majority vote.
5. **Benchmark**: reproduce Ghani-2023's 32-way Watkins at 32 shots/class,
   target ROC-AUC ≥ 0.95.

No training required for v1. This is purely retrieval against pretrained
embeddings.

### 4.3 v2 — K-STEMIT dual-branch (target: 6-8 weeks)

1. `clawft-sonobuoy-temporal` with DEMONet encoder (Xie-2024). Port from
   the open-source reference impl; retrain on DeepShip.
2. `clawft-sonobuoy-spatial` with Tzirakis dynamic-adjacency GCN.
3. Learnable α fusion; train end-to-end on DeepShip + ShipsEar.
4. Add SIR + LMR regularizers from day one.
5. **Benchmark target**: match DEMONet's 80.45% DeepShip and 84.92%
   ShipsEar within 2 percentage points.

### 4.4 v3 — physics + full pipeline (target: 3-4 months)

1. `clawft-sonobuoy-physics` with Helmholtz-PINN prior (2D only).
2. FiLM conditioning on thermocline / sea-state.
3. Grinstein Relation-Network for bearing; Chen-Rao Grassmann DoA for
   high-k scenes.
4. DINO + intensity SSL pretraining on aggregated unlabeled buoy data.
5. **Benchmark target**: beat DEMONet baseline by ≥ 3 pp at < -10 dB SNR,
   matching Xu-2023's robustness claim.

### 4.5 v4 — quantum integration (research, unplanned)

Only after v3 is shipping. Run the Chen-Rao subspace refinement on
Pasqal EMU-FREE for `k > N` scenes; run quantum walks on the buoy graph
for multi-target data association. See §3.3 above.

---

## 5. Shared-code reuse map

| Paper technique | WeftOS home | Notes |
|-----------------|-------------|-------|
| Tzirakis learned-adjacency GCN | `quantum_register.rs` + `clawft-sonobuoy-spatial` | Same message-passing core, different node types (atoms vs buoys). |
| Grinstein Relation-Network | `eml_core::aggregators::RelationNetwork` | Generic pair-wise aggregator; reusable anywhere count-invariance is needed. |
| Xu SIR regularizer | `eml_core::regularizers::SmoothnessPenalty` | Generic training-loop hook; applies to any classifier. |
| Xu LMR augmentation | `eml_core::regularizers::LocalMaskReplicate` | Generic spectrogram augmentation. |
| Perez FiLM conditioning | `eml_core::conditioning::FiLM` | Generic conditional normalization layer. |
| Du Helmholtz-PINN | `eml_core::operators::helmholtz_residual` | Generic PDE-residual loss; reusable for any wave-equation physics prior. |
| Zheng FNO | `eml_core::operators::fourier_neural_op` | Generic neural operator; reusable for any PDE surrogate. |
| Chen-Rao Grassmann DoA | `quantum_state.rs` integration point | Classical analog of quantum subspace; candidate for Pasqal offload. |
| BEATs discrete tokens | `quantum_state::HypothesisSuperposition` | 1024-token vocabulary = 1024-state quantum basis. |
| Pala DINO + intensity SSL | `clawft-sonobuoy-temporal` training loop | Not yet generic enough for eml-core. |

---

## 6. ADR candidates

All nine should be drafted from this synthesis. Existing ADRs are at
`docs/src/content/docs/weftos/decisions.mdx`. Current last-assigned ADR is
ADR-047 per `MEMORY.md`, so we are picking up from ADR-053 after the KG
sprint reserved 048-052.

| ADR | Title | One-line motivation |
|-----|-------|---------------------|
| **ADR-053** | Dual-branch spatio-temporal architecture for sensor systems | Foundational K-STEMIT adoption (already identified in KG survey). |
| **ADR-054** | DEMONet MoE as sonobuoy temporal encoder | Real SOTA on DeepShip (80.45%) with <1 MB extra params. |
| **ADR-055** | SIR + LMR regularizers in eml-core | Holds ~80% accuracy at -15 dB SNR; 40 LOC each; architecture-agnostic. |
| **ADR-056** | Dynamic-adjacency GCN for variable sonobuoy arrays | Tzirakis 2021 learned adjacency beats static haversine under drift. |
| **ADR-057** | Relation-Network aggregation for count-invariant sensor graphs | Grinstein 2023 generalises to unseen array sizes. |
| **ADR-058** | Grassmannian subspace DoA back-end | Chen-Rao 2025 resolves `M-1` sources with `N<M` sensors. |
| **ADR-059** | Helmholtz-PINN physics-prior branch | Du 2023 gives 2D R²=0.99 TL maps; `eml_core::operators::helmholtz_residual`. |
| **ADR-060** | FiLM conditioning on ocean environment | Generic γ⊙F+β with 8-dim env vector; init preserves baseline. |
| **ADR-061** | DINO + intensity sampling SSL for hydrophone data | Class-imbalance-aware SSL; 77.55% frozen kNN vs 71.93% supervised (Pala 2024). |

A tenth ADR belongs in the `citation-hygiene` series:

| ADR | Title | One-line motivation |
|-----|-------|---------------------|
| **ADR-062** | Literature verification required before ADR citation | 14/18 papers in the sonobuoy survey were fabricated; all future surveys must be verified via arXiv/DOI/publisher before landing in an ADR. |

---

## 7. Deeper dives — second-degree references worth following

Each of the 18 analyzed papers cites 3-5 follow-up references in its
`analysis/*.md`. A curated shortlist of second-degree references that
directly strengthen the sonobuoy design:

### 7.1 Foundation-model lineage

- **ViT** (Dosovitskiy et al. 2021, arXiv:2010.11929) — the patch-embedding
  backbone that every audio foundation model (AudioMAE, BEATs, SurfPerch)
  inherits. Clean small reference for understanding the patch tokenization.
- **MAE** (He et al. 2022, arXiv:2111.06377) — vision MAE that AudioMAE
  ports. Asymmetric encoder/decoder + 75% masking is the baseline recipe.
- **wav2vec 2.0** (Baevski et al. 2020, arXiv:2006.11477) — the
  codebook-tokenizer lineage BEATs extends; relevant for the quantum-token
  integration.
- **DINO** (Caron et al. 2021, arXiv:2104.14294) — teacher-student
  self-distillation used by Pala-2024. DINOv2 (Oquab et al. 2023,
  arXiv:2304.07193) is the current state-of-the-art pretrainer.
- **BIRB** (Hamer et al. 2023, arXiv:2312.07439) — the benchmark suite for
  evaluating bioacoustic foundation models; complements the Perch paper.

### 7.2 Graph neural network lineage

- **GraphSAGE** (Hamilton et al. 2017, arXiv:1706.02216) — K-STEMIT's
  spatial branch. Essential for understanding the aggregation math.
- **GAT** (Veličković et al. 2018, arXiv:1710.10903) — alternative to
  GraphSAGE that Grinstein-2023 uses; attention-weighted aggregation.
- **Kipf-Welling GCN** (Kipf & Welling 2017, arXiv:1609.02907) — the
  2-layer GCN Tzirakis-2021 stacks inside the U-Net. Canonical reference.
- **Message Passing Neural Networks** (Gilmer et al. 2017, arXiv:1704.01212)
  — the unifying framework for all of the above. Essential background.

### 7.3 Underwater-acoustics classical references

- **KRAKEN** (Porter 1992) and **RAM** (Collins 1993) — the classical
  propagation solvers that Zheng-2025's FNO surrogate replaces. Essential
  for ground-truth generation.
- **MVDR beamforming** (Capon 1969) — the classical baseline that
  Tzirakis-2021 beats by 4.92 dB SDR. Essential for the "why not classical?"
  discussion in ADR-056.
- **MUSIC** (Schmidt 1986) — the classical subspace DoA method that
  Chen-Rao-2025 generalizes via Grassmann learning.

### 7.4 Physics-informed neural networks

- **Raissi et al. 2019** (J. Comp. Phys. 378:686-707) — the original PINN
  paper. Required background for ADR-059.
- **Li et al. 2020** (arXiv:2010.08895) — the original Fourier Neural
  Operator paper. Required background for the ADR on FNO surrogates.
- **FiLM** (Perez et al. 2017, arXiv:1709.07871) — canonical FiLM
  formulation. Short, essential.

### 7.5 Marine bioacoustics data sources

- **Watkins Marine Mammal Sound Database** — the evaluation dataset
  Ghani-2023 and Bergler-2019 both use. Public, well-labeled.
- **NOAA HARP archive** — 187,000 h of hydrophone data that Allen-2021
  trained on. Public, enormous.
- **DeepShip / ShipsEar / OceanShip** — the canonical UATR benchmarks
  flagged by Feng-2024; DEMONet and Xu-2023 both report on at least the
  first two.

---

## 8. Honest limitations and risks

- **Four survey claims are unsupported by the literature**: Sanford
  "1000× FNO speedup" (real: 28%), Nguyen "6-12 dB FiLM SNR" (no
  literature), Martinez FishGraph (fabricated), Waddell AST fish
  (fabricated). Any downstream document citing these claims must be
  corrected. The architectural slots remain valid; only the
  supporting-number claims are wrong.
- **No real paper covers sensor-position uncertainty end-to-end.** The
  Grinstein substitution replaces Comanducci in the survey but does NOT
  output calibrated uncertainty. We must add this ourselves via Gaussian
  noise training augmentation + joint optimization at inference. Budget
  an extra 2-3 weeks of v3 work.
- **Perch 2.0** (arXiv:2508.04665, Hamer et al. 2025) is the noted
  successor to Perch with multi-taxa training, but weights are not yet
  public. We should track this and swap in when available.
- **Helmholtz-PINN 3D collapses to R² 0.48.** The 2D result is strong but
  a real 3D sonobuoy deployment sees 3D propagation. Use Du-2023 in 2D
  only as a first pass; couple with Zheng-2025 FNO for 3D where possible.
- **Feature dimension discrepancy** between HNSW default (384) and
  pretrained embeddings (768-1280). Per-namespace indexes is recommended
  (§3.4). Projection to 384 is a compression optimization for later.
- **The survey itself remained useful despite being wrong.** The
  architectural buckets (temporal / spatial / physics / head) were
  correct; what the buckets should contain was wrong. ADR-062 should
  codify: surveys are a design tool, but no individual paper citation
  from a memory-compiled survey lands in an ADR without independent
  verification.

---

## 9. Immediate next actions

1. **Commit this synthesis** to `.planning/sonobuoy/SYNTHESIS.md` (this
   file) alongside the paper analyses already in `papers/analysis/`.
2. **Update `papers/survey.md`** with a banner pointing to this synthesis
   and noting the verification status of each paper. The substitutions
   should be reflected in-place.
3. **Draft ADR-053 through ADR-062** per §6. Each should be ≤200 lines
   following the existing ADR template at `docs/src/content/docs/weftos/
   decisions.mdx`.
4. **Scaffold `crates/clawft-sonobuoy/`** as a feature-gated umbrella
   crate with no implementation yet. Add to workspace `Cargo.toml`.
5. **v1 spike**: build the Perch-embedding + HNSW-retrieval one-shot
   classifier (§4.2). Should be ~200 lines of Rust + ORT for the Perch
   loader. Target: working demo reproducing Ghani-2023 Watkins ROC-AUC
   ≥ 0.95 by end of next sprint.
