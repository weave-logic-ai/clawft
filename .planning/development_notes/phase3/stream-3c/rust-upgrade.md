# Stream 3C: Rust Toolchain Upgrade -- Round Summary

**Date**: 2026-02-17
**Stream**: 3C - Rust Toolchain Upgrade
**Status**: In Progress (verification agents running)
**Toolchain**: 1.85 -> 1.93.1

---

## Objective

Upgrade the Rust toolchain from 1.85 to 1.93.1 (current stable) to unblock
wasip2 WASM targets, access new language features, and stay current with the
Rust ecosystem.

---

## Decision Log

### Why 1.93.1?

Three versions were evaluated (1.87.0, 1.91.1, 1.93.1). The decision to go
directly to 1.93.1 was based on:

1. **Identical LOE** -- The effort is the same ~12 minutes (planned) whether
   upgrading to 1.91 or 1.93. Going to 1.87 saves one lint fix but provides
   8 months less runway.
2. **Current stable** -- 1.93.1 is the current Rust stable (released
   2026-02-12). Staying current minimizes future upgrade delta.
3. **Best WASM support** -- Each release improves `wasm32-wasip2` stdlib
   coverage. 1.93 has the most mature wasip2 support.
4. **New language features** -- Let chains (1.88), `File::lock` (1.89),
   `HashMap::extract_if` (1.88) become available.

Full research: `review_rust_update.md`

### `collapsible_if` Fix Approach

All `collapsible_if` warnings were fixed by combining nested `if` statements
into a single `if` with `&&`:

```rust
// BEFORE (triggers collapsible_if):
if condition_a {
    if condition_b {
        do_something();
    }
}

// AFTER:
if condition_a && condition_b {
    do_something();
}
```

**Alternative considered**: Let chains (available since Rust 1.88) could
simplify some patterns, especially those involving `if let`:

```rust
// Let chain alternative (NOT used):
if let Some(x) = maybe_x && x.is_valid() {
    use_x(x);
}
```

Let chains were not used because:
- The existing patterns did not involve `if let` -- they were plain `if`
  conditions, so let chains would not apply.
- Keeping the simpler `&&` combination is more familiar to contributors
  who may not yet be accustomed to let chain syntax.
- Let chains can be adopted incrementally in future code as the team
  becomes comfortable with them.

### `derivable_impls` Fix

The manual `Default` impl for `PolicyMode` was replaced with `#[derive(Default)]`
and `#[default]` attribute on the `Allowlist` variant. This is the standard
clippy-recommended approach and preserves identical behavior.

---

## Files Changed

### Config Files (2)

| File | Change |
|------|--------|
| `clawft/rust-toolchain.toml` | `channel = "1.85"` -> `channel = "1.93"` |
| `clawft/Cargo.toml` | `rust-version = "1.85"` -> `rust-version = "1.93"` |

### Clippy Fixes -- Originally Planned (3 files, 3 fixes)

| File | Lint | Instances |
|------|------|-----------|
| `clawft-cli/src/markdown/slack.rs` | `let_and_return` | 1 |
| `clawft-cli/src/markdown/telegram.rs` | `let_and_return` | 1 |
| `clawft-tools/src/security_policy.rs` | `derivable_impls` | 1 |

### Clippy Fixes -- Discovered During Execution (9 files, 18 fixes)

| File | Lint | Instances |
|------|------|-----------|
| `clawft-types/src/config.rs` | `collapsible_if` | 1 |
| `clawft-types/src/provider.rs` | `collapsible_if` | 6 |
| `clawft-types/src/session.rs` | `collapsible_if` | 1 |
| `clawft-core/src/agent/skills.rs` | `collapsible_if` | 1 |
| `clawft-core/src/pipeline/transport.rs` | `collapsible_if` | 1 |
| `clawft-core/src/pipeline/llm_adapter.rs` | `collapsible_if` | 1 |
| `clawft-channels/src/discord/channel.rs` | `collapsible_if` | 5 |
| `clawft-channels/src/host.rs` | `collapsible_if` | 1 |
| `clawft-channels/src/slack/channel.rs` | `collapsible_if` | 1 |

### CI Workflow (1)

| File | Change |
|------|--------|
| `.github/workflows/wasm-build.yml` | Remove `toolchain: "1.85"` pin, switch to wasip2 primary + wasip1 fallback |

### Summary

| Category | Files | Fixes |
|----------|-------|-------|
| Config | 2 | 2 |
| let_and_return | 2 | 2 |
| derivable_impls | 1 | 1 |
| collapsible_if | 9 | 18 |
| CI workflow | 1 | 1 |
| **Total** | **15** | **24** |

---

## Why the Research Underestimated Clippy Changes

The pre-upgrade research (`review_rust_update.md`) identified only 3 clippy
warnings, but execution revealed 14 total (3 original + 11 additional
`collapsible_if`).

**Root cause**: `rustup run 1.93.1 cargo clippy` reused cached build artifacts
from the 1.85 build in `target/`. Clippy only analyzes crates that are
recompiled -- cached crates are skipped. Most workspace crates were already
compiled under 1.85, so the stricter `collapsible_if` lint was only applied to
the few crates that happened to need recompilation.

**Mitigation for future upgrades**: Always run `cargo clean` before testing
clippy on a new Rust version.

---

## Verification Status

| Check | Status |
|-------|--------|
| `cargo check --workspace` | PASS |
| `cargo clippy --workspace -- -D warnings` | PENDING (verification agents) |
| `cargo test --workspace` | PENDING (verification agents) |
| `cargo fmt --all -- --check` | PENDING (verification agents) |
| `cargo check -p clawft-wasm --target wasm32-wasip2` | PENDING (verification agents) |
| `cargo check -p clawft-wasm --target wasm32-wasip1` | PENDING (verification agents) |
| `cargo build -p clawft-wasm --target wasm32-wasip2 --release` | PENDING (verification agents) |

---

## Impact on Other Streams

### Stream 3A (WASM Platform)

The Rust 1.93.1 upgrade unblocks `wasm32-wasip2` which was previously blocked
on Rust 1.87+. This means:
- `wasm32-wasip2` becomes the primary WASM target (was `wasip1`)
- Real WASI HTTP preview2 and WASI FS preview2 implementations are now possible
- The CI workflow has been updated to check both `wasip2` (primary) and
  `wasip1` (fallback)

### Stream 3B (CI/CD)

The `wasm-build.yml` workflow was updated to remove the hardcoded `toolchain: "1.85"`
pin and use `@stable` instead, which respects `rust-toolchain.toml`. No other
workflow changes were needed -- `ci.yml`, `release.yml`, and `benchmarks.yml`
already used `@stable`.
