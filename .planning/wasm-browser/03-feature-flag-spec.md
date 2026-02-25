# Feature Flag Specification

## Design Principle

Add `native` (default) and `browser` features to crates in the browser compilation path. Native code compiles by default; browser builds use `--no-default-features --features browser`.

---

## clawft-types

```toml
[features]
default = ["native"]
native = ["dep:dirs"]

[dependencies]
dirs = { workspace = true, optional = true }
```

```rust
// src/config/mod.rs
pub fn default_workspace_path(home: Option<&Path>) -> PathBuf {
    // Browser: caller passes None; returns relative path
    // Native: caller passes dirs::home_dir()
    match home {
        Some(h) => h.join(".clawft"),
        None => PathBuf::from(".clawft"),
    }
}
```

---

## clawft-platform

```toml
[features]
default = ["native"]
native = ["dep:tokio", "dep:reqwest", "dep:dirs"]
browser = ["dep:wasm-bindgen", "dep:wasm-bindgen-futures", "dep:web-sys", "dep:js-sys", "dep:gloo-net"]

[dependencies]
# Always available (trait definitions)
async-trait = { workspace = true }
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Native only
tokio = { workspace = true, optional = true }
reqwest = { workspace = true, optional = true }
dirs = { workspace = true, optional = true }

# Browser only
wasm-bindgen = { version = "0.2", optional = true }
wasm-bindgen-futures = { version = "0.4", optional = true }
web-sys = { version = "0.3", optional = true, features = [
    "Request", "RequestInit", "RequestMode", "Response", "Headers",
    "Window", "WorkerGlobalScope",
    "FileSystemDirectoryHandle", "FileSystemFileHandle",
    "FileSystemWritableFileStream",
    "StorageManager",
] }
js-sys = { version = "0.3", optional = true }
gloo-net = { version = "0.6", optional = true }
```

### Module structure

```rust
// src/lib.rs
pub mod http;     // HttpClient trait
pub mod fs;       // FileSystem trait
pub mod env;      // Environment trait
pub mod process;  // ProcessSpawner trait

#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
pub use native::NativePlatform;

#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "browser")]
pub use browser::BrowserPlatform;

// Trait definitions always available
#[cfg_attr(not(feature = "browser"), async_trait::async_trait)]
#[cfg_attr(feature = "browser", async_trait::async_trait(?Send))]
pub trait Platform: ... { ... }
```

---

## clawft-plugin

```toml
[features]
default = ["native"]
native = ["dep:tokio-util"]

[dependencies]
tokio-util = { workspace = true, optional = true }
```

```rust
// Replace CancellationToken with a feature-gated abstraction
#[cfg(feature = "native")]
pub use tokio_util::sync::CancellationToken;

#[cfg(not(feature = "native"))]
pub struct CancellationToken { /* oneshot-based impl */ }
```

---

## clawft-core

```toml
[features]
default = ["full", "native"]
full = []
native = [
    "dep:notify",
    "dep:dirs",
    "clawft-platform/native",
    "clawft-llm/native",
    "clawft-plugin/native",
]
browser = [
    "clawft-platform/browser",
    "clawft-llm/browser",
    "clawft-plugin/browser",
    "dep:wasm-bindgen-futures",
]
```

```rust
// src/agent/mod.rs
#[cfg(feature = "native")]
pub mod skill_watcher;

// src/runtime.rs -- async runtime abstraction
#[cfg(feature = "native")]
pub use tokio::spawn;

#[cfg(feature = "browser")]
pub fn spawn<F: Future<Output = ()> + 'static>(f: F) {
    wasm_bindgen_futures::spawn_local(f);
}
```

---

## clawft-llm

```toml
[features]
default = ["native"]
native = ["reqwest/rustls-tls", "dep:tokio"]
browser = ["reqwest/wasm"]
```

No code changes needed if using reqwest -- it supports `wasm32-unknown-unknown` with the `wasm` feature.

---

## clawft-tools

```toml
[features]
default = ["native"]
native = ["clawft-platform/native", "clawft-core/native"]
browser = ["clawft-platform/browser", "clawft-core/browser"]
native-exec = ["native"]  # already exists, gates shell/spawn
```

Fix: replace `tokio::fs::metadata()` at `file_tools.rs:374` with Platform trait call.

---

## clawft-wasm

```toml
[features]
default = ["browser"]
browser = [
    "clawft-core/browser",
    "clawft-llm/browser",
    "clawft-tools/browser",
    "dep:wasm-bindgen",
    "dep:wasm-bindgen-futures",
    "dep:web-sys",
    "dep:js-sys",
]

[lib]
crate-type = ["cdylib", "rlib"]
```

---

## Build Commands

```bash
# Native (default, unchanged)
cargo build
cargo test

# Browser WASM
cargo build --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser

# WASM check in CI
cargo check --target wasm32-unknown-unknown -p clawft-types --no-default-features
cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-core --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser

# wasm-pack for JS bindings
wasm-pack build crates/clawft-wasm --target web --no-default-features --features browser
```
