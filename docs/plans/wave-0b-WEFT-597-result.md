# WEFT-597 result — daemon tracing→ChainManager bridge

**Branch:** `wave0b/weft-597-chain-bridge`  
**Date:** 2026-07-30  
**Status:** Shipped (code + unit tests green)

## Ticket

ws02: daemon tracing→ChainManager bridge — 12 ExoChain events bypass the chain (BUG-3).

Non-kernel crates emit structured `chain_event` markers, but many only hit
stdout via tracing and never reached `ChainManager`, breaking the ADR-022
“all state changes auditable” invariant for daemon-side events.

### Acceptance criteria

| Criterion | Status |
|-----------|--------|
| Wire a tracing layer in `clawft-weave/main.rs` that forwards chain-target events to ChainManager | **Done** |
| The 12 currently-stdout events appear on-chain | **Done** (test-proven via Layer → pending buffer → `ChainManager::append`) |

## What shipped

### 1. `ChainEventLayer` (`crates/clawft-weave/src/chain_bridge.rs`)

Tracing [`Layer`] that:

1. Filters `target == "chain_event"`.
2. Extracts `source`, `kind`, and remaining fields into a JSON payload.
3. Pushes into `clawft_core::chain_event::push_chain_event` **unless**
   `chain_buffered = true` (avoids double-buffering events already pushed by
   the `chain_event!` macro).

`forward_pending_to_chain(&ChainManager)` drains the pending buffer and
appends each record (used by the daemon loop + tests).

### 2. Subscriber wiring (`crates/clawft-weave/src/main.rs`)

Replaced bare `fmt().init()` with:

```text
registry
  + EnvFilter
  + ChainEventLayer
  + fmt::layer
```

Installed once at process start so every weaver subcommand (including the
daemon) has the bridge active.

### 3. Macro flag (`crates/clawft-core/src/chain_event.rs`)

`chain_event!` now:

1. `push_chain_event(...)` first (works without a Layer — library/tests).
2. Emits tracing with `chain_buffered = true` so the Layer skips re-push.

### 4. Daemon drain loop (`crates/clawft-weave/src/daemon.rs`)

Existing 2s drain loop now calls `forward_pending_to_chain` (same semantics,
shared helper, debug log on non-empty drains + final shutdown drain).

## Events fixed (the 12)

Tracing-only sites that previously never entered the pending buffer (Layer
now captures them when the process runs under weaver):

| # | Source | Kind | Call site |
|---|--------|------|-----------|
| 1 | graphify | `graphify.build` | `clawft-graphify/src/build.rs` |
| 2 | graphify | `graphify.pipeline` | `clawft-graphify/src/pipeline.rs` |
| 3 | graphify | `graphify.ingest` | `clawft-graphify/src/ingest.rs` |
| 4 | graphify | `graphify.hook` | `clawft-graphify/src/hooks.rs` (install + uninstall) |
| 5 | weave | `project.init` | `clawft-weave/src/commands/init_cmd.rs` |
| 6 | soul | `soul.promote` (and other soul kinds via same path) | `clawft-weave/src/commands/soul_cmd.rs` |
| 7 | routing | `model_override_bypass` | `clawft-core` `chain_event!` (already buffered; Layer-safe) |

Plus buffered/macro path kinds exercised in the 12-event integration test:

| # | Source | Kind |
|---|--------|------|
| 8 | session | `session.create` |
| 9 | session | `session.destroy` |
| 10 | workspace | `workspace.create` |
| 11 | workspace | `workspace.config` |
| 12 | tools | `tool.register` |

Direct `push_chain_event` path (wasm_fs write/remove/create_dir/copy/move)
already drained by the daemon loop and is unchanged.

## Exceptions / notes

| Case | Disposition |
|------|-------------|
| CLI-only process (no daemon) with Layer installed | Events enter the pending buffer in-process; nothing drains to a live `ChainManager`. Soul CLI still writes a local audit log (pre-existing workaround). |
| `voice.audit` target | Out of scope for this ticket; different target, not `chain_event`. |
| Double-append | Prevented by `chain_buffered = true` on macro emissions. |

## Tests

```bash
cargo test -p clawft-weave --lib chain_bridge   # 4/4
cargo test -p clawft-core --lib chain_event     # 3/3
cargo test -p clawft-core --lib weft31_         # 2/2
cargo test -p clawft-weave --lib                # 128/128 (+1 ignored)
scripts/build.sh check                          # green
```

Key test: `forward_pending_appends_to_chain_manager` emits the 12 kinds via
tracing-only Layer capture, forwards to a real `ChainManager`, and asserts
all kinds appear on-chain.

## Files touched

- `crates/clawft-weave/src/chain_bridge.rs` (new)
- `crates/clawft-weave/src/lib.rs`
- `crates/clawft-weave/src/main.rs`
- `crates/clawft-weave/src/daemon.rs`
- `crates/clawft-core/src/chain_event.rs`
- `docs/plans/wave-0b-WEFT-597-result.md` (this file)
