# ADR-083: Browser WASM Support

- **Status**: Accepted (retroactive for landed W-BROWSER scope; 2026-07-31)
- **Closes**: WEFT-393
- **Related**: ADR-044 (WASI `wasm32-wasip2` target), ADR-055 (`BackendAdapter`
  / `WasmAdapter` dashboard mode), ADR-071 (WASM panel auth — webview, not
  this path), [docs/architecture/wasm-browser-portability-analysis.md](../architecture/wasm-browser-portability-analysis.md),
  [docs/architecture/wasm-bundle-size.md](../architecture/wasm-bundle-size.md),
  [docs/browser/](../browser/), `.planning/wasm-browser/00-consensus-plan.md`,
  `.planning/reviews/0.7.0-release-gate/16-browser-wasm.md`
- **Numbering note**: Planning originally reserved **ADR-027** for this topic.
  Slot `docs/adr/adr-027-*.md` is already **ADR-027 Selective libp2p**
  (unrelated). This record is **ADR-083**. Do not renumber 027.

## Context

WeftOS / clawft must run an agent loop inside a web browser tab (playground,
static demos, `clawft-ui` browser-only mode) without a local daemon. The
workspace already had:

1. A **WASI** WASM path (`clawft-wasm` → `wasm32-wasip2`, ADR-044) for
   edge/server hosts under wasmtime.
2. A **Platform trait** in `clawft-platform` that models HTTP / FS / env /
   process seams.
3. An early `clawft-wasm` crate that was **decoupled** from
   `clawft-core` / `clawft-platform` and returned stubs instead of the real
   pipeline.

The W-BROWSER program (2026-02 consensus + BW1–BW6 implementation) had to
choose how much of the native stack to re-host in the browser, how to keep
native builds unharmed, and how to ship artifacts. Those decisions landed in
code, CI, and `docs/browser/*` but were never recorded as an ADR. WEFT-393
(0.7.0 release-gate audit, ws16 P1.8) requires that record.

Primary analysis source:
[docs/architecture/wasm-browser-portability-analysis.md](../architecture/wasm-browser-portability-analysis.md).

## Decision

### 1. Hybrid architecture (not full port, not thin client)

**Adopt a hybrid model:**

| Approach | Description | Verdict |
|----------|-------------|---------|
| **Full port** | Every crate (channels, services, CLI, plugins, shell) compiles to browser WASM | Rejected — native I/O, multi-threaded Tokio, process spawn, and channel transports do not map to the browser sandbox; binary size and CORS make it product-hostile |
| **Thin client** | Pure TypeScript SPA calling a remote kernel (openbrowserclaw-style) | Rejected as the *only* browser story — loses offline demos, local API-key demos, and “real pipeline in the tab” product value; server-attached mode already exists via ADR-055 `axum` |
| **Hybrid (chosen)** | Real `AgentLoop` + pipeline + browser-safe tools in WASM; exclude channels / cron / CLI / native plugins; JS UI talks via wasm-bindgen | **Accepted** |

Concretely:

- **In browser WASM**: `clawft-types`, `clawft-platform` (browser impls),
  `clawft-core` (agent loop + pipeline stages), `clawft-llm` (browser
  transport), `clawft-tools` (browser-safe subset), `clawft-wasm`
  (`browser_entry` shim).
- **Out of browser WASM**: `clawft-cli`, `clawft-channels`, long-running
  `clawft-services` host roles, shell/spawn tools, native plugin crates,
  multi-threaded Tokio runtime.
- **JS host** owns chat UI, encrypted API-key storage (Web Crypto + IndexedDB;
  see ADR-055 §4), and optional playground chrome.
- **Entry surface**: wasm-bindgen exports `init` / `send_message` / `set_env`
  (plus sprint-16 side exports such as `analyze_files` / `boot_info`), not
  `fn main()`.

This reconnects `clawft-wasm` to the real `Platform` trait instead of a
parallel stub platform (the gap called out in the portability analysis).

### 2. Two distinct WASM targets (do not conflate)

| Target | Triple | Build entry | Runtime | ADR |
|--------|--------|-------------|---------|-----|
| **Browser** | `wasm32-unknown-unknown` | `scripts/build.sh browser` → `clawft-wasm` with `--features browser` | Browser / JS via wasm-bindgen | **This ADR** |
| **WASI** | `wasm32-wasip2` (canonical; wasip1 secondary) | `scripts/build.sh wasi` | wasmtime / WASI hosts | ADR-044 |

Browser builds use the `release-wasm` profile and wasm-bindgen (or
wasm-pack) for JS glue. WASI builds produce a standalone
`clawft_wasm.wasm` without browser FFI. Feature gates and platform
impls differ; sharing only the pure computation layers is intentional.

### 3. Feature-flag mutex: `native` ⊥ `browser`

