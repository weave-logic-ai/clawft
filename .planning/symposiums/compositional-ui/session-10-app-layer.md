# Session 10 — Modes, Inputs, and the App Layer

**Date**: 2026-04-20
**Round**: 3 (post-canon-implementation)
**Purpose**: Close the gap between *canon primitives exist* (shipped 2026-04-20) and *users can interact productively with WeftOS through a panel*. The canon is the vocabulary; this session designs the sentences, paragraphs, and the books they belong to.
**Inputs**: foundations §four-predicates, session-4 (ontology→UI), session-7 (dev-panel embedding), session-9 (agentic-OS canon), ADR-010 (reverse-DDD mapping), ADR-011 (dev-panel embedding hybrid), protocol-spec §7 (surface.compose verbs), the canon merge commit `7ddc5ee` (20 primitives live).

**Output**: five ADRs (015–019) and a mode-crosswalk for four reference panels. Directs the next implementation milestone (M1.5 "one real panel driven by a surface description") ahead of M2 (active-radar loop).

---

## 1. Context — what's built, what's missing

Two weeks of symposium work plus two days of parallel-agent implementation produced:

- **Canon** (21 primitives, `crates/clawft-gui-egui/src/canon/`). Trait `CanonWidget`, typed return-signals (topology/doppler/range/bearing), ADR-006 head fields. Demo lab wired into the WeftOS panel under the new `Canon` section.
- **Wire protocol** (`protocol-spec.md`, WSP-0.1, 11k words, 20 verbs, 18 primitives — now 21).
- **Kernel daemon** (`weaver kernel start`) with six registered services and a UDS socket.
- **VSCode extension bridge** — webview loads the egui wasm bundle, four RPC verbs proxied through the daemon, wasm hot-reload (`6e2df4d`).

Still missing:

1. **A way to author an app.** Today `Desktop::render_blocks_window` is a hand-coded `match` over `BlockKind::ALL`. There's no declarative path from "I want a panel that shows the current project's CI/CD status" to a running surface.
2. **Ontology-driven rendering.** ADR-010 and session-4 prescribe the rule `aggregate root → Surface, entity → Chip, value → Field`, but no code turns a live ontology graph into a layout. The canon primitives wait for a composer that doesn't exist.
3. **Data adapters.** `Live` polls four kernel RPC verbs. That's the sum total of WeftOS's connection to the outside world. No git adapter, no GH adapter, no workspace/file adapter, no LSP bridge, no filesystem watcher.
4. **An app model.** There is no concept of "an app" as a first-class artifact. Installed where? Launched how? With what permissions? Versioned how?
5. **Mode differentiation.** The panel only has one shape (the hand-coded `Desktop` shell). An operator using it in Cursor gets a mini-desktop-OS inside a webview tab, which is structurally wrong. A standalone WeftOS workstation would want the same code path but a different chrome. A hands-free avatar companion — entirely different again.

M2 as currently scoped (active-radar loop, voice capture, workspace-topic publication) would close the *observation* loop — but without an app layer to observe, it's instrumentation without a program. This session designs the program.

## 2. Design axes

Two orthogonal axes at the session level:

### 2.1 Mode (presentation / chrome / app-model)

| Mode | Chrome | Surface concurrency | App registry | Host integration |
|------|--------|---------------------|--------------|------------------|
| `single-app` | minimal (optional thin title strip + tray-chip row for captures) | exactly one | fixed at launch | passive |
| `desktop` | full (wallpaper, launcher, tray, windowed surfaces) | many | dynamic (install / launch / terminate) | self-contained |
| `ide` | full (= desktop chrome) + IDE bridge tools | many | dynamic | bidirectional — subscribes to host editor signals, exposes ide.* affordances back |

**`ide` is explicitly a *superset* of `desktop`, not a sibling.** The compositor code paths for launcher / windows / tray / registry are shared; `ide` is `desktop` with the IDE bridge module active. This minimises duplication and guarantees that any app working in `desktop` also works in `ide`.

**Modes are fixed at startup.** No runtime transitions. `single-app` in particular is a **locked kiosk** — the app cannot spawn additional surfaces, cannot launch other apps, cannot escape to a launcher. If the app wants to show more, it composes it inside its single surface. Escalating to multi-surface requires relaunching in `desktop` mode.

### 2.2 Input (interaction modality)

