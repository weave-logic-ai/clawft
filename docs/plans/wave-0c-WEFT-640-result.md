# WEFT-640 result — Real embedder (e5-small-v2) + record verbalization

**Branch:** `wave0c/weft-640-real-embedder`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb446-80ba-7202-9311-40fe186fc4d0`  
**Date:** 2026-07-30

## Summary

Replaces the SimHash / mock-hash *semantic* path for L2 session-tier graft with a flag-gated real embedder pipeline:

1. **`E5EmbeddingProvider`** (`e5-small-v2`, 384-d, MIT) — thin asymmetric wrapper over the existing BERT ONNX path (`query:` / `passage:` prefixes).
2. **`EmbeddingProvider::embed_query`** — default = `embed`; e5 and Qwen3 override. `SessionView::graft_text` now uses the query lane.
3. **Atom verbalizer** — classification-v2 + `VoiceAnalysis` → stable semantic strings before embedding; raw turn text still stored for graft payloads.
4. **Selection order** (no model download in CI): **Qwen3 → e5 → MiniLM → LLM → Mock**. Real inference requires staged ONNX + `onnx-embeddings` feature.

Project-canonical long-context path remains **ADR-059 Qwen3** when weights are present; e5 is the light 384-d RVF-aligned option from the TabSTAR eval.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-kernel/src/embedding.rs` | `embed_query` trait method; e5 search paths; select arm #2 for e5 |
| `crates/clawft-kernel/src/embedding_e5.rs` | **New** `E5EmbeddingProvider` + unit tests (hash fallback, no model) |
| `crates/clawft-kernel/src/embedding_qwen3.rs` | Trait `embed_query` so trait-object graft gets instruction prefix |
| `crates/clawft-kernel/src/context_graft.rs` | `graft_text` → `embed_query`; `index_chunk_with_embed` (embed ≠ store) |
| `crates/clawft-kernel/src/lib.rs` | Export `embedding_e5` + constants |
| `crates/clawft-service-agent/src/verbalize.rs` | **New** atom verbalizer + golden tests |
| `crates/clawft-service-agent/src/session_tier.rs` | Classify → verbalize → embed; store raw text |
| `crates/clawft-service-agent/src/lib.rs` | Export verbalize module |
| `docs/plans/wave-0c-WEFT-640-result.md` | This report |

## Staging (real inference — not CI)

```
.weftos/models/e5-small-v2/model.onnx   # or model_int8.onnx
.weftos/models/e5-small-v2/vocab.txt    # bert-base-uncased WordPiece
```

Also: `$HOME/.weftos/models/e5-small-v2/` or `$WEFTOS_MODEL_PATH`.

```bash
# Live e5 path (feature + staged artifacts)
cargo test -p clawft-kernel --features onnx-embeddings --lib embedding_e5

# Prefer Qwen3 if its bundle is staged (project-canonical)
# .weftos/models/Qwen3-Embedding-0.6B/{model_fp16.onnx,tokenizer.json}
```

Without artifacts or `onnx-embeddings`, selection degrades to Mock — builds/tests stay green.

## Verbalization shape

```
why did staging fall over again?
[act: question | topic: deploy | emotion: frustrated · high arousal (0.8) | goal: fix-oom]
[voice: angry · high arousal (0.9) · rising-pitch]
```

Scalars are word-binned (`low` / `mid` / `high`); raw float dumps avoided. Token arrays and capture health are omitted from the voice line.

## Recall benchmark (methodology)

Live model recall numbers require staged e5/Qwen3 weights (out of CI). Procedure for a local graft-quality check:

1. Build a `SessionTier` with `select_embedding_provider(None)` (real provider when staged).
2. Index N committed turns with classification blobs (act/topic/arousal clusters).
3. Query with a paraphrase of a target cluster (`graft_block` / `graft_text` → `embed_query`).
4. Report **recall@k** for “find turns like this one — similar act+arousal+topic” vs the same run on Mock/SimHash (baseline).

Expected qualitative result (from TabSTAR / e5-rvf study): mock/SimHash neighbours are near-random; e5/Qwen3 + verbalization clusters by meaning. The ignored hot-path budget test remains:

```bash
cargo test -p clawft-service-agent --features clawft-kernel/onnx-embeddings \
  -- --ignored budget_graft_latency
```

## Tests

```bash
scripts/build.sh check
# → ok

cargo test -p clawft-kernel --lib embedding_e5
# → 5 passed (hash-fallback asymmetric prefixes)

cargo test -p clawft-kernel --lib embedding::tests
# → 23 passed (incl. e5 search paths + embed_query default)

cargo test -p clawft-service-agent --lib verbalize
# → 8 passed

cargo test -p clawft-service-agent --lib session_tier
# → 22 passed, 1 ignored (live Qwen3 budget)
```

## Commit

- **Branch:** `wave0c/weft-640-real-embedder`
- **Message:** `feat(embed): WEFT-640 e5-small-v2 + record verbalization (replace SimHash)`
- **SHA:** run `git rev-parse wave0c/weft-640-real-embedder` (this file ships in the commit)

## Residual / follow-ups

- **Model download not in-tree** — operators stage ONNX under `.weftos/models/`; document in model-staging guide if missing.
- **`weave.toml [embedding]` still not fully wired** into daemon (study Part 5 item) — selection remains auto by artifact presence; explicit provider knob is a fast-follow.
- **RVF `embedder_id` META stamp** deferred (agenticow / COW plan); re-embed on producer swap still required.
- **Core `HashEmbedder` / graphify traits** unchanged — lower priority silos; kernel `EmbeddingProvider` is the L2 graft path.
- **Live recall@k numbers** not collected in this worktree (no staged e5 weights in CI/agent environment).
