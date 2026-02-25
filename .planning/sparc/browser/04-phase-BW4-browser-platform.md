# Phase BW4: BrowserPlatform

**Phase ID**: BW4
**Workstream**: W-BROWSER
**Duration**: Week 4-5
**Depends On**: BW1 (trait definitions), BW3 (CORS config types)
**Goal**: Implement the full `Platform` trait for browser with `BrowserHttpClient`, `BrowserFileSystem`, and `BrowserEnvironment`

---

## S -- Specification

### What Changes

This phase implements three browser-native platform components and bundles them into a `BrowserPlatform` struct that satisfies the `Platform` trait. The implementations use browser APIs via `web-sys`: the fetch API for HTTP, the Origin Private File System (OPFS) for filesystem operations, and an in-memory `HashMap` for environment variables.

### New Files

| File | Contents |
|---|---|
| `crates/clawft-platform/src/browser/mod.rs` | `BrowserPlatform` struct + `Platform` impl |
| `crates/clawft-platform/src/browser/http.rs` | `BrowserHttpClient` implementing `HttpClient` |
| `crates/clawft-platform/src/browser/fs.rs` | `BrowserFileSystem` implementing `FileSystem` (OPFS) |
| `crates/clawft-platform/src/browser/env.rs` | `BrowserEnvironment` implementing `Environment` |
| `crates/clawft-platform/src/browser/config.rs` | Config initialization from JS `init()` |

### Module Registration

Add to `crates/clawft-platform/src/lib.rs`:

```rust
#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "browser")]
pub use browser::BrowserPlatform;
```

### BrowserHttpClient Specification

- **Implements**: `HttpClient` trait from `crates/clawft-platform/src/http.rs`
- **Technology**: `web-sys` fetch API (via `web_sys::window().fetch_with_request()`)
- **CORS handling**: URL resolution uses `ProviderConfig::cors_proxy` from BW3
- **Streaming**: `Response.body()` -> `ReadableStream` -> chunk-by-chunk processing
- **Special headers**: Anthropic `browser_direct` header injection
- **Error mapping**: JS exceptions mapped to `Box<dyn Error + Send + Sync>`

### BrowserFileSystem Specification

- **Implements**: `FileSystem` trait from `crates/clawft-platform/src/fs.rs`
- **Technology**: OPFS (Origin Private File System) via `web-sys`
- **web-sys features needed**: `FileSystemDirectoryHandle`, `FileSystemFileHandle`, `FileSystemWritableFileStream`, `StorageManager`
- **Virtual path model**: All paths are relative to an OPFS root directory called `"clawft"`
- **Directory navigation**: Path components walked via `getDirectoryHandle()` with `create: true`
- **File read**: `getFileHandle()` -> `getFile()` -> `text()` -> `String`
- **File write**: `getFileHandle(create: true)` -> `createWritable()` -> `write()` -> `close()`
- **Persistence**: OPFS data persists across page reloads and browser restarts
- **Fallback**: `InMemoryFileSystem` for browsers without OPFS support (data lost on reload)

### BrowserEnvironment Specification

- **Implements**: `Environment` trait from `crates/clawft-platform/src/env.rs`
- **Technology**: `RefCell<HashMap<String, String>>`
- **Thread safety**: WASM is single-threaded, so `RefCell` is safe (no `Mutex` needed)
- **Initialization**: Populated from JavaScript via `set_env()` calls before `init()`
- **Pattern**: Identical to existing `WasiEnvironment` in `crates/clawft-wasm/src/env.rs`

### OPFS Browser Support Matrix

| Browser | Version | OPFS Support | Notes |
|---|---|---|---|
| Chrome | 102+ | Full | Includes `createSyncAccessHandle` in Workers |
| Firefox | 111+ | Full | |
| Safari | 15.2+ | Full | Since iOS 15.2 |
| Edge | 102+ | Full | Chromium-based |

For unsupported browsers, `BrowserFileSystem::new()` detects OPFS availability and falls back to `InMemoryFileSystem`.

---

## P -- Pseudocode

### BrowserHttpClient

