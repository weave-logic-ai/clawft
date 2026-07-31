# Web Worker harness (WEFT-400)

Main-thread WASM freezes the tab UI during long LLM calls and tool work.
`BrowserHttpClient` already prefers `WorkerGlobalScope::fetch` when present,
but the HTML harness historically only loaded the agent on the window thread.

WEFT-400 adds a **module Web Worker** harness so the agent loop runs off the
main thread. The host talks to the worker over a small, versioned
`postMessage` protocol.

## Quick start

```bash
# Build pkg into crates/clawft-wasm/www/pkg/
scripts/build.sh browser

# Serve the harness (default :8080)
scripts/build.sh serve
```

Open:

| URL | Mode |
|-----|------|
| `http://localhost:8080/` | Main-thread harness (`index.html` + `main.js`) |
| `http://localhost:8080/index-worker.html` | **Web Worker harness** (`main-worker.js` → `worker.js`) |

## Layout

```
crates/clawft-wasm/www/
  protocol.js         Shared request/response types + validators
  worker.js           Module worker entry (loads pkg/, handles messages)
  worker-client.js    Main-thread ClawftWorkerClient facade
  main-worker.js      Harness UI wired to the client
  index-worker.html   Worker harness page
  protocol.test.mjs   Node unit tests for the protocol
  main.js / index.html  Existing main-thread harness (unchanged)
  pkg/                wasm-pack output (gitignored; from build.sh browser)
```

## Message protocol (v1)

All messages are structured-cloneable plain objects. Every **request** carries
a correlation `id` (`string | number`). Responses echo that `id` except the
unsolicited `worker_ready` event.

### Host → Worker

| `type` | Fields | Effect |
|--------|--------|--------|
| `ping` | — | Health; replies `pong` with load/init flags |
| `load` | — | Import + init WASM binary (idempotent) |
| `init` | `configJson`, `envJson?` | `init(config, env)` inside the worker |
| `send` | `text` | `send_message(text)` → full reply |
| `stream` | `text` | `stream_chat` → zero+ `chunk` then `ok` |
| `set_env` | `key`, `value` | Live env mutation after init |
| `get_env` | `key` | Read env value (`null` if missing) |
| `boot_info` | — | Kernel boot trace JSON (no init required) |
| `analyze_files` | `filesJson` | Offline analyzer JSON (no init required) |
| `tool_list` | — | Tool inventory (requires init) |
| `tool_schema` | `key` (slug) | Single tool schema (requires init) |

### Worker → Host

| `type` | Fields | Notes |
|--------|--------|-------|
| `worker_ready` | `protocolVersion` | Unsolicited after eager WASM load; **no `id`** |
| `pong` | `id`, `protocolVersion`, `wasmLoaded`, `initialized` | Reply to `ping` |
| `ok` | `id`, `result`, optional `text` | Success |
| `chunk` | `id`, `text` | Streaming partial (same `id` as `stream`) |
| `error` | `id`, `error` | Failure string |

Invalid envelopes are rejected by `validateRequest` / `validateResponse` in
`protocol.js`. The worker answers invalid requests with `type: "error"`.

### Example

```javascript
import { ClawftWorkerClient } from "./worker-client.js";

const client = new ClawftWorkerClient();
await client.start();                 // waits for worker_ready / load
await client.init(JSON.stringify({
  providers: { anthropic: { apiKey: "…", browserDirect: true } },
  agents: { defaults: { model: "anthropic/claude-sonnet-4-20250514" } },
}));

const reply = await client.sendMessage("Hello from a worker");
// or stream:
const full = await client.streamChat("Hello", (chunk) => {
  process.stdout?.write?.(chunk); // or append to DOM
});
```

Raw `postMessage` shape (without the client):

```javascript
const worker = new Worker(new URL("./worker.js", import.meta.url), {
  type: "module",
});

worker.postMessage({
  id: 1,
  type: "init",
  configJson: JSON.stringify(config),
});

worker.onmessage = (ev) => {
  const { id, type, result, error, text } = ev.data;
  // type: worker_ready | ok | chunk | error | pong
};
```

## Why module workers

`scripts/build.sh browser` produces **wasm-pack `--target web`** ESM under
`www/pkg/`. Module workers can `import("./pkg/clawft_wasm.js")` the same way
`main.js` does on the window. Classic `importScripts` + `--target no-modules`
is **not** required and is not used by this harness.

## Platform notes

- **HTTP**: `BrowserHttpClient` already uses `WorkerGlobalScope::fetch` when
  `js_sys::global()` is a worker (see
  `crates/clawft-platform/src/browser/http.rs`).
- **OPFS** (`browser-opfs`): available in secure contexts in workers as well as
  windows; the same init path applies.
- **CORS proxy**: the worker harness injects the same localhost
  `/proxy` base as the main-thread harness when served via
  `scripts/build.sh serve`.
- **Streaming**: `stream` maps `stream_chat`'s JS callback to `chunk`
  messages so the UI can paint tokens without blocking.

## Tests

```bash
# Protocol contract (no browser, no WASM binary)
node --test crates/clawft-wasm/www/protocol.test.mjs

# Existing headless Chrome suite (wasm-bindgen entry contracts; main thread)
scripts/build.sh test-browser
```

Manual: open `index-worker.html`, initialize with a real key, send a message,
and confirm the status light stays green while the worker is waiting on the
provider (main-thread harness would jank on the same call).

## Decision record (gap close)

| Decision | Choice |
|----------|--------|
| Off-main-thread agent | **Yes** — default demo path is `index-worker.html` for long calls |
| Worker type | **ES module** worker (`type: "module"`) |
| IPC | Versioned JSON-like objects over `postMessage` (`protocol.js` v1) |
| Streaming | First-class `stream` + `chunk` (not only fire-and-forget `send`) |
| Main-thread harness | **Kept** (`index.html`) for simplest debugging / parity |
| Service worker / PWA | Out of scope (UI sprint / later WEFT) |

## Related docs

- [Architecture](architecture.md) — browser vs native, data flow
- [API reference](api-reference.md) — wasm-bindgen surface (`init`, `send_message`, …)
- [Building](building.md) — `scripts/build.sh browser` / `serve`
- [Quickstart](quickstart.md) — minimal main-thread page
