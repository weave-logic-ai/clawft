# TabSTAR — modeling evaluation for WeftOS indexing / data collection

> Research note. DESIGN/EVAL only — no code lands from this file.
> Requested by team-lead 2026-07-05. Subject: `alana89/TabSTAR` on HuggingFace,
> paper arXiv 2505.18125 (Arazi, Shapira, Reichart; May 2025).
> Grounded against: `weave.toml` `[kernel.agent]` (the open real-embedder
> decision, lines 80–82), `crates/clawft-kernel/src/embedding.rs`,
> `docs/brain/05-rvf-brain-and-research.md`, `.planning/voice/phase1-waves-plan.md` §W1.2.

## TL;DR verdict

**Pass on the TabSTAR weights. Steal one idea. Adopt its backbone.**

TabSTAR is a *supervised tabular classifier* that must be **LoRA-fine-tuned per
dataset** — it is not a general embedder and does not emit reusable
representations. It does not fit any WeftOS consumer as-is. But the evaluation
surfaced the single most useful concrete result for us: TabSTAR's frozen-then-
unfrozen backbone is **`intfloat/e5-small-v2`**, a 384-dimensional MIT-licensed
sentence embedder with existing ONNX exports. WeftOS's open "real embedder"
decision targets exactly **384-d** (`weave.toml:29`). The right takeaway is
therefore: (a) the **verbalization idea** — turn a structured record into a
semantic string before embedding — is directly applicable to our tabular-ish
node records; and (b) **e5-small-v2 is a ready, correctly-sized, license-clean,
ONNX-exportable answer to the placeholder-embedder problem**, independent of
TabSTAR itself.

## What TabSTAR actually is

| Attribute | Value |
|---|---|
| Task | Tabular **classification** with text columns (transfer learning) |
| Backbone | `intfloat/e5-small-v2` (pretrained text encoder, 384-d) |
| Size | 47.3M params, F32 |
| Adaptation | **Requires LoRA fine-tuning per downstream task** — not zero-shot, not frozen |
| License | **CC-BY-4.0** (weights); backbone e5-small-v2 is **MIT** |
| Maturity | ~10.3k downloads/month; SOTA on medium/large text-tabular classification; shows dataset-count scaling laws |
| ONNX | **None published** for TabSTAR; e5-small-v2 has community ONNX exports |
| Known limits | Regression **underperforms GBDTs**; memory blows up at "hundreds of features"; weak on purely-numerical / few-shot |

**Architecture, briefly.** Each row is *verbalized*: numerical columns are
z-scored (±3σ clip), quantile-binned into 10 bins, and rendered as semantic
strings like `"Age: 40–50 (Quantile 50–60%)"` while *also* keeping a parallel
numeric MLP embedding; categorical/text columns are rendered as
`"<column name>: <value>"`. A single **fusion transformer** layer attends over the
semantic + numeric embeddings per feature. The novelty is the **target token**:
every possible class is injected as a constant input element (its truth hidden),
so one shared prediction head serves any label set with **zero dataset-specific
parameters** — that architectural genericity is what lets it transfer across
datasets. The cost of that genericity is that using it on a new table still means
a **LoRA fine-tune**, and it emits a *classification head output*, not a recall
embedding.

## Fit assessment per WeftOS use case

### 1. Structured-record embeddings for HNSW recall — the anchor-path decision

This is the open decision (`weave.toml:80–82`): the kernel HNSW anchor path
currently embeds with a **deterministic SimHash** (`MockEmbeddingProvider::hash_embed`
in `embedding.rs:120`; also `clawft-core/src/vector_store.rs` per the brain doc) —
"KPI moves but neighbours are not semantic." Our records (v2 classification blob:
act/refined-act/topic/emotion-VAD/goal/entities; the `VoiceAnalysis` record's ~7
typed sections + token arrays, §W1.2; chain/spawn events) are exactly the
structured, semi-tabular payloads TabSTAR's *verbalization* was built for.

