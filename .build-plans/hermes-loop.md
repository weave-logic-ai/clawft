# Build Plan — Local Hermes Agent Loop

**Goal:** run the clawft agent loop on a local Hermes model with the long-context
memory tier, fully cloud-independent.

**Implements:** ADR-018 (umbrella) · **ADR-058** (context memory tier) ·
**ADR-059** (Qwen3 embedder) · **ADR-060** (Hermes serving + KV) ·
**ADR-061** (conversational voice front — Phase 6, wraps the loop). Each phase below
maps to those ADRs' acceptance criteria — read the ADR before starting a phase.

**Voice:** Phase 6 puts a real-time *conversation* in front of this loop (STT →
this grounded agent → dual-layer TTS, full-duplex). It was prototyped + measured
end-to-end in the `~/llm` voicelab harness; Phase 6 is the native weftos port, not
new R&D. Read **ADR-061** + `.planning/voice-ecc-synthesis.md` first.

## How agents use this plan
- Work phases in order; within a phase, respect the listed task deps. Pick the
  lowest-numbered task whose deps are done.
- **Task sizing (ADR-058 OQ1):** each task is scoped to fit one agent context. Treat
  loop instances as cattle, not pets — if a task balloons, split it; the chain/ADRs
  are the source of truth, re-derive rather than carry state.
- **Per task:** read the referenced files first; implement; satisfy the *Done-when*
  gate; then mark `[x]`.
- **Repo rules (`~/CLAUDE.md`):** branch (NEVER commit to master) · `cargo build`,
  `cargo clippy -- -D warnings`, `cargo test` green before commit · no
  `Co-Authored-By` trailer · files < 500 lines · read before edit · `cargo fmt`.
- **Status:** ADRs 058/059/060/061 are **Accepted (2026-06-28)** — all phases cleared to
  start. Serving default: **Hermes 4.3-36B `Q8_0` @ ctx 32k**. Voice default: **Chatterbox
  (fast) + Orpheus (slow)**. Tool-calling / web grounding is **out of voice scope** (agent-loop
  concern, ADR-060).

## Dependency overview
```
Phase 0 (serving)  ─┐
Phase 1 (embedder) ─┼─► Phase 2 (context builder) ─► Phase 3 (graft layer) ─► Phase 4 (prune+promote) ─► Phase 5 (integration)
                    └─► (Phase 2 needs P0 for summarization LLM; P3 needs P1 embedder)
Deferred backlog: independent, schedule anytime after its dep.
```

## Parallel execution — hive-mind swarm + git worktrees

The phases are *dependency*-ordered but **not** serial. Run **four concurrent
tracks**, each in its own git worktree + feature branch, isolated **by crate** to
minimize merge conflict. A hierarchical hive-mind coordinator (queen) owns the sync
gates; per-track leads spawn task workers and coordinate cross-track via SendMessage.

| Track | Worktree / branch | Owns (crates / files) | Tasks | Waits on |
|---|---|---|---|---|
| **A — Serving & Provider** | `feat/serving` | `~/llm` lab + `clawft-llm/src/local_provider.rs` | 0.1, 0.2 | — (start now) |
| **B — Embedder** | `feat/embedder` | `clawft-kernel`: `embedding_qwen3.rs` (new), `embedding.rs`, `lib.rs`, `Cargo.toml` | 1.1–1.4 | — (start now) |
| **C — Memory tier** | `feat/memory-tier` | `clawft-core/src/agent/context.rs` + loop; `clawft-kernel` ECC graft / `vector_*` | 2.x, 3.x, 4.x | S1 (for 2.3), S2 (for 3.3) |
| **D — Voice front** | `feat/voice` | `clawft-voice-aec`, `clawft-channels` | 6.1–6.7 | S1 (6.3), S3 (6.7) |

**Sync gates (coordinator-enforced, each = merge to the `integration` branch + green build):**
- **S1** — after **A.0.2** (LocalProvider ready) → unblocks **C.2.3** and **D.6.3**.
- **S2** — after **Track B** (embedder ready) → unblocks **C.3.3**.
- **S3** — merge **A + B + C** → **Phase 5** integration (single integrator agent).
- **S4** — **S3 + D.6.1–6.6** → **D.6.7** Talk-Mode (final end-to-end).

