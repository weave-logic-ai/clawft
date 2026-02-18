# SPARC Implementation Plan: Phase 3A - WASM Core

**Stream**: 3A - WASM Core (Week 11-14)
**Timeline**: 4 weeks
**Binary**: `clawft.wasm` (target: < 300 KB uncompressed, < 120 KB gzipped)

---

## 1. Agent Instructions

### Documentation References
- **Primary**: `repos/nanobot/.planning/02-technical-requirements.md` (Section 8: WASM Requirements)
- **Primary**: `repos/nanobot/.planning/04-rvf-integration.md` (Section 5.5: rvf-wasm microkernel)
- **Primary**: `repos/nanobot/.planning/05-ruvector-crates.md` (WASM size budget, micro-hnsw-wasm)
- **Supporting**: `repos/nanobot/.planning/01-business-requirements.md` (Success criteria: WASM < 300 KB)
- **Supporting**: `repos/nanobot/.planning/03-development-guide.md` (Phase 3 checklist)

### Module Structure
```
crates/
  clawft-wasm/           # New crate - WASM entrypoint
    src/
      lib.rs             # WASM exports, initialization
      http.rs            # WasiHttpClient implementation
      fs.rs              # WasiFileSystem implementation
      env.rs             # WasiEnvironment implementation
      allocator.rs       # talc global allocator
      vector.rs          # micro-hnsw-wasm integration
    Cargo.toml           # Features: micro-hnsw, temporal-tensor, sona-wasm
  clawft-core/           # Modified - conditional compilation
    src/
      platform/          # Platform abstraction traits
        mod.rs           # HttpClient, FileSystem, Environment traits
        native.rs        # Native implementations (existing)
        wasi.rs          # Re-export from clawft-wasm
```

### Branch Strategy
- **Feature Branch**: `feature/phase-3a-wasm-core`
- **PR Target**: `develop`
- **Sub-branches** (optional):
  - `feature/wasm-entrypoint` (Week 11)
  - `feature/wasm-platform` (Week 12)
  - `feature/wasm-vector` (Week 13)
  - `feature/wasm-validation` (Week 14)

### Agent Coordination
- **Coder Agent**: Implement platform abstractions, WASM entrypoint
- **Tester Agent**: Write WASM integration tests, size validation
- **Reviewer Agent**: Verify size budget, check for unnecessary dependencies
- **Performance Agent**: Profile binary size with `twiggy`, optimize allocations

---

## 2. Specification

### 2.1 Week 11: WASM Entrypoint

#### Requirements
1. Create `clawft-wasm` crate with `wasm32-wasip2` target
2. Implement WASM exports for agent loop initialization
3. Integrate WASI HTTP client for LLM API calls (OpenAI, Anthropic)
4. Implement WASI filesystem for config loading (`~/.clawft/config.toml`)
5. Implement WASI filesystem for session persistence (`~/.clawft/sessions/*.json`)

#### Acceptance Criteria
- `cargo build --target wasm32-wasip2 --profile release-wasm` succeeds
- WASM module exports `clawft_init()` and `clawft_run()` functions
- HTTP client can call OpenAI/Anthropic APIs via WASI HTTP preview2
- Config file loads from WASI filesystem
- Session state persists to WASI filesystem

#### Dependencies
- `wasi` crate (0.14+) for WASI preview2 bindings
- `wit-bindgen` for component model bindings
- `clawft-core` (with `wasm` feature flag)

### 2.2 Week 12: Platform Implementations

#### Requirements
1. Implement `WasiHttpClient` conforming to `HttpClient` trait
2. Implement `WasiFileSystem` conforming to `FileSystem` trait
3. Implement `WasiEnvironment` conforming to `Environment` trait
4. Strip exec tool from WASM build via `#[cfg(not(target_arch = "wasm32"))]`
5. Strip channel plugins from WASM build (WebSocket not available in WASI)
6. Strip CLI terminal I/O from WASM build

#### Acceptance Criteria
- All platform traits have WASI implementations
- `clawft-core` compiles for `wasm32-wasip2` with `wasm` feature
- Exec tool, channel plugins, CLI excluded from WASM build
- No compile errors or warnings for WASM target
- Binary size < 200 KB at this stage

