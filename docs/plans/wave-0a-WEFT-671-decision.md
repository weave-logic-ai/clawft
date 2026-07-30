# WEFT-671 Decision: Disposition of `clawft-plugin/src/voice`

**Date**: 2026-07-30  
**Status**: Accepted  
**Ticket**: WEFT-671  
**Branch**: `wave0a/weft-671-voice-disposition`  
**Deciders**: wave-0a coder-671 (grounded in tree + ADR-053/061/068)  
**Form**: ADR-style decision note (ticket-local; not a new numbered ADR — architecture already fixed by ADR-053/061/068)

---

## Context

Two overlapping voice implementations exist:

| Surface | Path | Last product role |
|---------|------|-------------------|
| **Live product stack** | `clawft-channels/src/voice/*`, `clawft-voice-talk`, `clawft-voice-tts`, `clawft-voice-onnx`, `clawft-voice-aec`, substrate `clawft-service-whisper` | Backs `weft voice talk` / listen / watch; ADR-053 STT; ADR-061/068 duplex |
| **Plugin scaffold** | `crates/clawft-plugin/src/voice/` (22 files, ~4247 LOC), `#[cfg(feature = "voice")]` | 0.7 audit-era in-process sherpa-rs plan; largely stubs |

An earlier reconciliation pass asserted the plugin module was dead. That was **false**:

- Live import: `crates/clawft-cli/src/commands/voice.rs` → `clawft_plugin::voice::{WakeDaemon, WakeWordConfig}` in `handle_wake` only.
- `weft voice talk` uses `clawft_voice_talk`, not plugin TalkMode.
- `clawft-channels` depends on `clawft-plugin` for traits/errors/messages, **not** for `voice/*`.
- Umbrella test: `crates/clawft-plugin/tests/voice_umbrella.rs` (WEFT-212 feature wiring).

Audit origin: `.planning/reviews/0.7.0-release-gate/10-voice.md`.  
Blocked cluster: WEFT-214, 216, 217, 218, 221, 222, 227, 233, 234, 238, 239, 240.

Ticket options:

- **(a)** Retire module; migrate `WakeDaemon`/`WakeWordConfig` to a live crate.
- **(b)** Keep as wake-word-only home; delete the other ~20 files.
- **(c)** Keep whole with a stated reason.

---

## Decision

**Hybrid (b) now, path to (a) later — not (c).**

1. **Keep** `clawft-plugin/src/voice` **feature-gated** under `voice` / `voice-*`.
2. **Designate wake as the only supported transitional surface** inside that module (`wake.rs`, `wake_daemon.rs` → CLI `weft voice wake`).
3. **Mark all other plugin voice submodules as deprecated legacy scaffold** (docs + module comments). Do **not** invest new product DSP / Talk Mode / STT / TTS work there.
4. **Canonical product voice** remains:
   - Talk / edge: `clawft-voice-talk` + `clawft-channels::voice`
   - TTS: `clawft-voice-tts`
   - AEC/NS: `clawft-voice-aec`
   - STT: substrate `clawft-service-whisper` (ADR-053); channel adapters as thin clients
   - Duplex floor: ADR-068
5. **Do not mass-delete** the ~20 non-wake files in this ticket (breaks WEFT-212 umbrella compile coverage and needs a coordinated cleanup). Deletion/archive is a follow-up after wake migration.
6. **Do not large-port** wake into a new crate in this ticket (non-trivial; stub only).

### Why not pure (a) this sprint

Migrating `WakeDaemon` requires a new home crate (or channels submod), CLI feature wiring, service install scripts, and tests. The detector is still a stub. Decision + docs + deprecation labels unblock the 12-item triage without a risky port.

### Why not pure (c)

The product has already moved Talk Mode, STT, TTS, and AEC off this tree. Keeping the whole module as a first-class development target would continue misrouting audit work.

### Why not bulk-cancel without naming supersession

