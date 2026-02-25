# BrowserPlatform Implementation Specification

## Overview

`BrowserPlatform` implements the `Platform` trait for `wasm32-unknown-unknown`, providing browser-native implementations of HTTP, filesystem, and environment abstractions.

---

## BrowserHttpClient

**Implements**: `HttpClient` trait
**Technology**: `web-sys` fetch API (or `gloo-net` wrapper)

```rust
pub struct BrowserHttpClient;

#[async_trait(?Send)]
impl HttpClient for BrowserHttpClient {
    async fn request(&self, req: HttpRequest) -> Result<HttpResponse, HttpError> {
        // 1. Build web_sys::RequestInit with method, headers, body
        // 2. Call web_sys::window().fetch_with_request()
        // 3. Await JS Promise via wasm-bindgen-futures
        // 4. Read response status, headers, body text
        // 5. Map to HttpResponse
    }
}
```

**Special headers**:
- `anthropic-dangerous-direct-browser-access: true` for Anthropic API direct access
- Standard CORS headers handled by browser

**Streaming**: For SSE (LLM streaming responses):
- Use `Response.body()` -> `ReadableStream` -> `ReadableStreamDefaultReader`
- Process chunks via `reader.read()` in a loop
- Map to clawft's streaming interface

---

## BrowserFileSystem

**Implements**: `FileSystem` trait
**Technology**: OPFS (Origin Private File System) via `web-sys`

### Virtual Path Model

```
/                           -> OPFS root / "clawft" directory
/.clawft/config.json        -> Config stored in OPFS
/workspace/                 -> Agent working directory
/workspace/src/             -> User project files
/memory/                    -> Memory/context files
```

### Implementation

```rust
pub struct BrowserFileSystem {
    root_name: String,  // "clawft" -- OPFS subdirectory
}

#[async_trait(?Send)]
impl FileSystem for BrowserFileSystem {
    async fn read_to_string(&self, path: &Path) -> Result<String, FsError> {
        // 1. Navigate OPFS directories: root -> path components
        // 2. Get FileSystemFileHandle
        // 3. Call getFile() -> File -> text() -> String
    }

    async fn write_string(&self, path: &Path, content: &str) -> Result<(), FsError> {
        // 1. Navigate/create OPFS directories
        // 2. Get FileSystemFileHandle (create: true)
        // 3. createWritable() -> write(content) -> close()
    }

    async fn exists(&self, path: &Path) -> bool {
        // Try to get FileSystemFileHandle or DirectoryHandle
        // Return true if found, false on NotFoundError
    }

    async fn list_dir(&self, path: &Path) -> Result<Vec<String>, FsError> {
        // Get DirectoryHandle, iterate entries via keys()
    }

    async fn create_dir_all(&self, path: &Path) -> Result<(), FsError> {
        // Navigate path components, getDirectoryHandle(create: true) for each
    }

    async fn remove_file(&self, path: &Path) -> Result<(), FsError> {
        // Get parent DirectoryHandle, removeEntry(name)
    }

    async fn home_dir(&self) -> Option<PathBuf> {
        // Return virtual root: Some(PathBuf::from("/"))
    }
}
```

### Config Storage Pattern

Config maps to OPFS path `/.clawft/config.json`:
```
JS init(config_json) ->
  BrowserFileSystem.write_string("/.clawft/config.json", config_json) ->
  config_loader reads from BrowserFileSystem ->
  Config struct parsed via serde_json
```

### Persistence

- OPFS data persists across page reloads and browser restarts
- Request persistent storage via `navigator.storage.persist()`
- OPFS supported: Chrome 102+, Firefox 111+, Safari 15.2+

### Fallback: In-Memory FileSystem

For browsers without OPFS support, provide `InMemoryFileSystem`:
```rust
pub struct InMemoryFileSystem {
    files: RefCell<HashMap<PathBuf, String>>,
}
```
Data lost on page reload but functional for single sessions.

---

## BrowserEnvironment

**Implements**: `Environment` trait
**Technology**: In-memory `HashMap`

```rust
pub struct BrowserEnvironment {
    vars: RefCell<HashMap<String, String>>,
}

impl BrowserEnvironment {
    pub fn new() -> Self {
        Self { vars: RefCell::new(HashMap::new()) }
    }

    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self { vars: RefCell::new(vars) }
    }
}

impl Environment for BrowserEnvironment {
    fn get_var(&self, name: &str) -> Option<String> {
        self.vars.borrow().get(name).cloned()
    }

    fn set_var(&self, name: &str, value: &str) {
        self.vars.borrow_mut().insert(name.to_string(), value.to_string());
    }

    fn remove_var(&self, name: &str) {
        self.vars.borrow_mut().remove(name);
    }
}
```

Initialized from JavaScript:
```javascript
const wasm = await init();
wasm.set_env("CLAWFT_MODEL", "claude-sonnet-4-5-20250929");
wasm.set_env("CLAWFT_MAX_TOKENS", "4096");
```

---

## BrowserPlatform

```rust
pub struct BrowserPlatform {
    http: BrowserHttpClient,
    fs: BrowserFileSystem,
    env: BrowserEnvironment,
}

#[async_trait(?Send)]
impl Platform for BrowserPlatform {
    fn http(&self) -> &dyn HttpClient { &self.http }
    fn fs(&self) -> &dyn FileSystem { &self.fs }
    fn env(&self) -> &dyn Environment { &self.env }
    fn process(&self) -> Option<&dyn ProcessSpawner> { None }
}
```

---

## WASM Entry Points

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn init(config_json: &str) -> Result<(), JsValue> {
    // 1. Parse config
    // 2. Create BrowserPlatform
    // 3. Write config to OPFS
    // 4. Initialize AgentLoop<BrowserPlatform>
    // 5. Store in thread_local or static
}

#[wasm_bindgen]
pub async fn send_message(text: &str) -> Result<String, JsValue> {
    // 1. Get AgentLoop from static
    // 2. Call process_message(text)
    // 3. Return response text
}

#[wasm_bindgen]
pub fn set_env(key: &str, value: &str) {
    // Set environment variable on BrowserPlatform
}
```

---

## JS Integration Example

```javascript
import init, { init as clawftInit, send_message } from './clawft_wasm.js';

async function main() {
    await init();  // Load WASM module

    // Load config from IndexedDB (or defaults)
    const config = await loadConfigFromIndexedDB();

    // Encrypt API key before storage
    const encryptedKey = await encryptApiKey(config.apiKey);

    await clawftInit(JSON.stringify({
        defaults: {
            model: config.model || "claude-sonnet-4-5-20250929",
            max_tokens: config.maxTokens || 4096,
        },
        providers: {
            anthropic: {
                api_key: config.apiKey,  // Passed at runtime, not stored in WASM
                base_url: "https://api.anthropic.com",
            }
        },
        routing: config.routing || { strategy: "static" },
    }));

    // Chat loop
    const response = await send_message("Hello, explain Rust ownership");
    displayMessage(response);
}
```
