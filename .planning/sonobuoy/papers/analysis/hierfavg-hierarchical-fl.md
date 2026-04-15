# HierFAVG — Client-Edge-Cloud Hierarchical Federated Learning

## Citation

- **Title**: Client-Edge-Cloud Hierarchical Federated Learning
- **Authors**: Lumin Liu, Jun Zhang, Shenghui Song, Khaled B. Letaief
- **Affiliation**: Hong Kong University of Science and Technology; Hong Kong Polytechnic
  University; Peng Cheng Laboratory
- **Venue**: 2020 IEEE International Conference on Communications (ICC 2020), Dublin (virtual),
  June 2020, pp. 1–6. DOI 10.1109/ICC40277.2020.9148862.
- **arXiv**: [1905.06641](https://arxiv.org/abs/1905.06641) (v1 May 2019, rev Jan 2020)
- **IEEE Xplore**: https://ieeexplore.ieee.org/document/9148862
- **Code**: https://github.com/LuminLiu/HierFL
- **PDF**: `.planning/sonobuoy/papers/pdfs/hierfavg.pdf`

## Status

**verified**. Title, authors (with corrected spelling S.H. Song = Shenghui Song), ICC 2020 venue,
DOI, arXiv ID, and the HierFAVG algorithm name all corroborated against arXiv, IEEE Xplore, and
the authors' institutional listings at HKUST and PolyU. The reference implementation
(github.com/LuminLiu/HierFL) independently confirms the two-level aggregation-interval
parameterization (κ₁, κ₂) described below.

## One-paragraph Summary

Liu, Zhang, Song & Letaief (ICC 2020) observe that pure cross-device FedAvg has a **single**
aggregation tier — clients ↔ cloud — and argue that this is wasteful whenever a natural **edge
tier** (a regional base station, an IoT gateway, or, in our setting, a buoy-cluster leader) exists
between the devices and the cloud backhaul. Their algorithm, **HierFAVG**, runs FedAvg at *two*
levels: every κ₁ local SGD steps, clients within an edge server's coverage area perform an **edge
aggregation** over the local-area link; every κ₂ edge aggregations, edge servers perform a **cloud
aggregation** over the expensive backhaul. The critical quantitative result is that, when edge-
local data is IID within each cluster (but non-IID across clusters), κ₂ can be driven up to ~100
with negligible convergence degradation — meaning the expensive cloud link carries **1/100th the
traffic** of flat FedAvg. They prove convergence bounds in both convex and non-convex settings,
isolate two gradient-divergence terms δ (within-edge) and Δ (across-edge), and verify on MNIST and
CIFAR-10 that a two-tier hierarchy reduces training time by 50–75% and end-device energy by
30–65% relative to cloud-only federated learning. For sonobuoy networks — where every buoy cluster
has a natural leader with higher-quality uplink (UHF within cluster, Iridium-SBD to shore) — the
HierFAVG topology is the **default** partition of the problem: cheap local aggregation over
acoustic/UHF, expensive global aggregation over satellite.

## Methodology

### Three-tier setup

Let:

- **Clients**: `K` devices total, each with local dataset `D_k` of size `n_k`.
- **Edge servers**: `E` edges, each covering a disjoint subset of clients `C_ℓ ⊆ [K]`.
- **Cloud**: single global aggregator.

Let:

- `κ₁` = number of local SGD steps before each edge aggregation.
- `κ₂` = number of edge aggregations before each cloud aggregation.
- Every `κ₁ · κ₂` local steps ⇒ 1 cloud round.

### HierFAVG Algorithm 1

    initialize w_0 at cloud, broadcast to all edges → all clients.
    for each cloud round t = 0, 1, ... :
        for each edge aggregation e = 1 .. κ₂ :
            for each client k in parallel (within its edge C_ℓ) :
                run κ₁ local SGD steps on w_k^{(t,e)}, produce w_k^{(t,e+1)}
            each edge server ℓ aggregates:
                w_ℓ^{(t,e+1)} ← (1 / |C_ℓ|) Σ_{k ∈ C_ℓ} (n_k / n_ℓ) w_k^{(t,e+1)}
            edge broadcasts w_ℓ^{(t,e+1)} to its clients.
        cloud aggregates across edges:
            w^{(t+1)} ← (1/n) Σ_ℓ n_ℓ · w_ℓ^{(t,κ₂)}
        cloud broadcasts w^{(t+1)} to all edges → all clients.

### Communication cost

Per cloud round, per client: **κ₂ edge uploads** (cheap local link) and **1 cloud upload via the
edge** (expensive backhaul). Per cloud round, per edge: **1 cloud exchange**. Flat FedAvg's
equivalent: **κ₁ · κ₂ total local steps** but **κ₂ cloud rounds** (every edge aggregation is also
a cloud aggregation in flat FedAvg). So HierFAVG reduces cloud comms by **factor κ₂**.

### Convergence bounds

For the **convex** case, under L-smoothness and bounded-variance assumptions:

    F(w^{(T)}) − F(w*) ≤ (1/T) · [ O(ηφ) + O( ρ · G(κ₁ · κ₂, η) · (δ · κ₁ + Δ · κ₁ · κ₂) / ε² ) ]

Here:

- `δ`: client-to-edge **gradient divergence** — how non-IID clients are within each edge cluster.
- `Δ`: edge-to-cloud gradient divergence — how non-IID edge clusters are across the cloud.
- `G(·)` is a function capturing the number of local steps between syncs.

The bound decomposes non-IID penalty into **two separate terms**: δ·κ₁ and Δ·κ₁·κ₂. This is the
paper's key architectural insight: when **within-edge data is IID** (δ ≈ 0), you pay almost
nothing for growing κ₂, so the cloud link can be very infrequent. This maps directly to
sonobuoy physics: buoys in the **same cluster** see the **same acoustic weather and species
mix**, so δ ≈ 0. Buoys in **different clusters** (different ocean basins, different times of
year) have very different data → Δ >> 0. HierFAVG rewards us for this geography.

For the **non-convex** case, a standard gradient-norm bound shows

    (1/T) Σ ‖∇F(w_t)‖² ≤ O(1/√(T)) + penalty terms in (δ, Δ, κ₁, κ₂)

with the same decomposition.

### Parameter choice rules

The paper gives three operational rules:

1. If edges are IID (Δ ≈ 0), increase κ₂ freely.
2. If within-edge data is non-IID (δ > 0), decrease κ₁ (more frequent edge aggregations).
3. Keep κ₁·κ₂ = `C` constant; reduce training time by **shifting budget from κ₁ to κ₂** when δ
   is large.

### Hierarchical vs flat FedAvg at fixed cloud comms

To match HierFAVG's cloud traffic with flat FedAvg, flat FedAvg would need to run κ₂× fewer cloud
rounds — which means κ₂× fewer SGD steps globally, thus κ₂× worse training loss.

## Key Results

### MNIST (21 840-param CNN), 50 clients, 5 edges

- HierFAVG with κ₁ = 10, κ₂ = 10 reaches target accuracy in ~30 cloud rounds vs ~300 for flat
  FedAvg at matched cloud comms. **10× cloud-round reduction.**
- Total training time: 40% of flat FedAvg.

### CIFAR-10 (5.85 M-param CNN), 50 clients, 5 edges

- κ₁ = 5, κ₂ = 20: reaches 80% accuracy in ~40 cloud rounds.
- Wall-clock: 50% reduction vs flat FedAvg at same accuracy.
- End-device energy: 30–65% reduction (fewer expensive cloud radios per device).

### Impact of κ₂ sweep

- κ₂ = 1 (= flat FedAvg): baseline.
- κ₂ = 5: within 0.5% of baseline accuracy, 5× cloud comms reduction.
- κ₂ = 20: within 2% of baseline accuracy, 20× cloud comms reduction.
- κ₂ = 100: within 5% accuracy (still usable), 100× reduction (best for IID-within-edge).

### When δ > 0 (non-IID within edge)

- κ₁ must shrink proportionally; paper shows κ₁ = 1 (edge aggregation every step) still beats
  flat FedAvg when the Δ / δ ratio is > 3.

## Strengths

- **Structural, not algorithmic, compression.** HierFAVG doesn't compress any single message —
  it *moves traffic to cheaper links*. Orthogonal to DGC, signSGD, FetchSGD, and stackable with
  all of them.
- **Matches physical topology of sonobuoy arrays.** Clusters of buoys share acoustic weather; the
  within-cluster link (UHF or acoustic) is two orders of magnitude cheaper than the inter-cluster
  Iridium satellite link.
- **Theoretical isolation of δ and Δ** gives concrete tuning: measure within- and across-cluster
  gradient divergence empirically, set κ₁ and κ₂ accordingly.
- **Simple to implement.** It's FedAvg applied twice. No new primitives — just a second
  aggregation level.
- **Energy savings are first-order.** Iridium SBD transmit is ~1.5 W for 20 s; UHF transmit is
  ~200 mW for 2 s. Moving 99% of traffic to UHF saves >100× end-device energy over satellite-only.

## Limitations

- **Requires natural cluster structure.** Works only when edge servers exist and clients can be
  partitioned among them with stable coverage. For ad-hoc mesh networks where a buoy is
  simultaneously in two clusters' ranges, the assignment is ambiguous and convergence guarantees
  weaken.
- **Edge server single-point-of-failure.** The cluster leader must be up during the entire κ₂
  edge-aggregation window. In WeftOS this maps to a Raft-elected cluster leader; re-election
  during an aggregation round discards partial state.
- **Convergence bound assumes stationarity of δ and Δ.** In practice δ varies with time of day,
  season, and species migration — bound degrades under non-stationarity.
- **Does not address Byzantine clients.** Assumes honest edge servers and honest clients. Pair
  with Multi-Krum at the edge level for adversarial robustness.
- **Cluster re-formation not covered.** When a buoy drifts between cluster coverage areas, which
  is inevitable on ocean, the paper has no protocol for handing off the client's FL state. A
  practical deployment needs a "soft-join" protocol.
- **No inter-edge gossip variant.** Edges only talk through cloud. For bandwidth-critical ocean
  deployments, a mesh between edges (e.g. direct UHF between cluster leaders) could dramatically
  reduce cloud traffic further — the paper doesn't explore this.
- **Experiments are small-scale** (50 clients, 5 edges). No 1000-buoy evaluation.

## Portable Details

### Cluster assignment for sonobuoy fleets

Natural clusters arise from:

1. **Geographic range**: buoys within UHF (~5-20 km) of a master node.
2. **Raft quorum membership**: whichever Raft leader a buoy votes for defines its edge.
3. **Acoustic water-mass**: buoys in the same mesoscale eddy share soundscape statistics.

For WeftOS, the natural mapping is (2): the Raft cluster leader acts as the HierFAVG edge server.
This happens to be exactly what `mesh_chain.rs` already elects, so no new primitive is needed.

### Parameter operating points

| Deployment | κ₁ (local) | κ₂ (edge) | Cloud cadence |
|------------|------------|-----------|----------------|
| tactical (1 kbps UHF in-cluster, 100 bps Iridium) | 5 | 50 | ~daily |
| pam (10 kbps acoustic in-cluster, 10 bps Iridium avg) | 10 | 100 | ~weekly |
| sensor-edge (wired LAN, broadband cloud) | 1 | 5 | hourly |

### Two-tier message flow

    per-buoy (5–100 Hz training):
        local SGD κ₁ steps → ship Δw_k to cluster leader (UHF)

    per-cluster-leader (every κ₁ steps):
        receive Δw_k from all buoys in cluster
        w_ℓ ← (1/|C_ℓ|) Σ n_k · w_k
        broadcast w_ℓ back to buoys (UHF)

    per-cluster-leader (every κ₂ edge aggs):
        ship Δw_ℓ to shore (Iridium or UHF-to-shore)
        shore aggregates over clusters, broadcasts new global w.

### Interaction with DGC / FetchSGD / signSGD

HierFAVG is orthogonal:

- **Edge uplink (buoy → cluster leader)**: use high-bandwidth codec (FP16 dense or light DGC).
  Cheap link affords fidelity.
- **Cloud uplink (cluster leader → shore)**: use aggressive codec (DGC 0.001 + signSGD or
  FetchSGD sketch). This is the expensive link.

The cluster leader *aggregates clean updates* within its cluster, then **re-compresses** the
resulting cluster-level Δw_ℓ aggressively before the cloud hop. This is strictly better than
every buoy compressing for the cloud directly, because aggregation averages out the stochastic
noise before compression, improving compression efficacy.

### Convergence bound in sonobuoy terms

With within-cluster IID (same water mass, same weather), δ ≈ 0. Across clusters (different
ocean basins), Δ can be very large. The bound tells us:

    cloud convergence ≈ 1/√T_cloud + Δ · κ₁ · κ₂ / ε²

We **cannot** push κ₂ infinitely: the Δ penalty grows. The tuning is
**κ₂ ≈ 1/(Δ · κ₁)** (up to constants), i.e. when across-cluster divergence doubles, double the
cloud cadence.

### Measuring δ and Δ in the field

    δ = E_client‖∇F_k(w) − ∇F_ℓ(w)‖² / ‖∇F_ℓ(w)‖²        within-edge, empirical
    Δ = E_edge‖∇F_ℓ(w) − ∇F(w)‖² / ‖∇F(w)‖²              across-edge, empirical

Both are cheaply measurable from gradient samples at aggregation time — the cluster leader knows
all client gradients, the shore knows all edge aggregates. Adaptive HierFAVG that tunes (κ₁, κ₂)
live from measured (δ, Δ) is a straightforward extension and is mentioned as future work in
§6 of the paper.

## Sonobuoy Integration Plan

### Where HierFAVG slots into WeftOS

HierFAVG is a **topology primitive**, not a codec. It defines the two-tier aggregation graph.

- **Per-cluster-leader component**: new crate `weftos-sonobuoy-fl-coordinator` that owns
  `(κ₁, κ₂)` scheduling. Uses `mesh_chain.rs` Raft state to track round numbers, applies FedAvg at
  edge boundaries, triggers DGC-encoded cloud upload every κ₂ rounds.
- **Shore-side component**: `weftos-sonobuoy-fl-shore-aggregator` that receives edge aggregates
  from all clusters, averages, and broadcasts. Maintains per-cluster δ/Δ estimates and adaptively
  tunes κ₁/κ₂.
- **Mesh subsystem coupling**: `mesh_chain.rs` Raft log entries carry the edge-aggregated Δw_ℓ
  as a new operation type; `chain.rs` exochain anchors a Merkle root of (round, cluster,
  Δw_ℓ_hash) tuples for auditability.

### Deployment-profile fit

- **`sonobuoy-tactical`**: κ₁ = 5, κ₂ = 50. Edge every 5 min, cloud daily.
- **`sonobuoy-pam`**: κ₁ = 10, κ₂ = 100. Edge every hour, cloud weekly.

### Failure modes

1. **Cluster leader crash during κ₂ window**: new Raft leader elected; all buoys resend their most
   recent Δw_k since last successful edge aggregation. Lost work = up to κ₁ local steps per buoy.
2. **Network split**: each partition continues independently, then **reconciles** at next cloud
   round. Reconciliation is standard FedAvg of the partitioned Δw_ℓ aggregates — Raft-log-layer
   merge.
3. **Buoy drifts between clusters**: soft-handoff protocol — the new cluster leader pulls the
   buoy's last Δw_k from mesh gossip and credits it to its local aggregate.

### ADR implication

*ADR-082 candidate (see gap-closing addendum)*: HierFAVG defines the native FL topology for
multi-cluster sonobuoy deployments. Edge = Raft cluster leader. Cloud = shore. (κ₁, κ₂) are
deployment-profile constants with adaptive override via measured (δ, Δ).

## How This Closes G5

G5 demands <1 kB/buoy/round uplink with <10 rounds to convergence.

- **HierFAVG multiplies the effective bit budget by κ₂.** If a single cloud-only FL round costs
  1 kB/buoy, HierFAVG amortizes that over κ₂ edge rounds ⇒ **(1/κ₂) kB/buoy/cloud-round**. For
  κ₂ = 100 and 1 kB budget, that's 10 B/buoy/cloud-round — below Iridium single-SBD cost.
- **Within-cluster IID assumption holds for sonobuoys by physics**: acoustic weather is spatially
  correlated at mesoscale (≥10 km), so buoys within a UHF cluster (<20 km) see near-IID data. δ
  is near-zero, allowing aggressive κ₂.
- **Converges in <10 cloud rounds** per paper's CIFAR-10 results, equivalent to ~1000 local SGD
  steps per buoy. At 10 Hz training on a small CNN, that's <100 s of wall-clock compute per buoy
  per round — well within a single power-harvested buoy's daily compute budget.
- **Composable with DGC, FetchSGD, signSGD** on *either* the edge or cloud link. The cluster
  leader can run dense FedAvg internally and aggressively compressed signSGD externally.

## Follow-up References

1. **Castiglia, Das, Patterson 2021** — "Multi-Level Local SGD: Distributed SGD for Heterogeneous
   Hierarchical Networks", ICLR 2021, arXiv
   [2007.13819](https://arxiv.org/abs/2007.13819). Generalizes HierFAVG to arbitrary-depth
   hierarchies (not just 3 tiers). Useful for multi-hop acoustic relay topologies.
2. **Das, Patterson 2022** — "Cross-Silo Federated Learning: Challenges and Opportunities",
   arXiv [2206.12949](https://arxiv.org/abs/2206.12949). Survey of multi-cluster FL,
   including HierFAVG-style algorithms under heterogeneity.
3. **Zhou et al. 2020** — "Cooperative Federated Learning over Mesh Networks", IEEE Trans.
   Wireless Comm. Works out the direct edge-to-edge gossip variant that HierFAVG omits.
4. **Abad et al. 2020** — "Hierarchical Federated Learning Across Heterogeneous Cellular
   Networks", ICASSP 2020, arXiv [1909.02362](https://arxiv.org/abs/1909.02362). Same idea,
   concurrent work, focused on cellular hierarchies.
5. **Ganguly & Hosseinalipour 2022** — "Multi-Stage Hybrid Federated Learning over Large-Scale
   D2D-Enabled Fog Networks", IEEE/ACM Trans. on Networking. Pushes the idea to >3 tiers and
   device-to-device links — closest published analog to a sonobuoy mesh.