**Conflict hotspots:** B and C both live in `clawft-kernel`. B touches the
`embedding_*` files; C touches ECC/`vector_*` — disjoint *except* `lib.rs` +
`Cargo.toml`. Rule: **merge B at S2 before C starts its `clawft-kernel` ECC work**
(C rebases on B), and treat `lib.rs`/`Cargo.toml` as coordinator-serialized edits.
Tracks A (mostly `~/llm` + `clawft-llm`) and D (`clawft-voice-aec`/`clawft-channels`)
are crate-disjoint from each other and from B → fully parallel.

**Swarm rules:** every worker honors the repo rules above (branch, build/clippy/test
before merge, no `Co-Authored-By`, files < 500 lines). Worktrees keep working trees
isolated; integration happens only at S1–S4 on the shared `integration` branch — never
on master. Critical path is **A → C → Phase 5 → S4**; B and D run alongside and join
at S2/S4. Effective wall-clock ≈ critical path, not the sum of phases.

---

## Phase 0 — Serving bring-up (ADR-060) — lab + provider, no kernel risk
- [ ] **0.1 Serve Hermes 4.3-36B on llama.cpp.** Fetch `Hermes-4.3-36B` GGUF
  (**Q8_0 default**); serve via `~/llm/bin/serve-llamacpp --kv q8_0
  --ctx-size 32768 --spec-type ngram-simple --cache-reuse 256 --jinja --port 8090`.
  *Done-when:* OpenAI `/v1/chat/completions` responds; `~/llm/bin/eval --endpoint
  http://127.0.0.1:8090/v1 <repo> --pack tool_use` passes at the chosen quant.
- [ ] **0.2 Wire `clawft-llm` `LocalProvider`** to the endpoint. Implement Hermes
  `<tool_call>` round-trip to OpenAI `tool_calls`; strip/route `<think>` out of
  tool-call parsing; per-turn reasoning toggle (on=planning, off=tool exec); set
  `num_ctx`/`--ctx-size` explicitly (no silent 4k truncation). Files:
  `crates/clawft-llm/src/local_provider.rs`. *Done-when:* a tool-calling round-trip
  test passes through the provider against the live endpoint; `<think>` never appears
  in parsed `tool_calls`. *Dep:* 0.1.

## Phase 1 — Embedder provider (ADR-059) — `clawft-kernel`
- [ ] **1.1 Add `tokenizers` crate** (workspace `Cargo.toml` + `clawft-kernel`
  under the `onnx-embeddings` feature). *Done-when:* builds with the feature.
- [ ] **1.2 `Qwen3EmbeddingProvider`** in new `crates/clawft-kernel/src/embedding_qwen3.rs`
  (export from `lib.rs`). Decoder-ONNX inputs (input_ids/attention_mask/position_ids
  + empty fp16 past_key_values derived from the graph), last-token pooling, MRL-512
  slice + L2-norm, `embed_query()` instruction prefix, `model_fp16.onnx`. Keep the
  BERT `OnnxEmbeddingProvider` intact. *Done-when:* unit tests (no model) pass —
  last-token-pool math, MRL slice+norm (≈1.0, len 512), tokenizer-absent fallback.
  *Dep:* 1.1.
- [ ] **1.3 Selection wiring.** `select_embedding_provider()` priority Qwen3 →
  MiniLM → LLM → Mock; search paths `.weftos/models/Qwen3-Embedding-0.6B/{model_fp16.onnx,
  tokenizer.json}` + `$WEFTOS_MODEL_PATH`; graceful Mock fallback. File:
  `crates/clawft-kernel/src/embedding.rs`. *Done-when:* selection unit test passes;
  default-feature build still compiles. *Dep:* 1.2.
- [ ] **1.4 Fetch model + live parity.** Place `onnx-community/Qwen3-Embedding-0.6B-ONNX`
  `model_fp16.onnx` + `tokenizer.json` in `.weftos/models/...`; run the `#[ignore]`
  live test; Rust-side **cosine-parity ≥ 0.99** vs the lab (`bin/embedlab`) on the
  shared probe set. *Done-when:* parity ≥ 0.99 confirmed in-Rust. *Dep:* 1.2.
