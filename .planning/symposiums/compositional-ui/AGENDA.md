# Symposium: Compositional UI for an Agentic OS

**Working Title**: "A wire protocol for surfaces — composed by agents, rendered anywhere, governed by project goals"
**Status**: Planning
**Target**: Sprint 18 (May 2026)
**Renderer canon**: egui (hard pivot, no slint/web in scope)
**Supersedes**: `.planning/sparc/weftos/0.1/weftos-gui-design-notes.md` (Tauri/Next.js era — composition philosophy preserved, renderer swapped)

---

## North Star

The GUI is not an application. It is a **disposable renderer** for a
protocol the substrate speaks. Surfaces are composed by agents at
runtime from:

1. **Task context** — what the user is doing, and what agents are doing alongside them.
2. **Substrate state** — the resource tree, mesh peers, chain events, service registry, live processes.
3. **Ontology** — the data's declared geometry (see `ontology-navigator` symposium) chooses the layout strategy.
4. **Constitutional governance** — project- and task-scoped goals that constrain what the agent may surface, defer, or refuse.

The protocol is the canon. Renderers come and go.

---

## Foundation principle (non-negotiable)

From the user, 2026-04-19:

> Start with composable, semantic primitives — buttons, panels, and
> forms whose properties are defined in an ontology or knowledge graph
> so AI can query and manipulate them. Every component exposes state,
> affordances, and confidence levels. Support async streaming results
> and partial views. Base components should feel like Lego for both
> humans and agents.

Codified in `foundations.md`. Every session, ADR, and protocol design
decision must satisfy these four predicates:

1. **Ontology-addressable** — each primitive has a typed identity in the
   knowledge graph; the agent queries it, doesn't guess.
2. **Self-describing** — state + affordances + confidence are read-
   able on the primitive, not inferred.
3. **Streaming-native** — partial values, progressive disclosure, and
   out-of-order updates are primitive-level concerns, not application-
   level workarounds.
4. **Dual-operator** — the same primitive is usable by a human
   (pointer / voice / keyboard) and by an agent (query / invoke /
   subscribe).

If a proposal violates any of these, it's rejected at the ADR stage.

---

## Literature we explicitly read against

These are **required anchors** for every research session. Each finding
doc cites which ideas from each book it accepts, rejects, or transforms.

### Alan Cooper — *About Face: The Essentials of User Interface Design*

The foundational text behind the Windows design language (Cooper was
one of its pioneers). Key concepts to weigh against our agentic /
compositional frame:

- **Goal-directed design** — users have goals, not tasks. The surface
  should make the goal visible, not drown the user in steps. This maps
  *directly* to our constitutional-governance layer: goals are first-
  class, sessions are transient.
- **Personas** — we must define *who* the agent-composed UI serves
  (developer-in-dev-panel, regional-manager-looking-at-stores,
  migration-war-room-participant). Personas set the composition
  heuristics the AI uses.
- **Postures** (sovereign / transient / daemonic / parasitic /
  auxiliary) — these map cleanly to our Desktop vs Mission Console
  vs embedded-dev-panel modes. Read the book's posture taxonomy
  against the two modes.
- **Behavioral vs interface design** — Cooper's split between what the
  thing *does* and what it *looks like*. Our protocol sits at this
  seam: the agent composes behaviour; the renderer decides
  appearance.
- **Axioms on modality, direct manipulation, undo, idioms vs
  affordances** — these are constraints the protocol must not violate.

### Daniel Rasmus — *The Agentic Operating System* (seriousinsights.net, 2025)

**Mandatory reading for every session.** Gets its own full research
track (RQ9 — "Agentic OS canon"). Rasmus writes the clearest statement
we have of *why this matters and what must be built*:

- **Four-layer reference architecture**: Semantic Foundation (vector-
  native store + knowledge graph), Probabilistic Kernel (arbitrates
  intent under uncertainty), Model Control Plane (context window as
  working memory), Specialized Agents (janitor, gatekeeper, archivist,
  strategist — replace "features" in today's apps).
- **Episodic UI**: "The system throws up exactly the UI needed to
  confirm a decision, tweak parameters, or explore alternatives.
  It then gets out of the way." This *is* our Mission Console
  ephemeral-renderer model, written down.
- **Six unsolved problems** he names — we already have substantive
  answers to most, and the symposium needs to surface them:
  1. **Identity and delegation** → ExoChain + dual signing + RVF
     crypto + governance gates.
  2. **Asynchronous execution** → mesh runtime + HeartbeatTracker
     + MeshClockSync + ECC for interruption prioritisation.
  3. **Multi-agent protocols** → MCP + our own A2A router + claims
     authorization.
  4. **Economics** → DeFi layer + bond staking + trust progression
     + slashing.
  5. **Training data lock-in** → ECC + reasoningbank + portable
     trajectories (we do not store user profiles; we store *causal
     models*).
  6. **Serendipity vs. efficiency** → ECC deliberately introduces
     productive disruption via hypothesis generation.
- **Disruption argument**: "Microsoft needs to scrap Windows and start
  over." The player who open-sources the coordination protocol and
  gets it adopted widely wins. This is our market position.

### Mei et al. — *AIOS: LLM Agent Operating System* (arXiv 2403.16971) + `agiresearch/AIOS`

Academic + reference-implementation prior art for an LLM-specific
kernel. Components: Scheduler, Context Manager, Memory Manager,
Storage Manager, Tool Manager, Access Control, LLM Manager. We
already ship analogues for most of these — the symposium should do
an explicit crosswalk (ours → theirs → gaps). Key lesson: AIOS has
no real surface-composition model, only an SDK. Our contribution is
to fill that gap.

### CedarCopilot — `cedar-OS`

Dormant React framework that got several primitives right:
`useCedarState` (agent-readable app state), Shadcn-style "you own the
code", context-aware mentions (@user/@file/@state), Spells & Quick
Actions radial menu, provider-agnostic agent backends. **Positive
lesson**: state registration + semantic mentions are primitive-level
concerns. **Negative lesson**: React-only and no confidence or
partial-view model — we do not want to inherit its framework lock-in
or its missing pieces.

### Eric Evans — *Domain-Driven Design: Tackling Complexity in the Heart of Software*

What we're building is arguably **DDD-in-reverse**. Instead of
modelling the domain and then designing an interface, we let the
*data's own declared geometry* (the ontology) drive the surface. That
flip is non-trivial and worth studying rigorously.

- **Ubiquitous language** — the ontology is the ubiquitous language
  made executable. Domain terms become schema types become surface
  primitives.
- **Bounded contexts** — each project / business / substrate tenant is
  a bounded context with its own vocabulary. The governance layer
  enforces context boundaries — an agent composing a surface in one
  context can't leak terms from another.
- **Aggregates, entities, value objects** — these correspond to
  substrate nodes, their subscriptions, and their rendered chips.
  Aggregate roots become the top of a surface; value objects become
  leaves.
- **Context mapping** — when Mission Console spans contexts (a
  regional-manager looking at many stores), we need an explicit
  context-map surface primitive.
- **Strategic vs tactical DDD** — the symposium produces both: a
  strategic framing (the protocol, the canon) and tactical pieces
  (the primitive set, the egui renderer).

**Reverse-DDD framing** — classical DDD runs `domain → model → code →
UI`. We run `substrate state + ontology → schema → composition →
renderer`. The arrow flips at the "UI" step: the UI is not the output
of modelling, it is a *view* into a modelled substrate. Every research
session interrogates this reversal: *where does classical DDD still
apply, and where does the reversal change the rules?*

---

## Non-Goals

- **No framework lock-in** — no Tailwind, no React, no Next.js, no Tauri. egui is the current primary renderer; the protocol must be portable.
- **No brand-color identity yet** — foundation first (typography, spacing, elevation, motion, density, radius, stroke). Color comes later.
- **No hand-designed screens** — every surface in the system is composed from primitives. If we find ourselves writing bespoke layout code, the protocol is wrong.
- **No CLI-shaped chat UI** — terminals are dead weight for agent interaction. We design for voice, interruption, multi-agent parallel output, and persistent goals from the start.

---

## Two Modes, One Protocol

| | **Desktop** | **Mission Console (priority)** |
|-|-|-|
| Users | one (ambient personal) | many (shared, real-time) |
| Surface | windows, tray, launcher | task-centric, everyone sees everything |
| Runs on | personal node | embedded in Cursor/VSCode panel *first*, standalone second |
| Example | open a file, read ExoChain head, run `weft analyze` | migration war-room, counselling session, live data exploration |
| Ordering | secondary | **primary — ship first if forced to choose** |

Both modes use the same primitives, same protocol, same substrate subscriptions. Mode is a hint, not a fork.

**The dev-panel surface is priority one**: it is the loop-closer that gets us out of CLI-in-terminal for the way we actually build software. It must handle voice input, interruptions, multiple concurrent agent threads with their reasoning visible, project-scoped constitutional goals that outlive individual agent sessions, and ECC-backed task/goal persistence so nothing is lost across context compactions or session restarts.

---

## The Research Questions

### RQ1 — Historical canon

What made prior protocol-defined UIs portable (X11, Wayland, NeWS, Display PostScript, AT-SPI, NSDocument, Morphic, Naked Objects)? What locked them in (Smalltalk-only Morphic, PS-only NeWS)? What would we keep, what would we reject?

### RQ2 — Agentic UI state of the art

How do v0, AI Elements, Continue.dev, Cursor's panels, Cline/Roo, Aider, Plandex, Open Interpreter, AutoGen, Copilot Chat, Devin, Claude Code surface agent state today? Where do they fail (CLI-in-terminal, single-thread, no goal persistence)? What's the best existing answer and what does it still miss?

### RQ3 — Shared real-time surfaces

Figma multiplayer, Liveblocks, Yjs/CRDTs, Google Docs OT, Plan 9 Acme (shared verbs), NORAD/mission-control, Twitch overlays, war-room dashboards. Which patterns survive when the surface is composed by agents instead of drawn by designers?

### RQ4 — Ontology → UI

Naked Objects, LinkedDataFragments, Polar, Roam/Obsidian/Notion, the ontology-navigator findings. How does data geometry select layout strategy without a human designer in the loop?

### RQ5 — Renderer contracts

React Native, Flutter, Lynx (ByteDance), Unity UI Toolkit, Epic Slate, Adaptive Cards, IMGUI patterns, egui internals. What's the smallest primitive set that composes into "everything we need"? How does it map to egui's immediate-mode model without leaking?

### RQ6 — Protocol design

LSP, MCP, Matrix, ActivityPub, Cap'n Proto. What's the right schema shape for a surface protocol — CBOR/FlatBuffers? Event-sourced or state-diff? How do subscriptions to substrate state join the composition? How do goal/governance assertions travel?

### RQ7 — Dev-panel embedding (priority)

How does an egui surface embed in Cursor/VSCode? Webview with wasm build, native sidecar with a custom protocol, X11-style remote renderer? What's the voice-input path? How do we surface multi-agent output without stealing focus? How do interruptions propagate?

### RQ8 — Goals as constitutional governance

How do we express project-scoped goals so they outlive agent sessions and constrain what surfaces the agent may compose? This connects to the existing `governance-counsel` agent, ADR-style decisions, and the ECC model. Goals become part of the substrate, not the agent memory.

### RQ9 — Agentic OS canon (dedicated track)

Read Rasmus in full. Crosswalk his four-layer reference architecture
against our stack (kernel, mesh, ExoChain, governance, ECC). For each
of his six unsolved problems, document our answer, its maturity, and
what's still missing. Read Mei et al. (AIOS arXiv) and
`agiresearch/AIOS` as the academic prior art — document gaps. Read
`cedar-OS` for positive and negative primitive-design lessons. Output
is a single synthesis doc that becomes the *framing paper* for the
rest of the symposium — every other RQ's findings are read against
it. This is the heaviest single track; budget more time.

---

## Sketches (non-binding)

See `sketches/` for two rough directional images (beach/crab + cosmos/
crab) with a frosted-glass dock and a named starter app set —
`Weft`, `Navigator`, `Supervisor`, `Cosmos`, `ExoChain`, `Mesh`,
`Forge`, `Control`. These are *one possible direction*, not a target.
`sketches/README.md` enumerates what to take and what not to take.
The starter-app set is the single most useful thing to carry forward:
each of those eight names is a candidate Mission Console surface
definition (RQ5 / RQ7 should test the protocol against all eight).

---

## Process

- **Round 1** — parallel research (one agent per RQ, ~half-day each, writing `session-N-findings.md`).
- **Round 2** — synthesis + ADRs (one ADR per canonical decision: protocol shape, primitive set, composition model, governance binding, embedding strategy).
- **Round 3** — spec the wire format (CBOR IDL, verbs, subscriptions, mode hints).
- **Round 4** — prototype: rewrite the current egui shell under the protocol, prove the dev-panel surface first (chat + channels + files + notifications + multi-agent threads + goal persistence), then the desktop shell on top.

---

## Deliverables

1. `session-N-findings.md` × 8 (one per RQ).
2. `adr-001` through `adr-NNN` capturing each canon decision.
3. `protocol-spec.md` — the surface wire format.
4. `FINAL_REPORT.md` — toolkit + roadmap.
5. Working prototype: egui dev-panel that replaces our current Cursor-chat workflow.

---

## Success criteria

- A second renderer (even a toy one) can be written against the protocol without touching the composition logic.
- An agent can compose a novel surface (given a task + schema) that none of us designed, and it reads correctly.
- Project goals persist across agent-session boundaries and visibly constrain what surfaces the agent produces.
- The dev panel makes the current CLI/terminal chat feel obsolete within our own day-to-day.
