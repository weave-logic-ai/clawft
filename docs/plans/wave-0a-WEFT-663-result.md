# WEFT-663 result — clawft-core browser Send-future / local_file_sink

**Ticket:** WEFT-663  
**Branch:** `wave0a/weft-663-local-file-sink-wasm`  
**Wave:** 0a  
**Date:** 2026-07-30

## Problem

`cargo check -p clawft-core --no-default-features --features browser --target wasm32-unknown-unknown` failed with **10** `future cannot be sent between threads safely` errors, all in `agent/local_file_sink.rs`.

Root cause: browser `clawft_platform::FileSystem` methods return `!Send` futures (`async_trait(?Send)`), but `ConversationSink` and `LocalFileSink`'s impl used plain `#[async_trait]`, which requires `Send` futures. Awaiting platform FS inside sink methods therefore failed the trait's Send cast.

## Fix

Match the established browser pattern (`pipeline/traits.rs` `LlmTransport`, `tools/registry.rs` `Tool`, platform traits):

```rust
#[cfg_attr(not(feature = "browser"), async_trait)]
#[cfg_attr(feature = "browser", async_trait(?Send))]
```

Applied to:

| Site | File |
|------|------|
| `ConversationSink` trait | `crates/clawft-core/src/agent/sink.rs` |
| `InMemorySink` impl | `crates/clawft-core/src/agent/sink.rs` |
| `LocalFileSink<P>` impl | `crates/clawft-core/src/agent/local_file_sink.rs` |

Native keeps strict `Send` futures for multi-threaded tokio. Browser relaxes only the method-future Send bound; trait object still requires `Send + Sync + 'static` on the sink type itself.

## Verification

### WASM (WEFT-663 scope)

```bash
cargo check -p clawft-core --no-default-features --features browser --target wasm32-unknown-unknown
```

| Metric | Before | After |
|--------|--------|-------|
| Send-future errors (`local_file_sink`) | **10** | **0** |
| Cascading E0282 from those Send errors | 4 | 0 |
| Remaining compile errors in clawft-core | — | **1** E0433 + 4 E0282 cascade from WEFT-672 |

Remaining failure is **WEFT-672** (out of scope): `pipeline/transport.rs` calls `clawft_llm::hermes::strip_think` while `hermes` is `#[cfg(feature = "native")]` only. Not caused by this change.

### Native

```bash
scripts/build.sh check   # cargo check --workspace — OK
cargo test -p clawft-core --lib local_file_sink   # 17 passed
cargo test -p clawft-core --lib sink::            # 28 passed (sink + local_file_sink filter overlap)
```

## Files changed

- `crates/clawft-core/src/agent/sink.rs`
- `crates/clawft-core/src/agent/local_file_sink.rs`
- `docs/plans/wave-0a-WEFT-663-result.md` (this file)

## Follow-ups

- **WEFT-672** — gate or provide browser-safe `strip_think` so clawft-core browser check is fully green.
- After 663+672 green: consider promoting the gate's browser matrix from soft to hard (ticket note).
- Native `SubstrateConversationSink` / weave test sinks keep plain `#[async_trait]`; they only build under native where the trait still requires Send futures.

## Commit

See git log on `wave0a/weft-663-local-file-sink-wasm`.