- *Phase gate:* `cargo build/clippy/test -p clawft-kernel --features onnx-embeddings`
  green; default build compiles.

## Phase 2 — Context builder core (ADR-058) — `clawft-core/src/agent/context.rs`
- [ ] **2.1 Real tokenizer for budgeting.** Replace whitespace `count_tokens` with a
  real tokenizer matching the served model. *Done-when:* token counts within ~5% of
  the model's on a fixture set.
- [ ] **2.2 Prefix-stable assembly + window cap.** Assemble `[stable head] → [graft
  block] → [rolling tail]`, head byte-identical turn-to-turn; cap the live window to
  the ADR-060 ctx-size. *Done-when:* head bytes stable across turns in a test; window
  never exceeds cap. *Dep:* P0 (ctx-size source).
- [ ] **2.3 Real compression.** Replace first-sentence `compress_context` with
  LLM-summarize-old-turns (via `LocalProvider`) + summary-in-window. *Done-when:*
  aged turns are summarized, not first-sentence-truncated; budget respected.
  *Dep:* 0.2 (LLM), 2.1.

## Phase 3 — L2 graft layer (ADR-058) — `clawft-kernel` ECC + `clawft-core`
- [ ] **3.1 Session-scoped view.** Filter ECC index queries by this conversation's
  chain sequences; ephemeral per-session HNSW (**single semantic index, v1** — no
  fusion). *Done-when:* a query returns only this session's chunks; view rebuildable
  from the chain. *Dep:* ADR-056 ECC indexes (existing).
- [ ] **3.2 Graft API.** query (scoped) → candidate branches → COW-reference graft
  into the working set; **keyed by chain sequence**; large payloads as RVF
  content-addressed blobs referenced by chain hash. *Done-when:* graft returns
  chain-sequence-keyed items with provenance backrefs; dedup by content hash works.
  *Dep:* 3.1.
- [ ] **3.3 Index population.** Embed `agent.chat.turn` / tool-output events with the
  ADR-059 embedder; **store f32@512** (int8 storage deferred — see backlog). *Done-when:*
  events are indexed and retrievable. *Dep:* Phase 1, 3.1.

## Phase 4 — Prune + promotion (ADR-058) — `clawft-core` loop + chain
- [ ] **4.1 Prune-to-graft eviction.** Evict from the live window when aged; origin
  stays on the chain; re-graftable via retrieval. No `--context-shift` reliance.
  *Done-when:* a pruned item is re-grafted correctly later. *Dep:* Phase 3.
- [ ] **4.2 Postmortem promotion stage.** Post-task review nominates promotions;
  multi-signal (task scoring + explicit "important" marks + reference count).
  *Done-when:* postmortem emits promotion candidates. *Dep:* Phase 3.
- [ ] **4.3 Promotion as chain event.** L2→L3 promotion emits a `memory.promote`
  `ChainLoggable` event recording source chain sequences; session view dropped only
  after promotion. *Done-when:* `memory.promote` lands on ExoChain with lineage.
  *Dep:* 4.2.

## Phase 5 — Integration + validation
- [ ] **5.1 End-to-end loop** on local Hermes + L2 tier (Phases 0–4 wired). *Dep:* 0–4.
- [ ] **5.2 Recall integration test:** a fact stated early, pruned from the window, is
  re-grafted correctly later in the same session — **with its witness chain**.
- [ ] **5.3 Budget validation:** prefix/KV reuse skips re-prefill of the stable head;
  query latency within the hot-path budget; cosine-parity gate green.

