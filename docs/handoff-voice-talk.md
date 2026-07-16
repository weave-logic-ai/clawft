# Handoff — Voice stack + COW/review-gate: current state, runbook, open items

**Date:** 2026-07-16 (supersedes the 2026-07-03 version — that predates voice
Wave 2 and the entire fork-adoption/COW track)
**Repo:** `/Users/mathewbeane/weftos` (branch `feat/hermes-loop-base`)
**Deployed:** daemon + binaries @ `4b037496` (version stamps carry
`(sha ts) [features]` — WEFT-656). **Pending:** WEFT-615 round-2 mic
confirmation (windowed playback pacing / self-echo fix / clarity retune).

**Standing directives:** `~/llm` voicelab is still the tuned reference — diff
against it first when the native voice stack misbehaves. Subagent model
policy: sonnet default, fable escalation-only (memory: agent-model-routing).

---

## Architecture as of today

| Stage | Impl | Notes |
|---|---|---|
| Capture + VAD | cpal → two-stage gate (energy floor w/ startup calibration + watchdog, spectral voiceness AND-gate) | plan appendices A–C; Silero seam = WEFT-644 |
| Endpointing | smart-turn v3.1 + SemanticEndpointer (min-voiced 400ms, short-audio discount 500ms) | |
| STT | parakeet-tdt int8 | per-token conf feeds the clarity gate |
| **Brain** | **daemon voice loop** via `weft voice talk --brain auto\|daemon\|local` (auto = probe; daemon = full agent loop w/ tools+grafts, voice-shaped replies) | local bare-prompt Hermes only as fallback |
| Interrupt brain | Wave 2: router (busy×intent×paralinguistics) → Stop/Refine/Backchannel/Queue executors; attempt nodes (register-early/commit-late); Contradicts edges; own busy axis (`VoiceShared.in_flight`) | all live-verified |
| Input gates | non-lexical (filler/backchannel/laughter → never engage brain) + unclear-STT (canonical `utterance_clarity` in talkmode.rs, wire-mirrored in voice_loop.rs — **KEEP IN LOCK-STEP**; low-SNR arm = conf<0.50 after live retune) | spurious-gate case pinned as test |
| Ack/filler | rotating pools (pre-warmed), ack speaks BEFORE the LLM call, 1.5s-gated contextual filler | WEFT-658 |
| **Playback interrupts** | WEFT-615 interim: answer pre-rendered ONCE (`render_answer`), played sentence-wise with ~350ms echo-free listening windows between (`sentence_window_ms`, 0=off); bare-stop swallowed (`WINDOW_STOP_PHRASES` — mirror of interrupt_router's lexicon, **KEEP IN SYNC**); content utterance seeds the next turn. Continuous in-playback barge-in = `barge_in_enabled` (headphones only). Full acoustic ERL floor = ADR-068 D1 / WEFT-628 end-state | round-2 deploy awaiting mic confirm |
| TTS | Kokoro fast / Orpheus slow via Ollama, per-call tier selection intact | |
| Chain/forest | every turn anchored; attempt lifecycle on `conversation.graph` (`attempt`/`goal` flat keys); `weft voice watch` renders ◇/⊘/◆/↯ + TurnGated | |

## The COW / review-gate stack (fork-adoption track — ALL CLOSED)

- `clawft-cow-memory`: BranchableMemory over rvf-runtime — checkpoint/rollback/
  promote/branch/fork, chain-walk reads (mandatory: RVF query ignores its own
  COW), crate-owned tombstones (RVF delete bitmap is permanent), pause/resume,
  durable manifest (full-topology reopen; restart id-collision fixed).
- Turn bracket (`[agents] cow_memory { enabled, path, ingest_turns, cadence }`,
  default OFF): checkpoint → turn → exchange-ingest → promote / rollback, all
  chain-witnessed via TurnLedger→DaemonTurnLedger (record_lineage verifiable —
  parent_hash is REAL, the zeroed-caveat was stale docs). `cadence="tool"` adds
  witnessed checkpoints at every tool boundary. Idle AutoPause / on-demand
  AutoResume.
- Review gate (`[kernel.agent] proposal { mode = "auto"|"review", timeout_secs }`):
  review holds each turn as a pending proposal; `agent.proposal.{list,accept,
  discard}` — **accept = promote (lineage-witnessed), discard = rollback +
  witnessed revert**; pending proposal fails new dispatches (typed
  ProposalPending); timeout DISCARDS (fail-closed). One global lineage ⇒ one
  hold parks the loop (the supervised-review contract).
- Cancellation alignment (WEFT-655): cancel-drop / turn-error / discard = ONE
  closure (BracketGuard fixes the select!-drop leak — see gotchas).

## Runbook

```bash
# LLM (36B needs the big context — 8192 default causes context-500s):
~/llm/bin/serve-llamacpp ~/.cache/huggingface/hub/models--NousResearch--Hermes-4.3-36B-GGUF/snapshots/*/hermes-4_3_36b-Q8_0.gguf \
  --port 8090 --alias hermes-4.3-36b --ctx 32768

# Build + deploy (feature-stamped; guard refuses feature-dropping replacements):
scripts/build.sh install --features voice-onnx && weaver kernel stop && weaver kernel start

# Live:
weft voice talk            # --brain auto picks the daemon brain
weft voice watch weft-talk # graph surface alongside

# Config (workspace .clawft/config.json): kernel.agent.{voice_loop=true,
# talk_loop, anchor_chain, anchor_causal, classification.mode="keyword",
# proposal.mode} + agents.cow_memory.{enabled,cadence}
```

## Open items (Plane)

| Item | State |
|---|---|
| WEFT-615 | interim windows deployed @ 4b037496 — **awaiting mic re-test**; full ERL floor stays under WEFT-628 (ADR-068 D1) |
| WEFT-628 | ADR-068 Phase 1 umbrella (edge/ERL/DuplexChannel) — not started |
| WEFT-644 | Silero voiceness behind the trait seam |
| WEFT-638 | retire TalkForest (voice → daemon-loop client) |
| WEFT-651 | Hermes runaway identical tool calls (also SUCCESSFUL ones; also writes land in daemon CWD not workspace) |
| WEFT-660/661 | vector: real-DiskANN search id=0; Hybrid cross-metric merge (recall 0.11) — DiskANN itself deferred "come back to it" (docs/brain/vector-backend-bench-2026-07.md) |
| WEFT-662 | upstream rvf-runtime 0.2 report list: macOS __errno_location; open() resets metric AND witness hash; permanent delete bitmap; no vector-by-id read |
| WEFT-663 | pre-existing wasm break: local_file_sink Send futures |
| unfiled | default CI skips clawft-cli `voice` feature (watch module untested); review-mode voice queue drains next-turn not on-accept; system messages DROPPED by inbound_from_params (voice policy rides skill_instructions) |

## Gotchas earned this session (the expensive ones)

1. **select!-drop class**: any bracket around cancellable work needs a Drop
   guard — `tokio::select!` DROPS the losing future; Ok/Err arms are not
   enough (WEFT-655 leak).
2. **Fat-LTO duplicate `no_mangle`**: two crates defining the same extern "C"
   symbol link fine in dev and DIE in release LTO ("failed to load bitcode",
   real error one line above). One shim home: clawft-cow-memory.
3. **Test fixtures must use `SessionTier::weak_view_resolver`** — a detached
   SingleViewResolver view silently no-ops every commit/prune (bit twice).
4. **`talk_loop::current_turn` is NOT a busy axis** — every anchored turn
   overwrites it and its commit clears it; the voice loop keeps its own.
5. **Bot speech → drain before reopening capture** (`drain_sink_consuming`)
   or the tail becomes a self-echo user turn the brain OBEYS.
6. **Per-sentence TTS calls serialize synthesis** — render once, window the
   playback only.
7. **Two deliberate cross-crate mirrors to keep in sync**: STOP lexicon
   (interrupt_router ↔ talkmode WINDOW_STOP_PHRASES) and the clarity rule
   (talkmode `utterance_clarity` ↔ voice_loop wire mirror).
8. **Agent ops**: ~6 sonnet subagents died mid-task on transient API errors
   this stretch — always inspect partial tree state and salvage (work was
   consistently sound up to the cut); agents must use TARGETED fmt only
   (whole-package cargo fmt collided with concurrent edits).

## Memory anchors

`weftos-current-state` (chronological session log, newest first) ·
`agent-model-routing` · `weftos-voice-vad-architecture` ·
`weftos-operational-gotchas` · `llm-lab-controller`.