Handoff rule (`docs/handoff-tracker-ci-memory.md`): do not bulk-cancel the 12 as superseded on a false “dead module” premise. Each item gets an explicit outcome below.

---

## Code grounding (evidence)

| Claim | Evidence |
|-------|----------|
| Plugin voice is feature-gated | `crates/clawft-plugin/src/lib.rs` `#[cfg(feature = "voice")] pub mod voice` |
| Sole live external caller is wake | `grep clawft_plugin::voice` → only CLI `handle_wake` |
| Talk uses live stack | CLI `handle_talk` → `clawft_voice_talk::{TalkConfig, live::…}` |
| Live AEC exists | `crates/clawft-voice-aec` (AEC3/NS); not plugin `echo.rs` |
| Live barge-in exists | `clawft-channels/src/voice/talkmode.rs` `speak_answer_with_barge_in`, config `barge_in_*` |
| Canonical STT | ADR-053 Accepted: substrate whisper, not plugin sherpa-rs stubs |
| Scaffold still stubs | `wake_daemon` waits on cancel; `echo`/`noise` passthrough; audit 10-voice.md |

Minimum code shipped with this decision:

- Module disposition docs: `crates/clawft-plugin/src/voice/mod.rs`
- Wake “supported transitional” docs: `wake.rs`, `wake_daemon.rs`
- Deprecated scaffold docs: `echo.rs`, `noise.rs`, `talk_mode.rs`
- Crate feature-flag disposition: `crates/clawft-plugin/src/lib.rs`
- CLI pointer: `crates/clawft-cli/src/commands/voice.rs` (`handle_wake`)

---

## Impact: re-triage of the 12 blocked audit items

Outcome vocabulary:

- **Cancel-superseded** — intent delivered (or intentionally replaced) on the live stack; plugin target is obsolete. Name the superseding surface.
- **Open-live** — still real work, but **must** target the live stack / tools, not plugin scaffold.
- **Open-wake** — still real work against the transitional wake home in plugin (until migration).
- **Open-cleanup** — delete/archive/orphan cleanup once policy is clear.

| WEFT | Title (short) | Outcome | Comment for Plane / tracker |
|------|---------------|---------|-----------------------------|
| **214** | `voice_listen` / `voice_speak` tools → real STT/TTS | **Open-live** | Tools live in `clawft-tools` as stubs. Wire to **live** stack (`clawft-channels` STT client / `clawft-voice-tts` / talk session), **not** `clawft-plugin/src/voice/{stt,tts,cloud_*}`. Plugin modules are deprecated scaffold (WEFT-671). |
| **216** | `WakeWordDetector` → rustpotter or alt | **Open-wake** | Sole supported plugin surface. Implement or document alternative **here** (or in the post-migration crate). Unblocked by WEFT-671. |
| **217** | EchoCanceller / NoiseSuppressor real DSP | **Cancel-superseded** | Superseded by **`clawft-voice-aec`** (WebRTC AEC3 + NS). Plugin `echo.rs` / `noise.rs` are intentional deprecated passthroughs; do not “fix” them. |
| **218** | WS `voice:status` broadcaster | **Open-live** | Re-home to daemon/services + UI against **live** talk session status, not plugin `events.rs` / `VoiceWsEvent` scaffold. |
| **221** | Talk Mode interruption / `TtsAbortHandle` | **Cancel-superseded** | Superseded by **`clawft-channels::voice::talkmode`** barge-in (`speak_answer_with_barge_in`, `barge_in_enabled` / grace / frames). Further barge-in product work tracks **WEFT-615** (ADR-068 ERL floor), not plugin `TtsAbortHandle`. |
| **222** | VoicePersonality → TTS dispatch | **Cancel-superseded** | Plugin `VoicePersonality` has no live consumer. Live TTS is **`clawft-voice-tts`** (profiles/voices there). If product wants per-agent voice IDs on the live path, open a **new** ticket against that stack — do not implement against plugin config. |
| **227** | Speaker diarization via sherpa-rs | **Cancel-superseded** (intent) | ADR-053 rejected in-process sherpa-rs as canonical. Live speaker work exists as **`clawft-channels::voice::speaker`** (`SpeakerRegistry` / embedder). Multi-party diarization, if needed, is a **new** live-stack ticket, not plugin sherpa-rs. |
| **233** | `audio_transcribe` / `audio_synthesize` codecs | **Open-live** | Tools are stubs in tools crate. Implement codecs on the **live** audio path; do not revive plugin file I/O stubs as the product surface. |
| **234** | Cleanup orphan voice surfaces | **Open-cleanup** | Still valid tech-debt: plugin orphans + UI `voice-chat.ts` etc. Scope after wake policy: prefer delete/archive of plugin non-wake files (follow-up WEFT below) rather than “completing” orphan APIs. |
| **238** | `VoiceConfig.tts.provider="browser"` default | **Cancel-superseded** | Default applies to unused plugin config. Live talk does not use plugin `tts.rs` browser dispatch. No product bug on the live path. |
| **239** | CloudFallbackConfig string→provider router | **Cancel-superseded** | Plugin cloud fallback chain has no product caller. Canonical STT is substrate whisper (ADR-053); live TTS is `clawft-voice-tts`. |
| **240** | `WakeConfig.sensitivity` vs `WakeWordConfig.threshold` | **Open-wake** | Unify knobs as part of wake work (with WEFT-216) on the transitional wake surface / migration crate. |

