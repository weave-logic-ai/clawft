# WEFT-282 result — vscode panel capture sidecar (mic/camera)

**Ticket:** WEFT-282  
**Branch:** `feat/weft-282-capture-sidecar`  
**Base:** `release/0.8-staging`  
**Date:** 2026-07-31  
**Agent:** developer (Grok Build / Ruflo)

## Problem

VS Code webviews cannot expose `allow="microphone"` /
`getUserMedia` ([microsoft/vscode#303293](https://github.com/microsoft/vscode/issues/303293)).
The WeftOS panel had no voice input path; capture was deferred to M2.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Design memo: sidecar process architecture (host-side capture + IPC) | **Yes** — `docs/architecture/vscode-panel-capture-sidecar.md` |
| Implement minimum viable mic capture path | **Yes** — host bridge + mock/process/unavailable backends |
| Smoke test against whisper service | **Yes** — `poll.whisper` PcmChunk shape + SMOKE §11 |
| Graceful unavailable | **Yes** — default backend; camera reserved unavailable |
| Tests / docs | **Yes** — `captureSidecar.test.ts`, README, SMOKE §11 |

## What shipped

| Path | Change |
|------|--------|
| `extensions/vscode-weft-panel/src/captureSidecar.ts` | **new** host bridge, backends, whisper helper |
| `extensions/vscode-weft-panel/src/captureSidecar.test.ts` | **new** unit tests (no VSCode host / no mic) |
| `extensions/vscode-weft-panel/src/extension.ts` | `capture-request` handler, consent wiring, local `sensor.mic.status`, dispose |
| `extensions/vscode-weft-panel/src/wsp.ts` | consent.request notes → WEFT-282 |
| `extensions/vscode-weft-panel/package.json` | `test:capture`, unit suite includes it |
| `extensions/vscode-weft-panel/README.md` | Capture sidecar section |
| `extensions/vscode-weft-panel/SMOKE.md` | §11 capture + whisper smoke |
| `docs/architecture/vscode-panel-capture-sidecar.md` | Design memo |
| `docs/plans/wave-0-WEFT-282-result.md` | This report |

## Design notes

- **Host bridge, not webview mic.** Extension host owns capture; webview
  uses `capture-request` / `capture-response` over postMessage.
- **Three backends:** `unavailable` (default, graceful), `mock`
  (synthetic PCM for CI), `process` (`WEFT_CAPTURE_SIDECAR` stdout
  s16le mono).
- **Consent gate:** `start` / `test_level` require `scope://mic`
  (WSP `consent.request` or `grant_consent`).
- **Camera:** status + consent reserved; start → graceful unavailable.
- **Whisper:** `poll` attaches a `whisper` object matching
  `clawft-service-whisper::PcmChunk` for smoke / publish helpers.
- **sensor.mic.status:** answered by the host (M1.5.2 reservation),
  not UDS, so the panel works without a daemon mic verb.

## Verification

```bash
cd extensions/vscode-weft-panel
npm install   # first time
npm run compile
npm run test:capture
# expected: all tests pass
```

Optional full unit suite:

```bash
npm run test:unit
```

## Out of scope (follow-ups)

- Native Node cpal addon (process sidecar covers MVP).
- Camera frames / preview.
- Automatic host→substrate PCM publish (WEFT-496: webview denylist).
- Wasm push-to-talk chrome bound to the bridge.
