# Fork Adoption — Build & Testing Plan (2026-07)

Source evaluations: 2026-07-06/07 review of four external repos forked into
`weave-logic-ai` — CubeSandbox (Tencent), shepherd (shepherd-agents), zvec
(Alibaba), plus kyutai-labs/pocket-tts (upstream, not forked). The three
forks are pristine upstream mirrors (no WeaveLogic divergence). Verdicts:
**CubeSandbox = pattern-only**, **shepherd = pattern-only**,
**zvec = rejected as core dep (RVF stays)**, **pocket-tts = watch (§4)**. Clones live in the session scratchpad; upstreams are
the reference of record.

This document turns those verdicts into four concrete work items, each with a
build plan and a testing plan. Items 1–2 feed the hermes-loop track; item 3 is
the vector service layer; item 4 is the voice stack.

---

## 1. Durable-loop checkpointing (cubecow pattern → extends the agenticow plan)

**What we're taking:** cubecow's shape — snapshot = independent frozen file,
rollback = cheap clone, fork = clone-of-clone, plus AutoPause/AutoResume
idle-suspend — implemented over our own substrate (`rvf_runtime::RvfStore`
COW + ExoChain witness), **not** FICLONE/reflink and **not** CubeSandbox
itself (Linux+KVM cluster; fatal platform mismatch).

**This extends, not replaces,** the existing
`.planning/ruv/integration/agenticow-integration-plan.md` (which already
specifies the `clawft-cow-memory` crate and the checkpoint/rollback verbs).
cubecow adds two things to that plan: event-level snapshot cadence and
idle-suspend/resume for the daemon loop.

### Build plan

- **Phase 1 — `clawft-cow-memory` crate (M, unchanged from agenticow plan).**
  New crate `crates/clawft-cow-memory` wrapping `RvfStore` with
  `BranchableMemory { checkpoint / rollback / branch / fork / promote / diff /
  lineage / query }` (chain-walk read-through, tombstones, edit-log).
- **Phase 2 — Hermes-loop wiring (M, unchanged).** Wrap
  `AgentLoop::handle_turn` (`crates/clawft-core/src/agent/loop_core.rs:565`)
  with checkpoint→(Ok: promote / Err: rollback), gated behind an
  `AgentsConfig` flag. Couple to `ChainManager`
  (`crates/clawft-kernel/src/chain.rs`): turn revert appends a compensating
  chain event; turn commit records lineage witness.
- **Phase 3 — cubecow additions (M, NEW).**
  - *Event-level snapshot cadence:* checkpoint at loop-significant events
    (turn boundary, tool-call boundary, subagent spawn), labelled with
    `chain_seq`, so any point in the durable loop is a rollback target —
    mirrors cubecow's v0.3 event-level snapshots.
  - *AutoPause/AutoResume:* idle detection in the daemon loop
    (`crates/clawft-cli/src/commands/agent_daemon.rs`,
    `crates/clawft-weave/src/daemon.rs`) freezes loop memory to a checkpoint
    (`working.freeze()` + witness append) and parks the loop; a new impulse
    rehydrates by deriving a fresh writable child from the frozen ancestor.
    Resume must be O(1) (derive, not copy).

### Testing plan

- Port agenticow's `bench/acceptance.js` as a Rust integration test:
  1,000-branch acceptance with recall@10 = 100% after branching (upstream
  reference numbers: ~472µs COW branch @1M vectors, rollback ~0.57ms p50 —
  treat as targets to confirm on our hardware, not assumptions).
- Crash-resume integration test: kill the daemon mid-turn after a checkpoint,
  restart, assert the loop resumes from the checkpoint with the partial turn
  rolled back and a compensating chain event witnessed.
- AutoPause round-trip test: idle → freeze → impulse → resume; assert no
  vector loss (query parity before/after) and witness chain continuity.
- All via `scripts/build.sh test`; add the acceptance test to
  `scripts/build.sh gate` once stable.

---

## 2. Retained-output review gate (shepherd pattern)

