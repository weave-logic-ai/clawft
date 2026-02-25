# Phase BW5: WASM Entry Point + Tools

**Phase ID**: BW5
**Workstream**: W-BROWSER
**Duration**: Week 5-6
**Depends On**: BW2 (Core Engine), BW3 (LLM Transport), BW4 (BrowserPlatform)
**Goal**: Wire the real `AgentLoop<BrowserPlatform>` via `wasm-bindgen` exports, fix platform leaks in tools, audit binary size

---

## S -- Specification

### What Changes

This phase connects all previous work. The `clawft-wasm` crate is updated to depend on `clawft-core`, `clawft-llm`, `clawft-tools`, and `clawft-platform` with the `browser` feature, replacing the current stub implementations with the real agent loop. Two platform abstraction leaks in `clawft-tools` are fixed. The WASM module exposes `init()`, `send_message()`, and `set_env()` via `wasm-bindgen`.

### Files to Change

| File | Change | Task |
|---|---|---|
| `crates/clawft-tools/src/file_tools.rs:374` | Replace `tokio::fs::metadata()` with Platform fs method | P5.1 |
| `crates/clawft-tools/src/file_tools.rs` | Replace `canonicalize()` with virtual path normalization | P5.2 |
| `crates/clawft-tools/Cargo.toml` | Add `native`/`browser` features | P5.1 |
| `crates/clawft-wasm/Cargo.toml` | Add `browser` feature with deps on core/llm/tools/platform | P5.4 |
| `crates/clawft-wasm/src/lib.rs` | Replace stubs with real `AgentLoop<BrowserPlatform>` | P5.4, P5.5 |
| `crates/clawft-wasm/src/tools.rs` | New: browser-safe tool registry | P5.3 |
| `docs/browser/building.md` | New: browser build guide | P5.8 |
| `docs/browser/quickstart.md` | New: browser quickstart | P5.9 |
| `docs/browser/api-reference.md` | New: wasm-bindgen API reference | P5.10 |

### Fix: tokio::fs::metadata Leak

**File**: `crates/clawft-tools/src/file_tools.rs`, line 374

Current code:
```rust
let metadata = tokio::fs::metadata(entry_path).await;
let (is_dir, size) = match metadata {
    Ok(m) => (m.is_dir(), m.len()),
    Err(_) => (false, 0),
};
```

This directly uses `tokio::fs::metadata` bypassing the Platform `fs` trait. Fix options:

**Option A**: Add `metadata()` method to `FileSystem` trait (preferred)
```rust
// In crates/clawft-platform/src/fs.rs, add to FileSystem trait:
async fn is_dir(&self, path: &Path) -> bool;
async fn file_size(&self, path: &Path) -> Option<u64>;
```

**Option B**: Use `exists()` + heuristic (simpler, less accurate)
```rust
let is_dir = self.platform.fs().exists(&entry_path.join(".")).await
    || self.platform.fs().list_dir(&entry_path).await.is_ok();
let size = 0; // Size not available without metadata
```

Recommendation: Option A. Add `is_dir()` and `file_size()` to the `FileSystem` trait with default implementations that return `false` and `None`. The native implementation uses `tokio::fs::metadata`; the browser implementation uses OPFS file handle type detection.

### Fix: canonicalize() for Browser

**File**: `crates/clawft-tools/src/file_tools.rs` (sandbox validation)

The file tools use `std::fs::canonicalize()` to resolve symlinks and validate sandbox paths. This is unavailable in WASM. Replace with:

```rust
/// Normalize a path without filesystem access.
/// Resolves `.`, `..`, removes trailing slashes.
/// OPFS has no symlinks, so this is sufficient for browser.
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => { components.pop(); }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

// Feature-gated path resolution:
#[cfg(feature = "native")]
fn resolve_path(path: &Path) -> std::io::Result<PathBuf> {
    std::fs::canonicalize(path)
}

#[cfg(feature = "browser")]
fn resolve_path(path: &Path) -> std::io::Result<PathBuf> {
    Ok(normalize_path(path))
}
```

### Exact Cargo.toml Changes

#### `crates/clawft-tools/Cargo.toml`

Current:
```toml
[features]
default = ["native-exec"]
native-exec = []
vector-memory = ["clawft-core/vector-memory"]
delegate = ["clawft-services/delegate"]

[dependencies]
clawft-types = { workspace = true }
clawft-platform = { workspace = true }
clawft-core = { workspace = true }
clawft-services = { workspace = true, optional = true }
async-trait = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
url = "2"
ipnet = "2"
```