```rust
// crates/clawft-platform/src/browser/http.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response, Headers};
use std::collections::HashMap;

use crate::http::{HttpClient, HttpResponse};

pub struct BrowserHttpClient;

impl BrowserHttpClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait(?Send)]
impl HttpClient for BrowserHttpClient {
    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        // 1. Build RequestInit
        let mut opts = RequestInit::new();
        opts.method(method);
        opts.mode(RequestMode::Cors);

        // 2. Set body if present
        if let Some(body_bytes) = body {
            let uint8_array = js_sys::Uint8Array::from(body_bytes);
            opts.body(Some(&uint8_array));
        }

        // 3. Build Request with URL
        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| format!("failed to create request: {:?}", e))?;

        // 4. Set headers
        let req_headers = request.headers();
        for (key, value) in headers {
            req_headers.set(key, value)
                .map_err(|e| format!("failed to set header {}: {:?}", key, e))?;
        }

        // 5. Execute fetch
        let window = web_sys::window()
            .ok_or("no global window object")?;
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| format!("fetch failed: {:?}", e))?;

        let resp: Response = resp_value.dyn_into()
            .map_err(|_| "response is not a Response object")?;

        // 6. Read response
        let status = resp.status();
        let resp_headers = resp.headers();

        // Convert Headers to HashMap (iterate via entries())
        let mut header_map = HashMap::new();
        // Note: web_sys Headers iteration requires JsIterator
        // For simplicity, extract commonly needed headers:
        if let Ok(ct) = resp_headers.get("content-type") {
            if let Some(ct) = ct {
                header_map.insert("content-type".to_string(), ct);
            }
        }

        // 7. Read body
        let array_buffer = JsFuture::from(
            resp.array_buffer()
                .map_err(|e| format!("failed to get array_buffer: {:?}", e))?
        ).await
        .map_err(|e| format!("failed to read body: {:?}", e))?;

        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let body_bytes = uint8_array.to_vec();

        Ok(HttpResponse {
            status,
            headers: header_map,
            body: body_bytes,
        })
    }
}
```

### BrowserFileSystem (OPFS)

