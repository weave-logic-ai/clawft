# Handoff — Tracker reconciliation, CI gates, memory store, embedded-rust brain

**Date:** 2026-07-30
**Repo:** `/Users/mathewbeane/weftos` (branch `feat/hermes-loop-base` @ `f5f7275c`)
**Scope:** infrastructure — Plane, CI, the AgentDB memory store, and a new
embedded-Rust agent cluster. **Not** the voice track.

> **The voice/COW track is unchanged and still current** — read
> `docs/handoff-voice-talk.md` for it. Its one pending item (WEFT-615, round-2
> mic confirmation) is still pending and still needs *you*, not more code.
> `docs/handoff.md` is a 177 KB historical log ending 2026-06-28; it is not a
> live document.

This session did no product work. It fixed the machinery that reports whether
product work is correct — and that machinery turned out to be lying in three
independent ways.

## Current state

- Branch `feat/hermes-loop-base` @ `f5f7275c`, **tree clean, 0 unpushed**.
  The branch was pushed for the first time this session (211 commits; it had
  never had an upstream).
- **CI is RED — expected, and it is the fix working.** The gates had never run
  in this branch's history. Their first execution exposed 9 failing jobs.
- Ruflo background daemon: **STOPPED** (I started it accidentally, see Gotchas).
- Plane: 676+ items, `0.8.x` is the live gate (2026-07-01 → **2026-09-30**).

## What's working (verified)

| Thing | State | Verified how |
|---|---|---|
| Native Rust build | works | `scripts/build.sh check` → `cargo check --workspace` finished, 4.92s, clean |
| AgentDB memory store | works, 232 entries | `memory_stats`: 100% embedding coverage; semantic recall spot-checked across 4 namespaces |
| Store migration fidelity | exact | 182/182 sha256 byte-identical vs a pre-migration manifest |
| CI gates fire on push | works | `gh run list` shows a `push`-triggered PR Gates run on a feature branch — the first ever on this branch |
| All 12 workflows parse | works | `yaml.safe_load` over `.github/workflows/*.yml` |
| Plane label rule | clean | 0 violations across all items (was 69) |
| Browser WASM build | **BROKEN** | the real gate command, 15 errors — see Measurements |

## Done this session

- **The memory store was silently split; it is now repaired.** 182 active rows
  migrated from the orphaned `.swarm/memory.db` into the live
  `.swarm/agentdb-memory.db`, namespaces preserved, content byte-verified.
  Before this, `clawft-knowledge`, `ruv/brain` and every `weftos/*` namespace
  were unretrievable by any agent.
- **Root cause of that split found and fixed** (WEFT-684): `.mcp.json` is
  tracked in git and said `@claude-flow/cli@latest`, so the process owning the
  store resolved a different version per machine, per day. Now pinned to
  `3.32.38`, with a machine-local override in `~/.claude.json` pointing at your
  local grok build.
- **CI gates now actually run** (WEFT-674). Plus two workflows that had *never*
  run on any event (`benchmarks.yml`, `wasm-build.yml` targeted a `main` branch
  that does not exist), and one that GitHub could not even parse
  (`release-gate.yml` was malformed YAML).
- **Two tickets were lying.** WEFT-154 (Email) and WEFT-159 (Matrix) were marked
  Done; both adapters are still planning stubs. Reopened.
- **Plane reconciled**: 659 → 676+ items, 5 state corrections, 18 new items,
  label violations 69 → 0, `ws18-firmware` label created.
- **Embedded-Rust expert cluster + cited brain** shipped (4 agents, 40 memory
  chunks, 12 distilled notes).

## Measurements & calibration

Real numbers, taken this session. Do not re-derive these.

- **Browser WASM: exactly 15 errors** from
  `cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser`
  — **10×** `future cannot be sent between threads safely` (WEFT-663, its
  original count, *unchanged*), **4×** cascading `E0282`, **1×** `E0433 could
  not find hermes in clawft_llm` (WEFT-672).
  ⚠ The build did **not** rot from 10→16. That earlier claim miscounted cargo's
  summary line. Scope WEFT-663 to the 10 Send errors; the 4 E0282s should vanish
  with them.
- **Memory store**: 222 → 232 entries, 100% embedding coverage, 384-dim.
  Migration verified 182/182 sha256. Retrieval: `ruv/brain/card-agenticow` 0.835,
  `clawft-knowledge/project-overview` 0.878.