#### Feature Flags
```toml
[features]
default = ["native-platform", "exec-tool", "channels", "cli"]
wasm = ["wasi-platform"]
native-platform = []
wasi-platform = []
exec-tool = []
channels = []
cli = []
```

### 2.3 Week 13: Vector Search

#### Requirements
1. Integrate `talc` allocator (< 1 KB overhead vs default)
2. Integrate `rvf-wasm` microkernel (< 8 KB, vector search via C-ABI)
3. Integrate `micro-hnsw-wasm` (11.8 KB, HNSW nearest-neighbor search)
4. Implement memory manager using `talc` for vector allocations
5. Expose vector search API from WASM module

#### Acceptance Criteria
- Global allocator uses `talc` on WASM target
- `rvf-wasm` microkernel loads and executes vector operations
- `micro-hnsw-wasm` performs nearest-neighbor search
- Memory overhead < 50 KB for 10,000 vectors (512-dim)
- Binary size < 250 KB

#### Technical Constraints
- `talc` must be initialized with a fixed-size memory region
- `rvf-wasm` must use C-ABI FFI (no Rust struct passing)
- `micro-hnsw-wasm` must use pre-allocated index buffer

### 2.4 Week 14: Validation

#### Requirements
1. Test WASM module in WAMR (IoT/interpreter mode)
2. Test WASM module in Wasmtime (edge/cloud JIT mode)
3. Validate binary size: < 300 KB uncompressed, < 120 KB gzipped
4. Benchmark startup time, memory usage, throughput
5. Document WASM deployment guide

#### Acceptance Criteria
- WASM module runs successfully in WAMR 2.0+
- WASM module runs successfully in Wasmtime 18.0+
- Uncompressed size ≤ 300 KB
- Gzipped size ≤ 120 KB
- Startup time < 50ms in Wasmtime
- Memory RSS < 10 MB for idle agent

#### Test Matrix
| Runtime | Mode | Expected Result |
|---------|------|-----------------|
| WAMR 2.0 | Interpreter | Pass, < 100ms startup |
| WAMR 2.0 | AOT | Pass, < 20ms startup |
| Wasmtime 18 | JIT | Pass, < 50ms startup |
| Wasmtime 18 | AOT | Pass, < 10ms startup |

---

## 3. Pseudocode

### 3.1 WASM Entrypoint (Week 11)

```rust
// crates/clawft-wasm/src/lib.rs

#[no_mangle]
pub extern "C" fn clawft_init(config_path: *const u8, config_len: usize) -> i32 {
    // 1. Parse config path from WASI memory
    let config_str = unsafe { std::slice::from_raw_parts(config_path, config_len) };
    let config_path = std::str::from_utf8(config_str).unwrap();

    // 2. Load config from WASI filesystem
    let fs = WasiFileSystem::new();
    let config = fs.read_file(config_path)?;

    // 3. Initialize agent context
    let context = AgentContext::from_config(config)?;

    // 4. Store context in thread-local storage
    AGENT_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));

    0 // Success
}

#[no_mangle]
pub extern "C" fn clawft_run(input_ptr: *const u8, input_len: usize) -> i32 {
    // 1. Parse input message from WASI memory
    let input_slice = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let message = serde_json::from_slice::<Message>(input_slice)?;

    // 2. Run agent loop
    let context = AGENT_CONTEXT.with(|ctx| ctx.borrow().clone().unwrap());
    let response = agent_loop(&context, message)?;

    // 3. Serialize response to WASI memory
    let output = serde_json::to_vec(&response)?;

    // 4. Write to WASI stdout
    println!("{}", String::from_utf8(output)?);

    0 // Success
}
```

### 3.2 WASI HTTP Client (Week 12)

```rust
// crates/clawft-wasm/src/http.rs

use wasi::http::types::{Method, Request, Response};

pub struct WasiHttpClient;

impl HttpClient for WasiHttpClient {
    async fn post(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>> {
        // 1. Create WASI HTTP request
        let request = Request::new(Method::Post, url);
        request.set_body(body);

        // 2. Send via WASI HTTP outgoing handler
        let response = wasi::http::outgoing_handler::handle(request).await?;

        // 3. Read response body
        let body = response.body().read_all().await?;

        Ok(body)
    }
}
```

