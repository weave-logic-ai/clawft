# Contributing to WeftOS

WeftOS is a **Rust agent operating system**: kernel, mesh, constitutional
governance (ECC / ExoChain), and clawft agent surfaces. Node packages in this
repo are **dev/orchestration only** (Ruflo, MetaHarness, docs tooling) — they
are not required to run the `weft` daemon.

## Build (mandatory path)

Use `scripts/build.sh` for **all** build, test, check, clippy, and gate work.
Do **not** run raw `cargo build` / `cargo test` / `cargo clippy` for normal
contribution flow (CI and agents follow the same script).

```bash
scripts/build.sh native-debug   # fast local binary
scripts/build.sh check          # compile check
scripts/build.sh test           # workspace tests
scripts/build.sh clippy         # lint (warnings as errors in gate)
scripts/build.sh gate           # full phase gate before release claims
```

npm aliases (same scripts): `npm run build`, `npm test`, `npm run gate`.

## Platform matrix (OS surface)

WeftOS is a **multi-crate, multi-target agent OS** — not a single library. Counts
are measured by `scripts/metaharness/weftos-score.sh` (`platformSurface`).

| Layer | What ships |
|-------|------------|
| **Workspace members** | 50+ crates under `crates/` (kernel, mesh, agent, world-model, voice, GUI, …) |
| **cargo-dist binaries** | Linux gnu/musl (x64+arm64), macOS Intel+Apple Silicon, Windows x64 |
| **WASM / browser / WASI** | `clawft-wasm`, browser package, wasip2 release assets |
| **Edge / mobile** | Android edge core; edge-pad paths |
| **Firmware / hardware (on-disk)** | ESP-IDF pad, leaf display/touch (GT911), LGFX RGB bus, scene/renderer — some path-deps outside the default workspace |
| **Sensors / world model** | sensor pipeline + wire, LeWM core/impls, BVH, splat pipeline, sonobuoy ranging |
| **Voice** | AEC, talk-mode, ONNX, TTS crates (feature-gated heavy deps) |
| **Memory / graph** | canon, COW memory, graphify, exo-core/dag substrate |

Upstream `metaharness score` only sees `crates/: dir` in its shallow inventory —
it does **not** count members or targets. Use **`weftosFoundationScore`** and
**`metaharness genome`** for OS readiness; treat the 5-dim scorecard as a canary.

## Agent harness & MetaHarness

Agent work is first-class in-tree — not a post-hoc scaffold:

| Concern | Where |
|---------|--------|
| Harness tasks (gate, plane-dag, fusion-view) | `.metaharness/tasks/` |
| Foundation + OS surface score (primary) | `scripts/metaharness/weftos-score.sh` |
| Upstream ADR-041 scorecard + **genome** | `scripts/metaharness/score.sh` (writes both JSON files) |
| Score flywheel (evaluate-only) | `scripts/metaharness/flywheel-score-eval.sh` |
| Graph ViewSpecs (sensor fusion fixtures) | `config/views/` + `validate-views.sh` |
| AgentDB patterns | `scripts/metaharness/seed-patterns.sh` |
| Grok rules | `.grok/rules/metaharness.md` |
| Claude / host skills | `.claude/skills/`, project skills trees |

```bash
scripts/metaharness/run-task.sh gate          # before claiming release readiness
scripts/metaharness/run-task.sh plane-dag     # Plane ready work
scripts/metaharness/run-task.sh fusion-view   # ViewSpec + anchors
scripts/metaharness/weftos-score.sh           # weftosFoundationScore (load-bearing)
scripts/metaharness/score.sh                  # ADR-041 inventory scorecard
```

### Policy (default-deny direction)

- **Freeze the model; evolve the harness / ViewSpecs / policies.**
- Changes to policy surfaces use MetaHarness flywheel discipline:
  **evaluate → immutable receipt → explicit promote** (confirm + keys / PR).
  No silent champion edits of ViewSpecs or gate relaxations.
- ECC / LeWM R1–R5 (ADR-090), dual-sign chain kinds, and gate phase requirements
  are not Darwin playgrounds — only human ADR + promote.
- MCP (`.mcp.json`) is an **integration surface** for host tools (claude-flow /
  Ruflo). The product genus remains the **Rust kernel + agent OS**, not
  “an MCP server repo waiting for a scaffold.”

See `.metaharness/README.md`, ADR-096, ADR-097.

## Work tracking

**Plane** is the authoritative tracker for WeftOS / clawft (`weftos` workspace).

```bash
scripts/plane-dag.sh ready --cycle 0.8.x
# or: .claude/skills/plane-workflow/scripts/plane.sh …
```

- Claim → In Progress before code.
- Close with what shipped, commit SHAs, tests/build status.
- Never commit secrets, `.env`, or promote keys.

## Code standards (short)

- Prefer editing existing files; keep modules focused and typed at boundaries.
- Domain-driven crates under `crates/`; docs under `docs/`.
- Never push to `master` from agent workflows unless project rules say otherwise.
- Security-sensitive changes: validate inputs at boundaries; path sanitization;
  no hardcoded credentials.

## Questions

Architecture: `docs/adr/`. Design surfaces: `docs/DESIGN.md`. Handoff context:
`docs/handoff.md` when present.