New:
```toml
[features]
default = ["native-exec", "native"]
native-exec = []
native = [
    "dep:tokio",
    "clawft-platform/native",
    "clawft-core/native",
    "clawft-types/native",
]
browser = [
    "clawft-platform/browser",
    "clawft-core/browser",
]
vector-memory = ["clawft-core/vector-memory"]
delegate = ["clawft-services/delegate"]

[dependencies]
clawft-types = { workspace = true, default-features = false }
clawft-platform = { workspace = true, default-features = false }
clawft-core = { workspace = true, default-features = false }
clawft-services = { workspace = true, optional = true }
async-trait = { workspace = true }
tracing = { workspace = true }
thiserror = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
url = "2"
ipnet = "2"

# Native only
tokio = { workspace = true, optional = true }
```

#### `crates/clawft-wasm/Cargo.toml`

Current (key sections):
```toml
[features]
default = []
wasm-plugins = [...]
```

New (add browser feature):
```toml
[features]
default = []
browser = [
    "dep:clawft-core",
    "dep:clawft-llm",
    "dep:clawft-tools",
    "dep:clawft-platform",
    "dep:clawft-security",
    "dep:wasm-bindgen",
    "dep:wasm-bindgen-futures",
    "dep:web-sys",
    "dep:js-sys",
    "clawft-core/browser",
    "clawft-llm/browser",
    "clawft-tools/browser",
    "clawft-platform/browser",
]
wasm-plugins = [...]

[dependencies]
clawft-types = { workspace = true, default-features = false }
serde = { workspace = true }
serde_json = { workspace = true }

# Browser feature deps
clawft-core = { workspace = true, default-features = false, optional = true }
clawft-llm = { workspace = true, default-features = false, optional = true }
clawft-tools = { workspace = true, default-features = false, optional = true }
clawft-platform = { workspace = true, default-features = false, optional = true }
clawft-security = { workspace = true, optional = true }
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }
web-sys = { version = "0.3", optional = true, features = ["console"] }
js-sys = { version = "0.3", optional = true }

# Existing deps stay for wasm-plugins and default features
# ...
```

---

## P -- Pseudocode

### wasm-bindgen Entry Points

