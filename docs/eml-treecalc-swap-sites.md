# EML + tree-calculus swap sites — running catalog

A running inventory of places in the tree where an `eml-core::EmlModel`,
the tree-calculus Form triage, or an already-implemented fast path can
replace a hot-path heuristic. Discovered while triaging the DEMOCRITUS
loop CPU-spin bug (see `Finding #1`).

**Conventions.** Each entry is:
- a file + line range,
- the current code (a heuristic or heavy computation),
- the swap (EML model, treecalc dispatch, or existing-but-unused fast path),
- expected impact (speed or behaviour).

Priority tags:
- `[P0]` — fixes an active bug or CPU-spin
- `[P1]` — live callsite, unused model already exists → wiring job
- `[P2]` — opportunity, needs a new model
- `[research]` — speculative / needs measurement

In-tree markers use `// TODO(eml-swap):` / `// TODO(treecalc-swap):`
so future grep picks them up.

---

## Immediate (tied to current bug)

### [P0] Finding #1 — DEMOCRITUS `needs_exact` sentinel forces spectral every tick

- **Where:** `crates/clawft-kernel/src/cognitive_tick.rs:373-381`
- **Current:**
  ```rust
  let needs_exact = drift > drift_threshold
      || ticks_since_exact >= exact_every_n
      || last_exact_coherence == 0.0; // first tick always exact

  if needs_exact {
      let spectral = causal.spectral_analysis(50);
      let exact_lambda_2 = spectral.lambda_2;
      ...
      last_exact_coherence = exact_lambda_2;
  ```
- **Bug:** on an empty causal graph (`causal=0, hnsw=0` at boot, visible
  in every `kernel.log` boot banner), `spectral_analysis` returns
  `lambda_2 = 0.0` every call. `last_exact_coherence == 0.0` stays true
  forever → `needs_exact` stays true → O(k·m) Lanczos runs every tick
  instead of every 100th.
- **Swap:** the two-tier design already intends EML `predict` to
  short-circuit spectral; the sentinel is the blocker. Fix the sentinel
  (`Option<f64>` or a `first_exact: bool`) and the EML fast path starts
  doing its job.
- **Impact:** 99% drop in `spectral_analysis` calls on steady state.
  On this machine that's the difference between 53% CPU pinned and
  <1% idle.

### [P0] Finding #2 — `detect_conversation_cycle` warn has no rate-limit

- **Where:** `crates/clawft-kernel/src/cognitive_tick.rs:404-417`
- **Current:**
  ```rust
  if coherence_history.len() % 20 == 0 {
      let state = detect_conversation_cycle(&coherence_history, 20, 0.01);
      match state {
          ConversationState::Stuck { .. }
          | ConversationState::Oscillating { .. } => {
              tracing::warn!("DEMOCRITUS: conversation appears stuck: {:?}", state);
          }
          _ => {}
      }
  }
  ```
- **Bug:** no edge-trigger, no backoff. Fires ~4.5 Hz on an idle graph.
  `kernel.log` at 5 MB and 31 000 warnings after 5 h uptime.
- **Swap (minimal):** log only on state *transitions* (Stuck→not-Stuck
  or not-Stuck→Stuck) with an exponential suppression counter for the
  steady-state case.
- **Swap (EML, P2 extension):** replace `detect_conversation_cycle`'s
  hardcoded thresholds with a trained classifier — see Finding #4.
- **Impact:** log stops drowning real warnings; no CPU delta but
  restores the signal value of `WARN` level.

### [P1] Finding #3 — `coherence_history: Vec<f64>` grows unbounded

- **Where:** `crates/clawft-kernel/src/cognitive_tick.rs:326, 402`
- **Current:** `let mut coherence_history: Vec<f64> = Vec::new();`
  with `coherence_history.push(exact_lambda_2)` every exact tick, no
  cap.
- **Swap:** `VecDeque<f64>` with a fixed capacity of ≥ the detector
  window (`20` today). `detect_conversation_cycle` only reads the
  tail anyway.
- **Impact:** bounded memory; trivial to do alongside Finding #1.

---

## Already-implemented fast paths that are unused

### [P1] Finding #4 — `spectral_analysis_rff` exists and is 3–6× faster, not called

- **Where:** fast path defined at
  `crates/clawft-kernel/src/causal.rs:1001`, used by 0 production
  callers (only tests at `:1164, :2929, :2955, :2981`).
- **Current claim in doc-comment:** *"O(m) per feature vector — 3-6x
  faster than Lanczos on large graphs"*, using random Fourier features
  to approximate the Laplacian spectrum.
- **Swap:** the DEMOCRITUS loop's exact-tier call
  (`cognitive_tick.rs:381`) can drop to `spectral_analysis_rff(64, 50)`
  once Finding #1 is fixed and the "exact" tier is genuinely rare.
  Keep `spectral_analysis` as the ground-truth sampler used every
  `train_every_n` cycles to retrain the EML model.
