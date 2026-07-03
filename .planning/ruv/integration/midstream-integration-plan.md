# MidStream → WeftOS Voice/ECC Integration Plan

> **Status:** Analysis + plan (no code changes). Author: midstream-integration-planner (ruv hive).
> **Date:** 2026-07-03
> **Source reviewed:** `github.com/ruvnet/midstream` @ default branch (workspace `0.2.x`, MIT OR Apache-2.0),
> full source clone. Cross-referenced against WeftOS `feat/hermes-loop-base`.
> **Verdict up front:** **Selective adopt (one crate) + reference (one design). No-go on framework/wholesale adoption.**

---

## 0. Executive summary + go/no-go

**Leaning: qualified GO on `midstreamer-temporal-compare` only; REFERENCE-only on the LTL solver; NO-GO on everything else.**

MidStream markets itself as "gate and steer LLM tokens mid-stream" with headline numbers
(DTW ~38µs, ns-scale scheduler, >1 GB/s QUIC). **MidStream's own `docs/BENCHMARKS.md` (driven by its
ADR-0009) retracts almost all of those numbers**: of 22 advertised targets, it says ~4 are credible; the
scheduler "<100 ns" claim is called "mathematically incompatible with the implementation," the QUIC
benches run against an in-memory mock (no quinn, no UDP, no TLS), and the end-to-end numbers are
"construction-overhead-polluted." This is a project being honest about being early-stage, which is to its
credit — but it means we must judge it on the *code*, not the pitch.

On the code:

- **`temporal-compare`** is genuinely useful and clean: textbook DP implementations of DTW / LCS /
  edit-distance + sliding-window fuzzy match, zero `unsafe`, WASM-clean, and a **near-perfect dependency
  match** with our workspace. This is the one piece worth taking. It gives us sequence-alignment primitives
  we do **not** currently have (our coherence path is spectral/EML, not alignment-based).
- **`temporal-neural-solver`** is mislabeled: the "neural" part does not exist (no model, no weights);
  `verify()` is a small but real bounded LTL model-checker, and `synthesize_controller()` is a hardcoded
  stub returning `["action1","action2"]`. The **LTL AST + checker is worth studying** as a way to express
  floor-safety invariants declaratively, but it is not load-bearing and should not be adopted as a dependency yet.
- **`nanosecond-scheduler`** is a priority queue behind three `RwLock` guards per push. It is **functionally
  redundant** with our `ImpulseQueue` (HLC-sorted) + `CognitiveTick`. Adopting it would duplicate the kernel.
- **`strange-loop`** is a meta-learning wrapper that **overlaps `WeaverEngine`/`ModelingSession`**, which is
  more tightly integrated with our ECC. Redundant.
- **`temporal-attractor-studio`** (Lyapunov / chaos detection) has no bearing on turn-taking. Out of scope.
- **`quic-multistream`** is real quinn code but its benches are mock-only, and QUIC transport is **not on the
  50ms voice tick's critical path**. Relevant at most to the future multiplayer/remote-participant story,
  where WeftOS already owns its own channel/substrate layer. Out of scope for voice.

**Top 3 reasons for the leaning:**
1. **Only one of six crates is both non-redundant and production-quality for our need** (`temporal-compare`),
   and even it is ~800 lines we could vendor rather than depend on. The rest is either already-owned by the
   ECC kernel (scheduler, strange-loop → ImpulseQueue/CognitiveTick/Weaver) or off the voice path (QUIC, attractor).
2. **MidStream does not actually "steer" a decoder.** Its "gating" is *analysis → a decision your app acts on* —
   which is exactly what our ECC already is (impulse → floor verdict → Talk-Mode cancels TTS). The orchestration
   shell is redundant; only the analysis *primitives* are additive.
3. **The headline latency claims are disowned by the project itself**, so the "fits in 50ms" argument has to be
   made from the algorithms (small-n DP is genuinely microsecond-scale) — which happens to hold for the adoptable
   piece and removes any reason to take the branded framing.

