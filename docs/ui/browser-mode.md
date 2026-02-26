# Browser Mode Guide

> Running ClawFT entirely in the browser with WebAssembly -- no server required.

## What is Browser Mode?

Browser mode allows the ClawFT dashboard to run without a backend server. Instead of communicating with an Axum REST/WS backend, the UI loads a WebAssembly (WASM) module that runs the clawft agent loop directly in the browser. Your API key is sent directly from the browser to the LLM provider.

This is useful for:

- **Quick demos** -- share a URL, no server setup required
- **Privacy** -- API keys never leave the browser (encrypted with Web Crypto)
- **Offline** -- once loaded, the WASM module works without a network connection (except for LLM API calls)
- **Edge deployment** -- host on any static file server (S3, Cloudflare Pages, GitHub Pages)

## Browser Requirements

| Browser | Minimum Version | Notes |
|---------|----------------|-------|
| Chrome / Edge | 102+ | Full support including OPFS |
| Firefox | 111+ | Full support |
| Safari | 15.2+ | OPFS support may be limited |

Required browser features:

- **WebAssembly** -- core requirement
- **Web Crypto API** -- for encrypting API keys in IndexedDB
- **IndexedDB** -- for persisting configuration
- **Origin Private File System (OPFS)** -- for file operations (falls back to in-memory if unavailable)
- **Fetch Streaming** -- for streaming LLM responses (non-streaming fallback if unavailable)

The UI automatically detects these features and shows warnings if any are missing.

## Building the WASM Module

```bash
# Prerequisites
cargo install wasm-pack

# Build with browser feature enabled
wasm-pack build crates/clawft-wasm --target web --features browser

# Output files:
#   crates/clawft-wasm/pkg/clawft_wasm.js       (JS glue)
#   crates/clawft-wasm/pkg/clawft_wasm_bg.wasm   (WASM binary)
```

Copy the output to the UI public directory:

```bash
cp crates/clawft-wasm/pkg/clawft_wasm.js ui/public/
cp crates/clawft-wasm/pkg/clawft_wasm_bg.wasm ui/public/
```

## Deploying Browser-Only UI

1. Build the UI:

```bash
cd ui
VITE_BACKEND_MODE=wasm npm run build
```

2. Copy WASM files to `dist/`:

```bash
cp public/clawft_wasm.js dist/
cp public/clawft_wasm_bg.wasm dist/
```

3. Deploy `dist/` to any static hosting:

```bash
# Example: Cloudflare Pages
npx wrangler pages deploy dist/

# Example: GitHub Pages
# Push dist/ contents to gh-pages branch

# Example: S3
aws s3 sync dist/ s3://my-bucket --delete
```

The server must set appropriate CORS and COOP/COEP headers. See the Deployment Guide for details.

## Activating Browser Mode

Browser mode is activated by setting `VITE_BACKEND_MODE=wasm` at build time, or by adding `?mode=wasm` to the URL at runtime:

```
https://your-app.example.com/?mode=wasm
```

Auto-detection mode (`?mode=auto` or `VITE_BACKEND_MODE=auto`) will:
1. Try to reach the Axum backend at `/api/health`
2. If unreachable, fall back to WASM mode

## Provider CORS Setup

### Anthropic

Anthropic supports direct browser access. No CORS proxy needed.

Set the API key in the Browser Mode Setup screen. The key is encrypted with AES-256-GCM using Web Crypto and stored in IndexedDB.

```
Provider: Anthropic (direct browser access)
API Key:  sk-ant-api03-...
Model:    claude-sonnet-4-5-20250929
```

### OpenAI

OpenAI does **not** allow direct browser access (no CORS headers). You need a CORS proxy.

**Option 1: Self-hosted proxy** (recommended)

Deploy a minimal CORS proxy:

```javascript
// cors-proxy.js (Cloudflare Worker)
export default {
  async fetch(request) {
    const url = new URL(request.url);
    const targetUrl = "https://api.openai.com" + url.pathname;

    const response = await fetch(targetUrl, {
      method: request.method,
      headers: {
        ...Object.fromEntries(request.headers),
        "Host": "api.openai.com",
      },
      body: request.body,
    });

    return new Response(response.body, {
      status: response.status,
      headers: {
        ...Object.fromEntries(response.headers),
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Headers": "Authorization, Content-Type",
        "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
      },
    });
  }
};
```

**Configuration:**

```
Provider:       OpenAI (requires CORS proxy)
API Key:        sk-...
Model:          gpt-4o
CORS Proxy URL: https://your-proxy.workers.dev/
```

### Ollama (Local)

Ollama runs locally and serves an OpenAI-compatible API.

```bash
# Start Ollama with CORS enabled
OLLAMA_ORIGINS="*" ollama serve
```

**Configuration:**

```
Provider: Ollama (local, http://localhost:11434)
Model:    llama3.3
```

No API key needed. Ollama must be running on the same machine as the browser.

### LM Studio (Local)

LM Studio provides a local API server.

1. Open LM Studio
2. Start the local server (default: `http://localhost:1234`)
3. Enable CORS in LM Studio settings

**Configuration:**

```
Provider: LM Studio (local, http://localhost:1234)
Model:    loaded-model
```

## API Key Security

In browser mode, API keys are handled as follows:

1. **Input** -- User enters the API key in the Browser Config screen
2. **Encryption** -- Key is encrypted with AES-256-GCM using Web Crypto
3. **Storage** -- Encrypted key is stored in IndexedDB; the CryptoKey is non-extractable
4. **Usage** -- Plain key is passed to the WASM module during initialization
5. **Transmission** -- Key is sent directly from the browser to the LLM provider via `fetch()`

**Security considerations:**

- The API key exists in browser memory while the page is open
- IndexedDB storage is origin-scoped (same-origin policy)
- The CryptoKey is non-extractable (cannot be read back from IndexedDB)
- Use API keys with restricted permissions (e.g., rate-limited, read-only where possible)
- Clear browser data to remove stored keys

## Feature Limitations (vs Axum Mode)

| Feature | Axum Mode | Browser Mode |
|---------|-----------|--------------|
| Chat / WebChat | Yes | Yes |
| Agents | Multiple | Single (browser-agent) |
| Sessions | Persistent (database) | In-memory (lost on reload) |
| Tools | All | Browser-safe subset (no shell, no spawn) |
| Memory | HNSW vector search | In-memory text search |
| Config | Full editable | Read-only (set at init) |
| Skills | Install from registry | Pre-loaded only |
| Channels | Discord, Slack, Telegram, Web | None |
| Cron Jobs | Yes | No |
| Delegation | Yes | No |
| Monitoring | Yes | No |
| WebSocket | Real-time events | No (poll-based) |
| Multi-user | Yes | No (single user) |

## Troubleshooting

### WASM fails to load

**Symptom:** Loading spinner never completes, error in console.

**Causes:**
- WASM files not in `public/` directory
- Incorrect MIME type for `.wasm` files (must be `application/wasm`)
- Server not setting required headers

**Fix:** Ensure your static server serves `.wasm` files with `Content-Type: application/wasm`. Add COOP/COEP headers if using SharedArrayBuffer:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

### CORS errors

**Symptom:** `No 'Access-Control-Allow-Origin' header` in console.

**Causes:**
- LLM provider does not support browser CORS
- CORS proxy not configured or not running

**Fix:** Use a CORS proxy for providers that do not support direct browser access (OpenAI, custom). For Ollama, start with `OLLAMA_ORIGINS="*"`.

### OPFS not available

**Symptom:** Warning "Origin Private File System is not available" on startup.

**Impact:** File operations use in-memory storage (data lost on page reload).

**Fix:** Use a modern browser (Chrome 102+, Firefox 111+). OPFS requires a secure context (HTTPS or localhost).

### IndexedDB blocked

**Symptom:** Configuration not persisting across reloads.

**Causes:**
- Private/incognito browsing mode
- Browser settings blocking IndexedDB

**Fix:** Use a normal (non-private) browser window.
