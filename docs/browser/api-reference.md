# Browser API Reference

clawft-wasm exports three functions via wasm-bindgen for browser use.

## Lifecycle

The expected call order is:

```
init(config_json) --> set_env(key, value) --> send_message(text)
                      (optional, repeatable)   (repeatable)
```

1. **`init()`** must be called once before any other function.
2. **`set_env()`** can be called zero or more times to inject environment
   variables (e.g., API keys) after initialization.
3. **`send_message()`** can be called any number of times to send messages
   through the pipeline.

---

## `init(config_json: string): Promise<void>`

Initialize the clawft-wasm browser runtime.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `config_json` | `string` | Yes | JSON string matching the clawft Config schema. |

### Behavior

1. Installs `console_error_panic_hook` for readable Rust panic messages.
2. Parses the JSON string into a `Config` struct.
3. Creates a `BrowserPlatform` instance (in-memory filesystem, fetch-based HTTP, in-memory environment).
4. Logs `"clawft-wasm initialized"` to the browser console.

### Errors

Returns a rejected `Promise<JsValue>` (string) if:

- **`config_json` is not valid JSON** -- Error message: `"config parse error: <serde details>"`.
- **`config_json` does not match the Config schema** -- Same parse error format. Note that unknown fields are silently ignored and all top-level sections default to empty, so only malformed JSON or type mismatches cause errors.

### Example

```javascript
import init_wasm, { init } from "./pkg/clawft_wasm.js";

await init_wasm(); // Load the .wasm binary

try {
  await init(JSON.stringify({
    providers: {
      anthropic: { apiKey: "sk-ant-...", browserDirect: true }
    }
  }));
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

Set an environment variable on the BrowserPlatform.

### Parameters

| Name | Type | Required | Description |
|------|------|----------|-------------|
| `key` | `string` | Yes | The environment variable name (e.g., `"ANTHROPIC_API_KEY"`). |
| `value` | `string` | Yes | The value to set. |

### Behavior

Stores the key-value pair in the BrowserPlatform's in-memory environment.
This is the browser equivalent of `process.env` or shell `export`.

Currently a stub -- will be wired to `BrowserPlatform.env().set_var()` once
the platform instance is stored globally during `init()`.

### Example

```javascript
set_env("ANTHROPIC_API_KEY", "sk-ant-...");
set_env("CLAWFT_MODEL", "anthropic/claude-sonnet-4-20250514");
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
export function init(config_json: string): Promise<void>;
export function send_message(text: string): Promise<string>;
export function set_env(key: string, value: string): void;

// Default initializer (loads .wasm binary)
export default function init_wasm(): Promise<void>;
```