- **Impact:** 3–6× speed on the rare-but-real exact path. Zero code
  changes needed in `spectral_analysis_rff` itself.

### [P1] Finding #5 — `eml_kernel::*` models defined, persisted, never called

File: `crates/clawft-kernel/src/eml_kernel.rs` plus
`eml_persistence.rs` (which defines a `KernelModelSet` and writes them
to JSON). None have a non-test call site outside those two files. The
kernel just keeps using the hardcoded heuristics these are supposed
to replace.

| Model (defined) | Hardcoded logic it should replace | Current heuristic location |
|---|---|---|
| `GovernanceScorerModel` | `EffectVector::magnitude()` (L2 norm, 5 dims) | `governance.rs:141` |
| `RestartStrategyModel` | `RestartState::next_backoff_ms` (`100 · 2^n`, cap 30 s) | `supervisor.rs:101` |
| `HealthThresholdModel` | Fixed probe pass/fail thresholds | `health.rs` (several) |
| `DeadLetterModel` | `1000 · 2^n`, discard after 5 | used in chain.rs retries |
| `GossipTimingModel` | Fixed mesh gossip intervals | mesh/gossip config |
| `ComplexityModel` | 500-line threshold | lsp-extract / analyze |

- **Swap:** each model has a `predict` with a documented fallback
  branch when `!trained`. Wire the predict call at the heuristic's
  existing site; the untrained fallback reproduces today's behaviour
  exactly (drop-in safe). Training data comes from outcome logs
  (restart success, probe correctness, etc.).
- **Impact per model:** correctness improves over time without any
  runtime-perf regression, because the fallback is the old heuristic.
  The "speed boost" flavour of this card is that the *sites are
  already micro-seconds* — what you gain is adaptive thresholds, not
  wall-clock.

### [P1] Finding #6 — `clawft-llm::RetryModel` exported, never called

- **Where:** `crates/clawft-llm/src/eml_retry.rs` +
  `clawft-llm/src/lib.rs:69` (`pub use eml_retry::RetryModel;`)
- **Grep:** zero non-test references to `RetryModel` elsewhere in the
  tree.
