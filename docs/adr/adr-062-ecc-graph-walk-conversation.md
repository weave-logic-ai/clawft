# ADR-062: ECC graph-walk conversation — responses as nodes built by walking the causal graph

**Date**: 2026-06-29
**Status**: Proposed
**Deciders**: Hive-mind coordinator + research swarm (SOTA literature, weftos doc inventory, kernel-reality analysis, clawstage symposium analysis), 2026-06-29
**Depends-On**: **ADR-042** (Three Operating Modes — this is the *Act* mode), **ADR-046** (Forest of Trees — the CMVG structures + CrossRef/Impulse linkage this walks), **ADR-047** (Self-Calibrating Cognitive Tick — the tick this hosts on), ADR-058 (per-conversation context memory tier — the L2 graft substrate), ADR-061 (conversational voice agent loop — the front end this is the brain of), ADR-053 (voice STT canonical path), ADR-018 + ADR-060 (Hermes/local LLM serving — the brain that fills committed nodes)
**Relates-To**: ADR-056 (BVH-on-RVF 4D spatial-temporal index — temporal/causal recall in the walk, v2 fusion), ADR-045 (Tiered Router — the speed/power model tiers behind the Speculative/Committed split), `.planning/voice-ecc-synthesis.md` (the clawstage→ECC bridge; §B.2 loop + §C.7 lifecycle are the seed of this ADR — **not superseded, this ADR crystallizes it as a decision**), the dormant `DemocritusLoop` (`crates/clawft-kernel/src/democritus.rs`), the live L2 tier (`context_graft.rs` / `session_tier.rs`)
**Supersedes-routing-in**: the heuristic TTS routing copied from the `~/llm` voicelab (`_route_expressive`: `searched ∨ tags ∨ words>25`) — replaced by node-state lifecycle.

## Context

ADRs exist for every *piece* of the local conversational agent — STT (053), embedder (059), serving (060), context tier (058), voice front end (061) — but **none records how a conversation is *modeled and routed*.** Without that, the orchestration drifts toward a ported if-ladder ("is it expressive? long? did it search?") bolted onto a controller. That is the wrong altitude. This ADR records the conversation model itself.

Four bodies of research informed it (2026-06-29 swarm):

1. **SOTA literature** — the model below is a native unification of three established lines: the **Incremental Unit** framework (Schlangen & Skantze: `add`/`revoke`/`commit` + "grounded-in" links; arxiv 2501.00953), **graph-based dialogue management** (GraphWOZ 2211.12852; documented to *win* in the multi-participant case), and **speculative / dual-tier rendering** (felt-latency: TTFA is the metric). The newest 2026 work — "unit-based semi-cascaded full-duplex" (2601.20230) — is an audio-native restatement of exactly this thesis. **The model is ratified by the field, not freelanced.**
2. **weftos docs** — the design truth is `.planning/voice-ecc-synthesis.md` (§B.2 50ms tick loop, §C.7 NodeState lifecycle) + ADR-058/061; no newer ADR exists. This ADR fills that gap.
3. **kernel reality** — the substrate is **two disconnected subsystems**: the *live* L2 context tier (`SessionView` + per-conv HNSW, `context_graft.rs:164`) runs the conversation today but is **cosine-recall-only**; the *rich* ECC cognitive substrate (`CausalGraph`, `CrossRefStore`, `ImpulseQueue`, and the **dormant** drain→embed→search→update→commit `DemocritusLoop`, `democritus.rs:88`) is fully built but **never wired into conversation**. ~60% of the walk skeleton exists.
4. **clawstage symposium** — the conversation engine already specified the exact primitives (formulas below), verbatim in `docs/engines/02-DCTE.md`, `04-RSTE.md`, `05-scoring-system.md`.

## Relationship to the cognitive architecture (this is not a new substrate)

This ADR does **not** introduce a new architecture — it is the **conversation realization** of decisions already accepted:

