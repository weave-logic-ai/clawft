# e5-small-v2 ∩ RVF — Whole-System Integration Study

**Status**: Research (planning only — no code). Leave uncommitted.
**Author**: ruv-researcher (RVF/ruvector specialist)
**Date**: 2026-07-05
**Branch**: `feat/hermes-loop-base`
**Trigger**: TabSTAR eval surfaced `intfloat/e5-small-v2` (384-d, MIT, ONNX
exports) as the ready answer to WeftOS's open "real embedder" decision
(`weave.toml:29` targets 384-d). See `.planning/research/tabstar-eval.md`.
**RVF facts cross-checked against**: `.planning/ruv/integration/agenticow-integration-plan.md`
(rvf-runtime 0.2 API, verified from extracted crate source), `docs/brain/05-rvf-brain-and-research.md`,
`.planning/ruv/brain/distilled-notes.md` (agentdb, `[brain, unverified]`).

---

## 0. TL;DR

1. **e5-small-v2 is a near drop-in for the existing all-MiniLM ONNX path**, not a
   new subsystem. Both are 384-d BERT-family encoders with **masked-mean-pool +
   L2-normalize** and a **bert-base-uncased WordPiece** tokenizer — all of which
   the kernel's `OnnxEmbeddingProvider` (`embedding_onnx.rs`) already implements
   on `ort` 2.0-rc.12 (the workspace pin). The *only* new behaviour e5 needs is
   the **`query: ` / `passage: ` prefix convention**.

2. **There is almost nothing to migrate.** Every *persisted* vector in the system
   today is either pseudo-random (SHA/SimHash hash-embed) or ephemeral. The
   production semantic-graft path resolves to **`MockEmbeddingProvider(64)`**
   (`daemon.rs:1221` calls `select_embedding_provider(None)` and no ONNX artifact
   is staged). So adopting e5 is **greenfield**, not a re-embed of a valuable
   corpus. This is the single biggest simplifier in the whole study.

3. **RVF is the container; e5 is a producer. Producer identity is not
   self-describing in stored vectors.** Swapping producers silently mixes vector
   spaces (MiniLM-vec vs e5-vec cosine is meaningless). The discipline to bake in
   **now** (before agenticow COW lands): pin an `embedder_id` in the store's META
   and **refuse to branch/derive/query across a different `embedder_id`**. A
   producer change is a *new base + re-embed*, never an in-place swap — and a COW
   lineage must be single-space end to end.

4. **The trait is missing the asymmetric query lane.** `EmbeddingProvider` has
   only `embed()` (document/passage mode). The query-side prefix lives *only* on
   `Qwen3EmbeddingProvider::embed_query` and is **not on the trait**, so
   `SessionView::graft_text` / `SessionTier::graft_block` embed queries in
   *document* mode today. For e5 that would tag a query with `passage: ` — wrong,
   and quality-load-bearing. Add `embed_query()` to the trait (default = `embed`)
   as part of this work; it also silently fixes Qwen3's current query lane.

