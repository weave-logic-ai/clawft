# SPARC Implementation Plan: Phase 3D - Real WASI Implementations + Docker Multi-Arch

**Stream**: 3D - WASI HTTP/FS + Docker Multi-Arch
**Status**: DEFERRED TO PHASE 4 (2026-02-17 decision -- needs more research on wasi-http-client maturity, Rust 1.93 compatibility, and WASM size impact)
**Timeline**: ~~3 weeks (Week 15-17)~~ → Phase 4
**Depends On**: Phase 3A (WASM stubs), Phase 3B (CI/CD), Phase 3C (Rust 1.93 upgrade)
**Primary Target**: `wasm32-wasip2` (Rust 1.93.1+)
**Deliverables**: Working WASI HTTP/FS, validated .wasm binary, multi-arch Docker images

---

## 1. Specification

### 1.1 Functional Requirements

| ID | Requirement | Priority | Acceptance Criteria |
|----|-------------|----------|---------------------|
| FR-3D-001 | WasiHttpClient makes real HTTPS requests via `wasi:http/outgoing-handler` | MUST | POST to httpbin.org returns 200 from within wasmtime |
| FR-3D-002 | WasiHttpClient supports auth headers for OpenAI/Anthropic APIs | MUST | `Authorization: Bearer <key>` header sent and received by mock server |
| FR-3D-003 | WasiHttpClient supports JSON request/response bodies | MUST | Serialize `CompletionRequest`, deserialize `CompletionResponse` |
| FR-3D-004 | WasiHttpClient supports configurable timeouts | SHOULD | 30s default, configurable via `connect_timeout` |
| FR-3D-005 | WasiFileSystem reads files from WASI preopened dirs | MUST | Load `config.json` from `--dir` mapped host path |
| FR-3D-006 | WasiFileSystem writes files for session persistence | MUST | Write JSON to `sessions/*.json`, read it back |
| FR-3D-007 | WasiFileSystem supports `create_dir_all` | MUST | Create nested directories for session/memory storage |
| FR-3D-008 | WasiFileSystem supports `list_dir` | SHOULD | Enumerate session files for restore |
| FR-3D-009 | WasiFileSystem supports `exists` check | MUST | Guard config loading with existence check |
| FR-3D-010 | WasiFileSystem supports `remove_file` | SHOULD | Clean up old session files |
| FR-3D-011 | WASM binary runs in wasmtime with HTTP + FS | MUST | End-to-end: load config, call LLM API, persist session |
| FR-3D-012 | WASM binary produces a `.wasm` component (not just rlib) | MUST | `wasmtime run clawft.wasm` succeeds |
| FR-3D-013 | Docker image supports `linux/amd64` | MUST | `docker run --platform linux/amd64` works |
| FR-3D-014 | Docker image supports `linux/arm64` | MUST | `docker run --platform linux/arm64` works |
| FR-3D-015 | Docker manifest list published to GHCR | MUST | Single tag resolves to correct arch |

### 1.2 Non-Functional Requirements

| ID | Category | Requirement | Measurement |
|----|----------|-------------|-------------|
| NFR-3D-001 | Size | WASM binary <= 350 KB uncompressed (relaxed from 300 KB for real HTTP/FS) | `wc -c clawft.wasm` |
| NFR-3D-002 | Size | WASM binary <= 140 KB gzipped | `gzip -9 -c clawft.wasm | wc -c` |
| NFR-3D-003 | Performance | HTTP round-trip < 5s for typical LLM call (network-bound) | wasmtime benchmark |
| NFR-3D-004 | Performance | FS read < 10ms for 10 KB config file | wasmtime benchmark |
| NFR-3D-005 | Performance | Docker image < 15 MB per arch | `docker images --format` |
| NFR-3D-006 | Compatibility | wasmtime 28+ (latest stable) | CI test matrix |
| NFR-3D-007 | Compatibility | WAMR: graceful fallback for preview2-only features | CI test, documented limitations |

### 1.3 Constraints

```yaml
constraints:
  technical:
    - "wasm32-wasip2 target (Rust 1.93.1+, already available)"
    - "clawft-wasm depends only on clawft-types, serde, serde_json (no tokio/reqwest)"
    - "wasi-http-client crate (0.2.x) or raw wasi crate (0.14.x) bindings"
    - "WASI HTTP outbound requires wasmtime --wasi http flag"
    - "WASI FS requires wasmtime --dir <host-path>::<guest-path> mapping"
    - "WAMR does NOT support WASI preview2 HTTP; fallback to stub for WAMR"
    - "dlmalloc allocator already configured for wasm32 target"
    - "Docker FROM scratch image -- no OS packages available"
  business:
    - "Phase 3D completes WASM from stub to production-usable"
    - "Multi-arch Docker is prerequisite for ARM deployment (Graviton, Raspberry Pi)"
    - "Must not break existing wasip1 fallback compilation"
  regulatory:
    - "API keys must never be hardcoded in WASM binary"
    - "API keys passed via WASI environment or config file at runtime"
```

### 1.4 Use Cases