- **Swap:** the LLM retry loop in `clawft-llm/src/retry.rs` should
  consult `RetryModel::predict_delay` before applying the hardcoded
  exponential backoff. Signature matches the design: 3 inputs
  (error ordinal, attempt #, hour-of-day) → 1 output (delay ms).
- **Impact:** LLM provider retries stop hammering rate-limited APIs
  at the worst intervals; provider-specific success curves get
  learned instead of hardcoded.

### [P1] Finding #7 — `WeaverEngine::recommend_tick_interval` hardcoded cpm thresholds

- **Where:** `crates/clawft-kernel/src/weaver.rs:2311-2380`
- **Current:**
  - `cpm > 10` → 200 ms
  - `1 ≤ cpm < 10` → 1000 ms
  - `cpm < 1` → 3000 ms
  - `idle_ticks ≥ 100` → 5000 ms
- **Swap:** 3-feature EML model `(cpm, idle_ticks, variance) → 1 out
  (recommended_ms)`. Same shape as the already-defined
  `GossipTimingModel`. Training signal: was the previous interval a
  good fit? (Measured by drift against the detector, or by observed
  response lag.)
- **Impact:** adaptive tick under non-stationary workload instead of
  four step-function tiers. Also becomes per-project-learnable.

---

## Tree-calculus opportunities

### [P2] Finding #8 — `detect_conversation_cycle` is a natural treecalc classifier

- **Where:** `crates/clawft-kernel/src/causal_predict.rs:175-210`
- **Current:** branches on two thresholds (`total_change < threshold`,
  `max_swing > threshold * 2`) to classify the last window as
  `Converging / Diverging / Stuck / Oscillating`. The code already
  matches the Atom / Sequence / Branch shape even though it doesn't
  call the crate.
- **Swap:** the four output variants map cleanly onto treecalc's
  three forms plus "idle" (Atom = Stuck, Sequence = Converging or
  Diverging, Branch = Oscillating). Use
  `clawft_graphify::extract::treecalc::Form`-style dispatch (lift it
  out of `extract/` into a shared `clawft-treecalc` crate, since it's
  a general algorithm, not extraction-specific) and compose the
  per-form scoring with EML for the rate estimate. See Finding #10
  for the crate move.
- **Impact:** "empty graph is not a conversation" becomes expressible
  structurally — the detector can say *"Atom without evidence of
  ever having been anything else → NotStarted, not Stuck"* rather
  than lying.

### [P2] Finding #9 — `CausalEdgeType` decay dispatch

- **Where:** `crates/clawft-kernel/src/causal.rs:~1788` (the switch on
  `Causes / Enables / Correlates` decay rates).
- **Current:** ad-hoc match on edge type with hardcoded decay coefficients.
- **Swap (treecalc):** decay-rate dispatch is already a treecalc Branch
  (typed children each decaying differently). Making this explicit
  (`Form::Branch { edge_kind }`) opens the door to:
  - SIMD-vectorising one-type-per-pass instead of branchy per-edge
  - per-type EML `DecayScheduleModel` that learns the half-life from
    observed edge-survival outcomes
- **Impact:** modest SIMD win for large causal graphs; decay rates
  become learnable per-domain.

### [P3/refactor] Finding #10 — `treecalc::Form` lives inside the extract module

- **Where:** `crates/clawft-graphify/src/extract/treecalc.rs:27-52`
- **Issue:** the `Form { Atom, Sequence, Branch }` enum + `triage`
  function are general-purpose tree-calculus; the extract module is
  the only caller only because no one else knows about it.
- **Move:** extract to `crates/eml-core/src/treecalc.rs` (same
  spirit as eml_core hosting the shared operator) or a new tiny
  `clawft-treecalc` crate. Export from workspace root so
  `clawft-kernel` can use it for Finding #8 and Finding #9 without
  pulling in graphify's dependency tree.
- **Impact:** unlocks Findings #8 and #9 without a circular dep.
  Zero perf delta on its own; structural refactor.

---

## Research-grade (needs measurement first)

### [research] Finding #11 — `surface_host::compose` tree walk

- **Where:** `crates/clawft-gui-egui/src/surface_host/compose.rs`
- **Thought:** render_node recurses through the surface tree on every
  frame. Already cheap (~µs). A treecalc triage (`Atom` = leaf
  primitive = direct render; `Sequence` = uniform container = batch
  layout; `Branch` = heterogeneous = dispatch) could simplify the code
  but probably doesn't move the needle on perf.
- **Verdict:** skip unless the composer shows up in a profile.

### [research] Finding #12 — EML for LLM cost-tracker budget reserve

- **Where:** `crates/clawft-core/src/pipeline/cost_tracker.rs:236`
- **Thought:** the comment says *"For the router hot path, prefer
  [`reserve_budget`] which atomically…"* — a pre-check EML estimator
  `(request_tokens_est, tier, recent_spend) → (over_budget?)` could
  skip the atomic fetch-add on the 99% of obvious-yes / obvious-no
  cases.
- **Verdict:** only worth it if the router is actually showing up as
  contention in a profile.

---

## The "mark it" convention

When you notice another candidate in the course of other work, drop a
line near the heuristic:

```rust
// TODO(eml-swap): [P1|P2] <one-line hypothesis> — see docs/eml-treecalc-swap-sites.md#finding-N
```

or for treecalc:

```rust
// TODO(treecalc-swap): <one-line hypothesis>
```

Then add a short entry here. Keep the doc authoritative; the inline
markers are just breadcrumbs.

---

## Status summary (as of 2026-04-21)

| Finding | Priority | Blocker | Status |
|---|---|---|---|
| #1 Sentinel forces spectral every tick | P0 | — | **Fixed** (2026-04-21) |
| #2 Stuck warn spam | P0 | — | **Fixed** (2026-04-21; edge-trigger + exp backoff) |
| #3 Unbounded coherence_history | P1 | — | **Fixed** (2026-04-21; `VecDeque` cap) |
| #4 `spectral_analysis_rff` unused | P1 | #1 fixed first | **Fixed** (2026-04-21; swapped in democritus loop) |
| #5 Kernel EML models unused (6) | P1 | — | **Fixed** (2026-04-21; all 6 wired: GovernanceScorerModel via `EffectVector::score`+`GovernanceEngine::with_scorer`; RestartStrategyModel via `RestartTracker::{next_backoff_ms,record_restart}_with_model`; HealthThresholdModel via `ProbeConfig::from_model`; DeadLetterModel via `ReliableQueue::with_model`; GossipTimingModel via `ClusterConfig::recommended_heartbeat_secs`; ComplexityModel via `ComplexityAnalyzer::with_model`) |
| #6 `RetryModel` unused | P1 | — | **Fixed** (2026-04-21; `RetryPolicy::with_model`) |
| #7 Tick-interval recommender | P1 | — | **Fixed** (2026-04-21; new `TickIntervalModel` in `eml_kernel`; `WeaverEngine::set_tick_interval_model` + `recommend_tick_interval` dispatches through model when trained, step-function preserved as fallback) |
| #8 Treecalc conversation state | P2 | #10 | **Fixed** (2026-04-21; `detect_conversation_cycle` dispatches on `Form`) |
| #9 Causal decay treecalc | P2 | #10 | Open |
| #10 Move `treecalc::Form` out of graphify | P3 | — | **Fixed** (2026-04-21; new `clawft-treecalc` crate) |
| #11 Composer treecalc | research | profile first | Deferred |
| #12 Cost-tracker EML pre-check | research | profile first | Deferred |

Findings #1 + #2 + #3 together unblock the daemon from the 53%-CPU
steady state and restore RPC responsiveness to the VSCode extension —
that was the triage that produced this list.
