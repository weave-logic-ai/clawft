# Phase BW6: Integration Testing

**Phase ID**: BW6
**Workstream**: W-BROWSER
**Duration**: Week 6+
**Depends On**: BW5 (WASM Entry Point -- working browser WASM module)
**Goal**: End-to-end validation, test harness, performance profiling, deployment guide

---

## S -- Specification

### What Changes

This phase validates the complete browser WASM pipeline end-to-end. A minimal HTML/JS test harness is created, OPFS operations are verified, config persistence is tested, and performance is profiled. A Web Worker variant is explored as a stretch goal. Documentation is completed and existing docs are updated.

### Deliverables

| Deliverable | Location | Type |
|---|---|---|
| HTML/JS test harness | `crates/clawft-wasm/www/index.html`, `main.js` | Code |
| E2E pipeline test | Browser-based | Manual + CI |
| OPFS file operations test | Browser-based | Manual |
| Config persistence test | Browser-based | Manual |
| Performance profiling results | `docs/browser/performance.md` | Doc |
| Web Worker variant (stretch) | `crates/clawft-wasm/www/worker.js` | Code |
| Deployment guide | `docs/browser/deployment.md` | Doc |
| Architecture overview | `docs/browser/architecture.md` | Doc |
| README.md update | `README.md` | Doc |
| CLAUDE.md update | `CLAUDE.md` | Doc |

### HTML/JS Test Harness Specification

A single-page application in `crates/clawft-wasm/www/` that:
1. Loads the WASM module via `wasm-pack` output
2. Provides a text input and send button
3. Displays agent responses
4. Shows initialization status
5. Allows API key input (stored in browser memory only, never persisted in WASM)
6. Shows config editor
7. Displays OPFS file listing

### E2E Pipeline Test

Verify the full pipeline works:
```
user msg -> classify -> route -> assemble -> LLM call -> tool use -> response
```

Test cases:
1. **Simple chat**: Send "hello", get a text response (no tool use)
2. **File operations**: Ask to write a file, verify it appears in OPFS
3. **Multi-turn**: Send multiple messages, verify session context is maintained
4. **Tool loop**: Trigger a tool call, verify it executes and result is returned
5. **Error handling**: Send to a misconfigured provider, verify error is returned

### OPFS Test Cases

| Test | Operation | Expected |
|---|---|---|
| Write file | `write_string("/.clawft/test.txt", "hello")` | File created in OPFS |
| Read file | `read_to_string("/.clawft/test.txt")` | Returns "hello" |
| List directory | `list_dir("/.clawft")` | Contains "test.txt" |
| Delete file | `remove_file("/.clawft/test.txt")` | File removed |
| Create nested dirs | `create_dir_all("/.clawft/a/b/c")` | All dirs created |
| Persistence | Write file, reload page, read file | File survives reload |
| Config persistence | Call `init(config)`, reload page, verify config exists | Config persists |

### Performance Profiling

| Metric | Target | Measurement |
|---|---|---|
| WASM module load time | < 500ms | `console.time("wasm-load")` |
| `init()` time | < 200ms | `console.time("init")` |
| First message latency | < 3s (includes LLM call) | `console.time("first-msg")` |
| Subsequent message latency | < 2s (session cached) | `console.time("msg")` |
| OPFS write latency | < 50ms | `console.time("opfs-write")` |
| OPFS read latency | < 20ms | `console.time("opfs-read")` |
| Memory usage (WASM heap) | < 32MB | `performance.memory` (Chrome) |

---

## P -- Pseudocode

### Test Harness HTML

