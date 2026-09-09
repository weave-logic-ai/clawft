# vec2vec — "Harnessing the Universal Geometry of Embeddings"

**Status:** Research note (analysis only — no code changes)
**Date:** 2026-08-26
**Paper:** Jha, Zhang, Shmatikov, Morris (Cornell) — arXiv:2505.12540v4 (26 Jan 2026),
NeurIPS 2025. https://arxiv.org/abs/2505.12540
**Audience:** vector-backend, ECC memory, substrate ACL, mesh/subscriber-node
**Relates to:** ADR-057 (substrate read ACL), ADR-059 (Qwen3 provider),
`.planning/research/e5-rvf-integration-study.md`, `crates/clawft-cow-memory/`

---

## 0. TL;DR

1. **Embeddings are not opaque.** vec2vec translates text embeddings from an
   *unknown* encoder into a *known* encoder's space with **no paired data, no
   access to the source encoder, and no candidate-match set** — reaching cosine
   0.92 and top-1 1.00 / rank 1.00 against ground truth on the best model pairs.

2. **This breaks a security assumption we hold implicitly.** Our `.rvf` stores
   persist **vectors only** — `texts` and `tags` are explicitly *not* written to
   the file (`crates/clawft-cow-memory/src/node.rs:68-71`). The natural reading
   is "a leaked `.rvf` is less sensitive than leaked text." vec2vec says
   otherwise: from vectors alone the authors recovered topic/attribute
   information and partial content for **up to 80% of Enron emails** and **67% of
   tweets**, judged by GPT-4o.

3. **Both of our real embedders are in the paper's evaluated set.** `e5` and
   `qwen3` are literally two of the seven models in Table 1. The `e5 ↔ gte` pair
   translates at cos 0.90 / top-1 1.00 / rank 1.01. We are not adjacent to this
   result; we are inside it.

4. **The correctness half of `embedder_id` gets *more* urgent; the security half
   gets *deleted*.** Stamping producer identity is still mandatory to stop
   silently mixing vector spaces — and it is **still unimplemented**
   (`crates/clawft-cow-memory/src/lib.rs:47` flags it as a Phase 2 open question,
   `RvfOptions` has no field for it). But we must never reason "different
   embedder ⇒ incomparable ⇒ safe." vec2vec is exactly the machine that makes
   different embedders comparable.

5. **ADR-057 moves from "governance hygiene" to "confidentiality control."**
   Its 9 acceptance criteria are **all unchecked** (0/9). Today any caller that
   can open the daemon's IPC channel can `substrate.read`/`subscribe` every path.
   If some of those paths carry embeddings, that is a plaintext read, not a
   vector read.

6. **The paper proposes no defenses.** Section 9 is Discussion and Future Work
   only. Any mitigation we adopt is ours to design; do not expect to lift one.

---

## 1. What the paper actually does

### 1.1 Threat model / problem

Given a dump of embedding vectors `{u_i} = M1(d_i)` from an **unknown** encoder
`M1` — no queries to `M1`, no knowledge of its architecture or training data, no
access to the documents `d_i` — extract information about `d_i`.

The attacker is assumed to have: one encoder `M2` they *can* query freely, and
coarse distributional knowledge of the hidden corpus (modality = text, language
= English). Nothing else.

This is strictly harder than the prior "matching / correspondence" literature,
which assumes both embedding sets cover the same or heavily overlapping inputs so
that every unknown vector has a candidate in the other set. vec2vec assumes no
such overlap.

### 1.2 The claim

The **Strong Platonic Representation Hypothesis**: networks trained with the same
objective and modality but different data and architecture converge to a
universal latent space, and a translation between their representations **can be
learned without any pairwise correspondence**. The paper's contribution is
demonstrating this constructively.

### 1.3 Architecture

Modular, deliberately unexotic:

- Input adapters `A1, A2 : R^d → R^Z` map each encoder space into a shared latent
  of dimension `Z`.
- A shared backbone `T : R^Z → R^Z` extracts the common latent.
- Output adapters `B1, B2 : R^Z → R^d` map back into encoder-specific spaces.

Translation and reconstruction are compositions:

```
F1 = B2 ∘ T ∘ A1      F2 = B1 ∘ T ∘ A2       (translate)
R1 = B1 ∘ T ∘ A1      R2 = B2 ∘ T ∘ A2       (reconstruct)
```

Embeddings have no spatial bias, so these are **MLPs** with residual connections,
LayerNorm, and SiLU — not CNNs. Discriminators mirror the structure but drop the
residuals.

### 1.4 Losses

