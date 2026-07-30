# WEFT-593 result — cargo-dist empty plan matrix / missing platform binaries

**Status:** fixed (config/workflow)  
**Branch:** `wave0a/weft-593-cargo-dist-matrix`  
**Date:** 2026-07-30

## Root cause

**Not** a cargo-dist 0.31 "back-to-back patch tag" bug, and **not** an empty
plan from `dist` itself.

Evidence from GitHub Actions run `28333666280` (tag `v0.6.21`, 2026-06-28):

1. `dist host --steps=create --tag=v0.6.21` succeeded and wrote a full
   `plan-dist-manifest.json` with **6** `artifacts_matrix.include` rows
   (same targets as the good `v0.6.20` run `28332022691`).
2. At end of the plan job:
   ```
   ##[warning]Skip output 'val' since it may contain secret.
   Set output 'tag'
   Set output 'tag-flag'
   Set output 'publishing'
   ```
   Job output `val` was **dropped** by GitHub Actions secret-scanning.
   Only `tag` / `tag-flag` / `publishing` were set.
3. `build-local-artifacts` condition is:
   ```yaml
   fromJson(needs.plan.outputs.val).ci.github.artifacts_matrix.include != null
   && …
   ```
   With `val` empty, that is false → job **SKIPPED** (matrix never expands).
4. Stock host job treated skip as OK and still ran `gh release create`,
   producing a "successful" binary-less release (only WASM/KB sub-release
   assets attached later). Homebrew publish was also skipped for the same
   empty `val`.

Trigger text: the v0.6.21 CHANGELOG announcement body (embedded in the
plan JSON as `announcement_github_body` / `announcement_changelog`)
contained a substring that matched a registered repo/org secret value.
GHA redacted it in logs as `***` (in the body:
`auth-gated (*** via TokenStore)` where the source said
`Bearer token via TokenStore`). The **exact** secret value is not known
and does not need to be — any future CHANGELOG phrase that collides with
a secret would re-break stock cargo-dist.

`v0.6.20` plan job correctly logged `Set output 'val'` (no secret skip),
which is why that release built all 6 targets.

## Fix

Hand-patch on top of cargo-dist **0.31.0** generated workflow:

| Change | File |
|--------|------|
| Slim plan/host job outputs: `del(.announcement_changelog, .announcement_github_body)` before writing `GITHUB_OUTPUT` | `.github/workflows/release.yml` |
| Dedicated `matrix` / `pr-run-mode` / `is-prerelease` / `publish-prereleases` plan outputs | same |
| `build-local-artifacts` reads `needs.plan.outputs.matrix` (not full `val`) | same |
| Host/announce requires `build-local-artifacts` **and** `build-global-artifacts` **success** when publishing (skip no longer OK) | same |
| Create GitHub Release reads title/body from files written in the host step (not from job-output JSON) | same |
| Plan fails on tag if `artifacts_matrix.include` length is 0 | same |
| `allow-dirty = ["ci"]` so `dist plan` / CI accept the hand-patched workflow | `Cargo.toml` `[workspace.metadata.dist]` |

Full plan JSON still uploads as artifact `artifacts-plan-dist-manifest`
(unchanged; secret-scan applies to **job outputs**, not artifacts).

## Local validation

```bash
source "$HOME/.cargo/env"
# cargo-dist 0.31.0 installed via installer script
dist plan --output-format=json | jq '.ci.github.artifacts_matrix.include | length'
# → 6
dist plan --tag=v0.6.20 --output-format=json | jq '[.releases[].app_name]'
# → clawft-gui-egui, clawft-weave, weftos, clawft-cli
```

Without `allow-dirty = ["ci"]`, `dist plan` errors with
`release.yml has out of date contents and needs to be regenerated`.

## How to verify on the next real tag (gh)

After this branch is on the default branch (or the release-cut branch):

```bash
# 1. Dry-run plan locally with the version you will cut
dist plan --tag=vX.Y.Z --output-format=json \
  | jq '{apps: [.releases[].app_name], n: (.ci.github.artifacts_matrix.include|length)}'

# 2. Cut + push tag as usual (see docs/deployment/release.md)

# 3. Inspect the Release workflow plan job
gh run list --workflow=release.yml --limit 5
gh run view <run-id> --log 2>&1 | grep -E "Skip output|Set output|matrix include|artifacts_matrix"

# Expect:
#   - NO "Skip output 'val' since it may contain secret"
#   - "Set output 'val'" / matrix outputs present
#   - "artifacts_matrix.include count=6"
#   - six build-local-artifacts jobs (not SKIPPED)

gh run view <run-id> --json jobs --jq '.jobs[] | {name, conclusion}'
# Expect build-local-artifacts for each of:
#   aarch64-apple-darwin, x86_64-apple-darwin,
#   aarch64-unknown-linux-gnu, x86_64-unknown-linux-gnu,
#   aarch64-unknown-linux-musl, x86_64-unknown-linux-musl

# 4. Release assets
gh release view vX.Y.Z --json assets --jq '[.assets[].name] | map(select(test("unknown-linux|apple-darwin"))) | length'
# Expect non-zero platform tarballs (v0.6.20 had 63 assets total)
```

## Residual risk

1. **Regenerate drift:** `dist generate` / cargo-dist version bumps will
   overwrite `release.yml`. The header comment + `allow-dirty = ["ci"]`
   document that the WEFT-593 patch must be re-applied. Long-term: bump
   to cargo-dist ≥1.x if upstream hardens job outputs (tracked separately
   as cargo-dist v0.31→v1 bump ticket).
2. **Secret still in CHANGELOG:** if a future release notes file embeds an
   actual secret value, logs will redact it; builds should still run
   because bodies no longer ride job outputs. Prefer never putting secret
   material in CHANGELOG.
3. **Host now requires successful builds:** library-only / matrix-empty
   releases would no longer announce. WeftOS always ships the 6 native
   targets; the plan-job empty-matrix guard fails earlier on tags.
4. **Untagged draft leftovers:** a draft/untagged `v0.6.21` release object
   may still exist from the rollback (`gh release list`). Clean up before
   re-using that version number if desired.
5. **End-to-end tag not run here:** this worktree did not push a release
   tag; verification above is required on the next cut. Docker image
   (WEFT-594) still depends on published binaries existing.

## Files changed

- `.github/workflows/release.yml` — secret-scan-safe outputs + announce gate
- `Cargo.toml` — `allow-dirty = ["ci"]` under `[workspace.metadata.dist]`
- `CHANGELOG.md` — document fix under `[Unreleased]`
- `docs/plans/wave-0a-WEFT-593-result.md` — this file