Across the portable crate chain (`clawft-types`, `clawft-platform`,
`clawft-core`, `clawft-llm`, `clawft-tools`, `clawft-plugin`,
`clawft-wasm`):

- **`native`** (default on host crates): Tokio multi-thread, `reqwest` with
  native TLS, `dirs`, `notify`, process spawn, full tool surface.
- **`browser`**: `wasm-bindgen` / `web-sys` / `js-sys`,
  `BrowserPlatform`, `BrowserLlmClient`, relaxed `Send` (below), browser-safe
  tools only.
- **Mutex**: A crate build is **either** native **or** browser for a given
  compilation unit — never both. Browser builds always use
  `--no-default-features --features browser` (and workspace deps
  `default-features = false`) so host-only crates do not pull Tokio/TLS into
  the WASM graph.
- **Validation**: `scripts/build.sh gate` (canonical phase gate; WEFT-409 /
  WEFT-564 retired the former `scripts/check-features.sh` shim) and its
  per-crate `cargo check --target wasm32-unknown-unknown … --features browser`
  matrix enforce the split. Accidental `default-features = true` on a
  portable path is a gate failure.

Optional sub-feature (does not break the mutex):

| Feature | Role |
|---------|------|
| `browser` | Required for browser triple + agent loop in-tab |
| `browser-opfs` | Implies `browser`; OPFS-backed FS + env (WEFT-13 / WEFT-14 / WEFT-392) |
| `wasm-plugins` | Host-side wasmtime plugin path (not the browser product surface) |

### 4. OPFS: deferred for MVP, then opt-in

**Original W-BROWSER decision (BW4):** ship `BrowserFileSystem` /
`BrowserEnvironment` as **in-memory** `HashMap` backends so the MVP agent
loop and compile gate could land without blocking on web-sys OPFS maturity
and secure-context constraints. Docs and module headers recorded OPFS as
planned.

**Later decision (WEFT-392 / WEFT-13 / WEFT-14):** land OPFS behind
**`browser-opfs`**, not as the unconditional default:

- Virtual home `/clawft` (workspace + `.clawft/` config).
- Runtime fallback to memory if OPFS is missing or the page is not a secure
  context.
- Stable web-sys FileSystem* APIs (verified ≥ 0.3.70; no
  `web_sys_unstable_apis` cfg required for the shipped path).
- Default `scripts/build.sh browser` remains `--features browser` (memory
  FS) for the smallest default playground path; enable OPFS explicitly when
  persistence is required.

In-memory remains a valid deployment mode (session-only demos). OPFS is the
supported persistence path when the feature is enabled.

### 5. `async_trait(?Send)` tax (browser-only)

Browser WASM is single-threaded; JS promises and `wasm-bindgen-futures` do
not need `Send`. Native multi-threaded Tokio **does**.

**Rule (mandatory):**

```rust
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
```

Apply the same pattern on `Platform`, `Tool`, and related async traits /
impls. **Never** apply bare `async_trait(?Send)` without the
`cfg_attr` split — that silently weakens native Send guarantees (risk N4 in
the consensus plan).

Related polyfills for the browser graph: channel / lock wrappers
(`futures_channel` unbounded mpsc, `std::sync::RwLock` thin async wrapper
where Tokio locks are gated), `CancellationToken` polyfill in
`clawft-plugin`, and time sources that use `js_sys::Date` under
`target_arch = "wasm32"`.

### 6. CORS-proxy convention

LLM providers mostly block browser-origin `fetch`. The browser transport
supports **per-provider** configuration on `ProviderConfig`:

| Field | Meaning |
|-------|---------|
| `cors_proxy: Option<String>` | When set and not in direct mode, rewrite/request via proxy (`{proxy}/{target}` style resolution in `BrowserLlmClient`) |
| `browser_direct: bool` | When true, call the provider origin directly (e.g. Anthropic with `anthropic-dangerous-direct-browser-access`) |

Operational rules:

- Prefer **allowlisted** CORS proxies; never document an open relay as a
  production default (API keys transit the proxy).
- Local providers (Ollama / LM Studio with CORS configured) typically need
  neither proxy nor dangerous headers.
- Full recipes live under [docs/browser/deployment.md](../browser/deployment.md);
  a dedicated per-provider guide remains a docs gap (audit P3.6), not a
  second architecture.

### 7. Signing / release model

Browser WASM is a **first-class release artifact**, independent of
cargo-dist’s native matrix and of the WASI workflow:

| Mechanism | Behavior |
|-----------|----------|
| **CI build** | `.github/workflows/wasm-browser.yml` — `cargo build` for `wasm32-unknown-unknown` + `release-wasm` + `--features browser`, then `wasm-bindgen-cli` (pinned version in workflow) → `browser-pkg/` |
| **PR / gate** | `wasm-browser-check` in `pr-gates.yml`; `scripts/build.sh gate` checks the six portable crates on the browser triple |
| **Artifact** | Upload `browser-pkg/` (and tarball `clawft-browser-wasm-<tag>.tar.gz` on version tags) via GitHub Actions artifacts / `gh release upload` |
| **Size budget** | `scripts/build.sh bundle-size` / WEFT-389 gate on post-bindgen `_bg.wasm` (raw + gzip thresholds documented in [wasm-bundle-size.md](../architecture/wasm-bundle-size.md)); original &lt;300 KB raw aspirational target superseded by realistic agent-loop budget |
| **Signing / versioning (WEFT-405)** | Same root of trust as cargo-dist natives and WASI: `VERSION.json` (tag + git SHA + workspace version) embedded in and beside the tarball; detached `*.sha256` + `SHA256SUMS`; GitHub Attestations via `actions/attest-build-provenance` (sigstore; `gh attestation verify`). Shared helper: `scripts/release/package-wasm-artifact.sh`. No separate browser-only code-signing root. Skill / panel capability signing (ADR-071) is a different surface. Verify: [docs/browser/verification.md](../browser/verification.md). |
| **Local** | `scripts/build.sh browser`, optional `scripts/build.sh serve`, wasm-pack workflow in [docs/browser/building.md](../browser/building.md) |

Dashboard integration reuses the same bindgen package through
`WasmAdapter` (ADR-055) when `mode=wasm`.

## Consequences

### Positive

- One agent pipeline for native and browser; no second TypeScript agent core
  to maintain for demos and offline playgrounds.
- Feature mutex + gate matrix prevent Tokio/TLS from poisoning the WASM
  graph and prevent `?Send` from poisoning native Send.
- Clear split from WASI (ADR-044) avoids “one WASM to rule them all”
  confusion in CI and docs.
- OPFS as opt-in keeps default bundle and secure-context constraints
  manageable while offering durable FS/env when needed.
- Release workflow ships a reproducible browser package consumable by the
  docs playground and `clawft-ui`.

### Negative

- Dual feature graphs increase compile-matrix cost (native + wasi + browser
  checks on every gate).
- `async_trait` cfg dualism is easy to get wrong on new traits; requires
  discipline and feature-check scripts.
- Browser capability set will always lag axum-backed mode (no channels,
  cron, multi-user, shell) — product messaging must not claim parity
  (ADR-055 capabilities flags encode this).
- CORS and API-key-in-browser remain inherent threat-model constraints;
  hybrid mode is not a substitute for a trusted gateway when keys must not
  touch the client.

### Neutral / follow-ups

- Bundle-size budget is calibrated to the full agent loop, not the early
  300 KB aspirational figure (see wasm-bundle-size.md).
- wasm-bindgen-test / headless browser regression
  (`scripts/build.sh test-browser`) continues to expand coverage.
- Panel webview auth (ADR-071) and substrate publish gates (ADR-072) apply
  to the VS Code / egui shell path, not the static playground bindgen
  package — keep those threat models separate.

## Alternatives considered

1. **TypeScript-only thin client** — faster UI iteration, zero Rust WASM
   toolchain; rejected as sole path because the product goal was “real
   clawft pipeline in the tab,” and axum mode already covers server-backed
   UIs.
2. **New `clawft-platform-browser` crate** instead of feature gates —
   cleaner crate graph at the cost of trait duplication and publish
   churn; rejected in favor of feature flags on existing crates (consensus
   plan).
3. **WASI-in-browser (wasmtime-wasi in the tab)** — possible via
   wasmtime-browser experiments; rejected for product path due to size,
   incomplete browser WASI, and existing wasm-bindgen investment.
4. **Always-on OPFS** — rejected for MVP and for default builds; secure
   context + API availability + size/testing cost favor opt-in
   `browser-opfs`.

## Implementation map (landed)

| Area | Location |
|------|----------|
| Browser entry | `crates/clawft-wasm` (`browser` / `browser-opfs` features) |
| Platform impls | `crates/clawft-platform/src/browser/` |
| LLM transport | `clawft-llm` browser transport + CORS fields |
| Tools | `clawft-tools` with shell/spawn excluded under `browser` |
| Build | `scripts/build.sh browser`, `test-browser`, `bundle-size`, `gate` |
| CI | `.github/workflows/wasm-browser.yml`, `pr-gates.yml` wasm-browser-check |
| Operator docs | `docs/browser/{architecture,building,deployment,quickstart,api-reference}.md` |
| UI adapter | `clawft-ui` `WasmAdapter` / `?mode=wasm` (ADR-055) |

## Status

**Accepted** retroactively for the hybrid W-BROWSER stack already shipping
in tree. Future changes that reverse the hybrid choice, collapse browser
and WASI triples, or remove the native⊥browser mutex require a superseding
ADR.
