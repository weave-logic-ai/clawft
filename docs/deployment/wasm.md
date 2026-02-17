# WASM Deployment

clawft compiles to a WebAssembly module targeting `wasm32-wasip2`. The WASM
build provides a lightweight agent core (target: < 300 KB uncompressed,
< 120 KB gzipped) suitable for edge devices, IoT, and browser-based
environments.

## Current Status

The WASM build is functional but uses **platform stubs** for several subsystems:

| Subsystem       | Status                    | Notes                                      |
|-----------------|---------------------------|--------------------------------------------|
| Environment     | Working (in-memory)       | `HashMap`-backed, thread-safe via `Mutex`  |
| HTTP client     | Stub (returns errors)     | Awaiting `wasi:http/outgoing-handler`      |
| Filesystem      | Stub (returns errors)     | Awaiting `wasi:filesystem` stabilisation   |
| Process spawn   | Not available             | No equivalent in WASM environments         |
| Channels        | Not available             | Telegram, Slack, Discord require native I/O|
| Shell tools     | Not available             | `exec_shell`, `spawn` excluded             |

Available tools in WASM: `read_file`, `write_file`, `edit_file`,
`list_directory`, `memory_read`, `memory_write`, `web_fetch`, `web_search`.

Excluded tools: `exec_shell`, `spawn`, `message`.

## Prerequisites

- **Rust 1.93+** with the `wasm32-wasip2` target:

  ```bash
  rustup target add wasm32-wasip2
  ```

- A WASI-compatible runtime. Supported options:
  - [Wasmtime](https://wasmtime.dev/) (recommended, full WASI preview 2)
  - [WAMR](https://github.com/bytecodealliance/wasm-micro-runtime) (minimal footprint, IoT)

## Building

Build the WASM crate from the workspace root:

```bash
cargo build -p clawft-wasm --target wasm32-wasip2 --release
```

The output module is at:

```
target/wasm32-wasip2/release/clawft_wasm.wasm
```

### Size Optimization

The release profile is pre-configured for size (`opt-level = "z"`, LTO,
strip, single codegen unit, `panic = "abort"`). To further reduce size:

```bash
# Install wasm-opt (part of binaryen)
apt install binaryen   # or: brew install binaryen

# Optimize the module
wasm-opt -Oz -o clawft_wasm.opt.wasm target/wasm32-wasip2/release/clawft_wasm.wasm
```

## Running with Wasmtime

```bash
wasmtime run target/wasm32-wasip2/release/clawft_wasm.wasm
```

To grant filesystem access (for config loading, once filesystem stubs are
replaced):

```bash
wasmtime run \
  --dir ~/.clawft::/root/.clawft \
  target/wasm32-wasip2/release/clawft_wasm.wasm
```

Pass environment variables:

```bash
wasmtime run \
  --env OPENAI_API_KEY="sk-..." \
  target/wasm32-wasip2/release/clawft_wasm.wasm
```

## Running with WAMR

```bash
iwasm target/wasm32-wasip2/release/clawft_wasm.wasm
```

WAMR uses less memory than Wasmtime and is suited for resource-constrained
devices. See the [WAMR documentation](https://github.com/bytecodealliance/wasm-micro-runtime)
for embedding in C/C++ applications.

## Platform Limitations

The WASM build excludes components that require native OS features:

1. **No shell execution** -- `exec_shell` and `spawn` tools are not registered.
2. **No messaging channels** -- Telegram, Slack, and Discord require long-lived
   TCP connections and are excluded.
3. **HTTP returns errors** -- The `WasiHttpClient` returns an error for all
   requests until `wasi:http/outgoing-handler` is implemented.
4. **Filesystem returns errors** -- The `WasiFileSystem` returns
   `ErrorKind::Unsupported` for all operations until WASI filesystem APIs are
   wired.
5. **No home directory** -- `home_dir()` returns `None` in WASM environments.
6. **Environment is in-memory** -- Variables set via `set_var` are stored in a
   `HashMap` and do not persist across module restarts.

## Size Budget

| Component    | Target     |
|--------------|------------|
| Uncompressed | < 300 KB   |
| Gzipped      | < 120 KB   |

The `release-wasm` Cargo profile inherits from `release` and uses `opt-level = "z"`
for aggressive size optimisation.

## Future Roadmap

- **WASI HTTP preview2**: Replace the HTTP stub with real outbound requests
  via `wasi:http/outgoing-handler`, enabling LLM API calls from WASM.
- **WASI filesystem**: Replace the filesystem stub with `wasi:filesystem/types`
  and `wasi:filesystem/preopens` for config and session persistence.
- **Browser target**: Add a `wasm32-unknown-unknown` build with `wasm-bindgen`
  for use in web applications.
- **Component model**: Package as a WASI component for composable deployment.
