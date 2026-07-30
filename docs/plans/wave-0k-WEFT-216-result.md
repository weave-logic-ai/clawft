# WEFT-216 result — WakeWordDetector: honest stub + engine gate

**Ticket:** WEFT-216 — ws10: WakeWordDetector — wire rustpotter or document an alternative  
**Branch:** `wave0k/weft-216-wake-word`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-30  
**Agent:** coder-216 (wave-0k)  
**Choice:** **Honest docs + feature gate** (rustpotter integration blocked)

## Problem

`WakeWordDetector::process_frame` always returned `false` with no engine
dependency, no model, and no CPU budget. Ticket AC asked for rustpotter (or
an alternative) under `voice-wake`, a shipped “hey weft” model + hash, real
detection, &lt;2% daemon throttle, and recorded-utterance tests.

## Decision

**Do not pretend rustpotter is integrated.** Prefer a documented alternative
and a reserved fail-closed Cargo feature (same spirit as WEFT-683 `mesh-rvf`
and WEFT-420 platform gates).

### Why rustpotter is blocked

| Check | Result |
|-------|--------|
| crates.io `rustpotter` | **3.0.2** (Apache-2.0) |
| Direct dep chain | `rustpotter` → `candle-core` **0.2.2** (+ candle-nn, hound, …) |
| Workspace compile probe | **Fails** — `candle-core` 0.2.2 vs modern `rand` 0.8/0.9 dual and `half`/`bf16` `SampleUniform` / `Distribution` bounds |
| Model artifact | **Missing** — no `models/voice/wake/hey-weft.rpw` (or ONNX) in tree; SC-7 integrity not done |
| Capture loop | **Missing** — `WakeDaemon::run` still waits on cancel only |

Pulling a broken optional dep into the workspace lockfile would poison
`cargo check --features voice-wake-rustpotter` without delivering detection.

### Concrete alternative (chosen direction)

| Option | Verdict |
|--------|---------|
| **OpenWakeWord-style ONNX KWS via `clawft-voice-onnx` (`ort`)** | **Chosen follow-up path** — reuses the existing ONNX stack; no candle-0.2; portable `.onnx` wake models; aligns with voice crate migration (WEFT-671 F1) |
| Wait for rustpotter + candle that builds | Acceptable **if** upstream releases a toolchain-compatible line; then `voice-wake-rustpotter` can grow a real `dep:rustpotter` |
| Picovoice Porcupine | Rejected for default path (license / proprietary runtime) unless product later opts in |
| sherpa-onnx keyword spotter | Possible secondary if sherpa lands for STT; not the primary plan |

**Not shipped in this ticket:** ONNX model, `ort` wiring in `WakeWordDetector`,
cpal capture, CPU throttle, recorded-utterance fixtures. Those are follow-ups
once the alternative engine ticket is opened.

## What shipped

### 1. Backend identity (honest API)

| Symbol | Role |
|--------|------|
| `WakeWordBackend::{Stub}` | Explicit backend enum (`non_exhaustive`) |
| `WakeWordDetector::backend` / `is_stub` | Introspection for CLI/tests |
| `WakeWordDetector::new` | **Always stub** — never silently “half-loads” an engine |
| `frames_since_detection` | Frame accounting reserved for future min-gap |

### 2. Feature gate `voice-wake-rustpotter`

| Item | Detail |
|------|--------|
| Cargo | `voice-wake-rustpotter = ["voice-wake"]` on `clawft-plugin` |
| `dep:rustpotter` | **Not added** (blocked) |
| API | `WakeWordDetector::try_with_rustpotter` → `PluginError::NotImplemented` |
| Constant | `RUSTPOTTER_BLOCKED_REASON` (stable message for tests/docs) |
| Umbrella | **`voice` does not enable** `voice-wake-rustpotter` (keeps default builds light and honest) |

### 3. Docs / CLI honesty

- Module rustdoc on `wake.rs`, `wake_daemon.rs`, `voice/mod.rs`, crate `lib.rs`
- CLI `weft voice wake` prints stub backend + no capture / no detection
- `docs/weftos/FEATURE_GATES.md` — plugin voice/wake table
- This result file

