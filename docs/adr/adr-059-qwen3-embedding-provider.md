# ADR-059: Qwen3-Embedding-0.6B as the clawft-kernel embedding provider (ort/ONNX prod lane)

**Date**: 2026-06-28
**Status**: Accepted (2026-06-28) — implements the embedder layer of ADR-058
**Deciders**: Main-thread design + validation thread 2026-06-28 (long-context agent-loop / local-embedder thread)
**Depends-On**: ADR-058 (per-conversation context memory tier — the consumer), ADR-056 (ECC index families / `VectorBackend`), ADR-011 (raw HNSW sufficient), ADR-031 (RVF)
**Relates-To**: ADR-018 (Hermes/local-model provider); validation harness `~/llm/bin/embedlab`; ruvector-core 2.1.0 quantization

## Context

ADR-058's L2 context tier needs a real semantic embedder for the ECC HNSW
index. Today `clawft-kernel` embeds via `OnnxEmbeddingProvider`
(`embedding_onnx.rs`) using **all-MiniLM-L6-v2 (384-d, 256-token ctx)** through
**`ort` (ONNX Runtime 2.0)** behind the `onnx-embeddings` feature; ruflo's JS
layer likewise defaults to `Xenova/all-MiniLM-L6-v2`. That model is a portability
default, not a quality one — 256-ctx/384-d cannot hold a single tool output or
function, far too small for a code-heavy, long-context L2.

The embedder is needed in **two lanes that must agree**:
- **Lab** — Python/MLX (`bin/embed`, mlx-embeddings): benchmarking, offline index
  builds, and fine-tuning (the closed-loop training target of ADR-058).
- **Prod** — Rust, in-process, on the agent hot path: `clawft-kernel` via `ort`.

A test harness (`~/llm/bin/embedlab`, config-driven, voicelab-style) was built and
run to choose empirically. Selected findings (full numbers in
`~/llm/docs/models/results/embedlab.{json,md}`):
- **Qwen3-Embedding-0.6B** has both an MLX repo and a published ONNX export
  (`onnx-community/Qwen3-Embedding-0.6B-ONNX`), 32K ctx, MRL, Apache-2.0 — the
  only candidate spanning both lanes plus fine-tuning.
- **Cosine-parity MLX-8bit ↔ ONNX-fp16/fp32 = 0.9996 mean / 0.9993 min, 100% of
  probes ≥ 0.99** → lab == prod holds.
- **MRL-512** is the sweet spot (recall@10 0.983, no cliff to 256).
- The published **int8 `model_quantized.onnx` is broken** (parity 0.86, recall
  cliff) → use `model_fp16.onnx`.
- Latency: ~8 ms query (MLX) / ~40 ms (ONNX CPU). Doc-encode is async; only the
  short query is hot-path-critical.
- Probe set is small (57 q) → model-vs-model recall deltas are within noise; the
  decision rests on parity + MRL-512 + both-lanes availability, not recall rank.

Separately (ruvector-core 2.1.0 source): the vector **storage** quant default is
`QuantizationConfig::Scalar` = int8 with **per-vector global min/max** (no per-dim
/ outlier handling); LogQuantized & RaBitQ are NOT shipping (clawft stub, PR #352
unmerged). This is a *different* int8 layer from the model export and is handled in
the storage decision below.

## Decision

Add **Qwen3-Embedding-0.6B** as the clawft-kernel prod embedder — one model across
lab (MLX), prod (`ort`/ONNX), and fine-tuning — replacing all-MiniLM as the
preferred provider.

### Provider

- New module **`crates/clawft-kernel/src/embedding_qwen3.rs`** with
  `Qwen3EmbeddingProvider` implementing the `EmbeddingProvider` trait. **Keep the
  existing BERT `OnnxEmbeddingProvider` intact** as a fallback. (New file, not a
  rewrite of `embedding_onnx.rs` — keeps both under the 500-line limit.)
