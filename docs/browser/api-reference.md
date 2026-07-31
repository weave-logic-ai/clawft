# Browser API Reference

clawft-wasm exports wasm-bindgen functions for browser use (`init`,
`send_message`, `set_env`, `get_env`, plus tool introspection helpers).

To run the same surface **off the main thread**, use the Web Worker harness
and `ClawftWorkerClient` (`crates/clawft-wasm/www/`) — see
[`web-worker.md`](web-worker.md) (WEFT-400). The worker protocol mirrors
these entry points via `postMessage` (`init`, `send`, `stream`, `set_env`, …).

## Lifecycle

The expected call order is:

```
init(config_json, env_json?) --> set_env(key, value) --> send_message(text)
                                 (optional, repeatable)   (repeatable)
```

1. **`init()`** must be called once before any other function.
2. **`set_env()`** can be called zero or more times to inject environment
   variables into the live `BrowserEnvironment` after initialization
   (WEFT-391). Optionally pre-seed via `init`'s second argument.
3. **`send_message()`** can be called any number of times to send messages
   through the pipeline.

---

## `init(config_json: string, env_json?: string): Promise<void>`

Initialize the clawft-wasm browser runtime.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config_json` | `string` | Yes | JSON string matching the clawft Config schema. |
| `env_json` | `string` | No | Optional JSON object of string→string pairs used to pre-seed `BrowserEnvironment` before the platform is moved into `AgentLoop` (WEFT-391). Omit, pass `undefined`, or `null` to skip. |

### Behavior

1. Installs `console_error_panic_hook` for readable Rust panic messages.
2. Parses the JSON string into a `Config` struct.
3. Parses optional `env_json` into an in-memory `BrowserEnvironment`.
4. Creates a `BrowserPlatform` sharing that environment via `Arc` (in-memory filesystem, fetch-based HTTP).
5. Wires `AgentLoop<BrowserPlatform>` and stores a live `Arc<BrowserEnvironment>` on the global `BrowserRuntime` so later `set_env` calls mutate the same map the agent sees.
6. Logs `"clawft-wasm initialized"` to the browser console.

### Errors

Returns a rejected `Promise<JsValue>` (string) if:

- **`config_json` is not valid JSON** -- Error message: `"config parse error: <serde details>"`.
- **`config_json` does not match the Config schema** -- Same parse error format. Note that unknown fields are silently ignored and all top-level sections default to empty, so only malformed JSON or type mismatches cause errors.
- **`env_json` is present but not a JSON object of strings** -- `"env map parse error: …"` or `"env map must be a JSON object…"` / `"env map value for key '…' must be a string"`.
- **No API key for the selected model** -- provider configuration error.

### Example

```javascript
import init_wasm, { init, set_env } from "./pkg/clawft_wasm.js";

await init_wasm(); // Load the .wasm binary

try {
  await init(
    JSON.stringify({
      providers: {
        anthropic: { apiKey: "sk-ant-...", browserDirect: true }
      }
    }),
    // Optional pre-seed (WEFT-391)
    JSON.stringify({ CLAWFT_MODEL: "anthropic/claude-sonnet-4-20250514" }),
  );
  // Live mutation after init — same Arc as Platform.env()
  set_env("CLAWFT_DEBUG", "1");
  console.log("Ready");
} catch (err) {
  console.error("Init failed:", err);
}
```

---

## `send_message(text: string): Promise<string>`

Send a message through the clawft pipeline and receive a response.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `text` | `string` | Yes | The user message to process. |

### Return value

A `Promise<string>` that resolves to the assistant's response text.

Currently returns a placeholder: `"clawft-wasm browser: received '<text>'"`.
Once the full AgentLoop is wired, this will return the LLM-generated response.

### Errors

Returns a rejected `Promise<JsValue>` (string) if:

- The pipeline encounters an error during processing.
- The configured LLM provider returns an error (once wired).

### Example

```javascript
try {
  const response = await send_message("What is 2 + 2?");
  console.log("Assistant:", response);
} catch (err) {
  console.error("Send failed:", err);
}
```

---

## `set_env(key: string, value: string): void`

Set an environment variable on the live `BrowserEnvironment` (WEFT-391).

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | `string` | Yes | The environment variable name (e.g., `"ANTHROPIC_API_KEY"`). |
| `value` | `string` | Yes | The value to set. |

### Behavior

After `init()`, mutates the shared `Arc<BrowserEnvironment>` held by
`BrowserRuntime` — the same map exposed via `Platform::env()` inside the
agent loop. This is the browser equivalent of `process.env` or shell
`export`.

Before `init()`, this is a safe no-op (does not panic). Prefer the optional
`env_json` argument to `init()` for pre-seed values.

### Example

```javascript
set_env("ANTHROPIC_API_KEY", "sk-ant-...");
set_env("CLAWFT_MODEL", "anthropic/claude-sonnet-4-20250514");
```

---

## `get_env(key: string): string | undefined`

Read an environment variable from the live `BrowserEnvironment` (WEFT-391).

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | `string` | Yes | The environment variable name. |

### Behavior

Returns the current value when the runtime is initialized and the key is
set; otherwise `undefined`. Useful for dashboard debugging and tests that
verify `set_env` round-trips.

### Example

```javascript
set_env("CLAWFT_DEBUG", "1");
console.log(get_env("CLAWFT_DEBUG")); // "1"
```

---

## Error Handling Patterns

### Catching init errors

```javascript
async function safeInit(config) {
  try {
    await init(JSON.stringify(config));
    return { ok: true };
  } catch (err) {
    // err is a string from JsValue
    return { ok: false, error: String(err) };
  }
}
```

### Catching message errors with retry

```javascript
async function sendWithRetry(text, maxRetries = 2) {
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await send_message(text);
    } catch (err) {
      if (attempt === maxRetries) throw err;
      // Exponential backoff
      await new Promise(r => setTimeout(r, 1000 * Math.pow(2, attempt)));
    }
  }
}
```

### Validating config before init

```javascript
function validateConfig(config) {
  if (!config.providers || Object.keys(config.providers).length === 0) {
    throw new Error("At least one provider must be configured");
  }
  for (const [name, provider] of Object.entries(config.providers)) {
    if (provider.apiKey && provider.apiKey.startsWith("YOUR_")) {
      throw new Error(`Provider '${name}' has a placeholder API key`);
    }
  }
}
```

---

## TypeScript Types

After building with wasm-pack, type declarations are available in `pkg/`:

```typescript
// From clawft_wasm.d.ts
export function init(config_json: string, env_json?: string): Promise<void>;
export function send_message(text: string): Promise<string>;
export function set_env(key: string, value: string): void;
export function get_env(key: string): string | undefined;

// Default initializer (loads .wasm binary)
export default function init_wasm(): Promise<void>;
```
