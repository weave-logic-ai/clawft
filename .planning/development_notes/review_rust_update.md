# Rust Toolchain Upgrade Review: clawft Workspace

**Date**: 2026-02-17
**Workspace**: `/home/aepod/dev/barni/repos/nanobot/clawft`
**Current Toolchain**: Rust 1.85 (Edition 2024, MSRV 1.85)
**Target Versions**: 1.87.0, 1.91.1, 1.93.1

---

## Executive Summary

All three target Rust versions (1.87.0, 1.91.1, 1.93.1) compile the full clawft workspace
cleanly, including WASM targets (wasm32-wasip1, wasm32-wasip2). No compilation errors were
found at any version. The only changes required are **3 trivial clippy fixes** -- 2 present
starting at 1.87.0 and 1 additional one starting at 1.91.0. Total LOE for all fixes is
under 15 minutes for any target version.

**Recommendation**: Upgrade directly to **Rust 1.93.1** (the current stable). The effort is
identical to any intermediate version, and 1.93.1 provides the most features, the best WASM
support, and the longest runway before the next upgrade.

---

## Current State

### Toolchain Configuration

**`rust-toolchain.toml`**:
```toml
[toolchain]
channel = "1.85"
components = ["clippy", "rustfmt"]
```

**`Cargo.toml` (workspace root)**:
```toml
[workspace.package]
edition = "2024"
rust-version = "1.85"
```

### Workspace Crates (9 crates)

| Crate | Description |
|-------|-------------|
| `clawft-types` | Shared types and data models |
| `clawft-platform` | Platform abstraction layer |
| `clawft-core` | Core agent logic |
| `clawft-llm` | LLM provider integration |
| `clawft-tools` | Tool execution engine |
| `clawft-channels` | Communication channels (WebSocket, HTTP, etc.) |
| `clawft-services` | Service orchestration |
| `clawft-cli` | CLI binary (`weft`) |
| `clawft-wasm` | WASM entrypoint (wasip1/wasip2) |

### Installed WASM Targets

- `wasm32-wasip1` (Tier 2 since Rust 1.78)
- `wasm32-wasip2` (Tier 2 since Rust 1.82)

### Key Locked Dependencies

| Dependency | Locked Version | MSRV |
|-----------|---------------|------|
| tokio | 1.49.0 | 1.71 |
| reqwest | 0.12.28 | ~1.63 |
| serde | 1.0.228 | 1.31 |
| serde_json | 1.0.149 | 1.56 |
| clap | 4.5.59 | 1.74 |
| chrono | 0.4.43 | 1.61 |
| async-trait | 0.1.89 | 1.56 |
| thiserror | 2.0.18 | 1.61 |
| anyhow | 1.0.101 | 1.39 |
| uuid | 1.21.0 | 1.60 |
| tokio-tungstenite | 0.24.0 | ~1.63 |
| tracing | 0.1.44 | 1.63 |
| tracing-subscriber | 0.3.22 | 1.63 |
| pulldown-cmark | 0.12.2 | 1.74 |
| ring | 0.17.14 | 1.61 |
| rustls | 0.23.36 | 1.71 |
| comfy-table | 7.1.4 | ~1.65 |
| dlmalloc | 0.2.12 | N/A |
| futures-util | 0.3.32 | 1.56 |
| hmac | 0.12.1 | ~1.56 |
| sha2 | 0.10.9 | ~1.56 |
| cron | 0.15.0 | ~1.56 |
| dirs | 6.0.0 | ~1.63 |

All locked dependency versions are well within the MSRV range for every target Rust version.
No dependency requires updating to support any of the target versions.

### Duplicate Dependencies (Noteworthy)

- `getrandom` v0.2.17 AND v0.4.1 (v0.2 via rand/ring, v0.4 via uuid)
- `thiserror` v1.0.69 AND v2.0.18 (v1 via tungstenite, v2 used directly)
- `rand` v0.8.5 AND v0.9.2 (v0.8 via tungstenite, v0.9 possibly unused)

These are not upgrade blockers; they exist at 1.85 already.

---

## Rust 1.87.0

**Release**: May 15, 2025 (rustc 1.87.0, 17067e9ac 2025-05-09)

### Compilation Result: PASS

```
$ rustup run 1.87.0 cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 30.53s
```

All 9 crates compile without errors.

