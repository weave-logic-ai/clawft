# FetchSGD — Communication-Efficient Federated Learning with Sketching

## Citation

- **Title**: FetchSGD: Communication-Efficient Federated Learning with Sketching
- **Authors**: Daniel Rothchild, Ashwinee Panda, Enayat Ullah, Nikita Ivkin, Ion Stoica,
  Vladimir Braverman, Joseph Gonzalez, Raman Arora
- **Affiliation**: UC Berkeley; Johns Hopkins; Amazon Research
- **Venue**: Proceedings of the 37th International Conference on Machine Learning (ICML 2020),
  PMLR 119:8253-8265
- **arXiv**: [2007.07682](https://arxiv.org/abs/2007.07682) (v1 Jul 2020, v2 Oct 2020)
- **PMLR**: https://proceedings.mlr.press/v119/rothchild20a.html
- **ICML virtual poster**: https://icml.cc/virtual/2020/poster/6724
- **PDF**: `.planning/sonobuoy/papers/pdfs/fetchsgd.pdf`

## Status

**verified**. Title, 8-author list, ICML 2020 venue, arXiv ID, and the PMLR v119 proceedings entry
all corroborated against arXiv abstract, PMLR proceedings page, and ICML 2020 poster listing. This
is the canonical Count-Sketch-for-FL paper.

## One-paragraph Summary

Rothchild, Panda, Ullah, Ivkin, Stoica, Braverman, Gonzalez & Arora (ICML 2020) observed that the
two dominant classes of communication-efficient FL algorithms — **local Top-k** (DGC-style) and
**local SGD** (FedAvg-style) — both require clients to maintain *state across rounds* (residual
buffers, accumulated local momentum). For cross-device FL, where each client may participate in
**one round every few thousand**, that state is stale-to-useless. FetchSGD's insight is that a
**Count Sketch is a linear sketch** — so momentum and error accumulation can be done **on the
server, inside the sketch**, eliminating the need for client-side state entirely. Each client
sketches its raw gradient into a fixed-size structure (`r` rows × `c` columns, typically a few MB
regardless of model size), uploads it, and the server adds it to a running sketch whose entries
are the coordinate-wise momentum + error-corrected gradient accumulator. At the end of each round
the server extracts the Top-k coordinates by *unsketching*, applies them to the model, and zeroes
those coordinates in the sketch (the error-feedback step). FetchSGD achieves up to **100× upload
compression** at accuracy parity with uncompressed federated SGD on PersonaChat (GPT-2 finetune),
CIFAR-10/100, and FEMNIST. For sonobuoys with rare participation (buoy gets one SBD window per
day), this property — that the client never needs to remember anything between rounds — is the
paper's decisive contribution.

## Methodology

### The Count Sketch primitive

A Count Sketch is a data structure parameterized by `r` hash rows and `c` bucket columns, with:

- `r` independent pairwise-independent hashes `h_i : [d] → [c]`
- `r` independent sign hashes `s_i : [d] → {−1, +1}`
- Storage: `r × c` floats — the sketch matrix `S`.

**Insert**: `S[i, h_i(j)] ← S[i, h_i(j)] + s_i(j) · x_j`   for all i, j.

**Query**: estimate `x̂_j = median_i ( s_i(j) · S[i, h_i(j)] )`.

Key properties:

1. **Linearity**: `Sketch(x + y) = Sketch(x) + Sketch(y)`. Sums of sketches are sketches of sums.
2. **Unbiased median estimate**: `E[x̂_j] = x_j`.
3. **Concentration** (Count Sketch theorem): for `c = O(1/ε²)` columns and `r = O(log(1/δ))`
   rows, `|x̂_j − x_j| ≤ ε‖x‖₂` with probability `≥ 1 − δ`.
4. **Top-k extraction**: given `Sketch(x)`, one can recover the Top-k coordinates of `x`
   (approximately) in `O(d log d)` server time — the *unsketch* operation.

### FetchSGD algorithm (roughly)

    # Server state (persists across rounds):
    S_mom ← 0          # sketch of EMA momentum
    S_err ← 0          # sketch of residual (error-feedback buffer)
    θ     ← init

    for each round t:
        S_t ← 0
        for each sampled client k in round t:
            g_k ← client computes local gradient on (θ, batch_k)
            Sketch(g_k) = CountSketch(g_k, r, c)     # r x c floats
            upload Sketch(g_k) to server
            S_t ← S_t + Sketch(g_k)                  # server aggregation

        # server-side momentum + error feedback IN THE SKETCH:
        S_mom ← ρ · S_mom + S_t
        S_corrected ← S_mom + S_err
        (i_1..i_k, v_1..v_k) ← TopK( unsketch(S_corrected, k) )
        apply_sparse_update(θ, (i_j, v_j))
        S_err ← S_corrected                          # carry residual forward
        zero out indices (i_1..i_k) in S_err         # remove applied component

        broadcast θ to next round's clients

The critical line is "server-side momentum + error feedback IN THE SKETCH". Because the sketch is
linear, `Sketch(a + b) = Sketch(a) + Sketch(b)`, and updates to the momentum/error state can be
done without ever materializing the full FP32 gradient vector on the server — the sketch *is* the
state. Clients are stateless between rounds.

### Communication cost per client per round

    uplink = r × c × 4 bytes (FP32)

For r = 5, c = 10⁵ (typical), uplink ≈ 2 MB. For r = 5, c = 10⁴, uplink ≈ 200 kB. For r = 1, c =
10³ (aggressive), uplink ≈ 4 kB.

**Critical**: uplink is **independent of `|θ|`**. A 100 M-parameter GPT-2 and a 10 k-parameter
buoy CNN ship the same sketch size if they are configured with the same `(r, c)`. Compression
ratio = `|θ| · 4 B / (r · c · 4 B) = |θ| / (r · c)`.

### Convergence theorem (informal)

Under standard non-convex smoothness and bounded-variance assumptions, FetchSGD with appropriately
tuned momentum and Top-k recovery converges at rate

    (1/T) Σ E‖∇F(θ_t)‖² ≤ O( 1/√T + σ² / (N·√T) + k/c )

where `k` is the Top-k cutoff and `c` is the sketch column count. The `k/c` term is the
**sketching error** — it vanishes as `c → ∞` (exact recovery). In practice `c = 10 · k` gives
<1% accuracy loss, `c = 100 · k` gives parity.

### Grid search in experiments

The authors sweep `k ∈ {10 k, 25 k, 50 k, 75 k, 100 k}` (Top-k entries per round) and
`c ∈ {325 k, 650 k, 1.3 M, 2 M, 3 M}` (sketch columns), finding that `c = 650 k, k = 50 k` is
competitive on GPT-2 PersonaChat, yielding 7.3× overall, 100× upload-only compression.

## Key Results

### GPT-2 / PersonaChat finetune

- Baseline uncompressed validation perplexity: 14.9.
- FetchSGD at 3.9× overall (10× upload): **14.8** — indistinguishable from baseline.
- FetchSGD at 7.3× overall (100× upload): **competitive** — slight perplexity lift.
- FedAvg baseline at same comms budget: **16.3** (worse).
- Local Top-k at same budget: **17.1** (worse).

### ResNet-9 / CIFAR-10 and CIFAR-100

- Upload compression at 25×: within 1% of uncompressed FedSGD accuracy.
- Upload compression at 100×: within 3%.
- Clearly outperforms local-state baselines at matched comms, because of client-stateless design.

### FEMNIST (LEAF benchmark)

- 3400 clients, client participation is naturally sparse (few rounds per client).
- FetchSGD significantly outperforms FedAvg and local Top-k at matched upload budget — the
  scenario is exactly the "stale local state" failure mode the paper targets.

### Why it wins on cross-device

Local Top-k and DGC maintain per-client residual buffers. If a client participates once per 1000
rounds, the residual is 1000 rounds stale — effectively random noise. FetchSGD's sketch lives on
the server, is updated every round regardless of which clients participate, and is always fresh.

## Strengths

- **Zero client state across rounds**. This is the paper's thesis. For any FL deployment where
  clients participate rarely (cross-device, ocean sonobuoy fleet, edge IoT), this is decisive.
  DGC/signSGD with local residuals *cannot* match this under sparse participation.
- **Uplink size independent of |θ|**. A 10 B-param model and a 10 M-param model ship the same
  sketch size. This is the **only** FL algorithm in this survey with that property.
- **Linear aggregation over clients at the sketch level**. The server's `S_t ← Σ_k Sketch(g_k)` is
  a single vector sum — trivially amenable to secure aggregation, homomorphic addition, or sum-
  based Byzantine defences (e.g. coordinate-wise median over rows).
- **Server-side momentum + error feedback are both "free"**, sitting inside the linear sketch.
  No extra round-trips, no client-side state.
- **Theoretical guarantees** with explicit rate bound including the sketching error term.
- **Compatible with privacy-preserving aggregation** — summation-only aggregation is ideal for
  secure multi-party computation.

## Limitations

- **Sketch size is fixed large-ish**. Even the smallest useful `(r, c)` = (1, 10⁴) is 40 kB — far
  above the 340 B Iridium SBD payload. For sub-kbps links, FetchSGD alone is **not viable** unless
  aggressive post-sketch compression (signSGD on the sketch values themselves, or PowerSGD-style
  rank reduction) is layered on top.
- **Unsketching cost is O(|θ| log |θ|)** at the server per round. For |θ| = 100 M this is ~2 GB
  of compute per round — fine for a shore server, non-trivial for a buoy Raft leader running on
  a Cortex-M7.
- **Sketching error is a lower bound on accuracy**. Unlike Top-k with residual buffers (which is
  unbiased), Count Sketch Top-k extraction has a non-zero error floor `O(k/c)`. You cannot drive
  it to zero without `c → ∞`.
- **Row-wise median is not Byzantine-resilient at the sketch level**. A malicious client can
  insert a huge value at a single sketch cell; because the sketch is *linear*, that poisons the
  sum. A secure aggregation protocol or a per-client sketch norm check is required.
- **Downlink is dense model** (θ, not sketch). The paper doesn't compress the downlink. For
  Iridium SBD downlink (300 B MT messages), a full model broadcast is infeasible; buoys would have
  to poll multiple SBD cycles or the server would compress the model with quantization-only
  (e.g. 8-bit weights). FetchSGD is fundamentally an *uplink* compression algorithm.
- **Parameters `(r, c, k)` are new hyperparameters** with a grid to sweep. Not as drop-in as DGC.
- **Assumes synchronous participation within a round** (standard federated assumption). Buoys
  with week-long wake cycles don't fit the paper's implicit timeline.

## Portable Details

### Sketch dimensioning for sonobuoys

Target: uplink ≤ 1 kB = 250 FP32 values. If we use Count Sketch with `r = 1`, `c = 250`, and
quantize each sketch value to INT8, that's 250 B — fits in one Iridium SBD packet. Expected
reconstruction fidelity: ε = 1/√c = 1/√250 ≈ 6% — rough but serviceable.

Alternative: `r = 3`, `c = 80`, INT8 = 240 B. Better median-based robustness. ε ≈ 11%.

### Sketch update / unsketch primitives

    # Client (stateless):
    fn sketch(g: &[f32], hashes: &[H; R], signs: &[S; R]) -> [[f32; C]; R] {
        let mut S = [[0.0; C]; R];
        for j in 0..d {
            for i in 0..R {
                S[i][hashes[i](j)] += signs[i](j) * g[j];
            }
        }
        S
    }

    # Server (unsketch for top-k):
    fn unsketch_topk(S: &[[f32; C]; R], k: usize, hashes, signs) -> Vec<(usize, f32)> {
        let mut heavy = BinaryHeap::with_capacity(k);
        for j in 0..d {
            let est = median((0..R).map(|i| signs[i](j) * S[i][hashes[i](j)]));
            if heavy.len() < k { heavy.push((est.abs(), j, est)); }
            else if est.abs() > heavy.peek().unwrap().0 { heavy.pop(); heavy.push(...) }
        }
        heavy.into_sorted_vec().into_iter().map(|(_, j, v)| (j, v)).collect()
    }

### Error-feedback in sketch space

    # server-side, after aggregating all client sketches S_t:
    S_total = ρ * S_mom + S_t + S_err            # momentum + residual
    topk = unsketch_topk(S_total, k)
    for (j, v) in topk:
        θ[j] -= η * v
        # subtract applied component from S_err via insert-with-negative-value:
        insert(S_err, j, -v)
    S_mom  = ρ * S_mom + S_t                     # update momentum
    S_err  = S_total                              # carry residual, then above subtracts applied

### Combined DGC + sketch + signSGD stack

Layer cascade for sub-kbps operation:

1. Client runs E local epochs (FedAvg-style), computes Δθ_k.
2. Client applies **DGC Top-k** locally with s = 0.001 → sparse index-value pairs.
3. Client **sketches the sparse update** (`r = 2, c = 400`) — now 3.2 kB of FP32.
4. Client **signSGD-quantizes the sketch values** → r·c bits = 800 bits = 100 B.
5. Send 100 B over Iridium SBD (fits in one MO packet).
6. Server aggregates 1-bit sketches, runs unsketch + scale to approximate Top-k of averaged
   updates.
7. Server applies error feedback in sketch space, broadcasts dense θ (or quantized θ) downlink.

This is the *core* sub-kbps FL codec for WeftOS. See gap-closing addendum for full protocol.

### When FetchSGD beats DGC

- Clients participate < 10% of rounds: FetchSGD wins (DGC residual goes stale).
- Clients participate > 50% of rounds: DGC wins (exact residual preserves rare-class gradients).

Sonobuoy fleets span both regimes. A 100-buoy tactical array with 10-minute rounds → 90%
participation → DGC. A 10 k-buoy PAM fleet with monthly rounds → <1% participation → FetchSGD.

## Sonobuoy Integration Plan

### Where FetchSGD slots into WeftOS

FetchSGD is a **server-side codec**: the sketch state lives on the Raft leader, not on the
buoys. This matches WeftOS's trust model (leader is trusted per-round, buoys are not).

- **Per-buoy component**: `codec::sketch_update` in `weftos-sonobuoy-fl-codec`. Stateless per
  round; just runs Count Sketch on gradient and uploads. Hash seeds are part of the round header.
- **Per-leader component**: `agg::fetch_sgd` in `weftos-sonobuoy-fl-aggregator`. Maintains
  `(S_mom, S_err)` in `mesh_chain.rs` Raft state or a dedicated shard. Linear-sum aggregation is
  trivially compatible with Raft log replication of `+=` operations.
- **Checkpointing**: Sketches are small enough to include in Raft snapshots — preserves global FL
  state across leader reelection.

### Deployment-profile fit

- **`sonobuoy-pam`**: ideal target. Sparse client participation, long rounds, tolerable to
  moderate sketching error. Use `r = 2, c = 200, k = 50`, INT8 values → ~800 B per round.
- **`sonobuoy-tactical`**: FetchSGD alone is overkill (plenty of UHF bandwidth, high
  participation). Still useful as a *backup* codec when radio weather degrades to Iridium-only.

### Interaction with Byzantine defence

The linear sum over clients is vulnerable. Three mitigations:

1. **Row-wise median of sketches** (instead of sum) — partial Byzantine robustness at row level,
   but breaks the linear-momentum property.
2. **Sketch-norm pre-filter**: reject any client whose sketch `‖S_k‖_F > τ` before summing.
   Preserves linearity on accepted clients; drops outlier magnitudes.
3. **Multi-Krum on sketch space**: rank clients by `Σ_j ‖S_k − S_j‖_F²` over (N − f − 2) nearest
   neighbours in sketch space, drop outliers. Works because Count Sketch approximately preserves
   ℓ₂ distances under appropriate (r, c).

### ADR implication

*ADR-082 candidate (see gap-closing addendum)*: FetchSGD is the primary *uplink* compression for
the sparse-participation deployment profile (`sonobuoy-pam`). Sketch dimensions are tuned to fit
the Iridium SBD 340 B payload. Server maintains persistent sketch state in `mesh_chain.rs`.

## How This Closes G5

G5 demands <1 kB/buoy/round at <10 rounds to converge.

- **Uplink size decoupled from |θ|**: FetchSGD lets us grow the on-buoy model to 100 kparam+
  without the uplink ballooning. DGC's index overhead grows linearly with k; FetchSGD's doesn't.
- **No residual state on client**: perfect for rare-participation sonobuoys where keeping a
  stale residual is worse than not keeping one.
- **Convergence in ~10 rounds**: the paper shows on CIFAR-10 FetchSGD converges in roughly the
  same number of rounds as uncompressed FedAvg, provided the sketching error term is small
  (`c ≥ 10k`). For our (c=200, k=20) budget, this means 10-round convergence is feasible if the
  model is small enough that the Top-20 coordinates carry >90% of the true-gradient signal.
- **Server-side momentum + error feedback**: the sketch carries momentum, so the per-round
  gradient on the wire can be "just this round's delta" without losing history. Effective
  compression >> naive ratio.
- **Combines additively with signSGD**: quantizing the FP32 sketch values to 1 bit reduces the
  800 B sketch to 100 B, crossing into single-SBD-packet territory. Plus DGC on top gives the full
  layered stack.

## Follow-up References

1. **Cormode & Muthukrishnan 2005** — "An Improved Data Stream Summary: The Count-Min Sketch and
   its Applications", Journal of Algorithms 55(1):58-75. The foundational sketch paper;
   Count-Min is a close cousin. Count Sketch (Charikar, Chen, Farach-Colton 2002) is the variant
   used by FetchSGD.
2. **Jiang et al. 2018** — "Sketchboost: Boosting for Count-Sketch Linear Learners". Applies
   sketches to boosting — earlier precedent for sketch-as-model-update.
3. **Vogels, Karimireddy, Jaggi 2019** — "PowerSGD: Practical Low-Rank Gradient Compression for
   Distributed Optimization", NeurIPS 2019, arXiv
   [1905.13727](https://arxiv.org/abs/1905.13727). An alternative low-rank approach that uses
   power iteration instead of sketches; also client-stateless if server tracks error feedback.
4. **Ivkin et al. 2019** — "Communication-efficient Distributed SGD with Sketching", NeurIPS 2019,
   arXiv [1903.04488](https://arxiv.org/abs/1903.04488). An earlier sketch-for-SGD paper by one
   of the FetchSGD authors.
5. **Konečný et al. 2016** — "Federated Learning: Strategies for Improving Communication
   Efficiency", arXiv [1610.05492](https://arxiv.org/abs/1610.05492). Introduced the
   structured/sketched update idea for FL, which FetchSGD operationalizes rigorously.