**The unifying frame** (per the user's directive): the **classified utterance
record is WeftOS's universal data atom** — turn text + classification-v2
(act/topic/emotion/goal/entities) + `VoiceAnalysis` where spoken. Text turns,
voice turns, agent replies, spawn goals/results, and future channel/GUI/chain
atoms are all instances of it, already flowing through one commit path
(`SessionTier::index_turn`). So e5 + verbalization is **the retrieval layer for
the whole substrate**, not a voice/memory feature. The design imperative that
follows: **one atom-level verbalizer + one e5 space serve every atom type
uniformly** (a spawn goal and a spoken complaint share one searchable space),
which is what enables cross-modality recall, agent self-reflection, and a single
coherent feature space for the Weaver's future learned weights.

**The overlay frame** (second directive, Part 6): the semantic/e5 space is **one
projection among several complementary indexes over the same atom stream — not a
replacement for any of them.** Temporal (witness chain / HLC), causal
(`CausalGraph` + cross-refs), semantic (HNSW/e5), spatial (BVH, ADR-056), and
lifecycle (`SessionView` frontier states) are all joined by **`chain_seq`**. They
compose, they do not compete (ADR-056 says this outright for the spatial lens). e5
sharpens **one** lens; the composite — "semantically similar AND within 2 causal
hops AND in the last hour" — is where the "much clearer view" lives. Today that
composite is dead because the semantic lens is hash-noise; **e5 upgrades the one
lens that is currently fake to load-bearing**, which is what makes the overlay pay
off. (Bonus: semantic *layout* surfaces — the ADR-067 graph view, a future
UMAP-style projection, and ADR-056's deferred §10 BVH+HNSW fingerprinting — also
cluster far better on e5 than on hash vectors.)

**Recommendation**: land e5 as a third `select_embedding_provider` candidate
(above MiniLM, below Qwen3), gated on staged artifacts + a `weave.toml` knob;
add `embed_query` to the trait; thread prefixes; define **one** schema-keyed
verbalizer at the atom level (not per-feature adapters); leave stored-vector
migration out of scope (nothing worth migrating). Sequence it **after** agenticow
Phase 0 only if COW lands first; otherwise it is independent and can ship now.

---

## PART 1 — CENSUS: every embedding / vector site

Three **separate, unrelated** `Embedder`/`EmbeddingProvider` traits coexist. This
is the central structural fact: there is no single embedding seam to swap.

| # | Site (file:line) | Trait / type | Dim | Producer today | Persisted? | e5 relevance |
|---|---|---|---|---|---|---|
| **Kernel embedding stack** |
| 1 | `clawft-kernel/src/embedding.rs:65` | **kernel `EmbeddingProvider`** trait | — | — | — | **primary seam** |
| 2 | `embedding.rs:335` `select_embedding_provider` | factory (ADR-059) | var | Qwen3→MiniLM-ONNX→LLM→**Mock(64)** | — | **add e5 candidate here** |
| 3 | `embedding.rs:108` `MockEmbeddingProvider` | Mock | 64 | SHA-256 hash | — | the actual prod fallback |
| 4 | `embedding.rs:188` `LlmEmbeddingProvider` | API (stubbed, always falls back) | 384 | Mock | — | alt path |
| 5 | `embedding_onnx.rs:332` `OnnxEmbeddingProvider` | ONNX all-MiniLM-L6-v2 | **384** | ONNX (masked-mean+L2, WordPiece) or hash | — | **e5 = this + prefixes** |
| 6 | `embedding_onnx.rs:678` `SentenceTransformerProvider` | ONNX (512 max, sentence-split) | 384 | same base | — | e5 long-doc variant |
| 7 | `embedding_onnx.rs:956` `AstEmbeddingProvider` | hybrid struct+text | 256 | hash | — | code-search, separate |
| 8 | `embedding_qwen3.rs:115` `Qwen3EmbeddingProvider` | ONNX decoder, MRL-512, **last-token pool**, `embed_query` | **512** | ONNX or Mock | — | **has the asymmetric lane e5 needs** |
| 9 | `context_graft.rs:138` `SessionView` | per-session HNSW; `index_chunk`/`graft`/`graft_text`; `for_embedder` | =embedder | — | **ephemeral** (rebuilt/session) | ADR-058 L2 quality path |
| 10 | `boot.rs:1433` `ecc_vector_backend` | `HnswBackend::with_defaults()` / DiskAnn / Hybrid | **384** (`HnswServiceConfig.default_dimensions`) | boot calibration writes; `ecc.search` reads; `custody.attest` signs len | in-mem (persistable) | the ECC brain index |
| 11 | `hnsw_service.rs` `HnswService` | HNSW | 384 default | — | — | index, producer-agnostic |
| 12 | `weaver.rs:1148` / `democritus.rs:94` | hold `Arc<dyn EmbeddingProvider>` | 64 | `MockEmbeddingProvider::new(64)` default | — | Weaver/Democritus embed path |
| **clawft-service-agent (L2 tier)** |
| 13 | `session_tier.rs:65` `SessionTier.embedder` | kernel `EmbeddingProvider` | =embedder | from daemon (`select…(None)`) → Mock(64) | ephemeral | **THE production graft embedder** |
| 13a | `session_tier.rs:276` `index_turn` → `SessionView::index_chunk` | — | — | embeds every committed turn (`embed`) | — | passage side |
| 13b | `session_tier.rs:528` `impl ContextGraftProvider::graft_block` | — | — | embeds query (`embed`, **not** embed_query) | — | **query side — needs prefix fix** |
| **clawft-weave (production wiring)** |
| 14 | `daemon.rs:1221` | `select_embedding_provider(None)` → `SessionTier::new` | — | Qwen3 if staged, else Mock(64); **weave `[embedding]` NOT consulted** | — | **the one line that picks prod embedder** |
| **clawft-core (its own trait + stub RVF)** |
| 15 | `embeddings/mod.rs:56` **core `Embedder`** trait | (separate) | — | — | — | 2nd trait |
| 16 | `embeddings/hash_embedder.rs:36` `HashEmbedder` | SimHash FNV-1a, **default 384** | 384 | deterministic hash (golden-tested) | — | non-semantic baseline |
| 17 | `embeddings/api_embedder.rs:61` `ApiEmbedder` | OpenAI `/embeddings`, SHA fallback | 384 | API or hash | — | `rvf` feature |
| 18 | `embeddings/rvf_stub.rs:118` `RvfStore` | in-mem brute-force cosine, JSON persist, **no COW** | any | — | **yes (JSON file)** | agenticow Phase-0 replaces this |
| 19 | `memory_bootstrap.rs:84` | core `Embedder` + `rvf_stub` | — | `ApiEmbedder` (hash in practice) | yes | doc-section index |
| 20 | `vector_store.rs` (per brain doc) | in-mem O(n·d) cosine + `HashEmbedder` | 384 | SimHash (non-semantic) | in-mem | legacy VectorStore |
| 21 | `agent/context_router/embedding/` | context-router embed | — | — | — | routing (see Part 4e) |
| **clawft-graphify (3rd trait)** |
| 22 | `graphify/src/bridge.rs:42` **graphify `EmbeddingProvider`** trait; `NoOpEmbedder` (zero vec) | (separate) | any | zero vector by default | into HNSW | 3rd trait; entity ingest |
| **clawft-services (MCP)** |
| 23 | `services/src/rvf_tools.rs:34` `rvf__` MCP store | in-mem brute-force, caller supplies vectors | param | caller | in-mem | producer-agnostic |
| 24 | `services/src/clawhub/search.rs:34` skill search | **keyword only** ("semantic via embeddings" is an unfulfilled doc-promise) | — | — | — | Part 4g candidate |

### Config / knobs surfaced

- `weave.toml [embedding] provider = "mock-sha256", dimensions = 384, batch_size = 16`
  — **not wired to the kernel selection path** (daemon passes `None`). Dead knob today.
- `weave.toml [kernel.agent] anchor_hnsw = true` (+ `.clawft/config.json` override):
  chat turns anchored with a **deterministic-hash 384-d** vector ("neighbours are
  not semantic … a future change will route through a real embedder"). M3-P4
  (`48e0dbe7`) already removed the redundant chat-turn hash-embed insert.
- `[kernel.vector] backend = hnsw|diskann|hybrid`; DiskAnn `dimensions` default
  (`config/kernel.rs:985`). HNSW `default_dimensions` is **hardcoded 384** in
  `boot.rs:1489`, not a config field.
- **Model staging dirs** (already implemented, `embedding.rs:376` / `embedding_onnx.rs:230`):
  `.weftos/models/<bundle>/`, `$HOME/.weftos/models/<bundle>/`, `$WEFTOS_MODEL_PATH`,
  `$WEFTOS_VOCAB_PATH`. e5 slots straight in as `.weftos/models/e5-small-v2/`.
- **`onnx-embeddings` feature** gates real ort inference; without it every ONNX
  provider degrades to hash. `ort = "2.0.0-rc.12"` (`Cargo.toml:226`).

**Census verdict**: the load-bearing production embedder is chosen at exactly one
line (`daemon.rs:1221`) through one factory (`select_embedding_provider`). e5
needs to be a new arm of that factory + the ephemeral L2 tier + (optionally) the
ECC `ecc_vector_backend` producer. The core `Embedder` and graphify traits are
independent and lower priority.

---

## PART 2 — RVF ∩ e5: container vs producer

**RVF (RuVector Format)** is the vector *container*: single-file, segmented
(VEC 0x01 embeddings · INDEX 0x02 HNSW · META 0x07 config/metadata · QUANT 0x06
codebooks · WITNESS 0x0A audit chain · POLICY_KERNEL/COST_CURVE), with COW
branching and `query_audited` witnesses in `rvf-runtime` 0.2. **e5 is a
producer** that fills VEC. The two are orthogonal — RVF neither knows nor cares
which model produced a vector.

### 2.1 Does RVF carry embedder metadata / versioning?

Not intrinsically in the published `rvf-runtime` 0.2 surface. The verified public
API (from the agenticow crate-source dive) exposes `dimension()`, `status()`,
`file_id()`, `metric`, COW lineage (`parent_id`/`lineage_depth`) — **but no
producer/model field**. The **META (0x07)** segment is "config/metadata" and is
the natural place to record it, but *WeftOS must write it deliberately*; nothing
enforces it. **Consequence: stored vectors are not self-describing about their
space.** Two `.rvf` files, both 384-d cosine, one MiniLM one e5, are
indistinguishable at the format level and will silently cross-contaminate on
query. → **Action: define an `embedder_id` (model name + revision + prefix
convention + MRL dims) and stamp it into META at store `create`; verify it on
`open`/`branch`/`query`.**

### 2.2 What swapping producers means for existing stored vectors

An embedder swap is a **change of vector space**. Cosine/L2 distances between
old-space and new-space vectors are noise. So in principle every persisted vector
must be **re-embedded** (recompute from source text) or **dual-indexed** (keep
both spaces, query the matching one).

**But in WeftOS today there is essentially nothing of value to migrate:**
- SessionTier L2 (the semantic-graft path) is **ephemeral** — rebuilt from the
  chain per session (`context_graft.rs` "Disposable: drop it at session end").
  Nothing persists across an embedder change; a new session just embeds with e5.
- `ecc_vector_backend` is in-memory and boot-calibrated; whatever it holds is
  re-derivable.
- The only genuinely persisted stores — `memory_bootstrap`'s `rvf_stub` JSON and
  the legacy `VectorStore` — are filled by **hash/SimHash pseudo-embeddings**,
  which have no semantic value to preserve. Re-embedding them with e5 is pure
  upside, and "dual-index" is pointless (the old space is worthless).

→ **Migration strategy: re-embed, don't dual-index. Treat existing indexes as
disposable.** The "incompatible spaces" problem is real in general but *nulled by
the fact that WeftOS never shipped a real embedder*. This is the study's key
lever: e5 adoption is greenfield.

### 2.3 What it means for agenticow COW branches

The agenticow plan (Phase 0 not yet done) gives each turn a branchable `.rvf`
view. COW correctness **requires a single vector space across a whole lineage**:
a child store's chain-walk query compares its own vectors against the parent's
(child-wins + tombstone mask + exact re-rank). If base was embedded with MiniLM
and a child with e5, the merge compares across spaces → garbage recall. Therefore:

- **Stamp `embedder_id` in the base `.rvf` META at fork time; `branch`/`derive`
  must inherit and enforce it.** Refuse to open a child whose parent space differs.
- **An embedder migration = fork a NEW base and re-embed the promoted set**, never
  an in-place producer swap on a live lineage. Promotion (`promote()`) across two
  spaces is forbidden.
- Because agenticow is still a plan, this is a **design constraint to bake in
  before COW lands**, not a migration to run later. It costs one field in the base
  manifest and one guard in `branch`/`derive`. Add it to the agenticow plan's §3
  API mapping (the `save/load` manifest already carries per-node metadata — put
  `embedder_id` there).

### 2.4 RuVector's own embedder — do we compare?

The `embeddings_generate`/`embeddings_init` MCP suite belongs to **agentdb**
(inside `agentic-flow`), whose default is **384-dim MiniLM** `[brain:
agentdb-primer.md, unverified]` — the same family and size as e5, and the same
384-d WeftOS already targets. So RuVector's stock embedder is *the model e5
improves on*: e5-small-v2 is a strictly stronger 384-d sentence encoder in the
same slot, still a **different space** (different weights → re-embed, not
interop). WeftOS's native embedding path is the **kernel ONNX providers**, not
the JS MCP embedder; we do not depend on agentdb's `embeddings_*`. So there is no
runtime coupling to break — we simply choose a better producer for the same
384-d container. (If we ever ingest agentdb-produced `.rvf` files, the
`embedder_id` guard from §2.1 is exactly what keeps us from mixing MiniLM and e5
vectors.)

**Part 2 verdict**: RVF is producer-agnostic and safe to keep; the risk is
entirely *un-tagged vector spaces*. Fix it with an `embedder_id` in META enforced
on open/branch. Existing stored vectors are non-semantic → re-embed, don't
dual-index. Bake the single-space-per-lineage rule into the agenticow COW plan
now.

---

## PART 3 — e5 practical fit

| Property | e5-small-v2 | WeftOS today | Fit |
|---|---|---|---|
| Dim | 384 | 384 everywhere (`weave.toml:29`, HNSW default, HashEmbedder) | **exact** |
| Family | BERT encoder (initialized from MiniLM) | `OnnxEmbeddingProvider` is all-MiniLM-L6-v2 | **same arch path** |
| Tokenizer | bert-base-uncased WordPiece (30522) | `WordPieceTokenizer` + `vocab.txt` already implemented | **reuse as-is** |
| Pooling | average pooling of last hidden state | `onnx_embed` does masked-mean-pool | **exact** |
| Normalize | L2 (cosine space) | `l2_normalize` applied | **exact** |
| Max tokens | 512 | `SentenceTransformerProvider` = 512; base = 128 | set 512 |
| **Prefix** | **`query: ` / `passage: ` REQUIRED** | trait has only `embed()`; asymmetric lane only on Qwen3 | **the one gap** |
| Runtime | ONNX | `ort` 2.0-rc.12 | **compatible** |
| Size | ~33M params → ~133MB fp32 / ~33MB int8 ONNX | Qwen3 path is ~1.2GB | **much lighter** |
| Latency (M-series CPU) | ~5–15ms fp32 short text; less int8 | Qwen3 decoder is far heavier | **fast** |
| License | **MIT** (weights) | — | clean |

### The prefix problem (load-bearing)

e5 was trained with `query: ` on queries and `passage: ` on documents; using the
wrong prefix (or none) measurably degrades retrieval. WeftOS's `EmbeddingProvider`
trait exposes only `embed()` — the **document/passage** side. The **query** side
(`embed_query`) exists *only* on `Qwen3EmbeddingProvider` and is **not on the
trait**, so:

- `SessionView::index_chunk` (`context_graft.rs:319`) → `embed()` ✔ passage side.
- `SessionView::graft_text` (`context_graft.rs:362`) and
  `SessionTier::graft_block` → **also `embed()`** ✗ — queries embedded in passage
  mode. This is already a latent asymmetry bug for Qwen3; for e5 it silently mis-
  prefixes every query.

**Fix (small, high-leverage)**: add `async fn embed_query(&self, q: &str)` to the
`EmbeddingProvider` trait with a default that calls `embed()` (back-compatible for
Mock/MiniLM). `E5OnnxProvider::embed` prepends `passage: `; `embed_query`
prepends `query: `. Route the query side of `graft_text`/`graft_block` through
`embed_query`. This is a prerequisite for e5 quality and a free correctness win
for Qwen3.

### Staging & wiring

- Bundle at `~/.weftos/models/e5-small-v2/` = `model.onnx` (or `model_int8.onnx`)
  + `vocab.txt`/`tokenizer.json`, matching the existing `qwen3_model_search_dirs`
  / `onnx_model_search_paths` pattern.
- Reuse `OnnxEmbeddingProvider`'s session load, WordPiece encode, masked-mean-pool,
  L2. e5 needs a thin subclass/config: prefixing + `model_name = "e5-small-v2"`.
- `ort` rc.12 already builds this path under `onnx-embeddings`; no new dep.
- int8 export recommended for the dev Mac (33MB, negligible quality loss for
  retrieval, faster cold start; fits the "sub-500ms first retrieval" brain goal).

---

## PART 4 — Real use cases (today → with e5 → gain)

### The organizing thesis: the classified utterance record is WeftOS's universal atom

Everything that commits to the forest is **one record shape**: turn text +
classification-v2 (act / topic / emotion / goal / entities; ADR-067
`KeywordTurnClassifier`) + `VoiceAnalysis` where spoken (`EmotionAnalysis`
valence/arousal/dominance/label + prosody, `analysis.rs:282`). Text turns, voice
turns, agent replies, spawn goals/results, and — future — channel messages, GUI
interactions, and narrated chain events are all instances of that atom. They
already flow through **one** commit path (`SessionTier::index_turn` →
`SessionView::index_chunk`) into **one** ephemeral space.

So the e5 + verbalization story is **not** a voice feature or a memory feature —
it is **the retrieval layer for the entire cognitive substrate**. The design
consequence, which drives both this Part and Part 5: **one verbalizer + one
embedding space serve all atom types uniformly.** A spawn goal and a spoken
complaint are rendered by the *same* record→string function and land in the
*same* searchable 384-d space, so a query can cross producer boundaries. The use
cases below are therefore not separate integrations — they are the same atom, the
same verbalizer, the same e5 space, viewed from different producers.

**a. Semantic turn graft (L2 recall feeding prompts) — the headline, and the base case for every other atom.**
- *Today*: `daemon.rs:1221` → `select_embedding_provider(None)` → **Mock(64)**
  SHA-256 (or 384-d hash). `SessionTier::graft_block` returns the top-k by cosine
  over hash vectors → **effectively random neighbours**. The grafted context
  injected into the prompt is noise the model must ignore.
- *With e5*: turns embedded `passage: <turn text>`; the current query embedded
  `query: <text>`; graft returns turns that are *actually about the same thing*.
- *Worked example*: user said 20 turns ago "the deploy to the aepod box keeps
  OOM-ing on the model load"; now asks "why did staging fall over again?" Hash
  cosine has no path from "fall over"/"staging" to "OOM"/"deploy"/"aepod" — the
  relevant turn is not grafted. e5 (STS-tuned) puts those turns in the same
  neighbourhood; the OOM turn is grafted into context. **Gain: the L2 semantic
  graft (ADR-058) stops being decorative and starts doing its job.** This is the
  single highest-value change and the reason the "open real embedder" decision
  exists.

**b. "Find turns like this" over classification-v2 / VoiceAnalysis records.**
- *Today*: turn nodes carry a 4-axis classification blob (act/arousal/topic;
  ADR-067). No semantic index over them beyond hash.
- *With e5 + verbalization*: render each record to a short semantic string
  ("assistant, reassuring tone, topic: deployment failure, aroused") and embed it
  `passage: …`. The scrubber/graph-view "find similar moments" query embeds
  `query: …`. **Gain**: act+arousal+topic similarity search for the timeline UI
  and future agent self-reflection ("when have I been in this situation before?").

**c. memory_search / brain recall (ruv/brain + weftos namespaces).**
- *Today*: `memory_bootstrap` embeds doc sections with `ApiEmbedder` → **hash
  fallback** in practice (no API key). Recall is lexical-ish at best.
- *With e5*: real semantic recall over the brain corpus; `query: `-prefixed
  lookups. **Gain**: `memory_search` returns conceptually-related notes, not
  hash collisions. Prereq to the RVF-brain vision in `docs/brain/05`.

**d. Spawn / task similarity (recurring goals → dedup / reuse).**
- *Today*: no semantic dedup of subagent goals.
- *With e5*: embed each spawn's goal `passage: …`; before spawning, `query: …`
  the store for near-duplicates. **Gain**: recurring goals reuse a prior
  subagent's result instead of re-running; a foundation for a reflexion/skill
  cache.

**e. Semantic routing (agentdb_semantic-route / ruvllm hnsw_route pattern).**
- *Today*: `context_router` embeds for routing (site #21); tier routing is
  keyword/heuristic.
- *With e5*: route by semantic proximity to exemplar prompts per tier/agent.
  **Gain**: better tier selection (the ADR-026 3-tier routing), fewer
  mis-routes. Moderate — routing tolerates coarse signal, so lower ROI than (a).

**f. Voice: speaker-independent semantic search over spoken conversation.**
- *Today*: transcripts land as turns embedded with hash; "what did I say about the
  deploy last week" can't find it.
- *With e5*: transcript turns embedded `passage: …`; spoken query embedded
  `query: …`; retrieval is over *meaning*, independent of who spoke or exact
  words. **Gain**: real conversational memory search — a flagship Talk-Mode
  feature. High value, rides entirely on (a).

**g. Census-surfaced extras.**
- **Skill search** (`clawhub/search.rs`) is **keyword-only** despite the module
  doc promising "semantic via embeddings … falls back to keyword." e5 fulfills
  that promise: embed skill descriptions `passage: …`, match user intent
  `query: …`. **Gain**: `/skill` discovery by intent, not substring.
- **Graphify entity ingest** (`bridge.rs`) defaults to `NoOpEmbedder` (zero
  vectors). e5 gives entity nodes real neighbourhoods. (Its trait is separate —
  wire an adapter.)

### What the unification enables that per-silo embeddings never could

Cases (a)–(g) are not seven features; they are one space seen seven ways. Because
every atom is verbalized by one function into one e5 space, three capabilities
fall out that a per-feature (per-silo) embedding could never deliver:

1. **Cross-modality recall.** Find the **spoken** turn that motivated a **typed**
   command — because the spoken complaint and the typed command sit in the same
   space, keyed by meaning, not by producer or modality. "Which of my voice
   gripes led to this refactor task?" becomes a single kNN, not a join across
   voice-index and task-index that never shared a metric.
2. **Agent self-reflection over its own behavioral history.** Agent replies, spawn
   goals, and spawn results are atoms too. The agent can query its own past
   behavior semantically ("when was I last in a situation like this, and what did
   I do?"), which is the substrate for a reflexion/skill cache — impossible when
   each behavior type lives in its own hash silo.
3. **One coherent feature space for the Weaver's future learned weights.** The
   Weaver (`weaver.rs`) is meant to learn strategy patterns over embeddings +
   causal structure. If every atom already lives in one honest 384-d semantic
   space, the Weaver learns over *one* feature geometry instead of trying to
   reconcile per-silo hash spaces that have no shared meaning. Unification is a
   prerequisite for the learned-weights work being coherent at all.

---

## PART 5 — Migration shape (sketch)

1. **`E5OnnxProvider`** (new, `embedding_e5.rs`), thin over the existing
   `OnnxEmbeddingProvider` machinery: e5 model + WordPiece vocab, `embed` prepends
   `passage: `, `embed_query` prepends `query: `, `model_name = "e5-small-v2"`,
   512 max tokens, MRL not needed (native 384).
2. **Trait change**: add `embed_query()` (default → `embed`) to
   `EmbeddingProvider`; route the query side of `SessionView::graft_text` and
   `SessionTier::graft_block` through it. (Free correctness win for Qwen3 too.)
3. **`select_embedding_provider` arm**: insert e5 **between Qwen3 and MiniLM** (or
   make order a `weave.toml` knob). Wire the currently-dead
   `weave.toml [embedding] provider`/`dimensions` into the daemon so
   `daemon.rs:1221` stops hardcoding `None` — this is where the knob finally
   becomes real.
4. **One atom-level verbalizer** — a *single* `fn verbalize(atom: &UtteranceRecord)
   -> String` keyed off the classification-v2 and `VoiceAnalysis` schemas, **not**
   per-feature adapters. Because every committed thing is the same atom, this one
   function serves turns, voice, agent replies, spawn goals/results, and future
   channel/GUI/chain atoms uniformly. Sketch of the contract:
   - Always: the turn text.
   - When classification-v2 is present: append a compact rendering of act / topic /
     goal / entities (e.g. `[act: complaint · topic: deployment · goal: fix-oom ·
     entities: aepod, model-load]`).
   - When `VoiceAnalysis` is present: append emotion (label + coarse
     valence/arousal bucket from `EmotionAnalysis`) and salient prosody (e.g.
     `[voice: frustrated · aroused · rising-pitch]`) — quantize the f32 axes to
     words so the semantic encoder can use them; do **not** feed raw floats.
   - The output is one string, embedded `passage: <verbalized>` on commit and the
     live query embedded `query: <verbalized-or-raw>`.
   This function is the **real design surface** of the whole effort — it is what
   makes one space cover all atom types, and it is what the Plane item
   `e5-small-v2 + verbalization, 0.8.x` should **expand** to own (schema-keyed
   verbalizer + its golden tests), rather than a bag of per-feature embedders.
   Define it once, at the atom level, in the tier that already owns the atom
   (`clawft-service-agent`, next to `SessionTier`/`turn_classifier`).
5. **Re-embed, don't dual-index**: existing persisted indexes are non-semantic →
   discard/rebuild. No dual-space bookkeeping needed.
6. **RVF `embedder_id` in META** + open/branch guard (§2.1, §2.3). Add the
   single-space-per-lineage rule to the agenticow plan before COW is built.
7. **Staging**: `~/.weftos/models/e5-small-v2/` (int8 for the Mac); document in
   the model-staging guide alongside Qwen3/MiniLM.

### Scope boundaries for the Plane item (0.8.x)
- **Expand to include**: the `embed_query` trait method + query-side threading;
  wiring `weave.toml [embedding]` into `daemon.rs:1221`; the `embedder_id` META
  stamp + guard; **the single atom-level verbalizer keyed off the
  classification-v2 / `VoiceAnalysis` schemas** (this is the real work — one
  function, not per-feature renderers).
- **Keep out of scope**: dual-space migration (nothing to migrate); agenticow COW
  itself (separate plan — just contribute the single-space constraint);
  fine-tuning/TabSTAR (this is unsupervised recall, not a supervised head);
  graphify/context-router adapters (fast-follows, not blockers).

---

## PART 6 — The overlay: one atom stream, many supportive projections

The second framing directive: e5/semantic is **one projection among several
complementary indexes over the same atom stream, not a replacement for any of
them.** WeftOS already builds (or plans) a family of indexes, each answering a
different question about the *same* atoms, all joined by the atom's identity
(**`chain_seq`** — "the universal ExoChain key", `context_graft.rs:48` — plus
`content_hash` for dedup). This is not incidental; it is the substrate's design.

### The index family (all keyed by `chain_seq`)

| # | Lens | Answers | Where | e5 touches it? |
|---|---|---|---|---|
| 1 | **Temporal / ordinal** | "when, in what order" | witness chain (`chain_seq` = global clock) + HLC | no |
| 2 | **Causal / relational** | "what led to / evidences what" | `CausalGraph` (8 typed edges: Follows, TriggeredBy, EvidenceFor…) + `CrossRefStore` (UNID); ADR-062 forest | no |
| 3 | **Semantic** | "what *means* the same" | HNSW over embeddings — `SessionView`, `ecc_vector_backend` | **yes — the e5 upgrade** |
| 4 | **Spatial / hierarchical** | "what is where, what shape, what overlaps" | **BVH** (ADR-056): AABB broad-phase, tagged-union leaves, cross-keyed into the causal store **by chain sequence** | indirectly (see bonus) |
| 5 | **Lifecycle / frontier** | "is this speculative / frontier / settled" | L2 `SessionView` `NodeState` per atom (ADR-062 D2) | no |

**Design principle (state it plainly): one atom stream, many supportive
projections, `chain_seq` as the join key.** ADR-056 already says this for the
spatial lens — the BVH "answers geometric overlap … they compose, they do not
compete" with HNSW, and it "cross-keys BVH leaves into [the causal] store by
chain sequence." e5 **strengthens lens #3 without displacing #1/#2/#4/#5.** An
embedder upgrade sharpens one lens; the composite is where the "much clearer
view" lives.

### Composite queries — where the overlay pays off

Because every lens keys off `chain_seq`, they intersect cheaply. Examples:

- **"Semantically similar to this turn AND within 2 causal hops AND in the last
  hour."** Semantic candidate set (HNSW/e5, top-N) → causal filter
  (`CausalGraph` 2-hop neighborhood of the anchor) → temporal window
  (`chain_seq`/HLC range). Each index narrows; **none suffices alone**, and the
  cheap join is the shared key.
- **Graph view (ADR-067) colored by semantic hue.** The conversation graph lays
  atoms out by causal edges (lens #2) and colors them by classification today; add
  **semantic-cluster hue** from e5 (lens #3) and the causal layout gains a second,
  orthogonal signal — "these two causally-unconnected turns are about the same
  thing" becomes visible as shared color.
- **Scrubber + spatial view, same atoms.** The scrubber replays **temporally**
  (`chain_seq` order, lens #1) while a BVH view (lens #4) shows the same atoms
  **spatially**; selecting one atom in either view resolves the other by
  `chain_seq`. Add e5 and a third pane can show the **semantic neighbourhood** of
  the scrubbed atom — three lenses, one selection.
- **Branch-diff across lenses, one witness.** RVF COW (semantic store) and BVH
  `derive()` (spatial store) both branch and both witness on the *same* append-only
  chain — so a speculative turn's semantic *and* spatial deltas roll back together
  under one exochain event (this is exactly the agenticow DualStateBridge shape).

### Why e5 matters *to the overlay specifically*

A composite is only as good as its weakest lens. **Today the semantic lens is
hash-noise**, so any composite that includes "semantically similar" collapses to
random — the overlay's headline query is effectively dead on arrival. The join
key already exists (`chain_seq`), the temporal and causal lenses already work, the
spatial lens is planned with the same key — **e5 is what upgrades the one lens
that is currently fake** to load-bearing, which is precisely what makes the
multi-index composite deliver the "much clearer view."

### Bonus — spatial *layout* from embeddings (accurate scope)

ADR-056's BVH indexes **geometric extent** (physical AABBs of units, sensor reads,
terrain) — it does **not** ingest embeddings, so e5 does not change the BVH's own
broad-phase. But two *semantic-layout* surfaces do consume embedding vectors, and
there e5 vs hash is night-and-day:

1. **Deferred BVH+HNSW fingerprinting.** The concept paper §10 ("Temporal
   Similarity Search via HNSW Fingerprinting"), explicitly **deferred to a future
   ADR** by ADR-056 (§Neutral), is the sanctioned overlay of semantic-into-spatial.
   A stronger e5 fingerprint is what would make that future composition cluster
   meaningfully instead of by hash noise — e5 raises the ceiling on that deferred
   work.
2. **Semantic layout of the conversation graph / any future 2D-3D projection.**
   Whenever atom *positions* are derived from their embeddings (a force-directed
   graph-view layout, or a future UMAP/t-SNE projection of atom vectors for a
   spatial conversation browser), hash vectors scatter atoms randomly while **e5
   places same-topic atoms together** — real visible clusters. So "the 3D
   projection could also consume e5 vectors for better spatial clustering than hash
   vectors" is a **real bonus use case**, landing on the semantic-*layout* surfaces
   (graph view, future projection, deferred §10), not on ADR-056's geometric
   broad-phase.

---

## Appendix — the three traits (why there's no single swap point)

| Trait | Crate | Impls | Used by |
|---|---|---|---|
| kernel `EmbeddingProvider` | clawft-kernel `embedding.rs` | Mock, Llm, Onnx(MiniLM), SentenceTransformer, Ast, Qwen3 | SessionTier/L2 graft, context_graft, democritus, weaver, ecc |
| core `Embedder` | clawft-core `embeddings/mod.rs` | HashEmbedder(384), ApiEmbedder(384) | memory_bootstrap, rvf_stub, legacy VectorStore |
| graphify `EmbeddingProvider` | clawft-graphify `bridge.rs` | NoOpEmbedder | entity ingest → HNSW |

e5 lands cleanly in the **kernel** trait (covers the high-value graft/voice/brain
paths). The core and graphify traits get e5 via thin adapters as fast-follows if
their paths ever carry real load; today they carry hash/zero vectors, so they are
not on the critical path.
