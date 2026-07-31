# WEFT-617 — MidStream for voice/ECC mid-stream gating (50 ms CognitiveTick)

> **Status:** Evaluation complete (spike + decision). Scaffold hooks landed.
> **Ticket:** WEFT-617 · cycle 0.8.x · lane E · labels `ws10-voice`, `gap`, `ruv-integration`
> **Date:** 2026-07-31
> **Detailed integration plan:** [`.planning/ruv/integration/midstream-integration-plan.md`](../../.planning/ruv/integration/midstream-integration-plan.md)
> **Scaffold:** `crates/clawft-voice-talk/src/midstream.rs`

---

## 1. Spike question

Voice/ECC needs mid-utterance interrupt/commit decisions on the kernel’s
[`CognitiveTick`](../../crates/clawft-kernel/src/cognitive_tick.rs) (default
**50 ms**, adaptive, budget-aware). Does
[ruvnet/midstream](https://github.com/ruvnet/midstream) provide primitives that
fit that tick for gating **without** a bespoke control plane?

---

## 2. Verdict (adopt / adapt / drop)

| MidStream surface | Decision | Rationale |
|---|---|---|
| `midstreamer-temporal-compare` (DTW / LCS / edit-distance / `find_similar`) | **ADOPT (vendored, Phase A–B)** | Fills a real gap: sequence alignment on partials / short VAP windows. We have spectral/EML coherence, not alignment. |
| `midstreamer-neural-solver` LTL `verify()` | **REFERENCE only** | Bounded checker is real; `"neural"` is marketing; `synthesize_controller` is a stub. Lab-prototype floor-safety specs first. |
| `midstreamer-scheduler` | **DROP** | Redundant with `ImpulseQueue` + `CognitiveTick`. |
| `midstreamer-strange-loop` | **DROP** | Overlaps `WeaverEngine` / `ModelingSession`. |
| `midstreamer-attractor` | **DROP** | Off turn-taking path. |
| `midstreamer-quic` | **DROP (voice path)** | Off 50 ms critical path; WeftOS owns transport/substrate. |

**Headline framing (“gate and steer LLM tokens mid-stream”) is not adopted.**
MidStream’s “gating” is *analysis → a decision your app acts on*. That is already
our ECC pattern: impulse → floor verdict → Talk-Mode cancels/commits TTS. Only
alignment **primitives** are additive.

**Recommended path:** vendor the small-n DP subset (not crates.io 0.x dep),
feature-gate under voice, never own control flow from the analyzer.

---

## 3. 50 ms CognitiveTick contract

| Constraint | Value / owner |
|---|---|
| Default interval | `CognitiveTickConfig::tick_interval_ms = 50` |
| Compute budget | `tick_budget_ratio` (default 0.3) of interval → ~15 ms soft budget |
| Coherence path | `run_democritus_loop` — O(1) EML predict every tick; Lanczos spectral on drift |
| Event bus | `ImpulseQueue::emit` / `drain_ready` (HLC-sorted) |
| Floor / commit authority | `TalkModeLoop` on tick (not the audio layer) |
| Audio layer role | Emit impulses only (`KernelImpulseSink`, STT partials, VAP priors) |

### Latency fit (Adopt piece)

| Op | Complexity | Tick-safe window | Est. cost @ n,m ≤ 64 |
|---|---|---|---|
| edit-distance / LCS / DTW (full matrix) | O(n·m) | yes | single-digit µs |
| `find_similar` (sliding DTW) | O(H · N²) | yes if H,N capped | sub-100 µs with ring buffer |
| `detect_recurring_patterns` / fuzzy miners | O(n³)-class | **no** | Weaver offline / batch only |
| LTL `verify` on short traces | O(trace²) worst | lab only | negligible on few states |

**Scaffold enforces** `TICK_WINDOW_CAP = 64` tokens/frames on the hot path
(`midstream.rs`). Pattern miners stay off-tick by construction.

---

## 4. Capability map (need → owner)

| Voice/ECC need | WeftOS today | MidStream | Action |
|---|---|---|---|
| 50 ms tick orchestration | `CognitiveTick` | scheduler | keep ours |
| Real-time turn signals | `ImpulseQueue` + turn codes 0x50–0x60 | scheduler PQ | keep ours |
| Coherence / spectral gate | DEMOCRITUS + `spectral_analysis` | — | keep ours |
| **Partial / IU prefix-vs-revise** | common-prefix scaffold only | **temporal-compare** | **Phase A vendor** |
| **Stall / loop → alert** | none on token window | pattern / `find_similar` | **Phase B** → `CoherenceAlert` |
| Floor-safety invariants | imperative Talk-Mode | LTL AST | reference / lab |
| Speculative→Committed lifecycle | ECC `metadata.state` + edges | strange-loop | keep ours |
| Barge-in cancel | Talk-Mode + ERL path (ADR-068) | synthesize stub | no-go |
| Weaver offline mining | `ModelingSession` | pattern miners | optional Phase D |

---

## 5. Integration points (no control-flow ownership)

1. **STT partials / IU restart-and-revise** — `clawft-voice-onnx` Parakeet partials
   → `MidstreamAnalyzer::diff_partial` → add / revise / commit of stable prefix
   before a `CausalNode` is written.
2. **Repetition / stall** — bounded token ring →
   `detect_stall` → `ImpulseType::CoherenceAlert` via `MidstreamImpulseBridge`
   (analysis helper only).
3. **VAP-frame DTW prior (optional)** — short energy/VAP windows vs continuer
   templates → cheap prior before model verdict (`Custom` / turn impulse map).
4. **Weaver batch (optional)** — offline `detect_recurring_patterns` over recorded
   Looms; never on tick.

Scaffold types: `MidstreamAnalyzer`, `PrefixMidstreamAnalyzer`,
`MidstreamImpulseBridge` in
[`crates/clawft-voice-talk/src/midstream.rs`](../../crates/clawft-voice-talk/src/midstream.rs).

---

## 6. Decision criteria (go / kill)

**Adopt vendored temporal-compare when** (all hold):

- fills a missing capability (alignment) rather than duplicating the kernel;
- zero or one small external dep (vendor → zero);
- `unsafe`-free, WASM-clean under `voice`;
- tick-hot windows hard-capped.

**Kill Phase A DTW** if prefix-diff does not beat the scaffold’s common-prefix
compare on partial churn / commit timing — keep `PrefixMidstreamAnalyzer` only.

**Never:** import the whole MidStream workspace, adopt the scheduler, or claim
“we steer tokens via MidStream” after taking only DP helpers.

---

## 7. Follow-up work items

| Phase | Scope | Effort | Plane |
|---|---|---|---|
| **A** | Vendor DTW/edit-distance; wire IU prefix path; tests | ~1–2 d | **WEFT-714** |
| **B** | Stall/repetition analyzer → `CoherenceAlert` on tick | ~1–2 d | **WEFT-715** |
| C (optional) | VAP-frame DTW prior + lab A/B | ~2–3 d | only if lab asks |
| D (optional) | Weaver offline miners | ~2 d | only if Loom mining earns it |
| LTL (lab) | Floor-safety invariants prototype | ~1–2 d | reference track |

Recommended ship scope for 0.8.x: **A + B only**.

---

## 8. Acceptance criteria (WEFT-617)

| AC | Status |
|---|---|
| Spike: midstream vs 50 ms CognitiveTick contract | **Done** — §3–§4 |
| Decide adopt / adapt / drop; record in `.planning/ruv/` | **Done** — plan + this doc; decision stamp in plan §0 |
| If adopt: integration follow-up item | **Done** — Phase A/B issues |

Evaluation only for this ticket; no full DTW vendor in WEFT-617. Scaffold hooks
land so Phase A can attach without reinventing seams.

---

## 9. Evidence anchors

- MidStream: clone review 2026-07-03 — `temporal-compare` solid; scheduler
  redundant; LTL `synthesize_controller` stub; BENCHMARKS.md self-retracts
  headline latency/QUIC claims.
- WeftOS: `cognitive_tick.rs` (50 ms default), `impulse.rs` (turn codes +
  `CoherenceAlert`), `TalkForest` / `TalkModeLoop`, voice-ecc synthesis §B.2,
  ADR-047 / ADR-058..062 / ADR-068.
- Prior plan: `.planning/ruv/integration/midstream-integration-plan.md`.