```html
<!-- crates/clawft-wasm/www/index.html -->
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>clawft Browser Agent</title>
    <style>
        body { font-family: monospace; max-width: 800px; margin: 0 auto; padding: 20px; }
        #chat { border: 1px solid #ccc; height: 400px; overflow-y: auto; padding: 10px; margin-bottom: 10px; }
        .user { color: blue; }
        .assistant { color: green; }
        .error { color: red; }
        .system { color: gray; }
        #input-area { display: flex; gap: 10px; }
        #message-input { flex: 1; padding: 8px; font-family: monospace; }
        button { padding: 8px 16px; cursor: pointer; }
        #config-area { margin-bottom: 20px; }
        textarea { width: 100%; height: 150px; font-family: monospace; }
    </style>
</head>
<body>
    <h1>clawft Browser Agent</h1>
    <div id="status" class="system">Loading WASM module...</div>

    <details id="config-area">
        <summary>Configuration</summary>
        <textarea id="config-input">{
  "providers": {
    "anthropic": {
      "api_key": "",
      "base_url": "https://api.anthropic.com",
      "browser_direct": true
    }
  },
  "agents": {
    "defaults": {
      "model": "claude-sonnet-4-5-20250929",
      "max_tokens": 4096,
      "temperature": 0.5,
      "max_tool_iterations": 10,
      "memory_window": 50,
      "workspace": "/.clawft/workspace"
    }
  }
}</textarea>
        <button id="init-btn">Initialize</button>
    </details>

    <div id="chat"></div>
    <div id="input-area">
        <input type="text" id="message-input" placeholder="Type a message..." disabled />
        <button id="send-btn" disabled>Send</button>
    </div>

    <script type="module" src="main.js"></script>
</body>
</html>
```

### Test Harness JavaScript

```javascript
// crates/clawft-wasm/www/main.js
import wasmInit, { init, send_message, set_env } from '../pkg/clawft_wasm.js';

const chat = document.getElementById('chat');
const status = document.getElementById('status');
const input = document.getElementById('message-input');
const sendBtn = document.getElementById('send-btn');
const initBtn = document.getElementById('init-btn');
const configInput = document.getElementById('config-input');

function addMessage(role, text) {
    const div = document.createElement('div');
    div.className = role;
    div.textContent = `[${role}] ${text}`;
    chat.appendChild(div);
    chat.scrollTop = chat.scrollHeight;
}

async function main() {
    try {
        // Load WASM module
        console.time('wasm-load');
        await wasmInit();
        console.timeEnd('wasm-load');
        status.textContent = 'WASM loaded. Configure and click Initialize.';

        initBtn.addEventListener('click', async () => {
            try {
                const config = configInput.value;
                console.time('init');
                await init(config);
                console.timeEnd('init');

                status.textContent = 'Initialized. Ready to chat.';
                input.disabled = false;
                sendBtn.disabled = false;
                addMessage('system', 'Agent initialized successfully.');
            } catch (e) {
                addMessage('error', `Init failed: ${e}`);
            }
        });

        sendBtn.addEventListener('click', sendMsg);
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') sendMsg();
        });

    } catch (e) {
        status.textContent = `Failed to load WASM: ${e}`;
        addMessage('error', `WASM load error: ${e}`);
    }
}

async function sendMsg() {
    const text = input.value.trim();
    if (!text) return;

    input.value = '';
    addMessage('user', text);
    sendBtn.disabled = true;
    input.disabled = true;

    try {
        console.time('msg');
        const response = await send_message(text);
        console.timeEnd('msg');
        addMessage('assistant', response);
    } catch (e) {
        addMessage('error', `Error: ${e}`);
    }

    sendBtn.disabled = false;
    input.disabled = false;
    input.focus();
}

main();
```

### Web Worker Variant (Stretch Goal)

```javascript
// crates/clawft-wasm/www/worker.js
importScripts('../pkg/clawft_wasm.js');

let initialized = false;

self.onmessage = async (event) => {
    const { type, data, id } = event.data;

    if (type === 'init') {
        try {
            await wasm_bindgen('../pkg/clawft_wasm_bg.wasm');
            await wasm_bindgen.init(data.config);
            initialized = true;
            self.postMessage({ id, type: 'init_ok' });
        } catch (e) {
            self.postMessage({ id, type: 'error', error: e.toString() });
        }
    }

    if (type === 'message') {
        if (!initialized) {
            self.postMessage({ id, type: 'error', error: 'not initialized' });
            return;
        }
        try {
            const response = await wasm_bindgen.send_message(data.text);
            self.postMessage({ id, type: 'response', text: response });
        } catch (e) {
            self.postMessage({ id, type: 'error', error: e.toString() });
        }
    }
};
```

