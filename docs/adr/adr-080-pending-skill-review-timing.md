# ADR-080: Pending-skill review timing — CLI + non-blocking start notice

- **Status**: Accepted (2026-07-30)
- **Closes**: WEFT-74
- **Related**: WEFT-59 (`weft skills approve|reject`), WEFT-60 (`weft skills pending`),
  WEFT-67 (`weft skills autogen`), `crates/clawft-core/src/agent/skill_autogen.rs`,
  `.planning/reviews/0.7.0-release-gate/04-plugin-skills.md` Q4

## Context

Autogen installs candidates under `~/.clawft/skills/<name>/` with a
`.pending` marker. Promotion requires human approval (`approve_skill` /
`reject_skill`). The 0.7.0 plugin-skills audit left an open question:

> Should the `.pending` marker also trigger an **interactive prompt** at
> next agent-loop start, or only on demand via **CLI**? Today neither
> happens automatically — pending skills can accumulate unseen.

By WEFT-59/60 the CLI surface exists (`weft skills pending|approve|reject`).
What remained was the **timing policy**: when (if ever) to interrupt the
operator at agent start.

## Decision

**CLI is the canonical review path. Interactive agent start may emit a
non-blocking one-line notice when pending skills exist. A blocking
interactive prompt at agent-loop start is rejected.**

| Path | Role |
|------|------|
| **`weft skills pending`** | List every `.pending` skill with path + SKILL.md preview |
| **`weft skills approve \| reject <name>`** | Promote or discard |
| **Interactive `weft agent` start notice** | Optional, non-blocking: print count + names + CLI hint, then continue the REPL |
| **Blocking TTY prompt at loop start** | **Not implemented** — deliberately out of policy |

Default policy enum: `PendingReviewTiming::CliWithStartNotice`
(`PendingReviewTiming::DEFAULT_POLICY`). `CliOnly` remains available for
callers that want silence.

### Notice shape (example)

```text
Pending skills (2): alpha-auto, beta-auto. Review: weft skills pending | approve <name> | reject <name>
```

- Emitted only when `count > 0` and policy emits start notices.
- Never reads stdin; never waits; safe for non-TTY / scripted launches that
  still use the interactive entry (notice goes to stdout with the banner).
- One-shot `-m` / headless gateway paths do **not** print the notice
  (no interactive session banner).

## Rationale

1. **Headless / CI / daemon first.** A blocking prompt at agent-loop start
   breaks non-TTY automation, gateway workers, and any path that constructs
   `AgentLoop` without a human at stdin. CLI review already works offline.
2. **Accumulation without interruption.** A start notice surfaces backlog
   without stealing the first turn. Operators who ignore it still have
   `weft skills pending` and autogen's `max_pending` cap.
3. **Single approval UX.** Approve/reject already live in CLI (WEFT-59).
   Duplicating a second interactive protocol in the REPL would fork state
   machines (TTY vs non-TTY, default-reject, shell-confirm patterns from
   `skills install`) with little gain.
4. **Matches operator preference.** Prefer "CLI + optional start notice"
   over "blocking interactive on start."

## Implications

- Core API: `list_pending_skills`, `format_pending_review_notice`,
  `pending_review_start_notice`, `PendingReviewTiming` in
  `clawft_core::agent::skill_autogen`.
- Interactive REPL (`weft agent` in-process and daemon-routed) prints the
  notice after the skills-registered banner.
- CLI `weft skills pending` remains the detail surface (previews, markers).
- No config flag required for v1; default policy is hard-coded. A future
  `skills.pending_review_timing` key may expose `cli_only` if operators ask.
- Autogen mid-session install still uses `tracing::info!` only; the next
  interactive start (or explicit CLI) surfaces the backlog.

## Alternatives considered

| Option | Verdict |
|--------|---------|
| CLI-only, no notice | Rejected — pending skills still accumulate unseen for interactive users |
| Blocking interactive prompt at loop start | Rejected — non-TTY / CI / daemon / focus cost |
| Modal in GUI only | Out of scope for this ticket; GUI can consume the same list API later |
| Prompt after every autogen install | Noisy during long sessions; notice at start + CLI is enough |

## Followups

- Optional config: `skills.pending_review_timing = "cli_only" | "cli_with_start_notice"`.
- GUI / panel surface listing pending skills (same `list_pending_skills`).
- Skill discovery skip of `.pending` directories at load time (separate from
  review timing; load path already intended to ignore unapproved markers).