- `select_embedding_provider()` priority becomes: **Qwen3 (if model + tokenizer
  present) → MiniLM ONNX → LLM → Mock.** Graceful Mock fallback when the model,
  tokenizer, or `onnx-embeddings` feature is absent (must not break builds/tests
  without the ~1.2 GB model).

### Model + inference

- ONNX repo `onnx-community/Qwen3-Embedding-0.6B-ONNX`, file **`model_fp16.onnx`**
  — **NOT** the int8 `model_quantized.onnx` (broken; parity 0.86).
- It is a **causal decoder-with-KV-cache** export — NOT a BERT encoder. Inspect the
  loaded session's input signature and feed: `input_ids`, `attention_mask`,
  `position_ids` (0..seq_len), and **empty `past_key_values.*` tensors for every
  layer**, dtype-matched to the model (fp16 → float16 empty cache, shape
  `[1, n_kv_heads, 0, head_dim]`). Derive layer count / kv-head / head-dim from the
  graph inputs — do not hardcode.
- **Last-token pooling** (hidden state at the last non-pad position), not mean/CLS.
- **MRL: slice to first 512 dims, then L2-renormalize** (slice-then-renormalize,
  never reversed). `dimensions()` = 512.

### Tokenizer

- Adopt the HF **`tokenizers`** crate (load `tokenizer.json`, BPE) for this provider
  — required for Qwen3 BPE + the special tokens last-token pooling depends on, and
  it removes a class of lab≠prod drift. The hand-rolled WordPiece path stays only
  for the legacy BERT provider.

### Lanes, asymmetry, runtime

- **Asymmetric query/doc:** trait `embed` = document mode (raw text); add inherent
  `embed_query()` that prepends the Qwen3 query **instruction prefix**.
- **Runtime:** `ort` is primary. **`candle`+Metal is the escape hatch** if the 0.6B
  *decoder* ONNX is too slow on `ort`-CPU — it runs the exact safetensors in-process
  with Metal, matching the lab numerically. `ort` 2.0 has **no Metal EP**; CoreML EP
  is finicky → default to the **CPU EP**, benchmark CoreML per-model.

### Vector storage (v1)

- **Store f32 @ 512** — lossless, ~2 KB/vector; trivial at our index scale. int8
  *storage* quant (ruvector `Scalar`) is a **scale/WASM optimization, deferred**;
  ruvector's outlier-robust path is **PQ via `ruvector-diskann`**, not naive Scalar.
  Enabling any int8 storage requires the `embedlab` storage-quant recall test first
  (Variants A/B/C + outlier-severity stat) — see ADR-058. Decoder-LM outlier dims
  + global-min/max int8 is a real risk; L2-normalization is the de-risker.

### Consistency contract (the silent-divergence guard)

Pin and version `{model + HF revision + dim(512) + pooling(last-token) +
query-instruction + L2-norm + storage(f32)}`. Enforce a **cosine-parity ≥ 0.99**
gate between MLX and `ort`/ONNX encodings of a probe set before any index goes live
(measured **0.9996**); re-run after every fine-tune→export cycle. **Changing the
model invalidates existing HNSW vectors → re-index;** never mix vectors from two
model versions in one index. Store the contract hash alongside each ExoChain-keyed
vector so a model bump is detectable and triggers re-embed.

## Consequences

### Positive
- One model, three formats (MLX lab / ONNX prod / PyTorch fine-tune), one vector
  space — fine-tune once, export, serve; parity empirically holds (0.9996).
- 32K ctx + MRL-512 + code-capable replaces a 256-ctx/384-d default — a large
  quality jump for code/long-context L2 recall.
- Reuses the committed `ort` prod lane and the ECC `VectorBackend` (ADR-056); no
  new runtime mandated (candle only if needed).

