# Dependency Audit: WASM Compilation Blockers

## Five Root-Cause Dependencies

Nearly all WASM compilation failures trace back to five workspace dependencies:

| # | Dependency | Workspace Location | Crates Affected | Blocker Reason |
|---|-----------|-------------------|-----------------|----------------|
| 1 | `tokio = { version = "1", features = ["full"] }` | `Cargo.toml:37` | 8+ crates | OS threads, file I/O, process spawning |
| 2 | `reqwest = { version = "0.12", features = ["rustls-tls"] }` | `Cargo.toml:40` | 5+ crates | Native TLS; but `reqwest` has a `wasm` feature |
| 3 | `dirs = "6"` | `Cargo.toml:63` | ~5 crates | Home directory detection, no WASM support |
| 4 | `notify = "7"` | `clawft-core/Cargo.toml:43` | clawft-core | OS filesystem watcher (inotify/FSEvents) |
| 5 | `tokio-util = "0.7"` | `clawft-plugin/Cargo.toml` | clawft-plugin, clawft-core | CancellationToken requires tokio |

## Three Platform Abstraction Leaks

| # | Location | Code | Fix |
|---|----------|------|-----|
| 1 | `clawft-tools/src/file_tools.rs:374` | `tokio::fs::metadata()` | Use Platform `fs` trait |
| 2 | `clawft-platform/src/config_loader.rs` | `path.exists()` (std::path) | Use `Platform::fs().exists()` |
| 3 | `clawft-core/Cargo.toml:43` | `notify` hard dep (no feature gate) | Gate behind `native` feature |

## Per-Crate WASM Compilation Status

| Crate | Compiles to WASM? | Blocker(s) | Fix Effort |
|-------|-------------------|------------|------------|
| `clawft-types` | Almost | `dirs` | Low -- feature-gate |
| `clawft-platform` | No | `tokio`, `reqwest`, `dirs` | Medium -- split traits/impls |
| `clawft-plugin` | Almost | `tokio-util` (CancellationToken) | Low -- replace or gate |
| `clawft-security` | **Yes** | None | None |
| `clawft-core` | No | `tokio`, `notify`, `dirs`, `futures-util` | High -- feature-gate |
| `clawft-llm` | No | `reqwest`, `tokio` | Medium -- swap transport |
| `clawft-tools` | No | `tokio` via platform | Medium -- follows platform |
| `clawft-channels` | No | `tokio-tungstenite`, `reqwest` | **Not needed** |
| `clawft-services` | No | `tokio`, `reqwest`, `cron` | **Not needed** |
| `clawft-cli` | No | `clap`, terminal I/O | **Not applicable** |
| `clawft-wasm` | **Yes** | N/A (already compiles) | N/A |

## WASM-Safe Dependencies (No Changes Needed)

```
serde, serde_json, serde_yaml
async-trait
chrono (with js-sys feature for Local::now)
uuid (with getrandom/js feature)
thiserror
tracing
sha2, hmac
ed25519-dalek
regex
percent-encoding
fnv
futures-util (works without tokio)
```

## Additional WASM Feature Requirements

| Dependency | Feature Needed | Why |
|-----------|---------------|-----|
| `getrandom` | `js` | Random number generation in browser (uuid v4, etc.) |
| `chrono` | `js-sys` | `Local::now()` needs JS Date API |
| `reqwest` | `wasm` (instead of `rustls-tls`) | Browser-compatible HTTP client |

## Minimal Browser Crate Subset

```
clawft-types       -- shared types
clawft-platform    -- trait definitions + BrowserPlatform impl
clawft-plugin      -- plugin traits
clawft-security    -- pure computation
clawft-core        -- agent loop + pipeline
clawft-llm         -- LLM transport
clawft-tools       -- browser-safe tool subset
clawft-wasm        -- wasm-bindgen entry point
```

Excluded from browser:
```
clawft-channels        -- server-side
clawft-services        -- server-side
clawft-cli             -- native binary
clawft-plugin-*        -- all plugin crates
```