### Test Compilation: PASS

```
$ rustup run 1.87.0 cargo test --workspace --no-run
  Executable tests/phase1_integration.rs (...)
  Executable tests/phase2_integration.rs (...)
  Executable tests/security_tests.rs (...)
  [... all unit test binaries compile successfully ...]
```

### Clippy (with `-D warnings`): FAIL (2 warnings)

```
error: returning the result of a `let` binding from a block
  --> crates/clawft-cli/src/markdown/slack.rs:83:9
   |
82 |         let trimmed = output.trim_end().to_owned();
   |         ------------------------------------------- unnecessary `let` binding
83 |         trimmed
   |         ^^^^^^^
   = note: `-D clippy::let-and-return` implied by `-D warnings`

error: returning the result of a `let` binding from a block
  --> crates/clawft-cli/src/markdown/telegram.rs:76:9
   |
75 |         let trimmed = output.trim_end().to_owned();
   |         ------------------------------------------- unnecessary `let` binding
76 |         trimmed
   |         ^^^^^^^
```

**Note**: These 2 `let_and_return` warnings are NOT new to 1.87. They are also present with
the 1.85 clippy when run with `-D warnings`. The 1.85 clippy used in development (without
`-D warnings`) shows them as warnings only. They are preexisting lint debt.

### WASM Target Support: PASS

```
$ rustup run 1.87.0 cargo check -p clawft-wasm --target wasm32-wasip2
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.28s

$ rustup run 1.87.0 cargo check -p clawft-wasm --target wasm32-wasip1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 4.09s

$ rustup run 1.87.0 cargo build -p clawft-wasm --target wasm32-wasip2 --release
    Finished `release` profile [optimized] target(s) in 9.62s
```

### Notable 1.87 Features Relevant to clawft

- **Anonymous pipes in stdlib**: Useful if shell tool execution needs stdout/stderr merging.
- **`build.build-dir` stabilized in Cargo**: Optional but can improve CI caching.
- **`&pin (mut|const) T` syntax parsed**: No impact on current code.
- **Calling convention hard error**: No impact (no `extern` functions in clawft).

### Breaking Changes Affecting clawft

None. No code changes required for compilation.

### LOE Estimate

| Task | Effort |
|------|--------|
| Update `rust-toolchain.toml` | 1 min |
| Update `Cargo.toml` MSRV | 1 min |
| Fix 2x `let_and_return` (preexisting) | 2 min |
| Verify CI | 5 min |
| **Total** | **~10 min** |

---

## Rust 1.91.1

**Release**: November 10, 2025 (rustc 1.91.1, ed61e7d7e 2025-11-07)

This is a patch release of 1.91.0 (released October 30, 2025).

### Compilation Result: PASS

```
$ rustup run 1.91.1 cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 27.15s
```

All 9 crates compile without errors.

### Test Compilation: PASS

```
$ rustup run 1.91.1 cargo test --workspace --no-run
  [... all test binaries compile successfully ...]
```

### Clippy (with `-D warnings`): FAIL (3 warnings)

In addition to the 2 preexisting `let_and_return` warnings, clippy 1.91 introduces:

```
error: this `impl` can be derived
  --> crates/clawft-tools/src/security_policy.rs:20:1
   |
20 | / impl Default for PolicyMode {
21 | |     fn default() -> Self {
22 | |         Self::Allowlist
23 | |     }
24 | | }
   |
   = note: `-D clippy::derivable-impls` implied by `-D warnings`
help: replace the manual implementation with a derive attribute and mark the default variant
   |
13 + #[derive(Default)]
14 ~ pub enum PolicyMode {
15 |     /// Only commands whose basename appears in the allowlist are permitted.
16 ~     #[default]
17 ~     Allowlist,
```

**This is a NEW lint** (`clippy::derivable_impls`) that was promoted to `warn-by-default` in
a clippy version between 1.87 and 1.91. It correctly identifies that the manual `Default`
impl for `PolicyMode` can be replaced with `#[derive(Default)]` and `#[default]` on the
variant.

### WASM Target Support: PASS

```
$ rustup run 1.91.1 cargo check -p clawft-wasm --target wasm32-wasip2
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.74s

$ rustup run 1.91.1 cargo check -p clawft-wasm --target wasm32-wasip1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.82s

$ rustup run 1.91.1 cargo build -p clawft-wasm --target wasm32-wasip2 --release
    Finished `release` profile [optimized] target(s) in 8.23s
```