### 3.3 talc Allocator Integration (Week 13)

```rust
// crates/clawft-wasm/src/allocator.rs

use talc::{Talc, Talck, locking::AssumeUnlockable, ClaimOnOom};

#[global_allocator]
static ALLOCATOR: Talck<AssumeUnlockable, ClaimOnOom> = Talc::new(unsafe {
    ClaimOnOom::new(talc::Span::empty())
}).lock();

#[no_mangle]
pub extern "C" fn clawft_init_memory(heap_base: *mut u8, heap_size: usize) {
    // Initialize talc with WASI linear memory region
    unsafe {
        let span = talc::Span::from_base_size(heap_base, heap_size);
        ALLOCATOR.lock().claim(span).unwrap();
    }
}
```

### 3.4 micro-hnsw Integration (Week 13)

```rust
// crates/clawft-wasm/src/vector.rs

use micro_hnsw_wasm::{Hnsw, HnswParams};

pub struct WasmVectorStore {
    hnsw: Hnsw<f32>,
}

impl WasmVectorStore {
    pub fn new(dim: usize, max_elements: usize) -> Self {
        let params = HnswParams {
            m: 16,          // Connections per layer
            ef_construction: 200,
            max_elements,
        };

        let hnsw = Hnsw::new(dim, params);
        Self { hnsw }
    }

    pub fn add(&mut self, id: u64, vector: &[f32]) {
        self.hnsw.add_point(id, vector);
    }

    pub fn search(&self, query: &[f32], k: usize) -> Vec<(u64, f32)> {
        self.hnsw.search(query, k, 50) // ef_search = 50
    }
}
```

---

## 4. Architecture

### 4.1 Crate Structure

```
clawft-wasm/
├── Cargo.toml          # WASM-specific dependencies
├── src/
│   ├── lib.rs          # WASM exports, initialization
│   ├── http.rs         # WasiHttpClient
│   ├── fs.rs           # WasiFileSystem
│   ├── env.rs          # WasiEnvironment
│   ├── allocator.rs    # talc global allocator
│   └── vector.rs       # micro-hnsw-wasm integration
├── wit/                # WIT component model definitions
│   ├── clawft.wit      # Agent interface
│   └── deps/           # WASI dependencies
└── tests/
    └── integration.rs  # WASM integration tests
```

### 4.2 Cargo.toml

```toml
[package]
name = "clawft-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
clawft-core = { path = "../clawft-core", default-features = false, features = ["wasm"] }
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }

# WASI bindings
wasi = "0.14"
wit-bindgen = "0.34"

# Allocator
talc = "4.4"

# Vector search (optional features)
micro-hnsw-wasm = { version = "0.1", optional = true }
ruvector-temporal-tensor = { path = "../../ruvector/crates/temporal-tensor", optional = true, default-features = false, features = ["wasm"] }
cognitum-gate-kernel = { path = "../../ruvector/crates/cognitum-gate-kernel", optional = true, default-features = false, features = ["wasm"] }
sona = { version = "0.1", optional = true, default-features = false, features = ["wasm"] }

[features]
default = ["micro-hnsw", "temporal-tensor", "sona-wasm"]
micro-hnsw = ["dep:micro-hnsw-wasm"]
temporal-tensor = ["dep:ruvector-temporal-tensor"]
sona-wasm = ["dep:sona"]
coherence = ["dep:cognitum-gate-kernel"]

[profile.release-wasm]
inherits = "release"
opt-level = "z"           # Optimize for size
lto = true                # Link-time optimization
codegen-units = 1         # Single codegen unit for smaller binary
panic = "abort"           # Abort on panic (no unwinding)
strip = true              # Strip symbols
```

### 4.3 Build Configuration

```toml
# .cargo/config.toml (add WASM target)

[target.wasm32-wasip2]
runner = "wasmtime"
rustflags = [
    "-C", "link-arg=--import-memory",
    "-C", "link-arg=--export-memory",
    "-C", "link-arg=--max-memory=16777216",  # 16 MB max memory
]

[build]
# Default to release-wasm profile for WASM builds
target-dir = "target"
```