The Web Worker approach moves the WASM agent loop off the main thread, keeping the UI responsive during LLM calls and tool execution. The main thread communicates via `postMessage()`.

---

## A -- Architecture

### Connection to UI Sprint (S1/S2/S3)

The WASM module produced by BW5 is the input to the UI sprint. The UI sprint builds the JavaScript application that consumes the WASM module.

```
BW5 Output: clawft_wasm.js + clawft_wasm_bg.wasm
    |
    v
UI Sprint S1: React/Svelte/Vanilla JS application
    |
    +-- Chat interface (consumes send_message())
    +-- Settings panel (calls init() with config)
    +-- File browser (calls OPFS tools)
    +-- Tool result display
    +-- Monaco/CodeMirror editor
    |
    v
UI Sprint S2: PWA packaging
    |
    +-- Service worker for offline support
    +-- Web manifest
    +-- Install prompt
    |
    v
UI Sprint S3: Deployment
    |
    +-- Static hosting (Vercel, Netlify, S3)
    +-- CORS proxy deployment
```

### How the UI Consumes the WASM Module

```javascript
// UI Sprint S1 integration pattern

import wasmInit, { init, send_message, set_env } from 'clawft-wasm';

class ClawftAgent {
    async initialize(config) {
        await wasmInit();
        await init(JSON.stringify(config));
    }

    async chat(message) {
        return await send_message(message);
    }

    setConfig(key, value) {
        set_env(key, value);
    }
}

// React component example:
function ChatApp() {
    const [agent] = useState(() => new ClawftAgent());
    const [messages, setMessages] = useState([]);

    const sendMessage = async (text) => {
        setMessages(prev => [...prev, { role: 'user', text }]);
        const response = await agent.chat(text);
        setMessages(prev => [...prev, { role: 'assistant', text: response }]);
    };

    // ...
}
```

### Deployment Architecture

```
Static Host (Vercel/Netlify/S3)
    |
    +-- index.html
    +-- main.js
    +-- clawft_wasm.js        (wasm-bindgen glue)
    +-- clawft_wasm_bg.wasm   (WASM binary)
    |
    v
Browser loads assets
    |
    +-- WASM module initializes
    +-- User enters API key
    +-- Config stored in OPFS
    |
    +-- Direct LLM calls -----> Anthropic API (browser_direct: true)
    |
    +-- Proxied LLM calls ----> CORS Proxy -----> OpenAI API
```

---

## R -- Refinement

### Risk: E2E Tests Require Real LLM API

The full pipeline test requires a real LLM API call. Mitigations:
1. **Mock LLM server**: Use a local HTTP server that returns canned responses
2. **Ollama**: Run local Ollama for free testing with `browser_direct: true`
3. **CI**: Use a test API key with rate limits for automated testing
4. **Snapshot testing**: Record responses and replay

### Risk: OPFS Tests Require Browser

OPFS is a browser API and cannot be tested in Node.js or cargo tests.
1. **wasm-pack test**: `wasm-pack test --headless --chrome` runs tests in a real browser
2. **Playwright**: Automate browser tests with Playwright
3. **Manual checklist**: Document manual test procedures for developers

### What About Safari?

Safari has OPFS support since 15.2 but with some quirks:
- `FileSystemWritableFileStream` may not be available (use `write()` on the file handle directly)
- Async iteration on directory handles may differ
- Test in Safari Technology Preview before claiming Safari support

### Final Regression Suite

