# Step 0 (A2): CI Pipeline Gates for Three Workstreams

**Date:** 2026-02-24
**Branch:** feature/three-workstream-implementation
**File modified:** `.github/workflows/pr-gates.yml`

## Summary

Added three new CI gate jobs to `pr-gates.yml` to support the three-workstream implementation plan. These gates are designed to be non-blocking initially (they warn instead of fail) and will become enforcing as each workstream delivers its implementation.

## Jobs Added

### 1. `wasm-browser-check` (Browser WASM workstream)

- **Target:** `wasm32-unknown-unknown` (browser target, distinct from the existing `wasm32-wasip2` server target)
- **What it checks:** `cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser`
- **Graceful degradation:** Uses `if ... 2>/dev/null` so it emits a `::warning` annotation instead of failing when the `browser` feature flag does not yet exist.
- **Also verifies:** Native workspace compilation still works (`cargo check --workspace`) to catch any accidental breakage from feature-flag changes.
- **Will become enforcing after:** BW1 (feature flags) is complete.

### 2. `voice-feature-check` (Voice workstream)

- **What it checks:** `cargo check --features voice -p clawft-plugin`
- **Graceful degradation:** Same pattern -- warns if the `voice` feature does not exist yet.
- **Will become enforcing after:** VS1.1 (voice dependencies) is complete.

### 3. `ui-check` (UI workstream)

- **What it checks:** Lint, type-check, and test for the `ui/` directory using pnpm + Node 20.
- **Graceful degradation:** First step checks whether `ui/` and `ui/package.json` exist. If not, all subsequent steps are skipped via `if: steps.check-ui.outputs.exists == 'true'` conditionals. Emits a `::notice` annotation.
- **Will become enforcing after:** The `ui/` directory is scaffolded.

## Design Decisions

- **No `needs` dependencies:** The three new jobs run independently and in parallel with all other jobs. They are not prerequisites for `smoke-test`.
- **Non-blocking by default:** All three jobs use soft checks that warn rather than fail, so they do not block PRs until the underlying features are implemented.
- **Separate cargo caches:** Each job uses a distinct cache key prefix (`cargo-wasm-browser-`, `cargo-voice-`, etc.) to avoid cache pollution between different compilation targets and feature sets.

## Existing Jobs (unchanged)

| Job | Purpose |
|-----|---------|
| `clippy` | Lint with warnings-as-errors |
| `test` | Full workspace test suite |
| `wasm-size` | WASM binary size gate (wasm32-wasip2) |
| `binary-size` | Release binary size gate (<10MB) |
| `smoke-test` | Docker integration smoke test (depends on `test`) |