---

## 1. What MidStream actually is (claim vs. code)

One Cargo workspace, six published library crates at a shared version, plus a binary, WASM bindings, and npm
shims. MSRV badge 1.81 / crate manifests pin `rust-version = "1.88"`. Dual MIT OR Apache-2.0. Workspace lints
are strict (`unsafe_code = "deny"`, `unused_must_use = "deny"`, `dbg_macro`/`todo`/`unimplemented` denied) and
the review notes zero `unsafe` in workspace code.

| Crate | What it *is* in the source | Reality check |
|---|---|---|
| `midstreamer-temporal-compare` | DTW (full-matrix O(n·m) w/ backtrack alignment), LCS, Levenshtein, "euclidean" (mismatch count), LRU-cached `compare()`, plus `find_similar` (sliding-window normalised-DTW match), `detect_pattern`, `detect_fuzzy_patterns`, `detect_recurring_patterns`. Deps: serde, thiserror, dashmap, lru. **No tokio; WASM-clean.** | **Solid.** Real, tested, property-tested. DTW is unbanded (no Sakoe-Chiba) so it's O(n·m) — fine for small windows, not for long sequences. The pattern-miners are brute-force O(n³)-ish — offline-only. |
| `midstreamer-scheduler` | Priority queue (`ScheduledTask`, `Priority`, `Deadline`, `laxity`) behind `parking_lot::RwLock`; EDF/laxity ordering; tokio optional (`native` feature), core is WASM-clean. Deps: crossbeam, parking_lot. | **Redundant** with our `ImpulseQueue`+`CognitiveTick`. Its own `BENCHMARKS.md`/ADR-0033 retract the "ns" branding (3 RwLock guards per `schedule()`). |
| `midstreamer-neural-solver` | LTL AST (`TemporalFormula` G/F/X/U/¬/∧/∨), bounded recursive model-checker `verify()` over a `TemporalTrace`, heuristic `calculate_confidence()`. Deps: ndarray, scheduler. | **"Neural" is marketing** — no model. `verify()` is real & useful for safety specs. `synthesize_controller()` is a **stub** (`Ok(vec!["action1","action2"])`). |
| `midstreamer-attractor` | Lyapunov exponents, phase-space reconstruction, attractor classification. Deps: nalgebra, ndarray. | **Off-topic** for turn-taking. |
| `midstreamer-strange-loop` | Meta-levels, `MetaKnowledge`, `SafetyConstraint`, self-modification rules; composes the four crates above. | **Overlaps `WeaverEngine`/`ModelingSession`.** |
| `midstreamer-quic` | quinn-backed multistream on native, secure-by-default TLS (platform verifier; insecure skip behind a feature), thin WASM shim. Deps: quinn, rustls-platform-verifier, rcgen. | Real transport, but **benches are mock-only** and it's **off the voice critical path**. |

---

## 2. Capability mapping — our 50ms-tick need → MidStream primitive

Our need (from `voice-ecc-synthesis.md` §B.2, ADR-058..061, and the working Talk-Mode loop on
`feat/hermes-loop-base`): mid-utterance gating/steering on the 50ms `CognitiveTick`; backchannel modeled as a
`CrossRef` not a turn; barge-in that cancels TTS < 50ms; a speculative/committed node lifecycle; a Weaver that
learns floor weights.

