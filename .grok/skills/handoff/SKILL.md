---
name: handoff
description: >-
  Write or refresh the current project's handoff doc so the work can be dropped
  and picked up cleanly by a future session. Use when the user types /handoff,
  says "prep a handoff", "ready this for drop", "write up where we are", or is
  wrapping a session. Captures verified state, dead ends, and exact resume steps.
  Canonical path for this project: docs/handoff.md.
---

# Handoff

> **Core principle:** a handoff is written for a stranger with no memory of this
> session — which is exactly what the next session is. Optimize for *resuming
> work*, not for describing it. The test: could someone read only this doc and
> take the next correct action without re-deriving anything?

## What this produces

**Canonical path for this project: `docs/handoff.md`** (create `docs/` if
absent). The file is **rewritten, not appended** — a snapshot of *now*, not a
changelog. History lives in git; the handoff answers "where are we and what's next."

Do **not** invent alternate paths (no `HANDOFF.md`, no `.weftos/SESSION_HANDOFF.md`)
unless the user explicitly asks. Always use `docs/handoff.md`.

## Procedure

Do these in order. Steps 1–3 are gathering; do not start writing until they're done.

1. **Read `docs/handoff.md`** if it exists so you carry forward still-true context
   and *delete what's now stale*. A handoff that accretes lies is worse than none.

2. **Gather hard state — never from memory, always from the machine:**
   ```bash
   git -C . log --oneline -15 ; git -C . status --short ; git -C . branch --show-current
   ```
   Also capture, where they apply: what's running right now (ports, background
   processes), build/test status (run them if fast and unverified), and the
   versions/toolchains a resumer would need.

3. **Mine the session for what the code doesn't say.** Walk back through this
   conversation for:
   - decisions made and *why* (especially ones that look arbitrary in the diff)
   - **dead ends and disproven approaches** — the highest-value content in the
     whole doc (see Rules)
   - measurements taken (numbers, benchmarks, calibrations) — these are
     expensive to reproduce; write the actual values down
   - things the user asked for that are *not yet done*
   - anything verified on real hardware/data vs assumed

4. **Check durable memory** (point at it; don't dump it into the handoff):
   - Claude: `~/.claude/projects/<slug>/memory/` (slug = cwd with `/`→`-`)
   - Ruflo: `npx -y ruflo@latest memory search --query "…"` / namespace `patterns`
   - Grok experimental memory only if enabled (`GROK_MEMORY` / `/memory`)
   - Agent teams state if active: `.claude-flow/teams/`, `scripts/grok-team-bus.mjs status`

5. **Write `docs/handoff.md`** using the template below. Then **verify it**:
   re-read it as if you know nothing, and check every command actually runs as written.

6. **Report** the path (`docs/handoff.md`) and a two-line summary of what a
   resumer would do first.

## Template

Adapt to the project — drop sections that don't apply rather than padding them.

```markdown
# Handoff — <project> — <YYYY-MM-DD>

<Two or three sentences: what this project is, and what state it's in right now.
Written for someone who has never seen it.>

## Current state

- Branch `<name>` @ `<sha>`, <clean | N files dirty (list them)>
- <what's running / deployed / flashed, and where>
- <build + test status, with when it was last actually run>

## What's working (verified)

| Thing | State | Verified how |
|---|---|---|
| <component> | <works / partial> | <the actual check that proved it> |

## Done this session

- <outcome, not activity — what is now true that wasn't before>

## Measurements & calibration

<Real numbers taken this session — latencies, thresholds, rates, tolerances.
These cost time to obtain; never make the next session re-measure them.>

## Dead ends — do not retry

- **<approach>** — <why it fails, with the evidence that killed it>

## Open threads

1. **<next action>** — <the first concrete step, and what "done" looks like>

## Resume here

```bash
# exact commands, copy-pasteable, in order
```

## Key paths

- `docs/handoff.md` — this file
- `<path>` — <what it is / why you'd open it>

## Gotchas

- <trap that will bite the next session, and how to avoid it>
```

## Rules

- **Always write `docs/handoff.md`.** No alternate path unless the user overrides.
- **Dead ends are the point.** Any approach that was tried and disproven gets its
  own entry with the evidence. Without it, the next session cheerfully burns hours
  re-walking the same path — the single most expensive failure mode a handoff
  prevents. Be specific: *"integrating the total-phase channel yields a random
  walk (|Δφ| p99 = 4.2 rad ≫ π); measured 34 mm drift over 150 s"* beats
  *"phase integration didn't work."*
- **Verified vs assumed, always marked.** State how each claim was checked. If it
  wasn't checked, say so. A handoff that overstates confidence is a trap.
- **Real numbers, not adjectives.** "lock 75–82%, 11/min, 220–270 µm" not "works well".
- **Outcomes, not activity.** "Alarm fires at 15.1 s into a hold (was: never)"
  not "worked on the watchdog".
- **Commands must run.** Every command in "Resume here" gets checked against what
  you actually ran this session — no aspirational invocations, no wrong paths.
- **Rewrite, don't append.** Stale sections get deleted. The doc describes now.
- **Point at memory, don't duplicate it.** Durable cross-session facts go in
  memory stores; the handoff links to them.
- **Say what's unfinished.** Including anything the user asked for that you didn't
  get to — that's the first thing the next session needs.
- **Don't commit unless asked.** Write the file; leave committing to the user.