```rust
// crates/clawft-wasm/src/lib.rs (browser feature section)

#[cfg(feature = "browser")]
mod browser_entry {
    use wasm_bindgen::prelude::*;
    use std::cell::RefCell;
    use std::sync::Arc;

    use clawft_platform::BrowserPlatform;
    use clawft_core::agent::loop_core::AgentLoop;
    use clawft_core::bus::MessageBus;
    use clawft_core::pipeline::traits::PipelineRegistry;
    use clawft_core::session::SessionManager;
    use clawft_types::config::Config;
    use clawft_types::event::{InboundMessage, OutboundMessage};

    // Thread-local storage for the initialized agent (WASM is single-threaded)
    thread_local! {
        static PLATFORM: RefCell<Option<Arc<BrowserPlatform>>> = RefCell::new(None);
        static BUS: RefCell<Option<Arc<MessageBus>>> = RefCell::new(None);
        static AGENT: RefCell<Option<AgentLoop<BrowserPlatform>>> = RefCell::new(None);
    }

    /// Initialize the WASM agent with a JSON configuration string.
    ///
    /// Must be called before `send_message()`. The config JSON follows
    /// the same schema as `~/.clawft/config.json` on native.
    ///
    /// # Arguments
    ///
    /// * `config_json` - JSON string with provider credentials, model settings, etc.
    ///
    /// # Example (JS)
    ///
    /// ```javascript
    /// await init(JSON.stringify({
    ///     providers: { anthropic: { api_key: "sk-ant-..." } },
    ///     agents: { defaults: { model: "claude-sonnet-4-5-20250929" } }
    /// }));
    /// ```
    #[wasm_bindgen]
    pub async fn init(config_json: &str) -> Result<(), JsValue> {
        // Set up console_error_panic_hook for better error messages
        console_error_panic_hook::set_once();

        // 1. Parse config
        let config: Config = serde_json::from_str(config_json)
            .map_err(|e| JsValue::from_str(&format!("config parse error: {}", e)))?;

        // 2. Create BrowserPlatform
        let platform = Arc::new(BrowserPlatform::new());

        // 3. Write config to OPFS for persistence
        platform.fs()
            .write_string(
                &std::path::PathBuf::from("/.clawft/config.json"),
                config_json,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("config write error: {}", e)))?;

        // 4. Create message bus
        let bus = Arc::new(MessageBus::new());

        // 5. Set up pipeline
        let pipeline = super::tools::build_browser_pipeline(&config, &platform)
            .map_err(|e| JsValue::from_str(&format!("pipeline error: {}", e)))?;

        // 6. Set up tool registry (browser-safe subset)
        let tools = super::tools::register_browser_tools(&platform);

        // 7. Create session manager
        let sessions = SessionManager::with_dir(
            platform.clone(),
            std::path::PathBuf::from("/.clawft/sessions"),
        );

        // 8. Create context builder
        let memory = Arc::new(clawft_core::agent::memory::MemoryStore::with_paths(
            std::path::PathBuf::from("/.clawft/memory/MEMORY.md"),
            std::path::PathBuf::from("/.clawft/memory/HISTORY.md"),
            platform.clone(),
        ));
        let skills = Arc::new(clawft_core::agent::skills::SkillsLoader::with_dir(
            std::path::PathBuf::from("/.clawft/skills"),
            platform.clone(),
        ));
        let context = clawft_core::agent::context::ContextBuilder::new(
            config.agents.clone(),
            memory,
            skills,
            platform.clone(),
        );

        // 9. Create agent loop
        let permission_resolver = clawft_core::pipeline::permissions::PermissionResolver::from_config(
            &config.routing,
        );

        let agent = AgentLoop::new(
            config.agents,
            platform.clone(),
            bus.clone(),
            pipeline,
            tools,
            context,
            sessions,
            permission_resolver,
        );

        // 10. Store in thread-local
        PLATFORM.with(|p| *p.borrow_mut() = Some(platform));
        BUS.with(|b| *b.borrow_mut() = Some(bus));
        AGENT.with(|a| *a.borrow_mut() = Some(agent));

        web_sys::console::log_1(&"clawft-wasm initialized".into());
        Ok(())
    }

    /// Send a message through the agent pipeline and get the response.
    ///
    /// The message is processed through the full 6-stage pipeline:
    /// classify -> route -> assemble -> LLM transport -> score -> learn
    ///
    /// Tool calls are executed automatically and the final text response
    /// is returned.
    ///
    /// # Arguments
    ///
    /// * `text` - The user message text.
    ///
    /// # Returns
    ///
    /// A Promise that resolves to the agent's response text.
    #[wasm_bindgen]
    pub async fn send_message(text: &str) -> Result<String, JsValue> {
        let bus = BUS.with(|b| b.borrow().clone())
            .ok_or_else(|| JsValue::from_str("not initialized: call init() first"))?;

        let agent = AGENT.with(|a| {
            // We need to borrow the agent to call process_message
            // This is safe because WASM is single-threaded
            a.borrow()
        });

        // Publish inbound message
        let inbound = InboundMessage {
            channel: "browser".into(),
            sender_id: "browser-user".into(),
            chat_id: "browser-chat".into(),
            content: text.to_string(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: Default::default(),
        };

        bus.publish_inbound(inbound)
            .map_err(|e| JsValue::from_str(&format!("publish error: {}", e)))?;

        // Consume and process
        let msg = bus.consume_inbound().await
            .ok_or_else(|| JsValue::from_str("no message on bus"))?;

        // Process through pipeline
        // Note: agent is borrowed immutably, process_message takes &self
        if let Some(ref agent) = *agent {
            // This is the tricky part -- AgentLoop::process_message is private.
            // We need to either:
            // a) Make it pub(crate) and add a public wrapper
            // b) Use the run() method with a single message
            // c) Add a public process_single() method

            // For now, use the bus pattern:
            // The agent processes the message and puts response on outbound bus
            drop(agent); // Release borrow before awaiting

            // ... process message pattern
        }

        // Consume outbound response
        let response = bus.consume_outbound().await
            .ok_or_else(|| JsValue::from_str("no response generated"))?;

        Ok(response.content)
    }

    /// Set an environment variable on the BrowserPlatform.
    ///
    /// Call this before `init()` to configure environment variables,
    /// or after `init()` to update them dynamically.
    #[wasm_bindgen]
    pub fn set_env(key: &str, value: &str) {
        PLATFORM.with(|p| {
            if let Some(ref platform) = *p.borrow() {
                platform.env().set_var(key, value);
            }
        });
    }
}

