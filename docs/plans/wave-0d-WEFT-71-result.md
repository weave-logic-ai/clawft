# WEFT-71 result — clawft.plugin.json schema roundtrip + version-compat tests

**Ticket:** WEFT-71  
**Branch:** `wave0d/weft-71-plugin-schema-tests`  
**Base:** `release/0.8-staging`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb45e-9f20-75c3-aedd-b59604048c75`  
**Commit:** `4f069984`  
**Date:** 2026-07-30  
**Agent:** coder-71 (wave-0d)

## Problem

`PluginManifest` is the public wire contract for `clawft.plugin.json`, but
the crate only had unit tests colocated in `src/manifest.rs`. There were no
integration-level roundtrip / schema-version / forward-compat tests under
`crates/clawft-plugin/tests/`.

## What shipped

New integration test module:

| File | Role |
|------|------|
| `crates/clawft-plugin/tests/manifest_schema_roundtrip.rs` | Public-API contract tests for `PluginManifest` |

### Coverage (21 tests)

| Category | Tests |
|----------|--------|
| **Roundtrip** | known-good lossless parse; second-hop serialize **byte-equal**; `Value` shape stability; minimal defaults; all capability variants |
| **Forward / version-compat** | unknown top-level fields ignored; `schema_version` ignored; unknown nested keys ignored; semver prerelease/build accepted; voice camelCase aliases |
| **Negative** | malformed JSON; missing required fields; empty id/name/capabilities; non-semver version; invalid id chars; unknown capability; wrong JSON types |

Notes on contract behaviour locked by these tests:

- There is **no hard `schema_version` pin** on `PluginManifest` today; unknown
  keys (including a future `schema_version`) are ignored by serde.
- Plugin identity versioning is the semver `version` field validated via
  `PluginManifest::validate`.
- Second-hop serialize is the byte-stable surface (first hop from hand-written
  JSON may reorder/default fields after typing).

No production code changes — tests only.

## Acceptance

| Criterion | Status |
|-----------|--------|
| Roundtrip: known-good → deserialize → serialize → stable | **Done** (lossless + second-hop byte-equal) |
| Forward-compat: future-version field preserved or ignored | **Done** (ignored gracefully) |
| Negative: malformed, missing required, wrong version | **Done** |
| Tests live in `crates/clawft-plugin/tests/` | **Done** |

## Tests

```bash
scripts/build.sh check
scripts/build.sh test clawft-plugin
# or focused:
cargo test -p clawft-plugin --test manifest_schema_roundtrip
```

- **check:** pass  
- **clawft-plugin package:** 135 passed (incl. 21 new integration tests)  
- **manifest_schema_roundtrip:** 21 passed  

## Files changed

- `crates/clawft-plugin/tests/manifest_schema_roundtrip.rs` (new)
- `docs/plans/wave-0d-WEFT-71-result.md` (this file)