- **ADR-042 (Three Operating Modes):** a live conversation is **Mode 1 "Act"** — actors produce utterances, the engine creates nodes and advances the wavefront, speculative branches are pruned/merged/committed. (The all-agents hive/swarm case — e.g. the very swarm that produced this ADR — is **Mode 3 "Generate"**, sharing the same floor + tick + chain.) This ADR specifies *how Act mode is driven for a voice conversation.*
- **ADR-046 (Forest of Trees):** the substrate is the CMVG **forest** {ExoChain, Resource Tree, Causal Graph, HNSW} linked by **CrossRefs** (which per ADR-046 "enable grafting (linking) and shaking (pruning) across structures") and **Impulses** (HLC-ordered). The "walk" in D1 *is* a **forest CrossRef traversal** (Causal lineage ⇄ HNSW recall ⇄ ExoChain witness); the L2 tier's graft/prune **are** the forest's graft/shake. The gap (kernel analysis) is that the live conversation path uses HNSW *in isolation* instead of traversing the forest — closing that is structural work, not a new design.
- **ADR-047 (Self-Calibrating Cognitive Tick):** the Talk-Mode loop is a `SystemService` on the **same self-calibrating tick** (50ms glasses / 10ms server, `tick_budget_ratio` 0.3, adaptive). The floor cadence *is* the tick cadence; we do not invent a timer.

## Decision

**A conversation is the agent walking and mutating a causal graph; a response is a node built by that walk; "routing" is not a subsystem — it is which node is current, decided by floor/coherence/node-state reads on a tick.**

Concretely, weftos adopts six positions:

### D1 — Responses are nodes; the turn is a graph walk