```yaml
use_cases:
  - id: "UC-3D-001"
    title: "WASM Agent Makes LLM API Call"
    actor: "WASI Host (wasmtime)"
    preconditions:
      - "clawft.wasm component built"
      - "Host grants wasi:http/outgoing-handler capability"
      - "config.json mounted with valid API key"
    flow:
      1. "Host instantiates clawft.wasm component"
      2. "Component reads config.json via wasi:filesystem"
      3. "Component receives user message via exported function"
      4. "Component constructs HTTP POST to api.openai.com"
      5. "Component sends request via wasi:http/outgoing-handler"
      6. "Host proxies HTTPS request to external API"
      7. "Component receives response, parses JSON"
      8. "Component persists session state via wasi:filesystem"
      9. "Component returns assistant response to host"
    postconditions:
      - "Session file written to guest filesystem"
      - "Response returned to caller"
    exceptions:
      - "No network: return error with 'HTTP unavailable' message"
      - "Auth failure: return error with 'Invalid API key' message"
      - "Timeout: return error after 30s with 'Request timed out'"

  - id: "UC-3D-002"
    title: "Multi-Arch Docker Deployment"
    actor: "DevOps / CI Pipeline"
    preconditions:
      - "Native binaries built for both amd64 and arm64"
    flow:
      1. "CI builds weft-linux-x86_64 and weft-linux-aarch64"
      2. "CI creates per-arch Docker images"
      3. "CI creates manifest list combining both images"
      4. "CI pushes manifest list to GHCR"
      5. "User pulls image -- Docker resolves correct arch automatically"
    postconditions:
      - "ghcr.io/<org>/clawft:<tag> resolves to correct arch"
    exceptions:
      - "ARM build failure: publish amd64-only with warning"
```

### 1.5 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `wasi-http-client` crate incompatible with Rust 1.93 | Low | High | Pin known-good version; fallback to raw `wasi` crate bindings |
| WASM binary exceeds 350 KB with real HTTP/FS | Medium | Medium | Profile with `twiggy`; strip unused wasi bindings; use `wasm-opt -Oz` |
| wasmtime WASI HTTP flaky in CI | Medium | Medium | Pin wasmtime version; retry logic in CI; record known-good version |
| WAMR cannot run wasip2 component at all | High | Low | Document WAMR limitation; wasip1 adapter fallback; defer WAMR p2 support |
| Docker ARM build slow in CI (QEMU emulation) | Medium | Low | Use native runner strategy with manifest merge; or accept slower builds |
| `wasi:filesystem` preopens not available in cdylib | Medium | High | Use `bin` crate-type instead of `cdylib`; or use `wasi` crate's FS API directly |

---

## 2. Pseudocode

### 2.1 Real WasiHttpClient

```rust
// crates/clawft-wasm/src/http.rs
//
// Replace the stub with real WASI HTTP preview2 outbound calls.
// Two implementation strategies:
//   Strategy A: Use the `wasi-http-client` crate (high-level, builder API)
//   Strategy B: Use raw `wasi` crate bindings (lower-level, smaller binary)
//
// Decision: Start with Strategy A (wasi-http-client). If binary size exceeds
// budget, switch to Strategy B. The public API stays identical either way.

use std::collections::HashMap;
use std::time::Duration;

pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

pub struct WasiHttpClient {
    default_timeout: Duration,
}

impl WasiHttpClient {
    pub fn new() -> Self {
        Self { default_timeout: Duration::from_secs(30) }
    }

    pub fn with_timeout(timeout: Duration) -> Self {
        Self { default_timeout: timeout }
    }

    pub fn request(
        &self,
        method: &str,
        url: &str,
        headers: &HashMap<String, String>,
        body: Option<&[u8]>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Strategy A: wasi-http-client
        //   let client = wasi_http_client::Client::new();
        //   let mut builder = match method {
        //       "GET"  => client.get(url),
        //       "POST" => client.post(url),
        //       _      => client.request(method.parse()?, url),
        //   };
        //   builder = builder.connect_timeout(self.default_timeout);
        //   for (k, v) in headers {
        //       builder = builder.header(k, v);
        //   }
        //   if let Some(body_bytes) = body {
        //       builder = builder.body(body_bytes);
        //   }
        //   let resp = builder.send()?;
        //   Ok(HttpResponse {
        //       status: resp.status(),
        //       headers: resp.headers().clone(), // adapt as needed
        //       body: resp.body(),
        //   })

        // Strategy B: raw wasi crate
        //   Use wasi::http::outgoing_handler::handle() directly
        //   Build OutgoingRequest from wasi::http::types
        //   Read IncomingResponse body stream

        todo!("implement after dependency evaluation")
    }

    pub fn get(&self, url: &str, headers: &HashMap<String, String>)
        -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>
    {
        self.request("GET", url, headers, None)
    }

    pub fn post(&self, url: &str, headers: &HashMap<String, String>, body: &[u8])
        -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>
    {
        self.request("POST", url, headers, Some(body))
    }
}
```

### 2.2 Real WasiFileSystem