- **TabSTAR weights: no fit.** It outputs a class prediction after a supervised
  fine-tune. We need an *unsupervised similarity embedding* ("find turns like this
  one") with no labels and no per-schema training. Wrong tool.
- **TabSTAR's backbone: direct fit.** `e5-small-v2` is a general sentence embedder
  producing 384-d vectors — our exact target dimension. Drop the `EmbeddingProvider`
  trait (`embedding.rs:65`) onto an ONNX e5-small-v2 runner and the placeholder is
  gone. e5's `query:`/`passage:` prefix convention is a natural home for
  record-vs-query asymmetry.
- **TabSTAR's verbalization idea: directly reusable.** Before embedding a node
  record, render it to a stable semantic string the same way — `"act: assertive |
  topic: scheduling | arousal: high (0.8) | goal: reschedule"` — rather than hashing
  raw fields. That single preprocessing choice is most of the semantic-recall win,
  and it is model-agnostic (works under e5 today, any embedder later).

### 2. Classification refinement (enrichment pipeline, `enrichment_classifier.rs`)

Our enrichment is currently keyword/LLM-tier (task #53/#54). TabSTAR is a
*trainable* classifier, so in principle it could refine act/emotion/goal labels
from the structured record. In practice: **pass.** It needs a labeled per-schema
training set and a LoRA fine-tune we don't have and don't want to maintain per
label taxonomy; the LLM tier already covers this with zero training. Revisit only
if we ever accumulate a large labeled corpus of node records *and* want to retire
the LLM from the hot path for cost — a genuinely later, 0.9.x+ consideration.

### 3. "Data collection" — schema-aware ingestion of external tabular data

If WeftOS ever ingests external tabular datasets (CSV/parquet knowledge sources)
into the brain, TabSTAR's **verbalization + quantile-binning recipe** is the useful
artifact: a principled, semantic, embedder-ready serialization of heterogeneous
typed columns (the exact problem `rag-retrieval-engineer`-style ingestion faces).
Adopt the *recipe*, not the weights — feed the verbalized string to e5-small-v2
and index. No fine-tune, no CC-BY-4.0 dependency.

## Licensing / inference feasibility

- **TabSTAR weights CC-BY-4.0** — permissive but attribution-required and not
  MIT/Apache; a mild friction if ever bundled, and moot since we're not adopting them.
- **e5-small-v2 is MIT** — clean for our purposes; fits the `~/.weftos/models`
  local-ONNX pattern; 47M-param encoder runs comfortably on modest CPU, ONNX-int8
  quantizable, no GPU required. This is the low-risk path.
- TabSTAR inference (LoRA fine-tune + fusion transformer + no ONNX) is heavier than
  we want on the hot anchor path and has no exported runtime — another reason the
  backbone-only route wins.

## Recommendation

**Adapt ideas + adopt the backbone; pass on the model.**

1. **Adopt `intfloat/e5-small-v2` (ONNX, MIT, 384-d) as the concrete candidate to
   close the open real-embedder decision** in `weave.toml [kernel.agent]`. It is
   correctly sized, license-clean, local-ONNX-friendly, and is literally the
   representation layer the TabSTAR authors judged strong enough to build on. This
   deserves a Plane work item against the HNSW-anchor real-embedder line
   (benchmark e5-small-v2 vs the SimHash placeholder on recall over real node
   records; wire behind the existing `EmbeddingProvider` trait, degraded-off).
2. **Steal the verbalization pattern** for structured node records: a stable
   `field: value | …` semantic serializer (with quantile-binned numerics for the
   scalar-heavy `VoiceAnalysis` fields) as the embedder's input, replacing raw-field
   hashing. Model-agnostic; it is the bulk of the semantic-recall quality lift.
3. **Monitor** TabSTAR (and the TabPFN-v2 / tabular-FM line) only for the
   niche future case of retiring the LLM enrichment tier over a large labeled
   node-record corpus. Not near-term.
4. **Pass** on TabSTAR weights for embeddings/recall — wrong output type, needs
   per-schema training, no ONNX, CC-BY-4.0.

## Sources

- HF model card: https://huggingface.co/alana89/TabSTAR
- Paper (arXiv HTML): https://arxiv.org/html/2505.18125v1 · abstract: https://arxiv.org/abs/2505.18125
- HF paper page: https://huggingface.co/papers/2505.18125
- Code: https://github.com/alanarazi7/TabSTAR
- Backbone: https://huggingface.co/intfloat/e5-small-v2 (MIT, 384-d)
