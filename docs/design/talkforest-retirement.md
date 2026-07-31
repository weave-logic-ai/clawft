# TalkForest retirement plan (WEFT-638)

**Status**: Plan + legacy flag (cutover not complete — voice must not break)  
**Depends-On**: ADR-068 Phase 1 (thin edge / daemon-hosted floor)  
**Relates-To**: ADR-061, ADR-062, WEFT-606, `clawft-voice-talk`, `clawft-kernel::thin_edge`

## Goal

Voice runs on **one** engine: the daemon-hosted shared forest (M2 multiplexed
loop + thin edge). The standalone in-memory [`TalkForest`]
(`crates/clawft-voice-talk/src/forest.rs`) becomes redundant once the thin edge
is proven.

## Current state

| Path | Role | Status |
|------|------|--------|
| **TalkForest + TalkSession** | In-process ECC forest + TalkModeLoop for `weft talk` / native assembly | **Default / production for local talk** |
| **Thin edge (`thin_edge`)** | Protocol + state machine; no private forest; streams to daemon | Scaffold / Phase 1 partial (ADR-068) |
| **Daemon TalkModeLoop** | Shared multiplexed cognition | Live for chat; voice edge cutover incomplete |

Cutover is **not** complete. Removing TalkForest now would break Talk-Mode.

## Legacy flag

| Mechanism | Default | Meaning |
|-----------|---------|---------|
| `TalkConfig::use_legacy_talk_forest` | `true` | Build and run the standalone TalkForest path |
| Env `WEFTOS_LEGACY_TALK_FOREST` | unset → true | `0` / `false` / `off` opts into non-legacy (thin-edge-only) assembly when available |

When the flag is **false** and thin-edge wiring is incomplete, session
construction logs a warning and **falls back to TalkForest** so voice does not
hard-fail. Hard removal of TalkForest is gated on Phase 1 proof + a later
ticket.

## Retirement phases

1. **Flag + plan (this ticket)** — document path; default legacy on.
2. **Dual-run** — thin edge + TalkForest shadow comparison (metrics / parity).
3. **Flip default** — `use_legacy_talk_forest = false` when ADR-068 Phase 1 AC
   green on desktop.
4. **Hard gate** — compile-time or feature-flag remove TalkForest module;
   keep only daemon forest client.
5. **Delete** — remove `TalkForest` types after one stable release on the new path.

## Non-goals (now)

- No functional regression in Talk-Mode
- No removal of midstream / ECC observer paths that still depend on TalkForest
- No force-cutover via silent default flip

## Acceptance (WEFT-638)

- [ ] Voice edge fully on daemon-hosted shared forest — **not yet** (blocked by ADR-068 Phase 1)
- [x] TalkForest standalone path hard-gated behind a legacy flag (default on)
- [x] No functional regression in Talk-Mode (default path unchanged)
- [x] Retirement plan documented (this file)
