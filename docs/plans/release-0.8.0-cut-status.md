# WeftOS 0.8.0 cut status

**Date:** 2026-07-31  
**Branch:** `release/0.8-staging`  
**Plane:** WEFT-729 (release), WEFT-730 (score flywheel)

## Version

- Workspace `version = "0.8.0"` (was 0.6.20)
- Path-dep versions lockstep in root `Cargo.toml`
- CHANGELOG `[0.8.0] - 2026-07-31` + releases-mdx regenerated

## Pre-tag

| Step | Status |
|------|--------|
| Version bump | Done |
| CHANGELOG | Done |
| sessions gc CLI | Done (bd04f8fd) |
| cargo-audit residual ignores for 0.8.0 | Done (tracked; post-tag upgrades) |
| `scripts/build.sh gate` | In progress / re-run after fixes |
| `scripts/build.sh release-dry-run` | After gate green |
| Push `release/0.8-staging` | Pending (not on origin yet) |
| Tag `v0.8.0` + push | Pending gate+dry-run |
| crates.io | Blocked unless `CRATES_API_TOKEN` in operator env |
| npm `@weftos/core` | Blocked unless `NPM_TOKEN` |
| Docker / Homebrew | Via tag-triggered workflows (cargo-dist + release-docker) |

## Deploy command sequence (operator / after gate green)

```bash
# never master — push staging branch then tag
git push -u origin release/0.8-staging
scripts/build.sh release-dry-run
git tag -a v0.8.0 -m "WeftOS 0.8.0"
git push origin v0.8.0
gh run list --limit 10
gh release view v0.8.0
```

## Score flywheel (WEFT-730)

Scaffolded at `.metaharness/flywheel-score/`:

- Primary: `weftosFoundationScore` → **100** already
- Upstream ADR-041 cannot hit 100 on memoryUsefulness (~60 cap) — see ceilings.md
- `scripts/metaharness/flywheel-score-eval.sh` evaluate-only

## Residual audit (post-0.8.0 Plane follow-ups recommended)

- quick-xml ≥0.41 (RUSTSEC-2026-0194/0195)
- wasmtime advisory pin (RUSTSEC-2026-0222)
- spin yanked via multer/axum
- failure / event-listener / lexical-core transitive
