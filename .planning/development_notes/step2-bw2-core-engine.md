# BW2: Core Engine -- WASM Compilation for clawft-core

Phase BW2 of the Browser Workstream. Makes `clawft-core` compile for
`wasm32-unknown-unknown` with `--no-default-features --features browser` by
feature-gating native-only dependencies and providing browser-compatible
alternatives.

## Exit Criteria (all met)

- `cargo check --target wasm32-unknown-unknown -p clawft-core --no-default-features --features browser` passes (1 pre-existing warning only)
- `cargo check -p clawft-core` passes (native, zero warnings)
- `cargo test --workspace` passes (all tests, zero failures)

## Files Changed

### clawft-llm

- **`crates/clawft-llm/Cargo.toml`**
  - Added `[features]` section: `default = ["native"]`, `native = ["dep:reqwest", "dep:tokio", "clawft-types/native"]`, `browser = ["clawft-types/browser"]`.
  - Changed `reqwest` and `tokio` to optional behind `native`.
  - Set `clawft-types` to `default-features = false`.

- **`crates/clawft-llm/src/lib.rs`**
  - Gated `failover`, `openai_compat`, `provider`, `retry`, `router` modules behind `#[cfg(feature = "native")]`.
  - Gated their re-exports similarly.

- **`crates/clawft-llm/src/error.rs`**
  - Gated `Http(#[from] reqwest::Error)` variant behind `#[cfg(feature = "native")]`.

### clawft-core

- **`crates/clawft-core/Cargo.toml`**
  - Added `[features]` section with `native` and `browser` feature flags.
  - `native` activates: `dep:notify`, `dep:dirs`, `dep:tokio`, `dep:tokio-util`, and propagates native features to downstream crates.
  - `browser` activates: `dep:wasm-bindgen-futures`, `dep:js-sys`, `dep:futures-channel`, and propagates browser features.
  - All internal crate deps set to `default-features = false`.
  - Native-only deps (`tokio`, `tokio-util`, `dirs`, `notify`) made optional.
  - Browser deps (`wasm-bindgen-futures`, `js-sys`, `futures-channel`) added as optional.

- **`crates/clawft-core/src/runtime.rs`** (NEW)
  - Platform-agnostic runtime abstraction module.
  - `now_millis()`: native uses `SystemTime::now()`, browser uses `js_sys::Date::now()`.
  - `Mutex` re-export: native uses `tokio::sync::Mutex`, browser uses `futures_util::lock::Mutex`.
  - `RwLock`: native re-exports `tokio::sync::RwLock`, browser provides a thin async-compatible wrapper around `std::sync::RwLock` with `.read().await` / `.write().await` methods that resolve immediately (safe in single-threaded WASM).

- **`crates/clawft-core/src/lib.rs`**
  - Added `pub mod runtime;`.
  - Gated `agent_bus` behind `#[cfg(feature = "native")]`.

- **`crates/clawft-core/src/agent/mod.rs`**
  - Gated `pub mod skill_watcher;` behind `#[cfg(feature = "native")]`.

- **`crates/clawft-core/src/agent/loop_core.rs`**
  - Replaced `use tokio_util::sync::CancellationToken` with `use clawft_plugin::CancellationToken` (polyfill from BW1).
  - Gated `tokio::select!` in `run()` behind `#[cfg(feature = "native")]` with a browser fallback using `is_cancelled()` polling.

- **`crates/clawft-core/src/agent/skill_autogen.rs`**
  - Gated `dirs::home_dir()` in `install_dir()` behind `#[cfg(feature = "native")]` with browser fallback to `.clawft/skills`.

- **`crates/clawft-core/src/agent/skills.rs`**
  - Changed `use tokio::sync::RwLock` to `use crate::runtime::RwLock`.

- **`crates/clawft-core/src/agent/skills_v2.rs`**
  - Changed `use tokio::sync::RwLock` to `use crate::runtime::RwLock`.
  - Gated `load_dir()` behind `#[cfg(feature = "native")]` with browser stub returning empty vec.
  - Gated `load_legacy_skill_async()` behind `#[cfg(feature = "native")]`.
  - Gated `validate_directory_name` import behind `#[cfg(feature = "native")]`.