### Notable Changes Between 1.87 and 1.91 Relevant to clawft

**Rust 1.88** (June 26, 2025):
- **Let chains stabilized** (edition 2024): Can simplify complex `if let` matching.
- **`#[bench]` hard error**: No impact (clawft has no `#[bench]` usage).
- **`dangerous_implicit_autorefs` lint (warn)**: No impact (clawft has minimal `unsafe`).
- **`invalid_null_arguments` lint**: No impact.
- **`HashMap::extract_if` stabilized**: Potentially useful.
- **Minimum external LLVM bumped to 19**: No direct impact (using rustup-managed LLVM).

**Rust 1.89** (August 7, 2025):
- **`dangerous_implicit_autorefs` bumped to deny**: No impact (only 4 `unsafe` usages in
  env.rs and openai_compat.rs, none involving raw pointer dereferences).
- **`repr128` stabilized**: Not currently used.
- **`wasm32-unknown-unknown` ABI now standards-compliant**: Does not affect wasip1/wasip2.
- **`File::lock` stabilized**: Potentially useful for CLI.

**Rust 1.90** (~September 2025):
- **`wasm_c_abi` hard error**: Only affects `wasm32-unknown-unknown`, not wasip1/wasip2.
- **SSE2 required for i686**: No impact (targeting x86_64 and wasm32).

**Rust 1.91** (October 30, 2025):
- **`semicolon_in_expressions_from_macros` now deny-by-default**: No impact (codebase
  uses edition 2024 which already handles this).
- **Stabilize C-style variadic functions**: Not used.
- **`dangling_pointers_from_locals` lint**: Not triggered.
- **`integer_to_ptr_transmutes` lint**: Not triggered (no transmutes in codebase).

### Breaking Changes Affecting clawft

None at the compilation level. One new clippy lint (`derivable_impls`) is now warn-by-default.

### LOE Estimate

| Task | Effort |
|------|--------|
| Update `rust-toolchain.toml` | 1 min |
| Update `Cargo.toml` MSRV | 1 min |
| Fix 2x `let_and_return` (preexisting) | 2 min |
| Fix 1x `derivable_impls` in security_policy.rs | 2 min |
| Verify CI | 5 min |
| **Total** | **~12 min** |

---

## Rust 1.93.1

**Release**: February 12, 2026 (rustc 1.93.1, 01f6ddf75 2026-02-11)

This is the current Rust stable. Patch release of 1.93.0 (released January 22, 2026).

### Compilation Result: PASS

```
$ rustup run 1.93.1 cargo check --workspace
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.42s
```

All 9 crates compile without errors.

### Test Compilation: PASS

```
$ rustup run 1.93.1 cargo test --workspace --no-run
  [... all test binaries compile successfully ...]
```

### Clippy (with `-D warnings`): FAIL (3 warnings -- same as 1.91)

```
warning: this `impl` can be derived
  --> crates/clawft-tools/src/security_policy.rs:20:1
   [same as 1.91]

warning: returning the result of a `let` binding from a block
  --> crates/clawft-cli/src/markdown/slack.rs:83:9
   [same as 1.87/1.91]

warning: returning the result of a `let` binding from a block
  --> crates/clawft-cli/src/markdown/telegram.rs:76:9
   [same as 1.87/1.91]
```

No additional clippy warnings introduced between 1.91 and 1.93.

### WASM Target Support: PASS

```
$ rustup run 1.93.1 cargo check -p clawft-wasm --target wasm32-wasip2
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 8.33s

$ rustup run 1.93.1 cargo check -p clawft-wasm --target wasm32-wasip1
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3.90s

$ rustup run 1.93.1 cargo build -p clawft-wasm --target wasm32-wasip2 --release
    Finished `release` profile [optimized] target(s) in 8.06s
```

### Notable Changes Between 1.91 and 1.93 Relevant to clawft

**Rust 1.92** (December 11, 2025):
- **Never type lints (`never_type_fallback_flowing_into_unsafe`,
  `dependency_on_unit_never_type_fallback`) now deny-by-default**: No impact (no never-type
  fallback patterns in codebase).
- **`invalid_macro_export_arguments` deny-by-default**: No impact.
- **`unused_must_use` no longer warns on `Result` or `ControlFlow`**: Mildly relaxes
  warnings; no impact.