| Our need | Where it lives today (WeftOS) | MidStream primitive | Verdict |
|---|---|---|---|
| 50ms tick orchestration / drain events | `cognitive_tick.rs:92` `CognitiveTick` (default 50ms, adaptive, drift) | `midstreamer-scheduler` | **Diverge/Redundant** — kernel already owns this. |
| Real-time event queue (EOU/VAP/turn signals) | `impulse.rs:114` `ImpulseQueue` (`emit`/`drain_ready`, HLC-sorted) | `midstreamer-scheduler` priority queue | **Redundant.** ImpulseQueue is the scheduler. |
| Coherence / transcript-gate verdict | `cognitive_tick::run_democritus_loop` (two-tier EML predict + Lanczos spectral) + `causal::spectral_analysis` (`causal.rs:795`) | — (MidStream has no equivalent) | **Keep ours.** Nothing to adopt. |
| **Sequence alignment on token/partial/VAP-frame windows** (repetition/stall detection, prefix-vs-template match, IU restart-and-revise diffing) | **Nothing today** — we have spectral coherence, not alignment | **`temporal-compare`** DTW/LCS/edit-distance + `find_similar` | **ADOPT (complement).** This is the genuine gap it fills. |
| Declarative floor-safety invariants ("Globally ¬talk_over_user", "Finally yield_floor") | Encoded imperatively in impulse scoring / Talk-Mode controller | `neural-solver` LTL `verify()` | **REFERENCE** — study the AST+checker; possibly adopt the ~400-line checker later, not now. |
| Speculative→Committed node lifecycle | ECC node `metadata.state` + `Contradicts`/prune (synthesis §C.7) | `strange-loop` meta-nodes | **Diverge** — ECC modeling is richer & native. |
| Weaver-learned floor weights | `weaver.rs:1142` `WeaverEngine` + `ModelingSession` (`weaver.rs:165`) | `strange-loop` meta-learning | **Redundant** — Weaver is the native form. |
| Barge-in decision / TTS cancel | Talk-Mode controller reads floor verdict (synthesis §B.2 step 9) | `neural-solver synthesize_controller` | **No-go** — that function is a stub. |
| Multi-participant / remote transport | WeftOS substrate + `ChannelAdapter`/`VoiceChannel` | `quic-multistream` | **Out of scope** for voice; revisit only for remote multiplayer. |

**Net:** exactly one **Adopt** (`temporal-compare`), one **Reference** (LTL checker), the rest **Redundant/Diverge/Out-of-scope**.

---

## 3. Integration points on our side (file citations)

If we adopt `temporal-compare`, it slots in as a **pure analysis helper feeding the impulse bus** — it never
owns control flow (consistent with the synthesis mandate that the audio layer *emits impulses*, it does not decide).

1. **STT partials / IU restart-and-revise** — `crates/clawft-voice-onnx/src/parakeet.rs` &
   `parakeet_tdt.rs` produce streaming partials. Use `edit_distance`/`find_similar` to diff a new partial
   against the last *stable* prefix to decide add/revoke/commit (synthesis §C.7). Emits nothing to the kernel
   directly — it stabilizes the text that later becomes a `CausalNode`.
2. **Repetition / stall / loop detection in the token stream** — a small analyzer that runs
   `detect_pattern`/`find_similar` over the recent token window and, on a hit, calls
   `ImpulseQueue.emit(ImpulseType::CoherenceAlert)` (`impulse.rs`). This is the one place MidStream's
   "inflight analysis" thesis maps cleanly onto our `CoherenceAlert` path.
3. **VAP-frame / turn-signal template matching** — in `crates/clawft-voice-talk/src/ecc.rs` (the audio→ECC
   bridge) and `session.rs`, DTW-match short VAP/energy frame windows against continuer/turn-shift templates as a
   cheap prior *before* the model verdict — feeding the `Custom(0x50/0x52/0x60)` impulse mapping (synthesis §C.3).
4. **Weaver offline pattern mining** — `detect_recurring_patterns`/`detect_fuzzy_patterns` (too heavy for the
   tick) are a fit for a `WeaverEngine::ModelingSession` batch pass over a recorded conversation Loom
   (`weaver.rs:165`), e.g. mining recurring interruption→floor-loss sequences.

Feature-gate all of it behind the existing `voice` feature (see §4). Nothing above touches `clawft-kernel`
public types — it's additive glue in the voice crates.

