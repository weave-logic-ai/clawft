# ADR-060: Local Hermes serving + KV management for the agent loop

**Date**: 2026-06-28
**Status**: Accepted (2026-06-28)
**Deciders**: Main-thread design + validation thread 2026-06-28 (long-context agent-loop / local-model serving thread)
**Depends-On**: ADR-018 (Hermes models as clawft-llm provider — the principle), ADR-058 (context memory tier — owns the window budget this serves), ADR-055 (backend-adapter contract)
**Relates-To**: ADR-059 (embedder); `~/llm` model lab (`bin/serve-llamacpp`, `bin/dsp-local`, `bin/eval`, model catalog)

## Context

ADR-018 established that clawft runs local models via `clawft-llm`'s
OpenAI-compatible `LocalProvider`, but only in principle ("any OpenAI-compatible
server works"). It does not record *which* model, *which* backend, or *how the KV
cache is managed* — and for a long-running agent loop those are the decisions that
determine whether it works. This ADR records them.

The binding constraint on a long-running loop is **KV-cache growth + prefill cost**:
the loop re-ingests a rolling context every turn. Verified serving facts (mid-2026,
runnable on the target Apple-silicon host):
- No importance-based KV eviction exists in any local backend (H2O/SnapKV/… are
  vLLM-CUDA/research). The most available is a sliding window.
- **llama.cpp context-shift is default-OFF** and regression-prone — not load-bearing.
- Precomputed-KV-block retrieval (CacheBlend etc.) is not viable locally.
- llama.cpp KV slot save/restore is **prefix-only** (contiguous from position 0).
- Speculative decoding accelerates **decode only, not prefill**; prefill dominates.

## Decision

### Model
- **Hermes 4.3-36B (Seed-OSS-36B base)** — **default `Q8_0` GGUF** (near-lossless;
  ~38 GB weights, fits 128 GB with huge headroom), 512K native ctx, strong agentic
  tool-caller. (`Q5_K_M` is the lighter/faster alternative.) **Fallbacks:**
  Hermes 4-70B (quality ceiling, Llama-3.1 base — can use a draft model) and
  **Nemotron-3-Nano-30B** (hybrid Mamba-2, near-flat KV — for 256k+ / max-throughput
  loops). Selection rationale lives in the `~/llm` catalog; this ADR pins the pick.

### Backend
- **Primary: llama.cpp via `~/llm/bin/serve-llamacpp`** (OpenAI endpoint :8090) — the
  only local backend with n-gram speculation, per-K/V cache-type quant, and slot
  save/restore. `clawft-llm` `LocalProvider` points at it.
- **Alternate: Ollama via `bin/dsp-local`** (native Anthropic API) for Claude-Code-
  style use. `bin/serve` (mlx_lm) is the lab/eval path, **not** the loop backend.

### KV management
- `-ctk q8_0 -ctv q8_0` (halves KV/token).
- **Cap `--ctx-size` to the live window — default 32k (~32–48k range), NOT the 512K
  native** — the
  context tier (ADR-058) holds the rest. Window bounding is application-level
  (ADR-058 prune-to-graft), **not** `--context-shift` (default-off, fragile).

### Speculation
- **n-gram / prompt-lookup** (`--spec-type ngram-simple`). Draft-model speculation is
  **impossible for the 36B** — Seed-OSS has no small same-vocab draft. n-gram is
  vocab-free and suits the loop's repetitive output (echoed tool output, code, diffs).
  The 70B fallback *can* use a `Llama-3.2-1B-Instruct` draft if chosen.

### Prefix caching (the biggest latency lever — prefill dominates)
- `--cache-reuse` + `--slot-save-path` keyed by a content-hash of the **stable head**.
- **Prefix-stable prompt layout** (shared with ADR-058): `[stable head: system +
  tool schema + pinned facts] → [graft block] → [rolling tail]`, head byte-identical
  turn-to-turn. Retrieval ordering MUST be cache-aware (never reshuffle the head).

### Provider integration (`clawft-llm` `LocalProvider`)
- **Hermes tool format:** Hermes emits `<tool_call>{…}</tool_call>` (XML-style), not
  native tokens. Serve with llama.cpp `--jinja` (embedded chat template) so `/v1/
  chat/completions` returns parsed `tool_calls`; verify round-trip. On Ollama, the
  GGUF template parses it but **`num_ctx` MUST be set explicitly** (default 4k
  silently truncates the loop — same failure class as the original dsp-local bug).
- **Hybrid reasoning (`<think>`):** Hermes 4.x emits `<think>…</think>`. Per-turn
  toggle — reasoning **on** for planning turns, **off** for tool-execution turns; the
  `LocalProvider` must strip/route `<think>` out of `tool_calls` parsing or arguments
  will be corrupted and latency blows up.

## Consequences

### Positive
- A concrete, reproducible serving recipe: model + backend + KV + speculation +
  caching, all on the host, no cloud dependency (fulfils ADR-018's air-gapped goal).
- Prefill-reuse + capped window keep a long loop fast where it would otherwise
  degrade; speculation adds lossless decode speedup on repetitive turns.
- Backend-pluggable (ADR-055): llama.cpp primary, Ollama alternate, MLX for the lab.

### Negative
- Decoder serving config is non-trivial (KV quant + ngram + cache-reuse + jinja tool
  template + `<think>` handling) — several knobs that must be set correctly together.
- n-gram only helps repetitive output; novel-reasoning turns see ~1× decode.
- `LocalProvider` needs real changes (`<tool_call>` round-trip, `<think>` routing,
  num_ctx awareness) — not just a URL.

### Neutral
- Quality ceiling (70B) and throughput/long-ctx (Nemotron-Nano) fallbacks are
  documented but not default; switching is a config change, not a redesign.

## Alternatives considered
- **Hermes 4-70B default** — stronger, but slower dense-70B and tighter KV; reserved
  as the quality fallback (and the one that *can* use draft-model speculation).
- **Nemotron-3-Nano-30B default** — flat KV / high throughput, but Hermes wins on
  tool-call format maturity; kept as the 256k+/throughput fallback.
- **Ollama-only** — simplest, but no n-gram and no KV-quant control; demoted to the
  alternate/Claude-Code path.
- **MLX `bin/serve` as the loop backend** — it's OpenAI-shaped (lab/eval) but lacks
  the speculation/KV-quant/slot-save the loop wants; lab-only.

## Acceptance criteria (serving + provider build checklist)
- [ ] Fetch `Hermes-4.3-36B` GGUF (**default Q8_0**); serve via `bin/serve-llamacpp`
      with `--kv q8_0 --ctx-size 32768 --spec-type ngram-simple --cache-reuse`
      and `--jinja`; OpenAI endpoint reachable.
- [ ] `bin/eval --endpoint … --pack tool_use` passes at the chosen quant (validate
      tool-call adherence before trusting the loop).
- [ ] `clawft-llm` `LocalProvider` points at the endpoint; `<tool_call>` round-trips
      to OpenAI `tool_calls`; `<think>` stripped/routed; per-turn reasoning toggle.
- [ ] `num_ctx`/`--ctx-size` set explicitly on every path (no silent 4k truncation).
- [ ] Stable-head slot-save + `--cache-reuse` verified to skip re-prefill of the head
      across turns; prefix-stable layout enforced by the context builder (ADR-058).
- [ ] Documented fallback switch to 70B (with Llama-3.2-1B draft) and Nemotron-Nano.

## Follow-ups
1. Benchmark prefill+decode latency at the capped window on the host; tune ctx-size.
2. Speculative-decode A/B (n-gram vs none) on real agent transcripts.
3. Revisit draft-model speculation if the 70B fallback becomes default.
