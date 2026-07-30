# Branching for 0.8.x publish (2026-07-30)

## Canonical branches

| Branch | Role |
|--------|------|
| **`master`** | Stable published line (today: v0.6.x / pre-hermes tip on `origin/master`) |
| **`release/0.8-staging`** | **Active 0.8 publish integration branch** — Hermes loop base + Wave 0a + ongoing 0.8.x work |
| **`feat/hermes-loop-base`** | Historical feature branch; **superseded** by `release/0.8-staging` for new work |

## Rules

1. **New 0.8.x work** branches from `release/0.8-staging` (e.g. `wave0b/weft-661-…`, `fix/…`).
2. **Do not** open long-lived work from `feat/hermes-loop-base` — merge/rebase onto staging instead.
3. **Wave / worktree agents** should use `release/0.8-staging` as the base commit.
4. When 0.8 is ready to publish: PR or merge `release/0.8-staging` → `master`, tag, cut release.
5. Beta (0.9.x Plane cycle) may later use `release/0.9-staging` from master after 0.8 ships.

## How this branch was created

```text
origin/master (1f2f742e, 2026-06-28 handoff)
    └── feat/hermes-loop-base (+~220 hermes/voice/memory commits)
            └── Wave 0a merges (WEFT-593,596,660,605,671,595,663,684)
                    └── release/0.8-staging  ← you are here
```

## Commands

```bash
git fetch origin
git checkout release/0.8-staging
git pull --ff-only   # once pushed

# New work
git checkout -b wave0b/weft-661-hybrid-metrics release/0.8-staging
```
