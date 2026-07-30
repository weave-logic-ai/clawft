# clawft-core fuzz targets (WEFT-33)

Cargo-fuzz harnesses for the eight pipeline / routing attack surfaces
identified in `.planning/sparc/phase4/01-tiered-router/security-review.md`
§6.1 and tracked as task **03-12** / Plane **WEFT-33**.

| Target | Surface | Property under test |
|--------|---------|---------------------|
| `routing_config_parsing` | `RoutingConfig` JSON/YAML + `validate_routing_config` | Never panics on arbitrary input |
| `permission_resolution` | `PermissionResolver` 5-layer merge + workspace ceiling | Never panics; finite numeric fields |
| `cost_tracker_concurrent` | `CostTracker::reserve_budget` under threads | No panic / lock poison on concurrent use |
| `rate_limiter_unique_ids` | `RateLimiter` + sender_id rotation / LRU | Never panics; bounded tracked set |
| `auth_context_threading` | `ChatRequest` / `AuthContext` serde boundary | `auth_context` never injected from JSON |
| `tool_permission_glob` | `check_tool_permission` allow/deny globs | Never panics; denylist wins; `*` allows |
| `escalation_chain` | `TieredRouter::route` escalation path | Never panics on empty tiers / NaN complexity |
| `budget_persistence` | `CostTracker::load` / `persist` (HMAC) | Never panics on corrupt / tampered files |

## Prerequisites

- **Nightly Rust** — `libfuzzer-sys` requires nightly.
  ```bash
  rustup toolchain install nightly
  ```
- **cargo-fuzz**
  ```bash
  cargo install cargo-fuzz --locked
  ```
- **clang** (libFuzzer link) — Xcode CLT on macOS, `clang` package on Linux.

The main workspace pins `rust-toolchain.toml` to stable `1.93`. These
targets live in an **isolated workspace** (`fuzz/Cargo.toml` has its own
`[workspace]` table and the path is listed under the root workspace
`exclude`). Invoke them with an explicit `+nightly` override.

## Run a single target

From the **repository root** (or from this directory):

```bash
cd crates/clawft-core
cargo +nightly fuzz run routing_config_parsing -- -max_total_time=60
```

Bounded CI-style smoke (5s each):

```bash
for t in \
  routing_config_parsing \
  permission_resolution \
  cost_tracker_concurrent \
  rate_limiter_unique_ids \
  auth_context_threading \
  tool_permission_glob \
  escalation_chain \
  budget_persistence
do
  cargo +nightly fuzz run "$t" -- -max_total_time=5 -max_len=4096
done
```

List targets:

```bash
cargo +nightly fuzz list
```

## Build without running

```bash
cargo +nightly fuzz build
# or a single binary:
cargo +nightly fuzz build routing_config_parsing
```

## How to add a new target

1. Create `fuzz_targets/<name>.rs`:

   ```rust
   #![no_main]
   use libfuzzer_sys::fuzz_target;

   fuzz_target!(|data: &[u8]| {
       // call the API under test; panics = findings
   });
   ```

2. Register a `[[bin]]` in `fuzz/Cargo.toml` pointing at that file
   (`test = false`, `doc = false`, `bench = false`).

3. Document the surface + property in the table above.

4. Add the target name to `.github/workflows/fuzz-nightly.yml`
   (`matrix.target`) and to the smoke loop in this README.

5. Prefer **minimal features** on `clawft-core` (`default-features = false`)
   so fuzz builds stay fast. Only enable a feature if the API needs it.

## Corpus & artifacts

cargo-fuzz stores state under:

```
crates/clawft-core/fuzz/corpus/<target>/
crates/clawft-core/fuzz/artifacts/<target>/
crates/clawft-core/fuzz/target/          # build output (own cargo target dir)
```

These paths are gitignored. Commit **seed corpora** only when they are
small, intentional fixtures (put them in `fuzz/seed_corpus/<target>/`
and pass `-seed_inputs=...` or copy into `corpus/` before a run).

## CI

`.github/workflows/fuzz-nightly.yml` runs each target for a **bounded**
time (`max_total_time=60`) on a nightly schedule and on manual dispatch.
Failures (crashes / OOMs recorded by libFuzzer) fail the job; the
artifact directory is uploaded for triage.

## Relationship to unit tests

These harnesses complement, not replace:

- `crates/clawft-core/tests/security_tests.rs`
- unit tests inside `pipeline/{permissions,cost_tracker,rate_limiter,tiered_router}.rs`
- `routing_validation` tests

Unit tests lock known edge cases; fuzzing searches for unknown panics
and invariant breaks on unstructured input.