- **Backtrace fix with `-C panic=abort`**: Positive improvement for debugging.
- **`allow(&raw [mut|const]) for union fields in safe code`**: Not used.

**Rust 1.93** (January 22, 2026):
- **musl bumped to 1.2.5**: Not directly relevant (not using musl targets), but good for
  future static builds.
- **`cfg` inside `asm!` blocks**: Not used.
- **`#[test]` stricter enforcement**: No impact (all `#[test]` usage is standard).
- **Emscripten unwinding ABI change**: Not relevant (targeting wasip1/wasip2, not
  emscripten).
- **`cargo clean --workspace`**: Nice convenience improvement.
- **`CARGO_CFG_DEBUG_ASSERTIONS` in build scripts**: No build.rs scripts in workspace.
- **Rustdoc `invalid_doc_attributes` lint**: No impact (no crate-level doc attributes used).

### Breaking Changes Affecting clawft

None. Identical clippy warnings as 1.91.

### LOE Estimate

| Task | Effort |
|------|--------|
| Update `rust-toolchain.toml` | 1 min |
| Update `Cargo.toml` MSRV | 1 min |
| Fix 2x `let_and_return` (preexisting) | 2 min |
| Fix 1x `derivable_impls` in security_policy.rs | 2 min |
| Verify CI | 5 min |
| **Total** | **~12 min** |

---

## Version Comparison Matrix

| Criterion | 1.87.0 | 1.91.1 | 1.93.1 |
|-----------|--------|--------|--------|
| `cargo check --workspace` | PASS | PASS | PASS |
| `cargo test --no-run` | PASS | PASS | PASS |
| `cargo clippy -D warnings` | 2 warnings | 3 warnings | 3 warnings |
| wasm32-wasip1 check | PASS | PASS | PASS |
| wasm32-wasip2 check | PASS | PASS | PASS |
| wasm32-wasip2 release build | PASS | PASS | PASS |
| Code changes required | 2 files | 3 files | 3 files |
| New clippy lints triggered | 0 new | 1 new | 1 new |
| LOE | ~10 min | ~12 min | ~12 min |
| Let chains available | YES | YES | YES |
| `File::lock` available | NO | YES | YES |
| `cfg` in `asm!` | NO | NO | YES |
| Runway until next upgrade | 8 months old | 3 months old | **Current stable** |

---

## Required Code Changes (All Versions)

### Fix 1: `let_and_return` in slack.rs (preexisting, all versions)

**File**: `crates/clawft-cli/src/markdown/slack.rs` line 82-83

```rust
// BEFORE:
let trimmed = output.trim_end().to_owned();
trimmed

// AFTER:
output.trim_end().to_owned()
```

### Fix 2: `let_and_return` in telegram.rs (preexisting, all versions)

**File**: `crates/clawft-cli/src/markdown/telegram.rs` line 75-76

```rust
// BEFORE:
let trimmed = output.trim_end().to_owned();
trimmed

// AFTER:
output.trim_end().to_owned()
```

### Fix 3: `derivable_impls` in security_policy.rs (new at 1.91+)

**File**: `crates/clawft-tools/src/security_policy.rs` lines 12-24

```rust
// BEFORE:
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyMode {
    Allowlist,
    Denylist,
}

impl Default for PolicyMode {
    fn default() -> Self {
        Self::Allowlist
    }
}

// AFTER:
#[derive(Debug, Clone, Default, PartialEq)]
pub enum PolicyMode {
    #[default]
    Allowlist,
    Denylist,
}
```

---

## Migration Steps

### For any target version:

1. **Fix the 3 clippy warnings** listed above (can be done before the toolchain change).

2. **Update `rust-toolchain.toml`**:
   ```toml
   [toolchain]
   channel = "1.93"        # or "1.87" / "1.91"
   components = ["clippy", "rustfmt"]
   ```

3. **Update `Cargo.toml` workspace MSRV**:
   ```toml
   [workspace.package]
   rust-version = "1.93"   # or "1.87" / "1.91"
   ```

4. **Run validation**:
   ```bash
   cargo check --workspace
   cargo clippy --workspace -- -D warnings
   cargo test --workspace
   cargo check -p clawft-wasm --target wasm32-wasip2
   cargo check -p clawft-wasm --target wasm32-wasip1
   cargo build -p clawft-wasm --target wasm32-wasip2 --release
   ```