- **0.7.x cycle contents**: 137 items = **129 Done + 8 Cancelled + 0 open.**
- **Plane label gaps**: 69 → 0. Of an apparent 35 stragglers, 18 were false
  positives (they carry `security`, which the skill's enumerated type list omits).
- **Native build**: `cargo check --workspace` clean in 4.92s.
- **npm**: 102 packages changed, 76 added, 42 removed. `ruvector` 0.1.100→0.2.40,
  `@ruvector/rvf` 0.1.9→0.2.3, `@ruvector/ruvllm` 0.2.4→**2.6.0** (major).

## Dead ends — do not retry

- **Do NOT bulk-cancel the 12 `clawft-plugin/src/voice` items as superseded.**
  An automated pass recommended it on the premise that the module is dead code.
  **False** — `crates/clawft-cli/src/commands/voice.rs:508` imports
  `WakeDaemon`/`WakeWordConfig` from it, and `clawft-channels` depends on the
  crate. The module is `#[cfg(feature = "voice")]`-gated, 4,247 lines, and
  partially live: **2 of 21 files** (`wake.rs`, `wake_daemon.rs`) have real
  callers; the other 19 have none. Decision item is **WEFT-671** and it carries
  the file-level map. Only WEFT-217 and WEFT-221 are legitimate supersession
  candidates.
- **Do NOT try to force `@ruvector/rvf` to 0.3.4.** `agentdb` declares
  `^0.2.3`, which on a 0.x package resolves `<0.3.0`. `npm update` cannot reach
  it; only an `overrides` block could, and that would force a transitive dep
  across a breaking boundary `agentdb` was not built against. Nothing in this
  repo imports it directly (verified: zero hits across our `.ts`/`.js`/`.mjs`).
- **Do NOT point the tracked `.mcp.json` at `~/dev/ruflo/...`.** It is committed;
  an absolute `/Users/mathewbeane/...` path breaks every other clone and CI.
  Machine-local overrides belong in `~/.claude.json` (already done).
- **Do NOT try to revive cycle 0.7.x.** A completed Plane cycle rejects new
  issues (`CYCLE_COMPLETED`) *and* refuses edits, so its dates cannot be
  extended. Confirmed by API response. Also: **nothing is stranded in it** — an
  older note claiming otherwise is wrong; it is 129 Done + 8 Cancelled + 0 open,
  so creating a `0.7.1` rescue cycle would move nothing.
- **`PrivacyIndicator` does not exist.** A pass flagged WEFT-207 as a false-Done
  because that type "has no callers". There is no such type — the real symbols
  (`IndicatorState`, `IndicatorPayload`, `InMemoryIndicatorPublisher`) have 3–8
  external references each. Confirm a symbol resolves before concluding anything
  from an absence-of-usage grep.
- **`ruflo memory store` writes to the wrong file.** It defaults to
  `cwd/.swarm/memory.db`, the orphaned store. Ingest through the MCP
  `memory_store`/`memory_import` tools, which run in the same process that reads
  it — the `sql.js` backend can clobber an external writer.

## Open threads

1. **Pull the CI failure logs and file the newly-exposed jobs.** `gh run view
   --log-failed` returns nothing while a run is in progress; wait for completion.
   9 jobs failing: 3 Browser-WASM (known: WEFT-663/672), plus **VSCode panel
   build, UI lint and type-check, Docs site build, weftos-design audit, Binary
   size check, Cargo audit** — the last six were invisible until today and have
   no tickets. *Done when each has a Plane item with a root cause.*
2. **Fix WEFT-672 first** — it is 1 isolated error (`transport.rs:381` calls
   `clawft_llm::hermes::strip_think` while `hermes` is `native`-only). WEFT-663
   is the harder structural one (10 Send errors; `local_file_sink.rs` is not
   feature-gated unlike its siblings). Both needed for green.
3. **WEFT-662 — nobody filed the upstream bugs.** Local workarounds for three
   `rvf-runtime` bugs shipped, so it reads done; its actual acceptance criterion
   is filing them against `ruvnet/rvf-runtime` and a repo-wide grep found no
   issue link. *Done when the issue URLs are on the ticket.*
4. **WEFT-154 / WEFT-159** — email and matrix adapters need real implementations
   or honest re-scoping. Reopened with evidence.
5. **Retire one of the duplicate ruflo installs.** `~/Library/pnpm/bin/ruflo`
   (3.14.1) and `~/.nvm/.../ruflo` → `~/dev/ruflo` (3.32.38) both sit on PATH and
   **PATH order silently decides** which a shell gets.
6. **169–170 Dependabot vulnerabilities** on the default branch (6 critical,
   54 high). Dwarfs WEFT-679/681. Probably wants its own item.
7. **`0.8.x` expires 2026-09-30** and will lock exactly as 0.7.x did. Push its
   `end_date` before then, or adopt the durable fix: carry gate semantics on a
   *label* (`release-gate-blocker` already exists) and let cycles be dated.

## Resume here

```bash
cd /Users/mathewbeane/weftos

# 0. RESTART CLAUDE CODE FIRST — the MCP pin only takes effect on restart.
#    Until then the old `npx @claude-flow/cli@latest` process is still serving.

# 1. Confirm the store is intact (should read 232 active, 13 namespaces)
scripts/brain-embedded-rust-ingest.sh stores

# 2. See what CI actually says now that the gates run
gh run list --branch feat/hermes-loop-base --limit 3
RID=$(gh run list --branch feat/hermes-loop-base --limit 1 --json databaseId --jq '.[0].databaseId')
gh run view "$RID" --json jobs --jq '.jobs[]|select(.conclusion=="failure")|.name'
gh run view "$RID" --log-failed | head -100    # only works once the run COMPLETES

# 3. Reproduce the browser break locally (15 errors expected)
cargo check --target wasm32-unknown-unknown -p clawft-wasm \
  --no-default-features --features browser 2>&1 | grep -c '^error'

# 4. Native build is healthy — sanity check
scripts/build.sh check

# 5. Plane (creds are in ~/.zshrc; 0.8.x is the live gate, 0.7.x is LOCKED)
cd .claude/skills/plane-workflow && ./scripts/plane.sh list-cycle 0.8.x
```

## Key paths

- `docs/handoff-voice-talk.md` — the voice/COW track, still current
- `.planning/embedded-rust/brain/` — cited Espressif-Rust corpus (README →
  trust tiers, `coverage-map.md` §3 → what it deliberately does NOT know)
- `scripts/brain-embedded-rust-ingest.sh` — `validate` / `plan` / `stores` /
  `ingest`; `stores` is the memory-store diagnostic
- `.mcp.json` — **tracked**; pins `@claude-flow/cli@3.32.38`
- `~/.claude.json` → `projects['/Users/mathewbeane/weftos'].mcpServers['claude-flow']`
  — machine-local override pointing at the grok build
- `~/.claude/backups/` — pre-migration store backups +
  `legacy-fidelity-manifest.json` (the 306 tags `memory_import` dropped)
- `.claude/skills/plane-workflow/SKILL.md` — now documents the 0.7.x lock
- `~/.claude/agents/embedded-rust-*` — the 4 new agents

## Gotchas

- **`nohup` + background: the completion notification fires for the *launching
  shell*, not the real process.** This bit me twice — I read a half-written
  `package-lock.json` and reported "0 changes" when npm was still writing.
  Verify from the artifact, never from the completion signal.
- **`ruflo mcp --help` auto-starts the background daemon**, which spawns headless
  Claude sessions and burns tokens. I started one accidentally and stopped it
  (`ruflo daemon stop`). Check `daemon status` if tokens drain unexpectedly.
- **PATH order decides which `ruflo` you get.** My own `ruflo --version` was
  3.14.1 early in the session and 3.32.38 after `source ~/.zshrc`. Any version
  claim is only true for the shell that made it.
- **Plane wrapper subcommands take issue UUIDs, not `WEFT-N`** — passing the
  sequence number 404s. `--assignee me` resolves to a bot; the human UUID is
  `0d63f76f-0231-49e8-b81a-b2471bb7b91a`.
- **Agent frontmatter: `: ` and ` #` in an unquoted `description:` silently
  truncate it.** ` #` starts a YAML comment. A truncated description under-routes
  with no error anywhere — `embedded-acoustic-firmware` had lost 390 of 760 chars
  this way. Verify with a yaml round-trip comparing written vs parsed length.
- **The firmware crates are out-of-workspace** (empty `[workspace]` table is
  load-bearing — it keeps the Xtensa toolchain out of the host workspace).
  Confirm whether `scripts/build.sh` reaches them before claiming a build result.

## Memory anchors

Durable cross-session facts live in
`~/.claude/projects/-Users-mathewbeane-weftos/memory/` — start at `MEMORY.md`.
Most relevant here: `weftos-operational-gotchas` (the store split, the CI gate
hole, the cycle lock, the frontmatter trap), `weftos-plane-access` (tracker
state + the cycle decision), `embedded-rust-brain-and-agents`.

Searchable in the store: `weftos/bugs`, `weftos/architecture`, `weftos/roadmap`,
`weftos/adr` carry this session's findings; `embedded-rust` (40 chunks) is
quarantined and must never be merged into `weftos/*`.