- **`crates/clawft-core/src/agent/context.rs`**
  - Changed `use tokio::sync::Mutex` to `use crate::runtime::Mutex`.
  - Gated `CachedFile` struct, `BootstrapCache` type alias, and `SystemTime`/`PathBuf` imports behind `#[cfg(feature = "native")]`.
  - Browser `BootstrapCache` uses `Arc<Mutex<()>>` as no-op placeholder.
  - Gated `load_cached_file()` with mtime-based caching behind `#[cfg(feature = "native")]`.
  - Added browser `load_cached_file()` that reads directly through the platform (no caching).

- **`crates/clawft-core/src/session.rs`**
  - Changed `use tokio::sync::Mutex` to `use crate::runtime::Mutex`.

- **`crates/clawft-core/src/workspace/config.rs`**
  - Gated `dirs::home_dir()` in `load_merged_config()`.

- **`crates/clawft-core/src/workspace/mod.rs`**
  - Gated `dirs::home_dir()` in `discover_workspace()` and `WorkspaceManager::new()`.

- **`crates/clawft-core/src/policy_kernel.rs`**
  - Gated `dirs::home_dir()` in `agent_path()` and `global_path()`.

- **`crates/clawft-core/src/embeddings/rvf_io.rs`**
  - Gated `dirs::home_dir()` in `segment_file_path()`.

- **`crates/clawft-core/src/bus.rs`**
  - Complete rewrite with dual conditional implementations.
  - Native: uses `tokio::sync::mpsc` bounded channels and `tokio::sync::Mutex`.
  - Browser: uses `futures_channel::mpsc::unbounded` and `futures_util::lock::Mutex`.
  - Same public API on both: `new()`, `with_capacity()`, `publish_inbound()`, `consume_inbound()`, etc.
  - `inbound_sender()` and `outbound_sender()` only available on native.
  - Native-only tests gated with `#[cfg(feature = "native")]`.

- **`crates/clawft-core/src/pipeline/mod.rs`**
  - Gated `pub mod llm_adapter;` behind `#[cfg(feature = "native")]`.

- **`crates/clawft-core/src/pipeline/transport.rs`**
  - Gated `complete_stream` on `LlmProvider` trait behind `#[cfg(feature = "native")]`.
  - Gated streaming `complete_stream` implementation on `OpenAiCompatTransport` behind `#[cfg(feature = "native")]`.
  - Gated `StreamCallback` import behind `#[cfg(feature = "native")]`.
  - Added `use tokio::sync::mpsc;` to test module (was previously a top-level import).

- **`crates/clawft-core/src/pipeline/traits.rs`**
  - Replaced `use std::time::Instant` + `Instant::now()` / `.elapsed()` with `crate::runtime::now_millis()` for WASM-safe latency measurement.

- **`crates/clawft-core/src/pipeline/tiered_router.rs`**
  - Replaced `SystemTime::now()` with `crate::runtime::now_millis()`.

- **`crates/clawft-core/src/bootstrap.rs`**
  - Gated `build_live_pipeline()` behind `#[cfg(feature = "native")]`.
  - Gated `enable_live_llm()` on `AppContext` behind `#[cfg(feature = "native")]`.

## Key Design Decisions

### 1. Runtime Abstraction Module
Created `crates/clawft-core/src/runtime.rs` as a central abstraction point for all platform-dependent async primitives. This avoids scattering `#[cfg]` gates throughout business logic code.

### 2. RwLock Async Wrapper
Rather than re-exporting `std::sync::RwLock` (which has incompatible sync `.read()/.write()` API), created a thin wrapper that provides `.read().await` and `.write().await` methods. On single-threaded WASM, these resolve immediately since there is no lock contention.

### 3. MessageBus Dual Implementation
The `MessageBus` has two complete implementations rather than a shared abstraction, because the API surfaces differ significantly (tokio bounded channels with `try_send` vs futures unbounded channels with `unbounded_send`).

### 4. Filesystem Operations
All `tokio::fs` operations in non-test code are gated behind native. Browser stubs return empty results or skip the operation. The `Platform` trait's filesystem abstraction handles reads/writes portably.

### 5. Time Measurement
Replaced all `std::time::Instant` and `std::time::SystemTime::now()` in non-gated code with `crate::runtime::now_millis()`, which uses `js_sys::Date::now()` on browser.

## Remaining Warnings (pre-existing, not introduced by this phase)

- `workspace/agent.rs:257`: unreachable expression (platform-specific symlink code)
