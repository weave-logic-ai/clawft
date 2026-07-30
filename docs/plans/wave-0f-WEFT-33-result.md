# Wave 0f — WEFT-33 result

**Ticket**: ws03: routing — scaffold fuzz targets for 8 attack surfaces  
**Branch**: `wave0f/weft-33-fuzz-targets`  
**Base**: `release/0.8-staging`  
**Commit**: `3debc60498c7de5b048e9e93364d437a6c2a5de9`  
**Status**: implemented; all 8 targets compile and ran ~5s each without findings  
**Pushed**: no (lead merge)

## What shipped

1. **cargo-fuzz skeleton** at `crates/clawft-core/fuzz/`
   - Isolated `[workspace]` (excluded from root workspace)
   - `libfuzzer-sys` + minimal `clawft-core` features (`default-features = false`)
   - README with run / extend instructions

2. **Eight targets** (security-review §6.1 / task 03-12):

   | Target | Surface |
   |--------|---------|
   | `routing_config_parsing` | `RoutingConfig` JSON + `validate_routing_config` |
   | `permission_resolution` | `PermissionResolver` + workspace ceiling |
   | `cost_tracker_concurrent` | concurrent `reserve_budget` / reconcile |
   | `rate_limiter_unique_ids` | sender_id rotation + LRU |
   | `auth_context_threading` | `ChatRequest` skip_deserializing + zero-trust default |
   | `tool_permission_glob` | `check_tool_permission` globs / denylist-wins |
   | `escalation_chain` | `TieredRouter::route` escalation edges |
   | `budget_persistence` | `CostTracker::load` / `persist` HMAC integrity |

3. **CI**: `.github/workflows/fuzz-nightly.yml`
   - schedule (daily) + `workflow_dispatch` + PR path filters
   - matrix over all 8 targets
   - bounded `max_total_time` (default 60s; overridable on dispatch)
   - uploads artifacts on failure

4. **Repo hygiene**
   - root `Cargo.toml` `exclude` includes `crates/clawft-core/fuzz`
   - `.gitignore` for fuzz `target/`, `corpus/`, `artifacts/`, `coverage/`

## Acceptance criteria

| AC | Status |
|----|--------|
| cargo-fuzz skeleton under `crates/clawft-core/fuzz/` | done |
| All 8 targets compile | done (`cargo +nightly fuzz build`) |
| Run each briefly if nightly allows | done (~5s each, no findings after harness fixes) |
| CI nightly job for bounded fuzz time | done (`fuzz-nightly.yml`) |
| README how to extend | done (`crates/clawft-core/fuzz/README.md`) |

## Nightly / env notes

- Host has **nightly** (`rustc 1.99.0-nightly`) and **cargo-fuzz** + **clang**.
- Main workspace stays on stable `1.93` via `rust-toolchain.toml`; fuzz uses explicit `cargo +nightly`.
- If an environment lacks nightly/cargo-fuzz: scaffolds still ship; document in README. CI installs nightly + cargo-fuzz on ubuntu-latest.

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-core/fuzz/Cargo.toml` | fuzz package + 8 bins |
| `crates/clawft-core/fuzz/README.md` | how to run / extend |
| `crates/clawft-core/fuzz/fuzz_targets/*.rs` | 8 harnesses |
| `.github/workflows/fuzz-nightly.yml` | bounded nightly matrix |
| `Cargo.toml` | exclude fuzz workspace |
| `.gitignore` | corpus/artifacts/target |
| `docs/plans/wave-0f-WEFT-33-result.md` | this result |

## Tests run

```text
cargo +nightly fuzz build                          # all 8 bins OK
for t in <8 targets>; do
  cargo +nightly fuzz run $t -- -max_total_time=5 -max_len=4096
done                                               # all OK
scripts/build.sh check                             # OK (workspace exclude)
```

### Harness fixes during first run

- `cost_tracker_concurrent`: OOB slice on short inputs (`data[3..]` → `data.get(3..)`)
- `tool_permission_glob`: over-strong assert that `["*"]` allows every name — false under WEFT-32 MCP namespace guard; assert narrowed to benign `read_file` + exact denylist-wins

## How to test (for tester)

```bash
# From worktree on branch wave0f/weft-33-fuzz-targets
rustup toolchain install nightly   # if needed
cargo install cargo-fuzz --locked  # if needed

cd crates/clawft-core
cargo +nightly fuzz list
cargo +nightly fuzz build
cargo +nightly fuzz run routing_config_parsing -- -max_total_time=10

# Smoke all eight (5s each):
for t in routing_config_parsing permission_resolution cost_tracker_concurrent \
         rate_limiter_unique_ids auth_context_threading tool_permission_glob \
         escalation_chain budget_persistence; do
  cargo +nightly fuzz run "$t" -- -max_total_time=5 -max_len=4096
done

# Workspace still clean:
scripts/build.sh check
```

See `crates/clawft-core/fuzz/README.md` for extending targets and CI details.
