# Wave 0e — WEFT-47 result

**Ticket**: ws03: routing — add `max_grantable_level` field to `RoutingConfig`  
**Branch**: `wave0e/weft-47-max-grantable-level`  
**Base**: `release/0.8-staging`  
**Commit**: `e8d3cdead2decec4fb37f2f95999b83f4852d95a`  
**Status**: implemented, package tests green for touched areas  
**Pushed**: no (lead merge)

## What shipped

1. **`RoutingConfig::max_grantable_level: u8`** (`clawft-types`)
   - Default / serde default = `1` (historical constant).
   - Alias: `maxGrantableLevel`.
   - Public helper: `default_max_grantable_level()`.

2. **Validator reads config** (`routing_validation::validate_workspace_ceiling`)
   - Uses `global.max_grantable_level` instead of a hard-coded ceiling.
   - `DEFAULT_MAX_GRANTABLE_LEVEL` kept as `pub const` for call sites/tests;
     docs updated to point at the config field (WEFT-47).

3. **Backward-compat**
   - Missing JSON field deserializes to `1`.
   - Existing ceiling tests still pass under default.

4. **Docs**
   - `docs/guides/routing.md` — Workspace Ceiling section + example.
   - `docs/reference/config.md` — `routing.max_grantable_level` table.

## Acceptance criteria

| AC | Status |
|----|--------|
| Add `max_grantable_level: u8` to `RoutingConfig` (default = 1) | done |
| Validator reads from config when present | done |
| Backward-compat: missing field uses existing default | done |
| Test: workspace elevated value accepted only when global allows | done (ties to WEFT-10 ceiling semantics) |
| Documented in routing.md | done (+ config.md) |

## Files changed

| File | Change |
|------|--------|
| `crates/clawft-types/src/routing.rs` | field, default, serde alias, unit tests |
| `crates/clawft-core/src/routing_validation.rs` | read `global.max_grantable_level`; WEFT-47 tests |
| `docs/guides/routing.md` | ceiling docs + example JSON |
| `docs/reference/config.md` | field reference |

## Tests run

```text
scripts/build.sh check                          # ok
scripts/build.sh test clawft-types              # 322 passed
cargo nextest run -p clawft-core -E 'test(routing_validation)'
                                                # 48 passed (incl. new WEFT-47 tests)
```

### New tests

- `routing::tests::max_grantable_level_serde_and_alias` (types)
- `routing_validation::tests::workspace_elevated_level_accepted_when_global_max_grantable_allows`
- `routing_validation::tests::max_grantable_level_missing_field_uses_default`

### Note

Full `scripts/build.sh test clawft-core` hit one unrelated pre-existing failure:
`workspace::config::tests::load_merged_config_mcp_servers` (null MCPServerConfig
JSON). Not introduced by this change; all routing_validation tests pass.

## How to test (for tester)

```bash
# From worktree on branch wave0e/weft-47-max-grantable-level
scripts/build.sh check
scripts/build.sh test clawft-types
cargo nextest run -p clawft-core -E 'test(routing_validation)'
# or:
cargo test -p clawft-core workspace_elevated_level_accepted_when_global_max_grantable_allows
cargo test -p clawft-core max_grantable_level_missing_field_uses_default
cargo test -p clawft-types max_grantable_level_serde_and_alias
```

Manual config smoke: set global `"routing": { "max_grantable_level": 2 }` and a
workspace `permissions.user.level: 2` — ceiling validation should accept; with
default (or omitted) field, level 2 must still error.

## Worktree

- Path: `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb46e-5ef4-7621-9495-d5a6a8710a3c`
- Branch: `wave0e/weft-47-max-grantable-level`
- Ready for lead merge (do not push from agent).