#[cfg(feature = "browser")]
pub use browser_entry::*;
```

### Browser Tool Registry

```rust
// crates/clawft-wasm/src/tools.rs

#[cfg(feature = "browser")]
use clawft_core::tools::registry::ToolRegistry;
#[cfg(feature = "browser")]
use clawft_platform::BrowserPlatform;
#[cfg(feature = "browser")]
use std::sync::Arc;

#[cfg(feature = "browser")]
pub fn register_browser_tools(platform: &Arc<BrowserPlatform>) -> ToolRegistry {
    use clawft_tools::file_tools::{ReadFileTool, WriteFileTool, EditFileTool, ListDirectoryTool};
    // use clawft_tools::web_tools::{WebSearchTool, WebFetchTool};
    // use clawft_tools::memory_tools::{MemoryReadTool, MemoryWriteTool};
    // use clawft_tools::message_tool::MessageTool;

    let mut registry = ToolRegistry::new();

    // File tools (use BrowserFileSystem via Platform trait)
    registry.register(Arc::new(ReadFileTool::new(platform.clone())));
    registry.register(Arc::new(WriteFileTool::new(platform.clone())));
    registry.register(Arc::new(EditFileTool::new(platform.clone())));
    registry.register(Arc::new(ListDirectoryTool::new(platform.clone())));

    // Web tools (use BrowserHttpClient via Platform trait)
    // registry.register(Arc::new(WebSearchTool::new(platform.clone())));
    // registry.register(Arc::new(WebFetchTool::new(platform.clone())));

    // Memory tools
    // registry.register(Arc::new(MemoryReadTool::new(platform.clone())));
    // registry.register(Arc::new(MemoryWriteTool::new(platform.clone())));

    // Excluded: ShellExecTool, SpawnTool (already behind native-exec feature)
    // Excluded: DelegateTaskTool (behind delegate feature)

    registry
}
```

---

## A -- Architecture

### How AgentLoop<BrowserPlatform> Is Constructed and Stored

```
JavaScript                          WASM Module
    |                                   |
    | init(config_json) ------>         |
    |                              1. Parse config JSON
    |                              2. Create BrowserPlatform
    |                                  +-- BrowserHttpClient (fetch API)
    |                                  +-- BrowserFileSystem (OPFS)
    |                                  +-- BrowserEnvironment (HashMap)
    |                              3. Write config to OPFS
    |                              4. Build pipeline (classifier, router, transport, etc.)
    |                              5. Register browser-safe tools
    |                              6. Create AgentLoop<BrowserPlatform>
    |                              7. Store in thread_local! (single-threaded WASM)
    |                                   |
    | send_message(text) ------>        |
    |                              1. Create InboundMessage
    |                              2. AgentLoop.process_single(msg)
    |                              3. Pipeline: classify -> route -> assemble -> LLM -> score
    |                              4. Tool loop: execute tools, re-invoke LLM
    |                              5. Return final text response
    |                                   |
    | <------ response text             |
```

### Binary Size Budget

**Target**: < 500KB gzipped

Expected size breakdown:
| Component | Estimated Size (uncompressed) |
|---|---|
| `serde` + `serde_json` | ~150KB |
| `reqwest` (wasm) | ~100KB |
| `clawft-core` (pipeline) | ~200KB |
| `clawft-types` | ~100KB |
| `clawft-tools` (browser subset) | ~50KB |
| `web-sys` bindings | ~100KB |
| `wasm-bindgen` glue | ~50KB |
| `serde_yaml` | ~200KB |
| Other | ~50KB |
| **Total (uncompressed)** | **~1000KB** |
| **Gzipped (estimated 50% ratio)** | **~500KB** |

### Size Optimization Strategies

If over budget:
1. **Disable `serde_yaml`**: Save ~200KB. Browser config is always JSON, never YAML. Gate `serde_yaml` behind `native` feature in clawft-core.
2. **`wasm-opt -Oz`**: Typically saves 10-20% on WASM binaries.
3. **`lto = true` + `opt-level = "z"`**: Already in `profile.release-wasm`.
4. **Audit `web-sys` features**: Only include features actually used (already minimal).
5. **Strip debug info**: `strip = true` in release profile (already set).

---

## R -- Refinement

### Regression Protection

This is the **most critical phase gate**. The real `AgentLoop` now runs in WASM, which means changes to `clawft-core`, `clawft-llm`, or `clawft-tools` could break the browser build.

**Mandatory checks after BW5**:
1. `cargo test --workspace` -- all native tests pass
2. `cargo build --release --bin weft` -- native CLI binary builds
3. `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` -- existing WASI build
4. `wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser` -- browser WASM

### Edge Cases

1. **`process_message` visibility**: `AgentLoop::process_message()` is currently `async fn process_message(&self, msg: InboundMessage) -> Result<()>` (private). Need to either:
   - Make it `pub` or add a `pub async fn process_single(...)` wrapper
   - Use the bus pattern: publish inbound, trigger processing, consume outbound

2. **Thread-local agent lifetime**: The `AgentLoop` is stored in `thread_local!`. It must not hold any references that outlive the WASM module. All data is owned (`Arc`, `String`, etc.), so this should be safe.

3. **Concurrent messages**: WASM is single-threaded, so only one message processes at a time. This is fine for browser usage (one user, sequential messages).

4. **OPFS availability at init time**: If the browser doesn't support OPFS, `init()` should fall back to `InMemoryFileSystem` and warn the user that data won't persist.

### Binary Size Audit Process

```bash
# Build browser WASM
wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser

# Check sizes
ls -la pkg/clawft_wasm_bg.wasm
gzip -c pkg/clawft_wasm_bg.wasm | wc -c

# If over budget, apply wasm-opt
wasm-opt -Oz pkg/clawft_wasm_bg.wasm -o pkg/clawft_wasm_bg.opt.wasm
gzip -c pkg/clawft_wasm_bg.opt.wasm | wc -c

# Analyze what's taking space
wasm-objdump -h pkg/clawft_wasm_bg.wasm
# Or use twiggy for detailed size analysis:
twiggy top pkg/clawft_wasm_bg.wasm | head -30
```

---

## C -- Completion

### Exit Criteria

- [ ] `wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser` succeeds
- [ ] `init(config_json)`, `send_message(text)`, `set_env(key, value)` exported via `wasm-bindgen`
- [ ] `tokio::fs::metadata` leak at `file_tools.rs:374` fixed
- [ ] `canonicalize()` replaced with virtual path normalization for browser
- [ ] Browser tool registry registers: ReadFile, WriteFile, EditFile, ListDirectory
- [ ] WASM binary < 500KB gzipped (or documented plan to achieve)
- [ ] `cargo test --workspace` -- zero regressions
- [ ] `cargo build --release --bin weft` -- native CLI builds
- [ ] `cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm` -- WASI build works
- [ ] `scripts/check-features.sh` passes

### Test Commands

```bash
# Full browser WASM build
wasm-pack build crates/clawft-wasm --target web --release --no-default-features --features browser

# Size check
gzip -c pkg/clawft_wasm_bg.wasm | wc -c
# Must be < 512000 bytes (500KB)

# Native regression
cargo test --workspace
cargo clippy --workspace
cargo build --release --bin weft

# WASI regression
cargo build --target wasm32-wasip2 --profile release-wasm -p clawft-wasm

# Feature flag validation
bash scripts/check-features.sh
```

### Documentation Deliverables

1. **`docs/browser/building.md`**: Browser build guide
   - Prerequisites: `wasm-pack`, `wasm32-unknown-unknown` target
   - Build commands
   - Output files: `pkg/clawft_wasm.js`, `pkg/clawft_wasm_bg.wasm`
   - JS integration patterns

2. **`docs/browser/quickstart.md`**: Browser quickstart
   - Minimal HTML/JS example
   - Config setup
   - Sending first message
   - Expected output

3. **`docs/browser/api-reference.md`**: wasm-bindgen API reference
   - `init(config_json: string): Promise<void>` -- params, errors
   - `send_message(text: string): Promise<string>` -- params, return, errors
   - `set_env(key: string, value: string): void` -- params
   - Error handling patterns
   - Lifecycle: init -> set_env -> send_message

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

echo "=== Gate 4: Browser WASM build ==="
wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser

echo "=== Gate 5: Size check ==="
SIZE=$(gzip -c pkg/clawft_wasm_bg.wasm | wc -c)
echo "Browser WASM gzipped: ${SIZE} bytes"
if [ "$SIZE" -gt 512000 ]; then
    echo "WARNING: Size exceeds 500KB budget"
fi

echo "BW5 phase gate PASSED"
```
