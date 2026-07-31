---
name: plane-dag
description: >
  Work the WeftOS Plane board as a dependency DAG: refresh inventory, list
  ready (wave-0) tickets, claim/close with Plane lifecycle discipline, and
  schedule parallel multi-wave delivery. Use when planning sprints, picking
  next work, updating board state during implementation, or when the user
  mentions plane-dag / wave plan / ticket dependency graph.
---

# plane-dag — Plane board as a dependency DAG

Plane is the **authoritative** tracker (see `plane-workflow` skill). This skill
adds a **graph layer**: every `WEFT-N` is a node; `blocked_by` edges define a
DAG; **waves** are maximal parallel-ready sets.

## Artifacts (always under `docs/plans/`)

| File | Role |
|------|------|
| `plane-dag.json` | Machine-readable nodes, edges, waves |
| `plane-board-inventory.md` | Human inventory of **every** ticket + per-open plan stub |
| `plane-wave-plan.md` | Waves, lanes, critical paths, mermaid |
| `plane-issues-raw.json` | Last full Plane dump (cache) |
| `plane-issues-open.json` | Open subset |

## Helper

```bash
# Load key (macOS agents): already in ~/.zshrc often
export PLANE_API_KEY=$(zsh -c 'source ~/.zshrc >/dev/null 2>&1; print -r -- $PLANE_API_KEY')

scripts/plane-dag.sh refresh                 # re-pull Plane → rebuild all artifacts
scripts/plane-dag.sh ready --cycle 0.8.x --priority high
scripts/plane-dag.sh ready --lane C --limit 20
scripts/plane-dag.sh waves
scripts/plane-dag.sh show WEFT-593
scripts/plane-dag.sh path WEFT-593           # what this unlocks
scripts/plane-dag.sh mermaid --cycle 0.8.x --priority high
scripts/plane-dag.sh claim WEFT-593          # In Progress + assignee + comment
scripts/plane-dag.sh note WEFT-593 "mid-work: ..."
scripts/plane-dag.sh done WEFT-593 \
  --shipped "..." --commits abc123 \
  --tests "scripts/build.sh test" --build "scripts/build.sh check"
```

Implementation: `scripts/plane-dag.sh` + `scripts/plane-dag-build.py`.
Plane HTTP: `.claude/skills/plane-workflow/scripts/plane.sh` (curl User-Agent;
Cloudflare bans bare Python-urllib).

## Graph semantics

```
Edge A → B  means  B is blocked_by A  (B cannot start until A is Done)
Wave 0      = open tickets with no open blockers (true parallelism)
Wave k      = becomes ready when all Wave < k blockers close
Lane A–J    = parallel tracks inside a wave (release / mesh / memory / agent / …)
```

Edges come from:

1. Explicit ticket text (`Dependencies`, `blocked-by`, `blocks`, `depends on`)
2. Plane parent links
3. **Inferred domain chains** in `plane-dag-build.py` (`INFERRED_NAME_EDGES`)
   — LeWM crate stack, K6 mesh foundation, vector/voice/agent/release lanes

If an inferred edge is wrong: comment on Plane, remove/adjust the pattern in
`INFERRED_NAME_EDGES`, `refresh`.

## Parallel multi-wave doctrine

1. Prefer **0.8.x** (publish/ship track) over 0.9.x / 1.0.x when scheduling.
2. **0.7.x** is frozen historical only — no open ship work (absorbed into 0.8.x).
3. **0.9.x** is beta-ready residual (LeWM, deep K6 mesh, GA end-states). See `docs/plans/plane-cycle-reorg-2026-07-30.md`.
4. Max ~6–8 implementers; use **worktree isolation** for parallel coders.
5. Inside Wave 0, fan out by **lane** (A release, B mesh, C memory, …).
6. After each batch of closes: `plane-dag.sh refresh` — waves recompute.
7. Critical paths in `plane-wave-plan.md` show longest unlock chains.

## Lifecycle (keep Plane true)

| When | Action |
|------|--------|
| Start work | `claim WEFT-N` **before** first code edit |
| Discovery | `note` + optionally `plane.sh create-issue` follow-up |
| Blocked | comment with blocker; don't silently stall |
| Finish | `done` with shipped / commits / tests / build / followups |
| Defer | `plane.sh defer <id> 0.9.x --reason "..."` |

Never silent-close. Never leave In Progress after ship.

## Spec quality gate

Before coding a ticket:

- [ ] `Source` (file / ADR / audit)
- [ ] `Problem / gap` (one paragraph)
- [ ] `Acceptance criteria` with checkboxes
- [ ] `Dependencies` (`blocked-by` / `blocks` as WEFT-N)
- [ ] Labels: workstream + finding-type

If AC is weak (`plane-dag.sh show` → `ac.level`), **strengthen first**
(description update or structured comment), then implement.

## Agent playbook (team of experts)

For large board operations, run a hierarchical team:

| Role | Job |
|------|-----|
| **requirements** | Strengthen weak/high tickets; write AC |
| **architect** | Validate inferred edges; critical path |
| **wave-planner** | Schedule Wave 0a/0b lanes for the week |
| **implementers** | Claim → code → tests → close (worktrees) |
| **tracker** | Refresh DAG; keep comments current |

Bus: `scripts/grok-team-bus.mjs` / skill `agent-teams-grok`.

## Related

- `.claude/skills/plane-workflow/SKILL.md` — create/transition/close/defer
- `docs/handoff.md` — project rule that Plane is SoT
- Cycles: **0.8.x publish**, 0.7.x historical, **0.9.x beta**, 1.0.x long-horizon
