# VS Code panel capture sidecar (WEFT-282)

**Ticket:** WEFT-282  
**Upstream:** [microsoft/vscode#303293](https://github.com/microsoft/vscode/issues/303293)  
**Code:** `extensions/vscode-weft-panel/src/captureSidecar.ts`

## Problem

VS Code / Cursor **webviews cannot expose** `allow="microphone"` or
`navigator.mediaDevices.getUserMedia`. The WeftOS dev panel therefore
cannot capture mic (or camera) audio/video *inside* the wasm/HTML
surface. Voice input was blocked; the gap was deferred to M2.

## Decision

Capture runs on the **extension host** (Node process that owns the
panel). The webview talks to a host bridge over the existing
`postMessage` channel. This is the “sidecar or host bridge” required
by the ticket: either a pure host module or an optional external
process that writes raw PCM to stdout.

```text
┌─────────────────────────────┐
│  Webview (egui-wasm / JS)   │
│  no getUserMedia            │
└─────────────┬───────────────┘
              │ capture-request / capture-response
              │ (optional capture-chunk)
              ▼
┌─────────────────────────────┐
│  Extension host             │
│  CaptureController          │
│    · consent (scope://mic)  │
│    · start / stop / poll    │
└─────────────┬───────────────┘
              │
     ┌────────┴─────────┐
     ▼                  ▼
 MockBackend      ProcessBackend
 (tests/CI)       WEFT_CAPTURE_SIDECAR
     │                  │
     └────────┬─────────┘
              ▼
     UnavailableBackend  ← default (graceful)
```

## Wire protocol

### Webview → host

```json
{
  "type": "capture-request",
  "id": 1,
  "method": "status|list_devices|start|stop|poll|test_level|grant_consent|revoke_consent",
  "params": { "media": "mic", "device": "optional" }
}
```

### Host → webview

```json
{
  "type": "capture-response",
  "id": 1,
  "method": "poll",
  "ok": true,
  "result": {
    "pcm_i16": [/* mono s16 */],
    "sample_rate": 16000,
    "peak": 0.12,
    "capturing": true,
    "pcm_b64": "…",
    "whisper": {
      "data": "…",
      "sample_rate": 16000,
      "channels": 1,
      "seq": 3,
      "samples": 1600,
      "chunk_ms": 100
    }
  }
}
```

On panel `ready`, `hello` includes a `capture` status snapshot so the
UI can hide or disable voice affordances when the backend is
unavailable.

### Methods

| Method | Purpose |
|--------|---------|
| `status` | Bridge + mic/camera availability, backend kind, consent |
| `list_devices` | Input device list (mock / process) |
| `grant_consent` / `revoke_consent` | Host-side privacy gate (`scope://mic`) |
| `start` / `stop` | Continuous mono capture session |
| `poll` | Drain PCM + peak + whisper-shaped chunk |
| `test_level` | Short probe for settings UI |

`consent.request` with `scope://mic` or `scope://camera` (WSP-0.1)
also grants the host bridge. `consent.revoke` clears it and stops
capture.

Raw RPC `sensor.mic.status` and WSP-mapped
`resource://sensor/mic/status` are answered by the host (not the
daemon), matching the M1.5.2 reservation of that method for the
audio bridge.

## Backends

| Kind | When | Behavior |
|------|------|----------|
| `unavailable` | **Default** | Status reports `available: false` with reason; `start` returns graceful error; panel still loads |
| `mock` | `WEFT_CAPTURE_BACKEND=mock` | Synthetic 440 Hz sine @ 16 kHz — unit tests + smoke without hardware |
| `process` | `WEFT_CAPTURE_SIDECAR=/path/to/bin …` | Spawn sidecar; read mono s16le from stdout |

### Environment

| Variable | Effect |
|----------|--------|
| `WEFT_CAPTURE_BACKEND` | `mock` \| `process` \| `unavailable` |
| `WEFT_CAPTURE_SIDECAR` | Command line for process backend (implies process when set) |
| `WEFT_CAPTURE_SAMPLE_RATE` | Sample rate for process/mock (default `16000`) |

### External sidecar contract

The process backend expects:

- Binary writes **continuous little-endian mono s16** PCM to **stdout**
- Sample rate matches `WEFT_CAPTURE_SAMPLE_RATE` (default 16 kHz)
- Env may include `WEFT_CAPTURE_MEDIA`, `WEFT_CAPTURE_DEVICE`
- Immediate exit → `start` fails with stderr tail (graceful unavailable)

Example (macOS, if `sox` is installed):

```bash
export WEFT_CAPTURE_SIDECAR='sox -q -d -t raw -b 16 -e signed -c 1 -r 16000 -'
export WEFT_CAPTURE_SAMPLE_RATE=16000
```

## Camera

Camera is **reserved** in the status/consent surface but **not
implemented** in this MVP. `start` with `media: "camera"` returns a
graceful `unavailable` error so UI can degrade without crashing.

## Whisper smoke path

`poll` attaches a `whisper` object shaped like
`clawft-service-whisper::PcmChunk` (`data` base64, `sample_rate`,
`channels`, `seq`, `samples`, `chunk_ms`). Smoke steps:

1. `WEFT_CAPTURE_BACKEND=mock` (or a real process sidecar).
2. Open the panel; `hello.capture.mic.available === true` (mock) or
   configure process.
3. WSP `consent.request` with `scope: "scope://mic"` **or**
   `capture-request` `grant_consent`.
4. `capture-request` `start` → `poll` → take `result.whisper`.
5. Publish that JSON onto
   `substrate/<node>/sensor/mic/pcm_chunk` (daemon-mediated path;
   webview must **not** call raw `substrate.publish` — WEFT-496).
   For a local smoke without full panel publish, post the chunk via
   a privileged host tool or the whisper service’s existing test
   harness.

See `extensions/vscode-weft-panel/SMOKE.md` §11.

## Privacy

- Default is **no capture** (unavailable backend; no device open).
- Consent is required before `start` / `test_level`.
- Panel dispose and `session.shutdown` stop capture and clear consent.
- Webview never receives OS-level mic tokens; only PCM the host chose
  to forward.

## Out of scope / follow-ups

- Native cpal Node addon (optional later; process sidecar covers MVP).
- Camera frames and preview.
- Automatic substrate publish of PCM from the extension host
  (today: shape helpers only; publish remains grant-gated daemon path).
- Wasm Live loop UI chrome for a push-to-talk control.
