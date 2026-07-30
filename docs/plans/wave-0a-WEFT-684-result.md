# WEFT-684 result — pin ruflo / @claude-flow/cli (schema-bearing MCP)

**Ticket:** WEFT-684  
**Branch:** `wave0a/weft-684-pin-ruflo`  
**Commit:** `71f31713576205653408cea1bc45e8719646b448`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb418-df94-7233-a1e3-57844d9ad65b`

## Pinned version

| Package | Version | Rationale |
|---------|---------|-----------|
| `@claude-flow/cli` | **3.32.38** (exact) | Same version already serving AgentDB on maintainer machines; deliberately not chasing npm `latest` (was 3.32.39 at ticket write; **3.32.41** at implement time). |
| `ruflo` | **3.32.38** (exact) | Version-locked twin of `@claude-flow/cli`; operator CLI face. |
| `overrides.@claude-flow/cli` | **3.32.38** | Stops `ruflo`'s `^3.32.38` caret from floating the MCP owner. |

Durable record: `package.json` → `weftos.rufloPin` = `"3.32.38"`.

## What changed

1. **package.json / package-lock.json** — exact deps + override; lock resolves both packages to 3.32.38.
2. **.mcp.json** — `npx --no-install @claude-flow/cli mcp start` (no `@latest`, no silent re-resolve). Requires `npm ci` / local install; fails closed if missing.
3. **Operator docs & skills** — MCP install / CLI examples switched from `@latest` to pinned / `--no-install` form (guides, CLAUDE.md, skills, grok README/rules).
4. **Prior partial fix** — `f5f7275c` already replaced `@claude-flow/cli@latest` with `@3.32.38` in `.mcp.json`; this commit completes package.json + lockfile discipline and doc alignment.

## Canonical install decision

| Role | Canonical |
|------|-----------|
| **Project MCP (committed)** | `package.json` pin + `.mcp.json` `npx --no-install @claude-flow/cli` |
| **Machine-local override** | `~/.claude.json` / `.grok/config.toml` may point at a local monorepo build — **never** put absolute `/Users/...` paths in committed MCP config |
| **Do not use** | `npx ruflo@latest` / `npx @claude-flow/cli@latest`; dirty `~/dev/ruflo` as the shared store owner |
| **PATH hygiene** (operator, not in-repo) | Prefer one global `ruflo` matching the pin; retire or rename the 3.14.1 pnpm install so PATH order is not the version control |

## Bump procedure (mandatory)

Before any deliberate pin bump:

1. Back up `.swarm/` (AgentDB files).
2. Read changelog between old pin and candidate.
3. Change `package.json` (`@claude-flow/cli`, `ruflo`, `overrides`, `weftos.rufloPin`) + regenerate lockfile.
4. `npm ci`, restart MCP host.
5. Verify with `scripts/brain-embedded-rust-ingest.sh stores` (entry counts + namespaces + `provenance_type`).

## Acceptance mapping

| AC | Status |
|----|--------|
| Pin MCP to explicit version (not `@latest`) | Done — 3.32.38 via package.json + `--no-install` |
| Decide canonical install / avoid multi-version mystery | Done in-repo; PATH multi-binary is operator note (not automatable in git) |
| Backup + verify before bump | Documented |
| Record pin durably | `package.json#weftos.rufloPin` + this result + lockfile |

## Files touched

- `.mcp.json`
- `package.json`
- `package-lock.json`
- `CLAUDE.md`
- `docs/grok/README.md`
- `docs/guides/mcp.md`
- `docs/guides/mcp-integration.md`
- `docs/guides/tool-calls.md`
- `docs/guides/testing-mcp-delegation.md`
- `docs/guides/routing.md`
- `docs/reference/tools.md`
- `docs/reference/config.md`
- `docs/skills/index.md`
- `docs/handoff-tracker-ci-memory.md`
- `docs/plans/wave-0a-WEFT-684-result.md` (this file)
- `scripts/brain-embedded-rust-ingest.sh`
- `skills/claude-flow/SKILL.md`
- `skills/agent-dispatch/SKILL.md`
- `.grok/rules/ruflo-grok.md` (worktree-local; may be untracked)
- `.grok/skills/handoff/SKILL.md` (worktree-local; may be untracked)
- `.grok/config.toml` (worktree-local; may be untracked)

## How to test

```bash
# From repo root on this branch:
node -e 'const p=require("./package.json"); const l=require("./package-lock.json");
  console.assert(p.devDependencies["@claude-flow/cli"]==="3.32.38");
  console.assert(p.devDependencies.ruflo==="3.32.38");
  console.assert(p.overrides["@claude-flow/cli"]==="3.32.38");
  console.assert(p.weftos.rufloPin==="3.32.38");
  console.assert(l.packages["node_modules/@claude-flow/cli"].version==="3.32.38");
  console.assert(l.packages["node_modules/ruflo"].version==="3.32.38");
  console.log("pin ok");'
grep -n 'no-install\|3.32.38' .mcp.json
! grep -E '@claude-flow/cli@latest|ruflo@latest' .mcp.json package.json
# After npm ci on a clean machine:
# npx --no-install @claude-flow/cli --version   # expect 3.32.38
```

No Rust/product code change; `scripts/build.sh` gate not required for this tooling pin.