```rust
// crates/clawft-wasm/src/fs.rs
//
// Replace the stub with real WASI filesystem preview2 calls.
// On wasm32-wasip2, Rust's std::fs is wired to WASI filesystem.
// The host must grant access via --dir flag:
//   wasmtime run --dir /host/path::/guest/path clawft.wasm
//
// Key insight: On wasm32-wasip2 target, std::fs "just works" for
// preopened directories. No need for raw wasi crate bindings for
// basic file operations -- Rust's std library handles the translation.

use std::path::{Path, PathBuf};

pub struct WasiFileSystem {
    /// Base path prefix for all operations (maps to preopened dir).
    base_path: PathBuf,
}

impl WasiFileSystem {
    pub fn new() -> Self {
        // Default: root of the preopened directory
        Self { base_path: PathBuf::from("/") }
    }

    pub fn with_base_path(base: impl Into<PathBuf>) -> Self {
        Self { base_path: base.into() }
    }

    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_path.join(path)
        }
    }

    pub fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        // On wasm32-wasip2, std::fs::read_to_string uses WASI filesystem
        std::fs::read_to_string(self.resolve(path))
    }

    pub fn write_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        let resolved = self.resolve(path);
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(resolved, content)
    }

    pub fn append_string(&self, path: &Path, content: &str) -> std::io::Result<()> {
        use std::io::Write;
        let resolved = self.resolve(path);
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(resolved)?;
        file.write_all(content.as_bytes())
    }

    pub fn exists(&self, path: &Path) -> bool {
        self.resolve(path).exists()
    }

    pub fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(self.resolve(path))? {
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    pub fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(self.resolve(path))
    }

    pub fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(self.resolve(path))
    }

    pub fn home_dir(&self) -> Option<PathBuf> {
        // WASI has no concept of home directory.
        // Return the base path as a reasonable default.
        Some(self.base_path.clone())
    }
}
```

### 2.3 WASM Binary Entry Point

```rust
// crates/clawft-wasm/src/main.rs (new file -- bin target)
//
// A thin main() that bridges WASI stdio to the agent pipeline.
// This is needed because `cdylib` crate-type does not get preopens
// automatically on wasip2. A `bin` target does.

fn main() {
    // 1. Initialize platform
    let platform = clawft_wasm::WasmPlatform::new();

    // 2. Read config from WASI filesystem
    let config_path = std::path::Path::new("/config/config.json");
    let config_str = match platform.fs().read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("clawft: failed to load config: {e}");
            std::process::exit(1);
        }
    };

    // 3. Parse config
    let config: clawft_types::Config = match serde_json::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("clawft: invalid config: {e}");
            std::process::exit(1);
        }
    };

    // 4. Read input from stdin (single-message mode)
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("failed to read stdin");

    // 5. Process message
    let response = clawft_wasm::process_message(input.trim());

    // 6. Write response to stdout
    println!("{response}");
}
```

### 2.4 Docker Multi-Arch Build

```dockerfile
# clawft/Dockerfile.multi-arch
#
# Multi-arch Dockerfile using pre-built static binaries.
# The correct binary is selected by TARGETARCH build arg,
# which Docker buildx sets automatically.

FROM scratch

ARG TARGETARCH

# Copy the correct binary based on target architecture
COPY docker-build/weft-linux-${TARGETARCH} /usr/local/bin/weft

VOLUME ["/root/.clawft"]

ENTRYPOINT ["/usr/local/bin/weft"]
CMD ["gateway"]
```

```yaml
# .github/workflows/release.yml additions (docker job)
#
# Replace single-arch docker job with multi-arch strategy:
#
# Job 1: build-docker-images (matrix: [amd64, arm64])
#   - Download per-arch binary artifact
#   - Build per-arch image, push by digest
#   - Export digest as artifact
#
# Job 2: merge-docker-manifest
#   - Download all digests
#   - Create manifest list with docker buildx imagetools create
#   - Push manifest list to GHCR
```

### 2.5 Wasmtime Validation Script

```bash
#!/usr/bin/env bash
# scripts/test-wasm-wasmtime.sh
#
# End-to-end validation of clawft.wasm in wasmtime runtime.

set -euo pipefail

WASM_BINARY="${1:-target/wasm32-wasip2/release-wasm/clawft-wasm.wasm}"
TEST_DIR=$(mktemp -d)

# Create minimal config
cat > "$TEST_DIR/config.json" <<'EOF'
{
  "agents": { "model": "gpt-4o-mini" },
  "providers": {
    "providers": {
      "openai": { "api_key_env": "OPENAI_API_KEY" }
    }
  }
}
EOF

# Create sessions directory
mkdir -p "$TEST_DIR/sessions"

# Run in wasmtime with WASI HTTP and FS access
echo '{"role":"user","content":"hello"}' | \
  wasmtime run \
    --wasi http \
    --dir "$TEST_DIR"::/config \
    --env "OPENAI_API_KEY=${OPENAI_API_KEY:-test-key}" \
    "$WASM_BINARY"

echo "Wasmtime validation passed"
rm -rf "$TEST_DIR"
```

---

## 3. Architecture

### 3.1 Module Structure (Changes to clawft-wasm)

```
crates/clawft-wasm/
  Cargo.toml              # Updated: add wasi-http-client or wasi dep
  src/
    lib.rs                # Updated: wasip2 platform string
    main.rs               # NEW: bin target for .wasm component
    http.rs               # REPLACED: real WASI HTTP implementation
    fs.rs                 # REPLACED: real WASI FS implementation
    env.rs                # UNCHANGED: in-memory HashMap (works as-is)
    platform.rs           # UPDATED: use real HTTP/FS, add with_base_path
    allocator.rs          # UNCHANGED: dlmalloc (already configured)
  tests/
    http_integration.rs   # NEW: wasmtime-based HTTP integration tests
    fs_integration.rs     # NEW: native FS tests (std::fs works on host too)
    wasm_validation.rs    # NEW: binary size + format validation

clawft/
  Dockerfile              # UPDATED: multi-arch support
  Dockerfile.multi-arch   # NEW: explicit multi-arch variant (or update existing)

.github/workflows/
  release.yml             # UPDATED: multi-arch docker job
  wasm-build.yml          # UPDATED: wasmtime run validation step

.cargo/
  config.toml             # UPDATED: add wasip2 target config

scripts/
  test-wasm-wasmtime.sh   # NEW: wasmtime E2E validation
  test-wasm-wamr.sh       # NEW: WAMR validation (graceful fallback)
```