```rust
// crates/clawft-platform/src/browser/fs.rs

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use std::path::{Path, PathBuf};
use crate::fs::FileSystem;

pub struct BrowserFileSystem {
    root_name: String,
}

impl BrowserFileSystem {
    pub fn new() -> Self {
        Self {
            root_name: "clawft".to_string(),
        }
    }

    /// Get the OPFS root directory handle.
    async fn get_opfs_root(&self) -> Result<web_sys::FileSystemDirectoryHandle, std::io::Error> {
        let navigator = web_sys::window()
            .ok_or_else(|| std::io::Error::other("no window"))?
            .navigator();
        let storage = navigator.storage();

        let root = JsFuture::from(storage.get_directory())
            .await
            .map_err(|e| std::io::Error::other(format!("OPFS not available: {:?}", e)))?;

        let root: web_sys::FileSystemDirectoryHandle = root.dyn_into()
            .map_err(|_| std::io::Error::other("not a directory handle"))?;

        // Navigate to our app's subdirectory
        let opts = web_sys::FileSystemGetDirectoryOptions::new();
        opts.set_create(true);

        let app_dir = JsFuture::from(
            root.get_directory_handle_with_options(&self.root_name, &opts)
        ).await
        .map_err(|e| std::io::Error::other(format!("failed to get app dir: {:?}", e)))?;

        Ok(app_dir.dyn_into().unwrap())
    }

    /// Navigate to a directory by walking path components.
    /// Creates intermediate directories if `create` is true.
    async fn navigate_to_dir(
        &self,
        root: &web_sys::FileSystemDirectoryHandle,
        dir_path: &Path,
        create: bool,
    ) -> Result<web_sys::FileSystemDirectoryHandle, std::io::Error> {
        let mut current = root.clone();

        for component in dir_path.components() {
            let name = component.as_os_str().to_string_lossy();
            if name == "/" || name == "." {
                continue;
            }

            let opts = web_sys::FileSystemGetDirectoryOptions::new();
            opts.set_create(create);

            current = JsFuture::from(
                current.get_directory_handle_with_options(&name, &opts)
            ).await
            .map_err(|e| {
                if create {
                    std::io::Error::other(format!("failed to create dir '{}': {:?}", name, e))
                } else {
                    std::io::Error::new(std::io::ErrorKind::NotFound, format!("dir '{}' not found", name))
                }
            })?
            .dyn_into()
            .map_err(|_| std::io::Error::other("not a directory handle"))?;
        }

        Ok(current)
    }
}

#[async_trait::async_trait(?Send)]
impl FileSystem for BrowserFileSystem {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        let root = self.get_opfs_root().await?;

        // Navigate to parent directory
        let parent = path.parent().unwrap_or(Path::new(""));
        let dir = self.navigate_to_dir(&root, parent, false).await?;

        // Get file handle
        let file_name = path.file_name()
            .ok_or_else(|| std::io::Error::other("no file name"))?
            .to_string_lossy();

        let file_handle = JsFuture::from(dir.get_file_handle(&file_name))
            .await
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"))?;

        let file_handle: web_sys::FileSystemFileHandle = file_handle.dyn_into().unwrap();

        // Get File object
        let file = JsFuture::from(file_handle.get_file())
            .await
            .map_err(|e| std::io::Error::other(format!("getFile failed: {:?}", e)))?;

        let file: web_sys::File = file.dyn_into().unwrap();

        // Read text
        let text = JsFuture::from(file.text())
            .await
            .map_err(|e| std::io::Error::other(format!("text() failed: {:?}", e)))?;

        Ok(text.as_string().unwrap_or_default())
    }

    async fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let root = self.get_opfs_root().await?;

        // Navigate to parent directory (create if needed)
        let parent = path.parent().unwrap_or(Path::new(""));
        let dir = self.navigate_to_dir(&root, parent, true).await?;

        // Get or create file handle
        let file_name = path.file_name()
            .ok_or_else(|| std::io::Error::other("no file name"))?
            .to_string_lossy();

        let opts = web_sys::FileSystemGetFileOptions::new();
        opts.set_create(true);

        let file_handle = JsFuture::from(
            dir.get_file_handle_with_options(&file_name, &opts)
        ).await
        .map_err(|e| std::io::Error::other(format!("create file failed: {:?}", e)))?;

        let file_handle: web_sys::FileSystemFileHandle = file_handle.dyn_into().unwrap();

        // Create writable stream
        let writable = JsFuture::from(file_handle.create_writable())
            .await
            .map_err(|e| std::io::Error::other(format!("createWritable failed: {:?}", e)))?;

        let writable: web_sys::FileSystemWritableFileStream = writable.dyn_into().unwrap();

        // Write content
        JsFuture::from(writable.write_with_str(content))
            .await
            .map_err(|e| std::io::Error::other(format!("write failed: {:?}", e)))?;

        // Close
        JsFuture::from(writable.close())
            .await
            .map_err(|e| std::io::Error::other(format!("close failed: {:?}", e)))?;

        Ok(())
    }

    async fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        // OPFS doesn't have a native append. Read + concatenate + write.
        let existing = match self.read_to_string(path).await {
            Ok(s) => s,
            Err(_) => String::new(),
        };
        self.write_string(path, &format!("{}{}", existing, content)).await
    }

    async fn exists(&self, path: &Path) -> bool {
        // Try to get the handle; NotFoundError means it doesn't exist
        let root = match self.get_opfs_root().await {
            Ok(r) => r,
            Err(_) => return false,
        };

        let parent = path.parent().unwrap_or(Path::new(""));
        let dir = match self.navigate_to_dir(&root, parent, false).await {
            Ok(d) => d,
            Err(_) => return false,
        };

        if let Some(file_name) = path.file_name() {
            let name = file_name.to_string_lossy();
            // Try as file first, then as directory
            JsFuture::from(dir.get_file_handle(&name)).await.is_ok()
                || JsFuture::from(dir.get_directory_handle(&name)).await.is_ok()
        } else {
            true // Root path exists
        }
    }

    async fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let root = self.get_opfs_root().await?;
        let dir = self.navigate_to_dir(&root, path, false).await?;

        let mut entries = Vec::new();

        // OPFS directory iteration via keys()
        let keys = js_sys::Reflect::get(&dir, &JsValue::from_str("keys"))
            .map_err(|_| std::io::Error::other("no keys() method"))?;
        let iterator = js_sys::Function::from(keys)
            .call0(&dir)
            .map_err(|_| std::io::Error::other("keys() call failed"))?;

        loop {
            let next = js_sys::Reflect::get(&iterator, &JsValue::from_str("next"))
                .and_then(|f| js_sys::Function::from(f).call0(&iterator))
                .map_err(|_| std::io::Error::other("iterator next failed"))?;

            let done = js_sys::Reflect::get(&next, &JsValue::from_str("done"))
                .map(|v| v.as_bool().unwrap_or(true))
                .unwrap_or(true);

            if done { break; }

            let value = js_sys::Reflect::get(&next, &JsValue::from_str("value"))
                .ok()
                .and_then(|v| v.as_string());

            if let Some(name) = value {
                entries.push(path.join(&name));
            }
        }

        Ok(entries)
    }

    async fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        let root = self.get_opfs_root().await?;
        self.navigate_to_dir(&root, path, true).await?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        let root = self.get_opfs_root().await?;
        let parent = path.parent().unwrap_or(Path::new(""));
        let dir = self.navigate_to_dir(&root, parent, false).await?;

        let file_name = path.file_name()
            .ok_or_else(|| std::io::Error::other("no file name"))?
            .to_string_lossy();

        JsFuture::from(dir.remove_entry(&file_name))
            .await
            .map_err(|e| std::io::Error::other(format!("remove failed: {:?}", e)))?;

        Ok(())
    }

    fn home_dir(&self) -> Option<PathBuf> {
        // Browser has no "home directory" concept.
        // Return virtual root so ~/... paths resolve to /...
        Some(PathBuf::from("/"))
    }
}
```