### Negative
- Decoder-with-KV-cache ONNX is fiddly in `ort` (position_ids + dtype-matched empty
  past_key_values; the Python harness hit exactly these). Higher implementation
  cost than swapping a BERT encoder.
- ONNX query latency ~40 ms CPU (vs 8 ms MLX) — acceptable (doc-encode async) but a
  watch item; candle/Metal or a clean re-quant is the lever if it bites.
- ~1.2 GB model artifact to fetch/cache; provider must degrade to Mock without it.
- Adds the `tokenizers` crate dependency to clawft-kernel.

### Neutral
- The broken int8 *model export* and ruvector's int8 *storage* quant are distinct
  layers; this ADR uses fp16 inference + f32 storage and defers storage int8.
- gte-modernbert remains the encoder fallback if parity/latency ever fail.

## Alternatives considered
- **gte-modernbert-base (encoder)** — clean ONNX + passes the mlx gate; fast, 8K,
  Apache. Loses MRL + 32K. **Fallback** if Qwen3 parity/latency regress.
- **nomic-embed-text-v1.5** — strong, MRL, keeps BERT-WordPiece (no tokenizer
  migration), but **single-lane** (no MLX) → breaks the one-model-both-lanes goal.
- **candle instead of ort** — would match the lab exactly with Metal, but ort is
  already integrated and portable (incl. WASM); kept as escape hatch, not default.
- **int8 model export / NV-Embed-v2 / Conan-v2 / jina-v4 / gemini-\*** — rejected
  (broken export; non-commercial+off-arch; cloud-only). See ADR-058 / embedlab.

## Acceptance criteria (the build checklist)
- [ ] `Qwen3EmbeddingProvider` in `embedding_qwen3.rs`, `EmbeddingProvider` impl,
      `dimensions()`==512, `model_name()`=="Qwen3-Embedding-0.6B".
- [ ] `tokenizers` crate wired (workspace + clawft-kernel under `onnx-embeddings`),
      loads `tokenizer.json`.
- [ ] Decoder-ONNX inference: position_ids + empty fp16 past_key_values derived from
      the graph; last-token pooling; MRL-512 slice + L2-norm; `model_fp16.onnx`.
- [ ] `embed_query()` prepends the query instruction; `embed()` = document mode.
- [ ] `select_embedding_provider()` priority Qwen3 → MiniLM → LLM → Mock; graceful
      Mock fallback with no model/tokenizer/feature.
- [ ] Builds: `cargo build/clippy -p clawft-kernel --features onnx-embeddings -D
      warnings`; default-feature build still compiles; `cargo test -p clawft-kernel`
      green. Unit tests (no model): last-token-pool math, MRL slice+norm (≈1.0, len
      512), fallback, selection. Live-ONNX test `#[ignore]` pending model fetch.
- [ ] Cosine-parity ≥ 0.99 (lab MLX vs prod ort/ONNX) re-checked on the real model
      before any index is built.

## Follow-ups (not blockers for this ADR)
1. Fetch `onnx-community/Qwen3-Embedding-0.6B-ONNX` `model_fp16.onnx` + `tokenizer.json`
   into `.weftos/models/Qwen3-Embedding-0.6B/`; run the `#[ignore]` live test.
2. Rust-side `ort` cosine-parity check on the embedlab probe set (closes the
   Python-onnxruntime-proxy caveat).
3. ruvector storage-quant recall test (embedlab Variants A/B/C + outlier-severity)
   before enabling int8 *storage*; until then store f32@512.
4. Reranker (`Qwen3-Reranker-0.6B`) integration — separate decision.
5. Embedder fine-tuning loop + MTEB(eng,v2) Retrieval guardrail — separate decision.

## Open questions deferred
1. **`ort` CoreML EP** worth enabling vs CPU EP on this host — measure per-model.
2. **candle migration trigger** — what query-latency threshold flips us from ort to
   candle+Metal.
3. **Batch doc-encode** scheduling (async, off hot path) — design with ADR-058's
   write path.