### 3.2 Cargo.toml Changes

```toml
# crates/clawft-wasm/Cargo.toml

[package]
name = "clawft-wasm"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true

# Produce both a library (for tests) and a binary (for the .wasm component)
[[bin]]
name = "clawft-wasm"
path = "src/main.rs"

[features]
default = []
# Strategy A: high-level HTTP client
wasi-http-client = ["dep:wasi-http-client"]
# Strategy B: raw wasi bindings (smaller binary, more code)
wasi-raw = ["dep:wasi"]

[dependencies]
clawft-types = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }

# Strategy A: wasi-http-client (builder API, ~20 KB overhead)
wasi-http-client = { version = "0.2", optional = true }

# Strategy B: raw WASI bindings (minimal overhead)
wasi = { version = "0.14", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
dlmalloc = { version = "0.2", features = ["global"] }
```

### 3.3 Conditional Compilation Strategy

```rust
// crates/clawft-wasm/src/http.rs -- conditional dispatch

// On wasm32-wasip2 with wasi-http-client feature:
#[cfg(all(target_arch = "wasm32", feature = "wasi-http-client"))]
mod wasi_http_impl {
    // Use wasi_http_client::Client for outbound requests
}

// On wasm32-wasip2 with raw wasi feature:
#[cfg(all(target_arch = "wasm32", feature = "wasi-raw"))]
mod wasi_raw_impl {
    // Use wasi::http::outgoing_handler::handle() directly
}

// On native (for testing):
#[cfg(not(target_arch = "wasm32"))]
mod native_stub_impl {
    // Return errors or use reqwest for host-side testing
}
```

```rust
// crates/clawft-wasm/src/fs.rs -- conditional dispatch

// On wasm32 target: use std::fs (wired to WASI filesystem)
// On native target: also use std::fs (works identically for testing)
//
// Key insight: No conditional compilation needed for FS!
// std::fs on wasm32-wasip2 transparently uses WASI filesystem.
// The same code works on native for testing.
```

### 3.4 .cargo/config.toml Update

```toml
# .cargo/config.toml

[target.wasm32-wasip1]
rustflags = ["-C", "opt-level=z"]

[target.wasm32-wasip2]
runner = "wasmtime run --wasi http --dir ."
rustflags = ["-C", "opt-level=z"]

[alias]
wasm = "build --target wasm32-wasip2 --profile release-wasm"
wasm-p1 = "build --target wasm32-wasip1 --profile release-wasm"
```

### 3.5 Size Budget (Updated for Real Implementations)

| Component | Stub Size | Real Size (est.) | Delta |
|-----------|-----------|-------------------|-------|
| clawft-types + serde | ~25 KB | ~25 KB | 0 |
| serde_json | ~40 KB | ~40 KB | 0 |
| WasiHttpClient (stub) | ~1 KB | ~30-60 KB | +30-60 KB |
| WasiFileSystem (stub) | ~1 KB | ~5 KB (uses std::fs) | +4 KB |
| WasiEnvironment | ~2 KB | ~2 KB | 0 |
| dlmalloc | ~8 KB | ~8 KB | 0 |
| wasi-http-client deps (url, serde_urlencoded) | 0 | ~20-40 KB | +20-40 KB |
| lib.rs + main.rs | ~3 KB | ~5 KB | +2 KB |
| **Total (Strategy A)** | **~80 KB** | **~135-185 KB** | +55-105 KB |
| **Total (Strategy B, raw wasi)** | **~80 KB** | **~110-150 KB** | +30-70 KB |
| **Budget** | - | **<= 350 KB** | OK |
| **Post wasm-opt -Oz** | - | **~90-130 KB** | OK |

Note: Size budget relaxed from 300 KB to 350 KB for this phase to accommodate real HTTP
transport. After `wasm-opt -Oz`, we expect significant reduction (30-40% typical).
The gzipped target remains 140 KB.

### 3.6 Docker Multi-Arch Architecture

```
Release Pipeline:
  build-release (matrix):
    ├── x86_64-unknown-linux-musl  → weft-linux-x86_64   (artifact)
    ├── aarch64-unknown-linux-musl → weft-linux-aarch64   (artifact)
    ├── x86_64-apple-darwin        → weft-macos-x86_64    (artifact)
    ├── aarch64-apple-darwin       → weft-macos-aarch64   (artifact)
    └── x86_64-pc-windows-msvc     → weft-windows-x86_64  (artifact)

  docker-build (matrix: [amd64, arm64]):
    ├── Download weft-linux-x86_64 (for amd64)  → build+push by digest
    └── Download weft-linux-aarch64 (for arm64)  → build+push by digest

  docker-merge:
    └── Create manifest list from both digests → push to GHCR
        ghcr.io/<org>/clawft:v0.x.y   (manifest list)
        ghcr.io/<org>/clawft:latest    (manifest list)
```