### 4.4 Platform Abstraction Layer

```rust
// crates/clawft-core/src/platform/mod.rs

pub trait HttpClient: Send + Sync {
    async fn post(&self, url: &str, body: Vec<u8>) -> Result<Vec<u8>>;
    async fn get(&self, url: &str) -> Result<Vec<u8>>;
}

pub trait FileSystem: Send + Sync {
    fn read_file(&self, path: &str) -> Result<Vec<u8>>;
    fn write_file(&self, path: &str, data: &[u8]) -> Result<()>;
    fn exists(&self, path: &str) -> bool;
}

pub trait Environment: Send + Sync {
    fn get_var(&self, key: &str) -> Option<String>;
    fn home_dir(&self) -> Option<String>;
}

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::{NativeHttpClient, NativeFileSystem, NativeEnvironment};

#[cfg(target_arch = "wasm32")]
pub use clawft_wasm::{WasiHttpClient, WasiFileSystem, WasiEnvironment};
```

### 4.5 Size Budget Tracking

| Component | Target Size | Week |
|-----------|-------------|------|
| clawft-core (agent loop, context, tools) | ~100 KB | 11-12 |
| micro-hnsw-wasm | 11.8 KB | 13 |
| ruvector-temporal-tensor (FFI) | < 10 KB | 13 |
| cognitum-gate-kernel | < 10 KB | 13 |
| sona (WASM subset) | ~30 KB | 13 |
| rvf-types | ~30 KB | 13 |
| WASI HTTP client | ~50 KB | 11 |
| **Total** | **~242 KB** | 14 |
| **Budget** | **< 300 KB** | ✓ |

---

## 5. Refinement (TDD Test Plan)

### 5.1 Week 11: WASM Entrypoint Tests

#### Test Files
- `crates/clawft-wasm/tests/entrypoint.rs`
- `crates/clawft-wasm/tests/http_client.rs`
- `crates/clawft-wasm/tests/filesystem.rs`

#### Test Cases

```rust
// tests/entrypoint.rs

#[test]
fn test_clawft_init_success() {
    // Given: Valid config path
    let config_path = "/test/config.toml";

    // When: Initialize WASM module
    let result = clawft_init(config_path.as_ptr(), config_path.len());

    // Then: Returns success
    assert_eq!(result, 0);
}

#[test]
fn test_clawft_run_message() {
    // Given: Initialized module and input message
    clawft_init(/* ... */);
    let message = r#"{"role":"user","content":"Hello"}"#;

    // When: Run agent loop
    let result = clawft_run(message.as_ptr(), message.len());

    // Then: Returns success and produces output
    assert_eq!(result, 0);
    // Verify stdout contains JSON response
}
```

```rust
// tests/http_client.rs

#[tokio::test]
async fn test_wasi_http_post() {
    // Given: WASI HTTP client and mock server
    let client = WasiHttpClient;
    let url = "http://localhost:8080/test";
    let body = b"test data";

    // When: POST request
    let response = client.post(url, body.to_vec()).await;

    // Then: Success with response body
    assert!(response.is_ok());
}
```

### 5.2 Week 12: Platform Implementation Tests

#### Test Files
- `crates/clawft-core/tests/wasm_platform.rs`

#### Test Cases

```rust
#[cfg(target_arch = "wasm32")]
#[test]
fn test_wasi_filesystem_read() {
    // Given: WASI filesystem with test file
    let fs = WasiFileSystem::new();
    fs.write_file("/test/file.txt", b"content").unwrap();

    // When: Read file
    let data = fs.read_file("/test/file.txt");

    // Then: Returns correct content
    assert_eq!(data.unwrap(), b"content");
}

#[test]
fn test_exec_tool_excluded_in_wasm() {
    // Given: WASM build
    #[cfg(target_arch = "wasm32")]
    {
        // When: Try to access exec tool
        // Then: Should not compile
        // This test verifies via compile-time feature flags
    }
}
```

### 5.3 Week 13: Vector Search Tests

#### Test Files
- `crates/clawft-wasm/tests/vector_search.rs`
- `crates/clawft-wasm/tests/allocator.rs`

#### Test Cases

