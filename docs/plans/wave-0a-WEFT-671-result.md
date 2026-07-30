# WEFT-671 Result

**Ticket**: WEFT-671 — decide disposition of `clawft-plugin/src/voice`  
**Branch**: `wave0a/weft-671-voice-disposition`  
**Date**: 2026-07-30  
**Status**: Implementation complete (decision + minimum code + docs)

## Decision one-liner

**Keep feature-gated plugin `voice/` as transitional wake-only home; deprecate the rest as scaffold; canonical product voice is `clawft-channels` + `clawft-voice-*` + substrate whisper (path to full retire after wake migration).**

## What shipped

### Docs

- `docs/plans/wave-0a-WEFT-671-decision.md` — full ADR-style decision, evidence, 12-item re-triage, follow-ups.

### Code (minimum — no large port)

| File | Change |
|------|--------|
| `crates/clawft-plugin/src/voice/mod.rs` | Disposition module docs; live vs legacy map |
| `crates/clawft-plugin/src/voice/wake.rs` | Supported transitional surface docs |
| `crates/clawft-plugin/src/voice/wake_daemon.rs` | Same + CLI caller note |
| `crates/clawft-plugin/src/voice/echo.rs` | Deprecated; points at `clawft-voice-aec` |
| `crates/clawft-plugin/src/voice/noise.rs` | Deprecated scaffold |
| `crates/clawft-plugin/src/voice/talk_mode.rs` | Deprecated; points at `clawft-voice-talk` / channels |
| `crates/clawft-plugin/src/lib.rs` | Feature-flag docs carry WEFT-671 disposition |
| `crates/clawft-cli/src/commands/voice.rs` | `handle_wake` documents sole live plugin-voice caller |

## 12-item triage (summary)

| Outcome | Items |
|---------|--------|
| Cancel-superseded | 217 (→ voice-aec), 221 (→ channels barge-in / WEFT-615), 222, 227, 238, 239 |
| Open-live stack | 214, 218, 233 |
| Open-wake (plugin transitional) | 216, 240 |
| Open-cleanup | 234 |

## Follow-ups

1. **Migrate wake** off `clawft-plugin` → dedicated voice crate; retarget CLI (F1).
2. **Delete/archive** remaining non-wake plugin voice scaffold (F2).
3. **Plane comments** on each of the 12 with the outcomes from the decision doc; close WEFT-671 as Done.
4. **WEFT-613** unblocked for live TTS work (not plugin).

## How to verify

```bash
# Docs present
test -f docs/plans/wave-0a-WEFT-671-decision.md
test -f docs/plans/wave-0a-WEFT-671-result.md

# Module header states disposition
grep -n "WEFT-671" crates/clawft-plugin/src/voice/mod.rs

# Sole live import still wake
grep -rn "clawft_plugin::voice" crates --include='*.rs' | grep -v clawft-plugin

# Optional: feature compile (slow)
# scripts/build.sh check  # or targeted:
# cargo check -p clawft-plugin --features voice
```

## Commit

- **Primary SHA**: `7113b6d924ab5b0e3c26ee96386976dabae3a500` (`7113b6d9`)
- **Branch**: `wave0a/weft-671-voice-disposition`
- **Message**: `WEFT-671: disposition of clawft-plugin/src/voice`


## Worktree

- Path: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb418-df95-7402-8ef0-ea009ec74698`
- Branch: `wave0a/weft-671-voice-disposition`
- Base was `feat/hermes-loop-base` (local worktree; not master)