---

## 4. Dependency / build impact

**Toolchain:** MidStream MSRV 1.88 ≤ our pinned 1.93 (`rust-toolchain.toml`, workspace `Cargo.toml`
`rust-version = "1.93"`). No toolchain conflict.

**Dependency alignment (excellent for `temporal-compare`):**

| Dep | MidStream | WeftOS workspace | Conflict? |
|---|---|---|---|
| serde | 1 | 1 | none |
| thiserror | 2.0 | 2 | none |
| dashmap | 6.1 | 6 | none |
| **lru** | 0.16 | **not currently used** | **new transitive dep** (small, well-trodden) |
| tokio | 1.42 | 1 | none (temporal-compare doesn't use tokio) |
| ndarray | 0.16 | 0.16 | none (only if we ever take neural-solver/attractor) |
| **nalgebra** | 0.34 | **not currently used** | new dep — **only** if attractor adopted (we won't) |
| **crossbeam** | 0.8 | **not currently used** | new dep — **only** if scheduler adopted (we won't) |
| quinn/rcgen/rustls-platform-verifier | present | — | **only** if QUIC adopted (out of scope) |

So the **Adopt path adds exactly one new small crate to the tree (`lru`)** and nothing else. `temporal-compare`
has no tokio/OS deps, so it will not perturb the `voice` feature's native/WASM split.

**Two ways to take it, pick per risk appetite:**
- **(a) Dependency** on `midstreamer-temporal-compare = "0.2"` from crates.io, behind `voice`. Pros: upstream
  fixes/property-tests come along. Cons: adds an external maintainer + `lru`; couples us to a 0.x that its own
  BENCHMARKS.md shows is still churning.
- **(b) Vendor** the `lib.rs` (DTW/LCS/edit-distance + `find_similar`) into a small internal module
  — *measured 896 lines total (≈798 code + ≈98 inline `#[cfg(test)]` from line 799), `unsafe` count 0,
  verified 2026-07-03 against the primary clone; the earlier "~700-line" figure understated it. The
  subset we keep on the tick (exclude the O(n³) `detect_recurring_patterns`/`detect_fuzzy_patterns`
  miners) is smaller, but the file to lift-and-trim is ~800 LOC* —
  under `clawft-voice-talk` (MIT/Apache attribution preserved). Pros: zero new external deps (drop `lru`, use
  our own cache or none), full control, WASM-trivial. Cons: we own it. **Recommended** — the surface we want is
  small, stable maths; vendoring removes supply-chain and version-churn risk for a trivial code cost.

**Build gate:** whichever path, it must pass `scripts/build.sh gate` (11 checks) and `scripts/build.sh clippy`
(warnings-as-errors) before commit, per repo rule. `temporal-compare`'s own lints are already `-D warnings`
clean, so vendored code should pass with minimal massaging.

---

## 5. Latency budget analysis — does it fit the 50ms tick?

The 50ms `CognitiveTick` already budgets for SNAC decode, AEC (`clawft-voice-aec`, WebRTC AEC3), VAD, and the
`run_democritus_loop` coherence pass. Any adopted analysis must be a *small slice* of the remaining headroom.

- **DTW/LCS/edit-distance are O(n·m)** with tiny constants (integer/float DP, no allocation beyond the matrix).
  For the window sizes we'd actually feed the tick — recent-token windows or VAP frame windows of n,m ≲ ~64 —
  that's a few thousand FP ops, i.e. **single-digit microseconds**, comfortably inside the tick. The retracted
  "38µs" headline is irrelevant; we only ever run it on small n, where it's cheaper still.
- **`find_similar` (sliding-window DTW)** is O(H · N²) for haystack H, needle N. Keep H and N bounded (a short
  ring buffer of recent partials/frames, not the whole utterance) and it stays sub-100µs. **Do not** run it over
  a full utterance on the tick.
- **`detect_recurring_patterns` / `detect_fuzzy_patterns` are O(n³)-class** brute-force miners — **explicitly
  off the tick.** Confine them to the Weaver's offline/batch `ModelingSession` pass.
- **LTL `verify()`** is bounded-recursive; `Until`/`Globally` are O(trace²) worst-case. On the short traces a
  floor-safety spec would use (last few states), it's negligible — but this is a *reference* item, not a tick adoption.

**Conclusion:** the Adopt piece fits the tick trivially **provided we cap window sizes** and keep the
pattern-miners off the hot path. This is a design constraint, not a blocker.

---

## 6. Phased adoption

- **Phase A — Vendored DTW/edit-distance helper (small, low-risk).** Bring the DP functions into a
  `clawft-voice-talk` submodule behind `voice`. Wire use-case #1 (IU restart-and-revise prefix diff in the
  Parakeet partial path) — the highest-value, lowest-risk use. Gate: `build.sh gate` green, a unit test on
  add/revoke/commit against recorded partials.
- **Phase B — Coherence/stall analyzer → `CoherenceAlert`.** Add use-case #2 (repetition/stall detection over a
  bounded token window emitting `ImpulseType::CoherenceAlert`). This is where MidStream's "inflight analysis"
  thesis actually earns its keep in our stack. Gate: shows up as a floor verdict in the Talk-Mode loop.
- **Phase C — VAP-frame DTW prior (optional).** Use-case #3, a cheap alignment prior feeding the turn-signal
  impulse mapping, *only if* the lab (`llm`) shows it improves backchannel-vs-interruption precision over the
  MaAI/VAP model alone. Otherwise skip.
- **Phase D — Weaver offline miner (optional).** Feed `detect_recurring_patterns` into a `ModelingSession` batch
  pass over recorded Looms. Off the tick by construction.
- **LTL (separate track, reference-first).** Prototype a handful of floor-safety invariants against the
  neural-solver AST *in the lab* to see whether a declarative safety layer pulls its weight before we adopt any
  checker code. No kernel change until proven.

Phases A–B are the whole recommended scope. C/D/LTL are conditional on measured benefit.

---

## 7. Go/no-go criteria vs. continuing hand-rolled

**Adopt `temporal-compare` (vendored) if all hold** — they do:
- It fills a capability we lack (sequence alignment) rather than duplicating the kernel. ✅
- It adds ≤1 external dep, or zero if vendored. ✅ (vendor → zero)
- It's `unsafe`-free and WASM-clean so it doesn't perturb the `voice` feature split. ✅
- The tick-hot uses are provably small-n. ✅ (with window caps)

**Stay hand-rolled / say no where** (all true here):
- The capability already exists natively and is more integrated (scheduler → ImpulseQueue/CognitiveTick;
  strange-loop → Weaver). ✅ → **no-go on scheduler, strange-loop.**
- The advertised feature is a stub or mislabeled (`synthesize_controller`, "neural"). ✅ → **no-go as dependency;
  reference-only for the LTL checker.**
- It's off the voice critical path and we already own the layer (QUIC → substrate/channels; attractor → n/a). ✅
  → **out of scope.**

**Kill criteria for the Adopt track:** if Phase A's prefix-diff doesn't measurably reduce partial churn / improve
commit timing versus a trivial common-prefix check, drop DTW and keep the plain prefix compare — don't carry the
code for its own sake.

---

## 8. Effort estimates

| Item | Effort | Notes |
|---|---|---|
| Phase A (vendor DTW/edit-distance + IU prefix diff, tests) | **~1–2 dev-days** | ~800 LOC to lift + trim (lib.rs is 896 lines incl. ~98 inline tests); wiring into `parakeet.rs` partial path; unit tests. |
| Phase B (stall/repetition analyzer → CoherenceAlert) | **~1–2 dev-days** | Bounded-window analyzer + impulse emit + Talk-Mode assertion. |
| Phase C (VAP-frame DTW prior) | **~2–3 dev-days** + lab eval | Conditional; needs `llm`-lab A/B vs VAP-only. |
| Phase D (Weaver offline miner) | **~2 dev-days** | Batch `ModelingSession` integration; off tick. |
| LTL reference prototype (lab) | **~1–2 dev-days** | AST + a few invariants, no kernel change. |
| **Recommended scope (A + B)** | **~3–4 dev-days** | The whole justified adoption. |

---

## 9. Risks

- **Version churn / 0.x instability (if depended, not vendored).** MidStream's own BENCHMARKS.md shows active
  cleanup and retraction; a 0.x API can move under us. **Mitigation:** vendor (Phase-A path (b)).
- **Cargo-culting the framing.** The real risk is adopting MidStream's *narrative* ("we now gate/steer tokens")
  when we've only taken a DTW function. Our steering is the ECC; be precise in docs/PRs so nobody over-claims.
- **Tick budget regression.** Naive `find_similar`/pattern-miners on long sequences would blow the 50ms budget.
  **Mitigation:** hard window caps; keep miners off the hot path; add a bench in `scripts/build.sh` if adopted.
- **Redundant-import temptation.** Pulling the whole workspace "to get the scheduler too" would duplicate the
  kernel. **Mitigation:** take exactly one crate (or vendor one module); never the workspace.
- **LTL over-investment.** A declarative safety layer is attractive but unproven for our floor logic; the
  `synthesize` half is a stub. **Mitigation:** reference-first, lab-prototype, no kernel change until it earns adoption.
- **Attribution.** MIT OR Apache-2.0 requires license/NOTICE retention when vendoring. **Mitigation:** carry the
  header + a NOTICE entry.

---

## Appendix — evidence anchors

**MidStream (clone):**
- `docs/BENCHMARKS.md` §1 — self-retraction of QUIC/scheduler/DTW headline numbers; ADR-0009 "honest benchmarks",
  ADR-0033 scheduler SLO.
- `crates/temporal-compare/src/lib.rs` — `dtw_sequences` (full-matrix, backtrack) `:179`; `compare` `:278`;
  `find_similar` (sliding-window normalised DTW); `detect_fuzzy_patterns`/`detect_recurring_patterns` (brute-force).
- `crates/temporal-neural-solver/src/lib.rs` — `verify`/`check_formula` (real bounded LTL) `:252`+;
  `synthesize_controller` **stub** returning `["action1","action2"]`; no neural model present.
- `crates/nanosecond-scheduler/src/lib.rs` — `schedule` `:224` (3× RwLock), `next_task` `:253`; `native` feature gates tokio.
- Workspace `Cargo.toml` — six members; deps tokio 1.42 / serde 1 / thiserror 2 / dashmap 6.1 / lru 0.16 / ndarray 0.16 / nalgebra 0.34.

**WeftOS side:**
- `.planning/voice-ecc-synthesis.md` §B.2 (tick loop), §C.3 (turn-signal→impulse map), §C.7 (IU restart-and-revise / Speculative nodes).
- `crates/clawft-kernel/src/`: `cognitive_tick.rs:92` (50ms tick, `run_democritus_loop`), `impulse.rs:114` (`ImpulseQueue`),
  `causal.rs:795` (`spectral_analysis`), `weaver.rs:1142`/`:165` (`WeaverEngine`/`ModelingSession`).
- Voice crates: `clawft-voice-onnx/src/parakeet.rs`, `parakeet_tdt.rs`, `smart_turn.rs`; `clawft-voice-talk/src/ecc.rs`, `session.rs`;
  `clawft-voice-aec` (WebRTC AEC3); `clawft-voice-tts` (kokoro/orpheus/snac).
- Toolchain: `rust-toolchain.toml` channel 1.93; workspace `Cargo.toml` `rust-version = "1.93"`.