## Phase 6 — Voice front (ADR-061) — `clawft-voice-aec` + `clawft-channels` Talk-Mode
Wraps the agent loop (Phases 0–5) in a real-time conversation: **STT → grounded agent →
dual-layer TTS**, full-duplex, speaker-aware. Each piece is **already validated in the `~/llm`
voicelab harness** (the reference impl + bench); these tasks are the native weftos port. Read
ADR-061 + `.planning/voice-ecc-synthesis.md`. The brain is this loop (don't fork it); voice is
a front end on the same `LocalProvider`/context tier.
- [~] **6.1 `clawft-voice-aec`** — native AEC channel (cpal I/O + WebRTC AEC3, feature-gated;
  downmix-any-mic-to-mono, 16k, `flush()` for barge-in; `aec-bridge` stdio bin). **Crate exists +
  builds + round-trip/AEC verified (−41dB).** Done-when: a `lib` API (not just the bin) that
  `clawft-channels::voice` consumes; `cargo build/clippy/test` green default + `--features webrtc-aec`.
- [ ] **6.2 STT leg (ADR-053).** `SttBackend` (parakeet English-fast / whisper-turbo multilingual),
  **pre-warmed at startup** (the "slow first turn" was cold-load). Done-when: warm transcribe < 0.2s,
  no cold-load on first utterance. *Dep:* ADR-053 path.
- [ ] **6.3 Spoken-answer policy.** Voice-turn system policy: **short answers** (hard token cap,
  1–2 spoken sentences). Tool-calling / web grounding is **out of scope** for voice (agent-loop
  concern, ADR-060; harness-only). Done-when: voice turns are concise; no wall-of-text. *Dep:* 0.2.
- [ ] **6.4 Dual-layer TTS.** FAST = **Chatterbox cloned voice** (contextual ack + chit-chat) +
  SLOW = **Orpheus** (default; literal `<laugh>`, preset voice, **streamed chunk-by-chunk**,
  gap-free RTF<1), barge-in-interruptible; reject blocking render-all-then-play. (Default trades
  single-voice identity for literal paralinguistics; `IndexTTS-2` = same-voice prebuffered swap.)
  Done-when: first-audio ~1–2s on the slow path, gap-free, flushes on interrupt. *Dep:* 6.1.
- [ ] **6.5 Turn-taking.** smart-turn semantic endpointing (keep listening while the turn reads open;
  finalize on complete / max-silence). Done-when: a mid-thought pause doesn't clip the user. *Dep:* 6.2.
- [ ] **6.6 Speaker identity → ECC node.** ECAPA d-vector → persistent named registry → an ECC
  per-speaker CrossRef/node (ADR-056); fed to the LLM as private context, never spoken. Done-when:
  enrolled voices identify ≥ threshold, unknowns rejected, utterances attribute to the speaker node.
  *Dep:* ADR-056 indexes. (Multiplayer foundation — N speakers.)
- [ ] **6.7 Talk-Mode controller (`clawft-channels`).** Assemble 6.1–6.6 over the AEC channel:
  capture → endpoint → STT → speaker-attribute → grounded agent → fast ack → streamed expressive
  answer; barge-in flushes. The speculative(ack)→committed(answer) ECC handoff. Done-when: end-to-end
  spoken conversation on local Hermes, interruptible, speaker-named, grounded. *Dep:* 6.1–6.6, Phase 5.

## Deferred backlog (independent — schedule after the named dep)
- [ ] **ruvector storage-quant test** (before enabling int8 *storage*): extend
  `~/llm/bin/embedlab` with Variants A (ruvector scalar int8, global min/max +
  avg-scale L2), B (per-dim), C (log-companded) + outlier-severity stat; gate
  recall@10 ≥ 0.95. Until then store f32@512. *Dep:* Phase 3.
- [ ] **Reranker** (`Qwen3-Reranker-0.6B`) two-stage retrieve→rerank. *Dep:* Phase 3.
- [ ] **Multi-index fusion v2** (causal + temporal + BVH per ADR-056), RRF over
  chain-sequence keys. *Dep:* Phase 3 + ADR-056 BVH build.
- [ ] **Embedder fine-tune loop** on chain-derived recall outcomes; MTEB(eng,v2)
  Retrieval anti-forgetting guardrail + CoIR/RTEB + `code_recall.jsonl`. *Dep:* Phase 4.
- [ ] **Larger `code_recall.jsonl`** probe set → robust recall numbers → registry
  `scores:`. *Dep:* none.
