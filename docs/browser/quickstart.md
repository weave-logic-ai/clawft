# Browser Quickstart

Get clawft running in a web browser in under five minutes.

## 1. Build the WASM module

```bash
cd crates/clawft-wasm
wasm-pack build --target web --no-default-features --features browser -- --no-default-features --features browser
```

This produces `pkg/clawft_wasm.js` and `pkg/clawft_wasm_bg.wasm`.

## 2. Create a minimal HTML page

Create `test.html` alongside the `pkg/` directory:

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8" />
  <title>clawft-wasm test</title>
</head>
<body>
  <pre id="output">Loading...</pre>
  <script type="module">
    import init_wasm, { init, send_message, set_env } from "./pkg/clawft_wasm.js";

    const output = document.getElementById("output");

    async function run() {
      // Step 1: Load the WASM binary.
      await init_wasm();
      output.textContent = "WASM loaded.\n";

      // Step 2: Configure and initialize.
      const config = {
        providers: {
          anthropic: {
            apiKey: "YOUR_API_KEY_HERE",
            browserDirect: true
          }
        },
        agents: {
          defaults: {
            model: "anthropic/claude-sonnet-4-20250514",
            maxTokens: 4096
          }
        }
      };

      await init(JSON.stringify(config));
      output.textContent += "Initialized.\n";

      // Step 3 (optional): Set environment variables.
      set_env("ANTHROPIC_API_KEY", "YOUR_API_KEY_HERE");

      // Step 4: Send a message.
      const response = await send_message("Hello from the browser!");
      output.textContent += "Response: " + response + "\n";
    }

    run().catch(err => {
      output.textContent = "Error: " + err;
    });
  </script>
</body>
</html>
```

## 3. Serve and open

```bash
npx serve crates/clawft-wasm/ --listen 8080
```

Open `http://localhost:8080/test.html` in a browser.

## 4. Expected output

```
WASM loaded.
Initialized.
Response: clawft-wasm browser: received 'Hello from the browser!'
```

The response is currently a placeholder. Once the full pipeline is wired,
this will return an LLM-generated response.

## Config setup

The config JSON mirrors the native `config.json` structure. For browser use,
the key fields are:

```json
{
  "providers": {
    "anthropic": {
      "apiKey": "sk-ant-...",
      "browserDirect": true
    }
  },
  "agents": {
    "defaults": {
      "model": "anthropic/claude-sonnet-4-20250514",
      "maxTokens": 4096,
      "temperature": 0.7
    }
  }
}
```

- **`browserDirect`**: Set to `true` for providers whose APIs support
  browser CORS (e.g., Anthropic). For other providers, configure a CORS
  proxy via `corsProxy`.
- **`apiKey`**: Your provider API key. In production, inject this at
  runtime rather than hardcoding it in source.

## Using the test harness

The built-in test harness at `www/index.html` provides a chat UI with:

- A JSON config editor
- An "Initialize" button
- A message display with user/assistant/error/system styling
- Console timing for load, init, and message latency

To use it:

```bash
npx serve crates/clawft-wasm/www --listen 8080
```

Then open `http://localhost:8080`.