```bash
#!/bin/bash
set -euo pipefail

echo "=== 1. Native unit tests ==="
cargo test --workspace

echo "=== 2. Native clippy ==="
cargo clippy --workspace

echo "=== 3. Native CLI binary ==="
cargo build --release --bin weft

echo "=== 4. WASI WASM build ==="
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm

echo "=== 5. Browser WASM build ==="
wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser

echo "=== 6. Size gate ==="
SIZE=$(gzip -c pkg/clawft_wasm_bg.wasm | wc -c)
echo "Browser WASM gzipped: ${SIZE} bytes"
[ "$SIZE" -lt 512000 ] || echo "WARNING: Exceeds 500KB budget"

echo "=== 7. Feature flag validation ==="
cargo check --target wasm32-unknown-unknown -p clawft-types --no-default-features
cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-core --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser

echo "=== 8. Docker smoke test ==="
# Run existing smoke test from pr-gates.yml if available

echo "=== FULL REGRESSION SUITE PASSED ==="
```

---

## C -- Completion

### Exit Criteria

- [ ] HTML/JS test harness loads WASM module and renders chat UI
- [ ] E2E pipeline test: user message -> full pipeline -> response (at least with mock or Ollama)
- [ ] OPFS: write file, read file, list directory, delete file all work
- [ ] OPFS persistence: file survives page reload
- [ ] Config persistence: init() config survives page reload
- [ ] Performance profiled: WASM load < 500ms, init < 200ms
- [ ] Web Worker variant explored (stretch: functional or documented blockers)
- [ ] Full regression suite passes (native + WASI + browser)
- [ ] No test duration regressions > 10% vs pre-browser baseline

### Documentation Deliverables

1. **`docs/browser/deployment.md`**: Deployment guide
   - Static hosting: Vercel, Netlify, S3 + CloudFront
   - CORS proxy deployment: Cloudflare Worker example, Vercel Edge Function example
   - PWA configuration: manifest.json, service worker
   - Headers: correct MIME types for `.wasm` files
   - HTTPS requirement (OPFS requires secure context)

2. **`docs/browser/architecture.md`**: Architecture overview
   - Updated diagram: browser vs native paths
   - Crate dependency graph with feature flags
   - Data flow: JS -> WASM -> LLM -> WASM -> JS
   - Platform trait implementation comparison table
   - What's shared, what's different

3. **README.md update**: Add "Browser" section
   - One-liner: "clawft runs in the browser via WebAssembly"
   - Build command
   - Link to `docs/browser/quickstart.md`

4. **CLAUDE.md update**: Add browser build commands
   ```
   # Browser WASM
   wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser
   ```

### Phase Gate

```bash
#!/bin/bash
set -euo pipefail

echo "=== Gate 1: Full regression suite ==="
cargo test --workspace
cargo build --release --bin weft
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm
wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser

echo "=== Gate 2: Size gate ==="
SIZE=$(gzip -c pkg/clawft_wasm_bg.wasm | wc -c)
echo "Browser WASM gzipped: ${SIZE} bytes"

echo "=== Gate 3: Test harness exists ==="
test -f crates/clawft-wasm/www/index.html
test -f crates/clawft-wasm/www/main.js

echo "=== Gate 4: Docs exist ==="
test -f docs/browser/deployment.md
test -f docs/browser/architecture.md
test -f docs/browser/building.md
test -f docs/browser/quickstart.md
test -f docs/browser/api-reference.md
test -f docs/browser/cors-provider-setup.md
test -f docs/browser/config-schema.md
test -f docs/architecture/adr-027-browser-wasm-support.md
test -f docs/development/feature-flags.md

echo "BW6 phase gate PASSED -- Workstream W-BROWSER COMPLETE"
```

### Success Criteria (Workstream Complete)

With BW6 complete, all seven workstream success criteria from the orchestrator are met:

1. `cargo build --target wasm32-unknown-unknown -p clawft-wasm --features browser` succeeds
2. WASM module loads in browser, accepts config JSON, initializes `AgentLoop<BrowserPlatform>`
3. Full pipeline executes: classify -> route -> assemble -> LLM call -> tool use -> response
4. File tools (read/write/edit) work via OPFS
5. Config persists across page reloads via OPFS
6. All existing native tests pass with default features (zero regressions)
7. WASM binary < 500KB gzipped
8. At least one LLM provider works from browser (Anthropic direct or Ollama local)