### 3.7 WAMR Compatibility Strategy

WAMR does not support WASI preview2 HTTP or the Component Model. The strategy:

1. **Primary target**: wasmtime (full wasip2 support)
2. **WAMR fallback**: Compile a separate `wasm32-wasip1` binary with stubs.
   WAMR can run the wasip1 binary for filesystem-only operations.
   HTTP is not available -- the agent returns an error message.
3. **Future**: When WAMR adds Component Model support, the wasip2 binary works as-is.
4. **CI validation**: Test both targets; WAMR test only validates FS + env, not HTTP.

---

## 4. Refinement (TDD Test Plan)

### 4.1 Week 15: WasiHttpClient Implementation

#### Task 15.1: Evaluate wasi-http-client vs raw wasi crate

**Agent Instructions (Coder)**:
1. Read `clawft/crates/clawft-wasm/src/http.rs` (current stub)
2. Read `clawft/crates/clawft-wasm/Cargo.toml` (current deps)
3. Add `wasi-http-client = { version = "0.2", optional = true }` to Cargo.toml
4. Build with `cargo build -p clawft-wasm --target wasm32-wasip2 --features wasi-http-client`
5. Measure size delta with `twiggy diff` or manual comparison
6. If size exceeds budget: switch to raw `wasi = "0.14"` bindings
7. Document decision in `clawft/crates/clawft-wasm/src/http.rs` header comment

**Tests (write first)**:
```rust
// crates/clawft-wasm/tests/http_integration.rs

#[test]
fn wasi_http_client_get_succeeds_on_native() {
    // On native, test that the API shape is correct.
    // Real WASI HTTP testing happens in wasmtime validation.
    let client = WasiHttpClient::new();
    // Verify public API exists and types are correct.
    let _: Result<HttpResponse, _> = client.get("https://httpbin.org/get", &HashMap::new());
}

#[test]
fn wasi_http_client_post_with_json_body() {
    let client = WasiHttpClient::new();
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), "Bearer test-key".to_string());
    let body = br#"{"model":"gpt-4o-mini","messages":[{"role":"user","content":"hi"}]}"#;
    let _: Result<HttpResponse, _> = client.post("https://api.openai.com/v1/chat/completions", &headers, body);
}

#[test]
fn wasi_http_client_timeout_configurable() {
    let client = WasiHttpClient::with_timeout(Duration::from_secs(5));
    // Verify timeout is stored
    assert_eq!(client.default_timeout, Duration::from_secs(5));
}

#[test]
fn http_response_text_and_success() {
    let resp = HttpResponse {
        status: 200,
        headers: HashMap::new(),
        body: br#"{"id":"chatcmpl-123"}"#.to_vec(),
    };
    assert!(resp.is_success());
    assert!(resp.text().unwrap().contains("chatcmpl"));
}
```

**Acceptance Criteria**:
- `cargo check -p clawft-wasm --target wasm32-wasip2` passes with HTTP feature
- All HTTP tests pass on native target
- HTTP implementation compiles for wasm32-wasip2
- Binary size delta documented

#### Task 15.2: Implement WasiHttpClient

**Agent Instructions (Coder)**:
1. Read `clawft/crates/clawft-platform/src/http.rs` (trait definition for reference)
2. Replace stub in `clawft/crates/clawft-wasm/src/http.rs` with real implementation
3. Use `#[cfg(target_arch = "wasm32")]` for WASI-specific code path
4. Use `#[cfg(not(target_arch = "wasm32"))]` for native fallback (returns errors, like current stub)
5. Preserve existing public API: `new()`, `request()`, `get()`, `post()`, `HttpResponse`
6. Add `with_timeout()` constructor
7. Run all existing tests: `cargo test -p clawft-wasm`

**Acceptance Criteria**:
- All 41 existing clawft-wasm tests still pass
- New HTTP tests pass
- `cargo check --target wasm32-wasip2 -p clawft-wasm` passes
- `cargo check --target wasm32-wasip1 -p clawft-wasm` passes (stub fallback)

### 4.2 Week 16: WasiFileSystem + Binary Production

#### Task 16.1: Implement Real WasiFileSystem

**Agent Instructions (Coder)**:
1. Read `clawft/crates/clawft-wasm/src/fs.rs` (current stub)
2. Read `clawft/crates/clawft-platform/src/fs.rs` (trait for reference)
3. Replace stub with `std::fs`-based implementation
4. Key insight: on `wasm32-wasip2`, `std::fs` uses WASI filesystem transparently
5. Same code works on native for testing -- no `#[cfg]` needed for FS
6. Add `with_base_path()` constructor for path prefix support
7. Update `home_dir()` to return base_path instead of None
8. Run all existing tests: `cargo test -p clawft-wasm`

