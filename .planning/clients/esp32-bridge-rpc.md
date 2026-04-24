# ESP32 bridge → WeftOS daemon RPC brief

Answering the Windows-side bridge: "how do we get mic-per-window data into the daemon so the audio chip lights up." As of 2026-04-22.

## TL;DR

You were aiming at **TCP 9470**. That's the weaver↔weaver mesh gossip port (Noise-wrapped peer auth, closed to non-peers — that's the rejection you hit). It is **not** an RPC port.

The RPC surface is line-delimited JSON over a **Unix domain socket**:

```
/home/aepod/dev/clawft/.weftos/runtime/kernel.sock
```

Three methods matter right now: `agent.register`, `substrate.publish`, `substrate.read`.

## Wire format

One JSON object per line, `\n`-terminated, both directions. No framing header in JSON mode. (A four-byte `RVFS` magic prefix triggers the RVF-framed path — don't send it.)

Source of truth: `crates/clawft-rpc/src/protocol.rs`, `crates/clawft-weave/src/daemon.rs` (`handle_json_connection`).

Request envelope:
```json
{"method": "<name>", "params": { ... }, "id": "optional-correlation-id"}
```

Response envelope:
```json
{"ok": true,  "result": { ... }}
{"ok": false, "error": "<reason>"}
```

## Endpoint: how to reach the Linux unix socket from Windows

Windows Python can't open a WSL2 `AF_UNIX` socket through the 9P mount. Three workable options, in order of simplicity:

### Option A — run the bridge inside WSL (cleanest)

`wsl python3 /mnt/c/path/to/bridge.py`. The unix socket is native, no relay, no TCP.

### Option B — built-in daemon TCP relay (recommended)

The daemon itself now speaks JSON-RPC over TCP when opted in. Add to `weave.toml`:

```toml
[kernel.ipc_tcp]
enabled = true
listen_addr = "127.0.0.1:9471"   # loopback-only default; use 0.0.0.0:9471 to accept from Windows host
```

On boot the daemon binds the TCP port alongside the unix socket; each accepted TCP connection is transparently byte-copied into a fresh unix-socket connection. Same line-delimited JSON protocol, same auth path. Binding `0.0.0.0` is an explicit opt-in since it widens the trust surface. Restart the daemon to pick up config changes: `target/release/weaver kernel start --foreground`.

### Option C — external socat relay (no daemon change)

If you can't restart the daemon, run the same relay externally:

```bash
socat TCP-LISTEN:9471,reuseaddr,fork \
  UNIX-CONNECT:/home/aepod/dev/clawft/.weftos/runtime/kernel.sock
```

### Option D — stdio through `wsl.exe`

```
wsl.exe socat STDIO UNIX-CONNECT:/home/aepod/dev/clawft/.weftos/runtime/kernel.sock
```
Pipe JSON lines through the subprocess. No port, no firewall surface.

Pick A if you want to own the whole thing in WSL; B for a permanent TCP endpoint with no extra moving parts; C for a zero-restart quick bring-up.

## Method: `agent.register`

Mints a server-side `agent_id` (UUID v4) for a bridge-owned Ed25519 keypair. Call once at bridge startup; persist the returned `agent_id`.

Param struct (`crates/clawft-weave/src/protocol.rs:451`):

| Field | Type | Notes |
|---|---|---|
| `name` | string | human-readable display name (`"esp32-bridge"` is fine) |
| `pubkey` | string | Ed25519 public key, 32 raw bytes encoded as hex **or** base64; both accepted |
| `proof` | string | Ed25519 signature, 64 raw bytes encoded as hex **or** base64 |
| `ts` | u64 | unix milliseconds the proof was generated at |

`proof` is a signature over the **register payload bytes** (`crates/clawft-kernel/src/agent_registry.rs:90`):
```
b"register\0" || name_utf8 || 0x00 || pubkey_32 || 0x00 || ts_u64_LITTLE_ENDIAN
```
No length prefixes, no padding. The separators are literal NUL bytes.

Response:
```json
{"ok": true, "result": {"agent_id": "<uuid>", "name": "esp32-bridge"}}
```

## Method: `substrate.publish`

Bulk data write. This is what you call once per audio window.

Param struct (`protocol.rs:388`):

| Field | Type | Required | Notes |
|---|---|---|---|
| `path` | string | yes | e.g. `"substrate/sensor/mic"` |
| `value` | JSON value | yes | see **value shape** below |
| `actor_id` | string | no | if present, `signature` also required |
| `signature` | string | no | Ed25519 sig (hex/base64) over publish payload |
| `ts` | u64 | no | unix ms, required when signing |

Publish payload byte layout (`agent_registry.rs:105`):
```
b"ipc.publish\0" || path_utf8 || 0x00 || value_bytes_utf8 || 0x00
  || ts_u64_LITTLE_ENDIAN || 0x00 || actor_id_utf8
```
`value_bytes` is the exact JSON serialisation you put on the wire — compact form, no pretty-printing. The daemon recomputes the same bytes via `serde_json::to_vec`. Byte-identical match required.

**Unsigned publish is currently accepted with a warn log** (policy migration in progress). Leave `actor_id` and `signature` out for the first bring-up loop; add them once peaks are rendering.

Response:
```json
{"ok": true, "result": {"path": "substrate/sensor/mic", "tick": 1234}}
```

### Value shape the GUI already expects

The Audio chip fixture `crates/clawft-surface/fixtures/weftos-chip-audio.toml` binds six fields. Publish this exact object at `substrate/sensor/mic` and the existing `peak_db` and `rms_db` gauges move immediately, no other GUI work:

```json
{
  "rms_db": -34.2,
  "peak_db": -12.1,
  "sample_rate": 16000,
  "samples_in_window": 8000,
  "available": true,
  "characterization": "Rate"
}
```

Units:
- `rms_db`, `peak_db` — dBFS, range `-120.0 … 0.0` (gauge domain is clamped to that). `0 dBFS = clipping`.
- `sample_rate` — samples/s as plain integer.
- `samples_in_window` — count of PCM samples in the window this record summarises.
- `available` — bool; false greys the chip.
- `characterization` — `"Rate"` until we add spectral bins.

Suggested cadence: one publish per 500 ms window (matches the existing `MicrophoneAdapter`'s window).

## Method: `substrate.read`

Sanity check your writes landed.

```json
{"method":"substrate.read","params":{"path":"substrate/sensor/mic"}}
```

Response:
```json
{"ok": true, "result": {"value": { ... }, "tick": 1234, "sensitivity": "capture"}}
```

`tick` increments on every publish to that path; useful for detecting stalls.

## Minimal Python sanity loop

```python
import json, socket

s = socket.socket(socket.AF_UNIX)            # or AF_INET + ("wsl-ip", 9471) for socat
s.connect("/home/aepod/dev/clawft/.weftos/runtime/kernel.sock")
f = s.makefile("rwb", buffering=0)

def call(method, params):
    f.write((json.dumps({"method": method, "params": params}) + "\n").encode())
    f.flush()
    return json.loads(f.readline())

# Unsigned fast path — works today.
print(call("substrate.publish", {
    "path": "substrate/sensor/mic",
    "value": {"rms_db": -34.2, "peak_db": -12.1, "sample_rate": 16000,
              "samples_in_window": 8000, "available": True,
              "characterization": "Rate"},
}))

print(call("substrate.read", {"path": "substrate/sensor/mic"}))
```

Expected: the chip's peak gauge moves the moment the first publish lands.

## Chain-side: you don't need to do anything

You mentioned the bridge already chains locally with 5691 BLAKE3 entries — good, keep that. The daemon's own `StreamWindowAnchor` handles the on-chain audit trail on this side once `[kernel.anchor]` is flipped on in `weave.toml`. Local-bridge chain ↔ daemon chain stay separate and that's fine — both are control-plane audit anchors for their respective halves.

## Where this lives in the tree

| File | Why |
|---|---|
| `crates/clawft-rpc/src/protocol.rs` | Request/Response envelopes, socket-path resolution |
| `crates/clawft-weave/src/daemon.rs` | `handle_json_connection`, dispatch table (L1823–L1825), handler bodies |
| `crates/clawft-weave/src/protocol.rs` | Param/result structs for every method |
| `crates/clawft-kernel/src/agent_registry.rs` | `register_payload` / `publish_payload` byte layouts |
| `crates/clawft-weave/tests/agent_register_and_sign.rs` | End-to-end test, good reference for signing |
| `crates/clawft-weave/tests/substrate_rpc.rs` | Read/publish/subscribe integration tests |
| `crates/clawft-surface/fixtures/weftos-chip-audio.toml` | Audio chip fixture — the shape the GUI binds against |