Adversarial at **two levels** — on output embeddings (`D1, D2`) and on latents
(`D1ᐟ, D2ᐟ`) — plus three generator constraints:

| Loss | Enforces |
|------|----------|
| **Reconstruction** `L_rec` | `R(x) ≈ x` — round-trip within one space |
| **Cycle-consistency** `L_CC` | `F2(F1(x)) ≈ x` — round-trip *across* spaces; the unsupervised stand-in for paired data |
| **Vector-space preservation** `L_VSP` | pairwise dot products survive translation — the geometry, not just the points |

Combined: `L_gen = λ_rec·L_rec + λ_CC·L_CC + λ_VSP·L_VSP`, traded off against
`L_adv` by `λ_gen`.

**Every component is load-bearing.** The `gte → gtr` ablation (Table 6) is
brutal: full method gets top-1 0.91 / rank 2.64; remove VSP → top-1 **0.00**,
rank 4197 (random is 4096). Remove CC → 0.00. Remove the latent GAN → 0.00.
Remove hyperparameter tuning → 0.00. There is no graceful degradation; the
method either works or collapses to noise.

### 1.5 Setup

- **Train:** two sets of embeddings over **disjoint** 1M-sequence samples from
  Natural Questions (64-token sequences).
- **Eval:** NQ (65,536, batched at 8,192), TweetTopic (800), MIMIC-III
  pseudo-reidentified (8,192), Enron (50 emails).
- **Models (Table 1):** gtr (T5, 768d), clip (CLIP, 512d), **e5** (BERT, 768d),
  gte (BERT, 768d), stella (BERT, 768d), granite (RoBERTa, 768d, multilingual),
  **qwen** (Qwen3, 4B params, 2560d).
- **Metrics:** mean cosine similarity ↑, top-1 accuracy ↑, mean rank ↓.
  Baselines: naïve `F(x) = x`, and an *oracle-aided* optimal-transport
  pseudo-baseline that is given the candidate set (strictly easier than vec2vec's
  actual task).

### 1.6 Results

**In-distribution (Table 2):** cosine up to **0.92**, top-1 up to **1.00**, rank
down to **1.00**. In *cross-backbone* pairings (e.g. granite↔gtr, T5 vs RoBERTa)
both baselines sit at **rank ≈ 4100 of 8192 — indistinguishable from random** —
while vec2vec lands at rank ~1.1. In *same-backbone* pairings (gte/e5/stella, all
BERT) the naïve identity baseline already works, which is its own finding: those
spaces are near-compatible out of the box.

**Out-of-distribution (Table 3):** trained on Wikipedia-derived NQ, evaluated on
tweets (colloquial, emoji) and medical records (MedCAT disease jargon). Still cos
0.85–0.91 and rank 1.00 for the strong pairs. Degradation is real but uneven —
`gtr → e5` on MIMIC falls to top-1 0.01, rank 1414.

**Cross-modal (Table 4):** CLIP↔text translation is markedly weaker (top-1
0.00–0.72) but still beats OT, which is at random. Directional, not solved.

**Data efficiency (Table 7):** **50K embeddings buys ~1M-embedding quality**
(top-1 0.81 vs 0.92). 10K is already better than random. The attacker's data bar
is low.

### 1.7 Information extraction (the part that matters to us)

- **Zero-shot attribute inference** (Table 5): translated embeddings beat the
  naïve baseline consistently and *sometimes beat the same-space oracle* that has
  ground-truth embeddings. On MIMIC this recovers disease attributes whose
  descriptions never appear in the training corpus — evidence the latent is
  genuinely semantic rather than memorized.