```rust
// tests/vector_search.rs

#[test]
fn test_micro_hnsw_search() {
    // Given: Vector store with 100 vectors
    let mut store = WasmVectorStore::new(512, 1000);
    for i in 0..100 {
        let vector = vec![i as f32; 512];
        store.add(i, &vector);
    }

    // When: Search for nearest neighbors
    let query = vec![50.0; 512];
    let results = store.search(&query, 5);

    // Then: Returns 5 nearest neighbors
    assert_eq!(results.len(), 5);
    assert_eq!(results[0].0, 50); // Exact match first
}

// tests/allocator.rs

#[test]
fn test_talc_allocator_overhead() {
    // Given: talc allocator initialized
    clawft_init_memory(/* heap base */, 1024 * 1024); // 1 MB

    // When: Allocate 1000 vectors
    let mut vectors = Vec::new();
    for _ in 0..1000 {
        vectors.push(vec![0.0f32; 512]);
    }

    // Then: Memory overhead < 50 KB
    let overhead = measure_allocator_overhead();
    assert!(overhead < 50 * 1024);
}
```

### 5.4 Week 14: Validation Tests

#### Test Files
- `crates/clawft-wasm/tests/runtime_validation.rs`
- `crates/clawft-wasm/tests/size_validation.rs`

#### Test Cases

```rust
// tests/size_validation.rs

#[test]
fn test_binary_size_under_300kb() {
    // Given: Built WASM binary
    let binary_path = "target/wasm32-wasip2/release-wasm/clawft.wasm";

    // When: Check file size
    let metadata = std::fs::metadata(binary_path).unwrap();
    let size_bytes = metadata.len();

    // Then: Size < 300 KB
    assert!(size_bytes < 300 * 1024, "WASM binary size {} exceeds 300 KB", size_bytes);
}

#[test]
fn test_gzipped_size_under_120kb() {
    // Given: Built and gzipped WASM binary
    let compressed_path = "target/wasm32-wasip2/release-wasm/clawft.wasm.gz";

    // When: Check compressed size
    let metadata = std::fs::metadata(compressed_path).unwrap();
    let size_bytes = metadata.len();

    // Then: Size < 120 KB
    assert!(size_bytes < 120 * 1024, "Gzipped WASM size {} exceeds 120 KB", size_bytes);
}
```

### 5.5 Test Coverage Targets

| Module | Target Coverage |
|--------|-----------------|
| clawft-wasm/lib.rs | > 80% |
| clawft-wasm/http.rs | > 90% |
| clawft-wasm/fs.rs | > 90% |
| clawft-wasm/allocator.rs | > 70% |
| clawft-wasm/vector.rs | > 85% |

---

## 6. Completion (Phase 3A Milestone Checklist)

### 6.1 Code Deliverables

- [ ] `crates/clawft-wasm/` crate created with WASM entrypoint
- [ ] `WasiHttpClient` implementation complete
- [ ] `WasiFileSystem` implementation complete
- [ ] `WasiEnvironment` implementation complete
- [ ] `talc` allocator integrated as global allocator
- [ ] `micro-hnsw-wasm` integrated for vector search
- [ ] `rvf-wasm` microkernel integrated (optional)
- [ ] Platform abstraction traits in `clawft-core`
- [ ] Feature flags: `wasm`, `micro-hnsw`, `temporal-tensor`, `sona-wasm`
- [ ] Conditional compilation excludes exec tool, channels, CLI from WASM

### 6.2 Build Artifacts

- [ ] `cargo build --target wasm32-wasip2 --profile release-wasm` succeeds
- [ ] `wasm-opt -Oz` optimization applied
- [ ] Binary size ≤ 300 KB uncompressed
- [ ] Binary size ≤ 120 KB gzipped
- [ ] `.cargo/config.toml` configured for WASM target

### 6.3 Testing

- [ ] Unit tests pass for all WASM modules
- [ ] Integration tests pass in Wasmtime runtime
- [ ] Integration tests pass in WAMR runtime
- [ ] Size validation tests pass
- [ ] Test coverage > 80% for core modules

### 6.4 Performance Benchmarks