### BrowserEnvironment

```rust
// crates/clawft-platform/src/browser/env.rs

use std::cell::RefCell;
use std::collections::HashMap;
use crate::env::Environment;

pub struct BrowserEnvironment {
    vars: RefCell<HashMap<String, String>>,
}

impl BrowserEnvironment {
    pub fn new() -> Self {
        Self {
            vars: RefCell::new(HashMap::new()),
        }
    }

    pub fn with_vars(vars: HashMap<String, String>) -> Self {
        Self {
            vars: RefCell::new(vars),
        }
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

### BrowserPlatform Bundle

```rust
// crates/clawft-platform/src/browser/mod.rs

pub mod config;
pub mod env;
pub mod fs;
pub mod http;

use crate::{Platform, env::Environment, fs::FileSystem, http::HttpClient, process::ProcessSpawner};

pub struct BrowserPlatform {
    http: http::BrowserHttpClient,
    fs: fs::BrowserFileSystem,
    env: env::BrowserEnvironment,
}

impl BrowserPlatform {
    pub fn new() -> Self {
        Self {
            http: http::BrowserHttpClient::new(),
            fs: fs::BrowserFileSystem::new(),
            env: env::BrowserEnvironment::new(),
        }
    }

    pub fn with_env(env_vars: std::collections::HashMap<String, String>) -> Self {
        Self {
            http: http::BrowserHttpClient::new(),
            fs: fs::BrowserFileSystem::new(),
            env: env::BrowserEnvironment::with_vars(env_vars),
        }
    }

    /// Get a mutable reference to the environment for JS interop.
    pub fn env_mut(&self) -> &env::BrowserEnvironment {
        &self.env
    }
}

#[async_trait::async_trait(?Send)]
impl Platform for BrowserPlatform {
    fn http(&self) -> &dyn HttpClient {
        &self.http
    }

    fn fs(&self) -> &dyn FileSystem {
        &self.fs
    }

    fn env(&self) -> &dyn Environment {
        &self.env
    }

    fn process(&self) -> Option<&dyn ProcessSpawner> {
        None // No process spawning in browser
    }
}
```

---

## A -- Architecture

### Virtual Path Model

```
OPFS Root (navigator.storage.getDirectory())
    |
    +-- clawft/                          <- BrowserFileSystem root
        |
        +-- .clawft/
        |   +-- config.json              <- Stored by init()
        |
        +-- workspace/                   <- Agent working directory
        |   +-- src/
        |   +-- docs/
        |
        +-- sessions/                    <- Session persistence
        |   +-- cli_chat1.json
        |
        +-- memory/
            +-- MEMORY.md
            +-- HISTORY.md