- **Zero-shot inversion** (Figure 5, 6): off-the-shelf inverters, applied to
  translated embeddings, leak information for **up to 80% of Enron emails** and
  **67% of tweets** on the best model pairs. Recovered content includes
  individual and company names, dates, promotions, financial information, and
  outages. Reconstructions are noisy ("Some emails discussing NROn Employee/s
  Complaint To thePublic..." from "Subject: Enron Bashing on Frontline") — noisy
  enough to be unusable as text, precise enough to be a disclosure.

### 1.8 Stated limitations (honest accounting)

- **GAN instability.** They select the best of multiple initializations and
  explicitly defer robust training to future work. This is not a
  press-the-button-and-it-works method.
- CLIP / cross-modal is preliminary.
- Inverters are generic, not specialized for translated embeddings — the authors
  frame all results as a **lower bound**.
- **No mitigations are proposed anywhere in the paper.**

---

## 2. Parallels to our vector stores

### 2.1 We store vectors without text and treat that as a reduction in exposure

`crates/clawft-cow-memory/src/node.rs:68-71` is explicit:

```rust
/// Optional text payload carried alongside a vector id (agenticow's
/// `texts` map). Not persisted into the `.rvf` file itself.
pub(crate) texts: HashMap<u64, String>,
/// Free-form tags ... Not persisted into the `.rvf` file itself.
pub(crate) tags: HashMap<u64, VectorTags>,
```

So a `.rvf` on disk is vectors + HNSW graph + witness chain, and the source text
lives only in process memory. That is a real and useful property — but it is a
property about *storage*, not about *confidentiality*. vec2vec's whole
contribution is that the vectors alone carry the semantics. A leaked or
exfiltrated `.rvf` should be triaged at roughly the sensitivity of the corpus
that produced it, not at "just numbers."

This matters most for the ECC / conversation-memory tier, where the corpus is
classified utterance records — the "universal data atom" framing in
`.planning/research/e5-rvf-integration-study.md`. If every atom type (voice
turns, agent replies, spawn goals, sensor annotations) lands in one e5 space, then
one `.rvf` is a semantic index of everything the system has heard.

### 2.2 Our production embedders are the paper's test subjects

| Our provider | File | Dims | In paper? |
|---|---|---|---|
| `E5EmbeddingProvider` (e5-small-v2) | `embedding_e5.rs:30` | 384 | **yes** (`e5`, Table 1) |
| `Qwen3EmbeddingProvider` | `embedding_qwen3.rs:31` | 512 | **yes** (`qwen`, Table 1) |
| `OnnxEmbeddingProvider` (MiniLM) | `embedding_onnx.rs:359` | 384 | BERT-family sibling |
| `AstEmbeddingProvider` | `embedding_onnx.rs:969` | 256 (+64 structural) | no — code/AST, out of scope |

Note the paper evaluates the 768-d e5 and the 2560-d Qwen3; ours are the smaller
384-d / 512-d configurations. The *architecture families* are the same, which is
what vec2vec's cross-backbone results turn on, but the specific checkpoints are
not the ones benchmarked. Treat the numbers as strongly indicative, not as a
measured result for our exact stack.

**The same-backbone finding is the sharper one for us.** e5 and MiniLM are both
BERT-family. In the paper, same-backbone pairs (gte/e5/stella) are close enough
that the **naïve identity baseline already achieves top-1 1.00** — no translation
model needed at all. If we ever have both a MiniLM store and an e5 store, they
are not just translatable, they are partially *directly* comparable.

### 2.3 We have a 384-d collision and no producer stamp

Two distinct providers both emit 384-d vectors (`E5_DIMS = 384`,
`OnnxEmbeddingProvider::DEFAULT_DIMS = 384`, matching `weave.toml:29`). Nothing
in the type system or the store format distinguishes them. And the mechanism that
was supposed to — `embedder_id` — is documented as absent:

```
crates/clawft-cow-memory/src/lib.rs:47
//! - The `embedder_id` stamp-and-enforce discipline the plan calls for
//!   (§3, "DESIGN CONSTRAINT") -- `RvfOptions` has no field for it and this
//!   crate does not yet add a sidecar for one; see the crate's test/report
//!   notes for why this is flagged as a Phase 2 open question rather than
//!   silently skipped.
```

`E5_MODEL_NAME` exists as a constant labelled "embedder_id / consistency
contract" (`embedding_e5.rs:26-27`), but nothing consumes it at the store
boundary.

vec2vec sharpens the reading of this gap in **both directions**:

- **Correctness — more urgent.** Mixing two 384-d spaces in one store yields
  cosine values that are arithmetically valid and semantically meaningless. This
  is the failure the stamp exists to prevent, and it is the *likelier* of the two
  failures to actually bite us.
- **Security — the opposite of what intuition suggests.** Space
  incompatibility is **not** an access control. Do not let "the attacker doesn't
  know which embedder produced this" enter any threat model as a mitigating
  factor. It costs an adversary ~50K sampled embeddings to erase.

### 2.4 ADR-057 is the load-bearing control and it is 0/9 implemented

ADR-057 (Accepted, MUST-HAVE for 0.8.x) records that `substrate.read`,
`substrate.list`, and `substrate.subscribe` are all `Capability::Read`, and the
anonymous baseline grants `Read` by default — so **any caller that can open the
daemon's IPC channel can read or subscribe to every substrate path**. All nine
acceptance criteria in the ADR are unchecked.

Before this paper, the strongest argument for ADR-057 was raw-sensor exposure
(mic PCM, IMU). vec2vec adds a second class: **any substrate path carrying
embeddings is an unauthenticated semantic read of its source corpus.** That
argument survives the "but it's only vectors" objection that could previously
have been used to deprioritize embedding-bearing paths relative to the mic path.

This is a strengthened justification for existing, already-accepted work — not
new scope.

### 2.5 The mesh/subscriber direction is where this compounds

The 0.8.x subscriber-node story admits remote subscribers to substrate paths.
`ruvector-replication` / `ruvector-cluster` / `ruvector-raft` are in the workspace
(`Cargo.toml:321-324`), i.e. vector state is designed to move between nodes. Every
such hop is a place where a vector set can be observed by a party that never had
the text. The paper's contribution is precisely that observing the vector set is
enough.

---

## 3. What this does *not* say

Guarding against over-reading, since the security framing invites it:

- **This is not a break of RVF, ruvector, or HNSW.** Nothing about the storage
  format, the index, or the witness chain is implicated. The result is about
  embeddings as a representation, wherever they are kept.
- **It is not an attack on a running system.** It requires possession of the
  vectors. It is a *post-compromise amplification* result: it raises the
  consequence of a leak, not the probability of one.
- **It does not make our stores interchangeable for free.** Translation quality
  is pair-dependent and GAN training is unstable; the authors pick best-of-N
  initializations. Do not read this as "we could migrate embedding spaces without
  re-embedding" — the honest version is section 4.3 below.
- **I have not verified whether `.rvf` supports encryption at rest.** A targeted
  `search_ruvnet` for RVF encryption returned no matching source, and the brain's
  implementation gate reported *unproven* — meaning the query found nothing, not
  that the feature is absent. This needs checking against `rvf-runtime` source
  before any recommendation depends on it.

---

## 4. Implications worth carrying forward

Framed as findings, not as adopted work. Nothing here is scheduled.

### 4.1 Reclassify embedding-bearing artifacts

Treat a `.rvf` (and any embedding-carrying substrate path or replication stream)
as carrying the sensitivity of its source corpus. Concretely, that means
`.gitignore` coverage, backup handling, and any future export/share verb should
regard an ECC memory store the way they'd regard a transcript file. Worth a short
paragraph in whichever doc currently describes `.rvf` durability.

### 4.2 Finish `embedder_id`, and write the reason down correctly

The Phase 2 item stands. When it lands, the doc comment should say it exists for
**space-mixing correctness** — and should explicitly *not* claim any
confidentiality benefit, so a future reader doesn't build a threat model on it.
The 384-d collision between e5 and MiniLM is the concrete motivating case.

### 4.3 The one genuinely constructive use — and its cost

vec2vec is symmetric: the machinery that lets an adversary read our vectors would
also let *us* translate a legacy MiniLM store into e5 space without re-embedding
the source text. Tempting, given that a producer change is otherwise "new base +
re-embed."

Do not pursue this as a migration path in current form. The reasons are concrete:
training needs ~50K sampled embeddings per space plus unstable GAN training with
best-of-N initialization; translated vectors are *approximations* with pair-
dependent quality; and the resulting store would have a lineage that is neither
honestly MiniLM nor honestly e5 — which is exactly the ambiguity `embedder_id`
exists to eliminate. Per the e5 study, our persisted vectors today are
pseudo-random hash-embeds or ephemeral anyway, so there is no valuable corpus to
rescue. Re-embedding remains correct and cheap.

Revisit only if we ever accumulate a large, expensive, genuinely irreplaceable
corpus in a space we need to abandon.

### 4.4 Feed ADR-057's justification section

ADR-057 does not currently cite embedding disclosure as a motivating risk class.
When it is next touched, this belongs in its Context — with the citation, so the
reasoning is auditable rather than folkloric.

---

## 5. Citation

```bibtex
@inproceedings{jha2025vec2vec,
  title     = {Harnessing the Universal Geometry of Embeddings},
  author    = {Jha, Rishi and Zhang, Collin and Shmatikov, Vitaly and Morris, John X.},
  booktitle = {Advances in Neural Information Processing Systems (NeurIPS)},
  year      = {2025},
  eprint    = {2505.12540},
  archivePrefix = {arXiv},
  primaryClass  = {cs.LG}
}
```

Related in-tree: `.planning/research/e5-rvf-integration-study.md` (embedder_id
design constraint, one-space discipline), `docs/research/diskann-and-large-scale-indexes.md`
(ANN tiering), `docs/adr/adr-057-substrate-read-acl.md`,
`docs/adr/adr-059-qwen3-embedding-provider.md`.