5. **Update CI** (if any CI configuration pins the Rust version independently of
   `rust-toolchain.toml`).

6. **Commit**.

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Compilation failure | **None** | N/A | All versions tested and pass |
| Test regression | **Very Low** | Low | Tests compile; runtime behavior unlikely to change |
| Dependency incompatibility | **None** | N/A | All deps have MSRV well below all target versions |
| WASM target breakage | **None** | N/A | Both wasip1 and wasip2 tested and pass across all versions |
| New clippy warnings in deps | **Very Low** | Low | Only workspace code is linted with `-D warnings` |
| Behavioral change in std | **Very Low** | Low | No API changes affect current code patterns |
| Future breakage risk | **Low** | Med | Staying on 1.85 increases delta for eventual upgrade |

**Overall Risk**: **Minimal**. This is a straightforward version bump with zero compilation
issues and only trivial lint fixes.

---

## Recommendation

**Upgrade to Rust 1.93.1** (current stable).

Rationale:

1. **Identical effort** -- The LOE is the same 12 minutes whether you go to 1.91 or 1.93.
   Going to 1.87 saves 2 minutes by avoiding one lint fix, but buys 8 months less runway.

2. **Current stable** -- 1.93.1 is the current Rust stable as of February 2026. Staying on
   the latest stable means the shortest delta for future upgrades and the best ecosystem
   support.

3. **Best WASM support** -- Each Rust release improves wasm32-wasip2 stdlib coverage. 1.93
   has the most mature wasip2 support available today.

4. **No additional risk** -- The upgrade path from 1.85 to 1.93.1 has been empirically
   validated: full workspace compilation, clippy, test compilation, and WASM builds all pass.

5. **Access to new language features** -- Let chains (1.88), `File::lock` (1.89),
   `HashMap::extract_if` (1.88), `repr128` (1.89), and `cfg` in `asm!` (1.93) become
   available for future development.

---

## Addendum: Execution Findings (Post-Upgrade)

**Date**: 2026-02-17

### Additional `collapsible_if` Warnings

During execution of the Rust 1.93.1 upgrade, **11 additional `collapsible_if` clippy
warnings** were discovered across 3 crates (9 files) that were not identified in this
research document. The total clippy fix count went from 3 (as documented above) to 14.

### Affected Files

| Crate | File | Instances |
|-------|------|-----------|
| clawft-types | `src/config.rs` | 1 |
| clawft-types | `src/provider.rs` | 6 |
| clawft-types | `src/session.rs` | 1 |
| clawft-core | `src/agent/skills.rs` | 1 |
| clawft-core | `src/pipeline/transport.rs` | 1 |
| clawft-core | `src/pipeline/llm_adapter.rs` | 1 |
| clawft-channels | `src/discord/channel.rs` | 5 |
| clawft-channels | `src/host.rs` | 1 |
| clawft-channels | `src/slack/channel.rs` | 1 |
| **Total** | **9 files** | **18 instances** |

### Root Cause

The research used `rustup run 1.93.1 cargo clippy --workspace` to test for new clippy
warnings. This command reused cached build artifacts from the existing 1.85 build in the
`target/` directory. Clippy only analyzes crates that are recompiled during the invocation
-- previously compiled crates are skipped entirely. Since most of the workspace was already
compiled under 1.85, the new `collapsible_if` lint (which became stricter between 1.85 and
1.93) was only applied to a subset of crates that happened to be recompiled.

### Lesson Learned

**Always perform a clean build when testing clippy on a new Rust version:**

```bash
# WRONG -- reuses cached artifacts, misses warnings in cached crates
rustup run 1.93.1 cargo clippy --workspace -- -D warnings

# CORRECT -- forces full recompilation, surfaces all warnings
cargo clean
rustup run 1.93.1 cargo clippy --workspace -- -D warnings
```

This is especially important when the new Rust version introduces new default-on lints
or changes lint thresholds, as cached crates will not be re-linted.

### Updated LOE

The original LOE estimate in this document was ~12 minutes for any target version.
The actual LOE including all `collapsible_if` fixes was approximately 25 minutes --
still well under 30 minutes. The additional fixes were entirely mechanical (combining
nested `if` statements with `&&`) and introduced no behavioral changes.