- [ ] Startup time < 50ms in Wasmtime (JIT)
- [ ] Startup time < 100ms in WAMR (interpreter)
- [ ] Memory RSS < 10 MB for idle agent
- [ ] Vector search: 1000 queries/sec for 10K vectors

### 6.5 Documentation

- [ ] WASM deployment guide (`docs/deployment/wasm.md`)
- [ ] API documentation for WASM exports
- [ ] Size budget tracking document updated
- [ ] Architecture decision records (ADRs) for WASM design choices
- [ ] Code comments for talc allocator initialization
- [ ] Code comments for micro-hnsw integration

### 6.6 Integration Points

- [ ] `clawft-core` compiles for both native and WASM targets
- [ ] Platform traits allow runtime selection of HTTP/FS/Env implementations
- [ ] Session persistence works via WASI filesystem
- [ ] Config loading works via WASI filesystem
- [ ] LLM API calls work via WASI HTTP client

### 6.7 Edge Cases Handled

- [ ] WASM memory limit (16 MB max) does not cause panics
- [ ] talc allocator handles OOM gracefully
- [ ] HTTP timeouts handled correctly in WASI
- [ ] File not found errors handled in WASI filesystem
- [ ] Invalid JSON input handled in agent loop

### 6.8 Phase 3A Exit Criteria

**MUST HAVE**:
1. WASM binary builds successfully for `wasm32-wasip2` target
2. Binary size ≤ 300 KB uncompressed, ≤ 120 KB gzipped
3. All tests pass in Wasmtime and WAMR runtimes
4. HTTP client works with OpenAI/Anthropic APIs
5. Config and session persistence work via WASI filesystem

**SHOULD HAVE**:
6. Vector search integrated (micro-hnsw-wasm)
7. Startup time < 50ms in Wasmtime
8. Memory usage < 10 MB for idle agent

**NICE TO HAVE**:
9. rvf-wasm microkernel integrated
10. Temporal tensor features available

### 6.9 Handoff to Phase 3B

**Artifacts to deliver**:
- `target/wasm32-wasip2/release-wasm/clawft.wasm` (optimized binary)
- `target/wasm32-wasip2/release-wasm/clawft.wasm.gz` (compressed)
- Size profiling report from `twiggy`
- Performance benchmark results
- WASM deployment guide

**Blockers for Phase 3B**:
- None (Phase 3B runs in parallel)

**Integration points**:
- CI/CD pipeline (3B) will build and validate WASM binary
- Binary size assertions (3B) will enforce < 300 KB constraint
- Release pipeline (3B) will publish WASM binary to GitHub Releases

---

## Appendix A: Build Commands

```bash
# Build WASM binary
cargo build -p clawft-wasm \
  --target wasm32-wasip2 \
  --profile release-wasm

# Optimize with wasm-opt
wasm-opt -Oz \
  -o target/wasm32-wasip2/release-wasm/clawft.wasm \
  target/wasm32-wasip2/release-wasm/clawft-wasm.wasm

# Compress with gzip
gzip -9 -c target/wasm32-wasip2/release-wasm/clawft.wasm \
  > target/wasm32-wasip2/release-wasm/clawft.wasm.gz

# Profile binary size
twiggy top -n 20 target/wasm32-wasip2/release-wasm/clawft.wasm
```

## Appendix B: Runtime Testing

```bash
# Test in Wasmtime
wasmtime run target/wasm32-wasip2/release-wasm/clawft.wasm

# Test in WAMR (interpreter)
iwasm target/wasm32-wasip2/release-wasm/clawft.wasm

# Test in WAMR (AOT)
wamrc -o clawft.aot target/wasm32-wasip2/release-wasm/clawft.wasm
iwasm clawft.aot
```

## Appendix C: Size Budget Breakdown

```
Total: ~242 KB
├── clawft-core (agent loop, tools)        100 KB
├── sona (WASM subset, JSON, LLM)           30 KB
├── rvf-types (vector types)                30 KB
├── WASI HTTP client (reqwest or manual)    50 KB
├── micro-hnsw-wasm                         11.8 KB
├── ruvector-temporal-tensor (FFI)          10 KB
├── cognitum-gate-kernel                    10 KB
└── Allocator overhead (talc)                0.2 KB
```