## Acceptance mapping

| AC | Status |
|----|--------|
| Add rustpotter **or alternative** under voice-wake feature | **Done as alternative + gate** — documented OpenWakeWord/ONNX; reserved `voice-wake-rustpotter` fail-closed; default `voice-wake` remains stub |
| Ship canonical “hey weft” model with verified hash | **Deferred** — blocked (no samples, SC-7); documented |
| Real `process_frame` returns true on detection | **Deferred** — no live engine; stub remains intentionally false |
| CPU budget auto-throttle (&lt;2% wake daemon) | **Deferred** — needs live capture + engine |
| Tests using a recorded wake utterance | **Deferred** — no model/fixture; unit tests cover stub + fail-closed path |
| `scripts/build.sh test` with `--features voice` / wake | **Targeted tests** — see below |

## Revisit triggers (when to implement a live engine)

1. **rustpotter builds** on this workspace (upstream candle bump or proven pin) **or**
2. **ONNX wake model** trained/verified (hash + SC-7 path) for “hey weft”, **and**
3. Mic capture available to `WakeDaemon` (cpal or shared voice capture), **and**
4. Product still wants always-on wake on the plugin transitional surface (else implement only after WEFT-671 wake migration crate).

Until then: **do not** claim hands-free wake works.

## Files changed

| Path | Change |
|------|--------|
| `crates/clawft-plugin/Cargo.toml` | `voice-wake-rustpotter` feature + comments |
| `crates/clawft-plugin/src/voice/wake.rs` | Backend enum, fail-closed API, honest docs, tests |
| `crates/clawft-plugin/src/voice/wake_daemon.rs` | Stub docs, `backend()` accessor, tests |
| `crates/clawft-plugin/src/voice/mod.rs` | Re-exports + disposition note |
| `crates/clawft-plugin/src/lib.rs` | Feature flag docs |
| `crates/clawft-cli/src/commands/voice.rs` | Honest CLI banner for `handle_wake` |
| `docs/weftos/FEATURE_GATES.md` | Plugin voice/wake section |
| `docs/plans/wave-0k-WEFT-216-result.md` | This file |

## How to test

```bash
# Default voice umbrella (stub wake)
scripts/build.sh test clawft-plugin -- --features voice
# or focused:
cargo test -p clawft-plugin --features voice --lib voice::wake
cargo test -p clawft-plugin --features voice --lib voice::wake_daemon

# Fail-closed rustpotter reserve
cargo test -p clawft-plugin --features voice-wake-rustpotter --lib voice::wake

# Docs / gate claims
grep -n 'voice-wake-rustpotter\|RUSTPOTTER_BLOCKED\|OpenWakeWord\|WEFT-216' \
  crates/clawft-plugin/Cargo.toml \
  crates/clawft-plugin/src/voice/wake.rs \
  docs/weftos/FEATURE_GATES.md \
  docs/plans/wave-0k-WEFT-216-result.md
```

## Follow-ups

1. Open ticket: **OpenWakeWord / ONNX KWS** on transitional wake (or post-migration `clawft-voice-*` crate) with model training + SC-7.
2. WEFT-240: unify `WakeConfig.sensitivity` vs `WakeWordConfig.threshold` once a live engine can validate the knob.
3. Wake migration off `clawft-plugin` (WEFT-671 F1) — prefer implementing live engine there if timing allows.
4. Plane: mark WEFT-216 Done with commit SHA + this result path after merge.

## Commit

- **Branch tip:** `git rev-parse wave0k/weft-216-wake-word` (implementation + this result)
- **Branch:** `wave0k/weft-216-wake-word`
- **Message:** `WEFT-216: WakeWordDetector stub honesty + voice-wake-rustpotter gate`
- **No push** (per wave instructions)

## Worktree

- Path: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4dd-9e94-7350-bce2-77e9b2d4966a`
- Branch: `wave0k/weft-216-wake-word`
- Base: `release/0.8-staging`