**Tests (write first)**:
```rust
// crates/clawft-wasm/tests/fs_integration.rs

#[test]
fn wasi_fs_write_and_read() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let path = Path::new("test.txt");
    fs.write_string(path, "hello clawft").unwrap();
    assert_eq!(fs.read_to_string(path).unwrap(), "hello clawft");
}

#[test]
fn wasi_fs_creates_parent_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let path = Path::new("sessions/2024/test.json");
    fs.write_string(path, "{}").unwrap();
    assert!(fs.exists(path));
}

#[test]
fn wasi_fs_list_dir() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    fs.write_string(Path::new("a.json"), "{}").unwrap();
    fs.write_string(Path::new("b.json"), "{}").unwrap();
    let entries = fs.list_dir(Path::new(".")).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn wasi_fs_append() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let path = Path::new("log.txt");
    fs.write_string(path, "line1\n").unwrap();
    fs.append_string(path, "line2\n").unwrap();
    assert_eq!(fs.read_to_string(path).unwrap(), "line1\nline2\n");
}

#[test]
fn wasi_fs_remove_file() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let path = Path::new("delete_me.txt");
    fs.write_string(path, "bye").unwrap();
    assert!(fs.exists(path));
    fs.remove_file(path).unwrap();
    assert!(!fs.exists(path));
}

#[test]
fn wasi_fs_read_nonexistent_returns_error() {
    let fs = WasiFileSystem::new();
    assert!(fs.read_to_string(Path::new("/nonexistent/file.txt")).is_err());
}

#[test]
fn wasi_fs_home_dir_with_base() {
    let fs = WasiFileSystem::with_base_path("/clawft");
    assert_eq!(fs.home_dir(), Some(PathBuf::from("/clawft")));
}

#[test]
fn wasi_fs_config_loading_scenario() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let config = r#"{"agents":{"model":"gpt-4o-mini"}}"#;
    fs.write_string(Path::new("config.json"), config).unwrap();
    let loaded: serde_json::Value = serde_json::from_str(
        &fs.read_to_string(Path::new("config.json")).unwrap()
    ).unwrap();
    assert_eq!(loaded["agents"]["model"], "gpt-4o-mini");
}

#[test]
fn wasi_fs_session_persistence_scenario() {
    let dir = tempfile::tempdir().unwrap();
    let fs = WasiFileSystem::with_base_path(dir.path());
    let session_dir = Path::new("sessions");
    fs.create_dir_all(session_dir).unwrap();
    fs.write_string(&session_dir.join("abc123.json"), r#"{"id":"abc123"}"#).unwrap();
    let sessions = fs.list_dir(session_dir).unwrap();
    assert_eq!(sessions.len(), 1);
}
```

**Acceptance Criteria**:
- All existing 41 clawft-wasm tests pass
- All new FS integration tests pass on native
- `cargo check --target wasm32-wasip2 -p clawft-wasm` passes
- Config loading scenario works
- Session persistence scenario works

#### Task 16.2: Create WASM Binary Entry Point

**Agent Instructions (Coder)**:
1. Create `clawft/crates/clawft-wasm/src/main.rs` (new file)
2. Update `clawft/crates/clawft-wasm/Cargo.toml` to add `[[bin]]` section
3. Build: `cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm`
4. Verify the output is a `.wasm` file (not just `.rlib`)
5. Run `wasm-opt -Oz` on the output
6. Measure final binary size

**Acceptance Criteria**:
- `target/wasm32-wasip2/release-wasm/clawft-wasm.wasm` exists
- Binary is a valid WASM component (validate with `wasm-tools validate`)
- Binary size <= 350 KB uncompressed
- Binary size <= 140 KB gzipped after `wasm-opt -Oz`

#### Task 16.3: Wasmtime E2E Validation

**Agent Instructions (Tester)**:
1. Create `scripts/test-wasm-wasmtime.sh`
2. Install wasmtime in CI (or use existing installation)
3. Create test config file and sessions directory
4. Run `wasmtime run --wasi http --dir <test-dir>::/config clawft-wasm.wasm`
5. Verify stdout contains valid response
6. Verify session file was created in test directory

**Acceptance Criteria**:
- Script exits 0 on success
- Config loaded from WASI filesystem
- HTTP request attempted (may fail without real API key, but must not crash)
- Session directory operations succeed

### 4.3 Week 17: Docker Multi-Arch + WAMR + Polish

#### Task 17.1: Docker Multi-Arch Images

**Agent Instructions (DevOps)**:
1. Read `clawft/Dockerfile` (current single-arch)
2. Read `.github/workflows/release.yml` (current docker job)
3. Update Dockerfile to use `ARG TARGETARCH` for binary selection
4. Update release.yml docker job to use matrix strategy:
   - Build per-arch images with `docker/build-push-action`
   - Merge with `docker buildx imagetools create`
5. Rename artifact binaries to match Docker convention:
   - `weft-linux-amd64` (from `weft-linux-x86_64`)
   - `weft-linux-arm64` (from `weft-linux-aarch64`)
   OR use a mapping step in the workflow

**Key Files**:
- `clawft/Dockerfile` -- update or create `Dockerfile.multi-arch`
- `.github/workflows/release.yml` -- docker job overhaul

**Acceptance Criteria**:
- `docker buildx build --platform linux/amd64,linux/arm64 .` succeeds locally
- Release workflow pushes manifest list to GHCR
- `docker pull ghcr.io/<org>/clawft:latest` resolves correct arch

#### Task 17.2: WAMR Validation

**Agent Instructions (Tester)**:
1. Create `scripts/test-wasm-wamr.sh`
2. Build wasip1 binary: `cargo build -p clawft-wasm --target wasm32-wasip1 --release`
3. Install WAMR (`iwasm`) in CI
4. Run `iwasm` with the wasip1 binary
5. Verify: FS operations work (wasip1 filesystem)
6. Verify: HTTP returns graceful error (not crash/panic)
7. Document WAMR limitations in test output