Each turn the agent **enters at the frontier** (the user's just-committed utterance `CausalNode`), **walks** (HNSW semantic recall *fused with* causal-lineage traversal `traverse_forward`/`trace_causal_chain` and `CrossRefStore.by_type` — not cosine alone), **mutates** (adds nodes; links `Follows`/`Contradicts`/`Enables`; prunes), and **renders responses from the nodes the walk produces.** The LLM (Hermes) is the step that *fills a Committed node's content*; the embedder/HNSW is the *recall step*. There is no separate "router."

### D2 — The 5-state node lifecycle (adopt clawstage's; it is already the kernel enum)

`NodeState { Speculative, Frontier, Committed, Stale, Pruned }` (`engines/02-DCTE.md §2.2`, and **already defined** at `context_graft.rs:46`):
- **Speculative** — above the wavefront, mutable, not yet hash-chained. *(Defined in the kernel but never assigned today → build target.)*
- **Frontier** — at the wavefront, being evaluated/scored for commitment.
- **Committed** — below the wavefront, immutable, signed, hash-chained (ExoChain `chain_seq`).
- **Stale** — soft rebase: kept, re-graftable, not authoritative (provenance retained).
- **Pruned** — hard rebase: tombstone.

Per-turn supersession (`Speculative→Frontier→Committed`) is the spine; today commit only fires at conversation end (`promote_and_drop`) — extending it per-turn is a build target.

### D3 — Quick vs considered = one actor, two self-branches (weft's genuine addition)

clawstage's branches competed **across actors**. weft's two-tier response is **one actor running two self-branches**: a **Speculative node** (a *cheap graph-local read* off the frontier — not a second LLM call — rendered immediately by the fast TTS layer, covering latency) and a **Committed node** (the deep walk: graft → LLM-ground → cohere) that **supersedes** it. The lifecycle outcome is `SpeculativeOutcome { Committed | Rebased | Pruned | Retained }` (`engines/02-DCTE.md §3.3`).

Because clawstage left "rebase an in-flight response" unresolved (`symposium/02:139-141`), weft **must define partial-node-update + repair semantics explicitly**: when the Committed answer **contradicts what the spoken Speculative ack already said**, the system emits an **overt-repair node** ("sorry — I heard X, you meant Y"), never a silent abandon (per the IU survey's strongest warning, 2501.00953). Revocation is a first-class primitive, not a patch.

### D4 — Floor is a scored read, not an engine (adopt clawstage's formula verbatim)

`compute_urgency` (`engines/02-DCTE.md §8.2`), a scored read over the `ImpulseQueue` each tick:

```
urgency = 0.30·semantic_relevance   // cosine(candidate, frontier head)
        + 0.15·emotional_arousal    // VAD arousal of this actor
        + 0.20·wait_time            // fairness pressure (normalized)
        + 0.10·crowd_density        // # contending actors
        + 0.25·content_readiness    // generated > partial > not-started
        ;  hard interrupt ⇒ f32::MAX (preempt, cancel generation)
```

`FloorState { Open, Held{holder,since}, Contended{candidates}, Locked{reason} }` (§8.3). `wait_time` and `content_readiness` are load-bearing and were missing from the bare sketch — `content_readiness` is what ties floor-winning to speculative-generation progress. Learned weights (Weaver/SONA `ruvector-tiny-dancer`) are **future, not v0.1**.

### D5 — Turn signals are impulses; backchannel is a cross-ref, never a node

Promote `ImpulseType::Custom(u8)` → named variants (both `ImpulseType` and `CrossRefType` are `#[non_exhaustive]`): **`EndOfUtterance` 0x51**, **`TurnClaim` 0x50**, **`TurnShift` 0x52**, **`Backchannel` 0x60**.
- **EOU** → trigger the commit path (Frontier→Committed).
- **TurnShift** → floor Open; next utterance commits `Follows`.
- **TurnClaim** (barge-in) → floor Contended; on grant, cancel TTS, new node links **`Contradicts`**, the in-flight Speculative/Committed node is pruned.
- **Backchannel** ("mm-hmm") → a **`Continuer` cross-ref** to the current speaker node; **no turn node, TTS continues.** This is the single most important invariant (confirmed first-class by 2026 hierarchical-EOT work, 2603.13379) — treating a backchannel as a turn is the documented failure mode.

Three-tier turn-taking (Silero VAD → semantic EOU → VAP for the joint 2-speaker future) is the production decomposition; VAD-only is "broken." **Primary-speaker segmentation** filters to the engaged speaker before the floor decision in multiplayer.

### D6 — Coherence is a soft dampener, not a hard gate

The RSTE coherence read (`engines/04-RSTE.md §5.2`): `0.30·relation_coverage + 0.25·relation_confidence + 0.20·structural_connectivity + 0.15·qap_closure + 0.10·topic_continuity`. Bands: ≥0.85 strong; 0.65–0.85 mild drift (floor weights toward coherence-restorers); 0.40–0.65 → `CoherenceWarning` + refocus prompt; <0.40 → `CoherenceBreak`. Below threshold, **all positive scores ×0.8 until recovery** — a dampener, not a block. `ImpulseType::CoherenceAlert` already exists (`impulse.rs:28`); the running tick detects spectral drift but gates nothing yet. (Sheaf-Laplacian topological coherence is aspirational/not-WASM-ready → deferred.)

## Architecture

- **Traverse the ADR-046 forest, not HNSW alone.** `index_turn` must also `CausalGraph.add_node` + `link(prev→new, Follows, weight=coherence)` + register speaker/emotion/goal CrossRefs, and `graft` must fuse HNSW recall with causal-lineage traversal (`traverse_forward`/`trace_causal_chain`) + `CrossRefStore.by_type` — so the walk follows *lineage + cross-structure links*, not cosine alone. This is the central structural change: make the live conversation path a forest traversal (ADR-046), which it is not today.
- **Host on a new Talk-Mode tick service modeled on the dormant `DemocritusLoop`** (`democritus.rs:88` already does drain→embed→search→update→commit on a compute budget), registered as a `SystemService` alongside `CognitiveTick`, sharing the ADR-047 self-calibrating budget/adaptive-interval primitives (`tick_budget_ratio`). **Leave the coherence `CognitiveTick` loop alone** (it stays the slow integrity monitor). Loop body per turn: **SENSE** = `ImpulseQueue.drain_ready()`; **FLOOR** = `compute_urgency` over `SessionView.live_seqs()`; **MUTATE** = `set_state` (Speculative→Frontier→Committed/Stale) + causal node/lineage/cross-ref; **RENDER** = Speculative→TTS, supersede with the Committed graft→LLM answer.
- **Per-tick DAG + next-tick feedback + sequence-number happens-before guard** (`engines/01-overview.md §5`, `symposium/02:818`): within-tick stages form a DAG; feedback (intention→new branch) takes effect *next* tick; the merge stage rejects out-of-order sequence numbers. This keeps the walk deterministic.
- **All inference native Rust, no Python** (per user direction 2026-06-29): STT (parakeet), speaker (ECAPA), endpoint (smart-turn) via `ort`/sherpa-onnx ONNX; TTS via native Rust ONNX with Orpheus token-gen over **Ollama HTTP** (a native runtime, not a wrapper) + **native SNAC decode** (`ort`). The LLM brain is `LocalProvider`→Hermes. AEC is the existing `clawft-voice-aec`.
- **The LLM is *not* incremental.** Incrementality lives in the graph/nodes and the stable-ASR prefix; speculative LLM start is gated on a completeness/entropy signal and committed on EOU; `serve-llamacpp --draft` makes wasted drafts cheap. Do not attempt token-by-token LLM revision.
- **Emotion + script as cross-refs**: VAD `{valence,arousal,dominance}` in node metadata (arousal *is* the 0.15 urgency term; dominance models floor control); `inject_impulse` soft-rebases an in-flight response (the model for tool-results / env-events arriving mid-thought).

## Build targets (what must be built — `file:line` anchors)

1. **Speculative-state assignment + per-turn Frontier→Committed supersession** (`context_graft.rs:46/393`; today Speculative is unused, commit is conv-end-only).
2. **Response-FROM-node rendering** (the biggest gap): no node→render(→TTS) path exists; today graft puts context *into the prompt*. Speculative node = cheap graph-local read → TTS; Committed node = graft→LLM supersedes via `Contradicts`/`Follows`.
3. **Floor-scoring + commit/prune on a tick** (the running tick is coherence-only): the new Talk-Mode `SystemService`.
4. **Turn impulses 0x50/51/52/60 + `Continuer` cross-ref** (promote from `Custom`, `impulse.rs:36` / `crossref.rs`).
5. **Join the graphs** (`loop_core.rs:679-744`, `substrate_sink.rs:329-366`, `session_tier.rs:239`, `causal.rs:429/503`).
6. **Overt-repair path** for Speculative→Committed contradictions (D3).

## Consequences

**Positive:** routing collapses into one mechanism (walk-and-mutate; node state is the answer); the dormant ECC substrate finally drives conversation; multiplayer + tiered fall out of a graph that is participant-agnostic; the design has academic lineage (IU + graph-DM) and a clear duplex migration path (make the node loop audio-native — *not* Moshi); the witness chain gives provenance + a self-improvement corpus (the eval-data gap the field hasn't solved).

**Negative / risks:** joining the two graphs touches the live conversation path (must stay green behind the `voice` feature); "rebase an in-flight LLM response" is cancel+re-prompt (seconds, not ms) — partial-node semantics must be explicit; the floor/coherence formulas are hand-tuned until the Weaver learns them.

**Deferred (not v0.1):** Weaver/SONA-learned floor weights; sheaf-Laplacian coherence; VAP retrain (weights academic-only); the full `select_merge_strategy` set (start with Priority); a hierarchical-EOT vs smart-turn-v3 latency bench (2603.13379 reports ~36ms vs ~800-1300ms — a real lab item).

## ADR updates this implies

- **ADR-061** (voice loop): reference ADR-062 as the conversation model; replace the dual-layer "expressive routing" heuristic with the Speculative/Committed node lifecycle; record the native-Rust component stack + no-Python decision; note Orpheus = Ollama token-gen + native SNAC decode.
- **ADR-058** (context tier): frame the L2 `SessionView` as the *frontier-walk substrate* that must be **joined to the `CausalGraph`** (lineage + cross-refs), not cosine-only; note per-turn (not just conv-end) commit.
- **ADR-053** (STT): record the native-Rust ONNX path (parakeet via sherpa-onnx/`ort`) as the cross-platform default alongside the substrate HTTP backend.

## Alternatives considered

- **Full-duplex S2S (Moshi/Kyutai)** — true talking-while-listening, ~200ms, but **welds the LLM + voice** (loses the swappable brain). Confirmed by the field as a swappability trap; "evaluate, don't bet." The duplex migration path is making *this* node/unit loop audio-native (the 2601.20230 semi-cascade), not adopting Moshi.
- **Keep the heuristic controller** (the 6.7 `TalkModeController` as-is) — rejected: it bolts routing onto a controller instead of deriving it from the conversation model; it cannot express backchannel/floor/repair as first-class.