**Summary counts:** Cancel-superseded **6** (217, 221, 222, 227, 238, 239) · Open-live **3** (214, 218, 233) · Open-wake **2** (216, 240) · Open-cleanup **1** (234).

Zero silent skips: every ID has a named outcome above.

---

## Follow-up tickets (recommended)

| Proposed | Purpose |
|----------|---------|
| **WEFT-671-F1** (or new Plane item) | Migrate `WakeDaemon` / `WakeWordConfig` / `WakeWordDetector` from `clawft-plugin` → e.g. `clawft-voice-wake` or `clawft-channels::voice::wake`; retarget CLI + install-service scripts; then drop plugin wake. |
| **WEFT-671-F2** | After F1: delete or `crates/archive/` the remaining non-wake `clawft-plugin/src/voice/*` scaffold; shrink/remove WEFT-212 umbrella assertions; drop unused `voice-stt`/`voice-tts` plugin features if empty. |
| **Plane hygiene** | Apply the 12 triage comments on each work item; set Cancelled (with supersession note) or keep Todo with **live/wake/cleanup** retarget text. WEFT-671 itself → Done with this commit. |

WEFT-613 (Chatterbox) was listed as blocked-by WEFT-671 in inventory; it targets live TTS parity and is **unblocked** by this disposition (work against `clawft-voice-tts`, not plugin).

---

## Consequences

### Positive

- Stops misrouting ~12 tickets at dead stubs.
- Keeps the one live CLI wake path compiling without a rushed port.
- Aligns docs with ADR-053/061/068 and the actual `weft voice talk` binary path.

### Negative / residual risk

- Dual tree remains until F1/F2 (~4k LOC still compiled under `--features voice`).
- `#[deprecated]` rustc attributes were **not** applied (would warn across umbrella tests / clippy-deny); disposition is documentation-enforced until delete.

### Out of scope

- Implementing rustpotter, real AEC in plugin, tool wiring, WS broadcaster.
- Plane API state transitions (comments/status) — record is this document; operators should mirror into Plane.

---

## Acceptance criteria map (WEFT-671)

| AC | Status |
|----|--------|
| Decide disposition (a)/(b)/(c) explicitly | **Done** — hybrid (b)→(a); not (c) |
| Record as ADR-style decision | **Done** — this file |
| Re-triage 12 items with no silent skips | **Done** — table above |
| Minimum code makes decision real | **Done** — module/crate/CLI docs |
| Result note + commit on branch | See `wave-0a-WEFT-671-result.md` |