**What we're taking:** shepherd's `select / release / discard` proposal seam —
"run the output without applying anything, then keep or throw it away; the
trace remembers either way" — as a review gate between Speculative and
Committed in the ECC forest. **Not** taking any shepherd code (Python, alpha;
its headline fork/replay + KV-reuse features are paper/spikes, not shipped).

**Load-bearing existing facts:** `SessionView` already has the full lifecycle
(`crates/clawft-kernel/src/context_graft.rs` — `set_speculative` :387,
`transition` :395, `commit` :412); M2's finding is that *nothing calls
commit* until the daemon hosts a `TalkModeLoop`
(`.planning/hermes-loop/m2-daemon-ecc-loop-design.md` §2.5). The gate slots
into exactly that actor.

### Build plan

- **Phase 1 — Design amendment (S).** Extend the M2 design with the gated
  lifecycle: `Frontier → Speculative → (review) → Committed | Discarded`.
  Policy enum in config: `auto` (today's behavior — commit on tick),
  `review` (hold at Speculative until accept/discard), with a discard-on-
  timeout guard. Codify the shepherd/WeftOS shared invariant explicitly:
  **a changeset is a view over the trace, never a second store** (matches
  the existing "SessionView is a view" rule and M3's one-store goal).
- **Phase 2 — TurnProposal view + RPC (M).** A `TurnProposal` read model over
  a `chain_seq` range (what this turn would commit: nodes, crossrefs, side
  effects), surfaced via `crates/clawft-service-agent/src/protocol.rs` as
  `agent.proposal.{list,accept,discard}`. Accept drives
  `SessionView::commit` + causal-node state advance
  (`crates/clawft-service-agent/src/session_forest.rs`); discard transitions
  to a tombstoned/pruned state using the existing prune machinery.
- **Phase 3 — Cancellation alignment (S).** Align with this branch's
  cancel→prune→witness Stop path: a cancelled/discarded turn **retains its
  witnessed partial trace** (shepherd's "artifacts persist across
  cancellation"). Discard ≠ delete; it is a state, recorded on chain.
- **Later (design note only, no build now):** shepherd's
  signature-as-permission-surface (grant → writable roots → syscall-level
  deny via Seatbelt/Landlock) is the pattern to reach for when the loop
  spawns filesystem-touching tool subprocesses; the seam is the custody
  witness surface (`crates/clawft-kernel/src/http_facade.rs`,
  `kernel_gate.rs`).

### Testing plan

- Unit: lifecycle transition table on `SessionView` — legal
  (`Speculative→Committed`, `Speculative→Discarded`) and illegal
  (`Discarded→Committed`) transitions; timeout policy firing.
- Integration (follow the `session_tier_wave2_tests.rs` pattern):
  `agent.chat` → turn indexed Frontier → loop advances to Speculative →
  `agent.proposal.accept` → assert Committed node + witness; and the discard
  path → assert pruned state + retained partial trace + witness.
- Mid-turn cancel test: Stop during generation → prune → assert the
  witnessed partial trace survives and the proposal is marked discarded.
- Deterministic-provider replay test (shepherd steal #3): drive the loop with
  a recorded model provider and assert byte-identical forest outcomes across
  two runs.

---

## 3. Vector layer hardening (zvec outcome: keep RVF, close our own gaps)

**Verdict recap:** zvec (C++/C-ABI, no WASM/no_std path, 18 vendored
submodules) is rejected as a core dependency. RVF/ruvector stays the
substrate. The kernel **already has** the hot/cold story zvec would have
provided: `VectorBackend` trait with HNSW / DiskANN / Hybrid backends
(`crates/clawft-kernel/src/vector_backend.rs`, `vector_diskann.rs`,
`vector_hybrid.rs`, `vector_quantization.rs`), selected at boot from
`kernel_config.vector.backend` (`boot.rs:1460`). The gaps are ours:

1. The `diskann` cargo feature is **not in kernel defaults**
   (`crates/clawft-kernel/Cargo.toml:19,62`); without it `DiskAnnBackend`
   **silently degrades to a brute-force linear-scan stub** even when config
   says diskann.
2. No benchmark has ever validated RVF ANN recall/latency at scale — the
   whole brain plan rests on paper claims.

### Build plan

- **Phase 1 — Fail loud on config/feature mismatch (S).** In boot backend
  construction (`boot.rs` ~:1460): when `VectorBackendKind::DiskAnn` or
  `Hybrid` is configured but the crate was built without `feature = "diskann"`,
  emit a prominent `BootEvent` warning naming the stub, and add a
  `vector.strict` config knob that turns it into a boot error. Document the
  stub behavior in `vector_diskann.rs` module docs (partially there) and
  `docs/`.
- **Phase 2 — build.sh named path (S).** `scripts/build.sh` already passes
  `--features` through generically and has a feature-regression guard
  (WEFT-643, `check_feature_regression`). Add: `diskann` to the documented
  feature list in `--help`; a compile-check of the `diskann` feature matrix
  to `scripts/build.sh gate` so the feature can't rot; extend the install
  regression guard so an installed binary built with diskann isn't silently
  replaced by a stub build.
- **Phase 3 — Benchmark spike (M) — the decision gate.** Bench harness (new
  bench alongside `crates/clawft-core/benches/pipeline_bench.rs` or a kernel
  bench) comparing `HnswBackend` vs `DiskAnnBackend` (PQ on/off) vs
  `HybridBackend` at 100K / 500K / 1M vectors, 384-dim: recall@10 vs
  brute-force ground truth, insert throughput, query p50/p95, RSS, on-disk
  size. Fixed-seed synthetic + one real corpus (docs/brain embeddings).
  Publish results to `docs/brain/` and record the verdict here.
- **Phase 3 VERDICT (recorded 2026-07-14, WEFT-366 closed):** HNSW stays
  the live/primary backend; **DiskANN = come back to it** (deferred, not
  disqualified) — query profile wins at 10K (0.994 recall, p50 363µs vs
  HNSW 0.943/398µs) but `ruvector-diskann`'s serial Vamana build (128s @
  10K/384d, one core; hours projected @1M) is wrong for the streaming ECC
  workload. Revisit when upstream ships parallel/incremental build, when
  WEFT-660 (search id=0) + WEFT-661 (hybrid cross-metric merge, recall
  0.113) land, or when an off-path cold-snapshot tier materializes. Full
  results: `docs/brain/vector-backend-bench-2026-07.md`. 500K/1M ladder
  deliberately not run; harness is ready when revisit triggers.
- **Phase 4 — Watch items (no build).** `ruvector-rabitq` (listed in
  `.planning/ruv/crate-index.md`): if real, it closes zvec's quantization
  edge — evaluate after Phase 3 baselines exist. Hybrid/FTS (BM25+dense
  fusion) remains the one capability with no ruvector answer; if manual-RAG
  needs it, evaluate `tantivy` + RRF before ever revisiting zvec.

### Testing plan

- Unit: stub-mismatch warning fires (build without `diskann`, configure
  DiskAnn, assert the BootEvent); `vector.strict` aborts boot.
- Feature-matrix compile in gate: `check` with and without `--features
  diskann` (and with `ecc` off) — catches cfg rot.
- Bench acceptance thresholds (set after first run; provisional): Hybrid
  recall@10 ≥ 0.95 vs ground truth at 1M; query p95 within 2× HNSW-only at
  100K. Regressions fail the bench job, not the build.

---

## 4. pocket-tts (kyutai-labs) — voice stack

**Verdict: watch / pattern-only. Do not adopt now.**

**What it is:** 100M-param streaming TTS on the Mimi codec family
(FlowLM latents → Mimi decode, 24kHz), producer/decoder thread split for
chunked streaming. Vendor claims: ~200ms time-to-first-audio, ~6x real-time
on 2 CPU cores (M4 Air), no GPU needed. Voice cloning from reference wav.
Six languages. **No paralinguistic tag performance** (`<laugh>` etc.).
Code MIT; weights CC-BY-4.0 but **gated** on HF (terms + contact info).
Active, versioned (2.1.0), tested, broad downstream adoption.

**Why not now (two blockers):**

1. **The official implementation is pure Python** (PyTorch + FastAPI). Both
   existing engines open with an explicit "No Python" invariant
   (`crates/clawft-voice-tts/src/kokoro.rs:1-18`, `orpheus.rs:1-20`) —
   Kokoro is native `ort`/ONNX, Orpheus is Rust-over-Ollama (Go) + local
   SNAC ONNX. Running `pocket-tts serve` would reintroduce exactly the
   runtime the crates ruled out. Community Rust/Candle/ONNX ports exist
   but are unofficial single-maintainer forks — adopting one means owning
   someone else's fork.
2. **It can't replace Orpheus's expressive role** (no tag performance), so
   the realistic ceiling is a Kokoro (fast-ack tier) swap — a narrower win
   than eliminating the Ollama hop.

**Why keep watching:** if Kyutai ships an official ONNX or Candle export
(community ports prove it's exportable), pocket-tts is a strong Kokoro
replacement: sub-200ms TTFA claimed on CPU, voice cloning vs Kokoro's fixed
styles, and — the real prize — **in-process synthesis makes barge-in cancel
instantaneous** (drop the generator) vs HTTP stream teardown, which
directly strengthens the cancel→prune→witness path.

### Build plan (conditional — only when an official Rust-native build exists)

- New sibling module `crates/clawft-voice-tts/src/pocket_tts.rs` mirroring
  `kokoro.rs`: auto-discovery (`$WEFTOS_*_DIR` env → `.weftos/models/`),
  graceful degrade (`runtime_available: bool`, clean `VoiceError` when
  weights/feature absent — `kokoro.rs:52-56`), implementing `TtsEngine`
  (`crates/clawft-channels/src/voice/tts.rs:52-65`) at `TtsTier::Fast`.
- Bind as the fast engine in `crates/clawft-voice-talk/src/tts.rs:26-30`
  (`native_dual_layer()`), behind the existing `onnx` feature or a new
  `candle` feature in `clawft-voice-tts/Cargo.toml`.
- Weight provisioning must handle the **gated** CC-BY-4.0 HF download
  (terms acceptance) — cannot silently vendor; document in install flow.

### Testing plan (conditional)

- Smoke: construct with no weights → clean `VoiceError`, no panic (matches
  Kokoro's model-free test pattern). Real-decode test `#[ignore]`-gated
  behind a local weights bundle.
- Latency: wall-clock to first `TtsChunk` from `synthesize_stream()` on the
  actual dev machine vs Kokoro's TTFA baseline; adopt only if it wins.
- Barge-in: cancel mid-stream via the `CancellationToken` and assert
  cutoff latency beats the current fast-tier engine.

---

## Sequencing & tracker

Suggested order: **3.1–3.2 first** (small, removes a live silent-degradation
trap), then **3.3 benchmark** (decision gate for everything vector), **2**
(review gate — unblocks supervised hermes loop, pairs with M2 work already
in flight), **1** (checkpointing — depends on `clawft-cow-memory`, already
planned via agenticow), **4** per its verdict.

**Filed in Plane 2026-07-07** — all in the **0.8.x** cycle (decision: start
after the voice waves finish):

| Plan section | Plane item |
|---|---|
| §1 Phases 1–2 (clawft-cow-memory + loop wiring) | WEFT-616 (pre-existing; cross-link comment added) |
| §1 Phase 3 (cubecow additions) | WEFT-652 (blocked-by WEFT-616) |
| §2 Phase 1 (review-gate design amendment) | WEFT-653 |
| §2 Phase 2 (TurnProposal + RPCs) | WEFT-654 (blocked-by WEFT-653) |
| §2 Phase 3 (cancellation alignment) | WEFT-655 (blocked-by WEFT-654) |
| §3 Phases 1–2 (fail-loud + build.sh + gate matrix) | WEFT-656 (medium priority — live trap) |
| §3 Phase 3 (benchmark spike) | WEFT-366 (pre-existing; scope-expansion comment added; blocked-by WEFT-656) |
| §3 Phase 4 (rabitq / FTS watch) | tracked in WEFT-366 comment, no separate item |
| §4 (pocket-tts) | WEFT-657 (watch; blocked-by upstream official ONNX/Candle export) |

Related pre-existing vector items not duplicated: WEFT-365, WEFT-351,
WEFT-126 (ship/validate the real DiskANN backend — referenced from
WEFT-656's description).
