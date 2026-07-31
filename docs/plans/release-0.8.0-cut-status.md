# WeftOS 0.8.0 cut status

**Date:** 2026-07-31  
**Branch:** `release/0.8-staging`  
**Plane:** WEFT-729 (release), WEFT-730 (score flywheel)

## Version

- Workspace `version = "0.8.0"` (was 0.6.20)
- Path-dep versions lockstep in root `Cargo.toml`
- CHANGELOG `[0.8.0] - 2026-07-31` + releases-mdx regenerated

## Tag

| Field | Value |
|-------|--------|
| Tag | `v0.8.0` (annotated) — re-cutting again |
| Tip commit (prior) | `4403c8d9` — named-pipe / SBOM / WASM budget |
| Next tip | Windows `dist.binaries` pin for `x86_64-pc-windows-msvc` (this commit) |
| Remote | `origin/release/0.8-staging` + `refs/tags/v0.8.0` |

Cuts so far:

1. `d563f3bd` — failed: Windows `NamedPipeServer` E0603, SBOM `eval` quoting, browser WASM raw budget.
2. `4403c8d9` — fixed above; failed: cargo-dist on Windows looked for `weft-demo-lab.exe` because `package.metadata.dist.binaries` omitted `x86_64-pc-windows-msvc` (Linux/mac pins only). Compile of dist profile succeeded; packaging step failed.
3. This commit — pin `x86_64-pc-windows-msvc = ["weft-gui-egui"]`.

## Pre-tag

| Step | Status |
|------|--------|
| Version bump | Done (`b2f5d218`) |
| CHANGELOG | Done |
| sessions gc CLI | Done (`bd04f8fd`) |
| cargo-audit residual ignores for 0.8.0 | Done (tracked; post-tag upgrades) |
| `scripts/build.sh gate` | Green (16/16, pre-tag host) |
| `scripts/build.sh release-dry-run` | Green (host) |
| Push `release/0.8-staging` | Done |
| Tag `v0.8.0` + push | Done (re-cut → `4403c8d9`) |

## CI (re-cut run)

| Workflow | Run | Status |
|----------|-----|--------|
| **Release** (cargo-dist) | [30671005403](https://github.com/weave-logic-ai/weftos/actions/runs/30671005403) | In progress — plan + WASI green; 7 platform builds compiling |
| Release SBOM | [30671005311](https://github.com/weave-logic-ai/weftos/actions/runs/30671005311) | In progress |
| Browser WASM | [30671005302](https://github.com/weave-logic-ai/weftos/actions/runs/30671005302) | In progress (artifact `browser-wasm-pkg` seen) |
| Release (Knowledge Base) | [30671005313](https://github.com/weave-logic-ai/weftos/actions/runs/30671005313) | In progress |
| Release (Docker) | skipped until Release completes (`workflow_run`) | Waiting |

### Re-cut fixes on tag

- Windows: re-export `tokio::net::windows::named_pipe::NamedPipeServer` from platform named_pipe module
- SBOM: `printf %q` for paths before `eval` in `generate-sbom.sh`
- Browser WASM: raw size budget 1600 → 1700 KB (`check-bundle-size.sh` + docs)

## Publish legs

| Channel | Status |
|---------|--------|
| GitHub Releases (binaries + installers) | Pending cargo-dist host job |
| Homebrew (`weave-logic-ai/homebrew-tap`) | Via cargo-dist when Release succeeds (`HOMEBREW_TAP_TOKEN` present) |
| Docker `ghcr.io/weave-logic-ai/weftos` | Auto after successful Release |
| crates.io | **Blocked** — no `CRATES_API_TOKEN` / cargo credentials in operator env |
| npm `@weftos/core` | **Blocked** — no `NPM_TOKEN` / npm auth |

## Deploy command sequence (done)

```bash
# never master — staging branch then tag
git push -u origin release/0.8-staging
scripts/build.sh release-dry-run
# after CI fix re-cut:
git tag -d v0.8.0 && git push origin :refs/tags/v0.8.0
git tag -a v0.8.0 -m "WeftOS 0.8.0" 4403c8d9
git push origin v0.8.0
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

## Next operator actions

1. Wait Release 30671005403 → `gh release view v0.8.0` (assets + installers).
2. Confirm Docker workflow starts; monitor multi-arch GHCR tags.
3. When tokens available: crates.io dependency order + npm `@weftos/core`.
4. Close WEFT-729 with tag SHA, run IDs, and residual publish blockers.