**Acceptance Criteria**:
- wasip1 binary runs in WAMR without crash
- FS operations work for config loading
- HTTP calls return structured error (not panic)
- Limitations documented

#### Task 17.3: Update Platform String and Capabilities

**Agent Instructions (Coder)**:
1. Update `clawft/crates/clawft-wasm/src/lib.rs`:
   - Change platform string from `wasm32-wasip1` to `wasm32-wasip2`
   - Update `capabilities()` to reflect real HTTP/FS
   - Update status from `initializing` to `ready`
2. Update `clawft/crates/clawft-wasm/src/platform.rs`:
   - `WasmPlatform::new()` should use real FS with default base path
   - Add `WasmPlatform::with_config(base_path, timeout)` constructor
3. Run all tests

**Acceptance Criteria**:
- `capabilities()` returns `"platform": "wasm32-wasip2"`
- `capabilities()` returns `"status": "ready"`
- All existing tests updated and passing

#### Task 17.4: Update CI Workflows

**Agent Instructions (DevOps)**:
1. Update `.github/workflows/wasm-build.yml`:
   - Add wasmtime validation step after build
   - Update size limits: `MAX_UNCOMPRESSED_KB: 350`, `MAX_GZIPPED_KB: 140`
   - Add WASM component validation with `wasm-tools validate`
2. Update `.github/workflows/release.yml`:
   - Add WASM binary to release artifacts
   - Add multi-arch Docker support
3. Update `.cargo/config.toml`:
   - Add `wasm32-wasip2` target configuration

**Key Files**:
- `.github/workflows/wasm-build.yml`
- `.github/workflows/release.yml`
- `clawft/.cargo/config.toml`

**Acceptance Criteria**:
- wasm-build.yml runs wasmtime validation
- release.yml produces multi-arch Docker images
- WASM binary included in GitHub Release artifacts
- Size limits updated and enforced

### 4.4 Test Coverage Targets

| Module | Current Coverage | Target Coverage |
|--------|-----------------|-----------------|
| clawft-wasm/http.rs | ~60% (stub tests) | > 85% |
| clawft-wasm/fs.rs | ~70% (stub tests) | > 90% |
| clawft-wasm/platform.rs | ~80% | > 85% |
| clawft-wasm/env.rs | ~95% | > 95% (unchanged) |
| clawft-wasm/lib.rs | ~80% | > 85% |
| clawft-wasm/main.rs | 0% (new) | > 70% |

---

## 5. Completion

### 5.1 Code Deliverables

- [ ] `crates/clawft-wasm/src/http.rs` -- real WASI HTTP implementation
- [ ] `crates/clawft-wasm/src/fs.rs` -- real WASI FS implementation (std::fs)
- [ ] `crates/clawft-wasm/src/main.rs` -- binary entry point for .wasm component
- [ ] `crates/clawft-wasm/src/platform.rs` -- updated with real backends
- [ ] `crates/clawft-wasm/src/lib.rs` -- updated platform string and capabilities
- [ ] `crates/clawft-wasm/Cargo.toml` -- add bin target, wasi deps
- [ ] `clawft/Dockerfile` -- multi-arch support
- [ ] `.cargo/config.toml` -- wasip2 target config

### 5.2 CI/CD Deliverables

- [ ] `.github/workflows/wasm-build.yml` -- wasmtime validation, updated size limits
- [ ] `.github/workflows/release.yml` -- multi-arch Docker, WASM in release artifacts
- [ ] `scripts/test-wasm-wasmtime.sh` -- E2E validation script
- [ ] `scripts/test-wasm-wamr.sh` -- WAMR validation script

### 5.3 Build Artifacts

- [ ] `target/wasm32-wasip2/release-wasm/clawft-wasm.wasm` -- production binary
- [ ] `target/wasm32-wasip2/release-wasm/clawft-wasm.wasm.gz` -- compressed binary
- [ ] `target/wasm32-wasip1/release/clawft-wasm.wasm` -- WAMR-compatible fallback
- [ ] Docker manifest list: `ghcr.io/<org>/clawft:latest` (linux/amd64 + linux/arm64)

### 5.4 Testing

- [ ] All existing 41 clawft-wasm tests pass
- [ ] New HTTP integration tests pass (native)
- [ ] New FS integration tests pass (native)
- [ ] wasmtime E2E validation passes (wasip2 binary)
- [ ] WAMR validation passes (wasip1 binary, FS only)
- [ ] Binary size assertions pass (350 KB / 140 KB)
- [ ] Docker multi-arch build succeeds (amd64 + arm64)
- [ ] `wasm-tools validate` passes on output binary

### 5.5 Exit Criteria

**MUST HAVE (blocks Phase 4)**:
1. WasiHttpClient makes real HTTPS requests in wasmtime
2. WasiFileSystem reads/writes files via WASI preopened dirs
3. `.wasm` component binary produced (not just rlib)
4. Binary size <= 350 KB uncompressed, <= 140 KB gzipped
5. wasmtime E2E validation passes
6. Docker images support linux/amd64 and linux/arm64
7. All existing tests pass (no regressions)

**SHOULD HAVE**:
8. WAMR runs wasip1 binary for FS-only operations
9. Binary validated with `wasm-tools validate`
10. Size profiling with `twiggy top`
11. wasmtime startup time < 100ms