```

All paths are relative to the OPFS `clawft/` directory. The `home_dir()` returns `Some(PathBuf::from("/"))` so that `~/.clawft/config.json` resolves to `/.clawft/config.json` which maps to `clawft/.clawft/config.json` in OPFS.

### Config from JS init()

```
JS: init('{"providers": {"anthropic": {"api_key": "sk-ant-..."}}}')
    |
    v
WASM: parse JSON -> Config struct
    |
    v
BrowserFileSystem.write_string("/.clawft/config.json", config_json)
    |
    v
config_loader::load_config_raw() reads from BrowserFileSystem
    |
    v
Config struct parsed via serde_json
    |
    v
AgentLoop<BrowserPlatform>::new(config, ...) initialized
```

### In-Memory Fallback

```rust
// crates/clawft-platform/src/browser/fs.rs

/// In-memory filesystem fallback for browsers without OPFS.
pub struct InMemoryFileSystem {
    files: RefCell<HashMap<PathBuf, String>>,
}

#[async_trait::async_trait(?Send)]
impl FileSystem for InMemoryFileSystem {
    async fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        self.files.borrow().get(path)
            .cloned()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
    }
    // ... similar simple implementations
}
```

---

## R -- Refinement

### OPFS API Caveats

1. **Async iteration**: OPFS directory `entries()` returns an `AsyncIterator`. The `web-sys` bindings may not fully support async iteration. Use `keys()` with manual `next()` calls instead.

2. **No symlinks**: OPFS has no symlink concept. This is actually simpler for our use case -- `canonicalize()` becomes simple path normalization.

3. **Storage quotas**: Browsers limit OPFS storage (typically 10-60% of available disk). Request persistent storage via `navigator.storage.persist()` to avoid automatic eviction.

4. **Worker vs Window**: OPFS is available in both `Window` and `Worker` contexts. The code should check for `window` first, then `WorkerGlobalScope`.

### Thread Safety

`BrowserEnvironment` uses `RefCell<HashMap>` instead of `Mutex<HashMap>`. This is safe because WASM is single-threaded. However, `RefCell` is `!Send`, which is why the `Platform` trait must use `async_trait(?Send)` on browser builds (already handled in BW1).

### Testing Strategy

Browser platform implementations cannot be tested with standard `cargo test` (they need a browser runtime). Testing approaches:

1. **Compilation check**: `cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser`
2. **wasm-pack test**: `wasm-pack test --headless --chrome crates/clawft-platform` (requires Chrome)
3. **Manual testing**: Load WASM in browser, run operations via JS console
4. **Mock testing**: Test logic with mock filesystem in native tests

---

## C -- Completion

### Exit Criteria

- [ ] `BrowserPlatform` implements `Platform` trait
- [ ] `BrowserHttpClient` implements `HttpClient` via `web-sys` fetch
- [ ] `BrowserFileSystem` implements `FileSystem` via OPFS
- [ ] `BrowserEnvironment` implements `Environment` via `RefCell<HashMap>`
- [ ] `InMemoryFileSystem` fallback exists
- [ ] `cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser` passes
- [ ] Config storage/retrieval via OPFS works (verified in BW6)
- [ ] `cargo test --workspace` -- zero regressions
- [ ] `cargo build --release --bin weft` -- native CLI builds

### Test Commands

```bash
# Browser WASM check
cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser

# Native regression
cargo test --workspace
cargo test -p clawft-platform

# Dependency tree
cargo tree -p clawft-platform --no-default-features --features browser
# Should show web-sys, js-sys, wasm-bindgen, NO tokio/reqwest/dirs
```

### Phase Gate

```bash
#!/bin/bash
set -euo pipefail

echo "=== Gate 1: Native tests ==="
cargo test --workspace

echo "=== Gate 2: Native CLI build ==="
cargo build --release --bin weft

echo "=== Gate 3: WASI WASM build ==="
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm

echo "=== Gate 4: Browser platform compiles ==="
cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser

echo "BW4 phase gate PASSED"
```