| Input | Primary interaction | Adaptation |
|-------|--------------------|------------|
| `pointer` | mouse + keyboard | default canon rendering |
| `touch` | finger / stylus | hit targets ≥ 44 px, hover demoted to long-press, swipe gestures, thumb-reachable chrome |
| `voice` | speech + TTS | affordances declare voice-invocable names, transcript+caption surface, wake-word |
| `hybrid` | multiple channels active simultaneously | the compositor auto-adapts per-frame based on the last-used channel |

**Input is also fixed at startup**, with one exception: `hybrid` is the default-on-detection promotion target. If a `pointer` session registers a touch event, the compositor upgrades to `hybrid` (adds touch affordances without losing pointer).

### 2.3 Composition of axes

3 modes × 4 inputs = 12 combinations. Common ones, named:

| | pointer | touch | voice | hybrid |
|---|---|---|---|---|
| **single-app** | dashboard-in-tab | wall kiosk | **avatar** | convertible tablet |
| **desktop** | standalone workstation | tablet OS | voice-controlled workstation | most common |
| **ide** | **Cursor/VSCode today** | iPad + VSCode | voice-pair-programmer | touchscreen dev laptop |

`avatar` is the canonical `single-app × voice` combination. Treated as an **app archetype**, not a mode — the archetype declares its chrome (a `foreign://avatar` character surface), its input (`voice` or `hybrid`), and its runtime contract (narration of subscribed ontology topics).

## 3. The app layer

### 3.1 What "an app" is

An app is a declaration of:

1. **Identity** — IRI + display name + icon.
2. **Supported modes** — subset of `{single-app, desktop, ide}`.
3. **Supported inputs** — subset of `{pointer, touch, voice, hybrid}`.
4. **Surfaces** — one or more surface descriptions (§4). `single-app` apps have exactly one; `desktop` apps may have several (one per virtual workspace-window).
5. **Ontology subscriptions** — which substrate topics the app's surfaces read (§5).
6. **Ontology influences** — which affordances the app exposes that can write back (e.g. "kill process", "deploy this build").
7. **Narration contract** (optional) — if the app declares `supported_inputs` including `voice`, a narration rule that turns subscribed topic state into speakable summaries.
8. **Permissions** — capture channels the app needs (camera/mic/screen/filesystem), governance-gated at install time.
9. **Entry points** — which host commands launch the app (`weaver gui`, VSCode command, system-tray icon, wake-word phrase).

ADR-015 formalises the manifest schema.

### 3.2 Who authors an app

Three authoring surfaces, in decreasing order of power:

1. **Rust crate** — full access to canon + ontology + RPC. An app is a Rust crate with a standard manifest and one or more `impl Surface` types. This is how WeftOS first-party apps (admin, project-OS) ship.
2. **Declarative TOML/RON document** — a static surface description with no code. Ontology bindings are paths (`substrate/kernel.processes`). Affordances are named verbs (`wsp.activate`). The compositor reads the document and builds a surface. This is how community apps ship without needing to compile Rust.
3. **Agent-authored** — an agent writes a document at runtime in response to user intent (`"make me a dashboard for my open PRs"`) and submits it via `surface.compose`. Governance approves, compositor renders.

ADR-016 formalises the surface description format for (2) and (3), with (1) being a Rust-level builder that emits the same internal representation.

### 3.3 How an app gets data

Apps subscribe to **ontology topics** — named paths in the substrate's state graph. Topics are populated by **ontology adapters**:

- `kernel` adapter (built in) — publishes `substrate/kernel/{status, processes, services, logs}` from the daemon.
- `git` adapter — publishes `substrate/git/{branch, status, log, diff}` from the workspace repo.
- `gh` adapter — publishes `substrate/gh/{issues, prs, workflows, deployments}` from the GitHub API (authenticated).
- `workspace` adapter (M2) — publishes `substrate/editor/{buffer, cursor, diagnostics, focus, terminal, tasks}` from the host IDE.
- `fs` adapter — publishes `substrate/fs/{tree, watched-paths}` from the local filesystem.
- `lsp` adapter — publishes `substrate/lsp/{symbols, diagnostics, references}` from the active LSP.
- `deployment` adapter — publishes `substrate/deploy/{environments, health, latest-release}` from Vercel/Fly/k8s.

Each adapter implements a common trait: subscribe, emit state deltas (`open / chunk / commit / cancel` per §protocol-spec stream verbs), handle governance checks on capture. Adapter registration is an app-scoped concern (an app declares which adapters it needs); governance mediates install-time capture permissions.

ADR-017 formalises the adapter contract and ships reference implementations for `kernel`, `git`, `gh`, and `workspace`.

### 3.4 How a surface gets rendered

The composer runtime:

1. **Resolves** the surface description — replacing ontology path placeholders with concrete `Substrate::snapshot()` reads, threading `variant_id`s, intersecting raw affordances with governance gates (§ADR-006 rule 2), producing a concrete primitive tree.
2. **Walks** the primitive tree, calling `canon::<Primitive>::show(ui)` for each node and emitting the `CanonResponse` return-signals onto the observation stream (§session-5 return-signal schema).
3. **Stamps** every `surface.update` with a `variant_id` so ECC can attribute echoes (§ADR-007).

This is the *hot path*. Every frame runs this pipeline. Performance target: ≤ 2 ms CPU for a 50-primitive surface on a modern laptop (egui's native redraw cost is already ~1 ms).

## 4. The IDE bridge

In `ide` mode, four linkage components bring the panel into bidirectional conversation with the host editor:

1. **Editor-in adapter.** Ontology topics under `substrate/editor/*`: `buffer` (active file + dirty), `cursor` (line/col + selection range), `diagnostics` (LSP errors/warnings for the active file), `focus` (which pane/tab has focus), `terminal` (integrated terminal output stream), `tasks` (running jobs, debug sessions).
2. **Editor-out affordances.** Named WSP verbs under the `ide.*` namespace that surfaces can invoke: `ide.open-file(path, line?)`, `ide.show-quick-pick(items)`, `ide.insert-at-cursor(text)`, `ide.reveal-in-explorer(path)`, `ide.run-command(id, args)`, `ide.write-terminal(text)`.
3. **Follow-editor apps.** An app manifest may declare `on-focus-change`: a rule that re-routes the active surface when the editor focus changes. The classic case: a project-OS app swaps to the project whose file is currently active.
4. **Chrome matching.** Optional colour/font bridge — the panel inherits the host's theme palette so it visually belongs.

ADR-018 specifies the message schema (postMessage for VSCode/Cursor, JSON-RPC for JetBrains) and the topic/affordance catalogs.

## 5. Input-modality adaptation

Input is cross-cutting — the compositor consults the session-level `input` value and adjusts every primitive's rendering:

**Hit-target floor.** Canon gains a `min_hit_size()` method; the compositor clamps each primitive's bounding box up to `44×44` when `input ∈ {touch, hybrid, voice}` (voice mode still needs touch fallbacks for confirm).

**Gesture mapping.**
- `click` → tap (all modes).
- `right-click` → long-press (touch/hybrid), context-menu affordance (voice: "show options").
- `drag` → single-finger drag on pannable primitives (Canvas, Plot), swipe on scrollable ones (Sheet, StreamView).
- `scroll-wheel` → two-finger scroll. Pinch on pinchable primitives (Canvas zoom).

**Hover semantics.**
- `pointer`: hover shows tooltip.
- `touch`: long-press shows tooltip.
- `voice`: tooltip content is narrated on "what is this".
- `hybrid`: hover if pointer present, long-press else.

**Voice affordance invocation.** Each `Affordance` (§ADR-006) gains an `invocations: [pointer, touch, voice, gesture]` field declaring which channels can trigger it. Invoking a non-declared channel is malformed (governance-level).

**Safe-area insets.** iOS notches, Android nav bars, foldable hinges. Compositor reads `ctx.input(|i| i.viewport())` and pads canvas rect.

**IME awareness.** Modal/floating surfaces respect the reduced viewport when a virtual keyboard is up.

**`foreign://avatar`.** New Tier-B ForeignSurface kind. The avatar shell is Tier-A (`egui::Frame` + caption stream + tray chip for voice capture); the character renderer is opaque (Unity, Live2D, Ready Player Me, WebGL shader). Same pattern as `foreign://browser`/`foreign://terminal`.

**Narration contract.** Per-app declaration in the manifest — a rule turning subscribed topic state into a speakable summary. Example:

```toml
[narration]
"substrate/admin/health" = """
{{count(services, s -> s.status = "healthy")}} services healthy,
{{count(services, s -> s.status = "at_risk")}} at risk.
"""
```

Avatar apps consume these declarations — the avatar itself doesn't know anything about admin/project/deploy internals. It just subscribes to topics from installed apps and reads their narration rules.

ADR-019 formalises input adaptation, `foreign://avatar`, and the narration rule language.

## 6. Reference panels (crosswalk)

Four concrete panels, each designed in ≥ 2 mode × input combinations, to prove the abstractions. These become the acceptance targets for M1.5.

### 6.1 WeftOS Admin

**Content**: kernel status chip, process table, service health gauges, live log StreamView, rpc affordances (restart service, kill process).
**Ontology**: `substrate/kernel/{status, processes, services, logs}`.
**Adapters**: `kernel` (built in).

| Mode × Input | Chrome | Layout |
|---|---|---|
| `ide × pointer` | side panel in Cursor | vertical Strip: status chip row → table → logs |
| `desktop × pointer` | floating window | Grid: overview / ps / services / logs quadrants |
| `single-app × touch` (wall kiosk) | full viewport | one Tab strip at the bottom: Status / Processes / Logs |
| `single-app × voice` (avatar) | avatar character | subscribes, narrates: *"three services healthy, one at risk: mesh listener is intermittent."* |

### 6.2 Project OS (the clawft-specific panel)

**Content**: git branch + status chip, open PRs table, CI/CD pipeline gauges per env, recent deployments StreamView, open issues Tree, affordance "run gate checks locally".
**Ontology**: `substrate/git/*`, `substrate/gh/*`, `substrate/deploy/*`.
**Adapters**: `git`, `gh`, `deployment`.

| Mode × Input | Chrome | Layout |
|---|---|---|
| `ide × pointer` (default) | right-side Cursor panel | Tabs: Status / PRs / Deploys. Follow-editor swaps project when workspace changes. |
| `desktop × pointer` | standalone workstation window | three-pane Strip: issues Tree left, PR table center, deploy StreamView right |
| `single-app × pointer` (browser tab dashboard) | minimal top bar | same as desktop version, maximized |

### 6.3 Deployment Monitor

**Content**: environment health gauges (prod/staging), recent deploys StreamView, affordance "rollback", "promote staging → prod".
**Ontology**: `substrate/deploy/{environments, health, latest-release}`.
**Adapters**: `deployment`.

| Mode × Input | Chrome | Layout |
|---|---|---|
| `single-app × pointer` (ops screen) | full viewport | Grid: gauges top, StreamView bottom |
| `single-app × touch` (wall dashboard) | full viewport, big text | single column Stack, Chips show env state |
| `single-app × voice` (avatar companion) | avatar | narrates: *"prod healthy. staging had a deploy 12 minutes ago — all checks green."* |

### 6.4 Avatar Companion

**Content**: the avatar itself + a captions StreamView + subscribed narration contracts from other installed apps.
**Ontology**: depends on which other apps are installed.
**Adapters**: mic (M2 `weft-capture`) + TTS + optional camera.

| Mode × Input | Chrome | Layout |
|---|---|---|
| `single-app × voice` (canonical) | `foreign://avatar` character surface | character top-half, caption stream bottom-third, capture-chip top-right |
| `single-app × hybrid` (tablet with mic) | avatar + touch-to-confirm buttons | same as above + inline confirm/dismiss buttons on destructive narration ("about to rollback prod — confirm?") |

## 7. Milestones

M1 (canon) shipped 2026-04-20. This session inserts **M1.5** ahead of M2.

### M1.5 — One real panel driven by a surface description

Acceptance criteria:
1. A minimal app-manifest crate exists (`clawft-app` or similar).
2. A minimal surface-description parser exists, handling at least IRIs + Stack/Strip/Grid + bindings to ontology paths.
3. The `kernel` adapter is refactored to emit ontology topic deltas (not just RPC polls).
4. The **WeftOS Admin** reference panel (§6.1) is implemented as a declarative surface description, registered as an app, and launchable in `ide × pointer` (Cursor) and `desktop × pointer` (`weaver gui`).
5. The `Desktop::render_blocks_window` hand-coded match is removed; the Blocks window is itself a declared app.

Out of scope for M1.5: git/gh/deploy adapters, voice/avatar, touch adaptation, follow-editor, full IDE bridge. Those unlock incrementally in M1.6–M1.9.

### M1.6 — IDE bridge editor-in

Adds `substrate/editor/*` topics (workspace adapter from M2, hoisted forward). Project-OS panel becomes viable (§6.2).

### M1.7 — IDE bridge editor-out

Adds the `ide.*` affordance catalog (open-file, quick-pick, etc.). Panels can now reach back into the editor.

### M1.8 — Touch adaptation

Compositor hit-target floor, gesture mapping, safe-area insets. Deployment Monitor on tablet viable (§6.3 row 2).

### M1.9 — Voice + avatar

`foreign://avatar`, narration contract parser, `weft-capture` (M2 precursor used here). Avatar Companion viable (§6.4).

### M2 (revised) — Active-radar loop closed end-to-end

Unchanged in spirit but now has apps to instrument. Return-signal emission, `variant-id` stamping through real widgets, goal+governance binding. Delivered *on top* of the app layer, not in parallel.

## 8. Recommendations (numbered for ADR crosswalk)

1. **Treat mode, input, and app as orthogonal concepts.** Three ADRs cover them (015 app manifest, 018 IDE bridge for `ide` mode specifically, 019 input adaptation). No omnibus.
2. **Single-app is locked.** No escape hatch, no launcher, no runtime mode transition. Documented in ADR-015.
3. **Desktop and IDE share the compositor code path.** IDE is desktop + bridge module. Enforced by ADR-018 (bridge is opt-in at session-launch time).
4. **Avatar is an app archetype**, not a fourth mode. `single-app × voice` with `foreign://avatar` as the surface. Documented in ADR-019.
5. **App manifests are TOML.** Cargo-familiar, strongly typed, comment-friendly. ADR-015.
6. **Surface descriptions are TOML-with-extensions** for declarative apps, with an equivalent Rust builder for code-level apps. Both compile to the same internal `SurfaceTree` representation. ADR-016.
7. **Ontology adapters implement a single trait.** Registered per-app, governance-gated at install. Shipped adapters: `kernel`, `git`, `gh`, `workspace`, `fs`, `lsp`, `deployment`. ADR-017.
8. **IDE bridge is a module** keyed off session config, not a fork of the desktop code. Four components: editor-in, editor-out, follow-editor, chrome-match. ADR-018.
9. **Input is a session-level value**, not per-primitive. The compositor adapts canon rendering; primitives don't know their own input mode. ADR-019.
10. **Voice affordance invocation is declared** on the `Affordance` struct (a new `invocations: &[Invocation]` field). This stays honest with the four-predicate rule: any surface the user voice-commands *must* have declared that verb was voice-invocable. ADR-019 amends ADR-006.
11. **Narration rules live in app manifests**, not in the avatar app. The avatar app subscribes to topics and reads the declared rules. Decentralises speech content; centralises speech rendering. ADR-019.
12. **Every reference panel (§6) ships as a test case.** Integration tests that load a manifest, bind ontology snapshots, render to a headless egui context, and assert primitive counts + affordance IRIs. ADR-016 ships one; ADR-019 adds the avatar case.
13. **Delete `BlockKind` at the end of M1.5.** The hand-coded enum is the tell that the app layer doesn't exist. Once it's replaced by a declarative admin-app manifest, the migration is complete.

## 9. Open questions (deferred)

- **App distribution.** Crates.io? Private registry? IPFS/content-addressed? Not urgent for M1.5.
- **App sandboxing.** Rust apps have full process access; declarative apps are sandboxed by construction (they can only read declared topics). Do we need a middle tier? Probably — but deferable until we ship the first community app.
- **Localisation.** Narration rules in English only for M1.9. i18n is a Round 4 concern.
- **Multiple avatars.** Can a session host two avatars (e.g. "technical" + "executive assistant")? Not designed here; out of scope.
- **Offline / intermittent-network apps.** Some adapters (gh, deploy) hit networks; what happens offline? Topics flag stale. Surfaces show stale-chip affordance. Details in ADR-017.

## 10. Related

- **Sessions**: `session-4-ontology-ui.md` (reverse-DDD mapping provides primitive picker), `session-7-dev-panel-embedding.md` (hybrid embedding for IDE mode), `session-9-agentic-os-canon.md` (agentic-OS framing that justifies apps-not-windows).
- **ADRs**: `adr-001` (canon, which this session consumes), `adr-006` (head; amended by ADR-019 for voice invocation), `adr-010` (reverse-DDD primitive mapping), `adr-011` (dev-panel embedding hybrid — predecessor to IDE bridge), `adr-013` (canvas), `adr-014` (modal modality).
- **New ADRs opened by this session**: `adr-015` (manifest), `adr-016` (surface description), `adr-017` (ontology adapters), `adr-018` (IDE bridge), `adr-019` (input + avatar + narration).
- **Protocol spec**: §surface.compose / §surface.update / §stream verbs (`open/chunk/commit/cancel`) — unchanged, but this session specifies *how* a surface description gets turned into a sequence of those calls.
- **Code**: `crates/clawft-gui-egui/src/canon/*` (the vocabulary), `crates/clawft-gui-egui/src/shell/desktop.rs` (the thing to refactor in M1.5), `crates/clawft-core/src/*` (where `clawft-app` and `clawft-surface` crates will land).