**NICE TO HAVE**:
12. `wasm-opt -Oz` applied in CI pipeline
13. Docker image < 15 MB per arch
14. WASM binary included in GitHub Release

### 5.6 LOE Estimate

| Week | Task | Effort | Confidence |
|------|------|--------|------------|
| 15 | WasiHttpClient real implementation | 3-5 days | Medium (crate compatibility unknown) |
| 15 | HTTP integration tests | 1 day | High |
| 16 | WasiFileSystem real implementation | 1-2 days | High (std::fs "just works") |
| 16 | Binary entry point (main.rs) | 1 day | High |
| 16 | wasmtime E2E validation | 1-2 days | Medium (CI setup) |
| 17 | Docker multi-arch | 2-3 days | High (well-documented pattern) |
| 17 | WAMR validation | 1 day | Medium (limited p2 support) |
| 17 | CI workflow updates | 1-2 days | High |
| 17 | Polish and size optimization | 1 day | High |
| **Total** | | **12-18 days** | |

### 5.7 Handoff to Phase 4

**Artifacts to deliver**:
- Production `.wasm` component binary validated in wasmtime
- Multi-arch Docker images on GHCR
- Updated CI pipelines
- Size profiling report

**What Phase 3D enables**:
- WASM binary can make real LLM API calls (OpenAI, Anthropic)
- WASM binary can persist config and sessions
- Docker images deploy on both AMD64 and ARM64
- Foundation for edge/IoT deployment via wasmtime
- Foundation for WAMR embedded deployment (FS-only, HTTP via host bridge)

**Remaining for future phases**:
- Wire WASM pipeline to clawft-core agent loop (requires solving tokio dep)
- Streaming HTTP responses in WASM
- Vector search integration (micro-hnsw-wasm)
- WASI preview3 async I/O (when available)
- WAMR Component Model support (when WAMR adds it)

---

## Appendix A: Research Sources

- [wasi-http-client crate](https://crates.io/crates/wasi-http-client) -- high-level HTTP for WASIp2
- [wasi crate 0.14](https://docs.rs/wasi/latest/wasi/) -- raw WASIp2 bindings
- [wasip2 crate](https://docs.rs/wasip2/latest/wasip2/) -- explicit WASIp2 bindings
- [wasmtime-wasi-http](https://docs.wasmtime.dev/api/wasmtime_wasi_http/index.html) -- host-side HTTP
- [Bytecode Alliance sample-wasi-http-rust](https://github.com/bytecodealliance/sample-wasi-http-rust) -- reference implementation
- [Rust WASI target changes](https://blog.rust-lang.org/2024/04/09/updates-to-rusts-wasi-targets/) -- target migration
- [wasm32-wasip2 tier 2](https://blog.rust-lang.org/2024/11/26/wasip2-tier-2/) -- tier 2 announcement
- [WASI and Component Model status](https://eunomia.dev/blog/2025/02/16/wasi-and-the-webassembly-component-model-current-status/) -- ecosystem overview
- [WAMR repo](https://github.com/bytecodealliance/wasm-micro-runtime) -- embedded runtime
- [Docker multi-arch GitHub Actions](https://docs.docker.com/build/ci/github-actions/multi-platform/) -- official docs
- [docker/build-push-action](https://github.com/docker/build-push-action) -- CI action
- [Multi-arch without QEMU](https://www.sredevops.org/en/kiss-goodbye-to-qemu-unleash-the-power-of-native-github-runners-for-multi-arch-docker-images/) -- native runner strategy

## Appendix B: Key Commands

```bash
# Build WASM binary (wasip2, production)
cargo build -p clawft-wasm --target wasm32-wasip2 --profile release-wasm

# Build WASM binary (wasip1, WAMR fallback)
cargo build -p clawft-wasm --target wasm32-wasip1 --release

# Optimize binary size
wasm-opt -Oz -o clawft.wasm target/wasm32-wasip2/release-wasm/clawft-wasm.wasm

# Validate WASM component
wasm-tools validate clawft.wasm

# Profile binary size
twiggy top -n 20 clawft.wasm

# Run in wasmtime
wasmtime run --wasi http --dir ./test-config::/config clawft.wasm

# Run in WAMR (wasip1 only)
iwasm target/wasm32-wasip1/release/clawft-wasm.wasm

# Docker multi-arch build (local test)
docker buildx build --platform linux/amd64,linux/arm64 -t clawft:test .

# Compress and measure
gzip -9 -c clawft.wasm > clawft.wasm.gz
ls -la clawft.wasm clawft.wasm.gz
```

## Appendix C: Feature Flag Matrix

| Feature | wasip2 (wasmtime) | wasip1 (WAMR) | Native (testing) |
|---------|-------------------|---------------|------------------|
| HTTP outbound | Real (wasi-http-client or raw wasi) | Stub (returns error) | Stub (returns error) |
| Filesystem | Real (std::fs -> WASI FS) | Real (std::fs -> WASI FS) | Real (std::fs) |
| Environment | In-memory HashMap | In-memory HashMap | In-memory HashMap |
| Process spawn | Unavailable | Unavailable | Unavailable |
| Allocator | dlmalloc | dlmalloc | System default |
| Binary type | .wasm component | .wasm core module | N/A (tests only) |
